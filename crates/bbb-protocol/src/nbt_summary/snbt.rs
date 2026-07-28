use uuid::Uuid;

use crate::codec::{ProtocolError, Result};

use super::{
    NbtSummaryEntry, NbtSummaryValue, MAX_NBT_ARRAY_ITEMS, MAX_NBT_COMPOUND_ENTRIES, MAX_NBT_DEPTH,
    MAX_NBT_LIST_ITEMS,
};

mod openjdk_character_names;

use openjdk_character_names::codepoint_by_name;

/// Parses the compound-only flattened SNBT surface used by vanilla
/// `TagParser.FLATTENED_CODEC`.
pub fn parse_snbt_compound(input: &str) -> Result<NbtSummaryValue> {
    let mut parser = SnbtParser::new(input);
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if !parser.is_empty() {
        return Err(parser.error("trailing data after SNBT value"));
    }
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(
            "flattened SNBT root is not a compound".to_string(),
        ));
    }
    Ok(value)
}

/// Validates the compound at the start of an argument string and returns the
/// byte length it consumed. This mirrors `TagParser.parseCompoundAsArgument`:
/// callers, such as `NbtPathArgument`, own the delimiter that follows it.
pub(crate) fn validate_snbt_compound_prefix(input: &str) -> Result<usize> {
    let mut parser = SnbtParser::new(input);
    let value = parser.parse_value(0)?;
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(
            "SNBT argument root is not a compound".to_string(),
        ));
    }
    Ok(parser.cursor)
}

pub(crate) fn validate_snbt_prefix(input: &str) -> Result<usize> {
    parse_snbt_prefix(input).map(|(consumed, _)| consumed)
}

pub(crate) fn parse_snbt_prefix(input: &str) -> Result<(usize, NbtSummaryValue)> {
    let mut parser = SnbtParser::new(input);
    let value = parser.parse_value(0)?;
    Ok((parser.cursor, value))
}

struct SnbtParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> SnbtParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn parse_value(&mut self, depth: usize) -> Result<NbtSummaryValue> {
        #[cfg(test)]
        crate::test_stack_budget::record();

