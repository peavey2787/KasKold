use offline_signer::{
    crypto::{
        adaptor::AdaptorPreSignature,
        schnorr::{schnorr_verify, SchnorrSignature},
    },
    derivation::covenant,
    transaction::{kspt, model::Transaction, private_swap as swap_tx},
};
use sha2::{Digest, Sha256};
use shared_signer::covenant_sign::private_swap::{
    self as wire, PrivateSwapResponse, RequestKind, ResponseKind,
};
use zeroize::Zeroize;

use super::{
    PrivateSwapMode, PrivateSwapPhase, PrivateSwapPrepared, PrivateSwapReview, PrivateSwapSession,
};
use crate::{entropy::fill_random, HotWallet, HotWalletError, WalletKind};

impl PrivateSwapSession {
    pub fn prepare_request(
        &mut self,
        wallet: &HotWallet,
        input: &[u8],
    ) -> Result<PrivateSwapPrepared, HotWalletError> {
        require_mnemonic(wallet)?;
        let request = wire::parse_request(input).map_err(|_| HotWalletError::InvalidToolInput)?;
        match request.kind {
            RequestKind::KeyInfo => self.prepare_key_info(wallet),
            RequestKind::Bind => self.prepare_bind(&request),
            RequestKind::PreSign => self.prepare_presign(wallet, &request),
            RequestKind::Complete => self.prepare_complete(wallet, &request),
        }
    }

    pub fn confirm(&mut self, wallet: &HotWallet) -> Result<Vec<u8>, HotWalletError> {
        require_mnemonic(wallet)?;
        match (self.mode, self.phase) {
            (PrivateSwapMode::Bind, PrivateSwapPhase::Prepared) => self.complete_binding(wallet),
            (PrivateSwapMode::PreSign, PrivateSwapPhase::Prepared) => {
                self.phase = PrivateSwapPhase::AwaitingReveal;
                self.build_response(ResponseKind::Nonce, [0; 64], false)
            }
            (PrivateSwapMode::Complete, PrivateSwapPhase::Prepared) => self.complete_claim(wallet),
            _ => Err(HotWalletError::InvalidToolInput),
        }
    }

    pub fn finalize_reveal(
        &mut self,
        wallet: &HotWallet,
        input: &[u8],
    ) -> Result<Vec<u8>, HotWalletError> {
        require_mnemonic(wallet)?;
        let mut reveal = wire::parse_reveal(input).map_err(|_| HotWalletError::InvalidToolInput)?;
        if !self.reveal_matches(&reveal) {
            reveal.host_secret.zeroize();
            return Err(HotWalletError::CryptoOperationFailed);
        }
        let presig = self.create_presignature(wallet, &reveal.host_secret)?;
        let relation = self.verify_presignature_relation(&reveal.host_secret, &presig);
        reveal.host_secret.zeroize();
        relation.map_err(|_| HotWalletError::CryptoOperationFailed)?;
        self.presignature = presig.bytes;
        self.presignature_negated = presig.negated;
        self.phase = PrivateSwapPhase::FinalResponse;
        self.build_response(ResponseKind::PreSignature, presig.bytes, presig.negated)
    }

    fn reveal_matches(&self, reveal: &wire::PrivateSwapReveal) -> bool {
        self.mode == PrivateSwapMode::PreSign
            && self.phase == PrivateSwapPhase::AwaitingReveal
            && reveal.session_id == self.session_id
            && reveal.key_id == self.key_id
            && reveal.sighash == self.sighash
            && shared_signer::anti_klepto::verify_host_secret(
                &self.host_commitment,
                &reveal.host_secret,
            )
    }

    fn create_presignature(
        &self,
        wallet: &HotWallet,
        host_secret: &[u8; 32],
    ) -> Result<AdaptorPreSignature, HotWalletError> {
        covenant::create_private_swap_adaptor_presignature(
            wallet.seed_bytes()?,
            &self.key_id,
            &self.sighash,
            &self.adaptor_point,
            &self.session_id,
            &self.aux_rand,
            host_secret,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)
    }

    fn verify_presignature_relation(
        &self,
        host_secret: &[u8; 32],
        presig: &AdaptorPreSignature,
    ) -> Result<(), offline_signer::crypto::adaptor::AdaptorError> {
        offline_signer::crypto::adaptor::verify_host_nonce_relation(
            &self.claim_pubkey,
            &self.sighash,
            &self.adaptor_point,
            &self.session_id,
            host_secret,
            &self.nonce_point,
            presig,
        )
    }

    fn prepare_key_info(
        &mut self,
        wallet: &HotWallet,
    ) -> Result<PrivateSwapPrepared, HotWalletError> {
        let mut key_id = [0u8; 32];
        fill_random(&mut key_id)?;
        if key_id == [0; 32] {
            return Err(HotWalletError::EntropyUnavailable);
        }
        let seed = wallet.seed_bytes()?;
        let pubkey = covenant::private_swap_public_key(seed, &key_id)
            .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        let adaptor = covenant::private_swap_adaptor_point(seed, &key_id)
            .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        self.reset_active();
        self.mode = PrivateSwapMode::KeyInfo;
        self.key_id = key_id;
        self.claim_pubkey = pubkey;
        self.adaptor_point = adaptor;
        self.pending_key_id = key_id;
        self.pending_pubkey = pubkey;
        self.pending_adaptor_point = adaptor;
        self.phase = PrivateSwapPhase::FinalResponse;
        self.build_response(ResponseKind::KeyInfo, [0; 64], false)
            .map(PrivateSwapPrepared::Response)
    }

