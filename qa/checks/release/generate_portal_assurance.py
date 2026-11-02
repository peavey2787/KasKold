#!/usr/bin/env python3
"""Create release-bound evidence that Portal's gates passed on the locked commit."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Callable

_RELEASE_DIR = Path(__file__).resolve().parent
if str(_RELEASE_DIR) not in sys.path:
    sys.path.insert(0, str(_RELEASE_DIR))

from readiness.model import SHA256_RE  # noqa: E402
from readiness.portal_assurance import (  # noqa: E402
    PORTAL_REPOSITORY,
    REQUIRED_WORKFLOWS,
    locked_portal_commit,
)
from readiness.signatures import sign_detached  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
RunQuery = Callable[[str, str], list[dict[str, str]]]


def successful_runs(workflow: str, commit: str) -> list[dict[str, str]]:
    result = subprocess.run(
        [
            "gh", "run", "list", "--repo", PORTAL_REPOSITORY, "--workflow", workflow,
            "--commit", commit, "--status", "success",
            "--json", "conclusion,headSha,url", "--limit", "1",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise ValueError(f"cannot query {workflow} runs: {result.stderr.strip()}")
    return json.loads(result.stdout)


def run_evidence(workflow: str, commit: str, query: RunQuery) -> dict[str, str]:
    runs = query(workflow, commit)
    if not runs:
        raise ValueError(
            f"{PORTAL_REPOSITORY} has no successful {workflow} run on {commit}; "
            f"dispatch it with: gh workflow run {workflow} --repo {PORTAL_REPOSITORY} --ref main"
        )
    return {
        "workflow": workflow,
        "head_sha": runs[0]["headSha"],
        "conclusion": runs[0]["conclusion"],
        "url": runs[0]["url"],
    }


def create(arguments: argparse.Namespace, query: RunQuery = successful_runs) -> Path:
    for label, value in (("source", arguments.source_sha256), ("release artifact", arguments.release_artifact_sha256)):
        if not SHA256_RE.fullmatch(value):
            raise ValueError(f"{label} SHA-256 must be 64 lowercase hex characters")
    commit = locked_portal_commit(ROOT / "Cargo.lock")
    report = {
        "schema": 2,
        "status": "pass",
        "source_sha256": arguments.source_sha256,
        "release_artifact_sha256": arguments.release_artifact_sha256,
        "signer_key_id": arguments.signer_key_id,
        "portal_commit": commit,
        "runs": {gate: run_evidence(workflow, commit, query) for gate, workflow in REQUIRED_WORKFLOWS.items()},
    }
    evidence_dir = arguments.evidence_dir.resolve()
    evidence_dir.mkdir(parents=True, exist_ok=True)
    output = evidence_dir / "portal_assurance.json"
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    sign_detached(output, arguments.signing_key)
    return output


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--source-sha256", required=True)
    parser.add_argument("--release-artifact-sha256", required=True)
    parser.add_argument("--signer-key-id", required=True)
    parser.add_argument("--signing-key", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        output = create(arguments)
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(f"PASS: release-bound Portal assurance written to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
