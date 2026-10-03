#[cfg(not(target_arch = "wasm32"))]
#[test]
fn browser_log_is_a_safe_noop_on_native_targets() {
    let result = std::panic::catch_unwind(|| {
        super::browser_log::info("native coverage logging");
        super::browser_log::info(String::from("owned native coverage logging"));
    });
    assert!(result.is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn host_entropy_adapter_fills_buffers_without_wasm_imports() {
    let mut first = [0u8; 32];
    let mut second = [0u8; 32];
    assert!(super::fill_secure_random(&mut first).is_ok());
    assert!(super::fill_secure_random(&mut second).is_ok());
    assert_ne!(first, [0u8; 32]);
    assert_ne!(first, second);
}
