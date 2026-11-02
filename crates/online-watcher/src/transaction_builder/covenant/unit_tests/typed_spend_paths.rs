//! Spend paths whose witness carries more than a signature, and the paths that
//! continue a covenant, through the real signer and finalizer.

use kaskold_protocol::{companion::verify_complete_pskt, Network};
use serde_json::json;

use kaspa_portal::transaction::interchange::pskt::pipeline::{
    VerifiedCovenantRoute, VerifiedTransaction, VerifiedWitnessPlan,
};

use super::spend_paths::{
    destination, device_signs, funding, keys_with_wallet_as, signatures_per_input,
    signs_and_finalizes, stranger, wallet_key, FEE, LOCK_TIME,
};
use crate::{
    account::address::{address_to_script_pubkey, encode_p2pk_address},
    account::utxo::UtxoEntry,
    contracts::covenant::script,
    transaction_builder::{
        covenant::{oracle_v1, payjoin, private_swap, vault::spend as vault},
        pskb::{
            self,
            application::{encode_covenant_request, CovenantSweepRequest},
            prepare_selected_sweep, GlobalThreadFamily, WithdrawalBuildRequest,
        },
        zk::{commit_reveal, merkle},
    },
    wasm_api::test_support::covenant_address,
};

fn covenant_utxo(redeem: &[u8]) -> UtxoEntry {
    UtxoEntry {
        tx_id: "63".repeat(32),
        index: 0,
        amount: 30_000_000,
        script_public_key: address_to_script_pubkey(&covenant_address(redeem)).expect("spk"),
        block_daa_score: 0,
        covenant_id: None,
    }
}

/// The x-only public key of a secret.
fn xonly_of(secret: &[u8; 32]) -> [u8; 32] {
    use k256::elliptic_curve::sec1::ToSec1Point;
    let public = k256::SecretKey::from_slice(secret)
        .expect("secret key")
        .public_key();
    public.to_sec1_point(true).as_bytes()[1..]
        .try_into()
        .expect("x-only key")
}

fn route_of(spend: &VerifiedTransaction) -> VerifiedCovenantRoute {
    match spend.inputs()[0].witness() {
        VerifiedWitnessPlan::SpecializedCovenant { route, .. } => *route,
        other => panic!("not a specialized route: {other:?}"),
    }
}

#[test]
fn a_private_swap_claim_is_signed_by_its_claimer() {
    let target = destination();
    let redeem = script::build_private_swap_script(
        &stranger(20),
        &wallet_key(),
        &address_to_script_pubkey(&target).expect("destination"),
        LOCK_TIME,
        &[4; 16],
    )
    .expect("swap script");
    let wire = private_swap::build_claim(
        &covenant_address(&redeem),
        &target,
        &hex::encode(&redeem),
        &funding(),
        FEE,
    )
    .expect("swap claim");
    assert_eq!(
        route_of(&signs_and_finalizes(&wire)),
        VerifiedCovenantRoute::PrivateSwapClaim
    );
}

#[test]
fn an_oracle_claim_carries_the_attestation_and_the_beneficiary_signature() {
    let oracle_secret = [3u8; 32];
    let oracle = xonly_of(&oracle_secret);
    let commitment = [0x44; 32];
    let attestation = offline_signer::crypto::schnorr::schnorr_sign(&oracle_secret, &commitment)
        .expect("attestation");
    let redeem = script::build_oracle_v1_covenant_script(
        &stranger(21),
        &wallet_key(),
        &oracle,
        &commitment,
        LOCK_TIME,
        &[5; 16],
    );
    let address = covenant_address(&redeem);
    let prepared = prepare_selected_sweep(&funding(), &address, &destination(), FEE, "", "")
        .expect("oracle funding");
    let wire = oracle_v1::encode_claim(&prepared, &redeem, &hex::encode(attestation.bytes))
        .expect("oracle claim");
    assert_eq!(
        route_of(&signs_and_finalizes(&wire)),
        VerifiedCovenantRoute::OracleV1Claim
    );
}

#[test]
fn a_commit_reveal_claim_carries_the_preimage_and_the_claimer_signature() {
    let (part_a, part_b) = ([0x61u8; 8], [0x62u8; 8]);
    let committed = kaspa_portal::primitives::hash::blake2b_256(&[part_a, part_b].concat());
    let redeem = crate::contracts::commit_reveal::script::build_commit_reveal_script(
        &wallet_key(),
        &committed,
        LOCK_TIME,
    );
    let address = covenant_address(&redeem);
    let target = destination();
    let parsed = commit_reveal::parse(
        &json!({
            "covenant_address": address,
            "dest_address": target,
            "redeem_script_hex": hex::encode(&redeem),
            "part_a_hex": hex::encode(part_a),
            "part_b_hex": hex::encode(part_b),
            "payload_hex": "",
            "fee": FEE.to_string(),
            "ws_url": "ws://unused",
        })
        .to_string(),
    )
    .expect("commit-reveal request");
    let request: CovenantSweepRequest<'_> = commit_reveal::sweep_request(&parsed);
    let prepared = prepare_selected_sweep(&funding(), &address, &target, FEE, "", "")
        .expect("commit-reveal funding");
    let wire = encode_covenant_request(&prepared, request).expect("commit-reveal claim");
    assert_eq!(
        route_of(&signs_and_finalizes(&wire)),
        VerifiedCovenantRoute::CommitRevealClaim
    );
}

