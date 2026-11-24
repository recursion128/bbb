use super::NbtSummaryValue;
use crate::component::{java_double_to_string, java_float_to_string};
use crate::java_string::JavaString;

/// Formats NBT with Vanilla's quoting; unpaired UTF-16 uses roundtrippable
/// Unicode escapes because Rust UTF-8 cannot hold Vanilla's raw surrogate output.
pub fn canonical_snbt_string(value: &NbtSummaryValue) -> String {
    let mut output = String::new();
    write_snbt(value, &mut output);
    output
}

/// Vanilla `StringTagVisitor` output with every Java UTF-16 code unit intact.
pub fn canonical_snbt_java_string(value: &NbtSummaryValue) -> JavaString {
    let mut output = Vec::<u16>::new();
    write_snbt(value, &mut output);
    output.into()
}

/// Formats a compound key with roundtrippable escapes for unpaired UTF-16.
pub fn canonical_snbt_key_units(value: &JavaString) -> String {
    let mut output = String::new();
    write_key_units(value, &mut output);
    output
}

/// [`canonical_snbt_key_units`] with the key's raw Java UTF-16 output.
pub fn canonical_snbt_key_java_string(value: &JavaString) -> JavaString {
    let mut output = Vec::<u16>::new();
    write_key_units(value, &mut output);
    output.into()
}

trait SnbtOutput {
    fn push(&mut self, value: char);
    fn push_str(&mut self, value: &str);
    fn push_unpaired(&mut self, unit: u16);
    fn start_quote(&mut self) -> usize;
    fn finish_quote(&mut self, position: usize, quote: u8);
}

impl SnbtOutput for Vec<u16> {
    fn push(&mut self, value: char) {
        let mut units = [0; 2];
        self.extend(value.encode_utf16(&mut units).iter().copied());
    }

    fn push_str(&mut self, value: &str) {
        self.extend(value.encode_utf16());
    }

    fn push_unpaired(&mut self, unit: u16) {
        Vec::push(self, unit);
    }

    fn start_quote(&mut self) -> usize {
        let position = self.len();
        Vec::push(self, u16::from(b' '));
        position
    }

    fn finish_quote(&mut self, position: usize, quote: u8) {
        self[position] = u16::from(quote);
        Vec::push(self, u16::from(quote));
    }
}

impl SnbtOutput for String {
    fn push(&mut self, value: char) {
        String::push(self, value);
    }

    fn push_str(&mut self, value: &str) {
        String::push_str(self, value);
    }

    fn push_unpaired(&mut self, unit: u16) {
        String::push_str(self, "\\u");
        for shift in [12, 8, 4, 0] {
            String::push(self, hex_digit((unit >> shift) as u8));
        }
    }

    fn start_quote(&mut self) -> usize {
        let position = self.len();
        String::push(self, ' ');
        position
    }

    fn finish_quote(&mut self, position: usize, quote: u8) {
        self.replace_range(
            position..position + 1,
            if quote == b'"' { "\"" } else { "'" },
        );
        String::push(self, char::from(quote));
    }
}

