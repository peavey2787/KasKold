#!/usr/bin/env python3
"""Regression coverage for the single pinned toolchain/version policy."""

from pathlib import Path
import json
import tomllib
import sys
import unittest

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))

from toolchains import REQUIRED, load_toolchains  # noqa: E402
from architecture.tooling import toolchain_policy  # noqa: E402
from security.fuzz_targets import registered_targets, validate_targets  # noqa: E402


class ToolchainPolicyTests(unittest.TestCase):
    def test_central_policy_is_complete_and_unique(self) -> None:
        pins = load_toolchains()
        self.assertEqual(set(pins), REQUIRED)
        for key, value in pins.items():
            self.assertTrue(value.strip(), key)

    def test_architecture_toolchain_policy_is_green(self) -> None:
        self.assertEqual(toolchain_policy.check(ROOT), [])

    def test_fuzz_manifest_is_the_only_target_registry(self) -> None:
        targets = registered_targets()
        self.assertGreaterEqual(len(targets), toolchain_policy.MINIMUM_FUZZ_TARGETS)
        self.assertEqual(validate_targets(), [])
        policy = json.loads((ROOT / "qa/checks/security/policy.json").read_text())
        self.assertNotIn("targets", policy["fuzz"])
        runner = (ROOT / "qa/linux/run-security-fuzz.sh").read_text()
        commands = (ROOT / "qa/linux/runner/commands.sh").read_text()
        for source in (runner, commands):
            self.assertIn("fuzz_targets.py", source)

    def test_fuzz_workflow_timeout_must_fit_every_target(self) -> None:
        workflow = "timeout-minutes: {}\n  run: FUZZ_SECONDS=300 bash fuzz.sh\n"
        self.assertEqual(toolchain_policy.fuzz_budget_errors(workflow.format(105), 17), [])
        self.assertEqual(
            toolchain_policy.fuzz_budget_errors(workflow.format(75), 17),
            ["fuzz workflow timeout-minutes must cover 105 minutes for 17 targets"],
        )
        self.assertEqual(
            toolchain_policy.fuzz_budget_errors("run: fuzz.sh\n", 17),
            ["fuzz workflow must state timeout-minutes and FUZZ_SECONDS"],
        )

    def test_root_lock_tracks_wasm_target_dependencies_used_by_workspace_members(self) -> None:
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        packages = {package["name"]: package for package in lock["package"]}
        protocol_dependencies = {entry.split()[0] for entry in packages["kaskold-protocol"].get("dependencies", [])}
        sdk_dependencies = {entry.split()[0] for entry in packages["kaskold-sdk"].get("dependencies", [])}
        # Cargo generates a workspace lock as if all features of every
        # workspace member and all target-specific dependencies are enabled.
        self.assertIn("js-sys", protocol_dependencies)
        self.assertIn("wasm-bindgen", protocol_dependencies)
        self.assertIn("js-sys", sdk_dependencies)
        self.assertIn("wasm-bindgen", sdk_dependencies)

    def test_root_lock_retains_ark_ff_edges(self) -> None:
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        packages = {package["name"]: package for package in lock["package"]}
        for name in (
            "ark-bn254",
            "ark-crypto-primitives",
            "ark-ec",
            "ark-groth16",
            "ark-poly",
            "ark-relations",
            "ark-snark",
        ):
            dependencies = {entry.split()[0] for entry in packages[name].get("dependencies", [])}
            self.assertIn("ark-ff", dependencies, name)

    def test_rqrr_is_independent_workspace_with_own_lock(self) -> None:
        root = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]
        self.assertNotIn("external/rqrr-nostd", root["members"])
        self.assertIn("external/rqrr-nostd", root["exclude"])
        rqrr_manifest = tomllib.loads((ROOT / "external/rqrr-nostd/Cargo.toml").read_text())
        self.assertEqual(rqrr_manifest["workspace"]["resolver"], "2")
        rqrr_lock = tomllib.loads((ROOT / "external/rqrr-nostd/Cargo.lock").read_text())
        self.assertEqual([(p["name"], p["version"]) for p in rqrr_lock["package"]], [("rqrr", "0.10.1-nostd")])


    def test_rustfmt_gate_checks_every_crate_with_pinned_stable(self) -> None:
        from workspace.check_rust_format import format_commands

        stable = load_toolchains()["KASKOLD_STABLE_RUST"]
        commands = format_commands(ROOT, stable)
        self.assertTrue(all(command[:3] == ["rustup", "run", stable] for command in commands))
        self.assertEqual(commands[0][3:], ["cargo", "fmt", "--all", "--", "--check"])
        excluded = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["exclude"]
        manifests = [command[command.index("--manifest-path") + 1] for command in commands if "--manifest-path" in command]
        self.assertEqual(manifests, [f"{entry}/Cargo.toml" for entry in excluded])
        path_only = [command for command in commands if command[3] == "rustfmt"]
        standalone = [command for command in commands[1:] if command not in path_only]
        self.assertEqual([command for command in commands[1:] if "--all" in command], [])
        self.assertEqual([command[-2:] for command in standalone], [["--", "--check"]] * len(excluded))
        self.assertEqual([command.count("--package") for command in standalone], [1] * len(excluded))
        self.assertEqual(
            [command[4:] for command in path_only],
            [
                ["--check", "--edition", "2021", crate_root]
                for crate_root in (
                    "external/serde-yaml-ng-adapter/src/lib.rs",
                    "qa/firmware-sim/shims/esp-alloc/src/lib.rs",
                    "qa/firmware-sim/shims/esp-bootloader-esp-idf/src/lib.rs",
                    "qa/firmware-sim/shims/esp-hal/src/lib.rs",
                    "qa/firmware-sim/shims/esp-hal-macros/src/lib.rs",
                    "qa/firmware-sim/shims/esp-println/src/lib.rs",
                )
            ],
        )


if __name__ == "__main__":
    unittest.main()
