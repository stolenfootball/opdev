#!/usr/bin/env python3
"""Exact-revision GitHub ARM64 gate; MR jobs need no GitHub credential."""

import argparse
import base64
import json
import os
import re
import subprocess
import sys
import time
import urllib.parse
import urllib.request

REPOSITORY = "stolenfootball/opdev"
WORKFLOW = ".github/workflows/arm64-installer.yml"
API = f"https://api.github.com/repos/{REPOSITORY}"


def read_json(url):
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "opdev-arm64-gate",
    })
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def validate_revision(revision):
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("expected an exact 40-character source revision")


def select_run(payload, revision):
    matches = [run for run in payload["workflow_runs"] if
               run.get("head_sha") == revision and
               run.get("head_branch") == f"opdev-arm64/{revision}" and
               run.get("event") == "push" and run.get("path") == WORKFLOW and
               run.get("head_repository", {}).get("full_name") == REPOSITORY]
    return max(matches, key=lambda run: (run["id"], run["run_attempt"]), default=None)


def qualified_jobs(payload):
    matches = [job for job in payload["jobs"] if job.get("name") == "installer-arm64"]
    return (len(matches) == 1 and matches[0].get("status") == "completed" and
            matches[0].get("conclusion") == "success" and
            "ubuntu-24.04-arm" in matches[0].get("labels", []))


def mirror_revision(revision, token):
    """Protected jobs only: publish one immutable test ref, never main or all refs."""
    validate_revision(revision)
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if actual != revision:
        raise ValueError("checkout does not match requested revision")
    environment = os.environ.copy()
    # Keep credentials out of URLs, command arguments and diagnostics.
    credential = base64.b64encode(f"x-access-token:{token}".encode()).decode()
    environment.update({"GIT_TERMINAL_PROMPT": "0", "GIT_CONFIG_COUNT": "1",
                        "GIT_CONFIG_KEY_0": "http.https://github.com/.extraheader",
                        "GIT_CONFIG_VALUE_0": f"AUTHORIZATION: basic {credential}"})
    subprocess.run(["git", "push", f"https://github.com/{REPOSITORY}.git",
                    f"{revision}:refs/heads/opdev-arm64/{revision}"],
                   env=environment, check=True, timeout=90)


def await_result(revision, timeout=1200, interval=60, fetch=read_json,
                 clock=time.monotonic, sleep=time.sleep):
    validate_revision(revision)
    if timeout <= 0 or interval <= 0:
        raise ValueError("timeout and polling interval must be positive")
    deadline = clock() + timeout
    query = urllib.parse.urlencode({"head_sha": revision, "event": "push", "per_page": 100})
    while clock() < deadline:
        run = select_run(fetch(f"{API}/actions/runs?{query}"), revision)
        if run is not None and run["status"] == "completed":
            if run["conclusion"] != "success":
                raise RuntimeError(f"ARM64 workflow {run['id']} concluded {run['conclusion']}; no pass")
            jobs = fetch(f"{API}/actions/runs/{run['id']}/attempts/{run['run_attempt']}/jobs?per_page=100")
            if not qualified_jobs(jobs):
                raise RuntimeError("ARM64 job missing, skipped, failed or on wrong runner; no pass")
            print(f"passed: native ARM64 installer for {revision}: https://github.com/{REPOSITORY}/actions/runs/{run['id']}", flush=True)
            return
        print(f"unverified: awaiting native ARM64 result for {revision}", flush=True)
        sleep(min(interval, max(0, deadline - clock())))
    raise TimeoutError("native ARM64 qualification unavailable; mirror this exact revision to opdev-arm64/<revision> using an authorized GitHub login")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--timeout", type=int, default=1200)
    args = parser.parse_args()
    try:
        validate_revision(args.revision)
        token = os.environ.get("GH_OPDEV_RELEASE_TOKEN")
        if token:
            mirror_revision(args.revision, token)
        else:
            print("Read-only MR gate: maintainer must push the exact revision to the GitHub opdev-arm64/<revision> ref.", flush=True)
        await_result(args.revision, args.timeout)
    except (ValueError, KeyError, TypeError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
