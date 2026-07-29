use std::collections::{HashMap, HashSet};

use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, vanilla_entity_type_is_serializable,
};
use bbb_protocol::packets::ProjectilePower as ProtocolProjectilePower;

use uuid::Uuid;

use crate::{HurtingProjectileState, ProjectilePowerUpdateState, WorldStore};

use super::schema::{
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
    vanilla_entity_metadata_type_is_hurting_projectile, vanilla_entity_metadata_type_is_pickable,
    vanilla_entity_metadata_type_is_projectile, vanilla_entity_metadata_type_is_wind_charge,
};
use super::{
    EntityAbstractArrowRuntime, EntityHurtingProjectile, EntityIdentity, EntityMount,
    EntityPickBoundsState, EntityProjectileLifecycle, EntityStore, EntityTransform, EntityVec3,
    ProjectileLifecycleState, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_TRIDENT_ID,
};

const DEFAULT_HURTING_PROJECTILE_ACCELERATION_POWER: f64 = 0.1;
const DEFAULT_WIND_CHARGE_ACCELERATION_POWER: f64 = 0.0;
#[derive(Debug, Clone, Copy)]
struct EntityAabb {
    min: [f64; 3],
    max: [f64; 3],
}

impl EntityAabb {
    fn expand_towards_and_inflate(self, delta: EntityVec3, amount: f64) -> Self {
        let mut bounds = self;
        for (axis, delta) in [delta.x, delta.y, delta.z].into_iter().enumerate() {
            if delta < 0.0 {
                bounds.min[axis] += delta;
            } else {
                bounds.max[axis] += delta;
            }
            bounds.min[axis] -= amount;
            bounds.max[axis] += amount;
        }
        bounds
    }

    fn intersects(self, other: Self) -> bool {
        self.min[0] < other.max[0]
            && self.max[0] > other.min[0]
            && self.min[1] < other.max[1]
            && self.max[1] > other.min[1]
            && self.min[2] < other.max[2]
            && self.max[2] > other.min[2]
    }
}

impl WorldStore {
    pub fn apply_projectile_power(&mut self, packet: ProtocolProjectilePower) -> bool {
        self.counters.projectile_power_packets += 1;
        let applied = self
            .entities
            .with_hurting_projectile_mut(packet.entity_id, |projectile| {
                projectile.acceleration_power = packet.acceleration_power;
            })
            .is_some();

        if applied {
            self.counters.projectile_power_updates_applied += 1;
        } else {
            self.counters.projectile_power_updates_ignored += 1;
        }

        self.last_projectile_power = Some(ProjectilePowerUpdateState {
            entity_id: packet.entity_id,
            acceleration_power: packet.acceleration_power,
            applied,
        });
        applied
    }
}

pub(crate) fn initial_hurting_projectile_state(
    entity_type_id: i32,
) -> Option<HurtingProjectileState> {
    if !is_vanilla_hurting_projectile_type(entity_type_id) {
        return None;
    }
    let acceleration_power = if is_vanilla_wind_charge_type(entity_type_id) {
        DEFAULT_WIND_CHARGE_ACCELERATION_POWER
    } else {
        DEFAULT_HURTING_PROJECTILE_ACCELERATION_POWER
    };
    Some(HurtingProjectileState { acceleration_power })
}

pub(crate) fn entity_hurting_projectile_from_state(
    entity_type_id: i32,
    state: Option<HurtingProjectileState>,
) -> Option<EntityHurtingProjectile> {
    if !is_vanilla_hurting_projectile_type(entity_type_id) {
        return None;
    }
    state
        .or_else(|| initial_hurting_projectile_state(entity_type_id))
        .map(EntityHurtingProjectile::from)
}

pub(crate) fn entity_projectile_lifecycle_from_state(
    entity_type_id: i32,
    state: Option<ProjectileLifecycleState>,
    spawn_owner_uuid: Option<Uuid>,
    spawn_owner_is_player: bool,
) -> Option<EntityProjectileLifecycle> {
    if !is_vanilla_saved_projectile_type(entity_type_id) {
        return None;
    }
    Some(EntityProjectileLifecycle::from(
        state.unwrap_or(ProjectileLifecycleState {
            owner_uuid: spawn_owner_uuid,
            left_owner: false,
            has_been_shot: false,
            trident_in_ground_time: 0,
            trident_dealt_damage: false,
            trident_return_ticks: 0,
            arrow_pickup_allowed: spawn_owner_is_player
                && vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
                    .is_some(),
            arrow_no_physics: false,
            arrow_last_block_state_id: None,
            arrow_hit_ground_sound_is_arrow: false,
        }),
    ))
}

