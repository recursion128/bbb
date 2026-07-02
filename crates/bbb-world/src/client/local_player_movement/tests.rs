use super::*;
use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind as ProtocolEntityDataValueKind, GameEvent as ProtocolGameEvent,
    ItemStackSummary as ProtocolItemStackSummary, MobEffectFlags as ProtocolMobEffectFlags,
    PlayerHealth as ProtocolPlayerHealth, SetEntityData as ProtocolSetEntityData,
    SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
    UpdateAttributes as ProtocolUpdateAttributes, UpdateMobEffect as ProtocolUpdateMobEffect,
    UseEffectsSummary as ProtocolUseEffectsSummary,
};
use uuid::Uuid;

use crate::{
    entities::{
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    },
    BlockPos, ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldCardinalLighting, WorldDimension, WorldLevelInfo,
};

const AIR_BLOCK_STATE_ID: i32 = 0;
const GRASS_BLOCK_STATE_ID: i32 = 9;
const OAK_TOP_SLAB_BLOCK_STATE_ID: i32 = 13331;
const OAK_BOTTOM_SLAB_BLOCK_STATE_ID: i32 = 13333;
const OAK_TOP_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID: i32 = 3928;
const OAK_BOTTOM_STRAIGHT_NORTH_STAIR_BLOCK_STATE_ID: i32 = 3918;
const OAK_BOTTOM_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID: i32 = 3938;
const OAK_LEAVES_BLOCK_STATE_ID: i32 = 255;
const MANGROVE_PROPAGULE_AGE_4_DRY_BLOCK_STATE_ID: i32 = 82;
const COBWEB_BLOCK_STATE_ID: i32 = 2247;
const DEAD_BUSH_BLOCK_STATE_ID: i32 = 2250;
const BUSH_BLOCK_STATE_ID: i32 = 2251;
const DANDELION_BLOCK_STATE_ID: i32 = 2321;
const FIRE_UP_AGE_0_BLOCK_STATE_ID: i32 = 3404;
const REDSTONE_WIRE_POWER_0_DOT_BLOCK_STATE_ID: i32 = 5171;
const WHEAT_AGE_7_BLOCK_STATE_ID: i32 = 5318;
const MANGROVE_ROOTS_WATERLOGGED_BLOCK_STATE_ID: i32 = 163;
const MANGROVE_ROOTS_DRY_BLOCK_STATE_ID: i32 = 164;
const MUDDY_MANGROVE_ROOTS_X_BLOCK_STATE_ID: i32 = 165;
const PISTON_EXTENDED_NORTH_BLOCK_STATE_ID: i32 = 2257;
const PISTON_EXTENDED_UP_BLOCK_STATE_ID: i32 = 2261;
const PISTON_HEAD_SOUTH_LONG_BLOCK_STATE_ID: i32 = 2279;
const MOVING_PISTON_NORTH_BLOCK_STATE_ID: i32 = 2309;
const POWERED_RAIL_NORTH_SOUTH_BLOCK_STATE_ID: i32 = 2200;
const DETECTOR_RAIL_NORTH_SOUTH_BLOCK_STATE_ID: i32 = 2224;
const TORCH_BLOCK_STATE_ID: i32 = 3370;
const WALL_TORCH_NORTH_BLOCK_STATE_ID: i32 = 3371;
const RAIL_NORTH_SOUTH_BLOCK_STATE_ID: i32 = 5728;
const LEVER_FLOOR_NORTH_BLOCK_STATE_ID: i32 = 6772;
const FARMLAND_MOISTURE_0_BLOCK_STATE_ID: i32 = 5319;
const SNOW_5_LAYERS_BLOCK_STATE_ID: i32 = 6923;
const SNOW_6_LAYERS_BLOCK_STATE_ID: i32 = 6924;
const STONE_BUTTON_FLOOR_NORTH_BLOCK_STATE_ID: i32 = 6896;
const CACTUS_AGE_0_BLOCK_STATE_ID: i32 = 6929;
const SOUL_SAND_BLOCK_STATE_ID: i32 = 6998;
const NETHER_PORTAL_X_BLOCK_STATE_ID: i32 = 7017;
const NETHER_PORTAL_Z_BLOCK_STATE_ID: i32 = 7018;
const CAKE_BITES_0_BLOCK_STATE_ID: i32 = 7027;
const CAKE_BITES_3_BLOCK_STATE_ID: i32 = 7030;
const END_PORTAL_FRAME_EYE_NORTH_BLOCK_STATE_ID: i32 = 9469;
const END_PORTAL_FRAME_EMPTY_NORTH_BLOCK_STATE_ID: i32 = 9473;
const END_PORTAL_BLOCK_STATE_ID: i32 = 9468;
const DRAGON_EGG_BLOCK_STATE_ID: i32 = 9478;
const COCOA_AGE_2_NORTH_BLOCK_STATE_ID: i32 = 9489;
const COCOA_AGE_2_SOUTH_BLOCK_STATE_ID: i32 = 9490;
const OAK_SHELF_NORTH_BLOCK_STATE_ID: i32 = 3121;
const OAK_SHELF_SOUTH_BLOCK_STATE_ID: i32 = 3137;
const COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID: i32 = 27288;
const WAXED_OXIDIZED_COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID: i32 = 27512;
const AMETHYST_CLUSTER_NORTH_BLOCK_STATE_ID: i32 = 23405;
const AMETHYST_CLUSTER_SOUTH_BLOCK_STATE_ID: i32 = 23409;
const AMETHYST_CLUSTER_UP_BLOCK_STATE_ID: i32 = 23413;
const SMALL_AMETHYST_BUD_UP_BLOCK_STATE_ID: i32 = 23449;
const SLIME_BLOCK_STATE_ID: i32 = 12532;
const FLOWER_POT_BLOCK_STATE_ID: i32 = 10629;
const POTTED_DANDELION_BLOCK_STATE_ID: i32 = 10641;
const SKELETON_SKULL_BLOCK_STATE_ID: i32 = 10931;
const SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID: i32 = 10948;
const PIGLIN_HEAD_BLOCK_STATE_ID: i32 = 11171;
const PIGLIN_WALL_HEAD_NORTH_BLOCK_STATE_ID: i32 = 11188;
const ACTIVATOR_RAIL_NORTH_SOUTH_BLOCK_STATE_ID: i32 = 11421;
const LIGHT_LEVEL_15_DRY_BLOCK_STATE_ID: i32 = 12566;
const OAK_CLOSED_NORTH_DOOR_BLOCK_STATE_ID: i32 = 5666;
const OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7121;
const OAK_TOP_OPEN_NORTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7117;
const OAK_TOP_OPEN_SOUTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7133;
const STONE_PRESSURE_PLATE_BLOCK_STATE_ID: i32 = 6796;
const OAK_SIGN_ROTATION_0_BLOCK_STATE_ID: i32 = 5336;
const OAK_WALL_SIGN_NORTH_BLOCK_STATE_ID: i32 = 5828;
const OAK_HANGING_SIGN_ATTACHED_ROTATION_0_BLOCK_STATE_ID: i32 = 5908;
const OAK_WALL_HANGING_SIGN_NORTH_BLOCK_STATE_ID: i32 = 6676;
const OAK_NORTH_FENCE_BLOCK_STATE_ID: i32 = 6988;
const OAK_CLOSED_NORTH_FENCE_GATE_BLOCK_STATE_ID: i32 = 8653;
const OAK_OPEN_NORTH_FENCE_GATE_BLOCK_STATE_ID: i32 = 8651;
const LILY_PAD_BLOCK_STATE_ID: i32 = 8920;
const SUGAR_CANE_AGE_0_BLOCK_STATE_ID: i32 = 6947;
const NETHER_WART_AGE_3_BLOCK_STATE_ID: i32 = 9450;
const TRIPWIRE_EMPTY_BLOCK_STATE_ID: i32 = 9726;
const CARROTS_AGE_7_BLOCK_STATE_ID: i32 = 10666;
const POTATOES_AGE_7_BLOCK_STATE_ID: i32 = 10674;
const GLASS_NORTH_PANE_BLOCK_STATE_ID: i32 = 8323;
const WHITE_CARPET_BLOCK_STATE_ID: i32 = 12896;
const COBBLESTONE_NORTH_EAST_WALL_BLOCK_STATE_ID: i32 = 10236;
const IRON_CHAIN_Y_AXIS_BLOCK_STATE_ID: i32 = 8249;
const LADDER_NORTH_BLOCK_STATE_ID: i32 = 5720;
const LADDER_SOUTH_BLOCK_STATE_ID: i32 = 5722;
const END_ROD_NORTH_BLOCK_STATE_ID: i32 = 14636;
const DIRT_PATH_BLOCK_STATE_ID: i32 = 14815;
const SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID: i32 = 20707;
const SCAFFOLDING_BOTTOM_DISTANCE_1_BLOCK_STATE_ID: i32 = 20709;
const SWEET_BERRY_BUSH_AGE_3_BLOCK_STATE_ID: i32 = 20944;
const LANTERN_STANDING_BLOCK_STATE_ID: i32 = 20840;
const CAMPFIRE_NORTH_LIT_BLOCK_STATE_ID: i32 = 20880;
const NETHER_SPROUTS_BLOCK_STATE_ID: i32 = 20961;
const CRIMSON_FUNGUS_BLOCK_STATE_ID: i32 = 20975;
const HONEY_BLOCK_STATE_ID: i32 = 21816;
const POWDER_SNOW_BLOCK_STATE_ID: i32 = 24689;
const FREEZE_IMMUNE_WEARABLE_ITEM_ID: i32 = 42;
const LEATHER_BOOTS_ITEM_ID: i32 = 43;
const PLAYER_FEET_EQUIPMENT_SLOT_ID: i32 = 36;
const CHEST_SINGLE_NORTH_BLOCK_STATE_ID: i32 = 3988;
const CHEST_LEFT_NORTH_BLOCK_STATE_ID: i32 = 3990;
const TRAPPED_CHEST_RIGHT_NORTH_BLOCK_STATE_ID: i32 = 11212;
const ENDER_CHEST_NORTH_BLOCK_STATE_ID: i32 = 9576;
const WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID: i32 = 1934;
const WATER_CAULDRON_LEVEL_3_BLOCK_STATE_ID: i32 = 9463;
const DAYLIGHT_DETECTOR_POWER_0_BLOCK_STATE_ID: i32 = 11295;
const HOPPER_NORTH_ENABLED_BLOCK_STATE_ID: i32 = 11314;
const ENCHANTING_TABLE_BLOCK_STATE_ID: i32 = 9451;
const GRINDSTONE_FLOOR_NORTH_BLOCK_STATE_ID: i32 = 20772;
const GRINDSTONE_WALL_NORTH_BLOCK_STATE_ID: i32 = 20776;
const GRINDSTONE_CEILING_NORTH_BLOCK_STATE_ID: i32 = 20780;
const LECTERN_NORTH_NO_BOOK_BLOCK_STATE_ID: i32 = 20787;
const BELL_FLOOR_NORTH_BLOCK_STATE_ID: i32 = 20806;
const BELL_CEILING_NORTH_BLOCK_STATE_ID: i32 = 20814;
const BELL_SINGLE_WALL_NORTH_BLOCK_STATE_ID: i32 = 20822;
const BELL_DOUBLE_WALL_NORTH_BLOCK_STATE_ID: i32 = 20830;
const BREWING_STAND_EMPTY_BLOCK_STATE_ID: i32 = 9459;
const STONECUTTER_NORTH_BLOCK_STATE_ID: i32 = 20801;
const ANVIL_NORTH_BLOCK_STATE_ID: i32 = 11195;
const COMPOSTER_LEVEL_7_BLOCK_STATE_ID: i32 = 21750;
const BEETROOTS_AGE_3_BLOCK_STATE_ID: i32 = 14814;
const COPPER_GRATE_BLOCK_STATE_ID: i32 = 27048;
const WAXED_COPPER_GRATE_BLOCK_STATE_ID: i32 = 27056;
const SCULK_SENSOR_INACTIVE_BLOCK_STATE_ID: i32 = 24691;
const CALIBRATED_SCULK_SENSOR_NORTH_INACTIVE_BLOCK_STATE_ID: i32 = 24787;
const SCULK_VEIN_UP_DRY_BLOCK_STATE_ID: i32 = 25294;
const SCULK_SHRIEKER_IDLE_BLOCK_STATE_ID: i32 = 25304;
const LIGHTNING_ROD_UP_UNPOWERED_BLOCK_STATE_ID: i32 = 27562;
const TURTLE_EGG_ONE_BLOCK_STATE_ID: i32 = 15090;
const TURTLE_EGG_TWO_BLOCK_STATE_ID: i32 = 15093;
const SNIFFER_EGG_BLOCK_STATE_ID: i32 = 15102;
const DRIED_GHAST_NORTH_DRY_BLOCK_STATE_ID: i32 = 15106;
const SEA_PICKLE_ONE_DRY_BLOCK_STATE_ID: i32 = 15268;
const SEA_PICKLE_FOUR_DRY_BLOCK_STATE_ID: i32 = 15274;
const CHORUS_PLANT_NORTH_BLOCK_STATE_ID: i32 = 14697;
const CHORUS_PLANT_NONE_BLOCK_STATE_ID: i32 = 14705;
const CHORUS_FLOWER_AGE_0_BLOCK_STATE_ID: i32 = 14706;
const CONDUIT_DRY_BLOCK_STATE_ID: i32 = 15277;
const BAMBOO_SAPLING_BLOCK_STATE_ID: i32 = 15278;
const BAMBOO_AGE_0_NO_LEAVES_STAGE_0_BLOCK_STATE_ID: i32 = 15279;
const CANDLE_ONE_DRY_UNLIT_BLOCK_STATE_ID: i32 = 23099;
const CANDLE_FOUR_DRY_UNLIT_BLOCK_STATE_ID: i32 = 23111;
const CANDLE_CAKE_UNLIT_BLOCK_STATE_ID: i32 = 23369;
const POINTED_DRIPSTONE_TIP_UP_BLOCK_STATE_ID: i32 = 27740;
const POINTED_DRIPSTONE_TIP_DOWN_BLOCK_STATE_ID: i32 = 27742;
const POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID: i32 = 27752;
const DECORATED_POT_NORTH_DRY_BLOCK_STATE_ID: i32 = 29602;
const PALE_MOSS_CARPET_BOTTOM_NO_SIDES_BLOCK_STATE_ID: i32 = 29704;
const PALE_MOSS_CARPET_TOPPER_WITH_SIDES_BLOCK_STATE_ID: i32 = 29830;
const BIG_DRIPLEAF_NORTH_NONE_DRY_BLOCK_STATE_ID: i32 = 27864;
const BIG_DRIPLEAF_NORTH_PARTIAL_DRY_BLOCK_STATE_ID: i32 = 27868;
const BIG_DRIPLEAF_NORTH_FULL_DRY_BLOCK_STATE_ID: i32 = 27870;
const BIG_DRIPLEAF_STEM_NORTH_DRY_BLOCK_STATE_ID: i32 = 27896;
const PINK_PETALS_ONE_NORTH_BLOCK_STATE_ID: i32 = 27814;
const LEAF_LITTER_ONE_NORTH_BLOCK_STATE_ID: i32 = 27846;
const MUD_BLOCK_STATE_ID: i32 = 27922;
const WHITE_BANNER_ROTATION_0_BLOCK_STATE_ID: i32 = 12927;
const WHITE_WALL_BANNER_NORTH_BLOCK_STATE_ID: i32 = 13183;
const AZALEA_BLOCK_STATE_ID: i32 = 27811;
const FLOWERING_AZALEA_BLOCK_STATE_ID: i32 = 27812;
const HEAVY_CORE_DRY_BLOCK_STATE_ID: i32 = 29702;
const FIREFLY_BUSH_BLOCK_STATE_ID: i32 = 29872;
const END_GATEWAY_BLOCK_STATE_ID: i32 = 14816;
const STRUCTURE_VOID_BLOCK_STATE_ID: i32 = 14851;
const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
const FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID: i32 = 89;
const SOURCE_LAVA_BLOCK_STATE_ID: i32 = 102;
const FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID: i32 = 105;
const VANILLA_MOB_EFFECT_BAD_OMEN_ID: i32 = 30;

