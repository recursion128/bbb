use super::*;

mod first_person_motion;
mod map_text;
mod registry_tooltips;
use crate::map_textures::map_color_rgba8;
use bbb_client_render::{
    EntityDefaultPlayerSkin, EntityPlayerSkin, GuiItemLightingEntry, HudEntityPreview,
    HudEntityPreviewItemDisplayContext, HudEntityPreviewItemLayer, HudEntityPreviewItemSlot,
    HudEntityPreviewRect, HudInventoryScreen, ParticleItemOptionState, PlayerModelPartVisibility,
    DEFAULT_ARMOR_STAND_MODEL_POSE, ENTITY_FULL_BRIGHT_LIGHT_COORDS,
};
use bbb_client_world::{
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type,
    vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type, ChunkColumn,
    ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldDimension,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID, VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
};
use bbb_protocol::packets::{
    AddEntity, BlockEvent, BlockPos as ProtocolBlockPos, BlockUpdate, CommonPlayerSpawnInfo,
    ContainerSetContent, DataComponentPatchSummary, EntityAnimation, EntityDataValue,
    EntityDataValueKind, EntityEvent, EquipmentSlotUpdate, MapColorPatch, MapItemData, OpenScreen,
    PlayLogin, RegistryData, RegistryDataEntry, RegistryTags, SetEntityData, SetEquipment,
    SetPlayerInventory, SwingAnimationSummary, TagNetworkPayload, UpdateTags, Vec3d,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const GRASS_BLOCK_STATE_ID: i32 = 9;

fn unit_block_quads() -> Vec<ItemModelQuad> {
    // A single full-cube face spanning the 0..16 Z range, enough to exercise depth + transforms.
    vec![ItemModelQuad {
        corners: [
            [0.0, 0.0, 0.0],
            [16.0, 0.0, 0.0],
            [16.0, 16.0, 16.0],
            [0.0, 16.0, 16.0],
        ],
        uvs: [[0.0, 0.0]; 4],
        tint: [1.0, 1.0, 1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
        shade: 1.0,
        translucent: false,
    }]
}

fn generated_slab_quads() -> Vec<ItemModelQuad> {
    // A generated-item slab: full 0..16 in X/Y, thin Z 7.5..8.5 (the extruded sprite face).
    vec![ItemModelQuad {
        corners: [
            [0.0, 0.0, 7.5],
            [16.0, 0.0, 7.5],
            [16.0, 16.0, 8.5],
            [0.0, 16.0, 8.5],
        ],
        uvs: [[0.0, 0.0]; 4],
        tint: [1.0; 4],
        normal: [0.0, 0.0, 1.0],
        shade: 1.0,
        translucent: false,
    }]
}

fn protocol_add_entity(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
        entity_type_id,
        position: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn world_with_level_dimension(dimension: &str) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 1,
        hardcore: false,
        levels: vec![dimension.to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: dimension.to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    });
    world
}

fn first_person_test_map_stack(item_runtime: &NativeItemRuntime, map_id: i32) -> ItemStackSummary {
    let mut item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:filled_map"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    item.component_patch.map_id = Some(map_id);
    item
}

fn first_person_test_consumable_stack(
    item_runtime: &NativeItemRuntime,
    item_id: &str,
    animation: ItemUseAnimationSummary,
    consume_seconds: f32,
) -> ItemStackSummary {
    let mut item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    item.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds,
        animation,
    });
    item
}

fn protocol_optional_block_state_data(data_id: u8, block_state: Option<i32>) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
        value: EntityDataValueKind::OptionalBlockState(block_state),
    }
}

fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(value),
    }
}

fn protocol_block_state_data(data_id: u8, block_state: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
        value: EntityDataValueKind::BlockState(block_state),
    }
}

fn world_with_enderman_carried_grass_block(entity_id: i32) -> WorldStore {
    let accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    )
    .expect("generated EnderMan render metadata accessors");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        entity_id,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: entity_id,
        values: vec![protocol_optional_block_state_data(
            accessors.carried_block_state_data_id,
            Some(GRASS_BLOCK_STATE_ID),
        )],
    }));
    world
}

fn world_with_chest_minecart(entity_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        entity_id,
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
    ));
    world
}

fn world_with_tnt_minecart(entity_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        entity_id,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    ));
    world
}

