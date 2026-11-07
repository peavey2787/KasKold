// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Camera activation, sensor tuning, and power-down. Touch stays event-loop owned.

#[cfg(not(feature = "workflow-test-auto"))]
pub(crate) use crate::runtime::input::is_scan_state;

#[cfg(not(feature = "workflow-test-auto"))]
macro_rules! run_step {
    ($ad:ident, $boot_display:ident, $delay:ident, $i2c:ident,
     $camera_session:ident, $camera:ident, $watchdog_feed:ident) => {
        // ─── Camera loop ─────────────────────────────────────────
        let camera_screen_active = matches!(
            $ad.navigation.app.state,
            $crate::runtime::input::AppState::ScanQR
                | $crate::runtime::input::AppState::DecryptSecretScan
                | $crate::runtime::input::AppState::SignMsgScan
        );
        // A stage-3 modal/operation owns the LCD and touch surface. Continuing
        // capture underneath it immediately paints live camera frames over the
        // error/loading UI and makes the user appear trapped in the scanner.
        let camera_active = camera_screen_active
            && !$crate::runtime::presentation::blocks_input($ad);

        if camera_active {
            // Always enter the camera controller while on a camera screen. It
            // owns preflight failure rendering as well as healthy capture.
            let mut camera_io = $crate::runtime::interactions::DeviceIo {
                boot_display: &mut $boot_display,
                delay: &mut $delay,
                i2c: &mut $i2c,
                liveness: &mut $watchdog_feed,
            };
            $crate::runtime::interactions::camera_loop::run_camera_cycle(
                &mut $camera_session, $ad, &mut camera_io, &mut $camera,
            );

        } else {
            $camera_session.leave_screen();
        }
    };
}

#[cfg(not(feature = "workflow-test-auto"))]
pub(crate) use run_step;
