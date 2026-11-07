// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! QR image decoding and confirmed-result routing.

use super::dispatch::process_confirmed_qr;
use super::multiframe::{is_multiframe, process_multiframe};
use super::state::CameraSessionState;
use super::timing::systick;
use super::{sound, AppData, Vec};
use crate::runtime::interactions::DeviceIo;

/// Decode QR codes from a grayscale image using rqrr.
/// Returns `(version, raw_bytes)` for each detected QR.
#[inline(never)]
pub(super) fn rqrr_decode(gray: &[u8], w: usize, h: usize) -> Vec<(u8, Vec<u8>)> {
    let t0 = systick();
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| gray[y * w + x]);
    let t1 = systick();

    let grids = img.detect_grids();
    let t2 = systick();

    let prep_ms = t1.wrapping_sub(t0) / 16_000;
    let det_ms = t2.wrapping_sub(t1) / 16_000;
    log!("   [rqrr] {}x{} prep={}ms det={}ms grids={}", w, h, prep_ms, det_ms, grids.len());

    let mut results = Vec::new();
    for grid in grids {
        let mut out = Vec::new();
        match grid.decode_to(&mut out) {
            Ok(meta) => {
                log!("   [rqrr] decoded V{} {} bytes", meta.version.0, out.len());
                results.push((meta.version.0 as u8, out));
            }
            Err(e) => {
                log!("   [rqrr] decode err: {}", e);
            }
        }
    }
    results
}

/// Route one Reed-Solomon-verified QR result.
#[inline(never)]
pub(super) fn handle_decode_result(
    session: &mut CameraSessionState,
    ver: u8,
    decoded: &[u8],
    ad: &mut AppData,
    io: &mut DeviceIo<'_, '_, '_>,
) {
    let len = decoded.len();
    if !session.decoder.finders_beeped {
        sound::qr_found();
        session.decoder.finders_beeped = true;
    }

    if (1..=40).contains(&ver) {
        session.decoder.guide_version = ver;
    }

    if is_multiframe(decoded, len) {
        process_multiframe(session, decoded, ad, io);
        return;
    }

    if matches!(
        ad.navigation.app.state,
        crate::runtime::input::AppState::PassphraseChoice | crate::runtime::input::AppState::PassphraseEntry
    ) {
        return;
    }

    session.decoder.cooldown = 90;
    session.decoder.finders_beeped = false;
    log!("   rqrr QR OK: {} bytes (V{})", len, ver);
    if ad.signing.anti_klepto.phase == crate::runtime::data::AntiKleptoPhase::AwaitingReveal {
        io.boot_display.draw_loading_wait_screen("Finalizing signature...");
    } else {
        io.boot_display.draw_loading_screen("Processing QR...");
    }
    log!("   QR processing UI shown");
    process_confirmed_qr(decoded, ad, io);
}
