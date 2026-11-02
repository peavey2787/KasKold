#!/usr/bin/env python3
"""Execute development-incremental or fresh candidate mutation runs."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from collections.abc import Callable
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from mutation_progress import MutationProgressReporter
from mutation_reuse import (
    carry_forward_document,
    copy_carried_result_files,
    inventory_sha256,
    load_mutant_inventory,
    plan_incremental_reuse,
    validate_mutant_inventory,
    write_iterate_skip_state,
)
from mutation_support import (
    ROOT,
    load_run_scope,
    locate_results,
    merge_outcome_documents,
    mutation_cache_action,
    mutation_config_sha256,
    mutation_scope_sha256,
    read_outcomes,
    restore_results,
    run,
    setup,
    workspace_test_sha256,
    write_outcome_lists,
    write_run_scope,
)


@contextmanager
def _mutation_scratch_environment(policy: dict[str, Any], *, fresh: bool):
    """Preflight mutation scratch capacity without changing the fast default path.

    cargo-mutants normally creates its copied workspace and build tree under the
    platform TEMP directory. Preserve that default because moving the scratch
    tree can materially change Windows filesystem/antivirus performance. An
    explicit KASKOLD_MUTATION_TEMP_ROOT override remains available for users who
    need a larger filesystem. Cargo incremental compilation is never modified.
    """
    configured = os.environ.get("KASKOLD_MUTATION_TEMP_ROOT")
    override_root: Path | None = None
    if configured:
        scratch_base = Path(configured)
        if not scratch_base.is_absolute():
            scratch_base = ROOT / scratch_base
        override_root = scratch_base / "kaskold-cargo-mutants"
        if fresh:
            shutil.rmtree(override_root, ignore_errors=True)
        override_root.mkdir(parents=True, exist_ok=True)
        scratch_filesystem = override_root
    else:
        # Do not rewrite TMP/TEMP/TMPDIR here. cargo-mutants' original system
        # temp behavior was substantially faster on the qualified Windows host.
        scratch_filesystem = Path(tempfile.gettempdir())

    minimum_gib = int(policy.get("minimum_scratch_free_gib", 8))
    free_bytes = shutil.disk_usage(scratch_filesystem).free
    minimum_bytes = minimum_gib * 1024**3
    if fresh and free_bytes < minimum_bytes:
        raise RuntimeError(
            "fresh mutation certification requires at least "
            f"{minimum_gib} GiB free in the mutation scratch filesystem; "
            f"only {free_bytes / 1024**3:.2f} GiB is available at {scratch_filesystem}. "
            "Free disk space or set KASKOLD_MUTATION_TEMP_ROOT to a larger drive."
        )

    keys = ("TMP", "TEMP", "TMPDIR")
    previous = {key: os.environ.get(key) for key in keys}
    if override_root is not None:
        for key in keys:
            os.environ[key] = str(override_root)
    try:
        if fresh:
            mode = (
                "explicit override"
                if override_root is not None
                else "cargo-mutants default system TEMP preserved"
            )
            print(
                "Fresh mutation scratch: "
                f"{scratch_filesystem} ({free_bytes / 1024**3:.2f} GiB free, "
                f"{mode}, Cargo incremental compilation enabled)"
            )
        yield scratch_filesystem
    finally:
        if override_root is not None:
            for key, value in previous.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value
            shutil.rmtree(override_root, ignore_errors=True)


def _mutant_record(outcome: dict[str, Any]) -> dict[str, Any] | None:
    scenario = outcome.get("scenario")
    if not isinstance(scenario, dict):
        return None
    mutant = scenario.get("Mutant")
    return mutant if isinstance(mutant, dict) else None


def _discover_current_mutants(policy: dict[str, Any]) -> tuple[list[dict[str, Any]] | None, str | None]:
    """Ask pinned cargo-mutants for the complete current inventory without executing tests."""
    command = [
        "rustup",
        "run",
        policy["toolchain"],
        "cargo",
        "mutants",
        "--workspace",
        "--test-workspace=true",
        "--list",
        "--json",
    ]
    print("+", " ".join(command), flush=True)
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        return None, f"cargo-mutants inventory discovery failed (exit {result.returncode}): {detail}"
    try:
        return validate_mutant_inventory(json.loads(result.stdout)), None
    except (json.JSONDecodeError, ValueError) as error:
        return None, f"cargo-mutants inventory discovery returned invalid JSON: {error}"


def _inventory_from_results(results: Path) -> tuple[list[dict[str, Any]] | None, str | None]:
    try:
        return load_mutant_inventory(results / "mutants.json"), None
    except ValueError as error:
        return None, str(error)


def _outcome_names(document: dict[str, Any]) -> set[str]:
    names: set[str] = set()
    for outcome in document.get("outcomes", []):
        if not isinstance(outcome, dict):
            continue
        mutant = _mutant_record(outcome)
        name = mutant.get("name") if mutant is not None else None
        if isinstance(name, str) and name:
            names.add(name)
    return names


def _inventory_names(inventory: list[dict[str, Any]]) -> set[str]:
    return {str(mutant["name"]) for mutant in inventory}


_INCREMENTAL_ACTIONS = frozenset({"iterate-source-changed", "iterate-tests-changed"})
_Summarizer = Callable[[dict[str, Any], Path, Path], tuple[list[str], dict[str, Any]]]


@dataclass
class _RunContext:
    """Mutable state shared by the phases of one mutation execution."""

    policy: dict[str, Any]
    fresh: bool
    summarize_fn: _Summarizer
    output_parent: Path
    artifact: Path
    use_iterate: bool
    current_scope: str
    current_test_scope: str
    current_config: str
    existing: Path
    prior_run_scope: dict[str, Any] | None
    has_existing_outcomes: bool
    cache_action: str
    evidence_mode: str
    prior_document: dict[str, Any] | None = None
    prior_backup: Path | None = None
    current_inventory: list[dict[str, Any]] | None = None
    reuse_plan: dict[str, Any] | None = None
    base_source_digest: str | None = None


def _format_checkpoint_counts(document: dict[str, Any]) -> str:
    return (
        f"{document.get('caught', 0)} caught, "
        f"{document.get('unviable', 0)} unviable, "
        f"{document.get('missed', 0)} missed, "
        f"{document.get('timeout', 0)} timeout"
    )


def _print_score(document: dict[str, Any]) -> None:
    print(
        f"Mutation score: {document['score_percent']:.2f}% "
        f"({document['counts']['caught']} caught / {document['viable_mutants']} viable)"
    )


def _print_errors(errors: list[str]) -> int:
    for error in errors:
        print(f"ERROR: {error}")
    return 1 if errors else 0


def _print_evidence_mode(document: dict[str, Any]) -> None:
    if document.get("candidate_certified"):
        print("Mutation evidence mode: fresh candidate certification")
    else:
        print(
            "Mutation evidence mode: development only; run `make qa` or `python3 qa/checks/security/mutation.py run --fresh` "
            "once the candidate is frozen"
        )


def _discard_prior_backup(ctx: _RunContext) -> None:
    if ctx.prior_backup is not None:
        shutil.rmtree(ctx.prior_backup.parent, ignore_errors=True)
        ctx.prior_backup = None


def _restore_checkpoint(output_parent: Path, *, fresh: bool, use_iterate: bool) -> None:
    if fresh:
        shutil.rmtree(output_parent, ignore_errors=True)
        return
    if not use_iterate or (locate_results(output_parent) / "outcomes.json").is_file():
        return
    if not restore_results(output_parent):
        return
    restored_document, restored_errors = read_outcomes(locate_results(output_parent))
    if restored_document is not None and not restored_errors:
        print(
            "Restored persisted mutation checkpoint: "
            f"{_format_checkpoint_counts(restored_document)}"
        )
    else:
        print("Restored persisted mutation checkpoint for development iteration")


def _cache_action_for(ctx: _RunContext) -> str:
    return mutation_cache_action(
        use_iterate=ctx.use_iterate,
        has_existing_outcomes=ctx.has_existing_outcomes,
        run_scope=ctx.prior_run_scope,
        current_scope=ctx.current_scope,
        current_test_scope=ctx.current_test_scope,
        reuse_unchanged=bool(ctx.policy.get("reuse_unchanged_results", True)),
    )


def _open_run_context(
    policy: dict[str, Any], *, fresh: bool, summarize_fn: _Summarizer
) -> _RunContext:
    output_parent = ROOT / policy["output_directory"]
    use_iterate = bool(policy.get("iterate", True)) and not fresh
    _restore_checkpoint(output_parent, fresh=fresh, use_iterate=use_iterate)
    existing = locate_results(output_parent)
    ctx = _RunContext(
        policy=policy,
        fresh=fresh,
        summarize_fn=summarize_fn,
        output_parent=output_parent,
        artifact=ROOT / policy["artifact"],
        use_iterate=use_iterate,
        current_scope=mutation_scope_sha256(),
        current_test_scope=workspace_test_sha256(),
        current_config=mutation_config_sha256(),
        existing=existing,
        prior_run_scope=load_run_scope(existing),
        has_existing_outcomes=(existing / "outcomes.json").is_file(),
        cache_action="",
        evidence_mode="certification-fresh" if fresh else "development-full",
    )
    ctx.cache_action = _cache_action_for(ctx)
    return ctx


def _recover_unprovenanced(ctx: _RunContext) -> None:
    print(
        "Local mutation output has no immutable run provenance; discarding that "
        "interrupted/untrusted output and attempting the persisted checkpoint"
    )
    shutil.rmtree(ctx.output_parent, ignore_errors=True)
    ctx.existing = locate_results(ctx.output_parent)
    ctx.has_existing_outcomes = False
    if not (ctx.use_iterate and restore_results(ctx.output_parent)):
        ctx.cache_action = "run"
        return
    ctx.existing = locate_results(ctx.output_parent)
    restored_document, restored_errors = read_outcomes(ctx.existing)
    restored_scope = load_run_scope(ctx.existing)
    if restored_document is None or restored_errors or restored_scope is None:
        ctx.cache_action = "run"
        return
    ctx.prior_run_scope = restored_scope
    ctx.has_existing_outcomes = True
    ctx.cache_action = _cache_action_for(ctx)
    print(
        "Recovered verified persisted mutation checkpoint after discarding "
        "unprovenanced local output: "
        f"{_format_checkpoint_counts(restored_document)}"
    )


def _upgrade_legacy_provenance(ctx: _RunContext) -> None:
    run_scope = ctx.prior_run_scope
    if run_scope is None or run_scope.get("schema_version") != 1:
        return
    inventory, inventory_error = _inventory_from_results(ctx.existing)
    if inventory is None:
        if inventory_error:
            print(f"WARNING: legacy mutation provenance could not be upgraded: {inventory_error}")
        return
    prior_document, prior_errors = read_outcomes(ctx.existing)
    version = ctx.policy["cargo_mutants_version"]
    if prior_document is not None and not prior_errors:
        version = str(
            prior_document.get("cargo_mutants_version")
            or prior_document.get("version")
            or version
        )
    write_run_scope(
        ctx.existing,
        source_digest=ctx.current_scope,
        test_digest=ctx.current_test_scope,
        tool_version=version,
        evidence_mode="development-reused-legacy",
        candidate_certified=False,
        config_digest=ctx.current_config,
        inventory_digest=inventory_sha256(inventory),
    )
    print("Upgraded exact-match legacy mutation provenance to schema 2 (development only)")


def _reuse_verified_evidence(ctx: _RunContext) -> int:
    print("Mutation source and workspace tests are unchanged; reusing cumulative evidence")
    _upgrade_legacy_provenance(ctx)
    errors, document = ctx.summarize_fn(ctx.policy, ctx.output_parent, ctx.artifact)
    _print_score(document)
    print(f"Persisted raw mutation evidence: {document['raw_artifact']}")
    return _print_errors(errors)


def _scope_permits_context_reuse(ctx: _RunContext) -> bool:
    run_scope = ctx.prior_run_scope
    if run_scope is None:
        return False
    if run_scope.get("schema_version") == 2:
        if run_scope.get("mutation_config_sha256") != ctx.current_config:
            print(
                "Mutation configuration changed; no old mutants will be skipped in this "
                "development run"
            )
            return False
        return True
    if ctx.cache_action == "iterate-source-changed":
        # A legacy global digest cannot distinguish source changes from config changes.
        print(
            "Legacy mutation provenance cannot prove the mutation configuration across a "
            "production-source change; running all current mutants once"
        )
        return False
    return True


def _context_proof(
    ctx: _RunContext,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]] | None:
    if not _scope_permits_context_reuse(ctx):
        return None
    previous_inventory, previous_error = _inventory_from_results(ctx.existing)
    if previous_inventory is None:
        print(
            "Mutation inventory is unavailable for context proof; running all current "
            f"mutants instead ({previous_error})"
        )
        return None
    current_inventory, discovery_error = _discover_current_mutants(ctx.policy)
    if current_inventory is None:
        print(
            "Mutation inventory discovery could not prove safe reuse; running all current "
            f"mutants instead ({discovery_error})"
        )
        return None
    return previous_inventory, current_inventory


def _plan_context_reuse(ctx: _RunContext, complete_prior: dict[str, Any]) -> bool:
    proof = _context_proof(ctx)
    if proof is None:
        return False
    previous_inventory, current_inventory = proof
    ctx.current_inventory = current_inventory
    reuse_plan = plan_incremental_reuse(complete_prior, previous_inventory, current_inventory)
    carry_caught_names = list(reuse_plan["carry_caught_names"])
    carry_unviable_names = list(reuse_plan["carry_unviable_names"])
    ctx.prior_document = carry_forward_document(
        complete_prior, carry_caught_names, carry_unviable_names
    )
    write_iterate_skip_state(ctx.existing, carry_caught_names, carry_unviable_names)
    ctx.reuse_plan = reuse_plan
    ctx.evidence_mode = "development-incremental"
    ctx.base_source_digest = (
        ctx.prior_run_scope.get("mutation_scope_sha256") if ctx.prior_run_scope else None
    )
    print(
        "Context-aware mutation reuse: "
        f"{reuse_plan['carried_forward_caught']} unchanged caught and "
        f"{reuse_plan['carried_forward_unviable']} unchanged unviable mutants carried; "
        f"{reuse_plan['rerun_mutants']} current mutants will run "
        f"({reuse_plan['new_mutants']} new, "
        f"{reuse_plan['changed_same_identity']} same-name/context-changed, "
        f"{reuse_plan['removed_mutants']} removed from the old inventory)"
    )
    return True


def _copy_prior_results(existing: Path) -> Path:
    target_qa = ROOT / "target/qa"
    target_qa.mkdir(parents=True, exist_ok=True)
    backup = Path(tempfile.mkdtemp(prefix="mutation-prior-", dir=target_qa)) / "mutants.out"
    shutil.copytree(existing, backup)
    return backup


def _restart_full_run(ctx: _RunContext) -> None:
    shutil.rmtree(ctx.output_parent, ignore_errors=True)
    ctx.existing = locate_results(ctx.output_parent)
    ctx.has_existing_outcomes = False
    _discard_prior_backup(ctx)


def _prepare_incremental(ctx: _RunContext) -> int | None:
    complete_prior, prior_errors = read_outcomes(ctx.existing)
    if prior_errors or complete_prior is None:
        for error in prior_errors or ["prior mutation evidence is invalid"]:
            print(f"ERROR: {error}")
        return 2
    ctx.prior_backup = _copy_prior_results(ctx.existing)
    if not _plan_context_reuse(ctx, complete_prior):
        _restart_full_run(ctx)
    return None


def _run_cargo_mutants(ctx: _RunContext) -> subprocess.CompletedProcess[str] | None:
    ctx.output_parent.mkdir(parents=True, exist_ok=True)
    command = [
        "rustup",
        "run",
        ctx.policy["toolchain"],
        "cargo",
        "mutants",
        "--workspace",
        "--test-workspace=true",
        "--output",
        str(ctx.output_parent),
    ]
    if ctx.prior_document is not None and (locate_results(ctx.output_parent) / "outcomes.json").is_file():
        command.append("--iterate")
    try:
        with _mutation_scratch_environment(ctx.policy, fresh=ctx.fresh), MutationProgressReporter(ctx.output_parent):
            return run(command, check=False)
    except RuntimeError as error:
        print(f"ERROR: {error}")
        return None


def _check_cargo_exit(ctx: _RunContext, returncode: int, results: Path) -> int | None:
    if returncode in (0, 2, 3) or results.is_dir():
        return None
    _discard_prior_backup(ctx)
    print(f"ERROR: cargo-mutants failed before producing evidence (exit {returncode})")
    return returncode or 1


def _merge_incremental_evidence(ctx: _RunContext, results: Path) -> int | None:
    current_document, current_errors = read_outcomes(results)
    if current_errors or current_document is None:
        _discard_prior_backup(ctx)
        for error in current_errors or ["incremental outcomes are missing"]:
            print(f"ERROR: {error}")
        return 2
    try:
        merged_document = merge_outcome_documents(ctx.prior_document, current_document)
    except ValueError as error:
        _discard_prior_backup(ctx)
        print(f"ERROR: incremental mutation evidence could not be merged: {error}")
        return 2
    assert ctx.current_inventory is not None
    missing = _inventory_names(ctx.current_inventory) - _outcome_names(merged_document)
    if missing:
        _discard_prior_backup(ctx)
        print(
            "ERROR: incremental mutation run did not produce outcomes for every current "
            f"mutant ({len(missing)} missing)"
        )
        return 2
    if ctx.prior_backup is not None:
        copy_carried_result_files(ctx.prior_backup, results, ctx.prior_document)
    (results / "outcomes.json").write_text(
        json.dumps(merged_document, indent=2, sort_keys=False) + "\n"
    )
    (results / "mutants.json").write_text(
        json.dumps(ctx.current_inventory, indent=2, sort_keys=False) + "\n"
    )
    write_outcome_lists(results, merged_document)
    print(
        "Merged context-proven development outcomes with the current run: "
        f"{current_document.get('total_mutants', 0)} executed, "
        f"{merged_document['total_mutants']} current cumulative"
    )
    return None


def _require_final_outcomes(ctx: _RunContext, results: Path) -> dict[str, Any] | None:
    final_document, final_errors = read_outcomes(results)
    if final_document is None or final_errors:
        _discard_prior_backup(ctx)
        for error in final_errors or ["mutation outcomes are unavailable"]:
            print(f"ERROR: {error}")
        return None
    return final_document


def _ensure_current_inventory(ctx: _RunContext, results: Path) -> int | None:
    if ctx.current_inventory is not None:
        return None
    ctx.current_inventory, inventory_error = _inventory_from_results(results)
    if ctx.current_inventory is None:
        _discard_prior_backup(ctx)
        print(f"ERROR: current mutation inventory is unavailable: {inventory_error}")
        return 2
    return None


def _ensure_inventory_evidenced(ctx: _RunContext, document: dict[str, Any]) -> int | None:
    missing = _inventory_names(ctx.current_inventory) - _outcome_names(document)
    if missing:
        _discard_prior_backup(ctx)
        print(
            "ERROR: mutation evidence is incomplete for the current inventory "
            f"({len(missing)} missing outcomes)"
        )
        return 2
    return None


def _write_run_provenance(ctx: _RunContext, results: Path, final_document: dict[str, Any]) -> None:
    final_version = (
        final_document.get("cargo_mutants_version")
        or final_document.get("version")
        or ctx.policy["cargo_mutants_version"]
    )
    carried = int(ctx.reuse_plan["carried_forward_caught"]) if ctx.reuse_plan else 0
    carried_unviable = int(ctx.reuse_plan["carried_forward_unviable"]) if ctx.reuse_plan else 0
    write_run_scope(
        results,
        source_digest=ctx.current_scope,
        test_digest=ctx.current_test_scope,
        tool_version=str(final_version),
        evidence_mode=ctx.evidence_mode,
        candidate_certified=ctx.fresh,
        config_digest=ctx.current_config,
        inventory_digest=inventory_sha256(ctx.current_inventory),
        carried_forward_caught=carried,
        carried_forward_unviable=carried_unviable,
        base_source_digest=ctx.base_source_digest,
    )


def _verify_and_record_provenance(ctx: _RunContext, results: Path) -> int | None:
    final_document = _require_final_outcomes(ctx, results)
    if final_document is None:
        return 2
    failure = _ensure_current_inventory(ctx, results)
    if failure is not None:
        return failure
    failure = _ensure_inventory_evidenced(ctx, final_document)
    if failure is not None:
        return failure
    _write_run_provenance(ctx, results, final_document)
    return None


def _report_result(ctx: _RunContext) -> int:
    errors, document = ctx.summarize_fn(ctx.policy, ctx.output_parent, ctx.artifact)
    _print_score(document)
    _print_evidence_mode(document)
    print(f"Persisted raw mutation evidence: {document['raw_artifact']}")
    return _print_errors(errors)


def _finish_run(ctx: _RunContext, returncode: int) -> int:
    results = locate_results(ctx.output_parent)
    failure = _check_cargo_exit(ctx, returncode, results)
    if failure is None and ctx.prior_document is not None:
        failure = _merge_incremental_evidence(ctx, results)
    if failure is None:
        failure = _verify_and_record_provenance(ctx, results)
    if failure is not None:
        return failure
    _discard_prior_backup(ctx)
    return _report_result(ctx)


def execute(
    policy: dict[str, Any],
    *,
    install: bool,
    fresh: bool,
    summarize_fn: _Summarizer,
) -> int:
    if install and setup(policy) != 0:
        return 2
    ctx = _open_run_context(policy, fresh=fresh, summarize_fn=summarize_fn)
    if ctx.cache_action == "fresh-unprovenanced":
        _recover_unprovenanced(ctx)
    if ctx.cache_action == "reuse":
        return _reuse_verified_evidence(ctx)
    if ctx.cache_action in _INCREMENTAL_ACTIONS and ctx.has_existing_outcomes:
        failure = _prepare_incremental(ctx)
        if failure is not None:
            return failure
    completed = _run_cargo_mutants(ctx)
    if completed is None:
        return 2
    return _finish_run(ctx, completed.returncode)
