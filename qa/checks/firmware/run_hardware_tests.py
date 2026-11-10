#!/usr/bin/env python3
"""Build, flash, and supervise the opt-in KasKold ESP hardware test image."""

from __future__ import annotations

import argparse
import glob
import os
from pathlib import Path
import queue
import re
import signal
import shutil
import subprocess
import sys
import threading
import time
from contextlib import contextmanager
from dataclasses import dataclass
from typing import TextIO

ROOT = Path(__file__).resolve().parents[3]
FIRMWARE = ROOT / "apps" / "kaskold-hardware"
TARGET = FIRMWARE / "target" / "xtensa-esp32s3-none-elf" / "release"
HASH_SOURCE = FIRMWARE / "src" / "firmware_hash.rs"
BOARD_HELPER_DIR = ROOT / "tools" / "build" / "firmware"
if str(BOARD_HELPER_DIR) not in sys.path:
    sys.path.insert(0, str(BOARD_HELPER_DIR))
CHECKS_DIR = Path(__file__).resolve().parent
if str(CHECKS_DIR) not in sys.path:
    sys.path.insert(0, str(CHECKS_DIR))
SERIAL_HELPER_DIR = ROOT / "scripts" / "common" / "lib"
if str(SERIAL_HELPER_DIR) not in sys.path:
    sys.path.insert(0, str(SERIAL_HELPER_DIR))

from board_layout import layout_for, validate_layout  # noqa: E402
from hash_builder import hash_builder_command  # noqa: E402
from serial_access import (  # noqa: E402
    SerialAccessError,
    prepare_serial_command,
    resolve_serial_port,
)
from hil_evidence import HilEvidence, reportable_interruptions  # noqa: E402

PASS_MARKER = "KASKOLD_HARDWARE_TESTS: PASS"
FAIL_MARKER = "KASKOLD_HARDWARE_TESTS: FAIL"
FLASH_COMPLETE_MARKER = "Flashing has completed!"
FLASH_CONNECT_ATTEMPTS = 3
M5STACK_FLASH_RESET_STRATEGIES = ("usb-reset", "no-reset", "default-reset")
MONITOR_RECONNECT_ATTEMPTS = 3
FLASH_TRANSPORT_TIMEOUT_SECONDS = 180
SERIAL_REENUMERATION_TIMEOUT_SECONDS = 15
RETRYABLE_TRANSPORT_MARKERS = (
    "espflash::connection_failed",
    "failed to connect to the device",
    "error while connecting to device",
    "failed to open serial port",
    "serial port not found",
    "no serial ports",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Flash a connected ESP32-S3 and run all registered on-device tests."
    )
    parser.add_argument("--board", required=True, choices=("m5stack", "m5stack-lite"))
    parser.add_argument("--port", help="Serial port; omit for safe auto-selection when unambiguous")
    parser.add_argument("--timeout", type=int, default=240, help="Monitor timeout in seconds")
    args = parser.parse_args()
    if args.timeout < 1:
        parser.error("--timeout must be positive")
    return args


def require_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise RuntimeError(f"required command not found: {name}")


def build(board: str) -> Path:
    env = os.environ.copy()
    # Hardware-test images are development images and must exercise the same
    # fail-closed software-verification path as normal development firmware.
    # Use the repository-public TEST key; production release keys are never used
    # by HIL. Without this, build_with_hash converges the code hash but emits
    # FIRMWARE_SIGNED=false and the connected device correctly rejects the image.
    hil_signing_key = ROOT / "tools" / "build" / "firmware" / "dev_test_signing_key.bin"
    if not hil_signing_key.is_file() or hil_signing_key.stat().st_size != 32:
        raise RuntimeError(f"HIL development signing key is missing or invalid: {hil_signing_key}")
    env["KASKOLD_SIGNING_KEY"] = str(hil_signing_key)
    feature = board
    original_hash_source = HASH_SOURCE.read_bytes()
    command = hash_builder_command(
        board,
        f"{board}-hardware-tests",
        ["--no-default-features", "--features", f"{feature},hardware-tests"],
    )
    print("  +", " ".join(command), flush=True)
    try:
        subprocess.run(command, cwd=ROOT, env=env, check=True)
    finally:
        # The generated hash is a build artifact. HIL must execute the ELF
        # containing the converged value, but QA must not leave the source
        # checkout dirty after the test completes or fails.
        HASH_SOURCE.write_bytes(original_hash_source)
    image = TARGET / "kaskold-hardware"
    if not image.is_file():
        raise RuntimeError(f"firmware ELF was not produced: {image}")
    return image


def reader(stream: TextIO, lines: queue.Queue[str | None]) -> None:
    try:
        while True:
            line = stream.readline()
            if line == "":
                break
            lines.put(line)
    except (ValueError, OSError):
        # stop_process closes the pipe once the supervisor has a verdict; a read
        # racing that close is end of stream. Errors on an open pipe still raise.
        if not stream.closed:
            raise
    lines.put(None)


def stop_process(process: subprocess.Popen[str]) -> None:
    if process.poll() is None:
        if os.name == "nt":
            process.terminate()
        else:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            if os.name == "nt":
                process.kill()
            else:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            process.wait(timeout=3)
    if process.stdout is not None:
        process.stdout.close()


@contextmanager
def private_monitor_input():
    """Give espflash a private TTY without attaching the caller's keyboard.

    Pinned espflash 3.3 initializes an interactive input reader whenever monitor
    mode is active. On POSIX, a private pseudo-terminal satisfies that contract
    while keeping the user's terminal completely outside the child process.
    """
    if os.name != "posix":
        yield None
        return
    import pty

    master_fd, slave_fd = pty.openpty()
    try:
        yield slave_fd
    finally:
        for fd in (slave_fd, master_fd):
            try:
                os.close(fd)
            except OSError:
                pass


