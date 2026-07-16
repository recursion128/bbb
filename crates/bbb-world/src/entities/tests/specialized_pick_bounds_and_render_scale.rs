use super::*;

fn polar_bear_standing_data(standing: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    )
    .expect("generated PolarBear standing accessor must resolve");
    protocol_bool_data(data_id, standing)
}

#[test]
fn generated_baby_accessor_membership_covers_all_ageable_families() {
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (180, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (181, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
        (182, VANILLA_ENTITY_TYPE_FROG_ID),
        (183, VANILLA_ENTITY_TYPE_GOAT_ID),
        (184, VANILLA_ENTITY_TYPE_PARROT_ID),
        (185, VANILLA_ENTITY_TYPE_SNIFFER_ID),
        (186, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        assert_eq!(store.entities.is_baby(id), Some(false));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
        }));
        assert_eq!(
            store.entities.is_baby(id),
            Some(true),
            "generated AgeableMob accessor: type {entity_type_id}"
        );
    }

    // Pillager.IS_CHARGING_CROSSBOW is also BOOLEAN id 17 and Raider.IS_CELEBRATING is BOOLEAN id
    // 16, but the class hierarchy has no DATA_BABY_ID accessor, so neither can make it a baby.
    store.apply_add_entity(protocol_add_entity_with_type(
        187,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 187,
        values: vec![protocol_bool_data(16, true), protocol_bool_data(17, true),],
    }));
    assert_eq!(store.entities.is_baby(187), Some(false));
}

#[test]
fn turtle_pick_bounds_compose_adult_baby_and_scale_dimensions() {
    let entity_id = 188;
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        entity_id,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
    ));
    assert_eq!(
        store.probe_entity_pick_bounds(entity_id),
        Some(EntityPickBoundsState::from_base_size(1.2, 0.4, 0.0))
    );

    let (baby_data_id, baby_default) =
        super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_TURTLE_ID,
        )
        .expect("generated Turtle baby accessor");
    assert!(!baby_default);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![protocol_bool_data(baby_data_id, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(entity_id),
        Some(EntityPickBoundsState::from_base_size(
            1.2 * 0.3,
            0.4 * 0.3,
            0.0,
        ))
    );

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(entity_id),
        Some(EntityPickBoundsState::from_base_size(
            1.2 * 0.3 * 2.0,
            0.4 * 0.3 * 2.0,
            0.0,
        ))
    );
}

