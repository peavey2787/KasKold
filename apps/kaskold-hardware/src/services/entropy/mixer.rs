// Small, explicit entropy-pool operations.

pub fn xor_digest(pool: &mut [u8; 32], digest: &[u8]) {
    for (destination, source) in pool.iter_mut().zip(digest.iter().copied()) {
        *destination ^= source;
    }
}

/// Mix optional user-entered d6 rolls into an already validated hardware/camera pool.
/// The hardware/camera pool remains mandatory; dice can only add a domain-separated
/// contribution and can never replace the checked device entropy path.
pub fn mix_additive_dice(pool: &mut [u8; 32], rolls: &[u8]) -> bool {
    shared_signer::seed_entropy::mix_additive_dice(pool, rolls)
}

/// Mix a completed user touch transcript into an already validated entropy pool.
/// Touch is additive only: it can never replace the mandatory checked device pool.
pub fn mix_additive_touch(pool: &mut [u8; 32], touch_digest: &mut [u8; 32]) {
    shared_signer::seed_entropy::mix_additive_touch(pool, touch_digest);
}

pub fn zeroize(bytes: &mut [u8]) {
    shared_signer::bytes::zeroize_bytes(bytes);
}
