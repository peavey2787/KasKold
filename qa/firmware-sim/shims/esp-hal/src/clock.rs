#[derive(Clone, Copy, Default)]
pub enum CpuClock {
    #[default]
    Max,
}

impl CpuClock {
    pub fn max() -> Self {
        Self::Max
    }
}
