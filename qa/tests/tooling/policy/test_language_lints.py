from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))

from quality import language_lints  # noqa: E402

PINS = {"KASKOLD_RUFF_VERSION": "1.2.3", "KASKOLD_MYPY_VERSION": "4.5.6", "KASKOLD_NODE_VERSION": "24.1.0"}


class TypedModuleTests(unittest.TestCase):
    def test_module_without_the_directive_is_reported_by_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            typed = root / "typed/deep"
            typed.mkdir(parents=True)
            (typed / "checked.js").write_text("// @ts-check\nexport const a = 1;\n", encoding="utf-8")
            (typed / "unchecked.js").write_text("export const b = 2;\n// @ts-check\n", encoding="utf-8")
            self.assertEqual(language_lints.untyped_modules(root, ("typed",)), ["typed/deep/unchecked.js"])

    def test_every_module_below_the_committed_typed_roots_opts_in(self) -> None:
        self.assertEqual(language_lints.untyped_modules(ROOT), [])


class PythonToolInstallTests(unittest.TestCase):
    def test_pinned_tools_install_once_per_pin_set(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def install(command: list[str], **_kwargs: object) -> None:
                Path(command[command.index("--target") + 1]).mkdir(parents=True)

            with mock.patch.object(language_lints, "run", side_effect=install) as run:
                tool_root = language_lints.install_python_tools(root, PINS)
                language_lints.install_python_tools(root, PINS)
            self.assertEqual(run.call_count, 1)
            self.assertEqual(run.call_args.args[0][-2:], ["ruff==1.2.3", "mypy==4.5.6"])
            self.assertEqual(json.loads((tool_root / "requirements.json").read_text()), ["ruff==1.2.3", "mypy==4.5.6"])

            with mock.patch.object(language_lints, "run", side_effect=install) as run:
                language_lints.install_python_tools(root, {**PINS, "KASKOLD_MYPY_VERSION": "9.9.9"})
            self.assertEqual(run.call_args.args[0][-1], "mypy==9.9.9")


class GateOutcomeTests(unittest.TestCase):
    def _main_with(self, **patches: object) -> int:
        which = patches.pop("which", lambda name: f"/bin/{name}")
        with (
            mock.patch.object(language_lints.shutil, "which", side_effect=which),
            mock.patch.object(language_lints, "untyped_modules", patches.pop("untyped", lambda _root: [])),
            mock.patch.object(language_lints, "load_toolchains", lambda: PINS),
            mock.patch.object(language_lints, "node_major_mismatch", patches.pop("node_mismatch", lambda _node, _pins: None)),
            mock.patch.object(language_lints, "install_python_tools", lambda _root, _pins: Path("py")),
            mock.patch.object(language_lints, "install_javascript_tools", lambda _root, _npm: Path("js")),
            mock.patch.object(language_lints, "run_python_lints", patches.pop("python", lambda _tools: None)),
            mock.patch.object(language_lints, "run_javascript_lints", patches.pop("javascript", lambda _tools, _node: None)),
            mock.patch("builtins.print"),
        ):
            return language_lints.main()

    def test_clean_repository_passes(self) -> None:
        self.assertEqual(self._main_with(), 0)

    def test_any_failing_tool_fails_the_gate(self) -> None:
        def findings(*_args: object) -> None:
            raise subprocess.CalledProcessError(1, ["tool"])

        self.assertEqual(self._main_with(python=findings), 1)
        self.assertEqual(self._main_with(javascript=findings), 1)

    def test_untyped_module_fails_before_any_tool_runs(self) -> None:
        def must_not_run(*_args: object) -> None:
            self.fail("tools ran although a typed module had opted out")

        self.assertEqual(self._main_with(untyped=lambda _root: ["a.js"], python=must_not_run), 1)

    def test_wrong_node_major_fails_before_any_tool_runs(self) -> None:
        def must_not_run(*_args: object) -> None:
            self.fail("tools ran on an unpinned Node.js")

        self.assertEqual(self._main_with(node_mismatch=lambda _node, _pins: "found v18", python=must_not_run), 1)

    def test_node_major_comparison_ignores_minor_and_patch(self) -> None:
        for version, mismatch in (("v24.19.0\n", False), ("v18.19.1\n", True), ("v2.4.0\n", True)):
            completed = subprocess.CompletedProcess(["node"], 0, stdout=version)
            with mock.patch.object(language_lints.subprocess, "run", return_value=completed):
                self.assertEqual(language_lints.node_major_mismatch("node", PINS) is not None, mismatch, version)

    def test_missing_node_fails_rather_than_skipping(self) -> None:
        self.assertEqual(self._main_with(which=lambda name: None if name == "node" else f"/bin/{name}"), 1)


class RuleCanaryTests(unittest.TestCase):
    def _rejection(self, returncode: int, stdout: str) -> object:
        completed = subprocess.CompletedProcess(["tool"], returncode, stdout=stdout, stderr="")
        with mock.patch.object(language_lints.subprocess, "run", return_value=completed), mock.patch("builtins.print"):
            return language_lints.require_rejection(["tool"], "source", "C901")

    def test_a_tool_that_reports_the_rule_passes(self) -> None:
        self.assertIsNone(self._rejection(1, "canary.py:1:5: C901 `canary` is too complex"))

    def test_a_tool_that_accepts_the_canary_fails_the_gate(self) -> None:
        with self.assertRaises(subprocess.CalledProcessError):
            self._rejection(0, "All checks passed!")

    def test_a_finding_from_some_other_rule_is_not_enough(self) -> None:
        with self.assertRaises(subprocess.CalledProcessError):
            self._rejection(1, "canary.py:1:5: F401 unused import")

    def test_a_crashed_tool_is_not_a_rejection(self) -> None:
        with self.assertRaises(subprocess.CalledProcessError):
            self._rejection(2, "C901 mentioned in a usage error")

    def test_canaries_exceed_the_complexity_limit_of_twelve(self) -> None:
        self.assertEqual(language_lints.PYTHON_TOO_COMPLEX.count("if value"), 13)
        self.assertEqual(language_lints.JAVASCRIPT_TOO_COMPLEX.count("if (value"), 13)


if __name__ == "__main__":
    unittest.main()
