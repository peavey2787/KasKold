//! Touch transcript timing.

use super::platform;

/// Latched hardware timer observation used by the touch transcript.
pub fn touch_timestamp() -> u32 {
    platform::systimer_low()
}
