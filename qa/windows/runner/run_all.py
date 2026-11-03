#!/usr/bin/env python3
"""Native Windows implementation of the KasKold master QA catalog."""
from __future__ import annotations

import argparse
from collections.abc import Callable, Sequence
from typing import IO
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from msvc_asan import with_asan_path  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
PYTHON = sys.executable
STEP_SKIPPED = False


def clean_cargo_fuzz_source_scratch() -> None:
    for relative in ("qa/fuzz/artifacts", "qa/fuzz/corpus"):
        shutil.rmtree(ROOT / relative, ignore_errors=True)

CATALOG = ROOT / "qa/config/run_all_steps.tsv"

def load_catalog() -> tuple[list[tuple[str, str, str, str]], dict[str, str]]:
    steps: list[tuple[str, str, str, str]] = []
    scopes: dict[str, str] = {}
    for raw in CATALOG.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith("#"):
            continue
        scope, category, workspace, step_id, description = raw.split("\t", 4)
        if step_id in scopes:
            raise RuntimeError(f"duplicate QA catalog step id: {step_id}")
        steps.append((category, workspace, step_id, description))
        scopes[step_id] = scope
    return steps, scopes

STEPS, STEP_SCOPES = load_catalog()

TEST_FILTER_STEPS = {
"unit.shared-signer","unit.kaskold-hardware-core","unit.offline-signer","unit.online-watcher","unit.kaskold-companion-web","unit.kaskold-vault-web","unit.external-rqrr","unit.tools",
"integration.shared-signer-conformance","integration.repository-layout","integration.offline-signer-firmware-signing",
}
WORKSPACES = {"signer-firmware","online-watcher","offline-signer","shared-signer","kaskold-hardware-core","external-rqrr","tools","repository","kaskold-companion-web","kaskold-vault-web"}
CATEGORIES = {"preflight","unit","integration","static","security","coverage","interactive","emulation","hardware","bench","mutation","fuzz"}



def load_toolchains() -> None:
    # Repository sources, docs, fixtures, and generated QA evidence are UTF-8.
    # Force child Python processes into UTF-8 mode so native Windows locale
    # code pages cannot corrupt reads/writes in the platform-neutral test suite.
    os.environ["PYTHONUTF8"] = "1"
    os.environ["PYTHONIOENCODING"] = "utf-8"
    os.environ["PYTHONUNBUFFERED"] = "1"
    for raw in (ROOT / "qa/config/toolchains.env").read_text(encoding="utf-8").splitlines():
        line=raw.strip()
        if not line or line.startswith("#"): continue
        key,value=line.split("=",1); os.environ[key]=value
    cargo_home=Path(os.environ.get("CARGO_HOME", Path.home()/".cargo"))
    os.environ["PATH"]=str(cargo_home/"bin")+os.pathsep+os.environ.get("PATH","")


def canonical_id(text: str) -> str:
    return {"unit.architecture-imports":"unit.repository-python-qa","fuzz.shared-signer-qr-payload":"fuzz.repository-security-targets"}.get(text,text)


def command_exists(name: str) -> bool: return shutil.which(name) is not None

def _prepend_path(directory: Path, environment: dict[str, str] | None = None) -> dict[str, str]:
    target = dict(environment or {})
    current = target.get("PATH", os.environ.get("PATH", ""))
    entries = current.split(os.pathsep) if current else []
    directory_text = str(directory)
    folded = {entry.casefold() for entry in entries}
    if directory_text.casefold() not in folded:
        current = directory_text + (os.pathsep + current if current else "")
    target["PATH"] = current
    return target


def _local_cargo_tool_root(kind: str, toolchain: str, version: str) -> Path:
    safe_toolchain = toolchain.replace("/", "-").replace("\\", "-")
    return ROOT / "target/development-tools" / f"{kind}-{safe_toolchain}-{version}"


def mutation_tool_environment() -> dict[str, str]:
    toolchain = os.environ["KASKOLD_STABLE_RUST"]
    version = os.environ["KASKOLD_CARGO_MUTANTS_VERSION"]
    root = _local_cargo_tool_root("cargo-mutants", toolchain, version)
    bin_dir = root / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    env = _prepend_path(bin_dir)
    env["CARGO_INSTALL_ROOT"] = str(root)
    return env


def configure_fuzz_tool_environment() -> tuple[Path, Path]:
    toolchain = os.environ["KASKOLD_BRANCH_RUST"]
    version = os.environ["KASKOLD_CARGO_FUZZ_VERSION"]
    root = _local_cargo_tool_root("cargo-fuzz", toolchain, version)
    bin_dir = root / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    os.environ.update(_prepend_path(bin_dir))
    os.environ["CARGO_INSTALL_ROOT"] = str(root)
    return root, bin_dir


def require(name: str, dry: bool=False) -> None:
    if not dry and not command_exists(name):
        if name=="cargo": raise RuntimeError("required command not found: cargo. Install Rust with rustup for Windows and reopen PowerShell.")
        raise RuntimeError(f"required command not found: {name}")


def run(cmd: list[str], *, cwd: Path=ROOT, env: dict[str,str]|None=None, dry: bool=False, allowed=(0,)) -> int:
    print("  +", subprocess.list2cmdline(cmd))
    if dry: return 0
    merged=os.environ.copy(); merged.update(env or {})
    code=subprocess.run(cmd,cwd=cwd,env=merged,check=False).returncode
    if code not in allowed: raise subprocess.CalledProcessError(code,cmd)
    return code


def capture(cmd: list[str], *, cwd: Path=ROOT, env: dict[str,str]|None=None) -> subprocess.CompletedProcess[str]:
    merged=os.environ.copy(); merged.update(env or {})
    return subprocess.run(cmd,cwd=cwd,env=merged,text=True,capture_output=True,check=False)


