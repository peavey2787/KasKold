//! CoreS3 GPIO35 role switch: SD MISO when LCD is deselected, LCD D/C while
//! the LCD chip-select is asserted.

use crate::hw::shared::{
    mmio::{read as read_register, write as write_register},
    registers::esp32s3::{
        GPIO_ENABLE1_W1TS, GPIO_FUNC_OUT_SEL_BASE, GPIO_OUT1_W1TC, GPIO_OUT1_W1TS,
    },
};

const GPIO_ENABLE1_W1TC: u32 = 0x6000_4034;
const GPIO35_BIT: u32 = 1u32 << 3;
const GPIO35_FUNC_OUT_SEL: u32 = GPIO_FUNC_OUT_SEL_BASE + 35 * 4;
const GPIO_ENABLE1_REG: u32 = 0x6000_402c;
const GPIO_MATRIX_GPIO_OUT: u32 = 256;
const FSPIQ_OUT_IDX: u32 = 102;

pub(super) fn select_lcd_dc() {
    unsafe {
        write_register(GPIO35_FUNC_OUT_SEL, GPIO_MATRIX_GPIO_OUT);
        write_register(GPIO_ENABLE1_W1TS, GPIO35_BIT);
    }
}

pub(super) fn select_sd_miso() {
    unsafe {
        write_register(GPIO35_FUNC_OUT_SEL, FSPIQ_OUT_IDX);
        write_register(GPIO_ENABLE1_W1TC, GPIO35_BIT);
    }
}

pub(super) fn diagnostic_state() -> (u32, bool) {
    unsafe {
        let selector = read_register(GPIO35_FUNC_OUT_SEL);
        let output_enabled = read_register(GPIO_ENABLE1_REG) & GPIO35_BIT != 0;
        (selector, output_enabled)
    }
}

pub(super) fn set_dc(high: bool) {
    let register = if high { GPIO_OUT1_W1TS } else { GPIO_OUT1_W1TC };
    unsafe {
        write_register(register, GPIO35_BIT);
    }
}
