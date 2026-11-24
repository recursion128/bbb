//! `BlockItem.getPlacementState` — which items place which default states.

use super::fixture::*;

// ---------------------------------------------------------------------------
// The three answers
// ---------------------------------------------------------------------------

#[test]
fn a_block_item_whose_block_declares_no_override_places_the_default_state() {
    // `Block.getStateForPlacement` (`Block.java:466-468`) is
    // `this.defaultBlockState()`, which `BlockItem.getPlacementState`
    // (`BlockItem.java:115-118`) hands straight to `placeBlock`.
    for name in ["minecraft:stone", "minecraft:glass"] {
        let (state, _) = place(name, stone());
        assert_eq!(state, Some(default_state_of(name)), "{name}");
    }
}

#[test]
fn the_note_block_item_places_the_state_its_own_override_computes() {
    // `NoteBlock.getStateForPlacement` (`NoteBlock.java:66-68`), the one
    // override this crate owns a body for.
    let (state, _) = place("minecraft:note_block", stone());
    assert_eq!(
        state,
        Some(note_block::state_for_placement(air(), stone())),
        "the note block places what its own override answers"
    );
    let instrument = vanilla_block_state_identity(state.expect("a placed state"))
        .expect("a built-in state")
        .properties["instrument"]
        .clone();
    assert_eq!(
        instrument, "basedrum",
        "`minecraft:stone` below is `NoteBlockInstrument.BASEDRUM`"
    );
}

#[test]
fn the_three_owned_placement_families_answer_their_context_properties() {
    assert_eq!(
        place("minecraft:oak_slab", stone()).0,
        Some(state_of(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "false")]
        ))
    );
    assert_eq!(
        place("minecraft:oak_stairs", stone()).0,
        Some(state_of(
            "minecraft:oak_stairs",
            &[
                ("facing", "south"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ]
        ))
    );
    let mut pillar = context(CLICKED);
    pillar.clicked_face = Direction::East;
    assert_eq!(
        state_for_placement(
            item_id("minecraft:oak_log"),
            pillar,
            vanilla_tags(),
            &mut |_| air()
        ),
        Some(state_of("minecraft:oak_log", &[("axis", "x")]))
    );
}

#[test]
fn a_vegetation_block_is_placed_only_where_its_own_support_tag_allows() {
    // `BlockItem.getPlacementState` (`BlockItem.java:115-118`) hands the
    // default state to `canPlace`, which asks `VegetationBlock.canSurvive`
    // (`VegetationBlock.java:44-47`). The answer is the one cell below, so
    // dirt takes a sapling and stone does not.
    for name in ["minecraft:oak_sapling", "minecraft:poppy"] {
        let (state, _) = place(name, dirt());
        assert_eq!(state, Some(default_state_of(name)), "{name} on dirt");
        assert_eq!(place(name, stone()).0, None, "{name} on stone");
    }
    // `NetherWartBlock.mayPlaceOn` (`NetherWartBlock.java:42-44`) is
    // `#minecraft:supports_nether_wart`, which is soul sand alone, so the
    // subclass body really is consulted instead of the base tag.
    assert_eq!(place("minecraft:nether_wart", dirt()).0, None);
    assert_eq!(
        place(
            "minecraft:nether_wart",
            default_state_of("minecraft:soul_sand")
        )
        .0,
        Some(default_state_of("minecraft:nether_wart"))
    );
}

#[test]
fn an_item_that_is_not_a_block_item_places_nothing() {
    // `Item.useOn` answers `InteractionResult.PASS` (`Item.java:179-181`);
    // only `BlockItem` overrides it (`BlockItem.java:41-46`).
    for name in [
        "minecraft:stick",
        "minecraft:diamond_sword",
        "minecraft:water_bucket",
        "minecraft:written_book",
    ] {
        assert_eq!(place(name, stone()).0, None, "{name}");
    }
}
