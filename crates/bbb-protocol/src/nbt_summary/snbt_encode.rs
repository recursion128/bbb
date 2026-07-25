use std::cmp::Ordering;

use super::NbtSummaryValue;
use crate::component::{java_double_to_string, java_float_to_string};

/// Formats an NBT value exactly like vanilla 26.1's `Tag.toString` /
/// `StringTagVisitor`.
pub fn canonical_snbt_string(value: &NbtSummaryValue) -> String {
    let mut output = String::new();
    write_snbt(value, &mut output);
    output
}

fn write_snbt(value: &NbtSummaryValue, output: &mut String) {
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
        NbtSummaryValue::String(value) => write_quoted(value, output),
        NbtSummaryValue::List(values) => {
            output.push('[');
            write_joined(values, output, write_snbt);
            output.push(']');
        }
        NbtSummaryValue::Compound(entries) => {
            output.push('{');
            let mut entries = super::last_wins_compound_entries(entries);
            entries.sort_by(|left, right| java_string_cmp(&left.name, &right.name));
            write_joined(&entries, output, |entry, output| {
                write_key(&entry.name, output);
                output.push(':');
                write_snbt(&entry.value, output);
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

fn write_joined<T>(
    values: &[T],
    output: &mut String,
    mut write_value: impl FnMut(&T, &mut String),
) {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write_value(value, output);
    }
}

fn write_key(value: &str, output: &mut String) {
    if !value.eq_ignore_ascii_case("true")
        && !value.eq_ignore_ascii_case("false")
        && is_unquoted_key(value)
    {
        output.push_str(value);
    } else {
        write_quoted(value, output);
    }
}

fn is_unquoted_key(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(is_unquoted_key_start) && chars.all(is_unquoted_key_continuation)
}

fn is_unquoted_key_start(value: char) -> bool {
    value.is_ascii_alphabetic() || matches!(value, '.' | '_')
}

fn is_unquoted_key_continuation(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '.' | '_' | '+' | '-')
}

fn write_quoted(value: &str, output: &mut String) {
    let quote_index = output.len();
    output.push(' ');
    let mut quote = None;
    for value in value.chars() {
        match value {
            '\\' => output.push_str("\\\\"),
            '"' | '\'' => {
                let selected = *quote.get_or_insert(if value == '"' { '\'' } else { '"' });
                if selected == value {
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
    let quote = quote.unwrap_or('"');
    output.replace_range(quote_index..quote_index + 1, &quote.to_string());
    output.push(quote);
}

fn hex_digit(value: u8) -> char {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    char::from(HEX[usize::from(value & 0x0f)])
}

fn java_string_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NbtSummaryEntry;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
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
                    NbtSummaryValue::String("value".to_string()),
                    NbtSummaryValue::Compound(Vec::new()),
                ]),
            ),
        ]);

        assert_eq!(
            canonical_snbt_string(&value),
            r#"{bytes:[B;-1B,2B],double:0.25d,float:0.5f,int:3,ints:[I;3,4],list:["value",{}],long:4L,longs:[L;5L,6L],short:2s,z:-1b}"#
        );
    }

    #[test]
    fn matches_java_key_order_and_string_escaping() {
        let value = NbtSummaryValue::Compound(vec![
            entry("\u{e000}", NbtSummaryValue::Int(2)),
            entry("true", NbtSummaryValue::String("a\"b'c".to_string())),
            entry("a-b", NbtSummaryValue::String("\\\n\u{0001}".to_string())),
            entry("\u{10000}", NbtSummaryValue::Int(1)),
            entry("1quoted", NbtSummaryValue::String(String::new())),
        ]);

        assert_eq!(
            canonical_snbt_string(&value),
            "{\"1quoted\":\"\",a-b:\"\\\\\\n\\x01\",\"true\":'a\"b\\'c',\"𐀀\":1,\"\":2}"
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
