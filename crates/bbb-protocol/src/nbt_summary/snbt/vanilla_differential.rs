//! The differential corpus for the ported SNBT grammar.
//!
//! Every table row was produced by driving the same input through Vanilla's own
//! packrat parser -- `SnbtGrammar.createParser(NbtOps.INSTANCE)` reached through
//! `Grammar.parseForCommands` (`Grammar.java:29-56`), the entry point
//! `TagParser.parseAsArgument` calls (`TagParser.java:79-81`) -- and then through
//! this crate's scanner. The `Vanilla` column is that oracle's outcome, not a
//! reading of the source.
//!
//! An accepted row carries the tag and the cursor. A rejected one carries the
//! whole refusal the oracle threw: the cursor `ErrorCollector.LongestOnly`
//! (`ErrorCollector.java:17-75`) kept, the translation key of the first entry
//! stored there, and the argument its `DynamicCommandExceptionType` formats in.

use super::{parse_snbt_compound_utf16, parse_snbt_prefix_utf16};

use crate::nbt_summary::canonical_snbt_string;

/// What Vanilla's parser did with one input.
enum Vanilla {
    /// `Grammar.parseForCommands` returned this tag and left the reader here.
    Value { snbt: &'static str, cursor: usize },
    /// The parse failed and the first error at the longest cursor sat here.
    Error {
        cursor: usize,
        key: &'static str,
        /// `""` where the key takes no argument.
        argument: &'static str,
    },
}

fn units(input: &str) -> Vec<u16> {
    input.encode_utf16().collect()
}

/// Drives `parse_snbt_prefix_utf16`, the `parseAsArgument` shape.
fn check_argument(cases: &[(&str, Vanilla)]) {
    for (input, expected) in cases {
        match (parse_snbt_prefix_utf16(&units(input)), expected) {
            (Ok((consumed, value)), Vanilla::Value { snbt, cursor }) => {
                assert_eq!(
                    canonical_snbt_string(&value),
                    *snbt,
                    "{input:?} parsed to the wrong tag"
                );
                assert_eq!(consumed, *cursor, "{input:?} consumed the wrong length");
            }
            (
                Err(failure),
                Vanilla::Error {
                    cursor,
                    key,
                    argument,
                },
            ) => {
                let arguments: Vec<&str> = if argument.is_empty() {
                    Vec::new()
                } else {
                    vec![argument]
                };
                assert_eq!(
                    (failure.cursor, failure.key, failure.arguments.clone()),
                    (
                        *cursor,
                        Some(*key),
                        arguments.iter().map(|a| (*a).to_string()).collect()
                    ),
                    "{input:?} was rejected differently"
                );
            }
            (Ok((consumed, value)), Vanilla::Error { cursor, key, .. }) => panic!(
                "{input:?} was accepted as {:?} @{consumed}, vanilla rejected it at {cursor} ({key})",
                canonical_snbt_string(&value)
            ),
            (Err(failure), Vanilla::Value { snbt, cursor }) => panic!(
                "{input:?} was rejected ({failure}), vanilla parsed {snbt} @{cursor}"
            ),
        }
    }
}

#[test]
fn numbers_that_only_a_prefix_grammar_accepts() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "1.5.5",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 3,
            },
        ),
        (
            "1.2.3",
            Vanilla::Value {
                snbt: "1.2d",
                cursor: 3,
            },
        ),
        (
            "1..2",
            Vanilla::Value {
                snbt: "1.0d",
                cursor: 2,
            },
        ),
        (
            "1e5b",
            Vanilla::Value {
                snbt: "100000.0d",
                cursor: 3,
            },
        ),
        (
            "1.5b",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 3,
            },
        ),
        (
            "1.5s",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 3,
            },
        ),
        (
            "1.5l",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 3,
            },
        ),
        (
            "1f2",
            Vanilla::Value {
                snbt: "1.0f",
                cursor: 2,
            },
        ),
        (
            "1fb",
            Vanilla::Value {
                snbt: "1.0f",
                cursor: 2,
            },
        ),
        (
            "1bd",
            Vanilla::Value {
                snbt: "1b",
                cursor: 2,
            },
        ),
        (
            "1abc",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
        (
            "1e",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
        (
            "1e+",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
        (
            "1.e",
            Vanilla::Value {
                snbt: "1.0d",
                cursor: 2,
            },
        ),
        (
            "1e400",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
        (
            "1e-400",
            Vanilla::Value {
                snbt: "0.0d",
                cursor: 6,
            },
        ),
        (
            "1_000",
            Vanilla::Value {
                snbt: "1000",
                cursor: 5,
            },
        ),
        (
            "1__0",
            Vanilla::Value {
                snbt: "10",
                cursor: 4,
            },
        ),
        (
            "0_1",
            Vanilla::Value {
                snbt: "0",
                cursor: 1,
            },
        ),
        (
            "0_0",
            Vanilla::Value {
                snbt: "0",
                cursor: 1,
            },
        ),
        (
            "0b",
            Vanilla::Value {
                snbt: "0b",
                cursor: 2,
            },
        ),
        (
            "0b2",
            Vanilla::Value {
                snbt: "0b",
                cursor: 2,
            },
        ),
        (
            "0b_1",
            Vanilla::Value {
                snbt: "0b",
                cursor: 2,
            },
        ),
        (
            "1sx",
            Vanilla::Value {
                snbt: "1s",
                cursor: 2,
            },
        ),
        (
            "1ux",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
        (
            "1usx",
            Vanilla::Value {
                snbt: "1s",
                cursor: 3,
            },
        ),
        (
            "1s",
            Vanilla::Value {
                snbt: "1s",
                cursor: 2,
            },
        ),
        (
            "1sb2",
            Vanilla::Value {
                snbt: "1b",
                cursor: 3,
            },
        ),
        (
            "1b2",
            Vanilla::Value {
                snbt: "1b",
                cursor: 2,
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn integers() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "0",
            Vanilla::Value {
                snbt: "0",
                cursor: 1,
            },
        ),
        (
            "1",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
        (
            "-1",
            Vanilla::Value {
                snbt: "-1",
                cursor: 2,
            },
        ),
        (
            "+1",
            Vanilla::Value {
                snbt: "1",
                cursor: 2,
            },
        ),
        (
            "1b",
            Vanilla::Value {
                snbt: "1b",
                cursor: 2,
            },
        ),
        (
            "1B",
            Vanilla::Value {
                snbt: "1b",
                cursor: 2,
            },
        ),
        (
            "1s",
            Vanilla::Value {
                snbt: "1s",
                cursor: 2,
            },
        ),
        (
            "1i",
            Vanilla::Value {
                snbt: "1",
                cursor: 2,
            },
        ),
        (
            "1l",
            Vanilla::Value {
                snbt: "1L",
                cursor: 2,
            },
        ),
        (
            "1L",
            Vanilla::Value {
                snbt: "1L",
                cursor: 2,
            },
        ),
        (
            "1ub",
            Vanilla::Value {
                snbt: "1b",
                cursor: 3,
            },
        ),
        (
            "1sl",
            Vanilla::Value {
                snbt: "1L",
                cursor: 3,
            },
        ),
        (
            "0x1F",
            Vanilla::Value {
                snbt: "31",
                cursor: 4,
            },
        ),
        (
            "0X1f",
            Vanilla::Value {
                snbt: "31",
                cursor: 4,
            },
        ),
        (
            "0x7fffffff",
            Vanilla::Value {
                snbt: "2147483647",
                cursor: 10,
            },
        ),
        (
            "255ub",
            Vanilla::Value {
                snbt: "-1b",
                cursor: 5,
            },
        ),
        (
            "-9223372036854775808l",
            Vanilla::Value {
                snbt: "-9223372036854775808L",
                cursor: 21,
            },
        ),
        (
            "9223372036854775807",
            Vanilla::Error {
                cursor: 19,
                key: "argument.literal.incorrect",
                argument: ".",
            },
        ),
        (
            "[I;0x10]",
            Vanilla::Value {
                snbt: "[I;16]",
                cursor: 8,
            },
        ),
        (
            "[L;1b]",
            Vanilla::Value {
                snbt: "[L;1L]",
                cursor: 6,
            },
        ),
        (
            "[I;1s,2]",
            Vanilla::Value {
                snbt: "[I;1,2]",
                cursor: 8,
            },
        ),
        (
            "[B;1,2]",
            Vanilla::Value {
                snbt: "[B;1B,2B]",
                cursor: 7,
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn whitespace_is_skipped_by_every_terminal() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "- 1",
            Vanilla::Value {
                snbt: "-1",
                cursor: 3,
            },
        ),
        (
            "+ 1",
            Vanilla::Value {
                snbt: "1",
                cursor: 3,
            },
        ),
        (
            "1 . 5",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 5,
            },
        ),
        (
            "1 .5",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 4,
            },
        ),
        (
            "1. 5",
            Vanilla::Value {
                snbt: "1.5d",
                cursor: 4,
            },
        ),
        (
            "1 e5",
            Vanilla::Value {
                snbt: "100000.0d",
                cursor: 4,
            },
        ),
        (
            "1e 5",
            Vanilla::Value {
                snbt: "100000.0d",
                cursor: 4,
            },
        ),
        (
            "0 x1",
            Vanilla::Value {
                snbt: "1",
                cursor: 4,
            },
        ),
        (
            "1 f",
            Vanilla::Value {
                snbt: "1.0f",
                cursor: 3,
            },
        ),
        (
            "1 b",
            Vanilla::Value {
                snbt: "1b",
                cursor: 3,
            },
        ),
        (
            ". 1",
            Vanilla::Value {
                snbt: "0.1d",
                cursor: 3,
            },
        ),
        (
            "  {a:1}  ",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 7,
            },
        ),
        (
            "{ a : 1 }",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 9,
            },
        ),
        (
            "[ 1 , 2 ]",
            Vanilla::Value {
                snbt: "[1,2]",
                cursor: 9,
            },
        ),
        (
            "[ I ; 1 , 2 ]",
            Vanilla::Value {
                snbt: "[I;1,2]",
                cursor: 13,
            },
        ),
        (
            "{a : 1}",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 7,
            },
        ),
        (
            "a (1)",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.no_such_operation",
                argument: "a/1",
            },
        ),
        (
            "bool (1)",
            Vanilla::Value {
                snbt: "1b",
                cursor: 8,
            },
        ),
        (
            "{a:1}\u{000A}",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 5,
            },
        ),
        (
            "\u{000A}1",
            Vanilla::Value {
                snbt: "1",
                cursor: 2,
            },
        ),
        (
            "1  2",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn floats_and_their_value_failures() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "1.",
            Vanilla::Value {
                snbt: "1.0d",
                cursor: 2,
            },
        ),
        (
            "1.0",
            Vanilla::Value {
                snbt: "1.0d",
                cursor: 3,
            },
        ),
        (
            ".1",
            Vanilla::Value {
                snbt: "0.1d",
                cursor: 2,
            },
        ),
        (
            "-.1",
            Vanilla::Value {
                snbt: "-0.1d",
                cursor: 3,
            },
        ),
        (
            "+.1",
            Vanilla::Value {
                snbt: "0.1d",
                cursor: 3,
            },
        ),
        (
            "1e5",
            Vanilla::Value {
                snbt: "100000.0d",
                cursor: 3,
            },
        ),
        (
            "1E5",
            Vanilla::Value {
                snbt: "100000.0d",
                cursor: 3,
            },
        ),
        (
            "1e+5",
            Vanilla::Value {
                snbt: "100000.0d",
                cursor: 4,
            },
        ),
        (
            "1e-5",
            Vanilla::Value {
                snbt: "1.0E-5d",
                cursor: 4,
            },
        ),
        (
            "1.5e3",
            Vanilla::Value {
                snbt: "1500.0d",
                cursor: 5,
            },
        ),
        (
            "1.5e3f",
            Vanilla::Value {
                snbt: "1500.0f",
                cursor: 6,
            },
        ),
        (
            "1.5e3d",
            Vanilla::Value {
                snbt: "1500.0d",
                cursor: 6,
            },
        ),
        (
            "1.0f",
            Vanilla::Value {
                snbt: "1.0f",
                cursor: 4,
            },
        ),
        (
            "0.0e0",
            Vanilla::Value {
                snbt: "0.0d",
                cursor: 5,
            },
        ),
        (
            "-0.0",
            Vanilla::Value {
                snbt: "0.0d",
                cursor: 4,
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn strings_and_escapes() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "\"a\"",
            Vanilla::Value {
                snbt: "\"a\"",
                cursor: 3,
            },
        ),
        (
            "'a'",
            Vanilla::Value {
                snbt: "\"a\"",
                cursor: 3,
            },
        ),
        (
            "\"\"",
            Vanilla::Value {
                snbt: "\"\"",
                cursor: 2,
            },
        ),
        (
            "''",
            Vanilla::Value {
                snbt: "\"\"",
                cursor: 2,
            },
        ),
        (
            "\"\\n\"",
            Vanilla::Value {
                snbt: "\"\\n\"",
                cursor: 4,
            },
        ),
        (
            "\"\\q\"",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "b",
            },
        ),
        (
            "\"\\x41\"",
            Vanilla::Value {
                snbt: "\"A\"",
                cursor: 6,
            },
        ),
        (
            "\"\\u0041\"",
            Vanilla::Value {
                snbt: "\"A\"",
                cursor: 8,
            },
        ),
        (
            "\"\\N{LATIN SMALL LETTER A}\"",
            Vanilla::Value {
                snbt: "\"a\"",
                cursor: 26,
            },
        ),
        (
            "\"\\N{}\"",
            Vanilla::Error {
                cursor: 4,
                key: "snbt.parser.invalid_character_name",
                argument: "",
            },
        ),
        (
            "'a\"b'",
            Vanilla::Value {
                snbt: "'a\"b'",
                cursor: 5,
            },
        ),
        (
            "\"a'b\"",
            Vanilla::Value {
                snbt: "\"a'b\"",
                cursor: 5,
            },
        ),
        (
            "\"\\s\"",
            Vanilla::Value {
                snbt: "\" \"",
                cursor: 4,
            },
        ),
        (
            "\"\\  n\"",
            Vanilla::Value {
                snbt: "\"\\n\"",
                cursor: 6,
            },
        ),
        (
            "\"\\u12\"",
            Vanilla::Error {
                cursor: 3,
                key: "snbt.parser.expected_hex_escape",
                argument: "4",
            },
        ),
        (
            "\"\\xZZ\"",
            Vanilla::Error {
                cursor: 3,
                key: "snbt.parser.expected_hex_escape",
                argument: "2",
            },
        ),
        (
            "\"\\U00110000\"",
            Vanilla::Error {
                cursor: 11,
                key: "snbt.parser.invalid_codepoint",
                argument: "U+00110000",
            },
        ),
        (
            "\"a",
            Vanilla::Error {
                cursor: 2,
                key: "snbt.parser.invalid_string_contents",
                argument: "",
            },
        ),
        (
            "\"\\N{NOT A NAME}\"",
            Vanilla::Error {
                cursor: 15,
                key: "snbt.parser.invalid_character_name",
                argument: "",
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn builtins() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "true",
            Vanilla::Value {
                snbt: "1b",
                cursor: 4,
            },
        ),
        (
            "false",
            Vanilla::Value {
                snbt: "0b",
                cursor: 5,
            },
        ),
        (
            "TRUE",
            Vanilla::Value {
                snbt: "1b",
                cursor: 4,
            },
        ),
        (
            "False",
            Vanilla::Value {
                snbt: "0b",
                cursor: 5,
            },
        ),
        (
            "bool(1)",
            Vanilla::Value {
                snbt: "1b",
                cursor: 7,
            },
        ),
        (
            "bool(true)",
            Vanilla::Value {
                snbt: "1b",
                cursor: 10,
            },
        ),
        (
            "bool()",
            Vanilla::Error {
                cursor: 6,
                key: "snbt.parser.no_such_operation",
                argument: "bool/0",
            },
        ),
        (
            "bool(1,2)",
            Vanilla::Error {
                cursor: 9,
                key: "snbt.parser.no_such_operation",
                argument: "bool/2",
            },
        ),
        (
            "uuid(1)",
            Vanilla::Error {
                cursor: 7,
                key: "snbt.parser.expected_string_uuid",
                argument: "",
            },
        ),
        (
            "uuid(\"00000000-0000-0000-0000-000000000000\")",
            Vanilla::Value {
                snbt: "[I;0,0,0,0]",
                cursor: 44,
            },
        ),
        (
            "uuid(\"x\")",
            Vanilla::Error {
                cursor: 9,
                key: "snbt.parser.expected_string_uuid",
                argument: "",
            },
        ),
        (
            "foo(1)",
            Vanilla::Error {
                cursor: 6,
                key: "snbt.parser.no_such_operation",
                argument: "foo/1",
            },
        ),
        (
            "a",
            Vanilla::Value {
                snbt: "\"a\"",
                cursor: 1,
            },
        ),
        (
            "a.b",
            Vanilla::Value {
                snbt: "\"a.b\"",
                cursor: 3,
            },
        ),
        (
            "-a",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "a(",
            Vanilla::Error {
                cursor: 2,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            "a()",
            Vanilla::Error {
                cursor: 3,
                key: "snbt.parser.no_such_operation",
                argument: "a/0",
            },
        ),
        (
            "a(1,)",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.no_such_operation",
                argument: "a/1",
            },
        ),
        (
            "1abc",
            Vanilla::Value {
                snbt: "1",
                cursor: 1,
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn compounds_lists_and_arrays() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "{}",
            Vanilla::Value {
                snbt: "{}",
                cursor: 2,
            },
        ),
        (
            "{a:1}",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 5,
            },
        ),
        (
            "{a:1,b:2}",
            Vanilla::Value {
                snbt: "{a:1,b:2}",
                cursor: 9,
            },
        ),
        (
            "{b:1,a:2,zz:3,m:4}",
            Vanilla::Value {
                snbt: "{a:2,b:1,m:4,zz:3}",
                cursor: 18,
            },
        ),
        (
            "{a:1,a:2}",
            Vanilla::Value {
                snbt: "{a:2}",
                cursor: 9,
            },
        ),
        (
            "{a:1,}",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 6,
            },
        ),
        (
            "{,}",
            Vanilla::Error {
                cursor: 1,
                key: "argument.literal.incorrect",
                argument: "\"",
            },
        ),
        (
            "{a}",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: ":",
            },
        ),
        (
            "{:1}",
            Vanilla::Error {
                cursor: 1,
                key: "argument.literal.incorrect",
                argument: "\"",
            },
        ),
        (
            "{\"\":1}",
            Vanilla::Error {
                cursor: 5,
                key: "argument.literal.incorrect",
                argument: ".",
            },
        ),
        (
            "{a:1 b:2}",
            Vanilla::Error {
                cursor: 6,
                key: "argument.literal.incorrect",
                argument: ",",
            },
        ),
        (
            "{a:1}}",
            Vanilla::Value {
                snbt: "{a:1}",
                cursor: 5,
            },
        ),
        (
            "{a:1",
            Vanilla::Error {
                cursor: 4,
                key: "argument.literal.incorrect",
                argument: ".",
            },
        ),
        (
            "{{a:1}:1}",
            Vanilla::Error {
                cursor: 1,
                key: "argument.literal.incorrect",
                argument: "\"",
            },
        ),
        (
            "{a:{b:{c:1}}}",
            Vanilla::Value {
                snbt: "{a:{b:{c:1}}}",
                cursor: 13,
            },
        ),
        (
            "{a:[1,2]}",
            Vanilla::Value {
                snbt: "{a:[1,2]}",
                cursor: 9,
            },
        ),
        (
            "[]",
            Vanilla::Value {
                snbt: "[]",
                cursor: 2,
            },
        ),
        (
            "[1]",
            Vanilla::Value {
                snbt: "[1]",
                cursor: 3,
            },
        ),
        (
            "[1,]",
            Vanilla::Value {
                snbt: "[1]",
                cursor: 4,
            },
        ),
        (
            "[,]",
            Vanilla::Error {
                cursor: 1,
                key: "argument.literal.incorrect",
                argument: "B",
            },
        ),
        (
            "[1 2]",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: ".",
            },
        ),
        (
            "[1;2]",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: ".",
            },
        ),
        (
            "[I;]",
            Vanilla::Value {
                snbt: "[I;]",
                cursor: 4,
            },
        ),
        (
            "[B;]",
            Vanilla::Value {
                snbt: "[B;]",
                cursor: 4,
            },
        ),
        (
            "[L;]",
            Vanilla::Value {
                snbt: "[L;]",
                cursor: 4,
            },
        ),
        (
            "[I;1,]",
            Vanilla::Value {
                snbt: "[I;1]",
                cursor: 6,
            },
        ),
        (
            "[I;1l]",
            Vanilla::Error {
                cursor: 6,
                key: "snbt.parser.invalid_array_element_type",
                argument: "",
            },
        ),
        (
            "[B;128b]",
            Vanilla::Error {
                cursor: 8,
                key: "snbt.parser.number_parse_failure",
                argument: "Value out of range. Value:\"128\" Radix:10",
            },
        ),
        (
            "[I;3000000000]",
            Vanilla::Error {
                cursor: 14,
                key: "snbt.parser.number_parse_failure",
                argument: "For input string: \"3000000000\"",
            },
        ),
        (
            "[B; true ]",
            Vanilla::Error {
                cursor: 4,
                key: "argument.literal.incorrect",
                argument: "+",
            },
        ),
        (
            "[b;1]",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "[I 1]",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: ";",
            },
        ),
        (
            "[x;1]",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "[B ; 1]",
            Vanilla::Value {
                snbt: "[B;1B]",
                cursor: 7,
            },
        ),
        (
            "[[1],[2]]",
            Vanilla::Value {
                snbt: "[[1],[2]]",
                cursor: 9,
            },
        ),
        (
            "[1,[2]]",
            Vanilla::Value {
                snbt: "[1,[2]]",
                cursor: 7,
            },
        ),
    ];
    check_argument(cases);
}

