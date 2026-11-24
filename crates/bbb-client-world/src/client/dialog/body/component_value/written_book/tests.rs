use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    packets::inventory::WrittenBookContentSummary, NbtSummaryEntry, NbtSummaryValue,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

mod fixtures;
use fixtures::*;

fn decode_written_book_content(
    value: &NbtSummaryValue,
    _what: &str,
) -> Result<DialogWrittenBookContentState, String> {
    let Some(DialogDataComponentValueState::WrittenBookContent { content }) =
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            "minecraft:written_book_content",
            value,
            None,
        )?
    else {
        unreachable!()
    };
    Ok(content)
}

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
                NbtSummaryValue::string("Compact page"),
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

    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        &value,
        None,
    );
    assert_eq!(
        patch.written_book,
        Some(WrittenBookContentSummary {
            title: "A title".into(),
            title_filter: None,
            author: "An author".into(),
            generation: 2,
            pages: vec!["Compact page".to_string(), "Raw".to_string()],
            page_filters: vec![None, Some("Filtered".to_string())],
            resolved: true,
        })
    );
    assert_eq!(
        patch.written_book_pages_nbt,
        vec![
            Some(NbtSummaryValue::string("Compact page")),
            Some(compound(vec![string_entry("text", "Raw")])),
        ]
    );
    assert_eq!(
        patch.written_book_page_filters_nbt,
        vec![None, Some(styled)]
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
    assert_eq!(
        defaults.title.filtered.as_ref(),
        Some(&"Filtered title".into())
    );
    let alternatives = decode_written_book_content(
        &written_book(vec![
            string_entry("title", "Title"),
            string_entry("author", "Author"),
            value_entry(
                "pages",
                NbtSummaryValue::List(vec![
                    compound(vec![
                        value_entry("raw", NbtSummaryValue::Int(1)),
                        string_entry("text", "Compact raw"),
                    ]),
                    compound(vec![
                        string_entry("raw", "Full"),
                        value_entry("filtered", NbtSummaryValue::Int(1)),
                        string_entry("text", "Compact filtered"),
                    ]),
                    compound(vec![
                        string_entry("raw", "Full wins"),
                        string_entry("text", "Compact"),
                    ]),
                ]),
            ),
        ]),
        "written book",
    )
    .unwrap();
    assert_eq!(
        alternatives
            .pages
            .iter()
            .map(|page| page.raw.text.as_str())
            .collect::<Vec<_>>(),
        ["Compact raw", "Compact filtered", "Full wins"]
    );
    assert_eq!(defaults.generation, 0);
    assert!(defaults.pages.is_empty());
    assert!(!defaults.resolved);
}

#[test]
fn accepts_written_book_title_page_and_collection_boundaries() {
    let title = "😀".repeat(16);
    let pages = vec![NbtSummaryValue::string(""); 101];
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
    for title in [
        NbtSummaryValue::string("a".repeat(33)),
        compound(vec![
            string_entry("raw", "Title"),
            string_entry("filtered", &"😀".repeat(17)),
        ]),
    ] {
        assert!(decode_written_book_content(
            &written_book(vec![
                value_entry("title", title),
                string_entry("author", "Author")
            ]),
            "written book",
        )
        .is_err());
    }
}
