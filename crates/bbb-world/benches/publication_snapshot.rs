//! Reproducible cost baselines for immutable simulation world generations.
//!
//! The fixture dimensions mirror the default 12-chunk view distance
//! (`25 * 25 == 625` columns). Each representative column carries all 24
//! Overworld sections, packed block-state payloads, biome palettes, heightmaps,
//! and sky/block light arrays. Mutation cases retain the previous `Arc` while
//! mutating and publishing the next generation so COW work cannot disappear
//! from the measured end-to-end path.

use std::{collections::BTreeMap, hint::black_box, sync::Arc, time::Duration};

use bbb_protocol::packets::{
    AddEntity, AttributeSnapshot, BlockPos, BlockUpdate, CommonPlayerSpawnInfo,
    DataComponentPatchSummary, EntityDataValue, EntityDataValueKind, EntityEvent, EntityMove,
    EquipmentSlot, EquipmentSlotUpdate, ItemStackSummary, MobEffectFlags, PlayLogin, PlayTime,
    RegistryData, RegistryDataEntry, RemoveEntities, SetEntityData, SetEquipment, SetPassengers,
    SetPlayerInventory, UpdateAttributes, UpdateMobEffect, Vec3d,
};
use bbb_protocol::{
    attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    },
    entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID, VANILLA_ENTITY_TYPE_BLAZE_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        VANILLA_ENTITY_TYPE_ENDERMITE_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    },
    game_types::{VANILLA_GAME_TYPE_NOT_SET_ID, VANILLA_GAME_TYPE_SURVIVAL_ID},
    item_types::{VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID},
    mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID,
};
use bbb_world::{
    vanilla_entity_metadata_accessor_for_entity_type, vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type, BlockStateRegistry,
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, EntityClientMovementEffectState,
    HeightmapData, ItemEquipmentSlot, LightData, LocalPlayerInputState, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldStore,
};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

const CHUNK_COUNTS: [usize; 3] = [81, 289, 625];
const ENTITY_COUNTS: [usize; 3] = [100, 500, 2_000];
const ENTITY_COUNT: usize = 2_000;
const SPARSE_ENTITY_COUNT: usize = 8;
const CHURN_ENTITY_COUNT: usize = 16;
const SECTION_COUNT: usize = 24;
const LIGHT_SECTION_COUNT: usize = SECTION_COUNT + 2;

fn block_states() -> PalettedContainerData {
    PalettedContainerData {
        domain: PaletteDomain::BlockStates,
        bits_per_entry: 15,
        palette_kind: PaletteKind::Global,
        palette_global_ids: Vec::new(),
        packed_data: vec![0; 1_024],
        entry_count: 4_096,
    }
}

fn biomes() -> PalettedContainerData {
    PalettedContainerData {
        domain: PaletteDomain::Biomes,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![0],
        packed_data: Vec::new(),
        entry_count: 64,
    }
}

fn representative_chunk(pos: ChunkPos) -> ChunkColumn {
    ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps: vec![HeightmapData {
            kind_id: 1,
            data: vec![0; 37],
        }],
        sections: (0..SECTION_COUNT)
            .map(|_| ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: block_states(),
                biomes: biomes(),
            })
            .collect(),
        block_entities: Vec::new(),
        light: LightData {
            sky_y_mask: vec![(1_i64 << LIGHT_SECTION_COUNT) - 1],
            block_y_mask: vec![(1_i64 << LIGHT_SECTION_COUNT) - 1],
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: vec![vec![0xff; 2_048]; LIGHT_SECTION_COUNT],
            block_updates: vec![vec![0; 2_048]; LIGHT_SECTION_COUNT],
        },
    }
}

fn build_chunk_world(count: usize) -> WorldStore {
    let side = count.isqrt();
    assert_eq!(
        side * side,
        count,
        "representative chunk count must be square"
    );
    let offset = side / 2;
    let mut world = WorldStore::new();
    for z in 0..side {
        for x in 0..side {
            world.insert_decoded_chunk(representative_chunk(ChunkPos {
                x: x as i32 - offset as i32,
                z: z as i32 - offset as i32,
            }));
        }
    }
    assert_eq!(world.chunk_count(), count);
    world
}

fn entity_packet(id: i32, index: usize, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0000 + id as u128),
        entity_type_id,
        position: Vec3d {
            x: (index % 64) as f64,
            y: 64.0,
            z: (index / 64) as f64,
        },
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn build_entity_world_with(
    count: usize,
    mut entity_type_for_index: impl FnMut(usize) -> i32,
) -> WorldStore {
    let mut world = WorldStore::new();
    for index in 0..count {
        let id = index as i32 + 1;
        world.apply_add_entity(entity_packet(id, index, entity_type_for_index(index)));
    }
    assert_eq!(world.entity_count(), count);
    world
}

fn build_entity_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |_| VANILLA_ENTITY_TYPE_ZOMBIE_ID)
}

fn build_heterogeneous_entity_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |index| {
        VANILLA_ENTITY_TYPES[index % VANILLA_ENTITY_TYPES.len()].id
    })
}

fn build_block_effect_item_world(count: usize, block_name: Option<&str>) -> WorldStore {
    build_block_effect_item_world_with_chunks(count, block_name, 1)
}

