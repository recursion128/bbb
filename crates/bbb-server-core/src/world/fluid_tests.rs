use super::ServerWorld;
use crate::fluid::*;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily, FluidState};
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

fn obsidian() -> i32 {
    state_of("minecraft:obsidian", &[])
}

fn cobblestone() -> i32 {
    state_of("minecraft:cobblestone", &[])
}

fn dandelion() -> i32 {
    state_of("minecraft:dandelion", &[])
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

const fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

const SPREAD_FLOOR_RADIUS: i32 = 13;
const DIM: bbb_server_types::save::Java26BuiltinDimension =
    bbb_server_types::save::Java26BuiltinDimension::Overworld;

struct World {
    world: ServerWorld,
}

impl World {
    fn new() -> Self {
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        let chunks: Vec<_> = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |z| bbb_protocol::packets::ChunkPos { x, z }))
            .collect();
        world.set_chunks_forced(DIM.into(), true, &chunks);
        world.refresh_loading_chunks(&DIM.into()).unwrap();
        Self { world }
    }

    fn actual(pos: BlockPos) -> BlockPos {
        BlockPos {
            y: pos.y + 80,
            ..pos
        }
    }

    fn set(&mut self, pos: BlockPos, state: i32) {
        let pos = Self::actual(pos);
        self.world
            .level_mut_by_id(&DIM.into())
            .chunks
            .set_live_block_state(pos.x, pos.y, pos.z, state)
            .unwrap();
    }

    fn place(&mut self, pos: BlockPos, state: i32) {
        assert!(self
            .world
            .set_block(DIM, Self::actual(pos), state, SET_BLOCK_UPDATE_ALL));
    }

    fn floor(&mut self, y: i32, radius: i32, state: i32) {
        for x in -radius..=radius {
            for z in -radius..=radius {
                self.set(at(x, y, z), state);
            }
        }
    }

    fn spread_floor(&mut self) {
        self.floor(0, SPREAD_FLOOR_RADIUS, stone());
    }

    fn state(&self, pos: BlockPos) -> i32 {
        let pos = Self::actual(pos);
        let chunks = &self.world.level_by_id(&DIM.into()).chunks;
        chunks.get_block_state(pos.x, pos.y, pos.z).unwrap()
    }

    fn fluid(&self, pos: BlockPos) -> FluidState {
        block_state_fluid_state(self.state(pos))
    }

    fn run_ticks(&mut self, count: usize) {
        for _ in 0..count {
            self.world.tick(Vec::new());
        }
    }

    fn items(&self) -> Vec<(String, i32)> {
        self.world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter_map(|id| {
                let entity = self.world.stored_entity_for_test(DIM, id)?;
                let crate::entity::EntityData::Item(item) = entity.data() else {
                    return None;
                };
                Some((
                    item.item().item_resource_id()?.to_owned(),
                    item.item().count(),
                ))
            })
            .collect()
    }
}

#[test]
fn a_source_over_air_creates_a_full_falling_column_below_it() {
    let mut world = World::new();
    world.floor(0, 2, stone());
    world.place(at(0, 4, 0), water(0));
    world.run_ticks(40);

    for y in 1..=3 {
        assert_eq!(
            world.fluid(at(0, y, 0)),
            FluidState::Flowing {
                family: FluidFamily::Water,
                amount: 8,
                falling: true,
            },
            "the column at y={y} must be a full falling state"
        );
    }
    // `getLegacyLevel` puts a full falling state on LEVEL 8
    // (`FlowingFluid.java:462-464`), which is what the block form carries.
    assert_eq!(world.state(at(0, 3, 0)), water(8));
    assert!(world.fluid(at(0, 4, 0)).is_source());
}

#[test]
fn a_source_over_a_hole_flows_down_through_the_floor() {
    let mut world = World::new();
    world.floor(0, 3, stone());
    world.set(at(0, 0, 0), air());
    world.place(at(0, 1, 0), water(0));
    world.run_ticks(10);

    assert_eq!(
        world.fluid(at(0, 0, 0)),
        FluidState::flowing(FluidFamily::Water, 8, true)
    );
    assert!(world.fluid(at(0, 1, 0)).is_source());
    for pos in [at(1, 0, 0), at(-1, 0, 0), at(0, 0, 1), at(0, 0, -1)] {
        assert_eq!(world.state(pos), stone());
    }
}

#[test]
fn water_on_a_flat_floor_drops_one_level_per_block() {
    let mut world = World::new();
    world.spread_floor();
    world.place(at(0, 1, 0), water(0));
    world.run_ticks(200);

    for distance in 1..=7 {
        assert_eq!(
            world.fluid(at(distance, 1, 0)).amount(),
            8 - distance,
            "water drop-off is 1 (WaterFluid.java:107-109)"
        );
        assert!(!world.fluid(at(distance, 1, 0)).falling());
    }
    assert_eq!(world.fluid(at(8, 1, 0)), FluidState::Empty);
}

