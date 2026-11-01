use super::display;
use crate::runtime::{data::AppData, input::AppState};

pub(super) fn redraw(ad: &AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    match ad.navigation.app.state {
        AppState::XprvExportMenu => {
            boot_display.update_menu_content("XPRV ACCOUNT", &ad.navigation.xprv_export_menu);
        }
        _ => return false,
    }
    true
}