fn build_block_effect_item_world_with_chunks(
    count: usize,
    block_name: Option<&str>,
    chunk_count: usize,
) -> WorldStore {
    let mut world = build_chunk_world(chunk_count);
    if let Some(block_name) = block_name {
        let block_state_id = BlockStateRegistry::vanilla_26_1()
            .iter()
            .find(|state| state.name == block_name)
            .unwrap_or_else(|| panic!("missing Vanilla block state {block_name}"))
            .id;
        assert!(world.apply_block_update(BlockUpdate {
            pos: BlockPos { x: 0, y: 64, z: 0 },
            block_state_id,
        }));
    }
    for index in 0..count {
        let id = index as i32 + 1;
        let mut packet = entity_packet(id, index, VANILLA_ENTITY_TYPE_ITEM_ID);
        packet.position = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        world.apply_add_entity(packet);
    }
    world
}

fn build_valid_fishing_hook_heterogeneous_world(count: usize) -> WorldStore {
    assert!(count >= 2);
    let mut world = WorldStore::new();
    world.apply_add_entity(entity_packet(1, 0, VANILLA_ENTITY_TYPE_PLAYER_ID));
    for index in 1..count - 1 {
        let id = index as i32 + 1;
        let entity_type_id = VANILLA_ENTITY_TYPES[index % VANILLA_ENTITY_TYPES.len()].id;
        world.apply_add_entity(entity_packet(id, index, entity_type_id));
    }
    let mut hook = entity_packet(
        count as i32,
        count - 1,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    );
    hook.data = 1;
    world.apply_add_entity(hook);
    assert_eq!(world.entity_count(), count);
    world
}

fn build_payload_heavy_entity_world(count: usize) -> WorldStore {
    let mut world = build_entity_world(count);
    let byte_accessors = [
        "DATA_SHARED_FLAGS_ID",
        "DATA_LIVING_ENTITY_FLAGS",
        "DATA_MOB_FLAGS_ID",
    ]
    .map(|java_name| {
        vanilla_entity_metadata_accessor_for_entity_type(VANILLA_ENTITY_TYPE_ZOMBIE_ID, java_name)
            .unwrap_or_else(|| panic!("missing generated Zombie accessor {java_name}"))
    });
    for id in 1..=count as i32 {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: byte_accessors
                .iter()
                .enumerate()
                .map(|(index, accessor)| EntityDataValue {
                    data_id: accessor.data_id,
                    serializer_id: accessor.serializer_id,
                    value: EntityDataValueKind::Byte(index as i8),
                })
                .collect(),
        }));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: id,
            slots: [
                EquipmentSlot::MainHand,
                EquipmentSlot::OffHand,
                EquipmentSlot::Feet,
                EquipmentSlot::Legs,
                EquipmentSlot::Chest,
                EquipmentSlot::Head,
            ]
            .into_iter()
            .enumerate()
            .map(|(slot_index, slot)| EquipmentSlotUpdate {
                slot,
                item: ItemStackSummary {
                    item_id: Some(slot_index as i32 + 1),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            })
            .collect(),
        }));
        assert!(world.apply_update_attributes(UpdateAttributes {
            entity_id: id,
            attributes: vec![AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 20.0 + f64::from(id % 5),
                modifiers: Vec::new(),
            }],
        }));
        for effect_offset in 0..3 {
            assert!(world.apply_update_mob_effect(UpdateMobEffect {
                entity_id: id,
                effect_id: VANILLA_MOB_EFFECT_SPEED_ID + effect_offset,
                amplifier: effect_offset,
                duration_ticks: 200,
                flags: MobEffectFlags::default(),
            }));
        }
    }
    world
}

fn entity_move(id: i32, delta: i16) -> EntityMove {
    EntityMove {
        id,
        has_position: true,
        delta_x: delta,
        delta_y: 0,
        delta_z: -delta,
        y_rot: Some(f32::from(delta)),
        x_rot: Some(f32::from(-delta)),
        on_ground: true,
    }
}

fn build_passenger_interpolation_world(count: usize) -> WorldStore {
    assert_eq!(count % 2, 0);
    let mut world = build_entity_world_with(count, |index| {
        if index % 2 == 0 {
            VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID
        } else {
            VANILLA_ENTITY_TYPE_ZOMBIE_ID
        }
    });
    for pair in 0..count / 2 {
        let vehicle_id = (pair * 2 + 1) as i32;
        let passenger_id = vehicle_id + 1;
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id,
            passenger_ids: vec![passenger_id],
        }));
    }
    for id in 1..=count as i32 {
        assert!(world.apply_entity_move(entity_move(id, 128)));
    }
    world
}

fn build_local_boat_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 99,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_owned()],
        max_players: 20,
        chunk_radius: 12,
        simulation_distance: 12,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_owned(),
            seed: 0,
            game_type: i8::try_from(VANILLA_GAME_TYPE_SURVIVAL_ID)
                .expect("survival game type fits the PlayLogin field"),
            previous_game_type: i8::try_from(VANILLA_GAME_TYPE_NOT_SET_ID)
                .expect("unset game type fits the PlayLogin field"),
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    });
    world.apply_add_entity(entity_packet(1, 0, VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID));
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("generated Entity metadata accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 1,
        values: vec![EntityDataValue {
            data_id: entity_accessors.no_gravity_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 1,
        passenger_ids: vec![99],
    }));
    world
}

fn build_local_happy_ghast_world() -> WorldStore {
    let mut world = build_local_boat_world();
    assert_eq!(
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![1],
        }),
        1
    );
    world.apply_add_entity(entity_packet(2, 0, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 2,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 2,
        passenger_ids: vec![99],
    }));
    world.set_local_player_pose(Default::default());
    world
}

