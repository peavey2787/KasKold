use crate::board;

#[derive(Clone, Copy, Debug, Default)]
pub struct Delay;

impl Delay {
    pub const fn new() -> Self {
        Self
    }

    pub fn delay_millis(&self, ms: u32) {
        board::clock::wait_micros(u64::from(ms) * 1_000);
    }

    pub fn delay_micros(&self, us: u32) {
        board::clock::wait_micros(u64::from(us));
    }

    pub fn delay_nanos(&self, ns: u32) {
        board::clock::wait_micros(u64::from(ns / 1_000));
    }
}

impl embedded_hal::delay::DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        self.delay_nanos(ns);
    }
}
