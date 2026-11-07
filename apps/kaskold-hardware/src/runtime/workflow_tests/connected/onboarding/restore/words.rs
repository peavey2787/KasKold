use crate::controllers::TouchInput;
use crate::runtime::input::AppState;

use super::{
    begin_restore, choose_no_passphrase, finish_restored_session, name_restored_wallet, RestoreIo, BUTTON_X, KEY_OK,
};

const KEY_A: (u16, u16) = (19, 100);
const KEY_B: (u16, u16) = (54, 100);
const KEY_L: (u16, u16) = (89, 130);
const KEY_M: (u16, u16) = (124, 130);
const KEY_O: (u16, u16) = (194, 130);
const KEY_Z: (u16, u16) = (299, 164);
const KEY_BACKSPACE: (u16, u16) = (0, 210);
const SUGGESTION_FIRST: (u16, u16) = (4, 80);

pub(super) fn restore_12(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    if !begin_restore(ctx)
        || ctx.source_touch(0) != Some(true)
        || ctx.base.ad.navigation.app.state != (AppState::RestoreWord { word_idx: 0 })
    {
        return false;
    }
    if !keyboard_error_matrix(ctx) || !enter_abandon(ctx) || !enter_abandon_repeated(ctx, 10) {
        return false;
    }
    if !enter_about(ctx) || ctx.base.ad.navigation.app.state != AppState::RestoreWord12Detected {
        return false;
    }
    ctx.redraw_step();
    if !detected_back_and_reenter(ctx)
        || ctx.detected_touch(93, false) != Some(true)
        || ctx.base.ad.navigation.app.state != AppState::PassphraseChoice
        || !choose_no_passphrase(ctx)
        || !name_restored_wallet(ctx, b"Restored 12")
    {
        return false;
    }
    if !finish_restored_session(ctx, 12) {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: RESTORE WORDS 12 CHECKSUM/DETECTED/BACK PASS");
    true
}

fn detected_back_and_reenter(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    ctx.detected_touch(20, true) == Some(true)
        && ctx.base.ad.navigation.app.state == (AppState::RestoreWord { word_idx: 11 })
        && enter_about(ctx)
        && ctx.base.ad.navigation.app.state == AppState::RestoreWord12Detected
}

pub(super) fn restore_24(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    if !begin_restore(ctx)
        || ctx.source_touch(0) != Some(true)
        || !enter_abandon_repeated(ctx, 11)
        || !enter_about(ctx)
        || ctx.base.ad.navigation.app.state != AppState::RestoreWord12Detected
    {
        return false;
    }
    if ctx.detected_touch(147, false) != Some(true)
        || ctx.base.ad.navigation.app.state != (AppState::RestoreWord { word_idx: 12 })
        || !enter_abandon_repeated(ctx, 11)
        || !enter_almost(ctx)
        || ctx.base.ad.navigation.app.state != AppState::PassphraseChoice
        || !choose_passphrase_a1(ctx)
        || !name_restored_wallet(ctx, b"Restored 24")
    {
        return false;
    }
    if !finish_restored_session(ctx, 24) {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: RESTORE WORDS 24 CONTINUE/PASSPHRASE PASS");
    true
}

pub(super) fn reject_invalid_24(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    if !begin_restore(ctx) || ctx.source_touch(0) != Some(true) || !enter_abandon_repeated(ctx, 24) {
        return false;
    }
    if ctx.base.ad.navigation.app.state != AppState::StorageSeedSourceChoice
        || ctx.base.ad.wallet.seeds.word_count != 0
        || ctx.base.ad.wallet.seeds.mnemonic_indices.iter().any(|word| *word != 0)
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: RESTORE INVALID 12 AUTO-CONTINUE + INVALID 24 REJECT OK");
    true
}

fn keyboard_error_matrix(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    if !tap_pending(ctx, KEY_Z) || !tap_pending(ctx, KEY_Z) || ctx.base.ad.wallet.seeds.word_input.match_count != 0 {
        return false;
    }
    if !tap_pending(ctx, KEY_BACKSPACE)
        || !tap_pending(ctx, KEY_BACKSPACE)
        || ctx.base.ad.wallet.seeds.word_input.prefix_len != 0
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: RESTORE WORD KEYBOARD NO-MATCH/BACKSPACE OK");
    true
}

fn enter_abandon_repeated(ctx: &mut RestoreIo<'_, '_, '_>, count: usize) -> bool {
    for _ in 0..count {
        if !enter_abandon(ctx) {
            return false;
        }
    }
    true
}

fn enter_abandon(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    tap_pending(ctx, KEY_A)
        && tap_pending(ctx, KEY_B)
        && ctx.base.seed_touch(TouchInput::new(SUGGESTION_FIRST.0, SUGGESTION_FIRST.1, false)) == Some(true)
}

fn enter_about(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    tap_pending(ctx, KEY_A)
        && tap_pending(ctx, KEY_B)
        && tap_pending(ctx, KEY_O)
        && ctx.base.seed_touch(TouchInput::new(SUGGESTION_FIRST.0, SUGGESTION_FIRST.1, false)) == Some(true)
}

fn enter_almost(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    tap_pending(ctx, KEY_A)
        && tap_pending(ctx, KEY_L)
        && tap_pending(ctx, KEY_M)
        && ctx.base.seed_touch(TouchInput::new(KEY_OK.0, KEY_OK.1, false)) == Some(true)
}

fn tap_pending(ctx: &mut RestoreIo<'_, '_, '_>, key: (u16, u16)) -> bool {
    ctx.base.seed_touch(TouchInput::new(key.0, key.1, false)) == Some(false)
}

fn choose_passphrase_a1(ctx: &mut RestoreIo<'_, '_, '_>) -> bool {
    if ctx.base.seed_touch(TouchInput::new(BUTTON_X, 193, false)) != Some(true)
        || ctx.base.ad.navigation.app.state != AppState::PassphraseEntry
    {
        return false;
    }
    if ctx.base.seed_touch(TouchInput::new(18, 97, false)) != Some(false)
        || ctx.base.seed_touch(TouchInput::new(18, 170, false)) != Some(false)
        || ctx.base.seed_touch(TouchInput::new(18, 170, false)) != Some(false)
        || ctx.base.seed_touch(TouchInput::new(18, 97, false)) != Some(false)
    {
        return false;
    }
    ctx.base.seed_touch(TouchInput::new(KEY_OK.0, KEY_OK.1, false)) == Some(true)
        && ctx.base.ad.navigation.app.state == (AppState::WalletNameEntry { purpose: 3 })
}