#[test]
fn local_player_input_stops_at_full_block_wall_and_reports_collision() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, GRASS_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 2, GRASS_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.x, 0.5, 0.000001);
    assert!(
        pose.position.z <= 1.70001,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_gravity_lands_on_floor_and_reports_grounded() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        pose.position.y >= 0.9999,
        "position was {:?}",
        pose.position
    );
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_gravity_attribute_scales_airborne_fall_velocity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_GRAVITY_ID,
            base: 0.04,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.0392, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_no_gravity_metadata_suppresses_airborne_gravity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_no_gravity(&mut world, 123, true));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_slow_falling_clamps_downward_gravity() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SLOW_FALLING_ID, 0,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        delta_movement: vec3(0.0, -0.08, 0.0),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 2.92, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.0882, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_climbable_limits_downward_velocity_and_resets_fall_distance() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.05, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_sneak_suppresses_climbable_sliding() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.0784, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_sneak_does_not_suppress_scaffolding_climbable_sliding() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.05, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_scaffolding_top_supports_player_when_not_descending() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        delta_movement: vec3(0.0, -0.2, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 2.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_sneak_descends_through_scaffolding_top() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        delta_movement: vec3(0.0, -0.2, 0.0),
        fall_distance: 2.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.8, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2744, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_scaffolding_unstable_bottom_supports_inside_player() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        SCAFFOLDING_BOTTOM_DISTANCE_1_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.125, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_open_trapdoor_above_matching_ladder_counts_as_climbable() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    set_test_block(
        &mut world,
        0,
        1,
        0,
        OAK_TOP_OPEN_SOUTH_TRAPDOOR_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.05, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_open_trapdoor_requires_matching_ladder_facing_for_climbable() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    set_test_block(
        &mut world,
        0,
        1,
        0,
        OAK_TOP_OPEN_NORTH_TRAPDOOR_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.1764, 0.000001);
    assert_f64_near(pose.fall_distance, 2.1, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_closed_trapdoor_above_matching_ladder_is_not_climbable() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, LADDER_NORTH_BLOCK_STATE_ID);
    set_test_block(
        &mut world,
        0,
        1,
        0,
        OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.1764, 0.000001);
    assert_f64_near(pose.fall_distance, 2.1, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_jump_on_climbable_applies_vanilla_upward_velocity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_climbable_clamps_horizontal_velocity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.65, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.15, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_levitation_moves_vertical_velocity_toward_effect_target() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_LEVITATION_ID, 1,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0196, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_steps_onto_bottom_slab() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, OAK_BOTTOM_SLAB_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 0.35);

    assert_f64_near(pose.position.y, 1.5, 0.0005);
    assert!(pose.position.z > 1.7, "position was {:?}", pose.position);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_does_not_step_through_top_slab() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, OAK_TOP_SLAB_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_steps_up_bottom_stair_from_low_side() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        2,
        OAK_BOTTOM_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 0.45);

    assert_f64_near(pose.position.y, 2.0, 0.0005);
    assert!(pose.position.z > 2.2, "position was {:?}", pose.position);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_does_not_step_up_bottom_stair_from_high_side() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        2,
        OAK_BOTTOM_STRAIGHT_NORTH_STAIR_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_step_through_top_stair() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        2,
        OAK_TOP_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_leaves() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, OAK_LEAVES_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 1, OAK_LEAVES_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z <= 0.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_mangrove_roots() {
    let cases = [
        ("dry mangrove roots", MANGROVE_ROOTS_DRY_BLOCK_STATE_ID),
        (
            "waterlogged mangrove roots",
            MANGROVE_ROOTS_WATERLOGGED_BLOCK_STATE_ID,
        ),
        (
            "muddy mangrove roots",
            MUDDY_MANGROVE_ROOTS_X_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_bamboo_stalk_collision() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        1,
        BAMBOO_AGE_0_NO_LEAVES_STAGE_0_BLOCK_STATE_ID,
    );
    set_test_block(
        &mut world,
        0,
        2,
        1,
        BAMBOO_AGE_0_NO_LEAVES_STAGE_0_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(pose.position.z < 1.7, "position was {:?}", pose.position);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_chorus_collision() {
    let cases = [
        (
            "chorus plant center cube",
            CHORUS_PLANT_NONE_BLOCK_STATE_ID,
            0.888,
        ),
        (
            "chorus plant north arm",
            CHORUS_PLANT_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
        ("chorus flower", CHORUS_FLOWER_AGE_0_BLOCK_STATE_ID, 0.7005),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_conduit_and_azalea_collision_shapes() {
    let cases = [
        ("conduit", CONDUIT_DRY_BLOCK_STATE_ID, 0.9, 1.013),
        ("azalea", AZALEA_BLOCK_STATE_ID, 0.0, 0.7005),
        (
            "flowering azalea",
            FLOWERING_AZALEA_BLOCK_STATE_ID,
            0.0,
            0.7005,
        ),
    ];

    for (name, block_state_id, min_z, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > min_z && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_steps_over_thin_ground_shapes() {
    let cases = [
        ("white carpet", WHITE_CARPET_BLOCK_STATE_ID, 1.0625),
        (
            "pale moss carpet base",
            PALE_MOSS_CARPET_BOTTOM_NO_SIDES_BLOCK_STATE_ID,
            1.0625,
        ),
        ("five snow layers", SNOW_5_LAYERS_BLOCK_STATE_ID, 1.5),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, expected_y, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_steps_onto_low_campfire_and_lantern_shapes() {
    let cases = [
        ("campfire", CAMPFIRE_NORTH_LIT_BLOCK_STATE_ID, 1.4375),
        ("standing lantern", LANTERN_STANDING_BLOCK_STATE_ID, 1.5625),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, expected_y, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_lands_on_vanilla_low_ground_shapes() {
    let cases = [
        ("farmland", FARMLAND_MOISTURE_0_BLOCK_STATE_ID, 1.9375),
        ("dirt path", DIRT_PATH_BLOCK_STATE_ID, 1.9375),
        ("soul sand", SOUL_SAND_BLOCK_STATE_ID, 1.875),
        ("mud", MUD_BLOCK_STATE_ID, 1.875),
        ("cake", CAKE_BITES_0_BLOCK_STATE_ID, 1.5),
        ("lily pad", LILY_PAD_BLOCK_STATE_ID, 1.09375),
        ("flower pot", FLOWER_POT_BLOCK_STATE_ID, 1.375),
        ("potted dandelion", POTTED_DANDELION_BLOCK_STATE_ID, 1.375),
        ("one candle", CANDLE_ONE_DRY_UNLIT_BLOCK_STATE_ID, 1.375),
        ("four candles", CANDLE_FOUR_DRY_UNLIT_BLOCK_STATE_ID, 1.375),
        ("one sea pickle", SEA_PICKLE_ONE_DRY_BLOCK_STATE_ID, 1.375),
        (
            "four sea pickles",
            SEA_PICKLE_FOUR_DRY_BLOCK_STATE_ID,
            1.4375,
        ),
        (
            "pointed dripstone tip up",
            POINTED_DRIPSTONE_TIP_UP_BLOCK_STATE_ID,
            1.6875,
        ),
        (
            "pointed dripstone tip down",
            POINTED_DRIPSTONE_TIP_DOWN_BLOCK_STATE_ID,
            2.0,
        ),
        (
            "pointed dripstone base",
            POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID,
            2.0,
        ),
        ("skeleton skull", SKELETON_SKULL_BLOCK_STATE_ID, 1.5),
        ("piglin head", PIGLIN_HEAD_BLOCK_STATE_ID, 1.5),
        ("one turtle egg", TURTLE_EGG_ONE_BLOCK_STATE_ID, 1.4375),
        ("two turtle eggs", TURTLE_EGG_TWO_BLOCK_STATE_ID, 1.4375),
        ("sniffer egg", SNIFFER_EGG_BLOCK_STATE_ID, 2.0),
        ("dried ghast", DRIED_GHAST_NORTH_DRY_BLOCK_STATE_ID, 1.625),
        (
            "daylight detector",
            DAYLIGHT_DETECTOR_POWER_0_BLOCK_STATE_ID,
            1.375,
        ),
        ("sculk sensor", SCULK_SENSOR_INACTIVE_BLOCK_STATE_ID, 1.5),
        (
            "calibrated sculk sensor",
            CALIBRATED_SCULK_SENSOR_NORTH_INACTIVE_BLOCK_STATE_ID,
            1.5,
        ),
        ("sculk shrieker", SCULK_SHRIEKER_IDLE_BLOCK_STATE_ID, 1.5),
        ("heavy core", HEAVY_CORE_DRY_BLOCK_STATE_ID, 1.5),
        (
            "copper golem statue",
            COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID,
            1.875,
        ),
        (
            "up-facing amethyst cluster",
            AMETHYST_CLUSTER_UP_BLOCK_STATE_ID,
            1.4375,
        ),
        (
            "up-facing small amethyst bud",
            SMALL_AMETHYST_BUD_UP_BLOCK_STATE_ID,
            1.1875,
        ),
        (
            "empty end portal frame",
            END_PORTAL_FRAME_EMPTY_NORTH_BLOCK_STATE_ID,
            1.8125,
        ),
        (
            "eye end portal frame",
            END_PORTAL_FRAME_EYE_NORTH_BLOCK_STATE_ID,
            2.0,
        ),
        ("candle cake", CANDLE_CAKE_UNLIT_BLOCK_STATE_ID, 1.875),
        ("oak shelf", OAK_SHELF_NORTH_BLOCK_STATE_ID, 2.0),
        ("decorated pot", DECORATED_POT_NORTH_DRY_BLOCK_STATE_ID, 2.0),
        ("chorus plant", CHORUS_PLANT_NONE_BLOCK_STATE_ID, 1.8125),
        ("chorus flower", CHORUS_FLOWER_AGE_0_BLOCK_STATE_ID, 2.0),
        ("conduit", CONDUIT_DRY_BLOCK_STATE_ID, 1.6875),
        ("azalea", AZALEA_BLOCK_STATE_ID, 2.0),
        ("flowering azalea", FLOWERING_AZALEA_BLOCK_STATE_ID, 2.0),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 1.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, expected_y, 0.0005);
        assert!(pose.on_ground, "{name}");
        assert!(!pose.horizontal_collision, "{name}");
    }
}

#[test]
fn local_player_ignores_pale_moss_carpet_topper_collision() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        0,
        PALE_MOSS_CARPET_TOPPER_WITH_SIDES_BLOCK_STATE_ID,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_ignores_bamboo_sapling_collision() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, BAMBOO_SAPLING_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_ignores_vanilla_no_collision_hazard_blocks() {
    let cases = [
        ("cobweb", COBWEB_BLOCK_STATE_ID),
        ("sweet berry bush", SWEET_BERRY_BUSH_AGE_3_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_ignores_vanilla_no_collision_vegetation_and_overlays() {
    let cases = [
        ("dandelion", DANDELION_BLOCK_STATE_ID),
        ("dead bush", DEAD_BUSH_BLOCK_STATE_ID),
        ("bush", BUSH_BLOCK_STATE_ID),
        ("firefly bush", FIREFLY_BUSH_BLOCK_STATE_ID),
        (
            "mangrove propagule",
            MANGROVE_PROPAGULE_AGE_4_DRY_BLOCK_STATE_ID,
        ),
        ("crimson fungus", CRIMSON_FUNGUS_BLOCK_STATE_ID),
        ("nether sprouts", NETHER_SPROUTS_BLOCK_STATE_ID),
        ("wheat", WHEAT_AGE_7_BLOCK_STATE_ID),
        ("carrots", CARROTS_AGE_7_BLOCK_STATE_ID),
        ("potatoes", POTATOES_AGE_7_BLOCK_STATE_ID),
        ("beetroots", BEETROOTS_AGE_3_BLOCK_STATE_ID),
        ("nether wart", NETHER_WART_AGE_3_BLOCK_STATE_ID),
        ("sugar cane", SUGAR_CANE_AGE_0_BLOCK_STATE_ID),
        ("pink petals", PINK_PETALS_ONE_NORTH_BLOCK_STATE_ID),
        ("leaf litter", LEAF_LITTER_ONE_NORTH_BLOCK_STATE_ID),
        ("sculk vein", SCULK_VEIN_UP_DRY_BLOCK_STATE_ID),
        ("fire", FIRE_UP_AGE_0_BLOCK_STATE_ID),
        ("redstone wire", REDSTONE_WIRE_POWER_0_DOT_BLOCK_STATE_ID),
        ("tripwire", TRIPWIRE_EMPTY_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_lands_on_big_dripleaf_leaf_collision_shapes() {
    let cases = [
        (
            "none tilt",
            BIG_DRIPLEAF_NORTH_NONE_DRY_BLOCK_STATE_ID,
            1.9375,
        ),
        (
            "partial tilt",
            BIG_DRIPLEAF_NORTH_PARTIAL_DRY_BLOCK_STATE_ID,
            1.8125,
        ),
    ];

    for (name, block_state_id, expected_y) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, expected_y, 0.0005);
        assert!(pose.on_ground, "{name}");
        assert!(!pose.horizontal_collision, "{name}");
    }
}

#[test]
fn local_player_ignores_full_tilt_big_dripleaf_and_stem_collision() {
    let cases = [
        ("full tilt leaf", BIG_DRIPLEAF_NORTH_FULL_DRY_BLOCK_STATE_ID),
        ("stem", BIG_DRIPLEAF_STEM_NORTH_DRY_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.on_ground, "{name}");
        assert!(!pose.horizontal_collision, "{name}");
    }
}

#[test]
fn local_player_respects_cake_bite_collision_width() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, CAKE_BITES_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.1, 3.0, 1.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_walks_up_to_narrow_cactus_and_honey_columns() {
    let cases = [
        ("cactus", CACTUS_AGE_0_BLOCK_STATE_ID),
        ("honey block", HONEY_BLOCK_STATE_ID),
        ("dragon egg", DRAGON_EGG_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.7005 && pose.position.z <= 0.763,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_cocoa_pod_collision() {
    let cases = [
        (
            "north-facing mature cocoa",
            COCOA_AGE_2_NORTH_BLOCK_STATE_ID,
            0.763,
        ),
        (
            "south-facing mature cocoa",
            COCOA_AGE_2_SOUTH_BLOCK_STATE_ID,
            1.138,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.7005 && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_shelf_collision() {
    let cases = [
        (
            "north-facing shelf",
            OAK_SHELF_NORTH_BLOCK_STATE_ID,
            1.38,
            1.388,
        ),
        (
            "south-facing shelf",
            OAK_SHELF_SOUTH_BLOCK_STATE_ID,
            0.699,
            0.7005,
        ),
    ];

    for (name, block_state_id, min_z, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > min_z && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_amethyst_cluster_collision() {
    let cases = [
        (
            "north-facing amethyst cluster",
            AMETHYST_CLUSTER_NORTH_BLOCK_STATE_ID,
            1.25,
            1.263,
        ),
        (
            "south-facing amethyst cluster",
            AMETHYST_CLUSTER_SOUTH_BLOCK_STATE_ID,
            0.699,
            0.7005,
        ),
    ];

    for (name, block_state_id, min_z, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > min_z && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_copper_golem_statue_collision() {
    for (name, block_state_id) in [
        (
            "copper golem statue",
            COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID,
        ),
        (
            "waxed oxidized copper golem statue",
            WAXED_OXIDIZED_COPPER_GOLEM_STATUE_STANDING_NORTH_BLOCK_STATE_ID,
        ),
    ] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.887 && pose.position.z <= 0.888,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_collide_with_pressure_plate_outline() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, STONE_PRESSURE_PLATE_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 0.2);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.position.z > 1.0, "position was {:?}", pose.position);
    assert!(!pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_collide_with_no_collision_rails_and_controls() {
    let cases = [
        ("rail", RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("powered rail", POWERED_RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("detector rail", DETECTOR_RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("activator rail", ACTIVATOR_RAIL_NORTH_SOUTH_BLOCK_STATE_ID),
        ("torch", TORCH_BLOCK_STATE_ID),
        ("wall torch", WALL_TORCH_NORTH_BLOCK_STATE_ID),
        ("lever", LEVER_FLOOR_NORTH_BLOCK_STATE_ID),
        ("button", STONE_BUTTON_FLOOR_NORTH_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_collide_with_sign_outline() {
    let cases = [
        ("standing sign", OAK_SIGN_ROTATION_0_BLOCK_STATE_ID),
        ("wall sign", OAK_WALL_SIGN_NORTH_BLOCK_STATE_ID),
        (
            "ceiling hanging sign",
            OAK_HANGING_SIGN_ATTACHED_ROTATION_0_BLOCK_STATE_ID,
        ),
        (
            "wall hanging sign",
            OAK_WALL_HANGING_SIGN_NORTH_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_collide_with_banner_outline() {
    let cases = [
        ("standing banner", WHITE_BANNER_ROTATION_0_BLOCK_STATE_ID),
        ("wall banner", WHITE_WALL_BANNER_NORTH_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_collide_with_invisible_nonblocking_blocks() {
    let cases = [
        ("end portal", END_PORTAL_BLOCK_STATE_ID),
        ("end gateway", END_GATEWAY_BLOCK_STATE_ID),
        ("structure void", STRUCTURE_VOID_BLOCK_STATE_ID),
        ("light", LIGHT_LEVEL_15_DRY_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_walks_through_nether_portal_plane() {
    let cases = [
        ("x-axis nether portal", NETHER_PORTAL_X_BLOCK_STATE_ID),
        ("z-axis nether portal", NETHER_PORTAL_Z_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_copper_grates() {
    let cases = [
        ("copper grate", COPPER_GRATE_BLOCK_STATE_ID),
        ("waxed copper grate", WAXED_COPPER_GRATE_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_extended_piston_base_shape() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, PISTON_EXTENDED_NORTH_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z > 0.7005 && pose.position.z <= 0.9505,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_respects_piston_head_rod_extending_outside_block() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 2, PISTON_HEAD_SOUTH_LONG_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z > 0.7005 && pose.position.z <= 1.4505,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_lands_on_up_extended_piston_base_plate() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, PISTON_EXTENDED_UP_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.75, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_does_not_collide_with_moving_piston_without_block_entity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, MOVING_PISTON_NORTH_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 0.2);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.position.z > 1.0, "position was {:?}", pose.position);
    assert!(!pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_chain_or_rod_shapes() {
    let cases = [
        (
            "vertical chain",
            IRON_CHAIN_Y_AXIS_BLOCK_STATE_ID,
            0.9,
            1.107,
        ),
        (
            "vertical lightning rod",
            LIGHTNING_ROD_UP_UNPOWERED_BLOCK_STATE_ID,
            0.9,
            1.076,
        ),
        ("north end rod", END_ROD_NORTH_BLOCK_STATE_ID, 0.0, 0.7005),
    ];

    for (name, block_state_id, min_z, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > min_z && pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_ladder_sheet_on_approached_side() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, LADDER_SOUTH_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert!(
        pose.position.z <= 0.7005,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_moves_through_scaffolding_side_instead_of_full_block() {
    let mut world = flat_collision_world();
    set_test_block(
        &mut world,
        0,
        1,
        1,
        SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
    );
    let pose = advance_forward_from_standard_start(&mut world, 0.1);

    assert!(pose.position.z > 0.8, "position was {:?}", pose.position);
    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(!pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_step_over_tall_snow_layer() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, SNOW_6_LAYERS_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 0.7005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_door_and_trapdoor_shapes() {
    let cases = [
        ("closed door", OAK_CLOSED_NORTH_DOOR_BLOCK_STATE_ID, 1.5135),
        (
            "closed top trapdoor",
            OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID,
            0.7005,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_does_not_walk_through_fence_pane_or_wall_connections() {
    let cases = [
        ("north fence", OAK_NORTH_FENCE_BLOCK_STATE_ID),
        ("north pane", GLASS_NORTH_PANE_BLOCK_STATE_ID),
        (
            "north/east wall",
            COBBLESTONE_NORTH_EAST_WALL_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "{name} position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_closed_and_open_fence_gate_collision() {
    let mut closed_world = flat_collision_world();
    set_test_block(
        &mut closed_world,
        0,
        1,
        1,
        OAK_CLOSED_NORTH_FENCE_GATE_BLOCK_STATE_ID,
    );
    let closed_pose = advance_forward_from_standard_start(&mut closed_world, 1.0);

    assert!(
        closed_pose.position.z <= 1.0755,
        "position was {:?}",
        closed_pose.position
    );
    assert!(closed_pose.horizontal_collision);
    assert!(closed_pose.on_ground);

    let mut open_world = flat_collision_world();
    set_test_block(
        &mut open_world,
        0,
        1,
        1,
        OAK_OPEN_NORTH_FENCE_GATE_BLOCK_STATE_ID,
    );
    let open_pose = advance_forward_from_standard_start(&mut open_world, 0.35);

    assert!(
        open_pose.position.z > 1.5,
        "position was {:?}",
        open_pose.position
    );
    assert!(!open_pose.horizontal_collision);
    assert!(open_pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_common_object_shapes() {
    let cases = [
        ("single chest", CHEST_SINGLE_NORTH_BLOCK_STATE_ID, 0.763),
        ("double chest left", CHEST_LEFT_NORTH_BLOCK_STATE_ID, 0.763),
        (
            "trapped double chest right",
            TRAPPED_CHEST_RIGHT_NORTH_BLOCK_STATE_ID,
            0.763,
        ),
        ("ender chest", ENDER_CHEST_NORTH_BLOCK_STATE_ID, 0.763),
        (
            "water cauldron",
            WATER_CAULDRON_LEVEL_3_BLOCK_STATE_ID,
            0.7005,
        ),
        ("north hopper", HOPPER_NORTH_ENABLED_BLOCK_STATE_ID, 0.7005),
        ("enchanting table", ENCHANTING_TABLE_BLOCK_STATE_ID, 0.7005),
        ("anvil", ANVIL_NORTH_BLOCK_STATE_ID, 0.7005),
        (
            "north wall skull",
            SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID,
            1.2005,
        ),
        (
            "north wall grindstone",
            GRINDSTONE_WALL_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
        (
            "pointed dripstone base",
            POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID,
            0.909,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_respects_piglin_wall_head_wider_collision() {
    let mut normal_world = flat_collision_world();
    set_test_block(
        &mut normal_world,
        0,
        1,
        1,
        SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID,
    );
    normal_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.06, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let normal_pose = normal_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        normal_pose.position.z > 1.5,
        "normal wall skull position was {:?}",
        normal_pose.position
    );
    assert!(!normal_pose.horizontal_collision);
    assert!(normal_pose.on_ground);

    let mut piglin_world = flat_collision_world();
    set_test_block(
        &mut piglin_world,
        0,
        1,
        1,
        PIGLIN_WALL_HEAD_NORTH_BLOCK_STATE_ID,
    );
    piglin_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.06, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let piglin_pose = piglin_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        piglin_pose.position.z <= 1.2005,
        "piglin wall head position was {:?}",
        piglin_pose.position
    );
    assert!(piglin_pose.horizontal_collision);
    assert!(piglin_pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_grindstone_floor_and_ceiling_faces() {
    let cases = [
        (
            "north floor grindstone",
            GRINDSTONE_FLOOR_NORTH_BLOCK_STATE_ID,
        ),
        (
            "north ceiling grindstone",
            GRINDSTONE_CEILING_NORTH_BLOCK_STATE_ID,
        ),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.8255,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_steps_onto_lectern_base_but_stops_at_center_column() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, LECTERN_NORTH_NO_BOOK_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.125, 0.0005);
    assert!(
        pose.position.z > 0.7005 && pose.position.z <= 0.9505,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_does_not_walk_through_bell_attachments() {
    let cases = [
        ("floor bell", BELL_FLOOR_NORTH_BLOCK_STATE_ID, 0.9505),
        (
            "single-wall bell",
            BELL_SINGLE_WALL_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
        (
            "double-wall bell",
            BELL_DOUBLE_WALL_NORTH_BLOCK_STATE_ID,
            0.7005,
        ),
    ];

    for (name, block_state_id, max_z) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= max_z,
            "{name} position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_collision_detects_ceiling_bell_body() {
    let mut world = flat_collision_world();
    let pos = BlockPos { x: 0, y: 1, z: 1 };
    set_test_block(
        &mut world,
        pos.x,
        pos.y,
        pos.z,
        BELL_CEILING_NORTH_BLOCK_STATE_ID,
    );

    assert!(world.local_player_pose_collides_with_block(
        pos,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 1.5),
            ..LocalPlayerPoseState::default()
        },
    ));
}

#[test]
fn local_player_steps_onto_common_low_object_shapes() {
    let cases = [
        ("north foot bed", WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID),
        ("north stonecutter", STONECUTTER_NORTH_BLOCK_STATE_ID),
    ];

    for (name, block_state_id) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, block_state_id);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.5625, 0.0005);
        assert!(
            pose.position.z > 1.0,
            "{name} position was {:?}",
            pose.position
        );
        assert!(!pose.horizontal_collision, "{name}");
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_steps_onto_brewing_stand_base_but_stops_at_center_rod() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, BREWING_STAND_EMPTY_BLOCK_STATE_ID);
    let pose = advance_forward_from_standard_start(&mut world, 1.0);

    assert_f64_near(pose.position.y, 1.125, 0.0005);
    assert!(
        pose.position.z > 1.0 && pose.position.z <= 1.1376,
        "position was {:?}",
        pose.position
    );
    assert!(pose.horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_composter_collision_uses_level_zero_shape_for_every_level() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, COMPOSTER_LEVEL_7_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 1.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            2.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.125, 0.0005);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_sneak_backs_off_from_block_edge() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.z <= 1.3005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.sneaking);
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_sneak_uses_crouching_collision_height() {
    let mut standing_world = flat_collision_world();
    set_test_block(&mut standing_world, 0, 2, 1, OAK_TOP_SLAB_BLOCK_STATE_ID);
    standing_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let standing = standing_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(standing.horizontal_collision);
    assert!(
        standing.position.z < 1.0,
        "position was {:?}",
        standing.position
    );
    assert!(!standing.sneaking);
    assert!(!standing.swimming);

    let mut crouching_world = flat_collision_world();
    set_test_block(&mut crouching_world, 0, 2, 1, OAK_TOP_SLAB_BLOCK_STATE_ID);
    crouching_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let crouching = crouching_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert!(
        crouching.position.z > 1.0,
        "position was {:?}",
        crouching.position
    );
    assert!(crouching.sneaking);
    assert!(!crouching.swimming);
    assert!(!crouching.horizontal_collision);
}

#[test]
fn local_player_low_ceiling_forces_crouching_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 2, 0, OAK_TOP_SLAB_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
        * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
        * LOCAL_PHYSICS_TICK_SECONDS;
    assert!(pose.sneaking);
    assert!(!pose.swimming);
    assert!(!pose.horizontal_collision);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_low_tunnel_forces_crawling_swimming_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
        * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
        * LOCAL_PHYSICS_TICK_SECONDS;
    assert!(!pose.sneaking);
    assert!(pose.swimming);
    assert!(!pose.horizontal_collision);
    assert_f64_near(pose.body_height(), 0.6, 0.000001);
    assert_f64_near(pose.eye_height(), 0.4, 0.000001);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_underwater_sprint_uses_swimming_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.sneaking);
    assert!(pose.swimming);
    assert_f64_near(pose.body_height(), 0.6, 0.000001);
    assert_f64_near(pose.eye_height(), 0.4, 0.000001);
}

#[test]
fn local_player_low_ceiling_does_not_force_crouch_while_flying() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    set_test_block(&mut world, 0, 2, 0, OAK_TOP_SLAB_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.sneaking);
    assert!(!pose.swimming);
    assert_f64_near(pose.position.y, 1.0, 0.000001);
}

#[test]
fn local_player_low_tunnel_does_not_force_crawling_while_flying() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.sneaking);
    assert!(!pose.swimming);
    assert_f64_near(pose.position.y, 1.0, 0.000001);
}

#[test]
fn local_player_sneak_backs_off_from_near_ground_edge_while_falling() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 1.25),
        fall_distance: 0.2,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(
        pose.position.z <= 1.3005,
        "position was {:?}",
        pose.position
    );
    assert_f64_near(pose.position.y, 1.2, 0.0005);
    assert!(!pose.on_ground);
    assert!(pose.fall_distance < LOCAL_PLAYER_STEP_HEIGHT);
}

#[test]
fn local_player_sneak_edge_backoff_stops_after_step_height_fall_distance() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.3, 1.25),
        fall_distance: LOCAL_PLAYER_STEP_HEIGHT,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
        * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
        * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 1.25 + expected_step, 0.000001);
    assert_eq!(pose.fall_distance, LOCAL_PLAYER_STEP_HEIGHT);
}

#[test]
fn local_player_fall_distance_accumulates_downward_motion_and_resets_on_ground() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 0.2,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let falling = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(falling.position.y, 1.9, 0.000001);
    assert_f64_near(falling.fall_distance, 0.3, 0.000001);
    assert!(!falling.on_ground);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.05, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: falling.fall_distance,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let landed = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(landed.position.y, 1.0, 0.0005);
    assert!(landed.on_ground);
    assert_f64_near(landed.fall_distance, 0.0, 0.000001);
}

#[test]
fn local_player_fall_distance_resets_when_touching_water_surface() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.95, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 0.4,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.on_ground);
    assert_f64_near(pose.position.y, 1.85, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
}

#[test]
fn local_player_fall_distance_does_not_reset_above_low_flowing_water_surface() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.7, 0.5),
        delta_movement: vec3(0.0, -0.1, 0.0),
        fall_distance: 0.4,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.on_ground);
    assert_f64_near(pose.position.y, 1.6, 0.000001);
    assert_f64_near(pose.fall_distance, 0.5, 0.000001);
}

#[test]
fn local_player_in_water_uses_fluid_relative_acceleration_and_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        fall_distance: 0.4,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_in_flowing_water_applies_vanilla_current_before_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.514, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0112, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_water_movement_efficiency_half_applies_when_airborne() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.56, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0403800018, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_water_movement_efficiency_fully_applies_on_ground() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.6, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.054600006, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_dolphins_grace_overrides_water_horizontal_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(
        123,
        VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID,
        0,
    )));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0192, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_no_gravity_metadata_suppresses_water_falling_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(apply_no_gravity(&mut world, 123, true));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_bad_omen_does_not_apply_dolphins_grace_water_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BAD_OMEN_ID, 0,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_sprinting_in_water_uses_vanilla_sprint_drag_without_sinking_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.018, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_low_food_prevents_water_sprint_and_swimming_pose() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    apply_player_health(&mut world, 6);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(!pose.swimming);
    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_swimming_pitch_down_pulls_velocity_toward_look_y() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        x_rot: 30.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.0575, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.034, 0.000001);
    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.018, 0.000001);
}

#[test]
fn local_player_swimming_pitch_up_rises_when_fluid_above() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        x_rot: -30.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.13, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.024, 0.000001);
}

#[test]
fn local_player_swimming_pitch_up_near_surface_does_not_auto_rise() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        x_rot: -30.0,
        swimming: true,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.swimming);
    assert_f64_near(pose.position.y, 1.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_jump_and_sneak_in_water_apply_liquid_vertical_impulses() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let upward = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(upward.position.y, 1.14, 0.000001);
    assert_f64_near(upward.delta_movement.y, 0.027, 0.000001);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let downward = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(downward.position.y, 1.06, 0.000001);
    assert_f64_near(downward.delta_movement.y, -0.037, 0.000001);
}

#[test]
fn local_player_inside_upward_bubble_column_applies_vanilla_push() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.delta_movement.y, 0.055, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
}

#[test]
fn local_player_above_upward_bubble_column_uses_surface_push() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.delta_movement.y, 0.095, 0.000001);
}

#[test]
fn local_player_drag_down_bubble_column_applies_downward_velocity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(true));
    set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.delta_movement.y, -0.035, 0.000001);
}

#[test]
fn local_player_flying_ignores_bubble_column_push() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_in_water_jumps_out_when_horizontal_collision_has_clear_space() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.45, 0.69),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.horizontal_collision);
    assert_f64_near(
        pose.delta_movement.y,
        LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
        0.000001,
    );
}

#[test]
fn local_player_in_water_does_not_jump_out_when_clearance_still_contains_fluid() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.69),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.horizontal_collision);
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
}

#[test]
fn local_player_in_lava_jumps_out_when_horizontal_collision_has_clear_space() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.45, 0.69),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.horizontal_collision);
    assert_f64_near(
        pose.delta_movement.y,
        LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
        0.000001,
    );
}

#[test]
fn local_player_in_lava_uses_lava_drag_and_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.01, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_no_gravity_metadata_suppresses_lava_gravity() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(apply_no_gravity(&mut world, 123, true));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.01, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_in_lava_ignores_water_movement_efficiency() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.52, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.01, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_in_lava_current_uses_vanilla_minimum_push_before_drag() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.5045, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.00225, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_in_lava_current_skips_minimum_push_when_already_moving() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        delta_movement: vec3(0.01, 0.0, 0.0),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.x, 0.51, 0.000001);
    assert_f64_near(pose.position.z, 0.5023333333333333, 0.000001);
    assert_f64_near(pose.delta_movement.x, 0.005, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0011666666666666668, 0.000001);
}

#[test]
fn local_player_in_nether_lava_current_uses_fast_lava_push() {
    let mut world = flat_collision_world();
    world.level = Some(WorldLevelInfo {
        dimension: "minecraft:the_nether".to_string(),
        dimension_type_id: 1,
        dimension_type_name: Some("minecraft:the_nether".to_string()),
        cardinal_lighting: WorldCardinalLighting::Nether,
        last_death_location: None,
        sea_level: 32,
        is_debug: false,
        is_flat: false,
    });
    set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.507, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.0035, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
}

#[test]
fn local_player_flying_in_water_uses_flying_movement_not_fluid_travel() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.1, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = f64::from(0.05_f32);
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        expected_step * LOCAL_INPUT_FLY_AIR_DRAG,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_without_sneak_walks_off_block_edge() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            0.5,
        )
        .unwrap();

    assert!(pose.position.z > 1.3, "position was {:?}", pose.position);
    assert!(
        pose.position.y < 1.0 || !pose.on_ground,
        "pose was {:?}",
        pose
    );
}

#[test]
fn local_player_sneak_backs_off_from_block_corner_diagonally() {
    let mut world = single_floor_block_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                right: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    assert!(
        pose.position.x >= -0.3005 && pose.position.z <= 1.3005,
        "position was {:?}",
        pose.position
    );
    assert!(pose.on_ground);
    assert!(!pose.horizontal_collision);
}

#[test]
fn local_player_sneak_edge_backoff_does_not_apply_while_flying() {
    let mut world = single_floor_block_world();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            0.5,
        )
        .unwrap();

    assert!(pose.position.z > 1.3, "position was {:?}", pose.position);
    assert!(!pose.sneaking);
}

#[test]
fn local_player_flying_hovers_without_gravity() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            1.0,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.0, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_flying_jump_and_sneak_move_vertically() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let upward = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert_f64_near(upward.position.y, 3.15, 0.000001);
    assert_f64_near(upward.delta_movement.y, 0.09, 0.000001);
    assert!(!upward.on_ground);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let downward = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    assert_f64_near(downward.position.y, 2.85, 0.000001);
    assert_f64_near(downward.delta_movement.y, -0.09, 0.000001);
    assert!(!downward.sneaking);
    assert!(!downward.on_ground);
}

#[test]
fn local_player_flying_vertical_momentum_damps_without_input() {
    let mut world = flat_collision_world();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        delta_movement: vec3(0.0, 0.09, 0.0),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 3.09, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.054, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_flying_horizontal_velocity_accumulates_and_drags() {
    let mut world = WorldStore::new();
    apply_flying_abilities(&mut world, 0.05);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 64.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let first = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    let acceleration = f64::from(0.05_f32);
    assert_f64_near(first.position.z, 0.5 + acceleration, 0.000001);
    assert_f64_near(
        first.delta_movement.z,
        acceleration * LOCAL_INPUT_FLY_AIR_DRAG,
        0.000001,
    );

    let second = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    let second_step = first.delta_movement.z + acceleration;
    assert_f64_near(
        second.position.z,
        0.5 + acceleration + second_step,
        0.000001,
    );
    assert_f64_near(
        second.delta_movement.z,
        second_step * LOCAL_INPUT_FLY_AIR_DRAG,
        0.000001,
    );
}

#[test]
fn local_player_flying_uses_abilities_speed_and_sprint_multiplier() {
    let mut world = WorldStore::new();
    apply_flying_abilities(&mut world, 0.1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 64.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = f64::from(0.1_f32) * LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        expected_step * LOCAL_INPUT_FLY_AIR_DRAG,
        0.000001,
    );
    assert_f64_near(pose.position.y, 64.0, 0.000001);
}

#[test]
fn local_player_sneak_moves_over_supported_ground() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            0.2,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0005);
    let expected_step =
        LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER * 0.2;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_sneak_uses_default_sneaking_speed_multiplier() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
        * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
        * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_movement_speed_attribute_scales_horizontal_movement() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: 0.2,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 2.0 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_slow_block_speed_factor_scales_horizontal_movement() {
    for (name, block_state_id, player_y) in [
        ("soul sand", SOUL_SAND_BLOCK_STATE_ID, 0.875),
        ("honey block", HONEY_BLOCK_STATE_ID, 0.9375),
    ] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, player_y, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
            * SLOW_BLOCK_SPEED_FACTOR
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_no_collision_hazards_apply_stuck_speed_multiplier() {
    let cases = [
        ("cobweb", COBWEB_BLOCK_STATE_ID, 0.25),
        (
            "sweet berry bush",
            SWEET_BERRY_BUSH_AGE_3_BLOCK_STATE_ID,
            0.8,
        ),
    ];

    for (name, block_state_id, expected_multiplier) in cases {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, block_state_id);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
            * expected_multiplier
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
        assert!(pose.on_ground, "{name}");
    }
}

#[test]
fn local_player_weaving_effect_relaxes_cobweb_stuck_speed_multiplier() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, COBWEB_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_WEAVING_ID, 0)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.5 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_movement_efficiency_offsets_slow_block_speed_factor() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID,
            base: 1.0,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 0.875, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_block_speed_factor_does_not_fallback_through_water() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
    let factor = local_player_block_speed_factor(
        &world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_f64_near(factor, DEFAULT_BLOCK_SPEED_FACTOR, 0.000001);
}

#[test]
fn local_player_block_speed_factor_does_not_fallback_through_bubble_column() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
    let factor = local_player_block_speed_factor(
        &world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_f64_near(factor, DEFAULT_BLOCK_SPEED_FACTOR, 0.000001);
}

#[test]
fn local_player_ticks_frozen_applies_powder_snow_slowdown() {
    let mut half_frozen_world = flat_collision_world();
    attach_local_player_entity(&mut half_frozen_world, 123);
    assert!(apply_ticks_frozen(&mut half_frozen_world, 123, 70));

    let half_frozen_pose =
        advance_forward_from_standard_start(&mut half_frozen_world, LOCAL_PHYSICS_TICK_SECONDS);
    let half_frozen_step =
        LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.75 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(
        half_frozen_pose.position.z,
        0.5 + half_frozen_step,
        0.000001,
    );
    assert_f64_near(
        half_frozen_pose.delta_movement.z,
        half_frozen_step,
        0.000001,
    );

    let mut fully_frozen_world = flat_collision_world();
    attach_local_player_entity(&mut fully_frozen_world, 124);
    assert!(apply_ticks_frozen(&mut fully_frozen_world, 124, 280));

    let fully_frozen_pose =
        advance_forward_from_standard_start(&mut fully_frozen_world, LOCAL_PHYSICS_TICK_SECONDS);
    let fully_frozen_step =
        LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.5 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(
        fully_frozen_pose.position.z,
        0.5 + fully_frozen_step,
        0.000001,
    );
    assert_f64_near(
        fully_frozen_pose.delta_movement.z,
        fully_frozen_step,
        0.000001,
    );
}

#[test]
fn local_player_powder_snow_speed_modifier_does_not_double_apply_synced_attribute_modifier() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE,
            modifiers: vec![ProtocolAttributeModifier {
                id: VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID.to_string(),
                amount: -0.025,
                operation_id: 0,
            }],
        }],
    }));
    assert!(apply_ticks_frozen(&mut world, 123, 70));

    let pose = advance_forward_from_standard_start(&mut world, LOCAL_PHYSICS_TICK_SECONDS);
    let expected_step =
        LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.75 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_in_powder_snow_increments_ticks_frozen() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(world.entities.ticks_frozen(123), Some(1));
    assert_eq!(
        world
            .entities
            .metadata(123)
            .unwrap()
            .data_values
            .iter()
            .find(|value| { value.data_id == crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID })
            .map(|value| value.serializer_id),
        Some(VANILLA_ENTITY_DATA_INT_SERIALIZER_ID)
    );
}

#[test]
fn local_player_in_powder_snow_caps_ticks_frozen_at_required_ticks() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(
        &mut world,
        123,
        LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE,
    ));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(
        world.entities.ticks_frozen(123),
        Some(LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE)
    );
}

#[test]
fn local_player_out_of_powder_snow_thaws_two_ticks_per_step() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(world.entities.ticks_frozen(123), Some(3));
}

#[test]
fn local_player_out_of_powder_snow_thaw_clamps_at_zero() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 1));

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(world.entities.ticks_frozen(123), Some(0));
}

#[test]
fn local_player_spectator_in_powder_snow_thaws_instead_of_freezing() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(world.entities.ticks_frozen(123), Some(3));
}

#[test]
fn local_player_freeze_immune_wearable_in_powder_snow_thaws_instead_of_freezing() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
        item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 1),
    });

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(world.entities.ticks_frozen(123), Some(3));
}

