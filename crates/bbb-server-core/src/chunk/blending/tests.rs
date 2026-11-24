use std::collections::BTreeMap;
use std::sync::Arc;

use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_STONE_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::{
    generation_state_chunk_document, Java26BlendingData, Java26ChunkContext, Java26ChunkData,
};
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkGenerationState, VANILLA_26_1_BIOME_REGISTRY_LEN,
};
use bbb_server_types::save::NamedCompound;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

use super::{
    height_to_offset, inside_index, old_density_column, outside_index, BlendingColumn,
    BlendingResolver,
};
use crate::chunk::noise_generator::{overworld_program, NoiseFinalDensityProgram};
use crate::chunk::ChunkStore;

const MIN_Y: i32 = -64;
const SECTION_COUNT: usize = 24;

fn program() -> Arc<NoiseFinalDensityProgram> {
    overworld_program(true, 0)
}

fn biome_id(program: &NoiseFinalDensityProgram, name: &str) -> i32 {
    (0..VANILLA_26_1_BIOME_REGISTRY_LEN as i32)
        .find(|&id| program.biome_name(id).as_deref() == Some(name))
        .unwrap()
}

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn registry_tags(leaves: &[&str], logs: &[&str]) -> Arc<RegistryTagSnapshot> {
    Arc::new(RegistryTagSnapshot::new([
        (
            "minecraft:block".to_owned(),
            "minecraft:leaves".to_owned(),
            leaves.iter().map(|member| (*member).to_owned()).collect(),
        ),
        (
            "minecraft:block".to_owned(),
            "minecraft:logs".to_owned(),
            logs.iter().map(|member| (*member).to_owned()).collect(),
        ),
    ]))
}

fn old_ground_density(store: &ChunkStore, state: i32) -> Vec<f64> {
    let column = ChunkColumn::new(MIN_Y, SECTION_COUNT, state, 0);
    old_density_column(
        &column,
        [0, 0],
        [MIN_Y, 15],
        2,
        MIN_Y,
        store.registry_tags(),
        (1, 0),
    )
    .unwrap()
}

fn blending_data() -> Java26BlendingData {
    blending_data_with_heights(None)
}

fn blending_data_with_heights(heights: Option<[f64; 16]>) -> Java26BlendingData {
    let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
    let column = ChunkColumn::new_with_biome_registry(
        MIN_Y,
        SECTION_COUNT,
        air,
        0,
        VANILLA_26_1_BIOME_REGISTRY_LEN,
    );
    let state = ChunkGenerationState::Carvers {
        postprocessing_positions: Vec::new(),
        carving_mask: Vec::new(),
    };
    let mut document: NamedCompound = generation_state_chunk_document(
        &column,
        0,
        0,
        &|_| Some("minecraft:plains".to_string()),
        &state,
        Some(&NoiseStructureChunkData::default().to_nbt()),
        &[],
    )
    .unwrap();
    let mut fields = vec![
        NbtSummaryEntry {
            name: "min_section".into(),
            value: NbtSummaryValue::Int(-4),
        },
        NbtSummaryEntry {
            name: "max_section".into(),
            value: NbtSummaryValue::Int(20),
        },
    ];
    if let Some(heights) = heights {
        fields.push(NbtSummaryEntry {
            name: "heights".into(),
            value: NbtSummaryValue::List(
                heights
                    .iter()
                    .map(|height| NbtSummaryValue::Double(height.to_bits()))
                    .collect(),
            ),
        });
    }
    document.entries_mut().push(NbtSummaryEntry {
        name: "blending_data".into(),
        value: NbtSummaryValue::Compound(fields),
    });
    Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: SECTION_COUNT,
        },
    )
    .unwrap()
    .blending_data()
    .cloned()
    .unwrap()
}

fn one_sample(biome: i32) -> Arc<BlendingColumn> {
    let mut biomes = vec![None; 16];
    biomes[3] = Some(vec![biome]);
    Arc::new(BlendingColumn {
        min_cell_x: 0,
        min_cell_z: 0,
        min_density_cell_y: 1,
        density_cell_count: 2,
        min_quart_y: 0,
        max_quart_y_exclusive: 1,
        heights: [f64::MAX; 16],
        densities: vec![None; 16],
        biomes,
    })
}

