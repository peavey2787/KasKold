use crate::{
    hw::{display::BootDisplay, touch::TouchZone},
    runtime::interactions::{sd::SdTouchContext, stego::StegoTouchContext, TouchInput},
    runtime::{data::AppData, input::AppState},
    services::backup::{BackupDevice, BackupError},
};
use aes_gcm::{
    aead::{generic_array::GenericArray, AeadInPlace, KeyInit},
    Aes256Gcm,
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};
use offline_signer::crypto::device_bound_storage::{Envelope, StoragePurpose, TAG_SIZE};

mod recovery_words;
mod seedqr;

/// Enter the Wallet menu from a clean Home through production controllers.
pub(super) fn enter_wallet_menu(ad: &mut AppData) -> bool {
    if !super::reset_tranche_to_home(ad) {
        return false;
    }
    let wallet = crate::ui::layout::HOME_GRID_ZONES[2];
    crate::runtime::interactions::menu::handle_connected_root_probe(
        ad,
        wallet.x + wallet.w / 2,
        wallet.y + wallet.h / 2,
    ) && ad.navigation.app.state == AppState::SeedsMenu
}

/// Enter Wallet -> Backup -> Advanced Backup through production controllers.
/// Connected SD and stego probes share this bounded setup so they cannot drift
/// into manufacturing Storage/Stego ownership with direct route injection.
pub(super) fn enter_advanced_backup(ad: &mut AppData) -> bool {
    enter_wallet_menu(ad)
        && crate::runtime::interactions::menu::primary::workflow_wallet_select(ad, 1)
        && ad.navigation.app.state == AppState::WalletBackupMethodsMenu
        && crate::runtime::interactions::menu::primary::workflow_wallet_backup_methods_select(ad, 3)
        && ad.navigation.app.state == AppState::BackupRecoveryMenu
        && crate::runtime::navigation::reconcile(ad)
}

pub(super) struct WorkflowBackupDevice;

impl BackupDevice for WorkflowBackupDevice {
    fn seal_backup_key(
        &mut self,
        purpose: StoragePurpose,
        envelope: Envelope<'_>,
        ciphertext: &mut [u8],
    ) -> Result<[u8; TAG_SIZE], BackupError> {
        core::hint::black_box(purpose);
        core::hint::black_box(envelope.salt);
        let cipher = Aes256Gcm::new(GenericArray::from_slice(envelope.stretched_credential));
        let tag = cipher
            .encrypt_in_place_detached(
                GenericArray::from_slice(envelope.nonce),
                envelope.authenticated_header,
                ciphertext,
            )
            .map_err(|_| BackupError::EncryptionFailed)?;
        let mut out = [0u8; TAG_SIZE];
        out.copy_from_slice(&tag);
        Ok(out)
    }

    fn open_backup_key(
        &mut self,
        purpose: StoragePurpose,
        envelope: Envelope<'_>,
        ciphertext: &mut [u8],
        tag: &[u8; TAG_SIZE],
    ) -> Result<(), BackupError> {
        core::hint::black_box(purpose);
        core::hint::black_box(envelope.salt);
        let cipher = Aes256Gcm::new(GenericArray::from_slice(envelope.stretched_credential));
        cipher
            .decrypt_in_place_detached(
                GenericArray::from_slice(envelope.nonce),
                envelope.authenticated_header,
                ciphertext,
                GenericArray::from_slice(tag),
            )
            .map_err(|_| BackupError::AuthenticationFailed)
    }
}

struct BackupContext<'ctx, 'display, 'hal> {
    ad: &'ctx mut AppData,
    display: &'ctx mut BootDisplay<'display>,
    i2c: &'ctx mut I2c<'hal, Blocking>,
    delay: &'ctx mut Delay,
    list: [TouchZone; 4],
    up: TouchZone,
    down: TouchZone,
    backup_device: WorkflowBackupDevice,
}

