#![no_main]

use kaskold_protocol::Network;
use libfuzzer_sys::fuzz_target;
use offline_signer::transaction::{
    kspt::parse_compact_kspt,
    model::Transaction,
    std_pskt::{parse_pskt, PSKB_MAGIC, PSKT_MAGIC},
};
use shared_signer::PsktParsed;

fn assert_same_semantic_model(vault: &Transaction, relayed: &Transaction) {
    assert_eq!(vault.version, relayed.version, "tx version drift");
    assert_eq!(vault.num_inputs, relayed.num_inputs, "input-count drift");
    assert_eq!(vault.num_outputs, relayed.num_outputs, "output-count drift");
    assert_eq!(vault.locktime, relayed.locktime, "locktime drift");
    assert_eq!(vault.subnetwork_id, relayed.subnetwork_id, "subnetwork drift");
    assert_eq!(vault.gas, relayed.gas, "gas drift");
    assert_eq!(vault.payload_len, relayed.payload_len, "payload-length drift");
    assert_eq!(
        &vault.payload[..vault.payload_len],
        &relayed.payload[..relayed.payload_len],
        "payload drift"
    );

    for index in 0..vault.num_inputs {
        let left = &vault.inputs[index];
        let right = &relayed.inputs[index];
        assert_eq!(left.previous_outpoint.transaction_id, right.previous_outpoint.transaction_id, "input[{index}] txid drift");
        assert_eq!(left.previous_outpoint.index, right.previous_outpoint.index, "input[{index}] outpoint-index drift");
        assert_eq!(left.utxo_entry.amount, right.utxo_entry.amount, "input[{index}] amount drift");
        assert_eq!(left.sequence, right.sequence, "input[{index}] sequence drift");
        assert_eq!(left.sig_op_count, right.sig_op_count, "input[{index}] sigop drift");
        assert_eq!(left.sighash_type, right.sighash_type, "input[{index}] sighash drift");
        assert_eq!(
            left.utxo_entry.script_public_key.version,
            right.utxo_entry.script_public_key.version,
            "input[{index}] script-version drift"
        );
        assert_eq!(
            left.utxo_entry.script_public_key.script_bytes(),
            right.utxo_entry.script_public_key.script_bytes(),
            "input[{index}] script drift"
        );
        assert_eq!(vault.redeem_bytes(index), relayed.redeem_bytes(index), "input[{index}] redeem drift");
        assert_eq!(left.covenant_execution_present, right.covenant_execution_present, "input[{index}] covenant proof presence drift");
        if left.covenant_execution_present {
            assert_eq!(left.covenant_execution_mask, right.covenant_execution_mask, "input[{index}] covenant proof mask drift");
            assert_eq!(left.covenant_execution_true_mask, right.covenant_execution_true_mask, "input[{index}] covenant proof truth drift");
        }
    }

    for index in 0..vault.num_outputs {
        let left = &vault.outputs[index];
        let right = &relayed.outputs[index];
        assert_eq!(left.value, right.value, "output[{index}] amount drift");
        assert_eq!(left.script_public_key.version, right.script_public_key.version, "output[{index}] script-version drift");
        assert_eq!(left.script_public_key.script_bytes(), right.script_public_key.script_bytes(), "output[{index}] script drift");
        assert_eq!(left.has_covenant, right.has_covenant, "output[{index}] covenant-presence drift");
        if left.has_covenant {
            assert_eq!(left.covenant_auth_input, right.covenant_auth_input, "output[{index}] covenant auth-input drift");
            assert_eq!(left.covenant_id, right.covenant_id, "output[{index}] covenant id drift");
        }
    }
}


