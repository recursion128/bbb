use std::collections::BTreeMap;

use serde_json::{json, Value};

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::BlockPos;

use super::super::{LootBlockStateAt, LootRuntimeReason};
use super::location_predicate_matches as matches;
use bbb_server_datapack::loot::parse_location_predicate;

const POS: BlockPos = BlockPos { x: 4, y: 70, z: -9 };

fn blocks(entries: &[(BlockPos, i32)]) -> impl Fn(BlockPos) -> Option<i32> {
    let states: BTreeMap<_, _> = entries
        .iter()
        .map(|(pos, state)| ((pos.x, pos.y, pos.z), *state))
        .collect();
    move |pos| states.get(&(pos.x, pos.y, pos.z)).copied()
}

fn below(distance: i32) -> BlockPos {
    BlockPos {
        y: POS.y - distance,
        ..POS
    }
}

fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties: BTreeMap<String, String> = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("the generated registry carries {name} {properties:?}"))
}

fn default_state_of(name: &str) -> i32 {
    let block = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("the generated registry carries {name}"));
    vanilla_block_default_state_id(block.id).unwrap_or_else(|| panic!("{name} has a default state"))
}

/// The predicate at the centre of [`POS`], which is the origin
/// `LocationCheck.test` offsets by zero.
fn at_center(predicate: &Value, level: &LootBlockStateAt<'_>) -> Result<bool, LootRuntimeReason> {
    matches(
        &parse_location_predicate(predicate).unwrap(),
        f64::from(POS.x) + 0.5,
        f64::from(POS.y) + 0.5,
        f64::from(POS.z) + 0.5,
        None,
        Some(level),
        Some(crate::inventory::test_item_resolver()),
    )
}

#[test]
fn a_block_tag_answers_the_holder_set_it_names() {
    let sign = state_of(
        "minecraft:oak_wall_sign",
        &[("facing", "north"), ("waterlogged", "false")],
    );
    let level = blocks(&[(POS, sign)]);
    let predicate = json!({"block":{"blocks":"#minecraft:all_signs"}});
    assert_eq!(at_center(&predicate, &level), Ok(true));

    let stone = blocks(&[(POS, default_state_of("minecraft:stone"))]);
    assert_eq!(at_center(&predicate, &stone), Ok(false));
}

/// `heart_transplanter`'s own term: a tag beside a state property.
#[test]
fn a_block_tag_and_a_state_property_are_both_read() {
    let predicate = json!({"block":{"blocks":"#minecraft:pale_oak_logs","state":{"axis":"y"}}});
    let upright = blocks(&[(POS, state_of("minecraft:pale_oak_log", &[("axis", "y")]))]);
    assert_eq!(at_center(&predicate, &upright), Ok(true));

    let sideways = blocks(&[(POS, state_of("minecraft:pale_oak_log", &[("axis", "x")]))]);
    assert_eq!(at_center(&predicate, &sideways), Ok(false));
}

/// `HolderSet.contains` on a tag nothing declares is false, not an error.
#[test]
fn an_undeclared_block_tag_is_an_empty_holder_set() {
    let level = blocks(&[(POS, default_state_of("minecraft:stone"))]);
    let predicate = json!({"block":{"blocks":"#minecraft:no_such_block_tag"}});
    assert_eq!(at_center(&predicate, &level), Ok(false));
}

