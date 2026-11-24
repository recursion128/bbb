use super::*;
use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_FEET_SLOT;
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_BAD_OMEN_ID, VANILLA_MOB_EFFECT_WEAVING_ID,
};
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind as ProtocolEntityDataValueKind, GameEvent as ProtocolGameEvent,
    InitializeBorder as ProtocolInitializeBorder, ItemStackSummary as ProtocolItemStackSummary,
    MobEffectFlags as ProtocolMobEffectFlags, PlayerHealth as ProtocolPlayerHealth,
    RegistryTags as ProtocolRegistryTags, SetEntityData as ProtocolSetEntityData,
    SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
    TagNetworkPayload as ProtocolTagNetworkPayload, UpdateAttributes as ProtocolUpdateAttributes,
    UpdateMobEffect as ProtocolUpdateMobEffect, UpdateTags as ProtocolUpdateTags,
    UseEffectsSummary as ProtocolUseEffectsSummary,
};
use uuid::Uuid;

use crate::{
    entities::{
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    },
    BlockPos, ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldCardinalLighting, WorldDimension, WorldLevelInfo,
};

mod flight_and_falling;
mod fluids;
mod sneak_and_sprint;
mod status_and_effects;
mod surfaces;

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
const TEST_DEFAULT_GROUND_FRICTION: f64 = 0.6;
const TEST_DEFAULT_MOVEMENT_SPEED: f64 = 0.1;

/// First-tick `travelInAir` displacement from rest on ground:
/// modifyInput scale x getSpeed x (0.21600002 / friction^3).
/// First-tick fluid `moveRelative` step: the `modifyInput` scale times the
/// fluid-relative speed.
fn first_fluid_step(speed: f64) -> f64 {
    LOCAL_PLAYER_INPUT_SCALE * speed
}

fn first_ground_step(speed: f64, input_scale: f64) -> f64 {
    input_scale
        * LOCAL_PLAYER_INPUT_SCALE
        * speed
        * (LOCAL_PLAYER_GROUND_ACCELERATION_FACTOR
            / (TEST_DEFAULT_GROUND_FRICTION
                * TEST_DEFAULT_GROUND_FRICTION
                * TEST_DEFAULT_GROUND_FRICTION))
}

/// The carried horizontal delta after an uncollided ground tick: the step
/// times blockSpeedFactor(1.0) times friction (0.6 * 0.91).
fn carried_after_ground_step(step: f64) -> f64 {
    step * TEST_DEFAULT_GROUND_FRICTION * LOCAL_HORIZONTAL_FRICTION
}

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
    assert!(!pose.minor_horizontal_collision);
    assert!(pose.on_ground);
}

#[test]
fn local_player_minor_horizontal_collision_tracks_tangential_input_in_air_and_fluid() {
    for fluid_state in [None, Some(SOURCE_WATER_BLOCK_STATE_ID)] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, GRASS_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
        if let Some(fluid_state) = fluid_state {
            set_test_block(&mut world, 1, 1, 0, fluid_state);
        }
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(1.3, 1.0, 0.5),
            delta_movement: vec3(-0.2, 0.0, 0.0),
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

        assert!(pose.horizontal_collision, "fluid state {fluid_state:?}");
        assert!(
            pose.minor_horizontal_collision,
            "fluid state {fluid_state:?}, pose {pose:?}"
        );
    }
}

#[test]
fn local_player_minor_horizontal_collision_angle_threshold_matches_vanilla() {
    let pose = LocalPlayerPoseState::default();
    assert!(local_player_horizontal_collision_is_minor(
        pose,
        0.0,
        0.98,
        [0.0, 0.0, 0.2],
    ));
    assert!(!local_player_horizontal_collision_is_minor(
        pose,
        0.0,
        0.98,
        [0.2, 0.0, 0.0],
    ));
    assert!(!local_player_horizontal_collision_is_minor(
        pose,
        0.0,
        0.0,
        [0.0, 0.0, 0.2],
    ));
}

#[test]
fn local_player_suffocation_uses_live_state_fact_and_collision_shape() {
    let mut world = flat_collision_world();
    let pose = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    let pos = BlockPos { x: 0, y: 1, z: 0 };

    // Farmland's 15/16-block collision shape is nevertheless explicitly
    // suffocating; mangrove roots retain collision but override the predicate
    // to false.
    set_test_block(
        &mut world,
        pos.x,
        pos.y,
        pos.z,
        FARMLAND_MOISTURE_0_BLOCK_STATE_ID,
    );
    assert!(world.local_player_suffocates_at(pose, pos, false));
    set_test_block(
        &mut world,
        pos.x,
        pos.y,
        pos.z,
        MANGROVE_ROOTS_DRY_BLOCK_STATE_ID,
    );
    assert!(!world.local_player_suffocates_at(pose, pos, false));
}