#[test]
fn baby_pick_bounds_follow_vanilla_metadata() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    let mut store = WorldStore::new();
    let (piglin_baby_data_id, piglin_baby_default) =
        super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin baby accessor");
    assert!(!piglin_baby_default);
    let zombie_accessors = super::schema::vanilla_entity_metadata_zombie_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .expect("generated Zombie metadata accessors");
    assert!(!zombie_accessors.baby_default);
    store.apply_add_entity(protocol_add_entity_with_type(34, 100));
    store.apply_add_entity(protocol_add_entity_with_type(35, 26));
    store.apply_add_entity(protocol_add_entity_with_type(36, 139));
    store.apply_add_entity(protocol_add_entity_with_type(37, 150));
    store.apply_add_entity(protocol_add_entity_with_type(38, 101));
    store.apply_add_entity(protocol_add_entity_with_type(39, 30));
    store.apply_add_entity(protocol_add_entity_with_type(40, 141));
    store.apply_add_entity(protocol_add_entity_with_type(45, 4));
    store.apply_add_entity(protocol_add_entity_with_type(46, 54));
    store.apply_add_entity(protocol_add_entity_with_type(47, 108));
    store.apply_add_entity(protocol_add_entity_with_type(71, 104));
    store.apply_add_entity(protocol_add_entity_with_type(48, 7));
    store.apply_add_entity(protocol_add_entity_with_type(49, 127));
    store.apply_add_entity(protocol_add_entity_with_type(50, 61));
    store.apply_add_entity(protocol_add_entity_with_type(51, 137));
    store.apply_add_entity(protocol_add_entity_with_type(52, 35));
    store.apply_add_entity(protocol_add_entity_with_type(53, 58));
    store.apply_add_entity(protocol_add_entity_with_type(54, 96));
    store.apply_add_entity(protocol_add_entity_with_type(55, 36));
    store.apply_add_entity(protocol_add_entity_with_type(56, 87));
    store.apply_add_entity(protocol_add_entity_with_type(57, 66));
    store.apply_add_entity(protocol_add_entity_with_type(58, 116));
    store.apply_add_entity(protocol_add_entity_with_type(59, 151));
    store.apply_add_entity(protocol_add_entity_with_type(60, 78));
    store.apply_add_entity(protocol_add_entity_with_type(61, 134));
    store.apply_add_entity(protocol_add_entity_with_type(62, 11));
    store.apply_add_entity(protocol_add_entity_with_type(63, 21));
    store.apply_add_entity(protocol_add_entity_with_type(64, 64));
    store.apply_add_entity(protocol_add_entity_with_type(65, 88));
    store.apply_add_entity(protocol_add_entity_with_type(66, 91));
    store.apply_add_entity(protocol_add_entity_with_type(67, 111));
    store.apply_add_entity(protocol_add_entity_with_type(68, 129));
    store.apply_add_entity(protocol_add_entity_with_type(69, 148));
    store.apply_add_entity(protocol_add_entity_with_type(70, 149));

    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 0.9, 0.0))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            0.9 * 0.5,
            0.0
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 34,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(34),
        Some(EntityPickBoundsState::from_base_size(0.9, 0.9, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(35),
        Some(EntityPickBoundsState::from_base_size(0.3, 0.4, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(52),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.65,
            0.6 * 0.65,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 36,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(36),
        Some(EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 37,
        values: vec![protocol_bool_data(zombie_accessors.baby_data_id, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(37),
        Some(EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 38,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(38),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.95, 0.0))
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 38,
        values: vec![protocol_bool_data(piglin_baby_data_id, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(38),
        Some(EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 39,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(39),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.4 * 0.5,
            0.0
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 40,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(40),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            1.95 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 45,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(45),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.6,
            0.65 * 0.6,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 45,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(45),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.6 * 2.0,
            0.65 * 0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 46,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(46),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.6,
            0.7 * 0.6,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 46,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(46),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.6 * 2.0,
            0.7 * 0.6 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 47,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(47),
        Some(EntityPickBoundsState::from_base_size(0.24, 0.4, 0.0))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 47,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(47),
        Some(EntityPickBoundsState::from_base_size(
            0.24 * 2.0,
            0.4 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(71),
        Some(EntityPickBoundsState::from_base_size(
            1.4 * 0.5,
            1.4 * 0.5,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 71,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(71),
        Some(EntityPickBoundsState::from_base_size(
            1.4 * 0.5 * 2.0,
            1.4 * 0.5 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(53),
        Some(EntityPickBoundsState::from_base_size(
            4.0 * 0.2375,
            4.0 * 0.2375,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 53,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(53),
        Some(EntityPickBoundsState::from_base_size(
            4.0 * 0.2375,
            4.0 * 0.2375,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 48,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(48),
        Some(EntityPickBoundsState::from_base_size(0.5, 0.25, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 49,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(49),
        Some(EntityPickBoundsState::from_base_size(0.5, 0.63, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(50),
        Some(EntityPickBoundsState::from_base_size(0.5, 0.63, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(54),
        Some(EntityPickBoundsState::from_base_size(
            1.3 * 0.5,
            1.25 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(55),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.5 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 56,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(56),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.6 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 57,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(57),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7,
            1.6 * 0.7,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 57,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(57),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7 * 2.0,
            1.6 * 0.7 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(58),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7,
            1.6 * 0.7,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 59,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(59),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.7,
            1.6 * 0.7,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(60),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.87 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(61),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.87 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(51),
        Some(EntityPickBoundsState::from_base_size(
            1.2 * 0.3,
            0.4 * 0.3,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 51,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(51),
        Some(EntityPickBoundsState::from_base_size(
            1.2 * 0.3 * 2.0,
            0.4 * 0.3 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(62),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.5,
            0.6 * 0.5,
            0.0,
        ))
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 62,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(62),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 0.5 * 2.0,
            0.6 * 0.5 * 2.0,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(63),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            0.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 64,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(64),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.4 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 65,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(65),
        Some(EntityPickBoundsState::from_base_size(
            0.875 * 0.5,
            0.95 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 66,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(66),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            0.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 67,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(67),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.3 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(68),
        Some(EntityPickBoundsState::from_base_size(
            0.9 * 0.5,
            1.7 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 69,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(69),
        Some(EntityPickBoundsState::from_base_size(
            0.6 * 0.5,
            0.85 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(70),
        Some(EntityPickBoundsState::from_base_size(
            1.3964844 * 0.5,
            1.4 * 0.5,
            0.0,
        ))
    );
}

#[test]
fn shulker_pick_bounds_follow_attach_face_and_peek_metadata() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    ));

    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(82),
        shulker_pick_bounds(DIRECTION_DOWN, 0.0, 1.0),
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![shulker_peek_data(100)],
    }));
    store.advance_entity_client_animations(20);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(82),
        shulker_pick_bounds(DIRECTION_DOWN, 1.0, 1.0),
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![shulker_attach_face_data(DIRECTION_UP)],
    }));
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(82),
        shulker_pick_bounds(DIRECTION_UP, 1.0, 1.0),
    );

    for (id, attach_face) in [
        (83, DIRECTION_UP),
        (84, DIRECTION_NORTH),
        (85, DIRECTION_SOUTH),
        (86, DIRECTION_WEST),
        (87, DIRECTION_EAST),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(
            id,
            VANILLA_ENTITY_TYPE_SHULKER_ID,
        ));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![
                shulker_attach_face_data(attach_face),
                shulker_peek_data(100),
            ],
        }));
        store.advance_entity_client_animations(20);
        assert_pick_bounds_close(
            store.probe_entity_pick_bounds(id),
            shulker_pick_bounds(attach_face, 1.0, 1.0),
        );
    }

    // Vanilla `Direction.BY_ID` uses positive-modulo wrap, so -1 wraps to EAST.
    store.apply_add_entity(protocol_add_entity_with_type(
        88,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![shulker_attach_face_data(-1), shulker_peek_data(100)],
    }));
    store.advance_entity_client_animations(20);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_EAST, 1.0, 1.0),
    );
}

#[test]
fn shulker_peek_pick_bounds_advance_toward_metadata_target() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        88,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![shulker_peek_data(100)],
    }));
    store.advance_entity_client_animations(0);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.0, 1.0),
    );

    store.advance_entity_client_animations(1);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.05, 1.0),
    );

    store.advance_entity_client_animations(9);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.5, 1.0),
    );

    let cloned = store.clone();
    assert_pick_bounds_close(
        cloned.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.5, 1.0),
    );
    let restored: WorldStore =
        serde_json::from_value(serde_json::to_value(&store).unwrap()).unwrap();
    assert_pick_bounds_close(
        restored.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.5, 1.0),
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![shulker_peek_data(0)],
    }));
    store.advance_entity_client_animations(1);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.45, 1.0),
    );

    store.advance_entity_client_animations(9);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(88),
        shulker_pick_bounds(DIRECTION_DOWN, 0.0, 1.0),
    );
}

