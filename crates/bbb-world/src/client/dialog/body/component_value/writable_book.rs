use bbb_protocol::NbtSummaryValue;
use serde::{Deserialize, Serialize};

use super::find_entry;

const MAX_WRITABLE_BOOK_PAGES: usize = 100;
const MAX_WRITABLE_BOOK_PAGE_UTF16_UNITS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogFilterableStringState {
    pub raw: String,
    #[serde(default)]
    pub filtered: Option<String>,
}

pub(super) fn decode_writable_book_content(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogFilterableStringState>, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let Some(pages) = find_entry(entries, "pages") else {
        return Ok(Vec::new());
    };
    let NbtSummaryValue::List(pages) = pages else {
        return Err(format!("{what} field pages is not a list"));
    };
    if pages.len() > MAX_WRITABLE_BOOK_PAGES {
        return Err(format!(
            "{what} field pages has {} entries, maximum is {MAX_WRITABLE_BOOK_PAGES}",
            pages.len()
        ));
    }
    pages
        .iter()
        .enumerate()
        .map(|page| {
            decode_filterable_string(
                page.1,
                MAX_WRITABLE_BOOK_PAGE_UTF16_UNITS,
                &format!("{what} field pages[{}]", page.0),
            )
        })
        .collect()
}

pub(super) fn decode_filterable_string(
    value: &NbtSummaryValue,
    max_utf16_units: usize,
    what: &str,
) -> Result<DialogFilterableStringState, String> {
    match value {
        NbtSummaryValue::String(raw) => Ok(DialogFilterableStringState {
            raw: bounded_string(raw, max_utf16_units, what)?,
            filtered: None,
        }),
        NbtSummaryValue::Compound(entries) => {
            let raw =
                find_entry(entries, "raw").ok_or_else(|| format!("{what} is missing field raw"))?;
            let NbtSummaryValue::String(raw) = raw else {
                return Err(format!("{what} field raw is not a string"));
            };
            let filtered = find_entry(entries, "filtered")
                .map(|value| {
                    let NbtSummaryValue::String(filtered) = value else {
                        return Err(format!("{what} field filtered is not a string"));
                    };
                    bounded_string(filtered, max_utf16_units, &format!("{what} field filtered"))
                })
                .transpose()?;
            Ok(DialogFilterableStringState {
                raw: bounded_string(raw, max_utf16_units, &format!("{what} field raw"))?,
                filtered,
            })
        }
        _ => Err(format!("{what} is neither a string nor a compound")),
    }
}

fn bounded_string(value: &str, max_utf16_units: usize, what: &str) -> Result<String, String> {
    let len = value.encode_utf16().count();
    if len > max_utf16_units {
        return Err(format!(
            "{what} has {len} UTF-16 units, maximum is {max_utf16_units}"
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests;
