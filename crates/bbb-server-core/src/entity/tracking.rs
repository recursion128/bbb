//! Entity tracking: who is told about an entity, and how often.
//!
//! [`crate`] makes the level's entities exist and move. This module is
//! the other half: the port of `ChunkMap.TrackedEntity`
//! (`net/minecraft/server/level/ChunkMap.java:1322-1432`) over `ServerEntity`
//! (`net/minecraft/server/level/ServerEntity.java:50-360`), which decides which
//! sessions may see an entity at all and turns the difference between two ticks
//! of its state into packets.
//!
//! # Shape
//!
//! * [`ServerEntity`] is one entity's outgoing state: the `VecDeltaCodec` base
//!   the relative move packets are measured from, the last rotation and
//!   movement it published, and the two counters that force a full resync.
//! * `TrackedEntity` adds the tracking radius and the set of sessions
//!   currently paired with the entity.
//! * [`EntityTracker`] is `ChunkMap.entityMap` (`ChunkMap.java:146`): one per
//!   `ServerLevel`, because a `ChunkMap` belongs to one level and therefore
//!   only ever pairs the sessions watching it. Both live in `chunk_map`.
//!
//! Nothing here reaches `ServerWorld`. Sessions arrive as [`TrackingViewer`],
//! a player avatar's own rows are read from the player the store holds, and
//! the two level facts the loop needs -- whether a session is being sent a
//! column, and whether a column is in entity-ticking range -- arrive as
//! callbacks, so the caller keeps ownership of both.
//!
//! # Scope notes
//!
//! Passenger trees publish `ClientboundSetPassengersPacket` and use the mounted
//! rotation/data path (`ServerEntity.java:92-99`, `:120-133`, `:303-309`).
//! Leash links publish `ClientboundSetEntityLinkPacket` (`:311-313`), and
//! minecarts with track interpolation enabled publish
//! `ClientboundMoveMinecartPacket` (`:134-135`, `:226-255`), falling back to
//! standard relative/absolute move packets otherwise.
//!
//! Every item-stack synched-data row projects through the level's registry
//! publication because stored stacks name registry holders by identifier; a
//! row that publication cannot resolve is omitted.

use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID, VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID, VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
    VANILLA_ITEM_ENTITY_ITEM_DATA_ID, VANILLA_ITEM_FRAME_ITEM_DATA_ID,
    VANILLA_ITEM_FRAME_ROTATION_DATA_ID, VANILLA_MOB_FLAGS_DATA_ID,
    VANILLA_PAINTING_VARIANT_DATA_ID, VANILLA_SPIDER_FLAGS_DATA_ID, VANILLA_SPIDER_FLAG_CLIMBING,
    VANILLA_WOLF_ANGER_END_TIME_DATA_ID,
};
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_PLAYER_ID};
use bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS;
use bbb_protocol::packets::{
    pack_degrees, unpack_degrees, AddEntity, AttributeSnapshot, BlockPos,
    EntityDataRegistryReference, EntityDataValue, EntityDataValueKind, EntityMove,
    EntityPositionSync, EquipmentSlotUpdate, ItemStackSummary, MinecartStep,
    MoveMinecartAlongTrack, PaintingVariantData, ProjectilePower, RemoveEntities, RotateHead,
    SetEntityData, SetEntityLink, SetEntityMotion, SetEquipment, SetPassengers, UpdateAttributes,
    Vec3d,
};
use bbb_registry_codec::item_stack_summary_is_empty;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::hanging::{framed_map_frame, FramedMapFrame, ItemFrameData, PaintingData};
use crate::entity::living::{
    push_dirty_entity_synched_values, push_dirty_living_synched_values, SynchedValue,
    ENTITY_SYNCHED_DATA_MASK, LIVING_SYNCHED_DATA_MASK,
};
use crate::entity::store::{section_of_block, EntityLifecycleEvent, EntityStore, SectionKey};
use crate::entity::{Entity, EntityData, GeneralLivingData};

mod item_stack_resolver;
use crate::registry_tags::RegistryTagSnapshot;
use item_stack_resolver::ItemStackRegistryPublication;
pub use item_stack_resolver::TrackingItemStackResolver;

pub(crate) mod player_metadata;
use player_metadata::{player_metadata, PlayerRows};

mod chunk_map;
#[cfg(test)]
pub(crate) use chunk_map::recipient_packets;
pub use chunk_map::{EntityTracker, TrackingPacket};

/// `ServerEntity.TOLERANCE_LEVEL_ROTATION` (`ServerEntity.java:52`), compared
/// against the difference of two packed degree bytes.
const TOLERANCE_LEVEL_ROTATION: i32 = 1;

/// `ServerEntity.TOLERANCE_LEVEL_POSITION` (`ServerEntity.java:53`): a `double`
/// field initialised from a `float` literal, so the widening is part of the
/// constant.
const TOLERANCE_LEVEL_POSITION: f64 = 7.6293945E-6_f32 as f64;

/// `ServerEntity.FORCED_POS_UPDATE_PERIOD` (`ServerEntity.java:54`): every
/// sixtieth pass publishes the position even when it did not move.
const FORCED_POS_UPDATE_PERIOD: i32 = 60;

/// `ServerEntity.FORCED_TELEPORT_PERIOD` (`ServerEntity.java:55`): after this
/// many passes without one, the next pass is an absolute resync.
const FORCED_TELEPORT_PERIOD: i32 = 400;

/// The squared movement difference that makes `sendChanges` publish a new
/// velocity (`ServerEntity.java:175`).
const MOVEMENT_TOLERANCE: f64 = 1.0E-7;

/// The inclusive bounds of the `short` the relative move packets carry
/// (`ServerEntity.java:147`).
const RELATIVE_MOVE_MIN: i64 = -32768;
const RELATIVE_MOVE_MAX: i64 = 32767;

/// `VecDeltaCodec.TRUNCATION_STEPS` (`VecDeltaCodec.java:7`).
const TRUNCATION_STEPS: f64 = 4096.0;

/// `DATA_HEALTH_ID` and `DATA_PLAYER_ABSORPTION_ID` as defined
/// (`LivingEntity.java:314`, `Player.java:224`).
const PLAYER_DEFAULT_VITALS: (f32, f32) = (1.0, 0.0);

/// One chunk of tracking radius, the unit `EntityType.clientTrackingRange` is
/// counted in (`ChunkMap.java:1142`).
const CHUNK_BLOCKS: i32 = 16;

/// `VecDeltaCodec.encode(double)` (`VecDeltaCodec.java:11-13`).
///
/// `Math.round(double)` breaks ties toward positive infinity, where Rust's
/// `f64::round` breaks them away from zero; the two disagree on every negative
/// half, which is a coordinate a falling entity reaches routinely. The
/// comparison against the floor rather than `floor(value + 0.5)` keeps the tie
/// exact for arguments where adding a half would itself round.
fn encode_position_component(input: f64) -> i64 {
    let scaled = input * TRUNCATION_STEPS;
    let floor = scaled.floor();
    if scaled - floor >= 0.5 {
        floor as i64 + 1
    } else {
        floor as i64
    }
}

