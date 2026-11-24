//! `ButtonBlock` interaction, scheduled reset, neighbour fan-out, and signals.
//!

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockRedstoneSignalOverride, VANILLA_BLOCK_BAMBOO_BUTTON_ID,
    VANILLA_BLOCK_CHERRY_BUTTON_ID, VANILLA_BLOCK_CRIMSON_BUTTON_ID,
    VANILLA_BLOCK_POLISHED_BLACKSTONE_BUTTON_ID, VANILLA_BLOCK_STONE_BUTTON_ID,
    VANILLA_BLOCK_WARPED_BUTTON_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{block_type_of, bool_property, footing, BlockInteractionPlayer};

const POWERED: &str = "powered";

#[must_use]
pub fn is_button(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.signal_override)
        == Some(VanillaBlockRedstoneSignalOverride::ButtonBlock)
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    if powered(block_state_id) {
        return true;
    }
    press(level, pos, block_state_id, player.source_entity_id());
    true
}

pub(super) fn press(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) {
    set_pressed(
        level,
        pos,
        block_state_id,
        true,
        source_entity_id,
        source_entity_id,
    );
    schedule_recheck(level, pos, block_state_id);
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if powered(block_state_id) {
        let _ = check_pressed(level, pos, block_state_id);
    }
}

pub(super) fn entity_inside(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    _inside: super::level::InsideEntity,
) -> bool {
    !can_arrows_press(block_state_id)
        || powered(block_state_id)
        || check_pressed(level, pos, block_state_id)
}

/// `ButtonBlock.checkPressed` (`ButtonBlock.java:162-178`).
fn check_pressed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> bool {
    let arrow = if can_arrows_press(state) {
        let Some(bounds) =
            bbb_server_types::geometry::static_block_outline_boxes(state).and_then(|boxes| {
                boxes
                    .into_iter()
                    .reduce(bbb_server_types::geometry::Aabb::minmax)
            })
        else {
            return false;
        };
        let bounds = bounds.moved(bbb_protocol::packets::Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        });
        let Some(arrow) = level.first_arrow_in_box(bounds) else {
            return false;
        };
        arrow
    } else {
        None
    };
    set_pressed(level, pos, state, arrow.is_some(), arrow, None);
    if arrow.is_some() {
        schedule_recheck(level, pos, state);
    }
    true
}

fn set_pressed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    pressed: bool,
    source: Option<i32>,
    sound_except: Option<i32>,
) {
    if powered(state) == pressed {
        return;
    }
    let Some(next) = with_property(state, POWERED, if pressed { "true" } else { "false" }) else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    footing::face_attached_update_neighbours(level, state, pos);
    // ButtonBlock.java:116-117: only the pressing player is spared the sound.
    level.play_block_sound_except(pos, button_sound(state, pressed), 1.0, 1.0, sound_except);
    level.game_event_from_entity(
        if pressed {
            crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE
        } else {
            crate::level_event::VanillaGameEvent::BLOCK_DEACTIVATE
        },
        pos,
        source,
        None,
    );
}

fn schedule_recheck(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    if let Some(block) = BlockTickType::of_block_state(state) {
        level.schedule_block_tick(
            pos,
            block,
            ticks_to_stay_pressed(state),
            TickPriority::Normal,
        );
    }
}

fn can_arrows_press(state: i32) -> bool {
    !matches!(
        block_type_of(state),
        Some(VANILLA_BLOCK_STONE_BUTTON_ID | VANILLA_BLOCK_POLISHED_BLACKSTONE_BUTTON_ID)
    )
}

#[must_use]
fn powered(block_state_id: i32) -> bool {
    bool_property(block_state_id, POWERED).unwrap_or(false)
}

#[must_use]
fn ticks_to_stay_pressed(block_state_id: i32) -> i32 {
    match block_type_of(block_state_id) {
        Some(VANILLA_BLOCK_STONE_BUTTON_ID | VANILLA_BLOCK_POLISHED_BLACKSTONE_BUTTON_ID) => 20,
        _ => 30,
    }
}

#[must_use]
fn button_sound(block_state_id: i32, pressed: bool) -> &'static str {
    match block_type_of(block_state_id) {
        Some(VANILLA_BLOCK_STONE_BUTTON_ID | VANILLA_BLOCK_POLISHED_BLACKSTONE_BUTTON_ID) => {
            if pressed {
                "minecraft:block.stone_button.click_on"
            } else {
                "minecraft:block.stone_button.click_off"
            }
        }
        Some(VANILLA_BLOCK_CHERRY_BUTTON_ID) => {
            if pressed {
                "minecraft:block.cherry_wood_button.click_on"
            } else {
                "minecraft:block.cherry_wood_button.click_off"
            }
        }
        Some(VANILLA_BLOCK_CRIMSON_BUTTON_ID | VANILLA_BLOCK_WARPED_BUTTON_ID) => {
            if pressed {
                "minecraft:block.nether_wood_button.click_on"
            } else {
                "minecraft:block.nether_wood_button.click_off"
            }
        }
        Some(VANILLA_BLOCK_BAMBOO_BUTTON_ID) => {
            if pressed {
                "minecraft:block.bamboo_wood_button.click_on"
            } else {
                "minecraft:block.bamboo_wood_button.click_off"
            }
        }
        _ if pressed => "minecraft:block.wooden_button.click_on",
        _ => "minecraft:block.wooden_button.click_off",
    }
}
