//! Typed, template-bound witness plans for the specialized covenant families
//! that are currently exposed by Companion transaction builders.
//!
//! Proprietary metadata is never allowed to select a branch on its own. A route
//! is accepted only when (1) the redeem script exactly matches the recognized
//! template, (2) `covenantExecution` supplies every structural selector, (3)
//! those selectors choose the route's branch, (4) the already-verified
//! transaction signature is bound to that branch, and (5) route-specific proof
//! material validates against commitments embedded in the redeem script.

use blake2b_simd::Params;
use k256::schnorr::{Signature as K256Signature, VerifyingKey};
use serde_json::{Map, Value};

use super::compact::{Input, Signature, SpecializedRoute, SpecializedWitness};
mod claims;
mod merkle_proof;
mod routes;
mod templates;
pub(crate) use claims::*;
pub(crate) use merkle_proof::*;
pub(crate) use routes::*;
pub(crate) use templates::*;

const OP_0: u8 = 0x00;
const OP_1: u8 = 0x51;
const OP_IF: u8 = 0x63;
const OP_ELSE: u8 = 0x67;
const OP_ENDIF: u8 = 0x68;
const OP_VERIFY: u8 = 0x69;
const OP_DROP: u8 = 0x75;
const OP_DUP: u8 = 0x76;
const OP_SWAP: u8 = 0x7c;
const OP_CAT: u8 = 0x7e;
const OP_EQUALVERIFY: u8 = 0x88;
const OP_SUB: u8 = 0x94;
const OP_NUMEQUALVERIFY: u8 = 0x9d;
const OP_LESSTHANOREQUAL: u8 = 0xa1;
const OP_GREATERTHANOREQUAL: u8 = 0xa2;
const OP_BLAKE2B: u8 = 0xaa;
const OP_CHECKSIGVERIFY: u8 = 0xad;
const OP_CHECKLOCKTIMEVERIFY: u8 = 0xb0;
const OP_TX_INPUT_COUNT: u8 = 0xb3;
const OP_TX_OUTPUT_COUNT: u8 = 0xb4;
const OP_TX_INPUT_AMOUNT: u8 = 0xbe;
const OP_TX_OUTPUT_AMOUNT: u8 = 0xc2;
const OP_TX_OUTPUT_SPK: u8 = 0xc3;
const OP_CHECKSIGFROMSTACK: u8 = 0xd7;
const PRIVATE_SWAP_MAX_FEE_SOMPI: u64 = 500_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RouteKind {
    PrivateSwap,
    OracleV1,
    CommitReveal,
    Merkle,
}
