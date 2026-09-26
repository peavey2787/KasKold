//! Parser for the PSKT global map.

use crate::transaction::model::{
    Transaction, MAX_INPUTS, MAX_OUTPUTS, MAX_PAYLOAD_SIZE, SUBNETWORK_ID_NATIVE,
};
use shared_signer::{PsktParsed, PsktUnknownScope};

use super::super::preservation::{capture_unknown, capture_unknown_keyed};
use super::super::{PskError, Tok, Tokenizer};
use super::helpers::{
    capture_nonempty_object, consume_object_separator, expect, expect_bool, expect_string,
    expect_u64, mark_schema_field, parse_hex_field, parse_legacy_u64_num, reject_empty_object,
    require_schema_fields, skip_value,
};
use super::{ParseContext, PSKT_VERSION_OK};

pub(super) fn parse_global(
    tok: &mut Tokenizer<'_>,
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
    context: &mut ParseContext,
) -> Result<(), PskError> {
    expect(tok, Tok::LBrace)?;
    reject_empty_object(tok)?;

    let mut parser = GlobalParser {
        tx,
        parsed,
        context,
        seen: 0,
    };
    loop {
        parser.parse_member(tok)?;
        if !consume_object_separator(tok)? {
            break;
        }
    }
    parser.require_fields()
}

struct GlobalParser<'a> {
    tx: &'a mut Transaction,
    parsed: &'a mut PsktParsed,
    context: &'a mut ParseContext,
    seen: u64,
}

impl GlobalParser<'_> {
    fn parse_member(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let key_start = tok.position();
        let key = expect_string(tok)?;
        expect(tok, Tok::Colon)?;
        mark_schema_field(
            tok,
            &mut self.seen,
            kaskold_protocol::wire::pskt_schema::Scope::Global,
            key,
        )?;
        self.parse_field(tok, key_start, key)
    }

    fn parse_primary_field(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
        key: &[u8],
    ) -> Option<Result<(), PskError>> {
        match key {
            b"version" => Some(self.parse_pskt_version(tok)),
            b"txVersion" => Some(self.parse_transaction_version(tok)),
            b"fallbackLockTime" => Some(self.parse_optional_lock_time(tok, key_start)),
            b"inputsModifiable" => Some(self.parse_modifiable(tok, key_start)),
            b"outputsModifiable" => Some(self.parse_modifiable(tok, key_start)),
            b"subnetworkId" => Some(self.parse_subnetwork_id(tok)),
            b"gas" => Some(self.parse_gas(tok)),
            b"txPayload" => Some(self.parse_tx_payload(tok)),
            _ => None,
        }
    }

    fn parse_field(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
        key: &[u8],
    ) -> Result<(), PskError> {
        if let Some(result) = self.parse_primary_field(tok, key_start, key) {
            return result;
        }
        match key {
            b"inputCount" => self.parse_input_count(tok),
            b"outputCount" => self.parse_output_count(tok),
            b"xpubs" => self.parse_preserved_object(tok, key_start),
            b"id" => self.parse_optional_id(tok, key_start),
            b"proprietaries" => self.parse_preserved_object(tok, key_start),
            b"covenantBranch" => self.parse_covenant_branch(tok, key_start),
            _ => self.preserve_unknown(tok, key_start),
        }
    }

    fn parse_pskt_version(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        if expect_u64(tok)? != PSKT_VERSION_OK {
            return Err(PskError::VersionNotSupported);
        }
        Ok(())
    }

    fn parse_transaction_version(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let version = expect_u64(tok)?;
        if !kaskold_protocol::wire::pskt_schema::supported_tx_version_u64(version) {
            return Err(PskError::VersionNotSupported);
        }
        self.tx.version = u16::try_from(version).map_err(|_| PskError::VersionNotSupported)?;
        Ok(())
    }

    fn parse_subnetwork_id(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        match tok.next_token()? {
            Tok::Null => {
                self.tx.subnetwork_id = SUBNETWORK_ID_NATIVE;
                Ok(())
            }
            Tok::Str(value) => {
                if value.len() != 40 {
                    return Err(PskError::InvalidScriptLen);
                }
                parse_hex_field(value, &mut self.tx.subnetwork_id).map(|_| ())
            }
            _ => Err(PskError::UnexpectedToken),
        }
    }

    fn parse_gas(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        self.tx.gas = match tok.next_token()? {
            Tok::Null => 0,
            Tok::Num(value) => parse_legacy_u64_num(value)?,
            Tok::Str(value) => super::super::parse_u64_num(value)?,
            _ => return Err(PskError::UnexpectedToken),
        };
        Ok(())
    }

    fn parse_tx_payload(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        match tok.next_token()? {
            Tok::Null => {
                self.tx.payload.fill(0);
                self.tx.payload_len = 0;
                Ok(())
            }
            Tok::Str(value) => {
                if value.len() > MAX_PAYLOAD_SIZE.saturating_mul(2) {
                    return Err(PskError::InvalidScriptLen);
                }
                self.tx.payload.fill(0);
                self.tx.payload_len = parse_hex_field(value, &mut self.tx.payload[..])?;
                Ok(())
            }
            _ => Err(PskError::UnexpectedToken),
        }
    }

    fn parse_optional_lock_time(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        match tok.next_token()? {
            Tok::Null => Ok(()),
            Tok::Num(value) => {
                self.tx.locktime = parse_legacy_u64_num(value)?;
                self.capture(key_start, tok.position())
            }
            Tok::Str(value) => {
                self.tx.locktime = super::super::parse_u64_num(value)?;
                self.capture(key_start, tok.position())
            }
            _ => Err(PskError::UnexpectedToken),
        }
    }

    fn parse_modifiable(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        if expect_bool(tok)? {
            return Ok(());
        }
        self.capture(key_start, tok.position())
    }

    fn parse_input_count(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let count = expect_u64(tok)?;
        let count = usize::try_from(count).map_err(|_| PskError::TooManyInputs)?;
        if count > MAX_INPUTS {
            return Err(PskError::TooManyInputs);
        }
        self.context.declared_input_count = Some(count);
        Ok(())
    }

    fn parse_output_count(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let count = expect_u64(tok)?;
        if count > MAX_OUTPUTS as u64 {
            return Err(PskError::TooManyOutputs);
        }
        self.context.declared_output_count = Some(count as usize);
        Ok(())
    }

    fn parse_preserved_object(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        capture_nonempty_object(tok, self.parsed, key_start, PsktUnknownScope::global())
    }

    fn parse_optional_id(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        match tok.next_token()? {
            Tok::Null => Ok(()),
            Tok::Str(_) => self.capture(key_start, tok.position()),
            _ => Err(PskError::UnexpectedToken),
        }
    }

    fn parse_covenant_branch(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        match tok.next_token()? {
            Tok::Null => Ok(()),
            Tok::Str(b"owner" | b"owner-time" | b"beneficiary" | b"savings") => {
                self.capture(key_start, tok.position())
            }
            _ => Err(PskError::UnexpectedToken),
        }
    }

    fn preserve_unknown(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        skip_value(tok)?;
        capture_unknown_keyed(
            self.parsed,
            PsktUnknownScope::global(),
            tok.source(),
            key_start,
            tok.position(),
        )
    }

    fn capture(&mut self, field_start: usize, field_end: usize) -> Result<(), PskError> {
        capture_unknown(
            self.parsed,
            PsktUnknownScope::global(),
            field_start,
            field_end,
        )
    }

    fn require_fields(&self) -> Result<(), PskError> {
        require_schema_fields(
            kaskold_protocol::wire::pskt_schema::Scope::Global,
            self.seen,
        )
    }
}
