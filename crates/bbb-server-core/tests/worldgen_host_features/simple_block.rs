use super::support::{default_state, origin, place_feature, retained_from_json, RecordingLevel};
use super::{block_is, offset_block_pos, UPDATE_CLIENTS};
use bbb_protocol::block_state_types::{
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_datapack::worldgen::ConfiguredFeature;
use bbb_server_worldgen::WorldGenLevel;
use std::collections::BTreeMap;
fn prepare_nylium_floor(level: &mut RecordingLevel, origin: BlockPos) {
    let nylium = default_state("minecraft:crimson_nylium");
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), nylium);
        }
    }
}
fn simple_block_json(state: &str, schedule_tick: Option<bool>) -> String {
    match schedule_tick {
        Some(tick) => format!(
            r#"{{"type":"minecraft:simple_block","config":{{"to_place":{{"type":"simple_state_provider","state":{{"Name":"{state}"}}}},"schedule_tick":{tick}}}}}"#
        ),
        None => format!(
            r#"{{"type":"minecraft:simple_block","config":{{"to_place":{{"type":"simple_state_provider","state":{{"Name":"{state}"}}}}}}}}"#
        ),
    }
}

fn simple_block_retained(state: &str, schedule_tick: Option<bool>) -> ConfiguredFeature {
    retained_from_json(simple_block_json(state, schedule_tick).as_bytes())
}

#[test]
fn place_configured_feature_simple_block_places_origin() {
    let retained = simple_block_retained("stone", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.sets.borrow(), [(origin, stone, UPDATE_CLIENTS)]);
    assert!(level.block_ticks.borrow().is_empty());
}

#[test]
fn place_configured_feature_simple_block_respects_can_survive() {
    let retained = simple_block_retained("nether_sprouts", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut air, origin));
    assert!(air.sets.borrow().is_empty());

    let mut nylium = RecordingLevel::admitting();
    prepare_nylium_floor(&mut nylium, origin);
    assert!(place_feature(&retained, &mut nylium, origin));
    assert!(block_is(
        nylium.get_block(origin),
        "minecraft:nether_sprouts"
    ));
}

#[test]
fn place_configured_feature_simple_block_uses_mushroom_survival() {
    let retained = simple_block_retained("red_mushroom", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let below = BlockPos { y: 63, ..origin };

    let mut dark_solid = RecordingLevel::admitting();
    dark_solid
        .cells
        .borrow_mut()
        .insert((below.x, below.y, below.z), default_state("minecraft:dirt"));
    dark_solid.raw_brightness = 12;
    assert!(place_feature(&retained, &mut dark_solid, origin));

    let mut bright_solid = RecordingLevel::admitting();
    bright_solid
        .cells
        .borrow_mut()
        .insert((below.x, below.y, below.z), default_state("minecraft:dirt"));
    bright_solid.raw_brightness = 13;
    assert!(!place_feature(&retained, &mut bright_solid, origin));

    let mut override_support = RecordingLevel::admitting();
    override_support.cells.borrow_mut().insert(
        (below.x, below.y, below.z),
        default_state("minecraft:crimson_nylium"),
    );
    override_support.raw_brightness = 15;
    assert!(place_feature(&retained, &mut override_support, origin));
}

#[test]
fn place_configured_feature_simple_block_places_double_plant_halves() {
    let retained = simple_block_retained("sunflower", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:dirt"),
    );
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].0, origin);
    assert_eq!(
        sets[1].0,
        BlockPos {
            x: origin.x,
            y: origin.y + 1,
            z: origin.z
        }
    );
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert!(block_is(sets[0].1, "minecraft:sunflower"));
    assert!(block_is(sets[1].1, "minecraft:sunflower"));
    let lower = vanilla_block_state_identity(sets[0].1).unwrap();
    let upper = vanilla_block_state_identity(sets[1].1).unwrap();
    assert_eq!(
        lower.properties.get("half").map(String::as_str),
        Some("lower")
    );
    assert_eq!(
        upper.properties.get("half").map(String::as_str),
        Some("upper")
    );
}

