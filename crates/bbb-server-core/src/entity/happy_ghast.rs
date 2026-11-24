use std::collections::BTreeSet;

use crate::connection::ClientboundPlayPacket;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityPositionSync, SoundSource};

use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::general_living::{set_spawner_brain_memory, spawner_brain_memory};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn is_panicking(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.happy_ghast_state().is_some()
        && data.is_baby()
        && spawner_brain_memory(entity, MemoryModuleType::IsPanicking).is_some_and(|memory| {
            matches!(memory.value, MemoryValue::Bool(_))
                && memory.time_to_live.is_none_or(|ttl| ttl > 0)
        })
}

// HappyGhastAi.java:34-45; AnimalPanic.java:60-77.
pub(crate) fn before_server_ai(entity: &mut Entity, level: &LevelCtx) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(state) = data.happy_ghast_state() else {
        return;
    };
    let baby = data.is_baby();
    if let Some(until) = state.panic_until {
        // Behavior.java:84-85: the endpoint itself still runs the behavior.
        if !baby || level.game_time() > until {
            let EntityData::GeneralLiving(data) = &mut entity.data else {
                unreachable!()
            };
            data.happy_ghast_state_mut().unwrap().panic_until = None;
            data.retained_brain_mut()
                .memories
                .retain(|memory| memory.memory_type != MemoryModuleType::IsPanicking);
        }
        return;
    }
    if !baby {
        return;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.retained_brain_mut().memories.retain(|memory| {
        memory.memory_type != MemoryModuleType::IsPanicking
            || memory.time_to_live.is_none_or(|ttl| ttl > 0)
    });
    if !is_panicking(entity) && !crate::entity::ai::mob::panic::damage_causes_panic(entity, level) {
        return;
    }
    // Behavior.java:45-49; AnimalPanic.java:27-28,69-70.
    let until = level.game_time() + 100 + i64::from(level.level_random().next_int(21));
    set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::IsPanicking,
            value: MemoryValue::Bool(true),
            time_to_live: None,
        },
    );
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.happy_ghast_state_mut().unwrap().panic_until = Some(until);
}

// HappyGhast.java:439-450.
pub(crate) fn update_home(entity: &mut Entity, store: &EntityStore) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if data.happy_ghast_state().is_none()
        || crate::entity::leash::is_leashed(entity)
        || !store.passengers(entity.id()).is_empty()
    {
        return;
    }
    let empty_body = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::Body))
        .is_none_or(|stack| stack.is_empty());
    let radius = if !data.is_baby() && empty_body {
        64
    } else {
        32
    };
    let position = entity.block_position();
    let replace = entity.mob_home().is_none_or(|(home, previous_radius)| {
        let dx = f64::from(home.x) - f64::from(position.x);
        let dy = f64::from(home.y) - f64::from(position.y);
        let dz = f64::from(home.z) - f64::from(position.z);
        previous_radius != radius || dx * dx + dy * dy + dz * dz >= f64::from((radius + 16).pow(2))
    });
    if replace {
        entity.set_mob_home(position, radius);
    }
}

// HappyGhast.java:300-326.
pub(crate) fn on_passenger_transition(
    entity: &mut Entity,
    occupied: bool,
) -> Option<crate::entity::store::PendingEntitySound> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
        return None;
    }
    if !occupied {
        if let Some((position, _)) = entity.mob_home() {
            entity.set_mob_home(position, -1);
        }
    }
    Some(crate::entity::store::PendingEntitySound {
        position: entity.position(),
        resource_id: if occupied {
            "minecraft:entity.happy_ghast.harness_goggles_down"
        } else {
            "minecraft:entity.happy_ghast.harness_goggles_up"
        }
        .to_owned(),
        source: SoundSource::Neutral,
        volume: 1.0,
        pitch: 1.0,
    })
}

pub(crate) fn before_ai_step(entity: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.happy_ghast_state_mut() {
            // HappyGhast.java:430-435: snapshot before movement and the tick suffix.
            state.requires_precise_position = state.stays_still || state.still_timeout > 0;
        }
    }
}