pub(crate) fn entity_abstract_arrow_runtime_from_uuid(
    entity_type_id: i32,
    uuid: Uuid,
) -> Option<EntityAbstractArrowRuntime> {
    entity_abstract_arrow_runtime_from_state(entity_type_id, uuid, None)
}

pub(crate) fn entity_abstract_arrow_runtime_from_state(
    entity_type_id: i32,
    uuid: Uuid,
    state: Option<&super::AbstractArrowRuntimeState>,
) -> Option<EntityAbstractArrowRuntime> {
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
        .map(|_| EntityAbstractArrowRuntime::from_state(uuid, state))
}

pub(crate) fn is_vanilla_saved_projectile_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_projectile(entity_type_id)
        && vanilla_entity_type_is_serializable(entity_type_id)
}

/// Vanilla 26.1 `EntityType.Builder.sized` dimensions for every saveable
/// `Projectile` subtype. Some are intentionally absent from the renderer's pick
/// table because `Projectile.isPickable()` is false, but `Projectile.LeftOwner`
/// still uses their real bounding box.
pub(crate) fn vanilla_saved_projectile_bounds(
    entity_type_id: i32,
) -> Option<EntityPickBoundsState> {
    is_vanilla_saved_projectile_type(entity_type_id).then_some(())?;
    let [width, height] = vanilla_entity_type_dimensions(entity_type_id)?;
    Some(EntityPickBoundsState::from_base_size(width, height, 0.0))
}

