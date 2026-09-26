//! Input UTXO and outpoint schema parsing.

use crate::transaction::model::TransactionInput;
use shared_signer::{PsktParsed, PsktUnknownScope};

use super::super::super::{
    hex_decode_strict,
    preservation::{capture_unknown, capture_unknown_keyed},
    PskError, Tok, Tokenizer,
};
use super::super::helpers::{
    consume_object_separator, expect, expect_bool, expect_string, expect_u64, mark_schema_field,
    parse_script_public_key, require_schema_fields, skip_value, validate_hex_string,
};

pub(super) fn parse_utxo_entry(
    tok: &mut Tokenizer<'_>,
    input: &mut TransactionInput,
    parsed: &mut PsktParsed,
    input_index: usize,
) -> Result<(), PskError> {
    expect(tok, Tok::LBrace)?;

    let mut parser = UtxoParser {
        input,
        parsed,
        input_index,
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

struct UtxoParser<'a> {
    input: &'a mut TransactionInput,
    parsed: &'a mut PsktParsed,
    input_index: usize,
    seen: u64,
}

impl UtxoParser<'_> {
    fn parse_member(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let key_start = tok.position();
        let key = expect_string(tok)?;
        expect(tok, Tok::Colon)?;
        mark_schema_field(
            tok,
            &mut self.seen,
            kaskold_protocol::wire::pskt_schema::Scope::InputUtxo,
            key,
        )?;
        self.parse_field(tok, key_start, key)
    }

    fn parse_field(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
        key: &[u8],
    ) -> Result<(), PskError> {
        match key {
            b"amount" => self.parse_amount(tok),
            b"scriptPublicKey" => self.parse_script_key(tok),
            b"blockDaaScore" => self.parse_block_daa_score(tok),
            b"isCoinbase" => self.parse_is_coinbase(tok, key_start),
            b"covenantId" => self.parse_covenant_id(tok, key_start),
            _ => self.preserve_unknown(tok, key_start),
        }
    }

    fn parse_amount(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        self.input.utxo_entry.amount = expect_u64(tok)?;
        Ok(())
    }

    fn parse_script_key(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let hex_str = expect_string(tok)?;
        parse_script_public_key(hex_str, &mut self.input.utxo_entry.script_public_key)
    }

    fn parse_block_daa_score(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        self.input.utxo_entry.block_daa_score = expect_u64(tok)?;
        Ok(())
    }

    fn parse_is_coinbase(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        let is_coinbase = expect_bool(tok)?;
        self.capture_when(is_coinbase, key_start, tok.position())
    }

    fn parse_covenant_id(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        match tok.peek()? {
            Tok::Null => {
                tok.next_token()?;
                Ok(())
            }
            Tok::Str(_) => {
                let value = expect_string(tok)?;
                if value.len() != 64 {
                    return Err(PskError::UnexpectedToken);
                }
                validate_hex_string(value)?;
                // The signer does not need the id value to authorize the spend,
                // but the Companion's verified materializer needs its presence
                // for exact consensus storage-mass accounting. Preserve it
                // byte-for-byte through signer round trips.
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
        let scope = PsktUnknownScope::input_utxo(
            u32::try_from(self.input_index).map_err(|_| PskError::CountMismatch)?,
        );
        capture_unknown_keyed(self.parsed, scope, tok.source(), key_start, tok.position())
    }

    fn capture_when(
        &mut self,
        should_capture: bool,
        field_start: usize,
        field_end: usize,
    ) -> Result<(), PskError> {
        if !should_capture {
            return Ok(());
        }
        self.capture(field_start, field_end)
    }

    fn capture(&mut self, field_start: usize, field_end: usize) -> Result<(), PskError> {
        let scope = PsktUnknownScope::input_utxo(
            u32::try_from(self.input_index).map_err(|_| PskError::CountMismatch)?,
        );
        capture_unknown(self.parsed, scope, field_start, field_end)
    }

    fn require_fields(&self) -> Result<(), PskError> {
        require_schema_fields(
            kaskold_protocol::wire::pskt_schema::Scope::InputUtxo,
            self.seen,
        )
    }
}

struct OutpointParser<'a> {
    input: &'a mut TransactionInput,
    parsed: &'a mut PsktParsed,
    input_index: usize,
    seen: u64,
}

impl OutpointParser<'_> {
    fn parse_object(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        loop {
            self.parse_member(tok)?;
            if !consume_object_separator(tok)? {
                break;
            }
        }
        self.require_fields()
    }

    fn parse_member(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let key_start = tok.position();
        let key = expect_string(tok)?;
        expect(tok, Tok::Colon)?;
        mark_schema_field(
            tok,
            &mut self.seen,
            kaskold_protocol::wire::pskt_schema::Scope::InputOutpoint,
            key,
        )?;
        match key {
            b"transactionId" => self.parse_transaction_id(tok),
            b"index" => self.parse_index(tok),
            _ => self.preserve_unknown(tok, key_start),
        }
    }

    fn parse_transaction_id(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let hex_str = expect_string(tok)?;
        if hex_str.len() != 64 {
            return Err(PskError::UnexpectedToken);
        }
        hex_decode_strict(hex_str, &mut self.input.previous_outpoint.transaction_id).map(|_| ())
    }

    fn parse_index(&mut self, tok: &mut Tokenizer<'_>) -> Result<(), PskError> {
        let index = expect_u64(tok)?;
        self.input.previous_outpoint.index =
            u32::try_from(index).map_err(|_| PskError::UnexpectedToken)?;
        Ok(())
    }

    fn preserve_unknown(
        &mut self,
        tok: &mut Tokenizer<'_>,
        key_start: usize,
    ) -> Result<(), PskError> {
        skip_value(tok)?;
        let scope = PsktUnknownScope::input_outpoint(
            u32::try_from(self.input_index).map_err(|_| PskError::CountMismatch)?,
        );
        capture_unknown_keyed(self.parsed, scope, tok.source(), key_start, tok.position())
    }

    fn require_fields(&self) -> Result<(), PskError> {
        require_schema_fields(
            kaskold_protocol::wire::pskt_schema::Scope::InputOutpoint,
            self.seen,
        )
    }
}

pub(super) fn parse_outpoint(
    tok: &mut Tokenizer<'_>,
    input: &mut TransactionInput,
    parsed: &mut PsktParsed,
    input_index: usize,
) -> Result<(), PskError> {
    expect(tok, Tok::LBrace)?;
    OutpointParser {
        input,
        parsed,
        input_index,
        seen: 0,
    }
    .parse_object(tok)
}
