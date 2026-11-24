use super::*;

pub(super) fn protocol_add_entity(id: i32) -> AddEntity {
    protocol_add_entity_of_type(id, 7)
}

pub(super) fn protocol_add_entity_of_type(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

pub(super) fn single_block_state_chunk(block_state_id: i32) -> crate::ChunkColumn {
    let single_value = |domain, entry_count, global_id| crate::PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: crate::PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    };
    crate::ChunkColumn {
        pos: crate::ChunkPos { x: 0, z: 0 },
        state: crate::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(crate::ChunkSection {
            non_empty_block_count: 4096,
            fluid_count: 4096,
            block_states: single_value(crate::PaletteDomain::BlockStates, 4096, block_state_id),
            biomes: single_value(crate::PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
        light: crate::LightData::default(),
    }
}

pub(super) fn cat_sound_variant_entity_data(registry_id: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CAT_ID,
    )
    .expect("generated Cat variant metadata accessors");
    EntityDataValue {
        data_id: accessors.sound_variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::CatSoundVariant,
            value: bbb_protocol::packets::EntityDataRegistryReference::unresolved(registry_id),
        },
    }
}

pub(super) fn cat_sound_variant_json() -> Vec<u8> {
    br#"{
            "adult_sounds": {
                "ambient_sound": "minecraft:entity.cat.ambient",
                "stray_ambient_sound": "minecraft:entity.cat.ambient",
                "hiss_sound": "minecraft:entity.cat.ambient",
                "hurt_sound": {"sound_id": "example:entity.cat.adult_hurt"},
                "death_sound": {"sound_id": "example:entity.cat.adult_death"},
                "eat_sound": "minecraft:entity.cat.ambient",
                "beg_for_food_sound": "minecraft:entity.cat.ambient",
                "purr_sound": "minecraft:entity.cat.ambient",
                "purreow_sound": "minecraft:entity.cat.ambient"
            },
            "baby_sounds": {
                "ambient_sound": "minecraft:entity.baby_cat.ambient",
                "stray_ambient_sound": "minecraft:entity.baby_cat.ambient",
                "hiss_sound": "minecraft:entity.baby_cat.ambient",
                "hurt_sound": {
                    "sound_id": "example:entity.cat.baby_hurt",
                    "range": 12.0
                },
                "death_sound": {
                    "sound_id": "example:entity.cat.baby_death",
                    "range": 24.0
                },
                "eat_sound": "minecraft:entity.baby_cat.ambient",
                "beg_for_food_sound": "minecraft:entity.baby_cat.ambient",
                "purr_sound": "minecraft:entity.baby_cat.ambient",
                "purreow_sound": "minecraft:entity.baby_cat.ambient"
            }
        }"#
    .to_vec()
}

pub(super) fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

pub(super) fn assert_close_f64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-12,
        "expected {expected}, got {actual}"
    );
}

pub(super) fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
    ProtocolVec3d { x, y, z }
}

pub(super) fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    let properties = props
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &properties)
        .unwrap()
        .id
}