#[test]
fn blender_biomes_preserves_strict_first_tie_and_null_fallback() {
    let resolver = BlendingResolver {
        by_chunk: BTreeMap::new(),
        height_columns: vec![one_sample(11), one_sample(29)],
        density_columns: Vec::new(),
    };
    assert_eq!(resolver.blend(0, 0, 0), Some(11));
    assert_eq!(resolver.blend(28, 0, 0), None);
    assert_eq!(resolver.blend(0, 1, 0), None);
}

#[test]
fn blender_biomes_uses_restored_boundary_columns_for_fresh_biomes() {
    let program = program();
    let old_biome = biome_id(&program, "minecraft:desert");
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();
    store.try_load_column(0, 0).unwrap();
    let min_quart_y = store.min_y().div_euclid(4);
    let max_quart_y = min_quart_y + store.section_count() as i32 * 4;
    let old_column = store.column_mut(0, 0).unwrap();
    for quart_y in min_quart_y..max_quart_y {
        for local_x in 0..4 {
            for local_z in 0..4 {
                old_column
                    .set_noise_biome(local_x, quart_y, local_z, old_biome)
                    .unwrap();
            }
        }
    }
    store.loaded_mut((0, 0)).blending = Some(Some(blending_data()));
    store.generation_blending_catalog_complete = true;

    let fresh = store.try_load_column(1, 0).unwrap();
    assert_eq!(fresh.get_noise_biome(0, min_quart_y, 0), Some(old_biome));
    assert_eq!(store.blending_witness((1, 0)), Some(&None));
}

fn noise_sample_column(chunk: (i32, i32), height: f64, density: f64) -> Arc<BlendingColumn> {
    let mut heights = [f64::MAX; 16];
    heights[inside_index(0, 0)] = height;
    let mut densities = vec![None; 16];
    densities[inside_index(0, 0)] = Some(vec![density, density]);
    Arc::new(BlendingColumn {
        min_cell_x: chunk.0 * 4,
        min_cell_z: chunk.1 * 4,
        min_density_cell_y: 1,
        density_cell_count: 2,
        min_quart_y: 0,
        max_quart_y_exclusive: 1,
        heights,
        densities,
        biomes: vec![None; 16],
    })
}

fn noise_resolver(columns: &[(i32, i32, Arc<BlendingColumn>)]) -> BlendingResolver {
    BlendingResolver {
        by_chunk: columns
            .iter()
            .map(|(x, z, column)| {
                (
                    bbb_server_worldgen::noise::structure_starts::nbt::pack_chunk(*x, *z),
                    Arc::clone(column),
                )
            })
            .collect(),
        height_columns: columns
            .iter()
            .map(|(_, _, column)| Arc::clone(column))
            .collect(),
        density_columns: columns
            .iter()
            .map(|(_, _, column)| Arc::clone(column))
            .collect(),
    }
}

#[test]
fn blender_noise_preserves_direct_precedence_weighting_and_fallbacks() {
    let current = noise_sample_column((0, 0), 64.0, 4.0);
    let northwest = noise_sample_column((-1, -1), 96.0, -8.0);
    let resolver = noise_resolver(&[
        (0, 0, Arc::clone(&current)),
        (-1, -1, Arc::clone(&northwest)),
    ]);

    let (_, direct_offset) = resolver.blend_offset_and_factor_value(0, 0);
    assert_eq!(direct_offset.to_bits(), height_to_offset(64.0).to_bits());
    assert_eq!(resolver.blend_density_value(0, 8, 0, -1.0), 0.4);

    let weighted = noise_resolver(&[(0, 0, current)]);
    let (alpha, offset) = weighted.blend_offset_and_factor_value(4, 0);
    let raw_alpha = 1.0_f64 / 28.0;
    assert_eq!(
        alpha.to_bits(),
        (3.0 * raw_alpha * raw_alpha - 2.0 * raw_alpha * raw_alpha * raw_alpha).to_bits()
    );
    assert_eq!(offset.to_bits(), height_to_offset(64.0).to_bits());
    assert_eq!(
        weighted.blend_density_value(4, 8, 0, -0.2).to_bits(),
        (0.4_f64 + (1.0 / 3.0) * (-0.2 - 0.4)).to_bits()
    );
    assert_eq!(weighted.blend_offset_and_factor_value(112, 0), (1.0, 0.0));
    assert_eq!(weighted.blend_density_value(16, 80, 16, -0.75), -0.75);
}

