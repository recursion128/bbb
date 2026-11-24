//! `PumpkinBlock.useItemOn` (`PumpkinBlock.java:38-74`): shears carve the
//! pumpkin; anything else is `TRY_WITH_EMPTY_HAND`.

use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_CARVED_PUMPKIN_ID, VANILLA_BLOCK_PUMPKIN_ID};
use bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID;
use bbb_protocol::packets::{
    BlockPos, Direction, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::BlockUseContext;
use crate::block_behavior::BlockUseResult;
use crate::block_behavior::{default_block_state_id, SET_BLOCK_UPDATE_IMMEDIATE};
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::entity::spawn_item_entity;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::LootEntity;
use crate::world::simulation_level::block_entity_loot::{DropBlockEntity, DropTable};
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::block_queries::direction_serialized_name;
use bbb_server_types::directions::opposite;
use bbb_server_types::stats::StatKey;

/// `BuiltInLootTables.CARVE_PUMPKIN` (`BuiltInLootTables.java:108`).
const CARVE_LOOT_TABLE: &str = "minecraft:carve/pumpkin";

/// `SoundEvents.PUMPKIN_CARVE` (`PumpkinBlock.java:67`).
const PUMPKIN_CARVE_SOUND: &str = "minecraft:block.pumpkin.carve";

/// `Block.UPDATE_ALL_IMMEDIATE` (`Block.java:106`), the flags
/// `PumpkinBlock.useItemOn` writes the carved pumpkin with (`:68`).
const SET_BLOCK_UPDATE_ALL_IMMEDIATE: i32 = SET_BLOCK_UPDATE_ALL | SET_BLOCK_UPDATE_IMMEDIATE;

/// `CarvedPumpkinBlock.FACING` (`CarvedPumpkinBlock.java:33`).
const FACING: &str = "facing";

/// The `matches:` column: `Blocks.PUMPKIN`.
pub(super) fn is_pumpkin(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_PUMPKIN_ID)
}

/// The `item_on:` column: [`ServerWorld::use_pumpkin_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_pumpkin_item_on(context, events)
}

fn facing_step(direction: Direction) -> (i32, i32) {
    match direction {
        Direction::North => (0, -1),
        Direction::South => (0, 1),
        Direction::West => (-1, 0),
        Direction::East => (1, 0),
        Direction::Down | Direction::Up => (0, 0),
    }
}

fn carved_pumpkin(facing: Direction) -> Option<i32> {
    with_property(
        default_block_state_id(VANILLA_BLOCK_CARVED_PUMPKIN_ID),
        FACING,
        direction_serialized_name(facing),
    )
}

impl ServerWorld {
    /// `PumpkinBlock.useItemOn` (`PumpkinBlock.java:38-74`).
    pub(super) fn use_pumpkin_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        if context.selected_item.item_id() != Some(VANILLA_ITEM_SHEARS_ID) {
            return BlockUseResult::TryWithEmptyHand;
        }

        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let hand = context.update.hand;
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let source_entity_id = player.entity_id();
        let clicked = context.update.hit.direction;
        // `:50`: Y-axis click uses `player.getDirection().getOpposite()`.
        let facing = if matches!(clicked, Direction::Down | Direction::Up) {
            super::horizontal_direction(player.rotation().y_rot)
                .map(opposite)
                .unwrap_or(Direction::North)
        } else {
            clicked
        };

        let tool = loot_tool_for_runtime_stack(
            &context.selected_item,
            &self.datapack_tables.enchantment_registry,
        );
        let interacting = self.loot_view_for_player(self.levels.player_at(connection_id));
        let stacks = self.with_level(dimension, |level| {
            let drawn = level.draw_block_loot_at(
                pos,
                context.block_state_id,
                DropTable::Named(CARVE_LOOT_TABLE),
                DropBlockEntity::Live,
                &tool,
                LootEntity::Facts(&interacting),
            );
            drawn
                .into_iter()
                .flat_map(|request| request.stacks)
                .collect::<Vec<_>>()
        });
        for stack in stacks {
            self.spawn_carved_pumpkin_seeds(dimension, pos, facing, &stack);
        }

        if let Some(registry_id) = vanilla_sound_event_registry_id(PUMPKIN_CARVE_SOUND) {
            let seed = self.random.next_long();
            let sound = SoundEvent {
                sound: SoundEventHolder::Reference { registry_id },
                source: SoundSource::Blocks,
                position: Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y) + 0.5,
                    z: f64::from(pos.z) + 0.5,
                },
                volume: 1.0,
                pitch: 1.0,
                seed,
            };
            self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
        }

        if let Some(carved) = carved_pumpkin(facing) {
            self.set_block(dimension, pos, carved, SET_BLOCK_UPDATE_ALL_IMMEDIATE);
        }

        self.hurt_and_break_in_hand(connection_id, hand, 1, events);
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(VanillaGameEvent::SHEAR, pos, Some(source_entity_id), None);
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, VANILLA_ITEM_SHEARS_ID),
            );
        }
        BlockUseResult::Consumed
    }

    /// `new ItemEntity` at `pos + 0.5 + dir*0.65` on XZ, `pos.y + 0.1`, then
    /// `setDeltaMovement` (`PumpkinBlock.java:59-64`).
    fn spawn_carved_pumpkin_seeds(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        facing: Direction,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) {
        let dimension = dimension.into();
        let (step_x, step_z) = facing_step(facing);
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5 + f64::from(step_x) * 0.65,
            y: f64::from(pos.y) + 0.1,
            z: f64::from(pos.z) + 0.5 + f64::from(step_z) * 0.65,
        };
        let item = RuntimeItemStack::from_summary(stack.clone());
        let spawned = spawn_item_entity(
            &mut self.level_mut_by_id(&dimension).entities,
            position,
            item,
        );
        let Some(entity_id) = spawned else {
            tracing::warn!("the entity store refused the item entity's drawn UUID");
            return;
        };
        let level = self.level_mut_by_id(&dimension);
        let delta = Vec3d {
            x: 0.05 * f64::from(step_x) + level.random.next_double() * 0.02,
            y: 0.05,
            z: 0.05 * f64::from(step_z) + level.random.next_double() * 0.02,
        };
        if let Some(entity) = level.entities.get_mut(entity_id) {
            entity.set_delta_movement(delta);
        }
    }
}
