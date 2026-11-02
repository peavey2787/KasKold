#!/usr/bin/env python3
"""Provision, run, summarize, persist, and gate critical mutation testing."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tomllib
import zipfile
from dataclasses import dataclass
from fnmatch import fnmatchcase
from pathlib import Path
from typing import Any

_SECURITY_DIR = Path(__file__).resolve().parent
if str(_SECURITY_DIR) not in sys.path:
    sys.path.insert(0, str(_SECURITY_DIR))

from mutation_reuse import inventory_sha256, load_mutant_inventory
from mutation_support import (
    POLICY,
    ROOT,
    RUN_SCOPE_FILE,
    archive_results,
    count_lines,
    load_policy,
    load_run_scope,
    locate_results,
    merge_outcome_documents,
    mutation_cache_action,
    mutation_config_sha256,
    mutation_scope_sha256,
    read_outcomes,
    setup,
    workspace_test_sha256,
    write_run_scope,
    write_triage,
)

# Names re-exported for callers that reach them through this module.
__all__ = [
    "RUN_SCOPE_FILE",
    "merge_outcome_documents",
    "mutation_cache_action",
    "write_run_scope",
]


def _selected_files(include_globs: list[str], exclude_globs: list[str]) -> set[str]:
    selected: set[str] = set()
    for pattern in include_globs:
        for path in ROOT.glob(pattern):
            if not path.is_file():
                continue
            relative = path.relative_to(ROOT).as_posix()
            if any(fnmatchcase(relative, excluded) for excluded in exclude_globs):
                continue
            selected.add(relative)
    return selected


def _global_mutation_files() -> set[str]:
    config = tomllib.loads((ROOT / ".cargo/mutants.toml").read_text())
    return _selected_files(
        list(config.get("examine_globs", [])),
        list(config.get("exclude_globs", [])),
    )


def _global_mutation_profile_errors() -> list[str]:
    """Reject stale cargo-mutants globs before an expensive mutation run starts."""
    config = _mutation_config()
    exclude_globs = list(config.get("exclude_globs", []))
    errors: list[str] = []
    examine_globs = list(config.get("examine_globs", []))
    if not examine_globs:
        errors.append("cargo-mutants profile has no examine_globs")
        return errors
    for pattern in examine_globs:
        if not _selected_files([pattern], exclude_globs):
            errors.append(f"cargo-mutants examine_glob selects no production files: {pattern}")
    return errors


def _mutation_config() -> dict[str, Any]:
    return tomllib.loads((ROOT / ".cargo/mutants.toml").read_text())


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _current_mutation_provenance(
    errors: list[str], run_scope: dict[str, Any] | None
) -> tuple[str | None, str | None, str | None, bool]:
    """Read current mutation provenance without losing completed run evidence.

    A mutation run can take many hours. If the checked-in cargo-mutants profile
    is removed or corrupted while the run is in progress, report that as a
    fail-closed integrity error instead of throwing away the completed outcomes
    with an uncaught FileNotFoundError.
    """
    profile_current = True
    try:
        source_digest = mutation_scope_sha256()
        config_digest = mutation_config_sha256()
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        profile_current = False
        errors.append(f"cargo-mutants profile cannot be verified after the run: {error}")
        source_digest = (
            run_scope.get("mutation_scope_sha256") if run_scope is not None else None
        )
        config_digest = (
            run_scope.get("mutation_config_sha256") if run_scope is not None else None
        )

    try:
        test_digest = workspace_test_sha256()
    except OSError as error:
        errors.append(f"mutation workspace test scope cannot be verified after the run: {error}")
        test_digest = run_scope.get("workspace_test_sha256") if run_scope is not None else None
        profile_current = False

    return source_digest, test_digest, config_digest, profile_current


def _mutant_record(outcome: dict[str, Any]) -> dict[str, Any] | None:
    scenario = outcome.get("scenario")
    if not isinstance(scenario, dict):
        return None
    mutant = scenario.get("Mutant")
    return mutant if isinstance(mutant, dict) else None

def _canonical_mutant_file(value: object) -> str:
    """Normalize cargo-mutants file paths to repository-relative POSIX form.

    cargo-mutants emits native path separators on some Windows versions while
    the authored mutation/security policy is intentionally platform-neutral.
    Keep the raw evidence untouched, but canonicalize paths before policy
    comparisons and report inventories.
    """
    if not isinstance(value, str) or not value:
        return ""
    text = value.replace("\\", "/")
    root = str(ROOT).replace("\\", "/").rstrip("/")
    compare_text = text.casefold() if os.name == "nt" else text
    compare_root = root.casefold() if os.name == "nt" else root
    prefix = compare_root + "/"
    if compare_text.startswith(prefix):
        text = text[len(root) + 1:]
    while text.startswith("./"):
        text = text[2:]
    return text


def _load_crypto_policy() -> dict[str, Any]:
    document = json.loads(POLICY.read_text())
    if document.get("schema_version") != 1:
        raise ValueError("unsupported security policy schema")
    policy = dict(document["crypto_mutation_domain"])
    if float(policy.get("minimum_score_percent", 0.0)) != 100.0:
        raise ValueError("cryptographic mutation domain must require exactly 100%")
    if int(policy.get("maximum_timeouts", -1)) != 0:
        raise ValueError("cryptographic mutation domain must require zero timeouts")
    return policy


_SUMMARY_KINDS = ("CaughtMutant", "MissedMutant", "Timeout", "Unviable")
_EQUIVALENT_FIELDS = (
    "name",
    "file",
    "function",
    "replacement",
    "source_sha256",
    "justification",
    "refactor_attempted",
)
_TEST_DIFFICULTY_PHRASES = ("hard to test", "difficult to test", "too hard to test")


@dataclass(frozen=True)
class _CryptoTally:
    caught: int
    viable_non_equivalent: int
    score: float
    remaining_missed: list[dict[str, Any]]
    timeouts: list[dict[str, Any]]


def _run_directory() -> Path:
    run_directory = Path(
        os.environ.get(
            "KASKOLD_SECURITY_RUN_DIR",
            str(ROOT / "target/qa/security/latest"),
        )
    )
    if not run_directory.is_absolute():
        run_directory = ROOT / run_directory
    return run_directory


def _document_text(document: dict[str, Any]) -> str:
    return json.dumps(document, indent=2, sort_keys=True) + "\n"


def _display_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def _load_inventory_digest(
    results: Path, run_scope: dict[str, Any] | None, errors: list[str]
) -> str | None:
    if run_scope is None or run_scope.get("schema_version") != 2:
        return None
    try:
        return inventory_sha256(load_mutant_inventory(results / "mutants.json"))
    except ValueError as error:
        errors.append(f"mutation evidence inventory is invalid: {error}")
        return None


def _candidate_certified(
    profile_current: bool,
    run_scope: dict[str, Any] | None,
    current_scope: str,
    current_tests: str,
    current_config: str,
    actual_inventory_digest: str | None,
) -> bool:
    return bool(
        profile_current
        and run_scope
        and run_scope.get("candidate_certified") is True
        and run_scope.get("evidence_mode") == "certification-fresh"
        and run_scope.get("mutation_scope_sha256") == current_scope
        and run_scope.get("workspace_test_sha256") == current_tests
        and run_scope.get("mutation_config_sha256") == current_config
        and actual_inventory_digest is not None
        and run_scope.get("mutant_inventory_sha256") == actual_inventory_digest
        and run_scope.get("carried_forward_caught") == 0
        and run_scope.get("carried_forward_unviable") == 0
    )


def _check_crypto_domain_scope(domain_files: set[str], errors: list[str]) -> None:
    if not domain_files:
        errors.append("cryptographic mutation domain selects no production files")
    missing_from_global = sorted(domain_files - _global_mutation_files())
    if missing_from_global:
        errors.append(
            "cryptographic mutation files are outside the cargo-mutants profile: "
            + ", ".join(missing_from_global)
        )


def _check_exclude_policy(policy: dict[str, Any], errors: list[str]) -> list[Any]:
    config = _mutation_config()
    configured_exclude_re = list(config.get("exclude_re", []))
    allowed_exclude_re = list(policy.get("allowed_global_exclude_re", []))
    if configured_exclude_re != allowed_exclude_re:
        errors.append(
            "cargo-mutants exclude_re changed outside the audited cryptographic policy: "
            f"configured={configured_exclude_re!r}, allowed={allowed_exclude_re!r}"
        )
    return configured_exclude_re


def _check_source_skips(domain_files: set[str], errors: list[str]) -> None:
    mutation_skip_files = sorted(
        relative
        for relative in domain_files
        if "mutants::skip" in (ROOT / relative).read_text(errors="replace")
    )
    if mutation_skip_files:
        errors.append(
            "cryptographic mutation domain contains source-level mutants::skip exclusions: "
            + ", ".join(mutation_skip_files)
        )


def _check_crypto_run_scope(
    run_scope: dict[str, Any] | None,
    current_scope: str,
    current_tests: str,
    current_config: str,
    actual_inventory_digest: str | None,
    errors: list[str],
) -> None:
    if run_scope is None:
        errors.append("mutation evidence has no immutable run provenance")
        return
    if run_scope.get("mutation_scope_sha256") != current_scope:
        errors.append("mutation evidence production scope is stale")
    if run_scope.get("workspace_test_sha256") != current_tests:
        errors.append("mutation evidence test scope is stale")
    schema_two = run_scope.get("schema_version") == 2
    if schema_two and run_scope.get("mutation_config_sha256") != current_config:
        errors.append("mutation evidence cargo-mutants configuration is stale")
    if (
        schema_two
        and actual_inventory_digest is not None
        and run_scope.get("mutant_inventory_sha256") != actual_inventory_digest
    ):
        errors.append("mutation evidence mutant inventory digest does not match mutants.json")


def _crypto_domain_outcomes(
    parsed: dict[str, Any] | None, domain_files: set[str]
) -> list[dict[str, Any]]:
    if parsed is None:
        return []
    domain_outcomes: list[dict[str, Any]] = []
    for outcome in parsed.get("outcomes", []):
        if not isinstance(outcome, dict):
            continue
        mutant = _mutant_record(outcome)
        if mutant is None or _canonical_mutant_file(mutant.get("file")) not in domain_files:
            continue
        domain_outcomes.append(outcome)
    return domain_outcomes


def _bucket_by_summary(outcomes: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    buckets: dict[str, list[dict[str, Any]]] = {kind: [] for kind in _SUMMARY_KINDS}
    for outcome in outcomes:
        summary = outcome.get("summary")
        if summary in buckets:
            buckets[summary].append(outcome)
    return buckets


def _missed_outcomes_by_name(missed: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    by_name: dict[str, dict[str, Any]] = {}
    for outcome in missed:
        name = (_mutant_record(outcome) or {}).get("name")
        if isinstance(name, str):
            by_name[name] = outcome
    return by_name


def _validated_equivalent_entry(entry: Any, errors: list[str]) -> dict[str, str] | None:
    if not isinstance(entry, dict):
        errors.append("equivalent-mutant policy entries must be objects")
        return None
    fields = {key: entry.get(key) for key in _EQUIVALENT_FIELDS}
    if not all(
        isinstance(fields[key], str) and fields[key].strip() for key in _EQUIVALENT_FIELDS
    ):
        errors.append(
            "equivalent-mutant entries require exact name, file, function, replacement, "
            "source_sha256, justification, and refactor_attempted"
        )
        return None
    name = str(fields["name"])
    source_sha256 = str(fields["source_sha256"])
    if len(source_sha256) != 64 or any(
        character not in "0123456789abcdef" for character in source_sha256.lower()
    ):
        errors.append(f"equivalent-mutant source_sha256 is invalid: {name}")
        return None
    combined_reason = f"{fields['justification']} {fields['refactor_attempted']}".lower()
    if any(phrase in combined_reason for phrase in _TEST_DIFFICULTY_PHRASES):
        errors.append(
            "equivalent-mutant entries may not use test difficulty as justification: " + name
        )
        return None
    return {key: str(fields[key]) for key in _EQUIVALENT_FIELDS}


def _equivalent_identity_mismatches(
    fields: dict[str, str],
    actual_file: str,
    actual_function: str,
    actual_replacement: str,
    actual_source_sha256: str,
) -> list[str]:
    mismatches: list[str] = []
    if fields["file"] != actual_file:
        mismatches.append(f"file {fields['file']!r} != {actual_file!r}")
    if fields["function"] != actual_function:
        mismatches.append(f"function {fields['function']!r} != {actual_function!r}")
    if fields["replacement"] != actual_replacement:
        mismatches.append(f"replacement {fields['replacement']!r} != {actual_replacement!r}")
    if fields["source_sha256"].lower() != actual_source_sha256:
        mismatches.append(
            f"source_sha256 {fields['source_sha256']!r} != {actual_source_sha256!r}"
        )
    return mismatches


def _verify_equivalent_identity(
    fields: dict[str, str], outcome: dict[str, Any], errors: list[str]
) -> dict[str, str] | None:
    mutant = _mutant_record(outcome) or {}
    function = mutant.get("function") or {}
    actual_file = str(mutant.get("file", ""))
    actual_function = str(function.get("function_name", ""))
    actual_replacement = str(mutant.get("replacement", ""))
    current_file = ROOT / actual_file
    actual_source_sha256 = _sha256_file(current_file) if current_file.is_file() else ""
    identity_errors = _equivalent_identity_mismatches(
        fields, actual_file, actual_function, actual_replacement, actual_source_sha256
    )
    if identity_errors:
        errors.append(
            "equivalent-mutant identity does not match current source/evidence for "
            f"{fields['name']}: " + "; ".join(identity_errors)
        )
        return None
    return {
        "name": fields["name"],
        "file": actual_file,
        "function": actual_function,
        "replacement": actual_replacement,
        "source_sha256": actual_source_sha256,
        "justification": fields["justification"].strip(),
        "refactor_attempted": fields["refactor_attempted"].strip(),
    }


def _approve_equivalent_mutants(
    policy: dict[str, Any],
    missed_by_name: dict[str, dict[str, Any]],
    errors: list[str],
) -> list[dict[str, str]]:
    approved: list[dict[str, str]] = []
    configured_names: set[str] = set()
    for entry in policy.get("equivalent_mutants", []):
        fields = _validated_equivalent_entry(entry, errors)
        if fields is None:
            continue
        name = fields["name"]
        if name in configured_names:
            errors.append(f"duplicate equivalent-mutant entry: {name}")
            continue
        configured_names.add(name)
        outcome = missed_by_name.get(name)
        if outcome is None:
            errors.append(
                "stale or non-missed equivalent-mutant entry does not match current evidence: "
                + name
            )
            continue
        approved_entry = _verify_equivalent_identity(fields, outcome, errors)
        if approved_entry is not None:
            approved.append(approved_entry)
    return approved


def _tally_crypto_outcomes(
    by_summary: dict[str, list[dict[str, Any]]], approved: list[dict[str, str]]
) -> _CryptoTally:
    approved_names = {entry["name"] for entry in approved}
    remaining_missed = [
        outcome
        for outcome in by_summary["MissedMutant"]
        if (_mutant_record(outcome) or {}).get("name") not in approved_names
    ]
    timeouts = by_summary["Timeout"]
    caught = len(by_summary["CaughtMutant"])
    viable_non_equivalent = caught + len(remaining_missed) + len(timeouts)
    score = 100.0 * caught / viable_non_equivalent if viable_non_equivalent else 0.0
    return _CryptoTally(caught, viable_non_equivalent, score, remaining_missed, timeouts)


def _check_crypto_score(
    parsed: dict[str, Any] | None, tally: _CryptoTally, errors: list[str]
) -> None:
    if parsed is None:
        errors.append("cryptographic mutation evidence is unavailable")
    if tally.viable_non_equivalent == 0:
        errors.append("cryptographic mutation domain produced no viable non-equivalent mutants")
    if tally.timeouts:
        errors.append(f"cryptographic mutation timeouts {len(tally.timeouts)} exceed 0")
    if tally.remaining_missed:
        errors.append(
            f"cryptographic mutation domain has {len(tally.remaining_missed)} viable non-equivalent missed mutants"
        )
    if tally.score < 100.0:
        errors.append(f"cryptographic mutation score {tally.score:.4f}% is below 100.0000%")


def _describe_outcome(outcome: dict[str, Any]) -> dict[str, Any]:
    mutant = _mutant_record(outcome) or {}
    function = mutant.get("function") or {}
    return {
        "name": mutant.get("name"),
        "file": _canonical_mutant_file(mutant.get("file")),
        "function": function.get("function_name"),
        "replacement": mutant.get("replacement"),
        "genre": mutant.get("genre"),
    }


def _mutated_domain_files(domain_outcomes: list[dict[str, Any]]) -> set[str]:
    return {
        canonical
        for outcome in domain_outcomes
        if (canonical := _canonical_mutant_file((_mutant_record(outcome) or {}).get("file")))
    }


def _crypto_document(
    *,
    policy: dict[str, Any],
    errors: list[str],
    domain_files: set[str],
    domain_outcomes: list[dict[str, Any]],
    by_summary: dict[str, list[dict[str, Any]]],
    approved: list[dict[str, str]],
    tally: _CryptoTally,
    configured_exclude_re: list[Any],
    scope_digest: str,
    test_digest: str,
    config_digest: str,
    inventory_digest: str | None,
    run_scope: dict[str, Any] | None,
    candidate_certified: bool,
) -> dict[str, Any]:
    mutated_files = _mutated_domain_files(domain_outcomes)
    return {
        "schema_version": 1,
        "healthy": not errors,
        "claim": (
            "100% of viable, non-equivalent mutants in the explicitly enumerated "
            "host-testable cryptographic/key/signing domain are caught, with zero timeouts."
        ),
        "scope_statement": policy.get("scope_statement"),
        "scope_limitations": list(policy.get("scope_limitations", [])),
        "minimum_score_percent": 100.0,
        "maximum_timeouts": 0,
        "score_percent": round(tally.score, 4),
        "score_formula": "caught / (caught + remaining_missed + timeout)",
        "domain_files": sorted(domain_files),
        "domain_file_sha256": {
            relative: _sha256_file(ROOT / relative) for relative in sorted(domain_files)
        },
        "domain_files_with_mutants": sorted(mutated_files),
        "domain_files_without_generated_mutants": sorted(domain_files - mutated_files),
        "cargo_mutants_exclude_re": configured_exclude_re,
        "counts": {
            "caught": tally.caught,
            "missed": len(by_summary["MissedMutant"]),
            "approved_equivalent": len(approved),
            "remaining_missed": len(tally.remaining_missed),
            "timeout": len(tally.timeouts),
            "unviable": len(by_summary["Unviable"]),
            "viable_non_equivalent": tally.viable_non_equivalent,
        },
        "approved_equivalent_mutants": approved,
        "remaining_missed_mutants": [_describe_outcome(o) for o in tally.remaining_missed],
        "timeout_mutants": [_describe_outcome(o) for o in tally.timeouts],
        "mutation_scope_sha256": scope_digest,
        "workspace_test_sha256": test_digest,
        "mutation_config_sha256": config_digest,
        "mutant_inventory_sha256": inventory_digest,
        "evidence_mode": run_scope.get("evidence_mode") if run_scope else None,
        "candidate_certified": candidate_certified,
        "evidence_run_scope": run_scope,
        "errors": errors,
    }


def _write_crypto_documents(document: dict[str, Any], artifact: Path) -> None:
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_text(_document_text(document))
    run_directory = _run_directory()
    run_directory.mkdir(parents=True, exist_ok=True)
    (run_directory / "crypto-mutation-summary.json").write_text(_document_text(document))


def summarize_crypto_domain(
    policy: dict[str, Any], output: Path, artifact: Path
) -> tuple[list[str], dict[str, Any]]:
    """Gate explicitly enumerated host-testable crypto/key/signing mutation evidence."""
    results = locate_results(output)
    parsed, errors = read_outcomes(results)
    domain_files = _selected_files(
        list(policy.get("include_globs", [])),
        list(policy.get("exclude_globs", [])),
    )
    _check_crypto_domain_scope(domain_files, errors)
    configured_exclude_re = _check_exclude_policy(policy, errors)
    _check_source_skips(domain_files, errors)
    run_scope = load_run_scope(results)
    current_scope, current_tests, current_config, profile_current = (
        _current_mutation_provenance(errors, run_scope)
    )
    actual_inventory_digest = _load_inventory_digest(results, run_scope, errors)
    _check_crypto_run_scope(
        run_scope, current_scope, current_tests, current_config, actual_inventory_digest, errors
    )
    candidate_certified = _candidate_certified(
        profile_current,
        run_scope,
        current_scope,
        current_tests,
        current_config,
        actual_inventory_digest,
    )
    domain_outcomes = _crypto_domain_outcomes(parsed, domain_files)
    by_summary = _bucket_by_summary(domain_outcomes)
    approved = _approve_equivalent_mutants(
        policy, _missed_outcomes_by_name(by_summary["MissedMutant"]), errors
    )
    tally = _tally_crypto_outcomes(by_summary, approved)
    _check_crypto_score(parsed, tally, errors)
    document = _crypto_document(
        policy=policy,
        errors=errors,
        domain_files=domain_files,
        domain_outcomes=domain_outcomes,
        by_summary=by_summary,
        approved=approved,
        tally=tally,
        configured_exclude_re=configured_exclude_re,
        scope_digest=current_scope,
        test_digest=current_tests,
        config_digest=current_config,
        inventory_digest=actual_inventory_digest,
        run_scope=run_scope,
        candidate_certified=candidate_certified,
    )
    _write_crypto_documents(document, artifact)
    return errors, document


def _parsed_summary_fields(
    parsed: dict[str, Any],
) -> tuple[dict[str, int], int, Any, Any, Any]:
    counts = {
        "caught": int(parsed.get("caught", 0)),
        "missed": int(parsed.get("missed", 0)),
        "timeout": int(parsed.get("timeout", 0)),
        "unviable": int(parsed.get("unviable", 0)),
    }
    total_mutants = int(parsed.get("total_mutants", sum(counts.values())))
    tool_version = parsed.get("cargo_mutants_version") or parsed.get("version")
    return (
        counts,
        total_mutants,
        tool_version,
        parsed.get("start_time"),
        parsed.get("end_time"),
    )


def _line_count_summary_fields(results: Path) -> tuple[dict[str, int], int, None, None, None]:
    counts = {
        name: count_lines(results / f"{name}.txt")
        for name in ("caught", "missed", "timeout", "unviable")
    }
    return counts, sum(counts.values()), None, None, None


def _summary_fields(
    parsed: dict[str, Any] | None, results: Path
) -> tuple[dict[str, int], int, Any, Any, Any]:
    if parsed is not None:
        return _parsed_summary_fields(parsed)
    return _line_count_summary_fields(results)


def _check_summary_inventory(
    run_scope: dict[str, Any] | None,
    actual_inventory_digest: str | None,
    config_digest: str,
    errors: list[str],
) -> None:
    if run_scope is None or run_scope.get("schema_version") != 2:
        return
    if (
        actual_inventory_digest is not None
        and run_scope.get("mutant_inventory_sha256") != actual_inventory_digest
    ):
        errors.append("mutation evidence mutant inventory digest does not match mutants.json")
    if run_scope.get("mutation_config_sha256") != config_digest:
        errors.append("mutation evidence cargo-mutants configuration is stale")


def _check_summary_thresholds(
    policy: dict[str, Any],
    counts: dict[str, int],
    tool_version: Any,
    errors: list[str],
) -> tuple[int, float]:
    viable = counts["caught"] + counts["missed"] + counts["timeout"]
    score = round((100.0 * counts["caught"] / viable), 4) if viable else 0.0
    if viable == 0:
        errors.append("mutation run produced no viable mutants")
    minimum = float(policy["minimum_score_percent"])
    if score < minimum:
        errors.append(f"mutation score {score:.2f}% is below {minimum:.2f}%")
    maximum_timeouts = int(policy["maximum_timeouts"])
    if counts["timeout"] > maximum_timeouts:
        errors.append(f"mutation timeouts {counts['timeout']} exceed {maximum_timeouts}")
    if tool_version != policy["cargo_mutants_version"]:
        errors.append(
            "cargo-mutants output version "
            f"{tool_version!r} does not match required {policy['cargo_mutants_version']!r}"
        )
    return viable, score


def _archive_raw_results(results: Path, raw_artifact: Path, errors: list[str]) -> str | None:
    try:
        return archive_results(results, raw_artifact)
    except (OSError, RuntimeError, zipfile.BadZipFile) as error:
        errors.append(f"mutation evidence could not be persisted: {error}")
        return None


def _write_summary_outputs(
    document: dict[str, Any], artifact: Path, raw_artifact: Path, parsed: dict[str, Any] | None
) -> None:
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_text(_document_text(document))
    latest_summary = raw_artifact.parent / "mutation-summary.json"
    latest_summary.write_text(_document_text(document))
    triage = artifact.with_name("mutation-triage.json")
    write_triage(parsed, triage)
    if triage.is_file():
        (raw_artifact.parent / "mutation-triage.json").write_bytes(triage.read_bytes())


def summarize(
    policy: dict[str, Any], output: Path, artifact: Path
) -> tuple[list[str], dict[str, Any]]:
    results = locate_results(output)
    parsed, errors = read_outcomes(results)
    run_scope = load_run_scope(results)
    scope_digest, test_digest, config_digest, profile_current = (
        _current_mutation_provenance(errors, run_scope)
    )
    actual_inventory_digest = _load_inventory_digest(results, run_scope, errors)
    _check_summary_inventory(run_scope, actual_inventory_digest, config_digest, errors)
    candidate_certified = _candidate_certified(
        profile_current,
        run_scope,
        scope_digest,
        test_digest,
        config_digest,
        actual_inventory_digest,
    )
    counts, total_mutants, tool_version, started_at, completed_at = _summary_fields(
        parsed, results
    )
    viable, score = _check_summary_thresholds(policy, counts, tool_version, errors)
    raw_artifact = _run_directory() / "mutation-results.zip"
    raw_digest = _archive_raw_results(results, raw_artifact, errors)
    document = {
        "schema_version": 2,
        "healthy": not errors,
        "tool": "cargo-mutants",
        "required_version": policy["cargo_mutants_version"],
        "reported_version": tool_version,
        "minimum_score_percent": float(policy["minimum_score_percent"]),
        "maximum_timeouts": int(policy["maximum_timeouts"]),
        "score_percent": score,
        "score_formula": "caught / (caught + missed + timeout)",
        "total_mutants": total_mutants,
        "viable_mutants": viable,
        "counts": counts,
        "started_at_utc": started_at,
        "completed_at_utc": completed_at,
        "errors": errors,
        "source_output": _display_path(results),
        "raw_artifact": _display_path(raw_artifact) if raw_digest else None,
        "raw_artifact_sha256": raw_digest,
        "mutation_scope_sha256": scope_digest,
        "workspace_test_sha256": test_digest,
        "mutation_config_sha256": config_digest,
        "mutant_inventory_sha256": actual_inventory_digest,
        "evidence_mode": run_scope.get("evidence_mode") if run_scope else None,
        "candidate_certified": candidate_certified,
        "evidence_run_scope": run_scope,
    }
    _write_summary_outputs(document, artifact, raw_artifact, parsed)
    return errors, document


from mutation_runner import execute as _execute_mutation_run


def execute(policy: dict[str, Any], *, install: bool, fresh: bool) -> int:
    """Facade preserving the public runner API for tooling tests and CLI callers."""
    return _execute_mutation_run(
        policy, install=install, fresh=fresh, summarize_fn=summarize
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("setup")
    run_parser = subcommands.add_parser("run")
    run_parser.add_argument("--no-install", action="store_true")
    run_parser.add_argument(
        "--fresh",
        action="store_true",
        help="discard development reuse and run every current mutant for candidate certification",
    )
    summary_parser = subcommands.add_parser("summarize")
    summary_parser.add_argument("--output", type=Path)
    crypto_parser = subcommands.add_parser("crypto-check")
    crypto_parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    try:
        policy = load_policy()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"ERROR: mutation policy cannot be read: {error}")
        return 2
    try:
        profile_errors = _global_mutation_profile_errors()
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        print(f"ERROR: cargo-mutants profile cannot be read: {error}")
        return 2
    if profile_errors:
        for error in profile_errors:
            print(f"ERROR: {error}")
        return 2
    if arguments.command == "setup":
        return setup(policy)
    if arguments.command == "run":
        return execute(policy, install=not arguments.no_install, fresh=arguments.fresh)
    if arguments.command == "crypto-check":
        try:
            crypto_policy = _load_crypto_policy()
        except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
            print(f"ERROR: cryptographic mutation policy cannot be read: {error}")
            return 2
        output = arguments.output or ROOT / policy["output_directory"]
        errors, document = summarize_crypto_domain(
            crypto_policy, output, ROOT / crypto_policy["artifact"]
        )
        print(
            f"Cryptographic mutation score: {document['score_percent']:.2f}% "
            f"({document['counts']['caught']} caught / "
            f"{document['counts']['viable_non_equivalent']} viable non-equivalent)"
        )
        for error in errors:
            print(f"ERROR: {error}")
        return 1 if errors else 0
    output = arguments.output or ROOT / policy["output_directory"]
    errors, document = summarize(policy, output, ROOT / policy["artifact"])
    print(json.dumps(document, indent=2, sort_keys=True))
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
