#!/usr/bin/env python3
"""Enforce operator acknowledgement and on-device consent around irreversible actions."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "target/qa/security/irreversible-action-policy.json"
ACK_WRAPPER = "qa/checks/release/irreversible_action_ack.py"

SCRIPT_ROOTS = (
    ROOT / "qa/linux",
    ROOT / "qa/windows",
    ROOT / "scripts",
    ROOT / "tools",
)
SCRIPT_SUFFIXES = {".sh", ".ps1", ".cmd", ".bat", ".py"}
MARKDOWN_ROOTS = (ROOT / "docs", ROOT / "qa/release")
PRODUCTION_SOURCE_ROOTS = (ROOT / "apps", ROOT / "crates", ROOT / "scripts", ROOT / "tools")
SOURCE_SUFFIXES = {".rs", ".py", ".c", ".h", ".cpp", ".cc"}
BOOTLOADER_PATCHER = ROOT / "tools/build/firmware/secure_bootloader/m5stack/patch_pop_it_bootloader.py"
BOOTLOADER_SECURITY_SOURCES = {
    ROOT / "tools/build/firmware/secure_bootloader/m5stack/owner_bootloader_patch.py",
    ROOT / "tools/build/firmware/secure_bootloader/m5stack/owner_secure_boot_patch.py",
}
FENCED_CODE = re.compile(r"```[^\n]*\n(.*?)```", re.DOTALL)
IRREVERSIBLE_COMMAND = re.compile(
    r"\b(?:burn[_-]efuse|burn[_-]key(?:[_-]digest)?|write[_-]protect[_-]efuse|"
    r"burn[_-]block|burn[_-]bit|burn[_-]custom[_-]mac)\b",
    re.IGNORECASE,
)
IRREVERSIBLE_API = re.compile(
    r"\b(?:esp_secure_boot_v2_permanently_enable|"
    r"esp_efuse_(?:write\w*|batch_write_commit|set_write_protect\w*))\b"
)
USER_CONSENT_LABEL = "on-device irreversible consent contract"
OWNER_CONSENT_LABEL = "owner-authority irreversible consent contract"
WRAPPER_LABEL = "developer irreversible acknowledgement contract"


def executable_scripts() -> list[Path]:
    paths: set[Path] = set()
    for root in SCRIPT_ROOTS:
        if not root.is_dir():
            continue
        for path in root.rglob("*"):
            if path.is_file() and path.suffix.lower() in SCRIPT_SUFFIXES:
                paths.add(path)
    return sorted(paths)


def live_command_lines(path: Path) -> list[str]:
    lines: list[str] = []
    for raw in path.read_text(errors="replace").splitlines():
        stripped = raw.strip()
        if not stripped or stripped.startswith(("#", "//", "*")):
            continue
        if IRREVERSIBLE_COMMAND.search(stripped):
            lines.append(stripped)
    return lines


def script_commands_are_guarded(path: Path) -> bool:
    lines = path.read_text(errors="replace").splitlines()
    matched = []
    for index, raw in enumerate(lines):
        stripped = raw.strip()
        if stripped and not stripped.startswith(("#", "//", "*")) and IRREVERSIBLE_COMMAND.search(stripped):
            matched.append(index)
    for index in matched:
        start = max(0, index - 8)
        end = min(len(lines), index + 9)
        context = "\n".join(lines[start:end])
        if ACK_WRAPPER not in context or "{device}" not in context:
            return False
    return True


def unauthorized_irreversible_source_calls() -> list[str]:
    violations: list[str] = []
    for root in PRODUCTION_SOURCE_ROOTS:
        if not root.is_dir():
            continue
        for path in root.rglob("*"):
            if not path.is_file() or path.suffix.lower() not in SOURCE_SUFFIXES:
                continue
            if path == BOOTLOADER_PATCHER or path in BOOTLOADER_SECURITY_SOURCES:
                continue
            for number, raw in enumerate(path.read_text(errors="replace").splitlines(), start=1):
                if IRREVERSIBLE_API.search(raw):
                    relative = path.relative_to(ROOT).as_posix()
                    violations.append(f"{relative}:{number}: {raw.strip()}")
    return violations


def unguarded_markdown_blocks(path: Path) -> list[str]:
    failures: list[str] = []
    for block in FENCED_CODE.findall(path.read_text(errors="replace")):
        if not IRREVERSIBLE_COMMAND.search(block):
            continue
        if ACK_WRAPPER not in block or "{device}" not in block:
            first = next(
                (line.strip() for line in block.splitlines() if IRREVERSIBLE_COMMAND.search(line)),
                "irreversible command",
            )
            failures.append(first)
    return failures


def _script_phase(errors: list[str]) -> tuple[list[dict[str, object]], list[str]]:
    destructive_scripts: list[dict[str, object]] = []
    guarded_scripts: list[str] = []
    for path in executable_scripts():
        commands = live_command_lines(path)
        if not commands:
            continue
        relative = path.relative_to(ROOT).as_posix()
        guarded = script_commands_are_guarded(path)
        destructive_scripts.append(
            {"path": relative, "guarded": guarded, "matched_lines": commands}
        )
        if guarded:
            guarded_scripts.append(relative)
        else:
            errors.append(
                f"{relative} contains an irreversible eFuse/security command without "
                f"the mandatory interactive wrapper {ACK_WRAPPER} and bound {{device}} token"
            )
    return destructive_scripts, guarded_scripts


def _markdown_files() -> list[Path]:
    paths: list[Path] = []
    for root in MARKDOWN_ROOTS:
        if root.is_dir():
            paths.extend(sorted(root.rglob("*.md")))
    return paths


def _markdown_phase(errors: list[str]) -> list[str]:
    guarded_markdown: list[str] = []
    for path in _markdown_files():
        failures = unguarded_markdown_blocks(path)
        relative = path.relative_to(ROOT).as_posix()
        if failures:
            for command in failures:
                errors.append(
                    f"{relative} contains a copy-pasteable irreversible command without "
                    f"{ACK_WRAPPER} and the bound {{device}} token: {command}"
                )
        elif any(IRREVERSIBLE_COMMAND.search(block) for block in FENCED_CODE.findall(path.read_text(errors="replace"))):
            guarded_markdown.append(relative)
    return guarded_markdown


def _source_phase(errors: list[str]) -> list[str]:
    source_violations = unauthorized_irreversible_source_calls()
    for violation in source_violations:
        errors.append(
            "irreversible production eFuse API exists outside the single Pop It-gated "
            f"bootloader patcher: {violation}"
        )
    return source_violations


@dataclass(frozen=True)
class _Sources:
    ui: str
    controller: str
    secure_boot_source: str
    owner_boot_source: str
    owner_ui: str
    owner_controller: str
    boot_control: str
    wrapper: str


def _read_source(relative: str) -> str:
    return (ROOT / relative).read_text()


def _load_sources() -> _Sources:
    return _Sources(
        ui=_read_source("apps/kaskold-hardware/src/ui/screens/device/pop_it.rs"),
        controller=_read_source("apps/kaskold-hardware/src/runtime/interactions/settings/advanced/pop_it.rs"),
        secure_boot_source=_read_source("tools/build/firmware/secure_bootloader/m5stack/owner_secure_boot_patch.py"),
        owner_boot_source=_read_source("tools/build/firmware/secure_bootloader/m5stack/owner_bootloader_patch.py"),
        owner_ui=_read_source("apps/kaskold-hardware/src/ui/screens/device/owner_firmware.rs"),
        owner_controller=_read_source("apps/kaskold-hardware/src/runtime/interactions/settings/advanced/owner_firmware.rs"),
        boot_control=_read_source("apps/kaskold-hardware/src/services/persistent_wallet/device/boot_control.rs"),
        wrapper=_read_source(ACK_WRAPPER),
    )


def _user_consent_checks(sources: _Sources) -> dict[str, bool]:
    return {
        "warns_permanent": "This permanently burns security eFuses." in sources.ui,
        "warns_cannot_undo": "It cannot be undone or reset later." in sources.ui,
        "requires_typed_pop_it": "FINAL: TYPE POP IT" in sources.ui
        and "confirmation_phrase_valid" in sources.controller,
        "arms_one_shot_request": "persistence.request_pop_it()" in sources.controller,
    }


def _pop_it_blocks(source: str) -> tuple[str, str]:
    try:
        armed = source.split("POP_IT_GATED_BLOCK = r'''", 1)[1].split("'''", 1)[0]
        commit = source.split("POP_IT_COMMIT_BLOCK = r'''", 1)[1].split("'''", 1)[0]
    except IndexError:
        return "", ""
    return armed, commit


def _bootloader_gated(source: str) -> bool:
    armed, commit = _pop_it_blocks(source)
    take = commit.find("kaskold_bootctl_take(KASKOLD_BOOTCTL_OP_POP_IT")
    burn = commit.find("esp_secure_boot_v2_permanently_enable(image_data)")
    return (
        "kaskold_pop_it_transition_armed = true" in armed
        and take >= 0 and burn > take
        and "esp_secure_boot_v2_permanently_enable" not in armed
    )


def _bootloader_phase(errors: list[str], secure_boot_source: str) -> bool:
    gated = _bootloader_gated(secure_boot_source)
    if not gated:
        errors.append(
            "bootloader irreversible Secure Boot enable call is not ordered after the "
            "consumed Pop It one-shot request"
        )
    return gated


def _owner_key_checks(sources: _Sources) -> dict[str, bool]:
    return {
        "warns_permanent_owner_key": "permanent" in sources.owner_ui and "cannot be undone" in sources.owner_ui,
        "requires_typed_owner_enroll": "ENROLL OWNER" in sources.owner_ui and 'b"ENROLL OWNER"' in sources.owner_controller,
        "requires_typed_owner_install": "INSTALL OWNER" in sources.owner_ui and 'b"INSTALL OWNER"' in sources.owner_controller,
        "secure_profile_only_boot_control": '#[cfg(feature = "secure-provisioning-core")]' in sources.boot_control,
        "development_simulation_only": sources.owner_controller.count("DEVELOPMENT SIMULATION") >= 2,
    }


def _owner_bootloader_checks(sources: _Sources) -> dict[str, bool]:
    return {
        "owner_efuse_bootloader_owned": "esp_efuse_write_key" in sources.owner_boot_source
        and "esp_efuse_set_write_protect_of_digest_revoke" in sources.owner_boot_source,
        "pop_it_enrollment_prerequisite": "Do this BEFORE Pop It" in sources.owner_ui,
        "owner_only_requires_enrollment": 'cfg!(feature = "secure-owner-only")' in sources.owner_controller
        or 'cfg!(feature = "secure-owner-only")' in sources.controller,
        "owner_only_sole_authority_policy": "KASKOLD_OWNER_ONLY_AUTHORITY" in sources.owner_boot_source
        and "esp_efuse_set_digest_revoke(1)" in sources.owner_boot_source
        and "esp_efuse_set_digest_revoke(2)" in sources.owner_boot_source,
    }


def _wrapper_checks(wrapper: str) -> dict[str, bool]:
    return {
        "explicit_irreversible_phrase": "I UNDERSTAND THIS IS IRREVERSIBLE" in wrapper,
        "interactive_only": "isatty()" in wrapper,
        "device_retype": "Retype the target device exactly" in wrapper,
        "command_device_binding": 'DEVICE_TOKEN = "{device}"' in wrapper
        and "bound_command" in wrapper,
        "no_environment_bypass": "os.environ" not in wrapper,
    }


def _record_missing(errors: list[str], checks: dict[str, bool], label: str) -> None:
    for name, met in checks.items():
        if not met:
            errors.append(f"{label} missing: {name}")


def audit() -> tuple[list[str], dict[str, object]]:
    errors: list[str] = []
    destructive_scripts, guarded_scripts = _script_phase(errors)
    guarded_markdown = _markdown_phase(errors)
    source_violations = _source_phase(errors)

    sources = _load_sources()
    user_consent_checks = _user_consent_checks(sources)
    _record_missing(errors, user_consent_checks, USER_CONSENT_LABEL)
    bootloader_gated = _bootloader_phase(errors, sources.secure_boot_source)
    owner_consent_checks = {**_owner_key_checks(sources), **_owner_bootloader_checks(sources)}
    _record_missing(errors, owner_consent_checks, OWNER_CONSENT_LABEL)
    wrapper_checks = _wrapper_checks(sources.wrapper)
    _record_missing(errors, wrapper_checks, WRAPPER_LABEL)

    document: dict[str, object] = {
        "schema_version": 1,
        "healthy": not errors,
        "developer_ack_wrapper": ACK_WRAPPER,
        "destructive_scripts": destructive_scripts,
        "guarded_scripts": guarded_scripts,
        "guarded_markdown_runbooks": guarded_markdown,
        "unauthorized_irreversible_source_calls": source_violations,
        "user_consent": user_consent_checks,
        "owner_authority_consent": owner_consent_checks,
        "bootloader_pop_it_precedes_irreversible_call": bootloader_gated,
        "developer_acknowledgement": wrapper_checks,
        "errors": errors,
    }
    return errors, document


def main() -> int:
    errors, document = audit()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        return 1
    print(
        "PASS: irreversible hardware actions require interactive developer acknowledgement; "
        "CoreS3 Secure Boot remains gated by typed on-device Pop It consent"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
