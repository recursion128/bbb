//! Passenger transitions and stored-entity transfer between served levels.

use crate::world::LevelId;
use bbb_server_types::save::Java26LevelKey;
use std::collections::BTreeSet;

use crate::chunk_sender::ChunkKey;
use crate::entity::{
    transfer_stored_entity_subtree_cross_level_with_context,
    transfer_stored_entity_tree_cross_level_with_context, CrossLevelEntityTreeError,
    CrossLevelEntityTreeOutcome, EntityStore, StoredEntityTransition,
};
use bbb_protocol::packets::Vec3d;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;

#[cfg(test)]
use super::block_position;
use super::{ServerLevel, ServerWorld, WorldEvent};

const RELATIVE_X: i32 = 1 << 0;
const RELATIVE_Y: i32 = 1 << 1;
const RELATIVE_Z: i32 = 1 << 2;
const RELATIVE_MASK: i32 = (1 << 9) - 1;

fn absolute_component(current: f64, change: f64, relatives_mask: i32, bit: i32) -> f64 {
    if relatives_mask & bit != 0 {
        current + change
    } else {
        change
    }
}

fn absolute_position(entity: &crate::entity::Entity, transition: StoredEntityTransition) -> Vec3d {
    Vec3d {
        x: absolute_component(
            entity.position().x,
            transition.position.x,
            transition.relatives_mask,
            RELATIVE_X,
        ),
        y: absolute_component(
            entity.position().y,
            transition.position.y,
            transition.relatives_mask,
            RELATIVE_Y,
        ),
        z: absolute_component(
            entity.position().z,
            transition.position.z,
            transition.relatives_mask,
            RELATIVE_Z,
        ),
    }
}

fn passenger_transition(
    transition: StoredEntityTransition,
    vehicle: &crate::entity::Entity,
    passenger: &crate::entity::Entity,
) -> StoredEntityTransition {
    let offset = Vec3d {
        x: passenger.position().x - vehicle.position().x,
        y: passenger.position().y - vehicle.position().y,
        z: passenger.position().z - vehicle.position().z,
    };
    StoredEntityTransition {
        position: Vec3d {
            x: transition.position.x
                + if transition.relatives_mask & RELATIVE_X != 0 {
                    0.0
                } else {
                    offset.x
                },
            y: transition.position.y
                + if transition.relatives_mask & RELATIVE_Y != 0 {
                    0.0
                } else {
                    offset.y
                },
            z: transition.position.z
                + if transition.relatives_mask & RELATIVE_Z != 0 {
                    0.0
                } else {
                    offset.z
                },
        },
        delta_movement: transition.delta_movement,
        y_rot: transition.y_rot
            + if transition.relatives_mask & (1 << 3) != 0 {
                0.0
            } else {
                passenger.y_rot() - vehicle.y_rot()
            },
        x_rot: transition.x_rot
            + if transition.relatives_mask & (1 << 4) != 0 {
                0.0
            } else {
                passenger.x_rot() - vehicle.x_rot()
            },
        relatives_mask: transition.relatives_mask,
    }
}

fn finite_vec3(value: Vec3d) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn valid_transition(transition: StoredEntityTransition) -> bool {
    transition.relatives_mask & !RELATIVE_MASK == 0
        && finite_vec3(transition.position)
        && finite_vec3(transition.delta_movement)
        && transition.y_rot.is_finite()
        && transition.x_rot.is_finite()
}

type ResolvedTransferTree = Vec<(i32, Vec3d, StoredEntityTransition)>;

pub(super) fn resolved_tree(
    store: &EntityStore,
    root_id: i32,
    root_transition: StoredEntityTransition,
    allow_passenger_root: bool,
) -> Result<Option<ResolvedTransferTree>, CrossLevelEntityTreeError> {
    if !valid_transition(root_transition) {
        return Err(CrossLevelEntityTreeError::InvalidTransition);
    }
    if !allow_passenger_root && store.vehicle(root_id).is_some() {
        return Ok(None);
    }
    let body = |id| {
        store
            .get(id)
            .or_else(|| store.player(id).map(super::PlayerState::body))
    };
    let mut stack = vec![(root_id, root_transition, false)];
    let mut seen = BTreeSet::new();
    let mut resolved = Vec::new();
    while let Some((id, transition, visited)) = stack.pop() {
        let Some(entity) = body(id) else {
            return Ok(None);
        };
        if visited {
            resolved.push((id, absolute_position(entity, transition), transition));
            continue;
        }
        if !seen.insert(id) {
            return Ok(None);
        }
        let position = absolute_position(entity, transition);
        if !finite_vec3(position) {
            return Err(CrossLevelEntityTreeError::InvalidTransition);
        }
        stack.push((id, transition, true));
        for &passenger_id in store.passengers(id).iter().rev() {
            let Some(passenger) = body(passenger_id) else {
                return Ok(None);
            };
            if store.vehicle(passenger_id) != Some(id) {
                return Ok(None);
            }
            stack.push((
                passenger_id,
                passenger_transition(transition, entity, passenger),
                false,
            ));
        }
    }
    Ok(Some(resolved))
}