impl EntityStore {
    /// Advances vanilla `Projectile.tick`'s client-owned parent state. The owner
    /// reference is fixed from the add-entity packet, while `LeftOwner` becomes
    /// sticky once the swept/inflated projectile box no longer intersects any
    /// pickable member of the owner's root-vehicle passenger graph.
    pub(crate) fn advance_projectile_lifecycles(
        &mut self,
        ticks: u32,
        spectator_player_uuids: &HashSet<Uuid>,
        scheduled: &HashSet<i32>,
    ) {
        if ticks == 0 {
            return;
        }
        let has_projectiles = self
            .ecs
            .query::<&EntityProjectileLifecycle>()
            .iter()
            .next()
            .is_some();
        if !has_projectiles {
            return;
        }
        let outside_owner_by_id =
            self.projectile_outside_owner_collision_by_id(spectator_player_uuids);
        let trident_counts_in_ground_by_id = self.trident_counts_in_ground_by_id();
        for (_, (identity, projectile)) in self
            .ecs
            .query_mut::<(&EntityIdentity, &mut EntityProjectileLifecycle)>()
        {
            if !scheduled.contains(&identity.id) {
                continue;
            }
            projectile.has_been_shot = true;
            if !projectile.left_owner {
                projectile.left_owner = outside_owner_by_id
                    .get(&identity.id)
                    .copied()
                    .unwrap_or(true);
            }
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
                advance_trident_lifecycle(
                    projectile,
                    ticks,
                    trident_counts_in_ground_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or(false),
                );
            }
        }
    }

    /// `AbstractArrow.tick` increments `inGroundTime` only while the synced
    /// in-ground flag is set and the synced no-physics flag is clear.
    fn trident_counts_in_ground_by_id(&self) -> HashMap<i32, bool> {
        self.order
            .iter()
            .filter_map(|id| {
                let entity = self.by_protocol_id.get(id).copied()?;
                let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
                if identity.entity_type_id != VANILLA_ENTITY_TYPE_TRIDENT_ID {
                    return None;
                }
                let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
                    identity.entity_type_id,
                )?;
                let in_ground = self.metadata_bool_for_entity(
                    *id,
                    accessors.in_ground_data_id,
                    accessors.in_ground_default,
                )?;
                let physics_enabled =
                    self.metadata_byte(*id, accessors.flags_data_id, accessors.flags_default)?
                        & accessors.no_physics_mask
                        == 0;
                Some((*id, in_ground && physics_enabled))
            })
            .collect()
    }

    pub(super) fn projectile_outside_owner_collision_by_id(
        &self,
        spectator_player_uuids: &HashSet<Uuid>,
    ) -> HashMap<i32, bool> {
        let mut outside = HashMap::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let (identity, transform, projectile) = {
                let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                    continue;
                };
                let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                    continue;
                };
                let Ok(projectile) = self.ecs.get::<&EntityProjectileLifecycle>(entity) else {
                    continue;
                };
                ((*identity).clone(), *transform, *projectile)
            };
            if projectile.left_owner {
                continue;
            }
            let is_outside = projectile.owner_uuid.is_none_or(|owner_uuid| {
                let Some(owner_id) = self.protocol_id_for_uuid(owner_uuid) else {
                    return true;
                };
                let Some(projectile_box) = self.entity_aabb(identity.id) else {
                    return true;
                };
                let swept_projectile_box =
                    projectile_box.expand_towards_and_inflate(transform.delta_movement, 1.0);
                let root_id = self.root_vehicle_id(owner_id);
                self.self_and_passenger_ids(root_id)
                    .into_iter()
                    .all(|candidate_id| {
                        !self.entity_is_pickable_for_owner_collision(
                            candidate_id,
                            spectator_player_uuids,
                        ) || self.entity_aabb(candidate_id).is_none_or(|candidate_box| {
                            !swept_projectile_box.intersects(candidate_box)
                        })
                    })
            });
            outside.insert(identity.id, is_outside);
        }
        outside
    }

    pub(crate) fn protocol_id_for_uuid(&self, uuid: Uuid) -> Option<i32> {
        self.protocol_id_by_uuid.get(&uuid).copied()
    }

    fn root_vehicle_id(&self, id: i32) -> i32 {
        let mut root_id = id;
        let mut visited = Vec::new();
        while !visited.contains(&root_id) {
            visited.push(root_id);
            let Some(vehicle_id) = self
                .by_protocol_id
                .get(&root_id)
                .and_then(|entity| self.ecs.get::<&EntityMount>(*entity).ok())
                .and_then(|mount| mount.vehicle_id)
            else {
                break;
            };
            root_id = vehicle_id;
        }
        root_id
    }

    fn self_and_passenger_ids(&self, root_id: i32) -> Vec<i32> {
        let mut ids = Vec::new();
        let mut pending = vec![root_id];
        while let Some(id) = pending.pop() {
            if ids.contains(&id) {
                continue;
            }
            ids.push(id);
            if let Some(mount) = self
                .by_protocol_id
                .get(&id)
                .and_then(|entity| self.ecs.get::<&EntityMount>(*entity).ok())
            {
                pending.extend(mount.passengers.iter().copied());
            }
        }
        ids
    }

    fn entity_aabb(&self, id: i32) -> Option<EntityAabb> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let bounds = self
            .pick_bounds(id)
            .or_else(|| vanilla_saved_projectile_bounds(identity.entity_type_id))?;
        Some(EntityAabb {
            min: [
                transform.position.x + f64::from(bounds.min[0]),
                transform.position.y + f64::from(bounds.min[1]),
                transform.position.z + f64::from(bounds.min[2]),
            ],
            max: [
                transform.position.x + f64::from(bounds.max[0]),
                transform.position.y + f64::from(bounds.max[1]),
                transform.position.z + f64::from(bounds.max[2]),
            ],
        })
    }

    fn entity_is_pickable_for_owner_collision(
        &self,
        id: i32,
        spectator_player_uuids: &HashSet<Uuid>,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
            return false;
        };
        if !vanilla_entity_metadata_type_is_pickable(identity.entity_type_id) {
            return false;
        }
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && spectator_player_uuids.contains(&identity.uuid)
        {
            return false;
        }
        identity.entity_type_id != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
            || vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|accessors| {
                self.metadata_byte(
                    id,
                    accessors.client_flags_data_id,
                    accessors.client_flags_default,
                )
                .unwrap_or(accessors.client_flags_default)
                    & accessors.marker_mask
                    == 0
            })
    }
}

