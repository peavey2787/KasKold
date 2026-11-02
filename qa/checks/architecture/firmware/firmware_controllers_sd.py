from __future__ import annotations
from pathlib import Path
import hashlib, re

from architecture.core.common import relative_posix

EXPECTED_SD_HANDLERS = frozenset({
    "handle_sd_delete_confirm",
    "handle_sd_file_list",
    "handle_sd_import_menu",
    "handle_sd_kpub_encrypt_ask",
    "handle_sd_kpub_file_list",
    "handle_sd_kpub_filename",
    "handle_sd_kspt_encrypt_ask",
    "handle_sd_kspt_encrypt_pass",
    "handle_sd_kspt_file_list",
    "handle_sd_kspt_filename",
    "handle_sd_ms_addr_encrypt_ask",
    "handle_sd_ms_addr_filename",
    "handle_sd_ms_desc_encrypt_ask",
    "handle_sd_ms_desc_filename",
    "handle_sd_overwrite_warning",
    "handle_sd_sig_filename",
    "handle_sd_xprv_export_passphrase",
    "handle_sd_xprv_filename",
    "handle_seed_backup_warning",
    "handle_seed_backup_filename",
    "handle_seed_backup_export_passphrase",
    "handle_wallet_backup_file_list",
    "handle_wallet_backup_import_passphrase",
    "handle_show_qr_mode_choice",
    "handle_show_qr_popup",
})

REQUIRED_SD_MODULES = (
    "common/context.rs",
    "common/filename.rs",
    "common/list_navigation.rs",
    "common/import_scan.rs",
    "common/encryption_prompt.rs",
    "common/overwrite.rs",
    "common/passphrase.rs",
    "common/shared.rs",
    "imports/file_browser.rs",
    "imports/payload_detection.rs",
    "imports/selected_file/mod.rs",
    "imports/selected_file/covenant_backup.rs",
    "imports/selected_file/private_key.rs",
    "imports/selected_file/xprv.rs",
    "imports/import_menu.rs",
    "imports/kspt_import.rs",
    "exports/kpub.rs",
    "exports/kspt_export/mod.rs",
    "exports/multisig.rs",
    "exports/qr.rs",
    "exports/signature.rs",
    "backup/mod.rs",
    "backup/seed.rs",
    "backup/import.rs",
    "backup/xprv/mod.rs",
    "backup/xprv/export.rs",
)
EXPECTED_SD_GROUPS = frozenset({"common", "imports", "exports", "backup"})
SHARED_HANDLER_EXCLUDES = frozenset({"context.rs", "shared.rs"})
SD_HANDLER_PATTERN = r"(?m)^pub\((?:super|crate)\) fn (handle_[A-Za-z0-9_]+)\("
FILENAME_HANDLERS = {
    "backup/xprv/export.rs": "handle_sd_xprv_filename",
    "exports/kpub.rs": "handle_sd_kpub_filename",
    "exports/kspt_export/filename.rs": "handle_sd_kspt_filename",
    "exports/multisig.rs": "handle_sd_ms_addr_filename",
}
LIST_HANDLER_MODULES = (
    "imports/file_browser.rs",
    "imports/kspt_import.rs",
    "exports/kpub.rs",
    "backup/import.rs",
)
DUPLICATED_LIST_TOKENS = ("let max_vis: usize = 4", "let mut tapped: Option<usize>")
SD_API_SIGNATURE_COUNT = 7
SD_API_SIGNATURE_DIGEST = "16c9a0da425f96313723f2f15824df34033a4968f5bea437db44e10995dbc0b7"
SD_SIGNATURE_PATTERN = re.compile(
    r"(?ms)^\s*(pub(?:\(crate\))?)\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(.*?)\{"
)


def check_sd_controllers(root: Path) -> list[str]:
    sd_facade = root / "apps/kaskold-hardware/src/runtime/interactions/sd/mod.rs"
    sd_root = root / "apps/kaskold-hardware/src/runtime/interactions/sd"
    production_files = list(sd_root.rglob("*.rs")) if sd_root.exists() else []
    facade_errors, facade_source = _facade_errors(sd_facade)
    errors = _missing_module_errors(root, sd_root)
    errors += _retired_path_errors(root, sd_root)
    errors += facade_errors
    errors += _group_and_root_errors(sd_root)
    errors += _line_limit_errors(root, production_files)
    errors += _handler_inventory_errors(production_files)
    errors += _shared_workflow_errors(sd_root, facade_source)
    errors += _filename_handler_errors(sd_root)
    errors += _list_navigation_errors(root, sd_root)
    errors += _api_signature_errors(sd_facade, sd_root)
    return errors


def _missing_module_errors(root: Path, sd_root: Path) -> list[str]:
    return [
        f"required firmware SD module is missing: {required.relative_to(root)}"
        for required in (sd_root / name for name in REQUIRED_SD_MODULES)
        if not required.exists()
    ]


def _retired_path_errors(root: Path, sd_root: Path) -> list[str]:
    retired_paths = (
        sd_root / "backup/seed_backup.rs",
        sd_root / "backup/seed_restore.rs",
        root / "apps/kaskold-hardware/src/ui/screens/storage/sd/warning.rs",
    )
    return [
        f"retired password-only wallet-backup tombstone must stay deleted: {retired.relative_to(root)}"
        for retired in retired_paths
        if retired.exists()
    ]


