from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[4]
CHECKS = ROOT / "qa/checks/firmware"
sys.path.insert(0, str(CHECKS))
import simulator_coverage  # noqa: E402

spec = importlib.util.spec_from_file_location("run_firmware_simulator", CHECKS / "run_firmware_simulator.py")
assert spec and spec.loader
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

WORK = Path("C:/work/kaskold")


def record(path: str, *lines: tuple[int, int]) -> str:
    rows = "".join(f"DA:{number},{count}\n" for number, count in lines)
    return f"SF:{path}\n{rows}end_of_record\n"


class LineCoverage(unittest.TestCase):
    def test_counts_production_lines_through_a_windows_relative_path(self) -> None:
        lcov = record(
            r"C:\work\kaskold\qa\firmware-sim\..\..\apps\kaskold-hardware\src\main.rs",
            (1, 4),
            (2, 0),
            (3, 1),
        )
        self.assertEqual(simulator_coverage.line_coverage(lcov, WORK), (2, 3))

    def test_a_line_hit_by_any_build_counts_once(self) -> None:
        path = "C:/work/kaskold/crates/kaskold-hardware-core/src/lib.rs"
        lcov = record(path, (1, 0), (2, 0)) + record(path, (1, 3), (2, 0))
        self.assertEqual(simulator_coverage.line_coverage(lcov, WORK), (1, 2))

    def test_test_code_and_other_crates_are_left_out(self) -> None:
        lcov = (
            record("C:/work/kaskold/apps/kaskold-hardware/src/runtime/workflow_tests/a.rs", (1, 1))
            + record("C:/work/kaskold/apps/kaskold-hardware/src/services/unit_tests/a.rs", (1, 1))
            + record("C:/work/kaskold/qa/firmware-sim/src/lib.rs", (1, 1))
            + record("C:/cargo/registry/kaspa-portal/src/lib.rs", (1, 1))
        )
        self.assertEqual(simulator_coverage.line_coverage(lcov, WORK), (0, 0))


class Floor(unittest.TestCase):
    def test_at_the_floor_passes(self) -> None:
        self.assertIsNone(simulator_coverage.floor_error(66, 100, 66.0))

    def test_below_the_floor_names_both_numbers(self) -> None:
        error = simulator_coverage.floor_error(65, 100, 66.0)
        self.assertIn("65.00%", error or "")
        self.assertIn("66.00%", error or "")

    def test_an_empty_profile_is_a_failure_not_a_pass(self) -> None:
        self.assertIn("no production", simulator_coverage.floor_error(0, 0, 0.0) or "")

    def test_the_committed_floor_is_a_real_percentage(self) -> None:
        self.assertTrue(0 < simulator_coverage.load_floor(runner.COVERAGE_FLOOR) <= 100)


class Runner(unittest.TestCase):
    def test_only_test_binaries_are_coverage_objects(self) -> None:
        artifact = {"reason": "compiler-artifact", "executable": "sim.exe", "profile": {"test": True}}
        build_script = {"reason": "compiler-artifact", "executable": "build.exe", "profile": {"test": False}}
        library = {"reason": "compiler-artifact", "executable": None, "profile": {"test": False}}
        finished = {"reason": "build-finished", "success": True}
        output = "\n".join(json.dumps(item) for item in (build_script, library, artifact, finished))
        self.assertEqual(runner.test_executables(output), ["sim.exe"])

    def test_every_scenario_build_is_instrumented(self) -> None:
        flags = runner.environment(runner.TEST_PROFILES)["RUSTFLAGS"]
        self.assertIn("-C instrument-coverage", flags)
        self.assertIn("-D warnings", flags)

    def test_the_simulator_builds_with_the_pinned_stable_toolchain(self) -> None:
        environment = runner.environment(runner.TEST_PROFILES)
        self.assertEqual(environment["RUSTUP_TOOLCHAIN"], runner.load_toolchains()["KASKOLD_STABLE_RUST"])

    def test_the_hil_image_runs_both_with_and_without_a_card(self) -> None:
        hil = [command[-2:] for command in runner.commands() if "workflow-hil-auto" in command]
        self.assertEqual(hil, [["workflow-hil-auto", "every_connected"], ["workflow-hil-auto", "hil_without_a_card"]])


if __name__ == "__main__":
    unittest.main()
