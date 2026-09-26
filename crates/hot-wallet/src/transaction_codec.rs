//! Shared compact-KSPT / PSKT / PSKB decoding for review and signing.

use offline_signer::transaction::{kspt, model::Transaction, std_pskt};
use zeroize::Zeroize;

use super::HotWalletError;

pub(crate) struct ParsedTransaction {
    pub format: std_pskt::DetectedFormat,
    pub transaction: Transaction,
    pub scratch: Vec<u8>,
    pub parsed: shared_signer::pskt::PsktParsed,
}

impl Drop for ParsedTransaction {
    fn drop(&mut self) {
        self.scratch.zeroize();
    }
}

pub(crate) fn parse_transaction(wire: &[u8]) -> Result<ParsedTransaction, HotWalletError> {
    let format = std_pskt::detect_tx_format(wire);
    let mut transaction = Transaction::try_new()?;
    let mut scratch = Vec::new();
    let mut parsed = shared_signer::pskt::PsktParsed::empty();
    match format {
        std_pskt::DetectedFormat::KsptCompact => {
            kspt::parse_compact_kspt(wire, &mut transaction)?;
        }
        std_pskt::DetectedFormat::PsktPskb | std_pskt::DetectedFormat::PsktSingle => {
            scratch.resize(wire.len(), 0u8);
            std_pskt::parse_pskt(wire, &mut scratch, &mut transaction, &mut parsed)?;
        }
        std_pskt::DetectedFormat::Unknown => return Err(HotWalletError::InvalidToolInput),
    }
    Ok(ParsedTransaction {
        format,
        transaction,
        scratch,
        parsed,
    })
}
