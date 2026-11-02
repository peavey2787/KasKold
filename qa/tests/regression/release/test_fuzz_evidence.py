#!/usr/bin/env python3
"""Unified fuzz evidence contracts."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

ROOT = Path(__file__).resolve().parents[4]
FIRMWARE_CHECKS = ROOT / "qa" / "checks" / "firmware"
SECURITY_CHECKS = ROOT / "qa" / "checks" / "security"
for path in (FIRMWARE_CHECKS, SECURITY_CHECKS):
    if str(path) not in sys.path:
        sys.path.insert(0, str(path))


FUZZ_SPEC = importlib.util.spec_from_file_location(
    "fuzz_results_contract", SECURITY_CHECKS / "fuzz_results.py"
)
assert FUZZ_SPEC and FUZZ_SPEC.loader
FUZZ_RESULTS = importlib.util.module_from_spec(FUZZ_SPEC)
FUZZ_SPEC.loader.exec_module(FUZZ_RESULTS)


class FuzzEvidenceTests(unittest.TestCase):
    def _run_fuzz_writer(self, root: Path, budget_flag: str, value: str) -> dict[str, object]:
        state = root / "target/qa/fuzz"
        logs = state
        artifacts = state / "artifacts"
        corpus = state / "corpus"
        logs.mkdir(parents=True)
        artifacts.mkdir()
        corpus.mkdir()
        (logs / "target-a.log").write_bytes(b"fuzz log\n")
        statuses = state / "statuses.tsv"
        statuses.write_text("target-a\t0\n", encoding="ascii")
        summary = root / "target/qa/security/fuzz-summary.json"
        latest = root / "target/qa/security/latest"
        with mock.patch.multiple(
            FUZZ_RESULTS,
            ROOT=root,
            SUMMARY=summary,
            LATEST=latest,
            RAW_ZIP=latest / "fuzz-results.zip",
            LOG_ROOT=logs,
            CRASH_ROOT=artifacts,
            CORPUS_ROOT=corpus,
        ), mock.patch.object(
            sys,
            "argv",
            [
                "fuzz_results.py",
                "--statuses", str(statuses),
                "--tool", "cargo-fuzz 0.13.1",
                "--started", "2026-08-28T00:00:00Z",
                "--completed", "2026-08-28T00:01:00Z",
                budget_flag, value,
            ],
        ):
            self.assertEqual(FUZZ_RESULTS.main(), 0)
        document = json.loads(summary.read_text(encoding="utf-8"))
        self.assertTrue((latest / "fuzz-results.zip.sha256").is_file())
        with zipfile.ZipFile(latest / "fuzz-results.zip") as archive:
            self.assertIn("fuzz/fuzz-summary.json", archive.namelist())
            self.assertIn("fuzz/logs/target-a.log", archive.namelist())
        return document

    def test_fuzz_writer_archives_files_with_pre_1980_mtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = root / "target/qa/fuzz"
            artifacts = state / "artifacts"
            corpus = state / "corpus"
            state.mkdir(parents=True)
            artifacts.mkdir()
            corpus.mkdir()
            log = state / "target-a.log"
            log.write_bytes(b"fuzz log\n")
            os.utime(log, (1, 1))
            summary = root / "target/qa/security/fuzz-summary.json"
            summary.parent.mkdir(parents=True)
            summary.write_text("{}\n", encoding="utf-8")
            latest = root / "target/qa/security/latest"
            with mock.patch.multiple(
                FUZZ_RESULTS,
                ROOT=root,
                SUMMARY=summary,
                LATEST=latest,
                RAW_ZIP=latest / "fuzz-results.zip",
                LOG_ROOT=state,
                CRASH_ROOT=artifacts,
                CORPUS_ROOT=corpus,
            ):
                digest = FUZZ_RESULTS.archive(summary)
            self.assertEqual(len(digest), 64)
            with zipfile.ZipFile(latest / "fuzz-results.zip") as archive:
                member = archive.getinfo("fuzz/logs/target-a.log")
                self.assertEqual(member.date_time, FUZZ_RESULTS.ZIP_TIMESTAMP)
                self.assertEqual(archive.read(member), b"fuzz log\n")

    def test_fuzz_writer_schema_is_shared_for_run_count_and_time_budget(self) -> None:
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            runs = self._run_fuzz_writer(Path(first), "--runs", "100000")
            seconds = self._run_fuzz_writer(Path(second), "--seconds", "300")
        self.assertEqual(runs["schema_version"], seconds["schema_version"])
        self.assertEqual(set(runs), set(seconds))
        self.assertEqual(runs["execution_budget"], {"mode": "runs", "value": 100000})
        self.assertEqual(seconds["execution_budget"], {"mode": "seconds", "value": 300})
        self.assertEqual(runs["runs_per_target"], 100000)
        self.assertIsNone(runs["seconds_per_target"])
        self.assertEqual(seconds["seconds_per_target"], 300)
        self.assertIsNone(seconds["runs_per_target"])


    def test_sigterm_is_converted_into_reportable_interruption(self) -> None:
        checks = ROOT / "qa/checks/firmware"
        sys.path.insert(0, str(checks))
        try:
            import hil_evidence
            with mock.patch.object(hil_evidence.signal, "getsignal", return_value="previous"), \
                 mock.patch.object(hil_evidence.signal, "signal") as install:
                with hil_evidence.reportable_interruptions():
                    handler = install.call_args_list[0].args[1]
                    with self.assertRaises(KeyboardInterrupt):
                        handler(hil_evidence.signal.SIGTERM, None)
                self.assertEqual(install.call_args_list[-1].args, (hil_evidence.signal.SIGTERM, "previous"))
        finally:
            sys.path.pop(0)

        hardware = (ROOT / "qa/checks/firmware/run_hardware_tests.py").read_text(encoding="utf-8")
        workflow = (ROOT / "qa/checks/firmware/run_workflow_tests.py").read_text(encoding="utf-8")
        self.assertIn("@reportable_interruptions()\ndef main() -> int:", hardware)
        self.assertIn("@reportable_interruptions()\ndef main() -> int:", workflow)


if __name__ == "__main__":
    unittest.main()
