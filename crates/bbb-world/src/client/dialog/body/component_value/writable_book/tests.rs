use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID, NbtSummaryEntry,
    NbtSummaryValue,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_writable_book_defaults_and_filterable_page_alternatives() {
    assert_eq!(
        decode_writable_book_content(&compound(Vec::new()), "writable book").unwrap(),
        Vec::new()
    );

    let value = compound(vec![value_entry(
        "pages",
        NbtSummaryValue::List(vec![
            NbtSummaryValue::String("compact page".to_string()),
            compound(vec![
                string_entry("raw", "raw page"),
                string_entry("filtered", "filtered page"),
            ]),
            compound(vec![string_entry("raw", "unfiltered page")]),
        ]),
    )]);
    let pages = decode_writable_book_content(&value, "writable book").unwrap();
    assert_eq!(
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
            "minecraft:writable_book_content",
            &value,
            None,
        )
        .unwrap(),
        Some(DialogDataComponentValueState::WritableBookContent {
            pages: pages.clone(),
        })
    );
    assert_eq!(
        pages,
        vec![
            DialogFilterableStringState {
                raw: "compact page".to_string(),
                filtered: None,
            },
            DialogFilterableStringState {
                raw: "raw page".to_string(),
                filtered: Some("filtered page".to_string()),
            },
            DialogFilterableStringState {
                raw: "unfiltered page".to_string(),
                filtered: None,
            },
        ]
    );
}

#[test]
fn accepts_writable_book_codec_collection_and_utf16_boundaries() {
    let pages = vec![NbtSummaryValue::String(String::new()); 100];
    assert_eq!(
        decode_writable_book_content(
            &compound(vec![value_entry("pages", NbtSummaryValue::List(pages))]),
            "writable book",
        )
        .unwrap()
        .len(),
        100
    );

    let astral_boundary = "😀".repeat(512);
    assert_eq!(
        decode_writable_book_content(
            &compound(vec![value_entry(
                "pages",
                NbtSummaryValue::List(vec![NbtSummaryValue::String(astral_boundary.clone())]),
            )]),
            "writable book",
        )
        .unwrap()[0]
            .raw,
        astral_boundary
    );
}

#[test]
fn rejects_malformed_writable_book_records_pages_and_limits() {
    for (value, expected) in [
        (NbtSummaryValue::List(Vec::new()), "is not a compound"),
        (
            compound(vec![string_entry("pages", "not a list")]),
            "field pages is not a list",
        ),
        (
            compound(vec![value_entry(
                "pages",
                NbtSummaryValue::List(vec![NbtSummaryValue::String(String::new()); 101]),
            )]),
            "field pages has 101 entries, maximum is 100",
        ),
        (
            page_list(vec![NbtSummaryValue::Int(1)]),
            "pages[0] is neither a string nor a compound",
        ),
        (
            page_list(vec![compound(Vec::new())]),
            "pages[0] is missing field raw",
        ),
        (
            page_list(vec![compound(vec![value_entry(
                "raw",
                NbtSummaryValue::Int(1),
            )])]),
            "pages[0] field raw is not a string",
        ),
        (
            page_list(vec![compound(vec![
                string_entry("raw", "raw"),
                value_entry("filtered", NbtSummaryValue::Int(1)),
            ])]),
            "pages[0] field filtered is not a string",
        ),
        (
            page_list(vec![NbtSummaryValue::String("😀".repeat(513))]),
            "has 1026 UTF-16 units, maximum is 1024",
        ),
        (
            page_list(vec![compound(vec![
                string_entry("raw", "raw"),
                string_entry("filtered", &"x".repeat(1025)),
            ])]),
            "field filtered has 1025 UTF-16 units, maximum is 1024",
        ),
    ] {
        let error = decode_writable_book_content(&value, "writable book").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} does not contain {expected:?}"
        );
    }
}

fn page_list(pages: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    compound(vec![value_entry("pages", NbtSummaryValue::List(pages))])
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::String(value.to_string()))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}
