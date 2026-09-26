#!/usr/bin/env python3
"""Run repository Python QA with mobile suites owned by dedicated commands."""
from __future__ import annotations

import argparse
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import traceback
import unittest

ROOT = Path(__file__).resolve().parents[3]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
TEST_ROOTS = (ROOT / "qa/tests/tooling", ROOT / "qa/tests/regression")
ANDROID_TESTS = {
    "test_android_gradle_bootstrap.py",
    "test_android_gradle_exec_recovery.py",
    "test_android_mutation_toolchain.py",
    "test_android_vault_isolation.py",
    "test_mobile_web_coverage_contracts.py",
}
IOS_TESTS = {
    "test_ios_xcode_application_tests.py",
    "test_mobile_web_coverage_contracts.py",
}
MOBILE_TESTS = ANDROID_TESTS | IOS_TESTS


def _git_for_windows_candidates() -> list[Path]:
    """Return plausible Git-for-Windows roots without trusting the WSL bash shim."""
    candidates: list[Path] = []
    override = os.environ.get("KASKOLD_GIT_FOR_WINDOWS")
    if override:
        candidates.append(Path(override))

    git = shutil.which("git") or shutil.which("git.exe")
    if git:
        git_path = Path(git)
        for parent in (git_path.parent, *git_path.parents):
            candidates.append(parent)

    for key, suffix in (
        ("ProgramFiles", "Git"),
        ("ProgramFiles(x86)", "Git"),
        ("LOCALAPPDATA", "Programs/Git"),
    ):
        base = os.environ.get(key)
        if base:
            candidates.append(Path(base) / suffix)

    candidates.extend((
        Path.home() / "scoop/apps/git/current",
        Path.home() / "AppData/Local/Programs/Git",
    ))

    unique: list[Path] = []
    seen: set[str] = set()
    for candidate in candidates:
        key = str(candidate).casefold()
        if key not in seen:
            unique.append(candidate)
            seen.add(key)
    return unique


def _git_for_windows_tools(root: Path) -> tuple[Path, Path] | None:
    bash_candidates = (root / "bin/bash.exe", root / "usr/bin/bash.exe")
    openssl_candidates = (root / "usr/bin/openssl.exe", root / "mingw64/bin/openssl.exe")
    bash = next((path for path in bash_candidates if path.is_file()), None)
    openssl = next((path for path in openssl_candidates if path.is_file()), None)
    if bash is None or openssl is None:
        return None
    return bash, openssl


def _find_git_for_windows_root() -> Path | None:
    for candidate in _git_for_windows_candidates():
        if _git_for_windows_tools(candidate) is not None:
            return candidate
    return None


def _install_git_for_windows() -> None:
    winget = shutil.which("winget") or shutil.which("winget.exe")
    if not winget:
        raise RuntimeError(
            "Git for Windows is required by the shared cross-platform QA fixtures, but "
            "winget is unavailable. Install App Installer or Git for Windows, then rerun QA."
        )
    print("Git Bash/OpenSSL test tools are missing; installing Git.Git with winget...", flush=True)
    result = subprocess.run(
        [
            winget,
            "install",
            "--id",
            "Git.Git",
            "--exact",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ],
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"winget failed to install Git.Git with exit code {result.returncode}")


def _prepend_windows_test_tools(root: Path, environment: dict[str, str]) -> None:
    """Prefer Git Bash/OpenSSL over Windows' WSL bash compatibility launcher."""
    prefixes = (root / "bin", root / "usr/bin", root / "mingw64/bin")
    current = environment.get("PATH", "")
    parts = [part for part in current.split(os.pathsep) if part]
    folded = {part.casefold() for part in parts}
    front: list[str] = []
    for prefix in prefixes:
        text = str(prefix)
        if text.casefold() not in folded:
            front.append(text)
            folded.add(text.casefold())
    environment["PATH"] = os.pathsep.join([*front, *parts])


def ensure_windows_shared_test_tools(platform: str | None = None) -> None:
    """Make the shared Python QA fixtures self-contained on native Windows.

    The shared suite intentionally executes a few Linux shell fixtures to prove that
    Linux and Windows frontends stay in sync, and release-readiness fixtures use
    OpenSSL Ed25519 operations.  Windows' built-in ``bash.exe`` is only a WSL
    launcher, so a machine with no WSL distribution otherwise produces dozens of
    misleading test failures.  Git for Windows supplies both a native MSYS2 bash and
    OpenSSL; provision it on demand and put those tools ahead of the WSL shim only
    for this Python QA process and its children.
    """
    if (platform or sys.platform) != "win32":
        return

    root = _find_git_for_windows_root()
    if root is None:
        _install_git_for_windows()
        root = _find_git_for_windows_root()
    if root is None:
        raise RuntimeError(
            "Git for Windows was installed/probed, but native bash.exe and openssl.exe "
            "could not be located. Reopen the terminal or set KASKOLD_GIT_FOR_WINDOWS "
            "to the Git installation root."
        )

    tools = _git_for_windows_tools(root)
    assert tools is not None
    bash, openssl = tools
    _prepend_windows_test_tools(root, os.environ)
    # Do not rely on PATH for Bash on native Windows. CreateProcess/SearchPath
    # may resolve System32\bash.exe (the WSL launcher) before Git Bash even
    # when Git's bin directory is first in PATH. Cross-platform QA fixtures use
    # this absolute executable path instead.
    os.environ["KASKOLD_QA_BASH"] = str(bash.resolve())
    os.environ["KASKOLD_QA_OPENSSL"] = str(openssl.resolve())

    for command, expected in (([str(bash), "--version"], "bash"), ([str(openssl), "version"], "openssl")):
        probe = subprocess.run(command, text=True, capture_output=True, check=False)
        if probe.returncode != 0:
            detail = (probe.stdout + probe.stderr).strip()
            raise RuntimeError(f"native Windows {expected} probe failed: {detail}")

    print(f"Windows shared QA shell: {bash}", flush=True)
    print(f"Windows shared QA OpenSSL: {openssl}", flush=True)


