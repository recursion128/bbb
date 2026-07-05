use super::*;
use crate::map_textures::map_color_rgba8;
use bbb_protocol::packets::{
    AddEntity, BlockPos as ProtocolBlockPos, BlockUpdate, CommonPlayerSpawnInfo,
    DataComponentPatchSummary, EntityAnimation, EntityDataValue, EntityDataValueKind, EntityEvent,
    EquipmentSlotUpdate, MapColorPatch, MapItemData, PlayLogin, SetEntityData, SetEquipment,
    SetPlayerInventory, SwingAnimationSummary, Vec3d,
};
use bbb_renderer::{
    EntityDefaultPlayerSkin, EntityPlayerSkin, ParticleItemOptionState, PlayerModelPartVisibility,
};
use bbb_world::{
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldDimension,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const ENDERMAN_CARRY_STATE_DATA_ID: u8 = 16;
const BLOCK_STATE_SERIALIZER_ID: i32 = 14;
const OPTIONAL_BLOCK_STATE_SERIALIZER_ID: i32 = 15;
const AIR_BLOCK_STATE_ID: i32 = 0;
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
        component_patch: DataComponentPatchSummary::default(),
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
        component_patch: DataComponentPatchSummary::default(),
    };
    item.component_patch.added = 1;
    item.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    item.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds,
        animation,
    });
    item
}

fn protocol_optional_block_state_data(data_id: u8, block_state: Option<i32>) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: OPTIONAL_BLOCK_STATE_SERIALIZER_ID,
        value: EntityDataValueKind::OptionalBlockState(block_state),
    }
}

fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: 1,
        value: EntityDataValueKind::Int(value),
    }
}

fn protocol_block_state_data(data_id: u8, block_state: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: BLOCK_STATE_SERIALIZER_ID,
        value: EntityDataValueKind::BlockState(block_state),
    }
}

