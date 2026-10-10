//! KasKold covenant application builders over Kaspa Portal's covenant core.

pub(crate) mod allowance;
pub(crate) mod global_thread;
pub(crate) mod oracle_v1;
pub(crate) mod payjoin;
pub(crate) mod private_swap;
pub(crate) mod sweep;
pub(crate) mod sweeps;
pub(crate) mod vault;

pub(crate) use kaspa_portal::transaction::builder::{
    CovenantBuildRequest, CovenantDustPolicy, CovenantEncoding,
};
pub(crate) use kaspa_portal::transaction::mass::CovenantFeeShape;

/// Covenant families whose manual sends fold sub-KIP-9 change into the
/// covenant output; every other family keeps explicit change.
pub(crate) fn dust_policy_for(covenant_type: &str) -> CovenantDustPolicy {
    const FOLD_SUB_KIP9_TYPES: [&str; 5] = [
        "additive",
        "timelocked-savings",
        "dms",
        "global-spending-limit",
        "global-allowance",
    ];
    if FOLD_SUB_KIP9_TYPES.contains(&covenant_type) {
        CovenantDustPolicy::FoldSubKip9Change
    } else {
        CovenantDustPolicy::Preserve
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
