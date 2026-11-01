#[derive(Clone, Copy)]
pub enum Priority {
    Max,
}

impl Priority {
    pub const fn max() -> Self {
        Self::Max
    }
}
