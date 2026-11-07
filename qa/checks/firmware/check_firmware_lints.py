#!/usr/bin/env python3
"""Run the enforced firmware lint profile across supported board modes."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))

from tools.build.firmware.matrix_runner import run_firmware_matrix  # noqa: E402
from firmware_lint_policy import check_policy  # noqa: E402

# Every default Clippy group (correctness, suspicious, complexity, perf and
# style) is denied across the whole firmware matrix, plus the architectural
# size, complexity and boolean-state limits this repository selects from the
# pedantic group. Item-level exceptions remain visible and review-locked.
LINT_ARGS = (
    "-D", "warnings",
    "-D", "clippy::all",
    "-D", "clippy::too_many_lines",
    "-D", "clippy::cognitive_complexity",
    "-D", "clippy::struct_excessive_bools",
    "-D", "dead_code",
    "-D", "unused_imports",
    "-D", "unused_variables",
    "-D", "unused_mut",
    "-D", "unused_assignments",
    "-D", "unused_unsafe",
)


def main() -> int:
    policy_errors = check_policy(ROOT)
    if policy_errors:
        for error in policy_errors:
            print(f"ERROR: {error}")
        return 1
    return run_firmware_matrix(
        "clippy",
        trailing_args=LINT_ARGS,
        success_message=(
            "PASS: firmware lint matrix is clean for hardware and QEMU modes"
        ),
    )


if __name__ == "__main__":
    sys.exit(main())