@contextmanager
def preserved_terminal():
    """Restore the caller terminal exactly after the supervised monitor exits."""
    if os.name == "nt" or not sys.stdin.isatty():
        yield
        return
    try:
        import termios
    except ImportError:
        yield
        return
    try:
        fd = sys.stdin.fileno()
        attributes = termios.tcgetattr(fd)
    except (OSError, termios.error):
        yield
        return
    try:
        yield
    finally:
        try:
            termios.tcsetattr(fd, termios.TCSADRAIN, attributes)
        except (OSError, termios.error):
            pass


def _descriptor_targets(fd_dir: Path, target: Path) -> bool:
    try:
        descriptors = tuple(fd_dir.iterdir())
    except OSError:
        return False
    for descriptor in descriptors:
        try:
            if descriptor.resolve(strict=True) == target:
                return True
        except OSError:
            continue
    return False


def _process_label(process_dir: Path) -> str:
    command = ""
    try:
        raw = (process_dir / "cmdline").read_bytes().replace(b"\0", b" ").strip()
        command = raw.decode(errors="replace")
    except OSError:
        pass
    if not command:
        try:
            command = (process_dir / "comm").read_text(errors="replace").strip()
        except OSError:
            command = "unknown"
    return command or "unknown"


def serial_port_owners(port: str | None) -> tuple[tuple[int, str], ...]:
    """Return Linux processes that currently hold an fd to *port*.

    This is best-effort diagnostics only: it never kills processes, changes
    permissions, or depends on external tools such as lsof/fuser.
    """
    if (
        not port
        or os.name != "posix"
        or not sys.platform.startswith("linux")
        or not port.startswith("/dev/")
    ):
        return ()
    try:
        target = Path(port).resolve(strict=True)
    except OSError:
        return ()
    owners: list[tuple[int, str]] = []
    for process_dir in Path("/proc").iterdir():
        if not process_dir.name.isdigit():
            continue
        if not _descriptor_targets(process_dir / "fd", target):
            continue
        owners.append((int(process_dir.name), _process_label(process_dir)))
    return tuple(sorted(owners))


def _process_parent_pid(pid: int) -> int | None:
    """Read one Linux process parent without depending on ps/procps."""
    try:
        for line in (Path("/proc") / str(pid) / "status").read_text(errors="replace").splitlines():
            if line.startswith("PPid:"):
                return int(line.split(":", 1)[1].strip())
    except (OSError, ValueError):
        return None
    return None


def _process_command(pid: int) -> str:
    try:
        raw = (Path("/proc") / str(pid) / "cmdline").read_bytes().replace(b"\0", b" ").strip()
        return raw.decode(errors="replace")
    except OSError:
        return ""


def _process_has_environment(pid: int, key: str, value: str) -> bool:
    try:
        raw = (Path("/proc") / str(pid) / "environ").read_bytes()
    except OSError:
        return False
    expected = f"{key}={value}".encode()
    return expected in raw.split(b"\0")


def _has_connected_runner_ancestor(pid: int) -> bool:
    """Return whether *pid* belongs to another live connected KasKold runner."""
    seen: set[int] = set()
    current = pid
    for _ in range(24):
        parent = _process_parent_pid(current)
        if parent is None or parent <= 1 or parent in seen:
            return False
        seen.add(parent)
        command = _process_command(parent)
        if "run_workflow_tests.py" in command or "run_hardware_tests.py" in command:
            return True
        current = parent
    return False


def _looks_like_managed_kaskold_espflash(command: str) -> bool:
    """Recognize the supervised flash+monitor command emitted by this repository."""
    normalized = command.replace("\\", "/")
    firmware_root = str(FIRMWARE).replace("\\", "/")
    return (
        "espflash flash --monitor" in normalized
        and firmware_root in normalized
        and "/partitions/" in normalized
        and "/target/xtensa-esp32s3-none-elf/release/kaskold-hardware" in normalized
    )


def _is_reclaimable_orphan(pid: int, command: str) -> bool:
    """Return True only for a managed or legacy orphaned KasKold espflash monitor."""
    if os.name == "nt" or not sys.platform.startswith("linux"):
        return False
    if not _looks_like_managed_kaskold_espflash(command):
        return False
    if _has_connected_runner_ancestor(pid):
        return False
    parent = _process_parent_pid(pid)
    managed_marker = _process_has_environment(pid, "KASKOLD_MANAGED_SERIAL_MONITOR", "1")
    # Supervised monitors carry an explicit environment marker. For a legacy
    # command that can already be stranded on a developer
    # machine, reclaim only when it has become a true orphan (PPID 1). Never
    # kill a manually launched lookalike that is still owned by an interactive shell.
    return managed_marker or parent in (0, 1)


def _wait_until_port_released(pid: int, port: str, seconds: float) -> bool:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if all(owner_pid != pid for owner_pid, _ in serial_port_owners(port)):
            return True
        time.sleep(0.05)
    return False


def _reclaim_stale_managed_owner(pid: int, command: str, port: str) -> bool:
    """Reap an orphaned monitor from an earlier KasKold connected run only."""
    if not _is_reclaimable_orphan(pid, command):
        return False
    try:
        if os.getpgid(pid) != pid:
            return False
    except (OSError, ProcessLookupError):
        return True

    print(
        f"  [transport] reclaiming stale KasKold espflash monitor PID {pid} holding {port}",
        flush=True,
    )
    try:
        os.killpg(pid, signal.SIGTERM)
    except ProcessLookupError:
        return True
    if _wait_until_port_released(pid, port, 2.0):
        return True
    try:
        os.killpg(pid, signal.SIGKILL)
    except ProcessLookupError:
        return True
    return _wait_until_port_released(pid, port, 1.0)


