use bbb_protocol::packets::BlockPos;

use super::support::{default_state, origin, place_feature, retained_from_json, RecordingLevel};
use super::{catalog_default_state, UPDATE_CLIENTS};

const ROOT_SYSTEM_JSON: &str = r##"{"type":"minecraft:root_system","config":{"feature":{"feature":{"type":"minecraft:simple_block","config":{"to_place":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:oak_log","Properties":{"axis":"y"}}}}},"placement":[]},"required_vertical_space_for_tree":1,"root_radius":2,"root_replaceable":"#minecraft:base_stone_overworld","root_state_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:rooted_dirt"}},"root_placement_attempts":4,"root_column_max_height":3,"hanging_root_radius":2,"hanging_roots_vertical_span":2,"hanging_root_state_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:hanging_roots","Properties":{"waterlogged":"false"}}},"hanging_root_placement_attempts":4,"allowed_vertical_water_for_tree":1,"allowed_tree_position":{"type":"minecraft:true"}}}"##;

/// Air origin inside a stone slab at the origin level (origin cell cut
/// out) with a two-block stone pillar above it: the tree lands on the
/// pillar top at y+2, rooted dirt goes into the slab, hanging roots hang
/// under it.
fn pillar_level() -> RecordingLevel {
    let level = RecordingLevel::admitting();
    let origin = origin();
    let stone = default_state("minecraft:stone");
    for dx in -2..=2 {
        for dz in -2..=2 {
            if (dx, dz) != (0, 0) {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, origin.y, origin.z + dz), stone);
            }
        }
    }
    for dy in 1..=2 {
        level
            .cells
            .borrow_mut()
            .insert((origin.x, origin.y + dy, origin.z), stone);
    }
    level
}

#[test]
fn root_system_places_tree_then_rooted_dirt_then_hanging_roots() {
    let mut level = pillar_level();
    let retained = retained_from_json(ROOT_SYSTEM_JSON.as_bytes());
    assert!(place_feature(&retained, &mut level, origin()));

    let origin = origin();
    let oak_log =
        crate::block_set_property(catalog_default_state("minecraft:oak_log"), "axis", "y");
    let rooted_dirt = default_state("minecraft:rooted_dirt");
    let hanging_roots = catalog_default_state("minecraft:hanging_roots");
    let sets = level.sets.borrow();
    // y=0 and y=1 fail (pillar above / air below), so the tree lands on
    // the pillar top before any root work.
    assert_eq!(
        sets[0],
        (
            BlockPos {
                x: origin.x,
                y: origin.y + 2,
                z: origin.z,
            },
            oak_log,
            UPDATE_CLIENTS,
        )
    );
    let rest = &sets[1..];
    assert!(!rest.is_empty(), "the seed must exercise dirt or roots");
    let first_root = rest
        .iter()
        .position(|(_, state, _)| *state == hanging_roots)
        .unwrap_or(rest.len());
    for (index, (pos, state, flags)) in rest.iter().enumerate() {
        assert_eq!(*flags, UPDATE_CLIENTS);
        let (dx, dy, dz) = (pos.x - origin.x, pos.y - origin.y, pos.z - origin.z);
        assert!(dx.abs() <= 1 && dz.abs() <= 1, "{pos:?}");
        if index < first_root {
            // `placeDirt` only covers the stone slab at the origin level.
            assert_eq!(*state, rooted_dirt);
            assert_eq!(dy, 0);
            assert_ne!((dx, dz), (0, 0));
        } else {
            // Empty with a sturdy block above: under the slab, or the
            // origin itself under the pillar.
            assert_eq!(*state, hanging_roots);
            assert!(
                (dy == -1 && (dx, dz) != (0, 0)) || (dy == 0 && (dx, dz) == (0, 0)),
                "{pos:?}"
            );
        }
    }
}

#[test]
fn root_system_weighted_provider_draws_at_each_replaceable_working_position() {
    let json = ROOT_SYSTEM_JSON
        .replace(
            r#""root_state_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:rooted_dirt"}}"#,
            r#""root_state_provider":{"type":"minecraft:weighted_state_provider","entries":[{"data":{"Name":"minecraft:rooted_dirt"},"weight":1},{"data":{"Name":"minecraft:dirt"},"weight":1}]}"#,
        )
        .replacen(
            r#""root_placement_attempts":4"#,
            r#""root_placement_attempts":16"#,
            1,
        );
    let retained = retained_from_json(json.as_bytes());
    let mut level = pillar_level();
    assert!(place_feature(&retained, &mut level, origin()));

    let origin = origin();
    let root_states: std::collections::BTreeSet<_> = level
        .sets
        .borrow()
        .iter()
        .filter(|(pos, _, _)| pos.y == origin.y)
        .map(|(_, state, _)| *state)
        .collect();
    assert!(root_states.contains(&default_state("minecraft:rooted_dirt")));
    assert!(root_states.contains(&default_state("minecraft:dirt")));
}

#[test]
fn root_system_returns_false_on_a_solid_origin_and_true_without_a_tree_spot() {
    let mut solid = RecordingLevel::filled(default_state("minecraft:stone"));
    let retained = retained_from_json(ROOT_SYSTEM_JSON.as_bytes());
    assert!(!place_feature(&retained, &mut solid, origin()));
    assert!(solid.sets.borrow().is_empty());

    let mut open = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut open, origin()));
    assert!(open.sets.borrow().is_empty());
}