#[test]
fn local_player_empty_freeze_immune_wearable_stack_in_powder_snow_still_freezes() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 5));
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
        item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 0),
    });

    local_player_update_powder_snow_freezing(
        &mut world,
        LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            ..LocalPlayerPoseState::default()
        },
    );

    assert_eq!(world.entities.ticks_frozen(123), Some(6));
}

#[test]
fn local_player_without_powder_snow_walkable_boots_sinks_through_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert!(pose.position.y < 2.0);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y < 0.0);
}

#[test]
fn local_player_leather_boots_stand_on_powder_snow_top_collision() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState::default(),
            LOCAL_PHYSICS_TICK_SECONDS * 2.0,
        )
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert_f64_near(pose.position.y, 2.0, 0.000001);
    assert!(pose.on_ground);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
}

#[test]
fn local_player_freeze_immune_non_leather_boots_do_not_walk_on_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
        item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 1),
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState::default(),
            LOCAL_PHYSICS_TICK_SECONDS * 2.0,
        )
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert!(pose.position.y < 2.0);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_sneak_descends_through_powder_snow_even_with_leather_boots() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 2.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS * 2.0,
        )
        .unwrap();

    let pose = world.local_player_pose().unwrap();
    assert!(pose.position.y < 2.0);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_high_fall_distance_uses_powder_snow_falling_collision_shape() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

    assert!(!world.local_player_pose_collides_with_block(
        BlockPos { x: 0, y: 1, z: 0 },
        LocalPlayerPoseState {
            position: vec3(0.5, 1.85, 0.5),
            fall_distance: 2.5,
            ..LocalPlayerPoseState::default()
        },
    ));
    assert!(world.local_player_pose_collides_with_block(
        BlockPos { x: 0, y: 1, z: 0 },
        LocalPlayerPoseState {
            position: vec3(0.5, 1.85, 0.5),
            fall_distance: 2.500001,
            ..LocalPlayerPoseState::default()
        },
    ));
}

