//! The one-byte header of a saved wallet slot: source in the low nibble, and
//! network and protection combined into a tag in the high nibble.

const SOURCE_MASK: u8 = 0x0f;
const TAG_SHIFT: u8 = 4;
const NETWORK_COUNT: u8 = 3;
const SOURCE_EMPTY: u8 = 0;
const SOURCE_MAX: u8 = 4;
const PROTECTION_MIN: u8 = 1;
const PROTECTION_MAX: u8 = 3;

/// Numeric codes for a slot's source, network and protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotHeader {
    pub source: u8,
    pub network: u8,
    pub protection: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotHeaderError {
    Source,
    Canonical,
}

/// An empty slot always encodes as zero, whatever else it carried.
pub const fn encode(header: SlotHeader) -> u8 {
    if header.source == SOURCE_EMPTY {
        return 0;
    }
    let tag = header.network + NETWORK_COUNT * header.protection;
    header.source | (tag << TAG_SHIFT)
}

/// Decode a header, rejecting any byte `encode` cannot produce.
pub const fn decode(value: u8) -> Result<SlotHeader, SlotHeaderError> {
    let source = value & SOURCE_MASK;
    if source > SOURCE_MAX {
        return Err(SlotHeaderError::Source);
    }
    if source == SOURCE_EMPTY {
        return if value == 0 {
            Ok(SlotHeader { source, network: 0, protection: PROTECTION_MIN })
        } else {
            Err(SlotHeaderError::Canonical)
        };
    }
    let tag = value >> TAG_SHIFT;
    let protection = tag / NETWORK_COUNT;
    if protection < PROTECTION_MIN || protection > PROTECTION_MAX {
        return Err(SlotHeaderError::Canonical);
    }
    Ok(SlotHeader { source, network: tag % NETWORK_COUNT, protection })
}