def ensure_serial_port_available(port: str | None) -> None:
    """Fail on live owners, but reclaim orphaned KasKold-supervised monitors."""
    owners = serial_port_owners(port)
    if not owners:
        return
    remaining: list[tuple[int, str]] = []
    for pid, command in owners:
        if port and _reclaim_stale_managed_owner(pid, command, port):
            continue
        remaining.append((pid, command))
    if not remaining:
        return
    details = "; ".join(f"PID {pid}: {command}" for pid, command in remaining)
    raise RuntimeError(
        f"serial port {port} is already open by another live process ({details}); "
        "close that serial monitor/process before running connected QA"
    )


def wait_for_explicit_port(port: str | None, timeout: int = SERIAL_REENUMERATION_TIMEOUT_SECONDS) -> None:
    """Wait for the exact POSIX serial node to exist after USB re-enumeration."""
    if not port or os.name != "posix" or not port.startswith("/dev/"):
        return
    path = Path(port)
    if path.exists():
        return
    print(f"  Waiting up to {timeout}s for serial port {port} to appear...", flush=True)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            print(f"  Serial port ready: {port}", flush=True)
            return
        time.sleep(0.25)
    candidates = sorted({*glob.glob("/dev/ttyACM*"), *glob.glob("/dev/ttyUSB*")})
    visible = ", ".join(candidates) if candidates else "none"
    raise RuntimeError(
        f"serial port {port} did not appear within {timeout}s; visible serial ports: {visible}"
    )


def retryable_transport_failure(output: list[str], flash_complete: bool) -> bool:
    if flash_complete:
        return False
    lowered = "\n".join(output).lower()
    return any(marker in lowered for marker in RETRYABLE_TRANSPORT_MARKERS)


def _replace_before_mode(command: list[str], mode: str) -> list[str]:
    """Return a copy of an espflash command with exactly one --before mode."""
    updated = list(command)
    try:
        before = updated.index("--before")
    except ValueError:
        insert_at = 3 if len(updated) >= 3 else len(updated)
        updated[insert_at:insert_at] = ["--before", mode]
        return updated
    if before + 1 >= len(updated):
        raise RuntimeError("espflash --before option is missing its reset mode")
    updated[before + 1] = mode
    return updated


def flash_attempt_command(base_command: list[str], board: str, attempt: int) -> tuple[list[str], str]:
    """Apply a board-specific pre-flash recovery strategy for this attempt."""
    if board not in {"m5stack", "m5stack-lite"}:
        return list(base_command), "board-default"
    if not 1 <= attempt <= len(M5STACK_FLASH_RESET_STRATEGIES):
        raise RuntimeError(f"invalid CoreS3 flash attempt: {attempt}")
    mode = M5STACK_FLASH_RESET_STRATEGIES[attempt - 1]
    return _replace_before_mode(base_command, mode), mode


def print_transport_diagnostics(board: str, port: str | None, attempts: int) -> None:
    """Print bounded, actionable diagnostics without mutating host/device state."""
    print(
        f"ERROR: unable to establish the ESP32-S3 flashing transport after {attempts} attempt(s).",
        file=sys.stderr,
    )
    if port:
        path = Path(port)
        exists = path.exists()
        print(f"  serial port: {port} (exists={str(exists).lower()})", file=sys.stderr)
        if exists and os.name == "posix":
            readable = os.access(path, os.R_OK)
            writable = os.access(path, os.W_OK)
            print(
                f"  caller-shell access before any dialout wrapper: read={str(readable).lower()} "
                f"write={str(writable).lower()}",
                file=sys.stderr,
            )
            owners = serial_port_owners(port)
            if owners:
                print("  serial port owner(s):", file=sys.stderr)
                for pid, command in owners:
                    print(f"    PID {pid}: {command}", file=sys.stderr)
    else:
        print("  serial port: auto-detect", file=sys.stderr)
    if board in {"m5stack", "m5stack-lite"}:
        strategies = " -> ".join(M5STACK_FLASH_RESET_STRATEGIES)
        print(
            f"  CoreS3 automatic reset ladder exhausted: {strategies}.",
            file=sys.stderr,
        )
        print(
            "  Close any other serial monitor. If all software entry modes fail, the running "
            "application may have left native USB unable to drive ROM download entry; a physical "
            "RESET/BOOT recovery can then be required.",
            file=sys.stderr,
        )


def print_failure_context(output: list[str], prefixes: tuple[str, ...] | None) -> None:
    if not prefixes:
        return
    matched: list[str] = []
    seen: set[str] = set()
    for raw in output:
        line = raw.strip()
        if not line or line in seen:
            continue
        if any(prefix in line for prefix in prefixes):
            matched.append(line)
            seen.add(line)
    if not matched:
        return
    print("  [monitor] failure context replay:", file=sys.stderr)
    for line in matched[-32:]:
        print(f"    {line}", file=sys.stderr)


@dataclass(frozen=True)
class _MonitorConfig:
    """Immutable settings for one supervised flash or monitor attempt."""

    timeout: int
    monitor_only: bool
    pass_marker: str
    fail_marker: str
    success_label: str
    status_interval: int | None
    repeat_abort_marker: str | None
    repeat_abort_arm_marker: str | None
    phase_start_marker: str | None
    phase_end_marker: str | None
    phase_timeout: int | None
    deadline_extension_marker: str | None
    deadline_extension_seconds: int | None
    required_markers: tuple[str, ...]
    ordered_markers: tuple[str, ...] | None
    failure_context_prefixes: tuple[str, ...] | None
    operation_start_prefix: str | None
    operation_end_prefix: str | None
    operation_timeout: int | None
    operation_timeouts: dict[str, int] | None
    runtime_state_prefix: str | None
    required_runtime_states: tuple[str, ...]
    uart_log: TextIO | None


