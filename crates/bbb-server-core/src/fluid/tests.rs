use super::*;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use std::collections::BTreeMap;

fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties: BTreeMap<String, String> = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("no built-in state for {name} {properties:?}"))
}

fn air() -> i32 {
    state_of("minecraft:air", &[])
}

fn stone() -> i32 {
    state_of("minecraft:stone", &[])
}

fn water(level: i32) -> i32 {
    state_of("minecraft:water", &[("level", &level.to_string())])
}

fn oak_slab(slab_type: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:oak_slab",
        &[
            ("type", slab_type),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

#[test]
fn can_pass_through_wall_is_open_between_empty_shapes_and_sealed_by_a_full_face() {
    assert!(can_pass_through_wall(Direction::Down, air(), air()));
    assert!(
        !can_pass_through_wall(Direction::Down, water(0), stone()),
        "a full cube below blocks the flow"
    );
    assert!(
        !can_pass_through_wall(Direction::East, stone(), air()),
        "and so does a full cube at the source"
    );
    assert!(
        can_pass_through_wall(Direction::Down, air(), oak_slab("bottom", false)),
        "a bottom slab leaves its top face open"
    );
    assert!(
        !can_pass_through_wall(Direction::Down, air(), oak_slab("top", false)),
        "a top slab seals the face the fluid would come through"
    );
    assert!(
        can_pass_through_wall(Direction::Up, air(), oak_slab("top", false)),
        "the same top slab is open from below"
    );
}

#[test]
fn waterlogging_only_accepts_the_water_source_and_never_a_double_slab() {
    let bottom = oak_slab("bottom", false);
    assert!(can_hold_any_fluid(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        bottom
    ));
    assert!(can_hold_specific_fluid(bottom, VanillaFluidIdentity::Water));
    assert!(
        !can_hold_specific_fluid(bottom, VanillaFluidIdentity::FlowingWater),
        "SimpleWaterloggedBlock.canPlaceLiquid compares against Fluids.WATER, the source"
    );
    assert!(!can_hold_specific_fluid(bottom, VanillaFluidIdentity::Lava));

    let double = oak_slab("double", false);
    assert!(
        !can_hold_specific_fluid(double, VanillaFluidIdentity::Water),
        "a double slab cannot be waterlogged"
    );

    let kelp = state_of("minecraft:kelp_plant", &[]);
    assert!(
        !can_hold_specific_fluid(kelp, VanillaFluidIdentity::Water),
        "KelpPlantBlock.canPlaceLiquid is a flat false"
    );

    assert!(
        !can_hold_any_fluid(
            crate::registry_tags::RegistryTagSnapshot::builtin(),
            stone()
        ),
        "a block that blocks motion holds nothing"
    );
    assert!(
        !can_hold_any_fluid(
            crate::registry_tags::RegistryTagSnapshot::builtin(),
            state_of("minecraft:sugar_cane", &[("age", "0")])
        ),
        "sugar cane is one of the blocks canHoldAnyFluid names by identity"
    );
    assert!(
        !can_hold_any_fluid(
            crate::registry_tags::RegistryTagSnapshot::builtin(),
            state_of("minecraft:bubble_column", &[("drag", "true")])
        ),
        "so is the bubble column, which is not a LiquidBlockContainer"
    );
    assert!(
        can_hold_any_fluid(
            crate::registry_tags::RegistryTagSnapshot::builtin(),
            state_of(
                "minecraft:ladder",
                &[("facing", "north"), ("waterlogged", "false")]
            )
        ),
        "the ladder is waterloggable, so canHoldAnyFluid's first branch already \
         says yes and the explicit Blocks.LADDER test below it never fires"
    );
}

#[test]
fn tick_delay_drop_off_and_slope_distance_follow_the_family_and_fast_lava() {
    assert_eq!(FlowingFluid::WATER.tick_delay(false), 5);
    assert_eq!(FlowingFluid::WATER.drop_off(false), 1);
    assert_eq!(FlowingFluid::WATER.slope_find_distance(false), 4);
    assert_eq!(FlowingFluid::LAVA.tick_delay(false), 30);
    assert_eq!(FlowingFluid::LAVA.drop_off(false), 2);
    assert_eq!(FlowingFluid::LAVA.slope_find_distance(false), 2);

    assert_eq!(
        FlowingFluid::WATER.tick_delay(true),
        5,
        "water ignores FAST_LAVA (WaterFluid.java:112-114)"
    );
    assert_eq!(FlowingFluid::LAVA.tick_delay(true), 10);
    assert_eq!(FlowingFluid::LAVA.drop_off(true), 1);
    assert_eq!(FlowingFluid::LAVA.slope_find_distance(true), 4);
}
