//! CoreS3 BMI270 entropy integration.
//!
//! KasKold owns point-of-use completeness/diversity checks and zeroization.
//! Seed creation requires at least one healthy BMI270 pre/post-camera window;
//! later `fill()` calls remain fail-closed on the checked hardware TRNG.

use esp_hal::{Blocking, delay::Delay, i2c::master::I2c};
use sha2::{Digest, Sha256};

use super::mixer;

pub(super) const SEED_SAMPLE_BYTES: usize = 33;

pub fn initialize(i2c: &mut I2c<'_, Blocking>, delay: &mut Delay) -> bool {
    crate::hw::imu::init(i2c, delay)
}

pub(super) fn collect_seed_sample(
    i2c: &mut I2c<'_, Blocking>,
    delay: &mut Delay,
    output: &mut [u8; SEED_SAMPLE_BYTES],
) -> usize {
    crate::hw::imu::collect(i2c, delay, output)
}

pub(super) fn mix_seed_sample(
    pool: &mut [u8; 32],
    sample: &mut [u8; SEED_SAMPLE_BYTES],
    count: usize,
    source_tag: u8,
) -> bool {
    let healthy = count == sample.len() && crate::hw::imu::buffer_is_healthy(sample);
    let distinct = crate::hw::imu::axis_distinct(&sample[..count]);
    crate::log!(
        "   [imu] seed window bytes {}/{} distinct X{} Y{} Z{}",
        count, sample.len(), distinct[0], distinct[1], distinct[2]
    );
    if healthy {
        let mut hasher = Sha256::new();
        // Legacy cryptographic domain string retained for seed compatibility.
        hasher.update(b"KasSigner-seed-imu-v2");
        hasher.update(*sample);
        hasher.update((count as u32).to_le_bytes());
        hasher.update([source_tag]);
        mixer::xor_digest(pool, &hasher.finalize());
    }
    mixer::zeroize(sample);
    healthy
}
