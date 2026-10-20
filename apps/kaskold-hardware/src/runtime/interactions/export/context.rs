// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Narrow capability contexts shared by export workflow handlers.

use crate::{
    hw::{display, touch},
    runtime::data::AppData,
    runtime::interactions::TouchInput,
    services::storage_device as sdcard,
};

/// Capabilities supplied by the firmware router to the export façade.
pub struct ExportTouchContext<'ctx, 'display, 'hal> {
    pub ad: &'ctx mut AppData,
    pub boot_display: &'ctx mut display::BootDisplay<'display>,
    pub delay: &'ctx mut esp_hal::delay::Delay,
    pub liveness: &'ctx mut dyn FnMut(),
    pub i2c: &'ctx mut esp_hal::i2c::master::I2c<'hal, esp_hal::Blocking>,
    pub sd_card_type: &'ctx Option<sdcard::SdCardType>,
    pub list_zones: &'ctx [touch::TouchZone; 4],
    pub page_up_zone: &'ctx touch::TouchZone,
    pub page_down_zone: &'ctx touch::TouchZone,
    pub input: TouchInput,
}

impl<'display, 'hal> ExportTouchContext<'_, 'display, 'hal> {
    /// Reborrow the SD-free capabilities for one export menu handler.
    pub(super) fn menu(&mut self) -> ExportMenuContext<'_, 'display> {
        ExportMenuContext {
            ad: &mut *self.ad,
            boot_display: &mut *self.boot_display,
            delay: &mut *self.delay,
            list_zones: self.list_zones,
            page_up_zone: self.page_up_zone,
            page_down_zone: self.page_down_zone,
            input: self.input,
        }
    }

    /// Reborrow the SD-capable capabilities for one export storage handler.
    pub(super) fn storage(&mut self) -> ExportStorageContext<'_, 'display, 'hal> {
        ExportStorageContext {
            ad: &mut *self.ad,
            boot_display: &mut *self.boot_display,
            delay: &mut *self.delay,
            liveness: &mut *self.liveness,
            i2c: &mut *self.i2c,
            sd_card_type: self.sd_card_type,
            list_zones: self.list_zones,
            page_up_zone: self.page_up_zone,
            page_down_zone: self.page_down_zone,
            input: self.input,
        }
    }
}

/// Capabilities for export menus that do not access the SD bus.
pub(super) struct ExportMenuContext<'ctx, 'display> {
    pub(super) ad: &'ctx mut AppData,
    pub(super) boot_display: &'ctx mut display::BootDisplay<'display>,
    pub(super) delay: &'ctx mut esp_hal::delay::Delay,
    pub(super) list_zones: &'ctx [touch::TouchZone; 4],
    pub(super) page_up_zone: &'ctx touch::TouchZone,
    pub(super) page_down_zone: &'ctx touch::TouchZone,
    pub(super) input: TouchInput,
}

/// Capabilities for export menus that may access the SD bus.
pub(super) struct ExportStorageContext<'ctx, 'display, 'hal> {
    pub(super) ad: &'ctx mut AppData,
    pub(super) boot_display: &'ctx mut display::BootDisplay<'display>,
    pub(super) delay: &'ctx mut esp_hal::delay::Delay,
    pub(super) liveness: &'ctx mut dyn FnMut(),
    pub(super) i2c: &'ctx mut esp_hal::i2c::master::I2c<'hal, esp_hal::Blocking>,
    pub(super) sd_card_type: &'ctx Option<sdcard::SdCardType>,
    pub(super) list_zones: &'ctx [touch::TouchZone; 4],
    pub(super) page_up_zone: &'ctx touch::TouchZone,
    pub(super) page_down_zone: &'ctx touch::TouchZone,
    pub(super) input: TouchInput,
}