#[test]
fn local_player_leather_boots_jump_climbs_out_of_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_without_leather_boots_jump_does_not_climb_out_of_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.2, 0.000001);
    assert!(pose.delta_movement.y < 0.0);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_leather_boots_horizontal_collision_climbs_out_of_powder_snow() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    equip_powder_snow_walkable_boots(&mut world, 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert!(pose.horizontal_collision);
    assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_advance_thaws_ticks_frozen_per_physics_step() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(apply_ticks_frozen(&mut world, 123, 7));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    world
        .advance_local_player_input(
            LocalPlayerInputState::default(),
            LOCAL_PHYSICS_TICK_SECONDS * 3.0,
        )
        .unwrap();

    assert_eq!(world.entities.ticks_frozen(123), Some(1));
}

#[test]
fn local_player_blindness_prevents_sprint_speed() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BLINDNESS_ID, 0,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_low_food_prevents_sprint_speed_unless_mayfly() {
    let mut survival_world = flat_collision_world();
    apply_player_health(&mut survival_world, 6);
    survival_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let survival_pose = survival_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    let walk_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(survival_pose.position.z, 0.5 + walk_step, 0.000001);
    assert_f64_near(survival_pose.delta_movement.z, walk_step, 0.000001);

    let mut creative_world = flat_collision_world();
    apply_player_health(&mut creative_world, 6);
    creative_world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    creative_world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let creative_pose = creative_world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();
    let sprint_step = LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(creative_pose.position.z, 0.5 + sprint_step, 0.000001);
    assert_f64_near(creative_pose.delta_movement.z, sprint_step, 0.000001);
}

