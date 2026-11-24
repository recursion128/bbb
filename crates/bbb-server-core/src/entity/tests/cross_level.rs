use super::*;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
};

use crate::entity::minecart_container;
use crate::entity::portal::PortalKind;
use crate::entity::{
    transfer_stored_entity_subtree_cross_level, transfer_stored_entity_tree_cross_level,
    EntityLifecycleEvent, StoredEntityTransition,
};

const ALL_RELATIVES: i32 = (1 << 9) - 1;

fn position(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

fn stores(seed: i64) -> (EntityStore, EntityStore) {
    let ids = EntityIdCounter::new();
    (
        EntityStore::new(seed, ids.clone()),
        EntityStore::new(seed.wrapping_add(1), ids),
    )
}

fn absolute_transition(destination: Vec3d) -> StoredEntityTransition {
    StoredEntityTransition {
        position: destination,
        delta_movement: position(0.25, -0.5, 0.75),
        y_rot: 35.0,
        x_rot: 120.0,
        relatives_mask: 0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn single_entity_keeps_uuid_and_persistent_state_with_a_fresh_id() {
    let (mut source, mut target) = stores(0x0043_524f_5353);
    let old_id =
        crate::entity::spawn_minecart(&mut source, position(1.0, 64.0, 3.0), 10.0).unwrap();
    let old_uuid = source.get(old_id).unwrap().uuid();
    let entity = source.get_mut(old_id).unwrap();
    entity.set_delta_movement(position(1.0, 2.0, 3.0));
    entity.set_rot(10.0, -20.0);
    entity.set_as_inside_portal(PortalKind::Nether, pos(1, 64, 3), 300);
    assert_eq!(entity.process_portal(10, true, 300), None);
    let portal_process = entity.portal_process().copied();
    entity.set_portal_cooldown();
    source.drain_lifecycle_events();

    let destination = position(20.0, 80.0, -4.0);
    let outcome = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        old_id,
        absolute_transition(destination),
    )
    .unwrap();

    assert_eq!(outcome.old_root_id, old_id);
    assert_ne!(outcome.new_root_id, old_id);
    assert_eq!(outcome.id_map.get(&old_id), Some(&outcome.new_root_id));
    assert!(source.get(old_id).is_none());
    let replacement = target.get(outcome.new_root_id).unwrap();
    assert_eq!(replacement.uuid(), old_uuid);
    assert_eq!(replacement.position(), destination);
    assert_eq!(replacement.old_position(), destination);
    assert_eq!(replacement.delta_movement(), position(0.25, -0.5, 0.75));
    assert_eq!((replacement.y_rot(), replacement.x_rot()), (35.0, 90.0));
    assert_eq!(replacement.portal_cooldown(), 300);
    assert_eq!(replacement.portal_process(), portal_process.as_ref());

    assert!(matches!(
        source.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::Removed { entity, .. }]
            if entity.id() == old_id
                && entity.removal_reason() == Some(RemovalReason::ChangedDimension)
    ));
    assert!(matches!(
        target.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::Added { entity, .. }] if entity.id() == outcome.new_root_id
    ));
}

#[test]
fn cross_level_tnt_marks_only_the_target_replacement_as_used_portal() {
    let (mut source, mut target) = stores(0x544e_5450_4f52_5441);
    let random = GameRandom::with_seed(1);
    let old_id =
        crate::entity::spawn_tnt(&mut source, &random, position(0.5, 64.0, 0.5), None).unwrap();
    assert!(matches!(
        source.get(old_id).unwrap().data(),
        EntityData::Tnt(data) if !data.used_portal()
    ));

    let outcome = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        old_id,
        absolute_transition(position(4.5, 70.0, 4.5)),
    )
    .unwrap();

    assert!(source.get(old_id).is_none());
    assert!(matches!(
        target.get(outcome.new_root_id).unwrap().data(),
        EntityData::Tnt(data) if data.used_portal()
    ));
}