        if depth > MAX_NBT_DEPTH {
            return Err(self.error("SNBT exceeded max depth"));
        }
        self.skip_whitespace();
        let Some(next) = self.peek() else {
            return Err(self.error("expected SNBT value"));
        };
        match next {
            '{' => self.parse_compound(depth),
            '[' => self.parse_list_or_array(depth),
            '\'' | '"' => self.parse_quoted_string().map(NbtSummaryValue::String),
            next if can_start_number(next) => {
                let token = self.read_unquoted();
                parse_number_token(token).map_err(|reason| self.error(reason))
            }
            _ => self.parse_unquoted_value(depth),
        }
    }

    fn parse_compound(&mut self, depth: usize) -> Result<NbtSummaryValue> {
        self.expect('{')?;
        let mut entries: Vec<NbtSummaryEntry> = Vec::new();
        let mut entry_count = 0usize;
        self.skip_whitespace();
        if self.consume('}') {
            return Ok(NbtSummaryValue::Compound(entries));
        }
        loop {
            if entry_count >= MAX_NBT_COMPOUND_ENTRIES {
                return Err(self.error(format!(
                    "SNBT compound has more than {MAX_NBT_COMPOUND_ENTRIES} entries"
                )));
            }
            entry_count += 1;
            let key = self.parse_key()?;
            if key.is_empty() {
                return Err(self.error("SNBT compound key is empty"));
            }
            self.expect(':')?;
            let value = self.parse_value(depth + 1)?;
            super::insert_compound_entry(&mut entries, key, value);
            self.skip_whitespace();
            if self.consume('}') {
                break;
            }
            self.expect(',')?;
            self.skip_whitespace();
            if self.consume('}') {
                break;
            }
        }
        Ok(NbtSummaryValue::Compound(entries))
    }

    fn parse_key(&mut self) -> Result<String> {
        self.skip_whitespace();
        match self.peek() {
            Some('\'' | '"') => self.parse_quoted_string(),
            Some(_) => {
                let value = self.read_unquoted();
                if value.is_empty() {
                    Err(self.error("expected SNBT compound key"))
                } else {
                    Ok(value.to_string())
                }
            }
            None => Err(self.error("expected SNBT compound key")),
        }
    }

    fn parse_list_or_array(&mut self, depth: usize) -> Result<NbtSummaryValue> {
        self.expect('[')?;
        self.skip_whitespace();
        if let Some(array_type) = self.array_prefix() {
            return self.parse_array(array_type);
        }
        let mut values = Vec::new();
        if self.consume(']') {
            return Ok(NbtSummaryValue::List(values));
        }
        loop {
            if values.len() >= MAX_NBT_LIST_ITEMS {
                return Err(self.error(format!(
                    "SNBT list has more than {MAX_NBT_LIST_ITEMS} entries"
                )));
            }
            values.push(self.parse_value(depth + 1)?);
            self.skip_whitespace();
            if self.consume(']') {
                break;
            }
            self.expect(',')?;
            self.skip_whitespace();
            if self.consume(']') {
                break;
            }
        }
        Ok(NbtSummaryValue::List(values))
    }

    fn array_prefix(&mut self) -> Option<IntegerType> {
        let saved = self.cursor;
        let array_type = match self.peek()? {
            'B' => IntegerType::Byte,
            'I' => IntegerType::Int,
            'L' => IntegerType::Long,
            _ => return None,
        };
        self.bump();
        self.skip_whitespace();
        if self.consume(';') {
            Some(array_type)
        } else {
            self.cursor = saved;
            None
        }
    }

    fn parse_array(&mut self, array_type: IntegerType) -> Result<NbtSummaryValue> {
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume(']') {
            return Ok(array_value(array_type, values));
        }
        loop {
            if values.len() >= MAX_NBT_ARRAY_ITEMS {
                return Err(self.error(format!(
                    "SNBT array has more than {MAX_NBT_ARRAY_ITEMS} entries"
                )));
            }
            self.skip_whitespace();
            let token = self.read_unquoted();
            if token.is_empty() {
                return Err(self.error("expected integer SNBT array element"));
            }
            let (value, actual_type) = parse_integer_token(token, Some(array_type))
                .map_err(|reason| self.error(reason))?;
            if !array_type.accepts(actual_type) {
                return Err(self.error(format!(
                    "SNBT {} array does not accept {} element {token}",
                    array_type.name(),
                    actual_type.name()
                )));
            }
            values.push(value);
            self.skip_whitespace();
            if self.consume(']') {
                break;
            }
            self.expect(',')?;
            self.skip_whitespace();
            if self.consume(']') {
                break;
            }
        }
        Ok(array_value(array_type, values))
    }

    fn parse_unquoted_value(&mut self, depth: usize) -> Result<NbtSummaryValue> {
        let identifier = self.read_unquoted();
        if identifier.is_empty() {
            return Err(self.error("expected unquoted SNBT string"));
        }
        if can_start_number(identifier.chars().next().unwrap()) {
            return Err(self.error("unquoted SNBT string cannot start like a number"));
        }
        self.skip_whitespace();
        if self.consume('(') {
            let arguments = self.parse_arguments(depth)?;
            return self.apply_builtin(identifier, arguments);
        }
        if identifier.eq_ignore_ascii_case("true") {
            Ok(NbtSummaryValue::Byte(1))
        } else if identifier.eq_ignore_ascii_case("false") {
            Ok(NbtSummaryValue::Byte(0))
        } else {
            Ok(NbtSummaryValue::String(identifier.to_string()))
        }
    }

    fn parse_arguments(&mut self, depth: usize) -> Result<Vec<NbtSummaryValue>> {
        let mut arguments = Vec::new();
        self.skip_whitespace();
        if self.consume(')') {
            return Ok(arguments);
        }
        loop {
            if arguments.len() >= MAX_NBT_LIST_ITEMS {
                return Err(self.error(format!(
                    "SNBT builtin has more than {MAX_NBT_LIST_ITEMS} arguments"
                )));
            }
            arguments.push(self.parse_value(depth + 1)?);
            self.skip_whitespace();
            if self.consume(')') {
                break;
            }
            self.expect(',')?;
            self.skip_whitespace();
            if self.consume(')') {
                break;
            }
        }
        Ok(arguments)
    }

    fn apply_builtin(
        &self,
        identifier: &str,
        arguments: Vec<NbtSummaryValue>,
    ) -> Result<NbtSummaryValue> {
        if arguments.len() != 1 {
            return Err(self.error(format!(
                "unknown SNBT builtin {identifier}/{}",
                arguments.len()
            )));
        }
        let argument = arguments.into_iter().next().unwrap();
        match identifier {
            "bool" => numeric_truth(&argument)
                .map(|value| NbtSummaryValue::Byte(i8::from(value)))
                .ok_or_else(|| self.error("bool SNBT builtin expects a number or boolean")),
            "uuid" => {
                let NbtSummaryValue::String(raw) = argument else {
                    return Err(self.error("uuid SNBT builtin expects a string"));
                };
                let uuid = Uuid::parse_str(&raw)
                    .map_err(|_| self.error(format!("invalid uuid SNBT builtin value {raw}")))?;
                let value = uuid.as_u128();
                Ok(NbtSummaryValue::IntArray(vec![
                    (value >> 96) as u32 as i32,
                    (value >> 64) as u32 as i32,
                    (value >> 32) as u32 as i32,
                    value as u32 as i32,
                ]))
            }
            _ => Err(self.error(format!("unknown SNBT builtin {identifier}/1"))),
        }
    }

    fn parse_quoted_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        let quote = self.bump().unwrap();
        debug_assert!(matches!(quote, '\'' | '"'));
        let mut output = Vec::new();
        loop {
            let Some(next) = self.bump() else {
                return Err(self.error("unterminated quoted SNBT string"));
            };
            if next == quote {
                return String::from_utf16(&output)
                    .map_err(|_| self.error("SNBT string contains an unpaired UTF-16 surrogate"));
            }
            if next != '\\' {
                append_java_codepoint(&mut output, next as u32);
                continue;
            }
            self.skip_whitespace();
            let Some(escape) = self.bump() else {
                return Err(self.error("unterminated SNBT string escape"));
            };
            let codepoint = match escape {
                'b' => '\u{0008}' as u32,
                's' => ' ' as u32,
                't' => '\t' as u32,
                'n' => '\n' as u32,
                'f' => '\u{000c}' as u32,
                'r' => '\r' as u32,
                '\\' => '\\' as u32,
                '\'' => '\'' as u32,
                '"' => '"' as u32,
                'x' => self.parse_hex_escape(2)?,
                'u' => self.parse_hex_escape(4)?,
                'U' => self.parse_hex_escape(8)?,
                'N' => self.parse_named_unicode_escape()?,
                _ => return Err(self.error(format!("unknown SNBT string escape \\{escape}"))),
            };
            append_java_codepoint(&mut output, codepoint);
        }
    }

    fn parse_named_unicode_escape(&mut self) -> Result<u32> {
        self.expect('{')?;
        let start = self.cursor;
        while self.peek().is_some_and(is_unicode_name_character) {
            self.bump();
        }
        let name = self.input[start..self.cursor].to_string();
        if name.is_empty() {
            return Err(self.error("SNBT named Unicode escape has an empty name"));
        }
        self.expect('}')?;
        codepoint_by_name(&name)
            .ok_or_else(|| self.error(format!("unknown SNBT Unicode character name {name}")))
    }

    fn parse_hex_escape(&mut self, digits: usize) -> Result<u32> {
        let start = self.cursor;
        for _ in 0..digits {
            let Some(value) = self.bump() else {
                return Err(self.error(format!(
                    "SNBT Unicode escape needs {digits} hexadecimal digits"
                )));
            };
            if !value.is_ascii_hexdigit() {
                return Err(self.error(format!(
                    "SNBT Unicode escape needs {digits} hexadecimal digits"
                )));
            }
        }
        let raw = &self.input[start..self.cursor];
        let codepoint = u32::from_str_radix(raw, 16).unwrap();
        if codepoint <= 0x10ffff {
            Ok(codepoint)
        } else {
            Err(self.error(format!("invalid SNBT Unicode codepoint U+{codepoint:08X}")))
        }
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        self.skip_whitespace();
        if self.consume(expected) {
            Ok(())
        } else {
            Err(self.error(format!("expected '{expected}' in SNBT")))
        }
    }

    fn read_unquoted(&mut self) -> &'a str {
        self.skip_whitespace();
        let start = self.cursor;
        while self.peek().is_some_and(is_unquoted_character) {
            self.bump();
        }
        &self.input[start..self.cursor]
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.cursor += value.len_utf8();
        Some(value)
    }

    fn is_empty(&self) -> bool {
        self.cursor == self.input.len()
    }

    fn error(&self, reason: impl Into<String>) -> ProtocolError {
        ProtocolError::InvalidData(format!("{} at byte {}", reason.into(), self.cursor))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntegerType {
    Byte,
    Short,
    Int,
    Long,
}

impl IntegerType {
    fn name(self) -> &'static str {
        match self {
            Self::Byte => "byte",
            Self::Short => "short",
            Self::Int => "int",
            Self::Long => "long",
        }
    }

    fn bits(self) -> u32 {
        match self {
            Self::Byte => 8,
            Self::Short => 16,
            Self::Int => 32,
            Self::Long => 64,
        }
    }

    fn accepts(self, actual: Self) -> bool {
        match self {
            Self::Byte => actual == Self::Byte,
            Self::Int => matches!(actual, Self::Byte | Self::Short | Self::Int),
            Self::Long => true,
            Self::Short => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Signedness {
    Signed,
    Unsigned,
}

fn parse_number_token(token: &str) -> std::result::Result<NbtSummaryValue, String> {
    if !has_base_prefix(token) {
        if let Some(float) = parse_float_token(token)? {
            return Ok(float);
        }
    }
    parse_integer_token(token, None).map(|(value, _)| value)
}

fn parse_float_token(token: &str) -> std::result::Result<Option<NbtSummaryValue>, String> {
    let (sign, unsigned) = strip_sign(token);
    if unsigned.is_empty() {
        return Err(format!("invalid numeric SNBT literal {token}"));
    }
    let (body, float_type) = match unsigned.as_bytes().last().copied() {
        Some(b'f' | b'F') => (&unsigned[..unsigned.len() - 1], Some('f')),
        Some(b'd' | b'D') => (&unsigned[..unsigned.len() - 1], Some('d')),
        _ => (unsigned, None),
    };
    let has_float_shape =
        float_type.is_some() || body.contains('.') || body.contains('e') || body.contains('E');
    if !has_float_shape {
        return Ok(None);
    }
    if body.is_empty() {
        return Err(format!("invalid floating SNBT literal {token}"));
    }
    let mut exponent_parts = body.split(['e', 'E']);
    let significand = exponent_parts.next().unwrap();
    let exponent = exponent_parts.next();
    if exponent_parts.next().is_some() {
        return Err(format!("invalid floating SNBT literal {token}"));
    }
    if let Some(exponent) = exponent {
        let (_, digits) = strip_sign(exponent);
        validate_digit_run(digits, 10, "floating exponent")?;
    }
    let mut decimal_parts = significand.split('.');
    let whole = decimal_parts.next().unwrap();
    let fraction = decimal_parts.next();
    if decimal_parts.next().is_some() {
        return Err(format!("invalid floating SNBT literal {token}"));
    }
    match fraction {
        Some(fraction) => {
            if whole.is_empty() && fraction.is_empty() {
                return Err(format!("invalid floating SNBT literal {token}"));
            }
            if !whole.is_empty() {
                validate_digit_run(whole, 10, "floating whole part")?;
            }
            if !fraction.is_empty() {
                validate_digit_run(fraction, 10, "floating fraction")?;
            }
        }
        None => validate_digit_run(whole, 10, "floating whole part")?,
    }
    let mut cleaned = String::new();
    if sign == Some('-') {
        cleaned.push('-');
    }
    cleaned.extend(body.chars().filter(|value| *value != '_'));
    if float_type == Some('f') {
        let value = cleaned
            .parse::<f32>()
            .map_err(|_| format!("invalid floating SNBT literal {token}"))?;
        if !value.is_finite() {
            return Err(format!("non-finite floating SNBT literal {token}"));
        }
        Ok(Some(NbtSummaryValue::Float(value.to_bits())))
    } else {
        let value = cleaned
            .parse::<f64>()
            .map_err(|_| format!("invalid floating SNBT literal {token}"))?;
        if !value.is_finite() {
            return Err(format!("non-finite floating SNBT literal {token}"));
        }
        Ok(Some(NbtSummaryValue::Double(value.to_bits())))
    }
}

fn parse_integer_token(
    token: &str,
    default_type: Option<IntegerType>,
) -> std::result::Result<(NbtSummaryValue, IntegerType), String> {
    let (sign, unsigned) = strip_sign(token);
    if unsigned.is_empty() {
        return Err(format!("invalid integer SNBT literal {token}"));
    }
    let (base, digits_start) = if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        (16, 2)
    } else if unsigned.starts_with("0b") || unsigned.starts_with("0B") {
        (2, 2)
    } else {
        (10, 0)
    };
    let rest = &unsigned[digits_start..];
    let digit_len = rest
        .char_indices()
        .take_while(|(_, value)| is_radix_digit_or_underscore(*value, base))
        .map(|(index, value)| index + value.len_utf8())
        .last()
        .unwrap_or(0);
    let digits = &rest[..digit_len];
    validate_digit_run(digits, base, "integer")?;
    if base == 10 {
        let cleaned = digits.replace('_', "");
        if cleaned.len() > 1 && cleaned.starts_with('0') {
            return Err(format!("leading zero in integer SNBT literal {token}"));
        }
    }
    let suffix = &rest[digit_len..];
    let (explicit_sign, explicit_type) = parse_integer_suffix(suffix)
        .ok_or_else(|| format!("invalid integer SNBT suffix in {token}"))?;
    let value_type = explicit_type.or(default_type).unwrap_or(IntegerType::Int);
    let signedness = explicit_sign.unwrap_or(if base == 10 {
        Signedness::Signed
    } else {
        Signedness::Unsigned
    });
    if signedness == Signedness::Unsigned && sign == Some('-') {
        return Err(format!("negative unsigned integer SNBT literal {token}"));
    }
    let cleaned = digits.replace('_', "");
    let magnitude = u128::from_str_radix(&cleaned, base)
        .map_err(|_| format!("integer SNBT literal is out of range {token}"))?;
    let value = integer_value(value_type, signedness, sign == Some('-'), magnitude)
        .ok_or_else(|| format!("integer SNBT literal is out of range {token}"))?;
    Ok((value, value_type))
}

fn parse_integer_suffix(suffix: &str) -> Option<(Option<Signedness>, Option<IntegerType>)> {
    if suffix.is_empty() {
        return Some((None, None));
    }
    let lower = suffix.to_ascii_lowercase();
    let (signedness, type_name) = match lower.len() {
        1 => (None, lower.as_str()),
        2 => {
            let signedness = match &lower[..1] {
                "s" => Signedness::Signed,
                "u" => Signedness::Unsigned,
                _ => return None,
            };
            (Some(signedness), &lower[1..])
        }
        _ => return None,
    };
    let value_type = match type_name {
        "b" => IntegerType::Byte,
        "s" => IntegerType::Short,
        "i" => IntegerType::Int,
        "l" => IntegerType::Long,
        _ => return None,
    };
    Some((signedness, Some(value_type)))
}

fn integer_value(
    value_type: IntegerType,
    signedness: Signedness,
    negative: bool,
    magnitude: u128,
) -> Option<NbtSummaryValue> {
    let bits = value_type.bits();
    let raw = match signedness {
        Signedness::Signed => {
            let max = (1u128 << (bits - 1)) - 1;
            let min_magnitude = 1u128 << (bits - 1);
            if negative {
                (magnitude <= min_magnitude).then(|| -(magnitude as i128))?
            } else {
                (magnitude <= max).then_some(magnitude as i128)?
            }
        }
        Signedness::Unsigned => {
            let max = if bits == 64 {
                u64::MAX as u128
            } else {
                (1u128 << bits) - 1
            };
            (magnitude <= max).then_some(magnitude as i128)?
        }
    };
    Some(match value_type {
        IntegerType::Byte => NbtSummaryValue::Byte(raw as u8 as i8),
        IntegerType::Short => NbtSummaryValue::Short(raw as u16 as i16),
        IntegerType::Int => NbtSummaryValue::Int(raw as u32 as i32),
        IntegerType::Long => NbtSummaryValue::Long(raw as u64 as i64),
    })
}

fn array_value(array_type: IntegerType, values: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    match array_type {
        IntegerType::Byte => NbtSummaryValue::ByteArray(
            values
                .into_iter()
                .map(|value| match value {
                    NbtSummaryValue::Byte(value) => value,
                    _ => unreachable!("byte array accepted only byte values"),
                })
                .collect(),
        ),
        IntegerType::Int => {
            NbtSummaryValue::IntArray(values.into_iter().map(widen_integer_to_i32).collect())
        }
        IntegerType::Long => {
            NbtSummaryValue::LongArray(values.into_iter().map(widen_integer_to_i64).collect())
        }
        IntegerType::Short => unreachable!("SNBT has no short array"),
    }
}

fn widen_integer_to_i32(value: NbtSummaryValue) -> i32 {
    match value {
        NbtSummaryValue::Byte(value) => value.into(),
        NbtSummaryValue::Short(value) => value.into(),
        NbtSummaryValue::Int(value) => value,
        _ => unreachable!("int array accepted only byte/short/int values"),
    }
}

fn widen_integer_to_i64(value: NbtSummaryValue) -> i64 {
    match value {
        NbtSummaryValue::Byte(value) => value.into(),
        NbtSummaryValue::Short(value) => value.into(),
        NbtSummaryValue::Int(value) => value.into(),
        NbtSummaryValue::Long(value) => value,
        _ => unreachable!("long array accepted only integer values"),
    }
}

fn numeric_truth(value: &NbtSummaryValue) -> Option<bool> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => *value != 0,
        NbtSummaryValue::Int(value) => *value != 0,
        NbtSummaryValue::Long(value) => *value != 0,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) != 0.0,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) != 0.0,
        _ => return None,
    })
}

