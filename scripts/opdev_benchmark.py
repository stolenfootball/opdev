"""Offline context measurements and bounded, opt-in Codex interpretation trials.

This is an effectiveness experiment, never an OpDev gate or production resolver.
Only synthetic fixtures are sent to a host. Raw events stay in the output folder.
"""
import argparse
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import random
import signal
import statistics
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / 'benchmarks/token-efficiency/cases.json'
ARMS = ('full', 'selected')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()


def load_cases(path=CASES):
    data = json.loads(path.read_text())
    if data['schema'] != 1 or data['kind'] != 'synthetic-context-microbenchmark':
        raise ValueError('unsupported benchmark schema')
    cases = data['cases']
    if not cases or len({c['id'] for c in cases}) != len(cases):
        raise ValueError('empty or duplicate cases')
    for case in cases:
        for key in ('id', 'family', 'question', 'full', 'selected', 'expected'):
            if key not in case:
                raise ValueError('incomplete case')
    return cases


def prompt(case, arm):
    return ('Interpret only the synthetic data below. Do not use tools, inspect files, '
            'delegate, or perform development work. Return only the requested JSON.\n'
            + case['question'] + '\nDATA:\n' + encoded(case[arm]).decode())


def payloads(cases, tokenizer=None):
    count = None
    version = None
    if tokenizer:
        import tiktoken  # Optional, pinned in the benchmark requirements file.
        encoder = tiktoken.get_encoding(tokenizer)
        count = lambda text: len(encoder.encode(text, disallowed_special=()))
        version = importlib.metadata.version('tiktoken')
    rows = []
    for case in cases:
        for arm in ARMS:
            text = prompt(case, arm)
            rows.append({'case': case['id'], 'arm': arm, 'bytes': len(text.encode()),
                         'sha256': digest(text.encode()),
                         'proxy_tokens': count(text) if count else None})
    return {'schema': 1, 'kind': 'static-payloads', 'cases_sha256': digest(encoded(cases)),
            'tokenizer': tokenizer, 'tokenizer_version': version, 'measurements': rows,
            'effectiveness': 'unverified'}


def parse_events(text):
    """Accept one completed turn; reject tools and incomplete usage, not count zero."""
    events = [json.loads(line) for line in text.splitlines() if line.strip()]
    completed = [e for e in events if e.get('type') == 'turn.completed']
    failed = any(e.get('type') in ('turn.failed', 'error') for e in events)
    items = [e['item'] for e in events if e.get('type') == 'item.completed']
    tools = [i for i in items if i.get('type') not in ('agent_message', 'reasoning')]
    messages = [i['text'] for i in items if i.get('type') == 'agent_message']
    usage = None
    if len(completed) == 1:
        raw = completed[0].get('usage', {})
        names = ('input_tokens', 'cached_input_tokens', 'output_tokens')
        if all(type(raw.get(k)) is int and raw[k] >= 0 for k in names):
            if raw['cached_input_tokens'] <= raw['input_tokens']:
                usage = {k: raw[k] for k in names}
                usage['uncached_input_tokens'] = raw['input_tokens'] - raw['cached_input_tokens']
                usage['total_tokens'] = raw['input_tokens'] + raw['output_tokens']
                for name in ('cache_write_input_tokens', 'reasoning_output_tokens'):
                    value = raw.get(name)
                    if value is not None and (type(value) is not int or value < 0):
                        usage = None
                        break
                    usage[name] = value
    # Started tools also invalidate the controlled single-agent experiment.
    started_tool = any(e.get('type') == 'item.started' and
                       e.get('item', {}).get('type') not in ('agent_message', 'reasoning')
                       for e in events)
    return {'usage': usage, 'message': messages[-1] if messages else None,
            'protocol_valid': len(completed) == 1 and not failed and not tools and not started_tool,
            'tool_items': len(tools)}


def grade(case, text, returncode=0):
    result = {'outcome': 'error', 'usage': None, 'tool_items': 0}
    try:
        parsed = parse_events(text)
        result.update({k: parsed[k] for k in ('usage', 'tool_items')})
        if returncode != 0 or not parsed['protocol_valid']:
            return result
        try:
            answer = json.loads(parsed['message'])
        except (TypeError, ValueError):
            result['outcome'] = 'failed'
        else:
            result['outcome'] = 'passed' if encoded(answer) == encoded(case['expected']) else 'failed'
        if result['usage'] is None:
            result['outcome'] = 'unverified'
    except (ValueError, KeyError, TypeError):
        pass
    return result


