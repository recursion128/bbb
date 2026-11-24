//! `FlowerPotBlock` potting and the potted-eyeblossom random tick.

use super::LevelCtx;
use std::collections::HashMap;
use std::sync::OnceLock;

use bbb_protocol::block_items::vanilla_block_item;
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, BlockTypeCatalogEntry,
    VanillaBlockUseItemOnOverride, VanillaRandomTickOverride, VANILLA_BLOCK_FLOWER_POT_ID,
    VANILLA_BLOCK_POTTED_CLOSED_EYEBLOSSOM_ID, VANILLA_BLOCK_POTTED_OPEN_EYEBLOSSOM_ID,
    VANILLA_BLOCK_TYPES,
};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::BlockPos;

use crate::fluid::SET_BLOCK_UPDATE_ALL;

use super::{block_type_of, default_block_state_id};

const OPEN_LONG_SOUND: &str = "minecraft:block.eyeblossom.open_long";
const CLOSE_LONG_SOUND: &str = "minecraft:block.eyeblossom.close_long";
const OPEN_PARTICLE_COLOR: i32 = 16_545_810;
const CLOSE_PARTICLE_COLOR: i32 = 6_250_335;

#[must_use]
pub fn is_flower_pot(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.random_tick_override)
        == Some(VanillaRandomTickOverride::FlowerPotBlock)
}

#[must_use]
pub fn is_empty_pot(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_FLOWER_POT_ID)
}

#[must_use]
pub fn empty_pot_state() -> i32 {
    default_block_state_id(VANILLA_BLOCK_FLOWER_POT_ID)
}

#[must_use]
pub fn potted_state_for_held_item(item_id: Option<i32>) -> Option<i32> {
    let content = vanilla_item_type(item_id?)?
        .block_item_behavior?
        .block_type_id;
    let pot_id = *potted_maps().by_content.get(&content)?;
    Some(default_block_state_id(pot_id))
}

#[must_use]
pub fn potted_content_item_id(block_state_id: i32) -> Option<i32> {
    let pot_id = block_type_of(block_state_id)?;
    if pot_id == VANILLA_BLOCK_FLOWER_POT_ID {
        return None;
    }
    potted_maps().item_by_potted.get(&pot_id).copied()
}

/// Catalog blocks whose `use_item_on_override` is `FlowerPotBlock`.
pub fn flower_pot_block_types() -> impl Iterator<Item = &'static BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES.iter().filter(|entry| {
        entry.use_item_on_override == Some(VanillaBlockUseItemOnOverride::FlowerPotBlock)
    })
}

struct PottedMaps {
    by_content: HashMap<i32, i32>,
    item_by_potted: HashMap<i32, i32>,
}

fn potted_maps() -> &'static PottedMaps {
    static MAPS: OnceLock<PottedMaps> = OnceLock::new();
    MAPS.get_or_init(|| {
        let mut by_content = HashMap::new();
        let mut item_by_potted = HashMap::new();
        for pot in flower_pot_block_types() {
            if let Some(content) = content_block_type_id(pot.resource_id) {
                by_content.insert(content, pot.id);
                if pot.id != VANILLA_BLOCK_FLOWER_POT_ID {
                    if let Some(item_id) = vanilla_block_item(content) {
                        item_by_potted.insert(pot.id, item_id);
                    }
                }
            }
        }
        PottedMaps {
            by_content,
            item_by_potted,
        }
    })
}

fn content_block_type_id(pot_resource_id: &str) -> Option<i32> {
    let content = match pot_resource_id {
        "minecraft:flower_pot" => "minecraft:air",
        "minecraft:potted_azalea_bush" => "minecraft:azalea",
        "minecraft:potted_flowering_azalea_bush" => "minecraft:flowering_azalea",
        id => {
            let rest = id.strip_prefix("minecraft:potted_")?;
            return vanilla_block_type_for_resource_id(&format!("minecraft:{rest}"))
                .map(|entry| entry.id);
        }
    };
    vanilla_block_type_for_resource_id(content).map(|entry| entry.id)
}

pub(crate) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    if !is_randomly_ticking(state) {
        return;
    }
    let currently_open = block_type_of(state) == Some(VANILLA_BLOCK_POTTED_OPEN_EYEBLOSSOM_ID);
    let Some(value) = level.eyeblossom_open(pos) else {
        return;
    };
    let should_be_open = value.to_boolean(currently_open);
    if currently_open == should_be_open {
        return;
    }
    let next_state = opposite_state(state);
    level.set_block(pos, next_state, SET_BLOCK_UPDATE_ALL, 512);
    spawn_transform_particle(level, pos, currently_open);
    level.play_block_sound(
        pos,
        if currently_open {
            CLOSE_LONG_SOUND
        } else {
            OPEN_LONG_SOUND
        },
    );
}

fn is_randomly_ticking(state: i32) -> bool {
    matches!(
        block_type_of(state),
        Some(VANILLA_BLOCK_POTTED_OPEN_EYEBLOSSOM_ID)
            | Some(VANILLA_BLOCK_POTTED_CLOSED_EYEBLOSSOM_ID)
    )
}

fn opposite_state(state: i32) -> i32 {
    if block_type_of(state) == Some(VANILLA_BLOCK_POTTED_OPEN_EYEBLOSSOM_ID) {
        default_block_state_id(VANILLA_BLOCK_POTTED_CLOSED_EYEBLOSSOM_ID)
    } else {
        default_block_state_id(VANILLA_BLOCK_POTTED_OPEN_EYEBLOSSOM_ID)
    }
}

fn spawn_transform_particle(level: &mut LevelCtx<'_, '_>, pos: BlockPos, was_open: bool) {
    let lifetime = 0.5 + level.level_random().next_double();
    let vx = level.level_random().next_double() - 0.5;
    let vy = level.level_random().next_double() + 1.0;
    let vz = level.level_random().next_double() - 0.5;
    let start_x = f64::from(pos.x) + 0.5;
    let start_y = f64::from(pos.y) + 0.5;
    let start_z = f64::from(pos.z) + 0.5;
    let target_x = start_x + vx * lifetime;
    let target_y = start_y + vy * lifetime;
    let target_z = start_z + vz * lifetime;
    let duration = (20.0 * lifetime) as i32;
    let color = if was_open {
        CLOSE_PARTICLE_COLOR
    } else {
        OPEN_PARTICLE_COLOR
    };
    level.send_trail_particle(
        [start_x, start_y, start_z],
        [target_x, target_y, target_z],
        color,
        duration,
    );
}
