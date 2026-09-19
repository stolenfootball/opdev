import copy
import unittest

from review_contract import (InvalidReview, packet, parse, render,
                             validate_assessments, validate_candidates)


class ReviewContractTests(unittest.TestCase):
    def setUp(self):
        self.evidence = packet({'id': 'test', 'request': 'Review W2', 'subject': 'W2',
            'documents': [{'id': 'R', 'path': 'accepted.md', 'revision': 'D1',
                           'content': 'Return one space.'},
                          {'id': 'S', 'path': 'source.txt', 'revision': 'W2',
                           'content': 'Returns no space.'}]})
        self.candidates = {'snapshot_id': self.evidence['snapshot_id'], 'limitations': [],
            'candidates': [{'id': 'C1', 'claim': 'W2 omits the required space.',
                'requirement': {'source': 'R', 'quote': 'Return one space.'},
                'evidence': [{'source': 'S', 'quote': 'Returns no space.'}],
                'basis': 'source', 'assumptions': [], 'question': 'Is the spacing consistent?'}]}
        self.assessments = {'snapshot_id': self.evidence['snapshot_id'], 'limitations': [],
            'assessments': [{'id': 'C1', 'disposition': 'supported',
                'rationale': 'Source contradicts accepted spacing.',
                'evidence': [{'source': 'R', 'quote': 'Return one space.'},
                             {'source': 'S', 'quote': 'Returns no space.'}],
                'final_claim': 'W2 source omits the required space.'}]}

    def test_valid_review_retains_claim_and_identity_without_rewriting(self):
        result = render(self.assessments, self.evidence, self.candidates)
        self.assertIn(self.evidence['snapshot_id'], result)
        self.assertIn(self.assessments['assessments'][0]['final_claim'], result)
        self.assertIn('source.txt @ W2', result)

    def test_rejects_wrong_snapshot_and_changed_evidence(self):
        bad = copy.deepcopy(self.evidence)
        for key, value in [('subject', 'W3'), ('request', 'Audit everything')]:
            bad = copy.deepcopy(self.evidence)
            bad[key] = value
            with self.assertRaises(InvalidReview):
                validate_candidates(self.candidates, bad)
        bad = copy.deepcopy(self.candidates)
        bad['snapshot_id'] = '0' * 64
        with self.assertRaises(InvalidReview):
            validate_candidates(bad, self.evidence)

    def test_quotes_and_sources_must_exist(self):
        for ref in [{'source': 'absent', 'quote': 'Return one space.'},
                    {'source': 'R', 'quote': 'Invented text'}, {'source': 'R', 'quote': ''}]:
            bad = copy.deepcopy(self.candidates)
            bad['candidates'][0]['requirement'] = ref
            with self.assertRaises(InvalidReview):
                validate_candidates(bad, self.evidence)

    def test_all_candidate_ids_assessed_once(self):
        for entries in [[], self.assessments['assessments'] * 2,
                        [{**self.assessments['assessments'][0], 'id': 'other'}]]:
            bad = {**self.assessments, 'assessments': entries}
            with self.assertRaises(InvalidReview):
                validate_assessments(bad, self.evidence, self.candidates)

    def test_dispositions_not_core_outcomes(self):
        bad = copy.deepcopy(self.assessments)
        bad['assessments'][0]['disposition'] = 'passed'
        with self.assertRaises(InvalidReview):
            validate_assessments(bad, self.evidence, self.candidates)

    def test_uncertain_does_not_become_a_supported_finding(self):
        bad = copy.deepcopy(self.assessments)
        item = bad['assessments'][0]
        item['disposition'] = 'uncertain'
        with self.assertRaises(InvalidReview):
            render(bad, self.evidence, self.candidates)
        item['final_claim'] = None
        result = render(bad, self.evidence, self.candidates)
        self.assertIn('No supported findings returned', result)
        self.assertNotIn('C1: W2 source omits', result)
        self.assertIn(item['rationale'], result)

    def test_empty_review_does_not_claim_complete_inspection(self):
        empty = {'snapshot_id': self.evidence['snapshot_id'], 'candidates': [],
                 'limitations': ['No implementation supplied.']}
        checked = {'snapshot_id': self.evidence['snapshot_id'], 'assessments': [], 'limitations': []}
        result = render(checked, self.evidence, empty)
        self.assertIn('No implementation supplied.', result)
        self.assertIn('does not establish complete inspection', result)

    def test_duplicate_json_unknown_fields_and_invalid_shapes_fail(self):
        with self.assertRaises(InvalidReview):
            parse('{"x":1,"x":2}')
        for raw in ['not json', 'x' * 128001]:
            with self.assertRaises(InvalidReview):
                parse(raw)
        bad = {**self.candidates, 'approved': True}
        with self.assertRaises(InvalidReview):
            validate_candidates(bad, self.evidence)
        bad = copy.deepcopy(self.candidates)
        bad['candidates'][0]['evidence'] = []
        with self.assertRaises(InvalidReview):
            validate_candidates(bad, self.evidence)

    def test_valid_references_cannot_prove_semantic_truth(self):
        wrong = copy.deepcopy(self.assessments)
        wrong['assessments'][0]['final_claim'] = 'All requirements are met.'
        # Intentionally accepted mechanically: semantics require independent review.
        validate_assessments(wrong, self.evidence, self.candidates)


if __name__ == '__main__':
    unittest.main()
