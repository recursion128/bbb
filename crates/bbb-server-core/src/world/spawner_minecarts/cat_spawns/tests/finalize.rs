use super::*;
use bbb_protocol::NbtSummaryEntry;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
use std::sync::Arc;

fn configured_config(eligible: bool) -> (crate::world::ServerWorldConfig, String, String) {
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let variants = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:cat_variant")
        .unwrap();
    assert!(variants.elements.len() >= 3);
    let fuzzy_variant = variants.elements[0].id.clone();
    for (index, variant) in variants.elements.iter_mut().enumerate() {
        let conditions = if eligible && index < 3 {
            let biome = ["minecraft:desert", "minecraft:plains", "minecraft:forest"][index];
            serde_json::json!([{"priority": 1, "condition": {"type": "minecraft:biome", "biomes": biome}}])
        } else {
            serde_json::json!([])
        };
        let mut json: serde_json::Value = serde_json::from_slice(&variant.source_json).unwrap();
        json["spawn_conditions"] = conditions;
        variant.source_json = serde_json::to_vec(&json).unwrap().into();
        variant.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&variant.source_json).unwrap());
        variant.source_json = serde_json::to_vec(&variant.json()).unwrap().into();
        variant.codec_value = None;
    }
    let sounds = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:cat_sound_variant")
        .unwrap();
    let classic = sounds
        .elements
        .iter()
        .find(|element| element.id == "minecraft:classic")
        .unwrap()
        .clone();
    let sound = classic.id.clone();
    sounds.elements = vec![classic];
    (config, fuzzy_variant, sound)
}

fn configured_world(eligible: bool) -> (ServerWorld, String, String) {
    let (config, variant, sound) = configured_config(eligible);
    (ServerWorld::new(config).unwrap(), variant, sound)
}

