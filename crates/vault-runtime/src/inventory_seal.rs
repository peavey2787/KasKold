//! Authenticated native multi-wallet persistence envelope.
//!
//! Individual wallet custody records remain encrypted by `HotWallet` KHV3.
//! This envelope authenticates the slot ordering, active slot, and wallet names
//! with a wrapping-key HMAC so Android/iOS can persist the shared runtime
//! inventory without inventing a platform-specific plaintext wallet catalog.

use hmac::{Hmac, Mac};
use hot_wallet::{HotWallet, PLATFORM_WRAPPING_KEY_LEN};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::{wallet_tools::validate_wallet_name, VaultRuntime, VaultRuntimeError};

const MAGIC: &[u8; 4] = b"KVI1";
const HEADER_LEN: usize = 8;
const TAG_LEN: usize = 32;
const HMAC_DOMAIN: &[u8] = b"KasKold/VaultInventory/v1";
const MAX_SEALED_SLOT_LEN: usize = 1024;

impl VaultRuntime {
    /// Seal the complete native wallet inventory, including active slot and
    /// names, under the same short-lived platform wrapping key used by KHV3.
    pub fn seal_native_inventory(
        &self,
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<Vec<u8>, VaultRuntimeError> {
        let active = validate_inventory_for_seal(self)?;

        let mut output = Vec::with_capacity(HEADER_LEN + self.wallets.len() * 256 + TAG_LEN);
        output.extend_from_slice(MAGIC);
        output.push(u8::try_from(active).map_err(|_| VaultRuntimeError::InvalidWalletIndex)?);
        output.push(
            u8::try_from(self.wallets.len()).map_err(|_| VaultRuntimeError::InvalidWalletIndex)?,
        );
        output.extend_from_slice(&[0, 0]);

        for (index, wallet) in self.wallets.iter().enumerate() {
            append_inventory_slot(self, index, wallet, wrapping_key, &mut output)?;
        }

        let tag = inventory_mac(wrapping_key, &output)?
            .finalize()
            .into_bytes();
        output.extend_from_slice(&tag);
        Ok(output)
    }

    /// Restore a complete native inventory (`KVI1`) or a single KHV3 wallet as one slot.
    pub fn unlock_native_inventory(
        &mut self,
        sealed: &[u8],
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<Option<String>, VaultRuntimeError> {
        if !sealed.starts_with(MAGIC) {
            return self.unlock_single_wallet(sealed, wrapping_key);
        }
        let body = authenticated_inventory_body(sealed, wrapping_key)?;
        let (active, count) = parse_inventory_header(body)?;
        let (wallets, names) = decode_inventory_slots(body, count, wrapping_key)?;

        let kpub = wallets[active].export_kpub().ok();
        self.install_inventory(wallets, names, active);
        Ok(kpub)
    }

    fn unlock_single_wallet(
        &mut self,
        sealed: &[u8],
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<Option<String>, VaultRuntimeError> {
        let wallet = HotWallet::restore_platform_sealed(sealed, wrapping_key)
            .map_err(VaultRuntimeError::Custody)?;
        let kpub = wallet.export_kpub().ok();
        self.install_inventory(vec![wallet], vec!["Wallet 1".to_owned()], 0);
        Ok(kpub)
    }

    fn install_inventory(&mut self, wallets: Vec<HotWallet>, names: Vec<String>, active: usize) {
        self.clear_signing_session();
        self.covenant.clear_all();
        self.wallets = wallets;
        self.wallet_names = names;
        self.active_wallet = Some(active);
    }
}

fn validate_inventory_for_seal(runtime: &VaultRuntime) -> Result<usize, VaultRuntimeError> {
    if runtime.wallets.is_empty() {
        return Err(VaultRuntimeError::Locked);
    }
    let active = runtime.active_wallet.ok_or(VaultRuntimeError::Locked)?;
    if active >= runtime.wallets.len() {
        return Err(VaultRuntimeError::InvalidWalletIndex);
    }
    if runtime.wallets.len() > crate::wallet_tools::MAX_SOFTWARE_WALLETS {
        return Err(VaultRuntimeError::InvalidWalletIndex);
    }
    Ok(active)
}

fn append_inventory_slot(
    runtime: &VaultRuntime,
    index: usize,
    wallet: &HotWallet,
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    output: &mut Vec<u8>,
) -> Result<(), VaultRuntimeError> {
    let name = runtime
        .wallet_names
        .get(index)
        .ok_or(VaultRuntimeError::InvalidWalletIndex)?;
    append_inventory_name(output, name)?;
    append_sealed_wallet(output, wallet, wrapping_key)
}

fn append_inventory_name(output: &mut Vec<u8>, name: &str) -> Result<(), VaultRuntimeError> {
    let validated = validate_wallet_name(name)?;
    let name_bytes = validated.as_bytes();
    output.push(u8::try_from(name_bytes.len()).map_err(|_| VaultRuntimeError::InvalidWalletName)?);
    output.extend_from_slice(name_bytes);
    Ok(())
}

fn append_sealed_wallet(
    output: &mut Vec<u8>,
    wallet: &HotWallet,
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
) -> Result<(), VaultRuntimeError> {
    let mut sealed = wallet
        .seal_for_platform(wrapping_key)
        .map_err(VaultRuntimeError::Custody)?;
    let sealed_len =
        u16::try_from(sealed.len()).map_err(|_| VaultRuntimeError::InvalidSealedInventory)?;
    output.extend_from_slice(&sealed_len.to_le_bytes());
    output.extend_from_slice(&sealed);
    sealed.zeroize();
    Ok(())
}

fn authenticated_inventory_body<'a>(
    sealed: &'a [u8],
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
) -> Result<&'a [u8], VaultRuntimeError> {
    if sealed.len() < HEADER_LEN + TAG_LEN {
        return Err(VaultRuntimeError::InvalidSealedInventory);
    }
    let body_len = sealed.len() - TAG_LEN;
    let (body, supplied_tag) = sealed.split_at(body_len);
    inventory_mac(wrapping_key, body)?
        .verify_slice(supplied_tag)
        .map(|()| body)
        .map_err(|_| VaultRuntimeError::InvalidSealedInventory)
}

fn parse_inventory_header(body: &[u8]) -> Result<(usize, usize), VaultRuntimeError> {
    let active = usize::from(body[4]);
    let count = usize::from(body[5]);
    if body[6] != 0 || body[7] != 0 {
        return Err(VaultRuntimeError::InvalidSealedInventory);
    }
    if count == 0 || count > crate::wallet_tools::MAX_SOFTWARE_WALLETS {
        return Err(VaultRuntimeError::InvalidSealedInventory);
    }
    if active >= count {
        return Err(VaultRuntimeError::InvalidSealedInventory);
    }
    Ok((active, count))
}

fn decode_inventory_slots(
    body: &[u8],
    count: usize,
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
) -> Result<(Vec<HotWallet>, Vec<String>), VaultRuntimeError> {
    let mut cursor = HEADER_LEN;
    let mut wallets = Vec::with_capacity(count);
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        names.push(read_inventory_name(body, &mut cursor)?);
        wallets.push(read_inventory_wallet(body, &mut cursor, wrapping_key)?);
    }
    if cursor != body.len() {
        return Err(VaultRuntimeError::InvalidSealedInventory);
    }
    Ok((wallets, names))
}

