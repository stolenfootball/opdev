"""Optional project-owned extension; Python 3 standard library, not OpDev core.

Run through OpDev for the outer process-tree deadline. No tool installation,
report import, in-place mutations, skipped baseline, or hidden retry is allowed.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

VERSION = "27.1.0"
MAX_JSON = 8 * 1024 * 1024


def read_json(path):
    with path.open("rb") as stream:
        data = stream.read(MAX_JSON + 1)
    if len(data) > MAX_JSON:
        raise ValueError("producer report exceeds 8 MiB")
    return json.loads(data)


def classify(report, exit_code, selected):
    """Interpret only completed supported reports, not a successful exit alone."""
    if exit_code not in (0, 2, 3, 4):
        raise ValueError("producer did not complete a supported evaluation")
    if not isinstance(report, dict) or report.get("cargo_mutants_version") != VERSION:
        raise ValueError("unsupported producer report version")
    outcomes = report.get("outcomes")
    if not isinstance(outcomes, list):
        raise ValueError("producer report has no outcome inventory")
    baseline = []
    mutations = []
    for item in outcomes:
        if not isinstance(item, dict):
            raise ValueError("invalid scenario record")
        scenario = item.get("scenario")
        if scenario == "Baseline":
            baseline.append(item)
        elif isinstance(scenario, dict) and set(scenario) == {"Mutant"}:
            if not isinstance(scenario["Mutant"], dict):
                raise ValueError("invalid mutation identity")
            mutations.append(item)
        else:
            raise ValueError("unsupported scenario kind")
    if len(baseline) > 1:
        raise ValueError("duplicate baseline")
    if not report.get("end_time") or not baseline:
        return "unverified", "Mutation analysis is incomplete; no test-strength pass."
    if baseline[0].get("summary") not in ("Success", "Failure", "Timeout"):
        raise ValueError("unsupported baseline outcome")
    if baseline[0]["summary"] != "Success":
        if exit_code != 4:
            raise ValueError("baseline outcome contradicts producer exit")
        return "unverified", "Unmodified baseline did not pass; test strength is unverified."
    for item in [baseline[0], *mutations]:
        if item.get("summary") not in ("Success", "CaughtMutant", "MissedMutant"):
            continue
        phases = item.get("phase_results")
        if not isinstance(phases, list) or not phases or not isinstance(phases[-1], dict):
            return "unverified", "A successful or caught scenario lacks actual test-phase evidence."
        last = phases[-1]
        status = last.get("process_status")
        caught = item["summary"] == "CaughtMutant"
        valid = (isinstance(status, dict) and set(status) == {"Failure"}
                 and type(status["Failure"]) is int and status["Failure"] > 0) if caught else status == "Success"
        if last.get("phase") != "Test" or not valid:
            return "unverified", "A successful or caught scenario lacks actual test-phase evidence."
    counts = Counter(item.get("summary") for item in mutations)
    keys = {"caught": "CaughtMutant", "missed": "MissedMutant",
            "timeout": "Timeout", "unviable": "Unviable", "success": "Success"}
    if set(counts) - set(keys.values()):
        raise ValueError("unsupported mutation outcome")
    for key, value in keys.items():
        if type(report.get(key)) is not int or report[key] != counts[value]:
            raise ValueError("producer summary contradicts scenario inventory")
    if type(report.get("total_mutants")) is not int or report["total_mutants"] != len(mutations):
        raise ValueError("invalid total mutation count")
    expected_exit = 3 if counts["Timeout"] else 2 if counts["MissedMutant"] else 0
    if exit_code != expected_exit:
        raise ValueError("producer exit contradicts its report")
    actual = [item["scenario"]["Mutant"] for item in mutations]
    # --list includes an explanatory diff not repeated in outcome identities.
    canonical = lambda item: json.dumps({k: v for k, v in item.items() if k != "diff"},
                                       sort_keys=True, separators=(",", ":"))
    if Counter(map(canonical, actual)) != Counter(map(canonical, selected)):
        return "unverified", "Selected mutation inventory was not fully evaluated."
    if counts["Timeout"] or counts["Success"]:
        return "unverified", "Some mutations lack a conclusive test verdict; inspect the report."
    if not counts["CaughtMutant"] + counts["MissedMutant"]:
        return "unverified", "No viable mutations were tested; no test-strength pass."
    # This example's selected policy is no missed viable mutations in its narrow scope.
    # A finding describes the tests, not proof of a defect in the original software.
    if counts["MissedMutant"]:
        return "failed", "Selected test-strength policy not met: viable mutations escaped the tests."
    return "passed", "Tests caught all evaluated viable mutations in the selected scope."


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, timeout=15)
    if result.returncode:
        raise ValueError("Git could not establish source identity")
    return result.stdout.decode("utf-8").strip()


def source_identity(root):
    if not Path(git(root, "rev-parse", "--show-toplevel")).samefile(root):
        raise ValueError("request root must be the Git repository root")
    if git(root, "status", "--porcelain", "--untracked-files=all"):
        return None
    return git(root, "rev-parse", "HEAD")


def run_producer(argv, root, directory, label):
    # Files preserve diagnostics without accumulating producer output in memory.
    # The enclosing OpDev invocation provides the total deadline/tree containment.
    with (directory / (label + ".stdout")).open("xb") as stdout, \
            (directory / (label + ".stderr")).open("xb") as stderr:
        return subprocess.run(argv, cwd=root, stdout=stdout, stderr=stderr,
                              stdin=subprocess.DEVNULL).returncode


def evaluate(args, request, evidence):
    if not isinstance(request, dict) or request.get("protocol_version") != "1.0.0":
        raise ValueError("unsupported extension request")
    if request.get("stage") not in ("verify", "pre_merge") or not request.get("check_id"):
        raise ValueError("example supports verify and pre_merge checks only")
    root = Path(request["project_root"]).resolve(strict=True)
    revision = source_identity(root)
    if revision is None:
        return "unverified", "Commit reviewed inputs first; this example requires clean source."
    base = root / "target" / "opdev-test-strength"
    ignored = subprocess.run(["git", "-C", str(root), "check-ignore", "-q", "--", str(base)],
                             capture_output=True, timeout=15)
    if ignored.returncode != 0:
        raise ValueError("target/opdev-test-strength must be Git-ignored before running")
    base.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=base))
    evidence.append({"kind": "run", "summary": "Fresh producer logs and reports; review before sharing.",
                     "location": directory.relative_to(root).as_posix()})
    producer = [args.producer, "mutants"]
    if run_producer(producer + ["--version"], root, directory, "version") != 0:
        raise ValueError("producer version lookup failed")
    with (directory / "version.stdout").open("rb") as stream:
        version = stream.read(256).decode("utf-8").strip()
    if version != "cargo-mutants " + VERSION:
        raise ValueError("example requires cargo-mutants " + VERSION)
    common = producer + ["--no-config", "--file", args.file, "--re", args.mutant,
                         "--colors", "never", "--no-shuffle"]
    if run_producer(common + ["--list", "--json"], root, directory, "selection") != 0:
        raise ValueError("producer selection failed")
    selected = read_json(directory / "selection.stdout")
    if not isinstance(selected, list) or any(not isinstance(item, dict) for item in selected):
        raise ValueError("unsupported selection report")
    if not 0 < len(selected) <= args.max_mutants:
        return "unverified", "Selection is empty or exceeds the explicit mutation budget."
    command = common + ["--baseline", "run", "--jobs", "1", "--jobserver-tasks", "1",
                        "--test-tool", "cargo", "--timeout", str(args.test_timeout),
                        "--build-timeout", str(args.build_timeout), "--output", str(directory),
                        "--cargo-arg=--locked", "--cargo-arg=--offline"]
    evidence.append({"kind": "invocation", "summary": json.dumps(command)})
    evidence.append({"kind": "source", "summary": "Clean committed source before execution.",
                     "location": revision})
    code = run_producer(command, root, directory, "execution")
    if source_identity(root) != revision:
        return "unverified", "Source changed during analysis; results cannot qualify this source."
    report_path = directory / "mutants.out" / "outcomes.json"
    report = read_json(report_path)
    evidence.append({"kind": "producer", "summary": "cargo-mutants " + VERSION,
                     "location": report_path.relative_to(root).as_posix()})
    evidence.append({"kind": "report_digest", "summary": "parsed_json_sha256=" + hashlib.sha256(
        json.dumps(report, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()})
    return classify(report, code, selected)


def positive(value):
    number = int(value)
    if number <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--producer", default="cargo-mutants", help="native executable, not a shell command")
    parser.add_argument("--file", required=True, help="reviewed source-file filter")
    parser.add_argument("--mutant", required=True, help="reviewed mutation-name regex")
    parser.add_argument("--max-mutants", required=True, type=positive)
    parser.add_argument("--test-timeout", required=True, type=positive)
    parser.add_argument("--build-timeout", required=True, type=positive)
    args = parser.parse_args()
    evidence = []
    started = time.monotonic()
    try:
        raw = sys.stdin.buffer.read(65537)
        if len(raw) > 65536:
            raise ValueError("extension request too large")
        outcome, summary = evaluate(args, json.loads(raw), evidence)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        outcome, summary = "error", "Test-strength evaluation could not complete."
        # Avoid echoing arbitrary producer output/JSON through exception strings.
        diagnostic = str(error) if type(error) is ValueError else type(error).__name__
        evidence.append({"kind": "diagnostic", "summary": diagnostic + "; inspect retained logs and supported inputs."})
    evidence.append({"kind": "duration", "summary": f"elapsed_seconds={time.monotonic() - started:.3f}"})
    print(json.dumps({"protocol_version": "1.0.0", "outcome": outcome,
                      "summary": summary, "evidence": evidence}))


if __name__ == "__main__":
    main()