pub(crate) fn after_super(entity: &mut Entity, store: &mut EntityStore) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return true;
    };
    let Some(state) = data.happy_ghast_state() else {
        return true;
    };
    // HappyGhast.java:416-424: decrement only after the first sixty ticks.
    if state.still_timeout > 0 {
        let timeout = state.still_timeout - i32::from(entity.tick_count() > 60);
        publish_still_timeout(entity, store, timeout);
    }
    let Some(above) = scan_player_above(entity, store, None) else {
        return false;
    };
    if above {
        publish_still_timeout(entity, store, 10);
    }
    true
}

/// `HappyGhast.continuousHeal` (`HappyGhast.java:452-459`).
pub(crate) fn continuous_heal(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        || !entity.is_alive_with_health()
    {
        return;
    }
    let Some(living) = entity.living() else {
        return;
    };
    if living.health() == living.max_health() {
        return;
    }
    let fast = level.is_in_clouds(entity.position(), entity.bb_height())
        || level.has_precipitation_at(entity.block_position());
    if entity.tick_count() % if fast { 20 } else { 600 } == 0 {
        let health = living.health();
        if health > 0.0 {
            crate::entity::living::set_health(entity, health + 1.0);
        }
    }
}

pub(crate) fn publish_still_timeout(entity: &mut Entity, store: &mut EntityStore, timeout: i32) {
    if let Some(packet) = set_server_still_timeout(entity, timeout) {
        store.broadcast_to_tracking(entity.id(), packet);
    }
}

pub(crate) fn added_passenger_timeout(
    entity: &Entity,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    passenger_id: i32,
) -> Option<Option<i32>> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return Some(None);
    };
    let Some(state) = data.happy_ghast_state() else {
        return Some(None);
    };
    // HappyGhast.java:305-311: the scan observes the new bidirectional edge.
    let above = scan_player_above_bodies(entity, bodies, Some((passenger_id, entity.id())))?;
    Some(if !above {
        Some(0)
    } else if state.still_timeout > 10 {
        Some(10)
    } else {
        None
    })
}

pub(crate) fn scan_player_above(
    entity: &Entity,
    store: &EntityStore,
    proposed_edge: Option<(i32, i32)>,
) -> Option<bool> {
    scan_player_above_bodies(
        entity,
        crate::entity::store::EntityBodyReadView::new(store, Some(entity), None),
        proposed_edge,
    )
}

fn scan_player_above_bodies(
    entity: &Entity,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    proposed_edge: Option<(i32, i32)>,
) -> Option<bool> {
    let store = bodies.store();
    let bounds = entity.bounding_box();
    let min_y = bounds.max_y - f64::from(1.0e-5_f32);
    let max_y = bounds.max_y + (bounds.max_y - bounds.min_y) / 2.0;
    let mut unknown = false;
    // HappyGhast.java:535-559: ALL players, root position, half-open bounds.
    for player in store
        .players()
        .filter(|player| !player.avatar_removed() && !player.is_spectator())
    {
        let player_id = player.body().id();
        let root = if proposed_edge.is_none_or(|(passenger, _)| passenger != player_id)
            && store.vehicle(player_id).is_none()
        {
            store
                .passenger_relation_matches(player_id, None)
                .then(|| (None, player.body().position()))
        } else {
            (|| {
                let mut id = player_id;
                let mut seen = BTreeSet::new();
                loop {
                    if !seen.insert(id) {
                        return None;
                    }
                    let proposed_parent = proposed_edge.filter(|(passenger, _)| *passenger == id);
                    let parent = proposed_parent
                        .map(|(_, vehicle)| vehicle)
                        .or_else(|| store.vehicle(id));
                    if proposed_parent.is_none() && !store.passenger_relation_matches(id, parent) {
                        return None;
                    }
                    match parent {
                        Some(parent) => id = parent,
                        None if id == player_id => return Some((None, player.body().position())),
                        None => {
                            let root = if id == entity.id() {
                                entity
                            } else {
                                bodies.get(id)?
                            };
                            return Some((Some(root.entity_type_id()), root.position()));
                        }
                    }
                    if id != entity.id() && bodies.get(id).is_none() {
                        return None;
                    }
                }
            })()
        };
        let Some((entity_type, position)) = root else {
            unknown = true;
            continue;
        };
        if entity_type == Some(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID) {
            continue;
        }
        if position.x >= bounds.min_x - 1.0
            && position.x < bounds.max_x + 1.0
            && position.y >= min_y
            && position.y < max_y
            && position.z >= bounds.min_z - 1.0
            && position.z < bounds.max_z + 1.0
        {
            return Some(true);
        }
    }
    (!unknown).then_some(false)
}