#[test]
fn local_player_suffocation_escape_selects_closest_open_direction() {
    let mut world = flat_collision_world();
    for (x, z) in [(1, 1), (0, 1), (1, 0), (1, 2)] {
        set_test_block(&mut world, x, 1, z, GRASS_BLOCK_STATE_ID);
    }
    let mut pose = LocalPlayerPoseState {
        position: vec3(1.5, 1.0, 1.5),
        delta_movement: vec3(-0.4, 0.25, 0.3),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };

    move_local_player_out_of_suffocating_blocks(&world, &mut pose, false);

    assert_f64_near(pose.delta_movement.x, 0.1, 0.000001);
    assert_f64_near(pose.delta_movement.y, 0.25, 0.000001);
    assert_f64_near(pose.delta_movement.z, 0.3, 0.000001);
}

#[test]
fn local_player_spectator_skips_suffocation_escape() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    for (x, z) in [(1, 1), (0, 1), (1, 0), (1, 2)] {
        set_test_block(&mut world, x, 1, z, GRASS_BLOCK_STATE_ID);
    }
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(1.5, 1.0, 1.5),
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

    assert_f64_near(pose.position.x, 1.5, 0.000001);
    assert_f64_near(pose.delta_movement.x, 0.0, 0.000001);
}

#[test]
fn local_player_auto_jump_arms_on_block_and_forces_next_tick_jump() {
    let mut world = flat_collision_world();
    set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        y_rot: 0.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        ..LocalPlayerInputState::default()
    };

    let first = world
        .advance_local_player_input(input, LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert!(world.local_player_auto_jump_is_armed());
    assert_f64_near(first.position.y, 1.0, 0.000001);

    let second = world
        .advance_local_player_input(input, LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
    assert!(
        second.position.y > 1.0,
        "position was {:?}",
        second.position
    );
    assert!(second.delta_movement.y > 0.0);
}

#[test]
fn local_player_auto_jump_disabled_option_and_low_jump_factor_do_not_arm() {
    for (label, enabled, support_state) in [
        ("disabled", false, GRASS_BLOCK_STATE_ID),
        ("honey support", true, HONEY_BLOCK_STATE_ID),
    ] {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, support_state);
        set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
        world.set_local_player_auto_jump_enabled(enabled);
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
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(
            !world.local_player_auto_jump_is_armed(),
            "{label} unexpectedly armed auto-jump"
        );
    }
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
fn local_player_shallow_water_stops_run_sprint_before_fluid_travel() {
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
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    assert_f64_near(pose.position.z, 0.5 + first_fluid_step(0.02), 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        first_fluid_step(0.02) * 0.8,
        0.000001,
    );
    assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
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

    let expected_step = first_ground_step(
        TEST_DEFAULT_MOVEMENT_SPEED,
        LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER,
    );
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
    assert!(pose.on_ground);
}

#[test]
fn camera_fov_includes_sprint_speed_without_mutating_movement_attribute() {
    let mut world = flat_collision_world();
    attach_local_player_entity(&mut world, 123);
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.2,
    });

    assert!((world.local_player_abilities_fov_modifier(false) - 0.75).abs() < 1.0e-6);
    assert!(world.probe_entity(123).unwrap().attributes.is_empty());

    assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 123,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            base: 0.2,
            modifiers: Vec::new(),
        }],
    }));
    assert!((world.local_player_abilities_fov_modifier(false) - 1.0).abs() < 1.0e-6);
    assert!((world.local_player_abilities_fov_modifier(true) - 1.15).abs() < 1.0e-6);

    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: true,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.2,
    });
    assert!((world.local_player_abilities_fov_modifier(false) - 1.1).abs() < 1.0e-6);
    assert!((world.local_player_abilities_fov_modifier(true) - 1.265).abs() < 1.0e-6);
}

#[test]
fn local_player_using_item_slows_input_without_stopping_an_active_sprint_latch() {
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
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = first_ground_step(
        TEST_DEFAULT_MOVEMENT_SPEED * LOCAL_PLAYER_SPRINT_SPEED_MODIFIER,
        0.2,
    );
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
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
                sprinting: true,
                ..LocalPlayerInputState::default()
            },
            LOCAL_PHYSICS_TICK_SECONDS,
        )
        .unwrap();

    let expected_step = first_ground_step(
        TEST_DEFAULT_MOVEMENT_SPEED * LOCAL_PLAYER_SPRINT_SPEED_MODIFIER,
        1.0,
    );
    assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
    assert_f64_near(
        pose.delta_movement.z,
        carried_after_ground_step(expected_step),
        0.000001,
    );
}