def _replay_qemu_log(path: Path, *, max_bytes: int = 131072) -> None:
    if not path.is_file():
        print(f"  ! QEMU diagnostic log not found: {path}", file=sys.stderr, flush=True)
        return
    try:
        data = path.read_bytes()[-max_bytes:]
    except OSError as error:
        print(f"  ! Could not read QEMU diagnostic log {path}: {error}", file=sys.stderr, flush=True)
        return
    print(f"\n===== persisted QEMU diagnostics: {path} =====", file=sys.stderr, flush=True)
    sys.stderr.write(data.decode("utf-8", errors="replace"))
    if data and not data.endswith((b"\n", b"\r")):
        sys.stderr.write("\n")
    print("===== end persisted QEMU diagnostics =====\n", file=sys.stderr, flush=True)


def run_qemu_emulation(*, dry: bool = False) -> int:
    cmd = [PYTHON, "scripts/common/lib/make_tasks.py", "entrypoint", "qemu-test"]
    print("  +", subprocess.list2cmdline(cmd), flush=True)
    if dry:
        return 0

    log_dir = ROOT / "target/qa/qemu"
    log_dir.mkdir(parents=True, exist_ok=True)
    master_log = log_dir / "windows-qemu-master.log"
    merged = os.environ.copy()
    merged["PYTHONUNBUFFERED"] = "1"

    with master_log.open("wb") as handle:
        process = subprocess.Popen(
            cmd,
            cwd=ROOT,
            env=merged,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            bufsize=0,
        )
        assert process.stdout is not None
        while True:
            chunk = process.stdout.read(4096)
            if not chunk:
                break
            handle.write(chunk)
            handle.flush()
            try:
                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
            except AttributeError:
                sys.stdout.write(chunk.decode("utf-8", errors="replace"))
                sys.stdout.flush()
        code = process.wait()

    if code != 0:
        print(
            f"ERROR: QEMU emulation subprocess exited with status {code}. "
            "Replaying every persisted Windows QEMU diagnostic log now.",
            file=sys.stderr,
            flush=True,
        )
        for path in (
            master_log,
            log_dir / "windows-qemu.log",
            log_dir / "windows-qemu-powershell.log",
        ):
            _replay_qemu_log(path)
        raise subprocess.CalledProcessError(code, cmd)
    return code


def metadata(directory: Path, manifest: str, extra: list[str]) -> subprocess.CompletedProcess[str]:
    return capture(["cargo","metadata","--manifest-path",manifest,"--format-version","1",*extra],cwd=directory)


def ensure_esp_rust_toolchain(dry: bool) -> None:
    """Provision the repository-pinned ESP Rust toolchain when Windows QA needs it."""
    helper = ROOT / "scripts/windows/lib/qemu-common.ps1"
    command = (
        ". '" + str(helper).replace("'", "''") + "'; "
        "Install-KasKoldRustupIfMissing; Install-KasKoldEspRustToolchain"
    )
    run(
        [
            "powershell.exe",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            command,
        ],
        dry=dry,
    )


def _restore_lock(lock: Path, existed: bool, backup: bytes | None) -> None:
    if existed: lock.write_bytes(backup or b"")
    elif lock.exists(): lock.unlink()


def _refresh_lock(directory: Path, manifest: str, lock: Path, existed: bool, backup: bytes | None) -> None:
    offline=metadata(directory,manifest,["--offline"])
    if offline.returncode!=0:
        _restore_lock(lock, existed, backup)
        print("  ! Offline refresh was unavailable; retrying with registry access.")
        online=metadata(directory,manifest,[])
        if online.returncode!=0:
            _restore_lock(lock, existed, backup)
            raise RuntimeError(f"Cargo could not refresh {lock}.\n{offline.stderr}\n{online.stderr}")
    final=metadata(directory,manifest,["--locked"])
    if final.returncode!=0:
        _restore_lock(lock, existed, backup)
        raise RuntimeError(f"refreshed lockfile still does not resolve under --locked: {lock}\n{final.stderr}")


def reconcile_lock(directory: Path, manifest: str, repair: bool, dry: bool) -> None:
    require("cargo",dry)
    print("  +", subprocess.list2cmdline(["cargo","metadata","--manifest-path",manifest,"--format-version","1","--locked"]))
    if dry:return
    first=metadata(directory,manifest,["--locked"])
    if first.returncode==0:return
    sys.stderr.write(first.stderr)
    if not repair: raise RuntimeError("locked Cargo graph is stale and --strict-lockfiles was requested")
    lock=directory/"Cargo.lock"; existed=lock.exists(); backup=lock.read_bytes() if existed else None
    print(f"  ! Locked graph is stale; refreshing {lock} transactionally.")
    _refresh_lock(directory, manifest, lock, existed, backup)
    print(f"  ! Refreshed and verified: {lock}")


def cargo_test(extra: list[str], ns: argparse.Namespace) -> None:
    cmd=["cargo","test",*extra]
    if ns.test_filter:
        cmd.append(ns.test_filter)
        if ns.exact: cmd += ["--","--exact"]
    env = {"CARGO_TARGET_DIR": str(ROOT / "target/qa")} if "qa/Cargo.toml" in extra else None
    run(cmd, env=env, dry=ns.dry_run)