#[test]
fn shulker_pick_bounds_apply_vanilla_scale_cap() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        89,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    ));

    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 89,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 4.0,
            modifiers: Vec::new(),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 89,
        values: vec![shulker_peek_data(100)],
    }));

    store.advance_entity_client_animations(20);
    assert_pick_bounds_close(
        store.probe_entity_pick_bounds(89),
        shulker_pick_bounds(DIRECTION_DOWN, 1.0, 3.0),
    );
}

#[test]
fn generated_sanitize_scale_overrides_apply_to_render_state() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let mut store = WorldStore::new();
    for (entity_id, entity_type_id, expected) in [
        (189, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, 1.0),
        (190, VANILLA_ENTITY_TYPE_SHULKER_ID, 3.0),
        (192, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.0),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(entity_id, entity_type_id));
        assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
                base: 4.0,
                modifiers: Vec::new(),
            }],
        }));
        assert_eq!(
            store
                .entity_model_source_at_partial_tick(entity_id, 0.0)
                .map(|source| source.scale),
            Some(expected),
            "sanitizeScale projection for type {entity_type_id}"
        );
    }

    // EnderDragonRenderer owns a dedicated non-LivingEntityRenderState path and therefore has no
    // generic model source `scale` field. Its LivingEntity attribute state must still accept the
    // packet; the generated schema test locks the fixed-1.0 sanitizer used by entity dimensions.
    store.apply_add_entity(protocol_add_entity_with_type(
        191,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    ));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 191,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 4.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity(191).unwrap().attributes,
        vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 4.0,
            modifiers: Vec::new(),
        }]
    );
}

