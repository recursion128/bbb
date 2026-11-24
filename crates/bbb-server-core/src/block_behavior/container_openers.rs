use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::relative;

use super::{barrel, block_type_of, chest};

/// `ContainerOpenersCounter.CHECK_TICK_DELAY` (`ContainerOpenersCounter.java:16`).
const CHECK_TICK_DELAY: i32 = 5;

/// The volume all three `playSound` calls pass (`ChestBlockEntity.java:115`,
/// `BarrelBlockEntity.java:139`, `EnderChestBlockEntity.java:25-26`).
const LID_SOUND_VOLUME: f32 = 0.5;

/// `SoundEvents.ENDER_CHEST_OPEN` / `ENDER_CHEST_CLOSE`
/// (`EnderChestBlockEntity.java:23`, `:37`) as the codec names them.
pub const ENDER_CHEST_OPEN_SOUND: &str = "minecraft:block.ender_chest.open";
pub const ENDER_CHEST_CLOSE_SOUND: &str = "minecraft:block.ender_chest.close";

/// `BarrelBlock.OPEN` (`BarrelBlock.java:30`).
const OPEN: &str = "open";

/// Which block entity's `onOpen`, `onClose` and `openerCountChanged` the shared
/// counter runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpenersOwner {
    Chest,
    Barrel,
    EnderChest,
}

impl OpenersOwner {
    fn on_open(self, level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
        match self {
            Self::Chest => level.play_chest_sound(pos, chest_sound(block_state_id, true)),
            Self::Barrel => {
                // `:31-32`: the sound comes out before `OPEN` is written.
                play_sound(level, pos, barrel::BARREL_OPEN_SOUND);
                update_barrel_state(level, pos, block_state_id, true);
            }
            Self::EnderChest => play_sound(level, pos, ENDER_CHEST_OPEN_SOUND),
        }
    }

    fn on_close(self, level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
        match self {
            Self::Chest => level.play_chest_sound(pos, chest_sound(block_state_id, false)),
            Self::Barrel => {
                play_sound(level, pos, barrel::BARREL_CLOSE_SOUND);
                update_barrel_state(level, pos, block_state_id, false);
            }
            Self::EnderChest => play_sound(level, pos, ENDER_CHEST_CLOSE_SOUND),
        }
    }

    ///
    /// The barrel's is empty: its lid is the `OPEN` state, not a block event.
    fn opener_count_changed(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
        previous: i32,
        current: i32,
    ) {
        if self == Self::Barrel {
            return;
        }
        let Some(block_type_id) = block_type_of(block_state_id) else {
            return;
        };
        level.block_event(pos, block_type_id, chest::EVENT_SET_OPEN_COUNT, current);
        if previous != current && chest::is_trapped_chest(block_state_id) {
            level.update_neighbors_at(pos, block_type_id);
            level.update_neighbors_at(relative(pos, Direction::Down), block_type_id);
        }
    }
}

pub(crate) fn schedule_recheck(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    level.schedule_block_tick(pos, block, CHECK_TICK_DELAY, TickPriority::Normal);
}

pub(super) fn recheck_open(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    owner: OpenersOwner,
) {
    let Some(previous) = level.container_openers_count(pos) else {
        return;
    };
    let (current, range) = level.container_openers_users(pos);
    let current = current.max(0);
    level.set_container_openers_range(pos, range);
    if previous != current {
        if current != 0 && previous == 0 {
            owner.on_open(level, pos, block_state_id);
            level.game_event_from_entity(VanillaGameEvent::CONTAINER_OPEN, pos, None, None);
        } else if current == 0 {
            owner.on_close(level, pos, block_state_id);
            level.game_event_from_entity(VanillaGameEvent::CONTAINER_CLOSE, pos, None, None);
        }
        level.set_container_openers_count(pos, current);
    }
    owner.opener_count_changed(level, pos, block_state_id, previous, current);
    if current > 0 {
        schedule_recheck(level, pos, block_state_id);
    }
}

// Blocks.java:6384-6426; SoundEvents.java:405-410.
fn chest_sound(state: i32, opening: bool) -> &'static str {
    if !chest::is_copper_chest(state) {
        return if opening {
            chest::CHEST_OPEN_SOUND
        } else {
            chest::CHEST_CLOSE_SOUND
        };
    }
    let block = block_type_of(state).unwrap();
    let unwaxed = bbb_protocol::item_types::VANILLA_AXE_WAX_OFF
        .iter()
        .find(|transform| transform.source_block_type_id == block)
        .map_or(block, |transform| transform.target_block_type_id);
    let age = bbb_protocol::block_types::vanilla_block_type(unwaxed)
        .and_then(|block| block.change_over_time_age);
    match (age, opening) {
        (Some(2), true) => "minecraft:block.copper_chest_weathered.open",
        (Some(2), false) => "minecraft:block.copper_chest_weathered.close",
        (Some(3), true) => "minecraft:block.copper_chest_oxidized.open",
        (Some(3), false) => "minecraft:block.copper_chest_oxidized.close",
        (_, true) => "minecraft:block.copper_chest.open",
        (_, false) => "minecraft:block.copper_chest.close",
    }
}

impl LevelCtx<'_, '_> {
    pub(crate) fn change_chest_openers(
        &mut self,
        pos: BlockPos,
        source_entity_id: i32,
        interaction_range: f64,
        opening: bool,
    ) {
        let state = self.block_state(pos);
        if chest::is_chest_block(state) {
            change_openers(
                self,
                pos,
                state,
                OpenersOwner::Chest,
                source_entity_id,
                interaction_range,
                opening,
            );
        }
    }
}

/// `Level.playSound(null, x, y, z, event, BLOCKS, 0.5F, random.nextFloat() * 0.1F + 0.9F)`,
/// the one line all three counters end their `onOpen` and `onClose` with.
fn play_sound(level: &mut LevelCtx<'_, '_>, pos: BlockPos, sound_resource_id: &str) {
    let pitch = level.level_random().next_float().mul_add(0.1, 0.9);
    level.play_block_sound_with_volume_pitch(pos, sound_resource_id, LID_SOUND_VOLUME, pitch);
}

fn update_barrel_state(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    open: bool,
) {
    let Some(next) = with_property(block_state_id, OPEN, if open { "true" } else { "false" })
    else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

pub(super) fn change_openers(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    owner: OpenersOwner,
    source_entity_id: i32,
    range: f64,
    opening: bool,
) -> bool {
    let Some(previous) = level.container_openers_count(pos) else {
        return false;
    };
    let current = previous + if opening { 1 } else { -1 };
    level.set_container_openers_count(pos, current);
    if opening && previous == 0 {
        owner.on_open(level, pos, state);
        level.game_event_from_entity(
            VanillaGameEvent::CONTAINER_OPEN,
            pos,
            Some(source_entity_id),
            None,
        );
        schedule_recheck(level, pos, state);
    } else if !opening && current == 0 {
        owner.on_close(level, pos, state);
        level.game_event_from_entity(
            VanillaGameEvent::CONTAINER_CLOSE,
            pos,
            Some(source_entity_id),
            None,
        );
        level.set_container_openers_range(pos, 0.0);
    }
    owner.opener_count_changed(level, pos, state, previous, current);
    if opening {
        level.set_container_openers_range(pos, range.max(level.container_openers_range(pos)));
    }
    (opening && previous == 0) || (!opening && current == 0)
}
