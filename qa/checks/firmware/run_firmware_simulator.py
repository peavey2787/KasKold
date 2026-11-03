#!/usr/bin/env python3
"""Run the real firmware on the virtual CoreS3 and hold its production line coverage to a floor."""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(ROOT / "qa/checks"))
import simulator_coverage  # noqa: E402
from toolchains import load_toolchains  # noqa: E402

TOOLCHAIN = load_toolchains()["KASKOLD_STABLE_RUST"]
SIMULATOR = ROOT / "qa/firmware-sim"
TARGET = ROOT / "target/firmware-sim"
COVERAGE = TARGET / "coverage"
BUILD_PROFILES = COVERAGE / "build"
TEST_PROFILES = COVERAGE / "profiles"
COVERAGE_FLOOR = ROOT / "qa/baselines/firmware_coverage.json"
WORKFLOW_TIERS = ("workflow-test-auto", "workflow-runtime-auto", "workflow-hil-auto")
WORKFLOW_GATE = "every_connected"
RETAIL_FEATURE = "production"
OWNER_FEATURE = "provisioning-ui"
OWNER_JOURNEYS = "owner_firmware"


def commands() -> list[list[str]]:
    journeys = ["cargo", "test", "--locked"]
    retail = [*journeys, "--features", RETAIL_FEATURE]
    owner = [*journeys, "--features", OWNER_FEATURE, OWNER_JOURNEYS]
    tiers = [[*journeys, "--features", tier, WORKFLOW_GATE] for tier in WORKFLOW_TIERS]
    return [journeys, retail, owner, *tiers]


def environment(profiles: Path) -> dict[str, str]:
    return {
        **os.environ,
        "RUSTUP_TOOLCHAIN": TOOLCHAIN,
        "CARGO_TARGET_DIR": str(TARGET),
        "RUSTFLAGS": "-D warnings -C instrument-coverage",
        "LLVM_PROFILE_FILE": str(profiles / "%p-%m.profraw"),
    }


def test_executables(cargo_output: str) -> list[str]:
    """The test binaries a `cargo test --no-run --message-format=json` built."""
    messages = (json.loads(line) for line in cargo_output.splitlines() if line.startswith("{"))
    return [
        message["executable"]
        for message in messages
        if message.get("reason") == "compiler-artifact"
        and message.get("executable")
        and message["profile"]["test"]
    ]


def build(command: list[str]) -> list[str] | None:
    result = subprocess.run(
        [*command, "--no-run", "--message-format=json-render-diagnostics"],
        cwd=SIMULATOR,
        env=environment(BUILD_PROFILES),
        check=False,
        stdout=subprocess.PIPE,
        text=True,
    )
    return test_executables(result.stdout) if result.returncode == 0 else None


def run_scenarios() -> list[str] | None:
    objects: list[str] = []
    for command in commands():
        print("+", " ".join(command), flush=True)
        built = build(command)
        if built is None:
            return None
        objects.extend(built)
        if subprocess.run(command, cwd=SIMULATOR, env=environment(TEST_PROFILES), check=False).returncode != 0:
            return None
    return objects


def main() -> int:
    simulator_coverage.ensure_llvm_tools(TOOLCHAIN)
    shutil.rmtree(COVERAGE, ignore_errors=True)
    TEST_PROFILES.mkdir(parents=True)
    BUILD_PROFILES.mkdir(parents=True)
    objects = run_scenarios()
    if objects is None:
        return 1
    lcov = simulator_coverage.export_lcov(TEST_PROFILES, objects, TOOLCHAIN)
    shutil.rmtree(TEST_PROFILES)
    shutil.rmtree(BUILD_PROFILES)
    hit, total = simulator_coverage.line_coverage(lcov, ROOT)
    print(f"production firmware line coverage: {hit}/{total} = {100 * hit / max(total, 1):.2f}%")
    error = simulator_coverage.floor_error(hit, total, simulator_coverage.load_floor(COVERAGE_FLOOR))
    if error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
