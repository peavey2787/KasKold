//! Host stand-in for `esp-println`: serial output is kept for tests to read, and
//! also shown when `KASKOLD_SIM_LOG` is set.
use std::fmt::{Arguments, Write};
use std::sync::Mutex;

static SERIAL: Mutex<String> = Mutex::new(String::new());

pub fn emit(line: Arguments<'_>, end: &str) {
    let mut serial = SERIAL.lock().expect("serial log poisoned");
    let start = serial.len();
    let _ = write!(serial, "{line}{end}");
    if std::env::var_os("KASKOLD_SIM_LOG").is_some() {
        std::print!("{}", &serial[start..]);
    }
}

/// Everything the firmware has written to its serial port so far.
pub fn captured() -> String {
    SERIAL.lock().expect("serial log poisoned").clone()
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::emit(core::format_args!(""), "
")
    };
    ($($arg:tt)*) => {
        $crate::emit(core::format_args!($($arg)*), "\n")
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::emit(core::format_args!($($arg)*), "")
    };
}
