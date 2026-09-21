"""Offline contract fixtures for the optional example; no producer installation."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "examples/test-strength/cargo_mutants.py"
spec = importlib.util.spec_from_file_location("adapter", SCRIPT)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
MUTANT = {"file": "src/lib.rs", "replacement": "<", "span": {"start": 4}}


def report(summary="CaughtMutant"):
    return {"cargo_mutants_version": adapter.VERSION, "end_time": "2026-09-21T00:00:00Z",
            "total_mutants": 1, "caught": int(summary == "CaughtMutant"),
            "missed": int(summary == "MissedMutant"), "timeout": int(summary == "Timeout"),
            "success": int(summary == "Success"), "unviable": int(summary == "Unviable"),
            "outcomes": [{"scenario": "Baseline", "summary": "Success",
                          "phase_results": [{"phase": "Test", "process_status": "Success"}]},
                         {"scenario": {"Mutant": MUTANT}, "summary": summary,
                          "phase_results": [{"phase": "Test", "process_status":
                              {"Failure": 101} if summary == "CaughtMutant" else "Success"}]}]}


class VerdictTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "nt", "Windows verbatim request roots")
    def test_verbatim_root_identifies_the_same_repository(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            verbatim = Path("\\\\?\\" + str(root))
            with patch.object(adapter, "git", side_effect=[str(root), "", "revision"]):
                self.assertEqual(adapter.source_identity(verbatim), "revision")

    def test_completed_selected_findings_and_incomplete_results(self):
        for summary, code, expected in [("CaughtMutant", 0, "passed"),
                                        ("MissedMutant", 2, "failed"),
                                        ("Timeout", 3, "unverified"),
                                        ("Unviable", 0, "unverified"),
                                        ("Success", 0, "unverified")]:
            with self.subTest(summary=summary):
                self.assertEqual(adapter.classify(report(summary), code, [MUTANT])[0], expected)

    def test_empty_partial_and_changed_selection_are_not_passes(self):
        empty = report()
        empty.update(total_mutants=0, caught=0, outcomes=empty["outcomes"][:1])
        for data, selected in [(empty, []), (empty, [MUTANT]),
                               (dict(report(), end_time=None), [MUTANT]),
                               (dict(report(), outcomes=report()["outcomes"][1:]), [MUTANT]),
                               (report(), [dict(MUTANT, replacement=">=")])]:
            with self.subTest(data=data):
                self.assertEqual(adapter.classify(data, 0, selected)[0], "unverified")
        self.assertEqual(adapter.classify(report(), 0, [dict(MUTANT, diff="explanatory diff")])[0], "passed")

    def test_baseline_failure_is_not_mutation_evidence(self):
        for summary in ("Failure", "Timeout"):
            data = report()
            data["outcomes"][0]["summary"] = summary
            self.assertEqual(adapter.classify(data, 4, [MUTANT])[0], "unverified")

    def test_build_only_and_missing_test_phases_do_not_establish_strength(self):
        for phases in ([], [{"phase": "Build", "process_status": "Success"}],
                       [{"phase": "Test", "process_status": "Success"}]):
            data = report()
            data["outcomes"][1]["phase_results"] = phases
            self.assertEqual(adapter.classify(data, 0, [MUTANT])[0], "unverified")

    def test_unsupported_or_contradictory_reports_fail_closed(self):
        variants = [dict(report(), cargo_mutants_version="999.0.0"),
                    dict(report(), caught=True), dict(report(), total_mutants=0),
                    dict(report(), missed=1), dict(report(), outcomes={}),
                    dict(report(), outcomes=[{"scenario": "Unknown"}]),
                    dict(report(), outcomes=[{"scenario": {"Mutant": None}}])]
        unknown = report()
        unknown["outcomes"][1]["summary"] = "NewOutcome"
        duplicate = report()
        duplicate["outcomes"].append(copy.deepcopy(duplicate["outcomes"][0]))
        for data, code in [(v, 0) for v in variants + [unknown, duplicate]] + [
                (report(), 70), (report(), 2), (report("MissedMutant"), 0)]:
            with self.subTest(data=data, code=code), self.assertRaises(ValueError):
                adapter.classify(data, code, [MUTANT])

    def test_read_limit_and_invalid_json(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "report.json"
            path.write_bytes(b" " * (adapter.MAX_JSON + 1))
            with self.assertRaises(ValueError):
                adapter.read_json(path)
            path.write_text("not JSON")
            with self.assertRaises(ValueError):
                adapter.read_json(path)

    def test_fresh_directories_and_source_change(self):
        # Fake producer at the invocation boundary, not a saved report importer.
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            request = {"protocol_version": "1.0.0", "check_id": "strength",
                       "stage": "verify", "project_root": str(root)}
            args = type("Args", (), dict(producer="selected-tool", file="src/lib.rs",
                        mutant="boundary", max_mutants=1, test_timeout=3, build_timeout=9))()
            calls = []

            def producer(argv, cwd, directory, label):
                calls.append((argv, directory, label))
                if label == "version":
                    (directory / "version.stdout").write_text("cargo-mutants " + adapter.VERSION)
                if label == "selection":
                    (directory / "selection.stdout").write_text(json.dumps([MUTANT]))
                if label == "execution":
                    (directory / "mutants.out").mkdir()
                    (directory / "mutants.out/outcomes.json").write_text(json.dumps(report()))
                return 0

            with patch.object(adapter, "source_identity", return_value="abc"), \
                    patch.object(adapter.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)), \
                    patch.object(adapter, "run_producer", side_effect=producer):
                directories = []
                for _ in range(2):
                    evidence = []
                    self.assertEqual(adapter.evaluate(args, request, evidence)[0], "passed")
                    directories.append(evidence[0]["location"])
                    self.assertTrue(any(e["kind"] == "source" for e in evidence))
                    self.assertTrue(any(e["kind"] == "report_digest" for e in evidence))
                self.assertNotEqual(*directories)
                execution = [argv for argv, _, label in calls if label == "execution"][0]
                self.assertIn("--no-config", execution)
                self.assertIn("--cargo-arg=--offline", execution)
                self.assertEqual(execution[execution.index("--jobs") + 1], "1")
                self.assertNotIn("--in-place", execution)
                self.assertNotIn("--iterate", execution)
                with patch.object(adapter, "source_identity", side_effect=["abc", "changed"]):
                    self.assertEqual(adapter.evaluate(args, request, [])[0], "unverified")
                calls.clear()
                with patch.object(adapter, "read_json", return_value=[MUTANT, MUTANT]):
                    self.assertEqual(adapter.evaluate(args, request, [])[0], "unverified")
                    self.assertFalse(any(label == "execution" for _, _, label in calls))
                # A previous passing file must not rescue a new run without a report.
                with patch.object(adapter, "run_producer", side_effect=lambda argv, cwd, directory, label:
                                  0 if label == "execution" else producer(argv, cwd, directory, label)):
                    with self.assertRaises(FileNotFoundError):
                        adapter.evaluate(args, request, [])

    def test_invalid_request_emits_protocol_error_not_process_failure(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "--file", "src/lib.rs",
                                 "--mutant", "boundary", "--max-mutants", "1",
                                 "--test-timeout", "3", "--build-timeout", "9"],
                                input="{}", text=True, capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0)
        value = json.loads(result.stdout)
        self.assertEqual(value["outcome"], "error")
        self.assertTrue(value["summary"])
        self.assertTrue(value["evidence"])


if __name__ == "__main__":
    unittest.main()