/// The `data` int `ClientboundAddEntityPacket` carries.
///
/// `Entity.getAddEntityPacket` (`Entity.java:3612-3614`) uses the two-argument
/// constructor, whose `data` is zero (`ClientboundAddEntityPacket.java:33-35`).
/// `FallingBlockEntity` is the one override among the kinds this crate holds
/// and passes `Block.getId(getBlockState())`
/// (`FallingBlockEntity.java:339-341`), which is the global block state id;
/// `Projectile` is the other and passes its owner's entity id
/// (`Projectile.java:346-349`), resolved by [`projectile_owner_entity_id`].
/// `FishingHook` narrows that to its own id when the owner is null
/// (`FishingHook.java:544-547`), a branch `spawn_fishing_bobber` cannot reach
/// because a hook is always cast by somebody.
fn add_entity_data(
    data: &EntityData,
    viewers: &[TrackingViewer],
    entity_ids: &FxHashMap<uuid::Uuid, i32>,
) -> i32 {
    if data.projectile_owner().is_some() {
        return projectile_owner_entity_id(data, viewers, entity_ids);
    }
    match data {
        EntityData::Player | EntityData::EnderDragonPart => 0,
        EntityData::GeneralLiving(_) => 0,
        EntityData::FallingBlock(falling) => falling.block_state_id(),
        EntityData::Item(_)
        | EntityData::ExperienceOrb(_)
        | EntityData::Tnt(_)
        | EntityData::ArmorStand(_)
        | EntityData::Arrow(_)
        | EntityData::SpectralArrow(_)
        | EntityData::Trident(_)
        | EntityData::FishingBobber(_)
        | EntityData::Snowball(_)
        | EntityData::Egg(_)
        | EntityData::EnderPearl(_)
        | EntityData::EndCrystal(_)
        | EntityData::EyeOfEnder(_)
        | EntityData::ExperienceBottle(_) => 0,
        EntityData::Marker(_)
        | EntityData::Interaction(_)
        | EntityData::BlockDisplay(_)
        | EntityData::ItemDisplay(_)
        | EntityData::TextDisplay(_) => 0,
        EntityData::FireworkRocket(_) => 0,
        EntityData::LlamaSpit(_)
        | EntityData::EvokerFangs(_)
        | EntityData::LightningBolt(_)
        | EntityData::LargeFireball(_)
        | EntityData::SmallFireball(_)
        | EntityData::DragonFireball(_)
        | EntityData::WitherSkull(_)
        | EntityData::WindCharge(_)
        | EntityData::BreezeWindCharge(_) => 0,
        EntityData::Painting(_) | EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_) => {
            data.hanging_add_packet_data().unwrap_or(0)
        }
        EntityData::LeashKnot(_) => 0,
        EntityData::AreaEffectCloud(_) => 0,
        EntityData::OminousItemSpawner(_) => 0,
        EntityData::SplashPotion(_) | EntityData::LingeringPotion(_) => 0,
        EntityData::ShulkerBullet(_) => 0,
        // `ServerEntity.sendPairingData` (`ServerEntity.java:161-170`).
        EntityData::Boat(_)
        | EntityData::ChestBoat(_)
        | EntityData::Minecart(_)
        | EntityData::FurnaceMinecart(_)
        | EntityData::ChestMinecart(_)
        | EntityData::HopperMinecart(_)
        | EntityData::CommandBlockMinecart(_)
        | EntityData::SpawnerMinecart(_)
        | EntityData::TntMinecart(_) => 0,
    }
}

fn hanging_synched_painting(painting: &PaintingData) -> Vec<EntityDataValue> {
    let mut values = Vec::new();
    let direction = painting.pose().direction();
    if direction != bbb_protocol::packets::Direction::South {
        values.push(EntityDataValue {
            data_id: VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
            value: EntityDataValueKind::Direction(i32::from(direction.id())),
        });
    }
    values.push(EntityDataValue {
        data_id: VANILLA_PAINTING_VARIANT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
        value: EntityDataValueKind::PaintingVariant(PaintingVariantData {
            registry: Some(EntityDataRegistryReference::unresolved(
                painting.variant().holder_id(),
            )),
            direct: None,
        }),
    });
    values
}

/// `ItemFrame.defineSynchedData` (`ItemFrame.java:74-78`).
fn hanging_synched_frame(frame: &ItemFrameData) -> Vec<SynchedValue> {
    let row = |data_id, serializer_id, value, is_default| SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id,
            value,
        },
        is_default,
    };
    let direction = frame.pose().direction();
    let mut rows = vec![row(
        VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
        EntityDataValueKind::Direction(i32::from(direction.id())),
        direction == bbb_protocol::packets::Direction::South,
    )];
    let item = frame
        .item()
        .map_or_else(ItemStackSummary::empty, |item| item.summary().clone());
    let is_default = item_stack_summary_is_empty(&item);
    rows.push(row(
        VANILLA_ITEM_FRAME_ITEM_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        EntityDataValueKind::ItemStack(item),
        is_default,
    ));
    rows.push(row(
        VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        EntityDataValueKind::Int(frame.rotation()),
        frame.rotation() == 0,
    ));
    rows
}

/// `ServerEntity.trackedDataValues` (`:87`), which is
/// `entity.getEntityData().getNonDefaultValues()`
/// (`SynchedEntityData.java:92-106`) as the snapshot `sendPairingData`
/// publishes (`ServerEntity.java:277-279`).
pub(crate) fn tracked_data_values(entity: &Entity) -> Vec<EntityDataValue> {
    non_default_values(synched_values(entity))
}

/// `getNonDefaultValues`' filter, over rows already projected.
fn non_default_values(rows: Vec<SynchedValue>) -> Vec<EntityDataValue> {
    let mut values: Vec<EntityDataValue> = rows
        .into_iter()
        .filter(|row| !row.is_default)
        .map(|row| row.value)
        .collect();
    if !values
        .windows(2)
        .all(|pair| pair[0].data_id <= pair[1].data_id)
    {
        values.sort_by_key(|value| value.data_id);
    }
    values
}

#[cfg(test)]
pub(crate) fn non_default_synched_values(entity: &Entity) -> Vec<EntityDataValue> {
    non_default_values(synched_values(entity))
}

fn push_general_living_subtype_values(
    entity: &Entity,
    data: &GeneralLivingData,
    rows: &mut Vec<SynchedValue>,
) {
    if let Some(flags) = data.mob_flags() {
        // `Mob.defineSynchedData` (`Mob.java:264-267`).
        rows.push(SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_MOB_FLAGS_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(flags),
            },
            is_default: flags == 0,
        });
    }
    if entity.is_spider() {
        let flags = if entity.horizontal_collision() {
            VANILLA_SPIDER_FLAG_CLIMBING
        } else {
            0
        };
        rows.push(SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_SPIDER_FLAGS_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(flags),
            },
            is_default: flags == 0,
        });
    }
    data.variant_synched_values(entity.entity_type_id(), rows);
    rows.extend(crate::entity::mooshroom::synched_values(entity));
    rows.extend(crate::entity::snow_golem::synched_values(entity));
    rows.extend(crate::entity::bogged::synched_values(entity));
    rows.extend(crate::entity::copper_golem::synched_values(entity));
    rows.extend(crate::entity::mannequin::synched_values(entity));
    if let Some(end_time) = crate::entity::ai::mob::target::anger::wolf_anger_end_time(entity) {
        // `Wolf.defineSynchedData` (`Wolf.java:220`).
        rows.push(SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_WOLF_ANGER_END_TIME_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(end_time),
            },
            is_default: end_time == -1,
        });
    }
}

#[inline]
fn dirty_mask_contains(mask: [u64; 4], data_id: u8) -> bool {
    (mask[(data_id >> 6) as usize] & (1u64 << (data_id & 63))) != 0
}

