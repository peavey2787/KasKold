"""Interaction remediation ownership guards: single owners for redraw, Back, feedback and menu mapping."""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path
import re


SourceReader = Callable[[Path], str]


def _direct_owner_violations(
    root: Path,
    controller_sources: dict[Path, str],
    owner: Path,
    pattern: str,
) -> list[Path]:
    return sorted(
        path.relative_to(root)
        for path, source in controller_sources.items()
        if path != owner and re.search(pattern, source)
    )


def _check_redraw_ownership(root: Path, controller_sources: dict[Path, str]) -> list[str]:
    errors: list[str] = []
    redraw_owners = sorted(
        path.relative_to(root)
        for path, source in controller_sources.items()
        if re.search(r"\bstruct\s+RedrawFlag\b", source)
    )
    expected_redraw_owner = [Path("apps/kaskold-hardware/src/runtime/interactions/support/feedback.rs")]
    if redraw_owners != expected_redraw_owner:
        errors.append(
            f"interaction redraw accumulator must be owned only by feedback.rs: {redraw_owners}"
        )
    invalid_redraw_calls = sorted(
        path.relative_to(root)
        for path, source in controller_sources.items()
        if re.search(r"(?:ad|self)\.runtime\.needs_redraw\s*\.\s*set\s*\(", source)
    )
    if invalid_redraw_calls:
        errors.append(f"plain runtime redraw bool is used as a wrapper: {invalid_redraw_calls}")
    return errors


def _check_camera_back_policy(
    firmware: Path,
    interactions: Path,
    read: SourceReader,
) -> list[str]:
    errors: list[str] = []
    camera = interactions / "camera_loop"
    camera_sources = {path: read(path) for path in camera.rglob("*.rs")}
    combined_camera = "\n".join(camera_sources.values())
    if len(re.findall(r"\bfn\s+route_camera_back\s*\(", combined_camera)) != 1:
        errors.append("camera back navigation must have exactly one policy owner")
    touch_input_source = camera_sources.get(camera / "touch_input.rs", "")
    capture_source = camera_sources.get(camera / "dvp_capture.rs", "")
    event_dispatch = read(firmware / "runtime/event_loop/dispatch.rs")
    if "pub(crate) fn route_camera_back" not in touch_input_source:
        errors.append("camera back navigation policy must remain owned by touch_input.rs")
    if "runtime::interactions::camera_loop::route_camera_back($ad)" not in event_dispatch:
        errors.append("event-loop touch owner must dispatch camera Back through the shared policy")
    for token in ("touch_service::", "read_touch(", "route_camera_back(ad)"):
        if token in capture_source:
            errors.append(f"camera capture must not own touch/back dispatch: {token}")
    return errors


def _check_navigation_cards(firmware: Path, read: SourceReader) -> list[str]:
    errors: list[str] = []
    navigation = firmware / "ui/screens/navigation"
    menu_source = read(navigation / "menu.rs")
    secondary_source = read(navigation / "secondary.rs")
    if menu_source.count("fn draw_navigation_card") != 1:
        errors.append("navigation card presentation must have exactly one rendering owner")
    if menu_source.count("draw_navigation_card(") < 2:
        errors.append("primary navigation menu bypasses shared card rendering")
    if "RoundedRectangle::new" in secondary_source:
        errors.append("secondary navigation menu duplicates card presentation details")
    return errors


def _owner_bypass_errors(
    root: Path,
    controller_sources: dict[Path, str],
    interactions: Path,
) -> list[str]:
    errors: list[str] = []
    direct_feedback = _direct_owner_violations(
        root,
        controller_sources,
        interactions / "support/feedback.rs",
        r"\.draw_(?:rejected|success)_screen\s*\(",
    )
    if direct_feedback:
        errors.append(
            f"interaction transient feedback bypasses feedback.rs: {direct_feedback}"
        )

    direct_menu_mapping = _direct_owner_violations(
        root,
        controller_sources,
        interactions / "support/menu_selection.rs",
        r"\.(?:visible_to_absolute|can_page_up|can_page_down)\s*\(",
    )
    if direct_menu_mapping:
        errors.append(
            f"interaction menu hit-testing bypasses menu_selection.rs: {direct_menu_mapping}"
        )
    return errors


def _pure_controller_errors(firmware: Path, read: SourceReader) -> list[str]:
    # The pure controller boundary may classify normalized input only. Effectful
    # hardware adapters deliberately live under runtime/interactions.
    pure_controller = read(firmware / "controllers.rs")
    return [
        f"pure controller boundary regained effectful dependency: {token}"
        for token in ("esp_hal", "crate::hw", "crate::services", "crate::ui", "PersistentWallet", "BootDisplay")
        if token in pure_controller
    ]


def check(root: Path, read: SourceReader) -> list[str]:
    firmware = root / "apps/kaskold-hardware/src"
    interactions = firmware / "runtime/interactions"
    controller_sources = {
        path: read(path) for path in interactions.rglob("*.rs")
    }

    errors = _check_redraw_ownership(root, controller_sources)
    errors.extend(_check_camera_back_policy(firmware, interactions, read))
    errors.extend(_check_navigation_cards(firmware, read))
    errors.extend(_owner_bypass_errors(root, controller_sources, interactions))
    errors.extend(_pure_controller_errors(firmware, read))
    return errors