#[test]
fn the_smokey_walk_finds_a_lit_campfire_within_five_below() {
    let lit = state_of(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", "true"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let air = default_state_of("minecraft:air");
    let hive = default_state_of("minecraft:beehive");
    let mut entries = vec![(POS, hive)];
    entries.extend((1..=4).map(|distance| (below(distance), air)));
    entries.push((below(5), lit));
    let level = blocks(&entries);
    assert_eq!(at_center(&json!({"smokey":true}), &level), Ok(true));
    assert_eq!(at_center(&json!({"smokey":false}), &level), Ok(false));
}

/// `:264-266` is reached before `:268`, so the sixth block below is out of
/// range and the walk falls through to false.
#[test]
fn the_smokey_walk_stops_five_blocks_down() {
    let lit = state_of(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", "true"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let air = default_state_of("minecraft:air");
    let mut entries = vec![(POS, default_state_of("minecraft:beehive"))];
    entries.extend((1..=5).map(|distance| (below(distance), air)));
    entries.push((below(6), lit));
    let level = blocks(&entries);
    assert_eq!(at_center(&json!({"smokey":true}), &level), Ok(false));
}

/// `:268-272`: a shape that meets the virtual post ends the walk at the one
/// block under it, whatever stands further down.
#[test]
fn a_shape_across_the_post_ends_the_walk_at_the_block_under_it() {
    let lit = state_of(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", "true"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let stone = default_state_of("minecraft:stone");
    let hive = default_state_of("minecraft:beehive");
    let blocked = blocks(&[
        (POS, hive),
        (below(1), stone),
        (below(2), default_state_of("minecraft:air")),
        (below(3), lit),
    ]);
    assert_eq!(at_center(&json!({"smokey":true}), &blocked), Ok(false));

    let carried = blocks(&[(POS, hive), (below(1), stone), (below(2), lit)]);
    assert_eq!(at_center(&json!({"smokey":true}), &carried), Ok(true));
}

/// A shape that misses the 4x4 post lets the smoke past.
#[test]
fn a_shape_beside_the_post_does_not_block_the_smoke() {
    let lit = state_of(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", "true"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let ladder = state_of(
        "minecraft:ladder",
        &[("facing", "north"), ("waterlogged", "false")],
    );
    let level = blocks(&[
        (POS, default_state_of("minecraft:beehive")),
        (below(1), ladder),
        (below(2), lit),
    ]);
    assert_eq!(at_center(&json!({"smokey":true}), &level), Ok(true));
}

#[test]
fn a_position_term_reads_the_offset_origin_and_no_level() {
    let level = blocks(&[]);
    let inside = json!({"position":{"y":{"min":70.0,"max":71.0}}});
    assert_eq!(at_center(&inside, &level), Ok(true));

    let above = json!({"position":{"y":{"min":71.0}}});
    assert_eq!(at_center(&above, &level), Ok(false));

    // The bare-number spelling is `Bounds.exactly`.
    assert_eq!(at_center(&json!({"position":{"x":4.5}}), &level), Ok(true));
    assert_eq!(at_center(&json!({"position":{"x":4.0}}), &level), Ok(false));

    // Nothing above reads the level, so an absent one is not a refusal.
    assert_eq!(
        matches(
            &parse_location_predicate(&inside).unwrap(),
            4.5,
            70.5,
            -8.5,
            None,
            None,
            None,
        ),
        Ok(true)
    );
}

#[test]
fn a_fluid_term_reads_the_block_state_s_own_fluid_state() {
    let level = blocks(&[(POS, state_of("minecraft:water", &[("level", "0")]))]);
    assert_eq!(
        at_center(&json!({"fluid":{"fluids":"minecraft:water"}}), &level),
        Ok(true)
    );
    assert_eq!(
        at_center(&json!({"fluid":{"fluids":"#minecraft:water"}}), &level),
        Ok(true)
    );
    assert_eq!(
        at_center(&json!({"fluid":{"fluids":"minecraft:lava"}}), &level),
        Ok(false)
    );
    // A source declares `falling` and no `level`, and a name the definition
    // has no property for makes the matcher false.
    assert_eq!(
        at_center(&json!({"fluid":{"state":{"falling":"false"}}}), &level),
        Ok(true)
    );
    assert_eq!(
        at_center(&json!({"fluid":{"state":{"level":"0"}}}), &level),
        Ok(false)
    );
    assert_eq!(
        at_center(
            &json!({"fluid":{"state":{"falling":{"min":"false","max":"true"}}}}),
            &level
        ),
        Ok(true)
    );
    assert_eq!(
        at_center(&json!({"fluid":{"state":{"level":{"min":"1"}}}}), &level),
        Ok(false)
    );
    let flowing = blocks(&[(POS, state_of("minecraft:water", &[("level", "3")]))]);
    for (range, expected) in [
        (json!({"min":"4","max":"6"}), true),
        (json!({"min":"6"}), false),
        (json!({"min":"0"}), false),
        (json!({"max":"9"}), false),
    ] {
        assert_eq!(
            at_center(&json!({"fluid":{"state":{"level":range}}}), &flowing),
            Ok(expected)
        );
    }

    let dry = blocks(&[(POS, default_state_of("minecraft:stone"))]);
    assert_eq!(
        at_center(&json!({"fluid":{"fluids":"minecraft:empty"}}), &dry),
        Ok(true)
    );
}

/// A waterlogged block carries the same source state its block does.
#[test]
fn a_waterlogged_block_answers_both_halves() {
    let slab = state_of(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "true")],
    );
    let level = blocks(&[(POS, slab)]);
    assert_eq!(
        at_center(
            &json!({"block":{"blocks":"minecraft:oak_slab"},"fluid":{"fluids":"minecraft:water"}}),
            &level
        ),
        Ok(true)
    );
}

/// An unloaded position is false before any of the four terms that gate on it
/// is read, which is what keeps a refusal off a position nothing shows.
#[test]
fn an_unloaded_position_is_false_before_the_terms_that_gate_on_it() {
    let level = blocks(&[]);
    for term in [
        json!({"biomes":"minecraft:meadow"}),
        json!({"structures":"minecraft:village_plains"}),
        json!({"light":{"light":{"min":10}}}),
        json!({"block":{"blocks":"minecraft:stone"}}),
        json!({"fluid":{"fluids":"minecraft:water"}}),
    ] {
        assert_eq!(at_center(&term, &level), Ok(false), "{term}");
    }
}

#[test]
fn dimension_uses_the_actual_level_key_without_reading_blocks() {
    let predicate = parse_location_predicate(&json!({"dimension":"test:moon"})).unwrap();
    super::check_location_predicate(&predicate).unwrap();
    let test = |dimension, level: Option<&LootBlockStateAt<'_>>| {
        matches(&predicate, 4.5, 70.5, -8.5, dimension, level, None)
    };
    assert_eq!(test(Some("test:moon"), None), Ok(true));
    assert_eq!(test(Some("minecraft:overworld"), None), Ok(false));
    assert_eq!(test(None, None), Ok(false));
    let unloaded = |_: BlockPos| None;
    assert_eq!(test(Some("test:moon"), Some(&unloaded)), Ok(true));
}
