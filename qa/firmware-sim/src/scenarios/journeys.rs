//! Journeys a person walks through by reading the screen and tapping what it shows.

/// A journey: each named phase is one power-on of the same device, in order.
macro_rules! journey {
    ($name:ident: $($phase:path),+ $(,)?) => {
        #[test]
        fn $name() {
            crate::harness::across_power_cycles(concat!(module_path!(), "::", stringify!($name)), &[$($phase),+]);
        }
    };
}

mod first_wallet;
mod known;
mod security;
mod signing;
mod walk;
