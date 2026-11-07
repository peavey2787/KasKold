use crate::{
    hw::display::BootDisplay,
    runtime::interactions::TouchInput,
    runtime::{data::AppData, input::AppState},
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

mod bip85;
mod exports;
mod messages;
mod secrets;

pub(super) type AdvancedToolsContext<'ctx, 'display, 'hal> =
    super::menu_context::MenuWorkflowContext<'ctx, 'display, 'hal>;

impl AdvancedToolsContext<'_, '_, '_> {
    pub(super) fn seed_touch(&mut self, input: TouchInput) -> Option<bool> {
        crate::runtime::interactions::seed::handle_seed_touch(self.ad, self.display, self.delay, &mut || {}, input)
    }

    pub(super) fn set_text(&mut self, value: &[u8]) {
        self.ad.wallet.seeds.pp_input.reset();
        let length = value.len().min(self.ad.wallet.seeds.pp_input.buf.len());
        self.ad.wallet.seeds.pp_input.buf[..length].copy_from_slice(&value[..length]);
        self.ad.wallet.seeds.pp_input.len = length;
        self.ad.wallet.seeds.pp_input.cursor = length;
    }

    pub(super) fn open_wallet_advanced(&mut self) -> bool {
        crate::runtime::effects::home(self.ad);
        if !crate::runtime::interactions::menu::handle_root_touch(self.ad, 82, 188)
            || self.ad.navigation.app.state != AppState::SeedsMenu
        {
            return false;
        }
        self.ad.navigation.production.wallet_menu.reset();
        if self.menu_touch(TouchInput::new(self.down.x + 20, self.down.y + 20, false)) != Some(true) {
            return false;
        }
        // Wallet -> Advanced is absolute item 6. After paging the seven-item
        // Wallet menu to scroll=4, item 6 occupies visible row 2.
        let zone = self.list[2];
        self.menu_touch(TouchInput::new(zone.x + zone.w / 2, zone.y + zone.h / 2, false)) == Some(true)
            && self.ad.navigation.app.state == AppState::WalletAdvancedMenu
    }

    pub(super) fn open_backup_advanced_item(&mut self, item: usize, expected: AppState) -> bool {
        crate::runtime::effects::home(self.ad);
        if !crate::runtime::interactions::menu::handle_root_touch(self.ad, 82, 188)
            || self.ad.navigation.app.state != AppState::SeedsMenu
        {
            return false;
        }
        self.ad.navigation.production.wallet_menu.reset();
        if !crate::runtime::interactions::menu::primary::workflow_wallet_select(self.ad, 1)
            || self.ad.navigation.app.state != AppState::WalletBackupMethodsMenu
            || !crate::runtime::interactions::menu::primary::workflow_wallet_backup_methods_select(self.ad, 3)
            || self.ad.navigation.app.state != AppState::BackupRecoveryMenu
        {
            return false;
        }
        self.ad.navigation.production.backup_recovery_menu.reset();
        if item >= 4 && self.menu_touch(TouchInput::new(self.down.x + 20, self.down.y + 20, false)) != Some(true) {
            return false;
        }
        let zone = self.list[item % 4];
        self.menu_touch(TouchInput::new(zone.x + zone.w / 2, zone.y + zone.h / 2, false)) == Some(true)
            && self.ad.navigation.app.state == expected
    }

    pub(super) fn open_advanced_item(&mut self, item: usize, expected: AppState) -> bool {
        if !self.open_wallet_advanced() {
            return false;
        }
        self.ad.navigation.production.wallet_advanced_menu.reset();
        let zone = self.list[item];
        self.menu_touch(TouchInput::new(zone.x + zone.w / 2, zone.y + zone.h / 2, false)) == Some(true)
            && self.ad.navigation.app.state == expected
    }
}

pub(super) fn exercise(
    ad: &mut AppData,
    display: &mut BootDisplay<'_>,
    i2c: &mut I2c<'_, Blocking>,
    delay: &mut Delay,
) -> bool {
    let mut ctx = AdvancedToolsContext::new(ad, display, i2c, delay);
    log!("KASKOLD_WORKFLOW_TESTS: ADVANCED WALLET TOOLS TRANCHE BEGIN");
    let mut summary = super::probe_status::ProbeSummary::new("ADVANCED-TOOLS");

    summary.begin("BIP85");
    let bip85_ok = bip85::exercise(&mut ctx);
    summary.record("BIP85", bip85_ok);
    recover(&mut ctx, bip85_ok);

    summary.begin("EXPORTS");
    let exports_ok = exports::exercise(&mut ctx);
    summary.record("EXPORTS", exports_ok);
    recover(&mut ctx, exports_ok);

    summary.begin("MESSAGES");
    let messages_ok = messages::exercise(&mut ctx);
    summary.record("MESSAGES", messages_ok);
    recover(&mut ctx, messages_ok);

    summary.begin("SECRETS");
    let secrets_ok = secrets::exercise(&mut ctx);
    summary.record("SECRETS", secrets_ok);
    recover(&mut ctx, secrets_ok);

    summary.begin("FINISH-HOME");
    let finish_ok = finish(&mut ctx);
    summary.record("FINISH-HOME", finish_ok);
    summary.finish(5)
}

fn recover(ctx: &mut AdvancedToolsContext<'_, '_, '_>, probe_ok: bool) {
    if probe_ok {
        return;
    }
    crate::runtime::effects::home(ctx.ad);
    if !crate::services::wallet_session::install_workflow_backup_mnemonic_fixture(ctx.ad) {
        log!("KASKOLD_WORKFLOW_TESTS: ADVANCED-TOOLS PROBE RECOVERY FIXTURE FAILED");
    }
}

fn finish(ctx: &mut AdvancedToolsContext<'_, '_, '_>) -> bool {
    crate::runtime::effects::home(ctx.ad);
    if !super::root::home_ok(ctx.ad) {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: ADVANCED TOOLS PHYSICAL SD/RNG HIL DEFERRED TO PERIPHERAL TRANCHE");
    log!("KASKOLD_WORKFLOW_TESTS: ADVANCED WALLET TOOLS TRANCHE PASS");
    true
}
