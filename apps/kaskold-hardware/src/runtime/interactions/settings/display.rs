// KasKold — Air-gapped offline signing device for Kaspa
// Focused settings controller component.
use super::AppData;
use crate::controllers::TouchInput;
use crate::ui::screens::device::{DISPLAY_DIM_ROW_Y, DISPLAY_PIN_ROW_Y};

fn persistent_controls(ad: &AppData) -> bool {
    ad.storage.persistence.advanced.saved_wallet
}

fn pin_lock_available(ad: &AppData) -> bool {
    persistent_controls(ad)
        && ad.storage.persistence.advanced.credential_kind
            == Some(crate::services::persistent_wallet::CredentialKind::Pin)
}

#[cfg(feature = "m5stack")]
pub(super) fn handle_display_settings(ad: &mut AppData, input: TouchInput) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SettingsMenu));
        return true;
    }
    if persistent_controls(ad) {
        if DISPLAY_DIM_ROW_Y.contains(&y) && ad.settings.step_screen_dim_timeout(x < 160) {
            return true;
        }
        if DISPLAY_PIN_ROW_Y.contains(&y) && pin_lock_available(ad) {
            ad.settings.toggle_pin_after_dim();
            return true;
        }
    }
    let Some(value) = super::scalar::update(ad.settings.brightness, x, y) else {
        return false;
    };
    if value == ad.settings.brightness {
        return false;
    }
    ad.settings.brightness = value;
    true
}
