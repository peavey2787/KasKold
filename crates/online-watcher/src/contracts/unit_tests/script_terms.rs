use serde_json::{json, Value};

use crate::contracts::covenant::{construction, oracle_v1, script_terms::verified_terms_json};

// Valid secp256k1 x-only keys for 1G, 2G and 3G.
const OWNER: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const OTHER: &str = "c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
const THIRD: &str = "f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9";
const NETWORK: &str = "testnet-10";

fn built(document: Result<String, String>) -> Value {
    serde_json::from_str(&document.expect("covenant builds")).expect("constructor JSON")
}

fn field<'a>(document: &'a Value, name: &str) -> &'a str {
    document[name].as_str().expect(name)
}

fn verify(covenant_type: &str, document: &Value, statement: &str) -> Result<Value, String> {
    verified_terms_json(
        covenant_type,
        field(document, "redeem_script_hex"),
        field(document, "address"),
        NETWORK,
        statement,
    )
    .map(|terms| serde_json::from_str(&terms).expect("terms JSON"))
}

fn assert_terms(covenant_type: &str, document: &Value, statement: &str, expected: Value) {
    let terms = verify(covenant_type, document, statement).expect("genuine invite verifies");
    for (name, value) in expected.as_object().expect("expected terms") {
        assert_eq!(&terms[name], value, "{covenant_type} {name}");
    }
    assert_eq!(terms["type"], covenant_type);
    assert_eq!(terms["address"], document["address"]);
    assert_eq!(terms["redeem_script_hex"], document["redeem_script_hex"]);
}

fn oracle_document() -> Value {
    built(oracle_v1::build_json_with_salt(
        OWNER,
        OTHER,
        THIRD,
        &"44".repeat(32),
        "release on delivery",
        700,
        NETWORK,
        [0x5a; 16],
    ))
}

#[test]
fn genuine_invites_return_the_terms_their_scripts_were_built_with() {
    let dms = built(construction::dms::build_json(OWNER, OTHER, 900, NETWORK));
    assert_terms(
        "dms",
        &dms,
        "",
        json!({"owner_pubkey_hex": OWNER, "heir_pubkey_hex": OTHER, "inactivity_daa": "900"}),
    );

    let savings = built(construction::savings::build_json(
        OWNER, OTHER, 1234, NETWORK,
    ));
    assert_terms(
        "timelocked-savings",
        &savings,
        "",
        json!({"wallet1_pubkey_hex": OWNER, "wallet2_pubkey_hex": OTHER, "locktime_daa": "1234"}),
    );

    let piggy = built(construction::additive::build_json(OWNER, 5000, 77, NETWORK));
    assert_terms(
        "additive",
        &piggy,
        "",
        json!({"owner_pubkey_hex": OWNER, "threshold_sompi": "5000", "deadline_daa": "77"}),
    );

    let allowance = built(construction::allowance::build_global_json(
        OWNER, OTHER, 300, 20, 40, NETWORK,
    ));
    assert_terms(
        "global-allowance",
        &allowance,
        "",
        json!({
            "owner_pubkey_hex": OWNER,
            "beneficiary_pubkey_hex": OTHER,
            "max_withdraw_sompi": "300",
            "cooldown_daa": "20",
            "start_daa": "40",
        }),
    );

    let limit = built(construction::spending_limit::build_global_json(
        OWNER, 450, 30, NETWORK,
    ));
    assert_terms(
        "global-spending-limit",
        &limit,
        "",
        json!({"owner_pubkey_hex": OWNER, "max_withdraw_sompi": "450", "cooldown_daa": "30"}),
    );

    let payjoin = built(construction::payjoin::build_json(
        OWNER, OTHER, 66, 2, 3, NETWORK,
    ));
    assert_terms(
        "payjoin",
        &payjoin,
        "",
        json!({
            "owner_pubkey_hex": OWNER,
            "beneficiary_pubkey_hex": OTHER,
            "locktime_daa": "66",
            "min_inputs": "2",
            "min_outputs": "3",
        }),
    );
}

#[test]
fn escrow_invites_resolve_destinations_to_addresses() {
    let p2pk = crate::account::address::encode_p2pk_address(&[0x51; 32], "kaspatest");
    let p2sh = built(construction::dms::build_json(OWNER, OTHER, 1, NETWORK));
    let p2sh = field(&p2sh, "address");
    let escrow = built(construction::escrow::build_json(
        OWNER, OTHER, THIRD, &p2pk, p2sh, NETWORK, [0x61; 8],
    ));
    assert_terms(
        "escrow",
        &escrow,
        "",
        json!({
            "alice_pk": OWNER,
            "bob_pk": OTHER,
            "arbiter_pk": THIRD,
            "alice_address": p2pk,
            "bob_address": p2sh,
            "salt": "6161616161616161",
        }),
    );
}

#[test]
fn oracle_invites_carry_only_the_statement_the_script_commits_to() {
    let oracle = oracle_document();
    let statement = field(&oracle, "attestation_statement");
    assert_terms(
        "oracle-v1",
        &oracle,
        statement,
        json!({
            "owner_pubkey_hex": OWNER,
            "beneficiary_pubkey_hex": OTHER,
            "oracle_pubkey_hex": THIRD,
            "message_commitment_hex": oracle["message_commitment_hex"],
            "locktime_daa": "700",
            "salt": "5a".repeat(16),
            "attestation_statement": statement,
        }),
    );

    let forged = statement.replace("delivery", "request");
    let unbound = statement.replace(&"5a".repeat(16), &"00".repeat(16));
    for statement in [forged.as_str(), unbound.as_str(), ""] {
        let error = verify("oracle-v1", &oracle, statement).expect_err("forged statement");
        assert!(error.contains("statement"), "{error}");
    }
}

#[test]
fn forged_invites_are_refused() {
    let dms = built(construction::dms::build_json(OWNER, OTHER, 900, NETWORK));
    let other = built(construction::dms::build_json(OWNER, THIRD, 900, NETWORK));
    let script = field(&dms, "redeem_script_hex");

    let swapped = verified_terms_json(
        "dms",
        field(&other, "redeem_script_hex"),
        field(&dms, "address"),
        NETWORK,
        "",
    );
    assert!(swapped.expect_err("swapped script").contains("address"));

    let mainnet = verified_terms_json("dms", script, field(&dms, "address"), "mainnet", "");
    assert!(mainnet.expect_err("wrong network").contains("address"));

    let mut tampered = hex::decode(script).expect("hex");
    tampered.push(0x51);
    let tampered = verified_terms_json(
        "dms",
        &hex::encode(tampered),
        field(&dms, "address"),
        NETWORK,
        "",
    );
    assert!(tampered
        .expect_err("trailing opcode")
        .contains("not a dms covenant"));

    let mislabelled = verify("escrow", &dms, "");
    assert!(mislabelled
        .expect_err("wrong family")
        .contains("not a escrow covenant"));

    for legacy in [
        "treasury",
        "timelocked-escrow",
        "spending-limit",
        "unknown",
        "",
    ] {
        let error = verify(legacy, &dms, "").expect_err("unsupported type");
        assert!(error.contains("Unsupported covenant type"), "{error}");
    }

    let not_hex = verified_terms_json("dms", "zz", field(&dms, "address"), NETWORK, "");
    assert!(not_hex.expect_err("bad hex").contains("not hex"));

    let network = verified_terms_json("dms", script, field(&dms, "address"), "moonnet", "");
    assert!(network
        .expect_err("bad network")
        .contains("unsupported Kaspa network"));
}
