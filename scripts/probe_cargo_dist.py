#!/usr/bin/env python3
"""Reproduce cargo-dist's current GitLab-only installer-generation limitation."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--dist', required=True, help='Path to checksum-verified cargo-dist 0.32.0')
args = parser.parse_args()
dist = str(Path(args.dist).resolve())
version = subprocess.run([dist, '--version'], check=True, capture_output=True, text=True).stdout.strip()
if version != 'cargo-dist 0.32.0':
    raise SystemExit(f'Expected dist 0.32.0, got {version!r}')
with tempfile.TemporaryDirectory(prefix='opdev-dist-probe-') as directory:
    source = Path(__file__).resolve().parents[1] / 'release/cargo-dist-probe'
    for path in source.glob('*.toml'):
        shutil.copy2(path, Path(directory) / path.name)
    result = subprocess.run([dist, 'build', '--artifacts=global', '--tag', 'v0.1.1'],
                            cwd=directory, capture_output=True, text=True, timeout=120)
    expected = result.returncode != 0 and 'No GitHub hosting is defined' in result.stderr
    print(json.dumps({'version': version, 'outcome': 'error' if expected else 'unverified',
                      'reason': 'GitLab-only generation is blocked by the GitHub-only install receipt' if expected else 'Behavior changed; review before adopting cargo-dist',
                      'exit_code': result.returncode, 'diagnostic': result.stderr}, indent=2))
    # A successful reproduction is not successful installer generation.
    raise SystemExit(0 if expected else 1)
