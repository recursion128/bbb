//! Copper golem statue pose changes and axe activation.

use crate::world::LevelId;
use bbb_protocol::packets::{BlockPos, SoundEvent, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::BlockUseContext;
use crate::block_behavior::copper_golem_statue as copper_golem_statue_block;
use crate::block_behavior::BlockUseResult;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_copper_golem_statue_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_copper_golem_statue_item_on(context, events)
}

impl ServerWorld {
    /// `CopperGolemStatueBlock.useItemOn` (`CopperGolemStatueBlock.java:102-117`)
    /// and the weathering subclass (`WeatheringCopperGolemStatueBlock.java:51-82`).
    pub(super) fn use_copper_golem_statue_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let block_state_id = context.block_state_id;
        let hand = context.update.hand;
        let selected_item = &context.selected_item;
        if copper_golem_statue_block::item_is_axe(selected_item.item_id()) {
            let block_id =
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id);
            if block_id == Some(bbb_protocol::block_types::VANILLA_BLOCK_COPPER_GOLEM_STATUE_ID)
                && self.with_level_and_entities(dimension, |level, entities| {
                    let Some(custom_name) = level
                        .chunks
                        .block_entities()
                        .get(pos)
                        .and_then(|entity| entity.copper_golem_statue_custom_name())
                    else {
                        return false;
                    };
                    let Some(yaw) =
                        crate::block_behavior::direction_property(block_state_id, "facing")
                            .and_then(bbb_server_types::directions::facing_y_rot)
                    else {
                        return false;
                    };
                    // `CopperGolemStatueBlockEntity.java:26-43`: create without finalizing.
                    let Some(mut entity) = crate::entity::save::create_fresh_entity(
                        entities,
                        "minecraft:copper_golem",
                        Vec3d {
                            x: f64::from(pos.x) + 0.5,
                            y: f64::from(pos.y),
                            z: f64::from(pos.z) + 0.5,
                        },
                    ) else {
                        return false;
                    };
                    entity.set_custom_name(custom_name);
                    entity.set_spawn_rotation(yaw);
                    crate::entity::copper_golem::play_spawn_sound(&entity, entities);
                    entities.add_fresh(entity);
                    level.remove_block(pos);
                    true
                })
            {
                self.hurt_and_break_in_hand(connection_id, hand, 1, events);
                return BlockUseResult::Consumed;
            }
            return BlockUseResult::Pass;
        }
        if copper_golem_statue_block::is_weathering(block_state_id) {
            if copper_golem_statue_block::item_is_honeycomb(selected_item.item_id()) {
                return BlockUseResult::Pass;
            }
            if self
                .level_by_id(&dimension)
                .chunks
                .block_entities()
                .get(pos)
                .is_none()
            {
                return BlockUseResult::Pass;
            }
        }
        let Some(next) = copper_golem_statue_block::next_state(block_state_id) else {
            return BlockUseResult::Pass;
        };
        self.set_block(dimension, pos, next, crate::fluid::SET_BLOCK_UPDATE_ALL);
        self.play_copper_golem_statue_sound(dimension, pos, events);
        // `CopperGolemStatueBlock.java:122`.
        let source_entity_id = self
            .levels
            .player(connection_id)
            .map(|player| player.entity_id());
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(VanillaGameEvent::BLOCK_CHANGE, pos, source_entity_id, None);
        BlockUseResult::Consumed
    }

    pub(super) fn play_copper_golem_statue_sound(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) =
            vanilla_sound_event_registry_id(copper_golem_statue_block::POSE_SOUND)
        else {
            return;
        };
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
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    }
}
