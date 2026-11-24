//! `BucketItem.use` (`BucketItem.java:43-95`) with `MobBucketItem`'s two
//! overrides (`MobBucketItem.java:31-56`): filling an empty bucket from the
//! cell the player looks at, and emptying a full one into it.

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::{
    vanilla_entity_type_id_for_resource_id, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaBucketFluid, VanillaBucketItemBehavior, VANILLA_ITEM_BUCKET_ID,
};
use bbb_protocol::packets::{
    BlockPos, ItemStackSummary, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::FluidFamily;
use bbb_server_types::geometry::clip::ClipHit;
use bbb_server_types::stats::StatKey;

use super::{single_item, ItemUseContext, ItemUseResult};
use crate::block_behavior::bucket_pickup::{pickup_block, pickup_sound};
use crate::connection::ConnectionId;
use crate::entity::SpawnReason;
use crate::fluid::{empty_bucket, liquid_block_container, BucketEmptied, BucketUser};
use crate::level_event::VanillaGameEvent;
use crate::world::advancement_triggers::FILLED_BUCKET;
use crate::world::block_use::spawn_egg::{load_entity_nbt, spawn_from_item};
use crate::world::pov_clip::{player_pov_clip, ClipFluid};
use crate::world::{LevelId, ServerWorld, WorldEvent};

/// The `BUCKET_ENTITY_DATA` keys `Bucketable.loadDefaultDataFromBucketTag`
/// (`Bucketable.java:62-69`) reads, plus the age `Axolotl` and `Tadpole`
/// add (`Axolotl.java:443-452`, `Tadpole.java:192-196`).
const BUCKET_TAG_KEYS: [&str; 8] = [
    "NoAI",
    "Silent",
    "NoGravity",
    "Glowing",
    "Invulnerable",
    "Health",
    "Age",
    "AgeLocked",
];

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let Some(behavior) = context
        .item
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|item| item.bucket_behavior)
    else {
        return ItemUseResult::Pass;
    };
    let fluid = match behavior.fluid {
        VanillaBucketFluid::Empty => None,
        VanillaBucketFluid::Water => Some(FluidFamily::Water),
        VanillaBucketFluid::Lava => Some(FluidFamily::Lava),
    };
    let mode = if fluid.is_none() {
        ClipFluid::SourceOnly
    } else {
        ClipFluid::None
    };
    let Some(hit) = player_pov_clip(world, context.connection_id, mode) else {
        return ItemUseResult::Pass;
    };
    if !world.may_use_item_at(context, hit.pos) {
        return ItemUseResult::Fail;
    }
    match fluid {
        None => fill(world, context, hit.pos, events),
        Some(fluid) => empty(world, context, behavior, fluid, hit, events),
    }
}

/// `:61-78`.
fn fill(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    pos: BlockPos,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let state = world.block_state_or_air(context.dimension, pos);
    let player = &world.levels[context.connection_id];
    let creative = player.game_type == GameType::Creative;
    let position = player.position();
    let player_entity_id = player.entity_id();
    let Some(taken) = world.with_level(context.dimension, |level| {
        pickup_block(level, pos, state, creative)
    }) else {
        return ItemUseResult::Fail;
    };
    world.levels[context.connection_id].award_stat_once(
        &mut world.scoreboard,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, VANILLA_ITEM_BUCKET_ID),
    );
    if let Some(registry_id) = pickup_sound(state).and_then(vanilla_sound_event_registry_id) {
        world.play_player_sound(
            context.connection_id,
            Some(context.connection_id),
            SoundEventHolder::Reference { registry_id },
            SoundSource::Players,
            position,
            1.0,
            1.0,
            events,
        );
    }
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::FLUID_PICKUP,
            pos,
            Some(player_entity_id),
            None,
        );
    let taken = single_item(taken);
    world.create_filled_result(context.connection_id, context.hand, taken.clone());
    world.trigger_item_criterion(context.connection_id, FILLED_BUCKET, &taken, events);
    ItemUseResult::Success
}

