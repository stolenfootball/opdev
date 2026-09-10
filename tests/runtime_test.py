"""Deterministic installer tests: no network or installed CLI required."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipIf(os.name == 'nt', 'POSIX installer; Windows has runtime_test.ps1')
class RuntimeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='opdev runtime test ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.plugin = self.root / 'plugin'
        shutil.copytree(ROOT / 'plugins/opdev', self.plugin)
        self.data = self.root / 'data with spaces'
        self.fixture = self.root / 'fixture'
        self.fixture.mkdir()
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.log = self.root / 'downloads'
        self.native_log = self.root / 'native'
        self.env = dict(os.environ, OPDEV_DATA_DIR=str(self.data), FIXTURE=str(self.fixture),
                        DOWNLOAD_LOG=str(self.log), NATIVE_LOG=str(self.native_log),
                        PATH=str(self.bin) + os.pathsep + os.environ['PATH'])
        self.verifier = self.fixture / 'verifier'
        self.verifier.write_text('''#!/bin/sh
set -eu
printf 'verify\\n' >> "$NATIVE_LOG"
[ "$1" = verify-blob ]
[ "$3" = --bundle ]
[ "$5" = --certificate-identity ]
[ "$6" = https://gitlab.com/stolenfootball-tools/opinionateddevelopment//.gitlab-ci.yml@refs/tags/v0.1.1 ]
[ "$7" = --certificate-oidc-issuer ]
[ "$8" = https://gitlab.com ]
[ "$(cat "$4")" = valid ]
''')
        digest = hashlib.sha256(self.verifier.read_bytes()).hexdigest()
        lock = self.plugin / 'runtime.lock'
        lines = lock.read_text().splitlines()
        lock.write_text('\n'.join(' '.join(line.split()[:-1] + [digest]) if line.startswith('target ') else line for line in lines) + '\n')
        curl = self.bin / 'curl'
        curl.write_text('''#!/bin/sh
set -eu
output=
while [ "$#" -gt 0 ]; do
 case "$1" in
  --output) output=$2; shift 2 ;;
  *) url=$1; shift ;;
 esac
done
printf '%s\\n' "$url" >> "$DOWNLOAD_LOG"
case "$url" in
 https://github.com/sigstore/cosign/releases/download/v3.1.3/cosign-*) cp "$FIXTURE/verifier" "$output" ;;
 https://gitlab.com/stolenfootball-tools/opdev/-/releases/v0.1.1/downloads/*.sigstore.json) cp "$FIXTURE/bundle" "$output" ;;
 https://gitlab.com/stolenfootball-tools/opdev/-/releases/v0.1.1/downloads/*.tar.gz) cp "$FIXTURE/archive" "$output" ;;
 *) exit 97 ;;
esac
''')
        curl.chmod(0o700)
        (self.fixture / 'bundle').write_text('valid')
        self.archive()

    def archive(self, version='0.1.1', compatible=True, symlink=False):
        payload = f'''#!/bin/sh
printf 'cli %s\\n' "$1" >> "$NATIVE_LOG"
case "$1" in version) printf 'opdev {version}\\nproject schema 1\\nrule catalog 1\\nextension protocol 1.0.0\\n';; plugin) exit {0 if compatible else 1};; *) printf '%s\\n' "$@";; esac
'''.encode()
        with tarfile.open(self.fixture / 'archive', 'w:gz') as archive:
            entry = tarfile.TarInfo('opdev')
            if symlink:
                entry.type = tarfile.SYMTYPE
                entry.linkname = '/etc/passwd'
                archive.addfile(entry)
            else:
                entry.size = len(payload)
                entry.mode = 0o700
                archive.addfile(entry, io.BytesIO(payload))

    def run_script(self, *args, ok=True):
        result = subprocess.run(['sh', str(self.plugin / 'scripts/runtime.sh'), *args], env=self.env,
                                capture_output=True, text=True, timeout=15, cwd=self.root)
        self.assertEqual(result.returncode == 0, ok, result.stdout + result.stderr)
        return result

    def assert_clean_failure(self):
        self.assertFalse(list(self.data.rglob('opdev')))
        self.assertFalse(list(self.data.rglob('.install.*')))
        self.assertFalse(list(self.data.rglob('*.lock')))

    def test_install_repeat_and_argument_forwarding(self):
        binary = Path(self.run_script('--install').stdout.strip())
        self.assertTrue(binary.is_file())
        self.assertTrue(str(binary).startswith(str(self.data)))
        downloads = self.log.read_text()
        self.assertEqual(self.run_script('--path').stdout.strip(), str(binary))
        self.assertEqual(self.run_script('--install').stdout.strip(), str(binary))
        self.assertEqual(self.log.read_text(), downloads)
        result = self.run_script('--run', 'check', 'argument with spaces')
        self.assertEqual(result.stdout, 'check\nargument with spaces\n')
        self.assertEqual(self.native_log.read_text().splitlines()[0], 'verify')

    def test_path_is_read_only_when_missing(self):
        self.run_script('--path', ok=False)
        self.assertFalse(self.data.exists())
        self.assertFalse(self.log.exists())

    def test_verifier_digest_failure_never_executes(self):
        self.verifier.write_text('corrupted')
        self.run_script('--install', ok=False)
        self.assertFalse(self.native_log.exists())
        self.assert_clean_failure()

    def test_bad_signature_never_executes_cli_and_can_retry(self):
        (self.fixture / 'bundle').write_text('invalid')
        self.run_script('--install', ok=False)
        self.assertEqual(self.native_log.read_text(), 'verify\n')
        self.assert_clean_failure()
        (self.fixture / 'bundle').write_text('valid')
        self.run_script('--install')

    def test_wrong_version_and_incompatibility_rejected(self):
        for version, compatible in [('9.9.9', True), ('0.1.1', False)]:
            with self.subTest(version=version, compatible=compatible):
                self.archive(version, compatible)
                self.run_script('--install', ok=False)
                self.assert_clean_failure()

    def test_symlink_payload_rejected(self):
        self.archive(symlink=True)
        self.run_script('--install', ok=False)
        self.assert_clean_failure()

    def test_damaged_cached_runtime_is_not_executed_or_replaced(self):
        binary = Path(self.run_script('--install').stdout.strip())
        binary.write_text('corrupted')
        before = self.native_log.read_text()
        downloads = self.log.read_text()
        self.run_script('--path', ok=False)
        self.run_script('--run', 'version', ok=False)
        self.run_script('--install', ok=False)
        self.assertEqual(binary.read_text(), 'corrupted')
        self.assertEqual(self.native_log.read_text(), before)
        self.assertEqual(self.log.read_text(), downloads)

    def test_prompt_hook_does_not_install_or_fall_back_on_damage(self):
        binary = Path(self.run_script('--install').stdout.strip())
        binary.write_text('damaged')
        before = self.log.read_text()
        env = dict(self.env, CLAUDE_PLUGIN_ROOT=str(self.plugin))
        result = subprocess.run(['bash', str(self.plugin / 'hooks/opdev-context.sh')],
                                env=env, cwd=self.root, capture_output=True, text=True, check=True)
        context = json.loads(result.stdout)['hookSpecificOutput']['additionalContext']
        self.assertIn('could not be validated', context)
        self.assertEqual(self.log.read_text(), before)

    def test_concurrent_install_lock_is_preserved(self):
        # A successful installation tells us the platform-specific destination.
        binary = Path(self.run_script('--install').stdout.strip())
        shutil.rmtree(binary.parent)
        lock = Path(str(binary.parent) + '.lock')
        lock.mkdir()
        self.run_script('--install', ok=False)
        self.assertTrue(lock.is_dir())


if __name__ == '__main__':
    unittest.main()
