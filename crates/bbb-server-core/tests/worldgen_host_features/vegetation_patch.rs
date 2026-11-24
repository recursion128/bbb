use bbb_protocol::packets::BlockPos;

use super::support::{default_state, origin, place_feature, RecordingLevel};
use super::{block_set_bool, catalog_default_state, UPDATE_CLIENTS};

pub(crate) fn patch_json(vegetation_chance: &str, vegetation_state: &str) -> String {
    patch_with_ground_provider_json(
        vegetation_chance,
        vegetation_state,
        r#"{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:moss_block"}}"#,
    )
}

fn patch_with_ground_provider_json(
    vegetation_chance: &str,
    vegetation_state: &str,
    ground_state: &str,
) -> String {
    format!(
        r##"{{"type":"minecraft:vegetation_patch","config":{{"replaceable":"#minecraft:base_stone_overworld","ground_state":{ground_state},"vegetation_feature":{{"feature":{{"type":"minecraft:simple_block","config":{{"to_place":{{"type":"minecraft:simple_state_provider","state":{vegetation_state}}}}}}},"placement":[]}},"surface":"floor","depth":1,"extra_bottom_block_chance":0.0,"vertical_range":1,"vegetation_chance":{vegetation_chance},"xz_radius":1,"extra_edge_column_chance":0.0}}}}"##
    )
}

/// Stone floor two layers thick under an air origin, wide enough for the
/// radius-2 patch and its neighbours.
pub(crate) fn floored_level() -> RecordingLevel {
    let level = RecordingLevel::admitting();
    let origin = origin();
    let stone = default_state("minecraft:stone");
    for dx in -4..=4 {
        for dz in -4..=4 {
            for dy in 1..=2 {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, origin.y - dy, origin.z + dz), stone);
            }
        }
    }
    level
}

/// The interior floor cells in the patch's column walk order.
fn surface_order(y: i32) -> Vec<BlockPos> {
    let origin = origin();
    (-1..=1)
        .flat_map(|dx| {
            (-1..=1).map(move |dz| BlockPos {
                x: origin.x + dx,
                y,
                z: origin.z + dz,
            })
        })
        .collect()
}

#[test]
fn vegetation_patch_lays_ground_in_column_order_and_vegetation_in_surface_order() {
    let mut level = floored_level();
    let retained = super::support::retained_from_json(
        patch_json("1.0", r#"{"Name":"minecraft:stone"}"#).as_bytes(),
    );
    assert!(place_feature(&retained, &mut level, origin()));

    let origin = origin();
    let moss = default_state("minecraft:moss_block");
    let stone = default_state("minecraft:stone");
    let mut expected = Vec::new();
    for dx in -1..=1 {
        for dz in -1..=1 {
            expected.push((
                BlockPos {
                    x: origin.x + dx,
                    y: origin.y - 1,
                    z: origin.z + dz,
                },
                moss,
                UPDATE_CLIENTS,
            ));
        }
    }
    for pos in surface_order(origin.y) {
        expected.push((pos, stone, UPDATE_CLIENTS));
    }
    assert_eq!(*level.sets.borrow(), expected);
}

#[test]
fn vegetation_patch_weighted_ground_provider_draws_for_each_below_position() {
    let mut level = floored_level();
    let json = patch_with_ground_provider_json(
        "0.0",
        r#"{"Name":"minecraft:stone"}"#,
        r#"{"type":"minecraft:weighted_state_provider","entries":[{"data":{"Name":"minecraft:moss_block"},"weight":1},{"data":{"Name":"minecraft:dirt"},"weight":1}]}"#,
    );
    let retained = super::support::retained_from_json(json.as_bytes());
    assert!(place_feature(&retained, &mut level, origin()));

    let states: std::collections::BTreeSet<_> = level
        .sets
        .borrow()
        .iter()
        .map(|(_, state, _)| *state)
        .collect();
    assert!(states.contains(&default_state("minecraft:moss_block")));
    assert!(states.contains(&default_state("minecraft:dirt")));
}

#[test]
fn waterlogged_vegetation_patch_floods_sheltered_cells_and_waterlogs_vegetation() {
    let mut level = floored_level();
    let json = patch_json(
        "1.0",
        r#"{"Name":"minecraft:mangrove_roots","Properties":{"waterlogged":"false"}}"#,
    )
    .replace(
        "minecraft:vegetation_patch",
        "minecraft:waterlogged_vegetation_patch",
    );
    let retained = super::support::retained_from_json(json.as_bytes());
    assert!(place_feature(&retained, &mut level, origin()));

    let origin = origin();
    let moss = default_state("minecraft:moss_block");
    let water = catalog_default_state("minecraft:water");
    let dry_roots = block_set_bool(
        catalog_default_state("minecraft:mangrove_roots"),
        "waterlogged",
        false,
    );
    let wet_roots = block_set_bool(dry_roots, "waterlogged", true);
    let mut expected = Vec::new();
    for dx in -1..=1 {
        for dz in -1..=1 {
            expected.push((
                BlockPos {
                    x: origin.x + dx,
                    y: origin.y - 1,
                    z: origin.z + dz,
                },
                moss,
                UPDATE_CLIENTS,
            ));
        }
    }
    // Every interior floor cell is walled by moss/stone, so all nine flood.
    for pos in surface_order(origin.y - 1) {
        expected.push((pos, water, UPDATE_CLIENTS));
    }
    for pos in surface_order(origin.y - 1) {
        expected.push((pos, dry_roots, UPDATE_CLIENTS));
        expected.push((pos, wet_roots, UPDATE_CLIENTS));
    }
    assert_eq!(*level.sets.borrow(), expected);
}

#[test]
fn vegetation_patch_returns_false_without_a_sturdy_floor() {
    let mut level = RecordingLevel::admitting();
    let retained = super::support::retained_from_json(
        patch_json("0.0", r#"{"Name":"minecraft:stone"}"#).as_bytes(),
    );
    assert!(!place_feature(&retained, &mut level, origin()));
    assert!(level.sets.borrow().is_empty());
}