#[test]
fn local_player_using_item_applies_default_use_effects_slowdown_and_sprint_suppression() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(42, 1),
    });
    world.set_local_using_item(true);

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.2 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_using_item_can_sprint_override_preserves_sprint_speed() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack_with_use_effects(42, 1, true, false, 1.0),
    });
    world.set_local_using_item(true);

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_effective_sprint_uses_sprintable_vehicle_instead_of_food_when_mounted() {
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        sprint: true,
        ..LocalPlayerInputState::default()
    };

    let mut camel_world = WorldStore::new();
    apply_player_health(&mut camel_world, 1);
    mount_local_player_on_entity(
        &mut camel_world,
        99,
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        vec![99],
    );
    assert!(camel_world.local_player_effective_sprint(input));

    let mut horse_world = WorldStore::new();
    apply_player_health(&mut horse_world, 20);
    mount_local_player_on_entity(
        &mut horse_world,
        99,
        20,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        vec![99],
    );
    assert!(!horse_world.local_player_effective_sprint(input));

    let mut boat_world = WorldStore::new();
    apply_player_health(&mut boat_world, 20);
    mount_local_player_on_entity(
        &mut boat_world,
        99,
        30,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        vec![99],
    );
    assert!(!boat_world.local_player_effective_sprint(input));
}

