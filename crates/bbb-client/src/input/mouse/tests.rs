use super::*;
use bbb_client_net::{
    AttackEntityCommand, InteractEntityCommand, UseItemCommand, UseItemOnCommand,
};
use bbb_client_world::{
    vanilla_entity_metadata_entity_accessors, BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos,
    ChunkSection, ChunkState, LightData, LocalPlayerPoseState, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldDimension,
};
use bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
    VANILLA_DATA_COMPONENT_TOOL_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
};
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_protocol::equipment_slots::{
    VANILLA_EQUIPMENT_SLOT_CHEST_ID, VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
    VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_BONE_MEAL_ID, VANILLA_ITEM_BOW_ID,
    VANILLA_ITEM_BRUSH_ID, VANILLA_ITEM_BUNDLE_ID, VANILLA_ITEM_CARROT_ON_A_STICK_ID,
    VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID, VANILLA_ITEM_COMPASS_ID, VANILLA_ITEM_CROSSBOW_ID,
    VANILLA_ITEM_DEBUG_STICK_ID, VANILLA_ITEM_ENDER_EYE_ID, VANILLA_ITEM_ENDER_PEARL_ID,
    VANILLA_ITEM_FIREWORK_ROCKET_ID, VANILLA_ITEM_FIRE_CHARGE_ID, VANILLA_ITEM_FISHING_ROD_ID,
    VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_GOAT_HORN_ID,
    VANILLA_ITEM_KNOWLEDGE_BOOK_ID, VANILLA_ITEM_MAP_ID, VANILLA_ITEM_OAK_BOAT_ID,
    VANILLA_ITEM_POTION_ID, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_SHIELD_ID,
    VANILLA_ITEM_SPYGLASS_ID, VANILLA_ITEM_STONE_ID, VANILLA_ITEM_TRIDENT_ID,
    VANILLA_ITEM_WATER_BUCKET_ID,
};
use bbb_protocol::packets::{
    AddEntity, AdventureModePredicateDataSummary, AttackEntity, AttackRangeSummary,
    BlockEntityData, BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
    BlockPredicateSummary, BlockUpdate, CommonPlayerSpawnInfo, DataComponentPatchSummary,
    EntityDataValue, EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EquippableSummary,
    GameEvent as ProtocolGameEvent, HolderSetSummary, InitializeBorder, InstrumentSummary,
    InteractEntity, ItemEnchantmentSummary, ItemStackSummary as ProtocolItemStackSummary,
    ItemStackTemplateSummary, LodestoneTargetSummary, OpenSignEditor, PickItemFromBlock,
    PickItemFromEntity, PlayLogin, PlayerAbilities, PlayerAction, PlayerCommand,
    PlayerCommandAction, PlayerHealth, RegistryData, RegistryDataEntry, RegistryTags,
    RemoveEntities, SetEntityData, SetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
    SoundEventSummary, TagNetworkPayload, ToolSummary, UpdateTags, UseItem, UseItemOn,
    Vec3d as ProtocolVec3d,
};
use std::collections::BTreeMap;
use uuid::Uuid;

const CROSSHAIR_BLOCK_POS: BlockPos = BlockPos { x: 0, y: 1, z: 3 };
const VANILLA_GRASS_BLOCK_STATE_ID: i32 = 9;
const VANILLA_WORLD_BORDER_ABSOLUTE_MAX_SIZE: i32 = 29_999_984;

#[derive(Default)]
struct RecordingAudioSink {
    entity_sounds: Vec<(bbb_client_world::SoundEntityEventState, Option<[f64; 3]>)>,
    positioned_sounds: Vec<bbb_client_world::SoundEventState>,
}

impl AudioEventSink for RecordingAudioSink {
    fn counters(&self) -> crate::control::AudioCounters {
        crate::control::AudioCounters::default()
    }

    fn set_sound_event_registry(&mut self, _registry: bbb_client_assets::SoundEventRegistry) {}

