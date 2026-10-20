//! Connected probes for production surfaces that were still catalog-only before connected coverage was added.
//!
//! Keep credit conservative: these probes execute real production parsers,
//! policy services, and navigation reducers, but physical flash/RTC/eFuse/SD
//! fault injection remains owned by the explicit HIL profile.

use crate::{
    hw::{display::BootDisplay, sdcard::SdCardType},
    runtime::interactions::TouchInput,
    runtime::{data::AppData, input::AppState},
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

mod covenant;
mod firmware_update;

pub(super) fn exercise(
    ad: &mut AppData,
    display: &mut BootDisplay<'_>,
    i2c: &mut I2c<'_, Blocking>,
    sd: &Option<SdCardType>,
    delay: &mut Delay,
) -> bool {
    log!("KASKOLD_WORKFLOW_TESTS: REMAINING PRODUCTION SURFACES BEGIN");
    let mut summary = super::remaining_status::ProbeSummary::new();
    summary.run("COVENANT-RESTORE", covenant::backup_forms);
    summary.run("COVENANT-SIGN", || covenant::signing_protocol(ad));
    summary.run("PRIVATE-SWAP", || private_swap_protocol(ad));
    summary.run("FIRMWARE-UPDATE", || firmware_update::exercise(ad));
    summary.run("MISSING-KEY", || missing_key_fail_closed(ad));
    summary.run("UNLOCK-GUARDS", || unlock_navigation_guards(ad));
    summary.run("DEVICE-BOUND-SD", || device_bound_sd_routing(ad, display, i2c, sd, delay));
    summary.run("FINISH-HOME", || finish(ad));
    summary.finish(8)
}

fn private_swap_protocol(ad: &mut AppData) -> bool {
    if !crate::services::wallet_session::install_workflow_backup_mnemonic_fixture(ad) {
        log!("KASKOLD_WORKFLOW_TESTS: PRIVATE SWAP MNEMONIC FIXTURE FAIL");
        return false;
    }
    use shared_signer::covenant_sign::private_swap::{self as wire, PrivateSwapRequest, RequestKind};
    if crate::services::private_swap::prepare_request(ad, b"PSWG", &mut || {}).is_ok() {
        return false;
    }
    let requests = [
        PrivateSwapRequest {
            kind: RequestKind::KeyInfo,
            session_id: [0; 16],
            host_commitment: [0; 32],
            key_id: [0; 32],
            binding_token: [0; 32],
            adaptor_point: [0; 32],
            presignature: [0; 64],
            presignature_negated: false,
            payload: &[],
        },
        PrivateSwapRequest {
            kind: RequestKind::Bind,
            session_id: [0; 16],
            host_commitment: [0; 32],
            key_id: [2; 32],
            binding_token: [0; 32],
            adaptor_point: [3; 32],
            presignature: [0; 64],
            presignature_negated: false,
            payload: b"swap-script",
        },
    ];
    let mut encoded = [0u8; 320];
    for (index, request) in requests.into_iter().enumerate() {
        let Ok(len) = wire::encode_request(&request, &mut encoded) else {
            return false;
        };
        let Ok(parsed) = wire::parse_request(&encoded[..len]) else {
            return false;
        };
        if parsed.kind != request.kind {
            return false;
        }
        if index == 0
            && (crate::services::private_swap::prepare_request(ad, &encoded[..len], &mut || {}).is_err()
                || ad.signing.private_swap.response_len == 0)
        {
            return false;
        }
    }
    let mut malformed = encoded;
    malformed[0] ^= 1;
    if wire::parse_request(&malformed).is_ok() {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: PRIVATE SWAP KEYINFO/BIND WIRE + MALFORMED REJECT PASS");
    true
}

fn missing_key_fail_closed(ad: &mut AppData) -> bool {
    if !crate::services::wallet_session::install_workflow_backup_mnemonic_fixture(ad) {
        return false;
    }
    use shared_signer::covenant_sign::{self as wire, BindingHint, CovenantSignRequest, KnownScheme, RequestKind};
    let old_active = ad.wallet.seeds.seed_mgr.active;
    ad.wallet.seeds.seed_mgr.clear_active();
    let request = CovenantSignRequest {
        kind: RequestKind::Opaque,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [1; wire::SESSION_ID_LEN],
        host_commitment: [2; 32],
        key_id: [3; 32],
        binding_token: [4; 32],
        commitment: [5; 32],
        script: b"missing-key-probe",
        context: &[],
    };
    let mut encoded = [0u8; 256];
    let Ok(len) = wire::encode_request(&request, &mut encoded) else {
        return false;
    };
    let missing = crate::services::covenant_sign::prepare_request(ad, &encoded[..len], &mut || {})
        == Err(crate::services::covenant_sign::CovenantSignError::MnemonicRequired);
    let restored = old_active == u8::MAX || ad.wallet.seeds.seed_mgr.set_active(usize::from(old_active));
    if !missing || !restored {
        return false;
    }

    let Some(expected_raw_slot) = ad.wallet.seeds.seed_mgr.find_free() else {
        return false;
    };
    let Some(raw_slot) = ad.wallet.seeds.seed_mgr.store_raw_key(&[7u8; 32]) else {
        return false;
    };
    if raw_slot != expected_raw_slot || !ad.wallet.seeds.seed_mgr.set_active(raw_slot) {
        if old_active != u8::MAX {
            let _ = ad.wallet.seeds.seed_mgr.set_active(usize::from(old_active));
        }
        return false;
    }
    let raw_rejected = crate::services::covenant_sign::prepare_request(ad, &encoded[..len], &mut || {})
        == Err(crate::services::covenant_sign::CovenantSignError::MnemonicRequired);
    ad.wallet.seeds.seed_mgr.delete(raw_slot);
    let restored = old_active == u8::MAX || ad.wallet.seeds.seed_mgr.set_active(usize::from(old_active));
    if !raw_rejected || !restored {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: MISSING MNEMONIC/RAW-HD SIGNING FAIL-CLOSED PASS");
    true
}

fn unlock_navigation_guards(ad: &mut AppData) -> bool {
    if !crate::runtime::interactions::persistence::workflow_unlock_back_guard(AppState::StorageUnlockPin)
        || !crate::runtime::interactions::persistence::workflow_unlock_back_guard(AppState::StorageUnlockPassword)
    {
        return false;
    }
    let pin_home_hidden =
        crate::runtime::navigation::workflow_home_shortcut_hidden_for_unlock(AppState::StorageUnlockPin);
    let password_home_hidden =
        crate::runtime::navigation::workflow_home_shortcut_hidden_for_unlock(AppState::StorageUnlockPassword);
    let backoff = crate::runtime::interactions::persistence::workflow_unlock_backoff_probe(ad);
    if !pin_home_hidden || !password_home_hidden || !backoff {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: UNLOCK BACK/HOME GUARDS + RETRY BACKOFF PASS");
    true
}

fn device_bound_sd_routing(
    ad: &mut AppData,
    display: &mut BootDisplay<'_>,
    i2c: &mut I2c<'_, Blocking>,
    sd: &Option<SdCardType>,
    delay: &mut Delay,
) -> bool {
    use crate::ui::screens::device::advanced_security::{ADV_CARD_X, SD_STORAGE_Y};
    crate::runtime::effects::home(ad);
    let settings_zone = crate::ui::layout::HOME_GRID_ZONES[3];
    if !crate::runtime::interactions::menu::handle_root_touch(
        ad,
        settings_zone.x + settings_zone.w / 2,
        settings_zone.y + settings_zone.h / 2,
    ) || ad.navigation.app.state != AppState::SettingsMenu
    {
        return false;
    }
    let (_, list, up, down) = crate::runtime::touch_dispatch::touch_zones();
    let security = list[2];
    if crate::runtime::interactions::settings::handle_settings_menu_navigation(
        ad,
        &list,
        &up,
        &down,
        TouchInput::new(security.x + security.w / 2, security.y + security.h / 2, false),
    ) != Some(true)
        || ad.navigation.app.state != AppState::AdvancedFeatures
    {
        return false;
    }
    crate::runtime::interactions::settings::advanced::workflow::install_saved_wallet_fixture(
        ad,
        crate::services::credential_policy::CredentialKind::Pin,
    );
    let x = *ADV_CARD_X.start() + 8;
    let y = *SD_STORAGE_Y.start() + 8;
    if crate::runtime::interactions::settings::advanced::workflow::open_card(
        TouchInput::new(x, y, false),
        ad,
        display,
        delay,
    ) != Some(true)
        || ad.navigation.app.state != AppState::StorageRecoveryAcknowledgement
    {
        return false;
    }
    if crate::runtime::interactions::persistence::workflow_handle_recovery_acknowledgement(
        TouchInput::new(20, 20, true),
        ad,
        display,
        delay,
    ) != Some(true)
        || ad.navigation.app.state != AppState::AdvancedFeatures
    {
        return false;
    }
    if crate::runtime::interactions::settings::advanced::workflow::open_card(
        TouchInput::new(x, y, false),
        ad,
        display,
        delay,
    ) != Some(true)
        || crate::runtime::interactions::persistence::workflow_handle_recovery_acknowledgement(
            TouchInput::new(160, 188, false),
            ad,
            display,
            delay,
        ) != Some(true)
        || ad.navigation.app.state != AppState::AdvancedSdStorageWarning
    {
        return false;
    }
    if crate::runtime::interactions::settings::advanced::handle_pure(TouchInput::new(20, 20, true), ad) != Some(true)
        || ad.navigation.app.state != AppState::AdvancedFeatures
    {
        return false;
    }
    super::redraw_step(ad, display, i2c, sd);
    log!("KASKOLD_WORKFLOW_TESTS: DEVICE-BOUND SD SETUP/ACK/CANCEL ROUTING PASS");
    true
}

fn finish(ad: &mut AppData) -> bool {
    crate::runtime::effects::home(ad);
    let ok = super::root::home_ok(ad);
    if ok {
        log!("KASKOLD_WORKFLOW_TESTS: REMAINING PRODUCTION SURFACES PASS");
    }
    ok
}
