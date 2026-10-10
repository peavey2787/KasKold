mod function_coverage;
mod sweep_preparation;

use crate::account::utxo::UtxoEntry;

use super::pskb::{
    global_thread::{
        plan_global_thread_topup, plan_global_thread_withdrawal, GlobalThreadPlanError,
        GlobalThreadTopupRequest, GlobalThreadWithdrawalRequest,
    },
    GlobalThreadPolicy,
};

fn utxo(transaction_byte: u8, index: u32, amount: u64) -> UtxoEntry {
    UtxoEntry {
        tx_id: format!("{transaction_byte:02x}").repeat(32),
        index,
        amount,
        script_public_key: vec![0x20; 34],
        block_daa_score: 0,
        covenant_id: None,
    }
}

fn decode_pskb_wire(wire: &str) -> serde_json::Value {
    let envelope = hex::decode(wire).expect("outer hex");
    assert_eq!(&envelope[..4], b"PSKB");
    let json_hex = core::str::from_utf8(&envelope[4..]).expect("JSON hex text");
    let json_bytes = hex::decode(json_hex).expect("JSON hex");
    serde_json::from_slice(&json_bytes).expect("PSKB JSON")
}

#[test]
fn typed_pskb_sweep_preserves_contract_metadata() {
    let inputs = vec![utxo(0x44, 7, 50_000)];
    let mut global = super::pskb::PskbGlobalPlan::standard()
        .with_lock_time(123)
        .with_branch("beneficiary");
    global.transaction_payload = Some(b"payload".to_vec());
    let policy = super::pskb::SweepInputPolicy::covenant(
        &[0x51, 0x75],
        9,
        serde_json::json!({"proof": "abcd"}),
    );
    let plan = super::pskb::plan_sweep(
        &inputs,
        &[0xaa, 0xbb],
        &[0xcc, 0xdd],
        49_000,
        global,
        &policy,
    );
    let document = decode_pskb_wire(&super::pskb::encode_wire(&plan).expect("encode"));
    let pskt = &document[0];

    assert_eq!(pskt["global"]["txVersion"], 0);
    assert_eq!(pskt["global"]["subnetworkId"], "00".repeat(20));
    assert_eq!(pskt["global"]["fallbackLockTime"], "123");
    assert_eq!(pskt["global"]["covenantBranch"], "beneficiary");
    assert_eq!(pskt["global"]["txPayload"], hex::encode(b"payload"));
    assert_eq!(pskt["inputs"][0]["sequence"], "9");
    assert_eq!(pskt["inputs"][0]["redeemScript"], "5175");
    assert_eq!(pskt["inputs"][0]["proprietaries"]["proof"], "abcd");
    assert_eq!(pskt["outputs"][0]["amount"], "49000");
    assert_eq!(pskt["outputs"][0]["scriptPublicKey"], "0000ccdd");
}

#[test]
fn pskb_output_binding_field_is_explicitly_configurable() {
    let plan = super::pskb::PskbPlan {
        global: super::pskb::PskbGlobalPlan::standard(),
        inputs: vec![super::pskb::PskbInputPlan::p2pk(
            utxo(0x55, 1, 10),
            &[0x20],
            serde_json::json!({}),
        )],
        outputs: vec![super::pskb::PskbOutputPlan::plain(9, &[0x21])
            .with_binding_field(serde_json::Value::Null)],
    };
    let document = decode_pskb_wire(&super::pskb::encode_wire(&plan).expect("encode"));
    assert!(document[0]["outputs"][0]
        .as_object()
        .expect("output object")
        .contains_key("covenantBinding"));
    assert!(document[0]["outputs"][0]["covenantBinding"].is_null());
}

