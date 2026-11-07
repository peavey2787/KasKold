// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Hardware-ownership boundary for the application event loop; focused touch,
//! dispatch, frame, and camera stages remain owned by their dedicated modules.

mod deferred;
mod hardware;
mod liveness;
#[cfg(feature = "m5stack")]
mod startup_audio;
mod startup_ui;
#[cfg(feature = "workflow-test-auto")]
mod workflow_auto;
#[cfg(not(feature = "workflow-test-auto"))]
pub(super) use deferred::service as service_deferred;
#[cfg(not(feature = "workflow-test-auto"))]
pub(super) use deferred::service_request as service_navigation_request;

pub(super) use deferred::cancel_operation as cancel_kpub_operation;
pub(super) use deferred::service_operation as service_kpub_operation;
#[cfg(all(feature = "m5stack", feature = "workflow-runtime-auto"))]
pub(crate) use deferred::workflow_drive_multisig_kpub;
#[cfg(all(feature = "m5stack", feature = "workflow-runtime-auto"))]
pub(crate) use deferred::{workflow_drive_address_cache, workflow_drive_connect_companion};
#[cfg(not(feature = "workflow-test-auto"))]
pub(super) use liveness::acknowledge;
#[cfg(all(feature = "m5stack", not(feature = "workflow-test-auto")))]
pub(crate) use liveness::acknowledge_runtime;
#[cfg(all(feature = "m5stack", feature = "workflow-runtime-auto"))]
pub(crate) use liveness::sync_watchdog_budget;

#[cfg(not(feature = "workflow-test-auto"))]
pub(crate) use hardware::InteractiveInput;
pub(crate) use hardware::RuntimeHardware;

use crate::runtime::data::AppData;

// LINT-JUSTIFICATION: Clippy scores four focused event-loop macro stages after expansion as one function even though touch, dispatch, frame, and camera policy remain separately owned modules.
#[allow(clippy::cognitive_complexity)]
pub(crate) fn run(
    ad: &mut AppData,
    hardware: RuntimeHardware<'_>,
    #[cfg(not(feature = "workflow-test-auto"))] input: InteractiveInput,
    #[cfg(feature = "m5stack")] watchdog_feed: impl FnMut(),
) -> ! {
    let RuntimeHardware { mut persistent_wallet, mut boot_display, delay, mut i2c, camera } = hardware;
    #[cfg(not(feature = "workflow-test-auto"))]
    let InteractiveInput {
        mut tracker,
        #[cfg(feature = "m5stack")]
        mut runtime_audio,
        zones,
    } = input;
    #[cfg(not(feature = "workflow-test-auto"))]
    let mut delay = delay;
    let startup = persistent_wallet.prepare_startup(ad);
    crate::runtime::interactions::persistence::apply_startup_navigation(ad, startup);
    #[cfg(feature = "m5stack")]
    startup_audio::apply_audio_preference(ad, &mut persistent_wallet);
    startup_ui::render(ad, &mut boot_display, &mut i2c);
    // Interactive loop alone owns the mutable watchdog acknowledgement.
    #[cfg(all(feature = "m5stack", not(feature = "workflow-test-auto")))]
    let mut watchdog_feed = watchdog_feed;
    #[cfg(all(feature = "m5stack", not(feature = "workflow-test-auto")))]
    acknowledge_runtime(ad, &mut watchdog_feed);
    #[cfg(feature = "workflow-test-auto")]
    workflow_auto::run(ad, boot_display, i2c, delay, camera, persistent_wallet, watchdog_feed);
    #[cfg(not(feature = "workflow-test-auto"))]
    {
        let mut camera = camera;
        let (grid_zones, list_zones, page_up_zone, page_down_zone) = zones;
        super::run!(
            ad,
            persistent_wallet,
            boot_display,
            delay,
            i2c,
            camera,
            tracker,
            runtime_audio,
            watchdog_feed,
            grid_zones,
            list_zones,
            page_up_zone,
            page_down_zone
        );
    }
}