def schedule(cases, repeats, seed):
    if repeats < 1:
        raise ValueError('repeats must be positive')
    jobs = [(c, arm, repeat) for c in cases for repeat in range(repeats) for arm in ARMS]
    random.Random(seed).shuffle(jobs)
    return jobs


def summarize(records):
    """Keep failures in denominators and compare only complete, compatible pairs."""
    groups = {}
    seen = set()
    for r in records:
        if type(r['repeat']) is not int or r['repeat'] < 0:
            raise ValueError('invalid repeat')
        if type(r['seconds']) not in (int, float) or not math.isfinite(r['seconds']) or r['seconds'] < 0:
            raise ValueError('invalid duration')
        if r['usage'] is not None:
            u = r['usage']
            names = ('input_tokens', 'output_tokens', 'cached_input_tokens',
                     'uncached_input_tokens', 'total_tokens')
            if not all(type(u.get(k)) is int and u[k] >= 0 for k in names):
                raise ValueError('invalid usage counters')
            if (u['cached_input_tokens'] + u['uncached_input_tokens'] != u['input_tokens'] or
                    u['input_tokens'] + u['output_tokens'] != u['total_tokens']):
                raise ValueError('inconsistent usage counters')
            for name in ('cache_write_input_tokens', 'reasoning_output_tokens'):
                value = u.get(name)
                if value is not None and (type(value) is not int or value < 0):
                    raise ValueError('invalid optional usage counter')
        key = (r['experiment'], r['case'], r['repeat'], r['arm'])
        if key in seen:
            raise ValueError('duplicate trial; retries require a new experiment')
        seen.add(key)
        if r['arm'] not in ARMS or r['outcome'] not in ('passed', 'failed', 'unverified', 'error'):
            raise ValueError('invalid trial arm or outcome')
        groups.setdefault(r['experiment'], []).append(r)
    summaries = []
    for experiment, rows in sorted(groups.items()):
        arms = {}
        for arm in ARMS:
            trials = [r for r in rows if r['arm'] == arm]
            accepted = sum(r['outcome'] == 'passed' for r in trials)
            known = [r for r in trials if r['usage'] is not None]
            total = sum(r['usage']['total_tokens'] for r in known)
            complete_usage = len(known) == len(trials) and bool(trials)
            values = sorted(r['usage']['total_tokens'] for r in known)
            arms[arm] = {'runs': len(trials), 'accepted': accepted,
                         'outcomes': {o: sum(r['outcome'] == o for r in trials)
                                      for o in ('passed', 'failed', 'unverified', 'error')},
                         'usage_complete': complete_usage, 'known_total_tokens': total,
                         'tokens_per_accepted': total / accepted if accepted and complete_usage else None,
                         'median_tokens': statistics.median(values) if complete_usage else None,
                         'p95_tokens': values[math.ceil(.95 * len(values)) - 1] if complete_usage else None,
                         'median_seconds': statistics.median(r['seconds'] for r in trials) if trials else None}
            for name in ('cached_input_tokens', 'uncached_input_tokens',
                         'cache_write_input_tokens', 'reasoning_output_tokens'):
                available = complete_usage and all(r['usage'].get(name) is not None for r in known)
                arms[arm][name] = sum(r['usage'][name] for r in known) if available else None
        pairs = {}
        for r in rows:
            pairs.setdefault((r['case'], r['repeat']), {})[r['arm']] = r
        complete = bool(pairs) and all(set(p) == set(ARMS) for p in pairs.values())
        clean = complete and all(r['outcome'] == 'passed' and r['usage'] is not None for r in rows)
        baseline = arms['full']['known_total_tokens']
        summaries.append({'experiment': experiment, 'arms': arms, 'complete_pairs': complete,
                          'observed_quality': 'passed' if clean else 'unverified',
                          'token_reduction_fraction': 1 - arms['selected']['known_total_tokens'] / baseline
                          if clean and baseline else None,
                          'end_to_end_effectiveness': 'unverified'})
    return {'schema': 1, 'kind': 'microbenchmark-summary', 'experiments': summaries}


