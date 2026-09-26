//! Universal covenant-key cryptography for software Vault shells.
//!
//! The transport/state machine stays in `vault-runtime`; this module keeps
//! mnemonic-derived covenant keys and anti-klepto signing inside custody.

use offline_signer::{
    crypto::{anti_klepto, schnorr::SchnorrSignature},
    derivation::covenant,
};
use zeroize::Zeroize;

use crate::{entropy::fill_random, HotWallet, HotWalletError};

pub struct CovenantProvisional {
    pub signature: [u8; 64],
    pub nonce_point: [u8; 33],
}

impl HotWallet {
    pub fn covenant_allocate_key(&self) -> Result<([u8; 32], [u8; 32]), HotWalletError> {
        let seed = self.seed_bytes()?;
        let mut key_id = [0u8; 32];
        fill_random(&mut key_id)?;
        if key_id == [0u8; 32] {
            key_id.zeroize();
            return Err(HotWalletError::EntropyUnavailable);
        }
        let pubkey = covenant::covenant_public_key(seed, &key_id)
            .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        Ok((key_id, pubkey))
    }

    pub fn covenant_public_key(&self, key_id: &[u8; 32]) -> Result<[u8; 32], HotWalletError> {
        covenant::covenant_public_key(self.seed_bytes()?, key_id)
            .map_err(|_| HotWalletError::CryptoOperationFailed)
    }

    pub fn covenant_binding_token(
        &self,
        key_id: &[u8; 32],
        script_hash: &[u8; 32],
    ) -> Result<[u8; 32], HotWalletError> {
        covenant::covenant_binding_token(self.seed_bytes()?, key_id, script_hash)
            .map_err(|_| HotWalletError::CryptoOperationFailed)
    }

    pub fn covenant_binding_matches(
        &self,
        key_id: &[u8; 32],
        script_hash: &[u8; 32],
        token: &[u8; 32],
    ) -> Result<bool, HotWalletError> {
        covenant::covenant_binding_matches(self.seed_bytes()?, key_id, script_hash, token)
            .map_err(|_| HotWalletError::CryptoOperationFailed)
    }

    pub fn covenant_begin_signature(
        &self,
        key_id: &[u8; 32],
        commitment: &[u8; 32],
    ) -> Result<CovenantProvisional, HotWalletError> {
        let mut aux = [0u8; 32];
        fill_random(&mut aux)?;
        let provisional =
            covenant::provisional_covenant_signature(self.seed_bytes()?, key_id, commitment, &aux)
                .map_err(|_| HotWalletError::CryptoOperationFailed);
        aux.zeroize();
        let provisional = provisional?;
        let nonce_point = anti_klepto::provisional_nonce_point(&provisional);
        Ok(CovenantProvisional {
            signature: provisional.bytes,
            nonce_point,
        })
    }

    pub fn covenant_finalize_signature(
        &self,
        key_id: &[u8; 32],
        commitment: &[u8; 32],
        provisional_signature: &[u8; 64],
        nonce_point: &[u8; 33],
        session_id: &[u8; shared_signer::covenant_sign::SESSION_ID_LEN],
        host_secret: &[u8; 32],
    ) -> Result<[u8; 64], HotWalletError> {
        let provisional = SchnorrSignature {
            bytes: *provisional_signature,
        };
        let final_signature = covenant::finalize_covenant_signature(
            self.seed_bytes()?,
            key_id,
            commitment,
            &provisional,
            session_id,
            host_secret,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        let pubkey_x = self.covenant_public_key(key_id)?;
        let mut public_key = [0u8; 33];
        public_key[0] = 0x02;
        public_key[1..].copy_from_slice(&pubkey_x);
        anti_klepto::verify_nonce_relation(
            nonce_point,
            &final_signature,
            session_id,
            host_secret,
            0,
            0,
            &public_key,
        )
        .map_err(|_| HotWalletError::CryptoOperationFailed)?;
        public_key.zeroize();
        Ok(final_signature.bytes)
    }
}
