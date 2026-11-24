use super::*;

pub(super) fn respawn_state_with_local_player_data() -> WorldStore {
    let mut store = WorldStore::new();
    store.local_player_id = Some(123);
    store.apply_add_entity(local_player_entity(123));
    store.apply_add_entity(protocol_add_entity(456));
    store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: true,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.2,
        walking_speed: 0.3,
    });
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 456 }));
    store.apply_player_health(ProtocolPlayerHealth {
        health: 4.0,
        food: 7,
        saturation: 0.5,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.25,
        level: 3,
        total: 40,
    });
    assert!(store.set_local_selected_hotbar_slot(8));
    store.local_player.default_spawn = Some(crate::DefaultSpawnState {
        dimension: "minecraft:overworld".to_string(),
        pos: BlockPos { x: 8, y: 64, z: 8 },
        yaw: 0.0,
        pitch: 0.0,
    });
    store.local_player.simulation_distance = Some(12);
    store.local_player.sprinting = true;
    store
        .entities
        .with_equipment_mut(123, |equipment| {
            equipment.equipment.push(ProtocolEquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ProtocolItemStackSummary {
                    item_id: Some(1),
                    count: 1,
                    component_patch: Default::default(),
                },
            });
        })
        .unwrap();
    store
        .entities
        .with_leash_mut(123, |leash| leash.holder_id = Some(456))
        .unwrap();
    store
        .entities
        .with_supporting_block_mut(123, |support| {
            support.main = Some(BlockPos { x: 4, y: 63, z: -2 });
            support.on_ground_no_blocks = true;
        })
        .unwrap();
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 28,
    }));
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.0,
            y: 65.0,
            z: -4.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        },
        on_ground: true,
        horizontal_collision: true,
        minor_horizontal_collision: false,
        fall_distance: 8.0,
        sneaking: true,
        swimming: true,
        no_jump_delay_ticks: 0,
        y_rot: 90.0,
        x_rot: 20.0,
        last_teleport_id: 77,
    });
    store.set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
    store.set_local_using_item(true);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 123,
        values: vec![entity_byte_data(0, 0x02)],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: 0.1,
            modifiers: vec![ProtocolAttributeModifier {
                id: "minecraft:test_speed".to_string(),
                amount: 0.25,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            }],
        }],
    }));
    assert!(store.apply_update_mob_effect(ProtocolUpdateMobEffect {
        entity_id: 123,
        effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
        amplifier: 1,
        duration_ticks: 200,
        flags: MobEffectFlags::default(),
    }));
    assert_eq!(store.counters().active_mob_effects_tracked, 1);
    store
}

pub(super) fn respawn_packet(data_to_keep: i8) -> ProtocolRespawn {
    ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: 0,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        data_to_keep,
    }
}

pub(super) fn entity_byte_data(data_id: u8, value: i8) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    }
}

pub(super) fn local_player_entity(id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        ..protocol_add_entity(id)
    }
}

pub(super) fn stale_chunk() -> ChunkColumn {
    ChunkColumn {
        pos: ChunkPos { x: 1, z: -2 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    }
}

pub(super) fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}

pub(super) fn login_packet(dimension_type_id: i32, dimension: &str) -> ProtocolPlayLogin {
    ProtocolPlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec![dimension.to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id,
            dimension: dimension.to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    }
}

pub(super) fn record_custom_dimension_type_registry(
    store: &mut WorldStore,
    raw_data: Vec<u8>,
) -> Result<(), crate::RegistryValidationError> {
    store.record_registry_data(RegistryData {
        registry: "minecraft:dimension_type".to_string(),
        raw_payload_len: raw_data.len(),
        entries: vec![RegistryDataEntry {
            id: "example:custom".to_string(),
            raw_data: Some(raw_data),
        }],
    });
    store.begin_registry_configuration();
    store.finish_registry_configuration()
}

