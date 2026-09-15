"""Hook state routing and non-execution tests; not an agent intent benchmark."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
BASH = shutil.which('bash')
if os.name == 'nt' and Path('C:/Program Files/Git/bin/bash.exe').exists():
    BASH = 'C:/Program Files/Git/bin/bash.exe'


@unittest.skipUnless(BASH, 'bash is required for the Claude hook')
class HookTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='opdev hook ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.git('init', '--quiet')
        self.plugin = self.root / 'plugin'
        shutil.copytree(ROOT / 'plugins/opdev', self.plugin)
        self.marker = self.root / 'runtime-called'
        # Any runtime probe is a bug, even if it would return missing/damaged.
        (self.plugin / 'scripts/runtime.sh').write_text(
            '#!/bin/sh\ntouch "$PROBE_MARKER"\nexit 2\n')
        (self.plugin / 'scripts/runtime.ps1').write_text(
            'New-Item -ItemType File -Path $env:PROBE_MARKER; exit 2\n')
        self.env = dict(os.environ, CLAUDE_PLUGIN_ROOT=self.plugin.as_posix(),
                        PROBE_MARKER=self.marker.as_posix(),
                        OPDEV_DATA_DIR=(self.root / 'runtime-data').as_posix())

    def git(self, *args):
        subprocess.run(['git', '-C', str(self.repo), *args], check=True,
                       capture_output=True)

    def hook(self, directory, prompt='Pull down the most recent changes to the courses repo and start the dev server'):
        result = subprocess.run([BASH, (self.plugin / 'hooks/opdev-context.sh').as_posix()],
                                env=dict(self.env, CLAUDE_PROJECT_DIR=directory.as_posix()),
                                cwd=self.root, input=json.dumps({'prompt': prompt}),
                                text=True, capture_output=True, check=True, timeout=15)
        self.assertFalse(self.marker.exists(), 'hook probed runtime before consent')
        self.assertFalse((self.root / 'runtime-data').exists())
        return json.loads(result.stdout)['hookSpecificOutput']['additionalContext']

    def configure(self, directory):
        (directory / '.opdev').mkdir()
        # Hook must defer malformed contract validation, not silently ignore it.
        (directory / '.opdev/project.yaml').write_text('invalid: [')

    def test_uninitialized_prompts_do_not_execute_runtime(self):
        for prompt in ['Pull down the most recent changes to the courses repo and start the dev server',
                       'Check git status', 'Add a feature', 'Convert this repo to OpDev',
                       'Set up OpDev', 'Tell me a joke']:
            with self.subTest(prompt=prompt):
                self.assertIn('no OpDev contract', self.hook(self.repo, prompt))
                self.assertFalse((self.repo / '.opdev').exists())

    def test_nested_configured_project_defers_runtime_and_validation(self):
        self.configure(self.repo)
        nested = self.repo / 'nested'
        nested.mkdir()
        self.assertIn('has an OpDev contract', self.hook(nested))

    def test_target_repository_does_not_inherit_callers_contract(self):
        self.configure(self.root)
        self.assertIn('no OpDev contract', self.hook(self.repo))

    def test_worktree_uses_its_own_contract(self):
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 'commit', '--allow-empty', '-m', 'fixture')
        worktree = self.root / 'worktree'
        self.git('worktree', 'add', '-b', 'fixture', str(worktree))
        self.configure(self.repo)
        self.assertIn('no OpDev contract', self.hook(worktree))
        self.configure(worktree)
        nested = worktree / 'nested'
        nested.mkdir()
        self.assertIn('has an OpDev contract', self.hook(nested))


if __name__ == '__main__':
    unittest.main()
