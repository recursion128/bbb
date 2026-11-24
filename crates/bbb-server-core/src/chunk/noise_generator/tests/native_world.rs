//! Whole-world checks for the native worldgen streams: the same seed always
//! generates the same chunks, and a fixed region holds the expected terrain,
//! ores, caves, fluids and biomes.

use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_data::{ChunkColumn, VANILLA_26_1_BIOME_REGISTRY_LEN};

use super::*;
use crate::chunk::ChunkStore;

fn store(program: Arc<super::super::NoiseFinalDensityProgram>) -> ChunkStore {
    ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap()
}

fn full_columns(store: &mut ChunkStore, centers: &[(i32, i32)]) -> Vec<ChunkColumn> {
    store.try_ensure_noise_full_columns(centers).unwrap();
    centers
        .iter()
        .map(|&(x, z)| store.column(x, z).unwrap().clone())
        .collect()
}

fn block_name(state: i32) -> &'static str {
    vanilla_block_state_identity(state).unwrap().name.as_str()
}

fn biome_ids(program: &super::super::NoiseFinalDensityProgram, names: &[&str]) -> BTreeSet<i32> {
    let len = i32::try_from(VANILLA_26_1_BIOME_REGISTRY_LEN).unwrap();
    (0..len)
        .filter(|id| {
            program
                .biome_name(*id)
                .is_some_and(|name| names.contains(&name.as_str()))
        })
        .collect()
}

#[test]
fn the_same_seed_generates_identical_chunks_in_any_order() {
    let chunks = [(0, 0), (7, -3), (-5, 9)];
    let reversed: Vec<_> = chunks.iter().rev().copied().collect();
    let first = full_columns(&mut store(fresh_overworld_program(true, 0)), &chunks);
    let second = full_columns(&mut store(fresh_overworld_program(true, 0)), &chunks);
    let mut backwards = full_columns(&mut store(fresh_overworld_program(true, 0)), &reversed);
    backwards.reverse();
    assert!(
        first == second,
        "regenerating the same seed changed a chunk"
    );
    assert!(first == backwards, "generation order changed a chunk");
    let other = full_columns(&mut store(fresh_overworld_program(true, 1)), &chunks);
    assert!(first != other, "seeds 0 and 1 generated the same chunks");
}

#[test]
fn a_fixed_region_has_layers_ores_caves_and_fluids() {
    let centers: Vec<_> = (-4..4).flat_map(|x| (-4..4).map(move |z| (x, z))).collect();
    let columns = full_columns(&mut store(overworld_program(true, 0)), &centers);
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut deep_counts = BTreeMap::<&str, usize>::new();
    let mut surface = Vec::new();
    for column in &columns {
        let min_y = column.min_y();
        let max_y = min_y + column.section_count() as i32 * 16;
        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(
                    block_name(column.get_block_state(x, min_y, z).unwrap()),
                    "minecraft:bedrock"
                );
                let mut top = None;
                for y in min_y..max_y {
                    let name = block_name(column.get_block_state(x, y, z).unwrap());
                    *counts.entry(name).or_default() += 1;
                    if y < -8 {
                        *deep_counts.entry(name).or_default() += 1;
                    }
                    if name != "minecraft:air" {
                        top = Some(y);
                    }
                }
                surface.push(top.unwrap());
            }
        }
    }
    let count = |name: &str| counts.get(name).copied().unwrap_or(0);
    for ore in [
        "coal", "iron", "copper", "gold", "redstone", "lapis", "diamond",
    ] {
        let found = count(&format!("minecraft:{ore}_ore"))
            + count(&format!("minecraft:deepslate_{ore}_ore"));
        assert!(found > 0, "no {ore} ore in the region");
    }
    assert!(deep_counts.get("minecraft:deepslate").copied().unwrap_or(0) > 100_000);
    assert!(count("minecraft:stone") > 100_000);
    assert!(
        deep_counts.get("minecraft:air").copied().unwrap_or(0)
            + deep_counts.get("minecraft:cave_air").copied().unwrap_or(0)
            > 1_000,
        "no caves below y = -8"
    );
    assert!(count("minecraft:water") > 0);
    assert!(
        deep_counts.get("minecraft:lava").copied().unwrap_or(0) > 0,
        "no lava aquifer"
    );
    let mean = surface.iter().map(|y| f64::from(*y)).sum::<f64>() / surface.len() as f64;
    let spread = surface.iter().max().unwrap() - surface.iter().min().unwrap();
    assert!((50.0..130.0).contains(&mean), "mean surface y {mean}");
    assert!(spread >= 4, "flat region: surface spread {spread}");
}

