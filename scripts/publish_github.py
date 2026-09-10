#!/usr/bin/env python3
"""Publish an exact signed CI bundle through a resumable GitHub draft release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import urllib.error
import urllib.parse
import urllib.request

REPO = 'stolenfootball/opdev'
API = f'https://api.github.com/repos/{REPO}'


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        # API writes must never forward publication credentials through redirects.
        return None


class GitHub:
    def __init__(self, token):
        if not token:
            raise ValueError('Missing protected GitHub release credential')
        self.token = token

    def request(self, method, path, body=None, binary=None):
        url = path if path.startswith('https://') else API + path
        if urllib.parse.urlparse(url).netloc not in ('api.github.com', 'uploads.github.com'):
            raise ValueError('Unexpected GitHub API host')
        headers = {'Authorization': f'Bearer {self.token}', 'Accept': 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28'}
        data = binary if binary is not None else (json.dumps(body).encode() if body is not None else None)
        headers['Content-Type'] = 'application/octet-stream' if binary is not None else 'application/json'
        request = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.build_opener(NoRedirect()).open(request, timeout=180) as response:
                raw = response.read()
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as error:
            if method == 'GET' and error.code == 404:
                return None
            raise RuntimeError(f'GitHub {method} failed with HTTP {error.code}') from None


def inventory(bundle, tag, revision):
    if not re.fullmatch(r'v0\.\d+\.\d+(?:-rc\.\d+)?', tag) or not re.fullmatch(r'[a-f0-9]{40}', revision):
        raise ValueError('Invalid immutable release identity')
    files = {}
    for p in bundle.iterdir():
        if p.is_symlink() or not p.is_file() or not re.fullmatch(r'[A-Za-z0-9_.-]+', p.name):
            raise ValueError('Unexpected release bundle entry')
        files[p.name] = hashlib.sha256(p.read_bytes()).hexdigest()
    version = tag[1:].split('-rc.')[0]
    required = {'opdev-installer.sh', 'opdev-installer.ps1', 'dist-manifest.json', 'SHA256SUMS', 'sha256.sum', 'opdev-release-manifest.json', 'opdev-provenance.intoto.json', f'opdev-{version}.cdx.json', f'opdev-plugin-{version}.tar.gz', 'cargo-dist-LICENSE-MIT.txt'}
    targets = ('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc', 'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu', 'x86_64-apple-darwin', 'aarch64-apple-darwin')
    for t in targets:
        ext = 'zip' if 'windows' in t else 'tar.gz'
        required.update((f'opdev-{version}-{t}.{ext}', f'opdev-{t}.{ext}'))
        required.add(f'opdev-{t}.{ext}.sha256')
    signed = {name for name in required if name.endswith(('.tar.gz', '.zip', '.sh', '.ps1'))}
    required.update(name + '.sigstore.json' for name in signed)
    if files.keys() != required:
        raise ValueError(f'Release inventory mismatch: missing={required - files.keys()}, extra={files.keys() - required}')
    covered = set()
    for line in (bundle / 'SHA256SUMS').read_text().splitlines():
        digest, name = line.split(maxsplit=1)
        name = name.lstrip('*')
        if name in covered or files.get(name) != digest:
            raise ValueError(f'Checksum mismatch: {name}')
        covered.add(name)
    expected_coverage = {n for n in files if not n.endswith('.sigstore.json')} - {'SHA256SUMS', 'opdev-release-manifest.json', 'opdev-provenance.intoto.json'}
    if covered != expected_coverage:
        raise ValueError('Checksum inventory is incomplete')
    # The provenance associates the same exact GitLab commit as the release tag.
    provenance = json.loads((bundle / 'opdev-provenance.intoto.json').read_text())
    sources = provenance['predicate']['buildDefinition']['resolvedDependencies']
    if not any(s.get('uri') == 'https://gitlab.com/stolenfootball-tools/opdev' and s.get('digest', {}).get('gitCommit') == revision for s in sources):
        raise ValueError('Exact GitLab source revision missing from provenance')
    return files


def tag_revision(api, tag):
    ref = api.request('GET', '/git/ref/tags/' + tag)
    if ref is None:
        return None
    obj = ref['object']
    for _ in range(10):
        if obj['type'] == 'commit':
            return obj['sha']
        if obj['type'] != 'tag':
            break
        obj = api.request('GET', '/git/tags/' + obj['sha'])['object']
    raise ValueError('Release tag does not resolve to a commit')


def assets(api, release_id):
    result = {}
    page = 1
    while True:
        batch = api.request('GET', f'/releases/{release_id}/assets?per_page=100&page={page}')
        for a in batch:
            if a['name'] in result or a['state'] != 'uploaded':
                raise ValueError('Duplicate or incomplete remote asset')
            result[a['name']] = a.get('digest', '')
        if len(batch) < 100:
            return result
        page += 1


def publish(api, bundle, tag, revision):
    expected = inventory(bundle, tag, revision)
    if not api.request('GET', '/immutable-releases')['enabled']:
        raise ValueError('Enable immutable GitHub releases before publication')
    actual_revision = tag_revision(api, tag)
    if actual_revision is None:
        # The build handoff must have mirrored this commit before publication.
        if api.request('GET', '/git/commits/' + revision) is None:
            raise ValueError('Qualified commit is missing from GitHub mirror')
        api.request('POST', '/git/refs', {'ref': 'refs/tags/' + tag, 'sha': revision})
    elif actual_revision != revision:
        raise ValueError('Refusing to move an existing release tag')
    if tag_revision(api, tag) != revision:
        raise ValueError('Release tag synchronization failed')
    release = api.request('GET', '/releases/tags/' + tag)
    if release is None:
        release = api.request('POST', '/releases', {'tag_name': tag, 'target_commitish': revision, 'name': 'OpDev ' + tag, 'draft': True, 'prerelease': '-rc.' in tag, 'body': 'Qualified and signed by the protected GitLab release pipeline. See the attached checksums, Sigstore bundles, SBOM, manifest, and provenance. Source: https://gitlab.com/stolenfootball-tools/opdev/-/tree/' + revision})
    current = assets(api, release['id'])
    for name, digest in current.items():
        if name not in expected or digest != 'sha256:' + expected[name]:
            raise ValueError('Existing release assets differ; no assets will be replaced')
    if not release['draft']:
        if not release.get('immutable') or current.keys() != expected.keys():
            raise ValueError('Published release is incomplete or mutable')
        return
    upload = release['upload_url'].split('{', 1)[0]
    for name in sorted(expected.keys() - current.keys()):
        api.request('POST', upload + '?name=' + urllib.parse.quote(name), binary=(bundle / name).read_bytes())
    if assets(api, release['id']) != {n: 'sha256:' + h for n, h in expected.items()}:
        raise ValueError('Uploaded release inventory or digests differ')
    if tag_revision(api, tag) != revision:
        raise ValueError('Release tag changed during upload')
    release = api.request('PATCH', f"/releases/{release['id']}", {'draft': False, 'make_latest': 'false' if '-rc.' in tag else 'true'})
    if release['draft'] or not release.get('immutable'):
        raise ValueError('Publication did not produce an immutable release')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    if args.check_only:
        inventory(args.bundle, args.tag, args.revision)
    else:
        if os.environ.get('CI_COMMIT_REF_PROTECTED') != 'true' or os.environ.get('CI_COMMIT_TAG') != args.tag:
            raise SystemExit('Publication requires a protected GitLab tag job')
        publish(GitHub(os.environ.get('GH_OPDEV_RELEASE_TOKEN')), args.bundle, args.tag, args.revision)
    print('Release inventory passed.' if args.check_only else 'Immutable GitHub release published and verified.')
