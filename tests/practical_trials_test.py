"""Offline probe accounting tests; never contact a model."""

import sys

sys.dont_write_bytecode = True
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import practical_local_trials as trials


class ProbeTests(unittest.TestCase):
    def invoke(self, root, responder):
        baseline = root / "old.md"
        candidate = root / "new.md"
        baseline.write_text("old guidance", encoding="utf-8")
        candidate.write_text("new guidance", encoding="utf-8")
        output = root / "results"
        with (
            patch.object(
                sys,
                "argv",
                [
                    "probe",
                    "--output",
                    str(output),
                    "--baseline",
                    str(baseline),
                    "--candidate",
                    str(candidate),
                ],
            ),
            patch.dict(
                "os.environ",
                {
                    "OPDEV_TRIAL_KEY": "test-private-key",
                    "OPDEV_TRIAL_ENDPOINT": "https://local.invalid/v1",
                },
            ),
            patch.object(trials.urllib.request, "urlopen", side_effect=responder),
            patch("sys.stdout", new_callable=io.StringIO) as stdout,
        ):
            result = trials.main()
        return result, output, stdout.getvalue()

    def test_frozen_inputs_account_for_every_arm_and_case_without_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            calls = []

            def respond(request, **kwargs):
                self.assertTrue((root / "results/protocol.json").exists())
                self.assertEqual(kwargs["timeout"], 900)
                body = json.loads(request.data)
                prompt = body["messages"][1]["content"]
                expected = next(
                    expected for _, text, expected in trials.CASES if prompt.startswith(text)
                )
                calls.append(body)
                return io.BytesIO(
                    json.dumps(
                        {
                            "choices": [
                                {
                                    "finish_reason": "stop",
                                    "message": {"content": json.dumps(expected)},
                                }
                            ],
                            "usage": {"total_tokens": 42},
                        }
                    ).encode()
                )

            result, output, stdout = self.invoke(root, respond)
            self.assertEqual(result, 0)
            self.assertEqual(len(calls), 9)
            records = json.loads((output / "summary.json").read_text())
            self.assertTrue(all(r["passed"] for r in records))
            self.assertEqual(len({(r["arm"], r["case"]) for r in records}), 9)
            self.assertNotIn("test-private-key", stdout)
            self.assertNotIn("test-private-key", (output / "protocol.json").read_text())
            with self.assertRaises(FileExistsError):
                trials.write_new(output / "summary.json", [])

    def test_infrastructure_error_is_retained_without_retry_or_secret_output(self):
        with tempfile.TemporaryDirectory() as directory:
            count = 0

            def fail(*args, **kwargs):
                nonlocal count
                count += 1
                raise TimeoutError("test-private-key")

            result, output, stdout = self.invoke(Path(directory), fail)
            self.assertEqual(result, 2)
            self.assertEqual(count, 1)
            self.assertNotIn("test-private-key", stdout)
            records = json.loads((output / "summary.json").read_text())
            self.assertEqual(records[0]["outcome"], "infrastructure_error")
            self.assertEqual(records[0]["error_type"], "TimeoutError")


if __name__ == "__main__":
    unittest.main()
