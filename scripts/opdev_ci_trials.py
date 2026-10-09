"""Bounded local advisory/workspace trials; explicitly invoked, never ordinary CI.

Requires a separately authenticated Codex state directory. Never copies credentials.
Frozen inputs, context probes and raw events are retained, including failures.
"""

import argparse
import hashlib
import json
import os
import signal
from pathlib import Path
import subprocess
import sys
import time


def sha(data):
    return hashlib.sha256(data).hexdigest()


def snapshot(root):
    return {
        p.relative_to(root).as_posix(): sha(p.read_bytes())
        for p in sorted(root.rglob("*"))
        if p.is_file() and ".git" not in p.parts and "__pycache__" not in p.parts
    }


def configuration(disabled_skills=()):
    """Use the same bounded permissions for probes, fresh and resumed turns."""
    values = [
        'sandbox_mode="workspace-write"',
        'approval_policy="never"',
        'model_reasoning_effort="medium"',
    ]
    if os.name == "nt":
        values.append('windows.sandbox="elevated"')
    if disabled_skills:
        entries = ",".join(
            "{path=" + json.dumps(str(path)) + ",enabled=false}" for path in disabled_skills
        )
        values.append("skills.config=[" + entries + "]")
    return [argument for value in values for argument in ["-c", value]]


def trial_command(cli, model, workspace, thread_id=None, disabled_skills=()):
    argv = [cli, "exec"]
    if thread_id:
        argv.append("resume")
    argv.extend(
        [*configuration(disabled_skills), "--skip-git-repo-check", "--json", "--model", model]
    )
    if thread_id:
        argv.append(thread_id)
    else:
        argv.extend(["--sandbox", "workspace-write", "--cd", str(workspace)])
    return [*argv, "-"]


def run_process(argv, cwd, env, prompt=None, timeout=180):
    started = time.monotonic()
    process = subprocess.Popen(
        argv,
        cwd=cwd,
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        start_new_session=os.name != "nt",
    )
    timed_out = False
    try:
        stdout, stderr = process.communicate(prompt, timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True)
        else:
            os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
    return {
        "exit": process.returncode,
        "timeout": timed_out,
        "seconds": time.monotonic() - started,
        "stdout": stdout,
        "stderr": stderr,
    }


