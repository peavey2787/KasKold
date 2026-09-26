#!/usr/bin/env python3
"""Regression tests for scope-aligned CRAP report composition."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
MERGER = ROOT / "qa/checks/quality/crap/merge_reports.py"

spec = importlib.util.spec_from_file_location("kaskold_crap_merge", MERGER)
assert spec is not None and spec.loader is not None
merge_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(merge_module)


def report(file_name: str, *, with_diagnostics: bool) -> dict[str, object]:
    document: dict[str, object] = {
        "version": "0.4.1",
        "entries": [
            {
                "file": file_name,
                "function": "example",
                "line": 3,
                "cyclomatic": 2.0,
                "coverage": 100.0 if with_diagnostics else None,
                "crap": 2.0 if with_diagnostics else 6.0,
            }
        ],
    }
    if with_diagnostics:
        document["diagnostics"] = {
            "analyzed_files": 1,
            "lcov_files": 1,
            "matched_files": 1,
            "source_only": {"count": 0, "examples": []},
            "lcov_only": {"count": 0, "examples": []},
        }
    return document


class CrapScopeMergeTests(unittest.TestCase):
    def test_merge_preserves_explicit_repository_scope_for_secondary_workspaces(self) -> None:
        merged = merge_module.merge_reports(
            report("C:/repo/kaskold/crates/shared-signer/src/lib.rs", with_diagnostics=True),
            report("src/main.rs", with_diagnostics=False),
            report("src/lib.rs", with_diagnostics=True),
            report("src/lib.rs", with_diagnostics=True),
        )
        files = {entry["file"] for entry in merged["entries"]}
        self.assertIn("C:/repo/kaskold/crates/shared-signer/src/lib.rs", files)
        self.assertIn("apps/kaskold-hardware/src/main.rs", files)
        self.assertIn("apps/kaskold-companion-web/src/lib.rs", files)
        self.assertIn("apps/kaskold-vault-web/src/lib.rs", files)

    def test_merge_rejects_any_coverage_scope_mismatch(self) -> None:
        host = report("src/lib.rs", with_diagnostics=True)
        host["diagnostics"]["source_only"]["count"] = 1  # type: ignore[index]
        with self.assertRaisesRegex(ValueError, "do not match exactly"):
            merge_module.merge_reports(
                host,
                report("src/main.rs", with_diagnostics=False),
                report("src/lib.rs", with_diagnostics=True),
                report("src/lib.rs", with_diagnostics=True),
            )

    def test_cli_accepts_kaskold_companion_web_argument_names(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            host_json = root / "host.json"
            firmware_json = root / "firmware.json"
            companion_json = root / "companion.json"
            vault_json = root / "vault.json"
            output_json = root / "merged.json"
            host_human = root / "host.txt"
            firmware_human = root / "firmware.txt"
            companion_human = root / "companion.txt"
            vault_human = root / "vault.txt"
            output_human = root / "merged.txt"

            host_json.write_text(json.dumps(report("src/lib.rs", with_diagnostics=True)), encoding="utf-8")
            firmware_json.write_text(json.dumps(report("src/main.rs", with_diagnostics=False)), encoding="utf-8")
            companion_json.write_text(json.dumps(report("src/lib.rs", with_diagnostics=True)), encoding="utf-8")
            vault_json.write_text(json.dumps(report("src/lib.rs", with_diagnostics=True)), encoding="utf-8")
            host_human.write_text("host report\n", encoding="utf-8")
            firmware_human.write_text("firmware report\n", encoding="utf-8")
            companion_human.write_text("companion report\n", encoding="utf-8")
            vault_human.write_text("vault report\n", encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(MERGER),
                    "--host-json",
                    str(host_json),
                    "--firmware-json",
                    str(firmware_json),
                    "--kaskold-companion-web-json",
                    str(companion_json),
                    "--kaskold-vault-web-json",
                    str(vault_json),
                    "--output-json",
                    str(output_json),
                    "--host-human",
                    str(host_human),
                    "--firmware-human",
                    str(firmware_human),
                    "--kaskold-companion-web-human",
                    str(companion_human),
                    "--kaskold-vault-web-human",
                    str(vault_human),
                    "--output-human",
                    str(output_human),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            self.assertTrue(output_json.is_file())
            self.assertTrue(output_human.is_file())
            human = output_human.read_text(encoding="utf-8")
            self.assertIn("KasKold Companion Web Rust shell", human)
            self.assertIn("KasKold Vault Web Rust shell", human)

    def test_human_report_explains_firmware_coverage_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = []
            for name in ("host", "firmware", "companion", "vault"):
                path = root / f"{name}.txt"
                path.write_text(f"{name} report\n", encoding="utf-8")
                paths.append(path)
            output = root / "full.txt"
            merge_module._write_human(
                output,
                [
                    ("Root Cargo workspace (coverage-backed CRAP)", paths[0]),
                    ("KasKold Companion Web Rust shell (coverage-backed CRAP)", paths[2]),
                    ("KasKold Vault Web Rust shell (coverage-backed CRAP)", paths[3]),
                    ("Signer firmware (complexity-only CRAP)", paths[1]),
                ],
            )
            text = output.read_text(encoding="utf-8")
            self.assertIn("host LCOV cannot instrument Xtensa firmware", text)
            self.assertIn("coverage-backed CRAP", text)
            self.assertIn("complexity-only CRAP", text)


if __name__ == "__main__":
    unittest.main()
