use super::SdWorkflowContext;
use crate::controllers::TouchInput;
use crate::runtime::input::AppState;

pub(super) fn exercise(ctx: &mut SdWorkflowContext<'_, '_, '_>) -> bool {
    if !physical_card_scan_when_hil(ctx) {
        return false;
    }
    paging_and_delete_cancel(ctx)
}

#[cfg(feature = "workflow-hil-auto")]
fn physical_card_scan_when_hil(ctx: &mut SdWorkflowContext<'_, '_, '_>) -> bool {
    import_menu_card_scan(ctx)
}

#[cfg(not(feature = "workflow-hil-auto"))]
fn physical_card_scan_when_hil(ctx: &mut SdWorkflowContext<'_, '_, '_>) -> bool {
    let _ = ctx;
    log!("KASKOLD_WORKFLOW_TESTS: SD IMPORT MENU PHYSICAL CARD SCAN DEFERRED TO workflow-hil");
    true
}

#[cfg(feature = "workflow-hil-auto")]
fn import_menu_card_scan(ctx: &mut SdWorkflowContext<'_, '_, '_>) -> bool {
    if crate::services::storage_device::known_card().is_none() {
        return false;
    }
    if !ctx.enter_import_menu() {
        return false;
    }
    let transaction = ctx.list[1];
    if ctx.sd_touch(TouchInput::new(transaction.x + transaction.w / 2, transaction.y + transaction.h / 2, false))
        != Some(true)
    {
        return false;
    }
    match ctx.ad.navigation.app.state {
        AppState::SdImportMenu => {}
        AppState::SdKsptFileList => {
            if ctx.sd_touch(TouchInput::new(20, 20, true)) != Some(true)
                || ctx.ad.navigation.app.state != AppState::SdImportMenu
            {
                return false;
            }
        }
        _ => return false,
    }
    if ctx.sd_touch(TouchInput::new(20, 20, true)) != Some(true) || ctx.ad.navigation.app.state != AppState::SeedsMenu {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: SD IMPORT MENU REAL-CARD SCAN/BACK OWNER OK");
    true
}

fn paging_and_delete_cancel(ctx: &mut SdWorkflowContext<'_, '_, '_>) -> bool {
    for index in 0..8usize {
        let mut name = *b"FILE000 TXT";
        name[4] = b'0' + index as u8;
        ctx.ad.storage.browser.file_list[index] = name;
    }
    ctx.ad.storage.browser.file_count = 8;
    ctx.ad.storage.browser.file_scroll = 0;
    if !ctx.enter_import_list(AppState::SdFileList) {
        return false;
    }
    if ctx.sd_touch(TouchInput::new(300, 80, false)) != Some(true) || ctx.ad.storage.browser.file_scroll != 4 {
        return false;
    }
    if ctx.sd_touch(TouchInput::new(20, 80, false)) != Some(true) || ctx.ad.storage.browser.file_scroll != 0 {
        return false;
    }
    let first = ctx.list[0];
    if ctx.sd_touch(TouchInput::new(first.x + first.w - 5, first.y + first.h / 2, false)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::SdDeleteConfirm
    {
        return false;
    }
    if ctx.sd_touch(TouchInput::new(90, 205, false)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::SdFileList
        || ctx.ad.storage.browser.file_count != 8
    {
        return false;
    }
    if ctx.sd_touch(TouchInput::new(20, 20, true)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::SdImportMenu
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: SD FILE LIST PAGING/DELETE-CANCEL/BACK OK");
    true
}
