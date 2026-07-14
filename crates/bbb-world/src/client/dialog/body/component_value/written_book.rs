use bbb_protocol::{
    canonical_component_json_utf16_len_with_owner, canonical_resolvable_profile_json_utf16_len,
    CanonicalComponentOwnerValue, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use super::super::super::{decode_component, DialogDecodeContext};
use super::profile::decode_profile;
use super::writable_book::decode_filterable_string;
use super::{
    component_bool, component_i32, find_entry, validate_i32_range, DialogFilterableStringState,
    DialogTextState,
};

const MAX_WRITTEN_BOOK_TITLE_UTF16_UNITS: usize = 32;
const MAX_WRITTEN_BOOK_PAGE_FLAT_JSON_UTF16_UNITS: usize = 32_767;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogFilterableComponentState {
    pub raw: DialogTextState,
    #[serde(default)]
    pub filtered: Option<DialogTextState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogWrittenBookContentState {
    pub title: DialogFilterableStringState,
    pub author: String,
    pub generation: i32,
    pub pages: Vec<DialogFilterableComponentState>,
    pub resolved: bool,
}

#[cfg(test)]
pub(super) fn decode_written_book_content(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogWrittenBookContentState, String> {
    decode_written_book_content_with_context(value, what, DialogDecodeContext::new(None, 0))
}

pub(super) fn decode_written_book_content_with_context(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogWrittenBookContentState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let title = find_entry(entries, "title")
        .ok_or_else(|| format!("{what} is missing field title"))
        .and_then(|value| {
            decode_filterable_string(
                value,
                MAX_WRITTEN_BOOK_TITLE_UTF16_UNITS,
                &format!("{what} field title"),
            )
        })?;
    let author = match find_entry(entries, "author") {
        Some(NbtSummaryValue::String(author)) => author.clone(),
        Some(_) => return Err(format!("{what} field author is not a string")),
        None => return Err(format!("{what} is missing field author")),
    };
    let generation = find_entry(entries, "generation").map_or(Ok(0), |value| {
        component_i32(value, &format!("{what} field generation"))
    })?;
    validate_i32_range(generation, 0, 3, &format!("{what} field generation"))?;
    let pages = match find_entry(entries, "pages") {
        None => Vec::new(),
        Some(NbtSummaryValue::List(pages)) => pages
            .iter()
            .enumerate()
            .map(|(index, page)| {
                decode_filterable_component(page, &format!("{what} field pages[{index}]"), context)
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(format!("{what} field pages is not a list")),
    };
    let resolved = find_entry(entries, "resolved").map_or(Ok(false), |value| {
        component_bool(value, &format!("{what} field resolved"))
    })?;
    Ok(DialogWrittenBookContentState {
        title,
        author,
        generation,
        pages,
        resolved,
    })
}

fn decode_filterable_component(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogFilterableComponentState, String> {
    if let NbtSummaryValue::Compound(entries) = value {
        if find_entry(entries, "raw").is_some() {
            match decode_full_filterable_component(entries, what, context) {
                Ok(component) => return Ok(component),
                Err(full_error) => {
                    return decode_page_component(value, what, context)
                        .map(|raw| DialogFilterableComponentState {
                            raw,
                            filtered: None,
                        })
                        .map_err(|_| full_error);
                }
            }
        }
    }
    decode_page_component(value, what, context).map(|raw| DialogFilterableComponentState {
        raw,
        filtered: None,
    })
}

fn decode_full_filterable_component(
    entries: &[NbtSummaryEntry],
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogFilterableComponentState, String> {
    let raw = find_entry(entries, "raw")
        .ok_or_else(|| format!("{what} is missing field raw"))
        .and_then(|value| decode_page_component(value, &format!("{what} field raw"), context))?;
    let filtered = find_entry(entries, "filtered")
        .map(|value| decode_page_component(value, &format!("{what} field filtered"), context))
        .transpose()?;
    Ok(DialogFilterableComponentState { raw, filtered })
}

fn decode_page_component(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogTextState, String> {
    if !has_component_shape(value) {
        return Err(format!("{what} is not a component"));
    }
    let component = decode_component(value, what, context)?;
    // Vanilla measures the semantic Component after decoding and canonical JsonOps re-encoding.
    // The protocol owner returns `None` when a remaining owner-sensitive payload or another
    // unimplemented canonical value codec prevents an exact answer; raw NBT size is never used as
    // a substitute. Player profiles reuse this module's sole semantic decoder before encoding.
    if let Some(encoded_len) =
        canonical_component_json_utf16_len_with_owner(value, |kind, value| match kind {
            CanonicalComponentOwnerValue::PlayerProfile => {
                decode_profile(value, "component player")
                    .ok()
                    .and_then(|profile| canonical_resolvable_profile_json_utf16_len(&profile))
            }
            CanonicalComponentOwnerValue::Dialog
            | CanonicalComponentOwnerValue::ItemComponentPatch => None,
        })
        .map_err(|error| format!("{what} cannot be canonically encoded: {error}"))?
    {
        if encoded_len > MAX_WRITTEN_BOOK_PAGE_FLAT_JSON_UTF16_UNITS {
            return Err(format!(
                "{what} encodes to {encoded_len} JSON UTF-16 units, maximum is {MAX_WRITTEN_BOOK_PAGE_FLAT_JSON_UTF16_UNITS}"
            ));
        }
    }
    Ok(component)
}

fn has_component_shape(value: &NbtSummaryValue) -> bool {
    match value {
        NbtSummaryValue::String(_) => true,
        NbtSummaryValue::List(values) => {
            !values.is_empty() && values.iter().all(has_component_shape)
        }
        NbtSummaryValue::Compound(entries) => entries.iter().any(|entry| {
            matches!(
                entry.name.as_str(),
                "text"
                    | "translate"
                    | "keybind"
                    | "score"
                    | "selector"
                    | "nbt"
                    | "object"
                    | "sprite"
                    | "player"
            )
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
