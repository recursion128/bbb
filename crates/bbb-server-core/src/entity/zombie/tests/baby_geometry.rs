use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::{vanilla_entity_type_dimensions, vanilla_entity_type_eye_height};

fn fixture(entity_type: i32) -> (EntityStore, i32) {
    let mut store = EntityStore::new(79, EntityIdCounter::new());
    let id = store.spawn_general_living(entity_type, position()).unwrap();
    (store, id)
}

fn set_baby(entity: &mut crate::entity::Entity, baby: bool, conversion: bool) {
    assert!(if conversion {
        apply_conversion_phenotype(entity, baby, true)
    } else {
        crate::entity::zombie::set_baby(entity, baby)
    });
}

fn assert_geometry(entity: &crate::entity::Entity, width: f32, height: f32, eye: f32) {
    let bbox = entity.bounding_box();
    assert!(
        (bbox.max_x - bbox.min_x - f64::from(width)).abs() < 1.0e-6,
        "width {} != {width}",
        entity.bb_width()
    );
    assert!(
        (bbox.max_y - bbox.min_y - f64::from(height)).abs() < 1.0e-6,
        "height {} != {height}",
        entity.bb_height()
    );
    assert!(
        (entity.eye_height() - eye).abs() < 1.0e-6,
        "eye {} != {eye}",
        entity.eye_height()
    );
}

fn assert_adult(entity: &crate::entity::Entity, scale: f32) {
    let [width, height] = vanilla_entity_type_dimensions(entity.entity_type_id()).unwrap();
    let eye = vanilla_entity_type_eye_height(entity.entity_type_id()).unwrap_or(height * 0.85);
    assert_geometry(entity, width * scale, height * scale, eye * scale);
}

fn dirty_scale(entity: &mut crate::entity::Entity, scale: f64) {
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(scale);
}

#[test]
fn zombie_baby_setter_and_conversion_refresh_geometry_and_metadata() {
    for entity_type in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
    ] {
        for conversion in [false, true] {
            let (mut store, id) = fixture(entity_type);
            let entity = store.get_mut(id).unwrap();
            assert_adult(entity, 1.0);
            entity.take_synched_dirty();
            set_baby(entity, true, conversion);
            assert_geometry(entity, 0.49, 0.99, 0.775);
            assert_eq!(
                entity.take_synched_dirty(),
                vec![VANILLA_ZOMBIE_BABY_DATA_ID]
            );
            assert!(non_default_synched_values(entity)
                .iter()
                .any(|row| row.data_id == VANILLA_ZOMBIE_BABY_DATA_ID
                    && row.value == EntityDataValueKind::Boolean(true)));
            assert!(entity
                .living()
                .unwrap()
                .attributes()
                .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                .unwrap()
                .modifier(BABY_SPEED_MODIFIER_ID)
                .is_some());
            set_baby(entity, false, conversion);
            assert_adult(entity, 1.0);
            assert_eq!(
                entity.take_synched_dirty(),
                vec![VANILLA_ZOMBIE_BABY_DATA_ID]
            );
            assert!(non_default_synched_values(entity)
                .iter()
                .all(|row| row.data_id != VANILLA_ZOMBIE_BABY_DATA_ID));
            assert!(entity
                .living()
                .unwrap()
                .attributes()
                .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                .unwrap()
                .modifier(BABY_SPEED_MODIFIER_ID)
                .is_none());
        }
    }
}

#[test]
fn zombie_sleep_baby_and_conversion_changes_keep_fixed_geometry_until_wake() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let entity = store.get_mut(id).unwrap();
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    set_baby(entity, true, false);
    dirty_scale(entity, 2.0);
    crate::entity::living::refresh_dirty_attributes(entity);
    assert_geometry(entity, 0.2, 0.2, 0.2);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_geometry(entity, 0.49 * 2.0, 0.99 * 2.0, 0.775 * 2.0);
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    set_baby(entity, false, true);
    assert_geometry(entity, 0.2, 0.2, 0.2);
    entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    assert_adult(entity, 2.0);
}