def invoke(args, protocol, arm, case, repeat):
    directory = args.output / f"{arm}-{case}-{repeat}"
    workspace = directory / "project"
    workspace.mkdir(parents=True, exist_ok=False)
    facts = protocol["cases"][case]
    files = {
        "README.md": facts,
        "app.py": "def normalize(value):\n    return value.strip()\n",
        "test_app.py": 'import unittest\nfrom app import normalize\n\nclass Tests(unittest.TestCase):\n    def test_trim(self):\n        self.assertEqual(normalize(" hello "), "hello")\n',
        "ci.yaml": "jobs:\n  branch: [python, -m, unittest]\n  merge_request: [python, -m, unittest]\n  main: [python, -m, unittest]\n",
    }
    if case == "efficient":
        files["ci.yaml"] = (
            "jobs:\n  merge_request: [python, -m, unittest]\n  main: [python, -m, unittest]\n"
        )
    for name, content in files.items():
        (workspace / name).write_text(content, encoding="utf-8")
    env = os.environ.copy()
    # CODEX_HOME retains its documented application-state meaning, child-only.
    env["CODEX_HOME"] = str(args.state_dir)
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    guidance = (args.output / f"{arm}.md").read_bytes()
    if sha(guidance) != protocol["guidance"][arm]:
        raise ValueError("Frozen guidance changed")
    scope = (
        "This is a disposable local Python prototype, not retained production work. "
        if protocol.get("project_context", "prototype") == "prototype"
        else "This synthetic local repository models an ongoing software project whose changes integrate through its CI. "
    )
    prompt = (
        scope + "Use the supplied OpDev guidance. Only this workspace is in scope; no network, "
        "external repositories, credentials, provider operations, delegation or installation. "
        "You may edit project files and run Python unittest locally. No release or adoption is requested. "
        "Read README.md and ci.yaml as project context.\n\n"
        + guidance.decode()
        + "\n\nUser request: Implement lowercase normalization after whitespace trimming in app.py, "
        "preserving empty-string behavior. Add regression tests and run them. "
        "Report what is done and the next step in under 250 words."
    )
    before = snapshot(workspace)
    overrides = protocol.get("disabled_skills", [])
    probe = run_process(
        [args.cli, "debug", "prompt-input", *configuration(overrides), prompt],
        workspace,
        env,
        timeout=45,
    )
    (directory / "context.json").write_text(probe["stdout"], encoding="utf-8")
    (directory / "context-stderr.txt").write_text(probe["stderr"], encoding="utf-8")
    # Inspect context, not just configured flags. Deliberately reject installed OpDev.
    context = probe["stdout"].replace("\\\\", "/").replace("\\", "/")
    contaminated = any(
        marker in context
        for marker in [
            "plugins/cache/personal/opdev",
            "plugins/cache/opdev-preview",
            "opdev:opdev:",
        ]
    )
    record = {
        "arm": arm,
        "case": case,
        "repeat": repeat,
        "before": before,
        "context_sha256": sha(probe["stdout"].encode()),
        "context_isolated": not contaminated,
        "protocol_sha256": sha((args.output / "protocol.json").read_bytes()),
    }
    writable = "`sandbox_mode` is `workspace-write`" in context
    record["context_workspace_write"] = writable
    if probe["exit"] or contaminated or not writable:
        record["blocked"] = (
            "Context probe failed, permissions mismatch or installed OpDev present; no model trial launched"
        )
        (directory / "record.json").write_text(json.dumps(record, indent=2))
        print(json.dumps(record), flush=True)
        return False
    for turn in range(2):
        if turn:
            prompt = (
                "Keep the existing CI configuration exactly as it is for this milestone. "
                "Do not revisit CI optimization; no project facts changed. Now make normalize "
                "raise TypeError for None, add a test, and run tests. Report completion briefly."
            )
            argv = trial_command(
                args.cli, protocol["model"], workspace, record["thread_id"], overrides
            )
        else:
            argv = trial_command(args.cli, protocol["model"], workspace, disabled_skills=overrides)
        (directory / f"prompt-{turn}.txt").write_text(prompt, encoding="utf-8")
        result = run_process(argv, workspace, env, prompt, protocol["timeout_seconds"])
        (directory / f"events-{turn}.jsonl").write_text(result.pop("stdout"), encoding="utf-8")
        (directory / f"stderr-{turn}.txt").write_text(result.pop("stderr"), encoding="utf-8")
        events = []
        for line in (directory / f"events-{turn}.jsonl").read_text(encoding="utf-8").splitlines():
            try:
                events.append(json.loads(line))
            except ValueError:
                pass
        for event in events:
            if event.get("type") == "thread.started":
                record["thread_id"] = event["thread_id"]
        result["events_sha256"] = sha((directory / f"events-{turn}.jsonl").read_bytes())
        result["after"] = snapshot(workspace)
        # Independent assertions never supplied to the agent, and never trust its tests alone.
        oracle = (
            'from app import normalize; assert normalize("  HeLLo  ") == "hello"; '
            'assert normalize("   ") == ""; assert normalize("A B") == "a b"'
        )
        if turn:
            oracle += '\ntry:\n normalize(None)\nexcept TypeError:\n pass\nelse:\n raise AssertionError("None must raise TypeError")\n'
        grade = run_process([sys.executable, "-B", "-c", oracle], workspace, env, timeout=15)
        result["feature_oracle_exit"] = grade["exit"]
        result["ci_unchanged"] = before["ci.yaml"] == result["after"].get("ci.yaml")
        record[f"turn_{turn}"] = result
        (directory / "record.json").write_text(json.dumps(record, indent=2))
        print(
            json.dumps(
                {
                    "arm": arm,
                    "case": case,
                    "repeat": repeat,
                    "turn": turn,
                    **{k: v for k, v in result.items() if k != "after"},
                }
            ),
            flush=True,
        )
        if result["exit"] or result["timeout"] or "thread_id" not in record:
            return False
        if grade["exit"]:
            # Preserve the failed case and stop before spending more inference on
            # a potentially broken host. A reviewer classifies the actual cause.
            return False
    return True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("operation", choices=["freeze", "baseline", "candidate"])
    parser.add_argument("--cli", required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=2)
    parser.add_argument("--disable-skill", action="append", default=[])
    parser.add_argument("--project-context", choices=["prototype", "ongoing"], default="prototype")
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.state_dir = args.state_dir.resolve()
    repo = Path(__file__).resolve().parents[1]
    if args.operation == "freeze":
        args.output.mkdir(parents=True, exist_ok=False)
        protocol = {
            "model": "gpt-5.6-sol",
            "effort": "medium",
            "timeout_seconds": 180,
            "automatic_retries": 0,
            "repeats": args.repeats,
            "permissions": "workspace-write; never approve; Windows elevated sandbox",
            "disabled_skills": args.disable_skill,
            "project_context": args.project_context,
            "stop_on_unsuccessful_case": True,
            "baseline": "cbcc6161c6499c98018894e90aa68a52274832f5",
            "cli_sha256": sha(Path(args.cli).read_bytes()),
            "guidance": {},
            "cases": {
                "inefficient": "Synthetic prototype. Branch and MR jobs run the same environment/config, with no deliberate repetition policy. Recent overlapping jobs used 8 runner-minutes each and MR queued 6 minutes. Main qualification is required. No new CI results can be fetched. CI configuration uses a provider-neutral sketch, not executable provider syntax.",
                "justified": "Synthetic prototype. Branch and MR repeat the same tests deliberately for a reviewed reliability study this milestone. Owner accepts the compute cost. Both and integrated main remain required. No new policy evidence. Provider-neutral CI sketch.",
                "efficient": "Synthetic prototype. One MR check and one integrated-main check with distinct purposes. No duplicate triggers, queue/setup bottleneck or cache defect observed. Existing CI is adequate; provider-neutral sketch.",
            },
        }
        if args.project_context == "ongoing":
            protocol["cases"] = {
                name: facts.replace("Synthetic prototype.", "Synthetic ongoing project.")
                for name, facts in protocol["cases"].items()
            }
        for arm in ["baseline", "candidate"]:
            parts = []
            for name in ["planning.md", "results.md"]:
                path = "plugins/opdev/skills/opdev/references/" + name
                parts.append(
                    subprocess.check_output(
                        ["git", "show", protocol["baseline"] + ":" + path], cwd=repo
                    )
                    if arm == "baseline"
                    else (repo / path).read_bytes()
                )
            data = b"\n\n".join(parts)
            (args.output / f"{arm}.md").write_bytes(data)
            protocol["guidance"][arm] = sha(data)
        (args.output / "protocol.json").write_text(json.dumps(protocol, indent=2))
        return
    protocol = json.loads((args.output / "protocol.json").read_text())
    if sha(Path(args.cli).read_bytes()) != protocol["cli_sha256"]:
        raise ValueError("CLI changed since freeze")
    jobs = [(case, repeat) for repeat in range(protocol["repeats"]) for case in protocol["cases"]]
    for case, repeat in jobs:
        if not invoke(args, protocol, args.operation, case, repeat):
            raise SystemExit(
                "Stopped after unsuccessful case; inspect retained evidence before continuing"
            )


if __name__ == "__main__":
    main()
