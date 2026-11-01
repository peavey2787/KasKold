#!/usr/bin/env python3
"""Run the real firmware on the virtual CoreS3: every journey, the retail build, every workflow tier."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
SIMULATOR = ROOT / "qa/firmware-sim"
TARGET = ROOT / "target/firmware-sim"
WORKFLOW_TIERS = ("workflow-test-auto", "workflow-runtime-auto", "workflow-hil-auto")
WORKFLOW_GATE = "every_connected"
RETAIL_FEATURE = "production"


def commands() -> list[list[str]]:
    journeys = ["cargo", "test", "--locked"]
    retail = [*journeys, "--features", RETAIL_FEATURE]
    tiers = [[*journeys, "--features", tier, WORKFLOW_GATE] for tier in WORKFLOW_TIERS]
    return [journeys, retail, *tiers]


def main() -> int:
    environment = {**os.environ, "CARGO_TARGET_DIR": str(TARGET), "RUSTFLAGS": "-D warnings"}
    for command in commands():
        print("+", " ".join(command), flush=True)
        result = subprocess.run(command, cwd=SIMULATOR, env=environment, check=False)
        if result.returncode != 0:
            return result.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main())
