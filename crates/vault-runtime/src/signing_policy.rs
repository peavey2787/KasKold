//! Portable session signing-policy enforcement shared by every software Vault.
//!
//! The physical M5 additionally authenticates policy state against its hardware
//! RTC/device storage. Software Vaults intentionally expose this only as a
//! session policy because host clocks are mutable and are not equivalent to
//! the hardware trust boundary.

use shared_signer::advanced_policy::{
    parse_utc_yyyymmddhhmm, parse_weekly_windows, SigningDecision, SigningPolicy,
};

use super::{VaultRuntime, VaultRuntimeError};

impl VaultRuntime {
    pub fn set_session_signing_policy(
        &mut self,
        not_before_utc: &str,
        weekly_windows: &str,
    ) -> Result<(), VaultRuntimeError> {
        let not_before_unix = if not_before_utc.trim().is_empty() {
            0
        } else {
            parse_utc_yyyymmddhhmm(not_before_utc.trim().as_bytes())
                .and_then(|value| value.to_unix_seconds())
                .map_err(|_| VaultRuntimeError::InvalidSigningPolicy)?
        };
        let (windows, weekly_count) = if weekly_windows.trim().is_empty() {
            ([shared_signer::advanced_policy::SigningWindow::EMPTY; 4], 0)
        } else {
            parse_weekly_windows(weekly_windows.trim().as_bytes())
                .map_err(|_| VaultRuntimeError::InvalidSigningPolicy)?
        };
        let policy = SigningPolicy {
            not_before_unix,
            weekly_enabled: weekly_count != 0,
            weekly_count,
            windows,
            rtc_floor_unix: 0,
        };
        policy
            .validate()
            .map_err(|_| VaultRuntimeError::InvalidSigningPolicy)?;
        self.signing_policy = policy;
        Ok(())
    }

    pub fn clear_session_signing_policy(&mut self) {
        self.signing_policy = SigningPolicy::disabled();
    }

    pub fn check_session_signing_policy(&self, now_unix: u64) -> Result<(), VaultRuntimeError> {
        match self.signing_policy.evaluate(now_unix) {
            SigningDecision::Allowed => Ok(()),
            SigningDecision::BeforeNotBefore => Err(VaultRuntimeError::SigningBlockedNotBefore),
            SigningDecision::OutsideWeeklyWindow => {
                Err(VaultRuntimeError::SigningBlockedWeeklyWindow)
            }
            SigningDecision::ClockRollback => Err(VaultRuntimeError::SigningBlockedClockRollback),
            SigningDecision::ClockInvalid => Err(VaultRuntimeError::SigningBlockedClockInvalid),
            SigningDecision::PolicyInvalid => Err(VaultRuntimeError::InvalidSigningPolicy),
        }
    }
}