#[test]
fn place_configured_feature_simple_block_double_plant_requires_empty_above() {
    let retained = simple_block_retained("sunflower", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:dirt"),
    );
    level.cells.borrow_mut().insert(
        (origin.x, origin.y + 1, origin.z),
        default_state("minecraft:stone"),
    );
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_simple_block_places_small_dripleaf_halves() {
    let retained = simple_block_retained("small_dripleaf", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:moss_block"),
    );

    assert!(place_feature(&retained, &mut level, origin));

    let sets = level.sets.borrow();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].0, origin);
    assert_eq!(sets[1].0, offset_block_pos(origin, Direction::Up));
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert_eq!(
        vanilla_block_state_identity(sets[0].1)
            .unwrap()
            .properties
            .get("half")
            .map(String::as_str),
        Some("lower")
    );
    assert_eq!(
        vanilla_block_state_identity(sets[1].1)
            .unwrap()
            .properties
            .get("half")
            .map(String::as_str),
        Some("upper")
    );
}

#[test]
fn place_configured_feature_simple_block_respects_mossy_carpet_survival() {
    let retained = simple_block_retained("pale_moss_carpet", None);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut unsupported = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut unsupported, origin));
    assert!(unsupported.sets.borrow().is_empty());

    let mut supported = RecordingLevel::admitting();
    supported.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:stone"),
    );
    assert!(place_feature(&retained, &mut supported, origin));
    assert_eq!(supported.sets.borrow().len(), 1);
    assert!(block_is(
        supported.sets.borrow()[0].1,
        "minecraft:pale_moss_carpet"
    ));
}

#[test]
fn place_configured_feature_simple_block_schedules_a_tick() {
    let retained = simple_block_retained("stone", Some(true));
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.block_ticks.borrow(), [(origin, 1)]);
}

#[test]
fn place_configured_feature_simple_block_rule_without_fallback_returns_false() {
    let json = r#"{"type":"minecraft:simple_block","config":{"to_place":{"type":"rule_based_state_provider","rules":[]}}}"#;
    let retained = retained_from_json(json.as_bytes());
    let mut level = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut level, origin()));
    assert!(level.sets.borrow().is_empty());
}

