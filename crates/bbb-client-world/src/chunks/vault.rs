use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::Result;

use super::{
    container_items::{summary_field, summary_item_stack_is_nonempty, summary_root},
    state::VaultSharedDataState,
};

const VANILLA_VAULT_DEFAULT_CONNECTED_PARTICLES_RANGE: f64 = 4.5;

pub(crate) fn decode_vault_shared_data(raw_nbt: &[u8]) -> Result<Option<VaultSharedDataState>> {
    let Some(NbtSummaryValue::Compound(entries)) = summary_root(raw_nbt)? else {
        return Ok(None);
    };
    let Some(NbtSummaryValue::Compound(shared_data)) = summary_field(&entries, "shared_data")
    else {
        return Ok(None);
    };

    let display_item = decode_display_item(shared_data);
    let connected_players = decode_connected_players(shared_data);
    let connected_particles_range = match summary_field(shared_data, "connected_particles_range") {
        Some(NbtSummaryValue::Double(bits)) => {
            let value = f64::from_bits(*bits);
            if value.is_finite() {
                value
            } else {
                VANILLA_VAULT_DEFAULT_CONNECTED_PARTICLES_RANGE
            }
        }
        _ => VANILLA_VAULT_DEFAULT_CONNECTED_PARTICLES_RANGE,
    };
    Ok(Some(VaultSharedDataState {
        display_item,
        connected_players,
        connected_particles_range,
    }))
}

/// `VaultSharedData.CODEC` stores the ejected reward under `display_item`;
/// `hasDisplayItem` is exactly "that stack is non-empty".
fn decode_display_item(entries: &[NbtSummaryEntry]) -> Option<NbtSummaryValue> {
    let value = summary_field(entries, "display_item")?;
    let NbtSummaryValue::Compound(display_item) = value else {
        return None;
    };
    summary_item_stack_is_nonempty(display_item).then(|| value.clone())
}

fn decode_connected_players(entries: &[NbtSummaryEntry]) -> Vec<Uuid> {
    let Some(NbtSummaryValue::List(players)) = summary_field(entries, "connected_players") else {
        return Vec::new();
    };
    players
        .iter()
        .filter_map(|player| match player {
            NbtSummaryValue::IntArray(values) => uuid_from_int_array(values),
            _ => None,
        })
        .collect()
}

fn uuid_from_int_array(values: &[i32]) -> Option<Uuid> {
    if values.len() != 4 {
        return None;
    }
    let high = u64::from(values[0] as u32) << 32 | u64::from(values[1] as u32);
    let low = u64::from(values[2] as u32) << 32 | u64::from(values[3] as u32);
    Some(Uuid::from_u128((u128::from(high) << 64) | u128::from(low)))
}