fn build_local_horse_world() -> WorldStore {
    let mut world = build_local_boat_world();
    assert_eq!(
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![1],
        }),
        1
    );
    const SADDLE_ITEM_ID: i32 = 1;
    world.set_default_item_equipment_slots(BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    world.apply_add_entity(entity_packet(2, 0, VANILLA_ENTITY_TYPE_HORSE_ID));
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("generated Entity metadata accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 2,
        values: vec![EntityDataValue {
            data_id: entity_accessors.no_gravity_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 2,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 2,
        passenger_ids: vec![99],
    }));
    world.set_local_player_pose(Default::default());
    world
}

fn build_local_camel_world() -> WorldStore {
    let mut world = build_local_horse_world();
    assert_eq!(
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![2],
        }),
        1
    );
    world.apply_world_time(PlayTime {
        game_time: 100,
        clock_updates: Vec::new(),
    });
    world.apply_add_entity(entity_packet(2, 0, VANILLA_ENTITY_TYPE_CAMEL_ID));
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("generated Entity metadata accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 2,
        values: vec![EntityDataValue {
            data_id: entity_accessors.no_gravity_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 2,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    assert!(world.apply_entity_move(EntityMove {
        id: 2,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 2,
        passenger_ids: vec![99],
    }));
    world.set_local_player_pose(Default::default());
    world
}

fn build_local_nautilus_world() -> WorldStore {
    let mut world = build_local_horse_world();
    assert_eq!(
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![2],
        }),
        1
    );
    world.apply_add_entity(entity_packet(2, 0, VANILLA_ENTITY_TYPE_NAUTILUS_ID));
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("generated Entity metadata accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 2,
        values: vec![EntityDataValue {
            data_id: entity_accessors.no_gravity_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 2,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 2,
        passenger_ids: vec![99],
    }));
    world.set_local_player_pose(Default::default());
    world
}

fn build_local_item_based_steering_world(entity_type_id: i32, steering_item_id: i32) -> WorldStore {
    let mut world = build_local_horse_world();
    assert_eq!(
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![2],
        }),
        1
    );
    world.apply_add_entity(entity_packet(2, 0, entity_type_id));
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("generated Entity metadata accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 2,
        values: vec![EntityDataValue {
            data_id: entity_accessors.no_gravity_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 2,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    assert!(world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(steering_item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        },
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 2,
        passenger_ids: vec![99],
    }));
    world.set_local_player_pose(Default::default());
    world
}

fn build_primed_tnt_minecart_world(count: usize) -> WorldStore {
    let mut world = build_entity_world_with(count, |_| VANILLA_ENTITY_TYPE_TNT_MINECART_ID);
    for id in 1..=i32::try_from(count).expect("benchmark count fits i32") {
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: id,
            event_id: 10,
        }));
    }
    world
}

fn build_fueled_furnace_minecart_world(count: usize) -> WorldStore {
    let mut world = build_entity_world_with(count, |_| VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID);
    let (data_id, default) =
        vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
        )
        .expect("generated MinecartFurnace fuel accessor");
    assert!(!default);
    for id in 1..=i32::try_from(count).expect("benchmark count fits i32") {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
    }
    world
}

fn build_blaze_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |_| VANILLA_ENTITY_TYPE_BLAZE_ID)
}

fn build_ender_ambient_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |index| {
        if index % 2 == 0 {
            VANILLA_ENTITY_TYPE_ENDERMAN_ID
        } else {
            VANILLA_ENTITY_TYPE_ENDERMITE_ID
        }
    })
}

fn build_guardian_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |_| VANILLA_ENTITY_TYPE_GUARDIAN_ID)
}

fn build_wet_moving_guardian_world(count: usize) -> WorldStore {
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    let mut world = build_chunk_world(1);
    for index in 0..count {
        let id = index as i32 + 1;
        let mut packet = entity_packet(id, index, VANILLA_ENTITY_TYPE_GUARDIAN_ID);
        packet.position = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        world.apply_add_entity(packet);
    }
    assert!(world.apply_block_update(BlockUpdate {
        pos: BlockPos { x: 0, y: 64, z: 0 },
        block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
    }));
    let moving_accessor = vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        "DATA_ID_MOVING",
    )
    .expect("generated Guardian moving accessor");
    for id in 1..=i32::try_from(count).expect("benchmark count fits i32") {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id: moving_accessor.data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
    }
    world
}

fn build_guardian_attack_world(count: usize) -> WorldStore {
    let mut world = build_guardian_world(count);
    let target_id = i32::try_from(count).expect("benchmark count fits i32") + 1;
    let mut target = entity_packet(target_id, count, VANILLA_ENTITY_TYPE_COW_ID);
    target.position.x = 16.0;
    world.apply_add_entity(target);
    let target_accessor = vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        "DATA_ID_ATTACK_TARGET",
    )
    .expect("generated Guardian attack-target accessor");
    for id in 1..=i32::try_from(count).expect("benchmark count fits i32") {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id: target_accessor.data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(target_id),
            }],
        }));
    }
    world
}

fn build_session_static_world() -> WorldStore {
    let mut world = WorldStore::new();
    for registry_index in 0..32 {
        world.record_registry_data(RegistryData {
            registry: format!("benchmark:registry_{registry_index}"),
            entries: (0..256)
                .map(|entry_index| RegistryDataEntry {
                    id: format!("benchmark:entry_{registry_index}_{entry_index}"),
                    raw_data: None,
                })
                .collect(),
            raw_payload_len: 0,
        });
    }
    world.set_default_item_max_stack_sizes((0..2_000).map(|item_id| (item_id, 64)).collect());
    world.set_default_item_max_damage((0..2_000).map(|item_id| (item_id, item_id + 1)).collect());
    world.set_default_item_crafting_remainders(
        (0..2_000)
            .map(|item_id| (item_id, (item_id + 1) % 2_000))
            .collect::<BTreeMap<_, _>>(),
    );
    world
}