fn world_with_enderman_carried_grass_block(entity_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        entity_id,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: entity_id,
        values: vec![protocol_optional_block_state_data(
            ENDERMAN_CARRY_STATE_DATA_ID,
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
    const PRIMED_TNT_FUSE_DATA_ID: u8 = 8;
    const PRIMED_TNT_BLOCK_STATE_DATA_ID: u8 = 9;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id, VANILLA_ENTITY_TYPE_TNT_ID));
    let mut values = vec![protocol_int_data(PRIMED_TNT_FUSE_DATA_ID, fuse)];
    if let Some(block_state) = block_state {
        values.push(protocol_block_state_data(
            PRIMED_TNT_BLOCK_STATE_DATA_ID,
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
        sections: vec![ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        }],
        block_entities: Vec::new(),
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

#[test]
fn carved_pumpkin_layer_uses_vanilla_default_block_state() {
    assert_eq!(
        carved_pumpkin_default_properties(),
        BTreeMap::from([("facing".to_string(), "north".to_string())])
    );
}

#[test]
fn poppy_layer_uses_vanilla_default_block_state() {
    assert_eq!(poppy_default_properties(), BTreeMap::new());
}

#[test]
fn mooshroom_mushroom_layers_use_vanilla_default_block_states() {
    assert_eq!(
        mooshroom_mushroom_block_id(MooshroomVariant::Red),
        RED_MUSHROOM_BLOCK_ID
    );
    assert_eq!(
        mooshroom_mushroom_block_id(MooshroomVariant::Brown),
        BROWN_MUSHROOM_BLOCK_ID
    );
    assert_eq!(mooshroom_mushroom_default_properties(), BTreeMap::new());
}

#[test]
fn entity_block_attachments_collect_snow_golem_iron_golem_and_mooshroom_layers() {
    let world = WorldStore::new();
    let snow_golem =
        EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0).with_snow_golem_pumpkin(true);
    let iron_golem = EntityModelInstance::iron_golem(74, [1.0, 64.0, 0.0], 0.0)
        .with_iron_golem_offer_flower_tick(400);
    let idle_iron_golem = EntityModelInstance::iron_golem(75, [2.0, 64.0, 0.0], 0.0);
    let mooshroom = EntityModelInstance::mooshroom(86, [3.0, 64.0, 0.0], 0.0, false);
    let baby_mooshroom = EntityModelInstance::mooshroom(87, [4.0, 64.0, 0.0], 0.0, true);

    let attachments = entity_block_attachments(
        &[
            snow_golem,
            iron_golem,
            idle_iron_golem,
            mooshroom,
            baby_mooshroom,
        ],
        &world,
        None,
        1.0,
    );

    assert_eq!(attachments.len(), 5);
    assert!(attachments
        .iter()
        .all(|attachment| !attachment.outline_only));
    assert_eq!(attachments[0].block_id.as_ref(), CARVED_PUMPKIN_BLOCK_ID);
    assert_eq!(
        attachments[0].properties,
        carved_pumpkin_default_properties()
    );
    assert_eq!(attachments[1].block_id.as_ref(), POPPY_BLOCK_ID);
    assert_eq!(attachments[1].properties, poppy_default_properties());
    for attachment in &attachments[2..] {
        assert_eq!(attachment.block_id.as_ref(), RED_MUSHROOM_BLOCK_ID);
        assert_eq!(
            attachment.properties,
            mooshroom_mushroom_default_properties()
        );
    }
}

#[test]
fn entity_block_attachments_record_invisible_glowing_outline_only_layers() {
    let world = WorldStore::new();
    let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
        .with_snow_golem_pumpkin(true)
        .with_invisible(true)
        .with_appears_glowing(true);
    let mooshroom = EntityModelInstance::mooshroom(86, [3.0, 64.0, 0.0], 0.0, false)
        .with_invisible(true)
        .with_appears_glowing(true);

    let attachments = entity_block_attachments(&[snow_golem, mooshroom], &world, None, 1.0);

    assert_eq!(attachments.len(), 4);
    assert!(attachments.iter().all(|attachment| attachment.outline_only));
    assert_eq!(attachments[0].block_id.as_ref(), CARVED_PUMPKIN_BLOCK_ID);
    assert_eq!(
        attachments[0].properties,
        carved_pumpkin_default_properties()
    );
    for attachment in &attachments[1..] {
        assert_eq!(attachment.block_id.as_ref(), RED_MUSHROOM_BLOCK_ID);
        assert_eq!(
            attachment.properties,
            mooshroom_mushroom_default_properties()
        );
    }
}

#[test]
fn entity_block_attachments_collect_enderman_carried_block_from_world() {
    let entity_id = 41;
    let world = world_with_enderman_carried_grass_block(entity_id);
    let light_coords = (6_u32 << 4) | (10_u32 << 20);
    let enderman = EntityModelInstance::enderman(entity_id, [0.0, 64.0, 0.0], 0.0)
        .with_enderman_carrying(true)
        .with_light_coords(light_coords);

    let attachments = entity_block_attachments(&[enderman], &world, None, 1.0);

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
    assert_eq!(
        attachments[0].properties,
        BTreeMap::from([("snowy".to_string(), "false".to_string())])
    );
    assert_eq!(attachments[0].light, [6.0 / 15.0, 10.0 / 15.0]);
}

#[test]
fn entity_block_attachments_collect_minecart_display_block_from_world() {
    let entity_id = 85;
    let world = world_with_chest_minecart(entity_id);
    let light_coords = (6_u32 << 4) | (10_u32 << 20);
    let minecart = EntityModelInstance::minecart(entity_id, [0.0, 64.0, 0.0], 45.0)
        .with_light_coords(light_coords);

    let attachments = entity_block_attachments(&[minecart], &world, None, 1.0);

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:chest");
    assert_eq!(
        attachments[0].properties,
        BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("type".to_string(), "single".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ])
    );
    assert_eq!(attachments[0].light, [6.0 / 15.0, 10.0 / 15.0]);
    assert_eq!(attachments[0].overlay, ITEM_MODEL_NO_OVERLAY);
    assert!(!attachments[0].outline_only);
    assert!(attachments[0]
        .transform
        .transform_point3(Vec3::splat(0.5))
        .is_finite());

    let hidden = minecart.with_invisible(true);
    assert!(entity_block_attachments(&[hidden], &world, None, 1.0).is_empty());

    let outline = hidden.with_appears_glowing(true);
    let outline_attachments = entity_block_attachments(&[outline], &world, None, 1.0);
    assert_eq!(outline_attachments.len(), 1);
    assert!(outline_attachments[0].outline_only);
}

#[test]
fn entity_block_attachments_apply_tnt_minecart_fuse_scale_and_white_overlay() {
    let entity_id = 133;
    let world = world_with_tnt_minecart(entity_id);
    let light_coords = (5_u32 << 4) | (12_u32 << 20);
    let primed = EntityModelInstance::minecart(entity_id, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords(light_coords)
        .with_minecart_tnt_fuse_remaining_in_ticks(4.5);

    let attachments = entity_block_attachments(&[primed], &world, None, 1.0);

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:tnt");
    assert_eq!(
        attachments[0].properties,
        BTreeMap::from([("unstable".to_string(), "false".to_string())])
    );
    assert_eq!(attachments[0].light, [5.0 / 15.0, 12.0 / 15.0]);
    assert_eq!(attachments[0].overlay, [15.0, 10.0]);
    assert_eq!(
        attachments[0].transform,
        minecart_tnt_display_block_transform(&primed, 6).unwrap()
    );

    let unprimed = primed.with_minecart_tnt_fuse_remaining_in_ticks(-1.0);
    let unprimed_attachment = entity_block_attachments(&[unprimed], &world, None, 1.0)
        .pop()
        .expect("unprimed tnt attachment");
    assert_eq!(unprimed_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
    assert_ne!(unprimed_attachment.transform, attachments[0].transform);

    let odd_strobe = primed.with_minecart_tnt_fuse_remaining_in_ticks(6.0);
    let odd_strobe_attachment = entity_block_attachments(&[odd_strobe], &world, None, 1.0)
        .pop()
        .expect("odd strobe tnt attachment");
    assert_eq!(odd_strobe_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
}

#[test]
fn entity_block_attachments_apply_primed_tnt_block_state_fuse_and_outline() {
    let entity_id = 134;
    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let world = world_with_primed_tnt(entity_id, 4, Some(GRASS_BLOCK_STATE_ID));
    let light_coords = (5_u32 << 4) | (12_u32 << 20);
    let tnt = EntityModelInstance::no_render(entity_id, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords(light_coords);

    let attachments = entity_block_attachments(&[tnt], &world, None, 0.5);

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
    assert_eq!(attachments[0].properties, grass_props);
    assert_eq!(attachments[0].light, [5.0 / 15.0, 12.0 / 15.0]);
    assert_eq!(attachments[0].overlay, [15.0, 10.0]);
    assert_eq!(
        attachments[0].transform,
        primed_tnt_block_transform(&tnt, 4.5).unwrap()
    );
    assert!(!attachments[0].outline_only);

    let hidden = tnt.with_invisible(true);
    assert!(entity_block_attachments(&[hidden], &world, None, 0.5).is_empty());
    let outline = hidden.with_appears_glowing(true);
    let outline_attachments = entity_block_attachments(&[outline], &world, None, 0.5);
    assert_eq!(outline_attachments.len(), 1);
    assert!(outline_attachments[0].outline_only);

    let default_world = world_with_primed_tnt(entity_id, 6, None);
    let default_attachment = entity_block_attachments(&[tnt], &default_world, None, 0.0)
        .pop()
        .expect("default primed tnt attachment");
    assert_eq!(default_attachment.block_id.as_ref(), "minecraft:tnt");
    assert_eq!(
        default_attachment.properties,
        BTreeMap::from([("unstable".to_string(), "false".to_string())])
    );
    assert_eq!(default_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
}

#[test]
fn entity_block_attachments_apply_falling_block_state_and_should_render_gate() {
    let entity_id = 135;
    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let mut world = world_with_falling_block(entity_id, GRASS_BLOCK_STATE_ID, [1.5, 4.0, 2.5]);
    let light_coords = (7_u32 << 4) | (11_u32 << 20);
    let falling = EntityModelInstance::no_render(entity_id, [1.5, 4.0, 2.5], 90.0)
        .with_light_coords(light_coords);

    let attachments = entity_block_attachments(&[falling], &world, None, 1.0);

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
    assert_eq!(attachments[0].properties, grass_props);
    assert_eq!(attachments[0].light, [7.0 / 15.0, 11.0 / 15.0]);
    assert_eq!(attachments[0].overlay, ITEM_MODEL_NO_OVERLAY);
    assert_eq!(
        attachments[0].transform,
        falling_block_transform(&falling).unwrap()
    );
    assert!(!attachments[0].outline_only);

    let hidden_outline = falling.with_invisible(true).with_appears_glowing(true);
    let hidden_attachments = entity_block_attachments(&[hidden_outline], &world, None, 1.0);
    assert_eq!(hidden_attachments.len(), 1);
    assert!(
        !hidden_attachments[0].outline_only,
        "vanilla FallingBlockRenderer.submit does not gate the body on isInvisible"
    );

    assert!(falling_block_should_render(
        &falling,
        &world,
        GRASS_BLOCK_STATE_ID
    ));
    insert_entity_block_test_chunk(&mut world);
    let feet = BlockPos { x: 1, y: 4, z: 2 };
    set_entity_block_test_block(&mut world, feet, GRASS_BLOCK_STATE_ID);
    assert!(!falling_block_should_render(
        &falling,
        &world,
        GRASS_BLOCK_STATE_ID
    ));
    assert!(entity_block_attachments(&[falling], &world, None, 1.0).is_empty());

    set_entity_block_test_block(&mut world, feet, AIR_BLOCK_STATE_ID);
    assert!(falling_block_should_render(
        &falling,
        &world,
        GRASS_BLOCK_STATE_ID
    ));
    assert_eq!(
        entity_block_attachments(&[falling], &world, None, 1.0).len(),
        1
    );
}

#[test]
fn copper_golem_antenna_attachment_uses_block_state_component_properties() {
    let copper_golem = EntityModelInstance::new(
        28,
        bbb_renderer::EntityModelKind::CopperGolem {
            weathering: bbb_renderer::CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let stack = ItemStackSummary {
        item_id: Some(901),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_state_properties: BTreeMap::from([
                ("facing".to_string(), "south".to_string()),
                ("powered".to_string(), "true".to_string()),
            ]),
            ..DataComponentPatchSummary::default()
        },
    };

    let attachment = copper_golem_antenna_block_attachment_from_stack(
        &copper_golem,
        &stack,
        "minecraft:oak_button",
    )
    .unwrap();

    assert_eq!(attachment.block_id.as_ref(), "minecraft:oak_button");
    assert_eq!(
        attachment.properties,
        BTreeMap::from([
            ("facing".to_string(), "south".to_string()),
            ("powered".to_string(), "true".to_string()),
        ])
    );
    assert!(attachment
        .transform
        .transform_point3(Vec3::splat(0.5))
        .is_finite());
    assert!(copper_golem_antenna_block_attachment_from_stack(
        &EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0),
        &stack,
        "minecraft:oak_button",
    )
    .is_none());
}

#[test]
fn block_ground_transform_seats_a_unit_block_just_above_the_entity_origin() {
    // The 0.25-scaled block centered then lifted +3/16 sits at y in [0.0625, 0.3125], plus the bob.
    let ground_matrix = display_matrix(&BLOCK_GROUND_FALLBACK, false);
    let (min_y, _) = ground_model_bounds(&unit_block_quads(), ground_matrix);
    let min_offset_y = -min_y + 1.0 / 16.0;
    // A full unit block already rests on the ground under the default transform (no extra lift).
    assert!(min_offset_y.abs() < 1e-4, "block needs no lift");
    let transform = base_transform([0.0, 64.0, 0.0], 0.0, 0, min_offset_y) * ground_matrix;
    let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
    let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.0));
    let top = transform.transform_point3(Vec3::new(1.0, 1.0, 1.0));
    assert!((bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4, "bottom y");
    assert!((top.y - (64.0 + bob + 0.3125)).abs() < 1e-4, "top y");
}

#[test]
fn flat_ground_transform_lifts_the_slab_to_rest_on_the_ground() {
    let ground_matrix = display_matrix(&GENERATED_GROUND_FALLBACK, false);
    let (min_y, _) = ground_model_bounds(&generated_slab_quads(), ground_matrix);
    let min_offset_y = -min_y + 1.0 / 16.0;
    // The 0.5-scaled slab's bottom sits at -0.125, so vanilla lifts it 0.1875 to rest on the ground.
    assert!((min_offset_y - 0.1875).abs() < 1e-4, "flat lift");
    let transform = base_transform([0.0, 64.0, 0.0], 0.0, 0, min_offset_y) * ground_matrix;
    let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
    let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.5));
    assert!(
        (bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4,
        "flat bottom y"
    );
}

#[test]
fn custom_ground_transform_reseats_from_its_own_bounds() {
    // A ground transform scaling the block to 0.5 (vs the default 0.25) drops its bottom to -0.25,
    // so vanilla's `-minY + 1/16` lift grows to 0.3125 to keep it resting on the ground — proving the
    // seating is derived per-model from the actual transform, not a hardcoded constant.
    let custom = BlockModelDisplayTransform {
        rotation: [0.0, 0.0, 0.0],
        translation: [0.0, 0.0, 0.0],
        scale: [0.5, 0.5, 0.5],
    };
    let (min_y, _) = ground_model_bounds(&unit_block_quads(), display_matrix(&custom, false));
    assert!(
        (min_y + 0.25).abs() < 1e-4,
        "0.5-scaled block bottom at -0.25"
    );
    let min_offset_y = -min_y + 1.0 / 16.0;
    assert!(
        (min_offset_y - 0.3125).abs() < 1e-4,
        "lift compensates the lower bottom"
    );
}

#[test]
fn rendered_amount_matches_vanilla_thresholds() {
    assert_eq!(rendered_amount(0), 1);
    assert_eq!(rendered_amount(1), 1);
    assert_eq!(rendered_amount(2), 2);
    assert_eq!(rendered_amount(16), 2);
    assert_eq!(rendered_amount(17), 3);
    assert_eq!(rendered_amount(32), 3);
    assert_eq!(rendered_amount(33), 4);
    assert_eq!(rendered_amount(48), 4);
    assert_eq!(rendered_amount(49), 5);
    assert_eq!(rendered_amount(64), 5);
}

#[test]
fn ground_model_bounds_classify_block_vs_flat_depth() {
    // A cube face spanning Z 0..16, scaled 0.25, is 0.25 deep → scatter branch.
    let (_, block_depth) = ground_model_bounds(
        &unit_block_quads(),
        display_matrix(&BLOCK_GROUND_FALLBACK, false),
    );
    assert!((block_depth - 0.25).abs() < 1e-6);
    assert!(block_depth > FLAT_ITEM_DEPTH_THRESHOLD);
    // A generated slab spans Z 7.5..8.5 (depth 1), scaled 0.5 → 0.03125 deep → stack branch.
    let (_, flat_depth) = ground_model_bounds(
        &generated_slab_quads(),
        display_matrix(&GENERATED_GROUND_FALLBACK, false),
    );
    assert!((flat_depth - 0.031_25).abs() < 1e-6);
    assert!(flat_depth <= FLAT_ITEM_DEPTH_THRESHOLD);
}

#[test]
fn custom_head_skull_items_are_reserved_for_the_skull_renderer() {
    for resource_id in [
        "minecraft:skeleton_skull",
        "minecraft:wither_skeleton_skull",
        "minecraft:player_head",
        "minecraft:zombie_head",
        "minecraft:creeper_head",
        "minecraft:dragon_head",
        "minecraft:piglin_head",
    ] {
        assert!(
            is_custom_head_skull_item_id(resource_id),
            "{resource_id} should not use the generic CustomHeadLayer item path"
        );
    }

    assert!(!is_custom_head_skull_item_id("minecraft:carved_pumpkin"));
    assert!(!is_custom_head_skull_item_id("minecraft:white_banner"));
}

#[test]
fn armor_stand_invisible_marker_keeps_held_and_custom_head_item_models() {
    // Vanilla `LivingEntityRenderer.submit` still runs armor-stand `ItemInHandLayer` and the
    // generic item branch of `CustomHeadLayer` when the marker base body has no render type.
    let root = unique_item_model_temp_dir("armor-stand-invisible-held-items");
    write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 601;
    const ARMOR_STAND_ENTITY_TYPE_ID: i32 = 5;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, ARMOR_STAND_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 1)));

    let visible = EntityModelInstance::armor_stand_with_marker(
        ENTITY_ID,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        true,
        bbb_renderer::DEFAULT_ARMOR_STAND_MODEL_POSE,
    );
    let hidden_glowing = visible.with_invisible(true).with_outline_color(0xff55_aa11);
    let terrain_textures = TerrainTextureState::default();

    let visible_models =
        held_item_models(&[visible], &world, Some(&item_runtime), &terrain_textures);
    let hidden_models = held_item_models(
        &[hidden_glowing],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert!(visible_models.block_meshes.is_empty());
    assert!(hidden_models.block_meshes.is_empty());
    assert_eq!(visible_models.flat_meshes.len(), 2);
    assert_eq!(hidden_models.flat_meshes.len(), 2);
    assert!(visible_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));
    assert!(hidden_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_invisible_keeps_held_and_custom_head_item_models() {
    // Vanilla `ItemInHandLayer` and the generic item branch of `CustomHeadLayer` have no
    // `state.isInvisible` gate, so ordinary humanoid mobs keep both item-model attachments.
    let root = unique_item_model_temp_dir("zombie-invisible-held-items");
    write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 602;
    const ZOMBIE_ENTITY_TYPE_ID: i32 = 150;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, ZOMBIE_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 1)));

    let visible = EntityModelInstance::zombie(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let hidden = visible.with_invisible(true);
    let hidden_glowing = visible.with_invisible(true).with_outline_color(0xff55_aa11);
    let terrain_textures = TerrainTextureState::default();

    let visible_models =
        held_item_models(&[visible], &world, Some(&item_runtime), &terrain_textures);
    let hidden_models = held_item_models(&[hidden], &world, Some(&item_runtime), &terrain_textures);
    let glowing_models = held_item_models(
        &[hidden_glowing],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    for models in [&visible_models, &hidden_models, &glowing_models] {
        assert!(models.block_meshes.is_empty());
        assert_eq!(models.flat_meshes.len(), 2);
        assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_item_in_hand_layer_uses_main_arm_for_hand_slot_mapping() {
    // Vanilla `ItemInHandLayer.submit` renders RIGHT then LEFT, but each arm resolves its stack via
    // `LivingEntity.getItemHeldByArm(arm)`. A left-main-arm player therefore renders the main-hand
    // stack on the left arm and the off-hand stack on the right arm.
    let root = unique_item_model_temp_dir("humanoid-main-arm-held-items");
    write_flat_item_runtime_fixture(&root, &["main_item", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    const ENTITY_ID: i32 = 606;
    const PLAYER_ENTITY_TYPE_ID: i32 = 155;
    let terrain_textures = TerrainTextureState::default();

    let mut main_world = WorldStore::new();
    main_world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
    assert!(main_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let right_main = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let left_main = right_main.with_main_arm_left(true);
    let right_main_models = held_item_models(
        &[right_main],
        &main_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let left_main_models = held_item_models(
        &[left_main],
        &main_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    assert_eq!(right_main_models.flat_meshes.len(), 1);
    assert_eq!(left_main_models.flat_meshes.len(), 1);
    assert_ne!(
        right_main_models.flat_meshes[0], left_main_models.flat_meshes[0],
        "main-hand item moves from the right arm to the left arm when mainArm is LEFT"
    );

    let mut off_world = WorldStore::new();
    off_world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
    assert!(off_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::OffHand, 1)));
    let right_off_models = held_item_models(
        &[right_main],
        &off_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let left_off_models = held_item_models(
        &[left_main],
        &off_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    assert_eq!(right_off_models.flat_meshes.len(), 1);
    assert_eq!(left_off_models.flat_meshes.len(), 1);
    assert_ne!(
        right_off_models.flat_meshes[0], left_off_models.flat_meshes[0],
        "off-hand item moves from the left arm to the right arm when mainArm is LEFT"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_bake_ordinary_local_hand_stack() {
    let root = unique_item_model_temp_dir("first-person-basic-item");
    write_flat_item_runtime_fixture(&root, &["hand_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(CameraPose {
            position: [4.0, 64.0, -2.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        }),
        1.0,
    );

    assert!(models.block_meshes.is_empty());
    assert_eq!(models.flat_meshes.len(), 1);
    assert!(!models.flat_meshes[0].is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_local_player_whack_swing() {
    let root = unique_item_model_temp_dir("first-person-whack-swing");
    write_flat_item_runtime_fixture(&root, &["hand_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let mut world = world_with_level_dimension("minecraft:overworld");
    world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

    let still = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(still.flat_meshes.len(), 1);

    assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
    world.advance_entity_client_animations(2);
    let swinging = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(swinging.flat_meshes.len(), 1);
    assert_ne!(
        still.flat_meshes[0], swinging.flat_meshes[0],
        "ordinary first-person WHACK items should receive vanilla swingArm transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_skip_unsupported_using_and_missing_map_data() {
    let root = unique_item_model_temp_dir("first-person-special-skip");
    write_flat_item_runtime_fixture(&root, &["hand_item", "filled_map"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let hand_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut filled_map = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:filled_map"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    filled_map.component_patch.map_id = Some(7);
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: hand_item,
    });
    using_world.set_local_using_item(true);
    assert!(first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    )
    .flat_meshes
    .is_empty());

    let mut special_world = WorldStore::new();
    special_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: filled_map,
    });
    assert!(first_person_item_models(
        &special_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    )
    .flat_meshes
    .is_empty());
    assert!(first_person_item_models(
        &special_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    )
    .map_surfaces
    .is_empty());
    let missing_map = first_person_item_models(
        &special_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(missing_map.map_background_surfaces.len(), 1);
    assert_eq!(
        missing_map.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Plain
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_render_two_handed_filled_map_surface() {
    let root = unique_item_model_temp_dir("first-person-map-two-handed");
    write_flat_item_runtime_fixture(&root, &["filled_map"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let packed_grass_high = (1 << 2) | 2;
    let map = first_person_test_map_stack(&item_runtime, 7);
    let pose = CameraPose {
        position: [2.0, 65.0, -4.0],
        y_rot: 35.0,
        x_rot: -15.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: map });
    assert!(world.apply_map_item_data(MapItemData {
        map_id: 7,
        scale: 0,
        locked: false,
        decorations: Some(Vec::new()),
        color_patch: Some(MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 1,
            height: 1,
            colors: vec![packed_grass_high],
        }),
    }));

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(pose),
        1.0,
    );

    assert!(models.block_meshes.is_empty());
    assert!(models.flat_meshes.is_empty());
    assert_eq!(models.map_background_surfaces.len(), 1);
    assert_eq!(
        models.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(
        models.map_background_surfaces[0].submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            0.0,
            0.0,
        )
    );
    assert_eq!(models.map_textures.len(), 1);
    assert_eq!(models.map_textures[0].map_id, 7);
    assert_eq!(
        &models.map_textures[0].rgba[0..4],
        &map_color_rgba8(packed_grass_high)
    );
    assert_eq!(models.map_surfaces.len(), 1);
    let surface = &models.map_surfaces[0];
    assert!(!surface.is_empty());
    assert_eq!(surface.vertex_count(), 4);
    assert_eq!(surface.index_count(), 6);
    assert_eq!(surface.submission.map_id, 7);
    assert_eq!(surface.submission.texture.vanilla_path(), "minecraft:map/7");
    assert_eq!(surface.submission.light, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    assert_eq!(
        surface.submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            0.0,
            0.0,
        )
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_render_one_handed_offhand_filled_map_surface() {
    let root = unique_item_model_temp_dir("first-person-map-one-handed");
    write_flat_item_runtime_fixture(&root, &["filled_map"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let map = first_person_test_map_stack(&item_runtime, 8);
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 20.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: map,
    });
    assert!(world.apply_map_item_data(MapItemData {
        map_id: 8,
        scale: 0,
        locked: false,
        decorations: Some(Vec::new()),
        color_patch: Some(MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 1,
            height: 1,
            colors: vec![(2 << 2) | 1],
        }),
    }));

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(pose),
        1.0,
    );

    assert!(models.flat_meshes.is_empty());
    assert_eq!(models.map_background_surfaces.len(), 1);
    assert_eq!(
        models.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(models.map_surfaces.len(), 1);
    let expected = first_person_one_handed_map_transform(
        first_person_camera_world_transform(pose),
        true,
        0.0,
        0.0,
    );
    assert_eq!(models.map_surfaces[0].submission.transform, expected);
    assert_ne!(
        models.map_surfaces[0].submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            0.0,
            0.0,
        ),
        "offhand maps use vanilla's one-handed map branch"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_player_arms_render_empty_main_hand_with_player_skin_light_and_parts() {
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 20.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let world = WorldStore::new();
    let parts =
        PlayerModelPartVisibility::from_vanilla_mask(PlayerModelPartVisibility::RIGHT_SLEEVE_MASK);
    let skin = EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve);
    let player = EntityModelInstance::player_with_skin(1, [0.0, 64.0, 0.0], 0.0, skin, parts)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20));

    let arms = first_person_player_arms(&world, None, Some(&player), Some(pose), 1.0);

    assert_eq!(arms.len(), 1);
    assert!(!arms[0].left);
    assert_eq!(arms[0].skin, skin);
    assert!(arms[0].sleeve_visible);
    assert_eq!(arms[0].light, [5.0 / 15.0, 11.0 / 15.0]);
    assert!(arms[0].transform.abs_diff_eq(
        first_person_render_player_arm_transform(
            first_person_camera_world_transform(pose),
            false,
            0.0,
            0.0,
        ),
        1.0e-5,
    ));

    assert!(
        first_person_player_arms(
            &world,
            None,
            Some(&player.with_invisible(true)),
            Some(pose),
            1.0,
        )
        .is_empty(),
        "vanilla renderArmWithItem skips empty-hand player arms while invisible",
    );
    assert!(first_person_player_arms(&world, None, Some(&player), None, 1.0).is_empty());
}

#[test]
fn first_person_player_arms_render_two_handed_and_one_handed_map_arms() {
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 20.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let camera_world = first_person_camera_world_transform(pose);
    let parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::LEFT_SLEEVE_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
    );
    let player = EntityModelInstance::player_with_parts(1, [0.0, 64.0, 0.0], 0.0, false, parts);
    let mut map = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    map.component_patch.map_id = Some(7);

    let mut main_map_world = WorldStore::new();
    main_map_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: map.clone(),
    });
    let main_map_arms =
        first_person_player_arms(&main_map_world, None, Some(&player), Some(pose), 1.0);
    assert_eq!(main_map_arms.len(), 2);
    assert_eq!(
        main_map_arms.iter().map(|arm| arm.left).collect::<Vec<_>>(),
        vec![false, true],
    );
    assert!(main_map_arms[0].sleeve_visible);
    assert!(main_map_arms[1].sleeve_visible);
    assert!(main_map_arms[0].transform.abs_diff_eq(
        first_person_two_handed_map_arm_transform(camera_world, pose.x_rot, 0.0, 0.0, false),
        1.0e-5,
    ));
    assert!(main_map_arms[1].transform.abs_diff_eq(
        first_person_two_handed_map_arm_transform(camera_world, pose.x_rot, 0.0, 0.0, true),
        1.0e-5,
    ));

    let mut offhand_map_world = WorldStore::new();
    offhand_map_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: map,
    });
    let offhand_map_arms =
        first_person_player_arms(&offhand_map_world, None, Some(&player), Some(pose), 1.0);
    assert_eq!(
        offhand_map_arms
            .iter()
            .map(|arm| arm.left)
            .collect::<Vec<_>>(),
        vec![false, true],
        "offhand maps leave the empty main hand visible and render one map hand",
    );
    assert!(offhand_map_arms[0].transform.abs_diff_eq(
        first_person_render_player_arm_transform(camera_world, false, 0.0, 0.0),
        1.0e-5,
    ));
    assert!(offhand_map_arms[1].transform.abs_diff_eq(
        first_person_one_handed_map_arm_transform(camera_world, true, 0.0, 0.0),
        1.0e-5,
    ));
}

#[test]
fn first_person_item_models_render_filled_map_decorations_and_text() {
    let item_runtime = NativeItemRuntime::empty_for_test();
    let mut map = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    map.component_patch.map_id = Some(9);
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: map });
    assert!(world.apply_map_item_data(MapItemData {
        map_id: 9,
        scale: 0,
        locked: false,
        decorations: Some(vec![
            bbb_protocol::packets::MapDecoration {
                type_id: 0,
                x: 0,
                y: 0,
                rot: 0,
                name: Some("Player".to_string()),
            },
            bbb_protocol::packets::MapDecoration {
                type_id: 1,
                x: -20,
                y: 30,
                rot: 7,
                name: Some("Frame".to_string()),
            },
        ]),
        color_patch: Some(MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 1,
            height: 1,
            colors: vec![(1 << 2) | 2],
        }),
    }));

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(pose),
        1.0,
    );

    assert_eq!(models.map_surfaces.len(), 1);
    assert_eq!(models.map_background_surfaces.len(), 1);
    assert_eq!(
        models.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(models.map_decoration_surfaces.len(), 2);
    assert_eq!(models.map_text_surfaces.len(), 2);
    let player = &models.map_decoration_surfaces[0];
    assert_eq!(player.submission.type_id, 0);
    assert_eq!(
        player.submission.texture.vanilla_sprite_id(),
        "minecraft:player"
    );
    assert_eq!(
        (player.submission.order, player.submission.submit_sequence),
        (0, 1)
    );
    assert_eq!(player.submission.decoration_index, 0);
    let frame = &models.map_decoration_surfaces[1];
    assert_eq!(frame.submission.type_id, 1);
    assert_eq!(
        frame.submission.texture.vanilla_sprite_id(),
        "minecraft:frame"
    );
    assert_eq!(
        (frame.submission.order, frame.submission.submit_sequence),
        (0, 2)
    );
    assert_eq!(frame.submission.decoration_index, 1);
    assert_eq!(models.map_text_surfaces[0].submission.type_id, 0);
    assert_eq!(models.map_text_surfaces[0].submission.text, "Player");
    assert_eq!(
        (
            models.map_text_surfaces[0].submission.order,
            models.map_text_surfaces[0].submission.submit_sequence
        ),
        (1, 0)
    );
    assert_eq!(models.map_text_surfaces[1].submission.type_id, 1);
    assert_eq!(models.map_text_surfaces[1].submission.text, "Frame");
    assert_eq!(
        (
            models.map_text_surfaces[1].submission.order,
            models.map_text_surfaces[1].submission.submit_sequence
        ),
        (1, 1)
    );
}

#[test]
fn first_person_item_models_render_idle_spyglass_and_hide_when_scoping() {
    let root = unique_item_model_temp_dir("first-person-spyglass-use");
    write_flat_item_runtime_fixture(&root, &["spyglass", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let spyglass = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:spyglass"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: spyglass.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut scoping_world = WorldStore::new();
    scoping_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: spyglass,
    });
    scoping_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    scoping_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    assert!(
        first_person_item_models(
            &scoping_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        )
        .flat_meshes
        .is_empty(),
        "vanilla ItemInHandRenderer skips hands/items while player.isScoping"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_block_use_pose_for_shield_and_blocks_attacks() {
    let root = unique_item_model_temp_dir("first-person-block-use");
    write_flat_item_runtime_fixture(&root, &["shield", "guard_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut shield = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:shield"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    shield.component_patch.added = 1;
    shield.component_patch.added_type_ids = vec![VANILLA_SWING_ANIMATION_COMPONENT_ID];
    shield.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 0,
    });
    let mut guard_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:guard_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    guard_item.component_patch.added = 1;
    guard_item.component_patch.added_type_ids = vec![VANILLA_BLOCKS_ATTACKS_COMPONENT_ID];
    let mut consumable_guard = guard_item.clone();
    consumable_guard
        .component_patch
        .added_type_ids
        .push(VANILLA_CONSUMABLE_COMPONENT_ID);
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut shield_world = WorldStore::new();
    shield_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: shield,
    });
    let idle_shield = first_person_item_models(
        &shield_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle_shield.flat_meshes.len(), 1);
    shield_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let blocking_shield = first_person_item_models(
        &shield_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(blocking_shield.flat_meshes.len(), 1);
    assert_eq!(
        idle_shield.flat_meshes[0], blocking_shield.flat_meshes[0],
        "vanilla ShieldItem BLOCK use keeps only the base arm transform"
    );

    let mut idle_guard_world = WorldStore::new();
    idle_guard_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: guard_item.clone(),
    });
    let idle_guard = first_person_item_models(
        &idle_guard_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle_guard.flat_meshes.len(), 1);

    let mut blocking_guard_world = WorldStore::new();
    blocking_guard_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: guard_item,
    });
    blocking_guard_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let blocking_guard = first_person_item_models(
        &blocking_guard_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(blocking_guard.flat_meshes.len(), 1);
    assert_ne!(
        idle_guard.flat_meshes[0], blocking_guard.flat_meshes[0],
        "non-shield BLOCK use applies ItemInHandRenderer's fixed BLOCK transform"
    );

    assert_eq!(
        first_person_stack_block_use_kind(&consumable_guard, &item_runtime),
        None,
        "CONSUMABLE wins over BLOCKS_ATTACKS in Item.getUseAnimation"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_consumable_eat_and_drink_use_pose() {
    let root = unique_item_model_temp_dir("first-person-consumable-use");
    write_flat_item_runtime_fixture(&root, &["snack"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut snack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:snack"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    snack.component_patch.added = 1;
    snack.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    snack.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut eating_world = WorldStore::new();
    eating_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack.clone(),
    });
    eating_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    eating_world.advance_local_using_item_ticks(12);
    let eating = first_person_item_models(
        &eating_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(eating.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], eating.flat_meshes[0],
        "consumable EAT applies ItemInHandRenderer.applyEatTransform before the arm transform"
    );

    let mut drink = snack;
    drink.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Drink,
    });
    let mut drinking_world = WorldStore::new();
    drinking_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: drink,
    });
    drinking_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    drinking_world.advance_local_using_item_ticks(12);
    let drinking = first_person_item_models(
        &drinking_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(drinking.flat_meshes.len(), 1);
    assert_eq!(
        eating.flat_meshes[0], drinking.flat_meshes[0],
        "vanilla EAT and DRINK share the first-person applyEatTransform matrix"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_default_consumable_use_pose() {
    let root = unique_item_model_temp_dir("first-person-default-consumable-use");
    write_default_consumable_item_runtime_fixture(&root, "snack");
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let snack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:snack"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    assert_eq!(
        snack
            .item_id
            .and_then(|item_id| item_runtime.item_default_consumable(item_id)),
        Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        })
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut eating_world = WorldStore::new();
    eating_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack,
    });
    eating_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    eating_world.advance_local_using_item_ticks(12);
    let eating = first_person_item_models(
        &eating_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(eating.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], eating.flat_meshes[0],
        "default item prototype CONSUMABLE applies vanilla EAT first-person use pose"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_custom_consumable_non_eat_use_animations() {
    let root = unique_item_model_temp_dir("first-person-custom-consumable-use");
    write_flat_item_runtime_fixture(&root, &["snack", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let snack_bow = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:snack",
        ItemUseAnimationSummary::Bow,
        1.6,
    );
    assert_eq!(
        first_person_stack_supported_use_animation(&snack_bow, &item_runtime),
        Some(FirstPersonUseAnimation::Bow {
            use_duration_ticks: 32.0,
        })
    );
    assert_eq!(
        first_person_stack_supported_use_animation(
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Trident,
                1.6,
            ),
            &item_runtime,
        ),
        Some(FirstPersonUseAnimation::Trident {
            use_duration_ticks: 32.0,
        })
    );
    assert_eq!(
        first_person_stack_supported_use_animation(
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Brush,
                1.6,
            ),
            &item_runtime,
        ),
        Some(FirstPersonUseAnimation::Brush {
            use_duration_ticks: 32.0,
        })
    );
    assert_eq!(
        first_person_stack_supported_use_animation(
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Bundle,
                1.6,
            ),
            &item_runtime,
        ),
        Some(FirstPersonUseAnimation::Bundle)
    );
    for animation in [
        ItemUseAnimationSummary::None,
        ItemUseAnimationSummary::Crossbow,
        ItemUseAnimationSummary::Spyglass,
        ItemUseAnimationSummary::TootHorn,
    ] {
        assert_eq!(
            first_person_stack_supported_use_animation(
                &first_person_test_consumable_stack(
                    &item_runtime,
                    "minecraft:snack",
                    animation,
                    1.6,
                ),
                &item_runtime,
            ),
            Some(FirstPersonUseAnimation::None),
            "generic {animation:?} has no ItemInHandRenderer switch case"
        );
    }
    assert_eq!(
        first_person_stack_supported_use_animation(
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Spear,
                1.6,
            ),
            &item_runtime,
        ),
        None,
        "SPEAR needs the kinetic SpearAnimations.firstPersonUse path"
    );

    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack_bow.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack_bow,
    });
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(
        using.flat_meshes.len(),
        2,
        "generic BOW animation is not an Items.BOW/CROSSBOW hand-selection special case"
    );
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "generic BOW consumables still apply the vanilla draw transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_goat_horn_base_use_pose() {
    let root = unique_item_model_temp_dir("first-person-goat-horn-use");
    write_flat_item_runtime_fixture(&root, &["goat_horn"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut goat_horn = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:goat_horn"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    goat_horn.component_patch.added = 1;
    goat_horn.component_patch.added_type_ids = vec![VANILLA_SWING_ANIMATION_COMPONENT_ID];
    goat_horn.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 0,
    });
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: goat_horn.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut tooting_world = WorldStore::new();
    tooting_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: goat_horn,
    });
    tooting_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let tooting = first_person_item_models(
        &tooting_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(tooting.flat_meshes.len(), 1);
    assert_eq!(
        idle.flat_meshes[0], tooting.flat_meshes[0],
        "vanilla TOOT_HORN first-person use keeps only the base arm transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_brush_use_pose() {
    let root = unique_item_model_temp_dir("first-person-brush-use");
    write_flat_item_runtime_fixture(&root, &["brush"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let brush = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:brush"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut edible_brush = brush.clone();
    edible_brush.component_patch.added = 1;
    edible_brush.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    edible_brush.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(&edible_brush, &item_runtime),
        Some(FirstPersonUseAnimation::Brush {
            use_duration_ticks: VANILLA_BRUSH_USE_DURATION_TICKS,
        }),
        "BrushItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: brush.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut brushing_world = WorldStore::new();
    brushing_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: brush,
    });
    brushing_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    brushing_world.advance_local_using_item_ticks(12);
    let brushing = first_person_item_models(
        &brushing_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(brushing.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], brushing.flat_meshes[0],
        "BRUSH use applies ItemInHandRenderer.applyBrushTransform after the arm transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_bundle_use_swing() {
    let root = unique_item_model_temp_dir("first-person-bundle-use");
    write_flat_item_runtime_fixture(&root, &["bundle"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let bundle = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:bundle"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut edible_bundle = bundle.clone();
    edible_bundle.component_patch.added = 1;
    edible_bundle.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    edible_bundle.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(&edible_bundle, &item_runtime),
        Some(FirstPersonUseAnimation::Bundle),
        "BundleItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut world = world_with_level_dimension("minecraft:overworld");
    world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: bundle,
    });
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let still = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(still.flat_meshes.len(), 1);

    assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
    world.advance_entity_client_animations(2);
    let swinging = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(swinging.flat_meshes.len(), 1);
    assert_ne!(
        still.flat_meshes[0], swinging.flat_meshes[0],
        "BUNDLE use applies ItemInHandRenderer.swingArm while the item is being used"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_trident_use_pose() {
    let root = unique_item_model_temp_dir("first-person-trident-use");
    write_flat_item_runtime_fixture(&root, &["trident"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let trident = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:trident"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut edible_trident = trident.clone();
    edible_trident.component_patch.added = 1;
    edible_trident.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    edible_trident.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(&edible_trident, &item_runtime),
        Some(FirstPersonUseAnimation::Trident {
            use_duration_ticks: VANILLA_TRIDENT_USE_DURATION_TICKS,
        }),
        "TridentItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: trident.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: trident,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(using.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "TRIDENT use applies ItemInHandRenderer's throw-charge transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_bow_use_pose_and_hand_selection() {
    let root = unique_item_model_temp_dir("first-person-bow-use");
    write_flat_item_runtime_fixture(&root, &["bow", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let bow = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:bow"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut edible_bow = bow.clone();
    edible_bow.component_patch.added = 1;
    edible_bow.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    edible_bow.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(&edible_bow, &item_runtime),
        Some(FirstPersonUseAnimation::Bow {
            use_duration_ticks: VANILLA_BOW_USE_DURATION_TICKS,
        }),
        "BowItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: bow.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: bow });
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(
        using.flat_meshes.len(),
        1,
        "vanilla renders only the used hand while drawing a bow"
    );
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "BOW use applies ItemInHandRenderer's draw transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_use_duration_range_dispatch_reads_local_use_ticks() {
    // Vanilla `ItemInHandRenderer.renderArmWithItem` passes the local player
    // as item owner for first-person stack rendering; `UseDuration.get` then
    // reads that local player's active use item and elapsed ticks.
    let root = unique_item_model_temp_dir("first-person-use-duration-range-dispatch");
    write_use_duration_selector_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:use_duration_selector",
        ItemUseAnimationSummary::None,
        2.0,
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let render = |using_ticks: Option<u32>| {
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: stack.clone(),
        });
        if let Some(ticks) = using_ticks {
            world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
            world.advance_local_using_item_ticks(ticks);
        }
        let mut models = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes.remove(0)
    };

    let idle = render(None);
    let using_start = render(Some(0));
    let using_mid = render(Some(13));
    let using_full = render(Some(18));

    assert_ne!(idle, using_start);
    assert_ne!(using_start, using_mid);
    assert_ne!(using_mid, using_full);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_crossbow_use_and_charged_pose() {
    let root = unique_item_model_temp_dir("first-person-crossbow-use");
    write_flat_item_runtime_fixture(&root, &["crossbow", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let crossbow = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:crossbow"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let mut edible_crossbow = crossbow.clone();
    edible_crossbow.component_patch.added = 1;
    edible_crossbow.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
    edible_crossbow.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(&edible_crossbow, &item_runtime),
        Some(FirstPersonUseAnimation::Crossbow {
            use_duration_ticks: VANILLA_CROSSBOW_USE_DURATION_TICKS,
            charge_duration_ticks: VANILLA_CROSSBOW_CHARGE_DURATION_TICKS,
        }),
        "CrossbowItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: crossbow.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: crossbow.clone(),
    });
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(
        using.flat_meshes.len(),
        1,
        "vanilla renders only the used hand while drawing a crossbow"
    );
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "uncharged CROSSBOW use applies ItemInHandRenderer's draw transform"
    );

    let mut charged_crossbow = crossbow;
    charged_crossbow.component_patch.charged_projectiles_items = vec![ItemStackTemplateSummary {
        item_id: 99,
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    }];
    let mut charged_world = WorldStore::new();
    charged_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: charged_crossbow,
    });
    let charged = first_person_item_models(
        &charged_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(charged.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], charged.flat_meshes[0],
        "charged main-hand crossbows apply the vanilla idle hold offset"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_stack_supported_use_animation_resolves_spear_kinetic_weapon() {
    let root = unique_item_model_temp_dir("first-person-spear-use-animation");
    write_flat_item_runtime_fixture(&root, &["wooden_spear", "snack"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let kinetic_weapon = SpearKineticWeapon {
        delay_ticks: 15.0,
        dismount_duration_ticks: 100.0,
        knockback_duration_ticks: 200.0,
        damage_duration_ticks: 300.0,
        forward_movement: 0.38,
    };
    let wooden_spear = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    assert_eq!(
        first_person_stack_supported_use_animation(&wooden_spear, &item_runtime),
        Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon,
            use_duration_ticks: VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
        })
    );

    let mut removed_kinetic = wooden_spear.clone();
    removed_kinetic
        .component_patch
        .removed_type_ids
        .push(VANILLA_KINETIC_WEAPON_COMPONENT_ID);
    assert_eq!(
        first_person_stack_supported_use_animation(&removed_kinetic, &item_runtime),
        None,
        "removing the prototype KINETIC_WEAPON component disables vanilla SPEAR use"
    );

    let snack_spear = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:snack",
        ItemUseAnimationSummary::Spear,
        1.6,
    );
    assert_eq!(
        first_person_stack_supported_use_animation(&snack_spear, &item_runtime),
        None,
        "generic SPEAR consumables need readable kinetic weapon data"
    );

    let wooden_spear_consumable = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:wooden_spear",
        ItemUseAnimationSummary::Spear,
        1.6,
    );
    assert_eq!(
        first_person_stack_supported_use_animation(&wooden_spear_consumable, &item_runtime),
        Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon,
            use_duration_ticks: 32.0,
        }),
        "consumable SPEAR keeps the consumable use duration while using the item kinetic component"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_spear_kinetic_use_pose_and_feedback() {
    let root = unique_item_model_temp_dir("first-person-spear-kinetic-use");
    write_flat_item_runtime_fixture(&root, &["wooden_spear"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let wooden_spear = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let mut idle_world = world_with_level_dimension("minecraft:overworld");
    idle_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: wooden_spear.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = world_with_level_dimension("minecraft:overworld");
    using_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: wooden_spear.clone(),
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(20);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(using.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "SPEAR use skips applyItemArmTransform and applies SpearAnimations.firstPersonUse"
    );

    let mut feedback_world = world_with_level_dimension("minecraft:overworld");
    feedback_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    feedback_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: wooden_spear,
    });
    feedback_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    feedback_world.advance_local_using_item_ticks(20);
    assert!(feedback_world.apply_entity_event(EntityEvent {
        entity_id: 1,
        event_id: 2,
    }));
    feedback_world.advance_entity_client_animations(2);
    let feedback = first_person_item_models(
        &feedback_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(feedback.flat_meshes.len(), 1);
    assert_eq!(
        feedback_world.local_player_ticks_since_kinetic_hit_feedback(0.5),
        2.5
    );
    assert_ne!(
        using.flat_meshes[0], feedback.flat_meshes[0],
        "local kinetic hit feedback contributes the vanilla first-person spear kick"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_stab_swing_for_spear_and_stack_patch() {
    let root = unique_item_model_temp_dir("first-person-stab-swing");
    write_flat_item_runtime_fixture(&root, &["stab_item", "wooden_spear"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut patched_stab = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:stab_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    patched_stab.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Stab,
        duration: 13,
    });
    let default_spear = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let assert_stab_moves = |item: ItemStackSummary, label: &str| {
        let mut world = world_with_level_dimension("minecraft:overworld");
        world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

        let still = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(still.flat_meshes.len(), 1, "{label} should render");

        assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
        world.advance_entity_client_animations(2);
        let swinging = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(
            swinging.flat_meshes.len(),
            1,
            "{label} should keep rendering"
        );
        assert_ne!(
            still.flat_meshes[0], swinging.flat_meshes[0],
            "{label} should receive vanilla SpearAnimations.firstPersonAttack"
        );
    };
    assert_stab_moves(patched_stab, "stack-patch STAB");
    assert_stab_moves(default_spear, "default spear STAB");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_arm_transform_matches_vanilla_apply_item_arm_transform() {
    let pose = CameraPose {
        position: [10.0, 64.0, -5.0],
        y_rot: 35.0,
        x_rot: -15.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let view = first_person_camera_view_matrix(pose);
    let camera_world = first_person_camera_world_transform(pose);
    let right = view * first_person_item_arm_transform_from_camera_world(camera_world, false, 0.0);
    let left = view * first_person_item_arm_transform_from_camera_world(camera_world, true, 0.0);
    let right_origin = right.transform_point3(Vec3::ZERO);
    let left_origin = left.transform_point3(Vec3::ZERO);

    assert!(
        right_origin.abs_diff_eq(Vec3::new(0.56, -0.52, -0.72), 1.0e-4),
        "right origin {right_origin:?}"
    );
    assert!(
        left_origin.abs_diff_eq(Vec3::new(-0.56, -0.52, -0.72), 1.0e-4),
        "left origin {left_origin:?}"
    );
}

#[test]
fn first_person_player_arm_transform_matches_vanilla_render_player_arm() {
    let attack = 0.25_f32;
    let inverse_arm_height = 0.2_f32;
    let expected = |arm_left: bool| {
        let invert = if arm_left { -1.0 } else { 1.0 };
        let sqrt_attack = attack.sqrt();
        let x_swing_position = -0.3 * (sqrt_attack * std::f32::consts::PI).sin();
        let y_swing_position = 0.4 * (sqrt_attack * std::f32::consts::TAU).sin();
        let z_swing_position = -0.4 * (attack * std::f32::consts::PI).sin();
        let z_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
        let y_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();

        Mat4::from_translation(Vec3::new(
            invert * (x_swing_position + 0.64000005),
            y_swing_position - 0.6 + inverse_arm_height * -0.6,
            z_swing_position - 0.71999997,
        )) * Mat4::from_rotation_y((invert * 45.0_f32).to_radians())
            * Mat4::from_rotation_y((invert * y_swing_rotation * 70.0).to_radians())
            * Mat4::from_rotation_z((invert * z_swing_rotation * -20.0).to_radians())
            * Mat4::from_translation(Vec3::new(invert * -1.0, 3.6, 3.5))
            * Mat4::from_rotation_z((invert * 120.0_f32).to_radians())
            * Mat4::from_rotation_x(200.0_f32.to_radians())
            * Mat4::from_rotation_y((invert * -135.0_f32).to_radians())
            * Mat4::from_translation(Vec3::new(invert * 5.6, 0.0, 0.0))
    };

    assert!(first_person_render_player_arm_transform(
        Mat4::IDENTITY,
        false,
        inverse_arm_height,
        attack,
    )
    .abs_diff_eq(expected(false), 1.0e-5));
    assert!(first_person_render_player_arm_transform(
        Mat4::IDENTITY,
        true,
        inverse_arm_height,
        attack,
    )
    .abs_diff_eq(expected(true), 1.0e-5));
}

#[test]
fn first_person_map_arm_transform_matches_vanilla_render_map_hand() {
    let attack = 0.25_f32;
    let x_rot = 20.0_f32;
    let inverse_arm_height = 0.2_f32;
    let sqrt_attack = attack.sqrt();
    let y_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let z_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let map_tilt = first_person_map_tilt(x_rot);
    let base = Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
            -0.72,
        ))
        * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
        * Mat4::from_rotation_y(90.0_f32.to_radians());
    let expected_map_hand = |arm_left: bool| {
        let invert = if arm_left { -1.0 } else { 1.0 };
        base * Mat4::from_rotation_y(92.0_f32.to_radians())
            * Mat4::from_rotation_x(45.0_f32.to_radians())
            * Mat4::from_rotation_z((invert * -41.0_f32).to_radians())
            * Mat4::from_translation(Vec3::new(invert * 0.3, -1.1, 0.45))
    };

    assert!(first_person_two_handed_map_arm_transform(
        Mat4::IDENTITY,
        x_rot,
        inverse_arm_height,
        attack,
        false,
    )
    .abs_diff_eq(expected_map_hand(false), 1.0e-5));
    assert!(first_person_two_handed_map_arm_transform(
        Mat4::IDENTITY,
        x_rot,
        inverse_arm_height,
        attack,
        true,
    )
    .abs_diff_eq(expected_map_hand(true), 1.0e-5));
}

#[test]
fn first_person_map_tilt_matches_vanilla_calculate_map_tilt() {
    assert!((first_person_map_tilt(0.0) - 1.0).abs() < 1.0e-6);
    let mid_tilt = 1.0 - 45.0 / 45.0 + 0.1;
    let expected_mid = -(mid_tilt * std::f32::consts::PI).cos() * 0.5 + 0.5;
    assert!((first_person_map_tilt(45.0) - expected_mid).abs() < 1.0e-6);
    assert!((first_person_map_tilt(90.0) - 0.0).abs() < 1.0e-6);
}

#[test]
fn first_person_one_handed_map_transform_matches_vanilla_render_one_handed_map() {
    let attack = 0.25_f32;
    let invert = -1.0_f32;
    let sqrt_attack = attack.sqrt();
    let x_swing = (sqrt_attack * std::f32::consts::PI).sin();
    let expected = Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
        * Mat4::from_translation(Vec3::new(invert * 0.51, -0.08, -0.75))
        * Mat4::from_translation(Vec3::new(
            invert * -0.5 * x_swing,
            0.4 * (sqrt_attack * std::f32::consts::TAU).sin() - 0.3 * x_swing,
            -0.3 * (attack * std::f32::consts::PI).sin(),
        ))
        * Mat4::from_rotation_x((x_swing * -45.0).to_radians())
        * Mat4::from_rotation_y((invert * x_swing * -30.0).to_radians())
        * Mat4::from_rotation_y(180.0_f32.to_radians())
        * Mat4::from_rotation_z(180.0_f32.to_radians())
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_SCALE))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE));
    let transformed = first_person_one_handed_map_transform(Mat4::IDENTITY, true, 0.0, attack);
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));
}

#[test]
fn first_person_two_handed_map_transform_matches_vanilla_render_two_handed_map() {
    let attack = 0.25_f32;
    let x_rot = 20.0_f32;
    let sqrt_attack = attack.sqrt();
    let map_tilt = first_person_map_tilt(x_rot);
    let expected = Mat4::from_translation(Vec3::new(
        0.0,
        -(-0.2 * (attack * std::f32::consts::PI).sin()) / 2.0,
        -0.4 * (sqrt_attack * std::f32::consts::PI).sin(),
    )) * Mat4::from_translation(Vec3::new(0.0, 0.04 + map_tilt * -0.5, -0.72))
        * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
        * Mat4::from_rotation_x(((sqrt_attack * std::f32::consts::PI).sin() * 20.0).to_radians())
        * Mat4::from_scale(Vec3::splat(2.0))
        * Mat4::from_rotation_y(180.0_f32.to_radians())
        * Mat4::from_rotation_z(180.0_f32.to_radians())
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_SCALE))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE));
    let transformed = first_person_two_handed_map_transform(Mat4::IDENTITY, x_rot, 0.0, attack);
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));
}

