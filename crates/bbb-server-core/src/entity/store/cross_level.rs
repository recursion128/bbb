use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::{mth_cos, mth_sin};
use thiserror::Error;
use uuid::Uuid;

use super::EntityStore;
use crate::entity::save::{
    entity_transfer_record, load_entity_transfer_record, EntityNbtReadContext,
};
use crate::entity::{Entity, RemovalReason};

const RELATIVE_X: i32 = 1 << 0;
const RELATIVE_Y: i32 = 1 << 1;
const RELATIVE_Z: i32 = 1 << 2;
const RELATIVE_Y_ROT: i32 = 1 << 3;
const RELATIVE_X_ROT: i32 = 1 << 4;
const RELATIVE_DELTA_X: i32 = 1 << 5;
const RELATIVE_DELTA_Y: i32 = 1 << 6;
const RELATIVE_DELTA_Z: i32 = 1 << 7;
const RELATIVE_ROTATE_DELTA: i32 = 1 << 8;
const RELATIVE_MASK: i32 = (1 << 9) - 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoredEntityTransition {
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub relatives_mask: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossLevelEntityTreeOutcome {
    pub old_root_id: i32,
    pub new_root_id: i32,
    pub id_map: BTreeMap<i32, i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CrossLevelEntityTreeError {
    #[error("cross-level entity transition is invalid")]
    InvalidTransition,
    #[error("stored root entity {0} is missing")]
    MissingRoot(i32),
    #[error("stored root entity {0} is already a passenger")]
    RootIsPassenger(i32),
    #[error("stored passenger tree is malformed at entity {0}")]
    MalformedPassengerTree(i32),
    #[error("stored passenger tree contains external player {0}")]
    ExternalPlayerPassenger(i32),
    #[error("stored entity {entity_id} runtime type {resource_id} is unavailable")]
    UnsupportedEntity { entity_id: i32, resource_id: String },
    #[error("stored entity {entity_id} cannot pass the canonical codec: {detail}")]
    Codec { entity_id: i32, detail: String },
    #[error("target level already contains entity UUID {0}")]
    TargetUuidConflict(Uuid),
    #[error("source and target resolve to the same level")]
    SameLevel,
    #[error("source or target level is not served")]
    UnservedLevel,
}

#[derive(Clone, Copy)]
struct AbsoluteTransition {
    position: Vec3d,
    delta_movement: Vec3d,
    y_rot: f32,
    x_rot: f32,
}

pub fn transfer_stored_entity_tree_cross_level(
    source: &mut EntityStore,
    target: &mut EntityStore,
    root_id: i32,
    transition: StoredEntityTransition,
) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
    transfer_stored_entity_tree_cross_level_inner(source, target, root_id, transition, false, None)
}

/// Portal transfer after `Entity.teleport` stops a requesting passenger from
/// riding (`Entity.java:2971-2979`). Only that entity and its descendants move.
pub fn transfer_stored_entity_subtree_cross_level(
    source: &mut EntityStore,
    target: &mut EntityStore,
    root_id: i32,
    transition: StoredEntityTransition,
) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
    transfer_stored_entity_tree_cross_level_inner(source, target, root_id, transition, true, None)
}

pub fn transfer_stored_entity_tree_cross_level_with_context(
    source: &mut EntityStore,
    target: &mut EntityStore,
    root_id: i32,
    transition: StoredEntityTransition,
    context: EntityNbtReadContext<'_>,
) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
    transfer_stored_entity_tree_cross_level_inner(
        source,
        target,
        root_id,
        transition,
        false,
        Some(context),
    )
}

pub fn transfer_stored_entity_subtree_cross_level_with_context(
    source: &mut EntityStore,
    target: &mut EntityStore,
    root_id: i32,
    transition: StoredEntityTransition,
    context: EntityNbtReadContext<'_>,
) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
    transfer_stored_entity_tree_cross_level_inner(
        source,
        target,
        root_id,
        transition,
        true,
        Some(context),
    )
}

