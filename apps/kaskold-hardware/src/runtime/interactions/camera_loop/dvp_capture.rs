// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! ESP-HAL DVP capture path for the CoreS3 camera.

use super::decoder::{handle_decode_result, rqrr_decode};
use super::session::CycleBuffers;
use super::state::CameraSessionState;
use super::{display, AppData};
use crate::runtime::interactions::DeviceIo;
use crate::services::camera_device::{CameraPort, FrameCaptureStatus};

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CaptureOutcome {
    Captured = 0,
    TimedOut = 1,
    ReceiveFailed = 2,
    ResourcesUnavailable = 3,
}

fn render_and_copy_frame(
    session: &mut CameraSessionState,
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    data: &[u8],
    decode: &mut [u8],
    crop: &mut [u8],
) -> (usize, bool, bool) {
    session.frame.number = session.frame.number.wrapping_add(1);
    let bpl = platform_bytes_per_line();
    let full_h = (data.len() / bpl).min(240);
    let frame_ok = full_h >= 100;
    if frame_ok && !session.decoder.error_showing {
        render_dvp_view(session, ad, boot_display, data, crop, bpl, full_h);
    }
    let is_decode_frame = is_dvp_decode_frame(session);
    if is_decode_frame && frame_ok && !session.decoder.error_showing {
        copy_dvp_decode_frame(data, decode, bpl, full_h);
    }
    (full_h, frame_ok, is_decode_frame)
}

fn platform_bytes_per_line() -> usize {
    320
}

fn is_dvp_decode_frame(session: &CameraSessionState) -> bool {
    session.frame.number.is_multiple_of(2)
}

fn render_dvp_view(
    session: &CameraSessionState,
    ad: &AppData,
    boot_display: &mut display::BootDisplay<'_>,
    data: &[u8],
    crop: &mut [u8],
    bpl: usize,
    full_h: usize,
) {
    let render_w = 240usize;
    let render_h = 180usize;
    render_dvp_m5stack(data, crop, bpl, full_h, render_w, render_h);
    let crop_slice = &crop[..render_w * render_h];
    let guide_base = session.decoder.guide_version | if session.decoder.finders_beeped { 0x80 } else { 0 };
    let _ = ad;
    let guide = guide_base;
    boot_display.blit_camera_frame(crop_slice, render_w, render_h, guide);
}

fn render_dvp_m5stack(data: &[u8], crop: &mut [u8], bpl: usize, full_h: usize, render_w: usize, render_h: usize) {
    for cy in 0..render_h {
        let src_y = full_h - 1 - (30 + cy);
        for cx in 0..render_w {
            let idx = src_y * bpl + 40 + cx;
            crop[cy * render_w + cx] = data.get(idx).copied().unwrap_or(0);
        }
    }
}

fn copy_dvp_decode_frame(data: &[u8], decode: &mut [u8], bpl: usize, full_h: usize) {
    for dy in 0..full_h {
        let dst_off = dy * 320;
        for dx in 0..320usize {
            let idx = (full_h - 1 - dy) * bpl + dx;
            decode[dst_off + dx] = data.get(idx).copied().unwrap_or(0);
        }
    }
}

/// Render the captured frame and copy it into the decode buffer.
///
/// Returns the frame height when this frame should be QR-decoded.
unsafe fn render_captured_frame(
    session: &mut CameraSessionState,
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    buffers: &CycleBuffers,
    data: &[u8],
) -> Option<usize> {
    // SAFETY: the camera loop allocates independent decode (320x240) and
    // viewfinder crop (240x180) buffers for the lifetime of this capture.
    // Convert the raw storage once at the reviewed boundary; helpers below
    // operate only on bounded slices.
    let decode = core::slice::from_raw_parts_mut(buffers.db_ptr, 320 * 240);
    let crop = core::slice::from_raw_parts_mut(buffers.crop_ptr, 240 * 180);
    let (full_h, frame_ok, is_decode_frame) = render_and_copy_frame(session, ad, boot_display, data, decode, crop);
    if !frame_ok {
        return None;
    }

    // Handle error cooldown
    if session.decoder.error_showing {
        if session.decoder.cooldown > 0 {
            session.decoder.cooldown -= 1;
        } else {
            session.decoder.error_showing = false;
        }
        return None;
    }

    // Skip QR decode on display-only frames
    if !is_decode_frame {
        return None;
    }
    if session.decoder.cooldown > 0 {
        session.decoder.cooldown -= 1;
        return None;
    }
    Some(full_h)
}

/// Decode the centre 240x240 window of a rendered frame and route any QR result.
unsafe fn decode_captured_frame(
    session: &mut CameraSessionState,
    ad: &mut AppData,
    io: &mut DeviceIo<'_, '_, '_>,
    buffers: &CycleBuffers,
    full_h: usize,
) {
    // SAFETY: same reviewed decode buffer as `render_captured_frame`; the
    // camera DMA buffer has already been released, so this is the only view.
    let decode = core::slice::from_raw_parts_mut(buffers.db_ptr, 320 * 240);
    // m5stack: crop center 240x240 from 320x240 for rqrr.
    // Compact rows in-place in DB buffer.
    let rqw: usize = 240;
    let rqh: usize = full_h.min(240);
    let x0: usize = 40; // (320 - 240) / 2
    for ry in 0..rqh {
        let src = ry * 320 + x0;
        let dst = ry * rqw;
        if src != dst {
            decode.copy_within(src..src + rqw, dst);
        }
    }
    let crop_slice = &decode[..rqw * rqh];
    let results = rqrr_decode(crop_slice, rqw, rqh);
    if let Some((ver, ref decoded)) = results.first() {
        handle_decode_result(session, *ver, decoded, ad, io);
    } else {
        session.decoder.finders_beeped = false;
    }
}

pub(super) unsafe fn run_capture(
    session: &mut CameraSessionState,
    ad: &mut AppData,
    io: &mut DeviceIo<'_, '_, '_>,
    camera: &mut CameraPort<'_>,
    buffers: &CycleBuffers,
) -> CaptureOutcome {
    let mut decode_rows = None;
    let status = camera.capture(io.delay, |frame| {
        decode_rows = render_captured_frame(session, ad, io.boot_display, buffers, frame);
    });
    match status {
        None => CaptureOutcome::ResourcesUnavailable,
        Some(FrameCaptureStatus::Complete) => {
            if let Some(full_h) = decode_rows {
                decode_captured_frame(session, ad, io, buffers, full_h);
            }
            CaptureOutcome::Captured
        }
        Some(FrameCaptureStatus::TimedOut) => {
            log!("   dvp: transfer timeout — partial frame rejected");
            CaptureOutcome::TimedOut
        }
        Some(FrameCaptureStatus::ReceiveFailed) => {
            log!("   dvp: receive failed");
            CaptureOutcome::ReceiveFailed
        }
    }
}
