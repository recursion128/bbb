//! `EnderEyeItem.use` (`EnderEyeItem.java:78-108`): an eye thrown toward the
//! nearest structure tagged `eye_of_ender_located`.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_END_PORTAL_FRAME_ID;
use bbb_protocol::packets::{
    BlockPos, ItemStackSummary, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::entity::eye_of_ender::signal_to;
use crate::entity::spawn_eye_of_ender;
use crate::level_event::VanillaGameEvent;
use crate::world::pov_clip::{player_pov_clip, ClipFluid};
use crate::world::{ServerWorld, WorldEvent};

const STRUCTURE_REGISTRY: &str = "minecraft:worldgen/structure";
const EYE_OF_ENDER_LOCATED: &str = "minecraft:eye_of_ender_located";

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    // `:77-80`.
    if player_pov_clip(world, context.connection_id, ClipFluid::None).is_some_and(|hit| {
        vanilla_block_state_block_type_id(world.block_state_or_air(context.dimension, hit.pos))
            == Some(VANILLA_BLOCK_END_PORTAL_FRAME_ID)
    }) {
        return ItemUseResult::Pass;
    }
    let player = &world.levels[context.connection_id];
    let position = player.position();
    let player_entity_id = player.entity_id();
    let height = player
        .bounding_box()
        .map_or(1.8, |bounds| bounds.max_y - bounds.min_y);
    let origin = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    // `:84-87`.
    let Some(feature) = nearest_eye_structure(world, context, origin) else {
        return ItemUseResult::Consume;
    };
    // `:89-92`: `getY(0.5)` and `setItem`'s single copy.
    let eye_position = Vec3d {
        y: position.y + height * 0.5,
        ..position
    };
    let item = RuntimeItemStack::from_summary(ItemStackSummary {
        count: 1,
        ..context.item.clone()
    });
    let store = &mut world.level_mut_by_id(&context.dimension).entities;
    if let Some(eye) = spawn_eye_of_ender(store, eye_position, item) {
        signal_to(
            store,
            eye,
            Vec3d {
                x: f64::from(feature.x),
                y: f64::from(feature.y),
                z: f64::from(feature.z),
            },
        );
    }
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            eye_position,
            Some(player_entity_id),
            None,
        );
    // `:94-96`.
    world.trigger_used_ender_eye(context.connection_id, feature, events);
    // `:98-99`: `Mth.lerp(nextFloat, 0.33F, 0.5F)`.
    let pitch = 0.33 + world.random.next_float() * (0.5 - 0.33);
    if let Some(registry_id) = vanilla_sound_event_registry_id("minecraft:entity.ender_eye.launch")
    {
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Neutral,
            position,
            volume: 1.0,
            pitch,
            seed: world.random.next_long(),
        };
        world.broadcast_level_sound_by_key(&context.dimension, &sound, None, events);
    }
    // `:100-101`.
    let player = &mut world.levels[context.connection_id];
    let infinite = player.abilities.instabuild;
    player.inventory.consume_in_hand(context.hand, 1, infinite);
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    ItemUseResult::SuccessServer
}

/// `ServerLevel.findNearestMapStructure(EYE_OF_ENDER_LOCATED, pos, 100, false)`;
/// a generator without structure placement finds nothing.
fn nearest_eye_structure(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    origin: BlockPos,
) -> Option<BlockPos> {
    let ids: Vec<String> = world
        .level_by_id(&context.dimension)
        .chunks
        .registry_tags()?
        .members(STRUCTURE_REGISTRY, EYE_OF_ENDER_LOCATED)
        .iter()
        .map(|id| id.to_string())
        .collect();
    world
        .level_mut_by_id(&context.dimension)
        .chunks
        .find_nearest_map_structure(origin, &ids, 100, false)
        .ok()
        .flatten()
        .map(|(pos, _)| pos)
}