#[test]
fn local_player_effective_sprint_requires_controlling_sprintable_vehicle_passenger() {
    let mut world = WorldStore::new();
    apply_player_health(&mut world, 20);
    world.apply_add_entity(ProtocolAddEntity {
        id: 123,
        uuid: Uuid::from_u128(0x87654321876543218765432187654321),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    mount_local_player_on_entity(
        &mut world,
        99,
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        vec![123, 99],
    );

    assert!(!world.local_player_effective_sprint(LocalPlayerInputState {
        focused: true,
        forward: true,
        sprint: true,
        ..LocalPlayerInputState::default()
    }));
}

#[test]
fn local_player_speed_and_slowness_effects_scale_horizontal_movement() {
    let mut speed_world = flat_collision_world();
    attach_local_player_entity(&mut speed_world, 123);
    assert!(speed_world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SPEED_ID, 1,)));
    let speed_pose =
        advance_forward_from_standard_start(&mut speed_world, LOCAL_PHYSICS_TICK_SECONDS);
    let speed_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 1.4 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(speed_pose.position.z, 0.5 + speed_step, 0.000001);
    assert_f64_near(speed_pose.delta_movement.z, speed_step, 0.000001);

    let mut slowness_world = flat_collision_world();
    attach_local_player_entity(&mut slowness_world, 124);
    assert!(slowness_world.apply_update_mob_effect(mob_effect(
        124,
        VANILLA_MOB_EFFECT_SLOWNESS_ID,
        0,
    )));
    let slowness_pose =
        advance_forward_from_standard_start(&mut slowness_world, LOCAL_PHYSICS_TICK_SECONDS);
    let slowness_step =
        LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.85 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(slowness_pose.position.z, 0.5 + slowness_step, 0.000001);
    assert_f64_near(slowness_pose.delta_movement.z, slowness_step, 0.000001);
}

