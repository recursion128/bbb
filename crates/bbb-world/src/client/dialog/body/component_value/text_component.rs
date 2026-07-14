use bbb_protocol::NbtSummaryValue;

use crate::RegistrySet;

use super::super::super::{decode_component, DialogTextState};
use super::component_list;

const MAX_LORE_LINES: usize = 256;

pub(super) fn decode_text_component(
    value: &NbtSummaryValue,
    what: &str,
    registries: Option<&RegistrySet>,
) -> Result<DialogTextState, String> {
    decode_component(value, what, registries)
}

pub(super) fn decode_lore(
    value: &NbtSummaryValue,
    what: &str,
    registries: Option<&RegistrySet>,
) -> Result<Vec<DialogTextState>, String> {
    let values = component_list(value).map_err(|_| format!("{what} is not a list"))?;
    if values.len() > MAX_LORE_LINES {
        return Err(format!(
            "{what} has {} lines, maximum is {MAX_LORE_LINES}",
            values.len()
        ));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_text_component(value, &format!("{what}[{index}]"), registries))
        .collect()
}

#[cfg(test)]
mod tests;
