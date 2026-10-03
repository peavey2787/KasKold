//! Recognizers for the specialized covenant script templates.

use super::{
    OP_0, OP_1, OP_BLAKE2B, OP_CAT, OP_CHECKLOCKTIMEVERIFY, OP_CHECKSIGFROMSTACK,
    OP_CHECKSIGVERIFY, OP_DROP, OP_DUP, OP_ELSE, OP_ENDIF, OP_EQUALVERIFY, OP_GREATERTHANOREQUAL,
    OP_IF, OP_LESSTHANOREQUAL, OP_NUMEQUALVERIFY, OP_SUB, OP_SWAP, OP_TX_INPUT_AMOUNT,
    OP_TX_INPUT_COUNT, OP_TX_OUTPUT_AMOUNT, OP_TX_OUTPUT_COUNT, OP_TX_OUTPUT_SPK, OP_VERIFY,
    PRIVATE_SWAP_MAX_FEE_SOMPI,
};

pub(crate) struct OracleTemplate {
    pub(crate) commitment: [u8; 32],
    pub(crate) oracle_key: [u8; 32],
}

pub(crate) struct CommitTemplate {
    pub(crate) commitment: [u8; 32],
}

pub(crate) struct MerkleTemplate {
    pub(crate) root: [u8; 32],
    pub(crate) depth: u8,
}

pub(crate) struct Cursor<'a> {
    pub(crate) script: &'a [u8],
    pub(crate) at: usize,
}

impl<'a> Cursor<'a> {
    fn new(script: &'a [u8]) -> Self {
        Self { script, at: 0 }
    }

    fn op(&mut self, expected: u8) -> Result<(), String> {
        if self.script.get(self.at) != Some(&expected) {
            return Err(format!("expected opcode 0x{expected:02x} at {}", self.at));
        }
        self.at += 1;
        Ok(())
    }

    fn push(&mut self, length: usize) -> Result<&'a [u8], String> {
        if length > 75 || self.script.get(self.at).copied() != u8::try_from(length).ok() {
            return Err(format!("expected canonical PUSH{length} at {}", self.at));
        }
        let start = self.at + 1;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "script position overflow".to_string())?;
        let data = self
            .script
            .get(start..end)
            .ok_or_else(|| "truncated script push".to_string())?;
        self.at = end;
        Ok(data)
    }

    fn integer(&mut self) -> Result<u64, String> {
        let opcode = *self
            .script
            .get(self.at)
            .ok_or_else(|| "missing script integer".to_string())?;
        if let Some(value) = small_script_integer(opcode) {
            self.at += 1;
            return Ok(value);
        }
        let data = self.integer_push(opcode)?;
        decode_script_integer(data)
    }

    fn integer_push(&mut self, opcode: u8) -> Result<&'a [u8], String> {
        let length = usize::from(opcode);
        if length == 0 || length > 9 || length > 75 {
            return Err("non-canonical script integer".to_string());
        }
        self.push(length)
    }

    fn finish(self) -> Result<(), String> {
        if self.at == self.script.len() {
            Ok(())
        } else {
            Err(format!("unexpected trailing script bytes at {}", self.at))
        }
    }
}

pub(crate) fn small_script_integer(opcode: u8) -> Option<u64> {
    if opcode == OP_0 {
        return Some(0);
    }
    (OP_1..=0x60)
        .contains(&opcode)
        .then(|| u64::from(opcode - 0x50))
}

pub(crate) fn decode_script_integer(data: &[u8]) -> Result<u64, String> {
    if data.last() == Some(&0) && data.len() > 1 && data[data.len() - 2] & 0x80 == 0 {
        return Err("non-minimal script integer".to_string());
    }
    if data.last().is_some_and(|byte| byte & 0x80 != 0) {
        return Err("negative script integer not permitted".to_string());
    }
    if data.len() == 9 && data[8] != 0 {
        return Err("script integer exceeds u64".to_string());
    }
    let mut bytes = [0u8; 8];
    let count = data.len().min(8);
    bytes[..count].copy_from_slice(&data[..count]);
    Ok(u64::from_le_bytes(bytes))
}

pub(crate) fn recognize_private_swap(script: &[u8]) -> Result<(), String> {
    let mut c = Cursor::new(script);
    let claimer = private_swap_prefix(&mut c)?;
    private_swap_claim_constraints(&mut c)?;
    private_swap_fee_constraints(&mut c)?;
    c.op(OP_1)?;
    c.op(OP_ELSE)?;
    private_swap_refund_branch(&mut c, &claimer)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    c.finish()
}

