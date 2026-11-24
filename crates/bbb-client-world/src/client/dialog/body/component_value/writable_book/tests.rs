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
            NbtSummaryValue::string("compact page"),
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
                raw: "compact page".into(),
                filtered: None,
            },
            DialogFilterableStringState {
                raw: "raw page".into(),
                filtered: Some("filtered page".into()),
            },
            DialogFilterableStringState {
                raw: "unfiltered page".into(),
                filtered: None,
            },
        ]
    );

    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        &value,
        None,
    );
    assert_eq!(
        patch.writable_book_pages,
        ["compact page", "raw page", "unfiltered page"]
    );
    assert_eq!(
        patch.writable_book_page_filters,
        vec![None, Some("filtered page".into()), None]
    );
}

#[test]
fn accepts_writable_book_codec_collection_and_utf16_boundaries() {
    let pages = vec![NbtSummaryValue::string(""); 100];
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
                NbtSummaryValue::List(vec![NbtSummaryValue::string(astral_boundary.clone())]),
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
    for value in [
        NbtSummaryValue::List(Vec::new()),
        compound(vec![string_entry("pages", "not a list")]),
        compound(vec![value_entry(
            "pages",
            NbtSummaryValue::List(vec![NbtSummaryValue::string(""); 101]),
        )]),
        page_list(vec![NbtSummaryValue::Int(1)]),
        page_list(vec![compound(Vec::new())]),
        page_list(vec![compound(vec![value_entry(
            "raw",
            NbtSummaryValue::Int(1),
        )])]),
        page_list(vec![compound(vec![
            string_entry("raw", "raw"),
            value_entry("filtered", NbtSummaryValue::Int(1)),
        ])]),
        page_list(vec![NbtSummaryValue::string("😀".repeat(513))]),
        page_list(vec![compound(vec![
            string_entry("raw", "raw"),
            string_entry("filtered", &"x".repeat(1025)),
        ])]),
    ] {
        let _ = decode_writable_book_content(&value, "writable book").unwrap_err();
    }
}

fn page_list(pages: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    compound(vec![value_entry("pages", NbtSummaryValue::List(pages))])
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::string(value))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