fn world_with_primed_tnt(entity_id: i32, fuse: i32, block_state: Option<i32>) -> WorldStore {
    let accessors = bbb_client_world::vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TNT_ID,
    )
    .expect("generated PrimedTnt metadata accessors");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id, VANILLA_ENTITY_TYPE_TNT_ID));
    let mut values = vec![protocol_int_data(accessors.fuse_data_id, fuse)];
    if let Some(block_state) = block_state {
        values.push(protocol_block_state_data(
            accessors.block_state_data_id,
            block_state,
        ));
    }
    assert!(world.apply_set_entity_data(SetEntityData {
        id: entity_id,
        values,
    }));
    world
}

fn world_with_falling_block(entity_id: i32, block_state_id: i32, position: [f64; 3]) -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let mut add = protocol_add_entity(entity_id, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID);
    add.data = block_state_id;
    add.position = Vec3d {
        x: position[0],
        y: position[1],
        z: position[2],
    };
    world.apply_add_entity(add);
    world
}

fn insert_entity_block_test_chunk(world: &mut WorldStore) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
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

fn set_entity_block_test_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        block_state_id,
    }));
}

fn equipment(entity_id: i32, slot: EquipmentSlot, item_id: i32) -> SetEquipment {
    SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    }
}

fn record_enchantment_registry(world: &mut WorldStore) {
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:power".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:quick_charge".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
}

fn record_trim_material_registry(world: &mut WorldStore) {
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:trim_material".to_string(),
        entries: vec![
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:quartz".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:iron".to_string(),
                raw_data: None,
            },
            bbb_protocol::packets::RegistryDataEntry {
                id: "minecraft:diamond".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
}

fn write_flat_item_runtime_fixture(root: &Path, item_ids: &[&str]) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, item_ids);
    for (index, item_id) in item_ids.iter().enumerate() {
        write_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                "model": {{
                    "type": "minecraft:model",
                    "model": "minecraft:item/{item_id}"
                }}
            }}"#
            ),
        );
        write_flat_item_model_and_texture(
            &assets,
            item_id,
            &[(64 + index * 40) as u8, 80, 120, 255],
        );
    }
}

fn write_component_select_registry_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["component_selector"]);
    write_json(
        &assets.join("items").join("component_selector.json"),
        r##"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:component",
                "component": "minecraft:damage_resistant",
                "cases": [
                    {
                        "when": { "types": "#example:fire" },
                        "model": {
                            "type": "minecraft:model",
                            "model": "minecraft:item/component_select_match"
                        }
                    }
                ],
                "fallback": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/component_select_fallback"
                }
            }
        }"##,
    );
    write_flat_item_model_and_texture(&assets, "component_select_match", &[180, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "component_select_fallback", &[40, 80, 180, 255]);
}

fn world_with_component_select_prototype(item_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![bbb_client_world::RegistryPacketEntry::stub(
            "example:in_fire",
        )],
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:damage_type".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:fire".to_string(),
                entries: vec![0],
            }],
        }],
    });
    world.set_default_item_component_codec_values(BTreeMap::from([(
        item_id,
        BTreeMap::from([(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
            serde_json::json!({ "types": "#example:fire" }),
        )]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    world
}

fn write_team_tint_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["team_tint"]);
    write_json(
        &assets.join("items").join("team_tint.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/team_tint",
                "tints": [
                    { "type": "minecraft:team", "default": 1193046 }
                ]
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "team_tint", &[255, 255, 255, 255]);
}

fn write_special_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["special_trident"]);
    write_json(
        &assets.join("items").join("special_trident.json"),
        r#"{
            "model": {
                "type": "minecraft:special",
                "base": "minecraft:item/special_trident_base",
                "model": { "type": "minecraft:trident" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "special_trident_base", &[64, 80, 120, 255]);
    write_json(
        &assets
            .join("models")
            .join("item")
            .join("special_trident_base.json"),
        r#"{
            "display": {
                "thirdperson_righthand": { "translation": [4, 0, 0] }
            },
            "textures": { "layer0": "minecraft:item/special_trident_base" }
        }"#,
    );
}

fn write_composite_special_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["composite_special"]);
    write_json(
        &assets.join("items").join("composite_special.json"),
        r#"{
            "model": {
                "type": "minecraft:composite",
                "models": [
                    {
                        "type": "minecraft:model",
                        "model": "minecraft:item/composite_before"
                    },
                    {
                        "type": "minecraft:special",
                        "base": "minecraft:item/composite_special_base",
                        "model": { "type": "minecraft:trident" }
                    },
                    {
                        "type": "minecraft:model",
                        "model": "minecraft:item/composite_after"
                    }
                ]
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "composite_before", &[64, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "composite_special_base", &[80, 120, 64, 255]);
    write_flat_item_model_and_texture(&assets, "composite_after", &[120, 64, 80, 255]);
}

fn world_with_item_runtime_defaults(item_runtime: &NativeItemRuntime) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:damage_type".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:bypasses_shield".to_string(),
                entries: vec![0],
            }],
        }],
    });
    crate::resource_reload::apply_item_runtime_world_defaults(item_runtime, &mut world).unwrap();
    world
}