#[test]
fn the_overworld_has_varied_biomes_and_emeralds_in_its_mountains() {
    let program = overworld_program(true, 0);
    let mut biomes = BTreeSet::new();
    for quart_x in (-2048..2048).step_by(16) {
        for quart_z in (-2048..2048).step_by(16) {
            biomes.insert(program.uncached_noise_biome(quart_x, 16, quart_z).unwrap());
        }
    }
    let names: BTreeSet<_> = biomes
        .iter()
        .map(|id| program.biome_name(*id).unwrap())
        .collect();
    assert!(names.len() >= 30, "{names:?}");
    for expected in [
        "minecraft:plains",
        "minecraft:forest",
        "minecraft:desert",
        "minecraft:ocean",
        "minecraft:deep_ocean",
        "minecraft:river",
        "minecraft:jungle",
        "minecraft:dark_forest",
        "minecraft:taiga",
        "minecraft:savanna",
    ] {
        assert!(names.contains(expected), "no {expected} in {names:?}");
    }

    let mut store = store(program.clone());
    let mountains = biome_ids(
        &program,
        &[
            "minecraft:jagged_peaks",
            "minecraft:stony_peaks",
            "minecraft:frozen_peaks",
        ],
    );
    let (found, _) = store
        .find_closest_biome_3d(BlockPos { x: 0, y: 120, z: 0 }, &mountains, 6_400, 32, 64)
        .expect("a mountain peak within 6400 blocks");
    let center = (found.x >> 4, found.z >> 4);
    let centers: Vec<_> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| (center.0 + x, center.1 + z)))
        .collect();
    let columns = full_columns(&mut store, &centers);
    let emeralds = columns
        .iter()
        .flat_map(|column| {
            let min_y = column.min_y();
            let max_y = min_y + column.section_count() as i32 * 16;
            (min_y..max_y).flat_map(move |y| {
                (0..16).flat_map(move |x| {
                    (0..16).map(move |z| column.get_block_state(x, y, z).unwrap())
                })
            })
        })
        .filter(|state| block_name(*state).ends_with("emerald_ore"))
        .count();
    assert!(emeralds > 0, "no emerald ore around the peak at {found:?}");
}

#[test]
fn locate_finds_villages_and_strongholds_near_spawn() {
    let program = overworld_program(true, 0);
    let mut store = store(program);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let villages: Vec<String> = ["plains", "desert", "savanna", "snowy", "taiga"]
        .iter()
        .map(|kind| format!("minecraft:village_{kind}"))
        .collect();
    let (village, id) = store
        .find_nearest_map_structure(origin, &villages, 100, false)
        .unwrap()
        .expect("a village within 100 placement cells");
    assert!(villages.contains(&id));
    assert!(village.x.abs().max(village.z.abs()) < 100 * 34 * 16);
    let (stronghold, id) = store
        .find_nearest_map_structure(origin, &["minecraft:stronghold".to_string()], 100, false)
        .unwrap()
        .expect("a stronghold in the first ring");
    assert_eq!(id, "minecraft:stronghold");
    let distance = f64::from(stronghold.x).hypot(f64::from(stronghold.z));
    assert!(
        (1_280.0..3_000.0).contains(&distance),
        "first-ring stronghold at {distance}"
    );
    let monument = store
        .find_nearest_map_structure(origin, &["minecraft:monument".to_string()], 100, false)
        .unwrap();
    assert!(monument.is_some(), "no ocean monument within 100 cells");
}