// HappyGhast.java:83-90: emit the live position before publishing a still flag.
pub(crate) fn set_server_still_timeout(
    entity: &mut Entity,
    timeout: i32,
) -> Option<ClientboundPlayPacket> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let previous = data.happy_ghast_state()?;
    let packet = (previous.still_timeout <= 0 && timeout > 0).then(|| {
        ClientboundPlayPacket::EntityPositionSync(EntityPositionSync {
            id: entity.id(),
            position: entity.position(),
            delta_movement: entity.delta_movement(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            on_ground: entity.on_ground(),
        })
    });
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    let state = data.happy_ghast_state_mut().unwrap();
    if state.set_still_timeout(timeout) {
        entity.mark_synched_dirty(
            bbb_protocol::entity_synched_data::VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID,
        );
    }
    packet
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[test]
fn happy_ghast_fresh_carry_keeps_a_riders_player_subtree_and_updates_the_detached_body() {
    use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};
    use crate::entity::ExternalPlayerCanRide;
    use bbb_protocol::packets::Vec3d;

    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let ghast = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, position)
        .unwrap();
    let chicken = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID,
            position,
        )
        .unwrap();
    assert!(store.attach_test_player(1000, chicken, ExternalPlayerCanRide::Allowed));
    let mut body = store.detach(ghast).unwrap();
    publish_still_timeout(&mut body, &mut store, 50);
    store.drain_lifecycle_events();
    assert!(store.fresh_carry(&mut body, chicken, false));
    assert!(store.is_detached(ghast));
    assert_eq!(store.vehicle(chicken), Some(ghast));
    assert_eq!(store.vehicle(1000), Some(chicken));
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert_eq!(data.happy_ghast_state().unwrap().still_timeout, 0);
    assert!(store
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event,
            EntityLifecycleEvent::PassengersChanged { vehicle_id, passenger_ids }
            if *vehicle_id == ghast && passenger_ids == &[chicken]
        )));
}

#[cfg(test)]
#[test]
fn happy_ghast_cloud_healing_uses_live_cloud_height_and_color_alpha() {
    use crate::entity::store::EntityIdCounter;
    use bbb_protocol::packets::Vec3d;
    use std::sync::Arc;

    let level_with_clouds = |cloud_height: f32, cloud_color: &str| {
        let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
        for registry in &mut startup.registries.registries {
            if registry.registry_id != "minecraft:dimension_type" {
                continue;
            }
            for element in &mut registry.elements {
                if element.id != "minecraft:overworld" {
                    continue;
                }
                let mut value: serde_json::Value =
                    serde_json::from_slice(&element.source_json).unwrap();
                value["timelines"] = serde_json::json!([]);
                value["attributes"]["minecraft:visual/cloud_height"] =
                    serde_json::json!(cloud_height);
                value["attributes"]["minecraft:visual/cloud_color"] =
                    serde_json::json!(cloud_color);
                element.source_json = serde_json::to_vec(&value).unwrap().into();
                element.source = Some(
                    bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap(),
                );
                element.codec_value = None;
            }
        }
        crate::entity::tests::native_level::NativeLevel::with_datapacks(Arc::new(startup))
    };

    let health_after = |native: &mut crate::entity::tests::native_level::NativeLevel, y: f64| {
        let mut store = EntityStore::new(17, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
                Vec3d { x: 0.5, y, z: 0.5 },
            )
            .unwrap();
        let body = store.get_mut(id).unwrap();
        body.set_no_gravity(true);
        let EntityData::GeneralLiving(data) = &mut body.data else {
            unreachable!();
        };
        data.set_no_ai(true);
        crate::entity::living::set_health(body, 10.0);
        for _ in 0..40 {
            native.with_level(|level| crate::entity::tick_entities(&mut store, level, true));
        }
        store.get(id).unwrap().living().unwrap().health()
    };

    let mut custom_height = level_with_clouds(120.0, "#ccffffff");
    assert_eq!(health_after(&mut custom_height, 118.0), 12.0);
    assert_eq!(health_after(&mut custom_height, 190.0), 10.0);

    let mut transparent_clouds = level_with_clouds(120.0, "#00ffffff");
    assert_eq!(health_after(&mut transparent_clouds, 118.0), 10.0);
}