#[test]
fn cross_level_item_merges_with_target_neighbours_after_publication() {
    let (mut source, mut target) = stores(0x4954_454d_504f_5254);
    target.set_item_publication(crate::entity::ItemPublication::builtin().clone());
    let explicit_default = RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:stone".into()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                    name: "minecraft:rarity".into(),
                    value: NbtSummaryValue::String("common".into()),
                }]),
            },
        ]),
        target.item_resolver(),
    )
    .unwrap();
    let destination = position(4.5, 70.0, 4.5);
    let old_id = spawn_item_entity(
        &mut source,
        position(0.5, 64.0, 0.5),
        stack_of("minecraft:stone", 1),
    )
    .unwrap();
    let neighbour = spawn_item_entity(&mut target, destination, explicit_default.clone()).unwrap();
    target.drain_lifecycle_events();

    let outcome = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        old_id,
        absolute_transition(destination),
    )
    .unwrap();

    assert!(target.get(outcome.new_root_id).is_none());
    let EntityData::Item(data) = target.get(neighbour).unwrap().data() else {
        unreachable!();
    };
    assert_eq!(stack_count(data.item()), 2);
    assert_eq!(
        data.item()
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                target.item_resolver(),
            )
            .unwrap(),
        Some(NbtSummaryValue::String("common".into()))
    );
    assert_eq!(
        target.get_mut(neighbour).unwrap().take_synched_dirty(),
        vec![bbb_protocol::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID]
    );
    assert!(matches!(
        target.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::Added { entity, .. }, EntityLifecycleEvent::Removed { .. }]
            if entity.id() == outcome.new_root_id
    ));
}

#[test]
fn cross_level_item_can_survive_after_absorbing_a_target_neighbour() {
    let (mut source, mut target) = stores(0x4954_454d_5355_5256);
    let destination = position(4.5, 70.0, 4.5);
    let old_id = spawn_item_entity(
        &mut source,
        position(0.5, 64.0, 0.5),
        stack_of("minecraft:stone", 3),
    )
    .unwrap();
    let neighbour =
        spawn_item_entity(&mut target, destination, stack_of("minecraft:stone", 1)).unwrap();
    target.drain_lifecycle_events();

    let outcome = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        old_id,
        absolute_transition(destination),
    )
    .unwrap();

    assert!(target.get(neighbour).is_none());
    let replacement = target.get_mut(outcome.new_root_id).unwrap();
    let EntityData::Item(data) = replacement.data() else {
        unreachable!();
    };
    assert_eq!(stack_count(data.item()), 4);
    assert_eq!(
        replacement.take_synched_dirty(),
        vec![bbb_protocol::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID]
    );
    assert!(target.drain_lifecycle_events().iter().any(
        |event| matches!(event, EntityLifecycleEvent::Removed { entity, .. } if entity.id() == neighbour)
    ));
}

#[test]
fn passenger_tree_moves_with_recursive_relative_transitions() {
    let (mut source, mut target) = stores(0x5041_5353_454e);
    let root = crate::entity::spawn_boat(
        &mut source,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position(1.0, 60.0, 2.0),
        10.0,
    )
    .unwrap();
    let first = crate::entity::spawn_armor_stand(&mut source, position(3.0, 61.0, 5.0)).unwrap();
    let second = crate::entity::spawn_armor_stand(&mut source, position(-1.0, 62.0, 4.0)).unwrap();
    let grandchild =
        crate::entity::spawn_armor_stand(&mut source, position(4.0, 63.0, 7.0)).unwrap();
    source.get_mut(first).unwrap().set_rot(20.0, 10.0);
    source.get_mut(second).unwrap().set_rot(-15.0, -10.0);
    source.get_mut(grandchild).unwrap().set_rot(40.0, 20.0);
    assert!(source.link_existing_passenger_forced(root, first));
    assert!(source.link_existing_passenger_forced(root, second));
    assert!(source.link_existing_passenger_forced(first, grandchild));
    source.drain_lifecycle_events();

    let transition = StoredEntityTransition {
        position: position(10.0, 4.0, -6.0),
        delta_movement: position(0.5, 1.0, -0.5),
        y_rot: 30.0,
        x_rot: 15.0,
        relatives_mask: ALL_RELATIVES,
    };
    let outcome =
        transfer_stored_entity_tree_cross_level(&mut source, &mut target, root, transition)
            .unwrap();

    let new_root = outcome.id_map[&root];
    let new_first = outcome.id_map[&first];
    let new_second = outcome.id_map[&second];
    let new_grandchild = outcome.id_map[&grandchild];
    assert_eq!(target.passengers(new_root), &[new_first, new_second]);
    assert_eq!(target.passengers(new_first), &[new_grandchild]);
    assert_eq!(target.vehicle(new_grandchild), Some(new_first));
    for (old, new) in [
        (root, new_root),
        (first, new_first),
        (second, new_second),
        (grandchild, new_grandchild),
    ] {
        let old_position = match old {
            id if id == root => position(1.0, 60.0, 2.0),
            id if id == first => position(3.0, 61.0, 5.0),
            id if id == second => position(-1.0, 62.0, 4.0),
            _ => position(4.0, 63.0, 7.0),
        };
        let actual = target.get(new).unwrap().position();
        assert_eq!(
            actual,
            position(
                old_position.x + 10.0,
                old_position.y + 4.0,
                old_position.z - 6.0,
            )
        );
    }
    assert_eq!(
        (
            target.get(new_root).unwrap().y_rot(),
            target.get(new_root).unwrap().x_rot()
        ),
        (40.0, 15.0)
    );
    assert_eq!(
        (
            target.get(new_first).unwrap().y_rot(),
            target.get(new_first).unwrap().x_rot()
        ),
        (50.0, 25.0)
    );
    assert_eq!(
        (
            target.get(new_second).unwrap().y_rot(),
            target.get(new_second).unwrap().x_rot()
        ),
        (15.0, 5.0)
    );

    assert!(source.is_empty());
}