#[test]
fn first_person_whack_swing_transform_matches_vanilla_swing_arm() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let attack = 0.25_f32;
    let swung = first_person_apply_whack_swing(base, false, attack);
    let origin = swung.transform_point3(Vec3::ZERO);
    let sqrt_attack = attack.sqrt();
    let x_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position = 0.2 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
    let expected_origin = Vec3::new(
        0.56 + x_swing_position,
        -0.52 + y_swing_position,
        -0.72 + z_swing_position,
    );
    assert!(
        origin.abs_diff_eq(expected_origin, 1.0e-4),
        "origin {origin:?}, expected {expected_origin:?}"
    );

    let expected = base
        * Mat4::from_translation(expected_origin - Vec3::new(0.56, -0.52, -0.72))
        * Mat4::from_rotation_y(
            (45.0 + (attack * attack * std::f32::consts::PI).sin() * -20.0).to_radians(),
        )
        * Mat4::from_rotation_z((xz_swing_rotation * -20.0).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * -80.0).to_radians())
        * Mat4::from_rotation_y((-45.0_f32).to_radians());
    assert!(swung.abs_diff_eq(expected, 1.0e-5));

    let runtime = NativeItemRuntime::empty_for_test();
    let mut none_stack = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    none_stack.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 6,
    });
    assert_eq!(
        first_person_stack_swing_animation(&none_stack, &runtime),
        FirstPersonSwingAnimation::None
    );

    none_stack.component_patch.removed_type_ids = vec![VANILLA_SWING_ANIMATION_COMPONENT_ID];
    assert_eq!(
        first_person_stack_swing_animation(&none_stack, &runtime),
        FirstPersonSwingAnimation::Whack,
        "removing swing_animation falls back to vanilla SwingAnimation.DEFAULT WHACK",
    );
}