pub(crate) fn private_swap_prefix(c: &mut Cursor<'_>) -> Result<Vec<u8>, String> {
    let salt = c.push(16)?;
    if salt.iter().all(|byte| *byte == 0) {
        return Err("zero salt".to_string());
    }
    c.op(OP_DROP)?;
    c.op(OP_IF)?;
    let claimer = c.push(32)?.to_vec();
    c.op(OP_CHECKSIGVERIFY)?;
    Ok(claimer)
}

pub(crate) fn private_swap_claim_constraints(c: &mut Cursor<'_>) -> Result<(), String> {
    private_swap_shape_constraints(c)?;
    private_swap_destination_constraint(c)
}

pub(crate) fn private_swap_shape_constraints(c: &mut Cursor<'_>) -> Result<(), String> {
    c.op(OP_TX_INPUT_COUNT)?;
    require_script_integer(c, 1, "input-count constraint changed")?;
    c.op(OP_NUMEQUALVERIFY)?;
    c.op(OP_TX_OUTPUT_COUNT)?;
    require_script_integer(c, 1, "output-count constraint changed")?;
    c.op(OP_NUMEQUALVERIFY)
}

pub(crate) fn private_swap_destination_constraint(c: &mut Cursor<'_>) -> Result<(), String> {
    require_script_integer(c, 0, "destination output index changed")?;
    c.op(OP_TX_OUTPUT_SPK)?;
    let destination = private_swap_destination(c)?;
    if destination.get(0..2) != Some(&[0, 0]) {
        return Err("destination SPK version changed".to_string());
    }
    c.op(OP_EQUALVERIFY)
}

pub(crate) fn private_swap_destination(c: &mut Cursor<'_>) -> Result<Vec<u8>, String> {
    let opcode = *c
        .script
        .get(c.at)
        .ok_or_else(|| "missing private-swap destination".to_string())?;
    if !(5..=75).contains(&usize::from(opcode)) {
        return Err("destination SPK length changed".to_string());
    }
    Ok(c.push(usize::from(opcode))?.to_vec())
}

pub(crate) fn private_swap_fee_constraints(c: &mut Cursor<'_>) -> Result<(), String> {
    private_swap_nonnegative_output_constraint(c)?;
    private_swap_fee_ceiling_constraint(c)
}

pub(crate) fn private_swap_nonnegative_output_constraint(c: &mut Cursor<'_>) -> Result<(), String> {
    require_script_integer(c, 0, "input amount index changed")?;
    c.op(OP_TX_INPUT_AMOUNT)?;
    c.op(OP_DUP)?;
    require_script_integer(c, 0, "output amount index changed")?;
    c.op(OP_TX_OUTPUT_AMOUNT)?;
    c.op(OP_GREATERTHANOREQUAL)?;
    c.op(OP_VERIFY)
}

pub(crate) fn private_swap_fee_ceiling_constraint(c: &mut Cursor<'_>) -> Result<(), String> {
    require_script_integer(c, 0, "fee output index changed")?;
    c.op(OP_TX_OUTPUT_AMOUNT)?;
    c.op(OP_SUB)?;
    require_script_integer(c, PRIVATE_SWAP_MAX_FEE_SOMPI, "fee ceiling changed")?;
    c.op(OP_LESSTHANOREQUAL)?;
    c.op(OP_VERIFY)
}

pub(crate) fn private_swap_refund_branch(c: &mut Cursor<'_>, claimer: &[u8]) -> Result<(), String> {
    let owner = c.push(32)?;
    if owner == claimer {
        return Err("owner and claimer keys are identical".to_string());
    }
    c.op(OP_CHECKSIGVERIFY)?;
    if c.integer()? == 0 {
        return Err("refund locktime is zero".to_string());
    }
    c.op(OP_CHECKLOCKTIMEVERIFY)
}

pub(crate) fn require_script_integer(
    c: &mut Cursor<'_>,
    expected: u64,
    error: &str,
) -> Result<(), String> {
    if c.integer()? != expected {
        return Err(error.to_string());
    }
    Ok(())
}

pub(crate) fn recognize_oracle_v1(script: &[u8]) -> Result<OracleTemplate, String> {
    let mut c = Cursor::new(script);
    recognize_timelock_owner_prefix(&mut c, true)?;
    let template = oracle_claim_branch(&mut c)?;
    c.finish()?;
    Ok(template)
}

