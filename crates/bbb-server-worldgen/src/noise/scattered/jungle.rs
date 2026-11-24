//! Jungle-temple `postProcess`.

use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::{update_average_ground_height, GeneratedPieceFrame};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_set_property, catalog_default_state, WorldGenLevel};

use super::{ScatteredFeatureChunkPlacement, ScatteredFeatureExtra, ScatteredFeaturePiece};

const TEMPLE_LOOT: &str = "minecraft:chests/jungle_temple";
const DISPENSER_LOOT: &str = "minecraft:chests/jungle_temple_dispenser";

pub fn place_jungle_temple<L: WorldGenLevel>(
    piece: &mut ScatteredFeaturePiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> ScatteredFeatureChunkPlacement {
    let mut bounding_box = piece.bounding_box();
    let mut height_position = piece.height_position();
    if !update_average_ground_height(
        level,
        chunk_bounds,
        &mut bounding_box,
        &mut height_position,
        0,
    ) {
        return ScatteredFeatureChunkPlacement {
            attempted: true,
            placed: false,
        };
    }
    *piece.bounding_box_mut() = bounding_box;
    *piece.height_position_mut() = height_position;
    let frame = GeneratedPieceFrame::from_array(piece.bounding_box(), piece.orientation());
    let width = piece.width();
    let depth = piece.depth();
    let air = state("minecraft:air");
    let mossy = state("minecraft:mossy_cobblestone");
    let chiseled = state("minecraft:chiseled_stone_bricks");
    let vine_south = block_set_property(state("minecraft:vine"), "south", "true");
    let vine_east = block_set_property(state("minecraft:vine"), "east", "true");
    selected(
        frame,
        level,
        chunk_bounds,
        random,
        0,
        -4,
        0,
        width - 1,
        0,
        depth - 1,
    );
    selected(frame, level, chunk_bounds, random, 2, 1, 2, 9, 2, 2);
    selected(frame, level, chunk_bounds, random, 2, 1, 12, 9, 2, 12);
    selected(frame, level, chunk_bounds, random, 2, 1, 3, 2, 2, 11);
    selected(frame, level, chunk_bounds, random, 9, 1, 3, 9, 2, 11);
    selected(frame, level, chunk_bounds, random, 1, 3, 1, 10, 6, 1);
    selected(frame, level, chunk_bounds, random, 1, 3, 13, 10, 6, 13);
    selected(frame, level, chunk_bounds, random, 1, 3, 2, 1, 6, 12);
    selected(frame, level, chunk_bounds, random, 10, 3, 2, 10, 6, 12);
    selected(frame, level, chunk_bounds, random, 2, 3, 2, 9, 3, 12);
    selected(frame, level, chunk_bounds, random, 2, 6, 2, 9, 6, 12);
    selected(frame, level, chunk_bounds, random, 3, 7, 3, 8, 7, 11);
    selected(frame, level, chunk_bounds, random, 4, 8, 4, 7, 8, 10);
    air_box(frame, level, chunk_bounds, air, 3, 1, 3, 8, 2, 11);
    air_box(frame, level, chunk_bounds, air, 4, 3, 6, 7, 3, 9);
    air_box(frame, level, chunk_bounds, air, 2, 4, 2, 9, 5, 12);
    air_box(frame, level, chunk_bounds, air, 4, 6, 5, 7, 6, 9);
    air_box(frame, level, chunk_bounds, air, 5, 7, 6, 6, 7, 8);
    air_box(frame, level, chunk_bounds, air, 5, 1, 2, 6, 2, 2);
    air_box(frame, level, chunk_bounds, air, 5, 2, 12, 6, 2, 12);
    air_box(frame, level, chunk_bounds, air, 5, 5, 1, 6, 5, 1);
    air_box(frame, level, chunk_bounds, air, 5, 5, 13, 6, 5, 13);
    place(frame, level, chunk_bounds, air, 1, 5, 5);
    place(frame, level, chunk_bounds, air, 10, 5, 5);
    place(frame, level, chunk_bounds, air, 1, 5, 9);
    place(frame, level, chunk_bounds, air, 10, 5, 9);
    let mut z = 0;
    while z <= 14 {
        selected(frame, level, chunk_bounds, random, 2, 4, z, 2, 5, z);
        selected(frame, level, chunk_bounds, random, 4, 4, z, 4, 5, z);
        selected(frame, level, chunk_bounds, random, 7, 4, z, 7, 5, z);
        selected(frame, level, chunk_bounds, random, 9, 4, z, 9, 5, z);
        z += 14;
    }
    selected(frame, level, chunk_bounds, random, 5, 6, 0, 6, 6, 0);
    let mut x = 0;
    while x <= 11 {
        let mut z = 2;
        while z <= 12 {
            selected(frame, level, chunk_bounds, random, x, 4, z, x, 5, z);
            z += 2;
        }
        selected(frame, level, chunk_bounds, random, x, 6, 5, x, 6, 5);
        selected(frame, level, chunk_bounds, random, x, 6, 9, x, 6, 9);
        x += 11;
    }
    selected(frame, level, chunk_bounds, random, 2, 7, 2, 2, 9, 2);
    selected(frame, level, chunk_bounds, random, 9, 7, 2, 9, 9, 2);
    selected(frame, level, chunk_bounds, random, 2, 7, 12, 2, 9, 12);
    selected(frame, level, chunk_bounds, random, 9, 7, 12, 9, 9, 12);
    selected(frame, level, chunk_bounds, random, 4, 9, 4, 4, 9, 4);
    selected(frame, level, chunk_bounds, random, 7, 9, 4, 7, 9, 4);
    selected(frame, level, chunk_bounds, random, 4, 9, 10, 4, 9, 10);
    selected(frame, level, chunk_bounds, random, 7, 9, 10, 7, 9, 10);
    selected(frame, level, chunk_bounds, random, 5, 9, 7, 6, 9, 7);
    let east = cobble_stair("east");
    let west = cobble_stair("west");
    let south = cobble_stair("south");
    let north = cobble_stair("north");
    place(frame, level, chunk_bounds, north, 5, 9, 6);
    place(frame, level, chunk_bounds, north, 6, 9, 6);
    place(frame, level, chunk_bounds, south, 5, 9, 8);
    place(frame, level, chunk_bounds, south, 6, 9, 8);
    place(frame, level, chunk_bounds, north, 4, 0, 0);
    place(frame, level, chunk_bounds, north, 5, 0, 0);
    place(frame, level, chunk_bounds, north, 6, 0, 0);
    place(frame, level, chunk_bounds, north, 7, 0, 0);
    place(frame, level, chunk_bounds, north, 4, 1, 8);
    place(frame, level, chunk_bounds, north, 4, 2, 9);
    place(frame, level, chunk_bounds, north, 4, 3, 10);
    place(frame, level, chunk_bounds, north, 7, 1, 8);
    place(frame, level, chunk_bounds, north, 7, 2, 9);
    place(frame, level, chunk_bounds, north, 7, 3, 10);
    selected(frame, level, chunk_bounds, random, 4, 1, 9, 4, 1, 9);
    selected(frame, level, chunk_bounds, random, 7, 1, 9, 7, 1, 9);
    selected(frame, level, chunk_bounds, random, 4, 1, 10, 7, 2, 10);
    selected(frame, level, chunk_bounds, random, 5, 4, 5, 6, 4, 5);
    place(frame, level, chunk_bounds, east, 4, 4, 5);
    place(frame, level, chunk_bounds, west, 7, 4, 5);
    for i in 0..4 {
        place(frame, level, chunk_bounds, south, 5, -i, 6 + i);
        place(frame, level, chunk_bounds, south, 6, -i, 6 + i);
        air_box(frame, level, chunk_bounds, air, 5, -i, 7 + i, 6, -i, 9 + i);
    }
    air_box(frame, level, chunk_bounds, air, 1, -3, 12, 10, -1, 13);
    air_box(frame, level, chunk_bounds, air, 1, -3, 1, 3, -1, 13);
    air_box(frame, level, chunk_bounds, air, 1, -3, 1, 9, -1, 5);
    let mut z = 1;
    while z <= 13 {
        selected(frame, level, chunk_bounds, random, 1, -3, z, 1, -2, z);
        z += 2;
    }
    let mut z = 2;
    while z <= 12 {
        selected(frame, level, chunk_bounds, random, 1, -1, z, 3, -1, z);
        z += 2;
    }
    selected(frame, level, chunk_bounds, random, 2, -2, 1, 5, -2, 1);
    selected(frame, level, chunk_bounds, random, 7, -2, 1, 9, -2, 1);
    selected(frame, level, chunk_bounds, random, 6, -3, 1, 6, -3, 1);
    selected(frame, level, chunk_bounds, random, 6, -1, 1, 6, -1, 1);
    place(frame, level, chunk_bounds, hook("east"), 1, -3, 8);
    place(frame, level, chunk_bounds, hook("west"), 4, -3, 8);
    place(
        frame,
        level,
        chunk_bounds,
        tripwire("east", "west"),
        2,
        -3,
        8,
    );
    place(
        frame,
        level,
        chunk_bounds,
        tripwire("east", "west"),
        3,
        -3,
        8,
    );
    let redstone_ns = redstone(&[("north", "side"), ("south", "side")]);
    place(frame, level, chunk_bounds, redstone_ns, 5, -3, 7);
    place(frame, level, chunk_bounds, redstone_ns, 5, -3, 6);
    place(frame, level, chunk_bounds, redstone_ns, 5, -3, 5);
    place(frame, level, chunk_bounds, redstone_ns, 5, -3, 4);
    place(frame, level, chunk_bounds, redstone_ns, 5, -3, 3);
    place(frame, level, chunk_bounds, redstone_ns, 5, -3, 2);
    place(
        frame,
        level,
        chunk_bounds,
        redstone(&[("north", "side"), ("west", "side")]),
        5,
        -3,
        1,
    );
    place(
        frame,
        level,
        chunk_bounds,
        redstone(&[("east", "side"), ("west", "side")]),
        4,
        -3,
        1,
    );
    place(frame, level, chunk_bounds, mossy, 3, -3, 1);
    let ScatteredFeatureExtra::JungleTemple { placed_trap1, .. } = piece.extra_mut() else {
        unreachable!("piece extra is not a jungle temple")
    };
    if !*placed_trap1 {
        *placed_trap1 = frame.create_dispenser(
            level,
            chunk_bounds,
            random,
            3,
            -2,
            1,
            "north",
            DISPENSER_LOOT,
        );
    }
    place(frame, level, chunk_bounds, vine_south, 3, -2, 2);
    place(frame, level, chunk_bounds, hook("north"), 7, -3, 1);
    place(frame, level, chunk_bounds, hook("south"), 7, -3, 5);
    place(
        frame,
        level,
        chunk_bounds,
        tripwire("north", "south"),
        7,
        -3,
        2,
    );
    place(
        frame,
        level,
        chunk_bounds,
        tripwire("north", "south"),
        7,
        -3,
        3,
    );
    place(
        frame,
        level,
        chunk_bounds,
        tripwire("north", "south"),
        7,
        -3,
        4,
    );
    place(
        frame,
        level,
        chunk_bounds,
        redstone(&[("east", "side"), ("west", "side")]),
        8,
        -3,
        6,
    );
    place(
        frame,
        level,
        chunk_bounds,
        redstone(&[("west", "side"), ("south", "side")]),
        9,
        -3,
        6,
    );
    place(
        frame,
        level,
        chunk_bounds,
        redstone(&[("north", "side"), ("south", "up")]),
        9,
        -3,
        5,
    );
    place(frame, level, chunk_bounds, mossy, 9, -3, 4);
    place(frame, level, chunk_bounds, redstone_ns, 9, -2, 4);
    let ScatteredFeatureExtra::JungleTemple {
        placed_main_chest,
        placed_trap2,
        ..
    } = piece.extra_mut()
    else {
        unreachable!("piece extra is not a jungle temple")
    };
    if !*placed_trap2 {
        *placed_trap2 = frame.create_dispenser(
            level,
            chunk_bounds,
            random,
            9,
            -2,
            3,
            "west",
            DISPENSER_LOOT,
        );
    }
    place(frame, level, chunk_bounds, vine_east, 8, -1, 3);
    place(frame, level, chunk_bounds, vine_east, 8, -2, 3);
    if !*placed_main_chest {
        *placed_main_chest = frame.create_chest(level, chunk_bounds, random, 8, -3, 3, TEMPLE_LOOT);
    }
    place(frame, level, chunk_bounds, mossy, 9, -3, 2);
    place(frame, level, chunk_bounds, mossy, 8, -3, 1);
    place(frame, level, chunk_bounds, mossy, 4, -3, 5);
    place(frame, level, chunk_bounds, mossy, 5, -2, 5);
    place(frame, level, chunk_bounds, mossy, 5, -1, 5);
    place(frame, level, chunk_bounds, mossy, 6, -3, 5);
    place(frame, level, chunk_bounds, mossy, 7, -2, 5);
    place(frame, level, chunk_bounds, mossy, 7, -1, 5);
    place(frame, level, chunk_bounds, mossy, 8, -3, 5);
    selected(frame, level, chunk_bounds, random, 9, -1, 1, 9, -1, 5);
    air_box(frame, level, chunk_bounds, air, 8, -3, 8, 10, -1, 10);
    place(frame, level, chunk_bounds, chiseled, 8, -2, 11);
    place(frame, level, chunk_bounds, chiseled, 9, -2, 11);
    place(frame, level, chunk_bounds, chiseled, 10, -2, 11);
    let lever = lever_wall("north");
    place(frame, level, chunk_bounds, lever, 8, -2, 12);
    place(frame, level, chunk_bounds, lever, 9, -2, 12);
    place(frame, level, chunk_bounds, lever, 10, -2, 12);
    selected(frame, level, chunk_bounds, random, 8, -3, 8, 8, -3, 10);
    selected(frame, level, chunk_bounds, random, 10, -3, 8, 10, -3, 10);
    place(frame, level, chunk_bounds, mossy, 10, -2, 9);
    place(frame, level, chunk_bounds, redstone_ns, 8, -2, 9);
    place(frame, level, chunk_bounds, redstone_ns, 8, -2, 10);
    place(
        frame,
        level,
        chunk_bounds,
        redstone(&[
            ("north", "side"),
            ("south", "side"),
            ("east", "side"),
            ("west", "side"),
        ]),
        10,
        -1,
        9,
    );
    let sticky_up = block_set_property(state("minecraft:sticky_piston"), "facing", "up");
    let sticky_west = block_set_property(state("minecraft:sticky_piston"), "facing", "west");
    place(frame, level, chunk_bounds, sticky_up, 9, -2, 8);
    place(frame, level, chunk_bounds, sticky_west, 10, -2, 8);
    place(frame, level, chunk_bounds, sticky_west, 10, -1, 8);
    place(
        frame,
        level,
        chunk_bounds,
        block_set_property(state("minecraft:repeater"), "facing", "north"),
        10,
        -2,
        10,
    );
    let ScatteredFeatureExtra::JungleTemple {
        placed_hidden_chest,
        ..
    } = piece.extra_mut()
    else {
        unreachable!("piece extra is not a jungle temple")
    };
    if !*placed_hidden_chest {
        *placed_hidden_chest =
            frame.create_chest(level, chunk_bounds, random, 9, -3, 10, TEMPLE_LOOT);
    }
    ScatteredFeatureChunkPlacement {
        attempted: true,
        placed: true,
    }
}

#[allow(clippy::too_many_arguments)]
fn selected<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
) {
    let cobble = state("minecraft:cobblestone");
    let mossy = state("minecraft:mossy_cobblestone");
    frame.generate_box_selected(
        level,
        chunk_bounds,
        x0,
        y0,
        z0,
        x1,
        y1,
        z1,
        false,
        random,
        |random, _edge| {
            if random.next_float() < 0.4 {
                cobble
            } else {
                mossy
            }
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn air_box<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    air: i32,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
) {
    frame.generate_box(level, chunk_bounds, x0, y0, z0, x1, y1, z1, air, air, false)
}

fn place<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    state: i32,
    x: i32,
    y: i32,
    z: i32,
) {
    frame.place_block(level, chunk_bounds, state, x, y, z)
}

fn hook(facing: &'static str) -> i32 {
    let hook = block_set_property(state("minecraft:tripwire_hook"), "facing", facing);
    block_set_property(hook, "attached", "true")
}

fn tripwire(a: &'static str, b: &'static str) -> i32 {
    let wire = block_set_property(state("minecraft:tripwire"), a, "true");
    let wire = block_set_property(wire, b, "true");
    block_set_property(wire, "attached", "true")
}

fn redstone(sides: &[(&'static str, &'static str)]) -> i32 {
    let mut state = state("minecraft:redstone_wire");
    for (name, value) in sides {
        state = block_set_property(state, name, value);
    }
    state
}

fn lever_wall(facing: &'static str) -> i32 {
    let lever = block_set_property(state("minecraft:lever"), "facing", facing);
    block_set_property(lever, "face", "wall")
}

fn cobble_stair(facing: &'static str) -> i32 {
    block_set_property(state("minecraft:cobblestone_stairs"), "facing", facing)
}

fn state(id: &'static str) -> i32 {
    catalog_default_state(id)
}
