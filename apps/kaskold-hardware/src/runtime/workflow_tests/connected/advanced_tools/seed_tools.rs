use crate::runtime::input::AppState;

use super::AdvancedToolsContext;

pub(super) fn exercise(ctx: &mut AdvancedToolsContext<'_, '_, '_>) -> bool {
    bip85(ctx)
}

fn install_parent(ctx: &mut AdvancedToolsContext<'_, '_, '_>) -> bool {
    crate::services::wallet_session::install_workflow_backup_mnemonic_fixture(ctx.ad)
}

fn choose_tool_word_count(ctx: &mut AdvancedToolsContext<'_, '_, '_>, item: usize, y: u16) -> bool {
    if !install_parent(ctx)
        || !ctx.open_advanced_item(item, AppState::ChooseWordCount { action: 4 })
    {
        return false;
    }
    crate::runtime::interactions::menu::seed_generation::workflow_existing_tool_word_count(
        ctx.ad, 160, y, false,
    )
}

fn bip85(ctx: &mut AdvancedToolsContext<'_, '_, '_>) -> bool {
    if !bip85_first_child(ctx) {
        return false;
    }
    let child_zero = ctx.ad.wallet.seeds.bip85_child_indices;
    if !walk_bip85_words(ctx, 12)
        || ctx.ad.navigation.app.state != AppState::WalletAdvancedMenu
        || !bip85_determinism(ctx, &child_zero)
        || !bip85_24_words(ctx)
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: ADVANCED BIP85 12/24 INDEX/BOUNDARY/DETERMINISM PASS");
    true
}

fn bip85_first_child(ctx: &mut AdvancedToolsContext<'_, '_, '_>) -> bool {
    if !choose_tool_word_count(ctx, 0, 100)
        || ctx.ad.navigation.app.state != (AppState::Bip85Index { word_count: 12 })
    {
        return false;
    }
    if ctx.seed_touch(105, 115, false) != Some(false) || ctx.ad.wallet.seeds.bip85_index != 0 {
        return false;
    }
    ctx.ad.wallet.seeds.bip85_index = 99;
    if ctx.seed_touch(215, 115, false) != Some(false) || ctx.ad.wallet.seeds.bip85_index != 99 {
        return false;
    }
    ctx.ad.wallet.seeds.bip85_index = 0;
    ctx.seed_touch(160, 166, false) == Some(true)
        && ctx.ad.navigation.app.state == (AppState::Bip85ShowWord { word_idx: 0, word_count: 12 })
}

fn bip85_determinism(ctx: &mut AdvancedToolsContext<'_, '_, '_>, child_zero: &[u16; 24]) -> bool {
    if !derive_bip85(ctx, 0) || &ctx.ad.wallet.seeds.bip85_child_indices != child_zero {
        return false;
    }
    if !walk_bip85_words(ctx, 12)
        || !derive_bip85(ctx, 1)
        || &ctx.ad.wallet.seeds.bip85_child_indices == child_zero
    {
        return false;
    }
    walk_bip85_words(ctx, 12)
}

fn bip85_24_words(ctx: &mut AdvancedToolsContext<'_, '_, '_>) -> bool {
    choose_tool_word_count(ctx, 0, 180)
        && ctx.ad.navigation.app.state == (AppState::Bip85Index { word_count: 24 })
        && ctx.seed_touch(160, 166, false) == Some(true)
        && walk_bip85_words(ctx, 24)
}

fn derive_bip85(ctx: &mut AdvancedToolsContext<'_, '_, '_>, index: u8) -> bool {
    if !choose_tool_word_count(ctx, 0, 100) {
        return false;
    }
    ctx.ad.wallet.seeds.bip85_index = index;
    ctx.seed_touch(160, 166, false) == Some(true)
        && matches!(ctx.ad.navigation.app.state, AppState::Bip85ShowWord { word_idx: 0, word_count: 12 })
}

fn walk_bip85_words(ctx: &mut AdvancedToolsContext<'_, '_, '_>, count: u8) -> bool {
    for _ in 0..count {
        if ctx.seed_touch(160, 120, false) != Some(true) {
            return false;
        }
    }
    true
}

