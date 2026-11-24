use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::relative;

use super::*;

mod native_world;
use native_world::NativeWorld;

/// The base 26.1 tags a test without a level reads blocks against.
pub(crate) fn vanilla_tags() -> &'static RegistryTagSnapshot {
    RegistryTagSnapshot::builtin()
}

pub mod amethyst_growth;
pub mod barriers;
pub mod beds;
pub mod beehives;
pub mod bells;
pub mod big_dripleaf;
pub mod brushable;
pub mod bubble_columns;
pub mod cakes;
pub mod candle_survival;
pub mod carpet_survival;
pub mod carved_pumpkin;
pub mod cauldrons;
pub mod chains;
pub mod chest_combine;
pub mod chiseled_bookshelves;
pub mod chorus;
pub mod comparators;
pub mod composters;
pub mod concrete_powders;
pub mod conduits;
pub mod container_openers;
pub mod containers;
pub mod copper_bulbs;
pub mod copper_chests;
pub mod copper_golem_statues;
pub mod copper_grates;
pub mod coral;
pub mod creaking_heart;
pub mod crops;
pub mod daylight_detectors;
pub mod decorated_pots;
pub mod dispatch;
pub mod dispensers;
pub mod door_halves;
pub mod double_plants;
pub mod dried_ghast;
pub mod dripstone;
pub mod dynamic_shapes;
pub mod eyeblossom;
pub mod falling_blocks;
pub mod farmland_moisture;
pub mod fence_gates;
pub mod fences;
pub mod fire;
pub mod flower_pot;
pub mod footing;
pub mod growing_plants;
pub mod growth;
pub mod heavy_cores;
pub mod hoppers;
pub mod huge_mushrooms;
pub mod interaction_sources;
pub mod iron_bars;
pub mod leaf_decay;
pub mod lecterns;
pub mod lightable;
pub mod lightning_rods;
pub mod live_registry_tags;
pub mod mangrove_roots;
pub mod melting_light;
pub mod multiface;
pub mod mushroom;
pub mod note_blocks;
pub mod nylium;
pub mod observers;
pub mod oxidation;
pub mod pistons;
pub mod pitcher_crops;
pub mod placement;
pub mod rails;
pub mod redstone_lamps;
pub mod redstone_ore;
pub mod redstone_torches;
pub mod redstone_wire;
pub mod repeaters;
pub mod respawn_anchors;
pub mod saplings;
pub mod scaffolding;
pub mod scheduled_occupancy;
pub mod sculk_catalysts;
pub mod sculk_sensors;
pub mod seagrass;
pub mod set_block_legs;
pub mod shelves;
pub mod shulker_boxes;
pub mod signs_and_banners;
pub mod skulls;
pub mod slabs;
pub mod sniffer_eggs;
pub mod sponges;
pub mod spreading;
pub mod stairs;
pub mod survival_light;
pub mod sweet_berry_bushes;
pub mod targets;
pub mod torches_lanterns_ladders;
pub mod trapdoors;
pub mod trapped_chests;
pub mod tripwires;
pub mod turtle_eggs;
pub mod vegetation_survival;
pub mod vines;
pub mod walls;
pub mod water_into_containers;

// ---------------------------------------------------------------------------
// Block-state helpers
// ---------------------------------------------------------------------------

fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties: BTreeMap<String, String> = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("no built-in state for {name} {properties:?}"))
}

/// `Block.defaultBlockState()` by resource id, for a block whose default is
/// not the property-free state.
fn default_state(name: &str) -> i32 {
    let entry = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"));
    vanilla_block_default_state_id(entry.id)
        .unwrap_or_else(|| panic!("{name} has no default state"))
}

fn air() -> i32 {
    state_of("minecraft:air", &[])
}

fn stone() -> i32 {
    state_of("minecraft:stone", &[])
}

fn dirt() -> i32 {
    state_of("minecraft:dirt", &[])
}

fn oak_log() -> i32 {
    state_of("minecraft:oak_log", &[("axis", "y")])
}

fn oak_leaves(distance: i32, persistent: bool, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:oak_leaves",
        &[
            ("distance", &distance.to_string()),
            ("persistent", if persistent { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn grass_block(snowy: bool) -> i32 {
    state_of(
        "minecraft:grass_block",
        &[("snowy", if snowy { "true" } else { "false" })],
    )
}

fn podzol(snowy: bool) -> i32 {
    state_of(
        "minecraft:podzol",
        &[("snowy", if snowy { "true" } else { "false" })],
    )
}

/// `minecraft:scaffolding`, whose `distance` is `STABILITY_DISTANCE`
/// (`ScaffoldingBlock.java:38`) and therefore starts at 0, one below the range
/// `LeavesBlock.DISTANCE` allows (`BlockStateProperties.java:103`, `:123`).
fn scaffolding(distance: i32) -> i32 {
    state_of(
        "minecraft:scaffolding",
        &[
            ("bottom", "false"),
            ("distance", &distance.to_string()),
            ("waterlogged", "false"),
        ],
    )
}

fn mycelium(snowy: bool) -> i32 {
    state_of(
        "minecraft:mycelium",
        &[("snowy", if snowy { "true" } else { "false" })],
    )
}

fn farmland_state(moisture: i32) -> i32 {
    state_of("minecraft:farmland", &[("moisture", &moisture.to_string())])
}

fn wheat(age: i32) -> i32 {
    state_of("minecraft:wheat", &[("age", &age.to_string())])
}

fn beetroots(age: i32) -> i32 {
    state_of("minecraft:beetroots", &[("age", &age.to_string())])
}

fn torchflower_crop(age: i32) -> i32 {
    state_of("minecraft:torchflower_crop", &[("age", &age.to_string())])
}

fn snow_layer(layers: i32) -> i32 {
    state_of("minecraft:snow", &[("layers", &layers.to_string())])
}

fn ice() -> i32 {
    state_of("minecraft:ice", &[])
}

fn water(level: i32) -> i32 {
    state_of("minecraft:water", &[("level", &level.to_string())])
}

/// The entity id every fixture click and dig acts as.
const INTERACTING_PLAYER_ENTITY_ID: i32 = 42;

const fn interaction_player(may_build: bool, direction: Direction) -> BlockInteractionPlayer {
    BlockInteractionPlayer {
        entity_id: INTERACTING_PLAYER_ENTITY_ID,
        may_build,
        horizontal_direction: direction,
    }
}

const fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}