#[test]
fn first_person_stab_swing_transform_matches_vanilla_first_person_attack() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let attack = 0.1_f32;
    let swung = first_person_apply_stab_swing(base, false, attack);
    let starting_amount =
        -((std::f32::consts::PI * first_person_progress(attack, 0.0, 0.05)).cos() - 1.0) / 2.0;
    let middle_progress = first_person_progress(attack, 0.05, 0.2);
    let middle_amount =
        1.0 + 2.70158 * (middle_progress - 1.0).powi(3) + 1.70158 * (middle_progress - 1.0).powi(2);
    let ending_amount = 0.0;

    let expected =
        base * Mat4::from_translation(Vec3::new(
            0.1 * (starting_amount - middle_amount),
            -0.075 * (starting_amount - ending_amount),
            0.65 * (starting_amount - middle_amount),
        )) * Mat4::from_rotation_x((-70.0 * (starting_amount - ending_amount)).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.25 * (ending_amount - middle_amount)));
    assert!(swung.abs_diff_eq(expected, 1.0e-5));

    let left = first_person_apply_stab_swing(
        first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, true, 0.0),
        true,
        attack,
    );
    let right_origin = swung.transform_point3(Vec3::ZERO);
    let left_origin = left.transform_point3(Vec3::ZERO);
    assert!(
        (right_origin.x + left_origin.x).abs() < 1.0e-4,
        "STAB x translation mirrors between arms: {right_origin:?} vs {left_origin:?}"
    );
}

