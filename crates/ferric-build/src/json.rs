use std::collections::BTreeMap;
use std::fmt;

const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Error {
    pub(super) offset: usize,
    pub(super) kind: ErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ErrorKind {
    DuplicateField(String),
    InvalidEscape,
    InvalidNumber,
    InvalidUnicode,
    NestingLimit,
    TrailingData,
    UnexpectedByte,
    UnexpectedEnd,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateField(field) => write!(formatter, "duplicate field {field:?}"),
            Self::InvalidEscape => formatter.write_str("invalid JSON string escape"),
            Self::InvalidNumber => formatter.write_str("invalid JSON number"),
            Self::InvalidUnicode => formatter.write_str("invalid JSON Unicode"),
            Self::NestingLimit => formatter.write_str("JSON nesting limit exceeded"),
            Self::TrailingData => formatter.write_str("trailing data after JSON value"),
            Self::UnexpectedByte => formatter.write_str("unexpected JSON byte"),
            Self::UnexpectedEnd => formatter.write_str("unexpected end of JSON input"),
        }
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Value, Error> {
    let mut parser = Parser { bytes, offset: 0 };
    parser.skip_whitespace();
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.offset != bytes.len() {
        return Err(parser.error(ErrorKind::TrailingData));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Parser<'_> {
    fn error(&self, kind: ErrorKind) -> Error {
        Error {
            offset: self.offset,
            kind,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.offset += 1;
        Some(byte)
    }

    fn skip_whitespace(&mut self) {
        while self
            .peek()
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.offset += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::NestingLimit));
        }
        match self
            .peek()
            .ok_or_else(|| self.error(ErrorKind::UnexpectedEnd))?
        {
            b'n' => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            b't' => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            b'f' => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            b'"' => self.string().map(Value::String),
            b'[' => self.array(depth + 1),
            b'{' => self.object(depth + 1),
            b'-' | b'0'..=b'9' => self.number().map(Value::Number),
            _ => Err(self.error(ErrorKind::UnexpectedByte)),
        }
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), Error> {
        if self.bytes.get(self.offset..self.offset + literal.len()) != Some(literal) {
            return Err(self.error(ErrorKind::UnexpectedByte));
        }
        self.offset += literal.len();
        Ok(())
    }

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
        self.offset += 1;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.offset += 1;
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.value(depth)?);
            self.skip_whitespace();
            match self.take() {
                Some(b',') => self.skip_whitespace(),
                Some(b']') => break,
                Some(_) => return Err(self.error(ErrorKind::UnexpectedByte)),
                None => return Err(self.error(ErrorKind::UnexpectedEnd)),
            }
        }
        Ok(Value::Array(values))
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        self.offset += 1;
        self.skip_whitespace();
        let mut fields = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.offset += 1;
            return Ok(Value::Object(fields));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err(self.error(ErrorKind::UnexpectedByte));
            }
            let key = self.string()?;
            self.skip_whitespace();
            if self.take() != Some(b':') {
                return Err(self.error(ErrorKind::UnexpectedByte));
            }
            self.skip_whitespace();
            let value = self.value(depth)?;
            if fields.insert(key.clone(), value).is_some() {
                return Err(self.error(ErrorKind::DuplicateField(key)));
            }
            self.skip_whitespace();
            match self.take() {
                Some(b',') => self.skip_whitespace(),
                Some(b'}') => break,
                Some(_) => return Err(self.error(ErrorKind::UnexpectedByte)),
                None => return Err(self.error(ErrorKind::UnexpectedEnd)),
            }
        }
        Ok(Value::Object(fields))
    }

    fn string(&mut self) -> Result<String, Error> {
        if self.take() != Some(b'"') {
            return Err(self.error(ErrorKind::UnexpectedByte));
        }
        let mut decoded = Vec::new();
        loop {
            match self.take() {
                Some(b'"') => {
                    return String::from_utf8(decoded)
                        .map_err(|_| self.error(ErrorKind::InvalidUnicode));
                }
                Some(b'\\') => self.escape(&mut decoded)?,
                Some(0x00..=0x1f) => return Err(self.error(ErrorKind::InvalidUnicode)),
                Some(byte) => decoded.push(byte),
                None => return Err(self.error(ErrorKind::UnexpectedEnd)),
            }
        }
    }

    fn escape(&mut self, decoded: &mut Vec<u8>) -> Result<(), Error> {
        match self.take() {
            Some(b'"') => decoded.push(b'"'),
            Some(b'\\') => decoded.push(b'\\'),
            Some(b'/') => decoded.push(b'/'),
            Some(b'b') => decoded.push(0x08),
            Some(b'f') => decoded.push(0x0c),
            Some(b'n') => decoded.push(b'\n'),
            Some(b'r') => decoded.push(b'\r'),
            Some(b't') => decoded.push(b'\t'),
            Some(b'u') => self.unicode_escape(decoded)?,
            Some(_) => return Err(self.error(ErrorKind::InvalidEscape)),
            None => return Err(self.error(ErrorKind::UnexpectedEnd)),
        }
        Ok(())
    }

    fn unicode_escape(&mut self, decoded: &mut Vec<u8>) -> Result<(), Error> {
        let first = self.hex_quad()?;
        let scalar = if (0xd800..=0xdbff).contains(&first) {
            if self.take() != Some(b'\\') || self.take() != Some(b'u') {
                return Err(self.error(ErrorKind::InvalidUnicode));
            }
            let second = self.hex_quad()?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(self.error(ErrorKind::InvalidUnicode));
            }
            0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00)
        } else if (0xdc00..=0xdfff).contains(&first) {
            return Err(self.error(ErrorKind::InvalidUnicode));
        } else {
            u32::from(first)
        };
        let character =
            char::from_u32(scalar).ok_or_else(|| self.error(ErrorKind::InvalidUnicode))?;
        let mut buffer = [0; 4];
        decoded.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    fn hex_quad(&mut self) -> Result<u16, Error> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let digit = match self.take() {
                Some(byte @ b'0'..=b'9') => u16::from(byte - b'0'),
                Some(byte @ b'a'..=b'f') => u16::from(byte - b'a') + 10,
                Some(byte @ b'A'..=b'F') => u16::from(byte - b'A') + 10,
                Some(_) => return Err(self.error(ErrorKind::InvalidEscape)),
                None => return Err(self.error(ErrorKind::UnexpectedEnd)),
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<String, Error> {
        let start = self.offset;
        if self.peek() == Some(b'-') {
            self.offset += 1;
        }
        match self.take() {
            Some(b'0') => {
                if self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                    return Err(self.error(ErrorKind::InvalidNumber));
                }
            }
            Some(b'1'..=b'9') => {
                while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                    self.offset += 1;
                }
            }
            _ => return Err(self.error(ErrorKind::InvalidNumber)),
        }
        if self.peek() == Some(b'.') {
            self.offset += 1;
            self.digits()?;
        }
        if self.peek().is_some_and(|byte| matches!(byte, b'e' | b'E')) {
            self.offset += 1;
            if self.peek().is_some_and(|byte| matches!(byte, b'+' | b'-')) {
                self.offset += 1;
            }
            self.digits()?;
        }
        String::from_utf8(self.bytes[start..self.offset].to_vec())
            .map_err(|_| self.error(ErrorKind::InvalidNumber))
    }

    fn digits(&mut self) -> Result<(), Error> {
        let start = self.offset;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.offset += 1;
        }
        if self.offset == start {
            return Err(self.error(ErrorKind::InvalidNumber));
        }
        Ok(())
    }
}
