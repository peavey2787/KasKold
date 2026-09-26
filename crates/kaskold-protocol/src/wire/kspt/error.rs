use core::fmt;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireError {
    BufferTooShort,
    OutputBufferTooSmall,
    InvalidMagic,
    UnsupportedVersion,
    InvalidFlags,
    CountOverflow,
    ScriptTooLong,
    RedeemTooLong,
    TooManySignatures,
    DuplicateSignaturePosition,
    InvalidSigHashType,
    InvalidNetwork,
    MissingNetwork,
    InvalidTrailer,
    TrailingData,
    TooManyInputs,
    TooManyOutputs,
    PayloadTooLong,
    UnsupportedTransactionVersion,
}

const WIRE_ERROR_MESSAGES: [&str; 19] = [
    "KSPT is truncated",
    "KSPT output buffer is too small",
    "invalid KSPT magic",
    "unsupported KSPT generation",
    "invalid KSPT flags",
    "KSPT count exceeds wire capacity",
    "KSPT script exceeds protocol limit",
    "KSPT redeem script exceeds u16 wire capacity",
    "KSPT input has too many signatures",
    "KSPT repeats a signature public-key position",
    "KSPT contains an invalid sighash type",
    "KSPT contains an invalid network code",
    "KSPT v4 is missing its network trailer",
    "KSPT contains an invalid or duplicate trailer",
    "KSPT contains unrecognized trailing data",
    "KSPT declares more inputs than the consumer permits",
    "KSPT declares more outputs than the consumer permits",
    "KSPT payload exceeds the consumer limit",
    "unsupported KSPT transaction version",
];

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(WIRE_ERROR_MESSAGES[*self as usize])
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum DecodeError<E> {
    Wire(WireError),
    Sink(E),
}

impl<E> From<WireError> for DecodeError<E> {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}