fn benchmark_configuration() -> Criterion {
    Criterion::default()
        .without_plots()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
}

fn bench_unchanged_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/unchanged");
    let empty = WorldStore::new();
    group.bench_function("empty", |b| {
        b.iter(|| black_box(Arc::new(empty.publication_snapshot())));
    });
    for count in CHUNK_COUNTS {
        let world = build_chunk_world(count);
        group.bench_with_input(BenchmarkId::new("chunks", count), &count, |b, _| {
            b.iter(|| black_box(Arc::new(world.publication_snapshot())));
        });
    }
    for count in ENTITY_COUNTS {
        let entities = build_entity_world(count);
        group.bench_with_input(BenchmarkId::new("entities", count), &count, |b, _| {
            b.iter(|| black_box(Arc::new(entities.publication_snapshot())));
        });
    }
    let session_static = build_session_static_world();
    group.bench_function("session_static/8192_registry_entries_2000_items", |b| {
        b.iter(|| black_box(Arc::new(session_static.publication_snapshot())));
    });
    group.finish();
}

fn bench_reused_world_arc(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/reused_exact_world_arc");
    let empty = Arc::new(WorldStore::new().publication_snapshot());
    group.bench_function("empty", |b| {
        b.iter(|| black_box(Arc::clone(black_box(&empty))));
    });
    let entities = Arc::new(build_entity_world(ENTITY_COUNT).publication_snapshot());
    group.bench_function("entities/2000", |b| {
        b.iter(|| black_box(Arc::clone(black_box(&entities))));
    });
    group.finish();
}

fn bench_entity_tick_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/tick_only");
    let mut empty = WorldStore::new();
    group.bench_function("empty", |b| {
        b.iter(|| empty.advance_entity_client_animations(black_box(1)));
    });
    for count in ENTITY_COUNTS {
        let mut world = build_entity_world(count);
        group.bench_with_input(BenchmarkId::new("zombies", count), &count, |b, _| {
            b.iter(|| world.advance_entity_client_animations(black_box(1)));
        });
    }
    let mut heterogeneous = build_heterogeneous_entity_world(ENTITY_COUNT);
    group.bench_function("heterogeneous/2000", |b| {
        b.iter(|| {
            heterogeneous.advance_entity_client_animations(black_box(1));
            black_box(
                heterogeneous
                    .drain_entity_client_movement_effect_states()
                    .count(),
            );
        });
    });
    let mut valid_fishing_hook = build_valid_fishing_hook_heterogeneous_world(ENTITY_COUNT);
    group.bench_function("valid_fishing_hook_heterogeneous/2000", |b| {
        b.iter(|| {
            valid_fishing_hook.advance_entity_client_animations(black_box(1));
            black_box(
                valid_fishing_hook
                    .drain_entity_client_movement_effect_states()
                    .count(),
            );
        });
    });
    group.finish();
}

fn bench_entity_tick_phase_observation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/phase_observation");
    for (name, mut world) in [
        ("zombies/2000", build_entity_world(ENTITY_COUNT)),
        (
            "heterogeneous/2000",
            build_heterogeneous_entity_world(ENTITY_COUNT),
        ),
        (
            "valid_fishing_hook_heterogeneous/2000",
            build_valid_fishing_hook_heterogeneous_world(ENTITY_COUNT),
        ),
    ] {
        let mut calls = 0_u128;
        let mut aggregate = bbb_world::EntityClientTickWork::default();
        group.bench_function(name, |b| {
            b.iter(|| {
                let work = world.advance_entity_client_animations_with_observation(black_box(1));
                black_box(world.drain_entity_client_movement_effect_states().count());
                calls = calls.saturating_add(1);
                aggregate.preparation_duration += work.preparation_duration;
                aggregate.schedule_duration += work.schedule_duration;
                aggregate.projectile_duration += work.projectile_duration;
                aggregate.interpolation_duration += work.interpolation_duration;
                aggregate.block_effect_duration += work.block_effect_duration;
                aggregate.block_effect_entities = aggregate
                    .block_effect_entities
                    .saturating_add(work.block_effect_entities);
                aggregate.block_effect_blocks_visited = aggregate
                    .block_effect_blocks_visited
                    .saturating_add(work.block_effect_blocks_visited);
                aggregate.block_effect_callbacks_applied = aggregate
                    .block_effect_callbacks_applied
                    .saturating_add(work.block_effect_callbacks_applied);
                aggregate.animation_duration += work.animation_duration;
                black_box(work);
            });
        });
        let average_nanos = |duration: Duration| {
            if calls == 0 {
                0
            } else {
                duration.as_nanos() / calls
            }
        };
        if calls > 0 {
            eprintln!(
                "entity tick phase average {name}: preparation={}ns schedule={}ns projectile={}ns interpolation={}ns block_effect={}ns block_effect_entities={} block_effect_blocks={} block_effect_callbacks={} animation={}ns samples={calls}",
                average_nanos(aggregate.preparation_duration),
                average_nanos(aggregate.schedule_duration),
                average_nanos(aggregate.projectile_duration),
                average_nanos(aggregate.interpolation_duration),
                average_nanos(aggregate.block_effect_duration),
                aggregate.block_effect_entities / usize::try_from(calls).unwrap_or(usize::MAX),
                aggregate.block_effect_blocks_visited
                    / usize::try_from(calls).unwrap_or(usize::MAX),
                aggregate.block_effect_callbacks_applied
                    / usize::try_from(calls).unwrap_or(usize::MAX),
                average_nanos(aggregate.animation_duration),
            );
        }
    }
    group.finish();
}