#[test]
fn rejected_tokens() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "!",
            Vanilla::Error {
                cursor: 0,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            "@",
            Vanilla::Error {
                cursor: 0,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            "#",
            Vanilla::Error {
                cursor: 0,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            "  ",
            Vanilla::Error {
                cursor: 2,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            "\u{00A0}",
            Vanilla::Error {
                cursor: 0,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            "\u{3000}",
            Vanilla::Error {
                cursor: 1,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            ".",
            Vanilla::Error {
                cursor: 1,
                key: "snbt.parser.expected_decimal_numeral",
                argument: "",
            },
        ),
        (
            "..",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "0x",
            Vanilla::Error {
                cursor: 2,
                key: "snbt.parser.expected_hex_numeral",
                argument: "",
            },
        ),
        (
            "0xg",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "+_1",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "1_",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "1__",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "1_.5",
            Vanilla::Error {
                cursor: 4,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "0x1f_",
            Vanilla::Error {
                cursor: 5,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
    ];
    check_argument(cases);
}

/// `TagParser.parseCompoundFully` (`TagParser.java:61-63`) is the compound-only
/// half of `FLATTENED_CODEC`, so a non-compound root is rejected there and only
/// there.
#[test]
fn flattened_codec_accepts_only_a_compound_root() {
    for (input, expected) in [
        ("{a:1}", Some("{a:1}")),
        ("{}", Some("{}")),
        ("{a:{b:[1,2s,3l]}}", Some("{a:{b:[1,2s,3L]}}")),
        ("[1]", None),
        ("1", None),
        ("{a:1} ", Some("{a:1}")),
        ("{a:1}x", None),
        ("{a:1", None),
    ] {
        let parsed = parse_snbt_compound_utf16(&units(input));
        match expected {
            Some(snbt) => assert_eq!(
                canonical_snbt_string(&parsed.expect(input)),
                snbt,
                "{input:?}"
            ),
            None => assert!(
                parsed.is_err(),
                "{input:?} should not be a flattened compound"
            ),
        }
    }
}

/// Every refusal the oracle reached, one row per key. `snbt.parser.expected_integer_type`
/// and `snbt.parser.expected_float_type` (`SnbtGrammar.java:234`, `:848`, `:859`) are
/// absent because no suffix rule yields a type of the other family, so the grammar
/// cannot reach them.
///
/// `snbt.parser.number_parse_failure` carries the JDK `NumberFormatException`
/// message of the call `IntegerLiteral.create` (`SnbtGrammar.java:825-867`) made,
/// so its six wordings are one row each.
#[test]
fn every_refusal_key_the_grammar_throws() {
    let cases: &[(&str, Vanilla)] = &[
        (
            "!",
            Vanilla::Error {
                cursor: 0,
                key: "snbt.parser.expected_unquoted_string",
                argument: "",
            },
        ),
        (
            ".",
            Vanilla::Error {
                cursor: 1,
                key: "snbt.parser.expected_decimal_numeral",
                argument: "",
            },
        ),
        (
            "0x",
            Vanilla::Error {
                cursor: 2,
                key: "snbt.parser.expected_hex_numeral",
                argument: "",
            },
        ),
        (
            "[B;0b2]",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.expected_binary_numeral",
                argument: "",
            },
        ),
        (
            "[B;0b_]",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.underscore_not_allowed",
                argument: "",
            },
        ),
        (
            "[B;01]",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.leading_zero_not_allowed",
                argument: "",
            },
        ),
        (
            "99999999999999999999999999e999f",
            Vanilla::Error {
                cursor: 31,
                key: "snbt.parser.infinity_not_allowed",
                argument: "",
            },
        ),
        (
            "-a(1)",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.invalid_unquoted_start",
                argument: "",
            },
        ),
        (
            "{\"\":\"x\"}",
            Vanilla::Error {
                cursor: 7,
                key: "snbt.parser.empty_key",
                argument: "",
            },
        ),
        (
            "[I;1l]",
            Vanilla::Error {
                cursor: 6,
                key: "snbt.parser.invalid_array_element_type",
                argument: "",
            },
        ),
        (
            "-1ub",
            Vanilla::Error {
                cursor: 4,
                key: "snbt.parser.expected_non_negative_number",
                argument: "",
            },
        ),
        (
            "bool()",
            Vanilla::Error {
                cursor: 6,
                key: "snbt.parser.no_such_operation",
                argument: "bool/0",
            },
        ),
        (
            "bool(\"x\")",
            Vanilla::Error {
                cursor: 9,
                key: "snbt.parser.expected_number_or_boolean",
                argument: "",
            },
        ),
        (
            "uuid(1)",
            Vanilla::Error {
                cursor: 7,
                key: "snbt.parser.expected_string_uuid",
                argument: "",
            },
        ),
        (
            "\"a",
            Vanilla::Error {
                cursor: 2,
                key: "snbt.parser.invalid_string_contents",
                argument: "",
            },
        ),
        (
            "\"\\u12\"",
            Vanilla::Error {
                cursor: 3,
                key: "snbt.parser.expected_hex_escape",
                argument: "4",
            },
        ),
        (
            "\"\\U00110000\"",
            Vanilla::Error {
                cursor: 11,
                key: "snbt.parser.invalid_codepoint",
                argument: "U+00110000",
            },
        ),
        (
            "\"\\N{}\"",
            Vanilla::Error {
                cursor: 4,
                key: "snbt.parser.invalid_character_name",
                argument: "",
            },
        ),
        (
            "128b",
            Vanilla::Error {
                cursor: 4,
                key: "snbt.parser.number_parse_failure",
                argument: "Value out of range. Value:\"128\" Radix:10",
            },
        ),
        (
            "0x80sb",
            Vanilla::Error {
                cursor: 6,
                key: "snbt.parser.number_parse_failure",
                argument: "Value out of range. Value:\"80\" Radix:16",
            },
        ),
        (
            "[I;3000000000]",
            Vanilla::Error {
                cursor: 14,
                key: "snbt.parser.number_parse_failure",
                argument: "For input string: \"3000000000\"",
            },
        ),
        (
            "0x100000000si",
            Vanilla::Error {
                cursor: 13,
                key: "snbt.parser.number_parse_failure",
                argument: "For input string: \"100000000\" under radix 16",
            },
        ),
        (
            "99999999999999999999ub",
            Vanilla::Error {
                cursor: 22,
                key: "snbt.parser.number_parse_failure",
                argument: "For input string: \"99999999999999999999\"",
            },
        ),
        (
            "256ub",
            Vanilla::Error {
                cursor: 5,
                key: "snbt.parser.number_parse_failure",
                argument: "out of range: 256",
            },
        ),
        (
            "0x10000us",
            Vanilla::Error {
                cursor: 9,
                key: "snbt.parser.number_parse_failure",
                argument: "out of range: 65536",
            },
        ),
        (
            "4294967296ui",
            Vanilla::Error {
                cursor: 12,
                key: "snbt.parser.number_parse_failure",
                argument: "String value 4294967296 exceeds range of unsigned int.",
            },
        ),
        (
            "18446744073709551616ul",
            Vanilla::Error {
                cursor: 22,
                key: "snbt.parser.number_parse_failure",
                argument: "String value 18446744073709551616 exceeds range of unsigned long.",
            },
        ),
        (
            "{a}",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: ":",
            },
        ),
        (
            "{,}",
            Vanilla::Error {
                cursor: 1,
                key: "argument.literal.incorrect",
                argument: "\"",
            },
        ),
        (
            "[,]",
            Vanilla::Error {
                cursor: 1,
                key: "argument.literal.incorrect",
                argument: "B",
            },
        ),
        (
            "[I 1]",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: ";",
            },
        ),
        (
            "[I;1.5]",
            Vanilla::Error {
                cursor: 4,
                key: "argument.literal.incorrect",
                argument: "u|U",
            },
        ),
        (
            "{a:1.5b}",
            Vanilla::Error {
                cursor: 6,
                key: "argument.literal.incorrect",
                argument: "e|E",
            },
        ),
        (
            "[B;-b]",
            Vanilla::Error {
                cursor: 4,
                key: "argument.literal.incorrect",
                argument: "0",
            },
        ),
        (
            "[B; true ]",
            Vanilla::Error {
                cursor: 4,
                key: "argument.literal.incorrect",
                argument: "+",
            },
        ),
        (
            "01",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: ".",
            },
        ),
        (
            "-a",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "(",
            },
        ),
        (
            "\"\\q\"",
            Vanilla::Error {
                cursor: 2,
                key: "argument.literal.incorrect",
                argument: "b",
            },
        ),
        (
            "\"\\N\"",
            Vanilla::Error {
                cursor: 3,
                key: "argument.literal.incorrect",
                argument: "{",
            },
        ),
    ];
    check_argument(cases);
}

