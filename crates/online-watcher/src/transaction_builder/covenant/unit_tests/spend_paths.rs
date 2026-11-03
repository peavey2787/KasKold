//! Every covenant spend path, through the real signer and finalizer.
//!
//! A plan is only worth building if a device can sign it and the Companion can
//! then finalize it, so each path runs the journey a real spend takes: relay to
//! KSPT, sign with the wallet's seed, merge the response, verify the result.

use offline_signer::{
    derivation::bip39::{mnemonic_from_entropy_12, seed_from_mnemonic_12},
    transaction::{
        kspt,
        model::{SigHashType, Transaction},
    },
};

use crate::{
    account::address::{address_to_script_pubkey, encode_p2pk_address},
    contracts::covenant::script,
    transaction_builder::{
        covenant::{
            execution::{execution_named, signatures_required, SpendPath},
            private_swap, spend_document,
            sweeps::{owner, savings, timelocked},
        },
        pskb::CovenantExecution,
    },
    wasm_api::test_support::{covenant_address, decode_pskb_document},
};

use kaskold_protocol::{companion::verify_complete_pskt, Network};
use kaspa_portal::transaction::interchange::pskt::pipeline::{
    VerifiedTransaction, VerifiedWitnessItem, VerifiedWitnessPlan,
};

/// The first receive key of the wallet whose seed signs these spends.
const WALLET_KEY: &str = "1bacea84ca721c95d67ecace19bc499a77c03726bc8739af637bcd89abaaf058";
pub(super) const FEE: u64 = 1_000_000;
pub(super) const LOCK_TIME: u64 = 500;

pub(super) fn wallet_key() -> [u8; 32] {
    hex::decode(WALLET_KEY)
        .expect("key hex")
        .try_into()
        .expect("key length")
}

pub(super) fn stranger(seed: u8) -> [u8; 32] {
    hex::decode(crate::wasm_api::test_support::xonly_key(seed))
        .expect("key hex")
        .try_into()
        .expect("key length")
}

/// Keys for a script's roles, with the wallet in exactly one of them.
pub(super) fn keys_with_wallet_as<const N: usize>(role: usize) -> [[u8; 32]; N] {
    let mut keys = [[0u8; 32]; N];
    for (index, key) in keys.iter_mut().enumerate() {
        *key = stranger(u8::try_from(index).expect("role") + 10);
    }
    keys[role] = wallet_key();
    keys
}

fn wallet_seed() -> [u8; 64] {
    let mnemonic = mnemonic_from_entropy_12(&[0u8; 16]);
    seed_from_mnemonic_12(&mnemonic, "").bytes
}

pub(super) fn destination() -> String {
    encode_p2pk_address(&stranger(99), "kaspa")
}

pub(super) fn funding() -> String {
    serde_json::json!([{"tx_id": "63".repeat(32), "index": 0, "amount": "30000000"}]).to_string()
}

pub(super) fn device_signs(wire: &str, expected_signatures: usize) -> String {
    let relayed = kaskold_protocol::encode_pskt_hex(wire, Network::Mainnet).expect("relay to KSPT");
    let mut transaction = Transaction::try_new().expect("transaction");
    kspt::parse_compact_kspt(&hex::decode(relayed).expect("KSPT hex"), &mut transaction)
        .expect("device imports the KSPT");
    let signed = kspt::sign_transaction_multisig(
        &mut transaction,
        &[(wallet_seed(), true)],
        SigHashType::All,
        Some(0),
    )
    .expect("device signs");
    assert_eq!(signed, expected_signatures, "signatures the wallet made");
    kaskold_protocol::merge_signed_kspt(
        wire,
        &kspt::serialize_compact_kspt_vec(&transaction).expect("signed KSPT"),
        Network::Mainnet,
    )
    .expect("signature merges into the PSKB")
}

pub(super) fn signs_and_finalizes(wire: &str) -> VerifiedTransaction {
    let merged = device_signs(wire, 1);
    verify_complete_pskt(&merged, Network::Mainnet).expect("signed spend finalizes")
}

