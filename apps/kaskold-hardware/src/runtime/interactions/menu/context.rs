// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Capabilities supplied by the firmware router to the menu façade.

use crate::{hw::display, hw::touch, runtime::data::AppData, runtime::interactions::TouchInput};
use esp_hal::{dma::DmaRxBuf, lcd_cam::cam::Camera as DvpCamera};

pub struct MenuTouchContext<'ctx, 'display, 'hal, 'camera> {
    pub ad: &'ctx mut AppData,
    pub boot_display: &'ctx mut display::BootDisplay<'display>,
    pub delay: &'ctx mut esp_hal::delay::Delay,
    pub liveness: &'ctx mut dyn FnMut(),
    pub i2c: &'ctx mut esp_hal::i2c::master::I2c<'hal, esp_hal::Blocking>,
    pub sd_card_type: &'ctx Option<crate::services::storage_device::SdCardType>,
    pub dvp_camera_opt: &'ctx mut Option<DvpCamera<'camera>>,
    pub cam_dma_buf_opt: &'ctx mut Option<DmaRxBuf>,
    pub list_zones: &'ctx [touch::TouchZone; 4],
    pub page_up_zone: &'ctx touch::TouchZone,
    pub page_down_zone: &'ctx touch::TouchZone,
    pub input: TouchInput,
}
