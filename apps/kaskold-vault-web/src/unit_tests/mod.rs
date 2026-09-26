use super::{
    hex_encode, kaskold_vault_export_kpub, kaskold_vault_is_unlocked, kaskold_vault_lock,
    kaskold_vault_restore, qr_svg,
};

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn restore_export_and_lock_keep_the_web_shell_public_only() {
    kaskold_vault_lock();
    assert!(!kaskold_vault_is_unlocked());

    let restored = kaskold_vault_restore(MNEMONIC, "").expect("known mnemonic restores");
    let exported = kaskold_vault_export_kpub().expect("unlocked vault exports kpub");
    let restored: serde_json::Value = serde_json::from_str(&restored).expect("restore JSON");
    let exported: serde_json::Value = serde_json::from_str(&exported).expect("export JSON");
    assert_eq!(restored["kpub"], exported["kpub"]);
    assert!(restored["kpub"].as_str().is_some_and(|value| !value.is_empty()));
    assert!(kaskold_vault_is_unlocked());

    kaskold_vault_lock();
    assert!(!kaskold_vault_is_unlocked());
}


#[test]
fn web_wallet_labels_and_switch_cover_inventory_presentation() {
    use super::workflows::{kaskold_vault_switch_wallet, wallet_kind_label};

    assert_eq!(wallet_kind_label(vault_runtime::WalletKind::Mnemonic), "Mnemonic");
    assert_eq!(
        wallet_kind_label(vault_runtime::WalletKind::AccountXprv),
        "Account XPrv"
    );
    assert_eq!(
        wallet_kind_label(vault_runtime::WalletKind::RawPrivateKey),
        "Raw Private Key"
    );

    kaskold_vault_lock();
    kaskold_vault_restore(MNEMONIC, "").expect("known mnemonic restores");
    let active = kaskold_vault_switch_wallet(0).expect("single restored wallet switches");
    let active: serde_json::Value = serde_json::from_str(&active).expect("wallet JSON");
    assert_eq!(active["index"].as_u64(), Some(0));
    assert_eq!(active["kind"].as_str(), Some("Mnemonic"));
    assert_eq!(active["active"].as_bool(), Some(true));
    kaskold_vault_lock();
}

#[test]
fn qr_and_hex_helpers_are_deterministic_and_self_contained() {
    assert_eq!(hex_encode(&[0x00, 0x0f, 0x10, 0xab, 0xff]), "000f10abff");
    let first = qr_svg(b"KasKold Vault Web").expect("QR renders");
    let second = qr_svg(b"KasKold Vault Web").expect("QR renders deterministically");
    assert_eq!(first, second);
    assert!(first.starts_with("<svg "));
    assert!(first.contains("shape-rendering=\"crispEdges\""));
    assert!(first.ends_with("</svg>"));
}

#[test]
fn profiled_qr_framing_covers_single_multi_and_rejected_profiles() {
    use super::qr::{render_profiled_response_frames, validate_profiled_payload};

    assert!(validate_profiled_payload(&[], 91).is_err());
    assert!(validate_profiled_payload(b"payload", 99).is_err());

    let single = render_profiled_response_frames(&[0x41; 100], 91).expect("single frame");
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].index, 0);
    assert_eq!(single[0].total, 1);

    let multi = render_profiled_response_frames(&[0x42; 180], 70).expect("multi frame");
    assert!(multi.len() >= 2);
    assert!(multi.iter().all(|frame| frame.total as usize == multi.len()));
    for (index, frame) in multi.iter().enumerate() {
        assert_eq!(frame.index as usize, index);
        assert!(!frame.payload_hex.is_empty());
        assert!(frame.svg.starts_with("<svg "));
    }
}

#[test]
fn web_hex_decoder_covers_valid_and_rejected_payloads() {
    use super::workflows::decode_hex_bytes;
    assert_eq!(decode_hex_bytes("00 af FF").unwrap(), vec![0x00, 0xaf, 0xff]);
    assert!(decode_hex_bytes("").is_err());
    assert!(decode_hex_bytes("0").is_err());
    assert!(decode_hex_bytes("zz").is_err());
}

