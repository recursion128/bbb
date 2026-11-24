//! The seam between a live entity and the entity-region save form.
//!
//! `bbb_server_persistence` already owns the entity-region *schema*: it parses,
//! validates and writes `entities/r.X.Z.mca` for every Vanilla 26.1 entity type,
//! including subtype fields for types no runtime has ever produced. What it has
//! never had is anything to serialize. This module is that half for the types
//! this crate simulates, expressed as a round trip through
//! [`Java26EntityChunkData`].
//!
//! # Which fields are modelled
//!
//!
//! Passenger trees are written beneath their vehicle root and restored in the
//! same recursive order as `EntityType.loadEntityRecursive`.

use crate::world::simulation_level::LevelCtx;
pub(crate) use bbb_protocol::nbt_summary::codec_double_value as command_number_f64;
pub(crate) mod beehive;
mod fields;
mod internal_inventory;
mod living;
mod nonliving;
#[cfg(test)]
mod partial_load_tests;
mod projectile;
mod read_context;
mod record;
pub use read_context::EntityNbtReadContext;
use record::{read_entity_record, read_entity_record_fields};

pub use beehive::{create_beehive_occupant, create_beehive_occupant_with_context};
use fields::*;
pub(crate) use fields::{
    byte_bool, optional_block_pos, optional_bool, optional_byte, optional_double, optional_field,
    optional_float, optional_int, optional_long, optional_short, optional_uuid,
    read_block_pos_codec_input, read_block_state_from_value_with_registry, read_list_codec,
    read_uuid_codec_input, uuid_int_array,
};
pub use living::read_player_living_codec_fields;
use living::*;
pub(crate) use living::{is_living_base_save_field, read_mob_retained_state};
use nonliving::*;
pub(crate) use nonliving::{
    block_state_value_with_registry, write_block_pos, write_container_items,
};
mod passengers;
use passengers::{
    append_passenger_records, apply_vehicle_save_position, saved_entity_record_with_view,
};
use projectile::*;

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::entity_synched_data::{
    VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID, VANILLA_ITEM_FRAME_ITEM_DATA_ID,
    VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
};
use bbb_protocol::entity_types::{vanilla_entity_type, vanilla_entity_type_id_for_resource_id};
use bbb_protocol::equipment_slots::{
    vanilla_equipment_slot_for_serialized_name, VANILLA_EQUIPMENT_SLOTS,
};
use bbb_protocol::packets::{Direction, Vec3d};
use bbb_protocol::{JavaString, NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
use thiserror::Error;
use uuid::Uuid;

use crate::entity::area_effect_cloud;
use crate::entity::armor_stand::{self, ArmorStandData, ArmorStandPose, Rotations};
use crate::entity::arrow::{ArrowData, ArrowPickup};
use crate::entity::attributes::PackedAttribute;
use crate::entity::boat;
use crate::entity::chest_boat::{self, ChestBoatData, CONTAINER_SIZE as CHEST_BOAT_CONTAINER_SIZE};
use crate::entity::display;
use crate::entity::end_crystal;
use crate::entity::evoker_fangs::EvokerFangsData;
use crate::entity::experience_orb::{self, ExperienceOrbData};
use crate::entity::eye_of_ender;
use crate::entity::falling_block::FallingBlockData;
use crate::entity::firework_rocket::FireworkRocketData;
use crate::entity::fishing_bobber::FishingBobberData;
use crate::entity::general_living::{MobRetainedState, DEFAULT_MOB_EQUIPMENT_DROP_CHANCE};
use crate::entity::hurting_projectile::dragon_fireball::DragonFireballData;
use crate::entity::hurting_projectile::fireball::LargeFireballData;
use crate::entity::hurting_projectile::small_fireball::SmallFireballData;
use crate::entity::hurting_projectile::wind_charge::WindChargeData;
use crate::entity::hurting_projectile::wither_skull::WitherSkullData;
use crate::entity::hurting_projectile::{HurtingProjectileState, ProjectileState};
use crate::entity::interaction;
use crate::entity::item::{self, ItemEntityData};
use crate::entity::level::builtin_registry_tag_membership;
use crate::entity::lightning_bolt::LightningBoltData;
use crate::entity::llama_spit::LlamaSpitData;
use crate::entity::marker;
use crate::entity::minecart;
use crate::entity::minecart_container::{MinecartContainerData, MinecartContainerKind};
use crate::entity::shulker_bullet::ShulkerBulletData;
use crate::entity::spectral_arrow::SpectralArrowData;
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile::ThrowableItemData;
use crate::entity::tnt::TntData;
use crate::entity::trident::TridentData;
use crate::entity::{ActiveMobEffect, Entity, EntityData, LastHurtByMob};
use crate::level_ticks::ChunkKey;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_persistence::entity_schema::entity_chunk_document;
use bbb_server_persistence::{Java26EntityChunkData, Java26EntityRecord, Java26EntityRecordRef};
use bbb_server_types::save::{NamedCompound, SaveError};

/// What can go wrong translating between an entity and its save record.
#[derive(Debug, Error)]
pub enum EntitySaveError {
    /// The entity-region schema refused the document. This is the same error
    /// the region reader raises, so a caller can treat both alike.
    #[error(transparent)]
    Schema(#[from] SaveError),
    /// A field the modelled entity needs is missing, of the wrong type, or
    /// names something outside the built-in 26.1 registries.
    #[error("entity record field {field} is unusable: {detail}")]
    Field { field: &'static str, detail: String },
    /// The record names an entity type this slice has no runtime for. Its
    /// schema is valid; there is simply nothing to simulate it with.
    #[error("entity type {0} has no runtime in this server")]
    UnsupportedEntityType(String),
    /// The store refused the record: its UUID is already live.
    #[error("entity UUID {0} is already registered")]
    DuplicateUuid(Uuid),
}

fn field_error(field: &'static str, detail: impl Into<String>) -> EntitySaveError {
    EntitySaveError::Field {
        field,
        detail: detail.into(),
    }
}

pub(crate) fn entity_transfer_record(
    store: &EntityStore,
    entity: &Entity,
) -> Result<Java26EntityRecord, EntitySaveError> {
    let entries = entity_record(entity, false, Some(store))?;
    let position = entity.position();
    Ok(Java26EntityRecord::new(
        entries,
        entity.data().entity_resource_id(),
        entity.uuid(),
        [position.x, position.y, position.z],
        Vec::new(),
    ))
}

pub(crate) fn load_entity_transfer_record(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    context: Option<EntityNbtReadContext<'_>>,
) -> Result<Box<Entity>, EntitySaveError> {
    let publication = store.item_publication();
    let mut tags = |registry: &str, tag: &str, element: &str| match context {
        Some(context) => context.registry_tags.contains(registry, tag, element),
        None => builtin_registry_tag_membership(registry, tag, element),
    };
    let id = read_entity_record(store, record, context, &mut tags, publication.resolver())?;
    store
        .take_fresh(id)
        .ok_or_else(|| field_error("id", "a loaded entity left the store"))
}

pub(crate) fn write_active_mob_effect(effect: &ActiveMobEffect) -> NbtSummaryValue {
    effect.to_saved_nbt()
}

/// Serializes every entity filed in one chunk column into the entity-region
/// chunk form.
pub fn write_entity_chunk(
    store: &EntityStore,
    chunk: ChunkKey,
) -> Result<Java26EntityChunkData, EntitySaveError> {
    Ok(Java26EntityChunkData::parse_with_registry(
        write_entity_chunk_excluding(store, chunk, &rustc_hash::FxHashSet::default())?,
        chunk.0,
        chunk.1,
        store.plugins().content_registry(),
    )?)
}

pub(crate) fn write_entity_chunk_excluding(
    store: &EntityStore,
    chunk: ChunkKey,
    excluded: &rustc_hash::FxHashSet<Uuid>,
) -> Result<NamedCompound, EntitySaveError> {
    let records = entity_chunk_records(store, chunk, excluded)?;
    Ok(entity_chunk_document(
        chunk.0,
        chunk.1,
        records.into_iter().map(Java26EntityRecord::into_raw),
    ))
}

pub(crate) fn entity_chunk_records(
    store: &EntityStore,
    chunk: ChunkKey,
    excluded: &rustc_hash::FxHashSet<Uuid>,
) -> Result<Vec<Java26EntityRecord>, EntitySaveError> {
    let mut records = Vec::new();
    for id in store.iter_chunk_section_ids(chunk) {
        if store.vehicle(id).is_some() || store.external_player_count_in_tree(id) == 1 {
            continue;
        }
        let Some(entity) = store.get(id) else {
            continue;
        };
        if excluded.contains(&entity.uuid()) {
            continue;
        }
        if entity
            .removal_reason()
            .is_some_and(|reason| !reason.should_save())
        {
            continue;
        }
        if !(bbb_protocol::entity_types::vanilla_entity_type_is_serializable(
            entity.entity_type_id(),
        ) || store
            .plugins()
            .content_registry()
            .entity_by_id(entity.entity_type_id())
            .is_some_and(|spec| spec.serializable))
        {
            continue;
        }
        records.push(passengers::saved_entity_record_excluding(
            crate::entity::store::EntityBodyReadView::new(store, Some(entity), None),
            entity,
            excluded,
        )?);
    }
    Ok(records)
}

pub(crate) fn write_player_entity_tree(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
) -> Result<Java26EntityRecord, EntitySaveError> {
    saved_entity_record_with_view(view, entity)
}

pub(crate) fn read_player_entity_tree(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    context: EntityNbtReadContext<'_>,
) -> Option<i32> {
    let publication = store.item_publication();
    let mut allocations = 0;
    read_entity_record_tree(
        store,
        record,
        &mut |registry, tag, element| context.registry_tags.contains(registry, tag, element),
        Some(context),
        publication.resolver(),
        "ender_pearls".into(),
        Some(&mut allocations),
    )
}

pub fn write_structure_template_entity(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
) -> Result<NbtSummaryValue, EntitySaveError> {
    if view.store().vehicle(entity.id()).is_some()
        || entity
            .removal_reason()
            .is_some_and(|reason| !reason.should_save())
        || !(bbb_protocol::entity_types::vanilla_entity_type_is_serializable(
            entity.entity_type_id(),
        ) || view
            .store()
            .plugins()
            .content_registry()
            .entity_by_id(entity.entity_type_id())
            .is_some_and(|spec| spec.serializable))
    {
        return Ok(NbtSummaryValue::Compound(Vec::new()));
    }
    Ok(NbtSummaryValue::Compound(
        saved_entity_record_with_view(view, entity)?.into_raw(),
    ))
}

/// Writes the external player's `RootVehicle` payload as its direct Attach
/// UUID plus the root entity tree.
pub fn write_external_player_root_vehicle(
    store: &EntityStore,
    player_entity_id: i32,
) -> Result<Option<(Uuid, Java26EntityRecord)>, EntitySaveError> {
    external_player_root_vehicle_parts(
        crate::entity::store::EntityBodyReadView::new(store, None, None),
        player_entity_id,
    )
}

pub fn write_external_player_root_vehicle_value(
    store: &EntityStore,
    player_entity_id: i32,
) -> Result<Option<bbb_server_persistence::Java26RootVehicle>, EntitySaveError> {
    write_external_player_root_vehicle_value_with_view(
        crate::entity::store::EntityBodyReadView::new(store, None, None),
        player_entity_id,
    )
}

pub fn write_external_player_root_vehicle_value_with_view(
    view: crate::entity::store::EntityBodyReadView<'_>,
    player_entity_id: i32,
) -> Result<Option<bbb_server_persistence::Java26RootVehicle>, EntitySaveError> {
    let Some((attach, record)) = external_player_root_vehicle_parts(view, player_entity_id)? else {
        return Ok(None);
    };
    let vehicle = bbb_server_persistence::Java26RootVehicle::new(attach, record);
    Ok(Some(vehicle))
}

fn external_player_root_vehicle_parts(
    view: crate::entity::store::EntityBodyReadView<'_>,
    player_entity_id: i32,
) -> Result<Option<(Uuid, Java26EntityRecord)>, EntitySaveError> {
    let Some(direct_vehicle_id) = view.external_player_vehicle(player_entity_id) else {
        return Ok(None);
    };
    let Some(root_vehicle_id) = view.external_player_root_vehicle(player_entity_id) else {
        return Ok(None);
    };
    if view.external_player_count_in_tree(root_vehicle_id) != 1 {
        return Ok(None);
    }
    let direct_uuid = view
        .get(direct_vehicle_id)
        .ok_or_else(|| field_error("RootVehicle.Attach", "direct vehicle is not live"))?
        .uuid();
    let root = view
        .get(root_vehicle_id)
        .ok_or_else(|| field_error("RootVehicle.Entity", "root vehicle is not live"))?;
    let record = saved_entity_record_with_view(view, root)?;
    Ok(Some((direct_uuid, record)))
}

/// Loads a saved `RootVehicle.Entity` tree and force-attaches the
/// external player to the unique entity named by Attach.
pub fn read_external_player_root_vehicle(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    attach_uuid: Uuid,
    player_entity_id: i32,
) -> Result<i32, EntitySaveError> {
    read_external_player_root_vehicle_with_registry_tag_lookup(
        store,
        record,
        attach_uuid,
        player_entity_id,
        builtin_registry_tag_membership,
    )
}

pub fn read_external_player_root_vehicle_with_registry_tags(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    attach_uuid: Uuid,
    player_entity_id: i32,
    registry_tags: &RegistryTagSnapshot,
) -> Result<i32, EntitySaveError> {
    read_external_player_root_vehicle_with_registry_tag_lookup(
        store,
        record,
        attach_uuid,
        player_entity_id,
        |registry_id, tag_id, element_resource_id| {
            registry_tags.contains(registry_id, tag_id, element_resource_id)
        },
    )
}

fn read_external_player_root_vehicle_with_registry_tag_lookup(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    attach_uuid: Uuid,
    player_entity_id: i32,
    registry_tag_membership: impl FnMut(&str, &str, &str) -> bool,
) -> Result<i32, EntitySaveError> {
    read_external_player_root_vehicle_with_registry_tag_lookup_and_context(
        store,
        record,
        RootVehicleAttachInput::Resolved(attach_uuid),
        player_entity_id,
        registry_tag_membership,
        None,
    )
    .and_then(|root| {
        root.ok_or_else(|| field_error("RootVehicle.Entity", "native load skipped the root entity"))
    })
}

pub fn read_external_player_root_vehicle_with_context(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    attach_uuid: Uuid,
    player_entity_id: i32,
    context: EntityNbtReadContext<'_>,
) -> Result<Option<i32>, EntitySaveError> {
    read_external_player_root_vehicle_with_registry_tag_lookup_and_context(
        store,
        record,
        RootVehicleAttachInput::Resolved(attach_uuid),
        player_entity_id,
        |registry, tag, element| context.registry_tags.contains(registry, tag, element),
        Some(context),
    )
}

pub fn read_external_player_root_vehicle_input_with_context(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    attach_input: Option<&NbtSummaryValue>,
    player_entity_id: i32,
    context: EntityNbtReadContext<'_>,
) -> Result<Option<i32>, EntitySaveError> {
    read_external_player_root_vehicle_with_registry_tag_lookup_and_context(
        store,
        record,
        RootVehicleAttachInput::Raw(attach_input),
        player_entity_id,
        |registry, tag, element| context.registry_tags.contains(registry, tag, element),
        Some(context),
    )
}

enum RootVehicleAttachInput<'a> {
    Resolved(Uuid),
    Raw(Option<&'a NbtSummaryValue>),
}

fn read_external_player_root_vehicle_with_registry_tag_lookup_and_context(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    attach_input: RootVehicleAttachInput<'_>,
    player_entity_id: i32,
    mut registry_tag_membership: impl FnMut(&str, &str, &str) -> bool,
    context: Option<EntityNbtReadContext<'_>>,
) -> Result<Option<i32>, EntitySaveError> {
    if let RootVehicleAttachInput::Resolved(attach_uuid) = &attach_input {
        if context.is_none() && !record.contains_uuid(*attach_uuid) {
            return Err(field_error(
                "RootVehicle.Attach",
                "UUID does not name an entity in RootVehicle.Entity",
            ));
        }
    }
    let publication = store.item_publication();
    let mut post_load_allocations = 0;
    {
        let root_id = read_entity_record_tree(
            store,
            record,
            &mut registry_tag_membership,
            context,
            publication.resolver(),
            "RootVehicle.Entity".into(),
            context.is_some().then_some(&mut post_load_allocations),
        );
        let Some(root_id) = root_id else {
            if context.is_some() {
                return Ok(None);
            }
            return Err(field_error(
                "RootVehicle.Entity",
                "native load skipped the root entity",
            ));
        };
        let attach_uuid = match attach_input {
            RootVehicleAttachInput::Resolved(uuid) => Some(uuid),
            RootVehicleAttachInput::Raw(raw) => raw.and_then(read_uuid_codec_input),
        };
        let vehicle_id = attach_uuid
            .and_then(|uuid| store.entity_id_for_uuid(uuid))
            .filter(|id| *id == root_id || store.has_indirect_passenger(root_id, *id));
        if vehicle_id.is_some_and(|vehicle_id| {
            store.attach_external_player_forced(
                player_entity_id,
                vehicle_id,
                store.external_player_insertion_index(vehicle_id),
            )
        }) {
            return Ok(Some(root_id));
        }
        store.remove(root_id, crate::entity::RemovalReason::Discarded);
        if context.is_some() {
            return Ok(None);
        }
        Err(field_error(
            "RootVehicle.Attach",
            "loaded tree could not attach the player",
        ))
    }
}

///
/// Returns the new ids in record order.
pub fn read_entity_chunk(
    store: &mut EntityStore,
    data: &Java26EntityChunkData,
) -> Result<Vec<i32>, EntitySaveError> {
    read_entity_chunk_with_registry_tag_lookup(store, data, builtin_registry_tag_membership)
}

pub fn read_entity_chunk_with_registry_tags(
    store: &mut EntityStore,
    data: &Java26EntityChunkData,
    registry_tags: &RegistryTagSnapshot,
) -> Result<Vec<i32>, EntitySaveError> {
    read_entity_chunk_with_registry_tag_lookup(
        store,
        data,
        |registry_id, tag_id, element_resource_id| {
            registry_tags.contains(registry_id, tag_id, element_resource_id)
        },
    )
}

fn read_entity_chunk_with_registry_tag_lookup(
    store: &mut EntityStore,
    data: &Java26EntityChunkData,
    registry_tag_membership: impl FnMut(&str, &str, &str) -> bool,
) -> Result<Vec<i32>, EntitySaveError> {
    read_entity_chunk_with_registry_tag_lookup_and_context(
        store,
        data,
        registry_tag_membership,
        None,
    )
}

pub fn read_entity_chunk_with_context(
    store: &mut EntityStore,
    data: &Java26EntityChunkData,
    context: EntityNbtReadContext<'_>,
) -> Result<Vec<i32>, EntitySaveError> {
    read_entity_chunk_with_registry_tag_lookup_and_context(
        store,
        data,
        |registry, tag, element| context.registry_tags.contains(registry, tag, element),
        Some(context.with_section_registration()),
    )
}

pub(crate) fn armor_stand_teams(
    data: &Java26EntityChunkData,
) -> rustc_hash::FxHashMap<Uuid, &bbb_protocol::JavaString> {
    fn visit<'a>(
        record: Java26EntityRecordRef<'a>,
        teams: &mut rustc_hash::FxHashMap<Uuid, &'a bbb_protocol::JavaString>,
    ) {
        if record.id() == "minecraft:armor_stand" {
            if let Some(NbtSummaryValue::String(team)) = optional_field(record.raw(), "Team") {
                teams.insert(record.uuid(), team);
            }
        }
        for passenger in record.passengers() {
            visit(passenger, teams);
        }
    }
    let mut teams = rustc_hash::FxHashMap::default();
    for record in data.entities() {
        visit(record, &mut teams);
    }
    teams
}

fn read_entity_chunk_with_registry_tag_lookup_and_context(
    store: &mut EntityStore,
    data: &Java26EntityChunkData,
    mut registry_tag_membership: impl FnMut(&str, &str, &str) -> bool,
    context: Option<EntityNbtReadContext<'_>>,
) -> Result<Vec<i32>, EntitySaveError> {
    let publication = store.item_publication();
    let mut ids = Vec::with_capacity(data.entities().len());
    for (index, record) in data.entities().enumerate() {
        ids.extend(read_entity_record_tree(
            store,
            record,
            &mut registry_tag_membership,
            context,
            publication.resolver(),
            format!("Entities[{index}]"),
            None,
        ));
    }
    Ok(ids)
}

///
/// The warm-up is this level's own store and nothing else, because the arm it
/// writes is an entity id of that store. A reference to an entity standing in
/// another dimension keeps the UUID arm -- which still saves the UUID Vanilla
/// holds -- and the tick's own lookup resolves it through the projection of the
/// levels beside this one (`entity_reference.rs`).
pub fn resolve_saved_last_hurt_by_mob_references(store: &mut EntityStore) {
    let unresolved: Vec<(i32, Uuid, i32)> = store
        .bodies()
        .filter_map(|entity| {
            let living = entity.living()?;
            let reference @ LastHurtByMob::Uuid(uuid) = living.last_hurt_by_mob()? else {
                return None;
            };
            let _ = reference;
            Some((entity.id(), uuid, living.last_hurt_by_mob_timestamp))
        })
        .collect();
    for (entity_id, uuid, timestamp) in unresolved {
        let Some(resolved) = LastHurtByMob::Uuid(uuid).stored_uuid_target(store) else {
            continue;
        };
        if let Some(living) = store.get_mut(entity_id).and_then(Entity::living_mut) {
            living.set_last_hurt_by_mob(
                Some(LastHurtByMob::Stored {
                    entity_id: resolved,
                    uuid: Some(uuid),
                }),
                timestamp,
            );
        }
    }
}

fn read_entity_record_tree(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    registry_tag_membership: &mut impl FnMut(&str, &str, &str) -> bool,
    context: Option<EntityNbtReadContext<'_>>,
    items: crate::entity::item::ItemResolver<'_>,
    path: String,
    mut post_load_allocations: Option<&mut usize>,
) -> Option<i32> {
    let vehicle_id = {
        let result = if let (Some(context), Some(allocations)) =
            (context, post_load_allocations.as_deref_mut())
        {
            record::read_entity_record_with_post_load(
                store,
                record,
                context,
                registry_tag_membership,
                items,
                allocations,
            )
        } else {
            read_entity_record(store, record, context, registry_tag_membership, items)
                .map(crate::entity::store::EntityRestoreOutcome::Published)
        };
        match result {
            Ok(crate::entity::store::EntityRestoreOutcome::Published(id)) => id,
            Ok(_) => return None,
            Err(error) => {
                tracing::warn!(
                    entity = %record.id(),
                    uuid = %record.uuid(),
                    %path,
                    %error,
                    "skipping unreadable entity"
                );
                return None;
            }
        }
    };
    for (index, passenger) in record.passengers().enumerate() {
        let Some(passenger_id) = read_entity_record_tree(
            store,
            passenger,
            registry_tag_membership,
            context,
            items,
            format!("{path}.Passengers[{index}]"),
            post_load_allocations.as_deref_mut(),
        ) else {
            continue;
        };
        if !store.link_existing_passenger_forced(vehicle_id, passenger_id) {
            tracing::warn!(vehicle = %record.id(), %path, "saved passenger left unmounted");
        }
    }
    Some(vehicle_id)
}

fn load_entity_creation(
    store: &mut EntityStore,
    spawn_data: &[NbtSummaryEntry],
    position: Vec3d,
    context: Option<EntityNbtReadContext<'_>>,
) -> Result<i32, EntitySaveError> {
    // EntityType.create(ValueInput) constructs once, then loads the supplied fields.
    let resource_id = spawn_data
        .iter()
        .find(|entry| entry.name == "id")
        .and_then(|entry| match &entry.value {
            NbtSummaryValue::String(id) => id
                .with_utf8(|raw| bbb_protocol::normalize_resource_location(raw))
                .and_then(Result::ok),
            _ => None,
        })
        .ok_or_else(|| field_error("id", "spawner entity id is not a built-in entity"))?;
    // The reader receives the resource id separately.
    let entries = if spawn_data[0].name == "id" {
        &spawn_data[1..]
    } else {
        spawn_data
    };
    if optional_float_pair(entries, "Rotation")
        .is_some_and(|rotation| rotation.iter().any(|angle| !angle.is_finite()))
    {
        return Err(field_error("Rotation", "rotation must be finite"));
    }
    let uuid = optional_uuid(entries, "UUID").unwrap_or_else(|| store.next_uuid());
    let mut tags = |registry: &str, tag: &str, element: &str| match context {
        Some(context) => context.registry_tags.contains(registry, tag, element),
        None => builtin_registry_tag_membership(registry, tag, element),
    };
    let publication = store.item_publication();
    let id = read_entity_record_fields(
        store,
        record::EntityRecordFields {
            resource_id: &resource_id,
            uuid,
            position: [position.x, position.y, position.z],
            entries,
            command_existing: None,
        },
        None,
        context,
        &mut tags,
        publication.resolver(),
    )?;
    // ItemEntity.java:326-335: an empty stack discards the body while reading NBT.
    if store
        .get(id)
        .is_some_and(|entity| entity.removal_reason().is_some())
    {
        if let Some(entity) = store.take_fresh(id) {
            store.discard_fresh(entity);
        }
        return Err(field_error("id", "entity was removed during creation"));
    }
    Ok(id)
}

/// `EntityType.create` by resource id: a fresh body, not yet added.
pub fn create_fresh_entity(
    store: &mut EntityStore,
    resource_id: &str,
    position: Vec3d,
) -> Option<Box<Entity>> {
    let resource_id = bbb_protocol::normalize_resource_location(resource_id).ok()?;
    let entity_type_id = vanilla_entity_type_id_for_resource_id(&resource_id).or_else(|| {
        store
            .plugins()
            .content_registry()
            .entity_by_resource_id(&resource_id)
            .map(|spec| spec.id)
    })?;
    let id = if let Some(data) =
        crate::entity::GeneralLivingData::new_with_plugins(entity_type_id, store.plugins())
    {
        store.spawn_general_living_data(entity_type_id, position, data)?
    } else {
        load_entity_creation(
            store,
            &[NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String(resource_id.into()),
            }],
            position,
            None,
        )
        .ok()?
    };
    store.take_fresh(id)
}

pub fn load_fresh_entity_tree(
    store: &mut EntityStore,
    plan: &crate::entity::AtomicSpawnPlan,
) -> Option<Box<Entity>> {
    load_fresh_entity_tree_at(store, &plan.tree, plan.position, None)
}

/// Loads and adds a spawn tree in one step, answering the root's id.
pub fn spawn_entity_tree(
    store: &mut EntityStore,
    plan: &crate::entity::AtomicSpawnPlan,
) -> Option<i32> {
    let root = load_fresh_entity_tree(store, plan)?;
    Some(store.add_fresh(root))
}

pub(crate) fn spawn_summoned_entity_with_context(
    store: &mut EntityStore,
    resource_id: &str,
    nbt: &[NbtSummaryEntry],
    position: Vec3d,
    context: EntityNbtReadContext<'_>,
) -> Option<i32> {
    // SummonCommand.java:83-95: the command owns the root id and position;
    // explicit NBT skips finalizeSpawn, including an empty compound.
    let mut entries = nbt.to_vec();
    set_field(
        &mut entries,
        "id",
        NbtSummaryValue::String(resource_id.into()),
    );
    let tree = crate::entity::SpawnEntityTree::decode_root_with_registry(
        &entries,
        store.plugins().content_registry(),
    )?;
    let mut uuids = rustc_hash::FxHashSet::default();
    for node in tree.self_and_passengers() {
        if let Some(uuid) = optional_uuid(&node.entity_nbt, "UUID") {
            if !uuids.insert(uuid)
                || store.contains_uuid(uuid)
                || store.player_by_uuid(uuid).is_some()
            {
                return None;
            }
        }
    }
    let root = load_fresh_entity_tree_at(store, &tree, position, Some(context))?;
    Some(store.add_fresh(root))
}

fn load_fresh_entity_tree_at(
    store: &mut EntityStore,
    tree: &crate::entity::SpawnEntityTree,
    position: Vec3d,
    context: Option<EntityNbtReadContext<'_>>,
) -> Option<Box<Entity>> {
    let root = load_entity_creation(store, &tree.entity_nbt, position, context).ok()?;
    load_passengers(store, root, &tree.passengers, position, context);
    store.take_fresh(root)
}

fn load_passengers(
    store: &mut EntityStore,
    vehicle: i32,
    nodes: &[crate::entity::SpawnEntityTree],
    position: Vec3d,
    context: Option<EntityNbtReadContext<'_>>,
) {
    for node in nodes {
        let Ok(id) = load_entity_creation(store, &node.entity_nbt, position, context) else {
            continue;
        };
        load_passengers(store, id, &node.passengers, position, context);
        if !store.link_existing_passenger_forced(vehicle, id) {
            if let Some(passenger) = store.take_fresh(id) {
                store.discard_fresh(passenger);
            }
        } else if let (Some(context), Some(entity)) = (context, store.get(id)) {
            context.apply_armor_stand_passenger_team(entity, &node.entity_nbt);
        }
    }
}

pub fn create_structure_template_entity_with_context(
    store: &mut EntityStore,
    nbt: &[NbtSummaryEntry],
    position: Vec3d,
    rotation: [f32; 2],
    transform: (u8, bbb_protocol::block_state_types::VanillaBlockStateMirror),
    context: EntityNbtReadContext<'_>,
) -> Option<Box<Entity>> {
    let entries = nbt
        .iter()
        .filter(|entry| entry.name != "UUID" && entry.name != "Passengers")
        .cloned()
        .collect::<Vec<_>>();
    let id = load_entity_creation(store, &entries, position, Some(context)).ok()?;
    let mut entity = store.take_fresh(id)?;
    let rotation =
        crate::entity::hanging::transform_template_pose(&mut entity, transform.0, transform.1)
            .unwrap_or(rotation);
    entity.set_pos(position);
    entity.set_rot(rotation[0], rotation[1]);
    entity.set_old_pos_and_rot();
    crate::entity::hanging::set_attached_pos(&mut entity, position);
    entity.set_y_head_rot(rotation[0]);
    Some(entity)
}

fn set_field(
    entries: &mut Vec<NbtSummaryEntry>,
    name: impl Into<JavaString>,
    value: NbtSummaryValue,
) {
    let name = name.into();
    entries.retain(|entry| entry.name != name);
    entries.push(NbtSummaryEntry { name, value });
}

///
/// `store` is the level's entity lookup, which only a record built outside a
/// live level can do without: a `lastHurtByMob` reference to a stored entity
/// writes that entity's UUID.
fn entity_record(
    entity: &Entity,
    command_root: bool,
    store: Option<&EntityStore>,
) -> Result<Vec<NbtSummaryEntry>, EntitySaveError> {
    let items: crate::entity::item::ItemResolver<'_> = match store {
        Some(store) => store.item_resolver(),
        None => &bbb_registry_codec::UnavailableHolderResolver,
    };
    let content_registry: &bbb_protocol::content_registry::ContentRegistry = store
        .map(|store| store.plugins().content_registry().as_ref())
        .filter(|registry| !registry.is_vanilla_only())
        .unwrap_or_else(|| items.content_registry());
    let resource_id = vanilla_entity_type(entity.entity_type_id())
        .map(|entry| entry.resource_id)
        .or_else(|| entity.plugin_mob().map(|plugin| plugin.resource_id))
        .ok_or_else(|| field_error("id", "entity type is outside the built-in 26.1 registry"))?;
    // The stored type id and the subtype data are set together at spawn time,
    // so a disagreement is a construction bug rather than a save problem; it is
    // still refused here, because writing it would produce a record whose `id`
    // and whose fields describe different entities.
    if resource_id != entity.data().entity_resource_id() {
        return Err(field_error(
            "id",
            format!(
                "entity type {resource_id} disagrees with its {} data",
                entity.data().entity_resource_id()
            ),
        ));
    }
    let mut entries = vec![
        NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::string(resource_id),
        },
        NbtSummaryEntry {
            name: "Pos".into(),
            value: double_list(entity.position()),
        },
        NbtSummaryEntry {
            name: "Motion".into(),
            value: double_list(entity.delta_movement()),
        },
        NbtSummaryEntry {
            name: "Rotation".into(),
            value: NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(entity.y_rot().to_bits()),
                NbtSummaryValue::Float(entity.x_rot().to_bits()),
            ]),
        },
        NbtSummaryEntry {
            name: "fall_distance".into(),
            value: NbtSummaryValue::Double(entity.fall_distance().to_bits()),
        },
        NbtSummaryEntry {
            name: "Fire".into(),
            value: NbtSummaryValue::Short(entity.remaining_fire_ticks() as i16),
        },
        NbtSummaryEntry {
            name: "Air".into(),
            value: NbtSummaryValue::Short(entity.air_supply() as i16),
        },
        NbtSummaryEntry {
            name: "OnGround".into(),
            value: byte_bool(entity.on_ground()),
        },
        NbtSummaryEntry {
            name: "Invulnerable".into(),
            value: byte_bool(entity.invulnerable()),
        },
        NbtSummaryEntry {
            name: "PortalCooldown".into(),
            value: NbtSummaryValue::Int(entity.portal_cooldown()),
        },
        NbtSummaryEntry {
            name: "UUID".into(),
            value: uuid_int_array(entity.uuid()),
        },
    ];
    if !entity.custom_data().is_empty() {
        entries.push(NbtSummaryEntry {
            name: "data".into(),
            value: NbtSummaryValue::Compound(entity.custom_data().to_vec()),
        });
    }
    if let Some(custom_name) = entity.custom_name() {
        entries.push(NbtSummaryEntry {
            name: "CustomName".into(),
            value: custom_name.component().clone(),
        });
    }
    if entity.custom_name_visible() {
        entries.push(NbtSummaryEntry {
            name: "CustomNameVisible".into(),
            value: byte_bool(true),
        });
    }
    if entity.is_silent() {
        entries.push(NbtSummaryEntry {
            name: "Silent".into(),
            value: byte_bool(true),
        });
    }
    // `:2000-2002`: written only when set, which is what the reader's
    // `getBooleanOr("NoGravity", false)` (`:2083`) expects.
    if entity.no_gravity() {
        entries.push(NbtSummaryEntry {
            name: "NoGravity".into(),
            value: byte_bool(true),
        });
    }
    if entity.has_glowing_tag() {
        entries.push(NbtSummaryEntry {
            name: "Glowing".into(),
            value: byte_bool(true),
        });
    }
    if entity.ticks_frozen() > 0 {
        entries.push(NbtSummaryEntry {
            name: "TicksFrozen".into(),
            value: NbtSummaryValue::Int(entity.ticks_frozen()),
        });
    }
    if entity.has_visual_fire() {
        entries.push(NbtSummaryEntry {
            name: "HasVisualFire".into(),
            value: byte_bool(true),
        });
    }
    // `:2017-2019`, written only when the set has something in it.
    if !entity.entity_tags().is_empty() {
        entries.push(NbtSummaryEntry {
            name: "Tags".into(),
            value: NbtSummaryValue::List(
                entity
                    .entity_tags()
                    .iter()
                    .map(|tag| NbtSummaryValue::string(tag.clone()))
                    .collect(),
            ),
        });
    }
    match entity.data() {
        EntityData::Player | EntityData::EnderDragonPart => {}
        EntityData::FallingBlock(data) => {
            write_falling_block(&mut entries, data, content_registry)?
        }
        EntityData::Item(data) => write_item_entity(&mut entries, data, items),
        EntityData::ExperienceOrb(data) => write_experience_orb(&mut entries, data),
        EntityData::Tnt(data) => write_tnt(&mut entries, data, content_registry)?,
        EntityData::Painting(data) => write_painting(&mut entries, data),
        EntityData::ItemFrame(data) | EntityData::GlowItemFrame(data) => {
            write_item_frame(&mut entries, data, items)
        }
        EntityData::LeashKnot(_) => {}
        EntityData::ArmorStand(data) => write_armor_stand(&mut entries, entity, data, store, items),
        EntityData::GeneralLiving(data) => {
            write_living_entity(&mut entries, entity, data.living(), false, store, items);
            if let Some(mob) = data.mob_retained() {
                crate::entity::leash::write_leash_data(&mut entries, mob.leash.as_ref())?;
                if mob.home_radius != -1 {
                    entries.push(NbtSummaryEntry {
                        name: "home_radius".into(),
                        value: NbtSummaryValue::Int(mob.home_radius),
                    });
                    if data.turtle_retained().is_none() {
                        entries.push(crate::entity::wandering_trader::block_pos_entry(
                            "home_pos",
                            mob.home_position,
                        ));
                    }
                }
                entries.push(NbtSummaryEntry {
                    name: "CanPickUpLoot".into(),
                    value: byte_bool(mob.can_pick_up_loot()),
                });
                entries.push(NbtSummaryEntry {
                    name: "PersistenceRequired".into(),
                    value: byte_bool(mob.persistence_required()),
                });
                if !mob.has_default_drop_chances() {
                    entries.push(NbtSummaryEntry {
                        name: "drop_chances".into(),
                        value: NbtSummaryValue::Compound(
                            VANILLA_EQUIPMENT_SLOTS
                                .iter()
                                .filter_map(|slot| {
                                    let chance = mob.drop_chance(slot.slot);
                                    (chance != DEFAULT_MOB_EQUIPMENT_DROP_CHANCE).then(|| {
                                        NbtSummaryEntry {
                                            name: slot.serialized_name.into(),
                                            value: NbtSummaryValue::Float(chance.to_bits()),
                                        }
                                    })
                                })
                                .collect(),
                        ),
                    });
                }
                entries.push(NbtSummaryEntry {
                    name: "LeftHanded".into(),
                    value: byte_bool(data.is_left_handed()),
                });
                if let Some(table) = mob.death_loot_table() {
                    entries.push(NbtSummaryEntry {
                        name: "DeathLootTable".into(),
                        value: NbtSummaryValue::string(table),
                    });
                }
                if mob.death_loot_table_seed() != 0 {
                    entries.push(NbtSummaryEntry {
                        name: "DeathLootTableSeed".into(),
                        value: NbtSummaryValue::Long(mob.death_loot_table_seed()),
                    });
                }
                if data.is_no_ai() {
                    entries.push(NbtSummaryEntry {
                        name: "NoAI".into(),
                        value: byte_bool(true),
                    });
                }
            }
            if let Some(ageable) = data.ageable_retained() {
                entries.extend([
                    NbtSummaryEntry {
                        name: "Age".into(),
                        value: NbtSummaryValue::Int(ageable.age()),
                    },
                    NbtSummaryEntry {
                        name: "ForcedAge".into(),
                        value: NbtSummaryValue::Int(ageable.forced_age()),
                    },
                    NbtSummaryEntry {
                        name: "AgeLocked".into(),
                        value: byte_bool(ageable.age_locked()),
                    },
                ]);
            }
            if let Some(animal) = data.animal_retained() {
                entries.push(NbtSummaryEntry {
                    name: "InLove".into(),
                    value: NbtSummaryValue::Int(animal.in_love()),
                });
                if let Some(love_cause) = animal.love_cause() {
                    entries.push(NbtSummaryEntry {
                        name: "LoveCause".into(),
                        value: uuid_int_array(love_cause),
                    });
                }
            }
            internal_inventory::write_plain_inventory(
                &mut entries,
                entity.entity_type_id(),
                data,
                items,
            );
            data.write_variant_save(
                &mut entries,
                entity,
                store.and_then(|store| {
                    store.live_brain_pack(entity.id()).or_else(|| {
                        store.mob_ai(entity.id())?.nautilus_brain_pack(
                            crate::entity::brain_packed(entity)?,
                            entity.tick_count(),
                        )
                    })
                }),
                items,
            )?;
            entries.extend(data.opaque_fields().iter().cloned());
            crate::entity::mannequin::write_pose(entity, &mut entries);
        }
        EntityData::Marker(_) => {}
        EntityData::Interaction(data) => interaction::write_save_data(&mut entries, data),
        EntityData::BlockDisplay(data) => {
            display::write_block_display_with_registry(&mut entries, data, content_registry)?
        }
        EntityData::ItemDisplay(data) => display::write_item_display(&mut entries, data, items)?,
        EntityData::TextDisplay(data) => display::write_text_display(&mut entries, data)?,
        EntityData::Arrow(data) => write_abstract_arrow(&mut entries, data, items)?,
        EntityData::SpectralArrow(data) => write_spectral_arrow(&mut entries, data, items)?,
        EntityData::Trident(data) => write_trident(&mut entries, data, items)?,
        EntityData::FishingBobber(data) => {
            if !command_root {
                write_fishing_bobber(data)?;
            }
        }
        EntityData::Snowball(data)
        | EntityData::Egg(data)
        | EntityData::EnderPearl(data)
        | EntityData::ExperienceBottle(data)
        | EntityData::SplashPotion(data)
        | EntityData::LingeringPotion(data) => write_throwable_item(&mut entries, data, items),
        EntityData::EndCrystal(data) => end_crystal::write_save_data(&mut entries, data),
        EntityData::EyeOfEnder(data) => eye_of_ender::write_save_data(&mut entries, data, items),
        EntityData::FireworkRocket(data) => write_firework_rocket(&mut entries, data, items),
        EntityData::LlamaSpit(data) => write_llama_spit(&mut entries, data),
        EntityData::EvokerFangs(data) => write_evoker_fangs(&mut entries, data),
        EntityData::LightningBolt(_) if command_root => {}
        EntityData::LightningBolt(_) => {
            return Err(field_error(
                "id",
                "minecraft:lightning_bolt is not serializable (EntityType.noSave)",
            ));
        }
        EntityData::LargeFireball(data) => write_large_fireball(&mut entries, data, items),
        EntityData::SmallFireball(data) => write_small_fireball(&mut entries, data, items),
        EntityData::DragonFireball(data) => {
            write_hurting_projectile(&mut entries, data.projectile())
        }
        EntityData::WitherSkull(data) => write_wither_skull(&mut entries, data),
        EntityData::WindCharge(data) | EntityData::BreezeWindCharge(data) => {
            write_hurting_projectile(&mut entries, data.projectile())
        }
        EntityData::AreaEffectCloud(data) => {
            area_effect_cloud::write_save_data(&mut entries, entity, data)
        }
        EntityData::OminousItemSpawner(data) => {
            if !data.item().is_empty() {
                entries.push(NbtSummaryEntry {
                    name: "item".into(),
                    value: crate::entity::item::write_saved_stack(data.item(), items),
                });
            }
            entries.push(NbtSummaryEntry {
                name: "spawn_item_after_ticks".into(),
                value: NbtSummaryValue::Long(data.spawn_item_after_ticks()),
            });
        }
        EntityData::ShulkerBullet(data) => write_shulker_bullet(&mut entries, data),
        EntityData::Boat(data) => boat::write_save_data(&mut entries, data)?,
        EntityData::ChestBoat(data) => {
            boat::write_save_data(&mut entries, data.boat())?;
            let cargo = data.cargo.borrow();
            nonliving::write_chest_vehicle_save_data(
                &mut entries,
                &cargo.items,
                cargo.loot.table_id(),
                cargo.loot.seed(),
                items,
            );
        }
        EntityData::Minecart(data) => {
            minecart::write_save_data_with_registry(&mut entries, data, content_registry)?
        }
        EntityData::FurnaceMinecart(data) => {
            minecart::write_furnace_save_data(&mut entries, data, content_registry)?
        }
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => {
            minecart::write_save_data_with_registry(
                &mut entries,
                data.minecart(),
                content_registry,
            )?;
            let cargo = data.cargo.borrow();
            nonliving::write_chest_vehicle_save_data(
                &mut entries,
                &cargo.items,
                cargo.loot.table_id(),
                cargo.loot.seed(),
                items,
            );
            if data.kind() == MinecartContainerKind::Hopper {
                entries.push(NbtSummaryEntry {
                    name: "Enabled".into(),
                    value: NbtSummaryValue::Byte(i8::from(data.is_enabled())),
                });
            }
        }
        EntityData::CommandBlockMinecart(data) => {
            minecart::write_save_data_with_registry(
                &mut entries,
                data.minecart(),
                content_registry,
            )?;
            data.write_save_data(&mut entries);
        }
        EntityData::SpawnerMinecart(data) => {
            data.write_save_data_with_registry(&mut entries, content_registry)?
        }
        EntityData::TntMinecart(data) => {
            minecart::write_save_data_with_registry(
                &mut entries,
                data.minecart(),
                content_registry,
            )?;
            data.write_save_data(&mut entries);
        }
    }
    Ok(entries)
}