fn transfer_stored_entity_tree_cross_level_inner(
    source: &mut EntityStore,
    target: &mut EntityStore,
    root_id: i32,
    transition: StoredEntityTransition,
    detach_requesting_passenger: bool,
    context: Option<EntityNbtReadContext<'_>>,
) -> Result<CrossLevelEntityTreeOutcome, CrossLevelEntityTreeError> {
    validate_transition(transition)?;
    if source.ids != target.ids {
        return Err(CrossLevelEntityTreeError::InvalidTransition);
    }
    source
        .get(root_id)
        .ok_or(CrossLevelEntityTreeError::MissingRoot(root_id))?;
    let root_is_passenger = source.vehicle(root_id).is_some();
    if root_is_passenger && !detach_requesting_passenger {
        return Err(CrossLevelEntityTreeError::RootIsPassenger(root_id));
    }

    let postorder = collect_postorder(source, root_id, transition)?
        .into_iter()
        .map(|(id, absolute)| (id, absolute, source.passengers(id).to_vec()))
        .collect::<Vec<_>>();
    if root_is_passenger && !source.stop_invalid_riding(root_id) {
        return Err(CrossLevelEntityTreeError::RootIsPassenger(root_id));
    }
    let mut id_map = BTreeMap::new();
    for (old_id, absolute, passengers) in postorder {
        let original = source
            .get(old_id)
            .ok_or(CrossLevelEntityTreeError::MalformedPassengerTree(old_id))?;
        let portal_process = original.portal_process().copied();
        let record = entity_transfer_record(source, original)
            .map_err(|error| map_save_error(old_id, error))?;
        let mut entity =
            load_entity_transfer_record(target, record.as_ref(), context).map_err(|error| {
                match error {
                    crate::entity::save::EntitySaveError::DuplicateUuid(uuid) => {
                        CrossLevelEntityTreeError::TargetUuidConflict(uuid)
                    }
                    error => map_save_error(old_id, error),
                }
            })?;
        entity.restore_portal_process(portal_process);
        crate::entity::tnt::mark_used_portal_after_teleport(&mut entity);
        apply_transition(&mut entity, absolute);
        source.remove_cross_level_entity(old_id);
        let new_id = entity.id();
        target.admit_entity(entity);
        id_map.insert(old_id, new_id);
        for passenger in passengers {
            if let Some(&passenger_id) = id_map.get(&passenger) {
                target.link_existing_passenger_forced(new_id, passenger_id);
            }
        }
    }

    let new_root_id = id_map[&root_id];
    for &new_id in id_map.values() {
        let Some(mut entity) = target.detach(new_id) else {
            continue;
        };
        if matches!(entity.data(), crate::entity::EntityData::Item(_)) {
            // `ItemEntity.teleport` merges the target replacement after
            // `super.teleport` returns it (`ItemEntity.java:369-375`).
            crate::entity::item::merge_with_neighbours(&mut entity, target);
        }
        target.attach(entity);
    }
    Ok(CrossLevelEntityTreeOutcome {
        old_root_id: root_id,
        new_root_id,
        id_map,
    })
}

fn map_save_error(
    entity_id: i32,
    error: crate::entity::save::EntitySaveError,
) -> CrossLevelEntityTreeError {
    match error {
        crate::entity::save::EntitySaveError::UnsupportedEntityType(resource_id) => {
            CrossLevelEntityTreeError::UnsupportedEntity {
                entity_id,
                resource_id,
            }
        }
        error => CrossLevelEntityTreeError::Codec {
            entity_id,
            detail: error.to_string(),
        },
    }
}

impl EntityStore {
    fn remove_cross_level_entity(&mut self, entity_id: i32) {
        let Some(mut entity) = self.detach(entity_id) else {
            return;
        };
        entity.set_removed(RemovalReason::ChangedDimension);
        // ChangedDimension's no-drop `removeLeash` runs after tracking stops, so
        // old viewers see no unlink (`Entity.java:3108-3116`;
        // `PersistentEntitySectionManager.java:427-448`; `ChunkMap.java:1214-1218`).
        crate::entity::leash::drop_leash_store_spawn(&mut entity, self, false, false);
        self.attach(entity);
    }
}

