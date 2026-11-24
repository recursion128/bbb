//! Player mining and creative item drops for occupied hives.

use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::block_behavior::beehive::{honey_level, is_beehive};
use crate::block_entity::{BlockEntity, BlockEntityItemProjection};
use crate::connection::ConnectionId;
use crate::entity::spawn_dropped_item_entity;
use crate::world::{ServerWorld, WorldEvent};

impl ServerWorld {
    /// `BeehiveBlock.playerDestroy` (`BeehiveBlock.java:90-108`) after `super.playerDestroy`:
    /// empties occupants from the detached block entity, updates neighbours, angers nearby
    /// bees, and fires `BEE_NEST_DESTROYED` with the post-release occupant count.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_beehive_player_destroy(
        &mut self,
        player: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
        destroyed_with: &RuntimeItemStack,
        block_entity: Option<&mut BlockEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        if !is_beehive(state) {
            return;
        }
        let Some(hive) = block_entity.and_then(BlockEntity::beehive_mut) else {
            return;
        };
        let Some(prevents_bee_spawns) = self.stack_has_enchantment_in_tag(
            dimension,
            destroyed_with.summary(),
            "minecraft:prevents_bee_spawns_when_mining",
        ) else {
            return;
        };
        if !prevents_bee_spawns {
            let sedated = self.is_beehive_smokey(dimension, pos);
            self.release_beehive_emergency_occupants_from(
                dimension,
                pos,
                state,
                Some(player),
                sedated,
                Some(hive),
                events,
            );
            self.with_level(dimension, |level| {
                crate::block_behavior::containers::update_neighbours_after_destroy(
                    level, pos, state,
                );
            });
            self.anger_bees_near_hive(dimension, pos);
        }
        let bees_inside = hive.bees().len() as i32;
        self.trigger_bee_nest_destroyed(player, state, destroyed_with, bees_inside, events);
    }

    /// BeehiveBlock.java:289-306 preserves bees and honey in creative drops.
    pub(super) fn drop_creative_beehive(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
    ) {
        let dimension = dimension.into();
        if !self.game_rules.block_drops {
            return;
        }
        let resolver = self.item_stack_resolver();
        let Some(block_entity) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
        else {
            return;
        };
        let Some(hive) = block_entity.beehive() else {
            return;
        };
        let honey = honey_level(state);
        if hive.is_empty() && honey == 0 {
            return;
        }
        let Some(block) = vanilla_block_state_block_type_id(state).and_then(vanilla_block_type)
        else {
            return;
        };
        let collected = block_entity
            .collect_components(BlockEntityItemProjection::with_resolver(resolver.as_dyn()));
        let mut components = Vec::new();
        for component in collected.entries() {
            let value = match &component.value {
                Ok(value) => value.clone(),
                Err(error) => {
                    tracing::warn!("creative hive component: {error}");
                    return;
                }
            };
            components.push(entry(&component.id, value));
        }
        components.retain(|component| component.name != "minecraft:block_state");
        components.push(entry(
            "minecraft:block_state",
            NbtSummaryValue::Compound(vec![entry(
                "honey_level",
                NbtSummaryValue::string(honey.to_string()),
            )]),
        ));
        let saved = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::String(block.resource_id.into())),
            entry("count", NbtSummaryValue::Int(1)),
            entry("components", NbtSummaryValue::Compound(components)),
        ]);
        let stack = match RuntimeItemStack::try_read(&saved, &resolver) {
            Ok(stack) => stack,
            Err(error) => {
                tracing::warn!("creative hive drop: {error}");
                return;
            }
        };
        let position = Vec3d {
            x: pos.x.into(),
            y: pos.y.into(),
            z: pos.z.into(),
        };
        spawn_dropped_item_entity(
            &mut self.level_mut_by_id(&dimension).entities,
            position,
            stack,
        );
    }
}
