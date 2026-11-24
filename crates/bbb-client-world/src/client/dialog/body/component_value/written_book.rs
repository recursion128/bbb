use bbb_protocol::{packets::DataComponentPatchSummary, JavaString};
use serde::{Deserialize, Serialize};

use super::super::super::{decode_component, DialogDecodeContext};
use super::{DialogFilterableStringState, DialogTextState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogFilterableComponentState {
    pub raw: DialogTextState,
    #[serde(default)]
    pub filtered: Option<DialogTextState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogWrittenBookContentState {
    pub title: DialogFilterableStringState<JavaString>,
    pub author: JavaString,
    pub generation: i32,
    pub pages: Vec<DialogFilterableComponentState>,
    pub resolved: bool,
}

pub(super) fn project_written_book_content(
    patch: &DataComponentPatchSummary,
    context: DialogDecodeContext<'_>,
) -> Result<DialogWrittenBookContentState, String> {
    let content = patch.written_book.as_ref().unwrap();
    let context = DialogDecodeContext {
        prepare_item_summary: false,
        ..context
    };
    Ok(DialogWrittenBookContentState {
        title: DialogFilterableStringState {
            raw: content.title.clone(),
            filtered: content.title_filter.clone(),
        },
        author: content.author.clone(),
        generation: content.generation,
        pages: patch
            .written_book_pages_nbt
            .iter()
            .zip(&patch.written_book_page_filters_nbt)
            .map(|(raw, filtered)| {
                Ok(DialogFilterableComponentState {
                    raw: decode_component(raw.as_ref().unwrap(), "written book page", context)?,
                    filtered: filtered
                        .as_ref()
                        .map(|value| decode_component(value, "written book page filter", context))
                        .transpose()?,
                })
            })
            .collect::<Result<_, String>>()?,
        resolved: content.resolved,
    })
}

#[cfg(test)]
mod tests;
