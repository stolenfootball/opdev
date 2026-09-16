"""Private synthetic adoption sessions; never publishes raw conversations."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import time
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[1]


def run(argv, cwd=None, timeout=120):
    result = subprocess.run([str(a) for a in argv], cwd=cwd, capture_output=True,
                            text=True, encoding="utf-8", errors="replace", timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{argv[0]} failed: {result.stderr[-1500:]}")
    return result.stdout.strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def privacy(project):
    data = json.loads(run(["glab", "api", "projects/" + quote(project, safe="")]))
    if data["visibility"] != "private":
        raise ValueError("canary repository must be private")
    return data


def claude_assessment_permissions(output):
    """Session-only permissions for the user-approved candidate assessment."""
    plugin = (output / "plugin").as_posix()
    runtime = (output / "runtime").as_posix()
    def file_rule(path):
        # Claude normalizes Windows drive paths to /c/... before matching.
        if re.match(r"^[A-Za-z]:/", path):
            return "//" + path[0].lower() + path[2:]
        return "/" + path
    executable = runtime + "/runtimes/v0.2.0/x86_64-pc-windows-msvc/opdev.exe"
    lookup = f"powershell -NoProfile -File {plugin}/scripts/runtime.ps1 -Mode Path"
    commands = [lookup, f"& '{plugin}/scripts/runtime.ps1' -Mode Path",
                f"& '{executable}' plugin verify --contract '{plugin}/opdev-compatibility.json'",
                *[f"& '{executable}' {suffix}" for suffix in
                  ("--help", "adoption --help", "adoption catalog", "init --dry-run",
                   "adoption plan --help", "adoption status --format json")],
                "python -m unittest discover -s tests"]
    return {"permissions": {
        "allow": [f"Read({file_rule(plugin)}/**)", f"Read({file_rule(runtime)}/**)",
                  *[f"PowerShell({command})" for command in commands]],
        "deny": [f"Edit({file_rule(plugin)}/**)", f"Edit({file_rule(runtime)}/**)"]}}, commands


def prepare(args):
    project = privacy(args.project)
    args.output.mkdir(parents=True, exist_ok=False)
    seed = args.output / "seed"
    if args.existing_seed:
        run(["git", "clone", "--quiet", "--branch", "develop", project["ssh_url_to_repo"], seed])
        if run(["git", "rev-parse", "HEAD"], seed) != args.existing_seed:
            raise ValueError("remote fixture revision changed; refusing reuse")
    else:
        shutil.copytree(ROOT / "benchmarks/adoption/fixture", seed)
    run([sys.executable, "-m", "unittest", "discover", "-s", "tests"], seed)
    for git_args in (["init", "-b", "develop"], ["config", "user.name", "OpDev Canary"],
                     ["config", "user.email", "opdev-canary@example.invalid"],
                     ["add", "."], ["commit", "-m", "test: synthetic adoption fixture"],
                     ["branch", "main"], ["remote", "add", "origin", project["ssh_url_to_repo"]],
                     ["push", "-u", "origin", "develop", "main"]):
        if not args.existing_seed:
            run(["git", *git_args], seed)
    plugin = args.output / "plugin"
    shutil.copytree(ROOT / "plugins/opdev", plugin)
    runtime = args.output / "runtime/runtimes/v0.2.0/x86_64-pc-windows-msvc"
    runtime.mkdir(parents=True)
    shutil.copy2(args.opdev, runtime / "opdev.exe")
    (runtime / "opdev.sha256").write_text(sha(args.opdev) + "\n", encoding="ascii")
    for host in ("codex", "claude"):
        checkout = args.output / host
        run(["git", "clone", "--quiet", "--branch", "develop", project["ssh_url_to_repo"], checkout])
        if host == "codex":
            # Skill discovery is native; preserve its relative runtime/reference paths.
            shutil.copytree(plugin / "skills", checkout / ".agents/skills")
            shutil.copytree(plugin / "scripts", checkout / ".agents/scripts")
            for name in ("runtime.lock", "opdev-compatibility.json"):
                shutil.copy2(plugin / name, checkout / ".agents" / name)
    save(args.output / "manifest.json", {
        "project": args.project, "project_id": project["id"], "visibility": project["visibility"],
        "seed_revision": run(["git", "rev-parse", "HEAD"], seed),
        "candidate_cli_sha256": sha(args.opdev), "source_revision": run(["git", "rev-parse", "HEAD"], ROOT),
        "candidate_plugin_files": {str(p.relative_to(plugin)): sha(p) for p in plugin.rglob("*") if p.is_file()},
        "scope": "test-only unpublished candidate, not a managed installation or release",
    })
    print(args.output)


def session(args):
    manifest = json.loads((args.output / "manifest.json").read_text())
    privacy(manifest["project"])
    checkout = args.output / args.host
    events = args.output / f"{args.host}-{args.label}.events.jsonl"
    errors = events.with_suffix(".stderr")
    if events.exists() or errors.exists():
        raise ValueError("use a new trial label; never overwrite a failed trial")
    before = run(["git", "status", "--porcelain"], checkout)
    environment = os.environ.copy()
    environment["OPDEV_DATA_DIR"] = str(args.output / "runtime")
    prompt = args.prompt + "\n\nSession environment: use the candidate OpDev integration available in this workspace. "
    prompt += "The isolated OPDEV_DATA_DIR contains the test-only candidate CLI, not the published build. "
    prompt += f"Only this checkout and its private remote {manifest['project']} are in scope. "
    prompt += "Do not access other projects, change global installations, publish a public release, or delegate."
    if args.host == "codex":
        prompt += " The candidate shared skill is .agents/skills/opdev/SKILL.md."
        argv = [args.executable, "exec", "--json", "--ignore-user-config", "--sandbox", "workspace-write",
                "-c", 'windows.sandbox="elevated"', "-c", 'approval_policy="never"', "--cd", checkout, "-"]
    else:
        permissions, commands = claude_assessment_permissions(args.output)
        prompt += " Candidate assessment commands are explicitly approved for this session: "
        prompt += json.dumps(commands) + ". These permissions do not approve project policy choices."
        argv = [args.executable, "-p", "--output-format", "stream-json", "--verbose",
                "--plugin-dir", args.output / "plugin", "--permission-mode", "acceptEdits",
                "--add-dir", args.output / "plugin",
                "--permission-prompts", "none", "--strict-mcp-config",
                "--settings", json.dumps(permissions)]
    if args.resume:
        if args.host == "codex":
            argv = argv[:-1] + ["resume", args.resume, "-"]
        else:
            argv += ["--resume", args.resume]
    save(events.with_suffix(".request.json"), {"prompt": prompt, "argv": [str(a) for a in argv],
         "host_version": run([args.executable, "--version"]), "before_status": before,
         "timeout_seconds": 600})
    started = time.monotonic()
    timed_out = False
    with events.open("w", encoding="utf-8") as out, errors.open("w", encoding="utf-8") as err:
        process = subprocess.Popen([str(a) for a in argv], cwd=checkout, env=environment,
                                   stdin=subprocess.PIPE, stdout=out, stderr=err, text=True, encoding="utf-8")
        try:
            process.communicate(prompt, timeout=600)
        except subprocess.TimeoutExpired:
            timed_out = True
            if os.name == "nt":
                run(["taskkill", "/PID", process.pid, "/T", "/F"])
            else:
                process.kill()
            process.communicate()
    save(events.with_suffix(".result.json"), {
        "exit_code": process.returncode, "seconds": time.monotonic() - started,
        "timed_out": timed_out,
        "events_sha256": sha(events), "after_status": run(["git", "status", "--porcelain"], checkout),
        "head": run(["git", "rev-parse", "HEAD"], checkout), "grade": "review_required",
    })
    print(events)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["prepare", "session"])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--project")
    parser.add_argument("--opdev", type=Path)
    parser.add_argument("--existing-seed", help="Reuse an exact existing private fixture commit without pushing")
    parser.add_argument("--host", choices=["codex", "claude"])
    parser.add_argument("--executable")
    parser.add_argument("--label")
    parser.add_argument("--prompt")
    parser.add_argument("--resume")
    args = parser.parse_args()
    required = ("project", "opdev") if args.mode == "prepare" else ("host", "executable", "label", "prompt")
    for name in required:
        if not getattr(args, name):
            parser.error(f"{args.mode} requires --{name}")
    if args.label and not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]*", args.label):
        parser.error("--label must be a simple trial name without path separators")
    args.output = args.output.resolve()
    (prepare if args.mode == "prepare" else session)(args)


if __name__ == "__main__":
    main()