/// How many signatures each input of a finalized spend gives consensus.
pub(super) fn signatures_per_input(spend: &VerifiedTransaction) -> Vec<usize> {
    spend
        .inputs()
        .iter()
        .map(|input| match input.witness() {
            VerifiedWitnessPlan::P2pk { .. } => 1,
            VerifiedWitnessPlan::Multisig { signatures, .. } => signatures.len(),
            VerifiedWitnessPlan::Covenant { witness, .. } => witness
                .iter()
                .filter(|item| matches!(item, VerifiedWitnessItem::Signature(_)))
                .count(),
            VerifiedWitnessPlan::SpecializedCovenant { .. } => {
                panic!("a named path never finalizes through a specialized route")
            }
        })
        .collect()
}

/// A spend the Companion builds from a named path, as its escrow screens do.
fn named_path_spend(redeem: &[u8], path: &str, lock_time: u64) -> String {
    spend_with_payload(redeem, path, lock_time, "")
}

fn spend_with_payload(redeem: &[u8], path: &str, lock_time: u64, payload_hex: &str) -> String {
    spend_document::encode_json(
        &serde_json::json!({
            "covenant_address": covenant_address(redeem),
            "redeem_script_hex": hex::encode(redeem),
            "path": path,
            "tx_version": 1,
            "lock_time": lock_time.to_string(),
            "payload_hex": payload_hex,
            "utxos": [{ "tx_id": "63".repeat(32), "index": 0, "amount": "30000000" }],
            "outputs": [{ "address": destination(), "amount": "29000000" }],
        })
        .to_string(),
    )
    .expect(path)
}

#[test]
fn a_dead_mans_switch_is_spent_by_its_owner_and_by_its_heir() {
    let [owner_key, heir] = keys_with_wallet_as::<2>(0);
    let redeem = script::build_dms_csv_script(&owner_key, &heir, 100);
    let (_, wire, _) = owner::build_selected(
        &covenant_address(&redeem),
        &destination(),
        &hex::encode(&redeem),
        &funding(),
        FEE,
        "owner",
    )
    .expect("owner spend");
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);

    let [owner_key, heir] = keys_with_wallet_as::<2>(1);
    let redeem = script::build_dms_csv_script(&owner_key, &heir, 100);
    let (_, wire, _) = timelocked::build_beneficiary_selected(
        &covenant_address(&redeem),
        &destination(),
        &hex::encode(&redeem),
        LOCK_TIME,
        &funding(),
        FEE,
    )
    .expect("heir spend");
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
}

#[test]
fn timelocked_savings_are_claimed_by_either_chosen_wallet() {
    for position in 0..2u8 {
        let [first, second] = keys_with_wallet_as::<2>(usize::from(position));
        let redeem = script::build_timelocked_savings_script(&first, &second, LOCK_TIME);
        let (_, wire) = savings::build_selected(
            &covenant_address(&redeem),
            &destination(),
            &hex::encode(&redeem),
            LOCK_TIME,
            position,
            &funding(),
            FEE,
        )
        .expect("savings claim");
        assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
    }
}

#[test]
fn a_piggy_bank_opens_at_its_goal_and_at_its_deadline() {
    let redeem = script::build_piggy_bank_script(&wallet_key(), 10_000_000, LOCK_TIME, &[1; 8]);
    let (_, wire, lock_time) = owner::build_selected(
        &covenant_address(&redeem),
        &destination(),
        &hex::encode(&redeem),
        &funding(),
        FEE,
        "owner",
    )
    .expect("goal spend");
    assert_eq!(lock_time, 0);
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);

    let (_, wire, lock_time) = owner::build_selected(
        &covenant_address(&redeem),
        &destination(),
        &hex::encode(&redeem),
        &funding(),
        FEE,
        "owner-time",
    )
    .expect("deadline spend");
    assert_eq!(lock_time, LOCK_TIME);
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
}

