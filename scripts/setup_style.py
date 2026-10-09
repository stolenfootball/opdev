"""Explicit, repository-local setup of pinned upstream style tools (no PATH edits)."""

import hashlib
import io
import json
from pathlib import Path
import platform
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "target/style-tools"


def executable_bytes(raw, url, name):
    if url.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            entries = [entry for entry in archive.infolist() if Path(entry.filename).name == name]
            if len(entries) != 1 or entries[0].is_dir():
                raise ValueError("expected exactly one executable")
            return archive.read(entries[0])
    if url.endswith(".tar.gz"):
        with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
            entries = [entry for entry in archive if Path(entry.name).name == name]
            if len(entries) != 1 or not entries[0].isfile():
                raise ValueError("expected exactly one regular executable")
            return archive.extractfile(entries[0]).read()
    return raw


def main():
    system = platform.system()
    if platform.machine().lower() not in ("amd64", "x86_64") or system not in ("Windows", "Linux"):
        raise ValueError("automated style setup supports Windows/Linux x64 development hosts")
    pins = json.loads((ROOT / "scripts/style-tools.json").read_text())
    for name, pin in pins.items():
        if system not in pin["assets"]:
            continue  # PowerShell qualification belongs to the required Windows job.
        url, digest, filename = pin["assets"][system]
        destination = TOOLS / name / pin["version"]
        # Download and verify even on warm setup; never trust an existing executable.
        with urllib.request.urlopen(url, timeout=60) as response:
            raw = response.read(64 * 1024 * 1024 + 1)
        if len(raw) > 64 * 1024 * 1024 or hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError(f"{name}: upstream package digest mismatch")
        destination.mkdir(parents=True, exist_ok=True)
        if name == "PSScriptAnalyzer":
            with zipfile.ZipFile(io.BytesIO(raw)) as archive:
                for entry in archive.infolist():
                    path = destination / entry.filename
                    if not path.resolve().is_relative_to(destination.resolve()):
                        raise ValueError("module entry escaped installation directory")
                archive.extractall(destination)
        else:
            output = destination / filename
            output.write_bytes(executable_bytes(raw, url, filename))
            output.chmod(0o755)
        print(f"Installed {name} {pin['version']} in repository-local target/style-tools")


if __name__ == "__main__":
    main()