fn strip_sign(value: &str) -> (Option<char>, &str) {
    match value.as_bytes().first().copied() {
        Some(b'+') => (Some('+'), &value[1..]),
        Some(b'-') => (Some('-'), &value[1..]),
        _ => (None, value),
    }
}

fn validate_digit_run(value: &str, radix: u32, part: &str) -> std::result::Result<(), String> {
    if value.is_empty()
        || value.starts_with('_')
        || value.ends_with('_')
        || !value
            .chars()
            .all(|digit| is_radix_digit_or_underscore(digit, radix))
    {
        return Err(format!("invalid {part} SNBT numeral {value}"));
    }
    Ok(())
}

fn has_base_prefix(value: &str) -> bool {
    let (_, value) = strip_sign(value);
    value.starts_with("0x")
        || value.starts_with("0X")
        || value.starts_with("0b")
        || value.starts_with("0B")
}

fn is_radix_digit_or_underscore(value: char, radix: u32) -> bool {
    value == '_' || value.is_digit(radix)
}

fn is_unquoted_character(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '_' | '-' | '.' | '+')
}

fn is_unicode_name_character(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '-' | ' ')
}

/// Appends `Character.toString(codePoint)`'s UTF-16 representation. BMP
/// surrogate codepoints remain code units until the completed string validates
/// whether they form pairs.
fn append_java_codepoint(output: &mut Vec<u16>, codepoint: u32) {
    debug_assert!(codepoint <= 0x10ffff);
    if codepoint <= u16::MAX as u32 {
        output.push(codepoint as u16);
    } else {
        let supplementary = codepoint - 0x10000;
        output.push(0xd800 | (supplementary >> 10) as u16);
        output.push(0xdc00 | (supplementary & 0x03ff) as u16);
    }
}

