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
        fn normalize_one(pskt: &mut serde_json::Value) {
            let input_count = pskt
                .get("inputs")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len);
            let output_count = pskt
                .get("outputs")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len);
            if let Some(global) = pskt
                .get_mut("global")
                .and_then(serde_json::Value::as_object_mut)
            {
                global
                    .entry("version")
                    .or_insert(serde_json::Value::from(0u8));
                global
                    .entry("txVersion")
                    .or_insert(serde_json::Value::from(0u8));
                global
                    .entry("inputCount")
                    .or_insert(serde_json::Value::from(input_count));
                global
                    .entry("outputCount")
                    .or_insert(serde_json::Value::from(output_count));
            }
            if let Some(inputs) = pskt
                .get_mut("inputs")
                .and_then(serde_json::Value::as_array_mut)
            {
                for input in inputs {
                    if let Some(obj) = input.as_object_mut() {
                        obj.entry("sighashType")
                            .or_insert(serde_json::Value::from(1u8));
                        if !obj
                            .get("proprietaries")
                            .is_some_and(serde_json::Value::is_object)
                        {
                            obj.insert(
                                "proprietaries".into(),
                                serde_json::Value::Object(serde_json::Map::new()),
                            );
                        }
                    }
                }
            }
            if let Some(outputs) = pskt
                .get_mut("outputs")
                .and_then(serde_json::Value::as_array_mut)
            {
                for output in outputs {
                    if let Some(obj) = output.as_object_mut() {
                        if !obj
                            .get("proprietaries")
                            .is_some_and(serde_json::Value::is_object)
                        {
                            obj.insert(
                                "proprietaries".into(),
                                serde_json::Value::Object(serde_json::Map::new()),
                            );
                        }
                    }
                }
            }
        }
        match &mut value {
            serde_json::Value::Array(items) => items.iter_mut().for_each(normalize_one),
            serde_json::Value::Object(_) => normalize_one(&mut value),
            _ => {}
        }
        value
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
        use k256::elliptic_curve::sec1::ToEncodedPoint;

        let point = (k256::ProjectivePoint::GENERATOR * k256::Scalar::from(u64::from(seed.max(1))))
            .to_affine()
            .to_encoded_point(true);
        hex::encode(&point.as_bytes()[1..])
    }
}