@dataclass
class _MonitorTimers:
    """Mutable deadlines for one attempt; None means the deadline is not armed."""

    transport_deadline: float
    test_started: float | None
    deadline: float | None
    next_status: float | None
    phase_deadline: float | None = None
    operation_deadline: float | None = None
    operation_name: str | None = None
    active_operation_timeout: int | None = None
    panic_diagnostic_deadline: float | None = None


_Outcome = tuple[int, bool, bool, bool]
# The firmware's fail-closed halts log "FATAL: " and then spin forever, so they
# end the run like a panic instead of waiting out the test timeout.
_PANIC_MARKERS = (
    "====================== PANIC",
    "panicked at ",
    "Guru Meditation Error",
    "FATAL: ",
)


def _new_timers(config: _MonitorConfig) -> _MonitorTimers:
    started = time.monotonic()
    return _MonitorTimers(
        transport_deadline=started + FLASH_TRANSPORT_TIMEOUT_SECONDS,
        test_started=started if config.monitor_only else None,
        deadline=started + config.timeout if config.monitor_only else None,
        next_status=(
            started + config.status_interval
            if config.monitor_only and config.status_interval
            else None
        ),
    )


def _monitor_environment() -> dict[str, str]:
    process_env = os.environ.copy()
    process_env["KASKOLD_MANAGED_SERIAL_MONITOR"] = "1"
    return process_env


def _report_missing(header: str, missing: list[str], item_label: str, limit: int) -> None:
    print(header, file=sys.stderr)
    for item in missing[:limit]:
        print(f"  {item_label}: {item}", file=sys.stderr)
    if len(missing) > limit:
        print(f"  ... and {len(missing) - limit} more", file=sys.stderr)


