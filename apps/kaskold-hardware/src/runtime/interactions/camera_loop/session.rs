// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! CoreS3 camera-session allocation, reset, and first-stream initialization.
//!
//! The camera loop owns scanner entry rendering and camera lifecycle. Generic
//! screen redraw code must not manipulate LCD_CAM or camera-session state.

use super::{camera, display, AppData};
use super::state::CameraSessionState;

pub(super) struct CyclePlatform {
    show_home: bool,
    anti_klepto_reveal: bool,
}

impl CyclePlatform {
    pub(super) fn new(ad: &AppData) -> Self {
        Self {
            show_home: crate::runtime::navigation::home_shortcut_visible(ad),
            anti_klepto_reveal: ad.signing.anti_klepto.phase
                == crate::runtime::data::AntiKleptoPhase::AwaitingReveal,
        }
    }

    fn draw_entry(&mut self, display: &mut display::BootDisplay<'_>) {
        if self.anti_klepto_reveal {
            display.draw_anti_klepto_reveal_camera_screen();
        } else {
            display.draw_camera_screen();
        }
        if self.show_home {
            display.draw_home_button();
        }
    }
}

pub(super) struct CycleBuffers {
    pub(super) db_ptr: *mut u8,
    pub(super) crop_ptr: *mut u8,
}

pub(super) fn prepare_cycle(
    session: &mut CameraSessionState,
    platform: &mut CyclePlatform,
    boot_display: &mut display::BootDisplay<'_>,
    cam_status: &mut camera::CameraStatus,
    reset_entry: bool,
) -> Option<CycleBuffers> {
    if !session.ensure_frame_buffers() {
        log!("   CAMERA memory allocation failed");
        return None;
    }

    let db_ptr = session.frame.db_ptr();
    let crop_ptr = session.frame.crop_ptr();
    if session.frame.number == 0 {
        log!("   CAMERA buffers ready: decode=76KB crop=43KB");
    }

    if reset_entry {
        session.reset_scan();
        log!("   CAMERA entry UI BEGIN");
        platform.draw_entry(boot_display);
        log!("   CAMERA entry UI DONE");
    }

    if *cam_status == camera::CameraStatus::SensorReady {
        *cam_status = camera::CameraStatus::Streaming;
        log!("   CAMERA streaming: QVGA Y-only 320x240 / rqrr");
    }

    Some(CycleBuffers { db_ptr, crop_ptr })
}
