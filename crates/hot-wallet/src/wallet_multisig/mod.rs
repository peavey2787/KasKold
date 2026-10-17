//! Multisig descriptor construction, import, and address derivation.

use offline_signer::{
    address::{script_to_p2sh_address, KaspaNetwork, MAX_ADDR_LEN},
    derivation::xpub::{self, KpubParts},
    transaction::model::{MultisigConfig, MAX_MULTISIG_KEYS},
};
use zeroize::Zeroize;

use super::{HotWallet, HotWalletError};

pub struct MultisigView {
    pub descriptor: String,
    pub address: String,
    pub threshold: u8,
    pub participants: u8,
    pub chain: u8,
    pub index: u32,
}

impl HotWallet {
    /// Build a canonical BIP45 `multi_hd45` descriptor containing this wallet
    /// and the supplied cosigner kpubs, then derive its P2SH address.
    pub fn create_multisig(
        &mut self,
        threshold: u8,
        cosigner_kpubs: &[&str],
        network: KaspaNetwork,
        chain: u8,
        index: u32,
    ) -> Result<MultisigView, HotWalletError> {
        let participant_count = cosigner_kpubs.len() + 1;
        validate_multisig_request(threshold, participant_count, network, chain, index)?;
        let own = xpub::derive_multisig_account_parts(self.seed_bytes()?, 0)?;
        let mut config = build_hd45_config(threshold, participant_count, chain, index, &own)?;
        add_cosigners(&mut config, cosigner_kpubs)?;
        finalize_owned_config(&mut config, &own)?;
        let descriptor = descriptor_text(&config)?;
        let address = config_address(&config, network)?;
        self.remember_multisig(config)?;
        Ok(MultisigView {
            descriptor,
            address,
            threshold,
            participants: participant_count as u8,
            chain,
            index,
        })
    }

    /// Import any supported hardware descriptor and reproduce its address.
    pub fn import_multisig_descriptor(
        &mut self,
        descriptor: &str,
        network: KaspaNetwork,
        chain: u8,
        index: u32,
    ) -> Result<MultisigView, HotWalletError> {
        use kaskold_protocol::wire::multisig_descriptor::{
            parse_multisig_descriptor, MultisigDescriptorKind,
        };
        validate_multisig_location(network, chain, index)?;
        let parsed = parse_multisig_descriptor(descriptor.as_bytes())
            .map_err(|_| HotWalletError::MultisigInvalid)?;
        let address = match parsed.kind {
            MultisigDescriptorKind::Static => static_multisig_address(&parsed, network)?,
            MultisigDescriptorKind::Hd44 | MultisigDescriptorKind::Hd45 => {
                self.import_hd_multisig(&parsed, network, chain, index)?
            }
        };
        Ok(MultisigView {
            descriptor: descriptor.trim().to_owned(),
            address,
            threshold: parsed.threshold,
            participants: parsed.participant_count,
            chain,
            index,
        })
    }

    fn import_hd_multisig(
        &mut self,
        parsed: &kaskold_protocol::wire::multisig_descriptor::ParsedMultisigDescriptor,
        network: KaspaNetwork,
        chain: u8,
        index: u32,
    ) -> Result<String, HotWalletError> {
        let mut config = MultisigConfig::from_descriptor(parsed);
        config.chain = chain;
        config.addr_index = index;
        self.resolve_imported_cosigner(&mut config)?;
        if config.build_script() == 0 {
            return Err(HotWalletError::MultisigInvalid);
        }
        config.active = true;
        let address = config_address(&config, network)?;
        self.remember_multisig(config)?;
        Ok(address)
    }

    /// Record this wallet's position in an imported HD45 descriptor. Only
    /// mnemonic wallets hold the seed that derives a BIP45 cosigner key.
    fn resolve_imported_cosigner(&self, config: &mut MultisigConfig) -> Result<(), HotWalletError> {
        let Ok(seed) = self.seed_bytes() else {
            return Ok(());
        };
        let own = xpub::derive_multisig_account_parts(seed, 0)?;
        let _ = config.resolve_cosigner_index(&own);
        Ok(())
    }
}

