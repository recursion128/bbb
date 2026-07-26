use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_CACTUS_ID, VANILLA_BLOCK_END_STONE_ID,
    VANILLA_BLOCK_FROSTED_ICE_ID, VANILLA_BLOCK_HONEY_BLOCK_ID, VANILLA_BLOCK_ICE_ID,
    VANILLA_BLOCK_PODZOL_ID, VANILLA_BLOCK_STONE_ID,
};

fn survives<const N: usize>(
    world: &WorldStore,
    pos: BlockPos,
    block_name: &str,
    properties: [(&str, &str); N],
) -> Option<bool> {
    let state = state_id(block_name, properties);
    let owner = vanilla_block_type_for_resource_id(block_name)?
        .can_survive_override
        .expect("tested block overrides canSurvive");
    world.local_block_state_can_survive(pos, state, owner)
}

fn default_survives(world: &WorldStore, pos: BlockPos, block_name: &str) -> Option<bool> {
    let block_type = vanilla_block_type_for_resource_id(block_name)?;
    let state = vanilla_block_default_state_id(block_type.id)?;
    let owner = block_type
        .can_survive_override
        .expect("tested block overrides canSurvive");
    world.local_block_state_can_survive(pos, state, owner)
}

#[test]
fn vegetation_crop_and_mushroom_use_live_tags_light_and_solid_render() {
    let pos = TARGET;
    let below = relative(pos, ProtocolDirection::Down);
    let mut world = placement_world();
    set_block_tags(
        &mut world,
        &[
            ("minecraft:supports_vegetation", &[VANILLA_BLOCK_STONE_ID]),
            ("minecraft:supports_crops", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:overrides_mushroom_light_requirement",
                &[VANILLA_BLOCK_PODZOL_ID],
            ),
        ],
    );
    set_block(&mut world, below, state_id("minecraft:stone", []));
    set_raw_light(&mut world, pos, 0, 8);

    assert_eq!(
        survives(&world, pos, "minecraft:short_grass", []),
        Some(true)
    );
    assert_eq!(
        survives(&world, pos, "minecraft:wheat", [("age", "0")]),
        Some(true)
    );
    assert_eq!(
        survives(&world, pos, "minecraft:brown_mushroom", []),
        Some(true)
    );

    set_raw_light(&mut world, pos, 0, 13);
    assert_eq!(
        survives(&world, pos, "minecraft:brown_mushroom", []),
        Some(false)
    );
    set_raw_light(&mut world, pos, 0, 0);
    set_block(&mut world, below, state_id("minecraft:glass", []));
    assert_eq!(
        survives(&world, pos, "minecraft:brown_mushroom", []),
        Some(false),
        "a full collision block without a full occlusion shape is not solid-render"
    );

    set_block(
        &mut world,
        below,
        state_id("minecraft:podzol", [("snowy", "false")]),
    );
    set_raw_light(&mut world, pos, 15, 15);
    assert_eq!(
        survives(&world, pos, "minecraft:brown_mushroom", []),
        Some(true),
        "the override tag bypasses both light and solid-render requirements"
    );

    set_block(&mut world, below, state_id("minecraft:air", []));
    assert_eq!(
        survives(&world, pos, "minecraft:short_grass", []),
        Some(false)
    );
    assert_eq!(
        survives(&world, pos, "minecraft:wheat", [("age", "0")]),
        Some(false)
    );
}