#[test]
fn first_person_block_use_transform_matches_vanilla_non_shield_block_case() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let transformed = first_person_apply_block_use_transform(base, false);
    let expected = base
        * Mat4::from_translation(Vec3::new(-0.14142136, 0.08, 0.14142136))
        * Mat4::from_rotation_x((-102.25_f32).to_radians())
        * Mat4::from_rotation_y(13.365_f32.to_radians())
        * Mat4::from_rotation_z(78.05_f32.to_radians());
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));

    let left = first_person_apply_block_use_transform(
        first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, true, 0.0),
        true,
    );
    let right_origin = transformed.transform_point3(Vec3::ZERO);
    let left_origin = left.transform_point3(Vec3::ZERO);
    assert!(
        (right_origin.x + left_origin.x).abs() < 1.0e-4,
        "BLOCK use x translation mirrors between arms: {right_origin:?} vs {left_origin:?}"
    );
}

#[test]
fn first_person_eat_drink_use_transform_matches_vanilla_apply_eat_transform() {
    let transformed =
        first_person_apply_eat_drink_use_transform(Mat4::IDENTITY, false, 32.0, 12.0, 0.5);
    let curr_usage_time = 32.0 - 12.0 - 0.5 + 1.0;
    let scaled_usage_time = curr_usage_time / 32.0;
    let extra_height_offset = (curr_usage_time / 4.0 * std::f32::consts::PI).cos().abs() * 0.1;
    let eat_jiggle = 1.0_f32 - scaled_usage_time.powf(27.0);
    let expected_eat = Mat4::from_translation(Vec3::new(0.0, extra_height_offset, 0.0))
        * Mat4::from_translation(Vec3::new(eat_jiggle * 0.6, eat_jiggle * -0.5, 0.0))
        * Mat4::from_rotation_y((eat_jiggle * 90.0).to_radians())
        * Mat4::from_rotation_x((eat_jiggle * 10.0).to_radians())
        * Mat4::from_rotation_z((eat_jiggle * 30.0).to_radians());
    let expected = first_person_item_arm_transform_from_camera_world(expected_eat, false, 0.0);
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));

    let left = first_person_apply_eat_drink_use_transform(Mat4::IDENTITY, true, 32.0, 12.0, 0.5);
    let right_origin = transformed.transform_point3(Vec3::ZERO);
    let left_origin = left.transform_point3(Vec3::ZERO);
    assert!(
        (right_origin.x + left_origin.x).abs() < 1.0e-4,
        "EAT/DRINK use x movement mirrors between arms: {right_origin:?} vs {left_origin:?}"
    );
}