#[test]
fn container_cargo_survives_without_source_drop_side_effects() {
    for minecart in [false, true] {
        let (mut source, mut target) = stores(0x0043_4152_474f + i64::from(minecart));
        let old_id = if minecart {
            crate::entity::spawn_minecart_container(
                &mut source,
                VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
                position(0.5, 64.0, 0.5),
                0.0,
            )
            .unwrap()
        } else {
            crate::entity::spawn_chest_boat(
                &mut source,
                VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
                position(0.5, 64.0, 0.5),
                0.0,
            )
            .unwrap()
        };
        let knot = if minecart {
            None
        } else {
            let knot = crate::entity::spawn_leash_knot(&mut source, BlockPos { x: 2, y: 64, z: 0 })
                .unwrap();
            let mut entity = source.detach(old_id).unwrap();
            assert!(crate::entity::leash::set_leashed_to(
                &mut entity,
                &mut source,
                knot,
                true,
            ));
            source.attach(entity);
            Some(knot)
        };
        let stack = crate::entity::item::plain_stack("minecraft:diamond", 7).unwrap();
        if minecart {
            let entity = source.get_mut(old_id).unwrap();
            minecart_container::minecart_container_mut(&mut entity.data)
                .unwrap()
                .set_item(0, Some(stack.clone()))
                .unwrap();
        } else {
            let entity = source.get_mut(old_id).unwrap();
            crate::entity::chest_boat::chest_boat_mut(&mut entity.data)
                .unwrap()
                .set_item(0, Some(stack.clone()))
                .unwrap();
        }
        source.drain_lifecycle_events();

        let outcome = transfer_stored_entity_tree_cross_level(
            &mut source,
            &mut target,
            old_id,
            absolute_transition(position(8.0, 70.0, 8.0)),
        )
        .unwrap();
        assert_eq!(source.len(), 0);
        assert_eq!(target.len(), 1);
        let source_events = source.drain_lifecycle_events();
        assert!(source_events
            .iter()
            .all(|event| !matches!(event, EntityLifecycleEvent::Added { .. })));
        if let Some(knot) = knot {
            assert!(source_events.iter().all(|event| !matches!(
                event,
                EntityLifecycleEvent::Broadcast {
                    entity_id,
                    packet: crate::connection::ClientboundPlayPacket::SetEntityLink(update),
                } if *entity_id == old_id && update.source_id == old_id && update.dest_id == 0
            )));
            assert!(source_events.iter().any(|event| matches!(
                event,
                EntityLifecycleEvent::Removed { entity, .. }
                    if entity.id() == knot
                        && entity.removal_reason() == Some(RemovalReason::Discarded)
            )));
        }
        let replacement = target.get(outcome.new_root_id).unwrap();
        let restored = if minecart {
            minecart_container::minecart_container(replacement.data())
                .unwrap()
                .item(0)
        } else {
            crate::entity::chest_boat::chest_boat(replacement.data())
                .unwrap()
                .item(0)
        };
        assert_eq!(restored.as_deref(), Some(&stack));
    }
}