fn source_entity_chunks(
    source: &ServerLevel,
    root_id: i32,
    transition: StoredEntityTransition,
    allow_passenger_root: bool,
) -> Result<BTreeSet<ChunkKey>, CrossLevelEntityTreeError> {
    let resolved = resolved_tree(&source.entities, root_id, transition, allow_passenger_root)?;
    Ok(resolved
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|(id, _, _)| source.entities.get(*id))
        .map(|entity| {
            let position = entity.block_position();
            (position.x >> 4, position.z >> 4)
        })
        .collect())
}

impl ServerWorld {
    // Entity.java:2985-3000.
    pub(super) fn teleport_stored_tree_same_level(
        &mut self,
        dimension: &LevelId,
        root_id: i32,
        transition: StoredEntityTransition,
    ) -> Result<bool, CrossLevelEntityTreeError> {
        let level = self.level_mut_by_id(dimension);
        let Some(tree) = resolved_tree(&level.entities, root_id, transition, true)? else {
            return Ok(false);
        };
        if tree.iter().any(|(id, _, _)| {
            level
                .entities
                .get(*id)
                .is_none_or(|entity| entity.removal_reason().is_some())
        }) {
            return Ok(false);
        }
        for (id, _, _) in tree {
            let entity = level
                .entities
                .get_mut(id)
                .expect("the resolved tree remains present");
            let old = entity.block_position();
            level.saved_entity_chunks.insert((old.x >> 4, old.z >> 4));
            crate::entity::tnt::mark_used_portal_after_teleport(entity);
            super::merchant_trading::stop_trading(entity);
        }
        Ok(level.entities.teleport_stored_entity(
            root_id,
            bbb_protocol::packets::movement::PlayerPositionState {
                position: transition.position,
                delta_movement: transition.delta_movement,
                y_rot: transition.y_rot,
                x_rot: transition.x_rot,
            },
            transition.relatives_mask,
        ))
    }

