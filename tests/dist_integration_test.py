#!/usr/bin/env python3
"""Generate real cargo-dist installers and exercise isolated offline installation."""
import argparse
import sys
sys.dont_write_bytecode = True
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

from dist_test import dist

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--dist', required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--opdev', required=True)
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='opdev dist integration ') as tmp:
    root = Path(tmp)
    inputs = root / 'inputs'
    inputs.mkdir()
    payload = b'#!/bin/sh\nprintf "opdev 0.1.2\\n"\n'
    for target in dist.TARGETS:
        if 'windows' in target:
            with zipfile.ZipFile(inputs / f'opdev-0.1.2-{target}.zip', 'w') as z:
                z.writestr('opdev.exe', payload)
        else:
            with tarfile.open(inputs / f'opdev-0.1.2-{target}.tar.gz', 'w:gz') as t:
                entry = tarfile.TarInfo('opdev')
                entry.mode = 0o755
                entry.size = len(payload)
                t.addfile(entry, io.BytesIO(payload))
    dist.prepare(str(Path(args.dist).resolve()), inputs, args.output.resolve(), '0.1.2', dist.TARGETS, opdev=str(Path(args.opdev).resolve()))
    candidate = root / 'candidate'
    dist.prepare(str(Path(args.dist).resolve()), inputs, candidate, '0.1.2', dist.TARGETS, 'v0.1.2-rc.1', str(Path(args.opdev).resolve()))
    for archive in args.output.iterdir():
        if archive.name.endswith(('.tar.gz', '.zip')):
            assert archive.read_bytes() == (candidate / archive.name).read_bytes(), 'Archive metadata is not deterministic'
    for ext in ('sh', 'ps1'):
        text = (candidate / ('opdev-installer.' + ext)).read_text()
        assert 'releases/download/v0.1.2-rc.1' in text
        assert 'refs/tags/v0.1.2-rc.1' in text
        assert 'releases/download/v0.1.2"' not in text
    # Exercise the actual OpDev evidence generator and publisher inventory together.
    bundle = root / 'release-bundle'
    shutil.copytree(args.output, bundle)
    for p in inputs.iterdir():
        shutil.copy2(p, bundle / p.name)
    shutil.copy2(next(inputs.glob('*.tar.gz')), bundle / 'opdev-plugin-0.1.2.tar.gz')
    (bundle / 'opdev-0.1.2.cdx.json').write_text(json.dumps({'bomFormat': 'CycloneDX', 'specVersion': '1.5', 'version': 1, 'metadata': {'component': {'type': 'application', 'name': 'opdev', 'version': '0.1.2'}}}))
    subprocess.run(['python3', str(dist.ROOT / 'scripts/release_evidence.py'), '--opdev', str(Path(args.opdev).resolve()), '--bundle', str(bundle), '--version', '0.1.2', '--revision', 'a' * 40, '--build-run', '1'], check=True)
    for p in list(bundle.iterdir()):
        if p.name.endswith(('.tar.gz', '.zip', '.sh', '.ps1')):
            (bundle / (p.name + '.sigstore.json')).write_text('fixture signature, not a published release')
    from dist_test import publisher
    publisher.inventory(bundle, 'v0.1.2', 'a' * 40)
    publisher.inventory(bundle, 'v0.1.2-rc.1', 'a' * 40)
    # Real generated scripts remain untouched in CI artifacts. Test only a private copy.
    source = args.output / 'opdev-installer.sh'
    subprocess.run(['sh', '-n', str(source)], check=True)
    fixture = root / 'fixtures'
    fixture.mkdir()
    verifier = fixture / 'verifier'
    verifier.write_text('''#!/bin/sh
set -eu
[ "$1" = verify-blob ]
[ "$5" = --certificate-identity ]
[ "$6" = https://gitlab.com/stolenfootball-tools/opdev//.gitlab-ci.yml@refs/tags/v0.1.2 ]
[ "$7" = --certificate-oidc-issuer ]
[ "$8" = https://gitlab.com ]
[ "$(cat "$4")" = valid ]
''')
    digest = hashlib.sha256(verifier.read_bytes()).hexdigest()
    text = source.read_text()
    import re
    text = re.sub(r'verifier_digest=[a-f0-9]{64}', 'verifier_digest=' + digest, text)
    script = root / 'installer.sh'
    script.write_text(text)
    # Mock curl is restricted to fixture bytes; it never contacts either host.
    tools = root / 'tools'
    tools.mkdir()
    curl = tools / 'curl'
    curl.write_text('''#!/usr/bin/env python3
import os,sys,shutil
from pathlib import Path
a=sys.argv[1:]
if '--version' in a: print('curl 8.0.0 OpenSSL/3.0');sys.exit(0)
if '--help' in a: print('--proto --tlsv1.2 --retry --silent --show-error --fail --location --output');sys.exit(0)
url=next((s for s in a if s.startswith('https://')),None)
out=a[a.index('--output')+1] if '--output' in a else a[a.index('-o')+1]
if url is None: sys.exit(92)
if '/sigstore/cosign/' in url: src=Path(os.environ['FIXTURE'])/'verifier'
elif url.endswith('.sigstore.json'): src=Path(os.environ['FIXTURE'])/'bundle'
elif url.startswith('https://github.com/stolenfootball/opdev/releases/download/v0.1.2/'): src=Path(os.environ['DIST_ASSETS'])/url.rsplit('/',1)[1]
else: sys.exit(93)
shutil.copyfile(src,out)
''')
    curl.chmod(0o755)
    for mode in ('valid', 'bad-signature', 'bad-verifier'):
        install = root / mode / 'bin'
        (fixture / 'bundle').write_text('valid' if mode != 'bad-signature' else 'invalid')
        if mode == 'bad-verifier':
            verifier.write_text('corrupt')
        env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ['PATH'], FIXTURE=str(fixture), DIST_ASSETS=str(args.output.resolve()), OPDEV_UNMANAGED_INSTALL=str(install))
        result = subprocess.run(['sh', str(script)], env=env, capture_output=True, text=True, timeout=45)
        if mode == 'valid':
            assert result.returncode == 0, result.stdout + result.stderr
            assert (install / 'opdev').read_bytes() == payload
            subprocess.run([str(install / 'opdev'), 'version'], check=True)
            repeat = subprocess.run(['sh', str(script)], env=env, capture_output=True, text=True, timeout=45)
            assert repeat.returncode == 0, repeat.stdout + repeat.stderr
        else:
            assert result.returncode != 0, 'Invalid verification accepted'
            assert not (install / 'opdev').exists(), 'Installed before verification'
        print(mode + ': passed')
print('Six-target generation, binary preservation, signed-install fixtures, and repeat installation passed.')
