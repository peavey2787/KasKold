//! Platform entropy adapter.
//!
//! Browser builds use getrandom's JavaScript backend (`crypto.getRandomValues`), while
//! native host/coverage builds use getrandom's native OS backend. Keeping the call
//! behind one adapter prevents browser-only randomness assumptions from leaking into
//! host-test code.

pub(crate) fn fill_secure_random(bytes: &mut [u8]) -> Result<(), getrandom::Error> {
    getrandom::getrandom(bytes)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod unit_tests {
    use super::fill_secure_random;

    #[test]
    fn host_entropy_adapter_is_callable_without_wasm_imports() {
        let mut bytes = [0u8; 32];
        fill_secure_random(&mut bytes).expect("platform entropy backend");
    }
}
