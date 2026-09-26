// KasKold — Air-gapped offline signing device for Kaspa
//! CoreS3 touch polling, dim suppression, wake handling, and idle policy.

#[cfg(not(feature = "workflow-test-auto"))]
macro_rules! prepare_iteration {
    ($touch_state:ident, $action:ident, $ad:ident, $delay:ident, $i2c:ident,
     $tracker:ident, $wake_debounce:ident, $dim_active:ident, $watchdog_feed:ident) => {
        #[cfg(feature = "mirror")]
        $crate::hw::screenshot::pump_rows();

        let ($touch_state, $action) = {
            let state = $ad.navigation.app.state;
            let release_was_required = $tracker.release_required();
            match $crate::runtime::touch_service::read_checked(&mut $i2c) {
                Ok(ts) => {
                    let act = $tracker.update(ts);
                    if release_was_required && !$tracker.release_required() {
                        $crate::log!("   TOUCH CoreS3 release barrier cleared: {:?}", state);
                    }
                    if !matches!(act, $crate::hw::touch::TouchAction::None) {
                        $crate::log!("   TOUCH CoreS3 {:?} sample {:?} -> {:?}", state, ts, act);
                    }
                    (ts, act)
                }
                Err(()) => {
                    // A transport failure is not a physical finger release.
                    // Preserve the contact gate and fail this poll closed.
                    $crate::log!("   TOUCH CoreS3 {:?} I2C read failed — gate preserved", state);
                    ($crate::hw::touch::TouchState::NoTouch, $crate::hw::touch::TouchAction::None)
                }
            }
        };

        $ad.runtime.idle_ticks = $ad.runtime.idle_ticks.saturating_add(1);
        let is_touch = !matches!($action, $crate::hw::touch::TouchAction::None);
        let raw_touch = !matches!($touch_state, $crate::hw::touch::TouchState::NoTouch);

        if $ad.runtime.display_asleep {
            if $crate::runtime::power_state::handle_wake(
                $ad, &mut $i2c, &mut $delay, &mut $tracker,
                &mut $wake_debounce, raw_touch || is_touch,
            ) {
                $dim_active = false;
                $crate::runtime::event_loop::runner::acknowledge(&mut $watchdog_feed);
                continue;
            }
            $delay.delay_millis(100);
            $crate::runtime::event_loop::runner::acknowledge(&mut $watchdog_feed);
            continue;
        }

        // Movement gating and domain separation live inside the entropy service;
        // this observation is supplemental and never bypasses the checked TRNG.
        if let $crate::hw::touch::TouchState::One(point) = $touch_state {
            $crate::services::entropy::stage_ambient_touch(point.x, point.y);
        }
        if raw_touch || is_touch {
            $ad.runtime.idle_ticks = 0;
            if $dim_active {
                $crate::log!("   TOUCH CoreS3 dim wake BEGIN");
                let wake_brightness = $crate::runtime::power_state::effective_brightness($ad);
                $crate::hw::pmu::set_brightness!(&mut $i2c, wake_brightness);
                $dim_active = false;
                if !matches!(
                    $ad.navigation.app.state,
                    $crate::runtime::input::AppState::ScanQR
                        | $crate::runtime::input::AppState::DecryptSecretScan
                        | $crate::runtime::input::AppState::SignMsgScan
                ) {
                    $crate::hw::sound::click();
                }
                // The current contact is wake-only. Require release before a
                // new action rather than introducing a time-based debounce.
                $tracker.require_release();
                $wake_debounce = 0;
                $crate::log!("   TOUCH CoreS3 dim wake DONE — release gate armed");
                $crate::runtime::event_loop::runner::acknowledge(&mut $watchdog_feed);
                continue;
            }
        }

        $crate::runtime::power_state::handle_idle($ad, &mut $i2c, &mut $dim_active);
    };
}

#[cfg(not(feature = "workflow-test-auto"))]
pub(crate) use prepare_iteration;
