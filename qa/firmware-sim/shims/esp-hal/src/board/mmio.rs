//! The SoC registers the firmware touches directly.
use std::{collections::HashMap, sync::Mutex};

use super::clock;

const GPIO_OUT1_W1TS: u32 = 0x6000_4014;
const GPIO_OUT1_W1TC: u32 = 0x6000_4018;
const GPIO35_BIT: u32 = 1 << 3;
const SYSTIMER_UNIT0_VALUE_LO: u32 = 0x6002_3044;
const RNG_DATA: u32 = 0x6003_507C;
const ADC_NOISE: u32 = 0x6004_0868;
const SYSTIMER_TICKS_PER_MICRO: u64 = 16;

static REGISTERS: Mutex<Option<HashMap<u32, u32>>> = Mutex::new(None);
static RNG_STATE: Mutex<u32> = Mutex::new(0x9E37_79B9);

fn with_registers<T>(operation: impl FnOnce(&mut HashMap<u32, u32>) -> T) -> T {
    let mut registers = REGISTERS.lock().expect("register model poisoned");
    operation(registers.get_or_insert_with(HashMap::new))
}

/// A deterministic xorshift stream: statistically healthy, and repeatable in every run.
fn next_noise() -> u32 {
    let mut state = RNG_STATE.lock().expect("rng model poisoned");
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *state = value;
    value
}

/// # Safety
/// Matches the firmware boundary; the model accepts every address.
pub unsafe fn read(address: u32) -> u32 {
    match address {
        RNG_DATA | ADC_NOISE => next_noise(),
        SYSTIMER_UNIT0_VALUE_LO => (clock::read_micros() * SYSTIMER_TICKS_PER_MICRO) as u32,
        _ => with_registers(|registers| registers.get(&address).copied().unwrap_or(0)),
    }
}

/// # Safety
/// Matches the firmware boundary; the model accepts every address.
pub unsafe fn write(address: u32, value: u32) {
    if value & GPIO35_BIT != 0 {
        match address {
            GPIO_OUT1_W1TS => super::pins::set(35, true),
            GPIO_OUT1_W1TC => super::pins::set(35, false),
            _ => {}
        }
    }
    with_registers(|registers| registers.insert(address, value));
}
