"""Menu mirror and workflow E2E state checks for the Vault Web gate."""
from __future__ import annotations

import json
import re

from vault_web_architecture_rules import ROOT, _read

# Vault Web mirrors the hardware navigation model wherever the feature has a
# meaningful software implementation. These Settings surfaces are deliberately
# physical-M5-only and must not be emulated in software Vaults.
SOFTWARE_VAULT_OMITTED_MENUS = {"AdvancedMenu", "OwnerFirmwareMenu"}
SOFTWARE_VAULT_OMITTED_SETTINGS_ACTIONS = {
    "settings.display",
    "settings.audio",
    "settings.advanced",
}
SOFTWARE_VAULT_LABEL_OVERRIDES = {
    ("ConfirmTx", "tx.confirm"): "Sign",
}

SOFTWARE_VAULT_OMITTED_STATES = {
    "AdvancedMenu",
    "AdvancedRtcEntry",
    "AdvancedSdStorageWarning",
    "AudioSettings",
    "DisplaySettings",
    "FactoryResetConfirm",
    "FactoryResetWarning",
    "OwnerFirmwareMenu",
    "OwnerInstallConfirm",
    "OwnerInstallWarning",
    "OwnerKeyConfirm",
    "OwnerKeyWarning",
    "PopItConfirm",
    "PopItExplain",
    "PopItPrompt",
}


def _expected_menu_items(menus: dict, menu_name: str) -> list[tuple[str, str]]:
    items = menus[menu_name]["items"]
    if menu_name == "SettingsMenu":
        items = [
            item for item in items
            if item["action"] not in SOFTWARE_VAULT_OMITTED_SETTINGS_ACTIONS
        ]
    return [
        (
            item["action"],
            SOFTWARE_VAULT_LABEL_OVERRIDES.get((menu_name, item["action"]), item["label"]),
        )
        for item in items
    ]


def _check_action_binding(errors: list[str], action: str, attrs: str, js: str) -> None:
    # Every retained M5 menu action must be wired to an executable Web
    # handler. Nested icon/span markup is allowed on the home grid.
    id_match = re.search(r'\bid="([^"]+)"', attrs, flags=re.I)
    if not id_match:
        errors.append(f"Vault Web workflow action {action} has no stable control id")
        return
    control_id = id_match.group(1)
    direct = "$('" + control_id + "').onclick"
    delegated = "click('" + control_id + "')"
    guarded = "bindClick('" + control_id + "'"
    if direct not in js and delegated not in js and guarded not in js:
        errors.append(
            f"Vault Web workflow action {action} ({control_id}) has no bound handler"
        )


def _menu_actions(errors: list[str], menu_html: str, js: str) -> list[tuple[str, str]]:
    actual: list[tuple[str, str]] = []
    for button in re.finditer(
        r'<button\b([^>]*)data-workflow-action="([^"]+)"([^>]*)>(.*?)</button>',
        menu_html,
        flags=re.I | re.S,
    ):
        attrs = button.group(1) + button.group(3)
        action = button.group(2)
        label = re.sub(r"<[^>]+>", "", button.group(4))
        actual.append((action, re.sub(r"\s+", " ", label).strip()))
        _check_action_binding(errors, action, attrs, js)
    return actual


def _check_m5_menu_mirror(errors: list[str], html: str, js: str) -> None:
    # Software Vault navigation mirrors the authoritative M5 production menu
    # graph instead of maintaining a second hand-written menu contract.
    workflow_graph = json.loads(_read(ROOT / "qa/config/workflow/production_ui_graph.json"))
    menus = {menu["state"]: menu for menu in workflow_graph["menus"]}
    mirrored = tuple(
        menu["state"]
        for menu in workflow_graph["menus"]
        if menu["state"] not in SOFTWARE_VAULT_OMITTED_MENUS
    )
    for menu_name in mirrored:
        section_match = re.search(
            rf'<section\b[^>]*data-workflow-menu="{re.escape(menu_name)}"[^>]*>(.*?)</section>',
            html,
            flags=re.I | re.S,
        )
        if not section_match:
            errors.append(f"Vault Web is missing mirrored M5 menu {menu_name}")
            continue
        actual = _menu_actions(errors, section_match.group(1), js)
        expected = _expected_menu_items(menus, menu_name)
        if actual != expected:
            errors.append(
                f"Vault Web menu {menu_name} drifted from its M5/software projection: "
                f"expected {expected!r}, got {actual!r}"
            )


def _rendered_states(html: str) -> set[str]:
    rendered_states: set[str] = set()
    for value in re.findall(r'data-workflow-state="([^"]+)"', html):
        rendered_states.update(value.split())
    return rendered_states


def _check_physical_states(errors: list[str], e2e: dict, rendered_states: set[str]) -> None:
    missing_render_states = sorted(
        set(e2e["workflow_e2e_physical_render_states"])
        - rendered_states
        - SOFTWARE_VAULT_OMITTED_STATES
    )
    if missing_render_states:
        errors.append(
            "Vault Web is missing authoritative M5 physical workflow states: "
            + ", ".join(missing_render_states)
        )


def _check_connected_states(errors: list[str], e2e: dict, rendered_states: set[str]) -> None:
    connected_states: set[str] = set()
    for scenario in e2e["scenarios"]:
        if scenario.get("level") in {"connected", "qa"} or scenario.get("id", "").startswith("qa-"):
            connected_states.update(scenario.get("states", []))
    missing_connected_states = sorted(
        connected_states - rendered_states - SOFTWARE_VAULT_OMITTED_STATES
    )
    if missing_connected_states:
        errors.append(
            "Vault Web is missing authoritative connected M5 E2E states/equivalents: "
            + ", ".join(missing_connected_states)
        )


def _check_e2e_states(errors: list[str], html: str) -> None:
    e2e = json.loads(_read(ROOT / "qa/config/workflow/production_e2e_scenarios.json"))
    rendered_states = _rendered_states(html)
    _check_physical_states(errors, e2e, rendered_states)
    _check_connected_states(errors, e2e, rendered_states)
