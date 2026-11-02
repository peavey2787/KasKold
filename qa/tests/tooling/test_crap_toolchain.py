#!/usr/bin/env python3
"""Toolchain, runner, and artifact tests for the consolidated CRAP quality check."""

from __future__ import annotations

from collections.abc import Callable
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

from qa.tests.tooling.crap_check_test_support import CrapCheckTestCase, ROOT

sys.path.insert(0, str(ROOT / "qa/checks"))
from toolchains import load_toolchains  # noqa: E402

PINS = load_toolchains()
PACKAGER = ROOT / "qa/checks/quality/crap/package_branch_artifacts.py"
BRANCH_EVIDENCE_FILES = (
    "cargo_crap.json",
    "current.json",
    "crap_summary.json",
    "health_summary.json",
    "coverage_run.txt",
    "crap_run.txt",
    "crap_report_prod.txt",
    "browser_recovery/summary.json",
    "browser_recovery/v8-coverage.json",
    "browser_recovery/report.txt",
    "web_runtime/summary.json",
    "web_runtime/v8-coverage.json",
    "web_runtime/report.txt",
)


def branch_run_document(requested: bool = True, available: bool = True, found: int = 4) -> str:
    branches = {"available": available, "found": found, "hit": 3, "percent": 75.0}
    return json.dumps({"branch_coverage_requested": requested, "coverage": {"branches": branches}})


def write_branch_evidence(evidence: Path) -> Path:
    evidence.mkdir()
    (evidence / "run.json").write_text(branch_run_document(), encoding="utf-8")
    (evidence / "lcov.info").write_text("BRF:4\nBRH:3\n", encoding="utf-8")
    for relative in BRANCH_EVIDENCE_FILES:
        path = evidence / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("evidence\n", encoding="utf-8")
    return evidence


def run_packager(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(PACKAGER), *arguments],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def rewrite_run(**fields: object) -> Callable[[Path], object]:
    return lambda evidence: (evidence / "run.json").write_text(branch_run_document(**fields), encoding="utf-8")


def remove_file(relative: str) -> Callable[[Path], None]:
    return lambda evidence: (evidence / relative).unlink()


BRANCH_EVIDENCE_DAMAGE = {
    "branch coverage not requested": rewrite_run(requested=False),
    "branch data unavailable": rewrite_run(available=False),
    "no branches found": rewrite_run(found=0),
    "lcov has no BRF record": lambda evidence: (evidence / "lcov.info").write_text("BRH:3\n", encoding="utf-8"),
    "web runtime summary missing": remove_file("web_runtime/summary.json"),
    "web runtime v8 coverage missing": remove_file("web_runtime/v8-coverage.json"),
    "web runtime report missing": remove_file("web_runtime/report.txt"),
}