class _FlashMonitorAttempt:
    """Supervises one serial-monitor process and classifies its output."""

    _timers: _MonitorTimers

    def __init__(
        self,
        config: _MonitorConfig,
        *,
        seen_markers: set[str],
        ordered_progress: list[int],
        seen_runtime_states: set[str],
    ) -> None:
        self._config = config
        self._seen_markers = seen_markers
        self._ordered_progress = ordered_progress
        self._seen_runtime_states = seen_runtime_states
        self._output: list[str] = []
        self._flash_complete = config.monitor_only

    def supervise(self, process: subprocess.Popen[str]) -> _Outcome:
        assert process.stdout is not None
        lines: queue.Queue[str | None] = queue.Queue()
        thread = threading.Thread(target=reader, args=(process.stdout, lines), daemon=True)
        thread.start()
        self._timers = _new_timers(self._config)
        while True:
            outcome = self._step(process, lines)
            if outcome is not None:
                return outcome

    def _terminal_failure(self) -> _Outcome:
        return 1, False, self._flash_complete, True

    def _step(self, process: subprocess.Popen[str], lines: queue.Queue[str | None]) -> _Outcome | None:
        now = time.monotonic()
        timers = self._timers
        if timers.panic_diagnostic_deadline is not None and now >= timers.panic_diagnostic_deadline:
            print(
                f"FAIL: connected ESP device panicked or halted during {self._config.success_label}; "
                "diagnostic tail captured",
                file=sys.stderr,
            )
            return self._terminal_failure()
        active_deadline = timers.deadline if timers.deadline is not None else timers.transport_deadline
        remaining = active_deadline - now
        if remaining <= 0:
            return self._timed_out(now)
        try:
            line = lines.get(timeout=min(0.25, remaining))
        except queue.Empty:
            return self._on_idle(process)
        if line is None:
            return self._on_stream_closed(process)
        return self._handle_line(line)

    def _timed_out(self, now: float) -> _Outcome:
        timers = self._timers
        if timers.deadline is None:
            print(
                "ERROR: espflash transport did not reach flash completion within "
                f"{FLASH_TRANSPORT_TIMEOUT_SECONDS}s",
                file=sys.stderr,
            )
            return 124, not self._flash_complete, self._flash_complete, False
        assert timers.test_started is not None
        print(
            f"ERROR: {self._config.success_label} timed out after {int(now - timers.test_started)}s",
            file=sys.stderr,
        )
        return 124, False, self._flash_complete, True

    def _exit_outcome(self, process: subprocess.Popen[str], description: str) -> _Outcome:
        code = process.returncode or 1
        retryable = retryable_transport_failure(self._output, self._flash_complete)
        print(
            f"ERROR: {description} with code {process.returncode} before a result marker",
            file=sys.stderr,
        )
        return code, (self._config.monitor_only or retryable), self._flash_complete, False

    def _on_idle(self, process: subprocess.Popen[str]) -> _Outcome | None:
        if process.poll() is not None:
            return self._exit_outcome(process, "espflash exited")
        now = time.monotonic()
        timers = self._timers
        label = self._config.success_label
        if timers.phase_deadline is not None and now >= timers.phase_deadline:
            print(
                f"ERROR: {label} stalled after {self._config.phase_start_marker!r}; "
                f"did not reach {self._config.phase_end_marker!r} within {self._config.phase_timeout}s",
                file=sys.stderr,
            )
            return 124, False, self._flash_complete, True
        if timers.operation_deadline is not None and now >= timers.operation_deadline:
            print(
                f"ERROR: {label} runtime action {timers.operation_name!r} exceeded "
                f"the {timers.active_operation_timeout}s production-liveness budget",
                file=sys.stderr,
            )
            return 124, False, self._flash_complete, True
        self._maybe_report_status(now)
        return None

    def _maybe_report_status(self, now: float) -> None:
        timers = self._timers
        if timers.next_status is None or now < timers.next_status:
            return
        assert timers.test_started is not None and timers.deadline is not None
        elapsed = int(now - timers.test_started)
        remaining_seconds = max(0, int(timers.deadline - now))
        print(
            f"  [monitor] device test still running: {elapsed}s elapsed, "
            f"{remaining_seconds}s until timeout",
            flush=True,
        )
        assert self._config.status_interval is not None
        timers.next_status = now + self._config.status_interval

    def _on_stream_closed(self, process: subprocess.Popen[str]) -> _Outcome | None:
        if process.poll() is not None:
            return self._exit_outcome(process, "serial monitor closed")
        return None

    def _handle_line(self, line: str) -> _Outcome | None:
        self._record_line(line)
        self._record_runtime_state(line)
        self._mark_flash_complete(line)
        self._track_operation_start(line)
        self._track_operation_end(line)
        self._record_required_markers(line)
        outcome = self._check_ordered_markers(line)
        if outcome is not None:
            return outcome
        # Fake monitor tests and future espflash formatting changes can
        # emit the firmware result before the informational flash line.
        # A terminal device marker wins only after every required runtime
        # evidence marker has actually been observed across reconnects.
        if self._config.pass_marker in line:
            return self._terminal_pass()
        if self._config.fail_marker in line:
            print_failure_context(self._output, self._config.failure_context_prefixes)
            print(
                f"FAIL: connected ESP device reported {self._config.success_label} failure",
                file=sys.stderr,
            )
            return self._terminal_failure()
        self._extend_deadline(line)
        self._track_phase(line)
        outcome = self._check_repeat_abort(line)
        if outcome is not None:
            return outcome
        self._track_panic(line)
        return None

    def _record_line(self, line: str) -> None:
        self._output.append(line)
        if self._config.uart_log is not None:
            self._config.uart_log.write(line)
            self._config.uart_log.flush()
        print(line, end="", flush=True)

    def _record_runtime_state(self, line: str) -> None:
        prefix = self._config.runtime_state_prefix
        if not prefix or prefix not in line:
            return
        raw_state = line.split(prefix, 1)[1].strip()
        match = re.match(r"([A-Za-z][A-Za-z0-9_]*)", raw_state)
        if match:
            self._seen_runtime_states.add(match.group(1))

    def _mark_flash_complete(self, line: str) -> None:
        if FLASH_COMPLETE_MARKER not in line or self._flash_complete:
            return
        self._flash_complete = True
        config = self._config
        timers = self._timers
        timers.test_started = time.monotonic()
        timers.deadline = timers.test_started + config.timeout
        timers.next_status = (
            timers.test_started + config.status_interval if config.status_interval else None
        )
        print(
            f"  [monitor] flash complete; starting the {config.timeout}s device-test deadline now",
            flush=True,
        )

    def _track_operation_start(self, line: str) -> None:
        prefix = self._config.operation_start_prefix
        if not prefix or prefix not in line:
            return
        timers = self._timers
        timers.operation_name = line.split(prefix, 1)[1].strip()
        timers.active_operation_timeout = (self._config.operation_timeouts or {}).get(
            timers.operation_name, self._config.operation_timeout
        )
        if timers.active_operation_timeout:
            timers.operation_deadline = time.monotonic() + timers.active_operation_timeout
            print(
                f"  [monitor] runtime action {timers.operation_name!r} must complete within "
                f"{timers.active_operation_timeout}s",
                flush=True,
            )
        else:
            timers.operation_deadline = None

    def _track_operation_end(self, line: str) -> None:
        prefix = self._config.operation_end_prefix
        if not prefix or prefix not in line:
            return
        timers = self._timers
        ended = line.split(prefix, 1)[1].strip()
        if timers.operation_name is None or ended == timers.operation_name:
            timers.operation_deadline = None
            timers.operation_name = None
            timers.active_operation_timeout = None

    def _record_required_markers(self, line: str) -> None:
        for marker in self._config.required_markers:
            if marker in line:
                self._seen_markers.add(marker)

    def _check_ordered_markers(self, line: str) -> _Outcome | None:
        ordered = self._config.ordered_markers
        progress = self._ordered_progress
        if not ordered or progress[0] >= len(ordered):
            return None
        pending = ordered[progress[0]:]
        matched_offset = next((index for index, marker in enumerate(pending) if marker in line), None)
        if matched_offset is None:
            return None
        if matched_offset != 0:
            print(
                f"FAIL: {self._config.success_label} runtime evidence arrived out of order; "
                f"expected {pending[0]!r} before {pending[matched_offset]!r}",
                file=sys.stderr,
            )
            return self._terminal_failure()
        progress[0] += 1
        return None

    def _terminal_pass(self) -> _Outcome:
        blocked = self._pass_blocker()
        if blocked is not None:
            return blocked
        print(f"PASS: connected ESP device completed {self._config.success_label}")
        return 0, False, self._flash_complete, True

    def _pass_blocker(self) -> _Outcome | None:
        config = self._config
        missing = [marker for marker in config.required_markers if marker not in self._seen_markers]
        if missing:
            _report_missing(
                "FAIL: connected ESP device emitted the terminal pass marker but "
                f"{len(missing)} required runtime evidence marker(s) were never observed",
                missing,
                "missing",
                20,
            )
            return self._terminal_failure()
        missing_states = [
            state for state in config.required_runtime_states
            if state not in self._seen_runtime_states
        ]
        if missing_states:
            _report_missing(
                "FAIL: connected ESP device emitted the terminal pass marker but "
                f"{len(missing_states)} graph-derived stable screen(s) were never physically rendered",
                missing_states,
                "missing render",
                30,
            )
            return self._terminal_failure()
        ordered = config.ordered_markers
        if ordered and self._ordered_progress[0] != len(ordered):
            expected = ordered[self._ordered_progress[0]]
            print(
                "FAIL: connected ESP device emitted the terminal pass marker before "
                f"ordered runtime evidence {expected!r}",
                file=sys.stderr,
            )
            return self._terminal_failure()
        return None

    def _extend_deadline(self, line: str) -> None:
        marker = self._config.deadline_extension_marker
        seconds = self._config.deadline_extension_seconds
        timers = self._timers
        if timers.deadline is None or not marker or marker not in line or not seconds or seconds <= 0:
            return
        extended = time.monotonic() + seconds
        if extended > timers.deadline:
            timers.deadline = extended
            print(
                "  [monitor] long-running device operation detected; "
                f"deadline extended by up to {seconds}s from now",
                flush=True,
            )

    def _track_phase(self, line: str) -> None:
        config = self._config
        timers = self._timers
        if config.phase_start_marker and config.phase_start_marker in line and config.phase_timeout:
            timers.phase_deadline = time.monotonic() + config.phase_timeout
            print(
                f"  [monitor] phase handoff must reach {config.phase_end_marker!r} "
                f"within {config.phase_timeout}s",
                flush=True,
            )
        if config.phase_end_marker and config.phase_end_marker in line:
            timers.phase_deadline = None

    def _check_repeat_abort(self, line: str) -> _Outcome | None:
        config = self._config
        marker = config.repeat_abort_marker
        if not marker or not self._flash_complete:
            return None
        arm_marker = config.repeat_abort_arm_marker
        repeat_key = f"__repeat_abort__:{marker}"
        repeat_arm_key = f"__repeat_abort_armed__:{marker}"
        if arm_marker and arm_marker in line:
            # The arm marker proves the first boot advanced beyond the BEGIN line.
            # This deliberately tolerates a duplicate/replayed BEGIN emitted by the
            # espflash monitor before the immediately-following PREBOARD marker.
            self._seen_markers.add(repeat_key)
            self._seen_markers.add(repeat_arm_key)
        if marker not in line:
            return None
        if repeat_key not in self._seen_markers:
            self._seen_markers.add(repeat_key)
            return None
        armed = arm_marker is None or repeat_arm_key in self._seen_markers
        if armed:
            print(
                f"FAIL: connected ESP device rebooted before {config.success_label} completed "
                f"({marker!r} appeared again after the first boot advanced "
                f"past {arm_marker!r})",
                file=sys.stderr,
            )
            return self._terminal_failure()
        print(
            f"  [monitor] duplicate/replayed {marker!r} before "
            f"{arm_marker!r}; ignoring",
            flush=True,
        )
        return None

    def _track_panic(self, line: str) -> None:
        if not any(marker in line for marker in _PANIC_MARKERS):
            return
        timers = self._timers
        if timers.panic_diagnostic_deadline is not None:
            return
        timers.panic_diagnostic_deadline = time.monotonic() + 1.5
        print(
            "  [monitor] panic detected; capturing the diagnostic tail before aborting",
            flush=True,
        )