#[test]
fn local_player_sprint_latch_uses_sprintable_vehicle_instead_of_food_when_mounted() {
    let input = LocalPlayerInputState {
        focused: true,
        forward: true,
        sprinting: true,
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
    camel_world.set_local_player_pose(LocalPlayerPoseState::default());
    assert!(camel_world.reconcile_local_player_sprinting(input, true));
    assert!(camel_world.local_player_is_sprinting());

    let mut horse_world = WorldStore::new();
    apply_player_health(&mut horse_world, 20);
    mount_local_player_on_entity(
        &mut horse_world,
        99,
        20,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        vec![99],
    );
    horse_world.set_local_player_pose(LocalPlayerPoseState::default());
    assert!(!horse_world.reconcile_local_player_sprinting(input, true));
    assert!(!horse_world.local_player_is_sprinting());

    let mut boat_world = WorldStore::new();
    apply_player_health(&mut boat_world, 20);
    mount_local_player_on_entity(
        &mut boat_world,
        99,
        30,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        vec![99],
    );
    boat_world.set_local_player_pose(LocalPlayerPoseState::default());
    assert!(!boat_world.reconcile_local_player_sprinting(input, true));
    assert!(!boat_world.local_player_is_sprinting());
}

#[test]
fn local_player_sprint_latch_requires_controlling_sprintable_vehicle_passenger() {
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

    assert!(!world.reconcile_local_player_sprinting(
        LocalPlayerInputState {
            focused: true,
            forward: true,
            sprinting: false,
            ..LocalPlayerInputState::default()
        },
        true,
    ));
    assert!(!world.local_player_is_sprinting());
}

fn advance_forward_from_standard_start(
    world: &mut WorldStore,
    seconds: f64,
) -> LocalPlayerPoseState {
    // This shared harness isolates travel/collision-shape behavior. Dedicated
    // auto-jump tests above exercise the default-enabled option and timer.
    world.set_local_player_auto_jump_enabled(false);
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

#[test]
fn local_player_sprint_particle_state_applies_vanilla_gates_and_on_pos_facts() {
    let mut world = flat_collision_world();
    let standing = LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: vec3(0.25, -0.08, -0.1),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    world.set_local_player_pose(standing);
    assert!(
        world.local_player_sprint_particle_state().is_none(),
        "no sprint flag -> no particle"
    );

    world.local_player.sprinting = true;
    let state = world
        .local_player_sprint_particle_state()
        .expect("sprinting on grass emits the on-pos facts");
    assert_eq!(state.block_state_id, GRASS_BLOCK_STATE_ID);
    assert_eq!(state.on_pos, BlockPos { x: 0, y: 0, z: 0 });
    assert_eq!(state.entity_block_pos, BlockPos { x: 0, y: 1, z: 0 });
    assert_f64_near(state.width, 0.6, 0.000001);
    assert_f64_near(state.velocity_x, -1.0, 0.000001);
    assert_f64_near(state.velocity_z, 0.4, 0.000001);
    assert_f64_near(state.position.y, 1.0, 0.000001);

    world.set_local_player_pose(LocalPlayerPoseState {
        sneaking: true,
        ..standing
    });
    assert!(
        world.local_player_sprint_particle_state().is_none(),
        "crouching suppresses the sprint particle"
    );

    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 8.0, 0.5),
        on_ground: false,
        ..standing
    });
    assert!(
        world.local_player_sprint_particle_state().is_none(),
        "the airborne on-pos block is invisible air"
    );

    world.set_local_player_pose(standing);
    world.local_player.abilities = Some(LocalPlayerAbilitiesState {
        invulnerable: false,
        flying: true,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert!(
        world.local_player_sprint_particle_state().is_none(),
        "the LocalPlayer override suppresses flying sprint particles"
    );
    world.local_player.abilities = None;

    world.local_player.health = Some(crate::LocalPlayerHealthState {
        health: 0.0,
        food: 20,
        saturation: 0.0,
    });
    assert!(
        world.local_player_sprint_particle_state().is_none(),
        "dead players spawn no sprint particles"
    );
    world.local_player.health = None;

    world.gameplay.game_type = 3;
    assert!(
        world.local_player_sprint_particle_state().is_none(),
        "spectators spawn no sprint particles"
    );
    world.gameplay.game_type = 0;
    assert!(world.local_player_sprint_particle_state().is_some());
}

#[test]
fn local_player_portal_transition_effect_tracks_vanilla_intensity_curve() {
    let mut world = flat_collision_world();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 1.0, 0.5),
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });

    world.advance_local_player_portal_transition_effect();
    assert_eq!(world.local_player.portal_effect_intensity, 0.0);
    assert_eq!(
        world.take_pending_portal_transition_effects(),
        (false, false),
        "outside a portal nothing is pending"
    );

    set_test_block(&mut world, 0, 1, 0, NETHER_PORTAL_X_BLOCK_STATE_ID);
    world.advance_local_player_portal_transition_effect();
    assert_f32_near(world.local_player.portal_effect_intensity, 0.0125);
    assert_eq!(world.local_player.previous_portal_effect_intensity, 0.0);
    assert_eq!(
        world.take_pending_portal_transition_effects(),
        (true, false),
        "the 0 -> rising edge queues the portal trigger sound once"
    );

    world.advance_local_player_portal_transition_effect();
    assert_f32_near(world.local_player.portal_effect_intensity, 0.025);
    assert_f32_near(world.local_player.previous_portal_effect_intensity, 0.0125);
    // `Gui.extractCameraOverlays` interpolates the two intensities.
    assert_f32_near(world.local_player_portal_effect_intensity(0.0), 0.0125);
    assert_f32_near(world.local_player_portal_effect_intensity(1.0), 0.025);
    assert_f32_near(world.local_player_portal_effect_intensity(0.5), 0.01875);
    assert_f32_near(world.local_player_portal_effect_intensity(f32::NAN), 0.0125);
    assert_eq!(
        world.take_pending_portal_transition_effects(),
        (false, false),
        "the trigger sound only fires on the rising edge"
    );

    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Container"),
    });
    world.advance_local_player_portal_transition_effect();
    assert_f32_near(world.local_player.portal_effect_intensity, 0.0375);
    assert_eq!(
        world.take_pending_portal_transition_effects(),
        (false, true),
        "standing inside the portal closes any open container screen"
    );

    set_test_block(&mut world, 0, 1, 0, GRASS_BLOCK_STATE_ID);
    world.advance_local_player_portal_transition_effect();
    assert_eq!(
        world.local_player.portal_effect_intensity, 0.0,
        "leaving the portal decays by 0.05 per tick, clamped at zero"
    );
    assert_f32_near(world.local_player.previous_portal_effect_intensity, 0.0375);

    assert!(
        world.close_local_container(7),
        "the consumer closes the container after draining the pending flag"
    );
    world.local_player.portal_effect_intensity = 0.999;
    set_test_block(&mut world, 0, 1, 0, NETHER_PORTAL_X_BLOCK_STATE_ID);
    world.advance_local_player_portal_transition_effect();
    assert_eq!(
        world.local_player.portal_effect_intensity, 1.0,
        "the rising intensity clamps at one"
    );
    assert_eq!(
        world.take_pending_portal_transition_effects(),
        (false, false),
        "a non-zero starting intensity does not retrigger the sound"
    );
}