#[test]
fn polar_bear_standing_pick_bounds_follow_client_animation_ticks() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    ));

    let adult_bounds = Some(EntityPickBoundsState::from_base_size(1.4, 1.4, 0.0));
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![polar_bear_standing_data(true)],
    }));
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.advance_entity_client_animations(0);
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.advance_entity_client_animations(1);
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(
            1.4,
            1.4 * (1.0 + 1.0 / 6.0),
            0.0,
        ))
    );

    store.advance_entity_client_animations(5);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );

    let cloned = store.clone();
    assert_eq!(
        cloned.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );
    let restored: WorldStore =
        serde_json::from_value(serde_json::to_value(&store).unwrap()).unwrap();
    assert_eq!(
        restored.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![polar_bear_standing_data(false)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(1.4, 2.8, 0.0))
    );

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_pick_bounds(80),
        Some(EntityPickBoundsState::from_base_size(
            1.4,
            1.4 * (1.0 + 5.0 / 6.0),
            0.0,
        ))
    );

    store.advance_entity_client_animations(5);
    assert_eq!(store.probe_entity_pick_bounds(80), adult_bounds);

    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![
            protocol_bool_data(AGEABLE_BABY_DATA_ID, true),
            polar_bear_standing_data(true),
        ],
    }));
    store.advance_entity_client_animations(7);
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 81,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(81),
        Some(EntityPickBoundsState::from_base_size(
            0.7 * 2.0,
            1.4 * 2.0,
            0.0
        ))
    );
}

#[test]
fn polar_bear_standing_projects_render_stand_scale() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let stand_scale = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .polar_bear_stand_scale
    };

    // A polar bear on all fours has no standing animation state.
    assert_eq!(stand_scale(&store, 80, 1.0), 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![polar_bear_standing_data(true)],
    }));

    // After one client tick clientSideStandAnimationO=0, clientSideStandAnimation=1,
    // so getStandingAnimationScale(a) = lerp(a, 0, 1) / 6 = a / 6.
    store.advance_entity_client_animations(1);
    assert_eq!(stand_scale(&store, 80, 0.0), 0.0);
    assert_eq!(stand_scale(&store, 80, 0.5), 0.5 / 6.0);
    assert_eq!(stand_scale(&store, 80, 1.0), 1.0 / 6.0);

    // Once fully reared (prev == current == 6) the scale saturates at 1.0.
    store.advance_entity_client_animations(10);
    assert_eq!(stand_scale(&store, 80, 0.0), 1.0);
    assert_eq!(stand_scale(&store, 80, 1.0), 1.0);

    // A non-polar-bear never carries a standing scale.
    assert_eq!(stand_scale(&store, 81, 1.0), 0.0);
}

