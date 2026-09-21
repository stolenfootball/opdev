"""Explicit live producer canary. Creates a new local-only fixture repository."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time


def execute(argv, root):
    result = subprocess.run(argv, cwd=root, capture_output=True, text=True, timeout=240)
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {argv}\n{result.stderr}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--opdev", type=Path, required=True)
    parser.add_argument("--producer", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="new directory; existing paths are refused")
    args = parser.parse_args()
    opdev, producer = str(args.opdev.resolve(strict=True)), str(args.producer.resolve(strict=True))
    root = args.output.resolve()
    if root.exists():
        parser.error("output already exists; use a fresh path")
    example = Path(__file__).resolve().parent
    shutil.copytree(example / "fixture", root)
    shutil.copyfile(example / "cargo_mutants.py", root / "strength.py")
    (root / ".opdev").mkdir()
    manifest = {
        "schema": 1, "project": {"kind": "library", "trunk": "main", "ci": {"provider": "unconfigured"}},
        "authorities": {},
        "commands": {
            "test": {"argv": ["cargo", "test", "--locked", "--offline"], "timeout_seconds": 60},
            "strength": {"argv": [sys.executable, "strength.py", "--producer", producer,
                                  "--file", "src/lib.rs", "--mutant", "replace < with <= in allowed",
                                  "--max-mutants", "1", "--test-timeout", "10", "--build-timeout", "30"],
                         "timeout_seconds": 120}},
        "quality": {"risks": ["functional"]},
        "testing": {"change_tests": "required", "escaped_defect_regressions": "required_or_justified",
                    "flake_policy": {"retries_visible": True, "quarantine_requires_owner_issue_expiry": True},
                    "coverage": {"mode": "unconfigured"},
                    "suites": [{"id": "tests", "command": "test", "stages": ["local"]}]},
        "delivery": {"status": "migration_required", "mode": "publish",
                     "artifact": {"kind": "fixture", "locator": "local-only"},
                     "environments": [], "recovery": {"strategy": "unconfigured"}},
        "assurance": {"profiles": [{"name": "opdev-core", "version": "1"}]},
        "context": {"always": [], "routes": {}},
        "extensions": {"checks": [{"id": "strength", "stage": "verify", "command": "strength", "blocking": True}]}}
    (root / ".opdev/project.yaml").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    for args_ in [["init", "--quiet", "-b", "main"], ["config", "user.name", "OpDev synthetic canary"],
                  ["config", "user.email", "canary@example.invalid"], ["add", "."],
                  ["commit", "--quiet", "-m", "fixture: initially weak boundary tests"]]:
        execute(["git", *args_], root)
    results = []
    for label, expected in [("weak", "failed"), ("strong", "passed")]:
        if label == "strong":
            path = root / "tests/boundary.rs"
            path.write_text(path.read_text().replace("// Deliberately weak: the canary adds assert!(!allowed(10)).",
                                                    "assert!(!allowed(10));"), encoding="utf-8")
            execute(["git", "add", "tests/boundary.rs"], root)
            execute(["git", "commit", "--quiet", "-m", "test: require exact capacity boundary"], root)
        start = time.monotonic()
        result = subprocess.run([opdev, "check", "--root", str(root), "--format", "json"],
                                cwd=root, capture_output=True, text=True, timeout=180)
        if result.returncode != 1:
            raise RuntimeError(f"expected blocked core gates, got {result.returncode}: {result.stderr}")
        report = json.loads(result.stdout)
        target = root / "target"
        target.mkdir(exist_ok=True)
        (target / (label + "-opdev.json")).write_text(result.stdout, encoding="utf-8")
        assert report["checks"][0]["outcome"] == "passed", report["checks"][0]
        check = report["checks"][1]
        assert check["outcome"] == expected, check
        gate = next(g for g in report["gates"] if g["gate"] == "development")
        assert gate["blocking_checks"] == (["strength"] if label == "weak" else []), gate
        assert gate["verdict"] == "blocked", "Extension must not qualify unrelated core rules"
        assert not execute(["git", "status", "--porcelain", "--untracked-files=all"], root).strip()
        results.append({"case": label, "suite": "passed", "extension": check,
                        "elapsed_seconds": round(time.monotonic() - start, 3)})
    (root / "target/canary-results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
