/// Multiply two values in GF(256) using primitive polynomial 0x11D.
pub(crate) fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let mut result = 0u8;
    let mut aa = a;
    let mut bb = b;
    for _ in 0..8 {
        if bb & 1 != 0 {
            result ^= aa;
        }
        let carry = aa & 0x80 != 0;
        aa <<= 1;
        if carry {
            // 0x11D with its x^8 term already shifted out of the byte.
            aa ^= 0x1D;
        }
        bb >>= 1;
    }
    result
}
