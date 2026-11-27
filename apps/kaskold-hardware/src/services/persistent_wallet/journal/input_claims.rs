//! Durable input-claim ledger.
//!
//! Kaspa's sighash does not commit to the amounts of a transaction's other
//! inputs, so a host could understate a different input in each of two
//! sessions and combine their signatures into one transaction paying a fee
//! the user never saw. The ledger remembers, across power cycles, the amount
//! every signed transaction claimed for each input, and signing stops when a
//! later claim contradicts one.
//!
//! The ledger journals A/B in its own two sectors like the configuration
//! record: each write goes to the older or damaged copy under the next
//! sequence number, so a power cut always leaves the previous ledger intact.
//! It holds only tags keyed by a wallet secret, never outpoints or amounts.

use alloc::boxed::Box;

use offline_signer::transaction::{
    input_claims::{InputClaimLedger, ENCODED_LEN},
    model::Transaction,
};
use sha2::{Digest, Sha256};

use super::{
    super::{
        flash::{self, AlignedBytes, DeviceFlash, FlashError},
        PersistentWallet,
    },
    is_newer,
};

const MAGIC: [u8; 8] = *b"KSCLAIM1";
const SEQUENCE_END: usize = MAGIC.len() + 4;
const LEDGER_END: usize = SEQUENCE_END + ENCODED_LEN;
const RECORD_SIZE: usize = LEDGER_END + 32;
type Record = AlignedBytes<RECORD_SIZE>;

/// Why a transaction's input claims were not accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClaimRefusal {
    /// An input's claimed amount contradicts one already signed under.
    Conflict,
    /// The ledger could not be read or saved, so nothing may be signed.
    Storage,
}

impl PersistentWallet<'_> {
    /// Accept `tx`'s input amounts against every claim signed under before, or
    /// refuse them all. New claims are on flash before this returns `Ok`.
    pub(crate) fn admit_input_claims(&mut self, wallet_key: &[u8; 32], tx: &Transaction) -> Result<(), ClaimRefusal> {
        let mut ledger = Box::<InputClaimLedger>::default();
        let newest = load(&mut self.flash, &mut ledger).map_err(|_| ClaimRefusal::Storage)?;
        match ledger.admit(wallet_key, tx) {
            Err(_) => Err(ClaimRefusal::Conflict),
            Ok(0) => Ok(()),
            Ok(_) => store(&mut self.flash, &ledger, newest).map_err(|_| ClaimRefusal::Storage),
        }
    }
}

/// Decode the newest intact copy into `ledger` and return where it is and its
/// sequence; `None` when neither copy is intact, as on first use.
#[inline(never)]
fn load(device: &mut DeviceFlash<'_>, ledger: &mut InputClaimLedger) -> Result<Option<(u32, u32)>, FlashError> {
    let mut record = Record::zeroed();
    let mut newest: Option<(u32, u32)> = None;
    for address in [flash::CLAIMS_A, flash::CLAIMS_B] {
        device.read(address, &mut record)?;
        let Some((sequence, encoded)) = intact(&record.0) else {
            continue;
        };
        if newest.is_some_and(|(_, current)| !is_newer(sequence, current)) {
            continue;
        }
        if ledger.decode_from(encoded).is_ok() {
            newest = Some((address, sequence));
        }
    }
    Ok(newest)
}

/// Write `ledger` over the copy that is not the newest.
#[inline(never)]
fn store(
    device: &mut DeviceFlash<'_>,
    ledger: &InputClaimLedger,
    newest: Option<(u32, u32)>,
) -> Result<(), FlashError> {
    let (target, sequence) = match newest {
        Some((flash::CLAIMS_A, sequence)) => (flash::CLAIMS_B, sequence.wrapping_add(1)),
        Some((_, sequence)) => (flash::CLAIMS_A, sequence.wrapping_add(1)),
        None => (flash::CLAIMS_A, 1),
    };
    let mut record = Record::zeroed();
    record.0[..MAGIC.len()].copy_from_slice(&MAGIC);
    record.0[MAGIC.len()..SEQUENCE_END].copy_from_slice(&sequence.to_le_bytes());
    let encoded: &mut [u8; ENCODED_LEN] =
        (&mut record.0[SEQUENCE_END..LEDGER_END]).try_into().map_err(|_| FlashError::Alignment)?;
    ledger.encode(encoded);
    let digest = Sha256::digest(&record.0[..LEDGER_END]);
    record.0[LEDGER_END..].copy_from_slice(&digest);
    device.replace_sector(target, &record)
}

/// The sequence and ledger bytes of a record whose magic and digest are intact.
fn intact(record: &[u8; RECORD_SIZE]) -> Option<(u32, &[u8; ENCODED_LEN])> {
    let (body, digest) = record.split_at(LEDGER_END);
    let (magic, rest) = body.split_first_chunk::<8>()?;
    let (sequence, ledger) = rest.split_first_chunk::<4>()?;
    let intact = *magic == MAGIC && Sha256::digest(body).as_slice() == digest;
    intact.then_some((u32::from_le_bytes(*sequence), ledger.try_into().ok()?))
}
