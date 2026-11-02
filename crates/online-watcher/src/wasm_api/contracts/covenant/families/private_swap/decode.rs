//! Fixed-width hex fields of the Private Swap browser boundary.

pub(super) fn decode16(value: &str, label: &str) -> Result<[u8; 16], String> {
    decode_fixed(value, label)
}
pub(super) fn decode32(value: &str, label: &str) -> Result<[u8; 32], String> {
    decode_fixed(value, label)
}
pub(super) fn decode33(value: &str, label: &str) -> Result<[u8; 33], String> {
    decode_fixed(value, label)
}
pub(super) fn decode64(value: &str, label: &str) -> Result<[u8; 64], String> {
    decode_fixed(value, label)
}

fn decode_fixed<const N: usize>(value: &str, label: &str) -> Result<[u8; N], String> {
    let bytes = hex::decode(value).map_err(|error| format!("Bad {label}: {error}"))?;
    if bytes.len() != N {
        return Err(format!("{label} must be {N} bytes"));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub(super) fn parse_presig(
    value: &str,
    negated: bool,
) -> Result<crate::protocol::private_swap::adaptor::AdaptorPreSignature, String> {
    Ok(
        crate::protocol::private_swap::adaptor::AdaptorPreSignature {
            bytes: decode64(value, "pre-signature")?,
            negated,
        },
    )
}