    fn set_jukebox_song_registry(&mut self, _songs: bbb_client_assets::JukeboxSongRegistry) {}

    fn play_local_sound(&mut self, _state: &bbb_client_world::LocalSoundEventState) {}

    fn play_positioned_sound(&mut self, state: &bbb_client_world::SoundEventState) {
        self.positioned_sounds.push(state.clone());
    }

    fn play_entity_sound(
        &mut self,
        state: &bbb_client_world::SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) {
        self.entity_sounds.push((state.clone(), position));
    }

    fn play_jukebox_song(&mut self, _state: &bbb_client_world::JukeboxLevelEventState) {}

    fn stop_jukebox_song(&mut self, _state: &bbb_client_world::JukeboxLevelEventState) {}

    fn stop_sound(&mut self, _state: &bbb_client_world::StopSoundEventState) {}

    fn set_music_gain(&mut self, _gain: f32) {}

    fn sync_ambient_loops(&mut self, _loops: &[bbb_client_world::AmbientLoopSoundState]) {}

    fn tick_entity_sound_positions(
        &mut self,
        _listener: Option<bbb_client_audio::AudioListenerState>,
        _entity_position: &dyn Fn(i32) -> Option<[f64; 3]>,
        _tickable_sounds: &[bbb_client_world::TickableEntitySoundState],
    ) {
    }
}

fn set_local_spectator(world: &mut WorldStore) {
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(world.local_player_is_spectator());
}

fn set_local_adventure(world: &mut WorldStore) {
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 2.0,
    });
    assert!(world.local_player_is_adventure());
}

fn set_flying_abilities(world: &mut WorldStore, flying_speed: f32) {
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying: true,
        can_fly: true,
        instabuild: false,
        flying_speed,
        walking_speed: 0.1,
    });
}

fn set_world_border_excluding_crosshair_block(world: &mut WorldStore) {
    world.apply_initialize_border(InitializeBorder {
        new_center_x: 0.0,
        new_center_z: 0.0,
        old_size: 2.0,
        new_size: 2.0,
        lerp_time: 0,
        new_absolute_max_size: VANILLA_WORLD_BORDER_ABSOLUTE_MAX_SIZE,
        warning_blocks: 5,
        warning_time: 15,
    });
    assert!(!world.world_border().contains_block_pos(CROSSHAIR_BLOCK_POS));
}

fn world_with_open_sign_editor(input: &mut ClientInputState) -> WorldStore {
    let pos = BlockPos { x: 1, y: 2, z: 3 };
    let mut world = WorldStore::new();
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: std::sync::Arc::new(vec![BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        )
        .with_raw_nbt(vec![0])
        .with_payload(bbb_client_world::BlockEntityPayload::SignText(
            Box::default(),
        ))]),
        light: LightData::default(),
    });
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world
}

fn world_with_crosshair_entity(entity_id: i32) -> WorldStore {
    world_with_crosshair_entity_type_at(entity_id, VANILLA_ENTITY_TYPE_MINECART_ID, [0.0, 1.0, 3.0])
}

fn world_with_crosshair_entity_type(entity_id: i32, entity_type_id: i32) -> WorldStore {
    world_with_crosshair_entity_type_at(entity_id, entity_type_id, [0.0, 1.0, 3.0])
}

fn local_attack_world_with_player_and_target(entity_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    world.set_local_player_pose(LocalPlayerPoseState::default());
    world.apply_add_entity(AddEntity {
        id: 42,
        uuid: Uuid::from_u128(42),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_add_entity(AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world
}

fn world_with_crosshair_command_minecart(entity_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 1,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 42,
        event_id: 26,
    }));
    world.set_local_player_pose(LocalPlayerPoseState::default());
    world.apply_add_entity(AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world
}

fn world_with_crosshair_entity_at(entity_id: i32, position: [f64; 3]) -> WorldStore {
    world_with_crosshair_entity_type_at(entity_id, VANILLA_ENTITY_TYPE_MINECART_ID, position)
}

fn world_with_crosshair_entity_type_at(
    entity_id: i32,
    entity_type_id: i32,
    position: [f64; 3],
) -> WorldStore {
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    world.apply_add_entity(AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id,
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world
}

fn world_with_crosshair_block() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(
                PaletteDomain::BlockStates,
                4096,
                bbb_client_world::vanilla_air_block_state_id(),
            ),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
    }));
    world
}

