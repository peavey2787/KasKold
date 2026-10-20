//! Touch context shared by the connected tranches that drive the production
//! menu, transaction, and export controllers through the home touch zones.

use crate::{
    hw::{display::BootDisplay, sdcard::SdCardType, touch::TouchZone},
    runtime::data::AppData,
    runtime::interactions::{export::ExportTouchContext, tx::TxTouchContext, TouchInput},
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

pub(super) struct MenuWorkflowContext<'ctx, 'display, 'hal> {
    pub(super) ad: &'ctx mut AppData,
    pub(super) display: &'ctx mut BootDisplay<'display>,
    pub(super) i2c: &'ctx mut I2c<'hal, Blocking>,
    pub(super) sd: &'ctx Option<SdCardType>,
    pub(super) delay: &'ctx mut Delay,
    pub(super) grid: [TouchZone; 4],
    pub(super) list: [TouchZone; 4],
    pub(super) up: TouchZone,
    pub(super) down: TouchZone,
}

impl<'ctx, 'display, 'hal> MenuWorkflowContext<'ctx, 'display, 'hal> {
    pub(super) fn new(
        ad: &'ctx mut AppData,
        display: &'ctx mut BootDisplay<'display>,
        i2c: &'ctx mut I2c<'hal, Blocking>,
        sd: &'ctx Option<SdCardType>,
        delay: &'ctx mut Delay,
    ) -> Self {
        let (grid, list, up, down) = crate::runtime::touch_dispatch::touch_zones();
        Self { ad, display, i2c, sd, delay, grid, list, up, down }
    }
}

impl MenuWorkflowContext<'_, '_, '_> {
    pub(super) fn redraw_step(&mut self) {
        super::redraw_step(self.ad, self.display, self.i2c, self.sd);
        super::show_step(self.delay);
    }

    pub(super) fn menu_touch(&mut self, x: u16, y: u16, is_back: bool) -> Option<bool> {
        crate::runtime::interactions::menu::handle_navigation_touch(
            self.ad,
            &self.grid,
            &self.list,
            &self.up,
            &self.down,
            TouchInput::new(x, y, is_back),
        )
    }

    pub(super) fn tx_touch(&mut self, x: u16, y: u16, is_back: bool) -> Option<bool> {
        let sd = self.sd;
        self.tx_touch_with(sd, TouchInput::new(x, y, is_back))
    }

    pub(super) fn tx_touch_without_sd(&mut self, x: u16, y: u16, is_back: bool) -> Option<bool> {
        self.tx_touch_with(&None, TouchInput::new(x, y, is_back))
    }

    fn tx_touch_with(&mut self, sd: &Option<SdCardType>, input: TouchInput) -> Option<bool> {
        crate::runtime::interactions::tx::handle_tx_touch(TxTouchContext {
            ad: self.ad,
            boot_display: self.display,
            delay: self.delay,
            liveness: &mut || {},
            i2c: self.i2c,
            persistent_wallet: None,
            sd_card_type: sd,
            list_zones: &self.list,
            input,
        })
    }

    /// Touch the export controller directly, as the multisig export screens see it.
    pub(super) fn export_screen_touch(&mut self, x: u16, y: u16, is_back: bool) -> Option<bool> {
        let sd = self.sd;
        self.export_touch_with(sd, TouchInput::new(x, y, is_back))
    }

    /// Touch the export menus first, then the export controller, as the
    /// single-signature advanced-tool screens see it.
    pub(super) fn export_touch(&mut self, x: u16, y: u16, is_back: bool) -> Option<bool> {
        let sd = self.sd;
        self.export_menu_touch_with(sd, TouchInput::new(x, y, is_back))
    }

    pub(super) fn export_touch_without_sd(&mut self, x: u16, y: u16, is_back: bool) -> Option<bool> {
        self.export_menu_touch_with(&None, TouchInput::new(x, y, is_back))
    }

    fn export_menu_touch_with(&mut self, sd: &Option<SdCardType>, input: TouchInput) -> Option<bool> {
        crate::runtime::interactions::export::menus::handle_navigation_touch(
            self.ad, &self.list, &self.up, &self.down, input,
        )
        .or_else(|| self.export_touch_with(sd, input))
    }

    fn export_touch_with(&mut self, sd: &Option<SdCardType>, input: TouchInput) -> Option<bool> {
        crate::runtime::interactions::export::handle_export_touch(ExportTouchContext {
            ad: self.ad,
            boot_display: self.display,
            delay: self.delay,
            liveness: &mut || {},
            i2c: self.i2c,
            sd_card_type: sd,
            list_zones: &self.list,
            page_up_zone: &self.up,
            page_down_zone: &self.down,
            input,
        })
    }
}