#[test]
fn local_player_speed_effect_does_not_double_apply_synced_attribute_modifier() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE,
            modifiers: vec![ProtocolAttributeModifier {
                id: VANILLA_SPEED_EFFECT_MODIFIER_ID.to_string(),
                amount: 0.4,
                operation_id: 2,
            }],
        }],
    }));
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SPEED_ID, 1,)));

    let pose = advance_forward_from_standard_start(&mut world, LOCAL_PHYSICS_TICK_SECONDS);
    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 1.4 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
}

#[test]
fn local_player_sneak_uses_sneaking_speed_attribute_when_present() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID,
            base: 0.4,
            modifiers: Vec::new(),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.4 * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_jump_uses_jump_strength_attribute_and_jump_boost_effect() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
            base: 0.5,
            modifiers: Vec::new(),
        }],
    }));
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_JUMP_BOOST_ID, 1,)));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.7, 0.000001);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_honey_block_jump_factor_scales_base_jump() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
    let honey_top_y = 15.0 / 16.0;
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, honey_top_y, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_jump = LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE * HONEY_BLOCK_JUMP_FACTOR;
    assert_f64_near(pose.position.y, honey_top_y + expected_jump, 0.000001);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_honey_block_jump_factor_keeps_jump_boost_additive() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
    attach_local_player_entity(&mut world, 123);
    assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_JUMP_BOOST_ID, 0,)));
    let honey_top_y = 15.0 / 16.0;
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, honey_top_y, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_jump = LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE * HONEY_BLOCK_JUMP_FACTOR
        + JUMP_BOOST_VELOCITY_PER_LEVEL;
    assert_f64_near(pose.position.y, honey_top_y + expected_jump, 0.000001);
    assert!(!pose.on_ground);
    assert!(pose.delta_movement.y > 0.0);
}