def selected_files(scope: str) -> list[Path]:
    files = sorted(path for root in TEST_ROOTS for path in root.glob("test_*.py"))
    if scope == "shared":
        return [path for path in files if path.name not in MOBILE_TESTS]
    wanted = ANDROID_TESTS if scope == "android" else IOS_TESTS
    return [path for path in files if path.name in wanted]




class TeeStream:
    """Mirror unittest output to the terminal and a persistent QA log."""

    def __init__(self, *streams: object) -> None:
        self.streams = streams

    def write(self, text: str) -> int:
        for stream in self.streams:
            stream.write(text)
        return len(text)

    def flush(self) -> None:
        for stream in self.streams:
            stream.flush()


def _test_id(test: object) -> str:
    identifier = getattr(test, "id", None)
    if callable(identifier):
        try:
            return str(identifier())
        except Exception:
            pass
    return str(test)


def _last_detail_line(detail: str) -> str:
    lines = [line.strip() for line in detail.splitlines() if line.strip()]
    return lines[-1] if lines else "no traceback detail available"


class SkippedModuleImport(unittest.TestCase):
    """Represent a module-level SkipTest as a normal unittest skip."""

    def __init__(self, path: Path, reason: str) -> None:
        super().__init__("runTest")
        try:
            self.display_path = path.relative_to(ROOT).as_posix()
        except ValueError:
            self.display_path = str(path)
        self.reason = reason

    def runTest(self) -> None:
        self.skipTest(self.reason)

    def shortDescription(self) -> str:
        return f"{self.display_path} (module import)"


def load_file(loader: unittest.TestLoader, path: Path, index: int) -> unittest.TestSuite:
    module_name = f"kaskold_repository_qa_{index}_{path.stem}"
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load Python QA module: {path.relative_to(ROOT)}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    try:
        spec.loader.exec_module(module)
    except unittest.SkipTest as exc:
        sys.modules.pop(module_name, None)
        return unittest.TestSuite([SkippedModuleImport(path, str(exc))])
    except BaseException:
        sys.modules.pop(module_name, None)
        raise
    return loader.loadTestsFromModule(module)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scope", choices=("shared", "android", "ios"), required=True)
    args = parser.parse_args(argv)

    if args.scope == "shared":
        try:
            ensure_windows_shared_test_tools()
        except (OSError, RuntimeError) as exc:
            print(f"ERROR: Windows shared QA dependency bootstrap failed: {exc}", flush=True)
            return 1

    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    files = selected_files(args.scope)
    log_dir = ROOT / "target/qa/repository-python-qa"
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / f"{args.scope}.log"

    with log_path.open("w", encoding="utf-8", newline="\n") as log_file:
        # unittest defaults to stderr.  Some nested Windows launchers only retain
        # stdout reliably, so mirror the complete verbose stream to stdout and a
        # deterministic on-disk log instead of relying on inherited stderr.
        stream = TeeStream(sys.stdout, log_file)
        print(
            f"Repository Python QA scope {args.scope}: {len(files)} modules",
            file=stream,
            flush=True,
        )
        print(
            f"Detailed log: {log_path.relative_to(ROOT)}",
            file=stream,
            flush=True,
        )
        try:
            for index, path in enumerate(files):
                suite.addTests(load_file(loader, path, index))
        except BaseException:
            print("\nERROR: repository Python QA module import failed", file=stream)
            traceback.print_exc(file=stream)
            print(f"Full log: {log_path}", file=stream, flush=True)
            return 1

        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
        if result.wasSuccessful():
            print(f"PASS: detailed repository Python QA log: {log_path}", file=stream, flush=True)
            return 0

        failures = [*result.failures, *result.errors]
        print("\nFAILED TEST SUMMARY", file=stream)
        print("=" * 80, file=stream)
        for test, detail in failures:
            print(f"- {_test_id(test)}", file=stream)
            print(f"  {_last_detail_line(detail)}", file=stream)
        print("=" * 80, file=stream)
        print(f"Full log: {log_path}", file=stream, flush=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
