//! Lodestone binding and tracked target validation.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_LODESTONE_ID;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID;
use bbb_protocol::item_types::VANILLA_ITEM_COMPASS_ID;
use bbb_protocol::packets::data_components::LodestoneTargetSummary;
use bbb_protocol::packets::{
    DataComponentPatchSummary, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::Java26LevelKey;

use super::ItemUseOnContext;
use crate::chunk::ChunkStore;
use crate::world::simulation_level::LevelCtx;
use crate::world::{ConnectionId, ServerWorld, WorldEvent};

/// `LodestoneTracker.tick` (`LodestoneTracker.java:26-38`). POI lookup also
/// covers saved columns without loading or generating their terrain.
fn updated_lodestone_compass(
    stack: &RuntimeItemStack,
    dimension: &Java26LevelKey,
    chunks: &mut ChunkStore,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Option<RuntimeItemStack> {
    let components = &stack.summary().component_patch;
    if stack.is_empty()
        || stack.item_id() != Some(VANILLA_ITEM_COMPASS_ID)
        || components.lodestone_tracked != Some(true)
    {
        return None;
    }
    let target = components.lodestone_target.as_ref()?;
    if target.dimension != dimension.as_str() {
        return None;
    }
    let pos = target.pos;
    if crate::world::level_blocks::is_in_world_bounds(chunks, pos)
        && chunks
            .poi_mut()
            .record([pos.x, pos.y, pos.z])
            .is_some_and(|record| record.type_id == "minecraft:lodestone")
    {
        return None;
    }
    stack
        .clone()
        .with_component_patch(
            &DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID],
                lodestone_tracked: Some(true),
                ..DataComponentPatchSummary::default()
            },
            resolver,
        )
        .ok()
}

impl LevelCtx<'_, '_> {
    pub(crate) fn updated_lodestone_compass(
        &mut self,
        stack: &RuntimeItemStack,
    ) -> Option<RuntimeItemStack> {
        updated_lodestone_compass(
            stack,
            self.dimension,
            self.chunks,
            self.loot_level_facts.item_stack_publication(),
        )
    }
}

impl ServerWorld {
    pub(in crate::world) fn tick_player_lodestone_compasses(
        &mut self,
        connection_id: ConnectionId,
    ) {
        let player = &self.levels[connection_id];
        if player.avatar_removed {
            return;
        }
        let (dimension, entity_id) = (player.dimension, player.entity_id());
        let resolver = self.item_stack_resolver();
        let level = self.level_mut_by_id(&dimension);
        let player = level
            .entities
            .player_mut(entity_id)
            .expect("online player slot");
        for index in 0.. {
            let Some(stack) = player.inventory.slot_mut(index) else {
                break;
            };
            if let Some(updated) =
                updated_lodestone_compass(stack, &level.dimension, &mut level.chunks, &resolver)
            {
                *stack = updated;
            }
        }
    }
}

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let dimension = context.dimension;
    let pos = context.pos();
    let state = world.level_by_id(&dimension).block_state(pos);
    if vanilla_block_state_block_type_id(state) != Some(VANILLA_BLOCK_LODESTONE_ID) {
        return false;
    }
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID],
        lodestone_target: Some(LodestoneTargetSummary {
            dimension: world.level_key(dimension).resource_id().to_owned(),
            pos,
        }),
        lodestone_tracked: Some(true),
        ..DataComponentPatchSummary::default()
    };
    let resolver = world.item_stack_resolver();
    let player = &mut world.levels[context.connection_id];
    let creative = player.abilities.instabuild;
    let inventory = &mut player.inventory;
    let original = inventory.item_in_hand_mut(context.hand());
    let Ok(mut bound) = original
        .with_count(1)
        .with_component_patch(&patch, &resolver)
    else {
        return false;
    };
    if !creative && original.count() == 1 {
        inventory.set_item_in_hand(context.hand(), bound);
    } else {
        inventory.consume_in_hand(context.hand(), 1, creative);
        if let Err(error) = inventory.add_anywhere(&resolver, &mut bound, creative) {
            tracing::warn!("lodestone compass output: {error}");
        }
        if !bound.is_empty() {
            world.drop_player_stack(context.connection_id, &bound, false);
        }
    }
    let sound = SoundEvent {
        sound: SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:item.lodestone_compass.lock")
                .expect("built-in lodestone compass lock sound"),
        },
        source: SoundSource::Players,
        position: Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        volume: 1.0,
        pitch: 1.0,
        seed: world.random.next_long(),
    };
    world.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    true
}
