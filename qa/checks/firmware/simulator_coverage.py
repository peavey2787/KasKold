"""Production firmware line coverage from the simulator's LLVM profiles."""
from __future__ import annotations

import json
import os
from pathlib import Path
import posixpath
import subprocess

PRODUCTION_ROOTS = ("apps/kaskold-hardware/src/", "crates/kaskold-hardware-core/src/")
TEST_DIRECTORIES = {"workflow_tests", "unit_tests"}


def relative_source(path: str, root: Path) -> str | None:
    normal = posixpath.normpath(path.strip().replace("\\", "/"))
    prefix = root.as_posix().rstrip("/") + "/"
    if not normal.lower().startswith(prefix.lower()):
        return None
    return normal[len(prefix):]


def is_production_source(relative: str) -> bool:
    return relative.startswith(PRODUCTION_ROOTS) and not TEST_DIRECTORIES.intersection(relative.split("/"))


def production_line_counts(lcov: str, root: Path) -> dict[str, dict[int, int]]:
    """Hit count per line of each production file; a line hit by any object counts."""
    files: dict[str, dict[int, int]] = {}
    current: dict[int, int] | None = None
    for line in lcov.splitlines():
        if line.startswith("SF:"):
            relative = relative_source(line[3:], root)
            keep = relative is not None and is_production_source(relative)
            current = files.setdefault(relative, {}) if keep else None
        elif line.startswith("DA:") and current is not None:
            number, count = line[3:].split(",")[:2]
            current[int(number)] = max(current.get(int(number), 0), int(count))
    return files


def line_coverage(lcov: str, root: Path) -> tuple[int, int]:
    lines = [count for file in production_line_counts(lcov, root).values() for count in file.values()]
    return sum(1 for count in lines if count), len(lines)


def floor_error(hit: int, total: int, floor: float) -> str | None:
    if total == 0:
        return "the simulator profile covers no production firmware lines"
    percent = 100 * hit / total
    if percent < floor:
        return f"production firmware line coverage {percent:.2f}% fell below the floor {floor:.2f}%"
    return None


def load_floor(baseline: Path) -> float:
    return float(json.loads(baseline.read_text(encoding="utf-8"))["production_line_percent"])


def _rustc(cwd: Path, *arguments: str) -> str:
    return subprocess.run(["rustc", *arguments], cwd=cwd, check=True, capture_output=True, text=True).stdout


def llvm_tool(name: str, cwd: Path) -> Path:
    sysroot = _rustc(cwd, "--print", "sysroot").strip()
    host = next(line.split()[1] for line in _rustc(cwd, "-vV").splitlines() if line.startswith("host:"))
    suffix = ".exe" if os.name == "nt" else ""
    return Path(sysroot) / "lib/rustlib" / host / "bin" / f"{name}{suffix}"


def export_lcov(profiles: Path, objects: list[str], cwd: Path) -> str:
    merged = profiles.parent / "merged.profdata"
    listing = profiles.parent / "profiles.txt"
    listing.write_text("\n".join(str(path) for path in sorted(profiles.glob("*.profraw"))), encoding="utf-8")
    subprocess.run(
        [llvm_tool("llvm-profdata", cwd), "merge", "-sparse", f"--input-files={listing}", "-o", merged],
        check=True,
    )
    pairs = [argument for executable in objects for argument in ("--object", executable)]
    exported = subprocess.run(
        [llvm_tool("llvm-cov", cwd), "export", "--format=lcov", f"--instr-profile={merged}", *pairs],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return exported.stdout
