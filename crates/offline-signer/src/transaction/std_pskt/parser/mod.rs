//! Top-level PSKT/PSKB parser orchestration.

mod derivation;
mod global;
mod helpers;
mod inputs;
mod outputs;

use crate::transaction::model::Transaction;
use shared_signer::{PsktParsed, PsktUnknownScope};

use super::preservation::capture_unknown_keyed;
use super::{
    hex_decode_strict, strip_pskt_magic, validate_monetary_shape, PskError, Tok, Tokenizer,
    PSKB_MAGIC, PSKT_MAGIC,
};
use global::parse_global;
use helpers::{expect, expect_string, mark_schema_field, require_schema_fields, skip_value};
use inputs::parse_inputs_array;
use outputs::parse_outputs_array;

pub(super) const PSKT_VERSION_OK: u64 = kaskold_protocol::wire::pskt_schema::PSKT_VERSION;
pub(super) const SIGHASH_ALL: u8 = kaskold_protocol::wire::pskt_schema::SIGHASH_ALL;

#[derive(Debug, Default)]
pub(super) struct ParseContext {
    pub(super) declared_input_count: Option<usize>,
    pub(super) declared_output_count: Option<usize>,
}

fn bundle_format(wire: &[u8]) -> Result<bool, PskError> {
    if &wire[..4] == PSKB_MAGIC {
        return Ok(true);
    }
    if &wire[..4] == PSKT_MAGIC {
        return Ok(false);
    }
    Err(PskError::BadMagic)
}

fn parse_decoded_json(
    json: &[u8],
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
    is_bundle: bool,
) -> Result<(), PskError> {
    kaskold_protocol::wire::pskt_schema::validate_canonical_json(json).map_err(|error| {
        use kaskold_protocol::wire::pskt_schema::JsonSyntaxError;
        match error {
            JsonSyntaxError::DuplicateKey => PskError::DuplicateField,
            JsonSyntaxError::NestingTooDeep => PskError::JsonNestingTooDeep,
            _ => PskError::UnexpectedToken,
        }
    })?;
    let mut tok = Tokenizer::new(json);
    if is_bundle {
        parse_bundle_array(&mut tok, tx, parsed)?;
    } else {
        parse_pskt_object(&mut tok, tx, parsed)?;
    }
    expect(&mut tok, Tok::Eof)
}

/// Decode a PSKB bundle or PSKT-single payload into the fixed transaction model.
pub fn parse_pskt(
    wire: &[u8],
    scratch: &mut [u8],
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
) -> Result<(), PskError> {
    let body_hex = strip_pskt_magic(wire)?;
    let is_bundle = bundle_format(wire)?;
    let json_len = hex_decode_strict(body_hex, scratch)?;
    let json_len_u16 = u16::try_from(json_len).map_err(|_| PskError::JsonTooLarge)?;
    *parsed = PsktParsed::empty();
    parsed.json_start = 0;
    parsed.json_len = json_len_u16;
    tx.prepare_for_parse();
    parse_decoded_json(&scratch[..json_len], tx, parsed, is_bundle)
}

fn parse_bundle_array(
    tok: &mut Tokenizer<'_>,
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
) -> Result<(), PskError> {
    expect(tok, Tok::LBracket)?;
    if matches!(tok.peek()?, Tok::RBracket) {
        return Err(PskError::MissingField);
    }

    parse_pskt_object(tok, tx, parsed)?;
    match tok.next_token()? {
        Tok::RBracket => Ok(()),
        Tok::Comma => Err(PskError::BundleMultiElement),
        _ => Err(PskError::UnexpectedToken),
    }
}

fn parse_top_level_field(
    tok: &mut Tokenizer<'_>,
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
    context: &mut ParseContext,
    seen: &mut u64,
) -> Result<(), PskError> {
    use kaskold_protocol::wire::pskt_schema::Scope;
    let key_start = tok.position();
    let key = expect_string(tok)?;
    expect(tok, Tok::Colon)?;
    let known = mark_schema_field(tok, seen, Scope::TopLevel, key)?;
    dispatch_top_level_field(tok, tx, parsed, context, key_start, key, known)
}

fn dispatch_top_level_field(
    tok: &mut Tokenizer<'_>,
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
    context: &mut ParseContext,
    key_start: usize,
    key: &[u8],
    known: bool,
) -> Result<(), PskError> {
    if let Some(result) = dispatch_known_top_level_field(tok, tx, parsed, context, key) {
        return result;
    }
    dispatch_unknown_top_level_field(tok, parsed, key_start, known)
}

fn dispatch_known_top_level_field(
    tok: &mut Tokenizer<'_>,
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
    context: &mut ParseContext,
    key: &[u8],
) -> Option<Result<(), PskError>> {
    match key {
        b"global" => Some(parse_global(tok, tx, parsed, context)),
        b"inputs" => Some(parse_inputs_array(tok, tx, parsed)),
        b"outputs" => Some(parse_outputs_array(tok, tx, parsed)),
        _ => None,
    }
}

fn dispatch_unknown_top_level_field(
    tok: &mut Tokenizer<'_>,
    parsed: &mut PsktParsed,
    key_start: usize,
    known: bool,
) -> Result<(), PskError> {
    if !known && kaskold_protocol::wire::pskt_schema::EXTENSION_GRAMMAR.preserve_unknown_fields {
        preserve_unknown_top_level(tok, parsed, key_start)
    } else {
        Err(PskError::UnexpectedToken)
    }
}

fn preserve_unknown_top_level(
    tok: &mut Tokenizer<'_>,
    parsed: &mut PsktParsed,
    key_start: usize,
) -> Result<(), PskError> {
    skip_value(tok)?;
    capture_unknown_keyed(
        parsed,
        PsktUnknownScope::top_level(),
        tok.source(),
        key_start,
        tok.position(),
    )
}

fn validate_top_level_object(
    tx: &Transaction,
    context: &ParseContext,
    seen: u64,
) -> Result<(), PskError> {
    require_schema_fields(kaskold_protocol::wire::pskt_schema::Scope::TopLevel, seen)?;
    if context.declared_input_count != Some(tx.num_inputs)
        || context.declared_output_count != Some(tx.num_outputs)
    {
        return Err(PskError::CountMismatch);
    }
    if tx.outputs[..tx.num_outputs]
        .iter()
        .any(|output| output.has_covenant && output.covenant_auth_input as usize >= tx.num_inputs)
    {
        return Err(PskError::InvalidCovenantBinding);
    }
    validate_monetary_shape(tx)
}

fn parse_pskt_object(
    tok: &mut Tokenizer<'_>,
    tx: &mut Transaction,
    parsed: &mut PsktParsed,
) -> Result<(), PskError> {
    expect(tok, Tok::LBrace)?;
    if matches!(tok.peek()?, Tok::RBrace) {
        return Err(PskError::MissingField);
    }

    let mut seen = 0u64;
    let mut context = ParseContext::default();
    loop {
        parse_top_level_field(tok, tx, parsed, &mut context, &mut seen)?;
        match tok.next_token()? {
            Tok::Comma => continue,
            Tok::RBrace => break,
            _ => return Err(PskError::UnexpectedToken),
        }
    }
    validate_top_level_object(tx, &context, seen)
}
