//! Host stand-in for `esp-hal`: the API the firmware calls, backed by a virtual CoreS3.
pub mod board;
pub mod clock;
pub mod delay;
pub mod dma;
pub mod efuse;
pub mod gpio;
pub mod hmac;
pub mod i2c;
pub mod i2s;
pub mod interrupt;
pub mod lcd_cam;
pub mod peripherals;
pub mod psram;
pub mod rom;
pub mod rtc_cntl;
pub mod spi;
pub mod sync;
pub mod system;
pub mod time;
pub mod timer;
pub mod xtensa_lx;

pub use esp_hal_macros::{main, ram};

#[derive(Clone, Copy, Debug, Default)]
pub struct Blocking;

#[derive(Clone, Copy, Default)]
pub struct Config {
    _cpu_clock: clock::CpuClock,
}

impl Config {
    pub fn with_cpu_clock(self, cpu_clock: clock::CpuClock) -> Self {
        Self {
            _cpu_clock: cpu_clock,
        }
    }
}

pub fn init(_config: Config) -> peripherals::Peripherals {
    peripherals::Peripherals::take()
}
