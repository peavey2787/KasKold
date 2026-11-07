//! ESP-HAL DVP acquisition adapter for camera entropy.

use esp_hal::delay::Delay;
use kaskold_hardware_core::entropy::frame_noise::CameraEntropyTracker;

use crate::hw::shared::dvp::{CameraPort, FrameCaptureStatus};

pub(super) fn mix_frame(
    pool: &mut [u8; 32],
    frame_tag: u8,
    idle_ticks: u32,
    delay: &mut Delay,
    camera: &mut CameraPort<'_>,
    tracker: &mut CameraEntropyTracker,
    #[cfg(feature = "e12-capture")] capture: Option<&mut crate::diagnostics::e12_capture::Capture>,
) -> bool {
    let status = camera.capture(delay, |frame| {
        super::mix_observation(
            pool,
            frame_tag,
            idle_ticks,
            frame,
            tracker,
            #[cfg(feature = "e12-capture")]
            capture,
        );
    });
    match status {
        Some(FrameCaptureStatus::Complete) => true,
        Some(FrameCaptureStatus::TimedOut) => {
            crate::log!("   Entropy DVP frame timed out; partial buffer rejected");
            false
        }
        Some(FrameCaptureStatus::ReceiveFailed) => {
            crate::log!("   Entropy DVP receive failed");
            false
        }
        None => false,
    }
}
