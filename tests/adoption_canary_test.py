"""Offline safety checks for the private adoption-session controller."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/opdev_adoption_canary.py"
spec = importlib.util.spec_from_file_location("adoption_canary", SCRIPT)
canary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(canary)


class CanarySafety(unittest.TestCase):
    def test_existing_fixture_mismatch_stops_before_push_or_candidate_copy(self):
        from argparse import Namespace
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "new-candidate"
            with patch.object(canary, "privacy", return_value={"ssh_url_to_repo": "fixture"}), \
                    patch.object(canary, "run", side_effect=["", "unexpected-sha"]) as run:
                with self.assertRaisesRegex(ValueError, "revision changed"):
                    canary.prepare(Namespace(project="group/private", output=output,
                                             existing_seed="expected-sha"))
                self.assertEqual(run.call_count, 2)
                self.assertFalse((output / "plugin").exists())
                self.assertFalse(any("push" in call.args[0] for call in run.call_args_list))

    def test_claude_permissions_are_candidate_scoped_and_read_only(self):
        settings, commands = canary.claude_assessment_permissions(Path("C:/canary"))
        permissions = settings["permissions"]
        self.assertIn("Read(//c/canary/plugin/**)", permissions["allow"])
        self.assertIn("Edit(//c/canary/runtime/**)", permissions["deny"])
        self.assertNotIn("PowerShell", permissions["allow"])
        self.assertNotIn("Bash", permissions["allow"])
        for command in commands:
            self.assertNotIn("*", command)
            self.assertNotIn(" approve", command)
            self.assertNotIn(" install", command)
        self.assertTrue(any(command.endswith("init --dry-run") for command in commands))

    def test_public_and_internal_projects_rejected(self):
        for visibility in ("public", "internal"):
            with patch.object(canary, "run", return_value=json.dumps({"visibility": visibility})):
                with self.assertRaises(ValueError):
                    canary.privacy("group/project")

    def test_private_project_checked_by_encoded_path(self):
        with patch.object(canary, "run", return_value='{"visibility":"private","id":123}') as run:
            self.assertEqual(canary.privacy("group/project")["id"], 123)
            run.assert_called_once_with(["glab", "api", "projects/group%2Fproject"])

    def test_required_mode_arguments_and_path_labels_rejected(self):
        for argv in (["prepare"], ["session"],
                     ["session", "--host", "codex", "--executable", "unused",
                      "--prompt", "test", "--label", "../escape"]):
            result = subprocess.run([sys.executable, SCRIPT, *argv, "--output", "."],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertNotIn("Traceback", result.stderr)

    def test_failed_trial_cannot_be_overwritten(self):
        from argparse import Namespace
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            canary.save(root / "manifest.json", {"project": "group/private"})
            events = root / "codex-first.events.jsonl"
            events.write_text("failed trial", encoding="utf-8")
            with patch.object(canary, "privacy"), patch.object(canary, "run") as run:
                with self.assertRaises(ValueError):
                    canary.session(Namespace(output=root, host="codex", label="first"))
                run.assert_not_called()
            self.assertEqual(events.read_text(), "failed trial")

    def test_fixture_behavior(self):
        result = subprocess.run([sys.executable, "-m", "unittest", "discover", "-s", "tests"],
                                cwd=ROOT / "benchmarks/adoption/fixture", capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