class CrapToolchainTests(CrapCheckTestCase):
    def test_windows_crap_runner_resolves_repository_root(self) -> None:
        runner = ROOT / "scripts/windows/quality/crap_windows.py"
        spec = importlib.util.spec_from_file_location("kaskold_crap_windows_root_test", runner)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        with mock.patch.dict(sys.modules, {spec.name: module}):
            spec.loader.exec_module(module)

        self.assertEqual(module.ROOT, ROOT)
        self.assertTrue((module.ROOT / "qa/checks/quality/crap/coverage_manifest.py").is_file())


    @unittest.skipUnless(os.name == "posix", "Linux CRAP runner execution is POSIX-specific")
    def test_optional_generator_skips_when_tools_are_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            for name in ("cargo", "rustup"):
                tool = fake_bin / name
                tool.write_text("#!/usr/bin/env sh\nexit 1\n")
                tool.chmod(0o755)

            output = root / "output"
            environment = dict(os.environ)
            environment["PATH"] = f"{fake_bin}:{environment.get('PATH', '')}"
            environment["CRAP_OUTPUT_DIR"] = str(output)

            result = subprocess.run(
                [str(ROOT / "scripts/linux/quality/crap.sh")],
                cwd=ROOT,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("CRAP report generation skipped", result.stdout)
            self.assertIn("remaining QA catalog will continue", result.stdout)
            self.assertFalse(output.exists())


    @unittest.skipUnless(os.name == "posix", "Linux CRAP runner execution is POSIX-specific")
    def test_generator_runs_visible_coverage_and_keeps_bundle_in_target_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake_bin = root / "bin"
            fake_bin.mkdir()

            cargo = fake_bin / "cargo"
            cargo.write_text(
                r"""#!/usr/bin/env python3
import json
from pathlib import Path
import sys

args = sys.argv[1:]
if args[:2] == ["llvm-cov", "--version"]:
    print("cargo-llvm-cov 0.6.21")
    raise SystemExit(0)
if args[:2] == ["crap", "--version"]:
    print("cargo-crap " + __import__("os").environ["TEST_CARGO_CRAP_VERSION"])
    raise SystemExit(0)
if args[:1] == ["llvm-cov"]:
    output = Path(args[args.index("--output-path") + 1])
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("TN:\nSF:crates/shared-signer/src/example.rs\nDA:1,1\nend_of_record\n")
    print("running 1 test")
    print("test result: ok. 1 passed; 0 failed")
    raise SystemExit(0)
if args[:1] == ["crap"]:
    output = Path(args[args.index("--output") + 1])
    output.parent.mkdir(parents=True, exist_ok=True)
    report_format = args[args.index("--format") + 1]
    if report_format == "json":
        secondary = any(scope in output.name for scope in ("companion_web", "vault_web", "firmware"))
        document = {
            "version": __import__("os").environ["TEST_CARGO_CRAP_VERSION"],
            "entries": [] if secondary else [{
                "file": "crates/shared-signer/src/example.rs",
                "function": "example",
                "line": 1,
                "cyclomatic": 1.0,
                "coverage": 100.0,
                "crap": 1.0
            }]
        }
        if "--lcov" in args:
            document["diagnostics"] = {
                "analyzed_files": 1,
                "lcov_files": 1,
                "matched_files": 1,
                "source_only": {"count": 0, "examples": []},
                "lcov_only": {"count": 0, "examples": []}
            }
        output.write_text(json.dumps(document))
    else:
        output.write_text(
            "│ ✓ ┆ 1.0 ┆ 1 ┆ ██████████ 100.0% ┆ example ┆ ./crates/shared-signer/src/example.rs:1 │\n"
            "✓ 0/1 function(s) exceed CRAP threshold 30.\n"
        )
    raise SystemExit(0)
raise SystemExit(2)
"""
            )
            cargo.chmod(0o755)

            rustup = fake_bin / "rustup"
            rustup.write_text(
                """#!/usr/bin/env sh
if [ "$1 $3" = "run rustc" ]; then
  echo "rustc 1.95.0"
  exit 0
fi
if [ "$1 $3" = "run cargo" ]; then
  shift 3
  exec cargo "$@"
fi
if [ "$1 $2" = "component list" ]; then
  echo "llvm-tools-preview-x86_64-unknown-linux-gnu (installed)"
  exit 0
fi
exit 2
"""
            )
            rustup.chmod(0o755)

            # Keep this tooling unit test focused on the Rust coverage/CRAP
            # runner. The production script still runs both heavyweight browser
            # coverage jobs; this fake Python shim intercepts only those two
            # entry points and emits valid supplemental documents.
            python3 = fake_bin / "python3"
            python3.write_text(
                """#!/usr/bin/env sh
set -eu
case "${1:-}" in
  *run_web_recovery_coverage.py)
    shift
    [ "${1:-}" = "--output-dir" ]
    out="$2"
    mkdir -p "$out"
    cat > "$out/summary.json" <<'JSON'
{"schema_version":1,"domain":"browser_recovery","tests":{"passed":true},"files":{"expected":1,"measured":1,"missing":[],"unexpected":[]},"coverage":{"lines":{"percent":100.0,"available":true,"target":90.0,"met":true},"functions":{"percent":100.0,"available":true,"target":90.0,"met":true},"branches":{"percent":100.0,"available":true,"target":90.0,"met":true}},"met":true}
JSON
    printf '{}\n' > "$out/v8-coverage.json"
    printf 'test browser recovery coverage\n' > "$out/report.txt"
    exit 0
    ;;
  *run_web_runtime_coverage.py)
    shift
    [ "${1:-}" = "--output-dir" ]
    out="$2"
    mkdir -p "$out"
    cat > "$out/summary.json" <<'JSON'
{"schema_version":1,"domain":"web_runtime","tests_passed":true,"files":{"reachable":1,"measured":1,"missing":[],"mapping_percent":100.0},"coverage":{"lines":100.0,"functions":100.0,"branches":100.0},"met":true}
JSON
    printf '{}\n' > "$out/v8-coverage.json"
    printf 'test web runtime coverage\n' > "$out/report.txt"
    exit 0
    ;;
esac
exec __REAL_PYTHON__ "$@"
""".replace("__REAL_PYTHON__", sys.executable)
            )
            python3.chmod(0o755)

            output = root / "output"
            ratchet = root / "ratchet.json"
            ratchet.write_text(json.dumps({
                "schema_version": 1,
                "coverage_profile": {
                    "branch_instrumentation": False,
                    "dev_opt_level": "0",
                    "test_opt_level": "0",
                },
                "host_production_minimum_percent": {
                    "lines": 0.0,
                    "functions": 0.0,
                    "branches": 0.0,
                },
            }))
            environment = dict(os.environ)
            environment["PATH"] = f"{fake_bin}:{environment.get('PATH', '')}"
            environment["CRAP_OUTPUT_DIR"] = str(output)
            environment["CRAP_RATCHET_PATH"] = str(ratchet)
            environment["TEST_CARGO_CRAP_VERSION"] = PINS["KASKOLD_CARGO_CRAP_VERSION"]

            result = subprocess.run(
                [str(ROOT / "scripts/linux/quality/crap.sh")],
                cwd=ROOT,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("[CRAP 1/4] Running", result.stdout)
            self.assertIn("scope-aligned coverage completed", result.stdout)
            self.assertIn("[CRAP 4/4] PASS", result.stdout)
            self.assertIn("remaining QA catalog starts now", result.stdout)
            for name in (
                "lcov.info",
                "coverage_run.txt",
                "crap_run.txt",
                "run.json",
                "cargo_crap.json",
                "crap_report_prod.txt",
                "crap_summary.json",
                "health_summary.json",
                "current.json",
            ):
                self.assertTrue((output / name).is_file(), name)
            self.assertGreater((output / "crap_run.txt").stat().st_size, 0)
            self.assertIn("KasKold CRAP analysis log", (output / "crap_run.txt").read_text())
            run = json.loads((output / "run.json").read_text())
            self.assertGreater(run["artifacts"]["lcov"]["bytes"], 0)
            refreshed = json.loads((output / "current.json").read_text())
            self.assertEqual(refreshed["summary"]["all"]["functions"], 1)
            self.assertFalse((ROOT / "qa/baselines/crap").exists())

    @unittest.skipUnless(os.name == "posix", "Linux CRAP runner execution is POSIX-specific")
    def test_requested_branch_coverage_fails_instead_of_soft_skipping(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake_bin = root / "bin"
            fake_bin.mkdir()

            cargo = fake_bin / "cargo"
            cargo.write_text(
                f"""#!/usr/bin/env sh
if [ "$1 $2" = "llvm-cov --version" ]; then
  echo "cargo-llvm-cov {PINS['KASKOLD_CARGO_LLVM_COV_VERSION']}"
  exit 0
fi
if [ "$1 $2" = "crap --version" ]; then
  echo "cargo-crap {PINS['KASKOLD_CARGO_CRAP_VERSION']}"
  exit 0
fi
exit 2
"""
            )
            cargo.chmod(0o755)

            rustup = fake_bin / "rustup"
            rustup.write_text(
                """#!/usr/bin/env sh
if [ "$1 $3" = "run rustc" ]; then
  echo "rustc 1.99.0-nightly"
  exit 0
fi
if [ "$1 $3" = "run cargo" ]; then
  shift 3
  exec cargo "$@"
fi
if [ "$1 $2" = "component list" ]; then
  exit 0
fi
exit 2
"""
            )
            rustup.chmod(0o755)

            node = fake_bin / "node"
            node.write_text("#!/usr/bin/env sh\nexit 0\n")
            node.chmod(0o755)

            output = root / "output"
            environment = dict(os.environ)
            environment["PATH"] = f"{fake_bin}:{environment.get('PATH', '')}"
            environment["CRAP_OUTPUT_DIR"] = str(output)
            environment["CRAP_ENABLE_BRANCH"] = "1"
            environment["CRAP_BRANCH_TOOLCHAIN"] = PINS["KASKOLD_BRANCH_RUST"]

            result = subprocess.run(
                [str(ROOT / "scripts/linux/quality/crap.sh")],
                cwd=ROOT,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )

            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn("requested branch coverage could not run", result.stderr)
            self.assertIn("branch-coverage-setup", result.stderr)
            self.assertNotIn("PASS: CRAP quality check", result.stdout)
            self.assertFalse((output / "run.json").exists())


    def test_branch_artifact_helper_validates_and_self_checks_zip(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            evidence = write_branch_evidence(root / "crap")
            validate = run_packager("--validate-only", "--input-dir", str(evidence))
            self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)
            self.assertIn("Requested: True", validate.stdout)
            self.assertIn("First BRF records:", validate.stdout)

            bundle = root / "branch.zip"
            package = run_packager("--input-dir", str(evidence), "--output", str(bundle))
            self.assertEqual(package.returncode, 0, package.stdout + package.stderr)
            self.assertIn("Validated", package.stdout)
            self.assertTrue(bundle.is_file())

    def test_branch_artifact_helper_refuses_incomplete_evidence(self) -> None:
        for name, damage in BRANCH_EVIDENCE_DAMAGE.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                evidence = write_branch_evidence(Path(directory) / "crap")
                damage(evidence)
                refused = run_packager(
                    "--input-dir", str(evidence), "--output", str(Path(directory) / "branch.zip")
                )
                self.assertNotEqual(refused.returncode, 0, refused.stdout + refused.stderr)

    def test_branch_artifact_archive_check_refuses_a_corrupt_member(self) -> None:
        spec = importlib.util.spec_from_file_location("kaskold_branch_packager_test", PACKAGER)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        marker = b"EVIDENCE-MARKER"
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "branch.zip"
            with zipfile.ZipFile(bundle, "w", zipfile.ZIP_STORED) as archive:
                for relative in module.REQUIRED_ARTIFACTS:
                    archive.writestr(f"crap/{relative}", marker)
            module.verify_archive(bundle)
            bundle.write_bytes(bundle.read_bytes().replace(marker, b"evidence-marker", 1))
            with self.assertRaises((Exception, SystemExit)):
                module.verify_archive(bundle)


    def test_quality_check_has_one_policy_and_one_compact_ratchet(self) -> None:
        policy_files = sorted(
            path.name for path in (ROOT / "qa/checks/quality/crap").glob("*.json")
        )
        contract_files = sorted(
            path.name for path in (ROOT / "qa/contracts/quality").glob("*.json")
        )
        self.assertEqual(policy_files, ["policy.json"])
        self.assertIn("crap_ratchets.json", contract_files)
        self.assertFalse((ROOT / "qa/baselines/crap").exists())


if __name__ == "__main__":
    unittest.main()