fn escrow(keys: &[[u8; 32]; 3]) -> Vec<u8> {
    let alice_spk = address_to_script_pubkey(&encode_p2pk_address(&keys[0], "kaspa")).expect("spk");
    let bob_spk = address_to_script_pubkey(&encode_p2pk_address(&keys[1], "kaspa")).expect("spk");
    script::build_escrow_script(&keys[0], &keys[1], &keys[2], &alice_spk, &bob_spk, &[2; 8])
}

#[test]
fn every_escrow_path_is_signed_by_the_party_it_names() {
    const BUYER: usize = 0;
    const SELLER: usize = 1;
    const ARBITER: usize = 2;
    for (path, role) in [
        ("buyer-release", BUYER),
        ("seller-refund", SELLER),
        ("arbiter-award-seller", ARBITER),
        ("arbiter-refund-buyer", ARBITER),
        ("buyer-dispute", BUYER),
        ("seller-dispute", SELLER),
    ] {
        let redeem = escrow(&keys_with_wallet_as::<3>(role));
        let spend = signs_and_finalizes(&named_path_spend(&redeem, path, 0));
        assert_eq!(signatures_per_input(&spend), [1], "{path}");
    }
}

fn shipment(keys: &[[u8; 32]; 4]) -> Vec<u8> {
    crate::contracts::shipping_escrow::script::build_ship_escrow_script(
        crate::contracts::shipping_escrow::script::ShippingEscrowScriptRequest {
            seller_pubkey: &keys[0],
            deliverer_pubkey: &keys[1],
            buyer_pubkey: &keys[2],
            arbiter_pubkey: &keys[3],
            product_sompi: 20_000_000,
            fee_sompi: 5_000_000,
            cltv1_deadline: LOCK_TIME,
            cltv2_deadline: LOCK_TIME * 2,
            salt: &[3; 8],
        },
    )
    .expect("shipment script")
}

#[test]
fn every_signed_shipment_path_is_signed_by_the_party_it_names() {
    const DELIVERER: usize = 1;
    const BUYER: usize = 2;
    const ARBITER: usize = 3;
    for (path, role) in [
        ("pickup", DELIVERER),
        ("state0-arb-refund", ARBITER),
        ("delivery", BUYER),
        ("state1-arb-award", ARBITER),
        ("state1-arb-refund", ARBITER),
    ] {
        let redeem = shipment(&keys_with_wallet_as::<4>(role));
        let spend = signs_and_finalizes(&named_path_spend(&redeem, path, 0));
        assert_eq!(signatures_per_input(&spend), [1], "{path}");
    }
}

#[test]
fn shipment_timeouts_finalize_with_no_signature_at_all() {
    let redeem = shipment(&keys_with_wallet_as::<4>(0));
    for path in ["state0-timeout", "state1-timeout"] {
        let wire = named_path_spend(&redeem, path, LOCK_TIME);
        let spend = verify_complete_pskt(&wire, Network::Mainnet).expect(path);
        assert_eq!(signatures_per_input(&spend), [0], "{path}");
    }
}

#[test]
fn each_owner_route_supplies_the_selectors_its_party_spends_through() {
    let piggy_bank = script::build_piggy_bank_script(&wallet_key(), 10_000_000, LOCK_TIME, &[1; 8]);
    let switch = script::build_dms_csv_script(&wallet_key(), &stranger(21), 100);
    let shipment = shipment(&keys_with_wallet_as::<4>(0));
    let supplying = |supplied_mask, supplied_true_mask| {
        Ok::<_, String>(CovenantExecution {
            supplied_mask,
            supplied_true_mask,
        })
    };
    assert_eq!(execution_named(&piggy_bank, "owner"), supplying(0b11, 0b11));
    assert_eq!(
        execution_named(&piggy_bank, "owner-time"),
        supplying(0b11, 0b01)
    );
    assert_eq!(execution_named(&switch, "owner"), supplying(0b1, 0b1));
    assert_eq!(execution_named(&switch, "beneficiary"), supplying(0b1, 0b0));
    assert_eq!(execution_named(&shipment, ""), supplying(0x7f, 0x7f));
}

