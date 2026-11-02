#!/usr/bin/env python3
"""Launch ESP32-S3 QEMU, stream UART, and enforce guest test markers."""

from __future__ import annotations

import argparse
import os
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
import queue
import signal
import subprocess
import threading
import sys
import time

BEGIN_MARKER = b"KASKOLD_QEMU_TESTS_BEGIN"
UART_MARKER = b"KASKOLD_QEMU_UART_PROBE"
PASS_MARKER = b"KASKOLD_QEMU_TESTS_PASS"
FAIL_MARKER = b"KASKOLD_QEMU_TESTS_FAIL"
# The firmware fault handler wipes and resets; a rebooted guest could pass on a
# later boot, so any panic fails the run.
PANIC_MARKER = b"KASKOLD_PANIC"
BOARD_MARKER = b"Board: ESP32-S3 QEMU"
REQUIRED_MARKERS = (BOARD_MARKER, BEGIN_MARKER, UART_MARKER)
TRACKED_MARKERS = (*REQUIRED_MARKERS, PASS_MARKER, FAIL_MARKER, PANIC_MARKER)
OBSERVED_LIMIT = 65536
OBSERVED_KEEP = 32768


@dataclass
class _GuestSession:
    process: subprocess.Popen[bytes]
    output_queue: queue.Queue[tuple[str, object]]
    observed: bytearray = field(default_factory=bytearray)
    seen: set[bytes] = field(default_factory=set)
    passed: bool = False
    stdout_eof: bool = False


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the KasKold ESP32-S3 QEMU hardware test image."
    )
    parser.add_argument("--qemu", required=True, type=Path)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--keep-running", action="store_true")
    return parser.parse_args()


def build_command(qemu: Path, image: Path) -> list[str]:
    return [
        str(qemu),
        "-nographic",
        "-machine",
        "esp32s3",
        "-drive",
        f"file={image},if=mtd,format=raw",
        "-global",
        "driver=timer.esp32c3.timg,property=wdt_disable,value=true",
    ]


def stop_process(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=3)


def validate_paths(qemu: Path, image: Path) -> None:
    if not qemu.is_file() or not os.access(qemu, os.X_OK):
        raise SystemExit(f"ERROR: QEMU executable is not runnable: {qemu}")
    if not image.is_file():
        raise SystemExit(f"ERROR: QEMU flash image does not exist: {image}")


def missing_required(seen: set[bytes]) -> list[str]:
    return [marker.decode() for marker in REQUIRED_MARKERS if marker not in seen]


def _start_guest(args: argparse.Namespace) -> subprocess.Popen[bytes]:
    validate_paths(args.qemu, args.image)
    command = build_command(args.qemu, args.image)
    print("QEMU test command:", " ".join(command), flush=True)
    return subprocess.Popen(
        command,
        # Automated QEMU tests never need keyboard input.  Give -nographic a
        # private pipe so it cannot put the maintainer console into raw/no-echo
        # mode, while keeping the input stream open for the life of the guest.
        # Explicit keep-running mode remains interactive for Ctrl-A/X.
        stdin=None if args.keep_running else subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=0,
    )


def _start_uart_reader(
    process: subprocess.Popen[bytes], output_queue: queue.Queue[tuple[str, object]]
) -> None:
    stdout = process.stdout
    assert stdout is not None

    def read_stdout() -> None:
        try:
            while True:
                chunk = stdout.read(4096)
                if not chunk:
                    break
                output_queue.put(("data", chunk))
        except BaseException as exc:  # surfaced to the main thread below
            output_queue.put(("error", exc))
        finally:
            output_queue.put(("eof", None))

    reader = threading.Thread(target=read_stdout, name="qemu-uart-reader", daemon=True)
    reader.start()


def _make_signal_forwarder(
    process: subprocess.Popen[bytes],
) -> Callable[[int, object], None]:
    def forward_signal(signum: int, _frame: object) -> None:
        if process.poll() is None:
            process.send_signal(signum)

    return forward_signal


def _record_chunk(session: _GuestSession, chunk: bytes) -> None:
    sys.stdout.buffer.write(chunk)
    sys.stdout.buffer.flush()
    session.observed.extend(chunk)
    if len(session.observed) > OBSERVED_LIMIT:
        del session.observed[:-OBSERVED_KEEP]
    for marker in TRACKED_MARKERS:
        if marker in session.observed:
            session.seen.add(marker)


