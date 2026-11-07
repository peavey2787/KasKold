// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Stable platform-adaptive camera capture façade.

use super::dvp_capture::CaptureOutcome;
use super::session::prepare_cycle;
use super::state::CameraSessionState;
use super::{camera, AppData};
use crate::runtime::data::CameraScanFault;
use crate::runtime::interactions::DeviceIo;
use crate::services::camera_device::CameraPort;

const CAPTURE_OUTCOME_FAULT: [CameraScanFault; 4] = [
    CameraScanFault::None,
    CameraScanFault::CaptureTimeout,
    CameraScanFault::CaptureFailed,
    CameraScanFault::RuntimeResources,
];

fn fault_error(fault: CameraScanFault) -> crate::runtime::presentation::ErrorSpec {
    match fault {
        CameraScanFault::None => crate::runtime::presentation::CAMERA_CAPTURE,
        CameraScanFault::MemoryUnavailable => crate::runtime::presentation::CAMERA_MEMORY,
        CameraScanFault::StartupUnavailable | CameraScanFault::RuntimeResources => {
            crate::runtime::presentation::CAMERA_UNAVAILABLE
        }
        CameraScanFault::CaptureTimeout | CameraScanFault::CaptureFailed => {
            crate::runtime::presentation::CAMERA_CAPTURE
        }
    }
}

fn latch_fault(ad: &mut AppData, fault: CameraScanFault) {
    ad.qr.scan.latch_camera_fault(fault);
    log!("   CAMERA fault {:?}", fault);
    let return_to = crate::runtime::presentation::previous_stable_screen(ad);
    crate::runtime::presentation::show_error_spec_to(ad, return_to, fault_error(fault));
}

fn m5_preflight_fault(status: camera::CameraStatus, stream_ready: bool) -> CameraScanFault {
    if status == camera::CameraStatus::Error {
        return CameraScanFault::StartupUnavailable;
    }
    if !stream_ready {
        return CameraScanFault::RuntimeResources;
    }
    CameraScanFault::None
}

fn handle_m5_outcome(session: &mut CameraSessionState, ad: &mut AppData, outcome: CaptureOutcome) {
    let fault = CAPTURE_OUTCOME_FAULT[outcome as usize];
    if !fault.is_fault() {
        session.capture_succeeded();
        return;
    }
    if session.capture_failed() {
        latch_fault(ad, fault);
    }
}

fn camera_entry_reset(session: &mut CameraSessionState, ad: &mut AppData) -> bool {
    let entered = session.enter_screen(ad.navigation.app.state);
    if entered {
        ad.qr.scan.begin_camera_entry();
        log!("   CAMERA session entry: {:?}", ad.navigation.app.state);
    }
    ad.qr.scan.take_camera_reset_request()
}

/// Run one camera capture and QR-decode cycle.
#[inline(never)]
pub fn run_camera_cycle(
    session: &mut CameraSessionState,
    ad: &mut AppData,
    io: &mut DeviceIo<'_, '_, '_>,
    camera: &mut CameraPort<'_>,
) {
    let reset_entry = camera_entry_reset(session, ad);
    if ad.qr.scan.has_camera_fault() {
        return;
    }

    let fault = m5_preflight_fault(camera.status, camera.is_ready());
    if fault.is_fault() {
        latch_fault(ad, fault);
        return;
    }

    let buffers = {
        let mut platform = super::session::CyclePlatform::new(ad);
        prepare_cycle(session, &mut platform, io.boot_display, &mut camera.status, reset_entry)
    };
    let Some(buffers) = buffers else {
        latch_fault(ad, CameraScanFault::MemoryUnavailable);
        return;
    };

    unsafe {
        let outcome = super::dvp_capture::run_capture(session, ad, io, camera, &buffers);
        handle_m5_outcome(session, ad, outcome);
    }
}
