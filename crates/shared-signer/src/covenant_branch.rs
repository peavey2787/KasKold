//! Fail-closed covenant branch/key resolver.
//!
//! This module parses script structure rather than searching raw bytes. A key is
//! resolvable only when it is a canonical PUSH32 immediately consumed by
//! CHECKSIG/CHECKSIGVERIFY. Each key is bound to the exact nested IF/ELSE path
//! that contains that signature check. Malformed control flow, malformed pushes,
//! excessive nesting, or ambiguous key-only lookups are rejected. Reusing the
//! same key on multiple explicit branches is valid and is represented as separate
//! occurrences; callers that need a unique occurrence must supply branch selectors.
//! Regression coverage lives in `unit_tests/covenant_branch_tests.rs`, including
//! `duplicate_key_binding_on_the_same_branch_is_rejected` and
//! `reused_key_requires_branch_disambiguation_and_malformed_flow_is_rejected`.

pub const MAX_COVENANT_BRANCH_DEPTH: usize = 16;
pub const MAX_COVENANT_BRANCH_KEYS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchResolveError {
    MalformedScript,
    BranchDepthExceeded,
    TooManyKeys,
    AmbiguousKey,
    KeyNotFound,
    PositionOutOfRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchKey {
    pub key: [u8; 32],
    /// Bits set for structural IF/NOTIF decisions active in this path.
    pub decision_mask: u16,
    /// For bits present in `decision_mask`, the truth value the spender must
    /// supply to reach this key. OP_NOTIF is accounted for, so this is a
    /// witness-selector truth mask rather than a raw IF/ELSE label.
    pub if_mask: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchResolution {
    keys: [BranchKey; MAX_COVENANT_BRANCH_KEYS],
    len: usize,
    selector_mask: u16,
}

impl BranchKey {
    /// Prove that an explicit witness-selector assignment reaches this key.
    #[must_use]
    pub const fn matches_selectors(self, supplied_mask: u16, supplied_true_mask: u16) -> bool {
        (supplied_mask & self.decision_mask) == self.decision_mask
            && ((supplied_true_mask ^ self.if_mask) & self.decision_mask) == 0
    }
}

impl BranchResolution {
    const EMPTY: BranchKey = BranchKey {
        key: [0; 32],
        decision_mask: 0,
        if_mask: 0,
    };

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Every structural IF/NOTIF selector bit in this covenant. A complete
    /// execution proof must assign every bit so downstream branches cannot be
    /// changed after the signer authorizes the transaction.
    #[must_use]
    pub const fn selector_mask(&self) -> u16 {
        self.selector_mask
    }

    pub fn key_at(&self, position: u8) -> Result<BranchKey, BranchResolveError> {
        self.keys
            .get(usize::from(position))
            .copied()
            .filter(|_| usize::from(position) < self.len)
            .ok_or(BranchResolveError::PositionOutOfRange)
    }

    pub fn unique_position_for_key(&self, key: &[u8; 32]) -> Result<u8, BranchResolveError> {
        let mut found = None;
        for position in 0..self.len {
            if &self.keys[position].key != key {
                continue;
            }
            if found.is_some() {
                return Err(BranchResolveError::AmbiguousKey);
            }
            found = Some(u8::try_from(position).map_err(|_| BranchResolveError::TooManyKeys)?);
        }
        found.ok_or(BranchResolveError::KeyNotFound)
    }

    pub fn position_for_key_with_selectors(
        &self,
        key: &[u8; 32],
        supplied_mask: u16,
        supplied_true_mask: u16,
    ) -> Result<u8, BranchResolveError> {
        let mut found = None;
        for position in 0..self.len {
            let binding = self.keys[position];
            if &binding.key != key || !binding.matches_selectors(supplied_mask, supplied_true_mask)
            {
                continue;
            }
            if found.is_some() {
                return Err(BranchResolveError::AmbiguousKey);
            }
            found = Some(u8::try_from(position).map_err(|_| BranchResolveError::TooManyKeys)?);
        }
        found.ok_or(BranchResolveError::KeyNotFound)
    }

    fn push(&mut self, item: BranchKey) -> Result<(), BranchResolveError> {
        if self.keys[..self.len].contains(&item) {
            // Two identical key+branch bindings cannot be represented
            // unambiguously by PSKT's pubkey-keyed partialSigs map.
            return Err(BranchResolveError::AmbiguousKey);
        }
        if self.len >= MAX_COVENANT_BRANCH_KEYS {
            return Err(BranchResolveError::TooManyKeys);
        }
        self.keys[self.len] = item;
        self.len += 1;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Frame {
    bit: u16,
    in_else: bool,
}

struct BranchParser {
    out: BranchResolution,
    frames: [Frame; MAX_COVENANT_BRANCH_DEPTH],
    depth: usize,
    next_branch: u32,
    decision_mask: u16,
    if_mask: u16,
    offset: usize,
}

impl BranchParser {
    fn new() -> Self {
        Self {
            out: BranchResolution {
                keys: [BranchResolution::EMPTY; MAX_COVENANT_BRANCH_KEYS],
                len: 0,
                selector_mask: 0,
            },
            frames: [Frame {
                bit: 0,
                in_else: false,
            }; MAX_COVENANT_BRANCH_DEPTH],
            depth: 0,
            next_branch: 0,
            decision_mask: 0,
            if_mask: 0,
            offset: 0,
        }
    }

    fn enter_branch(&mut self, opcode: u8) -> Result<(), BranchResolveError> {
        if self.depth >= MAX_COVENANT_BRANCH_DEPTH || self.next_branch >= 16 {
            return Err(BranchResolveError::BranchDepthExceeded);
        }
        let bit = 1u16
            .checked_shl(self.next_branch)
            .ok_or(BranchResolveError::BranchDepthExceeded)?;
        self.next_branch += 1;
        self.out.selector_mask |= bit;
        self.frames[self.depth] = Frame {
            bit,
            in_else: false,
        };
        self.depth += 1;
        self.decision_mask |= bit;
        if opcode == 0x63 {
            self.if_mask |= bit;
        } else {
            self.if_mask &= !bit;
        }
        self.offset += 1;
        Ok(())
    }

    fn enter_else(&mut self) -> Result<(), BranchResolveError> {
        if self.depth == 0 {
            return Err(BranchResolveError::MalformedScript);
        }
        let frame = &mut self.frames[self.depth - 1];
        if frame.in_else {
            return Err(BranchResolveError::MalformedScript);
        }
        frame.in_else = true;
        self.if_mask ^= frame.bit;
        self.offset += 1;
        Ok(())
    }

    fn leave_branch(&mut self) -> Result<(), BranchResolveError> {
        if self.depth == 0 {
            return Err(BranchResolveError::MalformedScript);
        }
        self.depth -= 1;
        let frame = self.frames[self.depth];
        self.decision_mask &= !frame.bit;
        self.if_mask &= !frame.bit;
        self.offset += 1;
        Ok(())
    }

    fn inspect_key_push(&mut self, script: &[u8]) -> Result<(), BranchResolveError> {
        let end = self
            .offset
            .checked_add(33)
            .ok_or(BranchResolveError::MalformedScript)?;
        if end > script.len() {
            return Err(BranchResolveError::MalformedScript);
        }
        if matches!(script.get(end), Some(0xac | 0xad)) {
            let mut key = [0u8; 32];
            key.copy_from_slice(&script[self.offset + 1..end]);
            self.out.push(BranchKey {
                key,
                decision_mask: self.decision_mask,
                if_mask: self.if_mask,
            })?;
        }
        self.offset = end;
        Ok(())
    }

    fn advance_pushdata(&mut self, script: &[u8], opcode: u8) -> Result<(), BranchResolveError> {
        let (header_len, payload_len) = pushdata_lengths(script, self.offset, opcode)?;
        self.offset = advance_push(script, self.offset, header_len, payload_len)?;
        Ok(())
    }

    fn step(&mut self, script: &[u8]) -> Result<(), BranchResolveError> {
        let opcode = script[self.offset];
        match opcode {
            0x63 | 0x64 => self.enter_branch(opcode),
            0x67 => self.enter_else(),
            0x68 => self.leave_branch(),
            0x20 => self.inspect_key_push(script),
            0x01..=0x4e => self.advance_pushdata(script, opcode),
            _ => {
                self.offset += 1;
                Ok(())
            }
        }
    }

    fn finish(self) -> Result<BranchResolution, BranchResolveError> {
        if self.depth != 0 {
            return Err(BranchResolveError::MalformedScript);
        }
        Ok(self.out)
    }
}

/// Resolve canonical covenant signature keys and their structural branch paths.
pub fn resolve_covenant_branches(script: &[u8]) -> Result<BranchResolution, BranchResolveError> {
    let mut parser = BranchParser::new();
    while parser.offset < script.len() {
        parser.step(script)?;
    }
    parser.finish()
}

fn pushdata_lengths(
    script: &[u8],
    offset: usize,
    opcode: u8,
) -> Result<(usize, usize), BranchResolveError> {
    match opcode {
        0x01..=0x4b => Ok((1, usize::from(opcode))),
        0x4c => Ok((
            2,
            usize::from(
                *script
                    .get(offset + 1)
                    .ok_or(BranchResolveError::MalformedScript)?,
            ),
        )),
        0x4d => {
            let bytes = script
                .get(offset + 1..offset + 3)
                .ok_or(BranchResolveError::MalformedScript)?;
            Ok((3, usize::from(u16::from_le_bytes([bytes[0], bytes[1]]))))
        }
        0x4e => {
            let bytes = script
                .get(offset + 1..offset + 5)
                .ok_or(BranchResolveError::MalformedScript)?;
            let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            usize::try_from(len)
                .map(|length| (5, length))
                .map_err(|_| BranchResolveError::MalformedScript)
        }
        _ => Err(BranchResolveError::MalformedScript),
    }
}

fn advance_push(
    script: &[u8],
    offset: usize,
    header: usize,
    payload: usize,
) -> Result<usize, BranchResolveError> {
    let next = offset
        .checked_add(header)
        .and_then(|v| v.checked_add(payload))
        .ok_or(BranchResolveError::MalformedScript)?;
    if next > script.len() || next <= offset {
        return Err(BranchResolveError::MalformedScript);
    }
    Ok(next)
}
