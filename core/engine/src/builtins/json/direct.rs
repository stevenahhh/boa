//! Builds the value of a JSON text in one pass, for `JSON.parse` calls without a reviver.
//!
//! `JSON.parse` otherwise validates the text with `serde_json` and then evaluates it as a script.
//! [`parse`] returns the value that path would return, or `None` for any text where it cannot
//! promise that; the caller then takes the other path, which also produces every error. Accepted
//! texts are a subset of the texts that path accepts, and values are built the way that script
//! builds them:
//! - objects are ordinary objects with `%Object.prototype%` whose members are defined in source
//!   order with `CreateDataProperty`, so a repeated key keeps the position of its first member and
//!   the value of its last, `__proto__` is an own data property, and canonical numeric keys become
//!   integer-indexed properties;
//! - arrays are created from their elements in order;
//! - numbers are converted the way the lexer converts a numeric literal, and a leading `-` is
//!   applied like the unary minus operator, so `-0` is negative zero.

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::{Context, JsObject, JsString, JsValue, builtins::Array};

/// Deeper texts are left to the other path, which rejects 128 levels of nesting.
const MAX_DEPTH: u32 = 100;

/// Texts with numbers of this magnitude or more are left to the other path, which rejects some
/// numbers near `f64::MAX` that convert to a finite value.
const MAX_MAGNITUDE: f64 = 1e308;

/// Exponents with more digits than this are left to the other path.
const MAX_EXPONENT_DIGITS: usize = 3;