#[test]
fn local_player_bounces_after_landing_on_slime_block() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SLIME_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0001);
    assert_f64_near(pose.delta_movement.y, 0.5, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_sneak_suppresses_slime_block_bounce() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 0, 0, SLIME_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.2, 0.5),
        delta_movement: vec3(0.0, -0.5, 0.0),
        fall_distance: 2.0,
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                sneak: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.y, 1.0, 0.0001);
    assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    assert!(pose.on_ground);
}

#[test]
fn local_player_sprint_jump_adds_vanilla_horizontal_impulse() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                jump: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_z = 0.5
        + LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS
        + SPRINT_JUMP_HORIZONTAL_IMPULSE;
    assert_f64_near(
        pose.position.y,
        1.0 + LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE,
        0.000001,
    );
    assert_f64_near(pose.position.z, expected_z, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_z - 0.5, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_ineligible_sprint_jump_does_not_add_horizontal_impulse() {
    let mut world = flat_collision_world();
    apply_player_health(&mut world, 6);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                jump: true,
                sprint: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_z = 0.5 + LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
    assert_f64_near(
        pose.position.y,
        1.0 + LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE,
        0.000001,
    );
    assert_f64_near(pose.position.z, expected_z, 0.000001);
    assert_f64_near(pose.delta_movement.z, expected_z - 0.5, 0.000001);
    assert!(!pose.on_ground);
}

#[test]
fn local_player_jump_starts_only_from_ground() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    let jump_pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            0.05,
        )
        .unwrap();

    assert!(
        jump_pose.position.y > 1.0,
        "position was {:?}",
        jump_pose.position
    );
    assert!(!jump_pose.on_ground);
    assert!(jump_pose.delta_movement.y > 0.0);

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 3.0, 0.5),
        on_ground: false,
        ..LocalPlayerPoseState::default()
    });
    let airborne_pose = world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                jump: true,
                ..LocalPlayerInputState::default()
            },
            0.1,
        )
        .unwrap();

    assert!(
        airborne_pose.position.y < 3.0,
        "position was {:?}",
        airborne_pose.position
    );
    assert!(!airborne_pose.on_ground);
}

fn advance_forward_from_standard_start(
    world: &mut WorldStore,
    seconds: f64,
) -> LocalPlayerPoseState {
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    world
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            seconds,
        )
        .unwrap()
}

fn flat_collision_world() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_test_chunk());
    for x in 0..3 {
        for z in 0..8 {
            set_test_block(&mut world, x, 0, z, GRASS_BLOCK_STATE_ID);
        }
    }
    world
}

fn single_floor_block_world() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_test_chunk());
    set_test_block(&mut world, 0, 0, 0, GRASS_BLOCK_STATE_ID);
    world
}

fn attach_local_player_entity(world: &mut WorldStore, id: i32) {
    world.local_player_id = Some(id);
    world.apply_add_entity(ProtocolAddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
}

fn mount_local_player_on_entity(
    world: &mut WorldStore,
    player_id: i32,
    vehicle_id: i32,
    entity_type_id: i32,
    passenger_ids: Vec<i32>,
) {
    attach_local_player_entity(world, player_id);
    world.apply_add_entity(ProtocolAddEntity {
        id: vehicle_id,
        uuid: Uuid::from_u128(0x22345678123456781234567812345678),
        entity_type_id,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id,
        passenger_ids,
    }));
}

fn mob_effect(entity_id: i32, effect_id: i32, amplifier: i32) -> ProtocolUpdateMobEffect {
    ProtocolUpdateMobEffect {
        entity_id,
        effect_id,
        amplifier,
        duration_ticks: 200,
        flags: ProtocolMobEffectFlags::default(),
    }
}

fn apply_player_health(world: &mut WorldStore, food: i32) {
    world.apply_player_health(ProtocolPlayerHealth {
        health: 20.0,
        food,
        saturation: 5.0,
    });
}

fn apply_no_gravity(world: &mut WorldStore, entity_id: i32, no_gravity: bool) -> bool {
    world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![ProtocolEntityDataValue {
            data_id: crate::entities::VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
            serializer_id: 8,
            value: ProtocolEntityDataValueKind::Boolean(no_gravity),
        }],
    })
}

fn apply_ticks_frozen(world: &mut WorldStore, entity_id: i32, ticks_frozen: i32) -> bool {
    world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![ProtocolEntityDataValue {
            data_id: crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
            serializer_id: 1,
            value: ProtocolEntityDataValueKind::Int(ticks_frozen),
        }],
    })
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: bbb_protocol::packets::DataComponentPatchSummary::default(),
    }
}

fn item_stack_with_use_effects(
    item_id: i32,
    count: i32,
    can_sprint: bool,
    interact_vibrations: bool,
    speed_multiplier: f32,
) -> ProtocolItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.added = 1;
    stack.component_patch.added_type_ids = vec![5];
    stack.component_patch.use_effects = Some(ProtocolUseEffectsSummary {
        can_sprint,
        interact_vibrations,
        speed_multiplier,
    });
    stack
}

fn equip_powder_snow_walkable_boots(world: &mut WorldStore, count: i32) {
    world.set_powder_snow_walkable_foot_item_ids(BTreeSet::from([LEATHER_BOOTS_ITEM_ID]));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
        item: item_stack(LEATHER_BOOTS_ITEM_ID, count),
    });
}

fn apply_flying_abilities(world: &mut WorldStore, flying_speed: f32) {
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: true,
        can_fly: true,
        instabuild: false,
        flying_speed,
        walking_speed: 0.1,
    });
}

fn bubble_column_block_state_id(drag_down: bool) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("drag".to_string(), drag_down.to_string());
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:bubble_column", &properties)
        .expect("vanilla 26.1 bubble_column block state exists")
        .id
}

fn empty_test_chunk() -> ChunkColumn {
    ChunkColumn {
        pos: crate::ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(
                PaletteDomain::BlockStates,
                4096,
                AIR_BLOCK_STATE_ID,
            ),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        }],
        block_entities: Vec::new(),
        light: LightData::default(),
    }
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

fn set_test_block(world: &mut WorldStore, x: i32, y: i32, z: i32, block_state_id: i32) {
    assert!(
        world.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: bbb_protocol::packets::BlockPos { x, y, z },
            block_state_id,
        })
    );
}

fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
    ProtocolVec3d { x, y, z }
}

fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "expected {actual} to be within {epsilon} of {expected}"
    );
}