    fn prepare_bind(
        &mut self,
        request: &wire::PrivateSwapRequest<'_>,
    ) -> Result<PrivateSwapPrepared, HotWalletError> {
        if request.key_id != self.pending_key_id
            || request.adaptor_point != self.pending_adaptor_point
        {
            return Err(HotWalletError::InvalidToolInput);
        }
        let policy = swap_tx::parse_private_swap_script(request.payload)
            .map_err(|_| HotWalletError::InvalidToolInput)?;
        if policy.claimer_pubkey != self.pending_pubkey {
            return Err(HotWalletError::InvalidToolInput);
        }
        self.reset_active();
        self.mode = PrivateSwapMode::Bind;
        self.phase = PrivateSwapPhase::Prepared;
        self.key_id = request.key_id;
        self.claim_pubkey = policy.claimer_pubkey;
        self.adaptor_point = request.adaptor_point;
        self.script_hash = Sha256::digest(request.payload).into();
        self.refund_locktime_daa = policy.refund_locktime_daa;
        self.destination_hash = Sha256::digest(&policy.destination_spk).into();
        Ok(PrivateSwapPrepared::Review(Box::new(self.review())))
    }

    fn complete_binding(&mut self, wallet: &HotWallet) -> Result<Vec<u8>, HotWalletError> {
        if self.key_id != self.pending_key_id {
            return Err(HotWalletError::InvalidToolInput);
        }
        self.binding_token = covenant::private_swap_binding_token(
            wallet.seed_bytes()?,
            &self.key_id,
            &self.script_hash,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        self.phase = PrivateSwapPhase::FinalResponse;
        self.pending_key_id.zeroize();
        self.pending_pubkey.zeroize();
        self.pending_adaptor_point.zeroize();
        self.build_response(ResponseKind::Binding, [0; 64], false)
    }

    fn prepare_presign(
        &mut self,
        wallet: &HotWallet,
        request: &wire::PrivateSwapRequest<'_>,
    ) -> Result<PrivateSwapPrepared, HotWalletError> {
        let seed = wallet.seed_bytes()?;
        let mut claim = parse_claim_context(seed, request)?;
        if !covenant::private_swap_binding_matches(
            seed,
            &request.key_id,
            &claim.script_hash,
            &request.binding_token,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?
        {
            return Err(HotWalletError::CryptoOperationFailed);
        }
        let mut aux = [0u8; 32];
        fill_random(&mut aux)?;
        let nonce = covenant::private_swap_adaptor_base_nonce_point(
            seed,
            &request.key_id,
            &claim.sighash,
            &request.adaptor_point,
            &request.session_id,
            &aux,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        self.reset_active();
        self.mode = PrivateSwapMode::PreSign;
        self.phase = PrivateSwapPhase::Prepared;
        self.session_id = request.session_id;
        self.host_commitment = request.host_commitment;
        self.key_id = request.key_id;
        self.claim_pubkey = claim.pubkey;
        self.binding_token = request.binding_token;
        self.adaptor_point = request.adaptor_point;
        self.script_hash = claim.script_hash;
        self.sighash = claim.sighash;
        self.nonce_point = nonce;
        self.aux_rand = aux;
        self.install_claim_review(&claim)?;
        claim.transaction.clear();
        Ok(PrivateSwapPrepared::Review(Box::new(self.review())))
    }

    fn prepare_complete(
        &mut self,
        wallet: &HotWallet,
        request: &wire::PrivateSwapRequest<'_>,
    ) -> Result<PrivateSwapPrepared, HotWalletError> {
        let seed = wallet.seed_bytes()?;
        let mut claim = parse_claim_context(seed, request)?;
        let own_adaptor = covenant::private_swap_adaptor_point(seed, &request.key_id)
            .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        if own_adaptor != request.adaptor_point {
            return Err(HotWalletError::InvalidToolInput);
        }
        let bound = covenant::private_swap_binding_matches(
            seed,
            &request.key_id,
            &claim.script_hash,
            &request.binding_token,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        let presig = AdaptorPreSignature {
            bytes: request.presignature,
            negated: request.presignature_negated,
        };
        if !bound
            || offline_signer::crypto::adaptor::verify_adaptor_presignature(
                &claim.pubkey,
                &claim.sighash,
                &presig,
                &request.adaptor_point,
            )
            .is_err()
        {
            claim.transaction.clear();
            return Err(HotWalletError::CryptoOperationFailed);
        }
        self.reset_active();
        self.mode = PrivateSwapMode::Complete;
        self.phase = PrivateSwapPhase::Prepared;
        self.key_id = request.key_id;
        self.claim_pubkey = claim.pubkey;
        self.binding_token = request.binding_token;
        self.adaptor_point = request.adaptor_point;
        self.script_hash = claim.script_hash;
        self.sighash = claim.sighash;
        self.presignature = request.presignature;
        self.presignature_negated = request.presignature_negated;
        self.install_claim_review(&claim)?;
        claim.transaction.clear();
        Ok(PrivateSwapPrepared::Review(Box::new(self.review())))
    }

    fn install_claim_review(&mut self, claim: &ClaimContext) -> Result<(), HotWalletError> {
        self.input_amount = claim.input_amount;
        self.output_amount = claim.output_amount;
        self.fee = claim
            .input_amount
            .checked_sub(claim.output_amount)
            .ok_or(HotWalletError::InvalidToolInput)?;
        self.refund_locktime_daa = claim.policy.refund_locktime_daa;
        self.destination_hash = Sha256::digest(&claim.policy.destination_spk).into();
        Ok(())
    }

    fn complete_claim(&mut self, wallet: &HotWallet) -> Result<Vec<u8>, HotWalletError> {
        let presig = AdaptorPreSignature {
            bytes: self.presignature,
            negated: self.presignature_negated,
        };
        let completed = covenant::complete_private_swap_adaptor_presignature(
            wallet.seed_bytes()?,
            &self.key_id,
            &presig,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        let signature = SchnorrSignature { bytes: completed };
        schnorr_verify(&self.claim_pubkey, &self.sighash, &signature)
            .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        self.phase = PrivateSwapPhase::FinalResponse;
        self.build_response(ResponseKind::Completed, completed, false)
    }

    fn review(&self) -> PrivateSwapReview {
        PrivateSwapReview {
            mode: self.mode,
            key_id: self.key_id,
            claim_pubkey: self.claim_pubkey,
            adaptor_point: self.adaptor_point,
            script_hash: self.script_hash,
            sighash: self.sighash,
            input_amount: self.input_amount,
            output_amount: self.output_amount,
            fee: self.fee,
            refund_locktime_daa: self.refund_locktime_daa,
            destination_hash: self.destination_hash,
        }
    }

    fn build_response(
        &self,
        kind: ResponseKind,
        signature: [u8; 64],
        negated: bool,
    ) -> Result<Vec<u8>, HotWalletError> {
        let response = PrivateSwapResponse {
            kind,
            session_id: if matches!(kind, ResponseKind::Nonce | ResponseKind::PreSignature) {
                self.session_id
            } else {
                [0; 16]
            },
            key_id: self.key_id,
            claim_pubkey: self.claim_pubkey,
            binding_token: if matches!(
                kind,
                ResponseKind::Binding
                    | ResponseKind::Nonce
                    | ResponseKind::PreSignature
                    | ResponseKind::Completed
            ) {
                self.binding_token
            } else {
                [0; 32]
            },
            adaptor_point: self.adaptor_point,
            commitment: match kind {
                ResponseKind::Binding => self.script_hash,
                ResponseKind::Nonce | ResponseKind::PreSignature | ResponseKind::Completed => {
                    self.sighash
                }
                ResponseKind::KeyInfo => [0; 32],
            },
            nonce_point: if matches!(kind, ResponseKind::Nonce | ResponseKind::PreSignature) {
                self.nonce_point
            } else {
                [0; 33]
            },
            signature,
            negated,
        };
        let mut out = vec![0u8; wire::RESPONSE_LEN];
        let length = wire::encode_response(&response, &mut out)
            .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        out.truncate(length);
        Ok(out)
    }
}

struct ClaimContext {
    transaction: Transaction,
    pubkey: [u8; 32],
    sighash: [u8; 32],
    policy: swap_tx::PrivateSwapScript,
    script_hash: [u8; 32],
    input_amount: u64,
    output_amount: u64,
}

fn parse_claim_context(
    seed: &[u8; 64],
    request: &wire::PrivateSwapRequest<'_>,
) -> Result<ClaimContext, HotWalletError> {
    let pubkey = covenant::private_swap_public_key(seed, &request.key_id)
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
    let mut transaction = Transaction::try_new()?;
    kspt::parse_compact_kspt(request.payload, &mut transaction)?;
    let (sighash, policy) = swap_tx::private_swap_claim_sighash(&transaction, &pubkey)
        .map_err(|_| HotWalletError::InvalidToolInput)?;
    let script_hash = Sha256::digest(transaction.redeem_bytes(0)).into();
    let input_amount = transaction.inputs[0].utxo_entry.amount;
    let output_amount = transaction.outputs[0].value;
    Ok(ClaimContext {
        transaction,
        pubkey,
        sighash,
        policy,
        script_hash,
        input_amount,
        output_amount,
    })
}

fn require_mnemonic(wallet: &HotWallet) -> Result<(), HotWalletError> {
    (wallet.wallet_kind() == WalletKind::Mnemonic)
        .then_some(())
        .ok_or(HotWalletError::UnsupportedForWalletType)
}
