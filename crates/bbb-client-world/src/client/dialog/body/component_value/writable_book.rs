#[cfg(test)]
use bbb_protocol::NbtSummaryValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogFilterableStringState<T = String> {
    pub raw: T,
    #[serde(default)]
    pub filtered: Option<T>,
}

pub(super) fn project_writable_book_content(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Vec<DialogFilterableStringState<bbb_protocol::JavaString>> {
    patch
        .writable_book_pages
        .iter()
        .zip(&patch.writable_book_page_filters)
        .map(|(raw, filtered)| DialogFilterableStringState {
            raw: raw.clone(),
            filtered: filtered.clone(),
        })
        .collect()
}

#[cfg(test)]
fn decode_writable_book_content(
    value: &NbtSummaryValue,
    _what: &str,
) -> Result<Vec<DialogFilterableStringState<bbb_protocol::JavaString>>, String> {
    let mut patch = bbb_protocol::packets::DataComponentPatchSummary::default();
    super::read_component_value_into(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        value,
        None,
        &mut patch,
    )?;
    Ok(project_writable_book_content(&patch))
}

#[cfg(test)]
mod tests;