    fn transfer_stored_tree_between_levels(
        &mut self,
        source_key: LevelId,
        target_key: LevelId,
        root_id: i32,
        transition: StoredEntityTransition,
        source_chunks: BTreeSet<ChunkKey>,
        detach_requesting_passenger: bool,
    ) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
        let camera_transitions =
            self.stored_camera_transitions(&source_key, &target_key, root_id, transition);
        let source_entities = &mut self.level_mut_by_id(&source_key).entities;
        for (id, _, _) in resolved_tree(source_entities, root_id, transition, true)
            .ok()
            .flatten()
            .unwrap_or_default()
        {
            if let Some(entity) = source_entities.get_mut(id) {
                super::merchant_trading::stop_trading(entity);
            }
        }
        if self.cold.config.datapacks.is_some() {
            if let Err(error) = self.materialize_merchant_offers_in_tree(&source_key, root_id) {
                tracing::warn!(%error, "could not materialize merchant offers before transfer");
            }
        }
        let input = self.entity_nbt_read_snapshot();
        let (source, target) = self.levels.pair_mut(source_key, target_key);
        let result = if detach_requesting_passenger {
            transfer_stored_entity_subtree_cross_level_with_context(
                &mut source.entities,
                &mut target.entities,
                root_id,
                transition,
                input.context(),
            )
        } else {
            transfer_stored_entity_tree_cross_level_with_context(
                &mut source.entities,
                &mut target.entities,
                root_id,
                transition,
                input.context(),
            )
        };
        if let Ok(ref outcome) = result {
            crate::world::raids::resolve_raider_raids_in_tree_parts(
                &mut target.entities,
                &mut target.raids,
                outcome.new_root_id,
            );
        }
        source.saved_entity_chunks.extend(source_chunks);
        self.refresh_cross_level_entities_across(&source_key, &target_key);
        for camera_transition in camera_transitions {
            let crate::commands::CommandCameraTarget::Stored { uuid, .. } = &camera_transition.1
            else {
                continue;
            };
            if result.is_ok()
                || (!self.level_by_id(&source_key).entities.contains_uuid(*uuid)
                    && self.level_by_id(&target_key).entities.contains_uuid(*uuid))
            {
                self.pending_camera_transitions.push(camera_transition);
            }
        }
        result
    }

    fn stored_camera_transitions(
        &self,
        source: &LevelId,
        target: &LevelId,
        root_id: i32,
        transition: StoredEntityTransition,
    ) -> Vec<(
        LevelId,
        crate::commands::CommandCameraTarget,
        super::TeleportTransition<LevelId>,
    )> {
        let store = &self.level_by_id(source).entities;
        let mut transitions = Vec::new();
        for (id, _, transition) in resolved_tree(store, root_id, transition, true)
            .ok()
            .flatten()
            .unwrap_or_default()
        {
            let Some(entity) = store.get(id) else {
                continue;
            };
            let identity = crate::commands::CommandCameraTarget::Stored {
                dimension: self.level_key(*source).clone(),
                entity_id: id,
                uuid: entity.uuid(),
            };
            if self.levels.players().any(|player| {
                &player.dimension == source && player.cold.camera.as_ref() == Some(&identity)
            }) {
                transitions.push((
                    *source,
                    identity,
                    super::TeleportTransition {
                        new_level: *target,
                        position: transition.position,
                        y_rot: transition.y_rot,
                        x_rot: transition.x_rot,
                        relatives_mask: transition.relatives_mask,
                    },
                ));
            }
        }
        transitions
    }

    /// `Entity.teleportCrossDimension` recursively derives each passenger's
    /// transition before creating replacements (`Entity.java:3002-3035,3047-3057`).
    pub fn transfer_stored_entity_tree_cross_level(
        &mut self,
        source_dimension: impl Into<Java26LevelKey>,
        target_dimension: impl Into<Java26LevelKey>,
        root_id: i32,
        transition: StoredEntityTransition,
    ) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
        let source_key = source_dimension.into();
        let target_key = target_dimension.into();
        if source_key == target_key {
            return Err(CrossLevelEntityTreeError::SameLevel);
        }
        let source = self
            .level_id(&source_key)
            .ok_or(CrossLevelEntityTreeError::UnservedLevel)?;
        let target = self
            .level_id(&target_key)
            .ok_or(CrossLevelEntityTreeError::UnservedLevel)?;
        self.transfer_stored_entity_tree_cross_level_id(source, target, root_id, transition)
    }

    fn transfer_stored_entity_tree_cross_level_id(
        &mut self,
        source_key: LevelId,
        target_key: LevelId,
        root_id: i32,
        transition: StoredEntityTransition,
    ) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
        if source_key == target_key {
            return Err(CrossLevelEntityTreeError::SameLevel);
        }
        let Some(source) = self.levels.get(&source_key) else {
            return Err(CrossLevelEntityTreeError::UnservedLevel);
        };
        if !self.levels.contains_key(&target_key) {
            return Err(CrossLevelEntityTreeError::UnservedLevel);
        }
        let source_chunks = source_entity_chunks(source, root_id, transition, false)?;
        self.transfer_stored_tree_between_levels(
            source_key,
            target_key,
            root_id,
            transition,
            source_chunks,
            false,
        )
    }

    pub(super) fn teleport_entity_subtree(
        &mut self,
        source: LevelId,
        target: LevelId,
        root_id: i32,
        transition: StoredEntityTransition,
        portal_sound: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Result<Option<i32>, CrossLevelEntityTreeError> {
        let source_level = self
            .levels
            .get(&source)
            .ok_or(CrossLevelEntityTreeError::UnservedLevel)?;
        if !self.levels.contains_key(&target) {
            return Err(CrossLevelEntityTreeError::UnservedLevel);
        }
        let Some(tree) = resolved_tree(&source_level.entities, root_id, transition, true)? else {
            return Ok(None);
        };
        let riders = tree
            .iter()
            .filter_map(|(id, _, transition)| {
                let player = source_level.entities.player(*id)?;
                let vehicle = source_level.entities.vehicle(*id)?;
                let seat = source_level
                    .entities
                    .passengers(vehicle)
                    .iter()
                    .position(|other| other == id)?;
                let controls_root = self.external_player_controlled_vehicle(player.connection_id())
                    == Some(root_id);
                Some((
                    player.connection_id(),
                    vehicle,
                    seat,
                    *transition,
                    controls_root,
                ))
            })
            .collect::<Vec<_>>();
        for (connection, _, _, _, _) in &riders {
            let player_id = self.levels[*connection].entity_id();
            self.level_mut_by_id(&source)
                .entities
                .detach_external_player(player_id);
        }
        let outcome =
            if source == target {
                if !self.teleport_stored_tree_same_level(&source, root_id, transition)? {
                    return Ok(None);
                }
                None
            } else {
                Some(self.transfer_stored_entity_subtree_cross_level(
                    source, target, root_id, transition,
                )?)
            };
        for (connection, vehicle, seat, rider_transition, controls_root) in riders {
            if !self.teleport_player_id(
                connection,
                super::TeleportTransition {
                    new_level: target,
                    position: rider_transition.position,
                    y_rot: rider_transition.y_rot,
                    x_rot: rider_transition.x_rot,
                    relatives_mask: rider_transition.relatives_mask,
                },
                events,
            ) {
                continue;
            }
            // Entity.java:3060-3072: the controller applies the relative root transition.
            if source == target {
                let root = self
                    .level_by_id(&target)
                    .entities
                    .get(root_id)
                    .expect("the teleported vehicle remains present");
                let change = if controls_root {
                    transition
                } else {
                    StoredEntityTransition {
                        position: root.position(),
                        delta_movement: root.delta_movement(),
                        y_rot: root.y_rot(),
                        x_rot: root.x_rot(),
                        relatives_mask: 0,
                    }
                };
                events.push(super::send(
                    connection,
                    crate::connection::ClientboundPlayPacket::TeleportEntity(
                        bbb_protocol::packets::TeleportEntity {
                            id: root_id,
                            position: change.position,
                            delta_movement: change.delta_movement,
                            y_rot: change.y_rot,
                            x_rot: change.x_rot,
                            relatives_mask: change.relatives_mask,
                            on_ground: root.on_ground(),
                        },
                    ),
                ));
            }
            if portal_sound {
                self.send_portal_travel_sound(connection, events);
            }
            let vehicle_id = match &outcome {
                Some(outcome) => outcome.id_map.get(&vehicle).copied(),
                None => Some(vehicle),
            };
            if let Some(vehicle_id) = vehicle_id {
                let player_id = self.levels[connection].entity_id();
                let attached = {
                    let entities = &mut self.level_mut_by_id(&target).entities;
                    let clamped_seat = seat.min(entities.passengers(vehicle_id).len());
                    entities.attach_external_player_forced(player_id, vehicle_id, clamped_seat)
                };
                if attached {
                    self.send_mounted_vehicle_state(connection, &target, vehicle_id, events);
                }
            }
        }
        Ok(Some(outcome.map_or(root_id, |outcome| outcome.new_root_id)))
    }

    fn transfer_stored_entity_subtree_cross_level(
        &mut self,
        source_key: LevelId,
        target_key: LevelId,
        root_id: i32,
        transition: StoredEntityTransition,
    ) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
        if source_key == target_key {
            return Err(CrossLevelEntityTreeError::SameLevel);
        }
        let Some(source) = self.levels.get(&source_key) else {
            return Err(CrossLevelEntityTreeError::UnservedLevel);
        };
        if !self.levels.contains_key(&target_key) {
            return Err(CrossLevelEntityTreeError::UnservedLevel);
        }
        let source_chunks = source_entity_chunks(source, root_id, transition, true)?;
        self.transfer_stored_tree_between_levels(
            source_key,
            target_key,
            root_id,
            transition,
            source_chunks,
            true,
        )
    }

    #[cfg(test)]
    pub(crate) fn ensure_entity_transfer_full_chunk_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        chunk: ChunkKey,
    ) {
        self.ensure_entity_transfer_full_chunk_for_test_by_key(&dimension.into(), chunk);
    }

    #[cfg(test)]
    pub(crate) fn ensure_entity_transfer_full_chunk_for_test_by_key(
        &mut self,
        dimension: &LevelId,
        chunk: ChunkKey,
    ) {
        self.level_mut_by_id(dimension)
            .chunks
            .try_ensure_served_full_column(chunk.0, chunk.1)
            .expect("the fixture source produces FULL synchronously");
    }

    #[cfg(test)]
    pub(crate) fn saved_entity_chunks_for_transfer_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> BTreeSet<ChunkKey> {
        self.level(dimension).saved_entity_chunks.clone()
    }

    #[cfg(test)]
    pub(crate) fn move_stored_entity_for_transfer_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        position: Vec3d,
        y_rot: f32,
        x_rot: f32,
    ) {
        let store = &mut self.level_mut(dimension).entities;
        let mut entity = store.detach(entity_id).expect("the test entity is stored");
        entity.set_pos(position);
        entity.set_rot(y_rot, x_rot);
        store.attach(entity);
    }

    #[cfg(test)]
    pub(crate) fn spawn_stored_entity_transfer_tree_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        root_position: Vec3d,
        passenger_position: Vec3d,
    ) -> Option<(i32, i32)> {
        use crate::entity::{AtomicSpawnPlan, SpawnEntityTree};
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        };
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
        use bbb_server_types::geometry::Aabb;

        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        let tree = SpawnEntityTree {
            entity_type_id: VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            resource_id: "minecraft:oak_chest_boat".to_string(),
            entity_nbt: vec![
                entry("id", NbtSummaryValue::string("minecraft:oak_chest_boat")),
                entry(
                    "Items",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("Slot", NbtSummaryValue::Byte(26)),
                        entry("id", NbtSummaryValue::string("minecraft:diamond")),
                        entry("count", NbtSummaryValue::Int(7)),
                    ])]),
                ),
            ],
            passengers: vec![SpawnEntityTree {
                entity_type_id: VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                resource_id: "minecraft:armor_stand".to_string(),
                entity_nbt: vec![entry(
                    "id",
                    NbtSummaryValue::string("minecraft:armor_stand"),
                )],
                passengers: Vec::new(),
            }],
        };
        let block = block_position(root_position);
        let plan = AtomicSpawnPlan {
            tree,
            position: root_position,
            spawn_block_position: block,
            spawner_position: block,
            collision_box: Aabb::of_entity(root_position, 1.375, 0.5625),
            custom_spawn_rules: None,
            placement_probe: None,
            finalize_root_mob: false,
            equipment: None,
            nearby_box: Aabb::of_entity(root_position, 1.375, 0.5625),
            max_nearby_entities: 6,
        };
        let store = &mut self.level_mut(dimension).entities;
        let root_id = crate::entity::spawn_entity_tree(store, &plan)?;
        let passenger_id = *store.passengers(root_id).first()?;
        let mut passenger = store.detach(passenger_id)?;
        passenger.set_pos(passenger_position);
        store.attach(passenger);
        Some((root_id, passenger_id))
    }

    #[cfg(test)]
    pub(crate) fn spawn_stored_throwable_passengers_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: Vec3d,
        requester_count: usize,
        include_living_sibling: bool,
    ) -> Option<(i32, Vec<i32>, Option<i32>)> {
        use crate::entity::{AtomicSpawnPlan, SpawnEntityTree};
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        };
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
        use bbb_server_types::geometry::Aabb;

        let entity_nbt = |resource_id: &str| {
            vec![NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(resource_id),
            }]
        };
        let snowball_item = crate::entity::item::plain_stack("minecraft:snowball", 1)?
            .write(&bbb_registry_codec::UnavailableHolderResolver)
            .ok()??;
        let snowball = || SpawnEntityTree {
            entity_type_id: VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            resource_id: "minecraft:snowball".to_string(),
            entity_nbt: vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::string("minecraft:snowball"),
                },
                NbtSummaryEntry {
                    name: "Item".into(),
                    value: snowball_item.clone(),
                },
            ],
            passengers: Vec::new(),
        };
        let mut passengers = (0..requester_count).map(|_| snowball()).collect::<Vec<_>>();
        if include_living_sibling {
            passengers.push(SpawnEntityTree {
                entity_type_id: VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                resource_id: "minecraft:armor_stand".to_string(),
                entity_nbt: entity_nbt("minecraft:armor_stand"),
                passengers: Vec::new(),
            });
        }
        let tree = SpawnEntityTree {
            entity_type_id: VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            resource_id: "minecraft:armor_stand".to_string(),
            entity_nbt: entity_nbt("minecraft:armor_stand"),
            passengers,
        };
        let block = block_position(position);
        let plan = AtomicSpawnPlan {
            tree,
            position,
            spawn_block_position: block,
            spawner_position: block,
            collision_box: Aabb::of_entity(position, 0.5, 1.975),
            custom_spawn_rules: None,
            placement_probe: None,
            finalize_root_mob: false,
            equipment: None,
            nearby_box: Aabb::of_entity(position, 0.5, 1.975),
            max_nearby_entities: 6,
        };
        let store = &mut self.level_mut(dimension).entities;
        let vehicle_id = crate::entity::spawn_entity_tree(store, &plan)?;
        let passenger_ids = store.passengers(vehicle_id);
        let requester_ids = passenger_ids.get(..requester_count)?.to_vec();
        let sibling_id = include_living_sibling
            .then(|| passenger_ids.get(requester_count).copied())
            .flatten();
        Some((vehicle_id, requester_ids, sibling_id))
    }
}
