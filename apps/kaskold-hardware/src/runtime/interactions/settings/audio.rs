// KasKold — Air-gapped offline signing device for Kaspa
// Focused settings controller component.
use super::AppData;
use crate::controllers::TouchInput;
use crate::{services::audio as sound, ui::screens::device::AUDIO_STARTUP_ROW_Y};

/// Handle CoreS3 audio settings without borrowing display/I2C/storage/camera
/// resources. The shared frame stage performs the redraw after a value change.
pub(super) fn handle_audio_settings(ad: &mut AppData, input: TouchInput) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SettingsMenu));
        return true;
    }

    if ad.storage.persistence.advanced.saved_wallet && AUDIO_STARTUP_ROW_Y.contains(&y) {
        ad.settings.toggle_startup_sound();
        return true;
    }

    let Some(value) = super::scalar::update(ad.settings.volume, x, y) else {
        return false;
    };
    if value == ad.settings.volume {
        return false;
    }

    ad.settings.set_volume(value);
    sound::set_volume(ad.settings.volume);
    true
}
