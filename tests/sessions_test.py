"""Offline protection for coding-session accounting and independent grading."""
import importlib.util
import json
import sys
import tempfile
import threading
import time
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('sessions', Path(__file__).resolve().parents[1] / 'scripts/opdev_sessions.py')
sessions = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sessions)
report_spec = importlib.util.spec_from_file_location('session_report', Path(__file__).resolve().parents[1] / 'scripts/opdev_sessions_report.py')
reporter = importlib.util.module_from_spec(report_spec)
report_spec.loader.exec_module(reporter)


def events(usage=None, extra=None):
    rows = [{'type': 'item.completed', 'item': {'type': 'command_execution', 'command': 'python -m unittest', 'exit_code': 0}},
            {'type': 'item.completed', 'item': {'type': 'agent_message', 'text': '{}'}},
            {'type': 'turn.completed', 'usage': usage or {'input_tokens': 100, 'cached_input_tokens': 60, 'output_tokens': 10}}]
    return '\n'.join(json.dumps(x) for x in rows + (extra or []))


class Accounting(unittest.TestCase):
    def test_counts_terminal_usage_once_with_tools(self):
        got = sessions.parse_session(events())
        self.assertEqual(got['usage']['total_tokens'], 110)
        self.assertEqual(got['usage']['uncached_input_tokens'], 40)
        self.assertEqual(got['tool_commands'], 1)

    def test_missing_usage_and_multiple_turns_rejected(self):
        for raw in [events({'input_tokens': 1}), events(extra=[{'type': 'turn.completed', 'usage': {}}]),
                    events(extra=[{'type': 'error', 'message': 'transport'}])]:
            with self.assertRaises(ValueError):
                sessions.parse_session(raw)

    def test_worker_or_unknown_item_rejected(self):
        with self.assertRaises(ValueError):
            sessions.parse_session(events(extra=[{'type': 'item.completed', 'item': {'type': 'collab_tool_call'}}]))

    def test_cached_input_not_double_counted(self):
        with self.assertRaises(ValueError):
            sessions.parse_session(events({'input_tokens': 1, 'cached_input_tokens': 2, 'output_tokens': 1}))

    def test_web_search_is_a_tool_in_the_same_accounted_session(self):
        got = sessions.parse_session(events(extra=[{'type': 'item.completed', 'item': {'type': 'web_search', 'query': 'fixture work item'}}]))
        self.assertEqual(got['web_searches'], 1)
        self.assertEqual(got['usage']['total_tokens'], 110)

    def test_failed_attempts_stay_in_denominator(self):
        rows = [{'arm': 'baseline', 'outcome': outcome, 'usage': {'total_tokens': 100, 'cached_input_tokens': 0}, 'seconds': 1} for outcome in ['passed', 'failed']]
        result = sessions.summarize(rows, 4)
        self.assertEqual(result['arms']['baseline']['tokens_per_accepted'], 200)
        self.assertIsNone(result['token_reduction_fraction'])

    def test_missing_usage_prevents_savings(self):
        rows = [{'arm': a, 'outcome': 'passed', 'usage': None, 'seconds': 1} for a in ['baseline', 'compact']]
        result = sessions.summarize(rows, 2)
        self.assertIsNone(result['arms']['baseline']['tokens_per_accepted'])
        self.assertIsNone(result['token_reduction_fraction'])

    def test_single_arm_pilot_cannot_claim_savings(self):
        rows = [{'case': 'rounding', 'repeat': 0, 'arm': 'compact', 'outcome': 'passed',
                 'usage': {'total_tokens': 100, 'cached_input_tokens': 0}, 'seconds': 1}]
        self.assertIsNone(sessions.summarize(rows, 1)['token_reduction_fraction'])

    def test_strict_preservation_rejects_strengthened_existing_test(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'tests').mkdir()
            original = (sessions.FIXTURE / 'tests/test_parcel.py').read_text()
            target = root / 'tests/test_parcel.py'
            target.write_text(original)
            self.assertTrue(sessions.preserved_tests(root))
            # Frozen experiment semantics: even a monotonic test extension fails.
            # Keep this limitation visible; issue 28 tracks a future protocol.
            target.write_text(original.replace('[0, -1, True, 1.5]', '[0, -1, True, False, 1.5]'))
            self.assertFalse(sessions.preserved_tests(root))

    def test_export_checks_hashes_and_removes_private_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            schedule = [['rounding', 'baseline', 0]]
            sessions.write_json(root / 'manifest.json', {'opdev': 'PRIVATE_LOCAL_PATH', 'codex': 'PRIVATE_LOCAL_PATH',
                'schedule_sha256': sessions.sha(json.dumps(schedule).encode())})
            sessions.write_json(root / 'schedule.json', schedule)
            raw = root / 'rounding-baseline-0.events.jsonl'
            raw.write_text(events())
            usage = sessions.parse_session(events())['usage']
            row = {'case': 'rounding', 'arm': 'baseline', 'repeat': 0, 'outcome': 'passed',
                   'seconds': 1, 'usage': usage, 'events_sha256': reporter.digest(raw),
                   'assessment': {'checks': {'behavior': True}, 'test_stderr': 'PRIVATE_LOCAL_PATH'}}
            records = root / 'records.jsonl'
            records.write_text(json.dumps(row) + '\n')
            result = reporter.report(root)
            self.assertNotIn('PRIVATE_LOCAL_PATH', json.dumps(result))
            self.assertIsNone(result[0]['token_reduction_fraction'])
            raw.write_text(events() + '\n')
            with self.assertRaises(ValueError):
                reporter.report(root)

    def test_baseline_defect_is_caught_by_external_oracle(self):
        run = sessions.command([sys.executable, '-c', sessions.acceptance('rounding')], cwd=sessions.FIXTURE, check=False)
        self.assertNotEqual(run.returncode, 0)

    def test_export_rejects_changed_schedule_and_usage(self):
        for fault in ('schedule', 'counter', 'optional_counter', 'missing_hash'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                schedule = [['rounding', 'baseline', 0]]
                sessions.write_json(root / 'manifest.json', {
                    'schedule_sha256': sessions.sha(json.dumps(schedule).encode())})
                sessions.write_json(root / 'schedule.json', schedule)
                raw = root / 'rounding-baseline-0.events.jsonl'
                raw.write_text(events())
                row = {'case': 'rounding', 'arm': 'baseline', 'repeat': 0,
                       'outcome': 'passed', 'seconds': 1,
                       'usage': sessions.parse_session(events())['usage'],
                       'events_sha256': reporter.digest(raw)}
                if fault == 'schedule':
                    sessions.write_json(root / 'schedule.json', [['express', 'baseline', 0]])
                elif fault == 'counter':
                    row['usage']['input_tokens'] += 1
                elif fault == 'optional_counter':
                    row['usage']['reasoning_output_tokens'] = 1
                else:
                    del row['events_sha256']
                (root / 'records.jsonl').write_text(json.dumps(row) + '\n')
                with self.assertRaises(ValueError):
                    reporter.report(root)

    def test_baseline_missing_feature_is_caught(self):
        run = sessions.command([sys.executable, '-c', sessions.acceptance('express')], cwd=sessions.FIXTURE, check=False)
        self.assertNotEqual(run.returncode, 0)

    def test_baseline_documentation_is_caught(self):
        run = sessions.command([sys.executable, '-c', sessions.acceptance('stale')], cwd=sessions.FIXTURE, check=False)
        self.assertNotEqual(run.returncode, 0)

    def test_staging_controller_rejects_policy_edits(self):
        self.stage_case('AGENTS.md', 'error')

    def test_staging_controller_stages_allowed_change(self):
        self.stage_case('README.md', 'staged')

    def stage_case(self, filename, expected):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sessions.git(root, 'init', '-q')
            sessions.git(root, 'config', 'user.name', 'Test')
            sessions.git(root, 'config', 'user.email', 'test@example.invalid')
            (root / '.gitignore').write_text('.benchmark/\n')
            (root / 'README.md').write_text('old')
            (root / 'AGENTS.md').write_text('policy')
            sessions.git(root, 'add', '.')
            sessions.git(root, 'commit', '-qm', 'fixture')
            (root / '.benchmark').mkdir()
            (root / filename).write_text('changed')
            (root / '.benchmark/stage.request').write_text('request-1')
            stop, actions = threading.Event(), []
            thread = threading.Thread(target=sessions.serve_staging, args=(root, 'stale', stop, actions))
            thread.start()
            try:
                deadline = time.monotonic() + 5
                while not actions and time.monotonic() < deadline:
                    time.sleep(.05)
            finally:
                stop.set()
                thread.join()
            self.assertEqual(actions[0]['status'], expected)
            staged = sessions.git(root, 'diff', '--cached', '--name-only')
            self.assertEqual(staged, filename if expected == 'staged' else '')


if __name__ == '__main__':
    unittest.main()
