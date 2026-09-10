"""Offline regression checks for release preparation and publication recovery."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import tarfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


dist = module('prepare_dist')
publisher = module('publish_github')


class ReleaseTests(unittest.TestCase):
    def test_archive_reader_refuses_links_and_duplicates(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'input.tar.gz'
            for linked in (True, False):
                with tarfile.open(path, 'w:gz') as t:
                    entry = tarfile.TarInfo('opdev')
                    if linked:
                        entry.type = tarfile.SYMTYPE
                        entry.linkname = 'elsewhere'
                        t.addfile(entry)
                    else:
                        entry.size = 3
                        t.addfile(entry, io.BytesIO(b'abc'))
                        t.addfile(entry, io.BytesIO(b'abc'))
                with self.assertRaises(ValueError):
                    dist.executable(path, 'x86_64-unknown-linux-gnu')

    def test_template_drift_stops_generation(self):
        for ext in ('sh', 'ps1'):
            with self.assertRaises(ValueError):
                dist.harden('upstream changed its extraction path', ext, '0.1.2')

    def bundle(self, directory):
        names = {'opdev-installer.sh', 'opdev-installer.ps1', 'dist-manifest.json', 'sha256.sum', 'opdev-release-manifest.json', 'opdev-provenance.intoto.json', 'opdev-0.1.2.cdx.json', 'opdev-plugin-0.1.2.tar.gz', 'cargo-dist-LICENSE-MIT.txt'}
        for target in dist.TARGETS:
            ext = 'zip' if 'windows' in target else 'tar.gz'
            names.update((f'opdev-0.1.2-{target}.{ext}', f'opdev-{target}.{ext}'))
            names.add(f'opdev-{target}.{ext}.sha256')
        names.update(n + '.sigstore.json' for n in list(names) if n.endswith(('.tar.gz', '.zip', '.sh', '.ps1')))
        for name in names:
            (directory / name).write_text('fixture ' + name)
        (directory / 'opdev-provenance.intoto.json').write_text(json.dumps({'predicate': {'buildDefinition': {'resolvedDependencies': [{'uri': 'https://gitlab.com/stolenfootball-tools/opdev', 'digest': {'gitCommit': 'a' * 40}}]}}}))
        sums = '\n'.join(hashlib.sha256((directory / n).read_bytes()).hexdigest() + '  ' + n for n in sorted(names) if not n.endswith('.sigstore.json') and n not in ('opdev-release-manifest.json', 'opdev-provenance.intoto.json'))
        (directory / 'SHA256SUMS').write_text(sums + '\n')

    def test_publication_resumes_draft_then_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle = Path(tmp)
            self.bundle(bundle)
            api = FakeGitHub()
            api.interrupt_upload = True
            with self.assertRaises(RuntimeError):
                publisher.publish(api, bundle, 'v0.1.2', 'a' * 40)
            self.assertTrue(api.release['draft'])
            self.assertEqual(len(api.uploaded), 1)
            publisher.publish(api, bundle, 'v0.1.2', 'a' * 40)
            self.assertFalse(api.release['draft'])
            uploads = len(api.uploaded)
            publisher.publish(api, bundle, 'v0.1.2', 'a' * 40)
            self.assertEqual(len(api.uploaded), uploads)
            self.assertEqual(api.published, 1)

    def test_refuses_wrong_tag_mutability_tampering_and_missing_signatures(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle = Path(tmp)
            self.bundle(bundle)
            for api in (FakeGitHub(revision='b' * 40), FakeGitHub(immutable=False)):
                with self.assertRaises(ValueError):
                    publisher.publish(api, bundle, 'v0.1.2', 'a' * 40)
                self.assertFalse(api.uploaded)
            script = bundle / 'opdev-installer.sh'
            script.write_text('changed')
            with self.assertRaises(ValueError):
                publisher.inventory(bundle, 'v0.1.2', 'a' * 40)
            self.bundle(bundle)
            (bundle / 'opdev-installer.sh.sigstore.json').unlink()
            with self.assertRaises(ValueError):
                publisher.inventory(bundle, 'v0.1.2', 'a' * 40)

    def test_refuses_draft_asset_replacement(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle = Path(tmp)
            self.bundle(bundle)
            api = FakeGitHub()
            api.release = {'id': 1, 'draft': True, 'upload_url': 'https://uploads.github.com/release{?name}'}
            api.uploaded['opdev-installer.sh'] = 'sha256:wrong'
            with self.assertRaises(ValueError):
                publisher.publish(api, bundle, 'v0.1.2', 'a' * 40)
            self.assertEqual(api.published, 0)


class FakeGitHub:
    def __init__(self, revision=None, immutable=True):
        self.revision = revision
        self.immutable = immutable
        self.release = None
        self.uploaded = {}
        self.published = 0
        self.interrupt_upload = False

    def request(self, method, path, body=None, binary=None):
        if path == '/immutable-releases':
            return {'enabled': self.immutable}
        if path.startswith('/git/ref/tags/'):
            return {'object': {'type': 'commit', 'sha': self.revision}} if self.revision else None
        if path.startswith('/git/commits/'):
            return {'sha': 'a' * 40}
        if path == '/git/refs':
            self.revision = body['sha']
            return {}
        if path.startswith('/releases/tags/'):
            return self.release
        if path == '/releases':
            self.release = {'id': 1, 'draft': True, 'upload_url': 'https://uploads.github.com/release{?name}'}
            return self.release
        if '/assets?' in path:
            return [{'name': n, 'digest': h, 'state': 'uploaded'} for n, h in self.uploaded.items()]
        if path.startswith('https://uploads.github.com/'):
            if self.interrupt_upload and self.uploaded:
                self.interrupt_upload = False
                raise RuntimeError('interrupted upload')
            self.uploaded[path.split('name=')[1]] = 'sha256:' + hashlib.sha256(binary).hexdigest()
            return {}
        if method == 'PATCH':
            self.published += 1
            self.release.update(draft=False, immutable=True)
            return self.release
        raise AssertionError((method, path))


if __name__ == '__main__':
    unittest.main()