#[test]
fn first_person_brush_use_transform_matches_vanilla_apply_brush_transform() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let transformed = first_person_apply_brush_use_transform(base, false, 200.0, 12.0, 0.5);
    let remaining = (200.0_f32 - 12.0).max(0.0) % 10.0;
    let delta_since_last_update = remaining - 0.5 + 1.0;
    let scaled_usage_time = 1.0 - delta_since_last_update / 10.0;
    let current_swipe_angle = -15.0 + 75.0 * (scaled_usage_time * 2.0 * std::f32::consts::PI).cos();
    let right_brush = Mat4::from_translation(Vec3::new(-0.25, 0.22, 0.35))
        * Mat4::from_rotation_x((-80.0_f32).to_radians())
        * Mat4::from_rotation_y(90.0_f32.to_radians())
        * Mat4::from_rotation_x(current_swipe_angle.to_radians());
    assert!(transformed.abs_diff_eq(base * right_brush, 1.0e-5));

    let left_base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, true, 0.0);
    let left = first_person_apply_brush_use_transform(left_base, true, 200.0, 12.0, 0.5);
    let left_brush = Mat4::from_translation(Vec3::new(0.1, 0.83, 0.35))
        * Mat4::from_rotation_x((-80.0_f32).to_radians())
        * Mat4::from_rotation_y((-90.0_f32).to_radians())
        * Mat4::from_rotation_x(current_swipe_angle.to_radians())
        * Mat4::from_translation(Vec3::new(-0.3, 0.22, 0.35));
    assert!(left.abs_diff_eq(left_base * left_brush, 1.0e-5));
}

#[test]
fn first_person_trident_use_transform_matches_vanilla_throw_charge_transform() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let transformed = first_person_apply_trident_use_transform(
        base,
        false,
        VANILLA_TRIDENT_USE_DURATION_TICKS,
        12.0,
        0.5,
    );
    let remaining_ticks = VANILLA_TRIDENT_USE_DURATION_TICKS - 12.0;
    let time_held = VANILLA_TRIDENT_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
    let power = (time_held / 10.0).min(1.0);
    let shake = ((time_held - 0.1) * 1.3).sin() * (power - 0.1);
    let expected = base
        * Mat4::from_translation(Vec3::new(-0.5, 0.7, 0.1))
        * Mat4::from_rotation_x((-55.0_f32).to_radians())
        * Mat4::from_rotation_y(35.3_f32.to_radians())
        * Mat4::from_rotation_z((-9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.2))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-45.0_f32).to_radians());
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));
}

#[test]
fn first_person_bow_use_transform_matches_vanilla_draw_transform() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let transformed = first_person_apply_bow_use_transform(
        base,
        false,
        VANILLA_BOW_USE_DURATION_TICKS,
        12.0,
        0.5,
    );
    let remaining_ticks = VANILLA_BOW_USE_DURATION_TICKS - 12.0;
    let time_held = VANILLA_BOW_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
    let mut power = time_held / 20.0;
    power = (power * power + power * 2.0) / 3.0;
    if power > 1.0 {
        power = 1.0;
    }
    let shake = ((time_held - 0.1) * 1.3).sin() * (power - 0.1);
    let expected = base
        * Mat4::from_translation(Vec3::new(-0.2785682, 0.18344387, 0.15731531))
        * Mat4::from_rotation_x((-13.935_f32).to_radians())
        * Mat4::from_rotation_y(35.3_f32.to_radians())
        * Mat4::from_rotation_z((-9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-45.0_f32).to_radians());
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));
}

#[test]
fn first_person_crossbow_transforms_match_vanilla_draw_and_charged_hold() {
    let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
    let transformed = first_person_apply_crossbow_use_transform(
        base,
        false,
        VANILLA_CROSSBOW_USE_DURATION_TICKS,
        VANILLA_CROSSBOW_CHARGE_DURATION_TICKS,
        12.0,
        0.5,
    );
    let remaining_ticks = VANILLA_CROSSBOW_USE_DURATION_TICKS - 12.0;
    let time_held = VANILLA_CROSSBOW_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
    let power = time_held / VANILLA_CROSSBOW_CHARGE_DURATION_TICKS;
    let shake = ((time_held - 0.1) * 1.3).sin() * (power - 0.1);
    let expected_draw = base
        * Mat4::from_translation(Vec3::new(-0.4785682, -0.094387, 0.05731531))
        * Mat4::from_rotation_x((-11.935_f32).to_radians())
        * Mat4::from_rotation_y(65.3_f32.to_radians())
        * Mat4::from_rotation_z((-9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-45.0_f32).to_radians());
    assert!(transformed.abs_diff_eq(expected_draw, 1.0e-5));

    let charged = first_person_apply_charged_crossbow_idle_transform(base, false);
    let expected_charged = base
        * Mat4::from_translation(Vec3::new(-0.641864, 0.0, 0.0))
        * Mat4::from_rotation_y(10.0_f32.to_radians());
    assert!(charged.abs_diff_eq(expected_charged, 1.0e-5));
}

#[test]
fn first_person_spear_use_transform_matches_vanilla_kinetic_use() {
    let kinetic_weapon = SpearKineticWeapon {
        delay_ticks: 15.0,
        dismount_duration_ticks: 100.0,
        knockback_duration_ticks: 200.0,
        damage_duration_ticks: 300.0,
        forward_movement: 0.38,
    };
    let transformed = first_person_apply_spear_use_transform(
        Mat4::IDENTITY,
        false,
        kinetic_weapon,
        VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
        20.0,
        0.5,
        2.5,
    );
    let remaining_ticks = VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS - 20.0;
    let time_held = VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
    let params = kinetic_weapon.use_params(time_held);
    let x_rotation_degrees = -65.0 * first_person_ease_in_out_back(params.raise_progress)
        - 35.0 * params.lower_progress
        + 100.0 * params.raise_back_progress
        - 0.5 * params.sway_scale_fast;
    let y_negative_axis_rotation_degrees = -90.0
        * first_person_progress(params.raise_progress, 0.5, 0.55)
        + 90.0 * params.sway_progress
        + 2.0 * params.sway_scale_slow;
    let expected = Mat4::from_translation(Vec3::new(0.56, -0.52, -0.72))
        * Mat4::from_translation(Vec3::new(
            params.raise_progress * 0.15
                + params.raise_progress_end * -0.05
                + params.sway_progress * -0.1
                + params.sway_scale_slow * 0.005,
            params.raise_progress * -0.075
                + params.raise_progress_middle * 0.075
                + params.sway_scale_fast * 0.01,
            params.raise_progress_start * 0.05
                + params.raise_progress_end * -0.05
                + params.sway_scale_slow * 0.005,
        ))
        * first_person_rotate_around(
            Vec3::new(0.0, 0.1, 0.0),
            Mat4::from_rotation_x(x_rotation_degrees.to_radians()),
        )
        * first_person_rotate_around(
            Vec3::new(0.15, 0.0, 0.0),
            Mat4::from_rotation_y((-y_negative_axis_rotation_degrees).to_radians()),
        )
        * Mat4::from_translation(Vec3::new(
            0.0,
            -first_person_spear_kinetic_hit_feedback_amount(2.5),
            0.0,
        ));
    assert!(transformed.abs_diff_eq(expected, 1.0e-5));
}

