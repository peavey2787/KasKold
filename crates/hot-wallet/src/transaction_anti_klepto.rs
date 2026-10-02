//! Two-round anti-klepto transaction signing shared by software Vaults.
//!
//! The wire protocol and signature finalization are the same primitives used by
//! the M5 runtime. The host first commits to a secret, the Vault returns the
//! nonce commitments produced by provisional signatures, and only a matching
//! reveal can finalize those exact signatures.

use offline_signer::{
    derivation::bip32,
    transaction::{
        kspt,
        model::Transaction,
    },
};
use zeroize::Zeroize;

use crate::{entropy::fill_random, HotWallet, HotWalletError};

pub struct AntiKleptoSession {
    session_id: [u8; shared_signer::anti_klepto::SESSION_ID_LEN],
    host_commitment: [u8; shared_signer::anti_klepto::HASH_LEN],
    transaction_digest: [u8; shared_signer::anti_klepto::HASH_LEN],
    initial_counts: Vec<u8>,
    transaction: Transaction,
}

impl Drop for AntiKleptoSession {
    fn drop(&mut self) {
        self.session_id.zeroize();
        self.host_commitment.zeroize();
        self.transaction_digest.zeroize();
        self.initial_counts.zeroize();
        self.transaction.clear();
    }
}

impl HotWallet {
    pub fn prepare_anti_klepto(
        &self,
        request_wire: &[u8],
    ) -> Result<(AntiKleptoSession, Vec<u8>), HotWalletError> {
        let request = shared_signer::anti_klepto::parse_request(request_wire)
            .map_err(|_| HotWalletError::InvalidToolInput)?;
        let mut transaction = Transaction::try_new()?;
        kspt::parse_compact_kspt(request.transaction, &mut transaction)?;
        kspt::validate_transaction_for_review(&transaction)?;

        let initial_counts = kspt::initial_signature_counts(&transaction);
        let mut signing_entropy = [0u8; 32];
        fill_random(&mut signing_entropy)?;
        let sign_result = self.sign_transaction_model(&mut transaction, &signing_entropy);
        signing_entropy.zeroize();
        sign_result?;

        let records = kspt::nonce_commitment_records(&transaction, &initial_counts)?;
        let mut response = vec![0u8; 128usize.saturating_add(records.len().saturating_mul(71))];
        let length = shared_signer::anti_klepto::encode_commitment(
            &request.session_id,
            &request.transaction_digest,
            &records,
            &mut response,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        response.truncate(length);

        Ok((
            AntiKleptoSession {
                session_id: request.session_id,
                host_commitment: request.host_commitment,
                transaction_digest: request.transaction_digest,
                initial_counts,
                transaction,
            },
            response,
        ))
    }

    pub fn finalize_anti_klepto(
        &self,
        session: &mut AntiKleptoSession,
        reveal_wire: &[u8],
    ) -> Result<Vec<u8>, HotWalletError> {
        let (session_id, mut host_secret) = shared_signer::anti_klepto::parse_reveal(reveal_wire)
            .map_err(|_| HotWalletError::InvalidToolInput)?;
        if session_id != session.session_id
            || !shared_signer::anti_klepto::verify_host_secret(
                &session.host_commitment,
                &host_secret,
            )
        {
            host_secret.zeroize();
            return Err(HotWalletError::CryptoOperationFailed);
        }

        let finalize = self.finalize_anti_klepto_model(
            &mut session.transaction,
            &session.initial_counts,
            &session.session_id,
            &host_secret,
        );
        host_secret.zeroize();
        finalize?;

        let proofs = kspt::proof_records(&session.transaction, &session.initial_counts)?;
        let mut signed_tx = kspt::serialize_compact_kspt_vec(&session.transaction)?;
        let response_capacity = 128usize
            .saturating_add(signed_tx.len())
            .saturating_add(proofs.len().saturating_mul(128));
        let mut response = vec![0u8; response_capacity];
        let encoded = shared_signer::anti_klepto::encode_signed(
            &session.session_id,
            &session.transaction_digest,
            &proofs,
            &signed_tx,
            &mut response,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed);
        signed_tx.zeroize();
        let length = encoded?;
        response.truncate(length);
        Ok(response)
    }

    fn finalize_anti_klepto_model(
        &self,
        transaction: &mut Transaction,
        initial_counts: &[u8],
        session_id: &[u8; shared_signer::anti_klepto::SESSION_ID_LEN],
        host_secret: &[u8; 32],
    ) -> Result<(), HotWalletError> {
        if let Some(raw_key) = self.raw_key_bytes() {
            return finalize_raw_anti_klepto(
                transaction,
                raw_key,
                initial_counts,
                session_id,
                host_secret,
            );
        }
        if let Some(account) = self.account_key() {
            return finalize_account_anti_klepto(
                transaction,
                account,
                initial_counts,
                session_id,
                host_secret,
            );
        }
        self.finalize_seed_anti_klepto(transaction, initial_counts, session_id, host_secret)
    }

    fn finalize_seed_anti_klepto(
        &self,
        transaction: &mut Transaction,
        initial_counts: &[u8],
        session_id: &[u8; shared_signer::anti_klepto::SESSION_ID_LEN],
        host_secret: &[u8; 32],
    ) -> Result<(), HotWalletError> {
        let mut account = bip32::derive_account_key(self.seed_bytes()?)?;
        let result = finalize_account_anti_klepto(
            transaction,
            &account,
            initial_counts,
            session_id,
            host_secret,
        );
        account.zeroize();
        result
    }
}

fn finalize_raw_anti_klepto(
    transaction: &mut Transaction,
    raw_key: &[u8; 32],
    initial_counts: &[u8],
    session_id: &[u8; shared_signer::anti_klepto::SESSION_ID_LEN],
    host_secret: &[u8; 32],
) -> Result<(), HotWalletError> {
    kspt::finalize_raw_key_signatures(
        transaction,
        raw_key,
        initial_counts,
        session_id,
        host_secret,
    )
    .map(|_| ())
    .map_err(HotWalletError::from)
}

fn finalize_account_anti_klepto(
    transaction: &mut Transaction,
    account: &bip32::ExtendedPrivKey,
    initial_counts: &[u8],
    session_id: &[u8; shared_signer::anti_klepto::SESSION_ID_LEN],
    host_secret: &[u8; 32],
) -> Result<(), HotWalletError> {
    kspt::finalize_account_signatures(
        transaction,
        account,
        initial_counts,
        session_id,
        host_secret,
    )
    .map(|_| ())
    .map_err(HotWalletError::from)
}
