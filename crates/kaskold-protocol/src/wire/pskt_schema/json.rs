//! Canonical JSON syntax validation for PSKT bodies.

use super::{fmt, MAX_JSON_NESTING};

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

pub(crate) struct Cursor<'a> {
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
            next => match next.and_then(keyword_literal) {
                Some(keyword) => self.literal(keyword),
                None => Err(JsonSyntaxError::UnexpectedToken),
            },
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
pub(crate) fn skip_value(
    input: &[u8],
    pos: &mut usize,
    depth: usize,
) -> Result<(), JsonSyntaxError> {
    ensure_skip_depth(depth)?;
    let mut cursor = Cursor { input, pos: *pos };
    cursor.ws();
    skip_value_at(&mut cursor, depth)?;
    *pos = cursor.pos;
    Ok(())
}

pub(crate) fn ensure_skip_depth(depth: usize) -> Result<(), JsonSyntaxError> {
    if depth > MAX_JSON_NESTING {
        return Err(JsonSyntaxError::NestingTooDeep);
    }
    Ok(())
}

pub(crate) fn skip_value_at(cursor: &mut Cursor<'_>, depth: usize) -> Result<(), JsonSyntaxError> {
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

pub(crate) fn skip_object(
    input: &[u8],
    pos: &mut usize,
    depth: usize,
) -> Result<(), JsonSyntaxError> {
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

pub(crate) fn finish_empty_object(cursor: &mut Cursor<'_>, pos: &mut usize) -> bool {
    if cursor.input.get(cursor.pos) != Some(&b'}') {
        return false;
    }
    cursor.pos += 1;
    *pos = cursor.pos;
    true
}

pub(crate) fn skip_object_member(
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

pub(crate) fn finish_object_member(
    cursor: &mut Cursor<'_>,
    pos: &mut usize,
) -> Result<bool, JsonSyntaxError> {
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

pub(crate) fn skip_array(
    input: &[u8],
    pos: &mut usize,
    depth: usize,
) -> Result<(), JsonSyntaxError> {
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

pub(crate) fn key_seen_before(
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

/// The JSON keyword a value starting with `first` must spell.
fn keyword_literal(first: u8) -> Option<&'static [u8]> {
    match first {
        b't' => Some(b"true"),
        b'f' => Some(b"false"),
        b'n' => Some(b"null"),
        _ => None,
    }
}