fn write_default_consumable_item_runtime_fixture(root: &Path, item_id: &str) {
    write_flat_item_runtime_fixture(root, &[item_id]);
    crate::item_registry_test_fixture::write_item_registry_fixture_with_profiles(
        root,
        &[item_id],
        &[(item_id, "apple")],
    );
}

fn write_main_hand_select_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["hand_selector"]);
    write_json(
        &assets.join("items").join("hand_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:main_hand",
                "cases": [
                    {
                        "when": "left",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_left" }
                    },
                    {
                        "when": "right",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_right" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/hand_selector" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "hand_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "hand_selector_left", &[120, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "hand_selector_right", &[80, 120, 40, 255]);
}

fn write_context_entity_type_select_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["entity_selector"]);
    write_json(
        &assets.join("items").join("entity_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:context_entity_type",
                "cases": [
                    {
                        "when": "minecraft:player",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_player" }
                    },
                    {
                        "when": "minecraft:witch",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_witch" }
                    },
                    {
                        "when": "minecraft:item",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_item" }
                    },
                    {
                        "when": "minecraft:snowball",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_snowball" }
                    },
                    {
                        "when": "minecraft:firework_rocket",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_firework_rocket" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/entity_selector" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "entity_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "entity_selector_player", &[120, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "entity_selector_witch", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "entity_selector_item", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "entity_selector_snowball", &[40, 120, 80, 255]);
    write_flat_item_model_and_texture(
        &assets,
        "entity_selector_firework_rocket",
        &[80, 40, 120, 255],
    );
}

fn write_context_dimension_select_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["dimension_selector"]);
    write_json(
        &assets.join("items").join("dimension_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:context_dimension",
                "cases": [
                    {
                        "when": "minecraft:overworld",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_overworld" }
                    },
                    {
                        "when": "minecraft:the_nether",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_nether" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "dimension_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "dimension_selector_overworld", &[120, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "dimension_selector_nether", &[80, 120, 40, 255]);
}

fn write_trim_material_select_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["iron_chestplate"]);
    write_json(
        &assets.join("items").join("iron_chestplate.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:trim_material",
                "cases": [
                    {
                        "when": "minecraft:quartz",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_quartz_trim" }
                    },
                    {
                        "when": "minecraft:diamond",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_diamond_trim" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "iron_chestplate", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(
        &assets,
        "iron_chestplate_quartz_trim",
        &[200, 200, 190, 255],
    );
    write_flat_item_model_and_texture(
        &assets,
        "iron_chestplate_diamond_trim",
        &[120, 200, 210, 255],
    );
}

fn write_using_item_condition_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["use_selector"]);
    write_json(
        &assets.join("items").join("use_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/use_selector" },
                "on_true": { "type": "minecraft:model", "model": "minecraft:item/use_selector_using" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "use_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "use_selector_using", &[120, 80, 40, 255]);
}

fn write_bow_use_duration_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["bow"]);
    write_json(
        &assets.join("items").join("bow.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/bow" },
                "on_true": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_duration",
                    "scale": 0.05,
                    "entries": [
                        {
                            "threshold": 0.65,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_1" }
                        },
                        {
                            "threshold": 0.9,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_2" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_0" }
                }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "bow", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "bow_pulling_0", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bow_pulling_1", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bow_pulling_2", &[120, 40, 80, 255]);
}

fn write_use_duration_selector_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["use_duration_selector"]);
    write_json(
        &assets.join("items").join("use_duration_selector.json"),
        r#"{
            "model": {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/use_duration_selector" },
                "on_true": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_duration",
                    "scale": 0.05,
                    "entries": [
                        {
                            "threshold": 0.65,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/use_duration_pulling_1" }
                        },
                        {
                            "threshold": 0.9,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/use_duration_pulling_2" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/use_duration_pulling_0" }
                }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "use_duration_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "use_duration_pulling_0", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "use_duration_pulling_1", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "use_duration_pulling_2", &[120, 40, 80, 255]);
}

fn write_crossbow_pull_item_runtime_fixture(root: &Path) {
    let assets = item_model_assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["crossbow"]);
    write_json(
        &assets.join("items").join("crossbow.json"),
        r#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:charge_type",
                "cases": [],
                "fallback": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/crossbow" },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:crossbow/pull",
                        "entries": [
                            {
                                "threshold": 0.58,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_1" }
                            },
                            {
                                "threshold": 1.0,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_2" }
                            }
                        ],
                        "fallback": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_0" }
                    }
                }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_pulling_0", &[70, 100, 130, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_pulling_1", &[100, 130, 70, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_pulling_2", &[130, 70, 100, 255]);
}

fn item_model_assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

fn write_item_atlases(assets: &Path) {
    write_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "block/",
                    "source": "block"
                }
            ]
        }"#,
    );
}

fn write_item_registry_source(root: &Path, item_ids: &[&str]) {
    crate::item_registry_test_fixture::write_item_registry_fixture(root, item_ids);
}

fn write_flat_item_model_and_texture(assets: &Path, model_id: &str, rgba: &[u8]) {
    write_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
            "parent": "minecraft:builtin/generated",
            "textures": {{
                "layer0": "minecraft:item/{model_id}"
            }}
        }}"#
        ),
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join(format!("{model_id}.png")),
        rgba,
    );
}

fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_test_rgba_png(path: &Path, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
}

fn unique_item_model_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-client-item-models-{label}-{nanos}"))
}

mod block_attachments_and_held;
mod first_person_poses;
mod predicates_dropped_and_display;

fn random_crossbow_charge_time_enchantment_nbt() -> Vec<u8> {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue as Nbt};
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let cost = Nbt::Compound(vec![
        entry("base", Nbt::Int(0)),
        entry("per_level_above_first", Nbt::Int(0)),
    ]);
    let root = Nbt::Compound(vec![
        entry("description", Nbt::string("Random Loading")),
        entry("supported_items", Nbt::string("minecraft:crossbow")),
        entry("weight", Nbt::Int(1)),
        entry("max_level", Nbt::Int(1)),
        entry("min_cost", cost.clone()),
        entry("max_cost", cost),
        entry("anvil_cost", Nbt::Int(0)),
        entry("slots", Nbt::List(vec![Nbt::string("mainhand")])),
        entry(
            "effects",
            Nbt::Compound(vec![entry(
                "minecraft:crossbow_charge_time",
                Nbt::Compound(vec![
                    entry("type", Nbt::string("minecraft:all_of")),
                    entry(
                        "effects",
                        Nbt::List(vec![
                            Nbt::Compound(vec![
                                entry("type", Nbt::string("minecraft:set")),
                                entry("value", Nbt::Float(5.0_f32.to_bits())),
                            ]),
                            Nbt::Compound(vec![
                                entry("type", Nbt::string("minecraft:remove_binomial")),
                                entry("chance", Nbt::Float(0.5_f32.to_bits())),
                            ]),
                        ]),
                    ),
                ]),
            )]),
        ),
    ]);
    bbb_protocol::encode_nbt_summary(&root, 4096).unwrap()
}

#[test]
fn crossbow_presentation_applies_random_enchantment_effects() {
    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![bbb_protocol::packets::RegistryDataEntry {
            id: "example:random_loading".to_string(),
            raw_data: Some(random_crossbow_charge_time_enchantment_nbt()),
        }],
        raw_payload_len: 0,
    });
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    let enchantments = [bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 0,
        level: 1,
    }];
    let runtime = NativeItemRuntime::empty_for_test();
    let durations = (0..32)
        .map(|_| {
            crate::item_models::crossbow_charge_duration_ticks(&runtime, &world, &enchantments)
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(durations.len() > 1);
    assert!(durations
        .iter()
        .all(|ticks| (0..=100).contains(ticks) && ticks % 20 == 0));
}
