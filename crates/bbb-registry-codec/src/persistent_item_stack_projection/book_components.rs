//! Book component values shared by saved stacks and resource predicates.

use std::borrow::Cow;

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::component::{styled_component_runs_from_nbt_value, styled_runs_summary_text};
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::packets::WrittenBookContentSummary;
use bbb_protocol::{JavaString, NbtSummaryValue};

use crate::registry_codec::RegistryElementValueRef;

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::nbt_support::{ComponentReader, MAX_PERSISTENT_LIST_ITEMS, MAX_PREALLOC_CAPACITY};

fn filterable(
    value: RegistryElementValueRef<'_>,
) -> (
    RegistryElementValueRef<'_>,
    Option<RegistryElementValueRef<'_>>,
) {
    match value.field("raw") {
        Some(raw) => (raw, value.field("filtered")),
        None => (value, None),
    }
}

fn filterable_string(
    c: ComponentReader,
    value: RegistryElementValueRef<'_>,
    max_units: usize,
) -> Result<(JavaString, Option<JavaString>)> {
    let string = |value: RegistryElementValueRef<'_>| {
        value
            .java_string_ref()
            .filter(|value| value.units().len() <= max_units)
            .cloned()
            .ok_or_else(|| {
                c.error(format!(
                    "expected a string of at most {max_units} UTF-16 units"
                ))
            })
    };
    let (raw, filtered) = filterable(value);
    Ok((string(raw)?, filtered.map(string).transpose()?))
}

pub fn read_writable(
    value: RegistryElementValueRef<'_>,
) -> Result<(Vec<JavaString>, Vec<Option<JavaString>>)> {
    let c = ComponentReader::new(
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        "minecraft:writable_book_content",
    );
    if !value.is_compound() {
        return Err(c.error("writable book is not a compound"));
    }
    let pages = value
        .field("pages")
        .map(|pages| {
            pages
                .list_elements()
                .ok_or_else(|| c.error("pages is not a list"))
        })
        .transpose()?;
    let len = pages.as_ref().map_or(0, ExactSizeIterator::len);
    if len > PERSISTENT_WRITABLE_BOOK_MAX_PAGES {
        return Err(c.error(format!(
            "pages exceeds {PERSISTENT_WRITABLE_BOOK_MAX_PAGES} entries"
        )));
    }
    let cap = len.min(MAX_PREALLOC_CAPACITY);
    let mut raw_pages = Vec::with_capacity(cap);
    let mut filters = Vec::with_capacity(cap);
    for page in pages.into_iter().flatten() {
        let (raw, filtered) =
            filterable_string(c, page, PERSISTENT_WRITABLE_BOOK_PAGE_UTF16_UNITS)?;
        raw_pages.push(raw);
        filters.push(filtered);
    }
    Ok((raw_pages, filters))
}

pub type WrittenBookPageNbtSlots = Vec<Option<NbtSummaryValue>>;

pub fn read_written(
    value: RegistryElementValueRef<'_>,
) -> Result<(
    WrittenBookContentSummary,
    WrittenBookPageNbtSlots,
    WrittenBookPageNbtSlots,
)> {
    let c = ComponentReader::new(
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        "minecraft:written_book_content",
    );
    if !value.is_compound() {
        return Err(c.error("written book is not a compound"));
    }
    let (title, title_filter) = filterable_string(
        c,
        value
            .field("title")
            .ok_or_else(|| c.error("missing title"))?,
        PERSISTENT_WRITTEN_BOOK_TITLE_UTF16_UNITS,
    )?;
    let author = value
        .field("author")
        .and_then(RegistryElementValueRef::java_string)
        .ok_or_else(|| c.error("author is not a string"))?;
    let generation = value
        .field("generation")
        .map(|value| {
            c.int_in(
                value,
                "generation",
                0,
                PERSISTENT_WRITTEN_BOOK_MAX_GENERATION,
            )
        })
        .transpose()?
        .unwrap_or(0);
    let resolved = value
        .field("resolved")
        .map(|value| c.flag(value, "resolved"))
        .transpose()?
        .unwrap_or(false);
    let entries = value
        .field("pages")
        .map(|entries| {
            entries
                .list_elements()
                .ok_or_else(|| c.error("pages is not a list"))
        })
        .transpose()?;
    let len = entries.as_ref().map_or(0, ExactSizeIterator::len);
    if len > MAX_PERSISTENT_LIST_ITEMS {
        return Err(c.error(format!("pages exceeds {MAX_PERSISTENT_LIST_ITEMS} entries")));
    }
    let cap = len.min(MAX_PREALLOC_CAPACITY);
    let mut pages = Vec::with_capacity(cap);
    let mut page_filters = Vec::with_capacity(cap);
    let mut pages_nbt = Vec::with_capacity(cap);
    let mut page_filters_nbt = Vec::with_capacity(cap);
    for entry in entries.into_iter().flatten() {
        let (raw, filtered) = filterable_component(c, entry)?;
        pages.push(styled_runs_summary_text(
            &styled_component_runs_from_nbt_value(&raw),
        ));
        page_filters.push(
            filtered.as_ref().map(|value| {
                styled_runs_summary_text(&styled_component_runs_from_nbt_value(value))
            }),
        );
        pages_nbt.push(Some(raw.into_owned()));
        page_filters_nbt.push(filtered.map(Cow::into_owned));
    }
    Ok((
        WrittenBookContentSummary {
            title,
            title_filter,
            author,
            generation,
            pages,
            page_filters,
            resolved,
        },
        pages_nbt,
        page_filters_nbt,
    ))
}