pub(crate) fn entity_record_tree_without_id(
    store: &EntityStore,
    entity: &Entity,
) -> Result<NbtSummaryValue, EntitySaveError> {
    let view = crate::entity::store::EntityBodyReadView::new(store, Some(entity), None);
    let mut record = entity_record(entity, true, Some(store))?;
    apply_vehicle_save_position(view, entity, &mut record);
    append_passenger_records(view, entity, &mut record)?;
    record.retain(|entry| entry.name != "id");
    Ok(NbtSummaryValue::Compound(record))
}

/// Reads one command-data compound into a fresh entity and overlays it on the
/// stored one.
pub(crate) fn load_entity_command_record(
    source_store: &EntityStore,
    entity: &Entity,
    entries: &[NbtSummaryEntry],
    game_time: i64,
    level: Option<&LevelCtx<'_, '_>>,
    context: Option<EntityNbtReadContext<'_>>,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<Entity, EntitySaveError> {
    let resource_id = vanilla_entity_type(entity.entity_type_id())
        .map(|entry| entry.resource_id)
        .or_else(|| entity.plugin_mob().map(|plugin| plugin.resource_id))
        .ok_or_else(|| field_error("id", "entity type is outside the built-in 26.1 registry"))?;
    let mut normalized = entries
        .iter()
        .filter(|entry| {
            !entry
                .name
                .with_utf8(|name| matches!(name, "id" | "UUID" | "Passengers"))
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let position = optional_vec3(&normalized, "Pos").unwrap_or_default();
    let rotation = optional_float_pair(&normalized, "Rotation").unwrap_or_default();
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !position.z.is_finite()
        || !rotation[0].is_finite()
        || !rotation[1].is_finite()
    {
        return Err(field_error("Pos", "position and rotation must be finite"));
    }
    let position = Vec3d {
        x: position.x.clamp(-30_000_512.0, 30_000_512.0),
        y: position.y.clamp(-20_000_000.0, 20_000_000.0),
        z: position.z.clamp(-30_000_512.0, 30_000_512.0),
    };
    loop {
        let mut temporary = source_store.for_command_decode(game_time);
        let mut tags = |registry: &str, tag: &str, element: &str| match context {
            Some(context) => context.registry_tags.contains(registry, tag, element),
            None => builtin_registry_tag_membership(registry, tag, element),
        };
        match read_entity_record_fields(
            &mut temporary,
            record::EntityRecordFields {
                resource_id,
                uuid: entity.uuid(),
                position: [position.x, position.y, position.z],
                entries: &normalized,
                command_existing: Some(entity),
            },
            level,
            context,
            &mut tags,
            items,
        ) {
            Ok(loaded_id) => {
                let mut loaded = *temporary
                    .detach(loaded_id)
                    .expect("command entity was restored");
                let _ = crate::entity::resolve_spawner_registry_holders(
                    &mut loaded,
                    |registry, resource| items.holder_id(registry, resource),
                );
                return Ok(overlay_command_loaded_entity(entity, loaded));
            }
            Err(EntitySaveError::Field { field, .. }) => {
                let top_level = field.split('.').next().unwrap_or(field);
                if matches!(top_level, "id" | "UUID")
                    || normalized.iter().all(|entry| entry.name != top_level)
                {
                    return Err(field_error(field, "command data could not be loaded"));
                }
                normalized.retain(|entry| entry.name != top_level);
            }
            Err(error) => return Err(error),
        }
    }
}

fn command_number_i32(value: &NbtSummaryValue) -> i32 {
    match value {
        NbtSummaryValue::Byte(value) => i32::from(*value),
        NbtSummaryValue::Short(value) => i32::from(*value),
        NbtSummaryValue::Int(value) => *value,
        NbtSummaryValue::Long(value) => *value as i32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits).floor() as i32,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits).floor() as i32,
        _ => unreachable!("command numeric value was checked"),
    }
}

