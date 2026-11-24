use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use crate::entity::{Entity, EntityData, GeneralLivingData};

fn spawn(entity_type_id: i32) -> Entity {
    Entity::new(
        1,
        Uuid::from_u128(1),
        entity_type_id,
        Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(entity_type_id).expect("catalog type is general living"),
        ),
    )
}

fn baby(entity_type_id: i32) -> Entity {
    let mut entity = spawn(entity_type_id);
    assert!(crate::entity::set_spawner_age(&mut entity, -24_000));
    entity
}

fn box_of(entity: &Entity) -> [f32; 2] {
    let bounding_box = entity.bounding_box();
    [
        (bounding_box.max_x - bounding_box.min_x) as f32,
        (bounding_box.max_y - bounding_box.min_y) as f32,
    ]
}

#[test]
fn adult_keeps_the_catalog_box() {
    let cow = spawn(VANILLA_ENTITY_TYPE_COW_ID);
    assert_eq!(box_of(&cow), [0.9, 1.4]);
}

// `AbstractCow.BABY_DIMENSIONS` (`AbstractCow.java:32`).
#[test]
fn cow_baby_halves_the_catalog_box() {
    let cow = baby(VANILLA_ENTITY_TYPE_COW_ID);
    assert_eq!(box_of(&cow), [0.45, 0.7]);
}

// `Chicken.BABY_DIMENSIONS` is absolute, not a factor (`Chicken.java:56`).
#[test]
fn chicken_baby_takes_the_named_box() {
    let chicken = baby(VANILLA_ENTITY_TYPE_CHICKEN_ID);
    assert_eq!(box_of(&chicken), [0.3, 0.4]);
}

// `Goat.getAgeScale` (`Goat.java:158-160`) with no `getDefaultDimensions`
// override of its own.
#[test]
fn goat_baby_uses_its_own_age_scale() {
    let goat = baby(VANILLA_ENTITY_TYPE_GOAT_ID);
    assert_eq!(box_of(&goat), [0.9 * 0.55, 1.3 * 0.55]);
}

// `LivingEntity.getAgeScale` (`LivingEntity.java:532-534`).
#[test]
fn pig_baby_uses_the_inherited_age_scale() {
    let pig = baby(VANILLA_ENTITY_TYPE_PIG_ID);
    assert_eq!(box_of(&pig), [0.9 * 0.5, 0.9 * 0.5]);
}

// Slime.java:346-348: default dimensions consume the retained size.
#[test]
fn slime_box_projects_retained_size_through_shared_dimensions() {
    let mut slime = spawn(VANILLA_ENTITY_TYPE_SLIME_ID);
    assert!(crate::entity::slime::set_size(&mut slime, 4, false));
    let [width, height] =
        bbb_protocol::entity_types::vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_SLIME_ID)
            .unwrap();
    assert_eq!(box_of(&slime), [width * 4.0, height * 4.0]);
    assert_eq!(
        super::geometry(slime.data(), VANILLA_ENTITY_TYPE_SLIME_ID, slime.pose_id()),
        Some([width * 4.0, height * 4.0, slime.eye_height()]),
    );
}

// `AgeableMob.setAge`'s boundary write rebuilds the box both ways
// (`AgeableMob.java:149-156`, `:179-185`).
#[test]
fn growing_up_restores_the_adult_box() {
    let mut cow = baby(VANILLA_ENTITY_TYPE_COW_ID);
    assert_eq!(box_of(&cow), [0.45, 0.7]);
    assert!(crate::entity::set_spawner_age(&mut cow, 0));
    assert_eq!(box_of(&cow), [0.9, 1.4]);
}

