#!/usr/bin/env python3
"""Offline regression coverage for exact-revision cross-provider qualification."""
import copy
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("await_arm64", ROOT / "scripts/await_arm64.py")
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)
REVISION = "a" * 40


def run():
    return {"id": 123, "run_attempt": 2, "head_sha": REVISION,
            "head_branch": f"opdev-arm64/{REVISION}", "event": "push",
            "path": gate.WORKFLOW, "head_repository": {"full_name": gate.REPOSITORY},
            "status": "completed", "conclusion": "success"}


def jobs():
    return {"jobs": [{"name": "installer-arm64", "status": "completed",
                      "conclusion": "success", "labels": ["ubuntu-24.04-arm"]}]}


class GateTests(unittest.TestCase):
    def test_requires_exact_source_repository_event_and_workflow(self):
        for field, value in [("head_sha", "b" * 40), ("head_branch", "main"),
                             ("event", "pull_request"), ("path", "other.yml"),
                             ("head_repository", {"full_name": "other/repo"})]:
            candidate = run()
            candidate[field] = value
            self.assertIsNone(gate.select_run({"workflow_runs": [candidate]}, REVISION))

    def test_latest_attempt_wins(self):
        earlier = run()
        earlier["run_attempt"] = 1
        self.assertEqual(gate.select_run({"workflow_runs": [run(), earlier]}, REVISION)["run_attempt"], 2)

    def test_wrong_architecture_or_skipped_job_never_passes(self):
        self.assertTrue(gate.qualified_jobs(jobs()))
        for field, value in [("labels", ["ubuntu-24.04"]), ("conclusion", "skipped"),
                             ("status", "in_progress"), ("name", "other")]:
            payload = jobs()
            payload["jobs"][0][field] = value
            self.assertFalse(gate.qualified_jobs(payload))
        payload = jobs()
        payload["jobs"].append(copy.deepcopy(payload["jobs"][0]))
        self.assertFalse(gate.qualified_jobs(payload))

    def test_success_uses_matching_attempt(self):
        urls = []
        def fetch(url):
            urls.append(url)
            return jobs() if "/jobs?" in url else {"workflow_runs": [run()]}
        gate.await_result(REVISION, fetch=fetch)
        self.assertIn("/attempts/2/jobs?", urls[-1])

    def test_failed_run_is_not_retried_until_green(self):
        failed = run()
        failed["conclusion"] = "failure"
        with self.assertRaises(RuntimeError):
            gate.await_result(REVISION, fetch=lambda _: {"workflow_runs": [failed]})

    def test_missing_result_times_out_and_transport_errors_propagate(self):
        ticks = [0]
        def sleep(seconds):
            ticks[0] += seconds
        with self.assertRaises(TimeoutError):
            gate.await_result(REVISION, timeout=3, interval=1, clock=lambda: ticks[0],
                              sleep=sleep, fetch=lambda _: {"workflow_runs": []})
        def unavailable(_):
            raise OSError("API unavailable")
        with self.assertRaises(OSError):
            gate.await_result(REVISION, fetch=unavailable)

    def test_invalid_revision_is_rejected_before_network(self):
        with self.assertRaises(ValueError):
            gate.await_result("main", fetch=lambda _: self.fail("network requested"))

    def test_protected_mirroring_is_single_ref_and_keeps_token_out_of_arguments(self):
        with patch.object(gate.subprocess, "check_output", return_value=REVISION), \
             patch.object(gate.subprocess, "run") as push:
            gate.mirror_revision(REVISION, "test-secret")
            args, kwargs = push.call_args
            self.assertEqual(args[0][-1], f"{REVISION}:refs/heads/opdev-arm64/{REVISION}")
            self.assertNotIn("test-secret", str(args))
            self.assertNotIn("--force", args[0])
            self.assertIn("GIT_CONFIG_VALUE_0", kwargs["env"])


if __name__ == "__main__":
    unittest.main()