fn can_start_number(value: char) -> bool {
    matches!(value, '+' | '-' | '.' | '0'..='9')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
        let NbtSummaryValue::Compound(entries) = value else {
            panic!("expected compound");
        };
        &entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap()
            .value
    }

    #[test]
    fn parses_compounds_lists_trailing_commas_and_last_duplicate_value() {
        let value = parse_snbt_compound(
            " { name: 'Alex', nested: {flag: true,}, values: [1, 2s, 'three',], name: final, } ",
        )
        .unwrap();
        assert_eq!(
            entry(&value, "name"),
            &NbtSummaryValue::String("final".into())
        );
        assert_eq!(
            entry(entry(&value, "nested"), "flag"),
            &NbtSummaryValue::Byte(1)
        );
        assert_eq!(
            entry(&value, "values"),
            &NbtSummaryValue::List(vec![
                NbtSummaryValue::Int(1),
                NbtSummaryValue::Short(2),
                NbtSummaryValue::String("three".into()),
            ])
        );
    }

    #[test]
    fn parses_typed_arrays_and_extended_integer_literals() {
        let value = parse_snbt_compound(
            "{bytes:[B;127b,255ub],ints:[I;1b,2s,0xFFFFui],longs:[L;1,2l,0b11ul]}",
        )
        .unwrap();
        assert_eq!(
            entry(&value, "bytes"),
            &NbtSummaryValue::ByteArray(vec![127, -1])
        );
        assert_eq!(
            entry(&value, "ints"),
            &NbtSummaryValue::IntArray(vec![1, 2, 65_535])
        );
        assert_eq!(
            entry(&value, "longs"),
            &NbtSummaryValue::LongArray(vec![1, 2, 3])
        );
    }

    #[test]
    fn parses_float_shapes_and_finite_suffixes() {
        let value = parse_snbt_compound("{a:.5f,b:1.,c:-2e3,d:4E-2d}").unwrap();
        assert_eq!(
            entry(&value, "a"),
            &NbtSummaryValue::Float(0.5f32.to_bits())
        );
        assert_eq!(
            entry(&value, "b"),
            &NbtSummaryValue::Double(1.0f64.to_bits())
        );
        assert_eq!(
            entry(&value, "c"),
            &NbtSummaryValue::Double((-2000.0f64).to_bits())
        );
        assert_eq!(
            entry(&value, "d"),
            &NbtSummaryValue::Double(0.04f64.to_bits())
        );
        assert!(parse_snbt_compound("{value:1e999}").is_err());
    }

    #[test]
    fn parses_vanilla_string_escapes_and_builtins() {
        let value = parse_snbt_compound(
            r#"{text:"line\n\x41\u0042\U00000043",face:"\U0001F600",truth:bool(-0.0f),uuid:uuid("00112233-4455-6677-8899-aabbccddeeff")}"#,
        )
        .unwrap();
        assert_eq!(
            entry(&value, "text"),
            &NbtSummaryValue::String("line\nABC".into())
        );
        assert_eq!(entry(&value, "face"), &NbtSummaryValue::String("😀".into()));
        assert_eq!(entry(&value, "truth"), &NbtSummaryValue::Byte(0));
        assert_eq!(
            entry(&value, "uuid"),
            &NbtSummaryValue::IntArray(vec![
                0x0011_2233,
                0x4455_6677,
                0x8899_aabbu32 as i32,
                0xccdd_eeffu32 as i32,
            ])
        );
    }

    #[test]
    fn joins_utf16_surrogate_pairs_from_unicode_escapes() {
        let value = parse_snbt_compound(
            r#"{short:"\uD83D\uDE00",long:"\U0000D83D\U0000DE00",named:"\N{HIGH SURROGATES D83D}\N{LOW SURROGATES DE00}",mixed:"\uD83D\U0000DE00"}"#,
        )
        .unwrap();
        for name in ["short", "long", "named", "mixed"] {
            assert_eq!(
                entry(&value, name),
                &NbtSummaryValue::String("😀".into()),
                "{name}"
            );
        }
    }

    #[test]
    fn rejects_unpaired_utf16_surrogate_escapes_explicitly() {
        for input in [
            r#"{value:"\uD800"}"#,
            r#"{value:"\uDC00"}"#,
            r#"{value:"\U0000D800"}"#,
            r#"{value:"\U0000DC00"}"#,
            r#"{value:"\N{HIGH SURROGATES D800}"}"#,
            r#"{value:"\N{LOW SURROGATES DC00}"}"#,
            r#"{value:"\uD800A"}"#,
            r#"{value:"A\uDC00"}"#,
            r#"{value:"\uD800\uD801"}"#,
            r#"{value:"\uDC00\uD800"}"#,
        ] {
            let error = parse_snbt_compound(input).unwrap_err().to_string();
            assert!(
                error.contains("unpaired UTF-16 surrogate"),
                "{input}: {error}"
            );
        }
    }

    #[test]
    fn rejects_unicode_escapes_outside_the_java_codepoint_range() {
        for input in [r#"{value:"\U00110000"}"#, r#"{value:"\UFFFFFFFF"}"#] {
            let error = parse_snbt_compound(input).unwrap_err().to_string();
            assert!(error.contains("invalid SNBT Unicode codepoint"), "{input}");
        }
    }

    #[test]
    fn rejects_non_compound_trailing_and_malformed_values() {
        for input in [
            "[]",
            "{} trailing",
            "{value:01}",
            "{value:0x}",
            "{value:128b}",
            "{value:[B;1s]}",
            "{value:'unterminated}",
            "{value:\"\\N{}\"}",
        ] {
            assert!(parse_snbt_compound(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn parses_openjdk_character_names_and_rejects_loose_matches() {
        let value = parse_snbt_compound(
            "{latin:'\\N{latin capital letter a}',control:'\\N{BEL}',hangul:'\\N{HANGUL SYLLABLES AC00}',private:'\\N{PRIVATE USE AREA E000}',spaced:'\\  N { BLACK STAR }'}",
        )
        .unwrap();
        assert_eq!(entry(&value, "latin"), &NbtSummaryValue::String("A".into()));
        assert_eq!(
            entry(&value, "control"),
            &NbtSummaryValue::String("\u{0007}".into())
        );
        assert_eq!(
            entry(&value, "hangul"),
            &NbtSummaryValue::String("가".into())
        );
        assert_eq!(
            entry(&value, "private"),
            &NbtSummaryValue::String("\u{e000}".into())
        );
        assert_eq!(
            entry(&value, "spaced"),
            &NbtSummaryValue::String("★".into())
        );
        for input in [
            "{value:'\\N{LATINCAPITALLETTERA}'}",
            "{value:'\\N{LATIN-CAPITAL LETTER A}'}",
            "{value:'\\N{HANGUL SYLLABLE GA}'}",
            "{value:'\\N{BS}'}",
            "{value:'\\N{BLACK_STAR}'}",
        ] {
            assert!(parse_snbt_compound(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn snbt_stack_budget_pins_depth_boundary_and_failure_recovery() {
        const STACK_SIZE_BYTES: usize = 2 * 1024 * 1024;
        const STACK_BUDGET_BYTES: usize = 3 * 1024 * 1024 / 2;

        std::thread::Builder::new()
            .stack_size(STACK_SIZE_BYTES)
            .spawn(|| {
                let accepted = nested_snbt_compound(MAX_NBT_DEPTH);
                let caller_marker = 0u8;
                crate::test_stack_budget::reset(&caller_marker);
                parse_snbt_compound(&accepted).unwrap();
                let used = crate::test_stack_budget::used_bytes();
                assert!(
                    used <= STACK_BUDGET_BYTES,
                    "depth-{MAX_NBT_DEPTH} flattened SNBT parse used {used} bytes of stack, over \
                     the {STACK_BUDGET_BYTES}-byte budget"
                );

                let rejected = nested_snbt_compound(MAX_NBT_DEPTH + 1);
                let error = parse_snbt_compound(&rejected).unwrap_err();
                assert!(
                    error.to_string().contains("SNBT exceeded max depth"),
                    "{error}"
                );

                assert_eq!(
                    parse_snbt_compound("{recovered:1}").unwrap(),
                    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "recovered".to_string(),
                        value: NbtSummaryValue::Int(1),
                    }])
                );
            })
            .expect("spawn flattened-SNBT stack-budget test thread")
            .join()
            .expect("flattened-SNBT stack-budget test thread");
    }

    fn nested_snbt_compound(depth: usize) -> String {
        assert!(depth > 0);
        let mut input = String::with_capacity(depth * 4 + 1);
        for _ in 0..depth {
            input.push_str("{a:");
        }
        input.push('0');
        for _ in 0..depth {
            input.push('}');
        }
        input
    }
}