fn component(
    c: ComponentReader,
    value: RegistryElementValueRef<'_>,
) -> Result<Cow<'_, NbtSummaryValue>> {
    let value = crate::json_nbt::element_value_to_nbt(value).map_err(|reason| c.error(reason))?;
    c.text(&value, "written book page")?;
    Ok(value)
}

fn filterable_component(
    c: ComponentReader,
    value: RegistryElementValueRef<'_>,
) -> Result<(Cow<'_, NbtSummaryValue>, Option<Cow<'_, NbtSummaryValue>>)> {
    if let Some(raw) = value.field("raw") {
        if let (Ok(raw), Ok(filtered)) = (
            component(c, raw),
            value
                .field("filtered")
                .map(|value| component(c, value))
                .transpose(),
        ) {
            return Ok((raw, filtered));
        }
    }
    Ok((component(c, value)?, None))
}

/// `Codec.string(0, 32)` for a written book title and `Codec.string(0, 1024)`
/// for a writable book page (`WrittenBookContent`,
/// `WritableBookContent`) count UTF-16 code units, and `Codec.validate`
/// runs while encoding as well.
pub(super) const PERSISTENT_WRITTEN_BOOK_TITLE_UTF16_UNITS: usize = 32;
pub(super) const PERSISTENT_WRITABLE_BOOK_PAGE_UTF16_UNITS: usize = 1024;
/// `PAGES_CODEC.sizeLimitedListOf(100)` (`WritableBookContent.java:17`).
pub(super) const PERSISTENT_WRITABLE_BOOK_MAX_PAGES: usize = 100;
/// `ExtraCodecs.intRange(0, 3)` (`WrittenBookContent.java:48`).
const PERSISTENT_WRITTEN_BOOK_MAX_GENERATION: i32 = 3;

/// One `Filterable` written through its record alternative.
///
/// `Filterable.codec` is `Codec.withAlternative(fullCodec, simpleCodec)`
/// and `withAlternative` only overrides decoding, so
/// encoding always writes the `{raw, filtered?}` record.
fn persistent_filterable_nbt(
    raw: NbtSummaryValue,
    filtered: Option<NbtSummaryValue>,
) -> NbtSummaryValue {
    let mut fields = vec![entry("raw", raw)];
    if let Some(filtered) = filtered {
        fields.push(entry("filtered", filtered));
    }
    NbtSummaryValue::Compound(fields)
}

