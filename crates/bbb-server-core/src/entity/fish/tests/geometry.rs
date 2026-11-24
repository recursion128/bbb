use super::*;
use crate::entity::fish::{set_puff_state, set_spawn_salmon_variant};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, vanilla_entity_type_eye_height,
    VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
};

fn fixture(entity_type: i32) -> (EntityStore, i32) {
    let mut store = EntityStore::new(83, EntityIdCounter::new());
    let id = store.spawn_general_living(entity_type, position()).unwrap();
    (store, id)
}

fn assert_box(entity: &crate::entity::Entity, width: f32, height: f32) {
    let bbox = entity.bounding_box();
    assert!(
        (bbox.max_x - bbox.min_x - f64::from(width)).abs() < 1.0e-6,
        "type {}: width {} != {width}",
        entity.entity_type_id(),
        bbox.max_x - bbox.min_x
    );
    assert!(
        (bbox.max_y - bbox.min_y - f64::from(height)).abs() < 1.0e-6,
        "type {}: height {} != {height}",
        entity.entity_type_id(),
        bbox.max_y - bbox.min_y
    );
}

fn assert_eye(entity: &crate::entity::Entity, expected: f32) {
    assert!(
        (entity.eye_height() - expected).abs() < 1.0e-6,
        "type {}: cached eye {} != {expected}",
        entity.entity_type_id(),
        entity.eye_height()
    );
}

fn assert_scaled_eye(entity: &crate::entity::Entity, scale: f32) {
    let [_, height] = vanilla_entity_type_dimensions(entity.entity_type_id()).unwrap();
    let eye = vanilla_entity_type_eye_height(entity.entity_type_id()).unwrap_or(height * 0.85);
    assert_eye(entity, eye * scale);
}

fn assert_scaled_geometry(entity: &crate::entity::Entity, scale: f32) {
    let [width, height] = vanilla_entity_type_dimensions(entity.entity_type_id()).unwrap();
    assert_box(entity, width * scale, height * scale);
    assert_scaled_eye(entity, scale);
}

fn assert_puff(entity: &crate::entity::Entity, state: i32) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert_eq!(data.fish_leaf(), Some(FishLeafState::Pufferfish(state)));
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| entry.name != "PuffState"));
}

#[test]
fn fish_puffer_constructor_and_changed_state_publish_geometry_and_dirty_metadata() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_PUFFERFISH_ID);
    let [width, height] =
        vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_PUFFERFISH_ID).unwrap();
    let entity = store.get_mut(id).unwrap();
    assert_puff(entity, 0);
    assert_box(entity, width * 0.5, height * 0.5);
    assert_scaled_eye(entity, 0.5);
    assert!(non_default_synched_values(entity)
        .iter()
        .all(|row| row.data_id != VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID));
    entity.take_synched_dirty();
    for (state, scale) in [(1, 0.7), (2, 1.0), (0, 0.5)] {
        let previous = match entity.data() {
            EntityData::GeneralLiving(data) => data.fish_leaf(),
            _ => unreachable!(),
        };
        assert!(set_puff_state(entity, state));
        assert_puff(entity, state);
        assert_box(entity, width * scale, height * scale);
        assert_scaled_eye(entity, scale);
        let [row] = synched_values(FishLeafState::Pufferfish(state));
        assert_eq!(row.value.data_id, VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID);
        assert_eq!(row.value.value, EntityDataValueKind::Int(state));
        assert_eq!(row.is_default, state == 0);
        let dirty = entity.take_synched_dirty();
        assert_eq!(
            dirty.contains(&VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID),
            previous != Some(FishLeafState::Pufferfish(state))
        );
        assert!(set_puff_state(entity, state));
        assert!(entity.take_synched_dirty().is_empty());
    }
}

#[test]
fn fish_puffer_save_cold_reload_and_command_clamp_only_upper_bound() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_PUFFERFISH_ID);
    for state in [0, 1, 2] {
        assert!(set_puff_state(store.get_mut(id).unwrap(), state));
        let chunk = write_entity_chunk(&store, (0, -1)).unwrap();
        let fields = chunk.entities().next().unwrap().raw();
        assert_eq!(
            fields
                .iter()
                .filter(|field| field.name == "PuffState")
                .count(),
            1
        );
        assert!(fields
            .iter()
            .any(|field| field.name == "PuffState" && field.value == NbtSummaryValue::Int(state)));
        let mut restored = EntityStore::new(89, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut restored, &chunk).unwrap();
        assert_puff(restored.get(ids[0]).unwrap(), state);
        assert_scaled_geometry(
            restored.get(ids[0]).unwrap(),
            match state {
                0 => 0.5,
                1 => 0.7,
                _ => 1.0,
            },
        );
        assert_eq!(
            restored.get(ids[0]).unwrap().bounding_box(),
            store.get(id).unwrap().bounding_box()
        );
    }
    for (input, expected) in [(i32::MAX, 2), (-9, -9), (0, 0)] {
        let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
            unreachable!()
        };
        fields
            .iter_mut()
            .find(|field| field.name == "PuffState")
            .unwrap()
            .value = NbtSummaryValue::Int(input);
        store.load_command_data(id, &fields).unwrap();
        assert_puff(store.get(id).unwrap(), expected);
        assert_scaled_geometry(
            store.get(id).unwrap(),
            if expected == 0 { 0.5 } else { 1.0 },
        );
        let chunk = write_entity_chunk(&store, (0, -1)).unwrap();
        assert!(chunk
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .any(
                |field| field.name == "PuffState" && field.value == NbtSummaryValue::Int(expected)
            ));
    }
    let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    fields.retain(|field| field.name != "PuffState");
    store.load_command_data(id, &fields).unwrap();
    assert_puff(store.get(id).unwrap(), 0);
    assert_scaled_geometry(store.get(id).unwrap(), 0.5);
    let entity = store.get_mut(id).unwrap();
    entity.set_sleeping_pos(Some(bbb_protocol::packets::BlockPos { x: 3, y: 62, z: -8 }));
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    fields
        .iter_mut()
        .find(|field| field.name == "PuffState")
        .unwrap()
        .value = NbtSummaryValue::Int(1);
    store.load_command_data(id, &fields).unwrap();
    assert_puff(store.get(id).unwrap(), 1);
    assert_box(store.get(id).unwrap(), 0.2, 0.2);
    assert_eye(store.get(id).unwrap(), 0.2);
    let entity = store.get_mut(id).unwrap();
    entity.set_sleeping_pos(None);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert!(set_puff_state(store.get_mut(id).unwrap(), 3));
}