#[test]
fn baby_dimensions_keep_the_species_eye_height_override() {
    for (entity_type, eye) in [
        (VANILLA_ENTITY_TYPE_COW_ID, 0.665),
        (VANILLA_ENTITY_TYPE_MOOSHROOM_ID, 0.665),
        (VANILLA_ENTITY_TYPE_CHICKEN_ID, 0.28),
        (VANILLA_ENTITY_TYPE_AXOLOTL_ID, 0.2),
        (VANILLA_ENTITY_TYPE_FOX_ID, 0.2975),
        (VANILLA_ENTITY_TYPE_RABBIT_ID, 0.39),
        (VANILLA_ENTITY_TYPE_SQUID_ID, 0.37),
        (VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, 0.37),
        (VANILLA_ENTITY_TYPE_VILLAGER_ID, 0.63),
    ] {
        let entity = baby(entity_type);
        assert_eq!(entity.eye_height(), eye, "type {entity_type}");
    }
    let goat = baby(VANILLA_ENTITY_TYPE_GOAT_ID);
    assert_eq!(goat.eye_height(), (1.3 * 0.85) * 0.55);
}

fn set_scale(entity: &mut Entity, scale: f64) {
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(scale);
}

#[test]
fn dirty_scale_refreshes_box_and_eye_height_together() {
    let mut cow = baby(VANILLA_ENTITY_TYPE_COW_ID);
    set_scale(&mut cow, 2.0);
    assert_eq!(box_of(&cow), [0.45, 0.7]);
    assert_eq!(cow.eye_height(), 0.665);
    crate::entity::living::refresh_dirty_attributes(&mut cow);
    assert_eq!(box_of(&cow), [0.9, 1.4]);
    assert_eq!(cow.eye_height(), 0.665 * 2.0);
    assert!(crate::entity::set_spawner_age(&mut cow, 0));
    assert_eq!(cow.eye_height(), 1.3 * 2.0);
    assert_eq!(box_of(&cow), [1.8, 2.8]);
}

#[test]
fn sitting_camel_scales_its_pose_eye_height_after_age() {
    let mut camel = baby(VANILLA_ENTITY_TYPE_CAMEL_ID);
    camel.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SITTING_ID);
    assert_eq!(camel.eye_height(), 0.845 * 0.6);
    set_scale(&mut camel, 2.0);
    crate::entity::living::refresh_dirty_attributes(&mut camel);
    assert_eq!(camel.eye_height(), (0.845 * 0.6) * 2.0);
    assert_eq!(camel.bb_height(), ((2.375 - 1.43) * 0.6) * 2.0);
}

#[test]
fn armor_stand_small_eye_and_fixed_marker_follow_dimension_refresh() {
    let mut entity = Entity::new(
        1,
        Uuid::from_u128(1),
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        EntityData::ArmorStand(crate::entity::armor_stand::ArmorStandData::new().unwrap()),
    );
    crate::entity::armor_stand::set_small(&mut entity, true);
    assert_eq!(entity.eye_height(), 0.9875);
    set_scale(&mut entity, 2.0);
    crate::entity::living::refresh_dirty_attributes(&mut entity);
    assert_eq!(entity.eye_height(), 0.9875 * 2.0);
    crate::entity::armor_stand::set_marker(&mut entity, true);
    assert_eq!(
        [entity.bb_width(), entity.bb_height(), entity.eye_height()],
        [0.0; 3]
    );
}

fn stored(entity_type: i32, baby: bool) -> (crate::entity::EntityStore, i32) {
    let mut store = crate::entity::EntityStore::new(91, crate::entity::EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = if entity_type == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
        crate::entity::spawn_armor_stand(&mut store, position)
    } else {
        store.spawn_general_living(entity_type, position)
    }
    .unwrap();
    if baby {
        assert!(crate::entity::set_spawner_age(
            store.get_mut(id).unwrap(),
            -24_000
        ));
    }
    if entity_type == VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID {
        assert!(crate::entity::finalize_spawner_villager_type(
            store.get_mut(id).unwrap(),
            "minecraft:plains"
        ));
    }
    (store, id)
}

