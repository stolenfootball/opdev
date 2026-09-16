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


class CheckoutTests(unittest.TestCase):
    def test_windows_checkout_keeps_plugin_shell_inputs_lf(self):
        # Exercise Git checkout conversion, not a newline-normalizing copy.
        with tempfile.TemporaryDirectory(prefix='opdev checkout ') as temporary:
            root = Path(temporary)
            source = root / 'source'
            source.mkdir()
            plugin = source / 'plugins/opdev'
            shutil.copytree(ROOT / 'plugins/opdev', plugin)
            shutil.copy2(ROOT / '.gitattributes', source / '.gitattributes')
            def git(*args):
                return subprocess.run(['git', '-C', str(source), *args], check=True,
                                      capture_output=True, text=True)
            git('init', '--quiet')
            git('config', 'core.autocrlf', 'true')
            git('add', '.')
            checkout = root / 'checkout'
            checkout.mkdir()
            git('checkout-index', '--all', '--prefix=' + checkout.as_posix() + '/')
            installed = checkout / 'plugins/opdev'
            for path in [*installed.rglob('*.sh'), installed / 'runtime.lock']:
                self.assertNotIn(b'\r', path.read_bytes(), str(path))
            if os.name != 'nt':
                result = subprocess.run(['sh', str(installed / 'scripts/runtime.sh'), '--path'],
                                        env=dict(os.environ, OPDEV_DATA_DIR=str(root / 'empty-runtime')),
                                        capture_output=True, text=True, timeout=15)
                self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
                self.assertFalse((root / 'empty-runtime').exists())


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
                        EXPECTED_IDENTITY='https://gitlab.com/stolenfootball-tools/opinionateddevelopment//.gitlab-ci.yml@refs/tags/v0.1.1',
                        PATH=str(self.bin) + os.pathsep + os.environ['PATH'])
        self.verifier = self.fixture / 'verifier'
        self.verifier.write_text('''#!/bin/sh
set -eu
printf 'verify\\n' >> "$NATIVE_LOG"
[ "$1" = verify-blob ]
[ "$3" = --bundle ]
[ "$5" = --certificate-identity ]
[ "$6" = "$EXPECTED_IDENTITY" ]
[ "$7" = --certificate-oidc-issuer ]
[ "$8" = https://gitlab.com ]
[ "$(cat "$4")" = valid ]
''')
        digest = hashlib.sha256(self.verifier.read_bytes()).hexdigest()
        lock = self.plugin / 'runtime.lock'
        lines = lock.read_text().splitlines()
        # Keep historical release coverage independent of the current package pin.
        fields = {'version': '0.1.1', 'tag': 'v0.1.1', 'identity': self.env['EXPECTED_IDENTITY']}
        lines = [f'{line.split()[0]} {fields[line.split()[0]]}'
                 if line and line.split()[0] in fields else line for line in lines]
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
 https://github.com/stolenfootball/opdev/releases/download/v0.2.0/*.sigstore.json) cp "$FIXTURE/bundle" "$output" ;;
 https://github.com/stolenfootball/opdev/releases/download/v0.2.0/*.tar.gz) cp "$FIXTURE/archive" "$output" ;;
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

    def test_current_github_pin_preserves_existing_historical_runtime(self):
        old_binary = Path(self.run_script('--install').stdout.strip())
        old_bytes = old_binary.read_bytes()
        lock = self.plugin / 'runtime.lock'
        identity = 'https://gitlab.com/stolenfootball-tools/opdev//.gitlab-ci.yml@refs/tags/v0.2.0'
        lock.write_text(lock.read_text().replace('0.1.1', '0.2.0').replace(
            'stolenfootball-tools/opinionateddevelopment//', 'stolenfootball-tools/opdev//'))
        self.env['EXPECTED_IDENTITY'] = identity
        self.archive('0.2.0')
        before = self.native_log.read_text()
        (self.fixture / 'bundle').write_text('invalid')
        self.run_script('--install', ok=False)
        self.assertEqual(self.native_log.read_text(), before + 'verify\n')
        self.assertEqual(old_binary.read_bytes(), old_bytes)
        (self.fixture / 'bundle').write_text('valid')
        binary = Path(self.run_script('--install').stdout.strip())
        self.assertNotEqual(binary, old_binary)
        self.assertEqual(old_binary.read_bytes(), old_bytes)
        self.assertIn('https://github.com/stolenfootball/opdev/releases/download/v0.2.0/opdev-0.2.0-', self.log.read_text())
        downloads = self.log.read_text()
        self.assertEqual(self.run_script('--install').stdout.strip(), str(binary))
        self.assertEqual(self.run_script('--path').stdout.strip(), str(binary))
        self.assertEqual(self.log.read_text(), downloads)

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

    def test_prompt_hook_defers_runtime_damage_to_activated_skill(self):
        binary = Path(self.run_script('--install').stdout.strip())
        binary.write_text('damaged')
        before = self.log.read_text()
        env = dict(self.env, CLAUDE_PLUGIN_ROOT=str(self.plugin))
        result = subprocess.run(['bash', str(self.plugin / 'hooks/opdev-context.sh')],
                                env=env, cwd=self.root, capture_output=True, text=True, check=True)
        context = json.loads(result.stdout)['hookSpecificOutput']['additionalContext']
        self.assertIn('no OpDev contract', context)
        self.assertNotIn('could not be validated', context)
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
