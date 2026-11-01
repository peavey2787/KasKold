/// The firmware asked for a reset; the run ends here with a recognisable payload.
pub struct SoftwareReset;

pub fn software_reset() -> ! {
    super::control::firmware_stopped();
    std::panic::panic_any(SoftwareReset)
}