#[test]
fn a_merkle_spend_proves_its_destination_and_carries_the_owner_signature() {
    let whitelist = [destination(), encode_p2pk_address(&stranger(22), "kaspa")];
    let addresses = serde_json::to_string(&whitelist).expect("addresses");
    let root: serde_json::Value = serde_json::from_str(
        &crate::contracts::merkle::application::root_from_addresses(&addresses).expect("root"),
    )
    .expect("root JSON");
    let proof_json =
        crate::contracts::merkle::application::proof_for_address(&addresses, &whitelist[0])
            .expect("proof");
    let proof: serde_json::Value = serde_json::from_str(&proof_json).expect("proof JSON");
    let root_bytes: [u8; 32] = hex::decode(root["root"].as_str().expect("root"))
        .expect("root hex")
        .try_into()
        .expect("root length");
    let redeem = crate::contracts::merkle::script::build_merkle_whitelist_script(
        &wallet_key(),
        &root_bytes,
        u8::try_from(root["depth"].as_u64().expect("depth")).expect("depth"),
        LOCK_TIME,
    );
    let address = covenant_address(&redeem);
    let redeem_hex = hex::encode(&redeem);
    let proof_text = proof["proof"].to_string();
    let mut request = merkle::MerkleSpendRequest {
        covenant_address: &address,
        destination_address: &whitelist[0],
        redeem_script_hex: &redeem_hex,
        proof_json: &proof_text,
        send_amount: 20_000_000,
        requested_fee: FEE,
        utxos: vec![covenant_utxo(&redeem)],
    };
    let prepared = merkle::build_merkle_whitelist_spend(&mut request).expect("merkle spend");
    assert_eq!(
        route_of(&signs_and_finalizes(&prepared.wire)),
        VerifiedCovenantRoute::MerkleClaim
    );
}

#[test]
fn a_payjoin_claim_is_signed_on_both_its_inputs_by_the_beneficiary() {
    let owner = stranger(23);
    let redeem = script::build_payjoin_covenant_script(&owner, &wallet_key(), LOCK_TIME, 2, 1);
    let mixing = encode_p2pk_address(&wallet_key(), "kaspa");
    let mixing_utxo = UtxoEntry {
        tx_id: "64".repeat(32),
        index: 1,
        amount: 2_000_000,
        script_public_key: address_to_script_pubkey(&mixing).expect("mixing script"),
        block_daa_score: 0,
        covenant_id: None,
    };
    let claim = payjoin::build_claim(
        &covenant_address(&redeem),
        &destination(),
        &hex::encode(&redeem),
        &mixing,
        FEE,
        vec![covenant_utxo(&redeem)],
        mixing_utxo,
    )
    .expect("payjoin claim");
    let merged = device_signs(&claim.wire, 2);
    let spend = verify_complete_pskt(&merged, Network::Mainnet).expect("payjoin finalizes");
    assert_eq!(signatures_per_input(&spend), [1, 1]);
}

#[test]
fn a_vault_continues_or_splits_under_its_owners_signature() {
    for (redeem, split) in [
        (
            crate::contracts::vault::script::build_tagged_vault_script(&wallet_key()),
            false,
        ),
        (
            crate::contracts::vault::script::build_split_vault_script(&wallet_key()),
            true,
        ),
    ] {
        let prepared = vault::prepare_from_utxos(
            &"22".repeat(32),
            &redeem,
            FEE,
            "mainnet",
            vec![covenant_utxo(&redeem)],
        )
        .expect("vault funding");
        let (wire, _, _) =
            vault::encode_vault_spend_pskb(prepared, &redeem, split).expect("vault spend");
        assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
    }
}

#[test]
fn a_spending_limit_withdrawal_is_signed_by_its_owner() {
    let redeem =
        script::build_global_spending_limit_script(&wallet_key(), 50_000_000, 100, &[6; 8]);
    let wire = pskb::build_global_thread_withdrawal(WithdrawalBuildRequest {
        family: GlobalThreadFamily::SpendingLimit,
        covenant_address: &covenant_address(&redeem),
        destination_address: &destination(),
        redeem_script_hex: &hex::encode(&redeem),
        covenant_id_hex: &"74".repeat(32),
        withdrawal: 20_000_000,
        fee: FEE,
        selected_utxos_json: &funding(),
    })
    .expect("spending-limit withdrawal")
    .wire;
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
}

#[test]
fn an_allowance_withdrawal_is_signed_by_its_beneficiary() {
    let [owner, beneficiary] = keys_with_wallet_as::<2>(1);
    let redeem = script::build_global_allowance_script(
        &owner,
        &beneficiary,
        50_000_000,
        100,
        LOCK_TIME,
        &[7; 8],
    );
    let wire = pskb::build_global_thread_withdrawal(WithdrawalBuildRequest {
        family: GlobalThreadFamily::Allowance,
        covenant_address: &covenant_address(&redeem),
        destination_address: &destination(),
        redeem_script_hex: &hex::encode(&redeem),
        covenant_id_hex: &"74".repeat(32),
        withdrawal: 20_000_000,
        fee: FEE,
        selected_utxos_json: &funding(),
    })
    .expect("allowance withdrawal")
    .wire;
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
}
