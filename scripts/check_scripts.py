"""Invoke standard pinned tools; no auto-install, auto-fix or custom style rules."""

import argparse
import json
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def commands(mode, system):
    pins = json.loads((ROOT / "scripts/style-tools.json").read_text())
    suffix = ".exe" if system == "Windows" else ""

    def tool(name):
        return str(ROOT / "target/style-tools" / name / pins[name]["version"] / (name + suffix))

    # Maintained scripts and installers only. Frozen benchmark inputs stay untouched.
    shell = sorted(
        str(path.relative_to(ROOT))
        for base in ("scripts", "tests", "plugins", "release")
        for path in (ROOT / base).rglob("*.sh")
    )
    if mode == "format":
        result = [
            [tool("ruff"), "format", "--check", "scripts", "tests", "examples"],
            [tool("shfmt"), "-d", "-i", "4", "-ci", *shell],
        ]
    else:
        result = [
            [tool("ruff"), "check", "scripts", "tests", "examples"],
            [tool("shellcheck"), "--severity=warning", *shell],
        ]
    if system == "Windows":
        result.append(
            ["powershell", "-NoProfile", "-File", "scripts/check_powershell.ps1", "-Mode", mode]
        )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("format", "lint"))
    args = parser.parse_args()
    for argv in commands(args.mode, platform.system()):
        subprocess.run(argv, cwd=ROOT, check=True, timeout=180)
    if platform.system() != "Windows":
        print("PowerShell style checks are separate required Windows CI checks, not verified here.")


if __name__ == "__main__":
    main()