fn read_inventory_name(body: &[u8], cursor: &mut usize) -> Result<String, VaultRuntimeError> {
    let name_len = usize::from(read_u8(body, cursor)?);
    let name_bytes = read_slice(body, cursor, name_len)?;
    let name =
        core::str::from_utf8(name_bytes).map_err(|_| VaultRuntimeError::InvalidSealedInventory)?;
    validate_wallet_name(name)
}

fn read_inventory_wallet(
    body: &[u8],
    cursor: &mut usize,
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
) -> Result<HotWallet, VaultRuntimeError> {
    let sealed_len_bytes = read_slice(body, cursor, 2)?;
    let sealed_len = usize::from(u16::from_le_bytes([
        sealed_len_bytes[0],
        sealed_len_bytes[1],
    ]));
    if sealed_len == 0 || sealed_len > MAX_SEALED_SLOT_LEN {
        return Err(VaultRuntimeError::InvalidSealedInventory);
    }
    let slot = read_slice(body, cursor, sealed_len)?;
    HotWallet::restore_platform_sealed(slot, wrapping_key).map_err(VaultRuntimeError::Custody)
}

fn read_u8(input: &[u8], cursor: &mut usize) -> Result<u8, VaultRuntimeError> {
    let value = *input
        .get(*cursor)
        .ok_or(VaultRuntimeError::InvalidSealedInventory)?;
    *cursor += 1;
    Ok(value)
}

fn read_slice<'a>(
    input: &'a [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'a [u8], VaultRuntimeError> {
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(VaultRuntimeError::InvalidSealedInventory)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

/// HMAC-SHA256 under the wrapping key over the inventory domain and body.
fn inventory_mac(
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    body: &[u8],
) -> Result<Hmac<Sha256>, VaultRuntimeError> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(wrapping_key)
        .map_err(|_| VaultRuntimeError::InvalidSealedInventory)?;
    mac.update(HMAC_DOMAIN);
    mac.update(body);
    Ok(mac)
}

#[cfg(test)]
mod unit_tests;