fn write_snbt<O: SnbtOutput>(value: &NbtSummaryValue, output: &mut O) {
    match value {
        NbtSummaryValue::Byte(value) => {
            output.push_str(&value.to_string());
            output.push('b');
        }
        NbtSummaryValue::Short(value) => {
            output.push_str(&value.to_string());
            output.push('s');
        }
        NbtSummaryValue::Int(value) => output.push_str(&value.to_string()),
        NbtSummaryValue::Long(value) => {
            output.push_str(&value.to_string());
            output.push('L');
        }
        NbtSummaryValue::Float(bits) => {
            output.push_str(&java_float_to_string(f32::from_bits(*bits)));
            output.push('f');
        }
        NbtSummaryValue::Double(bits) => {
            output.push_str(&java_double_to_string(f64::from_bits(*bits)));
            output.push('d');
        }
        NbtSummaryValue::ByteArray(values) => {
            output.push_str("[B;");
            write_joined(values, output, |value, output| {
                output.push_str(&value.to_string());
                output.push('B');
            });
            output.push(']');
        }
        NbtSummaryValue::String(value) => write_quoted_units(value.units().iter().copied(), output),
        NbtSummaryValue::List(values) => {
            output.push('[');
            write_joined(values, output, write_snbt);
            output.push(']');
        }
        NbtSummaryValue::Compound(entries) => {
            output.push('{');
            super::with_last_wins_compound_entries(entries, |entries| {
                entries.sort_by(|left, right| left.name.units().cmp(right.name.units()));
                write_joined(entries, output, |entry, output| {
                    write_key_units(&entry.name, output);
                    output.push(':');
                    write_snbt(&entry.value, output);
                });
            });
            output.push('}');
        }
        NbtSummaryValue::IntArray(values) => {
            output.push_str("[I;");
            write_joined(values, output, |value, output| {
                output.push_str(&value.to_string());
            });
            output.push(']');
        }
        NbtSummaryValue::LongArray(values) => {
            output.push_str("[L;");
            write_joined(values, output, |value, output| {
                output.push_str(&value.to_string());
                output.push('L');
            });
            output.push(']');
        }
    }
}

fn write_joined<T, O: SnbtOutput>(
    values: &[T],
    output: &mut O,
    mut write_value: impl FnMut(&T, &mut O),
) {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write_value(value, output);
    }
}

fn write_key_units<O: SnbtOutput>(value: &JavaString, output: &mut O) {
    let units = value.units();
    if is_unquoted_key(units) {
        for &unit in units {
            output.push(char::from(unit as u8));
        }
    } else {
        write_quoted_units(units.iter().copied(), output);
    }
}

fn is_unquoted_key(units: &[u16]) -> bool {
    let mut chars = units.iter().copied();
    if !chars.next().is_some_and(is_unquoted_key_start) || !chars.all(is_unquoted_key_continuation)
    {
        return false;
    }
    let lower = units.iter().map(|&unit| (unit as u8).to_ascii_lowercase());
    !lower.clone().eq(*b"true") && !lower.eq(*b"false")
}

fn is_unquoted_key_start(value: u16) -> bool {
    u8::try_from(value)
        .is_ok_and(|value| value.is_ascii_alphabetic() || matches!(value, b'.' | b'_'))
}

fn is_unquoted_key_continuation(value: u16) -> bool {
    u8::try_from(value).is_ok_and(|value| {
        value.is_ascii_alphanumeric() || matches!(value, b'.' | b'_' | b'+' | b'-')
    })
}

fn write_quoted_units<O: SnbtOutput>(units: impl Iterator<Item = u16>, output: &mut O) {
    let quote_position = output.start_quote();
    let mut quote = None;
    for decoded in char::decode_utf16(units) {
        let value = match decoded {
            Ok(value) => value,
            Err(error) => {
                output.push_unpaired(error.unpaired_surrogate());
                continue;
            }
        };
        match value {
            '\\' => output.push_str("\\\\"),
            '"' | '\'' => {
                let selected = *quote.get_or_insert(if value == '"' { b'\'' } else { b'"' });
                if char::from(selected) == value {
                    output.push('\\');
                }
                output.push(value);
            }
            '\u{0008}' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\u{000c}' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            value if value < ' ' => {
                output.push_str("\\x");
                output.push(hex_digit((value as u8) >> 4));
                output.push(hex_digit(value as u8));
            }
            value => output.push(value),
        }
    }
    let quote = quote.unwrap_or(b'"');
    output.finish_quote(quote_position, quote);
}