fn command_number_i64(value: &NbtSummaryValue) -> i64 {
    match value {
        NbtSummaryValue::Byte(value) => i64::from(*value),
        NbtSummaryValue::Short(value) => i64::from(*value),
        NbtSummaryValue::Int(value) => i64::from(*value),
        NbtSummaryValue::Long(value) => *value,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i64,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits).floor() as i64,
        _ => unreachable!("command numeric value was checked"),
    }
}

fn command_number_i8(value: &NbtSummaryValue) -> i8 {
    command_number_i32(value) as i8
}

pub(crate) fn numeric_tag_byte_value(value: &NbtSummaryValue) -> Option<i8> {
    command_number_f64(value).map(|_| command_number_i8(value))
}

pub(crate) fn numeric_tag_int_value(value: &NbtSummaryValue) -> Option<i32> {
    command_number_f64(value).map(|_| command_number_i32(value))
}

pub(crate) fn numeric_tag_long_value(value: &NbtSummaryValue) -> Option<i64> {
    command_number_f64(value).map(|_| command_number_i64(value))
}

/// The command-data overlay: the row comparison that decides which synched
/// values the load changed.
fn overlay_command_loaded_entity(entity: &Entity, mut loaded: Entity) -> Entity {
    let before_synched = crate::entity::tracking::synched_values(entity);
    loaded.data.restore_command_transients(&entity.data);
    let mut result = entity.clone();
    if let Some(reason) = loaded.removal_reason() {
        result.set_removed(reason);
    }
    result.width = loaded.width;
    result.height = loaded.height;
    result.eye_height = loaded.eye_height;
    result.position = loaded.position;
    result.old_position = loaded.old_position;
    result.block_position = loaded.block_position;
    result.bounding_box = loaded.bounding_box;
    result.delta_movement = loaded.delta_movement;
    result.y_rot = loaded.y_rot;
    result.x_rot = loaded.x_rot;
    result.fall_distance = loaded.fall_distance;
    result.remaining_fire_ticks = loaded.remaining_fire_ticks;
    result.air_supply = loaded.air_supply;
    result.on_ground = loaded.on_ground;
    result.invulnerable = loaded.invulnerable;
    result.portal_cooldown = loaded.portal_cooldown;
    result.no_gravity = loaded.no_gravity;
    result.no_physics = loaded.no_physics;
    result.needs_sync = true;
    result.custom_name = loaded.custom_name;
    result.custom_data = loaded.custom_data;
    result.tags = loaded.tags;
    result.custom_name_visible = loaded.custom_name_visible;
    result.silent = loaded.silent;
    result.has_glowing_tag = loaded.has_glowing_tag;
    result.ticks_frozen = loaded.ticks_frozen;
    result.has_visual_fire = loaded.has_visual_fire;
    let mut persisted_shared_flags =
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_GLOWING;
    if matches!(
        loaded.data,
        EntityData::GeneralLiving(_) | EntityData::ArmorStand(_)
    ) {
        persisted_shared_flags |=
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
    }
    if matches!(
        loaded.data,
        EntityData::ArmorStand(_) | EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_)
    ) {
        persisted_shared_flags |=
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_INVISIBLE;
    }
    result.shared_flags = (entity.shared_flags & !persisted_shared_flags)
        | (loaded.shared_flags & persisted_shared_flags);
    if matches!(loaded.data, EntityData::AreaEffectCloud(_)) {
        result.tick_count = loaded.tick_count;
    }
    if matches!(
        loaded.data,
        EntityData::GeneralLiving(_) | EntityData::ArmorStand(_)
    ) {
        result.pose_id = loaded.pose_id;
    }
    result.data = loaded.data;
    if let Some(shulker_box) =
        crate::entity::shulker::bounding_box(&result.data, result.position, result.width)
    {
        result.bounding_box = shulker_box;
    }
    let after_synched = crate::entity::tracking::synched_values(&result);
    for after in after_synched {
        if before_synched
            .iter()
            .find(|before| before.value.data_id == after.value.data_id)
            .is_none_or(|before| before.value != after.value)
        {
            result.mark_synched_dirty(after.value.data_id);
        }
    }
    crate::entity::display::mark_command_load_dirty(&mut result);
    result
}

fn finish_item_frame_load(entity: &mut Entity) {
    let (direction, has_item, rotation, invisible) = match entity.data() {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => (
            frame.pose().direction(),
            frame.item().is_some(),
            frame.rotation(),
            frame.invisible(),
        ),
        _ => return,
    };
    if direction != Direction::South {
        entity.mark_synched_dirty(VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID);
    }
    if has_item {
        entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ITEM_DATA_ID);
    }
    if rotation != 0 {
        entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ROTATION_DATA_ID);
    }
    entity.set_invisible(invisible);
}
