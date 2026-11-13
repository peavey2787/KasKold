//! Controller-facing removable-storage service facade.
//!
//! Controllers may request FAT32/SD operations here but may not import a board
//! storage driver directly. Board selection and shared-bus ownership remain in `hw`.

pub(crate) use crate::hw::sdcard::{
    create_file, delete_file, find_file_in_root, format_83_display, format_fat32, list_root_dir, mount_fat32,
    overwrite_file, read_file, to_83_name, with_sd_card, Fat32Info, SdCardType,
};

/// Probe the slot now, so a card inserted after power-on is found.
pub(crate) fn detect_card(
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
) -> Option<SdCardType> {
    workflow_view::shown().unwrap_or_else(|| crate::hw::sdcard::detect_card(i2c, delay))
}

/// The card found by the most recent probe or card operation, without touching the bus.
pub(crate) fn known_card() -> Option<SdCardType> {
    workflow_view::shown().unwrap_or_else(crate::hw::sdcard::known_card)
}

/// Lets a workflow test show controllers a slot other than the real one: empty
/// while a card is in it, or holding a card whose media the test substitutes.
#[cfg(feature = "workflow-test-auto")]
pub(crate) mod workflow_view {
    use super::SdCardType;
    use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

    const REAL: u8 = 0;
    const EMPTY: u8 = 1;
    static VIEW: AtomicU8 = AtomicU8::new(REAL);
    static PHYSICAL: AtomicBool = AtomicBool::new(false);

    /// Drive the real card for the rest of this run. Only a HIL image that
    /// found a card at boot does this; every other run keeps controller media.
    #[cfg(feature = "workflow-hil-auto")]
    pub(crate) fn drive_physical_media() {
        PHYSICAL.store(true, Ordering::Relaxed);
    }

    /// Whether this run reads and writes the real card.
    pub(crate) fn physical_media() -> bool {
        PHYSICAL.load(Ordering::Relaxed)
    }

    /// What controllers see: the real slot, or a stated card or empty slot.
    #[derive(Clone, Copy)]
    pub(crate) enum CardView {
        Real,
        Shown(Option<SdCardType>),
    }

    pub(crate) fn show(view: CardView) {
        let value = match view {
            CardView::Real => REAL,
            CardView::Shown(None) => EMPTY,
            CardView::Shown(Some(SdCardType::SdV1)) => 2,
            CardView::Shown(Some(SdCardType::SdV2Sc)) => 3,
            CardView::Shown(Some(SdCardType::SdV2Hc)) => 4,
        };
        VIEW.store(value, Ordering::Relaxed);
    }

    /// Run `operation` with controllers seeing `card`, then restore the view.
    pub(crate) fn while_showing<T>(card: Option<SdCardType>, operation: impl FnOnce() -> T) -> T {
        let before = VIEW.load(Ordering::Relaxed);
        show(CardView::Shown(card));
        let result = operation();
        VIEW.store(before, Ordering::Relaxed);
        result
    }

    pub(super) fn shown() -> Option<Option<SdCardType>> {
        match VIEW.load(Ordering::Relaxed) {
            EMPTY => Some(None),
            2 => Some(Some(SdCardType::SdV1)),
            3 => Some(Some(SdCardType::SdV2Sc)),
            4 => Some(Some(SdCardType::SdV2Hc)),
            _ => None,
        }
    }
}

#[cfg(not(feature = "workflow-test-auto"))]
mod workflow_view {
    pub(super) const fn shown() -> Option<Option<super::SdCardType>> {
        None
    }
}

/// Report whether the currently probed board storage is known to be password-locked.
/// This keeps controller/runtime code behind the storage service boundary.
pub(crate) fn card_is_known_locked() -> bool {
    crate::hw::sdcard::card_is_known_locked()
}

/// Unlock a password-locked SD card for the current powered session. The
/// password is caller-owned and must be zeroized immediately after this call.
pub(crate) fn unlock_locked_card(
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
    password: &[u8],
) -> Result<(), &'static str> {
    crate::hw::sdcard::unlock_locked_card_session(i2c, delay, password)
}

/// Destructively clear a card password using CMD42 force erase. This destroys
/// all card data and is only invoked after the runtime hold-to-confirm gate.
pub(crate) fn force_erase_locked_card(
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
) -> Result<bool, &'static str> {
    crate::hw::sdcard::force_erase_locked_card_session(i2c, delay, liveness)
}

/// HIL-only destructive preparation hook retained for disposable-media workflow
/// qualification. Production Format reaches the same protocol only through the
/// runtime hold-to-confirm destructive service.
#[cfg(all(feature = "workflow-hil-auto", feature = "m5stack"))]
pub(crate) fn workflow_force_erase_locked_card(
    card_type: SdCardType,
    delay: &mut esp_hal::delay::Delay,
) -> Result<bool, &'static str> {
    crate::hw::sdcard::workflow_force_erase_locked_card(card_type, delay)
}

/// HIL-only CoreS3 media-formatting hook. The HIL workflow requires a disposable
/// QA card and may intentionally destroy its contents while verifying real media.
#[cfg(all(feature = "workflow-hil-auto", feature = "m5stack"))]
pub(crate) fn workflow_format_fat32(
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
) -> bool {
    let mut liveness = || {};
    crate::hw::sdcard::format_fat32(i2c, delay, &mut liveness)
}