def _run_flash_monitor_attempt(
    command: list[str],
    timeout: int,
    *,
    monitor_only: bool = False,
    pass_marker: str,
    fail_marker: str,
    success_label: str,
    status_interval: int | None,
    repeat_abort_marker: str | None,
    repeat_abort_arm_marker: str | None,
    phase_start_marker: str | None,
    phase_end_marker: str | None,
    phase_timeout: int | None,
    deadline_extension_marker: str | None,
    deadline_extension_seconds: int | None,
    required_markers: tuple[str, ...] | None = None,
    seen_markers: set[str] | None = None,
    ordered_markers: tuple[str, ...] | None = None,
    ordered_progress: list[int] | None = None,
    failure_context_prefixes: tuple[str, ...] | None = None,
    operation_start_prefix: str | None = None,
    operation_end_prefix: str | None = None,
    operation_timeout: int | None = None,
    operation_timeouts: dict[str, int] | None = None,
    runtime_state_prefix: str | None = None,
    required_runtime_states: tuple[str, ...] | None = None,
    seen_runtime_states: set[str] | None = None,
    uart_log: TextIO | None = None,
) -> tuple[int, bool, bool, bool]:
    """Run one supervised process; return code/retry/flash-complete/terminal-result."""
    config = _MonitorConfig(
        timeout=timeout,
        monitor_only=monitor_only,
        pass_marker=pass_marker,
        fail_marker=fail_marker,
        success_label=success_label,
        status_interval=status_interval,
        repeat_abort_marker=repeat_abort_marker,
        repeat_abort_arm_marker=repeat_abort_arm_marker,
        phase_start_marker=phase_start_marker,
        phase_end_marker=phase_end_marker,
        phase_timeout=phase_timeout,
        deadline_extension_marker=deadline_extension_marker,
        deadline_extension_seconds=deadline_extension_seconds,
        required_markers=required_markers or (),
        ordered_markers=ordered_markers,
        failure_context_prefixes=failure_context_prefixes,
        operation_start_prefix=operation_start_prefix,
        operation_end_prefix=operation_end_prefix,
        operation_timeout=operation_timeout,
        operation_timeouts=operation_timeouts,
        runtime_state_prefix=runtime_state_prefix,
        required_runtime_states=required_runtime_states or (),
        uart_log=uart_log,
    )
    monitor = _FlashMonitorAttempt(
        config,
        seen_markers=set() if seen_markers is None else seen_markers,
        ordered_progress=[0] if ordered_progress is None else ordered_progress,
        seen_runtime_states=set() if seen_runtime_states is None else seen_runtime_states,
    )
    try:
        with preserved_terminal(), private_monitor_input() as monitor_stdin:
            process = subprocess.Popen(
                command,
                cwd=FIRMWARE,
                env=_monitor_environment(),
                stdin=monitor_stdin,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
                start_new_session=(os.name != "nt"),
            )
            try:
                return monitor.supervise(process)
            finally:
                stop_process(process)
    finally:
        print("  Serial monitor stopped; terminal state restored.", flush=True)


def monitor_reconnect_command(image: Path, port: str | None) -> list[str]:
    """Attach to already-running firmware without resetting or synchronizing it."""
    command = [
        "espflash",
        "monitor",
        "--non-interactive",
        "--chip",
        "esp32s3",
        "--before",
        "no-reset-no-sync",
        "--elf",
        str(image),
    ]
    if port:
        command.extend(("--port", port))
    return command


