#!/usr/bin/env python3
"""Line-oriented progress and ETA reporting for cargo-mutants runs."""
from __future__ import annotations

from collections import deque
from dataclasses import dataclass
import json
import os
from pathlib import Path
import threading
import time
from typing import Any

SUMMARY_KEYS = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
}


@dataclass(frozen=True)
class MutationProgressSnapshot:
    total: int
    completed: int
    reused: int
    caught: int
    missed: int
    timeout: int
    unviable: int
    last_mutant: str | None

    @property
    def accounted(self) -> int:
        value = self.completed + self.reused
        return min(self.total, value) if self.total else value


def _read_json(path: Path) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None


def _count_nonempty_lines(path: Path) -> int:
    try:
        return sum(1 for line in path.read_text(encoding="utf-8").splitlines() if line.strip())
    except (OSError, UnicodeDecodeError):
        return 0


def _mutant_name(outcome: dict[str, Any]) -> str | None:
    scenario = outcome.get("scenario")
    if not isinstance(scenario, dict):
        return None
    mutant = scenario.get("Mutant")
    if not isinstance(mutant, dict):
        return None
    name = mutant.get("name")
    return name if isinstance(name, str) and name else None


def read_progress_snapshot(output_parent: Path) -> MutationProgressSnapshot:
    """Read cargo-mutants' incrementally written evidence without blocking it."""
    results = output_parent / "mutants.out"
    inventory = _read_json(results / "mutants.json")
    total = len(inventory) if isinstance(inventory, list) else 0
    reused = _count_nonempty_lines(results / "previously_caught.txt")

    caught = missed = timeout = unviable = 0
    completed = 0
    last_mutant = None
    document = _read_json(results / "outcomes.json")
    if isinstance(document, dict):
        outcomes = document.get("outcomes")
        if isinstance(outcomes, list):
            for outcome in outcomes:
                if not isinstance(outcome, dict):
                    continue
                name = _mutant_name(outcome)
                if name is None:
                    continue
                completed += 1
                last_mutant = name
                key = SUMMARY_KEYS.get(outcome.get("summary"))
                if key == "caught":
                    caught += 1
                elif key == "missed":
                    missed += 1
                elif key == "timeout":
                    timeout += 1
                elif key == "unviable":
                    unviable += 1

    # cargo-mutants also updates the outcome text lists incrementally. They are a
    # fallback if outcomes.json happens to be between rewrites during a poll.
    if completed == 0:
        caught = _count_nonempty_lines(results / "caught.txt")
        missed = _count_nonempty_lines(results / "missed.txt")
        timeout = _count_nonempty_lines(results / "timeout.txt")
        unviable = _count_nonempty_lines(results / "unviable.txt")
        completed = caught + missed + timeout + unviable

    return MutationProgressSnapshot(
        total=total,
        completed=completed,
        reused=reused,
        caught=caught,
        missed=missed,
        timeout=timeout,
        unviable=unviable,
        last_mutant=last_mutant,
    )


def _format_duration(seconds: float | None) -> str:
    if seconds is None or seconds < 0:
        return "calculating"
    rounded = int(seconds + 0.5)
    hours, remainder = divmod(rounded, 3600)
    minutes, secs = divmod(remainder, 60)
    if hours:
        return f"{hours}h {minutes:02d}m"
    if minutes:
        return f"{minutes}m {secs:02d}s"
    return f"{secs}s"


