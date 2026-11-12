//! The CoreS3 internal I2C bus: a register file per chip plus the chip-specific behaviour.
use std::{collections::HashMap, sync::Mutex};

use super::{control, touch};
use crate::i2c::master::Error;

const FT6336U: u8 = 0x38;
const AW88298: u8 = 0x36;
const AXP2101: u8 = 0x34;
const AW9523B: u8 = 0x58;
const BM8563: u8 = 0x51;
const BMI270: u8 = 0x69;
const GC0308: u8 = 0x21;
const PRESENT: [u8; 7] = [FT6336U, AW88298, AXP2101, AW9523B, BM8563, BMI270, GC0308];

const FT6336U_TD_STATUS: u8 = 0x02;
const GC0308_CHIP_ID_VALUE: u8 = 0x9B;
const BMI270_CHIP_ID_VALUE: u8 = 0x24;
const BMI270_INTERNAL_STATUS: u8 = 0x21;
const BMI270_INTERNAL_READY: u8 = 0x01;
const BMI270_STATUS: u8 = 0x03;
const BMI270_ALL_DATA_READY: u8 = 0xE0;
const BMI270_ACCEL_DATA: u8 = 0x0C;
const IMU_SAMPLE_BYTES: u8 = 12;
const AW9523B_ID_REGISTER: u8 = 0x10;
const AW9523B_ID_VALUE: u8 = 0x23;

#[derive(Default)]
struct Bus {
    registers: HashMap<(u8, u8), u8>,
    pointers: HashMap<u8, u8>,
    imu_tick: u16,
}

static BUS: Mutex<Option<Bus>> = Mutex::new(None);

fn with_bus<T>(operation: impl FnOnce(&mut Bus) -> T) -> T {
    let mut bus = BUS.lock().expect("i2c model poisoned");
    operation(bus.get_or_insert_with(Bus::default))
}

fn require_present(address: u8) -> Result<(), Error> {
    PRESENT
        .contains(&address)
        .then_some(())
        .ok_or(Error::AcknowledgeCheckFailed)
}

pub fn write(address: u8, bytes: &[u8]) -> Result<(), Error> {
    require_present(address)?;
    with_bus(|bus| {
        let Some((register, values)) = bytes.split_first() else {
            return;
        };
        bus.pointers.insert(address, *register);
        for (offset, value) in values.iter().enumerate() {
            bus.registers.insert(
                (address, register.wrapping_add(offset.to_le_bytes()[0])),
                *value,
            );
        }
    });
    Ok(())
}

pub fn read(address: u8, buffer: &mut [u8]) -> Result<(), Error> {
    require_present(address)?;
    if address == FT6336U && with_bus(|bus| bus.pointers.get(&address) == Some(&FT6336U_TD_STATUS))
    {
        control::firmware_poll();
        touch::read_registers(buffer);
        return Ok(());
    }
    with_bus(|bus| bus.read(address, buffer));
    Ok(())
}

impl Bus {
    fn read(&mut self, address: u8, buffer: &mut [u8]) {
        let start = self.pointers.get(&address).copied().unwrap_or(0);
        for (offset, slot) in buffer.iter_mut().enumerate() {
            let register = start.wrapping_add(offset.to_le_bytes()[0]);
            *slot = self.register_value(address, register);
        }
    }

    fn register_value(&mut self, address: u8, register: u8) -> u8 {
        match (address, register) {
            (GC0308, 0x00) => GC0308_CHIP_ID_VALUE,
            (AW9523B, AW9523B_ID_REGISTER) => AW9523B_ID_VALUE,
            (BMI270, BMI270_STATUS) => BMI270_ALL_DATA_READY,
            (BMI270, 0x00) => BMI270_CHIP_ID_VALUE,
            (BMI270, BMI270_INTERNAL_STATUS) => BMI270_INTERNAL_READY,
            (BMI270, r)
                if (BMI270_ACCEL_DATA..BMI270_ACCEL_DATA + IMU_SAMPLE_BYTES).contains(&r) =>
            {
                self.imu_byte()
            }
            _ => self
                .registers
                .get(&(address, register))
                .copied()
                .unwrap_or(0),
        }
    }

    /// A moving, non-repeating sensor signal, so the IMU health checks see a live part.
    fn imu_byte(&mut self) -> u8 {
        self.imu_tick = self.imu_tick.wrapping_add(1);
        let tick = u32::from(self.imu_tick);
        (tick.wrapping_mul(2_654_435_761) >> 24) as u8
    }
}
