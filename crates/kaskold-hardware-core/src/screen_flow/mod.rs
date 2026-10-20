//! The device's screen states and the pure rules over them: which touch
//! controller owns each screen and which onboarding moves are legal.

mod handler_group;
pub mod onboarding;
mod state;

pub use handler_group::HandlerGroup;
pub use state::{is_scan_state, AppState};
