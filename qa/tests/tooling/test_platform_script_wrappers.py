#!/usr/bin/env python3
"""Regression coverage for organized native platform script trees."""
from __future__ import annotations

import importlib.util
import io
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
if str(ROOT / "qa/checks") not in sys.path:
    sys.path.insert(0, str(ROOT / "qa/checks"))
from toolchains import qa_bash_executable  # noqa: E402

SCRIPTS = ROOT / "scripts"
LINUX = SCRIPTS / "linux"
WINDOWS = SCRIPTS / "windows"
MAC = SCRIPTS / "mac"
COMMON = SCRIPTS / "common"
MAKEFILE = ROOT / "Makefile"
CATEGORIES = {"build", "install", "lib", "qemu", "quality"}
LINUX_RUN_ALL = "qa/linux/run-all.sh"



def run_linux_qa(*args: str) -> str:
    """Run the Linux QA facade without leaking Windows backslash paths into Git Bash."""
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


PUBLIC = {
    "install/install",
    "quality/run-all",
    "quality/funded-testnet-e2e",
    "quality/pinned-branch-coverage",
    "quality/production-hardening",
    "quality/real-node-integration",
    "quality/release-readiness",
    "quality/security-fuzz",
    "quality/software-assurance",
    "quality/crap",
    "quality/branch-coverage-setup",
    "build/kaskold-companion-web-build",
    "build/kaskold-vault-web-build",
    "build/sdk-build",
    "build/android-studio",
    "build/android-build",
    "build/android-runtime-sync",
    "build/ios-runtime-sync",
    "build/ios-build",
    "build/reproducible-build",
    "build/firmware-build",
    "build/firmware-build-production",
    "build/firmware-owner-build",
    "build/sdk-distribution-check",
    "qemu/setup",
    "qemu/build",
    "qemu/test",
    "qemu/run",
    "qemu/firmware-build",
}


