use super::plan;

pub(crate) struct WithdrawalSummary {
    pub(crate) wire: String,
    pub(crate) funding_count: usize,
    pub(crate) covenant_return: u64,
    pub(crate) borrower_receive: u64,
}

pub(crate) fn build_withdrawal(
    plan: &plan::BorrowerPlan,
    withdraw_sompi: u64,
    fee: u64,
) -> Result<WithdrawalSummary, String> {
    let funding_after_fee = plan
        .funding_total
        .checked_sub(fee)
        .ok_or("Borrower funding is below the fee".to_string())?;
    let covenant_return = plan
        .covenant
        .amount
        .checked_sub(withdraw_sompi)
        .ok_or_else(|| {
            format!(
                "Withdraw {withdraw_sompi} > covenant balance {}",
                plan.covenant.amount
            )
        })?;
    let borrower_receive = withdraw_sompi
        .checked_add(funding_after_fee)
        .ok_or("Borrower receive amount overflows u64".to_string())?;
    let outputs = withdraw_outputs(plan, covenant_return, borrower_receive)?;
    let funding_count = plan.funding.len();
    let inputs = plan.inputs();
    let input_count = inputs.len();
    let output_count = outputs.as_array().map_or(0, Vec::len);
    let wire = plan::encode_pskb(withdraw_global(input_count, output_count), inputs, outputs)?;
    Ok(WithdrawalSummary {
        wire,
        funding_count,
        covenant_return,
        borrower_receive,
    })
}

fn withdraw_outputs(
    plan: &plan::BorrowerPlan,
    covenant_return: u64,
    borrower_receive: u64,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([
        {
            "amount": covenant_return,
            "scriptPublicKey": plan.covenant_spk_hex.as_str(),
            "bip32Derivations": {},
            "proprietaries": {}
        },
        {
            "amount": borrower_receive,
            "scriptPublicKey": plan.receive_script()?,
            "bip32Derivations": {},
            "proprietaries": {}
        }
    ]))
}

fn withdraw_global(input_count: usize, output_count: usize) -> serde_json::Value {
    serde_json::json!({
        "version": 0,
        "txVersion": 0,
        "fallbackLockTime": 0,
        "inputsModifiable": false,
        "outputsModifiable": false,
        "inputCount": input_count,
        "outputCount": output_count,
        "xpubs": {},
        "id": serde_json::Value::Null,
        "proprietaries": {}
    })
}