#[test]
fn typed_sweep_matches_the_browser_pskb_shape() {
    let inputs = vec![utxo(0x66, 3, 42_000)];
    let global = super::pskb::PskbGlobalPlan::standard()
        .with_lock_time(77)
        .with_branch("savings");
    let policy = super::pskb::SweepInputPolicy::covenant(&[0x51, 0xac], 0, serde_json::json!({}));
    let typed = super::pskb::plan_sweep(
        &inputs,
        &[0xaa, 0xbb],
        &[0xcc, 0xdd],
        41_000,
        global,
        &policy,
    );

    let source = serde_json::json!({
        "global": {
            "version": 0,
            "txVersion": 0,
            "fallbackLockTime": "77",
            "covenantBranch": "savings",
            "inputsModifiable": false,
            "outputsModifiable": false,
            "inputCount": 1,
            "outputCount": 1,
            "xpubs": {},
            "proprietaries": {}
        },
        "inputs": [{
            "previousOutpoint": {
                "transactionId": inputs[0].tx_id,
                "index": inputs[0].index
            },
            "sequence": "0",
            "sighashType": 1,
            "sigOpCount": 1,
            "utxoEntry": {
                "amount": inputs[0].amount.to_string(),
                "scriptPublicKey": "0000aabb",
                "blockDaaScore": "0",
                "isCoinbase": false
            },
            "redeemScript": "51ac",
            "partialSigs": {},
            "minimumSignatures": 1,
            "bip32Derivations": {},
            "proprietaries": {},
            "finalScriptSig": serde_json::Value::Null,
            "minTime": "0"
        }],
        "outputs": [{
            "amount": "41000",
            "scriptPublicKey": "0000ccdd",
            "bip32Derivations": {},
            "proprietaries": {}
        }]
    });

    assert_eq!(
        super::pskb::encode_wire(&typed).expect("typed wire"),
        super::pskb::encode_pskt_value(source).expect("source wire")
    );
}

#[test]
fn typed_p2pk_sweep_matches_the_stealth_pskb_shape() {
    let inputs = vec![utxo(0x77, 4, 60_000)];
    let policy =
        super::pskb::SweepInputPolicy::p2pk(serde_json::json!({ "stealthTweak": "aa".repeat(32) }));
    let typed = super::pskb::plan_sweep(
        &inputs,
        &[0x11, 0x22],
        &[0x33, 0x44],
        59_000,
        super::pskb::PskbGlobalPlan::standard(),
        &policy,
    );

    let source = serde_json::json!({
        "global": {
            "version": 0,
            "txVersion": 0,
            "fallbackLockTime": serde_json::Value::Null,
            "inputsModifiable": false,
            "outputsModifiable": false,
            "inputCount": 1,
            "outputCount": 1,
            "xpubs": {},
            "proprietaries": {}
        },
        "inputs": [{
            "previousOutpoint": {
                "transactionId": inputs[0].tx_id,
                "index": inputs[0].index
            },
            "sequence": "0",
            "sighashType": 1,
            "sigOpCount": 1,
            "utxoEntry": {
                "amount": inputs[0].amount.to_string(),
                "scriptPublicKey": "00001122",
                "blockDaaScore": "0",
                "isCoinbase": false
            },
            "redeemScript": serde_json::Value::Null,
            "partialSigs": {},
            "minimumSignatures": 1,
            "bip32Derivations": {},
            "proprietaries": { "stealthTweak": "aa".repeat(32) },
            "finalScriptSig": serde_json::Value::Null,
            "minTime": "0"
        }],
        "outputs": [{
            "amount": "59000",
            "scriptPublicKey": "00003344",
            "bip32Derivations": {},
            "proprietaries": {}
        }]
    });

    assert_eq!(
        super::pskb::encode_wire(&typed).expect("typed wire"),
        super::pskb::encode_pskt_value(source).expect("source wire")
    );
}

#[test]
fn typed_keyless_covenant_preserves_zero_signature_requirement() {
    let mut policy = super::pskb::SweepInputPolicy::covenant(&[0x51], 0, serde_json::json!({}));
    policy.minimum_signatures = 0;
    let plan = super::pskb::plan_sweep(
        &[utxo(0x88, 0, 20_000)],
        &[0x55],
        &[0x66],
        19_000,
        super::pskb::PskbGlobalPlan::standard().with_lock_time(500),
        &policy,
    );
    assert!(
        super::pskb::encode_wire(&plan).is_err(),
        "zero-signature covenant plans must fail closed at the consumer encoder boundary"
    );
}