def ensure_fuzz_toolchain(dry: bool) -> None:
    require("rustup",dry); require("cargo",dry)
    tool_root, tool_bin = configure_fuzz_tool_environment()
    if dry:
        return
    stable=os.environ["KASKOLD_STABLE_RUST"]; nightly=os.environ["KASKOLD_BRANCH_RUST"]; ver=os.environ["KASKOLD_CARGO_FUZZ_VERSION"]
    for tc in (stable,nightly):
        if capture(["rustup","run",tc,"rustc","--version"]).returncode:
            run(["rustup","toolchain","install",tc,"--profile","minimal","--no-self-update"])
    # A direct --resume-from fuzz invocation must be self-contained. The earlier
    # branch-coverage gate normally installs LLVM tooling, but fuzz cannot rely
    # on a prior stage having run in this process/session. rustup is idempotent
    # when the pinned component is already present.
    run(["rustup","component","add","llvm-tools-preview","--toolchain",nightly])
    actual=capture(["rustup","run",nightly,"cargo","fuzz","--version"])
    if actual.returncode or f"cargo-fuzz {ver}" not in actual.stdout+actual.stderr:
        install=["rustup","run",stable,"cargo","install","cargo-fuzz","--version",ver,"--locked","--root",str(tool_root)]
        executable = tool_bin / ("cargo-fuzz.exe" if os.name == "nt" else "cargo-fuzz")
        if executable.is_file():
            install.append("--force")
        run(install)
        actual=capture(["rustup","run",nightly,"cargo","fuzz","--version"])
        if actual.returncode or f"cargo-fuzz {ver}" not in actual.stdout+actual.stderr:
            raise RuntimeError(f"expected cargo-fuzz {ver} in repository-local QA tooling")


def run_fuzz(ns: argparse.Namespace) -> None:
    ensure_fuzz_toolchain(ns.dry_run)
    if ns.fuzz_target:
        targets = [ns.fuzz_target]
    elif ns.dry_run:
        # Keep --dry-run completely host-tool-independent while still showing
        # the registered fuzz stage. Registry validity is enforced by normal
        # execution and repository tests.
        targets = ["<registered-fuzz-targets>"]
    else:
        registry = capture([PYTHON, "qa/checks/security/fuzz_targets.py", "--validate"])
        if registry.returncode:
            detail = (registry.stdout + registry.stderr).strip()
            raise RuntimeError(f"fuzz target registry validation failed: {detail}")
        targets = [line.strip() for line in registry.stdout.splitlines() if line.strip()]
    if not targets:
        raise RuntimeError("no fuzz targets are registered")
    state = ROOT / "target/qa/fuzz"
    status_file = state / "statuses.tsv"
    artifact_root = state / "artifacts"
    corpus_root = state / "corpus"
    actual = (
        f"cargo-fuzz {os.environ['KASKOLD_CARGO_FUZZ_VERSION']}"
        if ns.dry_run
        else capture(["rustup", "run", os.environ["KASKOLD_BRANCH_RUST"], "cargo", "fuzz", "--version"]).stdout.strip()
    )
    if ns.dry_run:
        for target in targets:
            run(
                [
                    "rustup", "run", os.environ["KASKOLD_BRANCH_RUST"], "cargo", "fuzz", "run", target,
                    "--", f"-runs={ns.fuzz_passes}",
                    f"-artifact_prefix={artifact_root / target}{os.sep}", str(corpus_root / target),
                ],
                cwd=ROOT / "qa/fuzz", env={"CARGO_TARGET_DIR": str(ROOT / "target/qa")}, dry=True,
            )
        print(
            "  +",
            subprocess.list2cmdline(
                [
                    PYTHON, "qa/checks/security/fuzz_results.py", "--statuses", str(status_file),
                    "--tool", actual, "--started", "<utc>", "--completed", "<utc>",
                    "--runs", str(ns.fuzz_passes),
                ]
            ),
        )
        return

    shutil.rmtree(state, ignore_errors=True)
    artifact_root.mkdir(parents=True)
    corpus_root.mkdir(parents=True)
    statuses: list[str] = []
    started = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    for target in targets:
        seed = ROOT / "qa/fuzz/seeds" / target
        corpus = corpus_root / target
        artifacts = artifact_root / target
        log = state / f"{target}.log"
        if not seed.is_dir():
            print(f"ERROR: authored fuzz seeds are missing for {target}: {seed}", file=sys.stderr)
            statuses.append(f"{target}\t2\n")
            continue
        corpus.mkdir(parents=True)
        artifacts.mkdir(parents=True)
        for item in seed.iterdir():
            dst = corpus / item.name
            shutil.copytree(item, dst) if item.is_dir() else shutil.copy2(item, dst)
        command = [
            "rustup", "run", os.environ["KASKOLD_BRANCH_RUST"], "cargo", "fuzz", "run", target,
            "--", f"-runs={ns.fuzz_passes}",
            f"-artifact_prefix={artifacts}{os.sep}", str(corpus),
        ]
        print(f"=== fuzz: {target} ({ns.fuzz_passes} runs) ===")
        print("  +", subprocess.list2cmdline(command))
        with log.open("w", encoding="utf-8") as output:
            fuzz_env = with_asan_path(dict(os.environ))
            fuzz_env["CARGO_TARGET_DIR"] = str(ROOT / "target/qa")
            process = subprocess.Popen(
                command, cwd=ROOT / "qa/fuzz", env=fuzz_env, stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT, text=True, bufsize=1,
            )
            assert process.stdout is not None
            for line in process.stdout:
                output.write(line)
                output.flush()
                print(line, end="")
            status = process.wait()
        statuses.append(f"{target}\t{status}\n")
        if status != 0:
            print(f"FAIL: fuzz target {target} (exit {status})", file=sys.stderr)
    status_file.write_text("".join(statuses), encoding="ascii")
    completed = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    run(
        [
            PYTHON, "qa/checks/security/fuzz_results.py", "--statuses", str(status_file),
            "--tool", actual, "--started", started, "--completed", completed,
            "--runs", str(ns.fuzz_passes),
        ]
    )