#[test]
fn vegetation_subclasses_dispatch_their_overridden_may_place_on_rules() {
    let pos = TARGET;
    let below = relative(pos, ProtocolDirection::Down);
    let mut world = placement_world();
    set_block_tags(
        &mut world,
        &[
            ("minecraft:supports_vegetation", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:supports_dry_vegetation",
                &[VANILLA_BLOCK_STONE_ID],
            ),
            ("minecraft:supports_wither_rose", &[VANILLA_BLOCK_STONE_ID]),
            ("minecraft:supports_pumpkin_stem", &[VANILLA_BLOCK_STONE_ID]),
            ("minecraft:supports_melon_stem", &[VANILLA_BLOCK_STONE_ID]),
            ("minecraft:supports_nether_wart", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:supports_warped_fungus",
                &[VANILLA_BLOCK_STONE_ID],
            ),
            (
                "minecraft:supports_crimson_fungus",
                &[VANILLA_BLOCK_STONE_ID],
            ),
            ("minecraft:supports_warped_roots", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:supports_crimson_roots",
                &[VANILLA_BLOCK_STONE_ID],
            ),
            (
                "minecraft:supports_nether_sprouts",
                &[VANILLA_BLOCK_STONE_ID],
            ),
            ("minecraft:supports_azalea", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:support_override_cactus_flower",
                &[VANILLA_BLOCK_CACTUS_ID],
            ),
            (
                "minecraft:supports_lily_pad",
                &[VANILLA_BLOCK_FROSTED_ICE_ID],
            ),
        ],
    );
    let water = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:water")
        .unwrap();
    let flowing_water = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:flowing_water")
        .unwrap();
    set_fluid_tags(
        &mut world,
        &[("minecraft:supports_lily_pad", &[water, flowing_water])],
    );

    set_block(&mut world, below, state_id("minecraft:stone", []));
    for block_name in [
        "minecraft:dead_bush",
        "minecraft:wither_rose",
        "minecraft:pumpkin_stem",
        "minecraft:melon_stem",
        "minecraft:nether_wart",
        "minecraft:warped_fungus",
        "minecraft:crimson_fungus",
        "minecraft:warped_roots",
        "minecraft:crimson_roots",
        "minecraft:nether_sprouts",
        "minecraft:azalea",
    ] {
        assert_eq!(
            default_survives(&world, pos, block_name),
            Some(true),
            "{block_name} must use its subclass support tag"
        );
    }

    assert_eq!(
        survives(&world, pos, "minecraft:sunflower", [("half", "lower")]),
        Some(true)
    );
    set_block(
        &mut world,
        below,
        state_id("minecraft:sunflower", [("half", "lower")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:sunflower", [("half", "upper")]),
        Some(true)
    );
    set_block(
        &mut world,
        below,
        state_id("minecraft:lilac", [("half", "lower")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:sunflower", [("half", "upper")]),
        Some(false)
    );

    set_block(
        &mut world,
        below,
        state_id("minecraft:cactus", [("age", "0")]),
    );
    assert_eq!(
        default_survives(&world, pos, "minecraft:cactus_flower"),
        Some(true)
    );
    set_block(&mut world, below, state_id("minecraft:air", []));
    assert_eq!(
        default_survives(&world, pos, "minecraft:cactus_flower"),
        Some(false)
    );

    set_block(
        &mut world,
        below,
        state_id("minecraft:water", [("level", "1")]),
    );
    assert_eq!(
        default_survives(&world, pos, "minecraft:lily_pad"),
        Some(true)
    );
    set_block(
        &mut world,
        pos,
        state_id("minecraft:water", [("level", "0")]),
    );
    assert_eq!(
        default_survives(&world, pos, "minecraft:lily_pad"),
        Some(false)
    );
    set_block(&mut world, pos, state_id("minecraft:air", []));
    set_block(
        &mut world,
        below,
        state_id("minecraft:frosted_ice", [("age", "0")]),
    );
    assert_eq!(
        default_survives(&world, pos, "minecraft:lily_pad"),
        Some(true)
    );
}

#[test]
fn ceiling_chain_torch_and_grindstone_rules_match_vanilla() {
    let pos = TARGET;
    let below = relative(pos, ProtocolDirection::Down);
    let above = relative(pos, ProtocolDirection::Up);
    let mut world = placement_world();
    let water = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:water")
        .unwrap();
    let flowing_water = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:flowing_water")
        .unwrap();
    set_fluid_tags(&mut world, &[("minecraft:water", &[water, flowing_water])]);

    set_block(&mut world, above, state_id("minecraft:stone", []));
    assert_eq!(
        survives(&world, pos, "minecraft:spore_blossom", []),
        Some(true)
    );
    set_block(
        &mut world,
        pos,
        state_id("minecraft:water", [("level", "0")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:spore_blossom", []),
        Some(false)
    );
    set_block(&mut world, pos, state_id("minecraft:air", []));

    assert_eq!(
        survives(
            &world,
            pos,
            "minecraft:pale_hanging_moss",
            [("tip", "true")]
        ),
        Some(true)
    );
    set_block(
        &mut world,
        above,
        state_id("minecraft:pale_hanging_moss", [("tip", "true")]),
    );
    assert_eq!(
        survives(
            &world,
            pos,
            "minecraft:pale_hanging_moss",
            [("tip", "true")]
        ),
        Some(true)
    );
    set_block(&mut world, above, state_id("minecraft:air", []));
    assert_eq!(
        survives(
            &world,
            pos,
            "minecraft:pale_hanging_moss",
            [("tip", "true")]
        ),
        Some(false)
    );

    set_block(&mut world, below, state_id("minecraft:stone", []));
    assert_eq!(survives(&world, pos, "minecraft:torch", []), Some(true));
    set_block(&mut world, below, state_id("minecraft:air", []));
    assert_eq!(survives(&world, pos, "minecraft:torch", []), Some(false));
    assert_eq!(
        survives(
            &world,
            pos,
            "minecraft:grindstone",
            [("face", "wall"), ("facing", "north")]
        ),
        Some(true)
    );
}

#[test]
fn sugar_cane_cactus_and_chorus_flower_inspect_exact_neighbors() {
    let pos = BlockPos { x: 4, y: 9, z: 4 };
    let below = relative(pos, ProtocolDirection::Down);
    let below_east = relative(below, ProtocolDirection::East);
    let below_west = relative(below, ProtocolDirection::West);
    let east = relative(pos, ProtocolDirection::East);
    let west = relative(pos, ProtocolDirection::West);
    let above = relative(pos, ProtocolDirection::Up);
    let mut world = placement_world();
    let water = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:water")
        .unwrap();
    let flowing_water = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:flowing_water")
        .unwrap();
    let lava = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:lava")
        .unwrap();
    let flowing_lava = world
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:flowing_lava")
        .unwrap();
    set_block_tags(
        &mut world,
        &[
            ("minecraft:supports_sugar_cane", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:supports_sugar_cane_adjacently",
                &[VANILLA_BLOCK_FROSTED_ICE_ID],
            ),
            ("minecraft:supports_cactus", &[VANILLA_BLOCK_STONE_ID]),
            (
                "minecraft:supports_chorus_flower",
                &[VANILLA_BLOCK_END_STONE_ID],
            ),
        ],
    );
    set_fluid_tags(
        &mut world,
        &[
            ("minecraft:water", &[water, flowing_water]),
            (
                "minecraft:supports_sugar_cane_adjacently",
                &[water, flowing_water],
            ),
            ("minecraft:lava", &[lava, flowing_lava]),
        ],
    );

    set_block(&mut world, below, state_id("minecraft:stone", []));
    set_block(
        &mut world,
        below_east,
        state_id("minecraft:water", [("level", "1")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:sugar_cane", [("age", "0")]),
        Some(true)
    );
    set_block(&mut world, below_east, state_id("minecraft:air", []));
    set_block(
        &mut world,
        below_west,
        state_id("minecraft:frosted_ice", [("age", "0")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:sugar_cane", [("age", "0")]),
        Some(true)
    );
    set_block(&mut world, below_west, state_id("minecraft:air", []));
    assert_eq!(
        survives(&world, pos, "minecraft:sugar_cane", [("age", "0")]),
        Some(false)
    );

    assert_eq!(
        survives(&world, pos, "minecraft:cactus", [("age", "0")]),
        Some(true)
    );
    set_block(&mut world, east, state_id("minecraft:stone", []));
    assert_eq!(
        survives(&world, pos, "minecraft:cactus", [("age", "0")]),
        Some(false)
    );
    set_block(
        &mut world,
        east,
        state_id("minecraft:lava", [("level", "1")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:cactus", [("age", "0")]),
        Some(false)
    );
    set_block(&mut world, east, state_id("minecraft:air", []));
    set_block(
        &mut world,
        above,
        state_id("minecraft:water", [("level", "0")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:cactus", [("age", "0")]),
        Some(false)
    );
    set_block(&mut world, above, state_id("minecraft:air", []));

    set_block(&mut world, below, state_id("minecraft:end_stone", []));
    assert_eq!(
        survives(&world, pos, "minecraft:chorus_flower", [("age", "0")]),
        Some(true)
    );
    set_block(&mut world, below, state_id("minecraft:air", []));
    set_block(
        &mut world,
        east,
        state_id(
            "minecraft:chorus_plant",
            [
                ("down", "false"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        ),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:chorus_flower", [("age", "0")]),
        Some(true)
    );
    set_block(
        &mut world,
        west,
        state_id(
            "minecraft:chorus_plant",
            [
                ("down", "false"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        ),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:chorus_flower", [("age", "0")]),
        Some(false)
    );
}

#[test]
fn snow_carpet_cake_and_pressure_plate_use_their_distinct_support_rules() {
    let pos = TARGET;
    let below = relative(pos, ProtocolDirection::Down);
    let mut world = placement_world();
    set_block_tags(
        &mut world,
        &[
            (
                "minecraft:cannot_support_snow_layer",
                &[VANILLA_BLOCK_ICE_ID],
            ),
            (
                "minecraft:support_override_snow_layer",
                &[VANILLA_BLOCK_HONEY_BLOCK_ID],
            ),
        ],
    );

    set_block(&mut world, below, state_id("minecraft:ice", []));
    assert_eq!(
        survives(&world, pos, "minecraft:snow", [("layers", "1")]),
        Some(false)
    );
    set_block(&mut world, below, state_id("minecraft:honey_block", []));
    assert_eq!(
        survives(&world, pos, "minecraft:snow", [("layers", "1")]),
        Some(true)
    );
    set_block(&mut world, below, state_id("minecraft:stone", []));
    assert_eq!(
        survives(&world, pos, "minecraft:snow", [("layers", "1")]),
        Some(true)
    );
    set_block(
        &mut world,
        below,
        state_id("minecraft:snow", [("layers", "8")]),
    );
    assert_eq!(
        survives(&world, pos, "minecraft:snow", [("layers", "1")]),
        Some(true)
    );

    set_block(&mut world, below, state_id("minecraft:torch", []));
    assert_eq!(
        survives(&world, pos, "minecraft:white_carpet", []),
        Some(true)
    );
    set_block(&mut world, below, state_id("minecraft:air", []));
    assert_eq!(
        survives(&world, pos, "minecraft:white_carpet", []),
        Some(false)
    );

    set_block(&mut world, below, state_id("minecraft:glass", []));
    assert_eq!(
        survives(&world, pos, "minecraft:cake", [("bites", "0")]),
        Some(true)
    );
    set_block(&mut world, below, state_id("minecraft:air", []));
    assert_eq!(
        survives(&world, pos, "minecraft:cake", [("bites", "0")]),
        Some(false)
    );

    set_block(
        &mut world,
        below,
        state_id(
            "minecraft:oak_fence",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        ),
    );
    assert_eq!(
        survives(
            &world,
            pos,
            "minecraft:stone_pressure_plate",
            [("powered", "false")]
        ),
        Some(true),
        "center support is sufficient even without rigid support"
    );
    set_block(&mut world, below, state_id("minecraft:air", []));
    assert_eq!(
        survives(
            &world,
            pos,
            "minecraft:stone_pressure_plate",
            [("powered", "false")]
        ),
        Some(false)
    );
}
