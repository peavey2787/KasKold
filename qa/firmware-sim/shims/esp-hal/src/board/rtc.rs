//! The BM8563 clock: its coin cell keeps the time when the rest of the device loses power.
use super::i2c;

const ADDRESS: u8 = 0x51;
const SECONDS_REGISTER: u8 = 0x02;
const VOLTAGE_LOW_FLAG: u8 = 0x80;
pub const IMAGE_BYTES: usize = 7;

pub fn image() -> [u8; IMAGE_BYTES] {
    let mut registers = [0u8; IMAGE_BYTES];
    i2c::write(ADDRESS, &[SECONDS_REGISTER]).expect("clock on the bus");
    i2c::read(ADDRESS, &mut registers).expect("clock on the bus");
    registers
}

pub fn restore(registers: &[u8]) {
    let mut write = vec![SECONDS_REGISTER];
    write.extend_from_slice(registers);
    i2c::write(ADDRESS, &write).expect("clock on the bus");
}

/// The coin cell died, so the chip flags that its time can no longer be trusted.
pub fn lose_battery() {
    let mut registers = image();
    registers[0] |= VOLTAGE_LOW_FLAG;
    restore(&registers);
}
