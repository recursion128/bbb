use bbb_server_worldgen::random::WorldgenRandom;

use bbb_server_worldgen::multiface::{all_shuffled, face_property};

use super::support::{default_state, origin, place_feature, retained_from_json, RecordingLevel};
use super::{
    block_set_bool, catalog_default_state, offset_block_pos, FEATURE_SET_BLOCK, UPDATE_CLIENTS,
};

fn single_charge_retained() -> bbb_server_datapack::worldgen::ConfiguredFeature {
    retained_from_json(
        br#"{"type":"minecraft:sculk_patch","config":{"charge_count":1,"amount_per_charge":1,"spread_attempts":1,"growth_rounds":0,"spread_rounds":1,"extra_rare_growths":0,"catalyst_chance":1.0}}"#,
    )
}

/// Stone everywhere with an air pocket at the origin: the one cursor spreads
/// a vein into the pocket, converts a stone neighbour to sculk, discharges,
/// and the patch is capped by a catalyst.
#[test]
fn sculk_patch_grows_vein_sculk_and_catalyst_from_an_air_pocket() {
    let stone = default_state("minecraft:stone");
    let mut level = RecordingLevel::filled(stone);
    let origin = origin();
    let air = default_state("minecraft:air");
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), air);

    assert!(place_feature(&single_charge_retained(), &mut level, origin));

    let sets = level.sets.borrow().clone();
    // `spreadAll` over the air pocket: faces in `Direction.values()` order,
    // each toward the four directions off its axis, so the vein gains
    // north, south, west, east (from the DOWN face) then down, up (from NORTH).
    let mut vein = catalog_default_state("minecraft:sculk_vein");
    let expected_faces = ["north", "south", "west", "east", "down", "up"];
    for (index, face) in expected_faces.iter().enumerate() {
        vein = block_set_bool(vein, face, true);
        assert_eq!(
            sets[index],
            (origin, vein, UPDATE_CLIENTS),
            "vein write {index}"
        );
    }
    assert_eq!(level.postprocess.borrow().len(), 6);
    // `attemptPlaceSculk` walks `Direction.allShuffled(random)` from the
    // feature random, which nothing before it has drawn from.
    let support = all_shuffled(&mut WorldgenRandom::with_seed(1))[0];
    let support_pos = offset_block_pos(origin, support);
    assert_eq!(
        sets[6],
        (
            support_pos,
            default_state("minecraft:sculk"),
            FEATURE_SET_BLOCK
        )
    );
    // Discharging drops the face that now looks at sculk.
    let discharged = block_set_bool(vein, face_property(support), false);
    assert_eq!(sets[7], (origin, discharged, FEATURE_SET_BLOCK));
    assert_eq!(
        sets[8],
        (
            origin,
            catalog_default_state("minecraft:sculk_catalyst"),
            FEATURE_SET_BLOCK
        )
    );
    assert_eq!(sets.len(), 9);
}

#[test]
fn sculk_patch_refuses_an_origin_inside_solid_ground() {
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(
        &single_charge_retained(),
        &mut level,
        origin()
    ));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn sculk_patch_refuses_an_air_origin_without_a_full_block_neighbour() {
    let mut level = RecordingLevel::admitting();
    assert!(!place_feature(
        &single_charge_retained(),
        &mut level,
        origin()
    ));
    assert!(level.sets.borrow().is_empty());
}
