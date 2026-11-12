use crate::controllers::TouchInput;
use crate::runtime::data::AppData;

const MIN_PARTICIPANTS: u8 = 1;
const MAX_PARTICIPANTS: u8 = 5;

pub(super) fn handle(ad: &mut AppData, input: TouchInput) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        ad.wallet.seeds.clear_multisig_wallet_return();
        ad.signing.multisig.creating.n = 0;
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigMenu));
        return true;
    }

    let changed = if (60..=110).contains(&x) && (65..=103).contains(&y) {
        decrement(&mut ad.signing.multisig.threshold)
    } else if (210..=260).contains(&x) && (65..=103).contains(&y) {
        increment(&mut ad.signing.multisig.threshold)
    } else if (60..=110).contains(&x) && (125..=163).contains(&y) {
        decrement(&mut ad.signing.multisig.participant_count)
    } else if (210..=260).contains(&x) && (125..=163).contains(&y) {
        increment(&mut ad.signing.multisig.participant_count)
    } else if (80..=240).contains(&x) && (190..=230).contains(&y) {
        return start_key_collection(ad);
    } else {
        false
    };

    let clamped = clamp_threshold(ad);
    changed || clamped
}

fn decrement(value: &mut u8) -> bool {
    if *value <= MIN_PARTICIPANTS {
        return false;
    }
    *value -= 1;
    true
}

fn increment(value: &mut u8) -> bool {
    if *value >= MAX_PARTICIPANTS {
        return false;
    }
    *value += 1;
    true
}

fn clamp_threshold(ad: &mut AppData) -> bool {
    if ad.signing.multisig.threshold <= ad.signing.multisig.participant_count {
        return false;
    }
    ad.signing.multisig.threshold = ad.signing.multisig.participant_count;
    true
}

fn start_key_collection(ad: &mut AppData) -> bool {
    let threshold = ad.signing.multisig.threshold;
    let participants = ad.signing.multisig.participant_count;
    if threshold < MIN_PARTICIPANTS || threshold > participants || participants > MAX_PARTICIPANTS {
        return false;
    }

    ad.wallet.seeds.clear_multisig_wallet_return();
    let mut config = offline_signer::transaction::model::MultisigConfig::new();
    config.m = threshold;
    config.n = participants;
    config.family = offline_signer::transaction::model::MultisigFamily::HD45_UNRESOLVED;
    ad.signing.multisig.begin(config, crate::runtime::data::MultisigDraft::Assembling);
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigAddKey { key_idx: 0 }));
    true
}