def reconnect_monitor(
    image: Path,
    port: str | None,
    timeout: int,
    *,
    pass_marker: str,
    fail_marker: str,
    success_label: str,
    status_interval: int | None,
    repeat_abort_marker: str | None,
    repeat_abort_arm_marker: str | None = None,
    phase_start_marker: str | None,
    phase_end_marker: str | None,
    phase_timeout: int | None,
    deadline_extension_marker: str | None,
    deadline_extension_seconds: int | None,
    required_markers: tuple[str, ...] | None = None,
    seen_markers: set[str] | None = None,
    ordered_markers: tuple[str, ...] | None = None,
    ordered_progress: list[int] | None = None,
    failure_context_prefixes: tuple[str, ...] | None = None,
    operation_start_prefix: str | None = None,
    operation_end_prefix: str | None = None,
    operation_timeout: int | None = None,
    operation_timeouts: dict[str, int] | None = None,
    runtime_state_prefix: str | None = None,
    required_runtime_states: tuple[str, ...] | None = None,
    seen_runtime_states: set[str] | None = None,
    uart_log: TextIO | None = None,
) -> int:
    """Recover a dropped post-flash monitor without ever reflashing the device."""
    if seen_markers is None:
        seen_markers = set()
    if ordered_progress is None:
        ordered_progress = [0]
    if seen_runtime_states is None:
        seen_runtime_states = set()
    base_command = monitor_reconnect_command(image, port)
    for attempt in range(1, MONITOR_RECONNECT_ATTEMPTS + 1):
        try:
            wait_for_explicit_port(port)
            command = prepare_serial_command(base_command, port)
        except (RuntimeError, SerialAccessError) as error:
            print(f"ERROR: {error}", file=sys.stderr)
            if attempt == MONITOR_RECONNECT_ATTEMPTS:
                return 1
            time.sleep(min(0.75 * attempt, 1.5))
            continue
        print(
            f"  [monitor] reconnect {attempt}/{MONITOR_RECONNECT_ATTEMPTS} without reflashing: "
            + " ".join(command),
            flush=True,
        )
        code, retryable, _flash_complete, terminal_result = _run_flash_monitor_attempt(
            command,
            timeout,
            monitor_only=True,
            pass_marker=pass_marker,
            fail_marker=fail_marker,
            success_label=success_label,
            status_interval=status_interval,
            repeat_abort_marker=repeat_abort_marker,
            repeat_abort_arm_marker=repeat_abort_arm_marker,
            phase_start_marker=phase_start_marker,
            phase_end_marker=phase_end_marker,
            phase_timeout=phase_timeout,
            deadline_extension_marker=deadline_extension_marker,
            deadline_extension_seconds=deadline_extension_seconds,
            required_markers=required_markers,
            seen_markers=seen_markers,
            ordered_markers=ordered_markers,
            ordered_progress=ordered_progress,
            failure_context_prefixes=failure_context_prefixes,
            operation_start_prefix=operation_start_prefix,
            operation_end_prefix=operation_end_prefix,
            operation_timeout=operation_timeout,
            operation_timeouts=operation_timeouts,
            runtime_state_prefix=runtime_state_prefix,
            required_runtime_states=required_runtime_states,
            seen_runtime_states=seen_runtime_states,
            uart_log=uart_log,
        )
        if code == 0 or terminal_result or not retryable:
            return code
        if attempt < MONITOR_RECONNECT_ATTEMPTS:
            time.sleep(min(0.75 * attempt, 1.5))
    return code


def _resolve_flash_port(port: str | None) -> tuple[str | None, int]:
    try:
        resolved = resolve_serial_port(port)
        ensure_serial_port_available(resolved)
    except RuntimeError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return port, 1
    return resolved, 0


def _print_flash_banner(base_command: list[str], timeout: int) -> None:
    print("  +", " ".join(base_command), flush=True)
    print(
        f"  Transport may retry up to {FLASH_CONNECT_ATTEMPTS} times; the {timeout}s "
        "device-test deadline starts only after flashing completes.",
        flush=True,
    )
    if os.name == "posix":
        print("  Supervised monitor uses a private pseudo-terminal; your keyboard is not attached.", flush=True)
    else:
        print("  Supervised monitor reserves the console only while the pinned espflash monitor is active.", flush=True)


def _announce_flash_attempt(board: str, attempt: int, reset_strategy: str) -> None:
    if attempt > 1:
        print(
            f"  [transport] retry {attempt}/{FLASH_CONNECT_ATTEMPTS} after ESP connection failure...",
            flush=True,
        )
        time.sleep(min(0.75 * (attempt - 1), 1.5))
    if board in {"m5stack", "m5stack-lite"}:
        suffix = " (sync-only; preserve prior ROM-download state)" if reset_strategy == "no-reset" else ""
        print(
            f"  [transport] CoreS3 reset strategy {attempt}/{FLASH_CONNECT_ATTEMPTS}: "
            f"{reset_strategy}{suffix}",
            flush=True,
        )


