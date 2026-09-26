#!/usr/bin/env python3
"""Regression contracts for the public make test / make qa orchestration."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
if str(ROOT / "qa/checks") not in sys.path:
    sys.path.insert(0, str(ROOT / "qa/checks"))
from toolchains import qa_bash_executable  # noqa: E402

CHECKER_PATH = ROOT / "qa/checks/workspace/check_qa_orchestration.py"
CATALOG = ROOT / "qa/config/run_all_steps.tsv"
LINUX_RUN_ALL = "qa/linux/run-all.sh"


def run_linux_qa(*args: str) -> str:
    """Run the Linux QA facade portably from either POSIX or Git Bash."""
    result = subprocess.run(
        [qa_bash_executable(), LINUX_RUN_ALL, *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            "Linux QA facade failed "
            f"with exit {result.returncode} for args {args!r}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )
    return result.stdout


def load_checker():
    spec = importlib.util.spec_from_file_location("qa_orchestration_contract", CHECKER_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def rows() -> list[list[str]]:
    return [
        line.split("\t", 4)
        for line in CATALOG.read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]


class QaOrchestrationCatalogTests(unittest.TestCase):
    def test_repository_has_no_orphaned_registered_entrypoints(self) -> None:
        checker = load_checker()
        self.assertEqual(checker.check(), [])

    def test_orphaned_entrypoint_is_a_hard_failure(self) -> None:
        checker = load_checker()
        discovered = checker.discover_entrypoints()
        with mock.patch.object(
            checker,
            "discover_entrypoints",
            return_value=set(discovered) | {"qa/checks/example_orphan.py"},
        ):
            errors = checker.check()
        self.assertTrue(any("orphaned registered test/check entrypoints" in error for error in errors))
        self.assertTrue(any("qa/checks/example_orphan.py" in error for error in errors))

    def test_crap_then_core_ci_precede_the_contiguous_make_test_catalog(self) -> None:
        catalog = rows()
        self.assertEqual(catalog[0][0], "qa")
        self.assertEqual(catalog[0][3], "preflight.crap-check")
        self.assertEqual(catalog[1][0], "qa")
        self.assertEqual(catalog[1][3], "preflight.core-ci")
        test_rows = [row for row in catalog if row[0] == "test"]
        self.assertTrue(test_rows)
        first = catalog.index(test_rows[0])
        last = catalog.index(test_rows[-1])
        self.assertEqual(first, 2)
        self.assertEqual(catalog[first:last + 1], test_rows)
        configured = [
            line.strip()
            for line in (ROOT / "qa/config/run_all_test_steps.txt").read_text().splitlines()
            if line.strip() and not line.startswith("#")
        ]
        self.assertEqual(configured, [row[3] for row in test_rows])

    def test_cross_platform_linux_qa_invocations_use_repo_relative_posix_paths(self) -> None:
        self.assertEqual(LINUX_RUN_ALL, "qa/linux/run-all.sh")
        self.assertFalse(Path(LINUX_RUN_ALL).is_absolute())
        self.assertNotIn("\\", LINUX_RUN_ALL)
        for relative in (
            "qa/tests/tooling/test_qa_orchestration_catalog.py",
            "qa/tests/tooling/test_platform_script_wrappers.py",
        ):
            source = (ROOT / relative).read_text(encoding="utf-8")
            bad = 'str(ROOT / ' + '"qa/linux/run-all.sh")'
            self.assertNotIn(bad, source)

    def test_make_qa_resume_is_forwarded_to_the_canonical_runner(self) -> None:
        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        helper = (ROOT / "scripts/common/lib/make_tasks.py").read_text(encoding="utf-8")
        public = (ROOT / "scripts/common/lib/make_public.py").read_text(encoding="utf-8")
        self.assertIn('qa "$(FUZZ_PASSES)" "$(STRICT_LOCKFILES)" "$(RESUME_FROM)"', makefile)
        self.assertIn('p.add_argument("resume_from", nargs="?", default="")', helper)
        self.assertIn('args.extend(["--resume-from", resume_from.strip()])', public)

    def test_resumed_full_qa_bootstraps_missing_ephemeral_crap_evidence(self) -> None:
        commands = (
            [qa_bash_executable(), LINUX_RUN_ALL, "--profile", "full", "--dry-run", "--resume-from", "coverage.critical-branch-targets", "--skip-fuzz", "--skip-qemu"],
            [sys.executable, str(ROOT / "qa/windows/runner/run_all.py"), "--profile", "full", "--dry-run", "--resume-from", "coverage.critical-branch-targets", "--skip-fuzz", "--skip-qemu"],
        )
        for command in commands:
            output = run_linux_qa(*command[2:]) if command[0] == qa_bash_executable() else subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True).stdout
            self.assertIn("[resume prerequisite] Fresh CRAP/coverage artifacts are required", output)
            self.assertIn("Regenerating preflight.crap-check only", output)
            self.assertIn("pinned-branch-coverage", output)
            self.assertIn("[coverage.critical-branch-targets]", output)

    def test_linux_and_windows_full_qa_start_with_crap_then_core_ci(self) -> None:
        commands = (
            [qa_bash_executable(), LINUX_RUN_ALL, "--profile", "full", "--dry-run", "--skip-fuzz", "--skip-qemu"],
            [sys.executable, str(ROOT / "qa/windows/runner/run_all.py"), "--profile", "full", "--dry-run", "--skip-fuzz", "--skip-qemu"],
        )
        for command in commands:
            output = run_linux_qa(*command[2:]) if command[0] == qa_bash_executable() else subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True).stdout
            selected = [line[1:line.index("]")] for line in output.splitlines() if line.startswith("[") and "]" in line]
            self.assertGreaterEqual(len(selected), 2)
            self.assertEqual(selected[:2], ["preflight.crap-check", "preflight.core-ci"])
            self.assertIn("target/qa/core-ci/core-ci.log", output.replace("\\", "/"))
            self.assertIn("cargo clippy --workspace --all-targets --locked -- -D warnings", output)
            self.assertIn("make test STRICT_LOCKFILES=1", output)
            self.assertNotIn("[unit.shared-signer]", output)


    def test_core_ci_reuses_complete_local_pinned_toolchain_before_installing(self) -> None:
        linux = (ROOT / "qa/linux/runner/commands.sh").read_text(encoding="utf-8")
        windows = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")

        self.assertIn('core_ci_toolchain_ready "$KASKOLD_STABLE_RUST"', linux)
        self.assertIn('rustup run "$toolchain" rustc --version', linux)
        self.assertIn('rustup run "$toolchain" rustfmt --version', linux)
        self.assertIn('rustup run "$toolchain" cargo clippy --version', linux)
        self.assertLess(
            linux.index('core_ci_toolchain_ready "$KASKOLD_STABLE_RUST"'),
            linux.index('rustup toolchain install "$KASKOLD_STABLE_RUST"', linux.index('run_core_ci_gate()')),
        )

        self.assertIn("def core_ci_toolchain_ready(toolchain: str) -> bool:", windows)
        self.assertIn('["rustup", "run", toolchain, "rustc", "--version"]', windows)
        self.assertIn('["rustup", "run", toolchain, "rustfmt", "--version"]', windows)
        self.assertIn('["rustup", "run", toolchain, "cargo", "clippy", "--version"]', windows)
        self.assertIn("if core_ci_toolchain_ready(stable):", windows)
        self.assertIn("Pinned core toolchain and required components are already installed; reusing local installation.", linux)
        self.assertIn("Pinned core toolchain and required components are already installed; reusing local installation.", windows)

    def test_core_ci_final_diff_check_supports_extracted_source_archives(self) -> None:
        linux = (ROOT / "qa/linux/runner/commands.sh").read_text(encoding="utf-8")
        windows = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")

        for source in (linux, windows):
            self.assertIn("archive-baseline.git", source)
            self.assertIn("Extracted source archive detected", source)
            self.assertIn("diff", source)
            self.assertIn("--check", source)

        self.assertIn('git rev-parse --is-inside-work-tree', linux)
        self.assertIn('--git-dir="$archive_git_dir" --work-tree="$ROOT_DIR" add -A', linux)
        self.assertIn('--git-dir="$archive_git_dir" --work-tree="$ROOT_DIR" diff --check', linux)
        self.assertIn('["git", "rev-parse", "--is-inside-work-tree"]', windows)
        self.assertIn('baseline_prefix + ["add", "-A"]', windows)
        self.assertIn('f"--git-dir={archive_git_dir}"', windows)

    def test_make_test_and_make_qa_contain_no_android_ios_or_hil_work(self) -> None:
        catalog = rows()
        test_rows = [row for row in catalog if row[0] == "test"]
        shared_rows = [row for row in catalog if row[0] in {"test", "qa"}]
        self.assertTrue(test_rows)
        self.assertTrue(all(row[2] not in {"companion-ios", "companion-android"} for row in shared_rows))
        test_ids = {row[3] for row in test_rows}
        self.assertTrue(all(row[1] != "hardware" for row in test_rows))
        self.assertFalse(any("hil" in step_id.lower() for step_id in test_ids))
        ownership = json.loads((ROOT / "qa/config/test_entrypoints.json").read_text())
        mobile_prefixes = ("qa/checks/ios/", "qa/checks/android/", "apps/kaskold-companion-ios/", "apps/kaskold-companion-android/")
        leaked = [
            entry["path"] for entry in ownership["entrypoints"]
            if ({"make test", "make qa"} & set(entry["commands"]))
            and entry["path"].startswith(mobile_prefixes)
        ]
        self.assertEqual(leaked, [])
        for entry in ownership["entrypoints"]:
            name = Path(entry["path"]).name
            if entry["path"].startswith(("qa/checks/ios/", "apps/kaskold-companion-ios/")) or name.startswith("test_ios_"):
                self.assertEqual(entry["commands"], ["make ios-qa"], entry["path"])
            if entry["path"].startswith(("qa/checks/android/", "apps/kaskold-companion-android/")) or name.startswith("test_android_"):
                self.assertEqual(entry["commands"], ["make android-qa"], entry["path"])
            if name.startswith("test_mobile_"):
                self.assertEqual(entry["commands"], ["make android-qa", "make ios-qa"], entry["path"])
        hardware_leaks = [
            entry["path"] for entry in ownership["entrypoints"]
            if entry.get("role") == "hardware-runner" and ({"make test", "make qa"} & set(entry["commands"]))
        ]
        self.assertEqual(hardware_leaks, [])

    def test_qa_excludes_physical_hardware_and_orders_interactive_before_campaigns(self) -> None:
        catalog = rows()
        ids = [row[3] for row in catalog]
        qa_ids = [row[3] for row in catalog if row[0] in {"test", "qa"}]
        hardware_ids = [row[3] for row in catalog if row[0] == "hardware"]
        self.assertTrue(hardware_ids)
        self.assertTrue(set(qa_ids).isdisjoint(hardware_ids))
        for interactive in ("integration.real-node", "integration.funded-testnet-e2e"):
            self.assertLess(ids.index(interactive), ids.index("mutation.repository-security-fresh"))
        self.assertLess(ids.index("bench.shared-signer-protocol-throughput"), ids.index("mutation.repository-security-fresh"))
        self.assertLess(ids.index("mutation.repository-security-fresh"), ids.index("mutation.repository-crypto-certification"))
        self.assertLess(ids.index("mutation.repository-crypto-certification"), ids.index("fuzz.repository-security-targets"))
        self.assertEqual(ids[-1], "hardware.signer-firmware-device")
        self.assertEqual(qa_ids[-1], "fuzz.repository-security-targets")

    def test_linux_and_windows_use_the_same_catalog_and_stable_ids(self) -> None:
        linux = run_linux_qa("--list")
        windows = subprocess.run(
            [sys.executable, str(ROOT / "qa/windows/runner/run_all.py"), "--list"],
            cwd=ROOT, text=True, capture_output=True, check=True,
        ).stdout
        expected = [row[3] for row in rows()]
        def ids(text: str) -> list[str]:
            return [line.split()[0] for line in text.splitlines()[2:] if line.strip()]
        self.assertEqual(ids(linux), expected)
        self.assertEqual(ids(windows), expected)

    def test_repository_python_qa_treats_module_level_skip_as_skip(self) -> None:
        runner_path = ROOT / "qa/checks/workspace/run_repository_python_qa.py"
        spec = importlib.util.spec_from_file_location("repository_python_qa_contract", runner_path)
        assert spec and spec.loader
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)

        with tempfile.TemporaryDirectory() as temporary:
            skipped_module = Path(temporary) / "test_platform_only.py"
            skipped_module.write_text(
                "import unittest\nraise unittest.SkipTest('platform-only test module')\n",
                encoding="utf-8",
            )
            suite = runner.load_file(unittest.TestLoader(), skipped_module, 0)

        result = unittest.TestResult()
        suite.run(result)
        self.assertTrue(result.wasSuccessful())
        self.assertEqual(len(result.skipped), 1)
        self.assertEqual(result.skipped[0][1], "platform-only test module")

    def test_repository_python_qa_persists_and_echoes_detailed_failures(self) -> None:
        source = (ROOT / "qa/checks/workspace/run_repository_python_qa.py").read_text(encoding="utf-8")
        self.assertIn('ROOT / "target/qa/repository-python-qa"', source)
        self.assertIn('TeeStream(sys.stdout, log_file)', source)
        self.assertIn('stream=stream, verbosity=2', source)
        self.assertIn('FAILED TEST SUMMARY', source)
        self.assertIn('Full log:', source)

    def test_repository_python_qa_windows_shared_tools_prefer_git_over_wsl(self) -> None:
        runner_path = ROOT / "qa/checks/workspace/run_repository_python_qa.py"
        spec = importlib.util.spec_from_file_location("repository_python_qa_tool_bootstrap", runner_path)
        assert spec and spec.loader
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "Git"
            (root / "bin").mkdir(parents=True)
            (root / "usr/bin").mkdir(parents=True)
            (root / "bin/bash.exe").write_bytes(b"")
            (root / "usr/bin/openssl.exe").write_bytes(b"")
            environment = {"PATH": "SYSTEM32"}
            runner._prepend_windows_test_tools(root, environment)

        entries = environment["PATH"].split(runner.os.pathsep)
        self.assertEqual(entries[0], str(root / "bin"))
        self.assertEqual(entries[1], str(root / "usr/bin"))
        self.assertEqual(entries[2], str(root / "mingw64/bin"))
        self.assertEqual(entries[3], "SYSTEM32")

    def test_repository_python_qa_windows_shared_tools_self_provision_git(self) -> None:
        source = (ROOT / "qa/checks/workspace/run_repository_python_qa.py").read_text(encoding="utf-8")
        self.assertIn('"Git.Git"', source)
        self.assertIn('root / "bin/bash.exe"', source)
        self.assertIn('root / "usr/bin/openssl.exe"', source)
        self.assertIn('root / "mingw64/bin/openssl.exe"', source)
        self.assertIn('ensure_windows_shared_test_tools()', source)
        self.assertLess(source.index('ensure_windows_shared_test_tools()'), source.index('suite.addTests(load_file'))

    def test_repository_python_qa_windows_shared_tools_install_when_missing(self) -> None:
        runner_path = ROOT / "qa/checks/workspace/run_repository_python_qa.py"
        spec = importlib.util.spec_from_file_location("repository_python_qa_tool_install", runner_path)
        assert spec and spec.loader
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "Git"
            (root / "bin").mkdir(parents=True)
            (root / "usr/bin").mkdir(parents=True)
            (root / "bin/bash.exe").write_bytes(b"")
            (root / "usr/bin/openssl.exe").write_bytes(b"")
            probe = subprocess.CompletedProcess(["tool"], 0, "ok", "")
            with (
                mock.patch.object(runner, "_find_git_for_windows_root", side_effect=[None, root]),
                mock.patch.object(runner, "_install_git_for_windows") as install,
                mock.patch.object(runner.subprocess, "run", return_value=probe),
                mock.patch.dict(runner.os.environ, {"PATH": "SYSTEM32"}, clear=False),
            ):
                runner.ensure_windows_shared_test_tools("win32")
                install.assert_called_once_with()
                entries = runner.os.environ["PATH"].split(runner.os.pathsep)
                self.assertEqual(entries[0], str(root / "bin"))
                self.assertEqual(entries[1], str(root / "usr/bin"))
                self.assertEqual(
                    runner.os.environ["KASKOLD_QA_BASH"],
                    str((root / "bin/bash.exe").resolve()),
                )
                self.assertEqual(
                    runner.os.environ["KASKOLD_QA_OPENSSL"],
                    str((root / "usr/bin/openssl.exe").resolve()),
                )

    def test_windows_runner_prints_exact_resume_command_and_python_qa_log(self) -> None:
        source = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")
        self.assertIn('Resume with: make qa RESUME_FROM={i} FUZZ_PASSES={ns.fuzz_passes}', source)
        self.assertIn('target/qa/repository-python-qa/shared.log', source)
        self.assertIn('os.environ["PYTHONUNBUFFERED"] = "1"', source)

    def test_windows_reproducible_build_module_skip_is_not_an_import_failure(self) -> None:
        runner_path = ROOT / "qa/checks/workspace/run_repository_python_qa.py"
        spec = importlib.util.spec_from_file_location("repository_python_qa_windows_contract", runner_path)
        assert spec and spec.loader
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)

        reproducible_tests = ROOT / "qa/tests/tooling/test_reproducible_build_runner.py"
        with mock.patch("os.name", "nt"):
            suite = runner.load_file(unittest.TestLoader(), reproducible_tests, 0)

        result = unittest.TestResult()
        suite.run(result)
        self.assertTrue(result.wasSuccessful())
        self.assertEqual(len(result.skipped), 1)
        self.assertEqual(
            result.skipped[0][1],
            "Linux reproducible-build runner tests are POSIX-specific",
        )

    def test_platform_ineligible_mobile_work_is_explicitly_skipped(self) -> None:
        ios = (ROOT / "qa/checks/ios/run_xcode_application_tests.py").read_text()
        android = (ROOT / "qa/checks/android/run_instrumentation_tests.py").read_text()
        ios_mutation = (ROOT / "qa/checks/ios/run_mutation_tests.py").read_text()
        android_mutation = (ROOT / "qa/checks/android/run_mutation_tests.py").read_text()
        self.assertIn("SKIP: iOS application XCTest/XCUITest requires", ios)
        self.assertIn("SKIP = 77", android)
        self.assertIn("SKIP = 77", ios_mutation)
        self.assertIn("SKIP = 77", android_mutation)

    def test_make_release_builds_candidate_and_readiness_is_explicit_without_qa_replay(self) -> None:
        source = (ROOT / "scripts/common/lib/make_public.py").read_text(encoding="utf-8")
        release = source[source.index("def release_build"):]
        self.assertIn('platform("reproducible-build"', release)
        self.assertNotIn('platform("release-readiness"', release)
        self.assertNotIn('platform("run-all"', release.split("def ", 1)[0] if "def " in release else release)
        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        self.assertIn("release-readiness:", makefile)
        self.assertIn("entrypoint release-readiness", makefile)
        help_text = (ROOT / "scripts/common/lib/make_help.txt").read_text(encoding="utf-8")
        self.assertIn("make test -> make qa -> make android-qa -> make ios-qa -> make test-hardware -> make workflow-e2e -> make workflow-hil -> make release -> make release-readiness", help_text)

    def test_ownership_manifest_uses_only_public_workflow_commands(self) -> None:
        document = json.loads((ROOT / "qa/config/test_entrypoints.json").read_text())
        allowed = {
            "make test", "make qa", "make android-qa", "make ios-qa",
            "make test-hardware", "make workflow-e2e", "make workflow-hil",
            "make release", "make release-readiness",
        }
        for entry in document["entrypoints"]:
            self.assertTrue(entry["commands"])
            self.assertTrue(set(entry["commands"]).issubset(allowed), entry["path"])


    def test_windows_qemu_failure_replays_persisted_diagnostics(self) -> None:
        source = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")
        self.assertIn("def run_qemu_emulation", source)
        self.assertIn("windows-qemu-master.log", source)
        self.assertIn("windows-qemu-powershell.log", source)
        self.assertIn("Replaying every persisted Windows QEMU diagnostic log now", source)
        self.assertIn('elif step_id=="emulation.signer-firmware-qemu": run_qemu_emulation', source)

if __name__ == "__main__":
    unittest.main()
