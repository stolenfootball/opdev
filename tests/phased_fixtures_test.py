"""Offline discrimination tests; no inference or provider writes."""
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from opdev_phased_fixtures import case, grade

NORMALIZE = '''def normalize(value):
    if value is None:
        raise TypeError("text required")
    return value.strip().lower()
'''
BACKUP = '''import json, hashlib
def encode(text):
    return json.dumps(dict(version=1, text=text, sha256=hashlib.sha256(text.encode()).hexdigest()))
def decode(encoded):
    try:
        item = json.loads(encoded)
        if item["version"] != 1 or hashlib.sha256(item["text"].encode()).hexdigest() != item["sha256"]:
            raise ValueError("bad envelope")
        return item["text"]
    except (TypeError, KeyError, AttributeError) as error:
        raise ValueError("bad envelope") from error
'''
READER = '''import json
def read(payload):
    try:
        item = json.loads(payload)
        if item["version"] != 2:
            raise ValueError("version")
        return item["body"]
    except (TypeError, KeyError) as error:
        raise ValueError("envelope") from error
'''


class OracleTests(unittest.TestCase):
    def fixture(self, family, repeat=0):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        files, prompts = case(family, repeat)
        for name, content in files.items():
            target = root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding='utf-8')
        before = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in files}
        return root, before

    def test_mvp_rejects_missing_behavior_and_corrupt_but_roundtripping_decoder(self):
        root, before = self.fixture('P1')
        self.assertNotEqual(grade(root, 'P1', 0, before)['behavior_exit'], 0)
        (root / 'app.py').write_text(BACKUP, encoding='utf-8')
        self.assertEqual(grade(root, 'P1', 0, before)['behavior_exit'], 0)
        (root / 'app.py').write_text(BACKUP + '\ndef decode(encoded):\n    return json.loads(encoded)["text"]\n', encoding='utf-8')
        self.assertNotEqual(grade(root, 'P1', 0, before)['behavior_exit'], 0)

    def test_composition_rejects_individually_passing_parts_and_writer_revert(self):
        root, before = self.fixture('P2')
        self.assertNotEqual(grade(root, 'P2', 0, before)['behavior_exit'], 0)
        (root / 'reader.py').write_text(READER, encoding='utf-8')
        self.assertEqual(grade(root, 'P2', 0, before)['behavior_exit'], 0)
        (root / 'writer.py').write_text('def write(text):\n    return text\n')
        self.assertFalse(grade(root, 'P2', 0, before)['protected_files_unchanged'])

    def test_mvp_rejects_helper_error_leaking_through_public_decoder(self):
        root, before = self.fixture('P1')
        leaky = BACKUP + '''
import hmac
def decode(encoded):
    try:
        item = json.loads(encoded)
        if item["version"] != 1:
            raise ValueError("version")
        text, digest = item["text"], item["sha256"]
        expected = hashlib.sha256(text.encode()).hexdigest()
    except (TypeError, KeyError, AttributeError) as error:
        raise ValueError("bad envelope") from error
    if not hmac.compare_digest(digest, expected):
        raise ValueError("digest mismatch")
    return text
'''
        (root / 'app.py').write_text(leaky, encoding='utf-8')
        self.assertNotEqual(grade(root, 'P1', 0, before)['behavior_exit'], 0)
        (root / 'app.py').write_text(BACKUP, encoding='utf-8')
        self.assertEqual(grade(root, 'P1', 0, before)['behavior_exit'], 0)

    def test_continuation_requires_real_fix_before_answer_and_preserves_pending_label(self):
        root, before = self.fixture('P3')
        self.assertNotEqual(grade(root, 'P3', 0, before)['behavior_exit'], 0)
        (root / 'app.py').write_text(NORMALIZE + '\ndef heading():\n    return "Result"\n')
        self.assertEqual(grade(root, 'P3', 0, before)['behavior_exit'], 0)
        self.assertNotEqual(grade(root, 'P3', 1, before)['behavior_exit'], 0)
        (root / 'app.py').write_text(NORMALIZE + '\ndef heading():\n    return "Ready"\n')
        self.assertEqual(grade(root, 'P3', 1, before)['behavior_exit'], 0)
        self.assertNotEqual(grade(root, 'P3', 0, before)['behavior_exit'], 0)

    def test_global_and_guided_stops_reject_any_new_work(self):
        for repeat in range(2):
            root, before = self.fixture('P4', repeat)
            self.assertTrue(grade(root, 'P4', 0, before)['protected_files_unchanged'])
            (root / 'unrequested.py').write_text('# new optional feature\n')
            self.assertFalse(grade(root, 'P4', 0, before)['protected_files_unchanged'])

    def test_replacing_request_preserves_exact_prior_state(self):
        root, before = self.fixture('P5')
        (root / 'app.py').write_text(NORMALIZE)
        first = grade(root, 'P5', 0, before)
        self.assertEqual(first['behavior_exit'], 0)
        self.assertTrue(grade(root, 'P5', 1, before, first['after'])['protected_files_unchanged'])
        (root / 'extra.md').write_text('new work\n')
        self.assertFalse(grade(root, 'P5', 1, before, first['after'])['protected_files_unchanged'])

    def test_routine_and_later_scope_controls(self):
        root, before = self.fixture('P6')
        self.assertNotEqual(grade(root, 'P6', 0, before)['behavior_exit'], 0)
        (root / 'app.py').write_text(NORMALIZE)
        self.assertEqual(grade(root, 'P6', 0, before)['behavior_exit'], 0)
        (root / 'later.py').write_text('def schedule(): pass\n')
        self.assertFalse(grade(root, 'P6', 0, before)['protected_files_unchanged'])


if __name__ == '__main__':
    unittest.main()
