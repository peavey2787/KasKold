//! Covenant signing fails closed on out-of-order calls, foreign bindings,
//! malformed shapes and every mismatched reveal field.

use shared_signer::covenant_sign::{
    self as wire, BindingHint, CovenantSignRequest, CovenantSignReveal, KnownScheme, RequestKind,
};

use super::{encode_covenant_request, MNEMONIC};
use crate::{VaultRuntime, VaultRuntimeError};

const SCRIPT: &[u8] = b"third-party covenant script";
const HOST_SECRET: [u8; 32] = [0x51; 32];
const COMMITMENT: [u8; 32] = [0x62; 32];
const SESSION: [u8; wire::SESSION_ID_LEN] = [0x73; wire::SESSION_ID_LEN];

struct Bound {
    runtime: VaultRuntime,
    key_id: [u8; 32],
    binding_token: [u8; 32],
}

fn request<'a>(kind: RequestKind, key_id: [u8; 32], script: &'a [u8]) -> CovenantSignRequest<'a> {
    CovenantSignRequest {
        kind,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id,
        binding_token: [0; 32],
        commitment: [0; 32],
        script,
        context: &[],
    }
}

fn bound(script: &[u8]) -> Bound {
    bound_with(|_| script.to_vec()).0
}

/// Allocate a covenant key, then bind the script built from that key.
fn bound_with(script_for: impl Fn(&[u8; 32]) -> Vec<u8>) -> (Bound, Vec<u8>) {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime
        .covenant_prepare(&encode_covenant_request(&request(
            RequestKind::KeyInfo,
            [0; 32],
            &[],
        )))
        .unwrap();
    let key_info = wire::parse_response(&runtime.covenant_confirm().unwrap()).unwrap();
    let script = script_for(&key_info.pubkey_x);
    runtime
        .covenant_prepare(&encode_covenant_request(&request(
            RequestKind::Bind,
            key_info.key_id,
            &script,
        )))
        .unwrap();
    let binding = wire::parse_response(&runtime.covenant_confirm().unwrap()).unwrap();
    let bound = Bound {
        runtime,
        key_id: key_info.key_id,
        binding_token: binding.binding_token,
    };
    (bound, script)
}

fn signing<'a>(bound: &Bound, script: &'a [u8]) -> CovenantSignRequest<'a> {
    CovenantSignRequest {
        session_id: SESSION,
        host_commitment: shared_signer::anti_klepto::host_commitment(&HOST_SECRET),
        binding_token: bound.binding_token,
        commitment: COMMITMENT,
        ..request(RequestKind::Opaque, bound.key_id, script)
    }
}

fn awaiting_reveal() -> Bound {
    let mut bound = bound(SCRIPT);
    let wire_request = encode_covenant_request(&signing(&bound, SCRIPT));
    bound.runtime.covenant_prepare(&wire_request).unwrap();
    bound.runtime.covenant_confirm().unwrap();
    bound
}

fn reveal(reveal: &CovenantSignReveal) -> [u8; wire::REVEAL_LEN] {
    let mut out = [0u8; wire::REVEAL_LEN];
    wire::encode_reveal(reveal, &mut out).unwrap();
    out
}

#[test]
fn every_mismatched_reveal_field_is_rejected_without_losing_the_session() {
    let mut bound = awaiting_reveal();
    let good = CovenantSignReveal {
        session_id: SESSION,
        key_id: bound.key_id,
        commitment: COMMITMENT,
        host_secret: HOST_SECRET,
    };
    for bad in [
        CovenantSignReveal {
            session_id: [0x74; wire::SESSION_ID_LEN],
            ..good
        },
        CovenantSignReveal {
            key_id: [0x75; 32],
            ..good
        },
        CovenantSignReveal {
            commitment: [0x76; 32],
            ..good
        },
        CovenantSignReveal {
            host_secret: [0x77; 32],
            ..good
        },
    ] {
        assert!(matches!(
            bound.runtime.covenant_finalize_reveal(&reveal(&bad)),
            Err(VaultRuntimeError::CovenantRevealMismatch)
        ));
    }
    assert!(bound
        .runtime
        .covenant_finalize_reveal(&reveal(&good))
        .is_ok());
    // A second reveal after the final signature is out of order.
    assert!(matches!(
        bound.runtime.covenant_finalize_reveal(&reveal(&good)),
        Err(VaultRuntimeError::InvalidCovenantState)
    ));
}

#[test]
fn out_of_order_confirmations_and_cancel_fail_closed() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    assert!(matches!(
        runtime.covenant_confirm(),
        Err(VaultRuntimeError::InvalidCovenantState)
    ));

    let mut bound = awaiting_reveal();
    // The nonce commitment was already issued; confirming again is refused.
    assert!(matches!(
        bound.runtime.covenant_confirm(),
        Err(VaultRuntimeError::InvalidCovenantState)
    ));
    bound.runtime.covenant_cancel();
    assert!(matches!(
        bound.runtime.covenant_response(),
        Err(VaultRuntimeError::NoCovenantResponse)
    ));
    assert!(matches!(
        bound.runtime.covenant_confirm(),
        Err(VaultRuntimeError::InvalidCovenantState)
    ));
}

#[test]
fn signing_requires_the_wallets_own_binding_token() {
    let mut bound = bound(SCRIPT);
    let mut foreign = signing(&bound, SCRIPT);
    foreign.binding_token = [0x99; 32];
    assert!(matches!(
        bound
            .runtime
            .covenant_prepare(&encode_covenant_request(&foreign)),
        Err(VaultRuntimeError::CovenantBindingMismatch)
    ));

    // Binding a key the wallet did not just allocate is refused.
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    assert!(matches!(
        runtime.covenant_prepare(&encode_covenant_request(&request(
            RequestKind::Bind,
            bound.key_id,
            SCRIPT
        ))),
        Err(VaultRuntimeError::CovenantBindingRequired)
    ));
}

#[test]
fn opaque_key_present_hint_requires_the_key_in_the_script() {
    let (mut bound, script) = bound_with(|key| {
        let mut script = vec![0x20];
        script.extend_from_slice(key);
        script.push(0xac);
        script
    });
    let mut hinted = signing(&bound, &script);
    hinted.binding = BindingHint::KeyPresent;
    assert!(bound
        .runtime
        .covenant_prepare(&encode_covenant_request(&hinted))
        .is_ok());

    let mut bound_without_key = self::bound(SCRIPT);
    let mut missing = signing(&bound_without_key, SCRIPT);
    missing.binding = BindingHint::KeyPresent;
    assert!(matches!(
        bound_without_key
            .runtime
            .covenant_prepare(&encode_covenant_request(&missing)),
        Err(VaultRuntimeError::InvalidCovenantContext)
    ));
}
