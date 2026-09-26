//! Executable canonical PSKT schema shared by Companion and Vault.
//!
//! This module is deliberately `no_std`: both the host parser and the hardware
//! parser consume the same field tables, required-field masks, nullability
//! rules, transaction/sighash versions, extension policy, and restricted JSON
//! syntax validator.  Environment-specific parsers are only responsible for
//! materializing values into their own semantic models.

use core::fmt;

pub const PSKT_VERSION: u64 = 0;
pub const MAX_SUPPORTED_TX_VERSION: u16 = 1;
pub const SIGHASH_ALL: u8 = 1;
pub const MAX_SIGNATURES_PER_INPUT: u8 = 5;
pub const MAX_JSON_NESTING: usize = 32;
pub const JS_MAX_SAFE_U64: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    TopLevel,
    Global,
    Input,
    InputUtxo,
    InputOutpoint,
    Output,
    CovenantExecution,
    CovenantBinding,
    PartialSignature,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NullRule {
    Forbidden,
    AllowedAsDefault,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldClass {
    Standard,
    ProductExtension,
}

/// JSON value category enforced by every PSKT consumer before semantic use.
/// Field-specific bounds (hex length, integer range, supported enum values) are
/// layered on top of this shared shape and never replace it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Object,
    Array,
    Boolean,
    ExactUnsigned,
    String,
    HexString,
    ScriptPublicKey,
}

/// Canonical meaning of an omitted field (or null when `AllowedAsDefault`).
/// This is descriptive and executable: host/Vault parsers use the same rule
/// rather than inventing local defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultRule {
    None,
    Zero,
    One,
    False,
    EmptyObject,
    EmptyBytes,
    NativeSubnetwork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldSpec {
    pub name: &'static str,
    pub required: bool,
    pub null_rule: NullRule,
    pub class: FieldClass,
    pub kind: ValueKind,
    pub default: DefaultRule,
}

const fn field(
    name: &'static str,
    required: bool,
    null_rule: NullRule,
    kind: ValueKind,
    default: DefaultRule,
) -> FieldSpec {
    FieldSpec {
        name,
        required,
        null_rule,
        class: FieldClass::Standard,
        kind,
        default,
    }
}

const fn product(
    name: &'static str,
    null_rule: NullRule,
    kind: ValueKind,
    default: DefaultRule,
) -> FieldSpec {
    FieldSpec {
        name,
        required: false,
        null_rule,
        class: FieldClass::ProductExtension,
        kind,
        default,
    }
}

pub const TOP_LEVEL_FIELDS: &[FieldSpec] = &[
    field(
        "global",
        true,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::None,
    ),
    field(
        "inputs",
        true,
        NullRule::Forbidden,
        ValueKind::Array,
        DefaultRule::None,
    ),
    field(
        "outputs",
        true,
        NullRule::Forbidden,
        ValueKind::Array,
        DefaultRule::None,
    ),
];

