use super::MultisigContext;
use crate::controllers::TouchInput;
use crate::runtime::input::AppState;

pub(super) fn exercise(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    if !multisig_kpub_export(ctx) {
        return false;
    }
    let first = ctx.list[0];
    if ctx.menu_touch(TouchInput::new(first.x + first.w / 2, first.y + first.h / 2, false)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::MultisigChooseMN
        || ctx.ad.signing.multisig.threshold != 2
        || ctx.ad.signing.multisig.participant_count != 3
    {
        return false;
    }
    ctx.redraw_step();
    if !minimum_boundaries(ctx) || !maximum_and_clamp(ctx) || !reset_to_two_of_three(ctx) {
        return false;
    }
    if ctx.tx_touch(TouchInput::new(160, 210, false)) != Some(true)
        || ctx.ad.navigation.app.state != (AppState::MultisigAddKey { key_idx: 0 })
        || ctx.ad.signing.multisig.creating.m != 2
        || ctx.ad.signing.multisig.creating.n != 3
        || !ctx.ad.signing.multisig.creating.v45
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: MULTISIG M/N MIN/MAX/CLAMP/START PASS");
    true
}

fn minimum_boundaries(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    if ctx.tx_touch(TouchInput::new(85, 84, false)) != Some(true) || ctx.ad.signing.multisig.threshold != 1 {
        return false;
    }
    if ctx.tx_touch(TouchInput::new(85, 84, false)) != Some(false) || ctx.ad.signing.multisig.threshold != 1 {
        return false;
    }
    if !drive_participant_count(ctx, 1) {
        return false;
    }
    ctx.tx_touch(TouchInput::new(85, 144, false)) == Some(false) && ctx.ad.signing.multisig.participant_count == 1
}

fn maximum_and_clamp(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    if !drive_participant_count(ctx, 5) {
        return false;
    }
    if ctx.tx_touch(TouchInput::new(235, 144, false)) != Some(false) {
        return false;
    }
    if !drive_threshold(ctx, 5) {
        return false;
    }
    if ctx.tx_touch(TouchInput::new(235, 84, false)) != Some(false) {
        return false;
    }
    if ctx.tx_touch(TouchInput::new(85, 144, false)) != Some(true) {
        return false;
    }
    ctx.ad.signing.multisig.participant_count == 4 && ctx.ad.signing.multisig.threshold == 4
}

fn reset_to_two_of_three(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    drive_participant_count(ctx, 3)
        && drive_threshold(ctx, 2)
        && ctx.ad.signing.multisig.threshold == 2
        && ctx.ad.signing.multisig.participant_count == 3
}

fn drive_participant_count(ctx: &mut MultisigContext<'_, '_, '_>, target: u8) -> bool {
    for _ in 0..5 {
        let current = ctx.ad.signing.multisig.participant_count;
        if current == target {
            return true;
        }
        let (x, expected_direction) = if current < target { (235, 1i8) } else { (85, -1i8) };
        if ctx.tx_touch(TouchInput::new(x, 144, false)) != Some(true) {
            return false;
        }
        let next = ctx.ad.signing.multisig.participant_count;
        if (expected_direction > 0 && next <= current) || (expected_direction < 0 && next >= current) {
            return false;
        }
    }
    ctx.ad.signing.multisig.participant_count == target
}

fn drive_threshold(ctx: &mut MultisigContext<'_, '_, '_>, target: u8) -> bool {
    for _ in 0..5 {
        let current = ctx.ad.signing.multisig.threshold;
        if current == target {
            return true;
        }
        let (x, expected_direction) = if current < target { (235, 1i8) } else { (85, -1i8) };
        if ctx.tx_touch(TouchInput::new(x, 84, false)) != Some(true) {
            return false;
        }
        let next = ctx.ad.signing.multisig.threshold;
        if (expected_direction > 0 && next <= current) || (expected_direction < 0 && next >= current) {
            return false;
        }
    }
    ctx.ad.signing.multisig.threshold == target
}

fn multisig_kpub_export(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    let zone = ctx.list[2];
    if ctx.signing_feedback_touch(TouchInput::new(zone.x + zone.w / 2, zone.y + zone.h / 2, false)) != Some(true) {
        return false;
    }
    // Runtime-auto queues a transient operation on MultisigMenu. Commit its
    // loading frame before the cooperative worker is allowed to advance.
    ctx.redraw_step();
    if !complete_runtime_kpub_operation(ctx)
        || ctx.ad.navigation.app.state != AppState::ExportKpub
        || ctx.ad.export.kpub_len == 0
    {
        return false;
    }
    if ctx.export_screen_touch(TouchInput::new(160, 120, false)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::ExportKpubPopup
    {
        return false;
    }
    if ctx.export_screen_touch(TouchInput::new(20, 20, true)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::MultisigMenu
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: MULTISIG KPUB EXPORT/POPUP/BACK PASS");
    true
}

/// Drive the multisig kpub derivation to completion before the production watchdog is armed.
#[cfg(all(feature = "m5stack", feature = "workflow-runtime-auto"))]
fn complete_runtime_kpub_operation(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    if !crate::runtime::presentation::operation_active(ctx.ad, crate::runtime::data::OperationKind::DeriveMultisigKpub)
    {
        return false;
    }
    // runtime_gui later repeats this with the real watchdog feed and physical rendering.
    let mut feed = || {};
    crate::runtime::event_loop::runner::workflow_drive_multisig_kpub(ctx.ad, ctx.display, ctx.delay, &mut feed)
}

/// Only runtime-auto images queue the derivation as an operation; other catalogs have nothing pending.
#[cfg(not(all(feature = "m5stack", feature = "workflow-runtime-auto")))]
fn complete_runtime_kpub_operation(ctx: &mut MultisigContext<'_, '_, '_>) -> bool {
    let _ = ctx;
    true
}
