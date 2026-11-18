//! KasKold covenant application builders over Kaspa Portal's covenant core.

pub(crate) mod execution;
pub(crate) mod global_thread;
pub(crate) mod oracle_v1;
pub(crate) mod payjoin;
pub(crate) mod private_swap;
pub(crate) mod redeem;
pub(crate) mod spend_document;
pub(crate) mod sweep;
pub(crate) mod sweeps;
pub(crate) mod vault;

pub(crate) use kaspa_portal::transaction::builder::{
    CovenantBuildRequest, CovenantEncoding, CovenantFundingPolicy,
};
pub(crate) use kaspa_portal::transaction::mass::CovenantFeeShape;

/// How each family's manual deposit may differ from the amount asked for.
/// A shipment escrow dispatches on its exact amount, so it is funded exactly
/// or not at all. The fold families put sub-KIP-9 change into the covenant
/// output; every other family keeps explicit change.
pub(crate) fn funding_policy_for(covenant_type: &str) -> CovenantFundingPolicy {
    const EXACT_TYPES: [&str; 1] = ["ship-escrow"];
    const FOLD_SUB_KIP9_TYPES: [&str; 5] = [
        "additive",
        "timelocked-savings",
        "dms",
        "global-spending-limit",
        "global-allowance",
    ];
    if EXACT_TYPES.contains(&covenant_type) {
        CovenantFundingPolicy::Exact
    } else if FOLD_SUB_KIP9_TYPES.contains(&covenant_type) {
        CovenantFundingPolicy::FoldSubKip9Change
    } else {
        CovenantFundingPolicy::AdjustToSelection
    }
}

/// Build a covenant PSKB against the Companion's node.
pub(crate) async fn build(
    websocket_url: &str,
    request: CovenantBuildRequest<'_>,
) -> Result<String, String> {
    kaspa_portal::transaction::builder::build_covenant(
        &crate::network::client(websocket_url)?,
        request,
    )
    .await
}

/// Build a covenant PSKB and return the genesis covenant ID it binds.
pub(crate) async fn build_with_binding(
    websocket_url: &str,
    request: CovenantBuildRequest<'_>,
) -> Result<(String, Option<[u8; 32]>), String> {
    kaspa_portal::transaction::builder::build_covenant_with_binding(
        &crate::network::client(websocket_url)?,
        request,
    )
    .await
}

#[cfg(test)]
mod unit_tests;