#[test]
fn global_thread_allowance_withdrawal_matches_browser_wire_shape() {
    let thread = utxo(0x91, 2, 100_000_000);
    let covenant_id = [0x42; 32];
    let planned = plan_global_thread_withdrawal(GlobalThreadWithdrawalRequest {
        thread_utxos: std::slice::from_ref(&thread),
        covenant_script_public_key: &[0xaa, 0xbb],
        destination_script_public_key: &[0xcc, 0xdd],
        redeem_script: &[0x51, 0xac],
        covenant_id: &covenant_id,
        withdrawal: 20_000_000,
        fee: 1_000_000,
        csv_sequence: 9,
        policy: &GlobalThreadPolicy::allowance(123),
    })
    .expect("allowance plan");

    let source = serde_json::json!({
        "global": {
            "version": 0,
            "txVersion": 1,
            "fallbackLockTime": "123",
            "covenantBranch": "beneficiary",
            "inputsModifiable": false,
            "outputsModifiable": false,
            "inputCount": 1,
            "outputCount": 2,
            "xpubs": {},
            "proprietaries": {}
        },
        "inputs": [{
            "previousOutpoint": {
                "transactionId": thread.tx_id,
                "index": thread.index
            },
            "sequence": "9",
            "sighashType": 1,
            "sigOpCount": 1,
            "utxoEntry": {
                "amount": thread.amount.to_string(),
                "scriptPublicKey": "0000aabb",
                "blockDaaScore": "0",
                "isCoinbase": false
            },
            "redeemScript": "51ac",
            "partialSigs": {},
            "minimumSignatures": 1,
            "bip32Derivations": {},
            "proprietaries": {},
            "finalScriptSig": serde_json::Value::Null,
            "minTime": "0"
        }],
        "outputs": [{
            "amount": "80000000",
            "scriptPublicKey": "0000aabb",
            "covenantBinding": {
                "authorizingInput": 0,
                "covenantId": hex::encode(covenant_id)
            },
            "bip32Derivations": {},
            "proprietaries": {}
        }, {
            "amount": "19000000",
            "scriptPublicKey": "0000ccdd",
            "covenantBinding": serde_json::Value::Null,
            "bip32Derivations": {},
            "proprietaries": {}
        }]
    });

    assert_eq!(
        super::pskb::encode_wire(&planned.plan).expect("typed wire"),
        super::pskb::encode_pskt_value(source).expect("source wire")
    );
}

#[test]
fn global_thread_spending_limit_close_keeps_explicit_null_policy_fields() {
    let planned = plan_global_thread_withdrawal(GlobalThreadWithdrawalRequest {
        thread_utxos: &[utxo(0x92, 0, 10_000_000)],
        covenant_script_public_key: &[0xaa],
        destination_script_public_key: &[0xbb],
        redeem_script: &[0x51],
        covenant_id: &[0x24; 32],
        withdrawal: 10_000_000,
        fee: 1_000_000,
        csv_sequence: 7,
        policy: &GlobalThreadPolicy::spending_limit(),
    })
    .expect("spending-limit close plan");
    let document = decode_pskb_wire(&super::pskb::encode_wire(&planned.plan).expect("wire"));

    assert_eq!(document[0]["global"]["fallbackLockTime"], "0");
    assert!(document[0]["global"]["covenantBranch"].is_null());
    assert_eq!(document[0]["outputs"].as_array().expect("outputs").len(), 1);
    assert!(document[0]["outputs"][0]["covenantBinding"].is_null());
}