pub(crate) fn recognize_timelock_owner_prefix(
    c: &mut Cursor<'_>,
    salted: bool,
) -> Result<Vec<u8>, String> {
    consume_optional_salt(c, salted)?;
    timelock_owner_branch(c)
}

pub(crate) fn consume_optional_salt(c: &mut Cursor<'_>, salted: bool) -> Result<(), String> {
    if salted {
        c.push(16)?;
        c.op(OP_DROP)?;
    }
    Ok(())
}

pub(crate) fn timelock_owner_branch(c: &mut Cursor<'_>) -> Result<Vec<u8>, String> {
    c.op(OP_IF)?;
    let owner = c.push(32)?.to_vec();
    c.op(OP_CHECKSIGVERIFY)?;
    c.integer()?;
    c.op(OP_CHECKLOCKTIMEVERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ELSE)?;
    Ok(owner)
}

pub(crate) fn oracle_claim_branch(c: &mut Cursor<'_>) -> Result<OracleTemplate, String> {
    c.push(32)?;
    c.op(OP_CHECKSIGVERIFY)?;
    let commitment = push32_array(c, "bad oracle commitment")?;
    let oracle_key = push32_array(c, "bad oracle key")?;
    c.op(OP_CHECKSIGFROMSTACK)?;
    c.op(OP_VERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    Ok(OracleTemplate {
        commitment,
        oracle_key,
    })
}

pub(crate) fn push32_array(c: &mut Cursor<'_>, error: &str) -> Result<[u8; 32], String> {
    c.push(32)?.try_into().map_err(|_| error.to_string())
}

pub(crate) fn recognize_commit_reveal(script: &[u8]) -> Result<CommitTemplate, String> {
    let mut c = Cursor::new(script);
    let owner = recognize_timelock_owner_prefix(&mut c, false)?;
    let commitment = recognize_commit_claim(&mut c, &owner)?;
    c.finish()?;
    Ok(CommitTemplate { commitment })
}

pub(crate) fn recognize_commit_claim(c: &mut Cursor<'_>, owner: &[u8]) -> Result<[u8; 32], String> {
    require_same_owner(c, owner)?;
    c.op(OP_CHECKSIGVERIFY)?;
    c.op(OP_CAT)?;
    c.op(OP_BLAKE2B)?;
    let commitment = push32_array(c, "bad commitment")?;
    c.op(OP_EQUALVERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    Ok(commitment)
}

pub(crate) fn recognize_merkle(script: &[u8]) -> Result<MerkleTemplate, String> {
    let mut c = Cursor::new(script);
    let owner = recognize_timelock_owner_prefix(&mut c, false)?;
    require_same_owner(&mut c, &owner)?;
    c.op(OP_CHECKSIGVERIFY)?;
    c.op(OP_BLAKE2B)?;
    let depth = recognize_merkle_layers(&mut c)?;
    let root = recognize_merkle_tail(&mut c)?;
    c.finish()?;
    Ok(MerkleTemplate { root, depth })
}

pub(crate) fn require_same_owner(c: &mut Cursor<'_>, owner: &[u8]) -> Result<(), String> {
    if c.push(32)? != owner {
        return Err("branch owner keys differ".to_string());
    }
    Ok(())
}

pub(crate) fn recognize_merkle_layers(c: &mut Cursor<'_>) -> Result<u8, String> {
    let pattern = [OP_SWAP, OP_IF, OP_SWAP, OP_ENDIF, OP_CAT, OP_BLAKE2B];
    let mut depth = 0u8;
    while c.script.get(c.at..c.at + pattern.len()) == Some(pattern.as_slice()) {
        c.at += pattern.len();
        depth = depth
            .checked_add(1)
            .ok_or_else(|| "merkle depth overflow".to_string())?;
        if depth > 15 {
            return Err("merkle depth exceeds selector capacity".to_string());
        }
    }
    Ok(depth)
}

pub(crate) fn recognize_merkle_tail(c: &mut Cursor<'_>) -> Result<[u8; 32], String> {
    let root = push32_array(c, "bad merkle root")?;
    c.op(OP_EQUALVERIFY)?;
    require_script_integer(c, 0, "merkle output index changed")?;
    c.op(OP_TX_OUTPUT_SPK)?;
    c.op(OP_EQUALVERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    Ok(root)
}