#[inline]
fn dirty_mask_is_non_empty(mask: [u64; 4]) -> bool {
    (mask[0] | mask[1] | mask[2] | mask[3]) != 0
}

/// Emit only the synched rows marked dirty by `dirty_mask` in ascending id order.
fn dirty_synched_values(entity: &Entity, dirty_mask: [u64; 4]) -> Vec<SynchedValue> {
    if matches!(entity.data(), EntityData::Player) {
        return Vec::new();
    }
    let total_dirty: u32 = dirty_mask.iter().map(|word| word.count_ones()).sum();
    let mut rows = Vec::with_capacity(total_dirty as usize);
    let high_bits = dirty_mask[1] | dirty_mask[2] | dirty_mask[3];
    if high_bits == 0 && dirty_mask[0] & !ENTITY_SYNCHED_DATA_MASK == 0 {
        push_dirty_entity_synched_values(entity, dirty_mask[0], &mut rows);
        return rows;
    }
    if let EntityData::GeneralLiving(data) = entity.data() {
        if dirty_mask[0] & ENTITY_SYNCHED_DATA_MASK != 0 {
            push_dirty_entity_synched_values(entity, dirty_mask[0], &mut rows);
        }
        if dirty_mask[0] & LIVING_SYNCHED_DATA_MASK != 0 {
            push_dirty_living_synched_values(entity, data.living(), dirty_mask[0], &mut rows);
        }
        let subtype_mask = [
            dirty_mask[0] & !(ENTITY_SYNCHED_DATA_MASK | LIVING_SYNCHED_DATA_MASK),
            dirty_mask[1],
            dirty_mask[2],
            dirty_mask[3],
        ];
        if dirty_mask_is_non_empty(subtype_mask) {
            let mut subtype_rows = Vec::new();
            push_general_living_subtype_values(entity, data, &mut subtype_rows);
            rows.extend(
                subtype_rows
                    .into_iter()
                    .filter(|row| dirty_mask_contains(subtype_mask, row.value.data_id)),
            );
            rows.sort_by_key(|row| row.value.data_id);
        }
        return rows;
    }
    rows = synched_values(entity);
    rows.retain(|row| dirty_mask_contains(dirty_mask, row.value.data_id));
    rows.sort_by_key(|row| row.value.data_id);
    rows
}

fn update_tracked_data_value(tracked: &mut Vec<EntityDataValue>, row: &SynchedValue) {
    match tracked.binary_search_by_key(&row.value.data_id, |entry| entry.data_id) {
        Ok(index) => {
            if row.is_default {
                tracked.remove(index);
            } else {
                tracked[index] = row.value.clone();
            }
        }
        Err(index) => {
            if !row.is_default {
                tracked.insert(index, row.value.clone());
            }
        }
    }
}

/// Every synched row of the entity in id order: the `Entity` rows, then the
/// subtype's.
pub(crate) fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    match entity.data() {
        EntityData::Player => Vec::new(),
        EntityData::EnderDragonPart => crate::entity::living::entity_synched_values(entity),
        EntityData::GeneralLiving(data) => {
            let mut rows = crate::entity::living::entity_synched_values(entity);
            rows.extend(crate::entity::living::living_synched_values(
                entity,
                data.living(),
            ));
            push_general_living_subtype_values(entity, data, &mut rows);
            rows.sort_by_key(|row| row.value.data_id);
            rows
        }
        EntityData::ArmorStand(_) => crate::entity::armor_stand::synched_values(entity),
        EntityData::Marker(_) => crate::entity::marker::synched_values(entity),
        EntityData::Interaction(_) => crate::entity::interaction::synched_values(entity),
        EntityData::BlockDisplay(_) | EntityData::ItemDisplay(_) | EntityData::TextDisplay(_) => {
            crate::entity::display::synched_values(entity)
        }
        EntityData::FallingBlock(falling) => {
            let mut rows = crate::entity::living::entity_synched_values(entity);
            // `FallingBlockEntity.DATA_START_POS` (`FallingBlockEntity.java:73`),
            // id 8 (first field after `Entity` 0..=7), default `BlockPos.ZERO` (`:133`).
            let start_pos = falling.start_pos();
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: 8,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID,
                    value: EntityDataValueKind::BlockPos(start_pos),
                },
                is_default: start_pos == BlockPos { x: 0, y: 0, z: 0 },
            });
            rows
        }
        EntityData::ExperienceOrb(orb) => {
            let mut rows = crate::entity::living::entity_synched_values(entity);
            // `ExperienceOrb.DATA_VALUE` (`ExperienceOrb.java:31`), whose
            // default is zero (`:93`).
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(orb.value()),
                },
                is_default: orb.value() == 0,
            });
            rows
        }
        EntityData::Tnt(tnt) => crate::entity::tnt::synched_values(entity, tnt),
        EntityData::Item(item) => {
            let mut rows = crate::entity::living::entity_synched_values(entity);
            // `isSetToDefault` (`SynchedEntityData.java:96`) drops the value of
            // an entity whose stack is still `ItemStack.EMPTY`.
            let stack = item.item().summary().clone();
            let is_default = stack.item_id.is_none();
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_ITEM_ENTITY_ITEM_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    value: EntityDataValueKind::ItemStack(stack),
                },
                is_default,
            });
            rows
        }
        EntityData::Painting(painting) => {
            let mut rows = crate::entity::living::entity_synched_values(entity);
            rows.extend(
                hanging_synched_painting(painting)
                    .into_iter()
                    .map(|value| SynchedValue {
                        value,
                        is_default: false,
                    }),
            );
            rows
        }
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => {
            let mut rows = crate::entity::living::entity_synched_values(entity);
            rows.extend(hanging_synched_frame(frame));
            rows
        }
        // Empty subtype data (`LeashFenceKnotEntity.java:42-43`).
        EntityData::LeashKnot(_) => crate::entity::living::entity_synched_values(entity),
        EntityData::Arrow(arrow) => crate::entity::arrow::synched_values(entity, arrow),
        EntityData::SpectralArrow(arrow) => {
            crate::entity::spectral_arrow::synched_values(entity, arrow)
        }
        EntityData::Trident(trident) => crate::entity::trident::synched_values(entity, trident),
        EntityData::FishingBobber(bobber) => {
            crate::entity::fishing_bobber::synched_values(entity, bobber)
        }
        EntityData::Snowball(data)
        | EntityData::Egg(data)
        | EntityData::EnderPearl(data)
        | EntityData::ExperienceBottle(data) => {
            crate::entity::throwable_projectile::synched_values(entity, data)
        }
        EntityData::EndCrystal(crystal) => {
            crate::entity::end_crystal::synched_values(entity, crystal)
        }
        EntityData::EyeOfEnder(eye) => crate::entity::eye_of_ender::synched_values(entity, eye),
        EntityData::FireworkRocket(rocket) => {
            crate::entity::firework_rocket::synched_values(entity, rocket)
        }
        EntityData::LlamaSpit(_) => crate::entity::llama_spit::synched_values(entity),
        EntityData::EvokerFangs(_) => crate::entity::evoker_fangs::synched_values(entity),
        EntityData::LightningBolt(_) => crate::entity::lightning_bolt::synched_values(entity),
        EntityData::LargeFireball(fireball) => {
            crate::entity::hurting_projectile::fireball::synched_values(entity, fireball)
        }
        EntityData::SmallFireball(fireball) => {
            crate::entity::hurting_projectile::small_fireball::synched_values(entity, fireball)
        }
        EntityData::DragonFireball(_) => crate::entity::living::entity_synched_values(entity),
        EntityData::WitherSkull(skull) => {
            crate::entity::hurting_projectile::wither_skull::synched_values(entity, skull)
        }
        EntityData::WindCharge(_) | EntityData::BreezeWindCharge(_) => {
            crate::entity::living::entity_synched_values(entity)
        }
        EntityData::AreaEffectCloud(cloud) => {
            crate::entity::area_effect_cloud::synched_values(entity, cloud)
        }
        EntityData::OminousItemSpawner(spawner) => {
            crate::entity::ominous_item_spawner::synched_values(entity, spawner)
        }
        EntityData::SplashPotion(potion) => crate::entity::thrown_potion::synched_values(
            entity,
            potion,
            crate::entity::ThrownPotionKind::Splash,
        ),
        EntityData::LingeringPotion(potion) => crate::entity::thrown_potion::synched_values(
            entity,
            potion,
            crate::entity::ThrownPotionKind::Lingering,
        ),
        EntityData::ShulkerBullet(_) => crate::entity::living::entity_synched_values(entity),
        EntityData::Boat(data) => crate::entity::boat::synched_values(entity, data),
        EntityData::ChestBoat(data) => crate::entity::boat::synched_values(entity, data.boat()),
        EntityData::Minecart(data) => crate::entity::minecart::synched_values(entity, data),
        EntityData::FurnaceMinecart(data) => {
            crate::entity::minecart::furnace_synched_values(entity, data)
        }
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => {
            crate::entity::minecart::synched_values(entity, data.minecart())
        }
        EntityData::CommandBlockMinecart(data) => {
            let mut rows = crate::entity::minecart::synched_values(entity, data.minecart());
            data.append_synched_values(&mut rows);
            rows
        }
        EntityData::SpawnerMinecart(data) => {
            crate::entity::minecart::synched_values(entity, data.minecart())
        }
        EntityData::TntMinecart(data) => {
            crate::entity::minecart::synched_values(entity, data.minecart())
        }
    }
}

