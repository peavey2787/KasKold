// SeedQR codecs are implemented once in shared-signer and re-exported here so
// existing hardware call sites keep their stable module path.

pub use shared_signer::seed_qr::{
    decode_compact_seedqr, decode_seedqr, encode_compact_seedqr, encode_seedqr,
};
