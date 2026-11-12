pub mod timer {
    /// One simulated cycle per microsecond is enough for timing-window logic.
    pub fn get_cycle_count() -> u32 {
        crate::board::mmio::low_word(crate::board::clock::read_micros())
    }
}
