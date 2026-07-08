use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::SpawnerBlockEntityData,
};

const DEFAULT_SPAWN_DELAY: i32 = 20;
const DEFAULT_MIN_SPAWN_DELAY: i32 = 200;
const DEFAULT_REQUIRED_PLAYER_RANGE: i32 = 16;

pub(crate) fn decode_spawner_block_entity_data(
    raw_nbt: &[u8],
) -> Result<Option<SpawnerBlockEntityData>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };

    let entity_id = spawner_entity_id(&entries);
    let spawn_delay = nbt_i32(find_entry(&entries, "Delay")).unwrap_or(DEFAULT_SPAWN_DELAY);
    let min_spawn_delay =
        nbt_i32(find_entry(&entries, "MinSpawnDelay")).unwrap_or(DEFAULT_MIN_SPAWN_DELAY);
    let required_player_range = nbt_i32(find_entry(&entries, "RequiredPlayerRange"))
        .unwrap_or(DEFAULT_REQUIRED_PLAYER_RANGE);

    if entity_id.is_none()
        && find_entry(&entries, "Delay").is_none()
        && find_entry(&entries, "MinSpawnDelay").is_none()
        && find_entry(&entries, "RequiredPlayerRange").is_none()
    {
        return Ok(None);
    }

    Ok(Some(SpawnerBlockEntityData {
        entity_id,
        spawn_delay,
        min_spawn_delay,
        required_player_range,
    }))
}

fn spawner_entity_id(entries: &[(String, NbtValue)]) -> Option<String> {
    let NbtValue::Compound(spawn_data) = find_entry(entries, "SpawnData")? else {
        return None;
    };
    let nested = match find_entry(spawn_data, "entity") {
        Some(NbtValue::Compound(entity)) => entity,
        _ => spawn_data,
    };
    let Some(NbtValue::String(id)) = find_entry(nested, "id") else {
        return None;
    };
    (!id.is_empty()).then(|| id.clone())
}

fn nbt_i32(value: Option<&NbtValue>) -> Option<i32> {
    match value? {
        NbtValue::Short(value) => Some(i32::from(*value)),
        NbtValue::Int(value) => Some(*value),
        _ => None,
    }
}