pub const GLOBAL_FIELDS: &[FieldSpec] = &[
    field(
        "version",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "txVersion",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "inputCount",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "outputCount",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "fallbackLockTime",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::ExactUnsigned,
        DefaultRule::Zero,
    ),
    field(
        "inputsModifiable",
        false,
        NullRule::Forbidden,
        ValueKind::Boolean,
        DefaultRule::False,
    ),
    field(
        "outputsModifiable",
        false,
        NullRule::Forbidden,
        ValueKind::Boolean,
        DefaultRule::False,
    ),
    field(
        "xpubs",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
    field(
        "id",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::String,
        DefaultRule::None,
    ),
    field(
        "proprietaries",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
    field(
        "subnetworkId",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::HexString,
        DefaultRule::NativeSubnetwork,
    ),
    field(
        "gas",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::ExactUnsigned,
        DefaultRule::Zero,
    ),
    field(
        "txPayload",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::HexString,
        DefaultRule::EmptyBytes,
    ),
    product(
        "covenantBranch",
        NullRule::AllowedAsDefault,
        ValueKind::String,
        DefaultRule::None,
    ),
];

pub const INPUT_FIELDS: &[FieldSpec] = &[
    field(
        "utxoEntry",
        true,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::None,
    ),
    field(
        "previousOutpoint",
        true,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::None,
    ),
    field(
        "sighashType",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "sequence",
        false,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::Zero,
    ),
    field(
        "minTime",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::ExactUnsigned,
        DefaultRule::Zero,
    ),
    field(
        "partialSigs",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
    field(
        "redeemScript",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::HexString,
        DefaultRule::EmptyBytes,
    ),
    field(
        "sigOpCount",
        false,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::One,
    ),
    field(
        "bip32Derivations",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
    field(
        "finalScriptSig",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::HexString,
        DefaultRule::EmptyBytes,
    ),
    field(
        "covenantExecution",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::Object,
        DefaultRule::None,
    ),
    field(
        "proprietaries",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
    product(
        "minimumSignatures",
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::One,
    ),
];

pub const UTXO_FIELDS: &[FieldSpec] = &[
    field(
        "amount",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "scriptPublicKey",
        true,
        NullRule::Forbidden,
        ValueKind::ScriptPublicKey,
        DefaultRule::None,
    ),
    field(
        "blockDaaScore",
        false,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::Zero,
    ),
    field(
        "isCoinbase",
        false,
        NullRule::Forbidden,
        ValueKind::Boolean,
        DefaultRule::False,
    ),
    field(
        "covenantId",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::HexString,
        DefaultRule::None,
    ),
];

pub const OUTPOINT_FIELDS: &[FieldSpec] = &[
    field(
        "transactionId",
        true,
        NullRule::Forbidden,
        ValueKind::HexString,
        DefaultRule::None,
    ),
    field(
        "index",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
];

pub const OUTPUT_FIELDS: &[FieldSpec] = &[
    field(
        "amount",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "scriptPublicKey",
        true,
        NullRule::Forbidden,
        ValueKind::ScriptPublicKey,
        DefaultRule::None,
    ),
    field(
        "redeemScript",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::HexString,
        DefaultRule::EmptyBytes,
    ),
    field(
        "bip32Derivations",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
    field(
        "covenantBinding",
        false,
        NullRule::AllowedAsDefault,
        ValueKind::Object,
        DefaultRule::None,
    ),
    field(
        "proprietaries",
        false,
        NullRule::Forbidden,
        ValueKind::Object,
        DefaultRule::EmptyObject,
    ),
];

pub const COVENANT_EXECUTION_FIELDS: &[FieldSpec] = &[
    field(
        "suppliedMask",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "suppliedTrueMask",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
];

pub const COVENANT_BINDING_FIELDS: &[FieldSpec] = &[
    field(
        "authorizingInput",
        true,
        NullRule::Forbidden,
        ValueKind::ExactUnsigned,
        DefaultRule::None,
    ),
    field(
        "covenantId",
        true,
        NullRule::Forbidden,
        ValueKind::HexString,
        DefaultRule::None,
    ),
];

pub const PARTIAL_SIGNATURE_FIELDS: &[FieldSpec] = &[field(
    "schnorr",
    true,
    NullRule::Forbidden,
    ValueKind::HexString,
    DefaultRule::None,
)];

#[must_use]
pub const fn fields(scope: Scope) -> &'static [FieldSpec] {
    match scope {
        Scope::TopLevel => TOP_LEVEL_FIELDS,
        Scope::Global => GLOBAL_FIELDS,
        Scope::Input => INPUT_FIELDS,
        Scope::InputUtxo => UTXO_FIELDS,
        Scope::InputOutpoint => OUTPOINT_FIELDS,
        Scope::Output => OUTPUT_FIELDS,
        Scope::CovenantExecution => COVENANT_EXECUTION_FIELDS,
        Scope::CovenantBinding => COVENANT_BINDING_FIELDS,
        Scope::PartialSignature => PARTIAL_SIGNATURE_FIELDS,
    }
}

#[must_use]
pub fn field_spec(scope: Scope, name: &[u8]) -> Option<&'static FieldSpec> {
    fields(scope)
        .iter()
        .find(|spec| spec.name.as_bytes() == name)
}

#[must_use]
pub fn field_bit(scope: Scope, name: &[u8]) -> Option<u64> {
    fields(scope)
        .iter()
        .position(|spec| spec.name.as_bytes() == name)
        .and_then(|index| (index < 64).then_some(1u64 << index))
}

#[must_use]
pub fn required_mask(scope: Scope) -> u64 {
    fields(scope)
        .iter()
        .enumerate()
        .filter_map(|(index, spec)| (spec.required && index < 64).then_some(1u64 << index))
        .fold(0u64, |mask, bit| mask | bit)
}

#[must_use]
pub fn required_fields_present(scope: Scope, seen: u64) -> bool {
    let required = required_mask(scope);
    seen & required == required
}

#[must_use]
pub fn null_rule(scope: Scope, name: &[u8]) -> NullRule {
    field_spec(scope, name).map_or(NullRule::Forbidden, |spec| spec.null_rule)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionGrammar {
    pub preserve_unknown_fields: bool,
    pub reject_duplicate_keys_at_every_depth: bool,
    pub printable_ascii_unescaped_strings_only: bool,
    pub arbitrary_canonical_json_values: bool,
}

pub const EXTENSION_GRAMMAR: ExtensionGrammar = ExtensionGrammar {
    preserve_unknown_fields: true,
    reject_duplicate_keys_at_every_depth: true,
    printable_ascii_unescaped_strings_only: true,
    arbitrary_canonical_json_values: true,
};

/// Legacy Companion fields that used to select or parameterize covenant witness
/// branches independently of `covenantExecution`. Consumer finalization rejects
/// these fields until each family has a typed branch-aware witness plan. Keeping
/// this list in the shared schema prevents host/protocol routing drift.
pub const SPECIALIZED_COVENANT_ROUTING_FIELDS: &[&str] = &[
    "escrowBranch",
    "shipBranch",
    "privateSwapClaim",
    "oracleV1Claim",
    "oracleV1Signature",
    "risc0OracleMb",
    "oracleMbPassthrough",
    "oracleMbHeartbeat",
    "oracleMbConsumer",
    "zkProof",
    "zkPublicInputs",
    "zkVk",
    "risc0Seal",
    "risc0Fields",
    "risc0Bridge",
    "groth16Bridge",
    "commitPartA",
    "commitPartB",
    "commitPreimage",
    "merkleProof",
    "merkleDestSpk",
    "withdrawalSpk",
    "rollupStateAdvance",
    "rollupStateRefund",
    "rollupProof",
    "rollupPrefix",
    "rollupSuffix",
    "rollupDepositAdvance",
    "rollupUnifiedAdvance",
    "rollupForcedExit",
    "depositHoldingCredit",
    "depositHoldingRefund",
];

/// Specialized witness-routing metadata that currently has a typed, template-
/// bound consumer witness plan. Every other legacy routing field is rejected
/// at the verified boundary rather than interpreted by an unverified finalizer.
pub const SUPPORTED_SPECIALIZED_COVENANT_ROUTING_FIELDS: &[&str] = &[
    "privateSwapClaim",
    "oracleV1Claim",
    "oracleV1Signature",
    "commitPartA",
    "commitPartB",
    "merkleProof",
    "merkleDestSpk",
];

#[must_use]
pub fn is_supported_specialized_covenant_routing_field(name: &str) -> bool {
    SUPPORTED_SPECIALIZED_COVENANT_ROUTING_FIELDS.contains(&name)
}

#[must_use]
pub fn is_specialized_covenant_routing_field(name: &str) -> bool {
    SPECIALIZED_COVENANT_ROUTING_FIELDS.contains(&name)
}

#[must_use]
pub const fn supported_tx_version(version: u16) -> bool {
    version <= MAX_SUPPORTED_TX_VERSION
}

#[must_use]
pub const fn supported_tx_version_u64(version: u64) -> bool {
    version <= MAX_SUPPORTED_TX_VERSION as u64
}

#[must_use]
pub fn default_rule(scope: Scope, name: &[u8]) -> DefaultRule {
    field_spec(scope, name).map_or(DefaultRule::None, |spec| spec.default)
}

#[must_use]
pub fn value_kind(scope: Scope, name: &[u8]) -> Option<ValueKind> {
    field_spec(scope, name).map(|spec| spec.kind)
}

/// Parse a canonical decimal u64 string. This byte-level helper is shared by
/// serde-based host code and allocation-free Vault code.
pub fn parse_canonical_u64_bytes(bytes: &[u8]) -> Result<u64, JsonNumberError> {
    if bytes.is_empty()
        || !bytes.iter().all(u8::is_ascii_digit)
        || (bytes.len() > 1 && bytes[0] == b'0')
    {
        return Err(JsonNumberError::NonCanonical);
    }
    let mut value = 0u64;
    for &byte in bytes {
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or(JsonNumberError::Overflow)?;
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonNumberError {
    NonCanonical,
    Overflow,
    LegacyUnsafeInteger,
}

#[must_use]
pub const fn legacy_json_integer_is_exact(value: u64) -> bool {
    value <= JS_MAX_SAFE_U64
}

/// Restricted JSON syntax error shared by host and hardware PSKT decoders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonSyntaxError {
    UnexpectedToken,
    InvalidNumber,
    InvalidString,
    DuplicateKey,
    NestingTooDeep,
    TrailingData,
}

impl fmt::Display for JsonSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnexpectedToken => "invalid canonical JSON structure",
            Self::InvalidNumber => "PSKT JSON numbers must be canonical non-negative integers",
            Self::InvalidString => "PSKT JSON strings must be printable ASCII without escapes",
            Self::DuplicateKey => "duplicate JSON object key",
            Self::NestingTooDeep => "PSKT JSON exceeds maximum nesting depth",
            Self::TrailingData => "PSKT JSON contains trailing data",
        })
    }
}

struct Cursor<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn ws(&mut self) {
        while matches!(self.input.get(self.pos), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }

    fn byte(&mut self, expected: u8) -> Result<(), JsonSyntaxError> {
        self.ws();
        if self.input.get(self.pos).copied() != Some(expected) {
            return Err(JsonSyntaxError::UnexpectedToken);
        }
        self.pos += 1;
        Ok(())
    }

    fn string(&mut self) -> Result<(usize, usize), JsonSyntaxError> {
        self.ws();
        if self.input.get(self.pos) != Some(&b'"') {
            return Err(JsonSyntaxError::UnexpectedToken);
        }
        self.pos += 1;
        let start = self.pos;
        while let Some(&byte) = self.input.get(self.pos) {
            match byte {
                b'"' => {
                    let end = self.pos;
                    self.pos += 1;
                    return Ok((start, end));
                }
                b'\\' | 0x00..=0x1f | 0x7f..=0xff => return Err(JsonSyntaxError::InvalidString),
                _ => self.pos += 1,
            }
        }
        Err(JsonSyntaxError::InvalidString)
    }

    fn number(&mut self) -> Result<(), JsonSyntaxError> {
        self.ws();
        let start = self.pos;
        match self.input.get(self.pos).copied() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.input.get(self.pos), Some(b'0'..=b'9')) {
                    return Err(JsonSyntaxError::InvalidNumber);
                }
            }
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(self.input.get(self.pos), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(JsonSyntaxError::InvalidNumber),
        }
        if self.pos == start
            || matches!(
                self.input.get(self.pos),
                Some(b'.' | b'e' | b'E' | b'+' | b'-')
            )
        {
            return Err(JsonSyntaxError::InvalidNumber);
        }
        Ok(())
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), JsonSyntaxError> {
        self.ws();
        if self
            .input
            .get(self.pos..self.pos.saturating_add(literal.len()))
            != Some(literal)
        {
            return Err(JsonSyntaxError::UnexpectedToken);
        }
        self.pos += literal.len();
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<(), JsonSyntaxError> {
        if depth > MAX_JSON_NESTING {
            return Err(JsonSyntaxError::NestingTooDeep);
        }
        self.ws();
        match self.input.get(self.pos).copied() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(|_| ()),
            Some(b'0'..=b'9') => self.number(),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            _ => Err(JsonSyntaxError::UnexpectedToken),
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), JsonSyntaxError> {
        if depth > MAX_JSON_NESTING {
            return Err(JsonSyntaxError::NestingTooDeep);
        }
        self.byte(b'{')?;
        self.ws();
        if self.consume_if(b'}') {
            return Ok(());
        }
        let members_start = self.pos;
        loop {
            self.object_member(depth, members_start)?;
            if self.object_member_terminator()? {
                return Ok(());
            }
        }
    }

    fn object_member(&mut self, depth: usize, members_start: usize) -> Result<(), JsonSyntaxError> {
        self.ws();
        let key_token_start = self.pos;
        let (key_start, key_end) = self.string()?;
        if key_seen_before(
            self.input,
            members_start,
            key_token_start,
            &self.input[key_start..key_end],
        )? {
            return Err(JsonSyntaxError::DuplicateKey);
        }
        self.byte(b':')?;
        self.value(depth)
    }

    fn object_member_terminator(&mut self) -> Result<bool, JsonSyntaxError> {
        self.ws();
        match self.input.get(self.pos).copied() {
            Some(b',') => {
                self.pos += 1;
                Ok(false)
            }
            Some(b'}') => {
                self.pos += 1;
                Ok(true)
            }
            _ => Err(JsonSyntaxError::UnexpectedToken),
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.input.get(self.pos) != Some(&expected) {
            return false;
        }
        self.pos += 1;
        true
    }

    fn array(&mut self, depth: usize) -> Result<(), JsonSyntaxError> {
        if depth > MAX_JSON_NESTING {
            return Err(JsonSyntaxError::NestingTooDeep);
        }
        self.byte(b'[')?;
        self.ws();
        if self.input.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(());
        }
        loop {
            self.value(depth)?;
            self.ws();
            match self.input.get(self.pos).copied() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => return Err(JsonSyntaxError::UnexpectedToken),
            }
        }
    }
}

/// Parse one value without duplicate-key checking. Used only to skip already
/// validated earlier values while comparing a new object key with its peers.
fn skip_value(input: &[u8], pos: &mut usize, depth: usize) -> Result<(), JsonSyntaxError> {
    ensure_skip_depth(depth)?;
    let mut cursor = Cursor { input, pos: *pos };
    cursor.ws();
    skip_value_at(&mut cursor, depth)?;
    *pos = cursor.pos;
    Ok(())
}

fn ensure_skip_depth(depth: usize) -> Result<(), JsonSyntaxError> {
    if depth > MAX_JSON_NESTING {
        return Err(JsonSyntaxError::NestingTooDeep);
    }
    Ok(())
}

fn skip_value_at(cursor: &mut Cursor<'_>, depth: usize) -> Result<(), JsonSyntaxError> {
    match cursor.input.get(cursor.pos).copied() {
        Some(b'{') => skip_object(cursor.input, &mut cursor.pos, depth + 1),
        Some(b'[') => skip_array(cursor.input, &mut cursor.pos, depth + 1),
        Some(b'"') => cursor.string().map(|_| ()),
        Some(b'0'..=b'9') => cursor.number(),
        Some(b't') => cursor.literal(b"true"),
        Some(b'f') => cursor.literal(b"false"),
        Some(b'n') => cursor.literal(b"null"),
        _ => Err(JsonSyntaxError::UnexpectedToken),
    }
}

fn skip_object(input: &[u8], pos: &mut usize, depth: usize) -> Result<(), JsonSyntaxError> {
    ensure_skip_depth(depth)?;
    let mut cursor = Cursor { input, pos: *pos };
    cursor.byte(b'{')?;
    cursor.ws();
    if finish_empty_object(&mut cursor, pos) {
        return Ok(());
    }
    loop {
        skip_object_member(input, &mut cursor, depth)?;
        if finish_object_member(&mut cursor, pos)? {
            return Ok(());
        }
    }
}

fn finish_empty_object(cursor: &mut Cursor<'_>, pos: &mut usize) -> bool {
    if cursor.input.get(cursor.pos) != Some(&b'}') {
        return false;
    }
    cursor.pos += 1;
    *pos = cursor.pos;
    true
}

fn skip_object_member(
    input: &[u8],
    cursor: &mut Cursor<'_>,
    depth: usize,
) -> Result<(), JsonSyntaxError> {
    cursor.string()?;
    cursor.byte(b':')?;
    let mut next = cursor.pos;
    skip_value(input, &mut next, depth)?;
    cursor.pos = next;
    cursor.ws();
    Ok(())
}

fn finish_object_member(cursor: &mut Cursor<'_>, pos: &mut usize) -> Result<bool, JsonSyntaxError> {
    match cursor.input.get(cursor.pos).copied() {
        Some(b',') => {
            cursor.pos += 1;
            Ok(false)
        }
        Some(b'}') => {
            cursor.pos += 1;
            *pos = cursor.pos;
            Ok(true)
        }
        _ => Err(JsonSyntaxError::UnexpectedToken),
    }
}

fn skip_array(input: &[u8], pos: &mut usize, depth: usize) -> Result<(), JsonSyntaxError> {
    if depth > MAX_JSON_NESTING {
        return Err(JsonSyntaxError::NestingTooDeep);
    }
    let mut c = Cursor { input, pos: *pos };
    c.byte(b'[')?;
    c.ws();
    if c.input.get(c.pos) == Some(&b']') {
        c.pos += 1;
        *pos = c.pos;
        return Ok(());
    }
    loop {
        let mut next = c.pos;
        skip_value(input, &mut next, depth)?;
        c.pos = next;
        c.ws();
        match c.input.get(c.pos).copied() {
            Some(b',') => c.pos += 1,
            Some(b']') => {
                c.pos += 1;
                *pos = c.pos;
                return Ok(());
            }
            _ => return Err(JsonSyntaxError::UnexpectedToken),
        }
    }
}

fn key_seen_before(
    input: &[u8],
    members_start: usize,
    current_key_token_start: usize,
    key: &[u8],
) -> Result<bool, JsonSyntaxError> {
    let mut c = Cursor {
        input,
        pos: members_start,
    };
    while c.pos < current_key_token_start {
        c.ws();
        if c.pos >= current_key_token_start {
            break;
        }
        let (start, end) = c.string()?;
        if input.get(start..end) == Some(key) {
            return Ok(true);
        }
        c.byte(b':')?;
        let mut next = c.pos;
        skip_value(input, &mut next, 1)?;
        c.pos = next;
        c.ws();
        if c.pos >= current_key_token_start {
            break;
        }
        if c.input.get(c.pos) != Some(&b',') {
            return Err(JsonSyntaxError::UnexpectedToken);
        }
        c.pos += 1;
    }
    Ok(false)
}

/// Validate the exact restricted JSON grammar used by both PSKT decoders,
/// including recursive duplicate-key rejection.
pub fn validate_canonical_json(input: &[u8]) -> Result<(), JsonSyntaxError> {
    let mut cursor = Cursor::new(input);
    cursor.value(0)?;
    cursor.ws();
    if cursor.pos != input.len() {
        return Err(JsonSyntaxError::TrailingData);
    }
    Ok(())
}
