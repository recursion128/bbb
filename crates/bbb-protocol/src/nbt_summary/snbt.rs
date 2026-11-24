use uuid::Uuid;

use super::{
    canonical_double_tag_bits, canonical_float_tag_bits, NbtSummaryEntry, NbtSummaryValue,
    MAX_NBT_ARRAY_ITEMS, MAX_NBT_COMPOUND_ENTRIES, MAX_NBT_DEPTH, MAX_NBT_LIST_ITEMS,
};
use crate::java_string::JavaString;

mod openjdk_character_names;
#[cfg(test)]
mod vanilla_differential;

use openjdk_character_names::codepoint_by_name;

/// The refusal `Grammar.parseForCommands` (`Grammar.java:29-56`) throws: the
/// first error `ErrorCollector.LongestOnly` kept at the longest cursor the parse
/// reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnbtParseFailure {
    /// Where the refusal is reported, in the same unit as the consumed length
    /// its entry point returns: UTF-16 units for `*_utf16`, bytes otherwise.
    pub cursor: usize,
    /// The Vanilla translation key, or `None` for a bound this crate adds and
    /// Vanilla does not have (`MAX_NBT_DEPTH` and the collection caps).
    pub key: Option<&'static str>,
    /// What the key's `DynamicCommandExceptionType` formats into it.
    pub arguments: Vec<String>,
}

/// `CommandSyntaxException.BUILT_IN_EXCEPTIONS.literalIncorrect()`, which every
/// `StringReaderTerms` terminal stores (`StringReaderTerms.java:50`, `:76`).
const LITERAL_INCORRECT: &str = "argument.literal.incorrect";

impl SnbtParseFailure {
    fn at(cursor: usize, key: &'static str) -> Self {
        Self {
            cursor,
            key: Some(key),
            arguments: Vec::new(),
        }
    }

    fn at_with(cursor: usize, key: &'static str, argument: String) -> Self {
        Self {
            cursor,
            key: Some(key),
            arguments: vec![argument],
        }
    }

    /// A bound with no Vanilla counterpart, carried as text so a reader of the
    /// message still sees which one stopped the parse.
    fn bound(cursor: usize, what: &'static str) -> Self {
        Self {
            cursor,
            key: None,
            arguments: vec![what.to_string()],
        }
    }

    fn in_bytes(mut self, input: &str) -> Self {
        self.cursor = crate::component::command_utf16_byte_cursor(input, self.cursor);
        self
    }
}

impl std::fmt::Display for SnbtParseFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.key {
            Some(key) => write!(f, "{key}")?,
            None => write!(f, "SNBT bound")?,
        }
        for argument in &self.arguments {
            write!(f, " [{argument}]")?;
        }
        write!(f, " at {}", self.cursor)
    }
}

type SnbtResult<T> = std::result::Result<T, SnbtParseFailure>;

/// Parses the compound-only flattened SNBT surface used by vanilla
/// `TagParser.FLATTENED_CODEC`.
pub fn parse_snbt_compound(input: &str) -> SnbtResult<NbtSummaryValue> {
    parse_snbt_compound_utf16(&input.encode_utf16().collect::<Vec<_>>())
        .map_err(|failure| failure.in_bytes(input))
}

pub fn parse_snbt_compound_utf16(input: &[u16]) -> SnbtResult<NbtSummaryValue> {
    let mut parser = SnbtParser::new(input);
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if !parser.is_empty() {
        // `TagParser.parseFully` (TagParser.java:69-77) owns the tail, so this
        // one is not a grammar failure.
        return Err(SnbtParseFailure::at(parser.cursor, "argument.nbt.trailing"));
    }
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(SnbtParseFailure::at(
            parser.cursor,
            "argument.nbt.expected.compound",
        ));
    }
    Ok(value)
}

/// Parses the compound prefix consumed by `TagParser.parseCompoundAsArgument`
/// and leaves any caller-owned suffix unread.
pub fn parse_snbt_compound_prefix(input: &str) -> SnbtResult<(usize, NbtSummaryValue)> {
    let (consumed, value) =
        parse_snbt_compound_prefix_utf16(&input.encode_utf16().collect::<Vec<_>>())
            .map_err(|failure| failure.in_bytes(input))?;
    Ok((
        crate::component::command_utf16_byte_cursor(input, consumed),
        value,
    ))
}

/// Returns the consumed UTF-16 unit count without reading the caller's suffix.
pub fn parse_snbt_compound_prefix_utf16(input: &[u16]) -> SnbtResult<(usize, NbtSummaryValue)> {
    let (consumed, value) = parse_snbt_prefix_utf16(input)?;
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        // `castToCompoundOrThrow` (TagParser.java:52-58) reports at the cursor
        // the grammar left, not where the value began.
        return Err(SnbtParseFailure::at(
            consumed,
            "argument.nbt.expected.compound",
        ));
    }
    Ok((consumed, value))
}

/// Validates the compound at the start of an argument string and returns the
/// byte length it consumed. This mirrors `TagParser.parseCompoundAsArgument`:
/// callers, such as `NbtPathArgument`, own the delimiter that follows it.
pub(crate) fn validate_snbt_compound_prefix(input: &str) -> SnbtResult<usize> {
    parse_snbt_compound_prefix(input).map(|(consumed, _)| consumed)
}

pub(crate) fn validate_snbt_prefix(input: &str) -> SnbtResult<usize> {
    parse_snbt_prefix(input).map(|(consumed, _)| consumed)
}

pub(crate) fn parse_snbt_prefix(input: &str) -> SnbtResult<(usize, NbtSummaryValue)> {
    let (consumed, value) = parse_snbt_prefix_utf16(&input.encode_utf16().collect::<Vec<_>>())
        .map_err(|failure| failure.in_bytes(input))?;
    Ok((
        crate::component::command_utf16_byte_cursor(input, consumed),
        value,
    ))
}