impl BackupContext<'_, '_, '_> {
    fn redraw(&mut self) {
        super::redraw_step(self.ad, self.display, self.i2c);
    }

    fn show_step(&mut self) {
        super::show_step(self.delay);
    }

    fn menu_select(&mut self, item: usize) -> bool {
        let menu = match self.ad.navigation.app.state {
            AppState::WalletBackupMethodsMenu => &self.ad.navigation.production.wallet_backup_methods_menu,
            AppState::BackupRecoveryMenu => &self.ad.navigation.production.backup_recovery_menu,
            _ => return false,
        };
        let zone = self.list[item % self.list.len()];
        let selected = crate::runtime::interactions::menu_selection::selected_visible_item(
            menu,
            &self.list,
            zone.x + zone.w / 2,
            zone.y + zone.h / 2,
        );
        if selected != u8::try_from(item).ok() {
            return false;
        }
        match self.ad.navigation.app.state {
            AppState::WalletBackupMethodsMenu => {
                crate::runtime::interactions::menu::primary::workflow_wallet_backup_methods_select(self.ad, item)
            }
            AppState::BackupRecoveryMenu => {
                crate::runtime::interactions::menu::primary::workflow_backup_recovery_select(self.ad, item)
            }
            _ => false,
        }
    }

    fn menu_back(&mut self) -> bool {
        crate::runtime::navigation::handle_back(self.ad)
    }

    fn export_touch(&mut self, input: TouchInput) -> Option<bool> {
        crate::runtime::interactions::export::handle_export_touch(
            crate::runtime::interactions::export::ExportTouchContext {
                ad: &mut *self.ad,
                boot_display: &mut *self.display,
                delay: &mut *self.delay,
                liveness: &mut || {},
                i2c: &mut *self.i2c,
                list_zones: &self.list,
                page_up_zone: &self.up,
                page_down_zone: &self.down,
                input,
            },
        )
    }

    fn sd_touch(&mut self, input: TouchInput) -> Option<bool> {
        crate::runtime::interactions::sd::handle_sd_touch(SdTouchContext {
            ad: &mut *self.ad,
            boot_display: &mut *self.display,
            delay: &mut *self.delay,
            liveness: &mut || {},
            i2c: &mut *self.i2c,
            backup_device: &mut self.backup_device,
            list_zones: &self.list,
            page_up_zone: &self.up,
            page_down_zone: &self.down,
            input,
        })
    }

    fn stego_back(&mut self) -> Option<bool> {
        crate::runtime::interactions::stego::handle_stego_touch(StegoTouchContext {
            ad: &mut *self.ad,
            boot_display: &mut *self.display,
            delay: &mut *self.delay,
            liveness: &mut || {},
            i2c: &mut *self.i2c,
            backup_device: &mut self.backup_device,
            list_zones: &self.list,
            page_up_zone: &self.up,
            page_down_zone: &self.down,
            input: TouchInput::new(20, 20, true),
        })
    }
}

