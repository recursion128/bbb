//! `CandleBlock` (`CandleBlock.java`): the support question it asks, the water
//! tick its shape update re-arms, and the removal it deliberately does not do.

use super::*;

use crate::block_behavior::candle;

/// The cell the candle stands in.
const CANDLE: BlockPos = BlockPos {
    x: 11,
    y: 65,
    z: -3,
};

/// Whether a candle would stand on `below_name`.
fn survives_on(below_name: &str) -> Option<bool> {
    let below_state = default_state(below_name);
    candle::can_survive(CANDLE, &mut |at| {
        if at == below(CANDLE) {
            below_state
        } else {
            air()
        }
    })
}

/// `minecraft:candle` with the three properties `createBlockStateDefinition`
/// declares (`CandleBlock.java:145-147`).
fn candle_state(candles: i32, lit: bool, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:candle",
        &[
            ("candles", &candles.to_string()),
            ("lit", if lit { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

#[test]
fn a_candle_needs_a_centre_sturdy_face_under_it_not_a_full_one() {
    // `CandleBlock.canSurvive` (`CandleBlock.java:181-183`) is
    // `Block.canSupportCenter(level, pos.below(), Direction.UP)`
    // (`Block.java:343-348`), i.e. `SupportType.CENTER`
    // (`SupportType.java:18-25`) — the middle two pixels of the face, not the
    // whole of it.
    assert_eq!(survives_on("minecraft:stone"), Some(true));
    // A fence post fills the centre and not the rest, so it holds a candle
    // while refusing everything that wants `SupportType.FULL`.
    assert_eq!(survives_on("minecraft:oak_fence"), Some(true));
    assert_eq!(survives_on("minecraft:oak_fence_gate"), Some(true));
    // A cactus is 15/16 wide and leaves the centre column empty at the top.
    assert_eq!(survives_on("minecraft:cactus"), Some(false));
    // Farmland is one pixel short of the top.
    assert_eq!(survives_on("minecraft:farmland"), Some(false));
    assert_eq!(survives_on("minecraft:air"), Some(false));
    assert_eq!(survives_on("minecraft:water"), Some(false));
    assert_eq!(survives_on("minecraft:torch"), Some(false));
}

#[test]
fn breaking_the_block_under_a_candle_leaves_the_candle_standing() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), stone());
    world.set(at(0, 71, 0), candle_state(1, false, false));

    world.set(at(0, 70, 0), air());
    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), candle_state(1, false, false));
    assert!(world.items().is_empty());
}

// ---------------------------------------------------------------------------
// The mechanical guard
// ---------------------------------------------------------------------------

#[test]
fn a_candle_item_places_one_dry_candle_on_stone() {
    let pos = BlockPos { x: 4, y: 70, z: -9 };
    {
        let name = "minecraft:black_candle";
        let item = bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
            .expect("a built-in item")
            .id;
        assert_eq!(
            state_for_placement(item, pos, vanilla_tags(), &mut |at| if at == below(pos) {
                stone()
            } else {
                air()
            }),
            Some(state_of(
                name,
                &[("candles", "1"), ("lit", "false"), ("waterlogged", "false")],
            )),
            "{name}"
        );
    }
}