#[test]
fn global_thread_topup_matches_mixed_input_shape() {
    let thread = utxo(0x93, 1, 50_000_000);
    let mut wallet_one = utxo(0x94, 2, 20_000_000);
    wallet_one.script_public_key = vec![0x11, 0x22];
    wallet_one.block_daa_score = 456;
    let mut wallet_two = utxo(0x95, 3, 30_000_000);
    wallet_two.script_public_key = vec![0x33, 0x44];
    wallet_two.block_daa_score = 789;
    let covenant_id = [0x66; 32];

    let planned = plan_global_thread_topup(GlobalThreadTopupRequest {
        thread_utxo: thread.clone(),
        wallet_utxos: &[wallet_one.clone(), wallet_two.clone()],
        covenant_script_public_key: &[0xaa, 0xbb],
        redeem_script: &[0x51, 0xac],
        covenant_id: &covenant_id,
        fee: 1_000_000,
        policy: &GlobalThreadPolicy::spending_limit_topup(11),
    })
    .expect("topup plan");

    let source = serde_json::json!({
        "global": {
            "version": 0,
            "txVersion": 1,
            "fallbackLockTime": "0",
            "covenantBranch": serde_json::Value::Null,
            "inputsModifiable": false,
            "outputsModifiable": false,
            "inputCount": 3,
            "outputCount": 1,
            "xpubs": {},
            "proprietaries": {}
        },
        "inputs": [{
            "previousOutpoint": { "transactionId": thread.tx_id, "index": thread.index },
            "sequence": "11",
            "sighashType": 1,
            "sigOpCount": 1,
            "utxoEntry": {
                "amount": thread.amount.to_string(),
                "scriptPublicKey": "0000aabb",
                "blockDaaScore": "0",
                "isCoinbase": false
            },
            "redeemScript": "51ac",
            "partialSigs": {},
            "minimumSignatures": 1,
            "bip32Derivations": {},
            "proprietaries": {},
            "finalScriptSig": serde_json::Value::Null,
            "minTime": "0"
        }, {
            "previousOutpoint": { "transactionId": wallet_one.tx_id, "index": wallet_one.index },
            "sequence": "0",
            "sighashType": 1,
            "sigOpCount": 1,
            "utxoEntry": {
                "amount": wallet_one.amount.to_string(),
                "scriptPublicKey": "00001122",
                "blockDaaScore": wallet_one.block_daa_score.to_string(),
                "isCoinbase": false
            },
            "redeemScript": serde_json::Value::Null,
            "partialSigs": {},
            "minimumSignatures": 1,
            "bip32Derivations": {},
            "proprietaries": {},
            "finalScriptSig": serde_json::Value::Null,
            "minTime": "0"
        }, {
            "previousOutpoint": { "transactionId": wallet_two.tx_id, "index": wallet_two.index },
            "sequence": "0",
            "sighashType": 1,
            "sigOpCount": 1,
            "utxoEntry": {
                "amount": wallet_two.amount.to_string(),
                "scriptPublicKey": "00003344",
                "blockDaaScore": wallet_two.block_daa_score.to_string(),
                "isCoinbase": false
            },
            "redeemScript": serde_json::Value::Null,
            "partialSigs": {},
            "minimumSignatures": 1,
            "bip32Derivations": {},
            "proprietaries": {},
            "finalScriptSig": serde_json::Value::Null,
            "minTime": "0"
        }],
        "outputs": [{
            "amount": "99000000",
            "scriptPublicKey": "0000aabb",
            "covenantBinding": {
                "authorizingInput": 0,
                "covenantId": hex::encode(covenant_id)
            },
            "bip32Derivations": {},
            "proprietaries": {}
        }]
    });

    assert_eq!(
        super::pskb::encode_wire(&planned.plan).expect("typed wire"),
        super::pskb::encode_pskt_value(source).expect("source wire")
    );
}

#[test]
fn global_thread_plan_errors_have_specific_actionable_messages() {
    let cases = [
        GlobalThreadPlanError::BalanceTooLow { total: 1, fee: 2 },
        GlobalThreadPlanError::WithdrawalNotAboveFee {
            withdrawal: 2,
            fee: 2,
        },
        GlobalThreadPlanError::ContinuationTooSmall { continuation: 3 },
        GlobalThreadPlanError::SelectedFundsTooLow {
            selected_total: 4,
            fee: 5,
        },
    ];
    for error in cases {
        let message = error.to_string();
        assert!(!message.is_empty());
        assert!(message.contains(|character: char| character.is_ascii_digit()));
    }
}