pub(super) fn dimension_type_nbt(
    cardinal_light: Option<&str>,
    has_sky_light: Option<bool>,
    has_ceiling: Option<bool>,
    shape: Option<(i32, i32, i32)>,
    skybox: Option<&str>,
    ambient_light: Option<f32>,
) -> Vec<u8> {
    let mut payload = vec![10];
    let (min_y, height, logical_height) = shape.unwrap_or((-64, 384, 384));
    for (name, value) in [
        ("min_y", min_y),
        ("height", height),
        ("logical_height", logical_height),
    ] {
        payload.push(3);
        write_nbt_string(&mut payload, name);
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload.push(1);
    write_nbt_string(&mut payload, "has_skylight");
    payload.push(u8::from(has_sky_light.unwrap_or(true)));
    payload.push(1);
    write_nbt_string(&mut payload, "has_ceiling");
    payload.push(u8::from(has_ceiling.unwrap_or(false)));
    payload.push(1);
    write_nbt_string(&mut payload, "has_ender_dragon_fight");
    payload.push(0);
    payload.push(6);
    write_nbt_string(&mut payload, "coordinate_scale");
    payload.extend_from_slice(&1.0_f64.to_bits().to_be_bytes());
    write_nbt_named_string(
        &mut payload,
        "infiniburn",
        "#minecraft:infiniburn_overworld",
    );
    payload.push(3);
    write_nbt_string(&mut payload, "monster_spawn_light_level");
    payload.extend_from_slice(&0_i32.to_be_bytes());
    payload.push(3);
    write_nbt_string(&mut payload, "monster_spawn_block_light_limit");
    payload.extend_from_slice(&0_i32.to_be_bytes());
    if let Some(cardinal_light) = cardinal_light {
        payload.push(8);
        write_nbt_string(&mut payload, "cardinal_light");
        write_nbt_string(&mut payload, cardinal_light);
    }
    if let Some(skybox) = skybox {
        payload.push(8);
        write_nbt_string(&mut payload, "skybox");
        write_nbt_string(&mut payload, skybox);
    }
    payload.push(5);
    write_nbt_string(&mut payload, "ambient_light");
    payload.extend_from_slice(&ambient_light.unwrap_or(0.0).to_bits().to_be_bytes());
    payload.push(0);
    payload
}

pub(super) fn dimension_type_nbt_with_environment_attributes(modified_fog_color: bool) -> Vec<u8> {
    let mut payload = dimension_type_nbt(
        None,
        Some(true),
        Some(false),
        Some((-64, 384, 384)),
        Some("overworld"),
        Some(0.0),
    );
    assert_eq!(payload.pop(), Some(0));
    payload.push(10);
    write_nbt_string(&mut payload, "attributes");
    if modified_fog_color {
        payload.push(10);
        write_nbt_string(&mut payload, "minecraft:visual/fog_color");
        payload.push(8);
        write_nbt_string(&mut payload, "modifier");
        write_nbt_string(&mut payload, "minecraft:override");
        payload.push(8);
        write_nbt_string(&mut payload, "argument");
        write_nbt_string(&mut payload, "#123456");
        payload.push(0);
    } else {
        write_nbt_named_string(&mut payload, "minecraft:visual/fog_color", "#123456");
    }
    write_nbt_named_string(&mut payload, "minecraft:visual/sky_color", "#345678");
    write_nbt_named_string(&mut payload, "minecraft:visual/water_fog_color", "#56789a");
    write_nbt_named_float(&mut payload, "minecraft:visual/fog_start_distance", 12.0);
    write_nbt_named_float(&mut payload, "minecraft:visual/fog_end_distance", 144.0);
    write_nbt_named_float(&mut payload, "minecraft:visual/sky_fog_end_distance", 96.0);
    write_nbt_named_float(
        &mut payload,
        "minecraft:visual/cloud_fog_end_distance",
        192.0,
    );
    write_nbt_named_float(
        &mut payload,
        "minecraft:visual/water_fog_start_distance",
        -4.0,
    );
    write_nbt_named_float(
        &mut payload,
        "minecraft:visual/water_fog_end_distance",
        48.0,
    );
    write_nbt_named_string(&mut payload, "minecraft:visual/block_light_tint", "#204060");
    write_nbt_named_string(&mut payload, "minecraft:visual/sky_light_color", "#406080");
    write_nbt_named_float(&mut payload, "minecraft:visual/sky_light_factor", 0.25);
    write_nbt_named_string(
        &mut payload,
        "minecraft:visual/night_vision_color",
        "#507090",
    );
    write_nbt_named_string(
        &mut payload,
        "minecraft:visual/ambient_light_color",
        "#305070",
    );
    payload.push(0);
    payload.push(0);
    payload
}

pub(super) fn dimension_type_nbt_with_network_scalars() -> Vec<u8> {
    let mut payload = vec![10];
    for (name, value) in [
        ("min_y", 0_i32),
        ("height", 256),
        ("logical_height", 256),
        ("monster_spawn_light_level", 7),
        ("monster_spawn_block_light_limit", 4),
    ] {
        payload.push(3);
        write_nbt_string(&mut payload, name);
        payload.extend_from_slice(&value.to_be_bytes());
    }
    for (name, value) in [
        ("has_skylight", false),
        ("has_ceiling", true),
        ("has_fixed_time", true),
        ("has_ender_dragon_fight", true),
    ] {
        payload.push(1);
        write_nbt_string(&mut payload, name);
        payload.push(u8::from(value));
    }
    payload.push(5);
    write_nbt_string(&mut payload, "ambient_light");
    payload.extend_from_slice(&0.1_f32.to_bits().to_be_bytes());
    payload.push(6);
    write_nbt_string(&mut payload, "coordinate_scale");
    payload.extend_from_slice(&8.0_f64.to_bits().to_be_bytes());
    write_nbt_named_string(&mut payload, "infiniburn", "#example:forever");
    payload.push(0);
    payload
}

pub(super) fn dimension_type_nbt_with_default_clock(default_clock: &str) -> Vec<u8> {
    let mut payload = dimension_type_nbt(None, None, None, None, None, None);
    assert_eq!(payload.pop(), Some(0));
    payload.push(8);
    write_nbt_string(&mut payload, "default_clock");
    write_nbt_string(&mut payload, default_clock);
    payload.push(0);
    payload
}

pub(super) fn dimension_type_nbt_with_timelines(timelines: &str) -> Vec<u8> {
    let mut payload = dimension_type_nbt(None, None, None, None, None, None);
    assert_eq!(payload.pop(), Some(0));
    write_nbt_named_string(&mut payload, "timelines", timelines);
    payload.push(0);
    payload
}

pub(super) fn write_nbt_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_nbt_string(out, name);
    write_nbt_string(out, value);
}

pub(super) fn write_nbt_named_float(out: &mut Vec<u8>, name: &str, value: f32) {
    out.push(5);
    write_nbt_string(out, name);
    out.extend_from_slice(&value.to_bits().to_be_bytes());
}

pub(super) fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(bytes);
}
