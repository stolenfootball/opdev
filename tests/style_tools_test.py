"""Offline coverage of tool routing and pinned executable extraction."""

import importlib.util
import io
from pathlib import Path
import tarfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / (name + ".py"))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


class StyleTests(unittest.TestCase):
    def test_tools_cover_maintained_languages_without_benchmark_mutation(self):
        check = module("check_scripts")
        for system in ("Windows", "Linux"):
            for mode in ("format", "lint"):
                commands = check.commands(mode, system)
                self.assertEqual(len(commands), 3 if system == "Windows" else 2)
                self.assertEqual(commands[0][-3:], ["scripts", "tests", "examples"])
                joined = " ".join(" ".join(argv) for argv in commands)
                self.assertNotIn("benchmarks", joined)
                self.assertNotIn("--fix", joined)
                self.assertNotIn(" -w ", joined)
                self.assertIn("plugins/opdev/scripts/runtime.sh", joined.replace("\\", "/"))
                self.assertIn("release/installers/verify.sh", joined.replace("\\", "/"))
                self.assertEqual("check_powershell.ps1" in joined, system == "Windows")

    def test_archive_extraction_selects_one_regular_executable_not_archive_paths(self):
        setup = module("setup_style")
        raw = io.BytesIO()
        with zipfile.ZipFile(raw, "w") as archive:
            archive.writestr("package/tool.exe", b"tool")
            archive.writestr("../../unrelated", b"ignored")
        self.assertEqual(setup.executable_bytes(raw.getvalue(), "tool.zip", "tool.exe"), b"tool")
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w:gz") as archive:
            entry = tarfile.TarInfo("tool")
            entry.type = tarfile.SYMTYPE
            entry.linkname = "elsewhere"
            archive.addfile(entry)
        with self.assertRaises(ValueError):
            setup.executable_bytes(raw.getvalue(), "tool.tar.gz", "tool")
        with self.assertRaises(zipfile.BadZipFile):
            setup.executable_bytes(b"", "malformed.zip", "tool")
        raw = io.BytesIO()
        with zipfile.ZipFile(raw, "w") as archive:
            archive.writestr("not-the-tool", b"unrelated")
        with self.assertRaises(ValueError):
            setup.executable_bytes(raw.getvalue(), "missing.zip", "tool")


if __name__ == "__main__":
    unittest.main()