fn set_monster_baby(entity: &mut Entity, baby: bool) {
    let accepted = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_PIGLIN_ID => crate::entity::set_piglin_baby(entity, baby),
        VANILLA_ENTITY_TYPE_ZOGLIN_ID => crate::entity::set_zoglin_baby(entity, baby),
        _ => crate::entity::zombie::set_baby(entity, baby),
    };
    assert!(accepted);
}

fn baby_monsters() -> [(i32, [f32; 3]); 3] {
    [
        (VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, [0.49, 0.99, 0.67]),
        (VANILLA_ENTITY_TYPE_PIGLIN_ID, [0.49, 0.99, 0.78]),
        (
            VANILLA_ENTITY_TYPE_ZOGLIN_ID,
            [1.3964844 * 0.5, 1.4 * 0.5, 1.4 * 0.85 * 0.5],
        ),
    ]
}

fn observed(entity: &Entity) -> [f32; 3] {
    [entity.bb_width(), entity.bb_height(), entity.eye_height()]
}

#[test]
fn monster_baby_setter_uses_species_box_eye_and_sleep_fixed_geometry() {
    for (entity_type, expected) in baby_monsters() {
        let (mut store, id) = stored(entity_type, false);
        let entity = store.get_mut(id).unwrap();
        let adult = observed(entity);
        set_monster_baby(entity, true);
        assert_eq!(observed(entity), expected, "type {entity_type}");
        entity.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID);
        set_monster_baby(entity, false);
        assert_eq!(observed(entity), [0.2; 3]);
        entity.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID);
        assert_eq!(observed(entity), adult);
    }
}

#[test]
fn monster_baby_scaled_geometry_survives_save_and_reload() {
    for (entity_type, expected) in baby_monsters() {
        let (mut store, id) = stored(entity_type, false);
        let entity = store.get_mut(id).unwrap();
        set_monster_baby(entity, true);
        set_scale(entity, 2.0);
        crate::entity::living::refresh_dirty_attributes(entity);
        let chunk = crate::entity::write_entity_chunk(&store, (0, 0)).unwrap();
        let mut restored =
            crate::entity::EntityStore::new(99, crate::entity::EntityIdCounter::new());
        let ids = crate::entity::read_entity_chunk(&mut restored, &chunk).unwrap();
        assert_eq!(
            observed(restored.get(ids[0]).unwrap()),
            expected.map(|v| v * 2.0),
            "type {entity_type}"
        );
    }
}

#[test]
fn piglin_baby_metadata_and_adjacent_attributes_have_one_owner() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_PIGLIN_BABY_DATA_ID, VANILLA_ZOGLIN_BABY_DATA_ID,
    };
    for (entity_type, data_id) in [
        (VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_PIGLIN_BABY_DATA_ID),
        (VANILLA_ENTITY_TYPE_ZOGLIN_ID, VANILLA_ZOGLIN_BABY_DATA_ID),
    ] {
        let (mut store, id) = stored(entity_type, false);
        let entity = store.get_mut(id).unwrap();
        entity.take_synched_dirty();
        let speed = entity
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
        set_monster_baby(entity, true);
        assert!(entity.take_synched_dirty().contains(&data_id));
        let rows = crate::entity::tracking::non_default_synched_values(entity);
        assert!(rows.iter().any(|row| row.data_id == data_id
            && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true)));
        set_scale(entity, 2.0);
        let geometry = observed(entity);
        set_monster_baby(entity, true);
        assert_eq!(observed(entity), geometry);
        assert!(entity.take_synched_dirty().is_empty());
        if entity_type == VANILLA_ENTITY_TYPE_PIGLIN_ID {
            assert_eq!(
                entity.living().unwrap().attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID
                ),
                speed + speed * f64::from(0.2_f32)
            );
        }
        set_monster_baby(entity, false);
        if entity_type == VANILLA_ENTITY_TYPE_ZOGLIN_ID {
            assert_eq!(
                entity.living().unwrap().attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID
                ),
                0.5
            );
        } else {
            assert_eq!(
                entity.living().unwrap().attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID
                ),
                speed
            );
        }
    }
}
