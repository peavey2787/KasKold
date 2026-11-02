use super::GlobalThreadPolicy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GlobalThreadFamily {
    Allowance,
    SpendingLimit,
}

pub(crate) fn withdrawal_policy_for(
    family: GlobalThreadFamily,
    redeem_script: &[u8],
) -> Result<(u64, GlobalThreadPolicy), String> {
    match family {
        GlobalThreadFamily::Allowance => {
            let locktime =
                crate::protocol::script::extract_cltv_locktime(redeem_script)?.unwrap_or(0);
            Ok((locktime, GlobalThreadPolicy::allowance(locktime)))
        }
        GlobalThreadFamily::SpendingLimit => Ok((0, GlobalThreadPolicy::spending_limit())),
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn topup_policy_for(
    family: GlobalThreadFamily,
    redeem_script: &[u8],
) -> Result<(u64, GlobalThreadPolicy), String> {
    match family {
        GlobalThreadFamily::Allowance => Ok((0, GlobalThreadPolicy::allowance(0))),
        GlobalThreadFamily::SpendingLimit => {
            let csv_sequence =
                crate::protocol::script::extract_csv_sequence(redeem_script)?.unwrap_or(0);
            Ok((
                csv_sequence,
                GlobalThreadPolicy::spending_limit_topup(csv_sequence),
            ))
        }
    }
}
