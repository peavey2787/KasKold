use crate::runtime::data::AppData;

pub(super) fn update_kspt(ad: &mut AppData) {
    let (present, required) = offline_signer::transaction::kspt::signature_status(&ad.signing.transaction.active);
    update(ad, present, required);
}

pub(super) fn update_pskt(ad: &mut AppData) {
    let (present, required) =
        offline_signer::transaction::std_pskt::pskt_signature_status(&ad.signing.transaction.active);
    update(ad, present, required);
}

/// End any anti-klepto session and take back the signatures it added.
pub(crate) fn abandon_anti_klepto(ad: &mut AppData) {
    if let Some(initial_counts) = ad.signing.anti_klepto.end() {
        rollback_added_signatures(ad, &initial_counts);
    }
}

pub(crate) fn rollback_added_signatures(ad: &mut AppData, initial_counts: &[u8]) {
    let input_count = ad.signing.transaction.active.num_inputs;
    let inputs = &mut ad.signing.transaction.active.inputs[..input_count];
    for (input, &initial) in inputs.iter_mut().zip(initial_counts) {
        input.sigs.truncate(initial);
    }
}

fn update(ad: &mut AppData, present: u32, required: u32) {
    ad.signing.transaction.signatures_present = present;
    ad.signing.transaction.signatures_required = required;
    if present < required {
        log!("   Partial: {}/{} sigs — pass to next signer", present, required);
    } else {
        log!("   Fully signed: {}/{}", present, required);
    }
}