fn prepare_dirt_floor(level: &mut RecordingLevel, origin: BlockPos) {
    let dirt = default_state("minecraft:dirt");
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), dirt);
        }
    }
}
fn still_water() -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("level".to_string(), "0".to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:water", &properties).unwrap()
}
fn nether_forest_json(state: &str, spread_width: i32, spread_height: i32) -> String {
    format!(
        r#"{{"type":"minecraft:nether_forest_vegetation","config":{{"state_provider":{{"type":"simple_state_provider","state":{{"Name":"{state}"}}}},"spread_width":{spread_width},"spread_height":{spread_height}}}}}"#
    )
}
fn nether_forest_retained(state: &str, spread_width: i32, spread_height: i32) -> ConfiguredFeature {
    retained_from_json(nether_forest_json(state, spread_width, spread_height).as_bytes())
}
fn bamboo_retained(probability: &str) -> ConfiguredFeature {
    retained_from_json(
        format!(r#"{{"type":"minecraft:bamboo","config":{{"probability":{probability}}}}}"#)
            .as_bytes(),
    )
}
fn seagrass_retained(probability: &str) -> ConfiguredFeature {
    retained_from_json(
        format!(r#"{{"type":"minecraft:seagrass","config":{{"probability":{probability}}}}}"#)
            .as_bytes(),
    )
}
fn prepare_ocean_floor(level: &mut RecordingLevel, origin: BlockPos) {
    let sand = default_state("minecraft:sand");
    let water = still_water();
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), sand);
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y, origin.z + dz), water);
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y + 1, origin.z + dz), water);
        }
    }
}
#[test]
fn place_configured_feature_nether_forest_vegetation_requires_nylium() {
    let retained = nether_forest_retained("nether_sprouts", 1, 1);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut dirt = RecordingLevel::admitting();
    dirt.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:dirt"),
    );
    assert!(!place_feature(&retained, &mut dirt, origin));
    assert!(dirt.sets.borrow().is_empty());
}
#[test]
fn place_configured_feature_nether_forest_vegetation_places_on_nylium() {
    let retained = nether_forest_retained("nether_sprouts", 1, 1);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sprouts = default_state("minecraft:nether_sprouts");
    let mut level = RecordingLevel::admitting();
    prepare_nylium_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.sets.borrow(), [(origin, sprouts, UPDATE_CLIENTS)]);
}
#[test]
fn place_configured_feature_nether_forest_vegetation_skips_cells_that_cannot_survive() {
    let retained = nether_forest_retained("nether_sprouts", 3, 1);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sprouts = default_state("minecraft:nether_sprouts");
    let stone = default_state("minecraft:stone");
    let mut level = RecordingLevel::admitting();
    prepare_nylium_floor(&mut level, origin);
    for dx in -2..=2 {
        for dz in -2..=2 {
            if dx != 0 || dz != 0 {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, origin.y, origin.z + dz), stone);
            }
        }
    }
    place_feature(&retained, &mut level, origin);
    assert!(level.sets.borrow().iter().all(|(pos, state, flags)| {
        *pos == origin && *state == sprouts && *flags == UPDATE_CLIENTS
    }));
}
#[test]
fn place_configured_feature_bamboo_places_on_dirt() {
    let retained = bamboo_retained("0.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_dirt_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(pos, state, flags)| pos.x == origin.x
        && pos.z == origin.z
        && block_is(*state, "minecraft:bamboo")
        && *flags == UPDATE_CLIENTS));
    assert!(block_is(level.get_block(origin), "minecraft:bamboo"));
}
#[test]
fn place_configured_feature_bamboo_returns_false_when_origin_is_occupied() {
    let retained = bamboo_retained("0.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}
#[test]
fn place_configured_feature_bamboo_returns_true_without_placing_when_it_cannot_survive() {
    let retained = bamboo_retained("0.0");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin()));
    assert!(level.sets.borrow().is_empty());
}
#[test]
fn place_configured_feature_bamboo_probability_one_places_podzol() {
    let retained = bamboo_retained("1.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_dirt_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().iter().any(|(pos, state, flags)| {
        pos.y == origin.y - 1 && block_is(*state, "minecraft:podzol") && *flags == UPDATE_CLIENTS
    }));
}
#[test]
fn place_configured_feature_seagrass_places_short_on_sand() {
    let retained = seagrass_retained("0.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_ocean_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 1);
    assert!(sets.iter().all(|(pos, state, flags)| pos.y == origin.y
        && block_is(*state, "minecraft:seagrass")
        && *flags == UPDATE_CLIENTS));
}
#[test]
fn place_configured_feature_seagrass_probability_one_places_tall() {
    let retained = seagrass_retained("1.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_ocean_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 2);
    assert!(sets.iter().all(|(_pos, state, flags)| {
        block_is(*state, "minecraft:tall_seagrass") && *flags == UPDATE_CLIENTS
    }));
    assert!(sets.iter().any(|(pos, state, _)| {
        pos.y == origin.y
            && vanilla_block_state_identity(*state)
                .is_some_and(|id| id.properties.get("half").map(String::as_str) == Some("lower"))
    }));
    assert!(sets.iter().any(|(pos, state, _)| {
        pos.y == origin.y + 1
            && vanilla_block_state_identity(*state)
                .is_some_and(|id| id.properties.get("half").map(String::as_str) == Some("upper"))
    }));
}
#[test]
fn place_configured_feature_seagrass_returns_false_when_ocean_floor_is_not_water() {
    let retained = seagrass_retained("0.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    let dirt = default_state("minecraft:dirt");
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), dirt);
        }
    }
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}
#[test]
fn place_configured_feature_seagrass_returns_false_when_it_cannot_survive() {
    let retained = seagrass_retained("0.0");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    let magma = default_state("minecraft:magma_block");
    let water = still_water();
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), magma);
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y, origin.z + dz), water);
        }
    }
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}
