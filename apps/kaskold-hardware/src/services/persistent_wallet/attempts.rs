//! Durable unlock-attempt accounting.
//!
//! Every unlock attempt is counted in flash before any credential is
//! evaluated, so cutting power mid-attempt cannot hide a guess. Success
//! resets the count; reaching `MAX_UNLOCK_FAILURES` erases every saved wallet.

use crate::runtime::data::AppData;
use crate::services::credential_policy::unlock_attempt_limit_reached;

use super::{journal, sd_backend, PersistError, PersistentWallet, StorageMode};

impl PersistentWallet<'_> {
    /// Consecutive failed unlock attempts recorded in flash.
    pub(crate) fn unlock_failures(&mut self) -> Result<u8, PersistError> {
        journal::read_unlock_failures(&mut self.flash).map_err(PersistError::from)
    }

    /// Count one attempt before its credential is evaluated. A device already
    /// at the limit (power was cut during the final attempt) is wiped instead.
    pub(crate) fn begin_unlock_attempt(
        &mut self,
        ad: &mut AppData,
        i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
        delay: &mut esp_hal::delay::Delay,
    ) -> Result<(), PersistError> {
        let failures = self.unlock_failures()?;
        if unlock_attempt_limit_reached(failures) {
            return Err(self.wipe_after_attempt_limit(ad, i2c, delay));
        }
        let counted = failures + 1;
        journal::write_unlock_failures(&mut self.flash, counted)?;
        ad.storage.persistence.unlock_failures = counted;
        Ok(())
    }

    /// Settle a counted attempt: success clears the count, and a failure that
    /// used the final attempt wipes the device.
    pub(crate) fn conclude_unlock_attempt(
        &mut self,
        result: Result<(), PersistError>,
        ad: &mut AppData,
        i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
        delay: &mut esp_hal::delay::Delay,
    ) -> Result<(), PersistError> {
        match result {
            Ok(()) => self.clear_unlock_failures(ad),
            Err(PersistError::DuressTriggered | PersistError::DeviceWipeFailed) => result,
            Err(error) => match self.unlock_failures() {
                Ok(failures) if unlock_attempt_limit_reached(failures) => {
                    Err(self.wipe_after_attempt_limit(ad, i2c, delay))
                }
                _ => Err(error),
            },
        }
    }

    /// A success whose reset cannot be stored must not leave the wallet open
    /// while flash still records the failures.
    fn clear_unlock_failures(&mut self, ad: &mut AppData) -> Result<(), PersistError> {
        if journal::write_unlock_failures(&mut self.flash, 0).is_ok() {
            ad.storage.persistence.unlock_failures = 0;
            return Ok(());
        }
        crate::services::device_wipe::zeroize_volatile(ad);
        Err(PersistError::Flash)
    }

    fn wipe_after_attempt_limit(
        &mut self,
        ad: &mut AppData,
        i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
        delay: &mut esp_hal::delay::Delay,
    ) -> PersistError {
        if self.erase_user_data(ad, i2c, delay) {
            PersistError::AttemptLimitWiped
        } else {
            PersistError::DeviceWipeFailed
        }
    }

    /// Erase every saved wallet (SD slots first, then internal flash) and all
    /// volatile secrets; reports whether both erasures succeeded.
    pub(super) fn erase_user_data(
        &mut self,
        ad: &mut AppData,
        i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
        delay: &mut esp_hal::delay::Delay,
    ) -> bool {
        crate::services::device_wipe::zeroize_volatile(ad);
        let sd_erased = self.mode != Some(StorageMode::SdCard) || sd_backend::erase_files(i2c, delay).is_ok();
        let internal_erased = journal::erase_all_user_data(&mut self.flash).is_ok();
        self.mode = None;
        self.clear_credential();
        self.security_policy = super::security_policy::SecurityPolicy::disabled();
        self.security_integrity_ok = true;
        self.sync_security_mirror(ad);
        sd_erased && internal_erased
    }
}