/// Returns the value of the JSON text `text`, or `None` if it must be parsed by the other path.
pub(super) fn parse(text: &str, context: &mut Context) -> Option<JsValue> {
    let mut parser = Parser {
        text,
        pos: 0,
        depth: 0,
    };
    parser.skip_whitespace();
    let value = parser.value(context)?;
    parser.skip_whitespace();
    (parser.pos == text.len()).then_some(value)
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    depth: u32,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.pos += 1;
        }
        found
    }

    fn skip_whitespace(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.pos += 1;
        }
    }

    fn skip_digits(&mut self) -> usize {
        let start = self.pos;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn value(&mut self, context: &mut Context) -> Option<JsValue> {
        match self.peek()? {
            b'{' => self.object(context),
            b'[' => self.array(context),
            b'"' => self.string().map(JsValue::from),
            b't' => self.keyword("true", JsValue::from(true)),
            b'f' => self.keyword("false", JsValue::from(false)),
            b'n' => self.keyword("null", JsValue::null()),
            b'-' | b'0'..=b'9' => self.number(context),
            _ => None,
        }
    }

    fn keyword(&mut self, word: &str, value: JsValue) -> Option<JsValue> {
        if !self.text[self.pos..].starts_with(word) {
            return None;
        }
        self.pos += word.len();
        Some(value)
    }

    fn enter(&mut self) -> Option<()> {
        if self.depth >= MAX_DEPTH {
            return None;
        }
        self.depth += 1;
        self.pos += 1;
        self.skip_whitespace();
        Some(())
    }

    fn object(&mut self, context: &mut Context) -> Option<JsValue> {
        self.enter()?;
        let object = JsObject::with_object_proto(context.intrinsics());
        if !self.eat(b'}') {
            loop {
                if self.peek()? != b'"' {
                    return None;
                }
                let key = self.string()?;
                self.skip_whitespace();
                if !self.eat(b':') {
                    return None;
                }
                self.skip_whitespace();
                let value = self.value(context)?;
                object
                    .create_data_property_or_throw(key, value, context)
                    .ok()?;
                self.skip_whitespace();
                if self.eat(b'}') {
                    break;
                }
                if !self.eat(b',') {
                    return None;
                }
                self.skip_whitespace();
            }
        }
        self.depth -= 1;
        Some(object.into())
    }

    fn array(&mut self, context: &mut Context) -> Option<JsValue> {
        self.enter()?;
        let mut elements = Vec::new();
        if !self.eat(b']') {
            loop {
                elements.push(self.value(context)?);
                self.skip_whitespace();
                if self.eat(b']') {
                    break;
                }
                if !self.eat(b',') {
                    return None;
                }
                self.skip_whitespace();
            }
        }
        self.depth -= 1;
        Some(Array::create_array_from_list(elements, context).into())
    }

    fn string(&mut self) -> Option<JsString> {
        self.pos += 1;
        let start = self.pos;
        loop {
            match self.peek()? {
                b'"' => {
                    let string = JsString::from(&self.text[start..self.pos]);
                    self.pos += 1;
                    return Some(string);
                }
                b'\\' => break,
                0x00..=0x1F => return None,
                _ => self.pos += 1,
            }
        }

        let mut units: Vec<u16> = self.text[start..self.pos].encode_utf16().collect();
        loop {
            match self.peek()? {
                b'"' => {
                    self.pos += 1;
                    return Some(JsString::from(units.as_slice()));
                }
                b'\\' => {
                    self.pos += 1;
                    let unit = match self.peek()? {
                        b'"' => 0x22,
                        b'\\' => 0x5C,
                        b'/' => 0x2F,
                        b'b' => 0x08,
                        b'f' => 0x0C,
                        b'n' => 0x0A,
                        b'r' => 0x0D,
                        b't' => 0x09,
                        b'u' => {
                            self.unicode_escape(&mut units)?;
                            continue;
                        }
                        _ => return None,
                    };
                    self.pos += 1;
                    units.push(unit);
                }
                0x00..=0x1F => return None,
                _ => {
                    let run = self.pos;
                    while self
                        .peek()
                        .is_some_and(|byte| byte != b'"' && byte != b'\\' && byte >= 0x20)
                    {
                        self.pos += 1;
                    }
                    units.extend(self.text[run..self.pos].encode_utf16());
                }
            }
        }
    }

    /// Reads the `uXXXX` of an escape, plus the low half of a surrogate pair. A surrogate that is
    /// not half of an escaped pair is left to the other path, which rejects it.
    fn unicode_escape(&mut self, units: &mut Vec<u16>) -> Option<()> {
        self.pos += 1;
        let unit = self.hex4()?;
        match unit {
            0xD800..=0xDBFF => {
                if !self.text[self.pos..].starts_with("\\u") {
                    return None;
                }
                self.pos += 2;
                let low = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                    return None;
                }
                units.extend([unit, low]);
            }
            0xDC00..=0xDFFF => return None,
            _ => units.push(unit),
        }
        Some(())
    }

    fn hex4(&mut self) -> Option<u16> {
        let digits = self.text.get(self.pos..self.pos + 4)?;
        if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        self.pos += 4;
        u16::from_str_radix(digits, 16).ok()
    }

    fn number(&mut self, context: &mut Context) -> Option<JsValue> {
        let negative = self.eat(b'-');
        let start = self.pos;
        match self.peek()? {
            b'0' => self.pos += 1,
            b'1'..=b'9' => {
                self.skip_digits();
            }
            _ => return None,
        }
        let mut integer = true;
        if self.eat(b'.') {
            integer = false;
            if self.skip_digits() == 0 {
                return None;
            }
        }
        if let Some(b'e' | b'E') = self.peek() {
            integer = false;
            self.pos += 1;
            if let Some(b'+' | b'-') = self.peek() {
                self.pos += 1;
            }
            let digits = self.skip_digits();
            if digits == 0 || digits > MAX_EXPONENT_DIGITS {
                return None;
            }
        }
        let literal = &self.text[start..self.pos];

        let magnitude = if integer {
            match literal.parse::<i32>() {
                Ok(value) => JsValue::new(value),
                Err(_) => JsValue::new(
                    BigInt::parse_bytes(literal.as_bytes(), 10)?
                        .to_f64()
                        .unwrap_or(f64::INFINITY),
                ),
            }
        } else {
            let value: f64 = fast_float2::parse(literal).ok()?;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the lexer stores a literal as an integer exactly when this truncation is lossless"
            )]
            let truncated = value as i32;
            #[expect(clippy::float_cmp, reason = "an exact comparison is the point")]
            if f64::from(truncated) == value {
                JsValue::new(truncated)
            } else {
                JsValue::new(value)
            }
        };
        if magnitude
            .as_number()
            .is_none_or(|value| value.abs() >= MAX_MAGNITUDE)
        {
            return None;
        }

        if negative {
            magnitude.neg(context).ok()
        } else {
            Some(magnitude)
        }
    }
}