#[test]
fn portal_passenger_transfer_detaches_only_the_requester_subtree() {
    let (mut source, mut target) = stores(0x504f_5254_414c);
    let vehicle =
        crate::entity::spawn_minecart(&mut source, position(0.0, 64.0, 0.0), 0.0).unwrap();
    let requester = crate::entity::spawn_snowball(
        &mut source,
        position(0.0, 65.0, 0.0),
        crate::entity::projectile_shoot::ProjectileShot::new(Vec3d::default()),
        crate::entity::item::plain_stack("minecraft:snowball", 1).unwrap(),
        None,
    )
    .unwrap();
    let child = crate::entity::spawn_armor_stand(&mut source, position(0.0, 66.0, 0.0)).unwrap();
    let sibling = crate::entity::spawn_armor_stand(&mut source, position(1.0, 65.0, 0.0)).unwrap();
    assert!(source.link_existing_passenger_forced(vehicle, requester));
    assert!(source.link_existing_passenger_forced(requester, child));
    assert!(source.link_existing_passenger_forced(vehicle, sibling));
    source.drain_lifecycle_events();

    let outcome = transfer_stored_entity_subtree_cross_level(
        &mut source,
        &mut target,
        requester,
        absolute_transition(position(20.0, 80.0, 20.0)),
    )
    .unwrap();

    assert!(source.get(vehicle).is_some());
    assert!(source.get(sibling).is_some());
    assert_eq!(source.passengers(vehicle), &[sibling]);
    assert!(source.get(requester).is_none());
    assert!(source.get(child).is_none());
    let new_requester = outcome.id_map[&requester];
    let new_child = outcome.id_map[&child];
    assert_eq!(target.passengers(new_requester), &[new_child]);
    assert_eq!(target.vehicle(new_child), Some(new_requester));
    assert!(source.drain_lifecycle_events().iter().any(|event| {
        matches!(
            event,
            EntityLifecycleEvent::PassengersChanged {
                vehicle_id,
                passenger_ids,
            } if *vehicle_id == vehicle && passenger_ids == &[sibling]
        )
    }));
}

#[test]
fn rotated_relative_delta_matches_position_move_rotation_order() {
    let (mut source, mut target) = stores(0x0044_454c_5441);
    let root = crate::entity::spawn_minecart(&mut source, position(0.0, 64.0, 0.0), 0.0).unwrap();
    let entity = source.get_mut(root).unwrap();
    entity.set_rot(10.0, 20.0);
    entity.set_delta_movement(position(1.0, 2.0, 3.0));
    let transition = StoredEntityTransition {
        position: position(0.0, 80.0, 0.0),
        delta_movement: position(0.5, 0.25, -0.5),
        y_rot: 30.0,
        x_rot: -10.0,
        relatives_mask: (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8),
    };
    let outcome =
        transfer_stored_entity_tree_cross_level(&mut source, &mut target, root, transition)
            .unwrap();
    let replacement = target.get(outcome.new_root_id).unwrap();
    assert_eq!((replacement.y_rot(), replacement.x_rot()), (40.0, 10.0));

    let x_radians = f64::from(10_f32).to_radians() as f32;
    let sin_x = bbb_server_types::geometry::mth_sin(f64::from(x_radians));
    let cos_x = bbb_server_types::geometry::mth_cos(f64::from(x_radians));
    let x_rotated = position(
        1.0,
        2.0 * f64::from(cos_x) + 3.0 * f64::from(sin_x),
        3.0 * f64::from(cos_x) - 2.0 * f64::from(sin_x),
    );
    let y_radians = f64::from(-30_f32).to_radians() as f32;
    let sin_y = bbb_server_types::geometry::mth_sin(f64::from(y_radians));
    let cos_y = bbb_server_types::geometry::mth_cos(f64::from(y_radians));
    let expected = position(
        x_rotated.x * f64::from(cos_y) + x_rotated.z * f64::from(sin_y) + 0.5,
        x_rotated.y + 0.25,
        x_rotated.z * f64::from(cos_y) - x_rotated.x * f64::from(sin_y) - 0.5,
    );
    let actual = replacement.delta_movement();
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.z, expected.z);
}

mod leash_holder;