fn assert_f32_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-6,
        "expected {actual} to be within 1e-6 of {expected}"
    );
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
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: ProtocolEntityDataValueKind::Boolean(no_gravity),
        }],
    })
}

fn advance_effects_tick_at(world: &mut WorldStore, position: ProtocolVec3d) {
    world.set_local_player_pose(LocalPlayerPoseState {
        position,
        ..LocalPlayerPoseState::default()
    });
    world
        .advance_local_player_input(LocalPlayerInputState::default(), LOCAL_PHYSICS_TICK_SECONDS)
        .unwrap();
}

fn apply_ticks_frozen(world: &mut WorldStore, entity_id: i32, ticks_frozen: i32) -> bool {
    world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![ProtocolEntityDataValue {
            data_id: crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: ProtocolEntityDataValueKind::Int(ticks_frozen),
        }],
    })
}

fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: bbb_protocol::packets::DataComponentPatchSummary::default().into(),
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
        slot: VANILLA_PLAYER_INVENTORY_FEET_SLOT,
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
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(
                PaletteDomain::BlockStates,
                4096,
                crate::vanilla_air_block_state_id(),
            ),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
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
    // Scene setup tolerates the vanilla identical-state no-op; the block must
    // hold the requested state either way.
    world.apply_block_update(bbb_protocol::packets::BlockUpdate {
        pos: bbb_protocol::packets::BlockPos { x, y, z },
        block_state_id,
    });
    assert_eq!(
        world
            .probe_block(crate::BlockPos { x, y, z })
            .map(|block| block.block_state_id),
        Some(block_state_id)
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

fn apply_fall_flying(world: &mut WorldStore, entity_id: i32) -> bool {
    let accessors = crate::entities::vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.shared_flags_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: ProtocolEntityDataValueKind::Byte(accessors.fall_flying_mask),
        }],
    })
}
