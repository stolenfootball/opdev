#!/usr/bin/env python3
"""Package existing native binaries and harden pinned cargo-dist installers.

Never compiles or publishes. Inputs come from the qualified native build handoff.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = [row.split()[3] for row in (ROOT / 'plugins/opdev/runtime.lock').read_text().splitlines() if row.startswith('target ')]


def executable(archive, target):
    """Read only the single expected regular executable; never extract archive paths."""
    name = 'opdev.exe' if 'windows' in target else 'opdev'
    if archive.suffix == '.zip':
        with zipfile.ZipFile(archive) as z:
            entries = [e for e in z.infolist() if e.filename == name]
            if len(entries) != 1 or (entries[0].external_attr >> 16) & 0o170000 not in (0, 0o100000):
                raise ValueError('Expected one regular executable')
            return z.read(entries[0])
    with tarfile.open(archive) as t:
        entries = [e for e in t.getmembers() if e.name == name]
        if len(entries) != 1 or not entries[0].isfile():
            raise ValueError('Expected one regular executable')
        return t.extractfile(entries[0]).read()


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise ValueError(f'cargo-dist template changed: expected one {old!r}')
    return text.replace(old, new)


def harden(text, extension, tag):
    rows = [l.split() for l in (ROOT / 'plugins/opdev/runtime.lock').read_text().splitlines() if l and not l.startswith('#')]
    pins = {r[0]: r[1] for r in rows if len(r) == 2}
    identity = f'https://gitlab.com/stolenfootball-tools/opdev//.gitlab-ci.yml@refs/tags/{tag}'
    addition = (ROOT / f'release/installers/verify.{extension}').read_text()
    addition = addition.replace('@COSIGN_VERSION@', pins['cosign']).replace('@IDENTITY@', identity)
    if extension == 'sh':
        cases = '\n'.join(f'        {r[1]}/{r[2]}) verifier_asset={r[4]}; verifier_digest={r[5]};;' for r in rows if r[0] == 'target' and r[1] != 'Windows')
        addition = addition.replace('@VERIFIER_CASES@', cases)
        text = replace_once(text, '    # unpack the archive', '    opdev_verify_archive "$_file" "$_url" || { rm -rf "$_dir"; exit 1; }\n\n    # unpack the archive')
        text = replace_once(text, 'download_binary_and_run_installer "$@" || exit 1', addition + '\ndownload_binary_and_run_installer "$@" || exit 1')
        text = replace_once(text, '    INSTALL_UPDATER=1', '    INSTALL_UPDATER=0 # Optional updater is not qualified by OpDev.')
    else:
        digest = next(r[5] for r in rows if r[:3] == ['target', 'Windows', 'X64'])
        addition = addition.replace('@WINDOWS_DIGEST@', digest)
        text = replace_once(text, '  Write-Verbose "Unpacking to $tmp"', '  try { Test-OpdevSignedArchive $dir_path $url } catch { Remove-Item -LiteralPath $tmp -Recurse -Force; throw }\n\n  Write-Verbose "Unpacking to $tmp"')
        text = replace_once(text, '# The default interactive handler', addition + '\n# The default interactive handler')
        text = replace_once(text, '  $install_updater = $true', '  $install_updater = $false # Optional updater is not qualified by OpDev.')
    if re.search(r'@[A-Z_]+@', text):
        raise ValueError('Unexpanded installer verification placeholder')
    return text


def prepare(dist, inputs, output, version, targets, tag=None, opdev=None):
    if not re.fullmatch(r'0\.\d+\.\d+(?:-rc\.\d+)?', version):
        raise ValueError('Invalid release version')
    tag = tag or 'v' + version
    if not re.fullmatch(re.escape('v' + version) + r'(?:-rc\.\d+)?', tag):
        raise ValueError('Release tag does not match the CLI version')
    if subprocess.check_output([dist, '--version'], text=True).strip() != 'cargo-dist 0.32.0':
        raise ValueError('Expected cargo-dist 0.32.0')
    if output.exists():
        raise ValueError('Refusing to overwrite dist output')
    if opdev is None:
        raise ValueError('The qualified OpDev packager is required')
    with tempfile.TemporaryDirectory(prefix='opdev-dist-') as tmp:
        work = Path(tmp)
        config = (ROOT / 'release/cargo-dist/dist-workspace.toml').read_text()
        config = re.sub(r'^targets = .*$', 'targets = ' + json.dumps(targets), config, flags=re.M)
        (work / 'dist-workspace.toml').write_text(config)
        # This generic build command only stages a previously built binary.
        helper = work / 'stage.py'
        helper.write_text('import os, shutil\nfrom pathlib import Path\nt = os.environ["CARGO_DIST_TARGET"]\nn = "opdev.exe" if "windows" in t else "opdev"\nshutil.copy2(Path("inputs") / t / n, n)\n')
        package = {'name': 'opdev', 'version': tag[1:], 'description': 'OpDev CLI', 'repository': 'https://github.com/stolenfootball/opdev', 'binaries': ['opdev'], 'build-command': ['python3', 'stage.py']}
        (work / 'dist.toml').write_text('[package]\n' + '\n'.join(f'{k} = {json.dumps(v)}' for k, v in package.items()) + '\n')
        for target in targets:
            ext = 'zip' if 'windows' in target else 'tar.gz'
            data = executable(inputs / f'opdev-{version}-{target}.{ext}', target)
            binary = work / 'inputs' / target / ('opdev.exe' if 'windows' in target else 'opdev')
            binary.parent.mkdir(parents=True)
            binary.write_bytes(data)
            binary.chmod(0o755)
        result = subprocess.run([dist, 'build', '--artifacts=all', '--tag', tag, '--output-format=json', '--no-local-paths'], cwd=work, check=True, text=True, capture_output=True)
        manifest = json.loads(result.stdout)
        distrib = work / 'target/distrib'
        checksum_changes = {}
        # Assert cargo-dist's repackaging preserved all executable bytes.
        for target in targets:
            ext = 'zip' if 'windows' in target else 'tar.gz'
            archive = distrib / f'opdev-{target}.{ext}'
            if ext == 'zip':
                data = executable(archive, target)
            else:
                with tarfile.open(archive) as t:
                    data = t.extractfile(f'opdev-{target}/opdev').read()
            original = (work / 'inputs' / target / ('opdev.exe' if 'windows' in target else 'opdev')).read_bytes()
            if data != original:
                raise ValueError('cargo-dist changed executable bytes')
            # cargo-dist archives retain wall-clock metadata. Normalize using our
            # existing deterministic packager before signing or embedding digests.
            before = hashlib.sha256(archive.read_bytes()).hexdigest()
            canonical = work / 'canonical' / archive.name
            binary = work / 'inputs' / target / ('opdev.exe' if 'windows' in target else 'opdev')
            entry = 'opdev.exe' if ext == 'zip' else f'opdev-{target}/opdev'
            subprocess.run([opdev, 'release', 'package', '--format', 'zip' if ext == 'zip' else 'tar-gz', '--executable-entry', f'{binary}={entry}', '--output', str(canonical)], check=True, capture_output=True)
            archive.write_bytes(canonical.read_bytes())
            checksum_changes[before] = hashlib.sha256(archive.read_bytes()).hexdigest()
        output.mkdir(parents=True)
        for path in distrib.iterdir():
            if path.is_file():
                if path.suffix in ('.sh', '.ps1'):
                    text = path.read_text()
                    for old, new in checksum_changes.items():
                        text = text.replace(old, new)
                    path.write_text(harden(text, path.suffix[1:], tag))
                shutil.copy2(path, output / path.name)
        for path in output.glob('*.sha256'):
            artifact = output / path.name.removesuffix('.sha256')
            path.write_text(hashlib.sha256(artifact.read_bytes()).hexdigest() + ' *' + artifact.name + '\n')
        sums = output / 'sha256.sum'
        sums.write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.name + '\n' for p in sorted(output.iterdir()) if p.is_file() and p != sums))
        # Refresh installer digests after applying the pinned verification extension.
        for name, artifact in manifest['artifacts'].items():
            path = output / name
            if path.is_file():
                artifact['checksums'] = {'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        (output / 'dist-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        shutil.copy2(ROOT / 'release/cargo-dist/LICENSE-MIT', output / 'cargo-dist-LICENSE-MIT.txt')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', required=True)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--tag')
    parser.add_argument('--opdev', required=True)
    parser.add_argument('--target', action='append', choices=TARGETS)
    args = parser.parse_args()
    prepare(str(Path(args.dist).resolve()), args.input.resolve(), args.output.resolve(), args.version, args.target or TARGETS, args.tag, str(Path(args.opdev).resolve()))
