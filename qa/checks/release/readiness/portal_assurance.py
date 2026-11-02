"""Release-bound verification of Kaspa Portal's mutation and fuzz gates.

The signing core lives in Portal, so a KasKold release is only as assured as the
Portal commit its lockfile names. Both gates must have passed on that exact commit.
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Any

PORTAL_REPOSITORY = "peavey2787/kaspa-portal"
REQUIRED_WORKFLOWS = {"mutation": "mutation.yml", "fuzz": "fuzz.yml"}
RUN_URL_PREFIX = f"https://github.com/{PORTAL_REPOSITORY}/actions/runs/"
_LOCK_ENTRY = re.compile(
    r'name = "kaspa-portal"\r?\nversion = "[^"]+"\r?\n'
    rf'source = "git\+https://github\.com/{PORTAL_REPOSITORY}\.git[^#"]*#([0-9a-f]{{40}})"'
)


def locked_portal_commit(lockfile: Path) -> str:
    match = _LOCK_ENTRY.search(lockfile.read_text())
    if match is None:
        raise ValueError(f"{lockfile} does not lock kaspa-portal to a git commit")
    return match.group(1)


def _run_errors(gate: str, run: object, commit: str) -> list[str]:
    label = f"portal_assurance.json: {gate}"
    if not isinstance(run, dict):
        return [f"{label} must describe a workflow run"]
    errors: list[str] = []
    if run.get("workflow") != REQUIRED_WORKFLOWS[gate]:
        errors.append(f"{label}: workflow must be {REQUIRED_WORKFLOWS[gate]}")
    if run.get("head_sha") != commit:
        errors.append(f"{label}: head_sha must be the locked Portal commit {commit}")
    if run.get("conclusion") != "success":
        errors.append(f"{label}: conclusion must be success")
    url = run.get("url")
    if not isinstance(url, str) or not url.startswith(RUN_URL_PREFIX):
        errors.append(f"{label}: url must name a {PORTAL_REPOSITORY} workflow run")
    return errors


def verify(document: dict[str, Any], lockfile: Path) -> list[str]:
    try:
        commit = locked_portal_commit(lockfile)
    except (OSError, ValueError) as exc:
        return [f"portal_assurance.json: {exc}"]
    errors: list[str] = []
    if document.get("portal_commit") != commit:
        errors.append(f"portal_assurance.json: portal_commit must be the locked Portal commit {commit}")
    runs = document.get("runs")
    if not isinstance(runs, dict):
        return errors + ["portal_assurance.json: runs must be an object"]
    for gate in REQUIRED_WORKFLOWS:
        errors.extend(_run_errors(gate, runs.get(gate), commit))
    return errors
