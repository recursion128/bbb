use std::io::Write;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::encode_named_nbt_summary;
use bbb_protocol::NbtSummaryEntry;
use bbb_server_worldgen::random::WorldgenRandom;
use flate2::write::GzEncoder;
use flate2::Compression;

use bbb_server_datapack::structure_template::parse_structure_template;

use super::catalog_default_state;
use super::seam::SeamLevel;
use super::support::{origin, retained_from_json, RecordingLevel};
use super::UPDATE_NONE;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;
use bbb_server_datapack::structure_template::StructureTemplate;
use bbb_server_worldgen::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use bbb_server_worldgen::place_configured_feature;
use bbb_server_worldgen::structure_processor::StructureProcessor;
use bbb_server_worldgen::structure_template_placement::{
    place_materialized_template, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
const ZERO: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}

fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

fn ints(values: &[i32]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.iter().map(|v| NbtSummaryValue::Int(*v)).collect())
}

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties).unwrap()
}

fn template(
    size: [i32; 3],
    palette: &[(&str, &[(&str, &str)])],
    blocks: &[([i32; 3], i32)],
) -> StructureTemplate {
    let blocks = blocks
        .iter()
        .map(|(pos, state)| (*pos, *state, None))
        .collect::<Vec<_>>();
    template_with_nbt(size, palette, &blocks)
}

fn template_with_nbt(
    size: [i32; 3],
    palette: &[(&str, &[(&str, &str)])],
    blocks: &[([i32; 3], i32, Option<NbtSummaryValue>)],
) -> StructureTemplate {
    let palette_nbt = palette
        .iter()
        .map(|(name, properties)| {
            let mut entry = vec![("Name", string(name))];
            if !properties.is_empty() {
                entry.push((
                    "Properties",
                    compound(properties.iter().map(|(k, v)| (*k, string(v))).collect()),
                ));
            }
            compound(entry)
        })
        .collect();
    let blocks_nbt = blocks
        .iter()
        .map(|(pos, state, nbt)| {
            let mut fields = vec![("pos", ints(pos)), ("state", NbtSummaryValue::Int(*state))];
            if let Some(nbt) = nbt {
                fields.push(("nbt", nbt.clone()));
            }
            compound(fields)
        })
        .collect();
    let root = compound(vec![
        ("size", ints(&size)),
        ("palette", NbtSummaryValue::List(palette_nbt)),
        ("blocks", NbtSummaryValue::List(blocks_nbt)),
        ("entities", NbtSummaryValue::List(Vec::new())),
    ]);
    let bytes = encode_named_nbt_summary("", &root, 1 << 20).unwrap();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&bytes).unwrap();
    parse_structure_template(&encoder.finish().unwrap()).unwrap()
}

const CONFIG_JSON: &[u8] = br##"{
    "type": "minecraft:fossil",
    "config": {
        "fossil_structures": ["minecraft:fossil/test_base"],
        "overlay_structures": ["minecraft:fossil/test_overlay"],
        "fossil_processors": {
            "processors": [
                {"processor_type": "minecraft:block_rot", "integrity": 1.0},
                {"processor_type": "minecraft:protected_blocks", "value": "#minecraft:features_cannot_replace"}
            ]
        },
        "overlay_processors": "minecraft:fossil_test",
        "max_empty_corners_allowed": 4
    }
}"##;

const OVERLAY_PROCESSOR_LIST: &str = r##"{"processors":[
    {"processor_type":"minecraft:block_rot","integrity":1.0},
    {"processor_type":"minecraft:rule","rules":[{
        "input_predicate":{"predicate_type":"minecraft:block_match","block":"minecraft:coal_ore"},
        "location_predicate":{"predicate_type":"minecraft:always_true"},
        "output_state":{"Name":"minecraft:deepslate_diamond_ore"}}]},
    {"processor_type":"minecraft:protected_blocks","value":"#minecraft:features_cannot_replace"}
]}"##;

/// A two-block pillar (axis x below axis y) and an overlay carrying coal
/// ore for the upper block only.
fn level_with_templates(inner: RecordingLevel) -> SeamLevel {
    let mut level = SeamLevel::wrap(inner);
    level.structure_templates.insert(
        "minecraft:fossil/test_base".to_string(),
        template(
            [1, 2, 1],
            &[
                ("minecraft:bone_block", &[("axis", "x")]),
                ("minecraft:bone_block", &[("axis", "y")]),
            ],
            &[([0, 0, 0], 0), ([0, 1, 0], 1)],
        ),
    );
    level.structure_templates.insert(
        "minecraft:fossil/test_overlay".to_string(),
        template([1, 2, 1], &[("minecraft:coal_ore", &[])], &[([0, 1, 0], 0)]),
    );
    level.insert_registry_json(
        PROCESSOR_LIST_REGISTRY,
        "minecraft:fossil_test",
        OVERLAY_PROCESSOR_LIST,
    );
    level
}

fn stone_level() -> SeamLevel {
    let mut inner = RecordingLevel::filled(state("minecraft:stone", &[]));
    inner.ocean_floor_wg_height = Some(64);
    level_with_templates(inner)
}