#[test]
fn global_thread_withdrawal_enforces_fee_close_and_continuation_boundaries_exactly() {
    use super::pskb::global_thread::MIN_THREAD_CONTINUATION_SOMPI;

    let covenant_script = [0xaa, 0xbb];
    let destination_script = [0xcc, 0xdd];
    let redeem_script = [0x51, 0xac];
    let covenant_id = [0x42; 32];
    let policy = GlobalThreadPolicy::allowance(0);

    let plan = |total: u64, withdrawal: u64, fee: u64| {
        let thread = [utxo(0xa1, 0, total)];
        plan_global_thread_withdrawal(GlobalThreadWithdrawalRequest {
            thread_utxos: &thread,
            covenant_script_public_key: &covenant_script,
            destination_script_public_key: &destination_script,
            redeem_script: &redeem_script,
            covenant_id: &covenant_id,
            withdrawal,
            fee,
            csv_sequence: 5,
            policy: &policy,
        })
    };

    assert!(matches!(
        plan(1_000_000, 2_000_000, 1_000_000),
        Err(GlobalThreadPlanError::BalanceTooLow {
            total: 1_000_000,
            fee: 1_000_000
        })
    ));
    assert!(matches!(
        plan(2_000_000, 1_000_000, 1_000_000),
        Err(GlobalThreadPlanError::WithdrawalNotAboveFee {
            withdrawal: 1_000_000,
            fee: 1_000_000
        })
    ));

    let exact_floor_total = 50_000_000;
    let exact_floor_withdrawal = exact_floor_total - MIN_THREAD_CONTINUATION_SOMPI;
    let exact_floor = plan(exact_floor_total, exact_floor_withdrawal, 1_000_000)
        .expect("exact continuation floor is valid");
    assert!(!exact_floor.is_close);
    assert_eq!(exact_floor.continuation, MIN_THREAD_CONTINUATION_SOMPI);
    assert_eq!(
        exact_floor.user_receives,
        exact_floor_withdrawal - 1_000_000
    );
    assert_eq!(
        exact_floor.plan.global.fallback_lock_time,
        serde_json::Value::Null
    );

    assert!(matches!(
        plan(exact_floor_total, exact_floor_withdrawal + 1, 1_000_000),
        Err(GlobalThreadPlanError::ContinuationTooSmall { continuation })
            if continuation == MIN_THREAD_CONTINUATION_SOMPI - 1
    ));

    let close = plan(25_000_000, 25_000_000, 1_000_000).expect("exact close");
    assert!(close.is_close);
    assert_eq!(close.continuation, 0);
    assert_eq!(close.user_receives, 24_000_000);
    assert_eq!(close.plan.outputs.len(), 1);
}

#[test]
fn global_thread_planning_reports_monetary_overflow_instead_of_panicking() {
    let covenant_script = [0xaa, 0xbb];
    let destination_script = [0xcc, 0xdd];
    let redeem_script = [0x51, 0xac];
    let covenant_id = [0x42; 32];
    let policy = GlobalThreadPolicy::allowance(0);
    let overflowing = [utxo(0xe1, 0, u64::MAX), utxo(0xe2, 1, 1)];

    assert!(matches!(
        plan_global_thread_withdrawal(GlobalThreadWithdrawalRequest {
            thread_utxos: &overflowing,
            covenant_script_public_key: &covenant_script,
            destination_script_public_key: &destination_script,
            redeem_script: &redeem_script,
            covenant_id: &covenant_id,
            withdrawal: 1,
            fee: 1,
            csv_sequence: 0,
            policy: &policy,
        }),
        Err(GlobalThreadPlanError::ArithmeticOverflow {
            operation: "summing thread UTXOs"
        })
    ));

    assert!(matches!(
        plan_global_thread_topup(GlobalThreadTopupRequest {
            thread_utxo: utxo(0xe3, 0, 1),
            wallet_utxos: &overflowing,
            covenant_script_public_key: &covenant_script,
            redeem_script: &redeem_script,
            covenant_id: &covenant_id,
            fee: 1,
            policy: &policy,
        }),
        Err(GlobalThreadPlanError::ArithmeticOverflow {
            operation: "summing wallet top-up UTXOs"
        })
    ));

    let max_wallet = [utxo(0xe4, 0, u64::MAX)];
    assert!(matches!(
        plan_global_thread_topup(GlobalThreadTopupRequest {
            thread_utxo: utxo(0xe5, 0, 1),
            wallet_utxos: &max_wallet,
            covenant_script_public_key: &covenant_script,
            redeem_script: &redeem_script,
            covenant_id: &covenant_id,
            fee: 1,
            policy: &policy,
        }),
        Err(GlobalThreadPlanError::ArithmeticOverflow {
            operation: "adding thread and wallet balances"
        })
    ));
}