/// Parses one tag and returns its consumed UTF-16 unit count.
pub fn parse_snbt_prefix_utf16(input: &[u16]) -> SnbtResult<(usize, NbtSummaryValue)> {
    let mut parser = SnbtParser::new(input);
    let value = parser.parse_value(0)?;
    Ok((parser.cursor, value))
}

struct SnbtParser<'a> {
    input: &'a [u16],
    cursor: usize,
    /// The farthest failure recorded so far.
    ///
    /// `Grammar.parseForCommands` reports the first error stored at the longest
    /// cursor reached (`ErrorCollector.LongestOnly` drops everything shorter and
    /// keeps entries in store order, `Grammar.java:33-41`), which is why every
    /// refusal below is recorded here and the mark -- not the caller's own
    /// reason -- is what leaves the parser.
    error_mark: Option<SnbtParseFailure>,
}

impl<'a> SnbtParser<'a> {
    fn new(input: &'a [u16]) -> Self {
        Self {
            input,
            cursor: 0,
            error_mark: None,
        }
    }

    fn parse_value(&mut self, depth: usize) -> SnbtResult<NbtSummaryValue> {
        #[cfg(test)]
        crate::test_stack_budget::record();

        if depth > MAX_NBT_DEPTH {
            return Err(SnbtParseFailure::bound(self.cursor, "MAX_NBT_DEPTH"));
        }
        self.skip_whitespace();
        let Some(next) = self.peek() else {
            return Err(self.error("snbt.parser.expected_unquoted_string"));
        };
        match next {
            '{' => self.parse_compound(depth),
            '[' => self.parse_list_or_array(depth),
            '\'' | '"' => self
                .parse_quoted_string()
                .map(NbtSummaryValue::string_from_utf16),
            next if can_start_number(next) => self.parse_number_value(depth),
            _ => self.parse_unquoted_value(depth),
        }
    }

    fn record(&mut self, failure: SnbtParseFailure) {
        match &self.error_mark {
            Some(best) if best.cursor >= failure.cursor => {}
            _ => self.error_mark = Some(failure),
        }
    }

    fn fail_at(&mut self, cursor: usize, key: &'static str) {
        self.record(SnbtParseFailure::at(cursor, key));
    }

    /// `TerminalCharacters` names the characters it accepts joined by `|`
    /// (`StringReaderTerms.java:49-50`).
    fn fail_terminal(&mut self, cursor: usize, accepted: &str) {
        let accepted = accepted
            .chars()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("|");
        self.record(SnbtParseFailure::at_with(
            cursor,
            LITERAL_INCORRECT,
            accepted,
        ));
    }

    /// Records this failure and hands back the farthest one, which is the entry
    /// `Grammar.parseForCommands` throws even when a probe recorded it earlier.
    fn error_at(&mut self, failure: SnbtParseFailure) -> SnbtParseFailure {
        self.record(failure);
        self.error_mark.clone().expect("record leaves a mark")
    }

    /// `StringReaderTerms.TerminalCharacters`: whitespace is skipped *before* the
    /// character is read, so a terminator may match past spaces.
    fn terminal(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if self.consume(expected) {
            return true;
        }
        self.fail_terminal(self.cursor, &expected.to_string());
        false
    }

    fn terminal_any(&mut self, accepted: &str) -> bool {
        self.skip_whitespace();
        if self.peek().is_some_and(|value| accepted.contains(value)) {
            self.bump();
            return true;
        }
        self.fail_terminal(self.cursor, accepted);
        false
    }

    /// `NumberRunParseRule`: a greedy run of one radix's digits and `_`, rejected
    /// when it is empty or when an underscore touches either edge.
    fn number_run(&mut self, radix: u32) -> Option<String> {
        self.skip_whitespace();
        let start = self.cursor;
        while self
            .peek()
            .is_some_and(|value| is_radix_digit_or_underscore(value, radix))
        {
            self.bump();
        }
        let end = self.cursor;
        if end == start {
            self.fail_at(start, empty_numeral_key(radix));
            return None;
        }
        if self.input[start] == u16::from(b'_') || self.input[end - 1] == u16::from(b'_') {
            self.cursor = start;
            self.fail_at(start, "snbt.parser.underscore_not_allowed");
            return None;
        }
        Some(String::from_utf16(&self.input[start..end]).unwrap())
    }

    fn decimal_numeral(&mut self) -> Option<String> {
        self.number_run(10)
    }

    /// `sign`: `+` before `-`, each with the terminal's whitespace skip, so
    /// `- 1` is the integer `-1`.
    fn parse_sign(&mut self) -> Option<char> {
        let saved = self.cursor;
        if self.terminal('+') {
            return Some('+');
        }
        self.cursor = saved;
        if self.terminal('-') {
            return Some('-');
        }
        self.cursor = saved;
        None
    }

    fn float_exponent_part(&mut self) -> Option<(Option<char>, String)> {
        let saved = self.cursor;
        if !self.terminal_any("eE") {
            self.cursor = saved;
            return None;
        }
        let sign = self.parse_sign();
        let Some(digits) = self.decimal_numeral() else {
            self.cursor = saved;
            return None;
        };
        Some((sign, digits))
    }

    fn float_type_suffix(&mut self) -> Option<char> {
        let saved = self.cursor;
        if self.terminal_any("fF") {
            return Some('f');
        }
        self.cursor = saved;
        if self.terminal_any("dD") {
            return Some('d');
        }
        self.cursor = saved;
        None
    }

    fn integer_suffix_type(&mut self) -> Option<IntegerType> {
        let saved = self.cursor;
        for (accepted, value_type) in [
            ("bB", IntegerType::Byte),
            ("sS", IntegerType::Short),
            ("iI", IntegerType::Int),
            ("lL", IntegerType::Long),
        ] {
            if self.terminal_any(accepted) {
                return Some(value_type);
            }
            self.cursor = saved;
        }
        None
    }

