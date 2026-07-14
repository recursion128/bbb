use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID, NbtSummaryEntry,
    NbtSummaryValue,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_written_book_fields_defaults_and_filterable_alternatives() {
    let styled = compound(vec![
        string_entry("text", "Filtered"),
        value_entry("italic", NbtSummaryValue::Byte(1)),
    ]);
    let value = written_book(vec![
        string_entry("title", "A title"),
        string_entry("author", "An author"),
        value_entry("generation", NbtSummaryValue::Byte(2)),
        value_entry(
            "pages",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("Compact page".to_string()),
                compound(vec![
                    value_entry("raw", compound(vec![string_entry("text", "Raw")])),
                    value_entry("filtered", styled.clone()),
                ]),
            ]),
        ),
        value_entry("resolved", NbtSummaryValue::Byte(1)),
    ]);
    let content = decode_written_book_content(&value, "written book").unwrap();
    assert_eq!(content.title.raw, "A title");
    assert_eq!(content.title.filtered, None);
    assert_eq!(content.author, "An author");
    assert_eq!(content.generation, 2);
    assert!(content.resolved);
    assert_eq!(content.pages.len(), 2);
    assert_eq!(content.pages[0].raw.text, "Compact page");
    assert_eq!(content.pages[0].filtered, None);
    assert_eq!(content.pages[1].raw.text, "Raw");
    assert_eq!(
        content.pages[1]
            .filtered
            .as_ref()
            .map(|page| &page.component),
        Some(&styled)
    );

    assert_eq!(
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            "minecraft:written_book_content",
            &value,
            None,
        )
        .unwrap(),
        Some(DialogDataComponentValueState::WrittenBookContent { content })
    );

    let title = compound(vec![
        string_entry("raw", "Raw title"),
        string_entry("filtered", "Filtered title"),
    ]);
    let defaults = decode_written_book_content(
        &written_book(vec![
            value_entry("title", title),
            string_entry("author", "Author"),
        ]),
        "written book",
    )
    .unwrap();
    assert_eq!(defaults.title.raw, "Raw title");
    assert_eq!(defaults.title.filtered.as_deref(), Some("Filtered title"));
    assert_eq!(defaults.generation, 0);
    assert!(defaults.pages.is_empty());
    assert!(!defaults.resolved);
}

#[test]
fn accepts_written_book_title_page_and_collection_boundaries() {
    let title = "😀".repeat(16);
    let pages = vec![NbtSummaryValue::String(String::new()); 101];
    let content = decode_written_book_content(
        &written_book(vec![
            string_entry("title", &title),
            string_entry("author", "Author"),
            value_entry("generation", NbtSummaryValue::Int(3)),
            value_entry("pages", NbtSummaryValue::List(pages)),
        ]),
        "written book",
    )
    .unwrap();
    assert_eq!(content.title.raw, title);
    assert_eq!(content.pages.len(), 101);
    assert_eq!(content.generation, 3);

    for page in [
        "x".repeat(32_765),
        "\"".repeat(16_382),
        "\u{2028}".repeat(5_460),
        "😀".repeat(16_382),
    ] {
        assert!(decode_written_book_content(
            &book_with_pages(vec![NbtSummaryValue::String(page)]),
            "written book",
        )
        .is_ok());
    }
}

#[test]
fn follows_filterable_full_first_then_component_fallback() {
    let fallback_component = compound(vec![
        value_entry("raw", NbtSummaryValue::Int(1)),
        string_entry("text", "Compact component"),
    ]);
    let content = decode_written_book_content(
        &book_with_pages(vec![fallback_component.clone()]),
        "written book",
    )
    .unwrap();
    assert_eq!(content.pages[0].raw.text, "Compact component");
    assert_eq!(content.pages[0].raw.component, fallback_component);
    assert_eq!(content.pages[0].filtered, None);

    for object in [
        compound(vec![string_entry("sprite", "minecraft:block/stone")]),
        compound(vec![string_entry("player", "Player")]),
    ] {
        assert!(
            decode_written_book_content(&book_with_pages(vec![object]), "written book",).is_ok()
        );
    }
}

#[test]
fn rejects_malformed_written_book_fields_filterables_and_exact_string_page_limit() {
    let cases = [
        (NbtSummaryValue::List(Vec::new()), "is not a compound"),
        (
            compound(vec![string_entry("author", "Author")]),
            "is missing field title",
        ),
        (
            compound(vec![string_entry("title", "Title")]),
            "is missing field author",
        ),
        (
            written_book(vec![
                value_entry("title", NbtSummaryValue::Int(1)),
                string_entry("author", "Author"),
            ]),
            "field title is neither a string nor a compound",
        ),
        (
            written_book(vec![
                string_entry("title", "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀"),
                string_entry("author", "Author"),
            ]),
            "field title has 34 UTF-16 units, maximum is 32",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                value_entry("author", NbtSummaryValue::Int(1)),
            ]),
            "field author is not a string",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                value_entry("generation", NbtSummaryValue::Int(-1)),
            ]),
            "field generation -1 is outside 0..=3",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                value_entry("generation", NbtSummaryValue::Int(4)),
            ]),
            "field generation 4 is outside 0..=3",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                string_entry("pages", "not a list"),
            ]),
            "field pages is not a list",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::Int(1)]),
            "pages[0] is not a component",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::List(Vec::new())]),
            "pages[0] is not a component",
        ),
        (
            book_with_pages(vec![compound(vec![value_entry(
                "raw",
                NbtSummaryValue::Int(1),
            )])]),
            "pages[0] field raw is not a component",
        ),
        (
            book_with_pages(vec![compound(vec![
                string_entry("raw", "Raw"),
                value_entry("filtered", NbtSummaryValue::Int(1)),
            ])]),
            "pages[0] field filtered is not a component",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("x".repeat(32_766))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("\"".repeat(16_383))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("\u{2028}".repeat(5_461))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("😀".repeat(16_383))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                string_entry("resolved", "true"),
            ]),
            "field resolved is not a boolean",
        ),
    ];
    for (value, expected) in cases {
        let error = decode_written_book_content(&value, "written book").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} does not contain {expected:?}"
        );
    }
}

fn book_with_pages(pages: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    written_book(vec![
        string_entry("title", "Title"),
        string_entry("author", "Author"),
        value_entry("pages", NbtSummaryValue::List(pages)),
    ])
}

fn written_book(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    compound(entries)
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