fn validate_transition(
    transition: StoredEntityTransition,
) -> Result<(), CrossLevelEntityTreeError> {
    if transition.relatives_mask & !RELATIVE_MASK != 0
        || !finite_vec3(transition.position)
        || !finite_vec3(transition.delta_movement)
        || !transition.y_rot.is_finite()
        || !transition.x_rot.is_finite()
    {
        return Err(CrossLevelEntityTreeError::InvalidTransition);
    }
    Ok(())
}

fn finite_vec3(value: Vec3d) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn collect_postorder(
    store: &EntityStore,
    root_id: i32,
    root_transition: StoredEntityTransition,
) -> Result<Vec<(i32, AbsoluteTransition)>, CrossLevelEntityTreeError> {
    enum Visit {
        Enter(i32, StoredEntityTransition),
        Exit(i32, StoredEntityTransition),
    }

    let mut stack = vec![Visit::Enter(root_id, root_transition)];
    let mut seen = BTreeSet::new();
    let mut postorder = Vec::new();
    while let Some(visit) = stack.pop() {
        match visit {
            Visit::Exit(id, transition) => {
                let entity = store
                    .get(id)
                    .ok_or(CrossLevelEntityTreeError::MalformedPassengerTree(id))?;
                let absolute = calculate_absolute(entity, transition)?;
                postorder.push((id, absolute));
            }
            Visit::Enter(id, transition) => {
                if store.is_external_player_passenger(id) {
                    return Err(CrossLevelEntityTreeError::ExternalPlayerPassenger(id));
                }
                let entity = store
                    .get(id)
                    .ok_or(CrossLevelEntityTreeError::MalformedPassengerTree(id))?;
                if !seen.insert(id) {
                    return Err(CrossLevelEntityTreeError::MalformedPassengerTree(id));
                }
                stack.push(Visit::Exit(id, transition));
                for passenger_id in store.passengers(id).iter().rev() {
                    if store.is_external_player_passenger(*passenger_id) {
                        return Err(CrossLevelEntityTreeError::ExternalPlayerPassenger(
                            *passenger_id,
                        ));
                    }
                    let passenger = store.get(*passenger_id).ok_or(
                        CrossLevelEntityTreeError::MalformedPassengerTree(*passenger_id),
                    )?;
                    if store.vehicle(*passenger_id) != Some(id) {
                        return Err(CrossLevelEntityTreeError::MalformedPassengerTree(
                            *passenger_id,
                        ));
                    }
                    let passenger_transition = passenger_transition(transition, entity, passenger);
                    validate_transition(passenger_transition)?;
                    stack.push(Visit::Enter(*passenger_id, passenger_transition));
                }
            }
        }
    }
    Ok(postorder)
}

fn passenger_transition(
    transition: StoredEntityTransition,
    vehicle: &Entity,
    passenger: &Entity,
) -> StoredEntityTransition {
    let offset = subtract(passenger.position(), vehicle.position());
    StoredEntityTransition {
        position: Vec3d {
            x: transition.position.x
                + if has(transition.relatives_mask, RELATIVE_X) {
                    0.0
                } else {
                    offset.x
                },
            y: transition.position.y
                + if has(transition.relatives_mask, RELATIVE_Y) {
                    0.0
                } else {
                    offset.y
                },
            z: transition.position.z
                + if has(transition.relatives_mask, RELATIVE_Z) {
                    0.0
                } else {
                    offset.z
                },
        },
        delta_movement: transition.delta_movement,
        y_rot: transition.y_rot
            + if has(transition.relatives_mask, RELATIVE_Y_ROT) {
                0.0
            } else {
                passenger.y_rot() - vehicle.y_rot()
            },
        x_rot: transition.x_rot
            + if has(transition.relatives_mask, RELATIVE_X_ROT) {
                0.0
            } else {
                passenger.x_rot() - vehicle.x_rot()
            },
        relatives_mask: transition.relatives_mask,
    }
}