fn place_template_for_test(
    template: &StructureTemplate,
    level: &mut SeamLevel,
    processors: &[StructureProcessor],
) -> bool {
    place_materialized_template(
        template.materialized(),
        level,
        MaterializedTemplatePlacement {
            position: ZERO,
            reference_pos: ZERO,
            rotation: JigsawRotation::None,
            mirror: bbb_server_worldgen::noise::ruined_portal::RuinedPortalMirror::None,
            pivot: BlockPos { x: 0, y: 0, z: 0 },
            finalize_entities: false,
            ignore_entities: false,
            bounding_box: JigsawBoundingBox {
                min: BlockPos {
                    x: -16,
                    y: -64,
                    z: -16,
                },
                max: BlockPos {
                    x: 16,
                    y: 320,
                    z: 16,
                },
            },
            processors,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            update_mode: UPDATE_NONE,
            known_shape: false,
        },
        &mut WorldgenRandom::with_seed(1),
        TemplateSettingsRandom::Shared,
    )
}

/// Places the fossil with seed 256 into `level` and returns its writes.
fn place_seed_256(level: &mut SeamLevel) -> bool {
    place_configured_feature(
        &retained_from_json(CONFIG_JSON),
        level,
        origin(),
        &mut WorldgenRandom::with_seed(256),
    )
}

/// The y of the lower bone of the seed-256 pillar.
fn pillar_base() -> i32 {
    let mut level = stone_level();
    assert!(place_seed_256(&mut level));
    let base = level.inner.sets.borrow()[0].0.y;
    base
}

/// The pillar sinks 15 + `nextInt(10)` below the surface with its lower bone
/// rotated onto a horizontal axis, and the overlay's coal ore is rewritten to
/// deepslate diamond ore by the rule.
#[test]
fn place_fossil_lowers_the_rotated_pillar_and_applies_the_overlay() {
    let mut level = stone_level();
    assert!(place_seed_256(&mut level));
    let sets = level.inner.sets.borrow();
    let base = sets[0].0.y;
    assert!((64 - 15 - 9..=64 - 15).contains(&base), "{base}");
    let at = |y| BlockPos { x: 4, y, z: -8 };
    assert_eq!(sets.len(), 3);
    assert_eq!(sets[0].0, at(base));
    assert!([
        state("minecraft:bone_block", &[("axis", "x")]),
        state("minecraft:bone_block", &[("axis", "z")]),
    ]
    .contains(&sets[0].1));
    assert_eq!(
        sets[1..],
        [
            (
                at(base + 1),
                state("minecraft:bone_block", &[("axis", "y")]),
                UPDATE_NONE
            ),
            (
                at(base + 1),
                state("minecraft:deepslate_diamond_ore", &[]),
                UPDATE_NONE
            ),
        ]
    );
    assert!(level.inner.fluid_ticks.borrow().is_empty());
    assert!(level.inner.block_ticks.borrow().is_empty());
}

#[test]
fn place_fossil_schedules_a_source_water_neighbour_through_update_shape_at_edge() {
    let base = pillar_base();
    let mut level = stone_level();
    let water = catalog_default_state("minecraft:water");
    level.set_cell(
        BlockPos {
            x: 5,
            y: base,
            z: -8,
        },
        water,
    );
    assert!(place_seed_256(&mut level));
    assert_eq!(
        level.inner.fluid_ticks.borrow().as_slice(),
        &[(
            BlockPos {
                x: 5,
                y: base,
                z: -8
            },
            5
        )]
    );
}

#[test]
fn place_fossil_schedules_a_lava_neighbour_at_the_slow_lava_delay() {
    let base = pillar_base();
    let lava = catalog_default_state("minecraft:lava");
    let mut level = stone_level();
    level.set_cell(
        BlockPos {
            x: 3,
            y: base + 1,
            z: -8,
        },
        lava,
    );
    assert!(place_seed_256(&mut level));
    // Once per `placeInWorld`: the base pass and the overlay pass both
    // expose the west face of the upper bone.
    assert_eq!(
        level.inner.fluid_ticks.borrow().as_slice(),
        &[
            (
                BlockPos {
                    x: 3,
                    y: base + 1,
                    z: -8
                },
                30
            ),
            (
                BlockPos {
                    x: 3,
                    y: base + 1,
                    z: -8
                },
                30
            )
        ]
    );
}

#[test]
fn place_fossil_updates_a_neighbour_with_a_block_behaviour_owner() {
    let base = pillar_base();
    let mut level = stone_level();
    level.set_cell(
        BlockPos {
            x: 4,
            y: base + 2,
            z: -8,
        },
        catalog_default_state("minecraft:oak_leaves"),
    );
    assert!(place_seed_256(&mut level));
}

#[test]
fn place_fossil_returns_false_when_too_many_corners_are_empty() {
    let mut level = level_with_templates(RecordingLevel::admitting());
    let placed = place_configured_feature(
        &retained_from_json(CONFIG_JSON),
        &mut level,
        origin(),
        &mut WorldgenRandom::with_seed(256),
    );
    assert!(!placed);
    assert!(level.inner.sets.borrow().is_empty());
}

#[test]
fn template_processors_consume_jigsaw_nbt_before_the_block_entity_seam() {
    let template = template_with_nbt(
        [1, 1, 1],
        &[("minecraft:jigsaw", &[("orientation", "north_up")])],
        &[(
            [0, 0, 0],
            0,
            Some(compound(vec![("final_state", string("minecraft:stone"))])),
        )],
    );
    let processors = [StructureProcessor::JigsawReplacement];
    let mut level = SeamLevel::filled(state("minecraft:stone", &[]));
    assert!(place_template_for_test(&template, &mut level, &processors));
    assert_eq!(
        level.inner.sets.borrow()[0].1,
        state("minecraft:stone", &[])
    );
}

const PROCESSOR_LIST_REGISTRY: &str = "minecraft:worldgen/processor_list";
