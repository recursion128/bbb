use std::cell::RefCell;

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::RegistryJsonValue as Json;

use super::catalog_default_state;
use super::support::{default_state, origin, RecordingLevel};
use bbb_server_datapack::worldgen::BlockPredicate;
use bbb_server_worldgen::block_predicates::block_predicate_test;
use bbb_server_worldgen::WorldGenLevel;

struct UnobstructedLevel {
    result: bool,
    queries: RefCell<Vec<BlockPos>>,
}

impl UnobstructedLevel {
    fn new(result: bool) -> Self {
        Self {
            result,
            queries: RefCell::new(Vec::new()),
        }
    }
}

impl WorldGenLevel for UnobstructedLevel {
    fn ensure_can_write(&self, _origin: BlockPos) -> bool {
        true
    }

    fn is_unobstructed(&self, origin: BlockPos) -> bool {
        self.queries.borrow_mut().push(origin);
        self.result
    }

    fn get_block(&self, _pos: BlockPos) -> i32 {
        default_state("minecraft:air")
    }

    fn set_block(&mut self, _pos: BlockPos, _state: i32, _update_flags: i32) -> bool {
        false
    }

    fn get_min_y(&self) -> i32 {
        -64
    }

    fn get_max_y(&self) -> i32 {
        319
    }

    fn get_sea_level(&self) -> i32 {
        63
    }

    fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

    fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

    fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}
}

fn object(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect(),
    )
}

fn decode(
    type_id: &str,
    mut fields: Vec<(&str, Json)>,
) -> Result<BlockPredicate, bbb_server_datapack::worldgen::DecodeError> {
    fields.push(("type", string(type_id)));
    BlockPredicate::decode(&object(fields))
}

fn typed(type_id: &str, fields: Vec<(&str, Json)>) -> BlockPredicate {
    decode(type_id, fields).unwrap()
}

fn string(value: &str) -> Json {
    Json::String(value.into())
}

fn set(level: &RecordingLevel, pos: BlockPos, state: i32) {
    level
        .cells
        .borrow_mut()
        .insert((pos.x, pos.y, pos.z), state);
}

fn below(pos: BlockPos, dy: i32) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y - dy,
        z: pos.z,
    }
}

#[test]
fn offset_families_read_the_offset_block() {
    let level = RecordingLevel::admitting();
    let origin = origin();
    set(&level, below(origin, 1), default_state("minecraft:stone"));
    set(
        &level,
        below(origin, 2),
        catalog_default_state("minecraft:water"),
    );

    let offset_down = |dy: i32| {
        (
            "offset",
            Json::Array(vec![
                Json::Number("0".into()),
                Json::Number((-dy).to_string()),
                Json::Number("0".into()),
            ]),
        )
    };
    assert!(block_predicate_test(
        &typed("minecraft:solid", vec![offset_down(1)]),
        &level,
        origin
    )
    .unwrap());
    assert!(!block_predicate_test(&typed("minecraft:solid", vec![]), &level, origin).unwrap());
    assert!(block_predicate_test(&typed("minecraft:replaceable", vec![]), &level, origin).unwrap());
    assert!(!block_predicate_test(
        &typed("minecraft:replaceable", vec![offset_down(1)]),
        &level,
        origin
    )
    .unwrap());
    assert!(block_predicate_test(
        &typed(
            "minecraft:matching_fluids",
            vec![offset_down(2), ("fluids", string("minecraft:water"))]
        ),
        &level,
        origin
    )
    .unwrap());
    assert!(block_predicate_test(
        &typed(
            "minecraft:matching_fluids",
            vec![offset_down(2), ("fluids", string("#minecraft:water"))]
        ),
        &level,
        origin
    )
    .unwrap());
    assert!(!block_predicate_test(
        &typed(
            "minecraft:matching_fluids",
            vec![
                offset_down(2),
                ("fluids", string("minecraft:flowing_water"))
            ]
        ),
        &level,
        origin
    )
    .unwrap());
    assert!(block_predicate_test(
        &typed(
            "minecraft:matching_fluids",
            vec![("fluids", string("minecraft:empty"))]
        ),
        &level,
        origin
    )
    .unwrap());
    assert!(block_predicate_test(
        &typed(
            "minecraft:has_sturdy_face",
            vec![offset_down(1), ("direction", string("up"))]
        ),
        &level,
        origin
    )
    .unwrap());
    assert!(!block_predicate_test(
        &typed(
            "minecraft:has_sturdy_face",
            vec![("direction", string("up"))]
        ),
        &level,
        origin
    )
    .unwrap());
}

#[test]
fn would_survive_and_world_bounds_consult_the_level() {
    let level = RecordingLevel::admitting();
    let origin = origin();
    let sapling = typed(
        "minecraft:would_survive",
        vec![(
            "state",
            object(vec![
                ("Name", string("minecraft:oak_sapling")),
                ("Properties", object(vec![("stage", string("0"))])),
            ]),
        )],
    );
    assert!(!block_predicate_test(&sapling, &level, origin).unwrap());
    set(&level, below(origin, 1), default_state("minecraft:dirt"));
    assert!(block_predicate_test(&sapling, &level, origin).unwrap());

    let bounds = |dy: i32| {
        typed(
            "minecraft:inside_world_bounds",
            vec![(
                "offset",
                Json::Array(vec![
                    Json::Number("0".into()),
                    Json::Number(dy.to_string()),
                    Json::Number("0".into()),
                ]),
            )],
        )
    };
    assert!(block_predicate_test(&bounds(0), &level, origin).unwrap());
    let top = BlockPos {
        y: level.max_y,
        ..origin
    };
    assert!(block_predicate_test(&bounds(0), &level, top).unwrap());
    assert!(!block_predicate_test(&bounds(1), &level, top).unwrap());
    let bottom = BlockPos {
        y: level.min_y,
        ..origin
    };
    assert!(!block_predicate_test(&bounds(-1), &level, bottom).unwrap());
}

#[test]
fn unobstructed_uses_the_typed_level_seam_and_ignores_its_stored_offset() {
    let level = UnobstructedLevel::new(false);
    let origin = origin();
    let predicate = typed(
        "minecraft:unobstructed",
        vec![(
            "offset",
            Json::Array(vec![
                Json::Number("9".into()),
                Json::Number("-3".into()),
                Json::Number("5".into()),
            ]),
        )],
    );

    assert!(!block_predicate_test(&predicate, &level, origin).unwrap());
    assert_eq!(&*level.queries.borrow(), &[origin]);
    assert!(decode(
        "minecraft:unobstructed",
        vec![(
            "offset",
            Json::Array(vec![Json::Number("1".into()), Json::Number("2".into())]),
        )]
    )
    .is_err());
}
