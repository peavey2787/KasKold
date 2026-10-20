//! Connected covenant restore-form and signing-protocol coverage.

use crate::runtime::data::AppData;

pub(super) fn backup_forms() -> bool {
    use kaskold_hardware_core::qr::classification::{decode_hex, is_covenant_hex, is_covenant_raw};
    let forms = [(b"COVB0".as_slice(), b"434f564230".as_slice()), (b"COVI0".as_slice(), b"434f564930".as_slice())];
    let mut decoded = [0u8; 16];
    for (raw, hex) in forms {
        if !is_covenant_raw(raw) || !is_covenant_hex(hex) {
            return false;
        }
        let Ok(len) = decode_hex(hex, &mut decoded) else {
            return false;
        };
        if &decoded[..len] != raw {
            return false;
        }
    }
    if is_covenant_raw(b"COVB") || is_covenant_hex(b"434f56GG") {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: COVENANT RESTORE RAW/HEX COVB/COVI BOUNDARIES PASS");
    true
}

pub(super) fn signing_protocol(ad: &mut AppData) -> bool {
    use shared_signer::covenant_sign;
    if !crate::services::wallet_session::install_workflow_backup_mnemonic_fixture(ad) {
        log!("KASKOLD_WORKFLOW_TESTS: COVENANT SIGN MNEMONIC FIXTURE FAIL");
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: COVENANT SIGN MNEMONIC FIXTURE READY");
    let mut request_wire = [0u8; 512];
    let mut reveal_wire = [0u8; covenant_sign::REVEAL_LEN];
    if !covenant_rejects_malformed(ad)
        || !covenant_opaque_round_trip(ad, &mut request_wire, &mut reveal_wire)
        || !covenant_known_round_trip(ad, &mut request_wire, &mut reveal_wire)
    {
        return false;
    }
    log!("KASKOLD_WORKFLOW_TESTS: COVENANT SIGN KEYINFO/KNOWN/BINDKNOWN/OPAQUE/BINDOPAQUE/REVEAL PASS");
    true
}

fn covenant_rejects_malformed(ad: &mut AppData) -> bool {
    crate::services::covenant_sign::prepare_request(ad, b"CVSG", &mut || {}).is_err()
}

fn covenant_key_info(ad: &mut AppData, request_wire: &mut [u8; 512]) -> Option<[u8; 32]> {
    use crate::runtime::data::CovenantSigningMode;
    use shared_signer::covenant_sign::{self as wire, BindingHint, CovenantSignRequest, KnownScheme, RequestKind};
    let request = CovenantSignRequest {
        kind: RequestKind::KeyInfo,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: [0; 32],
        binding_token: [0; 32],
        commitment: [0; 32],
        script: &[],
        context: &[],
    };
    let len = wire::encode_request(&request, request_wire).ok()?;
    crate::services::covenant_sign::prepare_request(ad, &request_wire[..len], &mut || {}).ok()?;
    (ad.signing.covenant.mode == CovenantSigningMode::KeyInfo && ad.signing.covenant.response_len > 0)
        .then_some(ad.signing.covenant.pending_key_id)
}

fn covenant_bind_opaque(
    ad: &mut AppData,
    request_wire: &mut [u8; 512],
    key_id: [u8; 32],
    script: &[u8],
) -> Option<[u8; 32]> {
    use crate::runtime::data::CovenantSigningMode;
    use shared_signer::covenant_sign::{self as wire, BindingHint, CovenantSignRequest, KnownScheme, RequestKind};
    let request = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id,
        binding_token: [0; 32],
        commitment: [0; 32],
        script,
        context: &[],
    };
    let len = wire::encode_request(&request, request_wire).ok()?;
    crate::services::covenant_sign::prepare_request(ad, &request_wire[..len], &mut || {}).ok()?;
    if ad.signing.covenant.mode != CovenantSigningMode::BindOpaque {
        return None;
    }
    crate::services::covenant_sign::complete_binding(ad, &mut || {}).ok()?;
    (ad.signing.covenant.binding_token != [0; 32]).then_some(ad.signing.covenant.binding_token)
}

fn covenant_opaque_round_trip(
    ad: &mut AppData,
    request_wire: &mut [u8; 512],
    reveal_wire: &mut [u8; shared_signer::covenant_sign::REVEAL_LEN],
) -> bool {
    use crate::runtime::data::{CovenantSigningMode, CovenantSigningPhase};
    use shared_signer::covenant_sign::{self as wire, BindingHint, CovenantSignRequest, KnownScheme, RequestKind};
    let Some(key_id) = covenant_key_info(ad, request_wire) else {
        return false;
    };
    let script = b"workflow opaque covenant";
    let Some(token) = covenant_bind_opaque(ad, request_wire, key_id, script) else {
        return false;
    };
    let host_secret = [0x39; 32];
    let session_id = [0x47; wire::SESSION_ID_LEN];
    let commitment = [0x51; 32];
    let request = CovenantSignRequest {
        kind: RequestKind::Opaque,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id,
        host_commitment: shared_signer::anti_klepto::host_commitment(&host_secret),
        key_id,
        binding_token: token,
        commitment,
        script,
        context: &[],
    };
    let Ok(len) = wire::encode_request(&request, request_wire) else {
        return false;
    };
    if crate::services::covenant_sign::prepare_request(ad, &request_wire[..len], &mut || {}).is_err()
        || ad.signing.covenant.mode != CovenantSigningMode::Opaque
        || crate::services::covenant_sign::begin_signing(ad, &mut || {}).is_err()
        || ad.signing.covenant.phase != CovenantSigningPhase::AwaitingReveal
    {
        return false;
    }
    covenant_reveal_matrix(ad, reveal_wire, session_id, key_id, commitment, host_secret)
}

fn covenant_reveal_matrix(
    ad: &mut AppData,
    reveal_wire: &mut [u8; shared_signer::covenant_sign::REVEAL_LEN],
    session_id: [u8; shared_signer::covenant_sign::SESSION_ID_LEN],
    key_id: [u8; 32],
    commitment: [u8; 32],
    host_secret: [u8; 32],
) -> bool {
    use crate::runtime::data::CovenantSigningPhase;
    use shared_signer::covenant_sign::{self as wire, CovenantSignReveal};
    if crate::services::covenant_sign::finalize_reveal(ad, b"CVRV", &mut || {}).is_ok()
        || ad.signing.covenant.phase != CovenantSigningPhase::AwaitingReveal
    {
        return false;
    }
    let wrong = CovenantSignReveal { session_id: [0x48; wire::SESSION_ID_LEN], key_id, commitment, host_secret };
    if wire::encode_reveal(&wrong, reveal_wire).is_err()
        || crate::services::covenant_sign::finalize_reveal(ad, reveal_wire, &mut || {}).is_ok()
        || ad.signing.covenant.phase != CovenantSigningPhase::AwaitingReveal
    {
        return false;
    }
    let valid = CovenantSignReveal { session_id, key_id, commitment, host_secret };
    wire::encode_reveal(&valid, reveal_wire).is_ok()
        && crate::services::covenant_sign::finalize_reveal(ad, reveal_wire, &mut || {}).is_ok()
        && ad.signing.covenant.phase == CovenantSigningPhase::FinalResponse
        && ad.signing.covenant.response_len > 0
}

fn covenant_known_round_trip(
    ad: &mut AppData,
    request_wire: &mut [u8; 512],
    reveal_wire: &mut [u8; shared_signer::covenant_sign::REVEAL_LEN],
) -> bool {
    use crate::runtime::data::{CovenantSigningMode, CovenantSigningPhase};
    use shared_signer::covenant_sign::{
        self as wire, BindingHint, CovenantSignRequest, CovenantSignReveal, KnownScheme, RequestKind,
    };
    let Some((key_id, commitment, script, token)) = prepare_known_covenant_binding(ad, request_wire) else {
        return false;
    };
    let context = b"workflow known covenant";
    let host_secret = [0x71; 32];
    let host_commitment = shared_signer::anti_klepto::host_commitment(&host_secret);
    let session_id = [0x72; wire::SESSION_ID_LEN];
    let known = CovenantSignRequest {
        kind: RequestKind::Known,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id,
        host_commitment,
        key_id,
        binding_token: token,
        commitment,
        script: &script,
        context,
    };
    let Ok(known_len) = wire::encode_request(&known, request_wire) else {
        return false;
    };
    if crate::services::covenant_sign::prepare_request(ad, &request_wire[..known_len], &mut || {}).is_err()
        || ad.signing.covenant.mode != CovenantSigningMode::Known
        || crate::services::covenant_sign::begin_signing(ad, &mut || {}).is_err()
    {
        return false;
    }
    let reveal = CovenantSignReveal { session_id, key_id, commitment, host_secret };
    wire::encode_reveal(&reveal, reveal_wire).is_ok()
        && crate::services::covenant_sign::finalize_reveal(ad, reveal_wire, &mut || {}).is_ok()
        && ad.signing.covenant.phase == CovenantSigningPhase::FinalResponse
}

fn prepare_known_covenant_binding(
    ad: &mut AppData,
    request_wire: &mut [u8; 512],
) -> Option<([u8; 32], [u8; 32], [u8; 67], [u8; 32])> {
    use crate::runtime::data::CovenantSigningMode;
    use shared_signer::covenant_sign::{self as wire, BindingHint, CovenantSignRequest, KnownScheme, RequestKind};
    let key_info = CovenantSignRequest {
        kind: RequestKind::KeyInfo,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: [0; 32],
        binding_token: [0; 32],
        commitment: [0; 32],
        script: &[],
        context: &[],
    };
    let Ok(key_len) = wire::encode_request(&key_info, request_wire) else {
        return None;
    };
    if crate::services::covenant_sign::prepare_request(ad, &request_wire[..key_len], &mut || {}).is_err() {
        return None;
    }
    let key_id = ad.signing.covenant.pending_key_id;
    let pubkey = ad.signing.covenant.pending_pubkey_x;
    let context = b"workflow known covenant";
    let Some(commitment) = wire::recompute_known_commitment(KnownScheme::Sha256Preimage, context) else {
        return None;
    };
    let mut script = [0u8; 67];
    script[0] = 0x20;
    script[1..33].copy_from_slice(&commitment);
    script[33] = 0x20;
    script[34..66].copy_from_slice(&pubkey);
    script[66] = 0xd7;
    let bind = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id,
        binding_token: [0; 32],
        commitment,
        script: &script,
        context,
    };
    let Ok(bind_len) = wire::encode_request(&bind, request_wire) else {
        return None;
    };
    if crate::services::covenant_sign::prepare_request(ad, &request_wire[..bind_len], &mut || {}).is_err()
        || ad.signing.covenant.mode != CovenantSigningMode::BindKnown
        || crate::services::covenant_sign::complete_binding(ad, &mut || {}).is_err()
    {
        return None;
    }
    let token = ad.signing.covenant.binding_token;
    Some((key_id, commitment, script, token))
}
