use super::{
    execution::{execution_for, SpendPath},
    redeem::{decode_redeem_script, require_redeem_for_address},
};
use crate::{
    account::{address, utxo::UtxoEntry},
    network,
    transaction_builder::pskb::{
        encode_wire, CovenantInputPolicy, PskbGlobalPlan, PskbInputPlan, PskbOutputPlan, PskbPlan,
    },
};

const MAX_COVENANT_INPUTS: usize = 4;

pub(crate) struct PayjoinClaim {
    pub(crate) wire: String,
    pub(crate) input_count: usize,
    pub(crate) covenant_input_count: usize,
    pub(crate) total: u64,
    pub(crate) send: u64,
    pub(crate) change: u64,
    pub(crate) fee: u64,
}

pub(crate) async fn create(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    mixing_address: &str,
    requested_fee: u64,
    websocket_url: &str,
) -> Result<PayjoinClaim, String> {
    let covenant = fetch_covenant_utxos(websocket_url, covenant_address).await?;
    let mixing = fetch_smallest_mixing_utxo(websocket_url, mixing_address).await?;
    let claim = build_claim(
        covenant_address,
        destination_address,
        redeem_script_hex,
        mixing_address,
        requested_fee,
        covenant,
        mixing,
    )?;
    Ok(claim)
}

pub(crate) fn build_claim(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    mixing_address: &str,
    requested_fee: u64,
    covenant_utxos: Vec<UtxoEntry>,
    mixing_utxo: UtxoEntry,
) -> Result<PayjoinClaim, String> {
    if covenant_utxos.is_empty() {
        return Err("No UTXOs at covenant address".to_string());
    }
    let redeem_script = decode_redeem_script(redeem_script_hex)?;
    require_redeem_for_address(covenant_address, &redeem_script)?;
    let amounts = calculate_amounts(&covenant_utxos, &mixing_utxo, requested_fee)?;
    let scripts = Scripts::new(covenant_address, destination_address, mixing_address)?;
    let inputs = build_inputs(&covenant_utxos, &mixing_utxo, &scripts, &redeem_script)?;
    let input_count = inputs.len();
    let wire = encode_wire(&PskbPlan {
        global: PskbGlobalPlan::standard(),
        inputs,
        outputs: build_outputs(&scripts, amounts.send, amounts.change),
    })?;
    Ok(PayjoinClaim {
        wire,
        input_count,
        covenant_input_count: covenant_utxos.len(),
        total: amounts.total,
        send: amounts.send,
        change: amounts.change,
        fee: amounts.fee,
    })
}

pub(crate) async fn fetch_covenant_utxos(
    websocket_url: &str,
    address: &str,
) -> Result<Vec<UtxoEntry>, String> {
    let mut utxos = network::queries::utxos::fetch_for_address(websocket_url, address).await?;
    if utxos.is_empty() {
        return Err("No UTXOs at covenant address".to_string());
    }
    crate::transaction_builder::selection::sort_largest_first(&mut utxos);
    utxos.truncate(MAX_COVENANT_INPUTS);
    Ok(utxos)
}

pub(crate) async fn fetch_smallest_mixing_utxo(
    websocket_url: &str,
    address: &str,
) -> Result<UtxoEntry, String> {
    network::queries::utxos::fetch_for_address(websocket_url, address)
        .await?
        .into_iter()
        .min_by_key(|utxo| utxo.amount)
        .ok_or("No UTXOs at your address for mixing — PayJoin requires your own inputs".to_string())
}

struct ClaimAmounts {
    total: u64,
    send: u64,
    change: u64,
    fee: u64,
}

fn calculate_amounts(
    covenant_utxos: &[UtxoEntry],
    mixing_utxo: &UtxoEntry,
    requested_fee: u64,
) -> Result<ClaimAmounts, String> {
    let covenant_total = covenant_utxos.iter().try_fold(0u64, |total, utxo| {
        total
            .checked_add(utxo.amount)
            .ok_or("Covenant balance overflows u64".to_string())
    })?;
    let total = covenant_total
        .checked_add(mixing_utxo.amount)
        .ok_or("PayJoin total overflows u64".to_string())?;
    let input_count = covenant_utxos
        .len()
        .checked_add(1)
        .ok_or("PayJoin input count overflow".to_string())?;
    let fee = requested_fee.max(required_fee(input_count)?);
    if total <= fee {
        return Err("Balance too low to cover fee".to_string());
    }
    let covenant_fee = fee
        .checked_mul(3)
        .ok_or("PayJoin fee split overflow".to_string())?
        / 4;
    let mixing_fee = fee
        .checked_sub(covenant_fee)
        .ok_or("PayJoin fee split underflow".to_string())?;
    let send = covenant_total
        .checked_sub(covenant_fee)
        .filter(|value| *value > 0)
        .ok_or("Covenant balance too low".to_string())?;
    let change = mixing_utxo
        .amount
        .checked_sub(mixing_fee)
        .ok_or("Mixing UTXO is too small to cover its fee share".to_string())?;
    Ok(ClaimAmounts {
        total,
        send,
        change,
        fee,
    })
}

fn required_fee(input_count: usize) -> Result<u64, String> {
    let input_count = input_count as u64;
    let compute_mass = input_count
        .checked_mul(1_300)
        .and_then(|value| value.checked_add(429))
        .ok_or("PayJoin mass overflow".to_string())?;
    compute_mass
        .checked_mul(115)
        .ok_or("PayJoin fee overflow".to_string())
}

struct Scripts {
    covenant: Vec<u8>,
    destination: Vec<u8>,
    mixing: Vec<u8>,
}

impl Scripts {
    fn new(covenant: &str, destination: &str, mixing: &str) -> Result<Self, String> {
        Ok(Self {
            covenant: address::address_to_script_pubkey(covenant)?,
            destination: address::address_to_script_pubkey(destination)?,
            mixing: address::address_to_script_pubkey(mixing)?,
        })
    }
}

fn build_inputs(
    covenant_utxos: &[UtxoEntry],
    mixing_utxo: &UtxoEntry,
    scripts: &Scripts,
    redeem_script: &[u8],
) -> Result<Vec<PskbInputPlan>, String> {
    let execution = execution_for(redeem_script, SpendPath::Key(1))?;
    let mut inputs = covenant_utxos
        .iter()
        .map(|utxo| {
            PskbInputPlan::covenant(
                utxo.clone(),
                &scripts.covenant,
                redeem_script,
                CovenantInputPolicy {
                    sequence: 0,
                    sig_op_count: 1,
                    minimum_signatures: 1,
                    proprietaries: serde_json::json!({}),
                    min_time: Some(0),
                    execution: Some(execution),
                },
            )
        })
        .collect::<Vec<_>>();
    inputs.insert(
        1,
        PskbInputPlan::p2pk(mixing_utxo.clone(), &scripts.mixing, serde_json::json!({})),
    );
    Ok(inputs)
}

fn build_outputs(scripts: &Scripts, send: u64, change: u64) -> Vec<PskbOutputPlan> {
    let mut outputs = vec![PskbOutputPlan::plain(send, &scripts.destination)];
    if change > 0 {
        outputs.push(PskbOutputPlan::plain(change, &scripts.mixing));
    }
    outputs
}

#[cfg(test)]
mod unit_tests;