fn apply_transition(entity: &mut Entity, absolute: AbsoluteTransition) {
    // `Entity.java:3002-3035,3047-3057,3075-3089`; `PositionMoveRotation.java:40-67`.
    entity.set_pos(absolute.position);
    entity.set_filed_section(super::section_of_block(entity.block_position()));
    entity.set_rot(absolute.y_rot, absolute.x_rot);
    entity.set_y_head_rot(absolute.y_rot);
    entity.set_old_pos_and_rot();
    entity.set_delta_movement(absolute.delta_movement);
    entity.movement_this_tick.clear();
    entity.final_movements_this_tick.clear();
}

fn validate_absolute(absolute: AbsoluteTransition) -> Result<(), CrossLevelEntityTreeError> {
    if !finite_vec3(absolute.position)
        || !finite_vec3(absolute.delta_movement)
        || !absolute.y_rot.is_finite()
        || !absolute.x_rot.is_finite()
    {
        return Err(CrossLevelEntityTreeError::InvalidTransition);
    }
    Ok(())
}

fn calculate_absolute(
    entity: &Entity,
    transition: StoredEntityTransition,
) -> Result<AbsoluteTransition, CrossLevelEntityTreeError> {
    let y_rot = if has(transition.relatives_mask, RELATIVE_Y_ROT) {
        entity.y_rot() + transition.y_rot
    } else {
        transition.y_rot
    };
    let raw_x_rot = if has(transition.relatives_mask, RELATIVE_X_ROT) {
        entity.x_rot() + transition.x_rot
    } else {
        transition.x_rot
    };
    if !y_rot.is_finite() || !raw_x_rot.is_finite() {
        return Err(CrossLevelEntityTreeError::InvalidTransition);
    }
    let x_rot = if raw_x_rot.is_finite() {
        raw_x_rot.clamp(-90.0, 90.0)
    } else {
        raw_x_rot
    };
    let mut current_delta = entity.delta_movement();
    if has(transition.relatives_mask, RELATIVE_ROTATE_DELTA) {
        let diff_x_rot = entity.x_rot() - x_rot;
        let diff_y_rot = entity.y_rot() - y_rot;
        if !diff_x_rot.is_finite() || !diff_y_rot.is_finite() {
            return Err(CrossLevelEntityTreeError::InvalidTransition);
        }
        current_delta = rotate_x(current_delta, radians(diff_x_rot));
        current_delta = rotate_y(current_delta, radians(diff_y_rot));
    }
    let absolute = AbsoluteTransition {
        position: Vec3d {
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
        },
        delta_movement: Vec3d {
            x: absolute_component(
                current_delta.x,
                transition.delta_movement.x,
                transition.relatives_mask,
                RELATIVE_DELTA_X,
            ),
            y: absolute_component(
                current_delta.y,
                transition.delta_movement.y,
                transition.relatives_mask,
                RELATIVE_DELTA_Y,
            ),
            z: absolute_component(
                current_delta.z,
                transition.delta_movement.z,
                transition.relatives_mask,
                RELATIVE_DELTA_Z,
            ),
        },
        y_rot,
        x_rot,
    };
    validate_absolute(absolute)?;
    Ok(absolute)
}

fn has(mask: i32, bit: i32) -> bool {
    mask & bit != 0
}

fn absolute_component(current: f64, change: f64, mask: i32, bit: i32) -> f64 {
    if has(mask, bit) {
        current + change
    } else {
        change
    }
}

fn subtract(left: Vec3d, right: Vec3d) -> Vec3d {
    Vec3d {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn radians(degrees: f32) -> f32 {
    f64::from(degrees).to_radians() as f32
}

fn rotate_x(value: Vec3d, radians: f32) -> Vec3d {
    let sin = mth_sin(f64::from(radians));
    let cos = mth_cos(f64::from(radians));
    Vec3d {
        x: value.x,
        y: value.y * f64::from(cos) + value.z * f64::from(sin),
        z: value.z * f64::from(cos) - value.y * f64::from(sin),
    }
}

fn rotate_y(value: Vec3d, radians: f32) -> Vec3d {
    let sin = mth_sin(f64::from(radians));
    let cos = mth_cos(f64::from(radians));
    Vec3d {
        x: value.x * f64::from(cos) + value.z * f64::from(sin),
        y: value.y,
        z: value.z * f64::from(cos) - value.x * f64::from(sin),
    }
}