fn set_biome(world: &mut ServerWorld, quart: (i32, i32, i32), name: &str) {
    let biome = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", name)
        .unwrap();
    let (x, y, z) = quart;
    let chunks = &mut world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks;
    chunks
        .try_ensure_served_full_column(x >> 2, z >> 2)
        .unwrap();
    assert!(chunks
        .column_mut(x >> 2, z >> 2)
        .unwrap()
        .set_noise_biome(x & 3, y, z & 3, biome)
        .unwrap()
        .is_some());
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    &fields
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

#[test]
fn cat_finalize_selects_fuzzy_zero_instead_of_raw_zero_or_final_position() {
    let (mut world, variant, sound) = configured_world(true);
    let seed = (0..100)
        .find(|&seed| fuzzy_biome_quart_position(biome_zoom_seed(seed), 0, 0, 0) != (0, 0, 0))
        .unwrap();
    world.cold.seed = seed;
    let fuzzy = fuzzy_biome_quart_position(biome_zoom_seed(seed), 0, 0, 0);
    let target = BlockPos {
        x: 160,
        y: 80,
        z: 160,
    };
    let target_fuzzy =
        fuzzy_biome_quart_position(biome_zoom_seed(seed), target.x, target.y, target.z);
    set_biome(&mut world, (0, 0, 0), "minecraft:plains");
    set_biome(&mut world, fuzzy, "minecraft:desert");
    set_biome(
        &mut world,
        (target.x >> 2, target.y >> 2, target.z >> 2),
        "minecraft:forest",
    );
    set_biome(&mut world, target_fuzzy, "minecraft:forest");
    assert_eq!(
        world.spawn_custom_cat(&world.require_level_id(&key()), target, false),
        Ok(true)
    );
    let level = world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap();
    let id = level.entities.ids()[0];
    let NbtSummaryValue::Compound(fields) = level.entities.command_data(id).unwrap() else {
        panic!("compound")
    };
    assert_eq!(
        field(&fields, "variant"),
        &NbtSummaryValue::String(variant.into())
    );
    assert_eq!(
        field(&fields, "sound_variant"),
        &NbtSummaryValue::String(sound.into())
    );
    assert_eq!(
        level.entities.get(id).unwrap().position(),
        Vec3d {
            x: 160.5,
            y: 80.0,
            z: 160.5
        }
    );
}

#[test]
fn cat_finalize_no_eligible_variant_falls_back_to_black() {
    let (mut world, _, sound) = configured_world(false);
    assert_eq!(
        world.spawn_custom_cat(
            &world.require_level_id(&key()),
            BlockPos {
                x: 32,
                y: 80,
                z: 32
            },
            false
        ),
        Ok(true)
    );
    let level = world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap();
    let id = level.entities.ids()[0];
    let NbtSummaryValue::Compound(fields) = level.entities.command_data(id).unwrap() else {
        panic!("compound")
    };
    assert_eq!(
        field(&fields, "variant"),
        &NbtSummaryValue::String("minecraft:black".into())
    );
    assert_eq!(
        field(&fields, "sound_variant"),
        &NbtSummaryValue::String(sound.into())
    );
}

#[test]
fn cat_moon_selector_reads_raw_zero_while_difficulty_reads_final_position() {
    let (mut config, _, sound) = configured_config(false);
    config.difficulty = Difficulty::Normal;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let mut expected_variant = String::new();
    for registry in &mut startup.registries.registries {
        for (index, element) in registry.elements.iter_mut().enumerate() {
            let mut value: serde_json::Value =
                serde_json::from_slice(&element.source_json).unwrap();
            match registry.registry_id.as_str() {
                "minecraft:dimension_type" if element.id == "minecraft:overworld" => {
                    value["timelines"] = serde_json::json!([]);
                }
                "minecraft:worldgen/biome" => {
                    let phase = match element.id.as_str() {
                        "minecraft:plains" => "new_moon",
                        "minecraft:desert" => "full_moon",
                        _ => "third_quarter",
                    };
                    value["attributes"]["minecraft:visual/moon_phase"] = serde_json::json!(phase);
                }
                "minecraft:cat_variant" if index < 3 => {
                    value["spawn_conditions"] = serde_json::json!([{"priority": 1, "condition": {"type": "minecraft:moon_brightness", "range": ([0.5, 1.0, 0.0][index])}}]);
                    if index == 2 {
                        expected_variant = element.id.clone();
                    }
                }
                _ => continue,
            }
            element.source_json = serde_json::to_vec(&value).unwrap().into();
            element.source = Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(value)).unwrap(),
                )
                .unwrap(),
            );
            element.codec_value = None;
        }
    }
    let mut world = ServerWorld::new(config).unwrap();
    let seed = (0..100)
        .find(|&seed| fuzzy_biome_quart_position(biome_zoom_seed(seed), 0, 0, 0) != (0, 0, 0))
        .unwrap();
    world.cold.seed = seed;
    let fuzzy = fuzzy_biome_quart_position(biome_zoom_seed(seed), 0, 0, 0);
    let target = BlockPos {
        x: 160,
        y: 80,
        z: 160,
    };
    set_biome(&mut world, fuzzy, "minecraft:desert");
    set_biome(&mut world, (0, 0, 0), "minecraft:plains");
    set_biome(&mut world, (40, 20, 40), "minecraft:forest");
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[clock];
    state.total_ticks = 1_600_000;
    world.set_world_clock_state_for_test(clock, state);
    world
        .with_spawn_runtime(&world.require_level_id(&key()), &[], |runtime, _, _| {
            let zero =
                cat_context::prepare(runtime, BlockPos { x: 0, y: 0, z: 0 }, "minecraft:desert")?;
            assert_eq!(zero.special_multiplier, 0.0);
            assert_eq!(
                runtime.spawn_environment.special_multiplier(
                    runtime.chunks,
                    target,
                    runtime.difficulty,
                    runtime.overworld_clock_time
                ),
                Some(0.125)
            );
            Ok::<_, String>(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        world.spawn_custom_cat(&world.require_level_id(&key()), target, false),
        Ok(true)
    );
    let level = world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap();
    let NbtSummaryValue::Compound(fields) = level
        .entities
        .command_data(level.entities.ids()[0])
        .unwrap()
    else {
        panic!("compound")
    };
    assert_eq!(
        field(&fields, "variant"),
        &NbtSummaryValue::String(expected_variant.into())
    );
    assert_eq!(
        field(&fields, "sound_variant"),
        &NbtSummaryValue::String(sound.into())
    );
}