def _wait_for_event(output_queue: queue.Queue[tuple[str, object]]) -> tuple[str, object]:
    try:
        return output_queue.get(timeout=0.1)
    except queue.Empty:
        return "idle", None


def _next_pending_event(output_queue: queue.Queue[tuple[str, object]]) -> tuple[str, object]:
    try:
        return output_queue.get_nowait()
    except queue.Empty:
        return "idle", None


def _handle_event(session: _GuestSession, event: str, payload: object) -> int | None:
    if event == "data":
        assert isinstance(payload, bytes)
        _record_chunk(session, payload)
    elif event == "error":
        stop_process(session.process)
        print(
            f"\nERROR: failed while reading QEMU UART output: {payload}",
            file=sys.stderr,
        )
        return 1
    elif event == "eof":
        session.stdout_eof = True
    return None


def _drain_events(session: _GuestSession) -> int | None:
    event, payload = _wait_for_event(session.output_queue)
    while event != "idle":
        code = _handle_event(session, event, payload)
        if code is not None:
            return code
        event, payload = _next_pending_event(session.output_queue)
    return None


def _guest_failure_code(session: _GuestSession) -> int | None:
    if PANIC_MARKER in session.seen:
        print("\nERROR: guest firmware panicked and reset.", file=sys.stderr)
        stop_process(session.process)
        return 1
    if FAIL_MARKER in session.seen:
        print("\nERROR: guest QEMU hardware tests failed.", file=sys.stderr)
        stop_process(session.process)
        return 1
    return None


def _handle_pass(session: _GuestSession, keep_running: bool) -> int | None:
    if PASS_MARKER not in session.seen or session.passed:
        return None
    missing = missing_required(session.seen)
    if missing:
        print(
            "\nERROR: guest passed without required markers: " + ", ".join(missing),
            file=sys.stderr,
        )
        stop_process(session.process)
        return 1
    session.passed = True
    print("\nPASS: ESP32-S3 QEMU hardware tests completed.", flush=True)
    if not keep_running:
        stop_process(session.process)
        return 0
    print("QEMU runtime remains active. Exit with Ctrl-A, then X.", flush=True)
    return None


def _exit_after_eof(session: _GuestSession) -> int | None:
    status = session.process.poll()
    if status is None or not session.stdout_eof:
        return None
    if session.passed:
        return 0
    print(
        f"\nERROR: QEMU exited before the pass marker (exit {status}).",
        file=sys.stderr,
    )
    return status or 1


def _step(session: _GuestSession, keep_running: bool) -> int | None:
    code = _drain_events(session)
    if code is None:
        code = _guest_failure_code(session)
    if code is None:
        code = _handle_pass(session, keep_running)
    if code is None:
        code = _exit_after_eof(session)
    return code


def _run_session(session: _GuestSession, args: argparse.Namespace, deadline: float) -> int:
    while True:
        if not session.passed and time.monotonic() >= deadline:
            print(
                f"\nERROR: QEMU tests timed out after {args.timeout} seconds.",
                file=sys.stderr,
            )
            stop_process(session.process)
            return 124
        code = _step(session, args.keep_running)
        if code is not None:
            return code


def launch(args: argparse.Namespace) -> int:
    process = _start_guest(args)
    output_queue: queue.Queue[tuple[str, object]] = queue.Queue()
    _start_uart_reader(process, output_queue)
    session = _GuestSession(process=process, output_queue=output_queue)
    deadline = time.monotonic() + args.timeout
    forward_signal = _make_signal_forwarder(process)
    previous_int = signal.signal(signal.SIGINT, forward_signal)
    previous_term = signal.signal(signal.SIGTERM, forward_signal)
    try:
        return _run_session(session, args, deadline)
    finally:
        signal.signal(signal.SIGINT, previous_int)
        signal.signal(signal.SIGTERM, previous_term)


def main() -> int:
    args = parse_args()
    if args.timeout <= 0:
        raise SystemExit("ERROR: --timeout must be a positive integer")
    return launch(args)


if __name__ == "__main__":
    sys.exit(main())