def run_codex(args, cases):
    """Explicit opt-in; no retries, repository edits, installs, or sandbox bypass."""
    jobs = schedule(cases, args.repeats, args.seed)
    if len(jobs) > args.max_runs:
        raise ValueError(f'{len(jobs)} trials exceed --max-runs {args.max_runs}; select fewer cases')
    if args.timeout <= 0 or args.timeout > 600:
        raise ValueError('timeout must be in (0, 600] seconds')
    args.output.mkdir(parents=True, exist_ok=False)
    version = subprocess.run(['codex', '--version'], capture_output=True, text=True, check=True).stdout.strip()
    settings = {'schema': 1, 'kind': 'synthetic-context-microbenchmark', 'host': version,
                'run_id': str(uuid.uuid4()),
                'model': args.model, 'effort': args.effort, 'repeats': args.repeats, 'seed': args.seed,
                'timeout': args.timeout, 'cases_sha256': digest(encoded(cases)),
                'harness_sha256': digest(Path(__file__).read_bytes()),
                'context': 'fresh-process; provider cache uncontrolled',
                'plugin': 'not activated; context interpretation only',
                'config': 'ignore-user-config; read-only; approval never'}
    experiment = digest(encoded(settings))
    (args.output / 'manifest.json').write_text(json.dumps(settings, indent=2) + '\n')
    records = []
    with tempfile.TemporaryDirectory(prefix='opdev-bench-') as work:
        for case, arm, repeat in jobs:
            name = f"{case['id']}-{arm}-{repeat}"
            text = prompt(case, arm)
            argv = ['codex', 'exec', '--json', '--ephemeral', '--ignore-user-config',
                    '--sandbox', 'read-only', '-c', 'approval_policy="never"',
                    '-c', 'model_reasoning_effort=' + json.dumps(args.effort),
                    '--model', args.model, '--skip-git-repo-check', '--cd', work, '-']
            start = time.monotonic()
            with (args.output / (name + '.jsonl')).open('w') as stdout, (args.output / (name + '.stderr')).open('w') as stderr:
                process = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr,
                                           text=True, start_new_session=os.name == 'posix')
                try:
                    process.communicate(text, timeout=args.timeout)
                    returncode = process.returncode
                except subprocess.TimeoutExpired:
                    if os.name == 'posix':
                        os.killpg(process.pid, signal.SIGKILL)
                    else:
                        process.kill()
                    process.communicate()
                    returncode = 124
            raw = (args.output / (name + '.jsonl')).read_text()
            result = grade(case, raw, returncode)
            result.update({'schema': 1, 'experiment': experiment, 'case': case['id'], 'arm': arm,
                           'repeat': repeat, 'seconds': time.monotonic() - start, 'returncode': returncode,
                           'prompt_sha256': digest(text.encode()), 'events_sha256': digest(raw.encode())})
            records.append(result)
            # Flush each attempt before starting another; timeouts and errors remain visible.
            with (args.output / 'records.jsonl').open('a') as f:
                f.write(json.dumps(result) + '\n')
            print(f"{name}: {result['outcome']}", flush=True)
            if result['outcome'] == 'error':
                break  # Authentication/transport errors should not spend the remaining budget.
    report = summarize(records)
    (args.output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    return 0 if records and len(records) == len(jobs) and all(r['outcome'] == 'passed' for r in records) else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    static = commands.add_parser('payloads', help='offline sizes; optional proxy token counts')
    static.add_argument('--tokenizer', choices=['o200k_base', 'cl100k_base'])
    live = commands.add_parser('run-codex', help='explicit live, potentially billable interpretation trials')
    live.add_argument('--model', required=True)
    live.add_argument('--effort', required=True)
    live.add_argument('--output', type=Path, required=True)
    live.add_argument('--case', action='append', default=[])
    live.add_argument('--repeats', type=int, default=1)
    live.add_argument('--seed', type=int, default=17)
    live.add_argument('--max-runs', type=int, required=True)
    live.add_argument('--timeout', type=int, default=120)
    summary = commands.add_parser('summarize')
    summary.add_argument('records', type=Path)
    args = parser.parse_args()
    try:
        if args.command == 'summarize':
            print(json.dumps(summarize([json.loads(s) for s in args.records.read_text().splitlines() if s.strip()]), indent=2))
        else:
            cases = load_cases()
            if args.command == 'payloads':
                print(json.dumps(payloads(cases, args.tokenizer), indent=2))
            else:
                if set(args.case) - {c['id'] for c in cases}:
                    raise ValueError('unknown case')
                selected = [c for c in cases if not args.case or c['id'] in args.case]
                return run_codex(args, selected)
    except (ValueError, OSError, KeyError, ImportError, subprocess.SubprocessError) as exc:
        parser.exit(2, f'benchmark error: {exc}\n')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