/// `ClientboundUpdateAttributesPacket(int, Collection<AttributeInstance>)`
/// (`ClientboundUpdateAttributesPacket.java:35`): the attribute, its base and
/// every modifier on it.
pub(crate) fn attribute_snapshot(
    instance: &crate::entity::attributes::AttributeInstance,
) -> AttributeSnapshot {
    AttributeSnapshot {
        attribute_id: instance.attribute_id(),
        base: instance.base_value(),
        modifiers: instance
            .modifiers()
            .iter()
            .map(|modifier| bbb_protocol::packets::AttributeModifier {
                id: modifier.id.clone(),
                amount: modifier.amount,
                operation_id: modifier.operation.id(),
            })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSyncFrame {
    pub shoulder_parrots: [Option<i32>; 2],
    pub shoulders_dirty: bool,
    pub entity_id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    /// `ServerPlayer.getKnownMovement()` (`ServerPlayer.java:2179-2182`).
    pub known_movement: Vec3d,
    /// `Entity.needsSync` (`Entity.java:274`), which `sendChanges` reads twice
    /// and clears once it has published (`ServerEntity.java:120`, `:172`,
    /// `:216`).
    pub needs_sync: bool,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: bool,
    pub riding: bool,
    pub shared_flags: i8,
    pub shared_flags_dirty: bool,
    pub air_supply: i32,
    pub air_supply_dirty: bool,
    pub silent: bool,
    pub silent_dirty: bool,
    pub pose_id: i32,
    pub pose_dirty: bool,
    pub sleeping_pos: Option<bbb_protocol::packets::BlockPos>,
    pub sleeping_pos_dirty: bool,
    pub ticks_frozen: i32,
    pub ticks_frozen_dirty: bool,
    /// `LivingEntity.DATA_LIVING_ENTITY_FLAGS` (`LivingEntity.java:178`).
    pub living_entity_flags: i8,
    pub living_entity_flags_dirty: bool,
    /// `LivingEntity.DATA_HEALTH_ID` (`LivingEntity.java:179`); its dirty bit
    /// is the publisher's comparison against the last value it sent.
    pub health: f32,
    /// `Player.DATA_PLAYER_ABSORPTION_ID` (`Player.java:134`), compared the
    /// same way.
    pub absorption: f32,
    /// `LivingEntity.DATA_EFFECT_PARTICLES` (`LivingEntity.java:180-182`).
    /// Particle payloads are copied only for pairing or a dirty particle row.
    pub effect_particles: Vec<bbb_protocol::packets::ParticlePayload>,
    pub effect_particles_dirty: bool,
    /// `LivingEntity.DATA_EFFECT_AMBIENCE_ID` (`LivingEntity.java:183`).
    pub effect_ambience: bool,
    pub effect_ambience_dirty: bool,
    /// `LivingEntity.DATA_ARROW_COUNT_ID` (`LivingEntity.java:184`).
    pub arrow_count: i32,
    pub arrow_count_dirty: bool,
    /// `LivingEntity.DATA_STINGER_COUNT_ID` (`LivingEntity.java:185`).
    pub stinger_count: i32,
    pub stinger_count_dirty: bool,
    pub main_hand: bbb_protocol::packets::ClientMainHand,
    pub main_hand_dirty: bool,
    pub model_customization: u8,
    pub model_customization_dirty: bool,
    pub dirty_attributes: Vec<AttributeSnapshot>,
    /// `Entity.hurtMarked` (`Entity.java:233`), which `sendChanges` clears as
    /// it publishes (`ServerEntity.java:220-223`).
    pub hurt_marked: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// Player-owned rows in `ServerEntity.sendPairingData`
/// (`ServerEntity.java:269-309`) after the add packet.
pub struct PlayerPairingData {
    pub metadata: Vec<EntityDataValue>,
    pub syncable_attributes: Vec<AttributeSnapshot>,
    pub equipment: Vec<EquipmentSlotUpdate>,
    pub vehicle_passengers: Option<SetPassengers>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerSyncPackets {
    pub ordered: Vec<PlayerSyncPacket>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSyncAudience {
    TrackingOnly,
    TrackingAndSelf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSyncPacket {
    pub audience: PlayerSyncAudience,
    pub packet: ClientboundPlayPacket,
}

#[derive(Debug, Clone, PartialEq)]
struct SyncFrame {
    position: Vec3d,
    tracking_position: Vec3d,
    delta_movement: Vec3d,
    known_movement: Vec3d,
    y_rot: f32,
    x_rot: f32,
    y_head_rot: f32,
    on_ground: bool,
    riding: bool,
    requires_precise_position: bool,
    is_abstract_arrow: bool,
    needs_sync: bool,
    fall_flying: bool,
    dirty_packets: Vec<ClientboundPlayPacket>,
    hurting_projectile_acceleration_power: Option<f64>,
    hurt_marked: bool,
    minecart_track_steps: Option<Vec<MinecartStep>>,
}

/// `Entity.trackingPosition()` (`Entity.java:3629-3631`); paintings measure
/// relative moves from their attached block (`Painting.java:191-193`).
fn tracking_position(entity: &Entity) -> Vec3d {
    match entity.data() {
        EntityData::Painting(_) => entity
            .data()
            .attached_block()
            .map_or(entity.position(), block_lower_corner),
        _ => entity.position(),
    }
}

fn block_lower_corner(pos: bbb_protocol::packets::BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
        z: f64::from(pos.z),
    }
}

/// `ServerEntity` (`ServerEntity.java:50-360`): one entity's outgoing state.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerEntity {
    entity_id: i32,
    /// `EntityType.updateInterval()` (`EntityType.java:1552-1554`), the tick
    /// period the whole of `sendChanges`' body is gated on
    /// (`ServerEntity.java:120`).
    update_interval: i32,
    /// `EntityType.trackDeltas()` (`EntityType.java:1556-1567`).
    track_delta: bool,
    /// `VecDeltaCodec.base` (`VecDeltaCodec.java:8`), which every relative move
    /// packet is measured against and which the pairing packet reports as the
    /// entity's position (`ClientboundAddEntityPacket.java:41-43`).
    position_base: Vec3d,
    last_sent_y_rot: i8,
    last_sent_x_rot: i8,
    last_sent_movement: Vec3d,
    /// `ServerEntity.tickCount` (`:66`), which counts calls to
    /// [`ServerEntity::send_changes`] and not level ticks.
    tick_count: i32,
    teleport_delay: i32,
    was_riding: bool,
    was_on_ground: bool,
    last_sent_y_head_rot: i8,
    /// `ServerEntity.trackedDataValues` (`:87`): the pairing snapshot, taken
    /// at construction and again whenever dirty data is sent (`:340`).
    tracked_data_values: Vec<EntityDataValue>,
    /// The health and absorption a player's last `send_player_changes`
    /// published, starting at their define-time defaults
    /// (`LivingEntity.java:314`, `Player.java:224`).
    player_vitals: (f32, f32),
    item_stack_registries: Arc<ItemStackRegistryPublication>,
}

impl ServerEntity {
    /// The `ServerEntity` constructor (`ServerEntity.java:73-88`).
    pub fn new(entity: &Entity, update_interval: i32, track_delta: bool) -> Self {
        Self::new_with_item_stack_registries(
            entity,
            update_interval,
            track_delta,
            Arc::new(ItemStackRegistryPublication::default()),
        )
    }

    fn new_with_item_stack_registries(
        entity: &Entity,
        update_interval: i32,
        track_delta: bool,
        item_stack_registries: Arc<ItemStackRegistryPublication>,
    ) -> Self {
        let tracked_data_values = tracked_data_values(entity);
        Self {
            entity_id: entity.id(),
            update_interval,
            track_delta,
            position_base: tracking_position(entity),
            last_sent_y_rot: pack_degrees(entity.y_rot()),
            last_sent_x_rot: pack_degrees(entity.x_rot()),
            last_sent_movement: entity.delta_movement(),
            tick_count: 0,
            teleport_delay: 0,
            was_riding: false,
            was_on_ground: entity.on_ground(),
            last_sent_y_head_rot: pack_degrees(entity.y_head_rot()),
            tracked_data_values,
            player_vitals: PLAYER_DEFAULT_VITALS,
            item_stack_registries,
        }
    }

    /// `ServerEntity.getPositionBase()` (`ServerEntity.java:316-318`).
    pub const fn position_base(&self) -> Vec3d {
        self.position_base
    }

    /// `ServerEntity.getLastSentMovement()` (`ServerEntity.java:320-322`).
    pub const fn last_sent_movement(&self) -> Vec3d {
        self.last_sent_movement
    }

    pub(super) fn framed_map_due(&self, entity: &Entity) -> Option<FramedMapFrame> {
        (self.tick_count % 10 == 0)
            .then(|| framed_map_frame(entity))
            .flatten()
    }

    /// `ServerEntity.sendPairingData` (`ServerEntity.java:269-313`); the
    /// passenger packets (`:303-309`) have nothing to act on here, because
    /// passenger trees publish their own.
    ///
    /// Every field of the add packet comes off *this* object rather than off
    /// the entity: `ClientboundAddEntityPacket`'s `(entity, serverEntity)`
    /// constructor reads the position base, the last sent rotations, the last
    /// sent movement and the last sent head rotation
    /// (`ClientboundAddEntityPacket.java:37-51`), so a session joining
    /// mid-flight is told the same position the relative moves it is about to
    /// receive are measured from.
    pub fn send_pairing_data(
        &self,
        entity: &Entity,
        viewers: &[TrackingViewer],
    ) -> Vec<ClientboundPlayPacket> {
        self.send_pairing_data_with_data(
            entity,
            add_entity_data(entity.data(), viewers, &FxHashMap::default()),
        )
    }

    fn send_pairing_data_with_data(
        &self,
        entity: &Entity,
        add_entity_data: i32,
    ) -> Vec<ClientboundPlayPacket> {
        let add_position = entity
            .data()
            .attached_block()
            .map_or(self.position_base, block_lower_corner);
        let mut packets = vec![ClientboundPlayPacket::AddEntity(AddEntity {
            id: self.entity_id,
            uuid: entity.uuid(),
            entity_type_id: entity.entity_type_id(),
            position: add_position,
            delta_movement: self.last_sent_movement,
            // `getLastSentXRot` / `getLastSentYRot` unpack the stored byte
            // (`ServerEntity.java:324-330`) and the packet packs it again
            // (`ClientboundAddEntityPacket.java:88-89`).
            x_rot: unpack_degrees(self.last_sent_x_rot),
            y_rot: unpack_degrees(self.last_sent_y_rot),
            y_head_rot: unpack_degrees(self.last_sent_y_head_rot),
            data: add_entity_data,
        })];
        // `:277-279`: the snapshot is sent only when there is one, which
        // `getNonDefaultValues` reports as a null list.
        if !self.tracked_data_values.is_empty() {
            packets.push(ClientboundPlayPacket::SetEntityData(SetEntityData {
                id: self.entity_id,
                values: self.tracked_data_values.clone(),
            }));
        }
        if let Some(living) = entity.living() {
            // `:281-286`.
            let attributes: Vec<AttributeSnapshot> = living
                .attributes()
                .syncable_attributes()
                .map(attribute_snapshot)
                .collect();
            if !attributes.is_empty() {
                packets.push(ClientboundPlayPacket::UpdateAttributes(UpdateAttributes {
                    entity_id: self.entity_id,
                    attributes,
                }));
            }
            // `:288-301`.
            let slots: Vec<EquipmentSlotUpdate> = VANILLA_EQUIPMENT_SLOTS
                .iter()
                .filter_map(|entry| {
                    let stack = living.item_by_slot(entry.slot)?;
                    (!stack.is_empty()).then(|| EquipmentSlotUpdate {
                        slot: entry.slot,
                        item: stack.summary().clone(),
                    })
                })
                .collect();
            if !slots.is_empty() {
                packets.push(ClientboundPlayPacket::SetEquipment(SetEquipment {
                    entity_id: self.entity_id,
                    slots,
                }));
            }
        }
        // `:311-313`.
        if let Some(holder) =
            crate::entity::leash::leash_data(entity).and_then(|data| data.leash_holder())
        {
            packets.push(ClientboundPlayPacket::SetEntityLink(SetEntityLink {
                source_id: self.entity_id,
                dest_id: holder,
            }));
        }
        packets
    }

    /// `ServerEntity.sendDirtyEntityData` (`ServerEntity.java:336-352`) over
    /// `SynchedEntityData.packDirty` (`SynchedEntityData.java:74-90`).
    fn take_dirty_entity_data(
        &mut self,
        entity: &mut Entity,
        emit_packets: bool,
    ) -> Vec<ClientboundPlayPacket> {
        let mut packets = Vec::new();
        let dirty_mask = entity.take_synched_dirty_mask();
        if dirty_mask_is_non_empty(dirty_mask) {
            let dirty_rows = dirty_synched_values(entity, dirty_mask);
            if emit_packets {
                let mut values = Vec::with_capacity(dirty_rows.len());
                for row in dirty_rows {
                    update_tracked_data_value(&mut self.tracked_data_values, &row);
                    values.push(row.value);
                }
                packets.push(ClientboundPlayPacket::SetEntityData(SetEntityData {
                    id: self.entity_id,
                    values,
                }));
            } else {
                for row in dirty_rows {
                    update_tracked_data_value(&mut self.tracked_data_values, &row);
                }
            }
        }
        if let Some(living) = entity.living_mut() {
            // `:344-350`.
            if emit_packets {
                let attributes: Vec<AttributeSnapshot> = living
                    .attributes_mut()
                    .drain_attributes_to_sync()
                    .map(attribute_snapshot)
                    .collect();
                if !attributes.is_empty() {
                    packets.push(ClientboundPlayPacket::UpdateAttributes(UpdateAttributes {
                        entity_id: self.entity_id,
                        attributes,
                    }));
                }
            } else {
                for _ in living.attributes_mut().drain_attributes_to_sync() {}
            }
        }
        packets
    }

    /// `ServerEntity.removePairing` (`ServerEntity.java:257-260`) minus
    /// `Entity.stopSeenByPlayer`, whose only implementation among these kinds
    /// is empty.
    pub fn remove_pairing(&self) -> ClientboundPlayPacket {
        ClientboundPlayPacket::RemoveEntities(RemoveEntities {
            entity_ids: vec![self.entity_id],
        })
    }

    /// `ServerEntity.sendChanges` (`ServerEntity.java:90-224`).
    ///
    /// The returned packets all go to the tracking sessions:
    /// `sendToTrackingPlayersAndSelf` (`:354-359`) only differs from
    /// `sendToTrackingPlayers` for a `ServerPlayer`, and no player is one of
    /// these entities.
    ///
    pub fn send_changes(&mut self, entity: &mut Entity) -> Vec<ClientboundPlayPacket> {
        self.send_changes_with_riding(entity, false, false, true)
    }

    fn send_changes_with_riding(
        &mut self,
        entity: &mut Entity,
        is_riding: bool,
        minecart_improvements: bool,
        emit_packets: bool,
    ) -> Vec<ClientboundPlayPacket> {
        let needs_sync = entity.needs_sync();
        let dirty_bypasses_interval = entity.synched_data_is_dirty();
        let due = self.changes_due(needs_sync, dirty_bypasses_interval);
        let dirty_packets = if due {
            self.take_dirty_entity_data(entity, emit_packets)
        } else {
            Vec::new()
        };
        let minecart_track_steps = if due && !is_riding {
            entity.take_minecart_track_steps(minecart_improvements)
        } else {
            None
        };
        let frame = SyncFrame {
            position: entity.position(),
            tracking_position: tracking_position(entity),
            delta_movement: entity.delta_movement(),
            known_movement: entity.delta_movement(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            y_head_rot: entity.y_head_rot(),
            on_ground: entity.on_ground(),
            riding: is_riding,
            requires_precise_position: matches!(entity.data(), EntityData::GeneralLiving(data)
                if data.happy_ghast_state().is_some_and(|state| state.requires_precise_position)),
            is_abstract_arrow: matches!(
                entity.data(),
                EntityData::Arrow(_) | EntityData::SpectralArrow(_) | EntityData::Trident(_)
            ),
            needs_sync,
            fall_flying: entity.is_fall_flying(),
            dirty_packets,
            hurting_projectile_acceleration_power: entity.hurting_projectile_acceleration_power(),
            hurt_marked: entity.take_hurt_marked(),
            minecart_track_steps,
        };
        let packets = self.send_frame_changes(frame, due, emit_packets);
        if due {
            entity.set_needs_sync(false);
        }
        if !emit_packets {
            return Vec::new();
        }
        packets
            .into_iter()
            .map(|targeted| targeted.packet)
            .collect()
    }

    /// `EntityType.PLAYER` uses range 32 and interval 2
    /// (`EntityType.java:1163-1173`) and is excluded from delta tracking
    /// (`EntityType.java:1556-1567`).
    pub fn for_player(frame: &PlayerSyncFrame) -> Self {
        let player = vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID)
            .expect("the generated entity catalog contains PLAYER");
        Self {
            entity_id: frame.entity_id,
            update_interval: player.update_interval,
            track_delta: player.track_deltas,
            position_base: frame.position,
            last_sent_y_rot: pack_degrees(frame.y_rot),
            last_sent_x_rot: pack_degrees(frame.x_rot),
            last_sent_movement: frame.delta_movement,
            tick_count: 0,
            teleport_delay: 0,
            was_riding: false,
            was_on_ground: frame.on_ground,
            last_sent_y_head_rot: pack_degrees(frame.y_head_rot),
            tracked_data_values: Vec::new(),
            player_vitals: PLAYER_DEFAULT_VITALS,
            item_stack_registries: Arc::new(ItemStackRegistryPublication::default()),
        }
    }

    /// `ServerEntity.sendChanges` (`ServerEntity.java:90-224`) for the player
    /// state kept by the server world rather than by [`EntityStore`].
    pub fn send_player_changes(&mut self, frame: PlayerSyncFrame) -> PlayerSyncPackets {
        let mut dirty_packets = Vec::new();
        // `SynchedEntityData.set` marks only a changed value dirty
        // (`SynchedEntityData.java:60-68`).
        let (health, absorption) = self.player_vitals;
        let values = player_metadata(
            &frame,
            PlayerRows::Dirty {
                health: health.to_bits() != frame.health.to_bits(),
                absorption: absorption.to_bits() != frame.absorption.to_bits(),
            },
        );
        self.player_vitals = (frame.health, frame.absorption);
        if !values.is_empty() {
            dirty_packets.push(ClientboundPlayPacket::SetEntityData(SetEntityData {
                id: self.entity_id,
                values,
            }));
        }
        if !frame.dirty_attributes.is_empty() {
            dirty_packets.push(ClientboundPlayPacket::UpdateAttributes(UpdateAttributes {
                entity_id: self.entity_id,
                attributes: frame.dirty_attributes,
            }));
        }
        let dirty_bypasses_interval = !dirty_packets.is_empty();
        let due = self.changes_due(frame.needs_sync, dirty_bypasses_interval);
        let ordered = self.send_frame_changes(
            SyncFrame {
                position: frame.position,
                tracking_position: frame.position,
                delta_movement: frame.delta_movement,
                known_movement: frame.known_movement,
                y_rot: frame.y_rot,
                x_rot: frame.x_rot,
                y_head_rot: frame.y_head_rot,
                on_ground: frame.on_ground,
                riding: frame.riding,
                requires_precise_position: false,
                is_abstract_arrow: false,
                needs_sync: frame.needs_sync,
                fall_flying: frame.shared_flags
                    & bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING
                    != 0,
                dirty_packets,
                hurting_projectile_acceleration_power: None,
                hurt_marked: frame.hurt_marked,
                minecart_track_steps: None,
            },
            due,
            true,
        );
        PlayerSyncPackets { ordered }
    }

    fn send_player_pairing_data(
        &self,
        uuid: uuid::Uuid,
        pairing: PlayerPairingData,
    ) -> Vec<ClientboundPlayPacket> {
        let mut packets = vec![ClientboundPlayPacket::AddEntity(AddEntity {
            id: self.entity_id,
            uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: self.position_base,
            delta_movement: self.last_sent_movement,
            x_rot: unpack_degrees(self.last_sent_x_rot),
            y_rot: unpack_degrees(self.last_sent_y_rot),
            y_head_rot: unpack_degrees(self.last_sent_y_head_rot),
            data: 0,
        })];
        if !pairing.metadata.is_empty() {
            packets.push(ClientboundPlayPacket::SetEntityData(SetEntityData {
                id: self.entity_id,
                values: pairing.metadata,
            }));
        }
        if !pairing.syncable_attributes.is_empty() {
            packets.push(ClientboundPlayPacket::UpdateAttributes(UpdateAttributes {
                entity_id: self.entity_id,
                attributes: pairing.syncable_attributes,
            }));
        }
        if !pairing.equipment.is_empty() {
            packets.push(ClientboundPlayPacket::SetEquipment(SetEquipment {
                entity_id: self.entity_id,
                slots: pairing.equipment,
            }));
        }
        if let Some(passengers) = pairing.vehicle_passengers {
            packets.push(ClientboundPlayPacket::SetPassengers(passengers));
        }
        packets
    }

    fn changes_due(&self, needs_sync: bool, dirty_bypasses_interval: bool) -> bool {
        self.tick_count % self.update_interval == 0 || needs_sync || dirty_bypasses_interval
    }

    fn send_frame_changes(
        &mut self,
        mut frame: SyncFrame,
        due: bool,
        emit_packets: bool,
    ) -> Vec<PlayerSyncPacket> {
        let mut packets = Vec::new();
        let tracking = |packet| PlayerSyncPacket {
            audience: PlayerSyncAudience::TrackingOnly,
            packet,
        };
        let tracking_and_self = |packet| PlayerSyncPacket {
            audience: PlayerSyncAudience::TrackingAndSelf,
            packet,
        };
        // `ServerEntity.sendChanges` (`ServerEntity.java:120-216`).
        if due {
            let y_rot = pack_degrees(frame.y_rot);
            let x_rot = pack_degrees(frame.x_rot);
            // `:123`. The subtraction is on `byte`s promoted to `int`, so it
            // does not wrap the way a byte subtraction would.
            let should_send_rotation = (i32::from(y_rot) - i32::from(self.last_sent_y_rot)).abs()
                >= TOLERANCE_LEVEL_ROTATION
                || (i32::from(x_rot) - i32::from(self.last_sent_x_rot)).abs()
                    >= TOLERANCE_LEVEL_ROTATION;
            if frame.riding {
                if should_send_rotation {
                    if emit_packets {
                        packets.push(tracking(ClientboundPlayPacket::MoveEntity(EntityMove {
                            id: self.entity_id,
                            has_position: false,
                            delta_x: 0,
                            delta_y: 0,
                            delta_z: 0,
                            y_rot: Some(frame.y_rot),
                            x_rot: Some(frame.x_rot),
                            on_ground: frame.on_ground,
                        })));
                    }
                    self.last_sent_y_rot = y_rot;
                    self.last_sent_x_rot = x_rot;
                }
                self.position_base = frame.tracking_position;
                if emit_packets {
                    packets.extend(frame.dirty_packets.drain(..).map(tracking_and_self));
                }
                self.was_riding = true;
            } else if let Some(lerp_steps) = frame.minecart_track_steps {
                // `ServerEntity.handleMinecartPosRot` (`ServerEntity.java:134-135`, `:226-255`).
                if emit_packets {
                    packets.extend(frame.dirty_packets.drain(..).map(tracking_and_self));
                }
                if lerp_steps.is_empty() {
                    let movement = frame.delta_movement;
                    let difference = distance_to_sqr(movement, self.last_sent_movement);
                    let current = frame.tracking_position;
                    let delta_x = current.x - self.position_base.x;
                    let delta_y = current.y - self.position_base.y;
                    let delta_z = current.z - self.position_base.z;
                    let position_changed =
                        delta_x * delta_x + delta_y * delta_y + delta_z * delta_z
                            >= TOLERANCE_LEVEL_POSITION;
                    let should_send_position =
                        position_changed || self.tick_count % FORCED_POS_UPDATE_PERIOD == 0;
                    if should_send_position
                        || should_send_rotation
                        || difference > MOVEMENT_TOLERANCE
                    {
                        self.last_sent_movement = movement;
                        if emit_packets {
                            packets.push(tracking(ClientboundPlayPacket::MoveMinecartAlongTrack(
                                MoveMinecartAlongTrack {
                                    entity_id: self.entity_id,
                                    lerp_steps: vec![MinecartStep {
                                        position: frame.position,
                                        movement: frame.delta_movement,
                                        y_rot: frame.y_rot,
                                        x_rot: frame.x_rot,
                                        weight: 1.0,
                                    }],
                                },
                            )));
                        }
                    }
                } else {
                    self.last_sent_movement = frame.delta_movement;
                    if emit_packets {
                        packets.push(tracking(ClientboundPlayPacket::MoveMinecartAlongTrack(
                            MoveMinecartAlongTrack {
                                entity_id: self.entity_id,
                                lerp_steps,
                            },
                        )));
                    }
                }
                self.last_sent_y_rot = y_rot;
                self.last_sent_x_rot = x_rot;
                self.position_base = frame.position;
                self.was_riding = false;
            } else {
                // `:137`, the `else` arm of the passenger and minecart tests.
                self.teleport_delay += 1;
                let current = frame.tracking_position;
                let delta_x = current.x - self.position_base.x;
                let delta_y = current.y - self.position_base.y;
                let delta_z = current.z - self.position_base.z;
                // `:139` over `VecDeltaCodec.delta` (`VecDeltaCodec.java:43-45`).
                let position_changed = delta_x * delta_x + delta_y * delta_y + delta_z * delta_z
                    >= TOLERANCE_LEVEL_POSITION;
                // `:141`.
                let pos = position_changed || self.tick_count % FORCED_POS_UPDATE_PERIOD == 0;
                let xa = encode_position_component(current.x)
                    - encode_position_component(self.position_base.x);
                let ya = encode_position_component(current.y)
                    - encode_position_component(self.position_base.y);
                let za = encode_position_component(current.z)
                    - encode_position_component(self.position_base.z);
                // `:147`.
                let delta_too_big = !(RELATIVE_MOVE_MIN..=RELATIVE_MOVE_MAX).contains(&xa)
                    || !(RELATIVE_MOVE_MIN..=RELATIVE_MOVE_MAX).contains(&ya)
                    || !(RELATIVE_MOVE_MIN..=RELATIVE_MOVE_MAX).contains(&za);
                let mut sent_position = false;
                let mut sent_rotation = false;
                let packet = if delta_too_big
                    || frame.requires_precise_position
                    || self.teleport_delay > FORCED_TELEPORT_PERIOD
                    || self.was_riding
                    || self.was_on_ground != frame.on_ground
                {
                    // `:148-157`. 26.1 resyncs with
                    // `ClientboundEntityPositionSyncPacket`, not with a teleport
                    // packet, and it carries the live movement rather than the
                    // last published one (`ClientboundEntityPositionSyncPacket.java:22-26`).
                    self.was_on_ground = frame.on_ground;
                    self.teleport_delay = 0;
                    sent_position = true;
                    sent_rotation = true;
                    emit_packets.then_some(ClientboundPlayPacket::EntityPositionSync(
                        EntityPositionSync {
                            id: self.entity_id,
                            position: frame.tracking_position,
                            delta_movement: frame.known_movement,
                            y_rot: frame.y_rot,
                            x_rot: frame.x_rot,
                            on_ground: frame.on_ground,
                        },
                    ))
                } else if (!pos || !should_send_rotation) && !frame.is_abstract_arrow {
                    // `:158-165`.
                    if pos {
                        sent_position = true;
                        emit_packets.then_some(ClientboundPlayPacket::MoveEntity(EntityMove {
                            id: self.entity_id,
                            has_position: true,
                            delta_x: xa as i16,
                            delta_y: ya as i16,
                            delta_z: za as i16,
                            y_rot: None,
                            x_rot: None,
                            on_ground: frame.on_ground,
                        }))
                    } else if should_send_rotation {
                        sent_rotation = true;
                        emit_packets.then_some(ClientboundPlayPacket::MoveEntity(EntityMove {
                            id: self.entity_id,
                            has_position: false,
                            delta_x: 0,
                            delta_y: 0,
                            delta_z: 0,
                            y_rot: Some(frame.y_rot),
                            x_rot: Some(frame.x_rot),
                            on_ground: frame.on_ground,
                        }))
                    } else {
                        None
                    }
                } else {
                    // `:166-170`.
                    sent_position = true;
                    sent_rotation = true;
                    emit_packets.then_some(ClientboundPlayPacket::MoveEntity(EntityMove {
                        id: self.entity_id,
                        has_position: true,
                        delta_x: xa as i16,
                        delta_y: ya as i16,
                        delta_z: za as i16,
                        y_rot: Some(frame.y_rot),
                        x_rot: Some(frame.x_rot),
                        on_ground: frame.on_ground,
                    }))
                };

                // `:172-191`: hurting projectiles keep power adjacent to motion in
                // an inner bundle so the client never predicts with one half.
                if frame.needs_sync || self.track_delta || frame.fall_flying {
                    // The server never integrates a gliding player's velocity,
                    // so trackers get the client-reported movement instead.
                    let movement = if frame.fall_flying {
                        frame.known_movement
                    } else {
                        frame.delta_movement
                    };
                    let difference = distance_to_sqr(movement, self.last_sent_movement);
                    if difference > MOVEMENT_TOLERANCE
                        || (difference > 0.0 && length_sqr(movement) == 0.0)
                    {
                        self.last_sent_movement = movement;
                        if emit_packets {
                            let power = frame.hurting_projectile_acceleration_power;
                            if power.is_some() {
                                packets.push(tracking(ClientboundPlayPacket::BundleDelimiter));
                            }
                            packets.push(tracking(ClientboundPlayPacket::SetEntityMotion(
                                SetEntityMotion {
                                    id: self.entity_id,
                                    delta_movement: self.last_sent_movement,
                                },
                            )));
                            if let Some(acceleration_power) = power {
                                packets.push(tracking(ClientboundPlayPacket::ProjectilePower(
                                    ProjectilePower {
                                        entity_id: self.entity_id,
                                        acceleration_power,
                                    },
                                )));
                                packets.push(tracking(ClientboundPlayPacket::BundleDelimiter));
                            }
                        }
                    }
                }

                // `:193-195`: the movement packet is queued before the position
                // one is written, so the velocity reaches the client first.
                if let Some(packet) = packet {
                    packets.push(tracking(packet));
                }
                // `:197`.
                if emit_packets {
                    packets.extend(frame.dirty_packets.drain(..).map(tracking_and_self));
                }
                // `:198-205`.
                if sent_position {
                    self.position_base = current;
                }
                if sent_rotation {
                    self.last_sent_y_rot = y_rot;
                    self.last_sent_x_rot = x_rot;
                }
                self.was_riding = false;
            }
            // `:210-214`; the subtraction is on `byte`s promoted to `int`.
            let y_head_rot = pack_degrees(frame.y_head_rot);
            if (i32::from(y_head_rot) - i32::from(self.last_sent_y_head_rot)).abs() >= 1 {
                if emit_packets {
                    packets.push(tracking(ClientboundPlayPacket::RotateHead(RotateHead {
                        id: self.entity_id,
                        y_head_rot: frame.y_head_rot,
                    })));
                }
                self.last_sent_y_head_rot = y_head_rot;
            }
        }
        // `:219-223`, outside the interval gate. This does not update
        // `lastSentMovement`: Vanilla's hurt resend writes the packet directly,
        // so the next regular delta pass may publish the same velocity again.
        self.tick_count += 1;
        if frame.hurt_marked && emit_packets {
            packets.push(tracking_and_self(ClientboundPlayPacket::SetEntityMotion(
                SetEntityMotion {
                    id: self.entity_id,
                    delta_movement: frame.delta_movement,
                },
            )));
        }
        packets
    }
}

/// `Vec3.distanceToSqr(Vec3)`.
fn distance_to_sqr(from: Vec3d, to: Vec3d) -> f64 {
    let dx = from.x - to.x;
    let dy = from.y - to.y;
    let dz = from.z - to.z;
    dx * dx + dy * dy + dz * dz
}

/// `Vec3.lengthSqr()`.
fn length_sqr(vector: Vec3d) -> f64 {
    vector.x * vector.x + vector.y * vector.y + vector.z * vector.z
}

/// One session the tracker may pair an entity with.
///
/// These are the three `ServerPlayer` facts `TrackedEntity.updatePlayer`
/// (`ChunkMap.java:1385-1408`) reads that the caller has to supply; the fourth,
/// `ChunkMap.isChunkTracked` (`ChunkMap.java:227-229`), arrives as a callback
/// because it reaches the session's chunk-tracking view and its pending-send
/// queue, both of which the caller owns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackingViewer {
    pub connection_id: ConnectionId,
    pub position: Vec3d,
    /// `ChunkMap.getPlayerViewDistance(player)` (`ChunkMap.java:821-823`).
    pub view_distance: i32,
    /// `Entity.getUUID()` and `Entity.getId()` of the session's avatar: what
    /// `level.getEntity(uuid)` answers for a player of this level, which is how
    /// a projectile's owner reaches `ClientboundAddEntityPacket.data`
    /// (`Projectile.getAddEntityPacket`, `Projectile.java:346-349`).
    pub uuid: uuid::Uuid,
    pub entity_id: i32,
}

/// `Projectile.getAddEntityPacket`'s `owner == null ? 0 : owner.getId()`
/// (`Projectile.java:346-349`) with `getOwner()` resolved against the sessions
/// and validated level-entity UUID indexes. A missing owner answers `0`.
fn projectile_owner_entity_id(
    data: &EntityData,
    viewers: &[TrackingViewer],
    entity_ids: &FxHashMap<uuid::Uuid, i32>,
) -> i32 {
    let Some(owner) = data.projectile_owner() else {
        return 0;
    };
    viewers
        .iter()
        .find(|viewer| viewer.uuid == owner)
        .map(|viewer| viewer.entity_id)
        .or_else(|| entity_ids.get(&owner).copied())
        .unwrap_or(0)
}

#[cfg(test)]
pub mod tests;
