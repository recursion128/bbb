use bbb_protocol::{visit_component_payloads, ComponentPayload, NbtSummaryValue};

use crate::RegistrySet;

use super::super::super::{decode_component, DialogTextState};
use super::component_list;

const MAX_LORE_LINES: usize = 256;

pub(super) fn decode_text_component(
    value: &NbtSummaryValue,
    what: &str,
    registries: Option<&RegistrySet>,
) -> Result<DialogTextState, String> {
    let component = decode_component(value, what)?;
    validate_component_payloads(value, what, registries)?;
    Ok(component)
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

fn validate_component_payloads(
    value: &NbtSummaryValue,
    what: &str,
    registries: Option<&RegistrySet>,
) -> Result<(), String> {
    visit_component_payloads(value, what, |payload, payload_what| match payload {
        ComponentPayload::ShowDialog(_) => Ok(()),
        ComponentPayload::ShowItemComponents(components) => {
            super::super::decode_data_component_patch(components, registries)
                .map(|_| ())
                .map_err(|error| format!("{payload_what}: {error}"))
        }
    })
}

#[cfg(test)]
mod tests;