#[test]
fn lava_outside_a_fast_lava_dimension_drops_two_levels_per_block() {
    let mut world = World::new();
    world.spread_floor();
    world.place(at(0, 1, 0), lava(0));
    world.run_ticks(400);

    for (distance, expected) in [(1, 6), (2, 4), (3, 2)] {
        assert_eq!(
            world.fluid(at(distance, 1, 0)).amount(),
            expected,
            "lava drop-off is 2 without FAST_LAVA (LavaFluid.java:165-166)"
        );
    }
    assert_eq!(world.fluid(at(4, 1, 0)), FluidState::Empty);
}

#[test]
fn slope_distance_steers_the_first_spread_towards_the_nearest_hole() {
    let mut world = World::new();
    world.spread_floor();
    // A single hole two blocks east of the source.
    world.set(at(2, 0, 0), air());
    world.place(at(0, 1, 0), water(0));
    world.run_ticks(5);

    assert_eq!(
        world.fluid(at(1, 1, 0)).amount(),
        7,
        "the east neighbour is one step from the hole"
    );
    for pos in [at(-1, 1, 0), at(0, 1, 1), at(0, 1, -1)] {
        assert_eq!(
            world.fluid(pos),
            FluidState::Empty,
            "water follows the nearest hole"
        );
    }
}

#[test]
fn a_solid_wall_is_never_crossed() {
    let mut world = World::new();
    world.spread_floor();
    for z in -SPREAD_FLOOR_RADIUS..=SPREAD_FLOOR_RADIUS {
        world.set(at(1, 1, z), stone());
    }
    world.place(at(0, 1, 0), water(0));
    world.run_ticks(300);

    for z in -7..=7 {
        assert_eq!(
            world.state(at(1, 1, z)),
            stone(),
            "canHoldAnyFluid refuses a block that blocks motion"
        );
        assert_eq!(
            world.fluid(at(2, 1, z)),
            FluidState::Empty,
            "and canPassThroughWall refuses the face behind it"
        );
    }
    for pos in [at(-1, 1, 0), at(0, 1, 1), at(0, 1, -1)] {
        assert_eq!(world.fluid(pos).amount(), 7, "the open sides still fill");
    }
}

#[test]
fn a_single_solid_block_is_flowed_around_and_not_through() {
    let mut world = World::new();
    world.spread_floor();
    world.set(at(1, 1, 0), stone());
    world.place(at(0, 1, 0), water(0));

    world.run_ticks(5);
    assert_eq!(
        world.fluid(at(2, 1, 0)),
        FluidState::Empty,
        "the direct step east is refused"
    );

    world.run_ticks(300);
    assert_eq!(
        world.state(at(1, 1, 0)),
        stone(),
        "the block is never replaced"
    );
    assert_eq!(
        world.fluid(at(2, 1, 0)).amount(),
        4,
        "but the four-step detour (0,1)->(1,1)->(2,1)->(2,0) still gets there"
    );
}

// ---------------------------------------------------------------------------
// Water meeting lava
// ---------------------------------------------------------------------------

#[test]
fn lava_flowing_down_onto_water_turns_the_water_into_stone() {
    let mut world = World::new();
    world.floor(0, 2, stone());
    world.set(at(0, 1, 0), water(0));
    world.place(at(0, 2, 0), lava(0));
    world.run_ticks(31);

    assert_eq!(
        world.state(at(0, 1, 0)),
        stone(),
        "descending lava turns water into stone"
    );
    assert_eq!(
        world.state(at(0, 2, 0)),
        lava(0),
        "the lava source itself stays put"
    );
}

#[test]
fn lava_beside_water_turns_into_obsidian_when_it_is_a_source() {
    let mut world = World::new();
    world.floor(0, 2, stone());
    world.set(at(1, 1, 0), water(0));
    world.place(at(0, 1, 0), lava(0));

    assert_eq!(world.state(at(0, 1, 0)), obsidian());
}

#[test]
fn flowing_lava_beside_water_turns_into_cobblestone() {
    let mut world = World::new();
    world.floor(0, 2, stone());
    world.set(at(1, 1, 0), water(0));
    world.place(at(0, 1, 0), lava(2));

    assert_eq!(world.state(at(0, 1, 0)), cobblestone());
}

#[test]
fn lava_over_soul_soil_beside_blue_ice_turns_into_basalt() {
    let mut world = World::new();
    world.floor(0, 2, stone());
    world.set(at(0, 0, 0), state_of("minecraft:soul_soil", &[]));
    world.set(at(1, 1, 0), state_of("minecraft:blue_ice", &[]));
    world.place(at(0, 1, 0), lava(0));

    assert_eq!(
        world.state(at(0, 1, 0)),
        state_of("minecraft:basalt", &[("axis", "y")]),
        "soul soil and blue ice make basalt"
    );
}