    /// `integer_suffix`: `u|U` and `s|S` are two-character prefixes whose second
    /// half may fall back to the bare type letter, so `1sx` is the short `1s`.
    fn parse_integer_suffix(&mut self) -> IntegerSuffix {
        let saved = self.cursor;
        if self.terminal_any("uU") {
            if let Some(value_type) = self.integer_suffix_type() {
                return IntegerSuffix {
                    signed: Some(Signedness::Unsigned),
                    value_type: Some(value_type),
                };
            }
        }
        self.cursor = saved;
        if self.terminal_any("sS") {
            if let Some(value_type) = self.integer_suffix_type() {
                return IntegerSuffix {
                    signed: Some(Signedness::Signed),
                    value_type: Some(value_type),
                };
            }
        }
        self.cursor = saved;
        match self.integer_suffix_type() {
            Some(value_type) => IntegerSuffix {
                signed: None,
                value_type: Some(value_type),
            },
            None => {
                self.cursor = saved;
                IntegerSuffix::EMPTY
            }
        }
    }

    /// `float_literal`: four alternatives in source order, each restarting after
    /// the optional sign. Every part is probed with the grammar's whitespace skip
    /// so `1 . 5` is the double `1.5`, and the first alternative that fits decides
    /// the literal even when its value is rejected.
    fn parse_float_literal(&mut self, start: usize) -> Option<NbtSummaryValue> {
        self.cursor = start;
        let sign = self.parse_sign();
        let after_sign = self.cursor;

        // whole '.' [fraction] [exponent] [suffix]
        if let Some(whole) = self.decimal_numeral() {
            if self.terminal('.') {
                let fraction = self.decimal_numeral();
                let exponent = self.float_exponent_part();
                let suffix = self.float_type_suffix();
                let mark = self.cursor;
                return self.build_float(
                    sign,
                    Some(&whole),
                    fraction.as_deref(),
                    exponent,
                    suffix,
                    mark,
                );
            }
        }

        // '.' fraction [exponent] [suffix]
        self.cursor = after_sign;
        if self.terminal('.') {
            if let Some(fraction) = self.decimal_numeral() {
                let exponent = self.float_exponent_part();
                let suffix = self.float_type_suffix();
                let mark = self.cursor;
                return self.build_float(sign, None, Some(&fraction), exponent, suffix, mark);
            }
        }

        // whole exponent [suffix]
        self.cursor = after_sign;
        if let Some(whole) = self.decimal_numeral() {
            if let Some(exponent) = self.float_exponent_part() {
                let suffix = self.float_type_suffix();
                let mark = self.cursor;
                return self.build_float(sign, Some(&whole), None, Some(exponent), suffix, mark);
            }
        }

        // whole [exponent] suffix
        self.cursor = after_sign;
        if let Some(whole) = self.decimal_numeral() {
            let exponent = self.float_exponent_part();
            if let Some(suffix) = self.float_type_suffix() {
                let mark = self.cursor;
                return self.build_float(sign, Some(&whole), None, exponent, Some(suffix), mark);
            }
        }

        self.cursor = start;
        None
    }

    /// `createFloat`: the digits are reassembled without underscores, `f`/`F`
    /// narrows to `Float.parseFloat` and everything else is `Double.parseDouble`,
    /// and a non-finite result is a recorded failure rather than a value.
    fn build_float(
        &mut self,
        sign: Option<char>,
        whole: Option<&str>,
        fraction: Option<&str>,
        exponent: Option<(Option<char>, String)>,
        suffix: Option<char>,
        mark: usize,
    ) -> Option<NbtSummaryValue> {
        let mut text = String::new();
        if sign == Some('-') {
            text.push('-');
        }
        for (value, separator) in [(whole, None), (fraction, Some('.'))] {
            if let Some(value) = value {
                if let Some(separator) = separator {
                    text.push(separator);
                }
                text.extend(value.chars().filter(|value| *value != '_'));
            }
        }
        if let Some((sign, digits)) = exponent {
            text.push('e');
            if sign == Some('-') {
                text.push('-');
            }
            text.extend(digits.chars().filter(|value| *value != '_'));
        }
        if suffix == Some('f') {
            let value = text.parse::<f32>().ok();
            match value.filter(|value| value.is_finite()) {
                Some(value) => Some(NbtSummaryValue::Float(canonical_float_tag_bits(value))),
                None => {
                    self.fail_at(mark, "snbt.parser.infinity_not_allowed");
                    None
                }
            }
        } else {
            let value = text.parse::<f64>().ok();
            match value.filter(|value| value.is_finite()) {
                Some(value) => Some(NbtSummaryValue::Double(canonical_double_tag_bits(value))),
                None => {
                    self.fail_at(mark, "snbt.parser.infinity_not_allowed");
                    None
                }
            }
        }
    }

    /// `integer_literal`:
    /// `sign? ( '0' ↑ ( x|X ↑ hex | b|B binary | decimal ↑ fail | ε="0" ) | decimal ) suffix?`.
    ///
    /// The cut after `0` makes `01` a hard failure while the missing cut before
    /// `binary_numeral` still lets `0b` become the byte `0`; the cut after `x`/`X`
    /// makes `0xg` fail instead of falling back to the zero marker.
    fn parse_integer_literal(&mut self, start: usize) -> Option<IntegerLiteral> {
        self.cursor = start;
        let sign = self.parse_sign();
        let numeral_start = self.cursor;
        let (base, digits) = if self.terminal('0') {
            let after_zero = self.cursor;
            if self.terminal_any("xX") {
                (16, self.number_run(16)?)
            } else if self.terminal_any("bB") {
                match self.number_run(2) {
                    Some(digits) => (2, digits),
                    None => {
                        self.cursor = after_zero;
                        self.decimal_zero_or_leading_zero(after_zero)?
                    }
                }
            } else {
                self.decimal_zero_or_leading_zero(after_zero)?
            }
        } else {
            self.cursor = numeral_start;
            (10, self.number_run(10)?)
        };
        let suffix = self.parse_integer_suffix();
        Some(IntegerLiteral {
            sign,
            base,
            digits,
            suffix,
        })
    }