fn hex_digit(value: u8) -> char {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    char::from(HEX[usize::from(value & 0x0f)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NbtSummaryEntry;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn formats_all_tag_shapes_like_string_tag_visitor() {
        let value = NbtSummaryValue::Compound(vec![
            entry("z", NbtSummaryValue::Byte(-1)),
            entry("short", NbtSummaryValue::Short(2)),
            entry("int", NbtSummaryValue::Int(3)),
            entry("long", NbtSummaryValue::Long(4)),
            entry("float", NbtSummaryValue::Float(0.5_f32.to_bits())),
            entry("double", NbtSummaryValue::Double(0.25_f64.to_bits())),
            entry("bytes", NbtSummaryValue::ByteArray(vec![-1, 2])),
            entry("ints", NbtSummaryValue::IntArray(vec![3, 4])),
            entry("longs", NbtSummaryValue::LongArray(vec![5, 6])),
            entry(
                "list",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("value".into()),
                    NbtSummaryValue::Compound(Vec::new()),
                ]),
            ),
        ]);

        assert_eq!(
            canonical_snbt_string(&value),
            r#"{bytes:[B;-1B,2B],double:0.25d,float:0.5f,int:3,ints:[I;3,4],list:["value",{}],long:4L,longs:[L;5L,6L],short:2s,z:-1b}"#
        );
        assert_eq!(
            canonical_snbt_java_string(&value).units(),
            canonical_snbt_string(&value)
                .encode_utf16()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn matches_java_key_order_and_string_escaping() {
        let value = NbtSummaryValue::Compound(vec![
            entry("\u{e000}", NbtSummaryValue::Int(2)),
            entry("TrUe", NbtSummaryValue::String("a\"b'c".into())),
            entry("a-b", NbtSummaryValue::String("\\\n\u{0001}".into())),
            entry("\u{10000}", NbtSummaryValue::Int(1)),
            entry("1quoted", NbtSummaryValue::String(String::new().into())),
        ]);

        assert_eq!(
            canonical_snbt_string(&value),
            "{\"1quoted\":\"\",\"TrUe\":'a\"b\\'c',a-b:\"\\\\\\n\\x01\",\"𐀀\":1,\"\":2}"
        );
        assert_eq!(
            canonical_snbt_java_string(&value).units(),
            canonical_snbt_string(&value)
                .encode_utf16()
                .collect::<Vec<_>>()
        );

        let key = JavaString::from(vec![0xd841]);
        let mut units = vec![0xdc42];
        units.extend("\\uD800😀".encode_utf16());
        let value = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: key,
            value: NbtSummaryValue::string_from_utf16(units),
        }]);
        let mut expected = "{\"".encode_utf16().collect::<Vec<_>>();
        expected.push(0xd841);
        expected.extend("\":\"".encode_utf16());
        expected.push(0xdc42);
        expected.extend("\\\\uD800😀\"}".encode_utf16());
        assert_eq!(canonical_snbt_java_string(&value).units(), expected);
        let text = canonical_snbt_string(&value);
        assert_eq!(text, r#"{"\uD841":"\uDC42\\uD800😀"}"#);
        assert_eq!(super::super::parse_snbt_compound(&text).unwrap(), value);
        assert_eq!(
            super::super::parse_snbt_compound_utf16(&expected).unwrap(),
            value
        );
    }

    #[test]
    fn uses_java_float_and_double_text() {
        assert_eq!(
            canonical_snbt_string(&NbtSummaryValue::Float(f32::from_bits(1).to_bits())),
            "1.4E-45f"
        );
        assert_eq!(
            canonical_snbt_string(&NbtSummaryValue::Double(f64::from_bits(1).to_bits())),
            "4.9E-324d"
        );
        assert_eq!(
            canonical_snbt_string(&NbtSummaryValue::Float(f32::NAN.to_bits())),
            "NaNf"
        );
        assert_eq!(
            canonical_snbt_string(&NbtSummaryValue::Double(f64::NEG_INFINITY.to_bits())),
            "-Infinityd"
        );
    }
}