#[test]
fn advancing_entity_client_animations_in_batches_matches_single_ticks() {
    let mut batch = WorldStore::new();
    batch.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    ));
    assert!(batch.apply_set_entity_data(ProtocolSetEntityData {
        id: 90,
        values: vec![polar_bear_standing_data(true)],
    }));

    let mut repeated = batch.clone();
    batch.advance_entity_client_animations(7);
    for _ in 0..7 {
        repeated.advance_entity_client_animations(1);
    }

    assert_eq!(
        batch.probe_entity_pick_bounds(90),
        repeated.probe_entity_pick_bounds(90)
    );
    assert_eq!(
        batch.probe_entity(90).unwrap().client_animations,
        repeated.probe_entity(90).unwrap().client_animations
    );
}

#[test]
fn armor_stand_pick_bounds_follow_client_flags() {
    let mut store = WorldStore::new();
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .expect("generated ArmorStand metadata accessors must resolve");
    store.apply_add_entity(protocol_add_entity_with_type(
        26,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        27,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        28,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(26),
        Some(EntityPickBoundsState::from_base_size(0.5, 1.975, 0.0))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 27,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.client_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.small_mask),
        }],
    }));
    assert_eq!(
        store.probe_entity_pick_bounds(27),
        Some(EntityPickBoundsState::from_base_size(
            0.5 * 0.5,
            1.975 * 0.5,
            0.0,
        ))
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 28,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.client_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.marker_mask),
        }],
    }));
    assert_eq!(store.probe_entity_pick_bounds(28), None);
}

#[test]
fn armor_stand_marker_has_model_target_without_pick_target() {
    let mut store = WorldStore::new();
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .expect("generated ArmorStand metadata accessors must resolve");
    store.apply_add_entity(protocol_add_entity_with_type(
        28,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));
    let marker_flag = protocol_byte_data(accessors.client_flags_data_id, accessors.marker_mask);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 28,
        values: vec![marker_flag.clone()],
    }));

    assert_eq!(store.probe_entity_pick_bounds(28), None);
    assert!(store
        .entity_pick_targets()
        .iter()
        .all(|target| target.entity_id != 28));

    let model_targets = store
        .entities
        .model_targets_at_partial_tick(1.0, &store.registries);
    let model_target = model_targets
        .iter()
        .find(|target| target.entity_id == 28)
        .unwrap_or_else(|| panic!("missing marker armor stand model target"));
    assert_eq!(
        model_target.bounds,
        EntityPickBoundsState::from_base_size(0.0, 0.0, 0.0)
    );

    let sources = store.entity_model_sources_at_partial_tick(1.0);
    let source = sources
        .iter()
        .find(|source| source.entity_id == 28)
        .unwrap_or_else(|| panic!("missing marker armor stand model source"));
    assert_eq!(source.entity_type_id, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
    assert_eq!(
        source.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(source.data_values, vec![marker_flag]);
}

#[test]
fn player_pick_bounds_skip_spectator_profile() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        29,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert_eq!(
        store.probe_entity_pick_bounds(29),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0))
    );

    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![
            ProtocolPlayerInfoAction::AddPlayer,
            ProtocolPlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![protocol_player_info_entry_with_mode(
            default_entity_uuid(),
            ProtocolGameType::Spectator,
        )],
    });
    assert_eq!(store.probe_entity_pick_bounds(29), None);

    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![ProtocolPlayerInfoAction::UpdateGameMode],
        entries: vec![protocol_player_info_entry_with_mode(
            default_entity_uuid(),
            ProtocolGameType::Survival,
        )],
    });
    assert_eq!(
        store.probe_entity_pick_bounds(29),
        Some(EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0))
    );
}

#[test]
fn local_player_fishing_bobber_uses_add_entity_owner_data() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    assert_eq!(store.local_player_fishing_bobber_id(), None);

    store.apply_add_entity(protocol_add_entity_with_type_data(
        10,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        99,
    ));
    assert_eq!(store.local_player_fishing_bobber_id(), Some(10));

    store.apply_add_entity(protocol_add_entity_with_type_data(
        11,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        123,
    ));
    assert_eq!(store.local_player_fishing_bobber_id(), Some(10));

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    assert_eq!(store.local_player_fishing_bobber_id(), None);
}