/// `WritableBookContent.CODEC` writes one
/// `pages` field of at most 100 `Filterable<String>` entries, dropped entirely
/// when the list is empty because `optionalFieldOf(name, default)` omits a
/// value equal to its default.
pub(super) fn write_writable(
    type_id: i32,
    resource_id: &'static str,
    pages: &[bbb_protocol::JavaString],
    filters: &[Option<bbb_protocol::JavaString>],
) -> Result<NbtSummaryValue> {
    let c = ComponentReader::new(type_id, resource_id);
    if filters.len() != pages.len() {
        return Err(c.error(format!(
            "{} page filters do not align with {} pages",
            filters.len(),
            pages.len()
        )));
    }
    if pages.len() > PERSISTENT_WRITABLE_BOOK_MAX_PAGES {
        return Err(c.error(format!(
            "book has {} pages, but maximum is {PERSISTENT_WRITABLE_BOOK_MAX_PAGES}",
            pages.len()
        )));
    }
    let mut encoded = Vec::with_capacity(pages.len());
    for (page, filtered) in pages.iter().zip(filters) {
        for text in std::iter::once(page).chain(filtered.iter()) {
            if text.units().len() > PERSISTENT_WRITABLE_BOOK_PAGE_UTF16_UNITS {
                return Err(c.error(format!(
                    "page exceeds {PERSISTENT_WRITABLE_BOOK_PAGE_UTF16_UNITS} UTF-16 code units"
                )));
            }
        }
        encoded.push(persistent_filterable_nbt(
            page.to_nbt(),
            filtered.as_ref().map(bbb_protocol::JavaString::to_nbt),
        ));
    }
    let mut fields = Vec::new();
    if !encoded.is_empty() {
        fields.push(entry("pages", NbtSummaryValue::List(encoded)));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `WrittenBookContent.CODEC` writes a
/// required `Filterable` `title` and `author`, then `generation`, `pages` and
/// `resolved` only when they differ from `0`, the empty list and `false`.
///
/// Each page is a `Filterable<Component>` whose value codec encodes through
/// plain `ComponentSerialization.CODEC`,
/// which is the per-page NBT the wire decode already kept; the summary's
/// flattened page text cannot rebuild that tree, so a page without it fails
/// closed.
pub(super) fn write_written(
    type_id: i32,
    resource_id: &'static str,
    content: &WrittenBookContentSummary,
    pages_nbt: &[Option<NbtSummaryValue>],
    page_filters_nbt: &[Option<NbtSummaryValue>],
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    for title in std::iter::once(&content.title).chain(content.title_filter.iter()) {
        if title.units().len() > PERSISTENT_WRITTEN_BOOK_TITLE_UTF16_UNITS {
            return Err(invalid(format!(
                "title exceeds {PERSISTENT_WRITTEN_BOOK_TITLE_UTF16_UNITS} UTF-16 code units"
            )));
        }
    }
    if !(0..=PERSISTENT_WRITTEN_BOOK_MAX_GENERATION).contains(&content.generation) {
        return Err(invalid(format!(
            "generation {} is outside 0..={PERSISTENT_WRITTEN_BOOK_MAX_GENERATION}",
            content.generation
        )));
    }
    if pages_nbt.len() != content.pages.len()
        || page_filters_nbt.len() != content.pages.len()
        || content.page_filters.len() != content.pages.len()
    {
        return Err(invalid(format!(
            "{} pages carry {} raw and {} filtered component trees",
            content.pages.len(),
            pages_nbt.len(),
            page_filters_nbt.len()
        )));
    }

    let c = ComponentReader::new(type_id, resource_id);
    let mut pages = Vec::with_capacity(content.pages.len());
    for (index, raw) in pages_nbt.iter().enumerate() {
        let raw = raw.clone().ok_or({
            ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id,
                resource_id,
            }
        })?;
        c.text(&raw, "written book page")?;
        let filtered = &page_filters_nbt[index];
        if filtered.is_some() != content.page_filters[index].is_some() {
            return Err(invalid(format!(
                "page {index} filtered text and component tree disagree"
            )));
        }
        if let Some(filtered) = filtered.as_ref() {
            c.text(filtered, "written book page")?;
        }
        pages.push(persistent_filterable_nbt(raw, filtered.clone()));
    }

    let mut fields = vec![
        entry(
            "title",
            persistent_filterable_nbt(
                content.title.to_nbt(),
                content
                    .title_filter
                    .as_ref()
                    .map(bbb_protocol::JavaString::to_nbt),
            ),
        ),
        entry("author", content.author.to_nbt()),
    ];
    if content.generation != 0 {
        fields.push(entry(
            "generation",
            NbtSummaryValue::Int(content.generation),
        ));
    }
    if !pages.is_empty() {
        fields.push(entry("pages", NbtSummaryValue::List(pages)));
    }
    if content.resolved {
        fields.push(entry("resolved", NbtSummaryValue::Byte(1)));
    }
    Ok(NbtSummaryValue::Compound(fields))
}
