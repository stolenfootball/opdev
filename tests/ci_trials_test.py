"""Offline harness regressions. Never launches a model or provider operation."""

import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import opdev_ci_trials as trials


class TrialHarnessTests(unittest.TestCase):
    def test_fresh_and_resume_preserve_permissions_and_non_git_support(self):
        for thread in [None, "thread-id"]:
            argv = trials.trial_command("codex", "gpt-5.6-sol", "/scratch", thread)
            self.assertIn("--skip-git-repo-check", argv)
            self.assertIn('sandbox_mode="workspace-write"', argv)
            self.assertIn('approval_policy="never"', argv)
            self.assertNotIn("--dangerously-bypass-approvals-and-sandbox", argv)
            self.assertEqual(argv[-1], "-")
        self.assertEqual(
            trials.trial_command("codex", "model", "/scratch", "id")[1:3], ["exec", "resume"]
        )

    def test_skill_exclusions_are_identical_on_resume(self):
        paths = ["C:/test/skills/opdev/SKILL.md", "C:/test/skills/setup/SKILL.md"]
        expected = 'skills.config=[{path="C:/test/skills/opdev/SKILL.md",enabled=false},{path="C:/test/skills/setup/SKILL.md",enabled=false}]'
        for thread in [None, "thread-id"]:
            self.assertIn(
                expected, trials.trial_command("codex", "model", "/scratch", thread, paths)
            )

    def test_read_only_or_contaminated_context_stops_before_inference(self):
        for context in [
            "`sandbox_mode` is `read-only`",
            "`sandbox_mode` is `workspace-write` plugins/cache/personal/opdev",
        ]:
            with self.subTest(context=context), tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)
                (output / "baseline.md").write_text("fixture guidance")
                (output / "protocol.json").write_text("{}")
                protocol = {
                    "cases": {"inefficient": "fixture"},
                    "guidance": {"baseline": trials.sha(b"fixture guidance")},
                }
                args = SimpleNamespace(output=output, state_dir=output, cli="codex")
                probe = {"stdout": context, "stderr": "", "exit": 0}
                with (
                    patch.object(trials, "run_process", return_value=probe) as run,
                    patch("builtins.print"),
                ):
                    self.assertFalse(trials.invoke(args, protocol, "baseline", "inefficient", 0))
                self.assertEqual(run.call_count, 1)
                record = json.loads((output / "baseline-inefficient-0/record.json").read_text())
                self.assertIn("blocked", record)
                self.assertNotIn("turn_0", record)

    def test_snapshot_detects_ci_mutation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "ci.yaml").write_text("required: true")
            before = trials.snapshot(root)
            (root / "ci.yaml").write_text("required: false")
            self.assertNotEqual(before["ci.yaml"], trials.snapshot(root)["ci.yaml"])


if __name__ == "__main__":
    unittest.main()
