//! Cancellation and abandonment cleanup for kpub operations.

use crate::runtime::data::AppData;

#[cfg(feature = "m5stack")]
pub(in crate::runtime::event_loop) fn service(ad: &mut AppData) {
    use crate::runtime::data::OperationKind;
    if matches!(
        crate::runtime::presentation::operation_kind(ad),
        Some(OperationKind::ConnectCompanion | OperationKind::DeriveMultisigKpub)
    ) {
        return;
    }
    cancel_abandoned_derivation(ad);
}

#[cfg(feature = "m5stack")]
fn cancel_abandoned_derivation(ad: &mut AppData) {
    use crate::services::wallet_keys::worker as kpub_worker;
    ad.export.kpub_seed_derivation = None;
    if let Some(generation) = ad.export.kpub_worker_generation.take() {
        kpub_worker::cancel(generation);
    }
    ad.export.multisig_seed_derivation = None;
    if let Some(generation) = ad.export.multisig_worker_generation.take() {
        kpub_worker::cancel(generation);
    }
    ad.export.kpub_progress = 0;
    if ad.wallet.addresses.cache_worker_generation.is_none() {
        kpub_worker::discard_completed();
    }
}



pub(in crate::runtime::event_loop) fn cancel_operation(
    ad: &mut AppData,
    kind: crate::runtime::data::OperationKind,
) {
    use crate::runtime::data::OperationKind;
    #[cfg(feature = "m5stack")]
    use crate::services::wallet_keys::worker as kpub_worker;

    match kind {
        OperationKind::ConnectCompanion => {
            ad.export.kpub_seed_derivation = None;
            #[cfg(feature = "m5stack")]
            if let Some(generation) = ad.export.kpub_worker_generation.take() {
                kpub_worker::cancel(generation);
            }
            ad.export.kpub_progress = 0;
            ad.export.kpub_len = 0;
        }
        OperationKind::DeriveMultisigKpub => {
            #[cfg(feature = "m5stack")]
            if let Some(generation) = ad.export.multisig_worker_generation.take() {
                kpub_worker::cancel(generation);
            }
            ad.export.reset_multisig_kpub_work();
            ad.export.kpub_len = 0;
        }
        _ => {}
    }
}

