use core::convert::Infallible;

use sha2::{Digest, Sha256};

use crate::{board, peripherals::HMAC};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HmacPurpose {
    ToJtag,
    ToDs,
    ToUser,
    DsOrUser,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyId {
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    KeyPurposeMismatch,
}

const BLOCK: usize = 64;

/// HMAC-SHA256 under the virtual device's eFuse key, as the peripheral computes it.
pub struct Hmac<'d> {
    _peripheral: HMAC<'d>,
    key: [u8; 32],
    message: Vec<u8>,
}

impl<'d> Hmac<'d> {
    pub fn new(peripheral: HMAC<'d>) -> Self {
        Self { _peripheral: peripheral, key: [0; 32], message: Vec::new() }
    }

    pub fn init(&mut self) {
        self.message.clear();
    }

    pub fn configure(&mut self, purpose: HmacPurpose, key: KeyId) -> nb::Result<(), Error> {
        match board::efuse::hmac_key(key as u8, purpose == HmacPurpose::ToUser) {
            Some(bytes) => {
                self.key = bytes;
                Ok(())
            }
            None => Err(nb::Error::Other(Error::KeyPurposeMismatch)),
        }
    }

    pub fn update<'a>(&mut self, message: &'a [u8]) -> nb::Result<&'a [u8], Infallible> {
        self.message.extend_from_slice(message);
        Ok(&[])
    }

    pub fn finalize(&mut self, output: &mut [u8]) -> nb::Result<(), Infallible> {
        let mut pad = [0u8; BLOCK];
        pad[..32].copy_from_slice(&self.key);
        let inner = Sha256::new()
            .chain_update(pad.map(|b| b ^ 0x36))
            .chain_update(&self.message)
            .finalize();
        let outer = Sha256::new().chain_update(pad.map(|b| b ^ 0x5c)).chain_update(inner).finalize();
        output.copy_from_slice(&outer[..output.len()]);
        self.message.clear();
        Ok(())
    }
}
