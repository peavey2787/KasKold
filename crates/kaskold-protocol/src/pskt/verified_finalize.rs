//! Host JSON materialization from the exact typed authorization result.
//!
//! No PSKT source text is accepted here. This is deliberate: once signatures
//! and covenant execution have been authorized there is no second semantic
//! parser capable of changing the transaction or witness meaning.

use serde_json::{json, Value};

use super::verified::VerifiedTransaction;

pub(super) fn finalize_json(transaction: &VerifiedTransaction) -> Result<String, String> {
    let mut inputs = Vec::new();
    inputs
        .try_reserve(transaction.inputs().len())
        .map_err(|_| "verified input count exceeds available memory".to_string())?;
    for input in transaction.inputs() {
        inputs.push(json!({
            "previousOutpoint": {
                "transactionId": hex::encode(input.previous_tx_id()),
                "index": input.previous_index(),
            },
            "signatureScript": hex::encode(input.witness().materialize_signature_script()?),
            "sequence": input.sequence().to_string(),
            "sigOpCount": input.sig_op_count(),
        }));
    }

    let mut outputs = Vec::new();
    outputs
        .try_reserve(transaction.outputs().len())
        .map_err(|_| "verified output count exceeds available memory".to_string())?;
    for output in transaction.outputs() {
        let covenant = output
            .covenant()
            .map_or(Value::Null, |(authorizing_input, id)| {
                json!({
                    "authorizingInput": authorizing_input,
                    "id": hex::encode(id),
                })
            });
        outputs.push(json!({
            "amount": output.amount().to_string(),
            "scriptPublicKey": {
                "version": output.script_version(),
                "script": hex::encode(output.script_public_key()),
            },
            "covenant": covenant,
        }));
    }

    serde_json::to_string(&json!({
        "version": transaction.version(),
        "inputEncoding": "budgeted",
        "inputs": inputs,
        "outputs": outputs,
        "lockTime": transaction.locktime().to_string(),
        "subnetworkId": hex::encode(transaction.subnetwork_id()),
        "gas": transaction.gas().to_string(),
        "payload": hex::encode(transaction.payload()),
    }))
    .map_err(|error| format!("Final transaction JSON failed: {error}"))
}