/// `:79-94`.
fn empty(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    behavior: VanillaBucketItemBehavior,
    fluid: FluidFamily,
    hit: ClipHit,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let dimension = context.dimension;
    let clicked = world.block_state_or_air(dimension, hit.pos);
    let place_pos = if fluid == FluidFamily::Water && liquid_block_container(clicked).is_some() {
        hit.pos
    } else {
        relative(hit.pos, hit.direction)
    };
    let player = &world.levels[context.connection_id];
    let user = BucketUser {
        shift_key_down: player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
        creative: player.game_type == GameType::Creative,
    };
    let infinite = player.abilities.instabuild;
    let player_entity_id = player.entity_id();
    let Some((at, emptied)) = world.with_level(dimension, |level| {
        empty_bucket(
            level,
            place_pos,
            fluid,
            user,
            Some((hit.pos, hit.direction)),
        )
    }) else {
        return ItemUseResult::Fail;
    };
    match (emptied, behavior.mob_empty_sound_resource_id) {
        (BucketEmptied::Evaporated, _) => {
            // `:126`.
            let pitch = 2.6 + (world.random.next_float() - world.random.next_float()) * 0.8;
            play_sound_at(
                world,
                context,
                at,
                "minecraft:block.fire.extinguish",
                SoundSource::Blocks,
                0.5,
                pitch,
                events,
            );
        }
        // `MobBucketItem.playEmptySound` (`MobBucketItem.java:39-41`) raises
        // no game event.
        (BucketEmptied::Placed, Some(sound)) => {
            play_sound_at(
                world,
                context,
                at,
                sound,
                SoundSource::Neutral,
                1.0,
                1.0,
                events,
            );
        }
        // `BucketItem.playEmptySound` (`:152-156`).
        (BucketEmptied::Placed, None) => {
            let sound = match fluid {
                FluidFamily::Water => "minecraft:item.bucket.empty",
                FluidFamily::Lava => "minecraft:item.bucket.empty_lava",
            };
            play_sound_at(
                world,
                context,
                at,
                sound,
                SoundSource::Blocks,
                1.0,
                1.0,
                events,
            );
            world
                .level_mut_by_id(&dimension)
                .event_sink
                .game_event_with_source(
                    VanillaGameEvent::FLUID_PLACE,
                    at,
                    Some(player_entity_id),
                    None,
                );
        }
    }
    if behavior.mob_empty_sound_resource_id.is_some() {
        spawn_bucketed_mob(
            world,
            Some(context.connection_id),
            dimension,
            &context.item,
            place_pos,
        );
        world
            .level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::ENTITY_PLACE,
                place_pos,
                Some(player_entity_id),
                None,
            );
    }
    world.trigger_item_used_on_location(
        context.connection_id,
        "minecraft:placed_block",
        place_pos,
        &context.item,
        events,
    );
    if let Some(item_id) = context.item.item_id {
        world.levels[context.connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    // `getEmptySuccessItem` (`:97-99`) hands a player with infinite materials
    // their own stack back, which `createFilledResult` then leaves alone.
    if !infinite {
        world.create_filled_result(
            context.connection_id,
            context.hand,
            single_item(VANILLA_ITEM_BUCKET_ID),
        );
    }
    ItemUseResult::Success
}

/// `MobBucketItem.spawn` (`MobBucketItem.java:43-55`): the bucket's mob is
/// named after the item (`cod_bucket` holds a `cod`).
pub(in crate::world) fn spawn_bucketed_mob(
    world: &mut ServerWorld,
    connection_id: Option<ConnectionId>,
    dimension: impl Into<LevelId>,
    item: &ItemStackSummary,
    pos: BlockPos,
) {
    let dimension = dimension.into();
    let Some(entity_type_id) = item
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|item| item.resource_id.strip_suffix("_bucket"))
        .and_then(vanilla_entity_type_id_for_resource_id)
    else {
        return;
    };
    let Some(entity_id) = spawn_from_item(
        world,
        connection_id,
        dimension,
        entity_type_id,
        pos,
        true,
        false,
        item,
        SpawnReason::Bucket,
    ) else {
        return;
    };
    let mut entries: Vec<NbtSummaryEntry> = match item
        .component_patch
        .codec_backed_nbt
        .get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID)
    {
        Some(NbtSummaryValue::Compound(entries)) => entries
            .iter()
            .filter(|entry| BUCKET_TAG_KEYS.iter().any(|key| entry.name == *key))
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    entries.push(NbtSummaryEntry {
        name: "FromBucket".into(),
        value: NbtSummaryValue::Byte(1),
    });
    load_entity_nbt(world, dimension, entity_id, &entries);
    // `Axolotl.loadFromBucketTag` (`Axolotl.java:443-452`).
    if entity_type_id == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
        let time_to_live = match item
            .component_patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID)
        {
            Some(NbtSummaryValue::Compound(tag)) => tag
                .iter()
                .find(|entry| entry.name == "HuntingCooldown")
                .and_then(|entry| crate::entity::save::numeric_tag_long_value(&entry.value)),
            _ => None,
        };
        if let Some(entity) = world
            .level_mut_by_id(&dimension)
            .entities
            .get_mut(entity_id)
        {
            entity.set_bucket_hunting_cooldown(time_to_live);
        }
    }
    world.play_spawn_ambient(dimension, entity_id);
}

#[allow(clippy::too_many_arguments)]
fn play_sound_at(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    pos: BlockPos,
    resource_id: &str,
    source: SoundSource,
    volume: f32,
    pitch: f32,
    events: &mut Vec<WorldEvent>,
) {
    let Some(registry_id) = vanilla_sound_event_registry_id(resource_id) else {
        return;
    };
    let sound = SoundEvent {
        sound: SoundEventHolder::Reference { registry_id },
        source,
        position: Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        volume,
        pitch,
        seed: world.random.next_long(),
    };
    world.broadcast_level_sound_by_key(
        &context.dimension,
        &sound,
        Some(context.connection_id),
        events,
    );
}
