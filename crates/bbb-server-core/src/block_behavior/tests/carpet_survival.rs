//! `CarpetBlock` (`CarpetBlock.java`): the one cell a carpet checks, and what
//! it does when that cell turns to air.

use super::*;

/// The cell the carpet lies in. Deliberately not the origin so a read at the
/// wrong position cannot pass by coincidence.
const CARPET: BlockPos = BlockPos { x: -6, y: 71, z: 2 };

/// Whether `name` would stay lying on `below_name`, asked through the crate's
/// own dispatcher so the routing is exercised alongside the body.
fn survives_on(name: &str, below_name: &str) -> Option<bool> {
    let below_state = default_state(below_name);
    can_survive(default_state(name), CARPET, vanilla_tags(), &mut |at| {
        if at == below(CARPET) {
            below_state
        } else {
            air()
        }
    })
}

#[test]
fn a_carpet_needs_the_cell_below_to_be_anything_other_than_air() {
    // `CarpetBlock.canSurvive` (`CarpetBlock.java:50-52`) is
    // `!level.isEmptyBlock(pos.below())`, and `LevelReader.isEmptyBlock` is
    // `getBlockState(pos).isAir()` (`LevelReader.java:84-86`). So the test is
    // emptiness, not solidity: everything with a block in it holds a carpet.
    for below_name in [
        "minecraft:stone",
        // A torch has no support shape at all and still counts.
        "minecraft:torch",
        // So does water, which is why a carpet can be laid on a water source.
        "minecraft:water",
    ] {
        assert_eq!(
            survives_on("minecraft:white_carpet", below_name),
            Some(true),
            "{below_name}"
        );
    }
    // The three `isAir` blocks are the whole negative side.
    for below_name in ["minecraft:air", "minecraft:cave_air"] {
        assert_eq!(
            survives_on("minecraft:white_carpet", below_name),
            Some(false),
            "{below_name}"
        );
    }
}

#[test]
fn a_carpet_becomes_air_as_soon_as_the_cell_below_it_does() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), stone());
    world.set(at(0, 71, 0), default_state("minecraft:red_carpet"));

    world.set(at(0, 70, 0), air());
    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), air());
    assert_eq!(world.items(), vec![("minecraft:red_carpet".to_string(), 1)]);
}

#[test]
fn a_carpet_over_a_torch_survives_a_shape_update_from_any_side() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), default_state("minecraft:torch"));
    world.set(at(0, 71, 0), default_state("minecraft:moss_carpet"));

    for direction in [Direction::Down, Direction::North] {
        world.shape_update(at(0, 71, 0), direction);
        assert_eq!(
            world.state(at(0, 71, 0)),
            default_state("minecraft:moss_carpet"),
            "{direction:?}"
        );
    }
    assert!(world.items().is_empty());
}

#[test]
fn the_update_comes_from_any_direction_not_only_from_below() {
    let mut world = NativeWorld::new();
    world.set(at(0, 71, 0), default_state("minecraft:blue_carpet"));

    world.shape_update(at(0, 71, 0), Direction::North);

    assert_eq!(world.state(at(0, 71, 0)), air());
}

// ---------------------------------------------------------------------------
// The mechanical guard
// ---------------------------------------------------------------------------

#[test]
fn a_carpet_item_is_placed_only_where_the_cell_below_is_not_air() {
    // `CarpetBlock` declares no `getStateForPlacement`, so
    // [`state_for_placement`] answers `Block.getStateForPlacement`'s default
    // (`Block.java:466-468`) and then runs `BlockItem.canPlace`
    // (`BlockItem.java:134-138`) against this module's `canSurvive`.
    let pos = BlockPos { x: 4, y: 70, z: -9 };
    {
        let name = "minecraft:white_carpet";
        let placed = state_for_placement(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
                .expect("a built-in item")
                .id,
            pos,
            vanilla_tags(),
            &mut |at| if at == below(pos) { stone() } else { air() },
        );
        assert_eq!(placed, Some(default_state(name)), "{name} over stone");
        let refused = state_for_placement(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
                .expect("a built-in item")
                .id,
            pos,
            vanilla_tags(),
            &mut |_| air(),
        );
        assert_eq!(refused, None, "{name} over air");
    }
}