def flash_and_monitor(
    board: str,
    image: Path,
    port: str | None,
    timeout: int,
    *,
    pass_marker: str = PASS_MARKER,
    fail_marker: str = FAIL_MARKER,
    success_label: str = "hardware tests",
    status_interval: int | None = None,
    repeat_abort_marker: str | None = None,
    repeat_abort_arm_marker: str | None = None,
    phase_start_marker: str | None = None,
    phase_end_marker: str | None = None,
    phase_timeout: int | None = None,
    deadline_extension_marker: str | None = None,
    deadline_extension_seconds: int | None = None,
    required_markers: tuple[str, ...] | None = None,
    ordered_markers: tuple[str, ...] | None = None,
    failure_context_prefixes: tuple[str, ...] | None = None,
    operation_start_prefix: str | None = None,
    operation_end_prefix: str | None = None,
    operation_timeout: int | None = None,
    operation_timeouts: dict[str, int] | None = None,
    runtime_state_prefix: str | None = None,
    required_runtime_states: tuple[str, ...] | None = None,
    uart_log: TextIO | None = None,
    connected_transport: bool = True,
) -> int:
    seen_markers: set[str] = set()
    seen_runtime_states: set[str] = set()
    ordered_progress = [0]
    if connected_transport:
        port, status = _resolve_flash_port(port)
        if status:
            return status
    layout = layout_for(board)
    validate_layout(layout)
    base_command = [
        "espflash",
        "flash",
        "--monitor",
        *layout.espflash_connection_args(),
        *layout.espflash_args(),
        *(("--port", port) if port else ()),
        str(image),
    ]
    _print_flash_banner(base_command, timeout)

    for attempt in range(1, FLASH_CONNECT_ATTEMPTS + 1):
        attempt_command, reset_strategy = flash_attempt_command(base_command, board, attempt)
        _announce_flash_attempt(board, attempt, reset_strategy)
        try:
            wait_for_explicit_port(port)
            command = prepare_serial_command(attempt_command, port)
        except (RuntimeError, SerialAccessError) as error:
            print(f"ERROR: {error}", file=sys.stderr)
            if attempt == FLASH_CONNECT_ATTEMPTS:
                print_transport_diagnostics(board, port, attempt)
                return 1
            continue

        code, retryable, flash_complete, terminal_result = _run_flash_monitor_attempt(
            command,
            timeout,
            pass_marker=pass_marker,
            fail_marker=fail_marker,
            success_label=success_label,
            status_interval=status_interval,
            repeat_abort_marker=repeat_abort_marker,
            repeat_abort_arm_marker=repeat_abort_arm_marker,
            phase_start_marker=phase_start_marker,
            phase_end_marker=phase_end_marker,
            phase_timeout=phase_timeout,
            deadline_extension_marker=deadline_extension_marker,
            deadline_extension_seconds=deadline_extension_seconds,
            required_markers=required_markers,
            seen_markers=seen_markers,
            ordered_markers=ordered_markers,
            ordered_progress=ordered_progress,
            failure_context_prefixes=failure_context_prefixes,
            operation_start_prefix=operation_start_prefix,
            operation_end_prefix=operation_end_prefix,
            operation_timeout=operation_timeout,
            operation_timeouts=operation_timeouts,
            runtime_state_prefix=runtime_state_prefix,
            required_runtime_states=required_runtime_states,
            seen_runtime_states=seen_runtime_states,
            uart_log=uart_log,
        )
        if code == 0 or terminal_result:
            return code
        if flash_complete:
            # Never reflash once device execution may have started. A dropped
            # USB monitor is recovered with no-reset/no-sync monitor attachment.
            print(
                "  [monitor] flash completed but the serial monitor closed; "
                "recovering monitor only (firmware will NOT be reflashed).",
                flush=True,
            )
            return reconnect_monitor(
                image,
                port,
                timeout,
                pass_marker=pass_marker,
                fail_marker=fail_marker,
                success_label=success_label,
                status_interval=status_interval,
                repeat_abort_marker=repeat_abort_marker,
                repeat_abort_arm_marker=repeat_abort_arm_marker,
                phase_start_marker=phase_start_marker,
                phase_end_marker=phase_end_marker,
                phase_timeout=phase_timeout,
                deadline_extension_marker=deadline_extension_marker,
                deadline_extension_seconds=deadline_extension_seconds,
                required_markers=required_markers,
                seen_markers=seen_markers,
                ordered_markers=ordered_markers,
                ordered_progress=ordered_progress,
                failure_context_prefixes=failure_context_prefixes,
                operation_start_prefix=operation_start_prefix,
                operation_end_prefix=operation_end_prefix,
                operation_timeout=operation_timeout,
                operation_timeouts=operation_timeouts,
                runtime_state_prefix=runtime_state_prefix,
                required_runtime_states=required_runtime_states,
                seen_runtime_states=seen_runtime_states,
                uart_log=uart_log,
            )
        if not retryable:
            return code
        if attempt == FLASH_CONNECT_ATTEMPTS:
            print_transport_diagnostics(board, port, attempt)
            return code

    return 1


@reportable_interruptions()
def main() -> int:
    args = parse_args()
    feature = args.board
    evidence = HilEvidence(
        kind="hardware",
        board=args.board,
        port=args.port,
        timeout_seconds=args.timeout,
        profile=f"{feature},hardware-tests",
        include_build_log=False,
    )
    code = 1
    error_text: str | None = None
    try:
        evidence.set_phase("tool-preflight")
        require_tool("cargo")
        require_tool("espflash")
        require_tool("python3")
        validate_layout(layout_for(args.board))
        evidence.set_phase("firmware-build")
        image = build(args.board)
        evidence.bind_firmware(image)
        evidence.set_phase("flash-and-uart")
        with evidence.open_uart() as uart_log:
            code = flash_and_monitor(
                args.board, image, args.port, args.timeout, uart_log=uart_log
            )
        evidence.set_phase("complete" if code == 0 else "flash-and-uart")
    except subprocess.CalledProcessError as error:
        code = error.returncode or 1
        error_text = f"command failed: {error.cmd}"
    except (OSError, RuntimeError) as error:
        code = 1
        error_text = str(error)
        print(f"ERROR: {error}", file=sys.stderr)
    except KeyboardInterrupt:
        code = 130
        error_text = "hardware tests interrupted"
        print("\nHardware tests interrupted.", file=sys.stderr)
    finally:
        evidence.finalize(code, error=error_text)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
