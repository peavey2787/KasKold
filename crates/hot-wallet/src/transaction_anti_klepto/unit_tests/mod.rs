//! Anti-klepto sessions wipe their commitments when dropped.

use core::ptr::addr_of;

use super::AntiKleptoSession;
use crate::unit_tests::zeroization::dropped;
use offline_signer::transaction::model::Transaction;

#[test]
fn dropping_a_session_wipes_its_commitments() {
    let session = AntiKleptoSession {
        session_id: [1; shared_signer::anti_klepto::SESSION_ID_LEN],
        host_commitment: [2; shared_signer::anti_klepto::HASH_LEN],
        transaction_digest: [3; shared_signer::anti_klepto::HASH_LEN],
        initial_counts: vec![1, 2],
        transaction: Transaction::try_new().unwrap(),
    };
    let slot = dropped(session);
    let session = slot.as_ptr();
    // SAFETY: plain arrays and one vector length are read after `drop`.
    let (session_id, host_commitment, digest, counts) = unsafe {
        (
            addr_of!((*session).session_id).read(),
            addr_of!((*session).host_commitment).read(),
            addr_of!((*session).transaction_digest).read(),
            (*addr_of!((*session).initial_counts)).len(),
        )
    };
    assert!(session_id.iter().all(|byte| *byte == 0));
    assert!(host_commitment.iter().all(|byte| *byte == 0));
    assert!(digest.iter().all(|byte| *byte == 0));
    assert_eq!(counts, 0);
}