    /// The last two alternatives under the `0` cut: a valid decimal run after the
    /// zero is the recorded leading-zero failure, anything else falls into the
    /// `marker(decimal_numeral, "0")` that makes the literal a plain zero.
    fn decimal_zero_or_leading_zero(&mut self, after_zero: usize) -> Option<(u32, String)> {
        if self.decimal_numeral().is_some() {
            self.fail_at(self.cursor, "snbt.parser.leading_zero_not_allowed");
            return None;
        }
        self.cursor = after_zero;
        Some((10, "0".to_string()))
    }

    /// The number half of `literal`.
    ///
    /// `integer_literal` only *builds* the literal; `literal`'s own action is what
    /// converts it (`integer.create(ops, state)`), so a value the conversion
    /// rejects still counts as a matched term: the `unquoted_string_or_builtin`
    /// alternative never runs and the cursor stays after the literal. Only a term
    /// that fails outright reaches that alternative, which then rejects the
    /// number-start token but still advances the error cursor to the end of the
    /// unquoted run it reads.
    fn parse_number_value(&mut self, depth: usize) -> SnbtResult<NbtSummaryValue> {
        let start = self.cursor;
        self.error_mark = None;
        if let Some(value) = self.parse_float_literal(start) {
            return Ok(value);
        }
        self.cursor = start;
        if let Some(literal) = self.parse_integer_literal(start) {
            let mark = self.cursor;
            return match literal.to_value(literal.suffix.value_type.unwrap_or(IntegerType::Int)) {
                Ok(value) => Ok(value),
                Err(failure) => Err(self.error_at(failure.at(mark))),
            };
        }
        self.cursor = start;
        self.note_unquoted_fallback(depth);
        Err(self.error_at(SnbtParseFailure::at(
            start,
            "snbt.parser.expected_unquoted_string",
        )))
    }

    /// `unquoted_string_or_builtin` still runs for a number-start token. It reads
    /// the unquoted run, probes the optional `(...)` group -- which can fail far
    /// past the run and take the longest-match mark with it -- and only then
    /// rejects the token for starting like a number.
    fn note_unquoted_fallback(&mut self, depth: usize) {
        let identifier = self.read_unquoted();
        if identifier.is_empty() {
            self.fail_at(self.cursor, "snbt.parser.expected_unquoted_string");
            return;
        }
        let end_of_run = self.cursor;
        let mut mark = end_of_run;
        if self.terminal('(') {
            match self.parse_arguments(depth) {
                Ok(_) => mark = self.cursor,
                Err(_) => self.cursor = end_of_run,
            }
        }
        self.cursor = end_of_run;
        self.fail_at(mark, "snbt.parser.invalid_unquoted_start");
    }