#[test]
fn the_lava_water_scan_looks_up_and_sideways_but_never_down() {
    let mut world = World::new();
    world.floor(0, 2, stone());
    world.set(at(0, 0, 0), water(0));
    world.place(at(0, 1, 0), lava(0));
    assert_eq!(
        world.state(at(0, 1, 0)),
        lava(0),
        "water directly below never triggers the conversion"
    );

    let mut above_world = World::new();
    above_world.floor(0, 2, stone());
    above_world.set(at(0, 2, 0), water(0));
    above_world.place(at(0, 1, 0), lava(0));
    assert_eq!(
        above_world.state(at(0, 1, 0)),
        obsidian(),
        "water directly above does"
    );
}

// ---------------------------------------------------------------------------
// beforeDestroyingBlock
// ---------------------------------------------------------------------------

#[test]
fn water_drops_the_plant_it_destroys_and_lava_does_not() {
    let mut flooded = World::new();
    flooded.spread_floor();
    flooded.set(at(1, 0, 0), state_of("minecraft:dirt", &[]));
    flooded.set(at(1, 1, 0), dandelion());
    flooded.place(at(0, 1, 0), water(0));
    flooded.run_ticks(10);
    assert!(flooded
        .items()
        .iter()
        .any(|(name, count)| name == "minecraft:dandelion" && *count == 1));
    assert!(flooded
        .fluid(at(1, 1, 0))
        .is_same_family(FluidFamily::Water));

    let mut burned = World::new();
    burned.spread_floor();
    burned.set(at(1, 0, 0), state_of("minecraft:dirt", &[]));
    burned.set(at(1, 1, 0), dandelion());
    burned.place(at(0, 1, 0), lava(0));
    burned.run_ticks(40);
    assert!(burned.items().is_empty());
    assert!(burned.fluid(at(1, 1, 0)).is_same_family(FluidFamily::Lava));
}

// ---------------------------------------------------------------------------
// Multi-tick settling
// ---------------------------------------------------------------------------

#[test]
fn a_water_source_reaches_its_final_shape() {
    let mut world = World::new();
    world.spread_floor();
    world.place(at(0, 1, 0), water(0));
    world.run_ticks(300);

    // Every position at Manhattan distance d from the source holds 8 - d, and
    // the eighth ring is gone entirely.
    for (x, z, expected) in [
        (1, 0, 7),
        (0, 1, 7),
        (2, 0, 6),
        (1, 1, 6),
        (3, 0, 5),
        (2, 2, 4),
        (7, 0, 1),
        (4, 3, 1),
    ] {
        assert_eq!(
            world.fluid(at(x, 1, z)).amount(),
            expected,
            "({x},{z}) is {} blocks out",
            x + z
        );
    }
    for (x, z) in [(8, 0), (4, 4), (0, 8)] {
        assert_eq!(world.fluid(at(x, 1, z)), FluidState::Empty);
    }
}

// ---------------------------------------------------------------------------
// Multi-tick settling
// ---------------------------------------------------------------------------

#[test]
fn waterlogged_blocks_restart_water_flow_when_a_neighbor_is_removed() {
    for name in [
        "minecraft:ender_chest",
        "minecraft:oak_shelf",
        "minecraft:light",
        "minecraft:sculk_shrieker",
    ] {
        for waterlogged in ["true", "false"] {
            let mut world = World::new();
            world.floor(0, 2, stone());
            let mut properties = vec![("waterlogged", waterlogged)];
            if matches!(name, "minecraft:ender_chest" | "minecraft:oak_shelf") {
                properties.push(("facing", "north"));
            }
            match name {
                "minecraft:oak_shelf" => {
                    properties.extend([("powered", "false"), ("side_chain", "unconnected")]);
                }
                "minecraft:light" => properties.push(("level", "15")),
                "minecraft:sculk_shrieker" => {
                    properties.extend([("can_summon", "false"), ("shrieking", "false")]);
                }
                _ => {}
            }
            let state = state_of(name, &properties);
            world.set(at(0, 1, 0), state);
            for pos in [at(1, 1, 0), at(-1, 1, 0), at(0, 1, 1), at(0, 1, -1)] {
                world.set(pos, stone());
            }
            world.run_ticks(10);

            world.place(at(1, 1, 0), air());
            world.run_ticks(10);

            assert_eq!(world.state(at(0, 1, 0)), state, "{name}");
            assert_eq!(
                world.fluid(at(1, 1, 0)),
                if waterlogged == "true" {
                    FluidState::flowing(FluidFamily::Water, 7, false)
                } else {
                    FluidState::Empty
                },
                "{name} waterlogged={waterlogged}"
            );
        }
    }
}

#[test]
fn a_water_source_waterlogs_a_slab_it_meets_instead_of_destroying_it() {
    let mut world = World::new();
    world.spread_floor();
    world.set(at(0, 1, 0), oak_slab("bottom", false));
    world.place(at(-1, 1, 0), water(0));
    world.place(at(1, 1, 0), water(0));
    world.run_ticks(60);

    assert_eq!(
        world.state(at(0, 1, 0)),
        oak_slab("bottom", true),
        "the slab is filled, not destroyed"
    );
    assert_eq!(
        world.fluid(at(0, 1, 0)),
        FluidState::source(FluidFamily::Water, false)
    );
    assert!(world.items().is_empty());
}