def core_ci_toolchain_ready(toolchain: str) -> bool:
    probes = (
        ["rustup", "run", toolchain, "rustc", "--version"],
        ["rustup", "run", toolchain, "rustfmt", "--version"],
        ["rustup", "run", toolchain, "cargo", "clippy", "--version"],
    )
    return all(
        subprocess.run(
            probe,
            cwd=ROOT,
            env=os.environ.copy(),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode == 0
        for probe in probes
    )


class _CoreCiLog:
    """Console and log-file writer for one core-ci run."""

    def __init__(self, output, log_path: Path) -> None:
        self.output = output
        self.log_path = log_path

    def emit(self, message: str = "") -> None:
        print(message)
        self.output.write(message + "\n")
        self.output.flush()

    def run_logged(self, command: list[str]) -> int:
        self.emit("+ " + subprocess.list2cmdline(command))
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            env=os.environ.copy(),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
        assert process.stdout is not None
        for line in process.stdout:
            print(line, end="")
            self.output.write(line)
            self.output.flush()
        return process.wait()

    def fail(self, stage: str, status: int) -> None:
        self.emit()
        self.emit(f"FAILED: {stage} (exit {status})")
        self.emit()
        self.emit(f"Core CI child exit code: {status}")
        self.emit(f"Full log: {self.log_path}")
        raise subprocess.CalledProcessError(status, stage)


_CORE_CI_GATES = ("FORMAT", "CLIPPY", "QA CLIPPY", "TOOLS CLIPPY", "VAULT WEB CLIPPY", "TEST")


def _core_ci_commands(stable: str) -> list[list[str]]:
    return [
        ["rustup", "toolchain", "install", stable, "--profile", "minimal", "--no-self-update", "--component", "rustfmt", "--component", "clippy"],
        [PYTHON, "qa/checks/workspace/check_rust_format.py"],
        ["rustup", "run", stable, "cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["rustup", "run", stable, "cargo", "clippy", "--manifest-path", "qa/Cargo.toml", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["rustup", "run", stable, "cargo", "clippy", "--manifest-path", "tools/Cargo.toml", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["rustup", "run", stable, "cargo", "clippy", "--manifest-path", "apps/kaskold-vault-web/Cargo.toml", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["make", "test", "STRICT_LOCKFILES=1"],
        ["git", "diff", "--check"],
    ]


def _print_core_ci_plan(log_path: Path, stable: str, commands: list[list[str]]) -> None:
    print(f"  + Core CI log: {log_path}")
    print(f"  + reuse local rustup {stable} toolchain when rustc, rustfmt, and clippy are already available")
    print("  +", subprocess.list2cmdline(commands[0]), "# only when missing locally")
    for command in commands[1:]:
        print("  +", subprocess.list2cmdline(command))


def _check_worktree_state(log: _CoreCiLog) -> None:
    status = log.run_logged(["git", "rev-parse", "--show-toplevel"])
    if status != 0:
        log.fail("repository check", status)
    status = log.run_logged(["git", "status", "--short"])
    if status != 0:
        log.fail("repository status", status)


def _capture_archive_baseline(log: _CoreCiLog, log_dir: Path) -> Path:
    archive_git_dir = log_dir / "archive-baseline.git"
    if archive_git_dir.exists():
        shutil.rmtree(archive_git_dir)
    baseline_prefix = [
        "git",
        f"--git-dir={archive_git_dir}",
        f"--work-tree={ROOT}",
    ]
    for command in (
        baseline_prefix + ["init", "-q"],
        baseline_prefix + ["config", "core.autocrlf", "false"],
        baseline_prefix + ["add", "-A"],
    ):
        status = log.run_logged(command)
        if status != 0:
            log.fail("archive baseline", status)
    log.emit("Extracted source archive detected; captured an external Git index baseline for the final diff check.")
    return archive_git_dir


def _record_repository_state(log: _CoreCiLog, log_dir: Path) -> Path | None:
    """Log repository state; return the archive baseline git dir when not in a worktree."""
    log.emit("===== REPOSITORY =====")
    log.emit(str(ROOT))
    in_worktree = subprocess.run(
        ["git", "rev-parse", "--is-inside-work-tree"],
        cwd=ROOT,
        env=os.environ.copy(),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    ).returncode == 0
    if in_worktree:
        _check_worktree_state(log)
        return None
    return _capture_archive_baseline(log, log_dir)


def _ensure_core_toolchain(log: _CoreCiLog, stable: str, install_command: list[str]) -> None:
    log.emit("===== ENSURE PINNED CORE TOOLCHAIN =====")
    if core_ci_toolchain_ready(stable):
        log.emit("Pinned core toolchain and required components are already installed; reusing local installation.")
        return
    log.emit(f"Pinned core toolchain or required components are missing; installing {stable}.")
    status = log.run_logged(install_command)
    if status != 0:
        log.emit("The pinned toolchain/components were not available locally and could not be installed.")
        log.fail("toolchain install", status)


def _run_gate(log: _CoreCiLog, header: str, stage: str, command: list[str]) -> None:
    log.emit()
    log.emit(header)
    status = log.run_logged(command)
    if status != 0:
        log.fail(stage, status)


def _final_diff_command(archive_git_dir: Path | None, default: list[str]) -> list[str]:
    if archive_git_dir is None:
        return default
    return [
        "git",
        f"--git-dir={archive_git_dir}",
        f"--work-tree={ROOT}",
        "diff",
        "--check",
    ]


def _run_final_diff_check(log: _CoreCiLog, archive_git_dir: Path | None, default: list[str]) -> None:
    log.emit()
    log.emit("===== FINAL DIFF CHECK =====")
    status = log.run_logged(_final_diff_command(archive_git_dir, default))
    if status != 0:
        log.fail("git diff --check", status)


def _emit_core_ci_passed(log: _CoreCiLog) -> None:
    log.emit()
    log.emit("================================")
    log.emit("ALL CORE CI GATES PASSED LOCALLY")
    log.emit("================================")
    log.emit()
    log.emit("Core CI child exit code: 0")
    log.emit(f"Full log: {log.log_path}")


def run_core_ci(ns: argparse.Namespace) -> None:
    log_dir = ROOT / "target/qa/core-ci"
    log_path = log_dir / "core-ci.log"
    log_dir.mkdir(parents=True, exist_ok=True)
    stable = os.environ["KASKOLD_STABLE_RUST"]
    commands = _core_ci_commands(stable)
    if ns.dry_run:
        _print_core_ci_plan(log_path, stable, commands)
        return

    for required in ("rustup", "cargo", "git", "make"):
        require(required)

    with log_path.open("w", encoding="utf-8", newline="") as output:
        log = _CoreCiLog(output, log_path)
        archive_git_dir = _record_repository_state(log, log_dir)
        log.emit()
        log.emit(f"KASKOLD_STABLE_RUST={stable}")
        log.emit()
        _ensure_core_toolchain(log, stable, commands[0])
        for label, command in zip(_CORE_CI_GATES, commands[1:-1], strict=True):
            _run_gate(log, f"===== CORE CI: {label} =====", f"CORE {label}", command)
        _run_final_diff_check(log, archive_git_dir, commands[-1])
        _emit_core_ci_passed(log)


StepHandler = Callable[[argparse.Namespace], None]
MAKE_TASKS = "scripts/common/lib/make_tasks.py"


def _script(*args: str) -> StepHandler:
    def handler(ns: argparse.Namespace) -> None:
        run([PYTHON, *args], dry=ns.dry_run)
    return handler


def _entrypoint(name: str) -> StepHandler:
    return _script(MAKE_TASKS, "entrypoint", name)


def _commands(commands: list[list[str]]) -> StepHandler:
    def handler(ns: argparse.Namespace) -> None:
        for cmd in commands:
            run(cmd, dry=ns.dry_run)
    return handler


def _sequence(*handlers: StepHandler) -> StepHandler:
    def handler(ns: argparse.Namespace) -> None:
        for step in handlers:
            step(ns)
    return handler


def _cargo_test(args: list[str]) -> StepHandler:
    def handler(ns: argparse.Namespace) -> None:
        cargo_test(args, ns)
    return handler


def _mutation(*args: str) -> StepHandler:
    def handler(ns: argparse.Namespace) -> None:
        run([PYTHON, "qa/checks/security/mutation.py", *args], env=mutation_tool_environment(), dry=ns.dry_run)
    return handler


def _ensure_esp_toolchain(ns: argparse.Namespace) -> None:
    ensure_esp_rust_toolchain(ns.dry_run)


def _cargo_check_firmware(features: str) -> StepHandler:
    def handler(ns: argparse.Namespace) -> None:
        run(["cargo", "check", "--locked", "--no-default-features", "--features", features],
            cwd=ROOT / "apps/kaskold-hardware", env={"ESP_HAL_CONFIG_PSRAM_MODE": "octal"}, dry=ns.dry_run)
    return handler


def _reconcile_cargo_resolution(ns: argparse.Namespace) -> None:
    for directory, manifest in [
        (ROOT, "Cargo.toml"),
        (ROOT / "apps/kaskold-hardware", "Cargo.toml"),
        (ROOT / "apps/kaskold-companion-web", "Cargo.toml"),
        (ROOT / "apps/kaskold-vault-web", "Cargo.toml"),
        (ROOT / "tools", "Cargo.toml"),
        (ROOT / "qa", "Cargo.toml"),
    ]:
        if directory == ROOT / "apps/kaskold-hardware":
            ensure_esp_rust_toolchain(ns.dry_run)
        reconcile_lock(directory, manifest, ns.repair_lockfiles, ns.dry_run)


def _funded_testnet_e2e(ns: argparse.Namespace) -> None:
    global STEP_SKIPPED
    code = run([PYTHON, MAKE_TASKS, "entrypoint", "funded-testnet-e2e"], dry=ns.dry_run, allowed=(0, 77))
    if code == 77:
        STEP_SKIPPED = True
        print("  ~ SKIP: funded testnet E2E requires an interactive maintainer terminal.")


def _run_qemu(ns: argparse.Namespace) -> None:
    run_qemu_emulation(dry=ns.dry_run)


def _hardware_device(ns: argparse.Namespace) -> None:
    cmd = [PYTHON, "qa/checks/firmware/run_hardware_tests.py", "--board", ns.hardware, "--timeout", str(ns.hardware_timeout)]
    if ns.hardware_port:
        cmd += ["--port", ns.hardware_port]
    run(cmd, dry=ns.dry_run)


def _bench_throughput(ns: argparse.Namespace) -> None:
    reconcile_lock(ROOT / "qa", "Cargo.toml", ns.repair_lockfiles, ns.dry_run)
    run(["cargo", "bench", "--manifest-path", "qa/Cargo.toml", "--bench", "protocol_throughput", "--locked"],
        env={"CARGO_TARGET_DIR": str(ROOT / "target/qa")}, dry=ns.dry_run)


def _preflight_handlers() -> dict[str, StepHandler]:
    return {
        "preflight.companion-build": _entrypoint("kaskold-companion-web-build"),
        "preflight.vault-web-build": _sequence(
            _script("qa/checks/web/check_vault_web_architecture.py"),
            _entrypoint("kaskold-vault-web-build"),
        ),
        "preflight.firmware-source-contracts": _script("qa/checks/firmware/check_firmware_source_contracts.py"),
        "preflight.repository-lockfiles": _script("qa/checks/workspace/check_lockfile.py"),
        "preflight.dependency-advisories": _script("qa/checks/security/dependency_advisories.py"),
        "preflight.crap-check": _entrypoint("pinned-branch-coverage"),
        "preflight.core-ci": run_core_ci,
        "preflight.security-assurance": _commands([
            [PYTHON, "qa/checks/security/security_invariants.py"],
            [PYTHON, "qa/checks/security/watcher_only_apps.py"],
            [PYTHON, "qa/checks/security/irreversible_action_policy.py"],
            [PYTHON, "qa/checks/security/test_quality.py"],
            [PYTHON, "qa/checks/security/repository_test_quality.py"],
            [PYTHON, "qa/checks/security/security_control_evidence.py"],
        ]),
        "preflight.cargo-resolution": _reconcile_cargo_resolution,
    }


def _unit_handlers() -> dict[str, StepHandler]:
    return {
        "unit.shared-signer": _cargo_test(["--manifest-path", "Cargo.toml", "-p", "shared-signer", "--all-features", "--locked"]),
        "unit.kaskold-hardware-core": _cargo_test(["--manifest-path", "Cargo.toml", "-p", "kaskold-hardware-core", "--all-features", "--locked"]),
        "unit.offline-signer": _cargo_test(["--manifest-path", "Cargo.toml", "-p", "offline-signer", "--all-features", "--locked"]),
        "unit.online-watcher": _cargo_test(["--manifest-path", "Cargo.toml", "-p", "online-watcher", "--all-features", "--locked"]),
        "unit.kaskold-companion-web": _cargo_test(["--manifest-path", "apps/kaskold-companion-web/Cargo.toml", "--lib", "--locked"]),
        "unit.kaskold-vault-web": _cargo_test(["--manifest-path", "apps/kaskold-vault-web/Cargo.toml", "--lib", "--locked"]),
        "unit.signer-firmware": _sequence(
            _ensure_esp_toolchain,
            _cargo_check_firmware("m5stack,verbose-boot"),
            _cargo_check_firmware("m5stack-lite,verbose-boot"),
        ),
        "unit.external-rqrr": _cargo_test(["--manifest-path", "external/rqrr-nostd/Cargo.toml", "--all-features", "--locked"]),
        "unit.tools": _cargo_test(["--manifest-path", "tools/Cargo.toml", "--lib", "--bins", "--locked"]),
        "unit.repository-python-qa": _script("qa/checks/workspace/run_repository_python_qa.py", "--scope", "shared"),
    }


def _integration_handlers() -> dict[str, StepHandler]:
    return {
        "integration.shared-signer-conformance": _cargo_test(["--manifest-path", "qa/Cargo.toml", "--test", "conformance", "--locked"]),
        "integration.repository-layout": _cargo_test(["--manifest-path", "qa/Cargo.toml", "--test", "integration", "--locked"]),
        "integration.offline-signer-firmware-signing": _cargo_test(["--manifest-path", "qa/Cargo.toml", "--test", "tooling_firmware_signing", "--locked"]),
        "integration.online-watcher-source": _script("qa/checks/web/check_web_javascript.py"),
        "integration.kaskold-companion-web-generated": _commands([
            [PYTHON, "tools/build/web/build_web_index.py", "--check"],
            [PYTHON, "tools/build/web/build_app_css.py", "--check"],
            [PYTHON, "qa/checks/web/check_web_dom_contract.py"],
            [PYTHON, "qa/checks/web/check_safe_html.py"],
            ["node", "--test", "qa/checks/web/wallet/vault_saved_wallets.test.mjs"],
            ["node", "qa/checks/web/transactions/network_routing.test.mjs"],
        ]),
        "integration.kaskold-companion-web-browser": _commands([
            ["node", "qa/checks/web/runtime/check_web_runtime.mjs"],
            ["node", "qa/checks/web/covenants/check_web_covenant_interactions.mjs"],
            ["node", "qa/checks/web/covenants/covenant_sign_protocol.test.mjs"],
            ["node", "qa/checks/web/runtime/check_web_critical_paths.mjs"],
            [PYTHON, "qa/checks/web/real_dom/run_real_dom_tests.py"],
            ["node", "--test", "qa/checks/web/real_wasm/contract.test.mjs"],
            ["node", "--test", "qa/checks/web/real_wasm/spend_documents.test.mjs"],
            ["node", "--test", "qa/checks/web/real_wasm/differential.test.mjs"],
        ]),
        "integration.real-node": _entrypoint("real-node-integration"),
        "integration.funded-testnet-e2e": _funded_testnet_e2e,
        "integration.signer-firmware-builds": _sequence(
            _ensure_esp_toolchain,
            _script("qa/checks/firmware/check_firmware_builds.py"),
        ),
        "integration.firmware-simulator": _script("qa/checks/firmware/run_firmware_simulator.py"),
        "integration.signer-firmware-lints": _sequence(
            _ensure_esp_toolchain,
            _script("qa/checks/firmware/check_firmware_lints.py"),
        ),
        "integration.repository-architecture": _script("qa/checks/check_architecture.py"),
    }


def _assurance_handlers() -> dict[str, StepHandler]:
    return {
        "static.language-lints": _script("qa/checks/quality/language_lints.py"),
        "static.qa-orchestration-catalog": _script("qa/checks/workspace/check_qa_orchestration.py"),
        "static.firmware-assurance-contracts": _commands([
            [PYTHON, script]
            for script in (
                "qa/checks/firmware/board_partition_contract.py",
                "qa/checks/firmware/m5stack_production_security.py",
                "qa/checks/firmware/production_e2e_coverage.py",
                "qa/checks/firmware/production_runtime_qualification.py",
                "qa/checks/firmware/production_ui_graph.py",
                "qa/checks/firmware/wallet_recovery_contract.py",
            )
        ]),
        "coverage.critical-branch-targets": _commands([
            [PYTHON, "qa/checks/security/branch_ratchets.py"],
            [PYTHON, "qa/checks/security/branch_ratchets.py", "--require-target"],
        ]),
        "mutation.repository-security-fresh": _mutation("run", "--fresh"),
        "mutation.repository-crypto-certification": _mutation("crypto-check"),
        "emulation.signer-firmware-qemu": _run_qemu,
        "hardware.signer-firmware-device": _hardware_device,
        "bench.shared-signer-protocol-throughput": _bench_throughput,
        "fuzz.repository-security-targets": run_fuzz,
    }


def _step_handlers() -> dict[str, StepHandler]:
    return {
        **_preflight_handlers(),
        **_unit_handlers(),
        **_integration_handlers(),
        **_assurance_handlers(),
    }


def run_step(step_id: str, ns: argparse.Namespace) -> None:
    global STEP_SKIPPED
    STEP_SKIPPED = False
    handler = _step_handlers().get(step_id)
    if handler is None:
        raise RuntimeError(f"unknown catalog step: {step_id}")
    handler(ns)


def parser() -> argparse.ArgumentParser:
    p=argparse.ArgumentParser(description="Run the complete KasKold test catalog in a stable, resumable order.")
    p.add_argument("--profile",choices=("full","test"),default="full"); p.add_argument("--list",action="store_true",dest="list_only"); p.add_argument("--resume-from","--from",dest="resume_from",default=""); p.add_argument("--only","--section",dest="only",default="")
    p.add_argument("--category"); p.add_argument("--workspace"); p.add_argument("--test",dest="test_filter",default=""); p.add_argument("--exact",action="store_true")
    p.add_argument("--fuzz-passes",type=int,default=100000); p.add_argument("--fuzz-target",default=""); p.add_argument("--skip-fuzz",action="store_true"); p.add_argument("--skip-qemu",action="store_true")
    p.add_argument("--hardware",choices=("m5stack","m5stack-lite"),default=""); p.add_argument("--hardware-port",default=""); p.add_argument("--hardware-timeout",type=int,default=240)
    p.add_argument("--dry-run",action="store_true"); p.add_argument("--strict-lockfiles",action="store_false",dest="repair_lockfiles",default=True); p.add_argument("--pause",action="store_true")
    return p


CRAP_RESUME_ARTIFACTS = (
    ROOT / "target/qa/crap/health_summary.json",
    ROOT / "target/qa/crap/lcov.info",
    ROOT / "target/qa/crap/run.json",
    ROOT / "target/qa/crap/cargo_crap.json",
    ROOT / "target/qa/crap/crap_summary.json",
    ROOT / "target/qa/crap/current.json",
)


def ensure_resume_prerequisites(ns: argparse.Namespace, selected: list[tuple[str, str, str, str]]) -> None:
    if not ns.resume_from or ns.profile != "full":
        return
    selected_ids = {record[2] for record in selected}
    if "coverage.critical-branch-targets" not in selected_ids or "preflight.crap-check" in selected_ids:
        return
    missing = [path for path in CRAP_RESUME_ARTIFACTS if not path.is_file() or path.stat().st_size == 0]
    if not missing and not ns.dry_run:
        return
    print("\n[resume prerequisite] Fresh CRAP/coverage artifacts are required by later resumed QA steps.", flush=True)
    if missing:
        print("  Missing: " + ", ".join(str(path.relative_to(ROOT)) for path in missing), flush=True)
    print("  Regenerating preflight.crap-check only; already-passed test steps remain skipped.", flush=True)
    run_step("preflight.crap-check", ns)


def acquire_lock() -> IO[bytes]|None:
    marker=os.environ.get("KASKOLD_QA_RUN_ALL_LOCK_ROOT")
    if marker and Path(marker).resolve()==ROOT.resolve(): return None
    state=ROOT/"target/qa/state"; state.mkdir(parents=True,exist_ok=True); path=state/"release-workflow.lock"; handle=open(path,"a+b")
    if sys.platform=="win32":
        import msvcrt
        try: msvcrt.locking(handle.fileno(),msvcrt.LK_NBLCK,1)
        except OSError:
            print("Another KasKold QA/reproducible-release workflow is active; waiting for it to finish.")
            while True:
                try: msvcrt.locking(handle.fileno(),msvcrt.LK_NBLCK,1); break
                except OSError: time.sleep(0.5)
    os.environ["KASKOLD_QA_RUN_ALL_LOCK_ROOT"]=str(ROOT)
    return handle


_SECTION_HEADINGS = {
    "preflight": "PREFLIGHT",
    "static": "STATIC / ARCHITECTURE TESTS",
    "security": "SECURITY POLICY TESTS",
    "coverage": "COVERAGE / CRAP TESTS",
    "interactive": "REAL-NODE / INTERACTIVE E2E TESTS",
    "bench": "BENCHMARKS",
    "mutation": "FRESH MUTATION TESTS",
    "emulation": "QEMU EMULATION TESTS",
    "hardware": "HARDWARE TESTS",
}


def _normalize_category(ns: argparse.Namespace, p: argparse.ArgumentParser) -> None:
    if ns.category == "benches": ns.category = "bench"
    if ns.category and ns.category not in CATEGORIES: p.error(f"unknown category: {ns.category}")


def _normalize_workspace(ns: argparse.Namespace, p: argparse.ArgumentParser) -> None:
    if ns.workspace:
        if ns.workspace=="kasee-web":ns.workspace="kaskold-companion-web"
        if ns.workspace not in WORKSPACES: p.error(f"unknown workspace: {ns.workspace}")


def _apply_qualified_test_filter(ns: argparse.Namespace, p: argparse.ArgumentParser) -> None:
    if ns.test_filter and "::" in ns.test_filter:
        ws,name=ns.test_filter.split("::",1); ws="kaskold-companion-web" if ws=="kasee-web" else ws
        if not name or ws not in WORKSPACES: p.error("invalid qualified --test")
        if ns.workspace and ns.workspace!=ws: p.error("--test workspace conflicts with --workspace")
        ns.workspace=ws; ns.test_filter=name


def _validate_run_options(ns: argparse.Namespace, p: argparse.ArgumentParser) -> None:
    if ns.fuzz_passes<=0 or ns.hardware_timeout<=0: p.error("counts/timeouts must be positive")
    if ns.hardware_port and not ns.hardware: p.error("--hardware-port requires --hardware BOARD")
    if ns.category=="hardware" and not ns.hardware: p.error("--category hardware requires --hardware BOARD")
    if ns.only.startswith("hardware.") and not ns.hardware: p.error("hardware steps require --hardware BOARD")


def _normalize_arguments(ns: argparse.Namespace, p: argparse.ArgumentParser) -> None:
    _normalize_category(ns, p)
    _normalize_workspace(ns, p)
    _apply_qualified_test_filter(ns, p)
    ns.only=canonical_id(ns.only); ns.resume_from=canonical_id(ns.resume_from)
    _validate_run_options(ns, p)


def _print_step_list() -> None:
    print(f"{'STEP ID':46} {'SCOPE':10} {'WORKSPACE':14} DESCRIPTION"); print(f"{'-------':46} {'-----':10} {'---------':14} -----------")
    for _c,w,i,d in STEPS: print(f"{i:46} {STEP_SCOPES[i]:10} {w:14} {d}")


def _resolve_resume_start(ns: argparse.Namespace, p: argparse.ArgumentParser) -> int:
    if not ns.resume_from:
        return 0
    for idx,(c,w,i,d) in enumerate(STEPS):
        if i==ns.resume_from or c==ns.resume_from or i.startswith(ns.resume_from): return idx
    p.error(f"unknown resume section: {ns.resume_from}")


def _passes_profile(scope: str, ns: argparse.Namespace) -> bool:
    if ns.profile=="test" and scope!="test": return False
    if ns.profile=="full" and scope=="hardware" and ns.category!="hardware" and not ns.hardware: return False
    if ns.profile=="full" and scope not in {"test","qa","hardware"}: return False
    return True


def _passes_step_filters(record: tuple[str, ...], ns: argparse.Namespace) -> bool:
    c,w,i,_d=record
    if ns.only and i!=ns.only: return False
    if ns.category and c!=ns.category: return False
    if ns.workspace and w!=ns.workspace: return False
    if ns.test_filter and i not in TEST_FILTER_STEPS: return False
    if c=="hardware" and not ns.hardware: return False
    if c=="fuzz" and ns.skip_fuzz: return False
    if c=="emulation" and ns.skip_qemu: return False
    return True


def _select_steps(ns: argparse.Namespace, start: int) -> list[tuple[str, str, str, str]]:
    selected=[rec for rec in STEPS[start:] if _passes_profile(STEP_SCOPES[rec[2]], ns) and _passes_step_filters(rec, ns)]
    if any(record[2] == "preflight.core-ci" for record in selected):
        selected=[record for record in selected if STEP_SCOPES[record[2]] != "test"]
    return selected


def _report_step_failure(step_id: str, code: int, ns: argparse.Namespace) -> None:
    print(f"FAIL: {step_id} (exit {code})", file=sys.stderr)
    print(f"Resume with: make qa RESUME_FROM={step_id} FUZZ_PASSES={ns.fuzz_passes}", file=sys.stderr)
    if step_id == "unit.repository-python-qa":
        detail_log = ROOT / "target/qa/repository-python-qa/shared.log"
        print(f"Detailed test log: {detail_log}", file=sys.stderr)


def _run_steps(selected: Sequence[tuple[str, ...]], ns: argparse.Namespace) -> int:
    current_c=current_w=""; passed=0; skipped=0
    for c,w,i,d in selected:
        if c!=current_c:
            current_c=c; current_w=""
            heading=_SECTION_HEADINGS.get(c,c.upper()+" TESTS")
            print("\n"+"="*80+f"\n {heading}\n"+"="*80)
        if w!=current_w: current_w=w; print(f"\n---- {w} ----")
        print(f"\n[{i}] {d}")
        try: run_step(i,ns)
        except (subprocess.CalledProcessError,RuntimeError) as exc:
            code=exc.returncode if isinstance(exc,subprocess.CalledProcessError) else 1
            _report_step_failure(i,code,ns)
            return code
        if STEP_SKIPPED: skipped+=1; print(f"SKIP: {i}")
        else: passed+=1; print(f"PASS: {i}")
    print(f"\nPASS: {passed} passed, {skipped} skipped, {len(selected)} selected test sections completed")
    return 0


def main(argv: list[str]) -> int:
    clean_cargo_fuzz_source_scratch()
    p=parser()
    ns=p.parse_args(argv)
    _normalize_arguments(ns, p)
    if ns.list_only:
        _print_step_list()
        return 0
    load_toolchains(); os.environ["KASKOLD_QA_CATALOG_ACTIVE"]="1"; lock=acquire_lock()
    ids=[x[2] for x in STEPS]
    if ns.only and ns.only not in ids: p.error(f"unknown exact step ID: {ns.only}")
    selected=_select_steps(ns, _resolve_resume_start(ns, p))
    if not selected: p.error("the selected filters matched no test steps")
    try:
        ensure_resume_prerequisites(ns, selected)
    except (subprocess.CalledProcessError, RuntimeError) as exc:
        code = exc.returncode if isinstance(exc, subprocess.CalledProcessError) else 1
        print(f"FAIL: resume prerequisite preflight.crap-check (exit {code})", file=sys.stderr)
        return code
    try:
        return _run_steps(selected, ns)
    finally:
        if lock: lock.close()


if __name__=="__main__":
    status=main(sys.argv[1:])
    if "--pause" in sys.argv[1:]:
        try: input(f"\nTest runner finished with exit code {status}. Press Enter to close this terminal...")
        except EOFError: pass
    raise SystemExit(status)