fn assert_verified_materialization_matches(
    wire_hex: &str,
    verified: &kaskold_protocol::compat::VerifiedTransaction,
) {
    let finalized = kaskold_protocol::finalize_json(wire_hex)
        .expect("verified PSKT must finalize from the same typed model");
    let value: serde_json::Value = serde_json::from_str(&finalized)
        .expect("verified finalizer output must be JSON");
    assert_eq!(value["version"].as_u64(), Some(u64::from(verified.version())));
    assert_eq!(value["inputEncoding"].as_str(), Some("budgeted"));
    let locktime = verified.locktime().to_string();
    let gas = verified.gas().to_string();
    let subnetwork = hex::encode(verified.subnetwork_id());
    let payload = hex::encode(verified.payload());
    assert_eq!(value["lockTime"].as_str(), Some(locktime.as_str()));
    assert_eq!(value["gas"].as_str(), Some(gas.as_str()));
    assert_eq!(value["subnetworkId"].as_str(), Some(subnetwork.as_str()));
    assert_eq!(value["payload"].as_str(), Some(payload.as_str()));

    let inputs = value["inputs"].as_array().expect("final inputs array");
    assert_eq!(inputs.len(), verified.inputs().len());
    for (index, input) in verified.inputs().iter().enumerate() {
        let expected_script = input
            .witness()
            .materialize_signature_script()
            .expect("verified witness must materialize");
        let expected_txid = hex::encode(input.previous_tx_id());
        let expected_script_hex = hex::encode(expected_script);
        let sequence = input.sequence().to_string();
        assert_eq!(
            inputs[index]["previousOutpoint"]["transactionId"].as_str(),
            Some(expected_txid.as_str()),
            "input[{index}] previous txid drift"
        );
        assert_eq!(
            inputs[index]["previousOutpoint"]["index"].as_u64(),
            Some(u64::from(input.previous_index())),
            "input[{index}] previous index drift"
        );
        assert_eq!(
            inputs[index]["signatureScript"].as_str(),
            Some(expected_script_hex.as_str()),
            "input[{index}] verified witness/materializer drift"
        );
        assert_eq!(inputs[index]["sequence"].as_str(), Some(sequence.as_str()));
        assert_eq!(
            inputs[index]["sigOpCount"].as_u64(),
            Some(u64::from(input.sig_op_count()))
        );
    }

    let outputs = value["outputs"].as_array().expect("final outputs array");
    assert_eq!(outputs.len(), verified.outputs().len());
    for (index, output) in verified.outputs().iter().enumerate() {
        let amount = output.amount().to_string();
        let script = hex::encode(output.script_public_key());
        assert_eq!(outputs[index]["amount"].as_str(), Some(amount.as_str()));
        assert_eq!(
            outputs[index]["scriptPublicKey"]["version"].as_u64(),
            Some(u64::from(output.script_version()))
        );
        assert_eq!(
            outputs[index]["scriptPublicKey"]["script"].as_str(),
            Some(script.as_str())
        );
        match output.covenant() {
            Some((authorizing_input, covenant_id)) => {
                let covenant_id = hex::encode(covenant_id);
                assert_eq!(
                    outputs[index]["covenant"]["authorizingInput"].as_u64(),
                    Some(u64::from(authorizing_input))
                );
                assert_eq!(
                    outputs[index]["covenant"]["id"].as_str(),
                    Some(covenant_id.as_str())
                );
            }
            None => assert!(outputs[index]["covenant"].is_null()),
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // Both production boundaries cap PSKT JSON below u16::MAX. Keep enough room
    // for the four-byte envelope and its ASCII-hex JSON body without masking
    // oversized-input rejection in the public host parser.
    let wire = &data[..data.len().min(131_074)];

    let mut scratch = vec![0u8; u16::MAX as usize];
    let mut parsed = PsktParsed::empty();
    let mut vault_transaction = match Transaction::try_new() {
        Ok(value) => value,
        Err(_) => return,
    };
    let vault_ok = parse_pskt(wire, &mut scratch, &mut vault_transaction, &mut parsed).is_ok();

    let has_magic = wire.starts_with(PSKT_MAGIC) || wire.starts_with(PSKB_MAGIC);
    if vault_ok {
        assert!(has_magic);
        assert!(
            kaskold_protocol::compat::decode_pskt_json_body(&wire[4..]).is_ok(),
            "Vault accepted PSKT syntax rejected by host canonical grammar"
        );
    }

    let host_hex = hex::encode(wire);
    let host_result = kaskold_protocol::encode_pskt(&host_hex, Network::Mainnet);
    assert_eq!(
        host_result.is_ok(),
        vault_ok,
        "Companion/Vault semantic parser acceptance drift"
    );
    if let Ok(compact) = host_result {
        let mut relayed = Transaction::try_new().expect("bounded differential transaction");
        parse_compact_kspt(&compact, &mut relayed)
            .expect("host-produced compact model must decode at Vault boundary");
        assert_same_semantic_model(&vault_transaction, &relayed);
    }

    if let Ok(verified) =
        kaskold_protocol::compat::verify_complete_pskt(&host_hex, Network::Mainnet)
    {
        assert!(vault_ok, "verified host model must have been accepted by Vault parser");
        assert_verified_materialization_matches(&host_hex, &verified);
    }
});