#[test]
fn fish_salmon_changed_variant_refreshes_box_and_sleep_setters_keep_fixed_box() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_SALMON_ID);
    let [width, height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_SALMON_ID).unwrap();
    let entity = store.get_mut(id).unwrap();
    for (variant, scale) in [
        (SalmonVariant::Small, 0.5),
        (SalmonVariant::Large, 1.5),
        (SalmonVariant::Medium, 1.0),
    ] {
        entity.take_synched_dirty();
        assert!(set_spawn_salmon_variant(entity, variant));
        assert_box(entity, width * scale, height * scale);
        assert_scaled_eye(entity, scale);
        assert!(entity
            .take_synched_dirty()
            .contains(&VANILLA_SALMON_VARIANT_DATA_ID));
        assert!(set_spawn_salmon_variant(entity, variant));
        assert!(entity.take_synched_dirty().is_empty());
    }
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    assert!(set_spawn_salmon_variant(entity, SalmonVariant::Large));
    assert_box(entity, 0.2, 0.2);
    assert_eye(entity, 0.2);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_box(entity, width * 1.5, height * 1.5);
    assert_scaled_eye(entity, 1.5);
}

#[test]
fn fish_puffer_changed_state_during_sleep_keeps_fixed_box_and_wakes_latest_size() {
    let (mut puffer, puffer_id) = fixture(VANILLA_ENTITY_TYPE_PUFFERFISH_ID);
    let entity = puffer.get_mut(puffer_id).unwrap();
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    for state in [1, 2, -1] {
        assert!(set_puff_state(entity, state));
        assert_box(entity, 0.2, 0.2);
        assert_eye(entity, 0.2);
    }
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    let [width, height] =
        vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_PUFFERFISH_ID).unwrap();
    assert_box(entity, width, height);
    assert_scaled_eye(entity, 1.0);
}

#[test]
fn fish_scale_and_leaf_geometry_survive_cold_command_and_sleep_restore() {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
    for (entity_type, name, first, first_scale, sleeping, awake_scale) in [
        (
            VANILLA_ENTITY_TYPE_SALMON_ID,
            "type",
            NbtSummaryValue::String("large".into()),
            1.5,
            NbtSummaryValue::String("small".into()),
            0.5,
        ),
        (
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
            "PuffState",
            NbtSummaryValue::Int(1),
            0.7,
            NbtSummaryValue::Int(2),
            1.0,
        ),
    ] {
        let (mut store, id) = fixture(entity_type);
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(2.0);
        crate::entity::living::refresh_dirty_attributes(store.get_mut(id).unwrap());
        let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
            unreachable!()
        };
        fields
            .iter_mut()
            .find(|field| field.name == name)
            .unwrap()
            .value = first;
        store.load_command_data(id, &fields).unwrap();
        assert_scaled_geometry(store.get(id).unwrap(), first_scale * 2.0);
        let chunk = write_entity_chunk(&store, (0, -1)).unwrap();
        let mut restored = EntityStore::new(97, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut restored, &chunk).unwrap();
        assert_scaled_geometry(restored.get(ids[0]).unwrap(), first_scale * 2.0);
        let entity = store.get_mut(id).unwrap();
        entity.set_sleeping_pos(Some(bbb_protocol::packets::BlockPos { x: 3, y: 62, z: -8 }));
        entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(3.0);
        crate::entity::living::refresh_dirty_attributes(entity);
        let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
            unreachable!()
        };
        fields
            .iter_mut()
            .find(|field| field.name == name)
            .unwrap()
            .value = sleeping;
        store.load_command_data(id, &fields).unwrap();
        assert_box(store.get(id).unwrap(), 0.2, 0.2);
        assert_eye(store.get(id).unwrap(), 0.2);
        let chunk = write_entity_chunk(&store, (0, -1)).unwrap();
        let mut restored = EntityStore::new(101, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut restored, &chunk).unwrap();
        let entity = restored.get_mut(ids[0]).unwrap();
        assert_box(entity, 0.2, 0.2);
        assert_eye(entity, 0.2);
        entity.set_sleeping_pos(None);
        entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
        assert_scaled_geometry(entity, awake_scale * 3.0);
    }
}