/// Advances the private fields in vanilla call order: `ThrownTrident.tick`
/// checks the previous `inGroundTime` before `AbstractArrow.tick` increments or
/// resets it. The closed form preserves that order without looping large tick
/// batches.
fn advance_trident_lifecycle(
    projectile: &mut EntityProjectileLifecycle,
    ticks: u32,
    counts_in_ground: bool,
) {
    debug_assert!(ticks > 0);
    let last_checked_in_ground_time = if counts_in_ground {
        projectile
            .trident_in_ground_time
            .saturating_add(ticks.saturating_sub(1))
    } else {
        projectile.trident_in_ground_time
    };
    if last_checked_in_ground_time > 4 {
        projectile.trident_dealt_damage = true;
    }
    projectile.trident_in_ground_time = if counts_in_ground {
        projectile.trident_in_ground_time.saturating_add(ticks)
    } else {
        0
    };
}

fn is_vanilla_hurting_projectile_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_hurting_projectile(entity_type_id)
}

fn is_vanilla_wind_charge_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_wind_charge(entity_type_id)
}

#[cfg(test)]
mod tests {
    use super::super::schema::{
        vanilla_entity_metadata_type_client_movement_policy,
        vanilla_entity_metadata_type_is_projectile, VanillaEntityClientMovementPolicy,
        VANILLA_ENTITY_TYPE_METADATA_26_1,
    };
    use super::*;
    use crate::WorldStore;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_EGG_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, EntityPositionSync as ProtocolEntityPositionSync,
        RemoveEntities as ProtocolRemoveEntities, SetEntityData as ProtocolSetEntityData,
        SetPassengers as ProtocolSetPassengers, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    #[test]
    fn saveable_projectile_membership_combines_class_and_builder_metadata() {
        for entry in VANILLA_ENTITY_TYPES {
            assert_eq!(
                is_vanilla_saved_projectile_type(entry.id),
                vanilla_entity_metadata_type_is_projectile(entry.id) && entry.serializable,
                "entity type {}",
                entry.resource_id
            );
        }
        assert_eq!(
            VANILLA_ENTITY_TYPES
                .iter()
                .filter(|entry| is_vanilla_saved_projectile_type(entry.id))
                .count(),
            18
        );
        assert!(VANILLA_ENTITY_TYPES.iter().any(|entry| {
            vanilla_entity_metadata_type_is_projectile(entry.id) && !entry.serializable
        }));
        assert!(VANILLA_ENTITY_TYPES.iter().any(|entry| {
            !vanilla_entity_metadata_type_is_projectile(entry.id) && entry.serializable
        }));
        assert!(!is_vanilla_saved_projectile_type(-1));
    }

    #[test]
    fn every_projectile_runtime_class_has_an_explicit_client_movement_owner() {
        let mut projectile_count = 0;
        for entry in VANILLA_ENTITY_TYPES {
            if !vanilla_entity_metadata_type_is_projectile(entry.id) {
                continue;
            }
            projectile_count += 1;
            let policy = vanilla_entity_metadata_type_client_movement_policy(entry.id)
                .expect("fixed EntityType has a generated movement policy");
            assert!(
                matches!(
                    policy,
                    VanillaEntityClientMovementPolicy::ThrowableProjectile
                        | VanillaEntityClientMovementPolicy::ThrownExperienceBottle
                        | VanillaEntityClientMovementPolicy::ThrownPotion
                        | VanillaEntityClientMovementPolicy::AbstractArrow
                        | VanillaEntityClientMovementPolicy::HurtingProjectile
                        | VanillaEntityClientMovementPolicy::WindCharge
                        | VanillaEntityClientMovementPolicy::DragonFireball
                        | VanillaEntityClientMovementPolicy::WitherSkull
                        | VanillaEntityClientMovementPolicy::LlamaSpit
                        | VanillaEntityClientMovementPolicy::FireworkRocket
                        | VanillaEntityClientMovementPolicy::FishingHook
                        | VanillaEntityClientMovementPolicy::ShulkerBullet
                ),
                "{} unexpectedly uses {policy:?}",
                entry.resource_id
            );
        }
        assert_eq!(projectile_count, 19);
    }