#[test]
fn held_generated_item_main_hand_select_uses_owner_main_arm_context() {
    // Vanilla `MainHand.get` returns `owner.getMainArm()`. Keep the attach
    // transform and physical hand fixed here so the mesh difference proves
    // the selected generated-item texture branch changed.
    let root = unique_item_model_temp_dir("held-main-hand-select");
    write_main_hand_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let bake = |owner_main_hand_left| {
        let mut block_meshes = Vec::new();
        let mut block_translucent_meshes = Vec::new();
        let mut block_glint_meshes = Vec::new();
        let mut block_glint_translucent_meshes = Vec::new();
        let mut flat_meshes = Vec::new();
        let mut flat_translucent_meshes = Vec::new();
        let mut flat_glint_meshes = Vec::new();
        let mut flat_glint_translucent_meshes = Vec::new();
        bake_item_stack_at_transform(
            &stack,
            Mat4::IDENTITY,
            BlockModelDisplayContext::ThirdPersonRightHand,
            false,
            owner_main_hand_left,
            None,
            false,
            0.0,
            None,
            None,
            None,
            None,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            &item_runtime,
            &terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        assert!(block_meshes.is_empty());
        assert!(block_translucent_meshes.is_empty());
        assert!(block_glint_meshes.is_empty());
        assert!(block_glint_translucent_meshes.is_empty());
        assert!(flat_translucent_meshes.is_empty());
        assert!(flat_glint_meshes.is_empty());
        assert!(flat_glint_translucent_meshes.is_empty());
        assert_eq!(flat_meshes.len(), 1);
        flat_meshes.remove(0)
    };

    let fallback = bake(None);
    let right = bake(Some(false));
    let left = bake(Some(true));

    assert_ne!(fallback, right);
    assert_ne!(fallback, left);
    assert_ne!(right, left);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_generated_item_context_entity_type_select_uses_owner_kind() {
    // Vanilla `ContextEntityType.get` returns the living owner entity type
    // key. Keep the transform fixed so mesh changes prove the generated
    // item texture branch changed, not the hand pose.
    let root = unique_item_model_temp_dir("held-context-entity-type-select");
    write_context_entity_type_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let bake = |context_entity_type| {
        let mut block_meshes = Vec::new();
        let mut block_translucent_meshes = Vec::new();
        let mut block_glint_meshes = Vec::new();
        let mut block_glint_translucent_meshes = Vec::new();
        let mut flat_meshes = Vec::new();
        let mut flat_translucent_meshes = Vec::new();
        let mut flat_glint_meshes = Vec::new();
        let mut flat_glint_translucent_meshes = Vec::new();
        bake_item_stack_at_transform(
            &stack,
            Mat4::IDENTITY,
            BlockModelDisplayContext::ThirdPersonRightHand,
            false,
            Some(false),
            context_entity_type,
            false,
            0.0,
            None,
            None,
            None,
            None,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            &item_runtime,
            &terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        assert!(block_meshes.is_empty());
        assert!(block_translucent_meshes.is_empty());
        assert!(block_glint_meshes.is_empty());
        assert!(block_glint_translucent_meshes.is_empty());
        assert!(flat_translucent_meshes.is_empty());
        assert!(flat_glint_meshes.is_empty());
        assert!(flat_glint_translucent_meshes.is_empty());
        assert_eq!(flat_meshes.len(), 1);
        flat_meshes.remove(0)
    };

    let player_kind = EntityModelKind::Humanoid {
        family: HumanoidModelFamily::Player,
        baby: false,
    };
    assert_eq!(
        entity_model_context_entity_type(player_kind),
        Some("minecraft:player")
    );
    assert_eq!(
        entity_model_context_entity_type(EntityModelKind::Witch),
        Some("minecraft:witch")
    );

    let fallback = bake(None);
    let player = bake(entity_model_context_entity_type(player_kind));
    let witch = bake(entity_model_context_entity_type(EntityModelKind::Witch));

    assert_ne!(fallback, player);
    assert_ne!(fallback, witch);
    assert_ne!(player, witch);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_generated_item_context_dimension_select_uses_world_level() {
    // Vanilla `ContextDimension.get` returns `level.dimension()` when the
    // item resolver is called with a `ClientLevel`. The held-item path uses
    // the real world level, so the same stack can select different
    // generated textures in different dimensions.
    let root = unique_item_model_temp_dir("held-context-dimension-select");
    write_context_dimension_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    const ENTITY_ID: i32 = 610;
    const PLAYER_ENTITY_TYPE_ID: i32 = 155;
    let bake = |dimension: Option<&str>| {
        let mut world = dimension.map_or_else(WorldStore::new, world_with_level_dimension);
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes[0].clone()
    };

    let fallback = bake(None);
    let overworld = bake(Some("minecraft:overworld"));
    let nether = bake(Some("minecraft:the_nether"));

    assert_ne!(fallback, overworld);
    assert_ne!(fallback, nether);
    assert_ne!(overworld, nether);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_generated_item_trim_material_select_uses_world_registry() {
    // Vanilla `TrimMaterialProperty.get` reads the stack's TRIM component
    // and unwraps the synced trim-material registry key. Held generated
    // items therefore need the same world registry context as GUI/dropped
    // item consumers.
    let root = unique_item_model_temp_dir("held-trim-material-select");
    write_trim_material_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    const ENTITY_ID: i32 = 611;
    const PLAYER_ENTITY_TYPE_ID: i32 = 155;
    let bake = |with_registry: bool, material_id: Option<i32>| {
        let mut world = WorldStore::new();
        if with_registry {
            record_trim_material_registry(&mut world);
        }
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: ENTITY_ID,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(0),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        armor_trim_material_id: material_id,
                        ..DataComponentPatchSummary::default()
                    },
                },
            }],
        }));
        let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes[0].clone()
    };

    let fallback = bake(false, Some(0));
    let quartz = bake(true, Some(0));
    let diamond = bake(true, Some(2));

    assert_ne!(fallback, quartz);
    assert_ne!(fallback, diamond);
    assert_ne!(quartz, diamond);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_held_generated_item_using_item_condition_matches_used_hand() {
    // Vanilla `IsUsingItem.get` is true only when the submitted stack is the
    // owner's active `getUseItem()` stack, not merely because the owner is
    // using some item in the other hand.
    let root = unique_item_model_temp_dir("held-using-item-condition");
    write_using_item_condition_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 607;
    const PLAYER_ENTITY_TYPE_ID: i32 = 155;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let terrain_textures = TerrainTextureState::default();
    let base = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);

    let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let main_using_models = held_item_models(
        &[base.with_is_using_item(true).with_use_item_off_hand(false)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let off_using_models = held_item_models(
        &[base.with_is_using_item(true).with_use_item_off_hand(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert_eq!(idle_models.flat_meshes.len(), 1);
    assert_eq!(main_using_models.flat_meshes.len(), 1);
    assert_eq!(off_using_models.flat_meshes.len(), 1);
    assert_ne!(idle_models.flat_meshes[0], main_using_models.flat_meshes[0]);
    assert_eq!(idle_models.flat_meshes[0], off_using_models.flat_meshes[0]);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_held_generated_item_use_duration_reads_owner_use_ticks() {
    // Vanilla `ItemModelResolver.updateForLiving` passes the owning living
    // entity into the item model; `UseDuration.get` then reads the active
    // `getUseItem()` stack's elapsed use ticks.
    let root = unique_item_model_temp_dir("held-use-duration-range-dispatch");
    write_bow_use_duration_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 608;
    const PLAYER_ENTITY_TYPE_ID: i32 = 155;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let terrain_textures = TerrainTextureState::default();
    let base = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);

    let bake = |instance: EntityModelInstance| {
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes[0].clone()
    };

    let idle = bake(base);
    let using_start = bake(
        base.with_is_using_item(true)
            .with_use_item_off_hand(false)
            .with_crossbow_charge_ticks(0.0),
    );
    let using_pulling = bake(
        base.with_is_using_item(true)
            .with_use_item_off_hand(false)
            .with_crossbow_charge_ticks(13.5),
    );
    let offhand_using = bake(
        base.with_is_using_item(true)
            .with_use_item_off_hand(true)
            .with_crossbow_charge_ticks(13.5),
    );

    assert_ne!(idle, using_start);
    assert_ne!(using_start, using_pulling);
    assert_eq!(idle, offhand_using);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_held_generated_crossbow_pull_applies_quick_charge_registry() {
    // Vanilla `CrossbowPull.get` divides elapsed use ticks by
    // `CrossbowItem.getChargeDuration`, which applies Quick Charge through
    // the synced enchantment registry before choosing the item-model
    // texture.
    let root = unique_item_model_temp_dir("held-crossbow-quick-charge-range-dispatch");
    write_crossbow_pull_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    const ENTITY_ID: i32 = 609;
    const PLAYER_ENTITY_TYPE_ID: i32 = 155;
    let quick_charge_crossbow = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 1,
                level: 2,
            }],
            ..DataComponentPatchSummary::default()
        },
    };
    let mut default_world = WorldStore::new();
    default_world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
    assert!(default_world.apply_set_equipment(SetEquipment {
        entity_id: ENTITY_ID,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: quick_charge_crossbow,
        }],
    }));
    let mut quick_charge_world = default_world.clone();
    record_enchantment_registry(&mut quick_charge_world);
    let terrain_textures = TerrainTextureState::default();
    let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false)
        .with_is_using_item(true)
        .with_use_item_off_hand(false)
        .with_crossbow_charge_ticks(10.0);

    let default_models = held_item_models(
        &[instance],
        &default_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let quick_charge_models = held_item_models(
        &[instance],
        &quick_charge_world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert_eq!(default_models.flat_meshes.len(), 1);
    assert_eq!(quick_charge_models.flat_meshes.len(), 1);
    assert!(!default_models.flat_meshes[0].is_empty());
    assert!(!quick_charge_models.flat_meshes[0].is_empty());
    assert_ne!(
        default_models.flat_meshes[0],
        quick_charge_models.flat_meshes[0]
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn illusioner_item_in_hand_layer_uses_vanilla_visibility_and_clones() {
    // Vanilla `IllusionerRenderer` installs a conditional `ItemInHandLayer`: idle illusioners do not
    // submit held items, while casting/aggressive illusioners submit inside the invisible clone loop.
    let root = unique_item_model_temp_dir("illusioner-held-item-clones");
    write_flat_item_runtime_fixture(&root, &["bow"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 604;
    const ILLUSIONER_ENTITY_TYPE_ID: i32 = 68;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, ILLUSIONER_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));

    let offsets = [
        [1.25, 0.0, -0.75],
        [-1.0, 0.5, 0.5],
        [0.5, 0.0, 1.0],
        [-1.5, 0.0, -1.25],
    ];
    let base = EntityModelInstance::illager(
        ENTITY_ID,
        [4.0, 64.0, -2.0],
        30.0,
        IllagerModelFamily::Illusioner,
    )
    .with_age_in_ticks(8.0)
    .with_illusioner_clone_offsets(offsets);
    let terrain_textures = TerrainTextureState::default();

    let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let aggressive_models = held_item_models(
        &[base.with_is_aggressive(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let invisible_aggressive_models = held_item_models(
        &[base.with_is_aggressive(true).with_invisible(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let invisible_casting_models = held_item_models(
        &[base.with_illager_spellcasting(true).with_invisible(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert!(idle_models.block_meshes.is_empty());
    assert!(idle_models.flat_meshes.is_empty());
    assert_eq!(aggressive_models.flat_meshes.len(), 1);
    assert_eq!(invisible_aggressive_models.flat_meshes.len(), 4);
    assert_eq!(invisible_casting_models.flat_meshes.len(), 4);
    for models in [
        &aggressive_models,
        &invisible_aggressive_models,
        &invisible_casting_models,
    ] {
        assert!(models.block_meshes.is_empty());
        assert!(models.block_translucent_meshes.is_empty());
        assert!(models.flat_translucent_meshes.is_empty());
        assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn illusioner_custom_head_item_branch_submits_per_invisible_clone() {
    // The generic item branch of vanilla `CustomHeadLayer` is inside `LivingEntityRenderer.submit`,
    // so `IllusionerRenderer`'s invisible clone loop runs it once for each clone root.
    let root = unique_item_model_temp_dir("illusioner-custom-head-item-clones");
    write_flat_item_runtime_fixture(&root, &["head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 605;
    const ILLUSIONER_ENTITY_TYPE_ID: i32 = 68;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, ILLUSIONER_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 0)));

    let offsets = [
        [1.25, 0.0, -0.75],
        [-1.0, 0.5, 0.5],
        [0.5, 0.0, 1.0],
        [-1.5, 0.0, -1.25],
    ];
    let base = EntityModelInstance::illager(
        ENTITY_ID,
        [4.0, 64.0, -2.0],
        30.0,
        IllagerModelFamily::Illusioner,
    )
    .with_age_in_ticks(8.0)
    .with_illusioner_clone_offsets(offsets);
    let terrain_textures = TerrainTextureState::default();

    let visible_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let invisible_models = held_item_models(
        &[base.with_invisible(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert!(visible_models.block_meshes.is_empty());
    assert!(invisible_models.block_meshes.is_empty());
    assert!(visible_models.flat_translucent_meshes.is_empty());
    assert!(invisible_models.flat_translucent_meshes.is_empty());
    assert_eq!(visible_models.flat_meshes.len(), 1);
    assert_eq!(invisible_models.flat_meshes.len(), 4);
    assert!(visible_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));
    assert!(invisible_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn allay_item_in_hand_layer_bakes_main_and_offhand_items() {
    // Vanilla `AllayRenderer` adds the standard `ItemInHandLayer`. Its `AllayModel.translateToHand`
    // supplies a special allay hand transform, but the item states still use the ordinary third-person
    // right/left hand display contexts from `ArmedEntityRenderState`.
    let root = unique_item_model_temp_dir("allay-held-items");
    write_flat_item_runtime_fixture(&root, &["main_item", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 603;
    const ALLAY_ENTITY_TYPE_ID: i32 = 2;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, ALLAY_ENTITY_TYPE_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::OffHand, 1)));

    let base = EntityModelInstance::allay(ENTITY_ID, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(7.0);
    let holding = base.with_allay_holding_item_progress(1.0);
    let terrain_textures = TerrainTextureState::default();

    let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let holding_models =
        held_item_models(&[holding], &world, Some(&item_runtime), &terrain_textures);

    for models in [&idle_models, &holding_models] {
        assert!(models.block_meshes.is_empty());
        assert!(models.block_translucent_meshes.is_empty());
        assert_eq!(models.flat_meshes.len(), 2);
        assert!(models.flat_translucent_meshes.is_empty());
        assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
    }
    assert_ne!(
        idle_models.flat_meshes[0], holding_models.flat_meshes[0],
        "holdingAnimationProgress feeds AllayModel.translateToHand through right_arm.xRot"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cluster_count_drives_geometry_and_is_non_empty() {
    let quads = unit_block_quads();
    // One copy and four copies both produce geometry; the four-copy mesh holds four times as much.
    let single = stacked_item_mesh(
        &quads,
        [0.0, 0.0, 0.0],
        0.0,
        0,
        &BLOCK_GROUND_FALLBACK,
        [4.0 / 15.0, 11.0 / 15.0],
        1,
        7,
        ItemModelFoil::None,
    );
    let cluster = stacked_item_mesh(
        &quads,
        [0.0, 0.0, 0.0],
        0.0,
        0,
        &BLOCK_GROUND_FALLBACK,
        [4.0 / 15.0, 11.0 / 15.0],
        4,
        7,
        ItemModelFoil::None,
    );
    assert!(!single.is_empty());
    assert!(!cluster.is_empty());
}

#[test]
fn foiled_cluster_mirrors_geometry_to_item_glint_buckets() {
    let mut quads = unit_block_quads();
    let mut translucent = quads[0].clone();
    translucent.translucent = true;
    translucent.tint = [0.7, 0.8, 1.0, 0.5];
    quads.push(translucent);
    let plain = stacked_item_mesh(
        &quads,
        [0.0, 0.0, 0.0],
        0.0,
        0,
        &BLOCK_GROUND_FALLBACK,
        [1.0, 1.0],
        1,
        7,
        ItemModelFoil::None,
    );
    let foiled = stacked_item_mesh(
        &quads,
        [0.0, 0.0, 0.0],
        0.0,
        0,
        &BLOCK_GROUND_FALLBACK,
        [1.0, 1.0],
        1,
        7,
        ItemModelFoil::Standard,
    );

    assert!(plain.glint.is_empty());
    assert!(plain.glint_translucent.is_empty());
    assert_eq!(foiled.glint, foiled.solid);
    assert_eq!(foiled.glint_translucent, foiled.translucent);
    assert!(!foiled.translucent.is_empty());
}

#[test]
fn ominous_item_spawner_base_transform_scales_in_and_spins() {
    let early = ominous_item_spawner_base_transform([1.0, 2.0, 3.0], 9.0);
    assert!((early.transform_point3(Vec3::ZERO) - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
    assert!((early.transform_vector3(Vec3::X) - Vec3::new(0.18, 0.0, 0.0)).length() < 1e-6);

    let full_size = ominous_item_spawner_base_transform([0.0, 0.0, 0.0], 60.0);
    assert!((full_size.transform_vector3(Vec3::X).length() - 1.0).abs() < 1e-6);
    assert!(
        (full_size.transform_vector3(Vec3::X) - Vec3::X).length() > 1.0,
        "60 ticks spins by 240 degrees, not identity"
    );
}

#[test]
fn ominous_item_spawner_models_emit_item_cluster_meshes() {
    const OMINOUS_ITEM_SPAWNER_DATA_ITEM_ID: u8 = 8;

    let root = unique_item_model_temp_dir("ominous-item-spawner");
    write_flat_item_runtime_fixture(&root, &["omen_drop"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:omen_drop")
        .expect("fixture item registered");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        801,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 801,
        values: vec![EntityDataValue {
            data_id: OMINOUS_ITEM_SPAWNER_DATA_ITEM_ID,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(item_id),
                count: 3,
                component_patch: DataComponentPatchSummary::default(),
            }),
        }],
    }));
    world.advance_entity_client_animations(25);

    let models = ominous_item_spawner_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        0.5,
        None,
        None,
        None,
    );

    assert!(models.block_meshes.is_empty());
    assert_eq!(models.flat_meshes.len(), 1);
    assert!(!models.flat_meshes[0].is_empty());
    assert!(models.handled_entity_ids.contains(&801));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_pickup_particle_item_models_emit_ordinary_stack_meshes() {
    let root = unique_item_model_temp_dir("item-pickup-particle");
    write_flat_item_runtime_fixture(&root, &["picked_up"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:picked_up")
        .expect("fixture item registered");
    let state = ItemPickupParticleRenderState {
        source_entity_id: 77,
        item: ParticleItemOptionState {
            item_id,
            count: 18,
            component_patch_len: 0,
        },
        component_patch: None,
        position: [1.25, 65.5, -3.0],
        age_ticks: 9.5,
        light: [6.0 / 15.0, 12.0 / 15.0],
    };

    let models = item_pickup_particle_item_models(
        &[state],
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        None,
    );

    assert!(models.block_meshes.is_empty());
    assert_eq!(models.flat_meshes.len(), 1);
    assert!(!models.flat_meshes[0].is_empty());
    assert!(models.handled_entity_ids.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_pickup_particle_component_rich_bake_matches_dropped_item_bake() {
    // A component-rich pickup stack must bake through the SAME item-model
    // projection as a dropped item entity. Here the ITEM_MODEL component
    // overrides the root model (vanilla `ItemModelResolver.appendItemLayers`),
    // so the resolved item texture differs from the plain item. With identical
    // geometry inputs (position / age / entity id / light / count / seed) the
    // pickup carried bake must be byte-identical to the dropped-item bake.
    const ITEM_ENTITY_DATA_ITEM_STACK_ID: u8 = 8;
    const ENTITY_ID: i32 = 555;
    const POSITION: [f32; 3] = [0.0, 64.0, 0.0];
    const AGE_TICKS: f32 = 9.5;
    const COUNT: i32 = 1;

    let root = unique_item_model_temp_dir("item-pickup-component-rich");
    write_flat_item_runtime_fixture(&root, &["pickup_tool", "override_skin"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:pickup_tool")
        .expect("fixture item registered");

    // ITEM_MODEL override -> the resolved icon uses `override_skin`'s atlas rect
    // instead of `pickup_tool`'s, changing the baked mesh.
    let component_patch = DataComponentPatchSummary {
        item_model: Some("minecraft:override_skin".to_string()),
        ..DataComponentPatchSummary::default()
    };
    let stack = ItemStackSummary {
        item_id: Some(item_id),
        count: COUNT,
        component_patch: component_patch.clone(),
    };

    // Dropped-item bake: a real item entity carrying the component-rich stack.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, VANILLA_ENTITY_TYPE_ITEM_ID));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: ENTITY_ID,
        values: vec![EntityDataValue {
            data_id: ITEM_ENTITY_DATA_ITEM_STACK_ID,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(stack.clone()),
        }],
    }));
    let dropped = dropped_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        AGE_TICKS,
        None,
        None,
        None,
    );

    // Pickup carried bake: the same stack rebuilt from the opaque patch bytes
    // the pickup channel round-tripped through the renderer, with geometry
    // inputs aligned to the dropped item entity.
    let patch_bytes = serde_json::to_vec(&component_patch).expect("serialize component patch");
    let component_rich_state = ItemPickupParticleRenderState {
        source_entity_id: ENTITY_ID,
        item: ParticleItemOptionState {
            item_id,
            count: COUNT,
            component_patch_len: 1,
        },
        component_patch: Some(patch_bytes),
        position: POSITION,
        age_ticks: AGE_TICKS,
        light: [1.0, 1.0],
    };
    let pickup = item_pickup_particle_item_models(
        &[component_rich_state.clone()],
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        None,
    );

    assert!(dropped.block_meshes.is_empty());
    assert_eq!(dropped.flat_meshes.len(), 1);
    assert!(!dropped.flat_meshes[0].is_empty());
    // Equality: the component-rich pickup bake matches the dropped-item bake.
    assert_eq!(pickup.block_meshes, dropped.block_meshes);
    assert_eq!(pickup.flat_meshes, dropped.flat_meshes);
    assert_eq!(
        pickup.flat_translucent_meshes,
        dropped.flat_translucent_meshes
    );

    // Consumption proof: dropping the patch selects `pickup_tool`'s own model,
    // producing a different mesh -- so the patch demonstrably drives the bake.
    let plain_state = ItemPickupParticleRenderState {
        component_patch: None,
        item: ParticleItemOptionState {
            component_patch_len: 0,
            ..component_rich_state.item
        },
        ..component_rich_state
    };
    let plain = item_pickup_particle_item_models(
        &[plain_state],
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        None,
    );
    assert_eq!(plain.flat_meshes.len(), 1);
    assert_ne!(pickup.flat_meshes, plain.flat_meshes);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_random_matches_java_sequence() {
    // Java `new Random(0).nextFloat()` is 0.7309678 — the LCG reproduces it bit-for-bit.
    let mut random = LegacyRandom::new(0);
    assert!((random.next_float() - 0.730_967_8).abs() < 1e-6);
}

#[test]
fn display_matrix_centers_the_model_at_the_translation() {
    // The display transform is about the model center (`T(-0.5)`), so the unit-cube center
    // (0.5,0.5,0.5) lands exactly at the (world-unit) translation regardless of rotation/scale.
    let generated =
        display_matrix(&GENERATED_THIRD_PERSON_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((generated - Vec3::new(0.0, 3.0 / 16.0, 1.0 / 16.0)).length() < 1e-6);
    let block =
        display_matrix(&BLOCK_THIRD_PERSON_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((block - Vec3::new(0.0, 2.5 / 16.0, 0.0)).length() < 1e-6);

    let generated_head =
        display_matrix(&GENERATED_HEAD_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((generated_head - Vec3::new(0.0, 13.0 / 16.0, 7.0 / 16.0)).length() < 1e-6);
    let block_head = display_matrix(&BLOCK_HEAD_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((block_head - Vec3::ZERO).length() < 1e-6);
}

#[test]
fn item_stack_foil_mode_projects_vanilla_special_context_scale() {
    let root = unique_item_model_temp_dir("special-foil-mode");
    write_flat_item_runtime_fixture(&root, &["clock", "spyglass"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let foiled_stack = |resource_id: &str| ItemStackSummary {
        item_id: item_runtime.item_protocol_id(resource_id),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantment_glint_override: Some(true),
            ..DataComponentPatchSummary::default()
        },
    };
    let plain_clock = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:clock"),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };

    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:clock"),
            &item_runtime,
            BlockModelDisplayContext::Ground
        ),
        ItemModelFoil::Special {
            decal_pose_scale: 1.0
        }
    );
    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:clock"),
            &item_runtime,
            BlockModelDisplayContext::Gui
        ),
        ItemModelFoil::Special {
            decal_pose_scale: 0.5
        }
    );
    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:clock"),
            &item_runtime,
            BlockModelDisplayContext::FirstPersonRightHand
        ),
        ItemModelFoil::Special {
            decal_pose_scale: 0.75
        }
    );
    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:spyglass"),
            &item_runtime,
            BlockModelDisplayContext::Ground
        ),
        ItemModelFoil::Standard
    );
    assert_eq!(
        item_stack_foil_mode(
            &plain_clock,
            &item_runtime,
            BlockModelDisplayContext::Ground
        ),
        ItemModelFoil::None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn left_hand_display_mirror_negates_x_translation_and_yz_rotation() {
    // Vanilla `ItemTransform.apply(applyLeftHandFix=true)` negates translation.x, rotation.y, and
    // rotation.z. A handheld-style transform with a Y rotation mirrors the model across the X plane.
    let handheld = BlockModelDisplayTransform {
        rotation: [0.0, -90.0, 55.0],
        translation: [1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0],
        scale: [0.85, 0.85, 0.85],
    };
    let right = display_matrix(&handheld, false);
    let left = display_matrix(&handheld, true);
    // The model center sits at the mirrored translation (x negated, y/z unchanged).
    let right_center = right.transform_point3(Vec3::splat(0.5));
    let left_center = left.transform_point3(Vec3::splat(0.5));
    assert!((right_center - Vec3::new(1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0)).length() < 1e-6);
    assert!((left_center - Vec3::new(-1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0)).length() < 1e-6);
    // The left fix mirrors the rotation across the X plane: `rotationXYZ(rx,-ry,-rz)` =
    // `M·rotationXYZ(rx,ry,rz)·M` (M = diag(-1,1,1)), so the rotated +X basis direction reflects to
    // `(x, -y, -z)` of the right-hand one.
    let right_dir = right.transform_vector3(Vec3::X);
    let left_dir = left.transform_vector3(Vec3::X);
    assert!((left_dir - Vec3::new(right_dir.x, -right_dir.y, -right_dir.z)).length() < 1e-6);
}

fn equipment(entity_id: i32, slot: EquipmentSlot, item_id: i32) -> SetEquipment {
    SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
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

fn write_default_consumable_item_runtime_fixture(root: &Path, item_id: &str) {
    write_flat_item_runtime_fixture(root, &[item_id]);
    let constant = item_id.to_ascii_uppercase();
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        &format!(
            r#"public class Items {{
            public static final Item {constant} = registerItem("{item_id}", new Item.Properties().food(Foods.{constant}));
        }}"#,
        ),
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
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/entity_selector" }
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "entity_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "entity_selector_player", &[120, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "entity_selector_witch", &[80, 120, 40, 255]);
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
        .join(bbb_pack::MC_VERSION)
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
    let declarations = item_ids
        .iter()
        .map(|item_id| {
            let constant = item_id.to_ascii_uppercase();
            format!("public static final Item {constant} = registerItem(\"{item_id}\");")
        })
        .collect::<Vec<_>>()
        .join("\n");
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        &format!(
            r#"public class Items {{
            {declarations}
        }}"#,
        ),
    );
}

fn write_flat_item_model_and_texture(assets: &Path, model_id: &str, rgba: &[u8]) {
    write_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
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
    std::env::temp_dir().join(format!("bbb-native-item-models-{label}-{nanos}"))
}
