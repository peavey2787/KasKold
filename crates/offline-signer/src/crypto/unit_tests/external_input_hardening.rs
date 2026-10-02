//! KasKold's own container parsers must be total. The shared signing parsers
//! (KSPT, PSKT, kpub, KDF metadata, Private Swap) are covered by Kaspa Portal.

use super::container_framing;

#[test]
fn externally_controlled_container_parsers_are_total_over_truncated_and_noise_inputs() {
    let mut seed = 0x3c6e_f372_fe94_f82bu64;
    for len in 0..=600usize {
        let mut bytes = alloc::vec![0u8; len];
        for byte in &mut bytes {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *byte = (seed >> 32) as u8;
        }
        let data = bytes.as_slice();
        let result = std::panic::catch_unwind(|| {
            let _ = container_framing::parse_backup_header(data);
            let _ = container_framing::parse_transport_header(data, data.len());
        });
        assert!(
            result.is_ok(),
            "external container parser panicked at length {len}"
        );
    }
}
