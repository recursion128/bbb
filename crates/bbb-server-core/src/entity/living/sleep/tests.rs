use crate::connection::ClientboundPlayPacket;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_POSE_DATA_ID, VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
};
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_server_types::geometry::Aabb;
use uuid::Uuid;

use crate::entity::ai::mob::target::anger::{self, NeutralAnger};
use crate::entity::store::EntityIdCounter;
use crate::entity::tracking::ServerEntity;
use crate::entity::{Entity, EntityStore};

fn fixture(entity_type: i32) -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let pos = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = if entity_type == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
        crate::entity::spawn_armor_stand(&mut store, pos)
    } else {
        store.spawn_general_living(entity_type, pos)
    }
    .unwrap();
    (store, id)
}

fn assert_geometry(entity: &Entity, width: f32, height: f32) {
    assert_eq!(entity.bb_width(), width);
    assert_eq!(entity.bb_height(), height);
    assert_eq!(
        entity.bounding_box(),
        Aabb::of_entity(entity.position(), width, height)
    );
}

fn assert_sleep_geometry(entity: &Entity) {
    assert_geometry(entity, 0.2, 0.2);
    assert_eq!(entity.eye_height(), 0.2);
}

fn set_scale(entity: &mut Entity, scale: f64) {
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(scale);
    crate::entity::living::refresh_dirty_attributes(entity);
}

#[test]
fn sleep_runtime_optional_metadata_setter_keeps_pose_position_and_geometry() {
    for entity_type in [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ] {
        let (mut store, id) = fixture(entity_type);
        let entity = store.get_mut(id).unwrap();
        let initial = (entity.pose_id(), entity.position(), entity.bounding_box());
        entity.take_synched_dirty();
        let rows = crate::entity::tracking::synched_values(entity);
        let row = rows
            .iter()
            .find(|row| row.value.data_id == VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID)
            .unwrap();
        assert_eq!(row.value.data_id, 14);
        assert_eq!(
            row.value.serializer_id,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID
        );
        assert_eq!(row.value.value, EntityDataValueKind::OptionalBlockPos(None));
        assert!(row.is_default);
        assert!(!crate::entity::tracking::non_default_synched_values(entity)
            .iter()
            .any(|row| row.data_id == VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID));
        let bed = BlockPos { x: -3, y: 70, z: 9 };
        entity.set_sleeping_pos(Some(bed));
        entity.set_sleeping_pos(Some(bed));
        assert_eq!(
            entity.take_synched_dirty(),
            vec![VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID]
        );
        assert_eq!(
            (entity.pose_id(), entity.position(), entity.bounding_box()),
            initial
        );
        let row = crate::entity::tracking::non_default_synched_values(entity)
            .into_iter()
            .find(|row| row.data_id == VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID)
            .unwrap();
        assert_eq!(row.value, EntityDataValueKind::OptionalBlockPos(Some(bed)));
        let mut tracker = ServerEntity::new(entity, i32::MAX, false);
        tracker.send_changes(entity);
        entity.set_sleeping_pos(None);
        entity.set_sleeping_pos(None);
        let packets = tracker.send_changes(entity);
        let values: Vec<_> = packets
            .iter()
            .filter_map(|packet| match packet {
                ClientboundPlayPacket::SetEntityData(data) => Some(data.values.as_slice()),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(values.len(), 1);
        assert_eq!(
            values[0].data_id,
            VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID
        );
        assert_eq!(values[0].value, EntityDataValueKind::OptionalBlockPos(None));
        assert_eq!(
            (entity.pose_id(), entity.position(), entity.bounding_box()),
            initial
        );
        assert!(!entity.synched_data_is_dirty());
    }
}

#[test]
fn sleep_runtime_cold_reload_publishes_sleep_pose_and_fixed_box() {
    for entity_type in [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ] {
        let (mut store, id) = fixture(entity_type);
        store
            .get_mut(id)
            .unwrap()
            .set_sleeping_pos(Some(BlockPos { x: 4, y: 70, z: 8 }));
        let chunk = crate::entity::save::write_entity_chunk(&store, (0, 0)).unwrap();
        let mut loaded = EntityStore::new(19, EntityIdCounter::new());
        let ids = crate::entity::save::read_entity_chunk(&mut loaded, &chunk).unwrap();
        let entity = loaded.get(ids[0]).unwrap();
        assert_sleep_geometry(entity);
        assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_SLEEPING_ID);
        let rows = crate::entity::tracking::non_default_synched_values(entity);
        assert!(rows
            .iter()
            .any(|row| row.data_id == VANILLA_ENTITY_POSE_DATA_ID
                && row.value == EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SLEEPING_ID)));
        assert!(rows.iter().any(
            |row| row.data_id == VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID
                && row.value == EntityDataValueKind::OptionalBlockPos(entity.sleeping_pos())
        ));
    }
}