def _facade_errors(sd_facade: Path) -> tuple[list[str], str]:
    if not sd_facade.exists():
        return [], ""
    source = sd_facade.read_text(errors="ignore")
    errors: list[str] = []
    if len(source.splitlines()) > 140:
        errors.append("controllers/sd/mod.rs must remain a small stable façade")
    if "let mut needs_redraw" in source:
        errors.append("controllers/sd/mod.rs must delegate workflow implementations")
    return errors, source


def _group_and_root_errors(sd_root: Path) -> list[str]:
    errors: list[str] = []
    actual_groups = {path.name for path in sd_root.iterdir() if path.is_dir()} if sd_root.exists() else set()
    if actual_groups != EXPECTED_SD_GROUPS:
        errors.append(
            f"firmware SD groups changed: expected {sorted(EXPECTED_SD_GROUPS)}, "
            f"got {sorted(actual_groups)}"
        )
    direct_modules = sorted(path.name for path in sd_root.glob("*.rs")) if sd_root.exists() else []
    if direct_modules != ["mod.rs"]:
        errors.append(f"firmware SD root must contain only mod.rs, found: {direct_modules}")
    return errors


def _line_limit_errors(root: Path, production_files: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in production_files:
        line_count = len(path.read_text(errors="ignore").splitlines())
        if line_count > 500:
            errors.append(
                f"firmware SD module exceeds 500-line SRP limit: "
                f"{path.relative_to(root)} ({line_count} lines)"
            )
    return errors


def _handler_inventory_errors(production_files: list[Path]) -> list[str]:
    handler_source = "\n".join(
        path.read_text(errors="ignore") for path in production_files
        if path.name not in SHARED_HANDLER_EXCLUDES
    )
    handlers = set(re.findall(SD_HANDLER_PATTERN, handler_source))
    handlers.discard("handle_file_list_touch")
    if handlers == EXPECTED_SD_HANDLERS:
        return []
    return [
        "firmware SD state handler inventory changed: "
        f"expected {sorted(EXPECTED_SD_HANDLERS)}, got {sorted(handlers)}"
    ]


def _read_optional(path: Path) -> str:
    return path.read_text(errors="ignore") if path.exists() else ""


def _shared_workflow_errors(sd_root: Path, facade_source: str) -> list[str]:
    filename_source = _read_optional(sd_root / "common/filename.rs")
    list_navigation_source = _read_optional(sd_root / "common/list_navigation.rs")
    common_facade_source = (sd_root / "common/mod.rs").read_text(errors="ignore")
    errors: list[str] = []
    if "run_filename_workflow(" not in filename_source or "struct FilenameWorkflow" not in filename_source:
        errors.append("firmware SD filename screens must share the configurable filename workflow")
    if "navigate_file_list(" not in list_navigation_source or "enum FileListAction" not in list_navigation_source:
        errors.append("firmware SD file lists must share pure paging and hit-testing")
    if "run_sd_file_list_context" not in common_facade_source or "run_sd_list_context" not in common_facade_source:
        errors.append("firmware SD list navigation must be re-exported through common/mod.rs")
    if "list_navigation::{FileListControllerOutcome" in facade_source:
        errors.append("controllers/sd/mod.rs must consume list navigation through the common façade")
    return errors


def _filename_handler_errors(sd_root: Path) -> list[str]:
    errors: list[str] = []
    for relative, handler in FILENAME_HANDLERS.items():
        source = (sd_root / relative).read_text(errors="ignore")
        if handler in source and "run_filename_workflow(" not in source:
            errors.append(f"firmware SD filename handler bypasses shared workflow: {relative}::{handler}")
    multisig_source = (sd_root / "exports/multisig.rs").read_text(errors="ignore")
    if "handle_sd_ms_desc_filename" in multisig_source and multisig_source.count("run_filename_workflow(") < 2:
        errors.append("multisig address and descriptor filenames must share the common workflow")
    return errors


def _list_navigation_errors(root: Path, sd_root: Path) -> list[str]:
    list_handlers = tuple(sd_root / relative for relative in LIST_HANDLER_MODULES)
    errors: list[str] = []
    for path in list_handlers:
        source = path.read_text(errors="ignore")
        if "run_sd_file_list_context(" not in source and "run_sd_list_context(" not in source:
            errors.append(f"firmware SD list bypasses shared navigation: {path.relative_to(root)}")
    for token in DUPLICATED_LIST_TOKENS:
        offenders = [
            relative_posix(path, root) for path in list_handlers
            if token in path.read_text(errors="ignore")
        ]
        if offenders:
            errors.append(f"duplicated SD list navigation returned ({token}): {offenders}")
    return errors


def _api_signature_errors(sd_facade: Path, sd_root: Path) -> list[str]:
    api_source = "\n".join(
        path.read_text(errors="ignore")
        for path in (sd_facade, sd_root / "common/routing.rs", sd_root / "common/shared.rs")
        if path.exists()
    )
    signatures = sorted(
        " ".join(f"{visibility} fn {name} {tail}".split())
        for visibility, name, tail in SD_SIGNATURE_PATTERN.findall(api_source)
    )
    signature_hash = hashlib.sha256("\n".join(signatures).encode()).hexdigest()
    # Descriptor parsing stays private to the SD subtree; camera QR import
    # calls the shared-signer parser directly so there is still one parser owner.
    if len(signatures) == SD_API_SIGNATURE_COUNT and signature_hash == SD_API_SIGNATURE_DIGEST:
        return []
    return [
        f"firmware SD API changed: expected 7 signatures with the locked digest, got "
        f"{len(signatures)} signatures / {signature_hash}"
    ]
