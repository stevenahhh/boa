//! Builds the value of a JSON text in one pass, for `JSON.parse` calls without a reviver.
//!
//! `JSON.parse` otherwise validates the text with `serde_json` and then evaluates it as a script.
//! [`parse`] returns the value that path would return, or `None` for any text where it cannot
//! promise that; the caller then takes the other path, which also produces every error. Accepted
//! texts are a subset of the texts that path accepts, except that they may nest to any depth, where
//! `serde_json` stops at 128 levels, and values are built the way that script builds them:
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

/// Texts with numbers of this magnitude or more are left to the other path, which rejects some
/// numbers near `f64::MAX` that convert to a finite value.
const MAX_MAGNITUDE: f64 = 1e308;

/// Exponents with more digits than this are left to the other path.
const MAX_EXPONENT_DIGITS: usize = 3;

/// Returns the value of the JSON text `text`, or `None` if it must be parsed by the other path.
///
/// Arrays and objects are built with an explicit stack of the open ones, so nesting takes heap
/// instead of native stack.
pub(super) fn parse(text: &str, context: &mut Context) -> Option<JsValue> {
    let mut parser = Parser { text, pos: 0 };
    let mut open = Vec::new();
    parser.skip_whitespace();
    let value = 'value: loop {
        let mut value = match parser.peek()? {
            b'{' => {
                parser.pos += 1;
                parser.skip_whitespace();
                let object = JsObject::with_object_proto(context.intrinsics());
                if !parser.eat(b'}') {
                    let key = parser.member_key()?;
                    open.push(Open::Object(object, key));
                    continue;
                }
                object.into()
            }
            b'[' => {
                parser.pos += 1;
                parser.skip_whitespace();
                if !parser.eat(b']') {
                    open.push(Open::Array(Vec::new()));
                    continue;
                }
                Array::create_array_from_list(Vec::new(), context).into()
            }
            b'"' => parser.string().map(JsValue::from)?,
            b't' => parser.keyword("true", JsValue::from(true))?,
            b'f' => parser.keyword("false", JsValue::from(false))?,
            b'n' => parser.keyword("null", JsValue::null())?,
            b'-' | b'0'..=b'9' => parser.number(context)?,
            _ => return None,
        };

        // Add the value to the array or object it is in, and close the ones that end after it.
        loop {
            let Some(container) = open.last_mut() else {
                break 'value value;
            };
            parser.skip_whitespace();
            match container {
                Open::Array(elements) => {
                    elements.push(value);
                    if parser.eat(b',') {
                        parser.skip_whitespace();
                        continue 'value;
                    }
                    if !parser.eat(b']') {
                        return None;
                    }
                    let Some(Open::Array(elements)) = open.pop() else {
                        unreachable!("the last open value is this array");
                    };
                    value = Array::create_array_from_list(elements, context).into();
                }
                Open::Object(object, key) => {
                    object
                        .create_data_property_or_throw(key.clone(), value, context)
                        .ok()?;
                    if parser.eat(b',') {
                        parser.skip_whitespace();
                        *key = parser.member_key()?;
                        continue 'value;
                    }
                    if !parser.eat(b'}') {
                        return None;
                    }
                    let Some(Open::Object(object, _)) = open.pop() else {
                        unreachable!("the last open value is this object");
                    };
                    value = object.into();
                }
            }
        }
    };
    parser.skip_whitespace();
    (parser.pos == text.len()).then_some(value)
}

/// An array or object whose closing bracket has not been read yet.
enum Open {
    /// The elements of an array read so far.
    Array(Vec<JsValue>),
    /// An object and the key of the member whose value comes next.
    Object(JsObject, JsString),
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
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

    fn keyword(&mut self, word: &str, value: JsValue) -> Option<JsValue> {
        if !self.text[self.pos..].starts_with(word) {
            return None;
        }
        self.pos += word.len();
        Some(value)
    }

    /// Reads the key of an object member and the `:` after it, and the whitespace around both.
    fn member_key(&mut self) -> Option<JsString> {
        if self.peek()? != b'"' {
            return None;
        }
        let key = self.string()?;
        self.skip_whitespace();
        if !self.eat(b':') {
            return None;
        }
        self.skip_whitespace();
        Some(key)
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