#[test]
fn blender_noise_lazy_columns_preserve_saved_heights_and_read_density() {
    let program = program();
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();
    store.try_load_column(0, 0).unwrap();
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    let old = store.column_mut(0, 0).unwrap();
    for y in MIN_Y..=31 {
        old.set_block_state(15, y, 0, stone).unwrap();
    }
    let mut saved_heights = [f64::MAX; 16];
    saved_heights[outside_index(4, 0)] = 31.25;
    store.loaded_mut((0, 0)).blending = Some(Some(blending_data_with_heights(Some(saved_heights))));
    store.generation_blending_catalog_complete = true;

    let resolver = store.blending_resolver((1, 0)).unwrap();
    let (alpha, offset) = resolver.blend_offset_and_factor_value(16, 0);
    assert_eq!(alpha, 0.0);
    assert_eq!(offset.to_bits(), height_to_offset(31.25).to_bits());
    assert_ne!(resolver.blend_density_value(16, 24, 0, f64::MAX), f64::MAX);
}

#[test]
fn blender_old_ground_density_uses_each_installed_registry_tag_snapshot() {
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program(), false).unwrap();
    let leaves = state("minecraft:oak_leaves");
    let logs = state("minecraft:oak_log");
    let air = state("minecraft:air");
    let brown_mushroom = state("minecraft:brown_mushroom_block");
    let red_mushroom = state("minecraft:red_mushroom_block");

    assert_eq!(old_ground_density(&store, leaves), vec![-1.0, -1.0]);
    assert_eq!(old_ground_density(&store, logs), vec![-1.0, -1.0]);

    store.install_registry_tags(registry_tags(&[], &[]));
    assert_eq!(old_ground_density(&store, leaves), vec![1.0, -1.0]);
    assert_eq!(old_ground_density(&store, logs), vec![1.0, -1.0]);
    assert_eq!(old_ground_density(&store, air), vec![-1.0, -1.0]);
    assert_eq!(old_ground_density(&store, brown_mushroom), vec![-1.0, -1.0]);
    assert_eq!(old_ground_density(&store, red_mushroom), vec![-1.0, -1.0]);

    store.install_registry_tags(registry_tags(&["minecraft:oak_leaves"], &[]));
    assert_eq!(old_ground_density(&store, leaves), vec![-1.0, -1.0]);
    assert_eq!(old_ground_density(&store, logs), vec![1.0, -1.0]);

    store.install_registry_tags(registry_tags(&[], &["minecraft:oak_log"]));
    assert_eq!(old_ground_density(&store, leaves), vec![1.0, -1.0]);
    assert_eq!(old_ground_density(&store, logs), vec![-1.0, -1.0]);
}

#[test]
fn blender_noise_is_bound_before_production_biomes_and_density_fill() {
    let program = program();
    let unblended = program.generate_fresh_carved_chunk(1, 0).unwrap();
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(Arc::clone(&program), false)
            .unwrap();
    store.try_load_column(0, 0).unwrap();
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    let old = store.column_mut(0, 0).unwrap();
    for y in MIN_Y..=96 {
        for z in 0..16 {
            old.set_block_state(15, y, z, stone).unwrap();
        }
    }
    store.loaded_mut((0, 0)).blending = Some(Some(blending_data()));
    store.generation_blending_catalog_complete = true;
    let blended = store.try_load_column(1, 0).unwrap();

    assert_ne!(blended, unblended.column());
    assert_eq!(store.blending_witness((1, 0)), Some(&None));
}
