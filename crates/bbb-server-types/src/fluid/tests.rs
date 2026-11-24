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

fn lava(level: i32) -> i32 {
    state_of("minecraft:lava", &[("level", &level.to_string())])
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
fn block_states_project_onto_the_fluid_states_liquid_block_caches() {
    assert_eq!(
        block_state_fluid_state(water(0)),
        FluidState::source(FluidFamily::Water, false)
    );
    assert_eq!(
        block_state_fluid_state(water(1)),
        FluidState::flowing(FluidFamily::Water, 7, false)
    );
    assert_eq!(
        block_state_fluid_state(water(7)),
        FluidState::flowing(FluidFamily::Water, 1, false)
    );
    assert_eq!(
        block_state_fluid_state(water(8)),
        FluidState::flowing(FluidFamily::Water, 8, true)
    );
    assert_eq!(
        block_state_fluid_state(water(15)),
        block_state_fluid_state(water(8)),
        "LiquidBlock.getFluidState indexes at min(level, 8) (LiquidBlock.java:125-127)"
    );
    assert_eq!(
        block_state_fluid_state(lava(0)),
        FluidState::source(FluidFamily::Lava, false)
    );
    assert_eq!(block_state_fluid_state(stone()), FluidState::Empty);
    assert_eq!(
        block_state_fluid_state(oak_slab("bottom", true)),
        FluidState::source(FluidFamily::Water, false),
        "the waterlogged pattern yields a water source"
    );
    assert_eq!(
        block_state_fluid_state(oak_slab("bottom", false)),
        FluidState::Empty
    );
    assert_eq!(
        block_state_fluid_state(state_of("minecraft:kelp_plant", &[])),
        FluidState::source(FluidFamily::Water, false),
        "KelpPlantBlock.getFluidState is an unconditional water source"
    );

    // The round trip through the block form.
    for level in 0..=8 {
        let projected = block_state_fluid_state(water(level));
        assert_eq!(projected.create_legacy_block(), water(level));
    }
    assert_eq!(FluidState::Empty.create_legacy_block(), air());
}