pub(super) fn exercise(
    ad: &mut AppData,
    display: &mut BootDisplay<'_>,
    i2c: &mut I2c<'_, Blocking>,
    delay: &mut Delay,
) -> bool {
    if !crate::services::wallet_session::install_workflow_backup_mnemonic_fixture(ad) {
        log!("KASKOLD_WORKFLOW_TESTS: BACKUP MNEMONIC FIXTURE FAIL");
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP MNEMONIC FIXTURE READY 12");
    if !super::reset_tranche_to_home(ad) {
        log!("KASKOLD_WORKFLOW_TESTS: BACKUP HOME RESET FAIL");
        return false;
    }
    let (_, list, up, down) = crate::runtime::touch_dispatch::touch_zones();
    let mut ctx = BackupContext { ad, display, i2c, delay, list, up, down, backup_device: WorkflowBackupDevice };
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP/RECOVERY TRANCHE BEGIN");

    enter_backup_menu(&mut ctx)
        && run_stage(&mut ctx, "BACKUP MENU NOOP", backup_menu_noop)
        && run_stage(&mut ctx, "BACKUP/RECOVERY SPLIT", backup_recovery_split)
        && run_stage(&mut ctx, "BACKUP RECOVERY WORDS", recovery_words)
        && run_stage(&mut ctx, "BACKUP STANDARD SEEDQR", standard_seedqr)
        && run_stage(&mut ctx, "BACKUP ENCRYPTED SD", encrypted_sd_entry)
        && run_stage(&mut ctx, "BACKUP ADVANCED", advanced_backup_routes)
        && run_stage(&mut ctx, "BACKUP RAW-KEY REJECTION", raw_key_rejections)
        && run_stage(&mut ctx, "BACKUP FINISH HOME", finish_home)
}

fn run_stage(
    ctx: &mut BackupContext<'_, '_, '_>,
    name: &str,
    stage: fn(&mut BackupContext<'_, '_, '_>) -> bool,
) -> bool {
    log!("KASKOLD_WORKFLOW_TESTS: {} BEGIN", name);
    if !stage(ctx) {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: {} OK", name);
    true
}

fn enter_backup_menu(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !super::root::home_ok(ctx.ad) {
        return false;
    }
    let wallet = crate::ui::layout::HOME_GRID_ZONES[2];
    if !crate::runtime::interactions::menu::handle_connected_root_probe(
        ctx.ad,
        wallet.x + wallet.w / 2,
        wallet.y + wallet.h / 2,
    ) || ctx.ad.navigation.app.state != AppState::SeedsMenu
    {
        return false;
    }
    ctx.ad.navigation.production.wallet_menu.reset();
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP MENU ENTRY BEGIN");
    if !crate::runtime::interactions::menu::primary::workflow_wallet_select(ctx.ad, 1)
        || ctx.ad.navigation.app.state != AppState::WalletBackupMethodsMenu
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP MENU ENTRY OK");
    true
}

fn backup_menu_noop(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    let untouched = crate::runtime::interactions::menu_selection::selected_visible_item(
        &ctx.ad.navigation.production.wallet_backup_methods_menu,
        &ctx.list,
        320,
        235,
    )
    .is_none()
        && ctx.ad.navigation.app.state == AppState::WalletBackupMethodsMenu;
    if untouched {
        log!("KASKOLD_WORKFLOW_TESTS: BACKUP MENU OUTSIDE-ITEM NOOP OK");
    }
    untouched
}

fn backup_recovery_split(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !crate::runtime::navigation::handle_back(ctx.ad)
        || ctx.ad.navigation.app.state != AppState::SeedsMenu
        || !crate::runtime::interactions::menu::primary::workflow_wallet_select(ctx.ad, 2)
        || ctx.ad.navigation.app.state != AppState::SdImportMenu
    {
        return false;
    }
    if ctx.sd_touch(TouchInput::new(20, 20, true)) != Some(true) || ctx.ad.navigation.app.state != AppState::SeedsMenu {
        return false;
    }
    crate::runtime::interactions::menu::primary::workflow_wallet_select(ctx.ad, 1)
        && ctx.ad.navigation.app.state == AppState::WalletBackupMethodsMenu
}

fn recovery_words(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    let ok = recovery_words::exercise(ctx);
    if ok {
        log!("KASKOLD_WORKFLOW_TESTS: BACKUP RECOVERY WORDS 12/12 + BOUNDARIES OK");
    }
    ok
}

fn standard_seedqr(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    let ok = seedqr::standard(ctx);
    if ok {
        log!("KASKOLD_WORKFLOW_TESTS: BACKUP STANDARD SEEDQR GRID BOUNDARIES OK");
    }
    ok
}

fn encrypted_sd_entry(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !ctx.menu_select(2) || ctx.ad.navigation.app.state != AppState::SdBackupWarning {
        return false;
    }
    ctx.redraw();
    if ctx.sd_touch(TouchInput::new(20, 20, true)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::WalletBackupMethodsMenu
    {
        return false;
    }
    if !ctx.menu_select(2)
        || !continue_seed_backup_without_scope_leak(ctx)
        || ctx.ad.navigation.app.state != AppState::SdSeedFilename
        || ctx.sd_touch(TouchInput::new(20, 20, true)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::WalletBackupMethodsMenu
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP ENCRYPTED-SD CARD ENTRY/BACK OK");
    true
}

/// A HIL run on its QA card continues through the real card; other runs prepare the filename directly.
fn continue_seed_backup_without_scope_leak(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if crate::services::storage_device::workflow_view::physical_media() {
        let (x, y) = crate::ui::layout::zone_center(crate::ui::layout::MODAL_LEFT_BUTTON_ZONE);
        return crate::services::storage_device::known_card().is_some()
            && ctx.sd_touch(TouchInput::new(x, y, false)) == Some(true);
    }
    crate::runtime::interactions::sd::workflow_prepare_seed_backup_filename(ctx.ad)
}

fn advanced_backup_routes(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !ctx.menu_select(3) || ctx.ad.navigation.app.state != AppState::BackupRecoveryMenu {
        return false;
    }
    ctx.redraw();
    if !advanced_compact(ctx) || !advanced_plain(ctx) || !advanced_stego(ctx) || !advanced_xprv(ctx) {
        return false;
    }
    if !ctx.menu_back() || ctx.ad.navigation.app.state != AppState::WalletBackupMethodsMenu {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP ADVANCED ROUTES/BACK OWNERS OK");
    true
}

fn advanced_compact(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !ctx.menu_select(0) || ctx.ad.navigation.app.state != AppState::ExportCompactSeedQR {
        return false;
    }
    ctx.redraw();
    if ctx.export_touch(TouchInput::new(20, 20, true)) != Some(true)
        || ctx.ad.navigation.app.state != AppState::BackupRecoveryMenu
    {
        return false;
    }
    if !ctx.menu_select(0)
        || ctx.export_touch(TouchInput::new(160, 120, false)) != Some(true)
        || ctx.ad.navigation.app.state != (AppState::SeedQrGrid { pan_x: 0, pan_y: 0, compact: true })
    {
        return false;
    }
    ctx.export_touch(TouchInput::new(20, 20, true)) == Some(true)
        && ctx.ad.navigation.app.state == AppState::BackupRecoveryMenu
}

fn advanced_plain(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !ctx.menu_select(1) || ctx.ad.navigation.app.state != AppState::ExportPlainWordsQR {
        return false;
    }
    ctx.redraw();
    ctx.export_touch(TouchInput::new(160, 120, false)) == Some(true)
        && ctx.ad.navigation.app.state == AppState::BackupRecoveryMenu
}

fn advanced_stego(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP ADVANCED STEGO ENTRY BEGIN");
    if !ctx.menu_select(2) || ctx.ad.navigation.app.state != AppState::StegoModeSelect {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP ADVANCED STEGO ENTRY OK");
    ctx.redraw();
    if ctx.stego_back() != Some(true) || ctx.ad.navigation.app.state != AppState::BackupRecoveryMenu {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP ADVANCED STEGO BACK OK");
    true
}

fn advanced_xprv(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if !ctx.menu_select(3) || ctx.ad.navigation.app.state != AppState::XprvExportMenu {
        return false;
    }
    ctx.export_touch(TouchInput::new(20, 20, true)) == Some(true)
        && ctx.ad.navigation.app.state == AppState::BackupRecoveryMenu
}

fn raw_key_rejections(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    if ctx.ad.navigation.app.state != AppState::WalletBackupMethodsMenu
        || !crate::services::wallet_session::install_workflow_wallet_inventory_fixture(ctx.ad)
    {
        return false;
    }
    for item in 0usize..=2 {
        ctx.ad.navigation.production.wallet_backup_methods_menu.reset();
        if !ctx.menu_select(item) || ctx.ad.navigation.app.state != AppState::WalletBackupMethodsMenu {
            return false;
        }
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP RAW-KEY MNEMONIC ROUTES REJECTED");
    true
}

fn finish_home(ctx: &mut BackupContext<'_, '_, '_>) -> bool {
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP FINISH HOME WALLET-BACK BEGIN");
    if !crate::runtime::navigation::handle_back(ctx.ad) || ctx.ad.navigation.app.state != AppState::SeedsMenu {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP FINISH HOME WALLET-BACK OK");
    if !crate::runtime::navigation::handle_back(ctx.ad) || !super::root::home_ok(ctx.ad) {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: BACKUP/RECOVERY TRANCHE PASS");
    true
}