    #[test]
    fn saveable_projectile_bounds_use_generated_builder_dimensions() {
        for entry in VANILLA_ENTITY_TYPES {
            let expected = is_vanilla_saved_projectile_type(entry.id).then(|| {
                let [width, height] = vanilla_entity_type_dimensions(entry.id).unwrap();
                EntityPickBoundsState::from_base_size(width, height, 0.0)
            });
            assert_eq!(
                vanilla_saved_projectile_bounds(entry.id),
                expected,
                "entity type {}",
                entry.resource_id
            );
        }
        assert_eq!(vanilla_saved_projectile_bounds(-1), None);
    }

    #[test]
    fn hurting_projectile_subclasses_come_from_generated_inheritance() {
        assert_eq!(
            (0..157)
                .filter(|id| is_vanilla_hurting_projectile_type(*id))
                .count(),
            6
        );
        assert_eq!(
            (0..157)
                .filter(|id| is_vanilla_wind_charge_type(*id))
                .count(),
            2
        );
        assert!(is_vanilla_hurting_projectile_type(
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID
        ));
        assert!(!is_vanilla_hurting_projectile_type(
            VANILLA_ENTITY_TYPE_ARROW_ID
        ));
    }

    #[test]
    fn owner_collision_pickability_uses_generated_class_and_tag_membership() {
        let mut store = WorldStore::new();
        let armor_stand_accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        )
        .expect("generated ArmorStand metadata accessors must resolve");
        for (entry, metadata) in VANILLA_ENTITY_TYPES
            .iter()
            .zip(VANILLA_ENTITY_TYPE_METADATA_26_1)
        {
            let id = 1_000 + entry.id;
            store.apply_add_entity(protocol_add_entity_with_type(id, entry.id));
            assert_eq!(
                store
                    .entities
                    .entity_is_pickable_for_owner_collision(id, &HashSet::new()),
                metadata.is_pickable_by_class || entry.redirectable_projectile,
                "entity type {}",
                entry.resource_id
            );
        }

