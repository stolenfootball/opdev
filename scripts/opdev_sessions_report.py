"""Export reviewed measurements without publishing private model transcripts."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def aggregate(rows):
    complete = bool(rows) and all(r.get('usage') is not None for r in rows)
    accepted = sum(r['outcome'] == 'passed' for r in rows)
    total = sum((r.get('usage') or {}).get('total_tokens', 0) for r in rows)
    result = {'runs': len(rows), 'accepted': accepted,
              'outcomes': {k: sum(r['outcome'] == k for r in rows) for k in ('passed', 'failed', 'error')},
              'usage_complete': complete, 'known_total_tokens': total,
              'tokens_per_accepted': total / accepted if accepted and complete else None,
              'median_seconds': statistics.median(r['seconds'] for r in rows) if rows else None,
              'slowest_seconds': max((r['seconds'] for r in rows), default=None),
              'median_tokens': statistics.median(r['usage']['total_tokens'] for r in rows) if complete else None,
              'tool_commands': sum(r.get('tool_commands', 0) for r in rows),
              'web_searches': sum(r.get('web_searches', 0) for r in rows),
              'compact_calls_proxy': sum(r.get('compact_calls', 0) for r in rows),
              'full_reads_proxy': sum(r.get('full_reads', 0) for r in rows),
              'false_gate_claims': sum(r.get('assessment', {}).get('checks', {}).get('honest_gates') is False for r in rows)}
    for key in ('input_tokens', 'cached_input_tokens', 'uncached_input_tokens', 'output_tokens',
                'reasoning_output_tokens', 'cache_write_input_tokens'):
        result[key] = sum(r['usage'][key] for r in rows) if complete and all(r['usage'].get(key) is not None for r in rows) else None
    return result


def report(source, verify_checkouts=False):
    manifest = json.loads((source / 'manifest.json').read_text())
    schedule = json.loads((source / 'schedule.json').read_text())
    if manifest.get('schedule_sha256') != hashlib.sha256(json.dumps(schedule).encode()).hexdigest():
        raise ValueError('schedule differs from frozen manifest')
    rows = [json.loads(line) for line in (source / 'records.jsonl').read_text().splitlines() if line.strip()]
    keys = [(r['case'], r['arm'], r['repeat']) for r in rows]
    if len(set(keys)) != len(keys) or keys != [tuple(j) for j in schedule[:len(rows)]]:
        raise ValueError('records differ from preregistered order or contain duplicates')
    for row in rows:
        name = f"{row['case']}-{row['arm']}-{row['repeat']}"
        raw = source / (name + '.events.jsonl')
        if digest(raw) != row.get('events_sha256'):
            raise ValueError('event hash mismatch')
        if row['usage'] is not None:
            turns = [e for line in raw.read_text().splitlines() if line.strip()
                     if (e := json.loads(line)).get('type') == 'turn.completed']
            if len(turns) != 1:
                raise ValueError('ambiguous terminal usage')
            u = turns[0]['usage']
            for key in ('input_tokens', 'cached_input_tokens', 'output_tokens'):
                if type(u.get(key)) is not int or u[key] < 0 or u[key] != row['usage'][key]:
                    raise ValueError('usage differs from saved events')
            if u['cached_input_tokens'] > u['input_tokens']:
                raise ValueError('cached input exceeds input')
            for key in ('cache_write_input_tokens', 'reasoning_output_tokens'):
                value = u.get(key)
                if value is not None and (type(value) is not int or value < 0):
                    raise ValueError('invalid optional usage')
                if value != row['usage'].get(key):
                    raise ValueError('optional usage differs from saved events')
            if (row['usage']['total_tokens'] != u['input_tokens'] + u['output_tokens'] or
                    row['usage']['uncached_input_tokens'] != u['input_tokens'] - u['cached_input_tokens']):
                raise ValueError('inconsistent derived usage')
    groups = {a: aggregate([r for r in rows if r['arm'] == a]) for a in ('baseline', 'compact')}
    cases = {c: {a: aggregate([r for r in rows if r['case'] == c and r['arm'] == a])
                 for a in ('baseline', 'compact')} for c in sorted({j[0] for j in schedule})}
    complete = len(rows) == len(schedule)
    membership = {}
    for case, arm, repeat in schedule:
        membership.setdefault((case, repeat), set()).add(arm)
    balanced = bool(membership) and all(a == {'baseline', 'compact'} for a in membership.values())
    accepted = balanced and complete and all(r['outcome'] == 'passed' and r.get('usage') for r in rows)
    pairs = []
    lookup = {(r['case'], r['repeat'], r['arm']): r for r in rows}
    for case, repeat in sorted({(r['case'], r['repeat']) for r in rows}):
        base = lookup.get((case, repeat, 'baseline'))
        compact = lookup.get((case, repeat, 'compact'))
        if base and compact and base.get('usage') and compact.get('usage'):
            b, c = base['usage']['total_tokens'], compact['usage']['total_tokens']
            pairs.append({'case': case, 'repeat': repeat, 'baseline_tokens': b, 'compact_tokens': c,
                          'difference_tokens': c - b, 'reduction_fraction': 1 - c / b if b else None})
    summary = {'schema': 1, 'planned': len(schedule), 'completed': len(rows), 'complete': complete,
               'arms': groups, 'cases': cases, 'pairs': pairs,
               'all_acceptance_passed': bool(accepted),
               'token_reduction_fraction': 1 - groups['compact']['known_total_tokens'] / groups['baseline']['known_total_tokens']
               if accepted and groups['baseline']['known_total_tokens'] else None,
               'general_development_effectiveness': 'unverified',
               'limitations': ['small synthetic repository', 'one model/host/OS', 'uncontrolled provider cache',
                               'explicit repository-protocol activation', 'controller-owned staging and CI',
                               'no model-driven CI repair, resumed sessions, subagents or installation',
                               'tool/read counts are command-text proxies; no dollar savings inferred']}
    keep = ('case', 'arm', 'repeat', 'usage', 'outcome', 'seconds', 'ci_seconds', 'revision',
            'seed_revision', 'prompt_sha256', 'events_sha256', 'tool_commands', 'web_searches',
            'failed_commands', 'full_reads', 'compact_calls', 'ci')
    sanitized = []
    cli = Path(manifest.get('opdev', ''))
    if verify_checkouts:
        for tool in ('opdev', 'codex'):
            binary = Path(manifest.get(tool, ''))
            if not binary.is_file() or digest(binary) != manifest[tool + '_sha256']:
                raise ValueError('selected executable identity changed: ' + tool)
    for row in rows:
        clean = {k: row[k] for k in keep if k in row}
        clean['acceptance_checks'] = row.get('assessment', {}).get('checks')
        clean['gates'] = row.get('assessment', {}).get('gates')
        if verify_checkouts:
            root = source / f"{row['case']}-{row['arm']}-{row['repeat']}"
            head = subprocess.run(['git', '-C', str(root), 'rev-parse', 'HEAD'], check=True,
                                  capture_output=True, text=True).stdout.strip()
            if head != row.get('revision'):
                raise ValueError('candidate checkout revision differs from recorded result')
            status = subprocess.run(['git', '-C', str(root), 'status', '--porcelain'], check=True,
                                    capture_output=True, text=True).stdout.strip()
            if status:
                raise ValueError('candidate checkout changed after grading')
            clean['fingerprint'] = subprocess.run([str(cli), 'evidence', 'fingerprint', '--root', str(root)],
                                                 check=True, capture_output=True, text=True).stdout.strip()
        sanitized.append(clean)
    public_manifest = {k: v for k, v in manifest.items() if k not in ('opdev', 'codex')}
    public_manifest['source_manifest_sha256'] = digest(source / 'manifest.json')
    public_manifest['records_sha256'] = digest(source / 'records.jsonl')
    public_manifest['source_schedule_file_sha256'] = digest(source / 'schedule.json')
    return summary, sanitized, public_manifest, schedule


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--verify-checkouts', action='store_true')
    parser.add_argument('--feedback', action='store_true', help='Read a separate preregistered workflow feedback experiment')
    args = parser.parse_args()
    if args.feedback:
        if args.output or args.verify_checkouts:
            raise ValueError('feedback mode prints reviewed aggregates only; retain raw observations privately')
        print(json.dumps(feedback_report(args.source), indent=2))
        return
    summary, records, manifest, schedule = report(args.source, args.verify_checkouts)
    if args.output:
        if not summary['complete']:
            raise ValueError('refusing final export of incomplete schedule')
        args.output.mkdir(parents=True, exist_ok=False)
        for name, value in [('summary', summary), ('records', records), ('manifest', manifest), ('schedule', schedule)]:
            (args.output / (name + '.json')).write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(summary, indent=2))


FEEDBACK_METRICS = ('first_feedback_seconds', 'completion_seconds', 'agent_seconds',
                    'command_seconds', 'queue_seconds', 'commands', 'pipelines',
                    'evidence_edits', 'failed_attempts')


def feedback_report(source):
    """Separate protocol; never reinterpret the historical compact experiment."""
    protocol_path = source / 'feedback-protocol.json'
    protocol = json.loads(protocol_path.read_text())
    if protocol.get('schema') != 1 or not protocol.get('identities'):
        raise ValueError('missing frozen workflow identities')
    schedule = protocol['schedule']
    planned = [(s['case'], s['context'], s['arm'], s['repeat']) for s in schedule]
    if len(set(planned)) != len(planned) or any(s[2] not in ('baseline', 'candidate') for s in planned):
        raise ValueError('invalid or duplicate trial schedule')
    criteria = protocol['criteria']
    for key in ('feedback_ratio_max', 'completion_ratio_max'):
        if type(criteria.get(key)) not in (int, float) or not math.isfinite(criteria[key]) or criteria[key] <= 0:
            raise ValueError('invalid comparison threshold')
    rows = [json.loads(line) for line in (source / 'feedback-records.jsonl').read_text().splitlines() if line.strip()]
    actual = [(r['case'], r['context'], r['arm'], r['repeat']) for r in rows]
    if actual != planned[:len(rows)] or len(set(actual)) != len(actual):
        raise ValueError('records differ from the frozen schedule')
    protocol_digest = digest(protocol_path)
    for row in rows:
        if row.get('protocol_sha256') != protocol_digest:
            raise ValueError('protocol changed after observation')
        if row['outcome'] not in ('passed', 'failed', 'error', 'unverified'):
            raise ValueError('invalid trial outcome')
        if set(row.get('oracles', {})) != {'behavior', 'test_preservation', 'gate_honesty'}:
            raise ValueError('independent oracles missing')
        if any(value is not None and type(value) is not bool for value in row['oracles'].values()):
            raise ValueError('oracle results must be true, false or unknown')
        for key in FEEDBACK_METRICS:
            value = row.get('metrics', {}).get(key)
            if value is not None and (type(value) not in (int, float) or not math.isfinite(value) or value < 0):
                raise ValueError('invalid measurement: ' + key)
        evidence = row.get('observation_file')
        if not evidence or Path(evidence).name != evidence or evidence in ('.', '..') or any(c in evidence for c in '/\\:'):
            raise ValueError('observation must be a local basename')
        if digest(source / evidence) != row.get('observation_sha256'):
            raise ValueError('observation changed after review')
    groups = {}
    for arm in ('baseline', 'candidate'):
        selected = [r for r in rows if r['arm'] == arm]
        groups[arm] = {'attempts': len(selected), 'accepted': sum(
            r['outcome'] == 'passed' and all(v is True for v in r['oracles'].values()) for r in selected), 'metrics': {}}
        for metric in FEEDBACK_METRICS:
            observed = [r.get('metrics', {}).get(metric) for r in selected]
            known = [v for v in observed if v is not None]
            groups[arm]['metrics'][metric] = {'observed': len(known), 'missing': len(observed) - len(known),
                                            'median': statistics.median(known) if known else None}
    pairs = {}
    for case, context, arm, repeat in planned:
        pairs.setdefault((case, context, repeat), set()).add(arm)
    balanced = bool(pairs) and all(arms == {'baseline', 'candidate'} for arms in pairs.values())
    complete = len(rows) == len(planned) and bool(rows)
    valid = complete and balanced and all(r['outcome'] == 'passed' and all(v is True for v in r['oracles'].values()) for r in rows)
    ratios = {}
    for metric in ('first_feedback_seconds', 'completion_seconds'):
        b = groups['baseline']['metrics'][metric]
        c = groups['candidate']['metrics'][metric]
        ratios[metric] = c['median'] / b['median'] if valid and not b['missing'] and not c['missing'] and b['median'] else None
    improved = all(value is not None for value in ratios.values())
    improved = improved and ratios['first_feedback_seconds'] <= criteria['feedback_ratio_max'] and ratios['completion_seconds'] <= criteria['completion_ratio_max']
    return {'schema': 1, 'planned': len(planned), 'completed': len(rows), 'complete': complete,
            'acceptance_equivalent': valid, 'arms': groups, 'ratios': ratios,
            'measured_improvement': bool(improved), 'general_effectiveness': 'unverified',
            'limitations': ['Missing observations remain unknown, not zero.',
                            'Queue, command and agent intervals can overlap; never add them as disjoint time.',
                            'Caller-supplied review claims are not authenticated or automatically proved.',
                            'This protocol does not establish cross-model, cross-host or production effectiveness.']}


if __name__ == '__main__':
    main()
