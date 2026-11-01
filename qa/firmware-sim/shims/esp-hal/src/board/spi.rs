//! Routes the shared SPI2 bus to whichever device's chip select is low.
use super::{lcd, pins, sd};

const LCD_CHIP_SELECT: u8 = 3;
const SD_CHIP_SELECT: u8 = 4;
const DATA_COMMAND: u8 = 35;
const IDLE_LEVEL: u8 = 0xFF;

pub fn exchange(out: &[u8], input: &mut [u8]) {
    let len = out.len().max(input.len());
    for index in 0..len {
        let sent = out.get(index).copied().unwrap_or(IDLE_LEVEL);
        let received = exchange_byte(sent);
        if let Some(slot) = input.get_mut(index) {
            *slot = received;
        }
    }
}

fn exchange_byte(sent: u8) -> u8 {
    if !pins::is_high(LCD_CHIP_SELECT) {
        lcd::receive(sent, pins::is_high(DATA_COMMAND));
        return IDLE_LEVEL;
    }
    if !pins::is_high(SD_CHIP_SELECT) {
        return sd::exchange(sent);
    }
    IDLE_LEVEL
}
