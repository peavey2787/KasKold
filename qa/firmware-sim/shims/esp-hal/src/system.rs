use crate::{board, peripherals::CPU_CTRL, rtc_cntl::SocResetReason};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cpu {
    ProCpu,
    AppCpu,
}

impl Cpu {
    pub fn other() -> impl Iterator<Item = Self> {
        [Self::AppCpu].into_iter()
    }
}

pub fn is_running(_cpu: Cpu) -> bool {
    false
}

pub struct CpuControl;

impl CpuControl {
    pub fn new(_control: CPU_CTRL<'_>) -> Self {
        Self
    }

    /// The peer core becomes a host thread running the same worker.
    pub fn start_app_core<const SIZE: usize, F>(
        &mut self,
        _stack: &'static mut Stack<SIZE>,
        entry: F,
    ) -> Result<AppCoreGuard, Error>
    where
        F: FnOnce() + Send + 'static,
    {
        std::thread::Builder::new()
            .name("firmware-core1".into())
            .stack_size(SIZE)
            .spawn(entry)
            .map_err(|_| Error::CoreAlreadyRunning)?;
        board::clock::peer_core_started();
        Ok(AppCoreGuard)
    }

    /// # Safety
    /// Matches the vendor signature; the virtual board has one running core.
    pub unsafe fn park_core(&mut self, _cpu: Cpu) {}

    pub fn unpark_core(&mut self, _cpu: Cpu) {}
}

pub fn software_reset() -> ! {
    board::halt::software_reset()
}

pub fn reset_reason() -> Option<SocResetReason> {
    Some(SocResetReason::ChipPowerOn)
}

pub struct Stack<const SIZE: usize>;

impl<const SIZE: usize> Stack<SIZE> {
    pub const fn new() -> Self {
        Self
    }
}

pub struct AppCoreGuard;

#[derive(Clone, Copy, Debug)]
pub enum Error {
    CoreAlreadyRunning,
}
