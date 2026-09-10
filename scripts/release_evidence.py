#!/usr/bin/env python3
"""Bind all already-built legacy/dist archives and installers to release evidence."""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--opdev', required=True)
parser.add_argument('--bundle', type=Path, required=True)
parser.add_argument('--version', required=True)
parser.add_argument('--revision', required=True)
parser.add_argument('--build-run', required=True)
args = parser.parse_args()
sbom = args.bundle / f'opdev-{args.version}.cdx.json'
command = [args.opdev, 'release', 'evidence']
for p in sorted(args.bundle.iterdir()):
    if p != sbom:
        command += ['--artifact', str(p)]
command += ['--sbom', str(sbom), '--source-uri', 'https://gitlab.com/stolenfootball-tools/opdev', '--source-revision', args.revision, '--builder-id', f'https://github.com/stolenfootball/opdev/actions/runs/{args.build_run}', '--assurance-limitation', 'Native binaries were built and smoke-tested by GitHub Actions once. The protected GitLab pipeline repackaged the same binary bytes with cargo-dist, generated verified installers, and bound these assets before signing and GitHub publication. The CycloneDX inventory is an all-target dependency superset. No trusted-builder provenance or SLSA Build level is claimed.', '--output', str(args.bundle)]
subprocess.run(command, check=True)
