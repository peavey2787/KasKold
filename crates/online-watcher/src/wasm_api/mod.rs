#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::JsValue;

#[cfg(not(target_arch = "wasm32"))]
mod host_compat {
    /// Native-test stand-in for the browser-only `wasm_bindgen::JsValue`.
    ///
    /// Host builds execute adapter/domain logic without crossing the wasm ABI.
    /// The actual JavaScript-facing type is restored unchanged on wasm32.
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct JsValue(Option<String>);

    impl JsValue {
        pub(crate) fn from_str(message: &str) -> Self {
            Self(Some(message.to_owned()))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use host_compat::JsValue;
macro_rules! wasm_error {
    ($message:expr $(,)?) => {
        $crate::wasm_api::utilities::common::js_error($message)
    };
}

mod contracts;
mod privacy;
mod protocol;
mod transactions;
mod utilities;
mod wallet;

pub use contracts::*;
pub use privacy::*;
pub use protocol::*;
pub use transactions::*;
pub use utilities::*;
pub use wallet::*;

#[cfg(test)]
pub(crate) mod test_support {
    use core::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    /// Fill the PSKT fields the canonical grammar requires but fixtures omit.
    pub(crate) fn canonical_test_pskt(mut value: serde_json::Value) -> serde_json::Value {
        match &mut value {
            serde_json::Value::Array(items) => items.iter_mut().for_each(fill_pskt_defaults),
            serde_json::Value::Object(_) => fill_pskt_defaults(&mut value),
            _ => {}
        }
        value
    }

    fn fill_pskt_defaults(pskt: &mut serde_json::Value) {
        let input_count = entry_count(pskt, "inputs");
        let output_count = entry_count(pskt, "outputs");
        if let Some(global) = pskt
            .get_mut("global")
            .and_then(serde_json::Value::as_object_mut)
        {
            for (key, default) in [
                ("version", 0usize),
                ("txVersion", 0),
                ("inputCount", input_count),
                ("outputCount", output_count),
            ] {
                global
                    .entry(key)
                    .or_insert(serde_json::Value::from(default));
            }
        }
        for entry in entries_mut(pskt, "inputs") {
            entry
                .entry("sighashType")
                .or_insert(serde_json::Value::from(1u8));
            ensure_proprietaries(entry);
        }
        for entry in entries_mut(pskt, "outputs") {
            ensure_proprietaries(entry);
        }
    }

    fn entry_count(pskt: &serde_json::Value, key: &str) -> usize {
        pskt.get(key)
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len)
    }

    fn entries_mut<'a>(
        pskt: &'a mut serde_json::Value,
        key: &str,
    ) -> impl Iterator<Item = &'a mut serde_json::Map<String, serde_json::Value>> {
        pskt.get_mut(key)
            .and_then(serde_json::Value::as_array_mut)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_object_mut)
    }

    fn ensure_proprietaries(entry: &mut serde_json::Map<String, serde_json::Value>) {
        if !entry
            .get("proprietaries")
            .is_some_and(serde_json::Value::is_object)
        {
            entry.insert(
                "proprietaries".into(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
    }

    /// Poll a boundary future that is expected to complete without browser I/O.
    pub(crate) fn ready<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("host boundary unexpectedly attempted browser I/O"),
        }
    }

    /// Unwrap an adapter result in tests. Native builds use the host-safe
    /// `JsValue` stand-in, while wasm32 keeps the real browser error type.
    pub(crate) fn expect_js<T>(result: Result<T, crate::wasm_api::JsValue>, label: &str) -> T {
        match result {
            Ok(value) => value,
            Err(_) => panic!("{label} returned a wasm adapter error"),
        }
    }

    /// Deterministic, curve-valid x-only public key (`seed·G`) for host fixtures.
    pub(crate) fn xonly_key(seed: u8) -> String {
        use k256::elliptic_curve::sec1::ToSec1Point;

        let point = (k256::ProjectivePoint::GENERATOR * k256::Scalar::from(u64::from(seed.max(1))))
            .to_affine()
            .to_sec1_point(true);
        hex::encode(&point.as_bytes()[1..])
    }
}
