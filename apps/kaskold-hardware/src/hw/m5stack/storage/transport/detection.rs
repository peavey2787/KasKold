//! CoreS3 card-session lifecycle on the single HAL-owned SPI2 bus.

use super::{
    power::power_cycle_card,
    protocol::{card_is_locked, initialize_card},
    Delay, SdCardType,
};
use crate::hw::m5stack::storage::record_card;

/// Power-cycle the slot and identify the card it holds now, so a card inserted
/// after power-on is found. An unlocked session keeps its card: cycling power
/// would lock it again.
pub(crate) fn detect_card<I2C>(i2c: &mut I2C, delay: &mut Delay) -> Option<SdCardType>
where
    I2C: embedded_hal::i2c::I2c,
{
    if let Some(card_type) = crate::hw::m5stack::storage::manual_unlock_session_card_type() {
        return Some(card_type);
    }
    let card = power_cycle_card(i2c, delay).and_then(|()| probe_card(delay));
    if let Err(error) = card {
        crate::log!("[SD] no usable card: {}", error);
    }
    record_card(card.ok());
    card.ok()
}

fn probe_card(delay: &mut Delay) -> Result<SdCardType, &'static str> {
    let card_type = initialize_card(delay)?;
    let locked = card_is_locked(true)?;
    crate::hw::m5stack::storage::record_card_lock_status(locked);
    if locked {
        crate::log!("[SD] {:?} card is password-locked; MBR read skipped", card_type);
    } else {
        log_mbr(card_type);
    }
    Ok(card_type)
}

fn log_mbr(card_type: SdCardType) {
    let mut sector0 = [0u8; 512];
    match super::block::sd_read_block(card_type, 0, &mut sector0) {
        Ok(()) => crate::log!(
            "[SD] {:?} card, MBR {:02x}{:02x}{:02x}{:02x}..sig={:02x}{:02x}",
            card_type,
            sector0[0],
            sector0[1],
            sector0[2],
            sector0[3],
            sector0[510],
            sector0[511]
        ),
        Err(error) => crate::log!("[SD] MBR read failed: {}", error),
    }
}
