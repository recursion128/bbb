use uuid::Uuid;

use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::VaultSharedDataState,
};

const VANILLA_VAULT_DEFAULT_CONNECTED_PARTICLES_RANGE: f64 = 4.5;

pub(crate) fn decode_vault_shared_data(raw_nbt: &[u8]) -> Result<Option<VaultSharedDataState>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(NbtValue::Compound(shared_data)) = find_entry(&entries, "shared_data") else {
        return Ok(None);
    };

    let connected_players = decode_connected_players(shared_data);
    let connected_particles_range = match find_entry(shared_data, "connected_particles_range") {
        Some(NbtValue::Double(value)) if value.is_finite() => *value,
        _ => VANILLA_VAULT_DEFAULT_CONNECTED_PARTICLES_RANGE,
    };
    Ok(Some(VaultSharedDataState {
        connected_players,
        connected_particles_range,
    }))
}

fn decode_connected_players(entries: &[(String, NbtValue)]) -> Vec<Uuid> {
    let Some(NbtValue::List(players)) = find_entry(entries, "connected_players") else {
        return Vec::new();
    };
    players
        .iter()
        .filter_map(|player| match player {
            NbtValue::IntArray(values) => uuid_from_int_array(values),
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