#[test]
fn web_covenant_labels_and_response_json_cover_every_response_shape() {
    use shared_signer::covenant_sign::{
        encode_response, CovenantSignResponse, KnownScheme, ResponseKind, RESPONSE_LEN,
        SESSION_ID_LEN,
    };
    use super::covenant::{mode_label, response_json, scheme_label};

    for (mode, label) in [
        (vault_runtime::CovenantMode::None, "None"),
        (vault_runtime::CovenantMode::KeyInfo, "KeyInfo"),
        (vault_runtime::CovenantMode::BindKnown, "BindKnown"),
        (vault_runtime::CovenantMode::BindOpaque, "BindOpaque"),
        (vault_runtime::CovenantMode::Known, "Known"),
        (vault_runtime::CovenantMode::Opaque, "Opaque"),
    ] {
        assert_eq!(mode_label(mode), label);
    }
    assert_eq!(scheme_label(KnownScheme::None), "Opaque");
    assert_eq!(scheme_label(KnownScheme::Sha256Preimage), "SHA256 Preimage");
    assert_eq!(scheme_label(KnownScheme::OracleV1), "Oracle v1");

    for kind in [
        ResponseKind::KeyInfo,
        ResponseKind::Binding,
        ResponseKind::NonceCommitment,
        ResponseKind::Signature,
    ] {
        let session_id = if matches!(kind, ResponseKind::NonceCommitment | ResponseKind::Signature) {
            [1; SESSION_ID_LEN]
        } else {
            [0; SESSION_ID_LEN]
        };
        let binding_token = if kind == ResponseKind::KeyInfo { [0; 32] } else { [4; 32] };
        let commitment = if kind == ResponseKind::KeyInfo { [0; 32] } else { [5; 32] };
        let nonce_point = if matches!(kind, ResponseKind::NonceCommitment | ResponseKind::Signature) {
            let mut point = [0; 33];
            point[0] = 0x02;
            point[1] = 6;
            point
        } else {
            [0; 33]
        };
        let signature = if kind == ResponseKind::Signature { [7; 64] } else { [0; 64] };
        let response = CovenantSignResponse {
            kind,
            session_id,
            key_id: [2; 32],
            pubkey_x: [3; 32],
            binding_token,
            commitment,
            nonce_point,
            signature,
        };
        let mut wire = [0u8; RESPONSE_LEN];
        let len = encode_response(&response, &mut wire).expect("response encode");
        let json: serde_json::Value = serde_json::from_str(
            &response_json(&wire[..len]).expect("response JSON"),
        )
        .expect("response JSON parses");
        assert!(json["kind"].as_str().is_some());
    }
}

#[test]
fn web_private_swap_labels_and_response_json_cover_every_response_shape() {
    use shared_signer::covenant_sign::private_swap::{
        encode_response, PrivateSwapResponse, ResponseKind, RESPONSE_LEN,
    };
    use super::private_swap::{mode_label, response_json, response_kind_label};

    for (mode, label) in [
        (vault_runtime::PrivateSwapMode::None, "None"),
        (vault_runtime::PrivateSwapMode::KeyInfo, "KeyInfo"),
        (vault_runtime::PrivateSwapMode::Bind, "Bind"),
        (vault_runtime::PrivateSwapMode::PreSign, "PreSign"),
        (vault_runtime::PrivateSwapMode::Complete, "Complete"),
    ] {
        assert_eq!(mode_label(mode), label);
    }

    for kind in [
        ResponseKind::KeyInfo,
        ResponseKind::Binding,
        ResponseKind::Nonce,
        ResponseKind::PreSignature,
        ResponseKind::Completed,
    ] {
        assert!(!response_kind_label(kind).is_empty());
        let mut response = PrivateSwapResponse {
            kind,
            session_id: [0; 16],
            key_id: [2; 32],
            claim_pubkey: [3; 32],
            binding_token: [0; 32],
            adaptor_point: [4; 32],
            commitment: [0; 32],
            nonce_point: [0; 33],
            signature: [0; 64],
            negated: false,
        };
        match kind {
            ResponseKind::KeyInfo => {}
            ResponseKind::Binding => {
                response.binding_token = [5; 32];
                response.commitment = [6; 32];
            }
            ResponseKind::Nonce => {
                response.session_id = [7; 16];
                response.binding_token = [5; 32];
                response.commitment = [6; 32];
                response.nonce_point = [8; 33];
            }
            ResponseKind::PreSignature => {
                response.session_id = [7; 16];
                response.binding_token = [5; 32];
                response.commitment = [6; 32];
                response.nonce_point = [8; 33];
                response.signature = [9; 64];
                response.negated = true;
            }
            ResponseKind::Completed => {
                response.binding_token = [5; 32];
                response.commitment = [6; 32];
                response.signature = [9; 64];
            }
        }
        let mut wire = [0u8; RESPONSE_LEN];
        let len = encode_response(&response, &mut wire).expect("private swap response encode");
        let json: serde_json::Value = serde_json::from_str(
            &response_json(&wire[..len]).expect("private swap response JSON"),
        )
        .expect("private swap JSON parses");
        assert!(json["kind"].as_str().is_some());
    }
}

#[test]
fn web_security_policy_validates_host_time_before_runtime_policy() {
    use super::security::{kaskold_vault_check_session_signing_policy, validate_host_time};
    assert!(validate_host_time(f64::NAN).is_err());
    assert!(validate_host_time(-1.0).is_err());
    assert_eq!(validate_host_time(0.0), Ok(0));
    assert!(kaskold_vault_check_session_signing_policy(0.0).is_ok());
}
