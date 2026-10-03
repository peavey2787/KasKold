//! Canonical PSKT field tables for every schema scope.

use super::{DefaultRule, FieldClass, FieldSpec, NullRule, ValueKind};

pub(crate) const fn field(
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

pub(crate) const fn product(
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