        let player_id = 1_000 + VANILLA_ENTITY_TYPE_PLAYER_ID;
        let armor_stand_id = 1_000 + VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;
        let mut spectator_players = HashSet::new();
        spectator_players.insert(Uuid::from_u128(player_id as u128));
        assert!(!store
            .entities
            .entity_is_pickable_for_owner_collision(player_id, &spectator_players));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: armor_stand_id,
            values: vec![ProtocolEntityDataValue {
                data_id: armor_stand_accessors.client_flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(armor_stand_accessors.marker_mask),
            }],
        }));
        assert!(!store
            .entities
            .entity_is_pickable_for_owner_collision(armor_stand_id, &spectator_players));
    }

    #[test]
    fn applies_projectile_power_to_hurting_projectiles_only() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
        ));
        store.apply_add_entity(protocol_add_entity_with_type(20, 7));

        assert_eq!(
            store.hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: DEFAULT_HURTING_PROJECTILE_ACCELERATION_POWER,
            })
        );
        assert_eq!(store.hurting_projectile(20), None);

        assert!(store.apply_projectile_power(ProtocolProjectilePower {
            entity_id: 10,
            acceleration_power: 0.75,
        }));
        assert_eq!(
            store.hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );
        assert_eq!(
            store
                .probe_entity(10)
                .and_then(|entity| entity.hurting_projectile),
            Some(HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );
        assert_eq!(
            store.clone().hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );

        assert!(!store.apply_projectile_power(ProtocolProjectilePower {
            entity_id: 20,
            acceleration_power: 0.25,
        }));
        assert!(!store.apply_projectile_power(ProtocolProjectilePower {
            entity_id: 404,
            acceleration_power: 0.5,
        }));

        let counters = store.counters();
        assert_eq!(counters.projectile_power_packets, 3);
        assert_eq!(counters.projectile_power_updates_applied, 1);
        assert_eq!(counters.projectile_power_updates_ignored, 2);
        assert_eq!(
            store.last_projectile_power_update(),
            Some(&ProjectilePowerUpdateState {
                entity_id: 404,
                acceleration_power: 0.5,
                applied: false,
            })
        );
    }

    #[test]
    fn wind_charge_projectiles_start_without_acceleration() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        ));

        assert_eq!(
            store.hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: DEFAULT_WIND_CHARGE_ACCELERATION_POWER,
            })
        );
    }

    #[test]
    fn projectile_parent_lifecycle_tracks_owner_vehicle_collision_and_stays_sticky() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_COW_ID,
            ProtocolVec3d {
                x: 8.0,
                y: 0.0,
                z: 0.0,
            },
            0,
        ));
        store.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            ProtocolVec3d::default(),
            0,
        ));
        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 11,
            passenger_ids: vec![10],
        }));
        store.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_EGG_ID,
            ProtocolVec3d::default(),
            10,
        ));

        assert_eq!(
            store.probe_entity(12).unwrap().projectile_lifecycle,
            Some(ProjectileLifecycleState {
                owner_uuid: Some(Uuid::from_u128(10)),
                left_owner: false,
                has_been_shot: false,
                trident_in_ground_time: 0,
                trident_dealt_damage: false,
                trident_return_ticks: 0,
                arrow_pickup_allowed: false,
                arrow_no_physics: false,
                arrow_last_block_state_id: None,
                arrow_hit_ground_sound_is_arrow: false,
            })
        );
        store.advance_entity_client_animations(1);
        assert_eq!(
            store.probe_entity(12).unwrap().projectile_lifecycle,
            Some(ProjectileLifecycleState {
                owner_uuid: Some(Uuid::from_u128(10)),
                left_owner: false,
                has_been_shot: true,
                trident_in_ground_time: 0,
                trident_dealt_damage: false,
                trident_return_ticks: 0,
                arrow_pickup_allowed: false,
                arrow_no_physics: false,
                arrow_last_block_state_id: None,
                arrow_hit_ground_sound_is_arrow: false,
            })
        );

        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![10],
            }),
            1
        );
        let mut replacement_owner =
            protocol_add_entity(14, VANILLA_ENTITY_TYPE_COW_ID, ProtocolVec3d::default(), 0);
        replacement_owner.uuid = Uuid::from_u128(10);
        store.apply_add_entity(replacement_owner);
        store.advance_entity_client_animations(1);
        assert!(
            !store
                .probe_entity(12)
                .unwrap()
                .projectile_lifecycle
                .unwrap()
                .left_owner
        );

        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 12,
                position: ProtocolVec3d {
                    x: 20.0,
                    y: 0.0,
                    z: 0.0,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: false,
            })
        );
        store.advance_entity_client_animations(1);
        assert!(
            store
                .probe_entity(12)
                .unwrap()
                .projectile_lifecycle
                .unwrap()
                .left_owner
        );

        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 14,
                position: ProtocolVec3d {
                    x: 20.0,
                    y: 0.0,
                    z: 0.0,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: false,
            })
        );
        store.advance_entity_client_animations(1);
        assert_eq!(
            store.clone().probe_entity(12).unwrap().projectile_lifecycle,
            Some(ProjectileLifecycleState {
                owner_uuid: Some(Uuid::from_u128(10)),
                left_owner: true,
                has_been_shot: true,
                trident_in_ground_time: 0,
                trident_dealt_damage: false,
                trident_return_ticks: 0,
                arrow_pickup_allowed: false,
                arrow_no_physics: false,
                arrow_last_block_state_id: None,
                arrow_hit_ground_sound_is_arrow: false,
            })
        );
    }

    #[test]
    fn projectile_without_spawn_resolved_owner_leaves_on_first_tick() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(
            20,
            VANILLA_ENTITY_TYPE_EGG_ID,
            ProtocolVec3d::default(),
            404,
        ));
        store.apply_add_entity(protocol_add_entity(
            404,
            VANILLA_ENTITY_TYPE_COW_ID,
            ProtocolVec3d::default(),
            0,
        ));

        assert_eq!(
            store.probe_entity(20).unwrap().projectile_lifecycle,
            Some(ProjectileLifecycleState {
                owner_uuid: None,
                left_owner: false,
                has_been_shot: false,
                trident_in_ground_time: 0,
                trident_dealt_damage: false,
                trident_return_ticks: 0,
                arrow_pickup_allowed: false,
                arrow_no_physics: false,
                arrow_last_block_state_id: None,
                arrow_hit_ground_sound_is_arrow: false,
            })
        );
        store.advance_entity_client_animations(1);
        assert_eq!(
            store.probe_entity(20).unwrap().projectile_lifecycle,
            Some(ProjectileLifecycleState {
                owner_uuid: None,
                left_owner: true,
                has_been_shot: true,
                trident_in_ground_time: 0,
                trident_dealt_damage: false,
                trident_return_ticks: 0,
                arrow_pickup_allowed: false,
                arrow_no_physics: false,
                arrow_last_block_state_id: None,
                arrow_hit_ground_sound_is_arrow: false,
            })
        );
    }

    #[test]
    fn arrow_pickup_allowed_is_frozen_from_the_spawn_resolved_player_owner() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(
            80,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
            0,
        ));
        store.apply_add_entity(protocol_add_entity(
            81,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            ProtocolVec3d::default(),
            80,
        ));
        assert!(store
            .probe_entity(81)
            .and_then(|entity| entity.projectile_lifecycle)
            .is_some_and(|projectile| projectile.arrow_pickup_allowed));

        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![80],
            }),
            1
        );
        assert!(store
            .probe_entity(81)
            .and_then(|entity| entity.projectile_lifecycle)
            .is_some_and(|projectile| projectile.arrow_pickup_allowed));
    }

    #[test]
    fn trident_dealt_damage_uses_previous_in_ground_time_and_physics_flag() {
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .expect("generated ThrownTrident inherited metadata accessors");
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            30,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        ));
        store.apply_add_entity(protocol_add_entity_with_type(
            31,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        ));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 30,
            values: vec![ProtocolEntityDataValue {
                data_id: accessors.in_ground_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 31,
            values: vec![
                ProtocolEntityDataValue {
                    data_id: accessors.flags_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(
                        accessors.flags_default | accessors.no_physics_mask,
                    ),
                },
                ProtocolEntityDataValue {
                    data_id: accessors.in_ground_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
            ],
        }));

        store.advance_entity_client_animations(5);
        let grounded = store
            .probe_entity(30)
            .unwrap()
            .projectile_lifecycle
            .unwrap();
        assert_eq!(grounded.trident_in_ground_time, 5);
        assert!(!grounded.trident_dealt_damage);
        let no_physics = store
            .probe_entity(31)
            .unwrap()
            .projectile_lifecycle
            .unwrap();
        assert_eq!(no_physics.trident_in_ground_time, 0);
        assert!(!no_physics.trident_dealt_damage);

        // ThrownTrident checks the old value (5) before AbstractArrow observes
        // this metadata transition and resets the counter.
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 30,
            values: vec![ProtocolEntityDataValue {
                data_id: accessors.in_ground_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(false),
            }],
        }));
        store.advance_entity_client_animations(1);
        let released = store
            .probe_entity(30)
            .unwrap()
            .projectile_lifecycle
            .unwrap();
        assert_eq!(released.trident_in_ground_time, 0);
        assert!(released.trident_dealt_damage);
    }

    #[test]
    fn uuid_index_tracks_readd_remove_reuse_and_publication_clone() {
        let mut store = WorldStore::new();
        let original_uuid = Uuid::from_u128(10);
        let replacement_uuid = Uuid::from_u128(11);
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_COW_ID,
        ));
        assert_eq!(store.entities.protocol_id_for_uuid(original_uuid), Some(10));

        let mut replacement = protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_COW_ID);
        replacement.uuid = replacement_uuid;
        store.apply_add_entity(replacement);
        assert_eq!(store.entities.protocol_id_for_uuid(original_uuid), None);
        assert_eq!(
            store.entities.protocol_id_for_uuid(replacement_uuid),
            Some(10)
        );

        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![10],
            }),
            1
        );
        assert_eq!(store.entities.protocol_id_for_uuid(replacement_uuid), None);

        let mut reused = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID);
        reused.uuid = original_uuid;
        store.apply_add_entity(reused);
        assert_eq!(store.entities.protocol_id_for_uuid(original_uuid), Some(20));
        assert_eq!(
            store
                .publication_snapshot()
                .entities
                .protocol_id_for_uuid(original_uuid),
            Some(20)
        );
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
        protocol_add_entity(id, entity_type_id, ProtocolVec3d::default(), 0)
    }

    fn protocol_add_entity(
        id: i32,
        entity_type_id: i32,
        position: ProtocolVec3d,
        data: i32,
    ) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position,
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data,
        }
    }
}
