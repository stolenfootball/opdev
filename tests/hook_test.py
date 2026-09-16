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

    def test_design_only_git_repository_is_unchanged_by_detection(self):
        design = self.repo / 'tmp.md'
        content = '# Design\nBuild a small local checklist CLI.\n'
        design.write_text(content)
        before = self.git_status()
        self.assertIn('no OpDev contract', self.hook(
            self.repo, 'I want to implement the design in tmp.md in this repo'))
        self.assertEqual(design.read_text(), content)
        self.assertEqual(self.git_status(), before)
        self.assertEqual({p.name for p in self.repo.iterdir()}, {'.git', 'tmp.md'})

    def git_status(self):
        return subprocess.run(['git', '-C', str(self.repo), 'status', '--porcelain'],
                              check=True, capture_output=True, text=True).stdout

    def test_design_only_folder_needs_neither_git_nor_runtime(self):
        folder = self.root / 'new project'
        folder.mkdir()
        design = folder / 'design.txt'
        design.write_text('Build a small local checklist CLI.\n')
        before = design.read_bytes()
        self.assertIn('no OpDev contract', self.hook(
            folder, 'Implement the design in design.txt'))
        self.assertEqual(design.read_bytes(), before)
        self.assertEqual({p.name for p in folder.iterdir()}, {'design.txt'})

    def test_hook_routes_state_without_keyword_classification_or_consent_memory(self):
        # Identical context leaves intent/consent to the agent, not a shell keyword
        # classifier. Repeated hooks must not write an offer/adoption marker.
        prompts = ['Implement tmp.md', 'No, do not use OpDev',
                   'Use Python for the first slice', 'Yes, use OpDev',
                   'Check git status', 'Explain the word development']
        outputs = [self.hook(self.repo, prompt) for prompt in prompts]
        self.assertEqual(len(set(outputs)), 1)
        self.assertEqual({p.name for p in self.repo.iterdir()}, {'.git'})

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