#[test]
fn a_spend_signs_only_as_often_as_its_path_checks() {
    let redeem = shipment(&keys_with_wallet_as::<4>(0));
    for (path, signatures) in [("state0-timeout", 0), ("pickup", 1)] {
        let execution = execution_named(&redeem, path).expect(path);
        assert_eq!(
            signatures_required(&redeem, &execution),
            signatures,
            "{path}"
        );
    }
}

#[test]
fn a_spend_document_stamps_a_lock_time_and_payload_only_when_it_has_them() {
    let redeem = shipment(&keys_with_wallet_as::<4>(0));
    let global = |lock_time, payload_hex| {
        decode_pskb_document(&spend_with_payload(
            &redeem,
            "pickup",
            lock_time,
            payload_hex,
        ))[0]["global"]
            .clone()
    };
    let bare = global(0, "");
    assert_eq!(bare["fallbackLockTime"], serde_json::Value::Null);
    assert_eq!(bare.get("txPayload"), None);
    let stamped = global(LOCK_TIME, "abcd");
    assert_eq!(stamped["fallbackLockTime"], LOCK_TIME.to_string());
    assert_eq!(stamped["txPayload"], "abcd");
}

#[test]
fn a_spend_document_cannot_pay_out_more_than_it_spends() {
    let redeem = shipment(&keys_with_wallet_as::<4>(0));
    let request = |outputs: serde_json::Value, payload: &str| {
        serde_json::json!({
            "covenant_address": covenant_address(&redeem),
            "redeem_script_hex": hex::encode(&redeem),
            "path": "pickup",
            "tx_version": 1,
            "lock_time": "0",
            "payload_hex": payload,
            "utxos": [{ "tx_id": "63".repeat(32), "index": 0, "amount": "30000000" }],
            "outputs": outputs,
        })
        .to_string()
    };
    let paying = |amount: &str| serde_json::json!([{ "address": destination(), "amount": amount }]);
    assert_eq!(
        spend_document::encode_json(&request(paying("30000001"), ""))
            .err()
            .as_deref(),
        Some("Covenant spend outputs exceed its inputs")
    );
    assert_eq!(
        spend_document::encode_json(&request(serde_json::json!([]), ""))
            .err()
            .as_deref(),
        Some("A covenant spend needs an output")
    );
    assert!(
        spend_document::encode_json(&request(paying("30000000"), "zz"))
            .err()
            .is_some_and(|error| error.starts_with("Bad covenant spend payload"))
    );
    assert!(spend_document::encode_json("{}")
        .err()
        .is_some_and(|error| error.starts_with("Bad covenant spend JSON")));
}

#[test]
fn a_private_swap_refunds_its_owner_after_the_lock_time() {
    let target = destination();
    let redeem = script::build_private_swap_script(
        &wallet_key(),
        &stranger(20),
        &address_to_script_pubkey(&target).expect("destination"),
        LOCK_TIME,
        &[4; 16],
    )
    .expect("swap script");
    let wire = private_swap::build_refund(
        &covenant_address(&redeem),
        &target,
        &hex::encode(&redeem),
        &funding(),
        FEE,
    )
    .expect("swap refund");
    assert_eq!(signatures_per_input(&signs_and_finalizes(&wire)), [1]);
}

#[test]
fn an_unknown_path_and_a_foreign_redeem_script_never_become_a_plan() {
    let redeem = escrow(&keys_with_wallet_as::<3>(0));
    assert_eq!(
        execution_named(&redeem, "pickup").err().as_deref(),
        Some("This covenant has no pickup path")
    );

    let other = script::build_dms_csv_script(&wallet_key(), &stranger(21), 100);
    assert_eq!(
        owner::build_selected(
            &covenant_address(&other),
            &destination(),
            &hex::encode(&redeem),
            &funding(),
            FEE,
            "owner",
        )
        .err()
        .as_deref(),
        Some("Covenant address does not match the supplied redeem script")
    );
    assert_eq!(
        crate::transaction_builder::covenant::execution::execution_for(&redeem, SpendPath::Key(99))
            .err()
            .as_deref(),
        Some("Covenant script has no such key: PositionOutOfRange")
    );
}