impl HotWallet {
    fn remember_multisig(&mut self, config: MultisigConfig) -> Result<(), HotWalletError> {
        if let Some(existing) = self
            .multisig_store
            .configs
            .iter_mut()
            .find(|existing| existing.active && existing.same_wallet_as(&config))
        {
            *existing = config;
            return Ok(());
        }
        let Some(index) = self.multisig_store.find_free() else {
            return Err(HotWalletError::MultisigInvalid);
        };
        self.multisig_store.configs[index] = config;
        Ok(())
    }
}

fn validate_multisig_request(
    threshold: u8,
    participant_count: usize,
    network: KaspaNetwork,
    chain: u8,
    index: u32,
) -> Result<(), HotWalletError> {
    if !(2..=MAX_MULTISIG_KEYS).contains(&participant_count) {
        return Err(HotWalletError::InvalidToolInput);
    }
    if threshold == 0 || usize::from(threshold) > participant_count {
        return Err(HotWalletError::InvalidToolInput);
    }
    validate_multisig_location(network, chain, index)
}

fn validate_multisig_location(
    network: KaspaNetwork,
    chain: u8,
    index: u32,
) -> Result<(), HotWalletError> {
    if chain > 1 || index >= 0x8000_0000 || network == KaspaNetwork::Unknown {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(())
}

fn build_hd45_config(
    threshold: u8,
    participant_count: usize,
    chain: u8,
    index: u32,
    own: &KpubParts,
) -> Result<MultisigConfig, HotWalletError> {
    let mut config = MultisigConfig::new();
    config.m = threshold;
    config.n = participant_count as u8;
    config.v45 = true;
    config.chain = chain;
    config.addr_index = index;
    if !config.set_cosigner(0, own) {
        return Err(HotWalletError::MultisigInvalid);
    }
    Ok(config)
}

fn add_cosigners(
    config: &mut MultisigConfig,
    cosigner_kpubs: &[&str],
) -> Result<(), HotWalletError> {
    for (offset, text) in cosigner_kpubs.iter().enumerate() {
        let parts = xpub::parse_kpub_parts(text.trim().as_bytes())
            .ok_or(HotWalletError::MultisigInvalid)?;
        if !config.set_cosigner(offset + 1, &parts) {
            return Err(HotWalletError::MultisigInvalid);
        }
    }
    Ok(())
}

fn finalize_owned_config(
    config: &mut MultisigConfig,
    own: &KpubParts,
) -> Result<(), HotWalletError> {
    config.sort_cosigners();
    if !config.resolve_cosigner_index(own) {
        return Err(HotWalletError::MultisigInvalid);
    }
    if config.build_script() == 0 {
        return Err(HotWalletError::MultisigInvalid);
    }
    config.active = true;
    Ok(())
}

fn static_multisig_address(
    parsed: &kaskold_protocol::wire::multisig_descriptor::ParsedMultisigDescriptor,
    network: KaspaNetwork,
) -> Result<String, HotWalletError> {
    let count = usize::from(parsed.participant_count);
    // Static cosigner keys are sorted before scripting, exactly as every
    // watch-only wallet reproduces the same descriptor's address.
    let mut keys = parsed.static_public_keys[..count].to_vec();
    keys.sort_unstable();
    let script = offline_signer::derivation::multisig::build_redeem_script(parsed.threshold, &keys)
        .map_err(|_| HotWalletError::MultisigInvalid)?;
    let prefix = network.hrp().ok_or(HotWalletError::InvalidToolInput)?;
    Ok(script_to_p2sh_address(&script, prefix))
}

fn descriptor_text(config: &MultisigConfig) -> Result<String, HotWalletError> {
    let mut encoded = [0u8; MultisigConfig::DESCRIPTOR_MAX_LEN];
    let length = config
        .write_descriptor(&mut encoded)
        .ok_or(HotWalletError::MultisigInvalid)?;
    let text = core::str::from_utf8(&encoded[..length])
        .map(str::to_owned)
        .map_err(|_| HotWalletError::MultisigInvalid);
    encoded.zeroize();
    text
}

fn config_address(
    config: &MultisigConfig,
    network: KaspaNetwork,
) -> Result<String, HotWalletError> {
    let mut encoded = [0u8; MAX_ADDR_LEN];
    let length = config.address(network, &mut encoded);
    if length == 0 {
        return Err(HotWalletError::InvalidToolInput);
    }
    core::str::from_utf8(&encoded[..length])
        .map(str::to_owned)
        .map_err(|_| HotWalletError::InvalidToolInput)
}

#[cfg(test)]
mod unit_tests;