    fn parse_compound(&mut self, depth: usize) -> SnbtResult<NbtSummaryValue> {
        self.expect('{')?;
        let mut entries = super::CompoundEntries::new(Vec::new());
        let mut entry_count = 0usize;
        self.skip_whitespace();
        if self.consume('}') {
            return Ok(NbtSummaryValue::Compound(entries.entries));
        }
        loop {
            if entry_count >= MAX_NBT_COMPOUND_ENTRIES {
                return Err(SnbtParseFailure::bound(
                    self.cursor,
                    "MAX_NBT_COMPOUND_ENTRIES",
                ));
            }
            entry_count += 1;
            let name = self.parse_key()?;
            self.expect(':')?;
            let value = self.parse_value(depth + 1)?;
            // `mapEntry`'s action rejects the empty key only after the value, so
            // the reported position is the value's end and not the key's.
            if name.is_empty() {
                return Err(self.error("snbt.parser.empty_key"));
            }
            entries.push(NbtSummaryEntry { name, value });
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
        Ok(NbtSummaryValue::Compound(entries.entries))
    }

    /// `map_key` (SnbtGrammar.java:581-585) is the same string alternative the
    /// value side uses, so a quoted key keeps its units.
    fn parse_key(&mut self) -> SnbtResult<JavaString> {
        self.skip_whitespace();
        match self.peek() {
            Some('\'' | '"') => Ok(self.parse_quoted_string()?.into()),
            _ => {
                let value = self.read_unquoted();
                if value.is_empty() {
                    // The quoted literal is tried first, so its terminal is the
                    // entry stored here.
                    self.fail_terminal(self.cursor, "\"");
                    Err(self.error("snbt.parser.expected_unquoted_string"))
                } else {
                    Ok(value.as_str().into())
                }
            }
        }
    }

    fn parse_list_or_array(&mut self, depth: usize) -> SnbtResult<NbtSummaryValue> {
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
                return Err(SnbtParseFailure::bound(self.cursor, "MAX_NBT_LIST_ITEMS"));
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

    /// `array_prefix` (SnbtGrammar.java:626-634) is an alternative of `B`, `L`
    /// and `I`, so a tail that is none of them still stores the first one.
    fn array_prefix(&mut self) -> Option<IntegerType> {
        let saved = self.cursor;
        let array_type = match self.peek() {
            Some('B') => IntegerType::Byte,
            Some('I') => IntegerType::Int,
            Some('L') => IntegerType::Long,
            _ => {
                self.fail_terminal(saved, "B");
                return None;
            }
        };
        self.bump();
        if self.terminal(';') {
            Some(array_type)
        } else {
            self.cursor = saved;
            None
        }
    }

    fn parse_array(&mut self, array_type: IntegerType) -> SnbtResult<NbtSummaryValue> {
        let mut literals = Vec::new();
        self.skip_whitespace();
        if self.consume(']') {
            return Ok(array_value(array_type, Vec::new()));
        }
        loop {
            if literals.len() >= MAX_NBT_ARRAY_ITEMS {
                return Err(SnbtParseFailure::bound(self.cursor, "MAX_NBT_ARRAY_ITEMS"));
            }
            self.error_mark = None;
            let start = self.cursor;
            let Some(literal) = self.parse_integer_literal(start) else {
                return Err(self.error_at(SnbtParseFailure::at(
                    start,
                    "snbt.parser.expected_decimal_numeral",
                )));
            };
            literals.push(literal);
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
        // `listLiteral`'s action builds the array after `]` is consumed, so an
        // element that is out of range or of the wrong type is reported at the
        // closing bracket rather than at the element.
        let mark = self.cursor;
        let mut values = Vec::with_capacity(literals.len());
        for literal in literals {
            let actual_type = match literal.suffix.value_type {
                None => array_type,
                Some(actual) if array_type.accepts_element(actual) => actual,
                Some(_) => {
                    return Err(self.error_at(SnbtParseFailure::at(
                        mark,
                        "snbt.parser.invalid_array_element_type",
                    )));
                }
            };
            values.push(
                literal
                    .to_value(actual_type)
                    .map_err(|failure| self.error_at(failure.at(mark)))?,
            );
        }
        Ok(array_value(array_type, values))
    }

    fn parse_unquoted_value(&mut self, depth: usize) -> SnbtResult<NbtSummaryValue> {
        let identifier = self.read_unquoted();
        if identifier.is_empty() {
            return Err(self.error("snbt.parser.expected_unquoted_string"));
        }
        if can_start_number(identifier.chars().next().unwrap()) {
            return Err(self.error("snbt.parser.invalid_unquoted_start"));
        }
        // The `(...)` suffix is optional, so a failed probe leaves the cursor at
        // the end of the run: `a b` reads back as `a` at unit 1.
        let end_of_run = self.cursor;
        if !self.terminal('(') {
            self.cursor = end_of_run;
        } else {
            let arguments = self.parse_arguments(depth)?;
            return self.apply_builtin(&identifier, arguments);
        }
        if identifier.eq_ignore_ascii_case("true") {
            Ok(NbtSummaryValue::Byte(1))
        } else if identifier.eq_ignore_ascii_case("false") {
            Ok(NbtSummaryValue::Byte(0))
        } else {
            Ok(NbtSummaryValue::String(identifier.to_string().into()))
        }
    }

    fn parse_arguments(&mut self, depth: usize) -> SnbtResult<Vec<NbtSummaryValue>> {
        let mut arguments = Vec::new();
        self.skip_whitespace();
        if self.consume(')') {
            return Ok(arguments);
        }
        loop {
            if arguments.len() >= MAX_NBT_LIST_ITEMS {
                return Err(SnbtParseFailure::bound(self.cursor, "MAX_NBT_LIST_ITEMS"));
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
        &mut self,
        identifier: &str,
        arguments: Vec<NbtSummaryValue>,
    ) -> SnbtResult<NbtSummaryValue> {
        if arguments.len() != 1 {
            return Err(self.no_such_operation(identifier, arguments.len()));
        }
        let argument = arguments.into_iter().next().unwrap();
        match identifier {
            "bool" => numeric_truth(&argument)
                .map(|value| NbtSummaryValue::Byte(i8::from(value)))
                .ok_or_else(|| self.error("snbt.parser.expected_number_or_boolean")),
            "uuid" => {
                let NbtSummaryValue::String(raw) = argument else {
                    return Err(self.error("snbt.parser.expected_string_uuid"));
                };
                let uuid = Uuid::parse_str(
                    &raw.try_to_string()
                        .map_err(|_| self.error("snbt.parser.expected_string_uuid"))?,
                )
                .map_err(|_| self.error("snbt.parser.expected_string_uuid"))?;
                let value = uuid.as_u128();
                Ok(NbtSummaryValue::IntArray(vec![
                    (value >> 96) as u32 as i32,
                    (value >> 64) as u32 as i32,
                    (value >> 32) as u32 as i32,
                    value as u32 as i32,
                ]))
            }
            _ => Err(self.no_such_operation(identifier, 1)),
        }
    }

    /// `SnbtGrammar`'s builtin arm (`:565-570`) names the operation it could not
    /// find as `name/arity`, which is `BuiltinKey.toString`.
    fn no_such_operation(&mut self, identifier: &str, arity: usize) -> SnbtParseFailure {
        self.error_at(SnbtParseFailure::at_with(
            self.cursor,
            "snbt.parser.no_such_operation",
            format!("{identifier}/{arity}"),
        ))
    }

    fn parse_quoted_string(&mut self) -> SnbtResult<Vec<u16>> {
        self.skip_whitespace();
        let quote = self.bump().unwrap();
        debug_assert!(matches!(quote, '\'' | '"'));
        let mut output = Vec::new();
        loop {
            let Some(next) = self.bump_unit() else {
                return Err(self.error("snbt.parser.invalid_string_contents"));
            };
            if next == quote as u16 {
                return Ok(output);
            }
            if next != u16::from(b'\\') {
                output.push(next);
                continue;
            }
            self.skip_whitespace();
            // `string_escape_sequence` tries its characters in source order, so
            // the first failure is recorded at the escape character itself.
            let escape_index = self.cursor;
            let Some(escape) = self.bump() else {
                return Err(self.escape_not_taken(escape_index));
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
                _ => return Err(self.escape_not_taken(escape_index)),
            };
            append_java_codepoint(&mut output, codepoint);
        }
    }

    /// `string_escape_sequence` (SnbtGrammar.java:450-490) is an alternative of
    /// single-character terminals in source order, so the first one stored --
    /// `b` -- is the entry that reaches the player.
    fn escape_not_taken(&mut self, escape_index: usize) -> SnbtParseFailure {
        self.fail_terminal(escape_index, "b");
        self.error_mark.clone().expect("record leaves a mark")
    }

    fn parse_named_unicode_escape(&mut self) -> SnbtResult<u32> {
        self.expect('{')?;
        let start = self.cursor;
        while self.peek().is_some_and(is_unicode_name_character) {
            self.bump();
        }
        let name = String::from_utf16(&self.input[start..self.cursor]).unwrap();
        if name.is_empty() {
            return Err(self.error("snbt.parser.invalid_character_name"));
        }
        self.expect('}')?;
        codepoint_by_name(&name).ok_or_else(|| self.error("snbt.parser.invalid_character_name"))
    }

    /// `SimpleHexLiteralParseRule`: a fixed-length run of hex digits with no
    /// whitespace skip, reported at the run's first digit.
    fn parse_hex_escape(&mut self, digits: usize) -> SnbtResult<u32> {
        let start = self.cursor;
        for _ in 0..digits {
            if !self.peek().is_some_and(|value| value.is_ascii_hexdigit()) {
                return Err(self.error_at(SnbtParseFailure::at_with(
                    start,
                    "snbt.parser.expected_hex_escape",
                    digits.to_string(),
                )));
            }
            self.bump();
        }
        let mark = self.cursor;
        let raw = String::from_utf16(&self.input[start..self.cursor]).unwrap();
        let codepoint = u32::from_str_radix(&raw, 16).unwrap();
        if codepoint <= 0x10ffff {
            Ok(codepoint)
        } else {
            Err(self.error_at(SnbtParseFailure::at_with(
                mark,
                "snbt.parser.invalid_codepoint",
                format!("U+{codepoint:08X}"),
            )))
        }
    }

    fn expect(&mut self, expected: char) -> SnbtResult<()> {
        if self.terminal(expected) {
            Ok(())
        } else {
            Err(self.error_mark.clone().expect("terminal leaves a mark"))
        }
    }

    fn read_unquoted(&mut self) -> String {
        self.skip_whitespace();
        let start = self.cursor;
        while self.peek().is_some_and(is_unquoted_character) {
            self.bump();
        }
        String::from_utf16(&self.input[start..self.cursor]).unwrap()
    }

    fn skip_whitespace(&mut self) {
        while self
            .input
            .get(self.cursor)
            .is_some_and(|unit| crate::java_string::java_character_is_whitespace(*unit))
        {
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
        self.input
            .get(self.cursor)
            .map(|&unit| char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER))
    }

    fn bump(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.cursor += 1;
        Some(value)
    }

    fn bump_unit(&mut self) -> Option<u16> {
        let value = *self.input.get(self.cursor)?;
        self.cursor += 1;
        Some(value)
    }

    fn is_empty(&self) -> bool {
        self.cursor == self.input.len()
    }

    fn error(&mut self, key: &'static str) -> SnbtParseFailure {
        let cursor = self.cursor;
        self.error_at(SnbtParseFailure::at(cursor, key))
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
    fn bits(self) -> u32 {
        match self {
            Self::Byte => 8,
            Self::Short => 16,
            Self::Int => 32,
            Self::Long => 64,
        }
    }

    /// `ArrayPrefix.computeType`: an element without a suffix takes the array's
    /// own type, and a suffixed one must be in the array's allowed set.
    fn accepts_element(self, actual: Self) -> bool {
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

#[derive(Debug, Clone, Copy)]
struct IntegerSuffix {
    signed: Option<Signedness>,
    value_type: Option<IntegerType>,
}

impl IntegerSuffix {
    const EMPTY: Self = Self {
        signed: None,
        value_type: None,
    };
}

/// One parsed `integer_literal`, kept raw so an array element can be checked
/// against the array's element type into which it is later converted.
#[derive(Debug, Clone)]
struct IntegerLiteral {
    sign: Option<char>,
    base: u32,
    digits: String,
    suffix: IntegerSuffix,
}

impl IntegerLiteral {
    fn signedness(&self) -> Signedness {
        self.suffix.signed.unwrap_or(if self.base == 10 {
            Signedness::Signed
        } else {
            Signedness::Unsigned
        })
    }

    /// `IntegerLiteral.create`: a decimal literal is signed and a `0x`/`0b` one is
    /// unsigned unless the suffix says otherwise, and an unsigned literal may not
    /// carry a minus sign.
    fn to_value(
        &self,
        value_type: IntegerType,
    ) -> std::result::Result<NbtSummaryValue, SnbtValueFailure> {
        let signedness = self.signedness();
        let negative = self.sign == Some('-');
        if signedness == Signedness::Unsigned && negative {
            return Err(SnbtValueFailure::plain(
                "snbt.parser.expected_non_negative_number",
            ));
        }
        // `cleanupDigits` (SnbtGrammar.java:809-819) removes `_` and keeps only
        // a minus sign; that text is what the JDK parser quotes back.
        let digits = self.digits.replace('_', "");
        let magnitude = u128::from_str_radix(&digits, self.base).ok();
        if let Some(value) = magnitude
            .and_then(|magnitude| integer_value(value_type, signedness, negative, magnitude))
        {
            return Ok(value);
        }
        let spelled = if negative {
            format!("-{digits}")
        } else {
            digits
        };
        Err(SnbtValueFailure::with(
            "snbt.parser.number_parse_failure",
            number_parse_failure(
                &spelled, self.base, magnitude, negative, signedness, value_type,
            ),
        ))
    }
}

/// One `IntegerLiteral.create` refusal (`SnbtGrammar.java:825-867`). The grammar
/// stores it at the literal's end, which the literal itself does not know.
struct SnbtValueFailure {
    key: &'static str,
    argument: Option<String>,
}

impl SnbtValueFailure {
    fn plain(key: &'static str) -> Self {
        Self {
            key,
            argument: None,
        }
    }

    fn with(key: &'static str, argument: String) -> Self {
        Self {
            key,
            argument: Some(argument),
        }
    }

    fn at(self, cursor: usize) -> SnbtParseFailure {
        SnbtParseFailure {
            cursor,
            key: Some(self.key),
            arguments: self.argument.into_iter().collect(),
        }
    }
}

/// The text `snbt.parser.number_parse_failure` carries is the JDK
/// `NumberFormatException` message of the call `IntegerLiteral.create` made, so
/// which call it was decides the wording: the narrow types go through
/// `Integer.parseInt` first, and only the unsigned `int`/`long` parsers name the
/// range they exceeded.
fn number_parse_failure(
    spelled: &str,
    radix: u32,
    magnitude: Option<u128>,
    negative: bool,
    signedness: Signedness,
    value_type: IntegerType,
) -> String {
    let fits_int = magnitude.is_some_and(|magnitude| {
        if negative {
            magnitude <= 1u128 << 31
        } else {
            magnitude <= i32::MAX as u128
        }
    });
    let for_input_string = || {
        let radix_suffix = if radix == 10 {
            String::new()
        } else {
            format!(" under radix {radix}")
        };
        format!("For input string: \"{spelled}\"{radix_suffix}")
    };
    match (signedness, value_type) {
        (_, IntegerType::Byte | IntegerType::Short) if !fits_int => for_input_string(),
        (Signedness::Signed, IntegerType::Byte | IntegerType::Short) => {
            format!("Value out of range. Value:\"{spelled}\" Radix:{radix}")
        }
        (Signedness::Signed, _) => for_input_string(),
        (Signedness::Unsigned, IntegerType::Byte | IntegerType::Short) => {
            format!("out of range: {}", magnitude.unwrap_or_default())
        }
        (Signedness::Unsigned, IntegerType::Int) => {
            format!("String value {spelled} exceeds range of unsigned int.")
        }
        (Signedness::Unsigned, IntegerType::Long) => {
            format!("String value {spelled} exceeds range of unsigned long.")
        }
    }
}

/// `NumberRunParseRule`'s empty-run error, one per radix
/// (`SnbtGrammar.java:101-127`).
const fn empty_numeral_key(radix: u32) -> &'static str {
    match radix {
        2 => "snbt.parser.expected_binary_numeral",
        16 => "snbt.parser.expected_hex_numeral",
        _ => "snbt.parser.expected_decimal_numeral",
    }
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

fn is_radix_digit_or_underscore(value: char, radix: u32) -> bool {
    value == '_' || value.is_digit(radix)
}

fn is_unquoted_character(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '_' | '-' | '.' | '+')
}

fn is_unicode_name_character(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '-' | ' ')
}

// SnbtGrammar.java:470-478 accepts surrogate codepoints as individual units.
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
    /// `map_key` (SnbtGrammar.java:581-585) is the same quoted/unquoted string
    /// alternative a value uses, so a key holds any unit. Measured against the
    /// 26.1 server jar: `TagParser.parseCompoundFully("{\'a<D800>\':1}")` reads
    /// the key `0061 D800`, and re-reading what that compound prints gives the
    /// same key back.
    #[test]
    fn an_snbt_compound_key_is_the_string_alternative_a_value_is() {
        let key: Vec<u16> = "a".encode_utf16().chain([0xd800]).collect();
        let source: Vec<u16> = "{'"
            .encode_utf16()
            .chain(key.iter().copied())
            .chain("':1}".encode_utf16())
            .collect();
        let expected = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: key.clone().into(),
            value: NbtSummaryValue::Int(1),
        }]);
        assert_eq!(parse_snbt_compound_utf16(&source).unwrap(), expected);
        assert_eq!(
            parse_snbt_compound_prefix_utf16(&source).unwrap(),
            (source.len(), expected.clone())
        );
        let printed = crate::nbt_summary::canonical_snbt_string(&expected);
        assert_eq!(parse_snbt_compound(&printed).unwrap(), expected);
    }

    #[test]
    fn raw_snbt_argument_prefix_keeps_units_and_leaves_arbitrary_suffix_unread() {
        let prefix: Vec<u16> = "{value:'"
            .encode_utf16()
            .chain([0xd800, 65, 0xdc00])
            .chain("'}".encode_utf16())
            .collect();
        for suffix in [vec![32, 0xdc00, 123], vec![93], vec![0xd800]] {
            let input: Vec<u16> = prefix.iter().copied().chain(suffix).collect();
            let (consumed, value) = parse_snbt_compound_prefix_utf16(&input).unwrap();
            assert_eq!(consumed, prefix.len());
            assert_eq!(
                entry(&value, "value"),
                &NbtSummaryValue::string_from_utf16(vec![0xd800, 65, 0xdc00])
            );
            assert!(parse_snbt_compound_utf16(&input).is_err());
        }
        for (text, expected) in [
            ("42 trailing", NbtSummaryValue::Int(42)),
            ("'😀' trailing", NbtSummaryValue::String("😀".into())),
        ] {
            let units: Vec<_> = text.encode_utf16().collect();
            let (consumed, value) = parse_snbt_prefix_utf16(&units).unwrap();
            assert_eq!(value, expected);
            assert_eq!(units[consumed], 32);
            assert!(parse_snbt_compound_prefix_utf16(&units).is_err());
            let (bytes, value) = parse_snbt_prefix(text).unwrap();
            assert_eq!(value, expected);
            assert_eq!(text[..bytes].encode_utf16().count(), consumed);
        }
    }

    #[test]
    fn raw_snbt_argument_prefix_joins_raw_and_escaped_key_units() {
        for (before, unit, after) in [
            ("{\"", 0xd800, "\\uDC00\":1}"),
            ("{\"\\uD800", 0xdc00, "\":1}"),
        ] {
            let mut input: Vec<u16> = before
                .encode_utf16()
                .chain([unit])
                .chain(after.encode_utf16())
                .collect();
            let end = input.len();
            input.extend([32, 0xd800]);
            let (consumed, value) = parse_snbt_compound_prefix_utf16(&input).unwrap();
            assert_eq!(consumed, end);
            assert_eq!(entry(&value, "𐀀"), &NbtSummaryValue::Int(1));
        }
    }

    #[test]
    fn raw_snbt_quotes_join_raw_and_escaped_surrogates_before_key_validation() {
        for (prefix, unit, suffix) in [
            ("{\"", 0xd800, "\\uDC00\":1}"),
            ("{\"\\uD800", 0xdc00, "\":1}"),
        ] {
            let units = prefix
                .encode_utf16()
                .chain([unit])
                .chain(suffix.encode_utf16())
                .collect::<Vec<_>>();
            let value = parse_snbt_compound_utf16(&units).unwrap();
            assert_eq!(entry(&value, "𐀀"), &NbtSummaryValue::Int(1));
        }
        let units = "{value:'"
            .encode_utf16()
            .chain([0xd800, 65, 0xdc00])
            .chain("'}".encode_utf16())
            .collect::<Vec<_>>();
        let value = parse_snbt_compound_utf16(&units).unwrap();
        assert_eq!(
            entry(&value, "value"),
            &NbtSummaryValue::string_from_utf16(vec![0xd800, 65, 0xdc00])
        );
        let text = "{value:'😀'} remaining";
        assert_eq!(
            parse_snbt_compound_prefix(text).unwrap().0,
            "{value:'😀'}".len()
        );
    }

    #[test]
    fn raw_snbt_whitespace_uses_java_character_rules() {
        for accepted in ['\u{001c}', '\u{2000}', '\u{3000}'] {
            assert!(parse_snbt_compound(&format!("{accepted}{{value:1}}{accepted}")).is_ok());
        }
        for rejected in ['\u{0085}', '\u{00a0}', '\u{2007}', '\u{202f}'] {
            assert!(parse_snbt_compound(&format!("{rejected}{{value:1}}")).is_err());
        }
    }

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
    fn compound_prefix_leaves_caller_owned_suffix_unread() {
        let (consumed, value) = parse_snbt_compound_prefix("{axis:'x'} trailing").unwrap();
        assert_eq!(&"{axis:'x'} trailing"[consumed..], " trailing");
        assert_eq!(entry(&value, "axis"), &NbtSummaryValue::String("x".into()));
        assert!(parse_snbt_compound_prefix("{unterminated:").is_err());
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
    fn byte_zero_suffix_wins_when_binary_prefix_has_no_numeral() {
        let value =
            parse_snbt_compound("{zero:0b,negative:-0B,binary:0b1,binary_byte:0b1b}").unwrap();
        assert_eq!(entry(&value, "zero"), &NbtSummaryValue::Byte(0));
        assert_eq!(entry(&value, "negative"), &NbtSummaryValue::Byte(0));
        assert_eq!(entry(&value, "binary"), &NbtSummaryValue::Int(1));
        assert_eq!(entry(&value, "binary_byte"), &NbtSummaryValue::Byte(1));
    }

    #[test]
    fn parses_float_shapes_and_finite_suffixes() {
        let value = parse_snbt_compound("{a:.5f,b:1.,c:-2e3,d:4E-2d,e:-0.0f,f:-0.0d}").unwrap();
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
        assert_eq!(
            entry(&value, "e"),
            &NbtSummaryValue::Float(0.0_f32.to_bits())
        );
        assert_eq!(
            entry(&value, "f"),
            &NbtSummaryValue::Double(0.0_f64.to_bits())
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
    fn preserves_unpaired_utf16_surrogate_escapes() {
        for (input, expected) in [
            (r#"{value:"\uD800"}"#, vec![0xd800]),
            (r#"{value:"\uDC00"}"#, vec![0xdc00]),
            (r#"{value:"\U0000D800"}"#, vec![0xd800]),
            (r#"{value:"\U0000DC00"}"#, vec![0xdc00]),
            (r#"{value:"\N{HIGH SURROGATES D800}"}"#, vec![0xd800]),
            (r#"{value:"\N{LOW SURROGATES DC00}"}"#, vec![0xdc00]),
            (r#"{value:"\uD800A"}"#, vec![0xd800, 65]),
            (r#"{value:"A\uDC00"}"#, vec![65, 0xdc00]),
            (r#"{value:"\uD800\uD801"}"#, vec![0xd800, 0xd801]),
            (r#"{value:"\uDC00\uD800"}"#, vec![0xdc00, 0xd800]),
        ] {
            let value = parse_snbt_compound(input).unwrap();
            assert_eq!(
                entry(&value, "value"),
                &NbtSummaryValue::string_from_utf16(expected)
            );
            let encoded = super::super::canonical_snbt_string(&value);
            assert_eq!(parse_snbt_compound(&encoded).unwrap(), value);
        }
    }

    #[test]
    fn rejects_unicode_escapes_outside_the_java_codepoint_range() {
        for input in [r#"{value:"\U00110000"}"#, r#"{value:"\UFFFFFFFF"}"#] {
            let failure = parse_snbt_compound(input).unwrap_err();
            assert_eq!(
                failure.key,
                Some("snbt.parser.invalid_codepoint"),
                "{input}"
            );
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
                let failure = parse_snbt_compound(&rejected).unwrap_err();
                assert_eq!(failure.key, None);
                assert_eq!(failure.arguments, ["MAX_NBT_DEPTH"]);

                assert_eq!(
                    parse_snbt_compound("{recovered:1}").unwrap(),
                    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "recovered".into(),
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
