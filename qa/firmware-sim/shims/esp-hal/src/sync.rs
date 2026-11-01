use crate::interrupt::Priority;

/// One firmware thread drives the board, so the lock only marks the section.
pub struct RawPriorityLimitedMutex;

impl RawPriorityLimitedMutex {
    pub const fn new(_priority: Priority) -> Self {
        Self
    }

    pub fn lock<T>(&self, operation: impl FnOnce() -> T) -> T {
        operation()
    }
}