def format_progress_line(
    snapshot: MutationProgressSnapshot,
    *,
    elapsed_seconds: float,
    rate_per_second: float | None,
) -> str:
    total = snapshot.total
    accounted = snapshot.accounted
    percent = (accounted / total * 100.0) if total else 0.0
    remaining = max(0, total - accounted) if total else 0
    eta_seconds = remaining / rate_per_second if rate_per_second and rate_per_second > 0 else None
    rate_per_minute = rate_per_second * 60.0 if rate_per_second and rate_per_second > 0 else None

    pieces = [
        f"[mutation] {accounted:,}/{total:,} ({percent:5.1f}%)"
        if total else f"[mutation] completed {snapshot.completed:,}",
        f"elapsed {_format_duration(elapsed_seconds)}",
        f"ETA ~{_format_duration(eta_seconds)}" if eta_seconds is not None else "ETA calculating",
    ]
    if rate_per_minute is not None:
        pieces.append(f"rate {rate_per_minute:.2f}/min")
    pieces.append(
        "outcomes "
        f"caught={snapshot.caught:,} missed={snapshot.missed:,} "
        f"timeout={snapshot.timeout:,} unviable={snapshot.unviable:,}"
    )
    if snapshot.reused:
        pieces.append(f"reused={snapshot.reused:,}")
    if snapshot.last_mutant:
        name = snapshot.last_mutant
        if len(name) > 110:
            name = name[:107] + "..."
        pieces.append(f"last={name}")
    return " | ".join(pieces)


class MutationProgressReporter:
    """Poll cargo-mutants evidence and emit readable progress under non-TTY launchers."""

    def __init__(self, output_parent: Path, *, interval_seconds: float | None = None) -> None:
        self.output_parent = output_parent
        configured = os.environ.get("KASKOLD_MUTATION_PROGRESS_INTERVAL_SECONDS")
        if interval_seconds is None:
            try:
                interval_seconds = float(configured) if configured else 15.0
            except ValueError:
                interval_seconds = 15.0
        self.interval_seconds = max(1.0, interval_seconds)
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None
        self._started = 0.0
        self._run_epoch = 0.0
        self._samples: deque[tuple[float, int]] = deque(maxlen=24)
        self._last_accounted = -1
        self.log_path = output_parent / "mutation-progress.log"

    def __enter__(self) -> "MutationProgressReporter":
        self.output_parent.mkdir(parents=True, exist_ok=True)
        self._started = time.monotonic()
        self._run_epoch = time.time()
        self._write(
            "[mutation] Progress monitor active; updates every "
            f"{self.interval_seconds:g}s. ETA becomes available after mutant outcomes begin."
        )
        self._thread = threading.Thread(target=self._loop, name="mutation-progress", daemon=True)
        self._thread.start()
        return self

    def __exit__(self, exc_type, exc, traceback) -> None:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=max(2.0, self.interval_seconds + 1.0))
        self.emit(force=True)

    def _write(self, line: str) -> None:
        print(line, flush=True)
        try:
            with self.log_path.open("a", encoding="utf-8", newline="\n") as handle:
                handle.write(line + "\n")
        except OSError:
            pass

    def _loop(self) -> None:
        while not self._stop.is_set():
            self.emit()
            self._stop.wait(self.interval_seconds)

    def _rate(self, now: float, accounted: int) -> float | None:
        if accounted < 1:
            return None
        if not self._samples or self._samples[-1][1] != accounted:
            self._samples.append((now, accounted))
        if len(self._samples) < 2:
            return None
        newest_time, newest_count = self._samples[-1]
        oldest_time, oldest_count = self._samples[0]
        for sample_time, sample_count in self._samples:
            if newest_time - sample_time <= 20 * 60:
                oldest_time, oldest_count = sample_time, sample_count
                break
        elapsed = newest_time - oldest_time
        delta = newest_count - oldest_count
        if elapsed < 1.0 or delta <= 0:
            return None
        return delta / elapsed

    def emit(self, *, force: bool = False) -> None:
        now = time.monotonic()
        lock = self.output_parent / "mutants.out" / "lock.json"
        try:
            current_run_visible = lock.stat().st_mtime >= self._run_epoch - 1.0
        except OSError:
            current_run_visible = False
        if not current_run_visible:
            if force:
                self._write("[mutation] cargo-mutants exited before current-run progress evidence became visible.")
            return

        snapshot = read_progress_snapshot(self.output_parent)
        accounted = snapshot.accounted
        rate = self._rate(now, accounted)
        line = format_progress_line(
            snapshot,
            elapsed_seconds=now - self._started,
            rate_per_second=rate,
        )
        self._write(line)
        self._last_accounted = accounted