/// Vanilla's `SnbtGrammar` has no depth cap; this scanner stops at
/// `MAX_NBT_DEPTH` so a hostile payload cannot spend the stack. Vanilla parses
/// the deeper input, so the bound is a deliberate divergence, named here and in
/// the ledger rather than hidden behind an equal-looking rejection.
#[test]
fn depth_bound_is_a_named_divergence() {
    let accepted = format!(
        "{}1{}",
        "[".repeat(super::MAX_NBT_DEPTH),
        "]".repeat(super::MAX_NBT_DEPTH)
    );
    assert!(parse_snbt_prefix_utf16(&units(&accepted)).is_ok());

    let rejected = format!(
        "{}1{}",
        "[".repeat(super::MAX_NBT_DEPTH + 1),
        "]".repeat(super::MAX_NBT_DEPTH + 1)
    );
    let failure = parse_snbt_prefix_utf16(&units(&rejected)).unwrap_err();
    // A bound carries no Vanilla key, which is how a consumer tells it apart.
    assert_eq!(failure.key, None);
    assert_eq!(failure.arguments, ["MAX_NBT_DEPTH"]);

    // Vanilla's own output for a 400-deep list, which this scanner rejects.
    let vanilla_deep = format!("{}1{}", "[".repeat(400), "]".repeat(400));
    assert!(parse_snbt_prefix_utf16(&units(&vanilla_deep)).is_err());
}
