"""Offline tests for benchmark grading, accounting, pairing, and fixture oracles."""
import sys
sys.dont_write_bytecode = True
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('benchmark', ROOT / 'scripts/opdev_benchmark.py')
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


def events(answer, usage=None, extra=()):
    usage = {'input_tokens': 100, 'cached_input_tokens': 80, 'output_tokens': 20} if usage is None else usage
    return '\n'.join(json.dumps(e) for e in [
        {'type': 'item.completed', 'item': {'type': 'agent_message', 'text': json.dumps(answer)}},
        *extra, {'type': 'turn.completed', 'usage': usage}])


class BenchmarkTests(unittest.TestCase):
    def setUp(self):
        self.cases = bench.load_cases()
        self.case = self.cases[0]

    def test_oracles_are_derived_from_full_facts(self):
        for case in self.cases:
            full = case['full']
            if case['family'] == 'report':
                blockers = [{'id': r['id'], 'outcome': r['outcome']} for r in full['rules']
                            if r['outcome'] not in ('passed', 'not_applicable')]
                expected = {'verdict': 'blocked' if blockers else 'passed', 'blockers': blockers}
                self.assertEqual(case['selected']['blocking_rules'], blockers)
            elif case['family'] == 'evidence':
                facts = {a['id']: a['outcome'] for a in full['project']}
                for change in full['changes']:
                    if change['fingerprint'] == full['fingerprint']:
                        facts.update({a['id']: a['outcome'] for a in change['assertions']})
                expected = {key: facts.get(key, 'unverified') for key in ('POLICY', 'CHANGE')}
            else:
                roles = full['always'] + full['routes']['code_change']
                expected = {'paths': sorted({full['authorities'][r] for r in roles})}
            self.assertEqual(case['expected'], expected)

    def test_all_case_answers_require_exact_outcomes(self):
        for case in self.cases:
            self.assertEqual(bench.grade(case, events(case['expected']))['outcome'], 'passed')
            self.assertEqual(bench.grade(case, events({'verdict': 'passed'}))['outcome'], 'failed')

    def test_cache_is_subset_not_additional_usage(self):
        result = bench.grade(self.case, events(self.case['expected']))
        self.assertEqual(result['usage']['total_tokens'], 120)
        self.assertEqual(result['usage']['uncached_input_tokens'], 20)

    def test_missing_malformed_or_inconsistent_usage_never_becomes_zero(self):
        for usage in ({}, {'input_tokens': -1}, {'input_tokens': 10, 'cached_input_tokens': 11, 'output_tokens': 2},
                      {'input_tokens': True, 'cached_input_tokens': 0, 'output_tokens': 1}):
            result = bench.grade(self.case, events(self.case['expected'], usage))
            self.assertEqual(result['outcome'], 'unverified')
            self.assertIsNone(result['usage'])

    def test_optional_host_counters_are_preserved_without_double_counting(self):
        usage = {'input_tokens': 100, 'cached_input_tokens': 50, 'output_tokens': 20,
                 'cache_write_input_tokens': 10, 'reasoning_output_tokens': 5}
        result = bench.grade(self.case, events(self.case['expected'], usage))
        self.assertEqual(result['usage']['total_tokens'], 120)
        self.assertEqual(result['usage']['cache_write_input_tokens'], 10)
        self.assertEqual(result['usage']['reasoning_output_tokens'], 5)
        absent = bench.grade(self.case, events(self.case['expected']))
        self.assertIsNone(absent['usage']['cache_write_input_tokens'])
        usage['reasoning_output_tokens'] = -1
        self.assertIsNone(bench.grade(self.case, events(self.case['expected'], usage))['usage'])

    def test_tools_extra_turns_and_transport_errors_invalidate_trial(self):
        for extra in ([{'type': 'turn.failed'}], [{'type': 'turn.completed'}],
                      [{'type': 'item.started', 'item': {'type': 'command_execution'}}],
                      [{'type': 'item.completed', 'item': {'type': 'mcp_tool_call'}}]):
            self.assertEqual(bench.grade(self.case, events(self.case['expected'], extra=extra))['outcome'], 'error')
        self.assertEqual(bench.grade(self.case, events(self.case['expected']), 124)['outcome'], 'error')
        self.assertEqual(bench.grade(self.case, '{broken')['outcome'], 'error')

    def record(self, arm, repeat=0, outcome='passed', usage=True):
        r = bench.grade(self.case, events(self.case['expected']))
        r.update(experiment='same', case='one', repeat=repeat, arm=arm, seconds=1., outcome=outcome)
        if not usage:
            r['usage'] = None
        return r

    def test_failures_count_toward_tokens_per_accepted(self):
        rows = [self.record('full'), self.record('full', 1, 'failed')]
        group = bench.summarize(rows)['experiments'][0]
        self.assertEqual(group['arms']['full']['tokens_per_accepted'], 240)
        self.assertIsNone(group['token_reduction_fraction'])
        self.assertFalse(group['complete_pairs'])

    def test_unknown_usage_does_not_create_savings(self):
        rows = [self.record('full'), self.record('selected', usage=False)]
        group = bench.summarize(rows)['experiments'][0]
        self.assertIsNone(group['arms']['selected']['tokens_per_accepted'])
        self.assertIsNone(group['token_reduction_fraction'])

    def test_versions_and_unpaired_cases_cannot_be_compared(self):
        a, b = self.record('full'), self.record('selected')
        b['experiment'] = 'different'
        groups = bench.summarize([a, b])['experiments']
        self.assertEqual(len(groups), 2)
        self.assertTrue(all(g['token_reduction_fraction'] is None for g in groups))
        with self.assertRaises(ValueError):
            bench.summarize([a, a])

    def test_corrupt_records_cannot_report_savings(self):
        for field, value in [('total_tokens', -1), ('cached_input_tokens', 999), ('input_tokens', True)]:
            row = self.record('full')
            row['usage'][field] = value
            with self.assertRaises(ValueError):
                bench.summarize([row])
        row = self.record('full')
        row['seconds'] = float('nan')
        with self.assertRaises(ValueError):
            bench.summarize([row])

    def test_balanced_reproducible_schedule_and_payloads(self):
        jobs = bench.schedule(self.cases, 2, 17)
        self.assertEqual(jobs, bench.schedule(self.cases, 2, 17))
        self.assertEqual(len(jobs), 4 * len(self.cases))
        self.assertNotEqual(jobs, bench.schedule(self.cases, 2, 18))
        self.assertEqual(bench.payloads(self.cases), bench.payloads(self.cases))
        self.assertTrue(all(r['proxy_tokens'] is None for r in bench.payloads(self.cases)['measurements']))
        with self.assertRaises(ValueError):
            bench.schedule(self.cases, 0, 17)


if __name__ == '__main__':
    unittest.main()