fn world_with_crosshair_lodestone() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|_| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_client_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
    let lodestone_state =
        bbb_client_world::vanilla_block_state_id("minecraft:lodestone", &BTreeMap::new()).unwrap();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: lodestone_state,
    }));
    world
}

fn world_with_crosshair_ignitable_block() -> WorldStore {
    let mut world = world_with_crosshair_lodestone();
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: -1, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|_| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_client_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: test_block_state_id("minecraft:oak_planks", &[]),
    }));
    world
}

fn test_block_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    bbb_client_world::vanilla_block_state_id(name, &properties)
        .unwrap_or_else(|| panic!("missing Vanilla block state {name} {properties:?}"))
}

fn world_with_crosshair_command_block() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 1,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 42,
        event_id: 26,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|_| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_client_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
    let command_block_state = bbb_client_world::vanilla_block_state_id(
        "minecraft:command_block",
        &BTreeMap::from([
            ("conditional".to_string(), "false".to_string()),
            ("facing".to_string(), "north".to_string()),
        ]),
    )
    .unwrap();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
        block_state_id: command_block_state,
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
            raw_nbt: vec![10, 0],
        })
        .unwrap());
    world
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn water_potion_item_stack(count: i32) -> ProtocolItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_POTION_ID, count);
    stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID);
    stack.component_patch.potion_contents.potion_id = Some(0);
    stack
}

fn install_consumable(world: &mut WorldStore, slot: i32, item_id: i32, consume_seconds: f32) {
    world.set_default_item_component_codec_values(BTreeMap::from([(
        item_id,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            serde_json::json!({"consume_seconds": consume_seconds}),
        )]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot,
        item: item_stack(item_id, 1),
    });
}

fn item_stack_with_attack_range(
    item_id: i32,
    count: i32,
    min_reach: f32,
    max_reach: f32,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID];
    stack.component_patch.attack_range = Some(AttackRangeSummary {
        min_reach,
        max_reach,
        min_creative_reach: min_reach,
        max_creative_reach: max_reach,
        hitbox_margin: 0.0,
        mob_factor: 1.0,
    });
    stack
}

fn set_selected_piercing_weapon(world: &mut WorldStore, item_id: i32) {
    let piercing_type =
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID;
    let catalog = bbb_client_assets::ItemRegistryCatalog::vanilla_26_1().unwrap();
    world.set_default_item_component_codec_values(BTreeMap::from([(
        item_id,
        BTreeMap::from([(
            piercing_type,
            catalog
                .default_component_codec_values("wooden_spear")
                .unwrap()[&piercing_type]
                .clone(),
        )]),
    )]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(item_id, 1),
    });
}

fn set_selected_can_break(world: &mut WorldStore, block: &str) {
    let block_id = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block)
        .unwrap()
        .id;
    let mut stack = item_stack(1, 1);
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID];
    stack.component_patch.can_break_persistent = Some(AdventureModePredicateDataSummary {
        predicates: vec![BlockPredicateSummary {
            blocks: Some(HolderSetSummary {
                registry_ids: vec![block_id],
                tag: None,
            }),
            ..BlockPredicateSummary::default()
        }],
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
}

fn assert_f64_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-12,
        "expected {expected}, got {actual}"
    );
}

fn world_with_ender_dragon() -> WorldStore {
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    world.apply_add_entity(AddEntity {
        id: 100,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 2.0,
            z: 9.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.advance_entity_client_animations(1);
    world
}

mod attack_and_destroy;
mod entity_and_block_interact;
mod pick_and_wheel;
mod use_item;
