use std::collections::BTreeMap;
use std::sync::Arc;

use crate::random::GameRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    vanilla_block_state_identity, vanilla_block_state_support_facts, VANILLA_BLOCK_STATE_COUNT,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, vanilla_entity_type_eye_height,
    vanilla_entity_type_id_for_resource_id, vanilla_entity_type_is_serializable,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
};
use bbb_protocol::packets::{
    BlockPos, DataComponentPatchSummary, ItemStackSummary, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::block_behavior::{FallingBlockFallRequest, FallingBlockHurt};
use crate::entity::experience_orb::ExperienceOrbData;
use crate::entity::falling_block::{
    can_be_replaced_by_empty_handed_place, is_free, DEFAULT_GRAVITY,
};
use crate::entity::item::{
    is_mergable, merge_with_neighbours, plain_stack, spawn_dropped_item_entity, spawn_item_entity,
    stack_count, ItemEntityData, DEFAULT_HEALTH, DEFAULT_PICKUP_DELAY, INFINITE_LIFETIME,
    INFINITE_PICKUP_DELAY, LIFETIME, MERGE_RATE_STILL,
};
use crate::entity::save::{
    read_entity_chunk, read_entity_chunk_with_registry_tags, write_entity_chunk,
};
use crate::entity::store::{section_of_block, EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::{
    falling_block_scheduled_tick, mth_equal, spawn_requested_fallen_block, tick_entities,
    EntityData, FallingBlockData, RemovalReason,
};
use crate::level_event::LevelEventSink;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::block_queries::is_falling_block;
use bbb_server_types::geometry::{axis_step_order, collide_with_boxes, Aabb, Axis};

#[cfg(test)]
pub(crate) mod native_level;

mod cross_level;
mod move_emission;
mod patrol;
mod portal;

/// A queued entity sound without the seed the store's own sound stream draws.
#[derive(Debug, PartialEq)]
struct QueuedSound {
    registry_id: i32,
    source: SoundSource,
    position: Vec3d,
    volume: f32,
    pitch: f32,
}

fn queued_sounds(store: &mut EntityStore) -> Vec<QueuedSound> {
    store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => {
                let SoundEventHolder::Reference { registry_id } = sound.sound else {
                    panic!("an entity sound names a registry event")
                };
                Some(QueuedSound {
                    registry_id,
                    source: sound.source,
                    position: sound.position,
                    volume: sound.volume,
                    pitch: sound.pitch,
                })
            }
            _ => None,
        })
        .collect()
}

fn queued(resource_id: &str, position: Vec3d, volume: f32, pitch: f32) -> QueuedSound {
    QueuedSound {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
            .expect("the sound event is in the generated catalog"),
        source: SoundSource::Neutral,
        position,
        volume,
        pitch,
    }
}

/// `Block.defaultBlockState()` for a built-in 26.1 block.
fn state_id(resource_id: &str) -> i32 {
    vanilla_block_default_state_id(block_type_id(resource_id))
        .unwrap_or_else(|| panic!("{resource_id} is a built-in 26.1 block"))
}

/// One exact built-in 26.1 block state, named by its block and the complete
/// property map the generated registry files it under.
fn state_with(resource_id: &str, properties: &[(&str, &str)]) -> i32 {
    let mut map = BTreeMap::new();
    for (name, value) in properties {
        map.insert((*name).to_string(), (*value).to_string());
    }
    vanilla_block_state_id_by_name_and_properties(resource_id, &map)
        .unwrap_or_else(|| panic!("{resource_id}{properties:?} is a built-in 26.1 block state"))
}

fn block_type_id(resource_id: &str) -> i32 {
    vanilla_block_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in 26.1 block"))
        .id
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn falling_block_state(store: &EntityStore, id: i32) -> FallingBlockData {
    match store.get(id).expect("entity is live").data() {
        EntityData::FallingBlock(data) => data.clone(),
        other => panic!("expected a falling block, found {other:?}"),
    }
}

fn item_state(store: &EntityStore, id: i32) -> ItemEntityData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Item(data) => data.clone(),
        other => panic!("expected an item entity, found {other:?}"),
    }
}

fn assert_single_added_matches_store(store: &mut EntityStore, id: i32) {
    let stored = store.get(id).expect("entity is live").clone();
    assert_eq!(
        store.drain_lifecycle_events(),
        vec![EntityLifecycleEvent::Added {
            entity: stored,
            tracked: true,
        }]
    );
}

fn stack_of(item_resource_id: &str, count: i32) -> RuntimeItemStack {
    plain_stack(item_resource_id, count)
        .unwrap_or_else(|| panic!("{item_resource_id} x{count} is a built-in 26.1 stack"))
}

pub(crate) fn registry_tag_snapshot(tags: &[(&str, &str, &[&str])]) -> RegistryTagSnapshot {
    RegistryTagSnapshot::new(tags.iter().map(|(registry_id, tag_id, members)| {
        (
            (*registry_id).to_string(),
            (*tag_id).to_string(),
            members.iter().map(|member| (*member).to_string()).collect(),
        )
    }))
}

// ---------------------------------------------------------------------------
// Identity and storage
// ---------------------------------------------------------------------------

pub mod area_effect_cloud;
pub mod armor_stand;
pub mod arrow;
mod berry_controller;
pub mod block_triggers;
pub mod boat;
mod bubble_column;
pub mod chest_boat;
mod command_data;
mod creaking;
pub mod display;
mod drop_equipment;
pub mod egg;
pub mod end_crystal;
pub mod ender_pearl;
mod environment;
pub mod evoker_fangs;
pub mod experience;
pub mod experience_bottle;
mod experience_orb;
mod external_player_boat_control;
pub mod eye_of_ender;
mod falling_block;
pub mod fireball;
pub mod firework_rocket;
pub mod fishing_bobber;
pub mod general_living;
mod happy_ghast_state;
pub mod hurting_projectile;
pub mod interaction;
mod item;
mod kill;
mod leash;
pub mod lightning_bolt;
mod living_invulnerability;
pub mod llama_spit;
pub mod marker;
pub mod minecart;
pub mod minecart_container;
mod name_tag;
pub mod ominous_item_spawner;
mod passengers;
mod piston;
mod save_coverage;
pub mod shulker_bullet;
pub mod snowball;
pub mod spectral_arrow;
mod store;
mod teleport;
pub mod thrown_potion;
pub mod tnt;
pub mod trident;
mod undead_mount;
mod waypoints;
pub mod wind_charge;

#[path = "wandering_trader/tests.rs"]
mod wandering_trader;

mod nonliving_context;

mod climbable_history;
mod living_flight_context;
mod living_travel_float;

mod living_fall_refresh;
mod moving_piston_cursor;

mod living_fall_particles;

mod stored_fall_resetting_ray;
