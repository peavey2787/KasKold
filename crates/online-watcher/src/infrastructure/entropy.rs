//! Platform entropy adapter.
//!
//! Browser builds use getrandom's JavaScript backend (`crypto.getRandomValues`), while
//! native host/coverage builds use getrandom's native OS backend. Keeping the call
//! behind one adapter prevents browser-only randomness assumptions from leaking into
//! host-test code.

pub(crate) fn fill_secure_random(bytes: &mut [u8]) -> Result<(), String> {
    getrandom::getrandom(bytes).map_err(|error| format!("RNG failed: {error}"))
}