#[test]
fn sleep_runtime_wake_uses_age_and_scale_changed_while_sleeping() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_COW_ID);
    let entity = store.get_mut(id).unwrap();
    let base = (entity.bb_width(), entity.bb_height());
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    assert!(crate::entity::general_living::set_spawner_age(
        entity, -24000
    ));
    set_scale(entity, 3.0);
    assert_sleep_geometry(entity);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_geometry(entity, base.0 * 0.5 * 3.0, base.1 * 0.5 * 3.0);
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    assert!(crate::entity::general_living::set_spawner_age(entity, 0));
    set_scale(entity, 2.0);
    assert_sleep_geometry(entity);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_geometry(entity, base.0 * 2.0, base.1 * 2.0);
}

#[test]
fn sleep_runtime_wake_size_geometry_keeps_slime_phantom_health_and_attributes() {
    for entity_type in [VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID] {
        let (mut store, id) = fixture(entity_type);
        let entity = store.get_mut(id).unwrap();
        let [base_width, base_height] = vanilla_entity_type_dimensions(entity_type).unwrap();
        entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
        let factor = if entity_type == VANILLA_ENTITY_TYPE_SLIME_ID {
            assert!(crate::entity::slime::set_size(entity, 5, false));
            5.0
        } else {
            assert!(crate::entity::phantom::set_size(entity, 10));
            1.0_f32 + 0.15_f32 * 10.0
        };
        set_scale(entity, 2.0);
        crate::entity::living::set_health(entity, 0.75);
        assert_sleep_geometry(entity);
        let attributes = entity.living().unwrap().packed_attributes();
        let health = entity.living().unwrap().health();
        entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
        assert_geometry(
            entity,
            base_width * factor * 2.0,
            base_height * factor * 2.0,
        );
        assert_eq!(entity.living().unwrap().health(), health);
        assert_eq!(entity.living().unwrap().packed_attributes(), attributes);
        entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
        assert_sleep_geometry(entity);
        assert_eq!(entity.living().unwrap().packed_attributes(), attributes);
    }
}

#[test]
fn sleep_runtime_armor_stand_small_marker_changes_keep_sleep_box_until_wake() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
    let entity = store.get_mut(id).unwrap();
    let base = (entity.bb_width(), entity.bb_height());
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    crate::entity::armor_stand::set_small(entity, true);
    assert_sleep_geometry(entity);
    crate::entity::armor_stand::set_marker(entity, true);
    assert_sleep_geometry(entity);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_geometry(entity, 0.0, 0.0);
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    crate::entity::armor_stand::set_marker(entity, false);
    assert_sleep_geometry(entity);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_geometry(entity, base.0 * 0.5, base.1 * 0.5);
}

#[test]
fn preventing_player_rest_uses_monster_class_without_health_filter() {
    let player = Uuid::from_u128(41);
    for entity_type in [
        VANILLA_ENTITY_TYPE_GHAST_ID,
        VANILLA_ENTITY_TYPE_PHANTOM_ID,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        VANILLA_ENTITY_TYPE_COW_ID,
    ] {
        let (store, id) = fixture(entity_type);
        assert!(!store
            .get(id)
            .unwrap()
            .is_preventing_player_rest(player, true, 100, true));
    }
    for entity_type in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    ] {
        let (mut store, id) = fixture(entity_type);
        let entity = store.get_mut(id).unwrap();
        assert!(entity.is_preventing_player_rest(player, true, 100, false));
        crate::entity::living::set_health(entity, 0.0);
        assert!(entity.is_preventing_player_rest(player, true, 100, false));
    }
}

#[test]
fn preventing_player_rest_zombified_piglin_uses_target_and_universal_anger_time() {
    let player = Uuid::from_u128(41);
    let other = Uuid::from_u128(42);
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID);
    let entity = store.get_mut(id).unwrap();
    for (target, end_time, universal, expected) in [
        (None, 200, false, false),
        (None, 200, true, true),
        (None, 100, true, false),
        (None, -1, true, false),
        (Some(player), 200, false, true),
        (Some(player), -1, false, true),
        (Some(other), 200, true, false),
    ] {
        anger::write(entity, NeutralAnger { target, end_time });
        assert_eq!(
            entity.is_preventing_player_rest(player, true, 100, universal),
            expected
        );
        assert!(!entity.is_preventing_player_rest(player, false, 100, universal));
    }
}
