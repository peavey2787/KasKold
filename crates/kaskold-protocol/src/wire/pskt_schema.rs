//! Executable canonical PSKT schema shared by Companion and Vault.
//!
//! This module is deliberately `no_std`: both the host parser and the hardware
//! parser consume the same field tables, required-field masks, nullability
//! rules, transaction/sighash versions, extension policy, and restricted JSON
//! syntax validator.  Environment-specific parsers are only responsible for
//! materializing values into their own semantic models.

use core::fmt;
mod fields;
mod json;
pub use fields::*;
pub use json::*;

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