class PlatformScriptWrapperTests(unittest.TestCase):
    def test_windows_native_capture_preserves_nonzero_native_exit_codes(self) -> None:
        common = (WINDOWS / "lib/common.ps1").read_text(encoding="utf-8")
        start = common.index("function Invoke-KasKoldCapture")
        end = common.index("function Remove-KasKoldPath", start)
        capture = common[start:end]
        self.assertIn("$savedPreference = $ErrorActionPreference", capture)
        self.assertIn("$ErrorActionPreference = 'Continue'", capture)
        self.assertIn("$ErrorActionPreference = $savedPreference", capture)
        self.assertIn("$LASTEXITCODE", capture)
        self.assertIn("2>&1 | Out-String", capture)
        self.assertIn("catch [System.Management.Automation.CommandNotFoundException]", capture)
        self.assertIn("$code = 127", capture)

    def test_windows_zero_argument_real_node_facade_does_not_forward_empty_array(self) -> None:
        facade = (WINDOWS / "quality/real-node-integration.ps1").read_text(encoding="utf-8")
        dispatcher = (WINDOWS / "lib/_invoke.ps1").read_text(encoding="utf-8")
        self.assertIn("if (@($args).Count -ne 0)", facade)
        self.assertIn("-Target 'qa/windows/run-real-node-integration.ps1'", facade)
        self.assertNotIn("-CommandArguments", facade)
        self.assertIn("if (@($CommandArguments).Count -eq 0)", dispatcher)
        self.assertIn("-File $targetPath\n", dispatcher)
        self.assertIn("-File $targetPath @CommandArguments", dispatcher)

    def test_windows_real_node_canonical_script_is_truly_zero_argument(self) -> None:
        path = ROOT / "qa/windows/run-real-node-integration.ps1"
        source = path.read_text(encoding="utf-8")
        self.assertNotIn("ValueFromRemainingArguments", source)
        self.assertNotIn("$RemainingArgs", source)
        self.assertIn("if (@($args).Count -ne 0)", source)
        self.assertIn("public-node resolver only; no local-node mode exists", source)

    def test_windows_zero_argument_funded_e2e_facade_does_not_forward_empty_array(self) -> None:
        facade = (WINDOWS / "quality/funded-testnet-e2e.ps1").read_text(encoding="utf-8")
        dispatcher = (WINDOWS / "lib/_invoke.ps1").read_text(encoding="utf-8")
        self.assertIn("if (@($args).Count -ne 0)", facade)
        self.assertIn("-Target 'qa/windows/run-funded-testnet-e2e.ps1'", facade)
        self.assertNotIn("-CommandArguments", facade)
        self.assertIn("if (@($CommandArguments).Count -eq 0)", dispatcher)

    def test_windows_funded_e2e_canonical_script_is_truly_zero_argument(self) -> None:
        path = ROOT / "qa/windows/run-funded-testnet-e2e.ps1"
        source = path.read_text(encoding="utf-8")
        self.assertNotIn("ValueFromRemainingArguments", source)
        self.assertNotIn("$RemainingArgs", source)
        self.assertIn("if (@($args).Count -ne 0)", source)
        self.assertIn("asks for the public Kaspa testnet interactively", source)

    def test_windows_funded_e2e_preserves_interactive_prompts_and_skip_exit_77(self) -> None:
        source = (ROOT / "qa/windows/run-funded-testnet-e2e.ps1").read_text(encoding="utf-8")
        self.assertIn("& $python 'qa/checks/integration/funded_testnet_e2e.py'", source)
        self.assertIn("$LASTEXITCODE", source)
        self.assertIn("@(0,77) -notcontains $status", source)
        self.assertIn("exit $status", source)
        self.assertNotIn("Invoke-KasKoldCommand -Command $python -Arguments @('qa/checks/integration/funded_testnet_e2e.py')", source)
        self.assertNotIn("| Out-Null", source[source.index("$python = Get-KasKoldPython"):])

    def test_make_tasks_forwards_exact_python_to_native_wrappers(self) -> None:
        source = (ROOT / "scripts/common/lib/make_tasks.py").read_text(encoding="utf-8")
        self.assertIn('effective_env["KASKOLD_PYTHON"] = sys.executable', source)
        self.assertIn("effective_env.update(env or {})", source)

    def test_windows_qemu_facade_mirrors_and_persists_native_output(self) -> None:
        source = (ROOT / "scripts/common/lib/make_tasks.py").read_text(encoding="utf-8")
        self.assertIn("def run_streaming_log(", source)
        self.assertIn('entry == "qemu-test"', source)
        self.assertIn('ROOT / "target/qa/qemu/windows-qemu.log"', source)
        self.assertIn("stderr=subprocess.STDOUT", source)
        self.assertIn("sys.stdout.buffer.write(chunk)", source)
        self.assertIn("Detailed QEMU log", source)
        self.assertIn("QEMU child log tail", source)
        self.assertIn("stdin=subprocess.DEVNULL", source)

    def test_windows_qemu_test_facade_is_direct_transcribed_and_zero_argument(self) -> None:
        source = (WINDOWS / "qemu/test.ps1").read_text(encoding="utf-8")
        self.assertIn("if (@($args).Count -ne 0)", source)
        self.assertIn("Start-Transcript", source)
        self.assertIn("windows-qemu-powershell.log", source)
        self.assertIn("Invoke-KasKoldQemuHarness", source)
        self.assertIn("PowerShell script stack", source)
        self.assertNotIn("$runScript", source)

    def test_windows_qemu_run_filters_phantom_null_remaining_arguments(self) -> None:
        source = (WINDOWS / "qemu/run.ps1").read_text(encoding="utf-8")
        self.assertIn("$unsupportedArgs = @($RemainingArgs | Where-Object", source)
        self.assertIn("[string]::IsNullOrEmpty($_)", source)
        self.assertIn("$unsupportedArgs.Count -gt 0", source)
        self.assertIn("$unsupportedArgs[0]", source)
        self.assertNotIn("$RemainingArgs[0]", source)

    def test_windows_qemu_run_uses_shared_harness_and_reports_full_failure_context(self) -> None:
        source = (WINDOWS / "qemu/run.ps1").read_text(encoding="utf-8")
        common = (WINDOWS / "lib/qemu-common.ps1").read_text(encoding="utf-8")
        self.assertIn("Invoke-KasKoldQemuHarness", source)
        self.assertIn("ERROR: QEMU setup/test failed", source)
        self.assertIn("PowerShell script stack", source)
        self.assertIn("function Invoke-KasKoldQemuHarness", common)
        harness = common[common.index("function Invoke-KasKoldQemuHarness"):]
        self.assertIn("& $python @arguments", harness)
        self.assertIn("$status = if ($null -eq $LASTEXITCODE)", harness)
        self.assertIn("$ErrorActionPreference = 'Continue'", harness)
        self.assertIn("QEMU host harness exited with status", harness)
        self.assertNotIn("Invoke-KasKoldCommand -Command $python", harness)

    def test_windows_qemu_provisioning_and_builds_stream_diagnostics(self) -> None:
        common = (WINDOWS / "lib/qemu-common.ps1").read_text(encoding="utf-8")
        build = (ROOT / "tools/firmware/qemu/build.ps1").read_text(encoding="utf-8")
        self.assertIn("Invoke-KasKoldStreamingCommand", common)
        self.assertIn("QEMU setup: resolving pinned Espressif QEMU", common)
        self.assertIn("Ensure-KasKoldQemuWindowsRuntime", common)
        self.assertIn("3221225781", common)
        self.assertIn("mingw-w64-x86_64-glib2", common)
        self.assertIn("mingw-w64-x86_64-pixman", common)
        self.assertIn("mingw-w64-x86_64-libgcrypt", common)
        self.assertIn("mingw-w64-x86_64-libslirp", common)
        self.assertIn("mingw-w64-x86_64-SDL2", common)
        self.assertIn("mingw64/bin", common)
        self.assertLess(common.index("Ensure-KasKoldQemuWindowsRuntime | Out-Null"), common.index("'tools/idf_tools.py'),'install','qemu-xtensa'"))
        self.assertIn("'-machine','help'", common)
        self.assertIn("required esp32s3 machine", common)
        self.assertIn("Invoke-KasKoldStreamingCommand -Command 'cargo'", build)
        self.assertIn("Invoke-KasKoldStreamingCommand -Command 'espflash'", build)
        command_lines = [line for line in build.splitlines() if "Invoke-KasKoldStreamingCommand" in line]
        self.assertTrue(command_lines)
        self.assertTrue(all("| Out-Null" not in line for line in command_lines))

    def test_windows_optional_tool_probes_can_bootstrap_missing_espflash(self) -> None:
        qemu = (WINDOWS / "lib/qemu-common.ps1").read_text(encoding="utf-8")
        self.assertIn("Invoke-KasKoldCapture -Command 'espflash'", qemu)
        self.assertIn("'install','espflash','--version',$env:KASKOLD_ESPFLASH_VERSION", qemu)
        self.assertLess(
            qemu.index("Invoke-KasKoldCapture -Command 'espflash'"),
            qemu.index("Require-KasKoldCommand espflash"),
        )

    def test_windows_android_gradle_uses_verified_python_for_runtime_builder(self) -> None:
        source = (WINDOWS / "build/android-build.ps1").read_text(encoding="utf-8")
        self.assertIn("$python = Get-KasKoldPython", source)
        self.assertIn("$gradleEnvironment = @{ 'PYTHON' = $python }", source)
        self.assertIn('Write-Host "==> Python: $python"', source)
        companion = "Invoke-KasKoldCommand -Command $gradle -Arguments (@('--project-dir',$android,'--no-daemon') + $companionTasks) -WorkingDirectory $root -Environment $gradleEnvironment"
        vault = "Invoke-KasKoldCommand -Command $gradle -Arguments (@('--project-dir',$vault,'--no-daemon') + $vaultTasks) -WorkingDirectory $root -Environment $gradleEnvironment"
        self.assertIn(companion, source)
        self.assertIn(vault, source)

    def test_windows_python_selector_requires_tomllib_capable_runtime(self) -> None:
        common = (WINDOWS / "lib/common.ps1").read_text(encoding="utf-8")
        start = common.index("function Get-KasKoldPython")
        end = common.index("function Require-KasKoldCommand", start)
        selector = common[start:end]
        self.assertIn("Get-Command 'py.exe'", selector)
        self.assertIn("'3.14','3.13','3.12','3.11'", selector)
        self.assertIn("sys.version_info >= (3, 11)", selector)
        self.assertIn("$env:KASKOLD_PYTHON", selector)
        self.assertIn("python3.14.exe", selector)
        self.assertIn("Python 3.11 or newer is required", selector)
        self.assertIn("Run make install", selector)

    def test_windows_standalone_lock_repair_does_not_require_python(self) -> None:
        locks = (WINDOWS / "lib/cargo_locks.ps1").read_text(encoding="utf-8")
        start = locks.index("function Get-KasKoldLockPackageCount")
        end = locks.index("function Invoke-KasKoldHostCargoMetadata", start)
        counter = locks[start:end]
        self.assertNotIn("Get-KasKoldPython", counter)
        self.assertNotIn("tomllib", counter)
        self.assertIn("Select-String", counter)
        self.assertIn(r"\[\[package\]\]", counter)

    def test_windows_cargo_metadata_compatibility_avoids_powershell_json_case_collision(self) -> None:
        locks = (WINDOWS / "lib/cargo_locks.ps1").read_text(encoding="utf-8")
        self.assertNotIn("$Json | ConvertFrom-Json", locks)
        self.assertIn("cargo_metadata_compat.py", locks)
        self.assertIn("Write-KasKoldUtf8NoBom", locks)

        helper = ROOT / "scripts/common/lib/cargo_metadata_compat.py"
        metadata = {
            "packages": [
                {
                    "name": "caseful",
                    "version": "1.0.0",
                    "rust_version": "1.84",
                    "metadata": {"Default": True, "default": False},
                }
            ]
        }
        with tempfile.TemporaryDirectory() as temporary:
            metadata_path = Path(temporary) / "metadata.json"
            metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
            compatible = subprocess.run(
                [sys.executable, str(helper), "--metadata", str(metadata_path), "--max-rust", "1.85"],
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(compatible.returncode, 0, compatible.stderr)
            incompatible = subprocess.run(
                [sys.executable, str(helper), "--metadata", str(metadata_path), "--max-rust", "1.83"],
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(incompatible.returncode, 1, incompatible.stderr)
            self.assertIn("caseful 1.0.0 requires Rust 1.84", incompatible.stdout)

    def test_windows_runner_lock_creates_fresh_target_parent_tree(self) -> None:
        runner_path = ROOT / "qa/windows/runner/run_all.py"
        spec = importlib.util.spec_from_file_location("windows_run_all_fresh_lock", runner_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        marker_name = "KASKOLD_QA_RUN_ALL_LOCK_ROOT"
        previous_marker = os.environ.pop(marker_name, None)
        original_root = module.ROOT
        handle = None
        try:
            with tempfile.TemporaryDirectory() as temporary:
                fresh_root = Path(temporary) / "fresh-repository"
                fresh_root.mkdir()
                module.ROOT = fresh_root
                handle = module.acquire_lock()
                self.assertTrue((fresh_root / "target/qa/state/release-workflow.lock").is_file())
                self.assertIsNotNone(handle)
                handle.close()
                handle = None
        finally:
            if handle is not None:
                handle.close()
            module.ROOT = original_root
            if previous_marker is None:
                os.environ.pop(marker_name, None)
            else:
                os.environ[marker_name] = previous_marker

    def test_scripts_root_contains_only_platform_directories(self) -> None:
        self.assertEqual({p.name for p in SCRIPTS.iterdir()}, {"common", "linux", "mac", "windows"})
        self.assertTrue(all(p.is_dir() for p in SCRIPTS.iterdir()))

    def test_platform_trees_have_same_top_level_categories(self) -> None:
        for tree in (LINUX, WINDOWS):
            self.assertEqual({p.name for p in tree.iterdir() if p.is_dir()}, CATEGORIES)
            self.assertFalse(any(p.is_file() for p in tree.iterdir()))

    def test_macos_tree_is_make_backed_and_ios_scoped(self) -> None:
        self.assertEqual({p.name for p in MAC.iterdir()}, {"build"})
        self.assertEqual(
            {p.name for p in (MAC / "build").iterdir()},
            {"ios-build.sh", "ios-runtime-sync.sh"},
        )
        self.assertFalse((ROOT / "install.sh").exists())
        self.assertFalse((ROOT / "install.ps1").exists())
        self.assertFalse((ROOT / "apps/kaskold-companion-ios/setup-macos.command").exists())

    def test_public_entrypoints_are_mirrored_by_relative_path(self) -> None:
        linux = {
            p.relative_to(LINUX).with_suffix("").as_posix()
            for p in LINUX.rglob("*.sh")
            if p.parent.name in {"build", "install", "qemu", "quality"}
        }
        windows = {
            p.relative_to(WINDOWS).with_suffix("").as_posix()
            for p in WINDOWS.rglob("*.ps1")
            if p.parent.name in {"build", "install", "qemu", "quality"}
        }
        self.assertEqual(PUBLIC, linux)
        self.assertEqual(PUBLIC, windows)

    def test_make_helpers_have_one_common_owner(self) -> None:
        helper = COMMON / "lib/make_tasks.py"
        self.assertTrue(helper.is_file())
        for name in ("make_tasks.py", "serial_access.py", "esp_toolchain.py", "make_public.py", "make_clean.py"):
            self.assertTrue((COMMON / "lib" / name).is_file())
            self.assertFalse((LINUX / "lib" / name).exists())
            self.assertFalse((WINDOWS / "lib" / name).exists())
        helper_source = helper.read_text(encoding="utf-8")
        flash_source = (COMMON / "lib/make_flash.py").read_text(encoding="utf-8")
        self.assertIn('return resolve_serial_port(port)', flash_source)
        self.assertIn("prepare_serial_command", flash_source)
        self.assertIn('DEFAULT_SCRIPT_ROOT = ROOT / "scripts" / ("windows" if IS_WINDOWS else "linux")', helper_source)
        self.assertIn('MAC_NATIVE_ENTRYPOINTS = {"ios-runtime-sync", "ios-build"}', helper_source)
        source = MAKEFILE.read_text(encoding="utf-8")
        self.assertIn("scripts/common/lib/make_tasks.py", source)
        self.assertNotIn("SCRIPT_PLATFORM", source)
        self.assertNotIn("scripts/platform.py", source)
        self.assertNotIn("scripts/entrypoints.json", source)


    def test_windows_runner_forces_utf8_for_child_python(self) -> None:
        source = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")
        self.assertIn('os.environ["PYTHONUTF8"] = "1"', source)
        self.assertIn('os.environ["PYTHONIOENCODING"] = "utf-8"', source)

    def test_windows_qa_self_provisions_missing_esp_rust_toolchain(self) -> None:
        runner = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")
        helper = (ROOT / "scripts/windows/lib/qemu-common.ps1").read_text(encoding="utf-8")

        self.assertIn("def ensure_esp_rust_toolchain", runner)
        self.assertIn('scripts/windows/lib/qemu-common.ps1', runner)
        self.assertIn("Install-KasKoldRustupIfMissing; Install-KasKoldEspRustToolchain", runner)
        cargo_resolution = runner.index('elif step_id=="preflight.cargo-resolution"')
        esp_bootstrap = runner.index("ensure_esp_rust_toolchain(ns.dry_run)", cargo_resolution)
        reconciliation = runner.index("reconcile_lock(directory,manifest", cargo_resolution)
        self.assertLess(esp_bootstrap, reconciliation)
        self.assertGreaterEqual(runner.count("ensure_esp_rust_toolchain(ns.dry_run)"), 4)

        self.assertIn("ESP Rust toolchain is missing; provisioning pinned ESP Rust", helper)
        self.assertIn("KASKOLD_ESPUP_VERSION", helper)
        self.assertIn("KASKOLD_ESP_RUST", helper)
        self.assertIn("'run',$env:KASKOLD_STABLE_RUST,'cargo','install','espup'", helper)
        self.assertIn("'install','--toolchain-version',$env:KASKOLD_ESP_RUST", helper)

    def test_posix_only_tooling_execution_is_platform_scoped(self) -> None:
        expectations = {
            "test_master_launcher.py": "Linux master-launcher tests are POSIX-specific",
            "test_reproducible_build_runner.py": "Linux reproducible-build runner tests are POSIX-specific",
        }
        tooling = ROOT / "qa/tests/tooling"
        for filename, reason in expectations.items():
            source = (tooling / filename).read_text(encoding="utf-8")
            self.assertIn(reason, source, filename)
        qemu = (tooling / "test_qemu_runner.py").read_text(encoding="utf-8")
        self.assertIn("QemuRunnerPortabilityTests", qemu)
        self.assertIn('QEMU fake guest fixture is POSIX-specific', qemu)
        self.assertNotIn('QEMU host marker runner uses POSIX pipe/fd semantics', qemu)

    def test_windows_sources_never_bridge_to_wsl_or_bash(self) -> None:
        forbidden = ("wsl.exe", "wslpath", "_invoke-wsl", "bash.exe", "/bin/bash")
        for path in WINDOWS.rglob("*.ps1"):
            source = path.read_text(encoding="utf-8").lower()
            for token in forbidden:
                self.assertNotIn(token, source, f"{path.relative_to(ROOT)}: {token}")

    def test_linux_dispatcher_preserves_arguments_and_exit_code(self) -> None:
        fixture_dir = ROOT / "target/qa/platform-wrapper-test"
        fixture_dir.mkdir(parents=True, exist_ok=True)
        fixture = fixture_dir / "fixture.sh"
        # Executable shell fixtures must be byte-exact LF even when this test runs
        # under native Windows Python; CRLF would turn `exit 23` into `exit 23\r`.
        fixture.write_bytes(
            b"#!/usr/bin/env bash\n"
            b"printf '<%s>\\n' \"$@\"\n"
            b"exit 23\n"
        )
        try:
            result = subprocess.run(
                [
                    qa_bash_executable(),
                    "scripts/linux/lib/_invoke.sh",
                    fixture.relative_to(ROOT).as_posix(),
                    "alpha beta",
                    "--flag=value",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(result.returncode, 23, result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["<alpha beta>", "<--flag=value>"])
        finally:
            shutil.rmtree(fixture_dir, ignore_errors=True)

    def test_dispatchers_fail_closed_on_target_escape(self) -> None:
        self.assertIn('"$TARGET" != *".."*', (LINUX / "lib/_invoke.sh").read_text(encoding="utf-8"))
        windows = (WINDOWS / "lib/_invoke.ps1").read_text(encoding="utf-8")
        self.assertIn("[System.IO.Path]::IsPathRooted($Target)", windows)

    def test_make_helper_maps_to_organized_platform_paths(self) -> None:
        helper = COMMON / "lib/make_tasks.py"
        spec = importlib.util.spec_from_file_location("make_tasks", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertEqual(module.ENTRYPOINTS["run-all"], "quality/run-all")
        self.assertEqual(module.ENTRYPOINTS["android-build"], "build/android-build")
        self.assertEqual(module.ENTRYPOINTS["qemu-test"], "qemu/test")
        self.assertEqual(module.ENTRYPOINTS["sdk-build"], "build/sdk-build")

        calls = []
        original_run = module.run
        original_macos = module.IS_MACOS
        original_windows = module.IS_WINDOWS
        original_default_script_root = module.DEFAULT_SCRIPT_ROOT
        try:
            module.IS_MACOS = True
            module.IS_WINDOWS = False
            module.DEFAULT_SCRIPT_ROOT = module.ROOT / "scripts" / "linux"
            module.run = lambda command, **kwargs: calls.append(command) or 0
            self.assertEqual(module.platform("ios-build", ["build"]), 0)
            self.assertIn("scripts/mac/build/ios-build.sh", calls.pop()[1].replace("\\", "/"))
            self.assertEqual(module.platform("kaskold-companion-web-build", []), 0)
            self.assertIn("scripts/linux/build/kaskold-companion-web-build.sh", calls.pop()[1].replace("\\", "/"))
            self.assertEqual(module.platform("kaskold-vault-web-build", []), 0)
            self.assertIn("scripts/linux/build/kaskold-vault-web-build.sh", calls.pop()[1].replace("\\", "/"))
        finally:
            module.run = original_run
            module.IS_MACOS = original_macos
            module.IS_WINDOWS = original_windows
            module.DEFAULT_SCRIPT_ROOT = original_default_script_root

    def test_mobile_product_selectors_are_forwarded_through_make_tasks(self) -> None:
        helper = COMMON / "lib/make_tasks.py"
        spec = importlib.util.spec_from_file_location("make_tasks_mobile_products", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        calls = []

        def fake_platform(entry, args=None, *, env=None):
            calls.append((entry, args, env))
            return 0

        with mock.patch.object(module, "platform", side_effect=fake_platform):
            self.assertEqual(module.android_action("build", "vault"), 0)
            self.assertEqual(module.ios_action("build", "vault"), 0)

        self.assertEqual(calls[0], ("android-build", ["debug"], {"KASKOLD_ANDROID_PRODUCTS": "vault"}))
        self.assertEqual(calls[1], ("ios-build", ["build"], {"KASKOLD_IOS_PRODUCTS": "vault"}))

    def test_android_portable_smoke_skips_without_standalone_kotlin_cli(self) -> None:
        runner = ROOT / "qa/checks/android/run_core_tests.py"
        spec = importlib.util.spec_from_file_location("android_portable_core", runner)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with mock.patch.object(module.shutil, "which", return_value=None):
            self.assertEqual(module.main(), 77)

    def test_android_qa_accepts_optional_helper_skip_after_gradle_tests(self) -> None:
        helper = COMMON / "lib/make_tasks.py"
        spec = importlib.util.spec_from_file_location("make_tasks_android_qa", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        calls: list[list[str]] = []
        results = iter((0, 77, 0, 0, 0, 0))
        with mock.patch.object(module, "platform", return_value=0), mock.patch.object(
            module.shutil, "which", return_value="/toolchain/bin/tool"
        ), mock.patch.object(
            module, "run", side_effect=lambda command, **_kwargs: calls.append(command) or next(results)
        ):
            self.assertEqual(module.android_action("qa"), 0)
        self.assertEqual(len(calls), 6)
        self.assertTrue(any(command[-1].endswith("run_core_tests.py") for command in calls))
        self.assertTrue(any(command[-2:] == ["--scope", "android"] for command in calls))

    def test_android_qa_skips_portable_smoke_before_invocation_without_global_kotlin_cli(self) -> None:
        helper = COMMON / "lib/make_tasks.py"
        spec = importlib.util.spec_from_file_location("make_tasks_android_qa_no_kotlinc", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        calls: list[list[str]] = []
        with mock.patch.object(module, "platform", return_value=0), mock.patch.object(
            module.shutil, "which", side_effect=lambda name: None if name in {"kotlinc", "java"} else f"/{name}"
        ), mock.patch.object(
            module, "run", side_effect=lambda command, **_kwargs: calls.append(command) or 0
        ):
            self.assertEqual(module.android_action("qa"), 0)
        self.assertEqual(len(calls), 5)
        self.assertFalse(any(command[-1].endswith("run_core_tests.py") for command in calls))
        self.assertTrue(any(command[-2:] == ["--scope", "android"] for command in calls))

    def test_flash_firmware_spawns_separate_windows_uart_console_when_launcher_has_no_tty(self) -> None:
        helper = COMMON / "lib/make_flash.py"
        spec = importlib.util.spec_from_file_location("make_flash_managed", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        board_results = [
            subprocess.CompletedProcess(args=[], returncode=0, stdout="--chip\nesp32s3\n--before\nusb-reset\n"),
            subprocess.CompletedProcess(args=[], returncode=0, stdout="--partition-table\nparts.csv\n--flash-size\n16mb\n"),
        ]
        flashed: list[list[str]] = []
        stdin = io.StringIO()
        stdout = io.StringIO()
        stderr = io.StringIO()
        build = mock.Mock(return_value=(0, Path("firmware.elf")))
        execute = lambda command: flashed.append(command) or 0
        monitor = mock.Mock()
        with mock.patch.object(module.subprocess, "run", side_effect=board_results), \
             mock.patch.object(module.subprocess, "Popen", monitor), \
             mock.patch.object(module.shutil, "which", return_value=r"C:\tools\espflash.exe"), \
             mock.patch.object(module.sys, "platform", "win32"), \
             mock.patch.object(module, "resolve_serial_port", return_value="COM11"), \
             mock.patch.object(module, "prepare_serial_command", side_effect=lambda command, _port: command):
            self.assertEqual(
                module.flash_firmware(
                    ROOT, "m5stack", "", build, execute,
                    stdin=stdin, stdout=stdout, stderr=stderr,
                ),
                0,
            )
        self.assertEqual(len(flashed), 1)
        command = flashed[0]
        self.assertEqual(command[:2], ["espflash", "flash"])
        self.assertNotIn("--monitor", command)
        self.assertNotIn("--non-interactive", command)
        self.assertEqual(command[command.index("--port") + 1], "COM11")
        monitor.assert_called_once()
        monitor_command = monitor.call_args.args[0]
        self.assertEqual(monitor_command[:2], [r"C:\tools\espflash.exe", "monitor"])
        self.assertEqual(monitor_command[monitor_command.index("--port") + 1], "COM11")
        self.assertEqual(monitor_command[monitor_command.index("--before") + 1], "no-reset-no-sync")
        self.assertEqual(monitor_command[monitor_command.index("--elf") + 1], "firmware.elf")
        self.assertTrue(monitor.call_args.kwargs["close_fds"])
        self.assertNotEqual(monitor.call_args.kwargs["creationflags"], 0)
        self.assertIn("separate console window", stdout.getvalue())
        self.assertIn("UART monitor opened in a new console", stdout.getvalue())

    def test_flash_firmware_keeps_interactive_monitor_on_real_tty(self) -> None:
        helper = COMMON / "lib/make_flash.py"
        spec = importlib.util.spec_from_file_location("make_flash_tty", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        class TtyBuffer(io.StringIO):
            def isatty(self) -> bool:
                return True

        board_results = [
            subprocess.CompletedProcess(args=[], returncode=0, stdout="--chip\nesp32s3\n--before\nusb-reset\n"),
            subprocess.CompletedProcess(args=[], returncode=0, stdout="--partition-table\nparts.csv\n--flash-size\n16mb\n"),
        ]
        flashed: list[list[str]] = []
        stdin = TtyBuffer()
        stdout = TtyBuffer()
        stderr = io.StringIO()
        build = mock.Mock(return_value=(0, Path("firmware.elf")))
        execute = lambda command: flashed.append(command) or 130
        with mock.patch.object(module.subprocess, "run", side_effect=board_results), \
             mock.patch.object(module, "resolve_serial_port", return_value="COM11"), \
             mock.patch.object(module, "prepare_serial_command", side_effect=lambda command, _port: command):
            self.assertEqual(
                module.flash_firmware(
                    ROOT, "m5stack", "", build, execute,
                    stdin=stdin, stdout=stdout, stderr=stderr,
                ),
                0,
            )
        command = flashed[0]
        self.assertEqual(command[:3], ["espflash", "flash", "--monitor"])
        self.assertNotIn("--non-interactive", command)
        self.assertIn("press CTRL+C to exit", stdout.getvalue())

    def test_flash_release_uses_only_existing_checksum_verified_signed_image(self) -> None:
        helper = COMMON / "lib/make_flash.py"
        spec = importlib.util.spec_from_file_location("make_flash_release", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        with tempfile.TemporaryDirectory() as td:
            release = Path(td)
            image = release / "kaskold-m5stack-full.bin"
            image.write_bytes(b"signed-merged-release-fixture")
            digest = __import__("hashlib").sha256(image.read_bytes()).hexdigest()
            (release / "SHA256SUMS").write_text(
                f"{digest}  {image.name}\n", encoding="utf-8"
            )
            board_result = subprocess.CompletedProcess(
                args=[], returncode=0, stdout="--chip\nesp32s3\n--before\nusb-reset\n"
            )
            flashed: list[list[str]] = []
            stdout = io.StringIO()
            stderr = io.StringIO()
            execute = lambda command: flashed.append(command) or 0
            with mock.patch.object(module.subprocess, "run", return_value=board_result), \
                 mock.patch.object(module, "prepare_serial_command", side_effect=lambda command, _port: command):
                self.assertEqual(
                    module.flash_release(
                        ROOT, "m5stack", "COM7", str(release), execute,
                        stdout=stdout, stderr=stderr,
                    ),
                    0,
                )

            self.assertEqual(len(flashed), 1)
            command = flashed[0]
            self.assertEqual(command[0:2], ["espflash", "write-bin"])
            self.assertIn("--port", command)
            self.assertIn("COM7", command)
            self.assertEqual(command[-2:], ["0x0", str(image)])
            self.assertNotIn("flash", command[1:])
            self.assertFalse(any("unsigned" in token for token in command))

            image.write_bytes(b"tampered")
            flashed.clear()
            self.assertEqual(
                module.flash_release(
                    ROOT, "m5stack", "", str(release), execute,
                    stdout=stdout, stderr=stderr,
                ),
                2,
            )
            self.assertEqual(flashed, [])

    def test_secure_release_profiles_are_nonflashing_and_owner_only_drops_vendor_identity(self) -> None:
        helper = COMMON / "lib/make_firmware_profiles.py"
        spec = importlib.util.spec_from_file_location("make_firmware_profiles_test", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        with tempfile.TemporaryDirectory() as td:
            temp = Path(td)
            owner = temp / "owner.pem"
            vendor = temp / "vendor.pem"
            schnorr = temp / "vendor.schnorr"
            owner.write_text("owner-rsa-fixture", encoding="utf-8")
            vendor.write_text("vendor-rsa-fixture", encoding="utf-8")
            schnorr.write_bytes(b"v" * 32)
            calls: list[tuple[list[str], dict[str, str]]] = []

            def record(command, **kwargs):
                calls.append((command, dict(kwargs["env"])))
                return 0

            base_env = {"KASKOLD_SIGNING_KEY": "inherited-vendor-key"}
            with mock.patch.object(module, "prepare_esp_build_environment", return_value=base_env.copy()), \
                 mock.patch.object(module.shutil, "which", side_effect=lambda name: "/bin/bash" if name == "bash" else None):
                self.assertEqual(
                    module.secure_release_profile(ROOT, False, record, "owner-only", str(temp / "owner-out"), str(owner), ""),
                    0,
                )
            command, env = calls.pop()
            self.assertIn("--owner-only", command)
            self.assertTrue(command[-1].endswith("owner-out"))
            self.assertEqual(env["KASKOLD_OWNER_SECURE_BOOT_KEY"], str(owner.resolve()))
            self.assertNotIn("KASKOLD_SIGNING_KEY", env)
            self.assertFalse(any(token in command for token in ("flash", "espefuse", "espflash")))

            with mock.patch.object(module, "prepare_esp_build_environment", return_value={}), \
                 mock.patch.object(module.shutil, "which", side_effect=lambda name: "/bin/bash" if name == "bash" else None):
                self.assertEqual(
                    module.secure_release_profile(ROOT, False, record, "dual", str(temp / "dual-out"), str(vendor), str(schnorr)),
                    0,
                )
            command, env = calls.pop()
            self.assertNotIn("--owner-only", command)
            self.assertEqual(env["KASKOLD_SECURE_BOOT_SIGNING_KEY"], str(vendor.resolve()))
            self.assertEqual(env["KASKOLD_SIGNING_KEY"], str(schnorr.resolve()))

            with mock.patch.object(module, "prepare_esp_build_environment", return_value={"KASKOLD_SIGNING_KEY": "inherited"}), \
                 mock.patch.object(module.shutil, "which", side_effect=lambda name: "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe" if name == "powershell.exe" else None):
                self.assertEqual(
                    module.secure_release_profile(ROOT, True, record, "owner-only", str(temp / "win-owner-out"), str(owner), ""),
                    0,
                )
            command, env = calls.pop()
            self.assertIn("prepare_m5stack_secure_release.ps1", " ".join(command).replace("\\", "/"))
            self.assertIn("-OwnerOnly", command)
            self.assertNotIn("KASKOLD_SIGNING_KEY", env)

    def test_secure_release_preparation_rejects_stale_opposite_policy_artifacts(self) -> None:
        script = "tools/build/firmware/prepare_m5stack_secure_release.sh"
        fixture_root = ROOT / "target" / "qa" / "secure-release-stale-policy-test"
        fixture_root.mkdir(parents=True, exist_ok=True)

        # Keep this fixture inside the repository and pass Bash a repo-relative
        # POSIX path. This avoids host-specific C:\\... -> /c/... vs /mnt/c/...
        # translation entirely when Windows launches Git Bash, MSYS, or WSL.
        for mode, stale_name, expected in (
            ("owner-only", "kaskold-m5stack-secure-provisioning.bin", "stale dual-authority artifact"),
            ("dual", "kaskold-m5stack-secure-owner-only.bin", "stale owner-only artifact"),
        ):
            out = fixture_root / mode
            shutil.rmtree(out, ignore_errors=True)
            out.mkdir(parents=True)
            (out / stale_name).write_bytes(b"stale-policy-fixture")
            relative_out = out.relative_to(ROOT).as_posix()
            command = [qa_bash_executable(), script]
            if mode == "owner-only":
                command.append("--owner-only")
            command.append(relative_out)
            try:
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    text=True,
                    capture_output=True,
                )
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn(expected, result.stderr)
            finally:
                shutil.rmtree(out, ignore_errors=True)

    def test_make_is_the_stable_public_developer_facade(self) -> None:
        source = MAKEFILE.read_text(encoding="utf-8")
        expected = {
            "companion", "vault-web", "sdk", "ios", "ios-vault", "ios-release", "ios-test", "ios-qa",
            "android", "android-vault", "android-release", "android-test", "android-qa",
            "firmware", "flash", "flash-release", "secure-provisioning", "secure-owner-only",
            "owner-firmware", "test-hardware",
            "workflow-e2e", "workflow-hil", "firmware-qemu-setup",
            "firmware-qemu", "firmware-qemu-test", "test", "qa",
            "release", "release-readiness", "clean", "help",
        }
        targets = {
            match.group(1)
            for match in re.finditer(r"(?m)^([A-Za-z0-9_.-]+):(?:\s|$)", source)
            if match.group(1) != ".PHONY"
        }
        self.assertEqual(targets, expected)
        for forbidden in (
            "test-fast", "reproducible-build", "real-node-integration", "branch-coverage-setup",
            "branch-coverage-bundle", "all", "firmware-qemu-run", "firmware-dev",
            "firmware-m5", "firmware-check", "firmware-lint", "architecture",
            "crap-check", "health-check", "security-invariants", "branch-ratchets",
            "mutation-critical", "mutation-certify", "fuzz-security", "firmware-features",
        ):
            self.assertNotRegex(source, rf"(?m)^{re.escape(forbidden)}:")
        self.assertIn('firmware:\n\t$(MAKE_TASK) firmware "$(BOARD)"', source)
        self.assertIn('flash:\n\t$(MAKE_TASK) flash "$(BOARD)" "$(PORT)"', source)
        self.assertIn('flash-release:\n\t$(MAKE_TASK) flash-release "$(BOARD)" "$(PORT)" "$(RELEASE_DIR)"', source)
        self.assertIn('secure-provisioning:\n\t$(MAKE_TASK) secure-release dual "$(SECURE_DIR)" "$(SECURE_BOOT_KEY)" "$(SIGNING_KEY)"', source)
        self.assertIn('secure-owner-only:\n\t$(MAKE_TASK) secure-release owner-only "$(SECURE_DIR)" "$(OWNER_KEY)" ""', source)
        self.assertIn('test:\n\t$(MAKE_TASK) test "$(STRICT_LOCKFILES)"', source)
        self.assertIn('qa:\n\t$(MAKE_TASK) qa "$(FUZZ_PASSES)" "$(STRICT_LOCKFILES)"', source)
        self.assertIn('release:\n\t$(MAKE_TASK) release', source)
        help_text = subprocess.run(["make", "help"], cwd=ROOT, text=True, capture_output=True, check=True).stdout
        for command in ("make test", "make qa", "make firmware", "make flash", "make flash-release",
                        "make secure-provisioning", "make secure-owner-only", "make ios", "make ios-vault", "make android", "make android-vault",
                        "make release", "make release-readiness"):
            self.assertIn(command, help_text)
        for forbidden in ("make architecture", "make crap-check", "make mutation-certify", "make reproducible-build", "make firmware-m5"):
            self.assertNotIn(forbidden, help_text)
        self.assertIn("BOARD defaults to m5stack", help_text)

    def test_mobile_make_targets_are_real_native_build_and_test_operations(self) -> None:
        makefile = MAKEFILE.read_text(encoding="utf-8")
        for recipe in (
            'ios:\n\t$(MAKE_TASK) ios build all',
            'ios-vault:\n\t$(MAKE_TASK) ios build vault',
            'ios-release:\n\t$(MAKE_TASK) ios release',
            'ios-test:\n\t$(MAKE_TASK) ios test',
            'android:\n\t$(MAKE_TASK) android build all',
            'android-vault:\n\t$(MAKE_TASK) android build vault',
            'android-release:\n\t$(MAKE_TASK) android release',
            'android-test:\n\t$(MAKE_TASK) android test',
        ):
            self.assertIn(recipe, makefile)
        ios = (LINUX / "build/ios-build.sh").read_text(encoding="utf-8")
        self.assertIn('[[ "$(uname -s)" != "Darwin" ]]', ios)
        mac_ios = (MAC / "build/ios-build.sh").read_text(encoding="utf-8")
        self.assertIn('[[ "$(uname -s)" != "Darwin" ]]', mac_ios)
        self.assertIn("xcodebuild", mac_ios)
        self.assertIn("-configuration Debug", mac_ios)
        self.assertIn("xcodebuild archive", mac_ios)
        self.assertIn("-configuration Release", mac_ios)
        self.assertGreaterEqual(mac_ios.count("KASKOLD_IOS_RUNTIME_SYNCED=1"), 3)
        self.assertIn("XCTest", (COMMON / "lib/make_help.txt").read_text(encoding="utf-8"))
        windows_ios = (WINDOWS / "build/ios-build.ps1").read_text(encoding="utf-8")
        self.assertIn("require macOS with Xcode", windows_ios)
        android = (LINUX / "build/android-build.sh").read_text(encoding="utf-8")
        self.assertIn("assembleDebug", android)
        self.assertIn("assembleRelease", android)
        self.assertIn("testDebugUnitTest", android)
        self.assertIn("app/build/outputs/apk/$MODE", android)
        self.assertIn("Built artifact", android)
        self.assertNotIn("lintDebug", android)
        windows_android = (WINDOWS / "build/android-build.ps1").read_text(encoding="utf-8")
        self.assertIn("app/build/outputs/apk/$Mode", windows_android)
        self.assertIn("Built artifact", windows_android)
        self.assertIn('-derivedDataPath "$DERIVED_DATA"', ios)
        self.assertIn("Built artifact", ios)
        self.assertIn("Built archive:", ios)
        self.assertIn("Test result bundle:", ios)
        self.assertIn('-derivedDataPath "$COMPANION_DERIVED_DATA"', mac_ios)
        self.assertIn('-derivedDataPath "$VAULT_DERIVED_DATA"', mac_ios)
        self.assertIn("Built Companion artifact", mac_ios)
        self.assertIn("Built Vault artifact", mac_ios)
        self.assertIn("Built Companion archive", mac_ios)
        self.assertIn("Built Vault archive", mac_ios)
        self.assertIn('PRODUCTS="${KASKOLD_IOS_PRODUCTS:-all}"', mac_ios)
        self.assertIn("Companion test result bundle:", mac_ios)

    @unittest.skipUnless(os.name == "posix", "mobile artifact-reporting fixture uses POSIX shell stubs")
    def test_mobile_build_wrappers_print_concrete_artifact_locations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fake_bin = root / "fake-bin"
            fake_bin.mkdir(parents=True)

            # Android: use a project-local fake Gradle that creates the same
            # release APK location produced by the Android Gradle plugin.
            android_script = root / "scripts/linux/build/android-build.sh"
            android_script.parent.mkdir(parents=True)
            shutil.copy2(LINUX / "build/android-build.sh", android_script)
            android_script.chmod(0o755)
            android_sdk_helper = root / "scripts/linux/lib/android-sdk.sh"
            android_sdk_helper.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(LINUX / "lib/android-sdk.sh", android_sdk_helper)
            android_sdk_helper.chmod(0o755)
            android = root / "apps/kaskold-companion-android"
            wrapper = android / "gradle/wrapper/gradle-wrapper.properties"
            wrapper.parent.mkdir(parents=True)
            wrapper.write_text(
                "distributionUrl=https\\://services.gradle.org/distributions/gradle-9.5.0-bin.zip\n"
                "distributionSha256Sum=553c78f50dafcd54d65b9a444649057857469edf836431389695608536d6b746\n",
                encoding="utf-8",
            )
            daemon_jvm = android / "gradle/gradle-daemon-jvm.properties"
            daemon_jvm.write_text("toolchainVersion=25\n", encoding="utf-8")
            toolchains = root / "qa/config/toolchains.env"
            toolchains.parent.mkdir(parents=True)
            toolchains.write_text(
                "KASKOLD_ANDROID_JDK=25\n"
                "KASKOLD_ANDROID_API=37\n"
                "KASKOLD_ANDROID_BUILD_TOOLS=37.0.0\n"
                "KASKOLD_STABLE_RUST=1.92.0\n"
                "KASKOLD_CARGO_NDK_VERSION=4.1.2\n"
                "KASKOLD_ANDROID_NDK=30.0.16248370\n",
                encoding="utf-8",
            )
            sdk_platform = root / "sdk/platforms/android-37"
            sdk_platform.mkdir(parents=True)
            (sdk_platform / "android.jar").write_bytes(b"")
            (sdk_platform / "source.properties").write_text("AndroidVersion.ApiLevel = 37\n", encoding="utf-8")
            aapt2 = root / "sdk/build-tools/37.0.0/aapt2"
            aapt2.parent.mkdir(parents=True)
            aapt2.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            aapt2.chmod(0o755)
            (root / "sdk/ndk/30.0.16248370").mkdir(parents=True)
            (root / "apps/kaskold-vault-android").mkdir(parents=True)
            rustup = fake_bin / "rustup"
            rustup.write_text(
                "#!/usr/bin/env bash\n"
                "printf '%s\n' aarch64-linux-android x86_64-linux-android aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios\n",
                encoding="utf-8",
            )
            rustup.chmod(0o755)
            cargo = fake_bin / "cargo"
            cargo.write_text("#!/usr/bin/env bash\necho 'cargo-ndk 4.1.2'\n", encoding="utf-8")
            cargo.chmod(0o755)
            gradle = fake_bin / "gradle"
            gradle.write_text(
                "#!/usr/bin/env bash\n"
                "set -eu\n"
                "if [[ \"${1:-}\" == \"--version\" ]]; then echo 'Gradle 9.5.0'; exit 0; fi\n"
                "project=''\n"
                "while [[ $# -gt 0 ]]; do\n"
                "  if [[ \"$1\" == '--project-dir' ]]; then project=\"$2\"; shift 2; continue; fi\n"
                "  shift\n"
                "done\n"
                "mkdir -p \"$project/app/build/outputs/apk/release\"\n"
                ": > \"$project/app/build/outputs/apk/release/app-release-unsigned.apk\"\n",
                encoding="utf-8",
            )
            gradle.chmod(0o755)
            fake_jdk = root / "fake-jdk"
            java = fake_jdk / "bin/java"
            java.parent.mkdir(parents=True)
            java.write_text(
                "#!/usr/bin/env bash\n"
                "echo 'openjdk version \"25.0.0\"' >&2\n",
                encoding="utf-8",
            )
            java.chmod(0o755)
            env = os.environ.copy()
            env.update({
                "KASKOLD_ANDROID_SDK_ROOT": str(root / "sdk"),
                "GRADLE_BIN": str(gradle),
                "PATH": str(fake_bin) + os.pathsep + env.get("PATH", ""),
                "JAVA_HOME": str(fake_jdk),
            })
            android_run = subprocess.run(
                [qa_bash_executable(), str(android_script), "release"], cwd=root, env=env,
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(android_run.returncode, 0, android_run.stdout + android_run.stderr)
            expected_apk = android / "app/build/outputs/apk/release/app-release-unsigned.apk"
            expected_vault_apk = root / "apps/kaskold-vault-android/app/build/outputs/apk/release/app-release-unsigned.apk"
            self.assertIn("Built artifact:", android_run.stdout)
            self.assertIn(str(expected_apk), android_run.stdout)
            self.assertIn(str(expected_vault_apk), android_run.stdout)

            # iOS: fake Darwin/Xcode and make xcodebuild materialize the
            # deterministic target/ios paths owned by the wrapper.
            ios_script = root / "scripts/mac/build/ios-build.sh"
            ios_script.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(MAC / "build/ios-build.sh", ios_script)
            ios_script.chmod(0o755)
            runtime_sync = root / "scripts/mac/build/ios-runtime-sync.sh"
            runtime_sync.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            runtime_sync.chmod(0o755)
            uname = fake_bin / "uname"
            uname.write_text("#!/usr/bin/env bash\necho Darwin\n", encoding="utf-8")
            uname.chmod(0o755)
            xcodebuild = fake_bin / "xcodebuild"
            xcodebuild.write_text(
                "#!/usr/bin/env bash\n"
                "set -eu\n"
                "archive=''\n"
                "derived=''\n"
                "result=''\n"
                "mode=build\n"
                "[[ \" ${*} \" == *' archive '* ]] && mode=archive\n"
                "while [[ $# -gt 0 ]]; do\n"
                "  case \"$1\" in\n"
                "    -archivePath) archive=\"$2\"; shift 2 ;;\n"
                "    -derivedDataPath) derived=\"$2\"; shift 2 ;;\n"
                "    -resultBundlePath) result=\"$2\"; shift 2 ;;\n"
                "    *) shift ;;\n"
                "  esac\n"
                "done\n"
                "if [[ -n \"$archive\" ]]; then mkdir -p \"$archive\"; fi\n"
                "if [[ -n \"$derived\" ]]; then mkdir -p \"$derived/Build/Products/Debug-iphonesimulator/KasKold.app\" \"$derived/Build/Products/Debug-iphonesimulator/KasKoldVault.app\"; fi\n"
                "if [[ -n \"$result\" ]]; then mkdir -p \"$result\"; fi\n",
                encoding="utf-8",
            )
            xcodebuild.chmod(0o755)
            ios_env = os.environ.copy()
            ios_env["PATH"] = str(fake_bin) + os.pathsep + ios_env.get("PATH", "")
            ios_debug = subprocess.run(
                [qa_bash_executable(), str(ios_script), "build"], cwd=root, env=ios_env,
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(ios_debug.returncode, 0, ios_debug.stdout + ios_debug.stderr)
            expected_app = root / "target/ios/CompanionDerivedData/Build/Products/Debug-iphonesimulator/KasKold.app"
            expected_vault_app = root / "target/ios/VaultDerivedData/Build/Products/Debug-iphonesimulator/KasKoldVault.app"
            self.assertIn("Built Companion artifact:", ios_debug.stdout)
            self.assertIn("Built Vault artifact:", ios_debug.stdout)
            self.assertIn(str(expected_app), ios_debug.stdout)
            self.assertIn(str(expected_vault_app), ios_debug.stdout)

            ios_env["KASKOLD_IOS_DEVELOPMENT_TEAM"] = "TESTTEAM"
            ios_release = subprocess.run(
                [qa_bash_executable(), str(ios_script), "release"], cwd=root, env=ios_env,
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(ios_release.returncode, 0, ios_release.stdout + ios_release.stderr)
            expected_archive = root / "target/ios/KasKold.xcarchive"
            expected_vault_archive = root / "target/ios/KasKoldVault.xcarchive"
            self.assertIn("Built Companion archive:", ios_release.stdout)
            self.assertIn("Built Vault archive:", ios_release.stdout)
            self.assertIn(str(expected_archive), ios_release.stdout)
            self.assertIn(str(expected_vault_archive), ios_release.stdout)

    def test_public_make_device_commands_have_explicit_profiles_and_monitor_semantics(self) -> None:
        helper = (COMMON / "lib/make_tasks.py").read_text(encoding="utf-8")
        firmware_body = helper.split("def firmware(board: str) -> int:", 1)[1].split("def flash_firmware", 1)[0]
        flash_body = (COMMON / "lib/make_flash.py").read_text(encoding="utf-8")
        self.assertNotIn("espflash", firmware_body)
        self.assertIn("Build only: no device was flashed", firmware_body)
        self.assertIn('features = "m5stack,workflow-tests,argon2-bench"', helper)
        self.assertIn('features = "m5stack-lite,workflow-tests,argon2-bench"', helper)
        self.assertNotIn('workflow-test-auto', firmware_body)
        self.assertIn('command = ["espflash", "flash"]', flash_body)
        self.assertIn('command.append("--monitor")', flash_body)
        self.assertIn("explicit --port bypasses its dialog UI", flash_body)
        self.assertIn("press CTRL+C to exit", flash_body)
        self.assertNotIn('reset_command = ["espflash", "reset"', flash_body)

        hardware = (ROOT / "qa/checks/firmware/run_hardware_tests.py").read_text(encoding="utf-8")
        workflow = (ROOT / "qa/checks/firmware/run_workflow_tests.py").read_text(encoding="utf-8")
        self.assertIn('f"{feature},hardware-tests"', hardware)
        self.assertIn('f"{feature},{profile}"', workflow)
        self.assertIn('profile = "workflow-hil-auto" if hil else "workflow-runtime-auto"', workflow)
        self.assertIn("flash_and_monitor", hardware)
        self.assertIn("flash_and_monitor", workflow)

    def test_public_make_helper_translates_only_facade_parameters(self) -> None:
        helper = COMMON / "lib/make_public.py"
        spec = importlib.util.spec_from_file_location("make_public_contract", helper)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        calls: list[tuple[str, list[str] | None]] = []

        def platform(name: str, args: list[str] | None = None) -> int:
            calls.append((name, args))
            return 0

        self.assertEqual(module.run_all_profile(platform, "full", "250000", "1"), 0)
        self.assertEqual(calls.pop(), ("run-all", ["--profile", "full", "--fuzz-passes", "250000", "--strict-lockfiles"]))
        self.assertEqual(module.run_all_profile(platform, "test", "999", ""), 0)
        self.assertEqual(calls.pop(), ("run-all", ["--profile", "test"]))
        self.assertEqual(module.run_all_profile(platform, "full", "100000", "", "unit.repository-python-qa"), 0)
        self.assertEqual(calls.pop(), ("run-all", ["--profile", "full", "--fuzz-passes", "100000", "--resume-from", "unit.repository-python-qa"]))
        self.assertEqual(module.test_hardware(platform, "m5stack", "/dev/ttyACM0", "300", ""), 0)
        self.assertEqual(
            calls.pop(),
            ("run-all", ["--category", "hardware", "--hardware", "m5stack", "--hardware-timeout", "300", "--hardware-port", "/dev/ttyACM0"]),
        )
        self.assertEqual(module.release_build(platform, False, "release-out", "key.bin", "1"), 0)
        self.assertEqual(
            calls.pop(),
            ("reproducible-build", ["--output-dir", "release-out", "--signing-key", "key.bin", "--refresh-inputs"]),
        )
        self.assertEqual(calls, [])
        self.assertEqual(module.release_build(platform, True, "release-out", "key.bin", "1"), 0)
        self.assertEqual(
            calls.pop(),
            ("reproducible-build", ["-OutputDir", "release-out", "-SigningKey", "key.bin", "-RefreshInputs"]),
        )
        self.assertEqual(calls, [])

    def test_linux_dry_run_does_not_require_flock(self) -> None:
        source = (ROOT / LINUX_RUN_ALL).read_text(encoding="utf-8")
        function_start = source.index("acquire_release_workflow_lock() {")
        function_end = source.index("\n}\n\n\nmain()", function_start)
        body = source[function_start:function_end]
        self.assertIn("if $DRY_RUN; then", body)
        self.assertLess(body.index("if $DRY_RUN; then"), body.index("command -v flock"))

    def test_normal_test_profile_is_one_filter_over_the_canonical_catalog(self) -> None:
        profile = ROOT / "qa/config/run_all_test_steps.txt"
        test_ids = [
            line.strip() for line in profile.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        self.assertEqual(len(test_ids), len(set(test_ids)))
        linux_list = run_linux_qa("--list")
        catalog_ids = [line.split()[0] for line in linux_list.splitlines()[2:] if line.strip()]
        self.assertTrue(set(test_ids).issubset(catalog_ids))
        for excluded in (
            "preflight.crap-check",
            "preflight.security-assurance",
            "preflight.firmware-source-contracts",
            "unit.repository-python-qa",
            "integration.repository-architecture",
            "integration.companion-ios-quality",
            "integration.companion-android-quality",
            "integration.signer-firmware-builds",
            "integration.signer-firmware-lints",
            "emulation.signer-firmware-qemu",
            "hardware.signer-firmware-device",
            "bench.shared-signer-protocol-throughput",
            "fuzz.repository-security-targets",
        ):
            self.assertNotIn(excluded, test_ids)

        expected = [step_id for step_id in catalog_ids if step_id in set(test_ids)]
        linux = run_linux_qa("--profile", "test", "--dry-run")
        windows = subprocess.run(
            [str(Path(__import__("sys").executable)), str(ROOT / "qa/windows/runner/run_all.py"),
             "--profile", "test", "--dry-run"],
            cwd=ROOT, text=True, capture_output=True, check=True,
        ).stdout

        def selected(text: str) -> list[str]:
            return [
                match.group(1)
                for line in text.splitlines()
                if (match := re.match(r"^\[([^]]+)\]", line))
            ]

        self.assertEqual(selected(linux), expected)
        self.assertEqual(selected(windows), expected)

    def test_windows_full_qa_dry_run_is_tool_independent_through_fuzz(self) -> None:
        runner = ROOT / "qa/windows/runner/run_all.py"
        environment = os.environ.copy()
        environment["PATH"] = ""
        result = subprocess.run(
            [
                sys.executable, str(runner), "--profile", "full",
                "--dry-run", "--fuzz-passes", "1",
            ],
            cwd=ROOT, env=environment, text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("integration.funded-testnet-e2e", result.stdout)
        self.assertIn("mutation.repository-security-fresh", result.stdout)
        self.assertIn("fuzz.repository-security-targets", result.stdout)

        expected = []
        for raw in (ROOT / "qa/config/run_all_steps.tsv").read_text(encoding="utf-8").splitlines():
            if not raw or raw.startswith("#"):
                continue
            scope, _category, _workspace, step_id, _description = raw.split("\t", 4)
            if scope == "qa":
                expected.append(step_id)

        selected = [
            match.group(1)
            for line in result.stdout.splitlines()
            if (match := re.match(r"^\[([^]]+)\]", line))
        ]
        self.assertEqual(selected, expected)
        self.assertIn(
            f"{len(expected)} passed, 0 skipped, {len(expected)} selected test sections completed",
            result.stdout,
        )

    def test_windows_qa_tail_uses_repository_local_mutation_and_fuzz_tools(self) -> None:
        source = (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8")
        compact = source.replace(" ", "")
        self.assertIn('ROOT / "target/development-tools"', source)
        self.assertIn('env["CARGO_INSTALL_ROOT"] = str(root)', source)
        self.assertIn('"cargo-mutants", toolchain, version', source)
        self.assertIn('"cargo-fuzz", toolchain, version', source)
        self.assertIn('"--root",str(tool_root)', compact)
        self.assertIn('env=mutation_tool_environment()', compact)

    def test_mutation_setup_preserves_local_plugin_priority_and_explicit_install_root(self) -> None:
        support_path = ROOT / "qa/checks/security/mutation_support.py"
        spec = importlib.util.spec_from_file_location("mutation_support_windows_local_tools", support_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        with tempfile.TemporaryDirectory() as temporary:
            local_root = Path(temporary) / "local-mutants"
            local_bin = local_root / "bin"
            cargo_bin = Path(temporary) / "cargo-home/bin"
            original_path = os.pathsep.join((str(local_bin), str(cargo_bin)))
            environment = {
                "PATH": original_path,
                "CARGO_HOME": str(cargo_bin.parent),
                "CARGO_INSTALL_ROOT": str(local_root),
            }
            with mock.patch.dict(os.environ, environment, clear=True), mock.patch.object(
                module.shutil, "which", side_effect=lambda name: str(cargo_bin / "rustup") if name == "rustup" else None
            ):
                self.assertEqual(module.ensure_rustup(), 0)
                self.assertEqual(os.environ["PATH"], original_path)

            commands: list[list[str]] = []
            expected = "cargo-mutants 27.1.0"
            with (
                mock.patch.dict(os.environ, environment, clear=True),
                mock.patch.object(module, "ensure_rustup", return_value=0),
                mock.patch.object(module, "reconcile_root_lock", return_value=0),
                mock.patch.object(module, "captured", side_effect=["cargo-mutants 0.0.0", expected]),
                mock.patch.object(module, "run", side_effect=lambda command, **_kwargs: commands.append(list(command))),
            ):
                self.assertEqual(
                    module.setup({"toolchain": "1.95.0", "cargo_mutants_version": "27.1.0"}),
                    0,
                )
            install = next(command for command in commands if "cargo-mutants" in command)
            self.assertIn("--root", install)
            self.assertEqual(install[install.index("--root") + 1], str(local_root))

    def test_windows_funded_skip_77_is_nonfatal_in_master_catalog(self) -> None:
        runner_path = ROOT / "qa/windows/runner/run_all.py"
        spec = importlib.util.spec_from_file_location("windows_run_all_funded_skip", runner_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        namespace = type("Args", (), {"dry_run": False})()
        with mock.patch.object(module, "run", return_value=77) as call:
            module.run_step("integration.funded-testnet-e2e", namespace)
        self.assertTrue(module.STEP_SKIPPED)
        self.assertEqual(call.call_args.kwargs["allowed"], (0, 77))

    def test_windows_fuzz_bootstrap_is_self_contained_and_local(self) -> None:
        runner_path = ROOT / "qa/windows/runner/run_all.py"
        spec = importlib.util.spec_from_file_location("windows_run_all_fuzz_bootstrap", runner_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "cargo-fuzz-local"
            bin_dir = root / "bin"
            bin_dir.mkdir(parents=True)
            environment = {
                "KASKOLD_STABLE_RUST": "1.95.0",
                "KASKOLD_BRANCH_RUST": "nightly-2026-07-31",
                "KASKOLD_CARGO_FUZZ_VERSION": "0.13.2",
                "PATH": "",
            }
            probes = [
                subprocess.CompletedProcess([], 0, "rustc stable", ""),
                subprocess.CompletedProcess([], 0, "rustc nightly", ""),
                subprocess.CompletedProcess([], 1, "", "missing"),
                subprocess.CompletedProcess([], 0, "cargo-fuzz 0.13.2", ""),
            ]
            commands: list[list[str]] = []
            with (
                mock.patch.dict(os.environ, environment, clear=True),
                mock.patch.object(module, "require"),
                mock.patch.object(module, "configure_fuzz_tool_environment", return_value=(root, bin_dir)),
                mock.patch.object(module, "capture", side_effect=probes),
                mock.patch.object(module, "run", side_effect=lambda command, **_kwargs: commands.append(list(command)) or 0),
            ):
                module.ensure_fuzz_toolchain(False)
            self.assertIn(
                ["rustup", "component", "add", "llvm-tools-preview", "--toolchain", "nightly-2026-07-31"],
                commands,
            )
            install = next(command for command in commands if "cargo-fuzz" in command)
            self.assertIn("--root", install)
            self.assertEqual(install[install.index("--root") + 1], str(root))

    def test_windows_fuzz_registry_failure_is_not_mistaken_for_empty_success(self) -> None:
        runner_path = ROOT / "qa/windows/runner/run_all.py"
        spec = importlib.util.spec_from_file_location("windows_run_all_fuzz_registry", runner_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        namespace = type("Args", (), {"fuzz_target": "", "dry_run": False, "fuzz_passes": 1})()
        failed = subprocess.CompletedProcess([], 1, "ERROR: broken registry", "")
        with mock.patch.object(module, "ensure_fuzz_toolchain"), mock.patch.object(module, "capture", return_value=failed):
            with self.assertRaisesRegex(RuntimeError, "fuzz target registry validation failed"):
                module.run_fuzz(namespace)

    def test_windows_and_linux_master_catalogs_have_same_step_ids(self) -> None:
        linux = run_linux_qa("--list")
        windows = subprocess.run(
            [str(Path(__import__("sys").executable)), str(ROOT / "qa/windows/runner/run_all.py"), "--list"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout

        def ids(text: str) -> list[str]:
            return [line.split()[0] for line in text.splitlines()[2:] if line.strip()]

        self.assertEqual(ids(windows), ids(linux))

    def test_powershell_unscoped_variables_are_delimited_before_colons(self) -> None:
        invalid = re.compile(
            r"\$(?!(?:env|script|global|local|private|variable|function|alias):)"
            r"[A-Za-z_][A-Za-z0-9_]*:"
        )
        failures: list[str] = []
        for path in ROOT.rglob("*.ps1"):
            for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if invalid.search(line):
                    failures.append(f"{path.relative_to(ROOT)}:{line_number}: {line.strip()}")
        self.assertEqual(failures, [], "invalid PowerShell variable/colon interpolation:\n" + "\n".join(failures))

    def test_powershell_sources_parse_when_available(self) -> None:
        powershell = next(
            (path for name in ("pwsh", "powershell.exe", "powershell") if (path := shutil.which(name))),
            None,
        )
        if not powershell:
            self.skipTest("PowerShell parser is not installed on this host")
        sources = sorted(str(path) for path in ROOT.rglob("*.ps1"))
        parser = (
            "$paths=Get-Content -LiteralPath $env:KASKOLD_PS1_PARSE_LIST -Encoding UTF8; "
            "$failed=$false; foreach($path in $paths){"
            "$tokens=$null; $errors=$null; "
            "[System.Management.Automation.Language.Parser]::ParseFile([string]$path,[ref]$tokens,[ref]$errors)|Out-Null; "
            "if($errors.Count){$failed=$true; foreach($parseError in @($errors)){"
            "$message=[string]$parseError.Message; [Console]::Error.WriteLine(([string]$path + ': ' + $message))}}}; "
            "if($failed){exit 1}"
        )
        with tempfile.TemporaryDirectory() as temporary:
            parse_list = Path(temporary) / "powershell-sources.txt"
            parse_list.write_text("\n".join(sources) + "\n", encoding="utf-8")
            environment = os.environ.copy()
            environment["KASKOLD_PS1_PARSE_LIST"] = str(parse_list)
            result = subprocess.run(
                [powershell, "-NoProfile", "-NonInteractive", "-Command", parser],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
                env=environment,
            )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_native_installer_helpers_are_internal_and_root_facades_are_absent(self) -> None:
        linux = (LINUX / "install/install.sh").read_text(encoding="utf-8")
        windows = (WINDOWS / "install/install.ps1").read_text(encoding="utf-8")
        self.assertFalse((ROOT / "install.sh").exists())
        self.assertFalse((ROOT / "install.ps1").exists())
        self.assertFalse((MAC / "install.sh").exists())
        self.assertFalse((ROOT / "tools/install/macos").exists())
        self.assertIn("qa/linux/run-all.sh", linux)
        self.assertIn("qa/windows/run-all.ps1", windows)
        for token in ("make", "python", "git", "node", "npm", "rustup", "cargo", "espup", "espflash", "gradle", "kotlinc"):
            self.assertIn(token, linux.lower())
            self.assertIn(token, windows.lower())
        self.assertIn("android-${KASKOLD_ANDROID_API}", linux)
        self.assertIn("KASKOLD_ANDROID_API", windows)


    def test_installers_verify_pinned_native_readiness(self) -> None:
        linux = (LINUX / "install/install.sh").read_text(encoding="utf-8")
        windows = (WINDOWS / "install/install.ps1").read_text(encoding="utf-8")
        environment = (ROOT / "qa/linux/runner/environment.sh").read_text(encoding="utf-8")
        for token in (
            "KASKOLD_CARGO_MUTANTS_VERSION", "KASKOLD_CARGO_FUZZ_VERSION",
            "KASKOLD_CARGO_LLVM_COV_VERSION", "KASKOLD_CARGO_CRAP_VERSION",
            "KASKOLD_ANDROID_BUILD_TOOLS",
            "KASKOLD_ANDROID_NDK", "KASKOLD_CARGO_NDK_VERSION",
        ):
            self.assertIn(token, linux)
            self.assertIn(token, windows)
        self.assertIn("KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256", linux)
        self.assertIn("KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256", windows)
        self.assertIn('prepend_path_once "${HOME}/.local/bin"', environment)
        self.assertIn("KASKOLD_ANDROID_JDK", environment)
        self.assertIn("ANDROID_SDK_ROOT", environment)

    @unittest.skipUnless(os.name == "posix", "Linux bootstrap environment execution is POSIX-specific")
    def test_linux_runner_discovers_managed_bootstrap_without_reopening_shell(self) -> None:
        fixture = ROOT / "target/qa/bootstrap-environment-test"
        shutil.rmtree(fixture, ignore_errors=True)
        jdk_bin = fixture / ".local/share/kaskold/jdk-25/bin"
        jdk_bin.mkdir(parents=True)
        java = jdk_bin / "java"
        java.write_text("#!/bin/sh\nexit 0\n")
        java.chmod(0o755)
        (fixture / "Android/Sdk/platform-tools").mkdir(parents=True)
        (fixture / "Android/Sdk/cmdline-tools/latest/bin").mkdir(parents=True)
        command = (
            f'export HOME="{fixture}"; export PATH="/usr/bin:/bin"; '
            f'ROOT_DIR="{ROOT}"; source "{ROOT / "qa/linux/runner/environment.sh"}"; '
            'initialize_test_environment; '
            'printf "%s\\n%s\\n%s\\n" "$JAVA_HOME" "$ANDROID_SDK_ROOT" "$PATH"'
        )
        try:
            result = subprocess.run([qa_bash_executable(), "-c", command], cwd=ROOT, text=True, capture_output=True, check=True)
            lines = result.stdout.splitlines()
            self.assertEqual(lines[0], str(fixture / ".local/share/kaskold/jdk-25"))
            self.assertEqual(lines[1], str(fixture / "Android/Sdk"))
            self.assertIn(str(fixture / "Android/Sdk/platform-tools"), lines[2])
            self.assertIn(str(fixture / ".local/share/kaskold/jdk-25/bin"), lines[2])
        finally:
            shutil.rmtree(fixture, ignore_errors=True)

    def test_windows_installer_requires_native_msvc_asan(self) -> None:
        windows = (WINDOWS / "install/install.ps1").read_text(encoding="utf-8")
        self.assertIn("Microsoft.VisualStudio.Component.VC.Tools.x86.x64", windows)
        self.assertIn("Microsoft.VisualStudio.Component.VC.ASAN", windows)
        self.assertNotIn("wsl", windows.lower())

    def test_windows_fuzz_uses_native_msvc_mode(self) -> None:
        self.assertIn("--no-include-main-msvc", (ROOT / "qa/windows/run-security-fuzz.ps1").read_text(encoding="utf-8"))
        self.assertIn("--no-include-main-msvc", (ROOT / "qa/windows/runner/run_all.py").read_text(encoding="utf-8"))

    def test_ios_xcode_runtime_sync_is_not_linux_bound(self) -> None:
        project = (ROOT / "apps/kaskold-companion-ios/KasKold.xcodeproj/project.pbxproj").read_text(encoding="utf-8")
        helper = (ROOT / "tools/build/ios/sync_runtime.py").read_text(encoding="utf-8")
        self.assertIn('tools/build/web/build_companion_runtime.py', project)
        self.assertIn('tools/build/ios/sync_runtime.py', project)
        self.assertNotIn('scripts/linux/build/ios-runtime-sync.sh', project)
        self.assertIn('shutil.copytree', helper)
        self.assertIn('companion_web_bg.wasm', helper)


if __name__ == "__main__":
    unittest.main()