fn bench_entity_block_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/block_effects");
    for count in ENTITY_COUNTS {
        for (terrain, block_name) in [("loaded_air", None), ("cobweb", Some("minecraft:cobweb"))] {
            let fixture = build_block_effect_item_world(count, block_name);
            group.bench_with_input(BenchmarkId::new(terrain, count), &count, |b, _| {
                b.iter_batched(
                    || fixture.publication_snapshot(),
                    |mut world| {
                        black_box(world.advance_entity_client_animations_with_observation(1));
                    },
                    BatchSize::SmallInput,
                );
            });
        }
    }

    let fixture = build_block_effect_item_world(ENTITY_COUNT, Some("minecraft:cobweb"));
    group.bench_function("cobweb_then_publish_retained/2000", |b| {
        b.iter_batched(
            || {
                let world = fixture.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                black_box(world.advance_entity_client_animations_with_observation(1));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    for (terrain, block_name) in [
        ("loaded_air_625_columns", None),
        ("cobweb_625_columns", Some("minecraft:cobweb")),
    ] {
        let fixture = build_block_effect_item_world_with_chunks(ENTITY_COUNT, block_name, 625);
        group.bench_function(format!("{terrain}/2000"), |b| {
            b.iter_batched(
                || fixture.publication_snapshot(),
                |mut world| {
                    black_box(world.advance_entity_client_animations_with_observation(1));
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_entity_tick_schema_lookups(c: &mut Criterion) {
    const CLIENT_TICK_ACCESSOR_NAMES: [&str; 17] = [
        "DATA_BABY_ID",
        "DATA_ID_FLAGS",
        "DASH",
        "COPPER_GOLEM_STATE",
        "IS_CHARGING_CROSSBOW",
        "DATA_IS_CHARGING_CROSSBOW",
        "DATA_LIVING_ENTITY_FLAGS",
        "DATA_DANCING",
        "DATA_PLAYING_DEAD",
        "IS_TEARING_DOWN",
        "DATA_ANGER_LEVEL",
        "DATA_INTERESTED_ID",
        "IS_LYING",
        "RELAX_STATE_ONE",
        "DATA_ID_PADDLE_LEFT",
        "DATA_ID_BUBBLE_TIME",
        "DATA_ID_FIREWORKS_ITEM",
    ];

    let mut group = c.benchmark_group("entity_tick/schema_accessor_lookup");
    let zombie_types = vec![VANILLA_ENTITY_TYPE_ZOMBIE_ID; ENTITY_COUNT];
    group.bench_function("17_accessors_2000_zombies", |b| {
        b.iter(|| {
            for entity_type_id in &zombie_types {
                for java_name in CLIENT_TICK_ACCESSOR_NAMES {
                    black_box(vanilla_entity_metadata_accessor_for_entity_type(
                        black_box(*entity_type_id),
                        black_box(java_name),
                    ));
                }
            }
        });
    });
    let heterogeneous_types = (0..ENTITY_COUNT)
        .map(|index| VANILLA_ENTITY_TYPES[index % VANILLA_ENTITY_TYPES.len()].id)
        .collect::<Vec<_>>();
    group.bench_function("17_accessors_2000_heterogeneous", |b| {
        b.iter(|| {
            for entity_type_id in &heterogeneous_types {
                for java_name in CLIENT_TICK_ACCESSOR_NAMES {
                    black_box(vanilla_entity_metadata_accessor_for_entity_type(
                        black_box(*entity_type_id),
                        black_box(java_name),
                    ));
                }
            }
        });
    });
    group.finish();
}

fn bench_entity_tick_and_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/tick_and_publish");
    let mut empty = WorldStore::new();
    let mut empty_held = Arc::new(empty.publication_snapshot());
    group.bench_function("empty", |b| {
        b.iter(|| {
            empty.advance_entity_client_animations(black_box(1));
            empty_held = black_box(Arc::new(empty.publication_snapshot()));
        });
    });
    black_box(empty_held);
    for count in ENTITY_COUNTS {
        let mut world = build_entity_world(count);
        let mut held = Arc::new(world.publication_snapshot());
        group.bench_with_input(BenchmarkId::new("zombies", count), &count, |b, _| {
            b.iter(|| {
                world.advance_entity_client_animations(black_box(1));
                held = black_box(Arc::new(world.publication_snapshot()));
            });
        });
        black_box(held);
    }
    let mut heterogeneous = build_heterogeneous_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(heterogeneous.publication_snapshot());
    group.bench_function("heterogeneous/2000", |b| {
        b.iter(|| {
            heterogeneous.advance_entity_client_animations(black_box(1));
            black_box(
                heterogeneous
                    .drain_entity_client_movement_effect_states()
                    .count(),
            );
            held = black_box(Arc::new(heterogeneous.publication_snapshot()));
        });
    });
    black_box(held);
    let mut valid_fishing_hook = build_valid_fishing_hook_heterogeneous_world(ENTITY_COUNT);
    let mut valid_fishing_hook_held = Arc::new(valid_fishing_hook.publication_snapshot());
    group.bench_function("valid_fishing_hook_heterogeneous/2000", |b| {
        b.iter(|| {
            valid_fishing_hook.advance_entity_client_animations(black_box(1));
            black_box(
                valid_fishing_hook
                    .drain_entity_client_movement_effect_states()
                    .count(),
            );
            valid_fishing_hook_held =
                black_box(Arc::new(valid_fishing_hook.publication_snapshot()));
        });
    });
    black_box(valid_fishing_hook_held);
    group.finish();
}

fn bench_entity_retained_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_publication/retained_snapshot");
    let world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("next_generation/2000_entities", |b| {
        b.iter(|| {
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_payload_heavy_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_publication/payload_heavy");
    let mut world = build_payload_heavy_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("unchanged/2000", |b| {
        b.iter(|| {
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    group.bench_function("sparse_metadata_8_of_2000_then_publish", |b| {
        let mut value = 1_i8;
        b.iter(|| {
            value = -value;
            for id in 1..=SPARSE_ENTITY_COUNT as i32 {
                assert!(world.apply_set_entity_data(SetEntityData {
                    id,
                    values: vec![EntityDataValue {
                        data_id: 0,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                        value: EntityDataValueKind::Byte(value),
                    }],
                }));
            }
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_sparse_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/sparse");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    let mut delta = 1_i16;
    group.bench_function("transform_metadata_8_of_2000_then_publish", |b| {
        b.iter(|| {
            delta = -delta;
            for id in 1..=SPARSE_ENTITY_COUNT as i32 {
                assert!(world.apply_entity_move(entity_move(id, delta)));
                assert!(world.apply_set_entity_data(SetEntityData {
                    id,
                    values: vec![EntityDataValue {
                        data_id: 0,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                        value: EntityDataValueKind::Byte(delta as i8),
                    }],
                }));
            }
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_mutation_families(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/families");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    let mut value = 1_i8;
    group.bench_function(
        "metadata_equipment_effect_attribute_event_then_publish",
        |b| {
            b.iter(|| {
                value = -value;
                assert!(world.apply_set_entity_data(SetEntityData {
                    id: 1,
                    values: vec![EntityDataValue {
                        data_id: 0,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                        value: EntityDataValueKind::Byte(value),
                    }],
                }));
                assert!(world.apply_set_equipment(SetEquipment {
                    entity_id: 2,
                    slots: vec![EquipmentSlotUpdate {
                        slot: EquipmentSlot::MainHand,
                        item: if value > 0 {
                            ItemStackSummary {
                                item_id: Some(1),
                                count: 1,
                                component_patch: DataComponentPatchSummary::default(),
                            }
                        } else {
                            ItemStackSummary::empty()
                        },
                    }],
                }));
                assert!(world.apply_update_mob_effect(UpdateMobEffect {
                    entity_id: 3,
                    effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
                    amplifier: i32::from(value > 0),
                    duration_ticks: 200,
                    flags: MobEffectFlags::default(),
                }));
                assert!(world.apply_update_attributes(UpdateAttributes {
                    entity_id: 4,
                    attributes: vec![AttributeSnapshot {
                        attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                        base: 20.0 + f64::from(value),
                        modifiers: Vec::new(),
                    }],
                }));
                assert!(world.apply_entity_event(EntityEvent {
                    entity_id: 5,
                    event_id: 2,
                }));
                held = black_box(Arc::new(world.publication_snapshot()));
            });
        },
    );
    black_box(held);
    group.finish();
}

fn bench_entity_spawn_remove_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/spawn_remove_churn");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    let churn_ids =
        ((ENTITY_COUNT - CHURN_ENTITY_COUNT + 1) as i32..=ENTITY_COUNT as i32).collect::<Vec<_>>();
    group.bench_function("16_of_2000_then_publish", |b| {
        b.iter(|| {
            assert_eq!(
                world.apply_remove_entities(RemoveEntities {
                    entity_ids: churn_ids.clone(),
                }),
                CHURN_ENTITY_COUNT
            );
            for &id in &churn_ids {
                world.apply_add_entity(entity_packet(
                    id,
                    usize::try_from(id - 1).unwrap(),
                    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                ));
            }
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_interpolation_and_passengers(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/interpolation_passengers");
    let mut world = build_passenger_interpolation_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("1000_vehicles_1000_passengers_then_publish", |b| {
        b.iter(|| {
            world.advance_entity_client_animations(black_box(1));
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_local_boat_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/local_boat");
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        right: true,
        ..LocalPlayerInputState::default()
    };
    let mut physics = build_local_boat_world();
    group.bench_function("physics_only", |b| {
        b.iter(|| {
            black_box(
                physics
                    .advance_local_boat_vehicle_input(black_box(input), black_box(1))
                    .expect("local boat"),
            );
            black_box(submit_entity_client_movement_effects(&mut physics));
        });
    });
    let mut publish = build_local_boat_world();
    let mut held = Arc::new(publish.publication_snapshot());
    group.bench_function("physics_then_publish_retained", |b| {
        b.iter(|| {
            black_box(
                publish
                    .advance_local_boat_vehicle_input(black_box(input), black_box(1))
                    .expect("local boat"),
            );
            black_box(submit_entity_client_movement_effects(&mut publish));
            held = black_box(Arc::new(publish.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_local_happy_ghast_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/local_happy_ghast");
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        right: true,
        ..LocalPlayerInputState::default()
    };
    let mut physics = build_local_happy_ghast_world();
    group.bench_function("physics_only", |b| {
        b.iter(|| {
            black_box(
                physics
                    .advance_local_happy_ghast_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled Happy Ghast"),
            );
        });
    });
    let mut publish = build_local_happy_ghast_world();
    let mut held = Arc::new(publish.publication_snapshot());
    group.bench_function("physics_then_publish_retained", |b| {
        b.iter(|| {
            black_box(
                publish
                    .advance_local_happy_ghast_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled Happy Ghast"),
            );
            held = black_box(Arc::new(publish.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_local_horse_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/local_horse");
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        right: true,
        ..LocalPlayerInputState::default()
    };
    let mut physics = build_local_horse_world();
    group.bench_function("physics_only", |b| {
        b.iter(|| {
            black_box(
                physics
                    .advance_local_abstract_horse_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled AbstractHorse"),
            );
        });
    });
    let mut publish = build_local_horse_world();
    let mut held = Arc::new(publish.publication_snapshot());
    group.bench_function("physics_then_publish_retained", |b| {
        b.iter(|| {
            black_box(
                publish
                    .advance_local_abstract_horse_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled AbstractHorse"),
            );
            held = black_box(Arc::new(publish.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_local_camel_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/local_camel");
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        right: true,
        ..LocalPlayerInputState::default()
    };
    let mut physics = build_local_camel_world();
    group.bench_function("physics_only", |b| {
        b.iter(|| {
            black_box(
                physics
                    .advance_local_abstract_horse_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled Camel"),
            );
        });
    });
    let mut publish = build_local_camel_world();
    let mut held = Arc::new(publish.publication_snapshot());
    group.bench_function("physics_then_publish_retained", |b| {
        b.iter(|| {
            black_box(
                publish
                    .advance_local_abstract_horse_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled Camel"),
            );
            held = black_box(Arc::new(publish.publication_snapshot()));
        });
    });
    black_box(held);
    group.bench_function("dash_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = build_local_camel_world();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                assert!(world.apply_local_player_riding_jump(90));
                black_box(
                    world
                        .advance_local_abstract_horse_vehicle_input(black_box(input), black_box(1))
                        .expect("locally controlled Camel dash"),
                );
                let next = black_box(Arc::new(world.publication_snapshot()));
                black_box((held, next));
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_local_nautilus_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/local_nautilus");
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        right: true,
        ..LocalPlayerInputState::default()
    };
    let mut physics = build_local_nautilus_world();
    group.bench_function("physics_only", |b| {
        b.iter(|| {
            black_box(
                physics
                    .advance_local_nautilus_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled AbstractNautilus"),
            );
        });
    });
    let mut publish = build_local_nautilus_world();
    let mut held = Arc::new(publish.publication_snapshot());
    group.bench_function("physics_then_publish_retained", |b| {
        b.iter(|| {
            black_box(
                publish
                    .advance_local_nautilus_vehicle_input(black_box(input), black_box(1))
                    .expect("locally controlled AbstractNautilus"),
            );
            held = black_box(Arc::new(publish.publication_snapshot()));
        });
    });
    black_box(held);
    group.bench_function("dash_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = build_local_nautilus_world();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                assert!(world.apply_local_player_riding_jump(90));
                black_box(
                    world
                        .advance_local_nautilus_vehicle_input(black_box(input), black_box(1))
                        .expect("locally controlled AbstractNautilus dash"),
                );
                let next = black_box(Arc::new(world.publication_snapshot()));
                black_box((held, next));
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_local_item_based_steering_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/local_item_based_steering");
    for (name, entity_type_id, steering_item_id) in [
        (
            "pig",
            VANILLA_ENTITY_TYPE_PIG_ID,
            VANILLA_ITEM_CARROT_ON_A_STICK_ID,
        ),
        (
            "strider",
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
        ),
    ] {
        let mut physics = build_local_item_based_steering_world(entity_type_id, steering_item_id);
        group.bench_function(format!("{name}_physics_only"), |b| {
            b.iter(|| {
                black_box(
                    physics
                        .advance_local_item_based_steering_vehicle_input(black_box(1))
                        .expect("locally controlled item-steered mount"),
                );
            });
        });
        let mut publish = build_local_item_based_steering_world(entity_type_id, steering_item_id);
        let mut held = Arc::new(publish.publication_snapshot());
        group.bench_function(format!("{name}_physics_then_publish_retained"), |b| {
            b.iter(|| {
                black_box(
                    publish
                        .advance_local_item_based_steering_vehicle_input(black_box(1))
                        .expect("locally controlled item-steered mount"),
                );
                held = black_box(Arc::new(publish.publication_snapshot()));
            });
        });
        black_box(held);
    }

    group.bench_function("pig_boost_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let mut world = build_local_item_based_steering_world(
                    VANILLA_ENTITY_TYPE_PIG_ID,
                    VANILLA_ITEM_CARROT_ON_A_STICK_ID,
                );
                let boost = vanilla_entity_metadata_accessor_for_entity_type(
                    VANILLA_ENTITY_TYPE_PIG_ID,
                    "DATA_BOOST_TIME",
                )
                .expect("generated Pig boost-time accessor");
                assert!(world.apply_set_entity_data(SetEntityData {
                    id: 2,
                    values: vec![EntityDataValue {
                        data_id: boost.data_id,
                        serializer_id: boost.serializer_id,
                        value: EntityDataValueKind::Int(140),
                    }],
                }));
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                black_box(
                    world
                        .advance_local_item_based_steering_vehicle_input(black_box(1))
                        .expect("boosted local Pig"),
                );
                let next = black_box(Arc::new(world.publication_snapshot()));
                black_box((held, next));
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn submit_entity_client_movement_effects(world: &mut WorldStore) -> usize {
    let mut effects = world.take_entity_client_movement_effect_states();
    let mut submitted = 0;
    for effect in effects.drain(..) {
        match effect {
            EntityClientMovementEffectState::Particle(_) => submitted += 1,
            EntityClientMovementEffectState::Sound(state) => {
                submitted += usize::from(
                    world
                        .record_entity_client_movement_sound_if_audible(state)
                        .is_some(),
                );
            }
        }
    }
    world.recycle_entity_client_movement_effect_states(effects);
    submitted
}

fn bench_entity_effect_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/effect_generation");
    let mut world =
        build_entity_world_with(ENTITY_COUNT, |_| VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID);
    group.bench_function("2000_firework_trails", |b| {
        b.iter(|| {
            world.advance_entity_client_animations(black_box(1));
            black_box(world.drain_entity_client_movement_effect_states().count());
        });
    });
    let primed_tnt_minecarts = build_primed_tnt_minecart_world(ENTITY_COUNT);
    group.bench_function("2000_tnt_minecart_fuses", |b| {
        b.iter_batched(
            || primed_tnt_minecarts.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_tnt_minecart_fuses_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = primed_tnt_minecarts.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    let fueled_furnace_minecarts = build_fueled_furnace_minecart_world(ENTITY_COUNT);
    group.bench_function("2000_fueled_furnace_minecarts", |b| {
        b.iter_batched(
            || fueled_furnace_minecarts.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_fueled_furnace_minecarts_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = fueled_furnace_minecarts.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    let blazes = build_blaze_world(ENTITY_COUNT);
    group.bench_function("2000_blazes_tick_only", |b| {
        b.iter_batched(
            || blazes.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(world);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_blazes", |b| {
        b.iter_batched(
            || blazes.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_blazes_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = blazes.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    let ender_ambient = build_ender_ambient_world(ENTITY_COUNT);
    group.bench_function("2000_ender_ambient", |b| {
        b.iter_batched(
            || ender_ambient.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_ender_ambient_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = ender_ambient.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    let guardians = build_guardian_world(ENTITY_COUNT);
    group.bench_function("2000_guardians_dry", |b| {
        b.iter_batched(
            || guardians.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_guardians_dry_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = guardians.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    let wet_moving_guardians = build_wet_moving_guardian_world(ENTITY_COUNT);
    group.bench_function("2000_guardians_wet_moving", |b| {
        b.iter_batched(
            || wet_moving_guardians.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    let guardian_attack = build_guardian_attack_world(ENTITY_COUNT);
    group.bench_function("2000_guardians_active_target", |b| {
        b.iter_batched(
            || guardian_attack.publication_snapshot(),
            |mut world| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("2000_guardians_active_target_then_publish_retained", |b| {
        b.iter_batched(
            || {
                let world = guardian_attack.publication_snapshot();
                let held = Arc::new(world.publication_snapshot());
                (world, held)
            },
            |(mut world, held)| {
                world.advance_entity_client_animations(black_box(1));
                black_box(submit_entity_client_movement_effects(&mut world));
                black_box(Arc::new(world.publication_snapshot()));
                black_box(held);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_entity_projection(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_projection");
    let world = build_heterogeneous_entity_world(ENTITY_COUNT);
    group.bench_function("full_entity_state/2000", |b| {
        b.iter(|| {
            for id in 1..=ENTITY_COUNT as i32 {
                black_box(world.probe_entity(black_box(id)));
            }
        });
    });
    group.bench_function("narrow_transforms/2000", |b| {
        b.iter(|| black_box(world.entity_transforms()));
    });
    group.bench_function("protocol_lookup_narrow_transform/2000", |b| {
        b.iter(|| {
            for id in 1..=ENTITY_COUNT as i32 {
                black_box(world.probe_entity_transform(black_box(id)));
            }
        });
    });
    group.bench_function("renderer_model_sources/2000", |b| {
        b.iter(|| black_box(world.entity_model_sources_at_partial_tick(black_box(0.5))));
    });
    group.finish();
}

fn bench_entity_protocol_lookup_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/protocol_lookup");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut delta = 1_i16;
    group.bench_function("1000_transform_updates_of_2000", |b| {
        b.iter(|| {
            delta = -delta;
            for id in 1..=1_000 {
                assert!(world.apply_entity_move(entity_move(black_box(id), delta)));
            }
        });
    });
    group.finish();
}

fn bench_mutation_then_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/mutation_then_publish");

    let mut block_world = build_chunk_world(625);
    let mut held = Arc::new(block_world.publication_snapshot());
    let mut block_state_id = 1;
    group.bench_function("single_block/625_chunks", |b| {
        b.iter(|| {
            block_state_id = if block_state_id == 0 { 1 } else { 0 };
            assert!(block_world.apply_block_update(BlockUpdate {
                pos: BlockPos { x: 0, y: 0, z: 0 },
                block_state_id,
            }));
            let next = Arc::new(block_world.publication_snapshot());
            held = black_box(next);
        });
    });
    black_box(held);

    group.finish();
}

fn bench_retained_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/retained_snapshot");
    let world = build_chunk_world(625);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("next_generation/625_chunks", |b| {
        b.iter(|| {
            let next = Arc::new(world.publication_snapshot());
            held = black_box(next);
        });
    });
    black_box(held);
    group.finish();
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets =
        bench_unchanged_publication,
        bench_reused_world_arc,
        bench_entity_tick_only,
        bench_entity_tick_phase_observation,
        bench_entity_block_effects,
        bench_entity_tick_schema_lookups,
        bench_entity_tick_and_publication,
        bench_entity_retained_snapshot,
        bench_entity_payload_heavy_publication,
        bench_entity_sparse_mutation,
        bench_entity_mutation_families,
        bench_entity_spawn_remove_churn,
        bench_entity_interpolation_and_passengers,
        bench_local_boat_physics,
        bench_local_horse_physics,
        bench_local_camel_physics,
        bench_local_nautilus_physics,
        bench_local_item_based_steering_physics,
        bench_local_happy_ghast_physics,
        bench_entity_effect_generation,
        bench_entity_projection,
        bench_entity_protocol_lookup_mutation,
        bench_mutation_then_publication,
        bench_retained_snapshot
}
criterion_main!(benches);
