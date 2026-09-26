//! Transaction signing strategy for software Vault custody.
//!
//! Compact KSPT and standard PSKT/PSKB share one decoder. Wallet source type
//! selects the same signing strategy used by the M5: raw keys sign matching
//! P2PK inputs, imported account XPrvs sign account-owned inputs, and mnemonic
//! wallets additionally support BIP45 multisig derivation.

use offline_signer::transaction::{kspt, model::ScriptType, model::SigHashType, std_pskt};
use zeroize::Zeroize;

use crate::{
    entropy::fill_random, transaction_codec::parse_transaction,
    transaction_review::validate_generic_vault_semantics, HotWallet, HotWalletError,
};

impl HotWallet {
    pub fn sign_transaction(&self, wire: &[u8]) -> Result<Vec<u8>, HotWalletError> {
        let mut parsed = parse_transaction(wire)?;
        kspt::validate_transaction_for_review(&parsed.transaction)?;
        validate_generic_vault_semantics(&parsed.transaction)?;

        let mut signing_entropy = [0u8; 32];
        fill_random(&mut signing_entropy)?;
        let sign_result = self.sign_transaction_model(&mut parsed.transaction, &signing_entropy);
        signing_entropy.zeroize();
        sign_result?;
        serialize_signed_transaction(parsed)
    }

    pub fn sign_compact_kspt(&self, wire: &[u8]) -> Result<Vec<u8>, HotWalletError> {
        if std_pskt::detect_tx_format(wire) != std_pskt::DetectedFormat::KsptCompact {
            return Err(HotWalletError::InvalidToolInput);
        }
        self.sign_transaction(wire)
    }

    pub(crate) fn sign_transaction_model(
        &self,
        transaction: &mut offline_signer::transaction::model::Transaction,
        signing_entropy: &[u8; 32],
    ) -> Result<(), HotWalletError> {
        if let Some(raw_key) = self.raw_key_bytes() {
            return sign_raw_transaction(transaction, raw_key, signing_entropy);
        }
        let has_multisig = transaction_has_multisig(transaction);
        if let Some(account) = self.account_key() {
            return sign_account(transaction, account, has_multisig, signing_entropy);
        }
        self.sign_mnemonic_transaction(transaction, has_multisig, signing_entropy)
    }

    fn sign_mnemonic_transaction(
        &self,
        transaction: &mut offline_signer::transaction::model::Transaction,
        has_multisig: bool,
        signing_entropy: &[u8; 32],
    ) -> Result<(), HotWalletError> {
        if has_multisig {
            self.sign_mnemonic_multisig(transaction, signing_entropy)
        } else {
            self.sign_mnemonic_single(transaction, signing_entropy)
        }
    }

    fn sign_mnemonic_multisig(
        &self,
        transaction: &mut offline_signer::transaction::model::Transaction,
        signing_entropy: &[u8; 32],
    ) -> Result<(), HotWalletError> {
        let mut seeds = [(*self.seed_bytes()?, true)];
        let result = kspt::sign_transaction_multisig_with_entropy(
            transaction,
            &seeds,
            SigHashType::All,
            Some(0),
            signing_entropy,
        )
        .map(|_| ())
        .map_err(HotWalletError::from);
        seeds[0].0.zeroize();
        result
    }

    fn sign_mnemonic_single(
        &self,
        transaction: &mut offline_signer::transaction::model::Transaction,
        signing_entropy: &[u8; 32],
    ) -> Result<(), HotWalletError> {
        let seed = self.seed_bytes()?;
        kspt::sign_transaction_multi_addr_with_entropy(
            transaction,
            seed,
            SigHashType::All,
            signing_entropy,
        )
        .map(|_| ())
        .map_err(HotWalletError::from)
    }
}

fn transaction_has_multisig(transaction: &offline_signer::transaction::model::Transaction) -> bool {
    (0..transaction.num_inputs).any(|index| {
        let (script_type, _) = kspt::analyze_input_script(transaction, index);
        matches!(script_type, ScriptType::Multisig | ScriptType::P2SH)
    })
}

fn sign_raw_transaction(
    transaction: &mut offline_signer::transaction::model::Transaction,
    raw_key: &[u8; 32],
    signing_entropy: &[u8; 32],
) -> Result<(), HotWalletError> {
    kspt::sign_matching_inputs_in_place_with_entropy(
        transaction,
        raw_key,
        SigHashType::All,
        signing_entropy,
    )
    .map(|_| ())
    .map_err(HotWalletError::from)
}

fn serialize_signed_transaction(
    mut parsed: crate::transaction_codec::ParsedTransaction,
) -> Result<Vec<u8>, HotWalletError> {
    match parsed.format {
        std_pskt::DetectedFormat::KsptCompact => {
            Ok(kspt::serialize_compact_kspt_vec(&parsed.transaction)?)
        }
        std_pskt::DetectedFormat::PsktPskb | std_pskt::DetectedFormat::PsktSingle => {
            std_pskt::move_ksp_sigs_to_pskt(&mut parsed.transaction)?;
            serialize_standard_pskt(&parsed)
        }
        std_pskt::DetectedFormat::Unknown => Err(HotWalletError::InvalidToolInput),
    }
}

fn serialize_standard_pskt(
    parsed: &crate::transaction_codec::ParsedTransaction,
) -> Result<Vec<u8>, HotWalletError> {
    let Some(input_format) = parsed.format.to_tx_input_format() else {
        return Err(HotWalletError::InvalidToolInput);
    };
    std_pskt::serialize_pskt_vec(
        &parsed.transaction,
        &parsed.parsed,
        &parsed.scratch,
        input_format,
    )
    .map_err(HotWalletError::from)
}

fn sign_account(
    transaction: &mut offline_signer::transaction::model::Transaction,
    account: &offline_signer::derivation::bip32::ExtendedPrivKey,
    has_multisig: bool,
    signing_entropy: &[u8; 32],
) -> Result<(), HotWalletError> {
    if has_multisig {
        let mut raw = account.to_raw();
        let accounts = [(raw, true)];
        let result = kspt::sign_transaction_multisig_accounts_with_entropy(
            transaction,
            &accounts,
            SigHashType::All,
            Some(0),
            signing_entropy,
        );
        raw.zeroize();
        result?;
    } else {
        kspt::sign_transaction_account_multi_addr_with_entropy(
            transaction,
            account,
            SigHashType::All,
            signing_entropy,
        )?;
    }
    Ok(())
}
