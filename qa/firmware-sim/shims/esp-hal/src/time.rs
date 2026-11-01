use crate::board;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rate(u32);

impl Rate {
    pub const fn from_hz(hz: u32) -> Self {
        Self(hz)
    }
    pub const fn from_khz(khz: u32) -> Self {
        Self(khz * 1_000)
    }
    pub const fn from_mhz(mhz: u32) -> Self {
        Self(mhz * 1_000_000)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Duration(u64);

impl Duration {
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms * 1_000)
    }
    pub const fn from_micros(us: u64) -> Self {
        Self(us)
    }
    pub const fn from_secs(s: u64) -> Self {
        Self(s * 1_000_000)
    }
    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000
    }
    pub const fn as_micros(&self) -> u64 {
        self.0
    }
    pub const fn as_secs(&self) -> u64 {
        self.0 / 1_000_000
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Instant(u64);

impl Instant {
    pub fn now() -> Self {
        Self(board::clock::read_micros())
    }
    pub fn duration_since_epoch(&self) -> Duration {
        Duration(self.0)
    }
    pub fn elapsed(&self) -> Duration {
        Duration(board::clock::read_micros().saturating_sub(self.0))
    }
}

impl core::ops::Sub for Instant {
    type Output = Duration;
    fn sub(self, other: Self) -> Duration {
        Duration(self.0.saturating_sub(other.0))
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, other: Duration) -> Instant {
        Instant(self.0 + other.0)
    }
}
