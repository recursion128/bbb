//! Headless CPU baselines for terrain snapshot materialization and dirty-owner meshing.
//!
//! GPU buffer creation is intentionally not benchmarked here: it requires a
//! concrete adapter/device and belongs to the fixed playable performance
//! fixture. These cases isolate immutable CPU input, historical 1/8/32 full
//! chunk work, and production-equivalent section-dirty workloads.

use std::{hint::black_box, sync::Arc, time::Duration};

use bbb_client_assets::{
    BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier,
};
use bbb_client_render::{
    scene::terrain_textures::{TerrainResourceSource, TerrainTextureState},
    terrain::{
        build_terrain_section_meshes,
        fixture::{TerrainCell, TerrainChunkFixture, TerrainFixtureWorld},
        TerrainBlockRenderTable, TerrainCardinalLighting, TerrainChunkSnapshot,
        TerrainMaterialClass, TerrainSectionMeshLayers, TerrainTextureAtlas, TerrainTransparency,
    },
};
use bbb_protocol::{
    block_state_types::{vanilla_block_default_state_id, vanilla_block_state_name},
    block_types::vanilla_block_type_for_resource_id,
};
use bbb_render_types::{
    PaletteDomain, PaletteKind, PalettedContainerData, TerrainBlockRenderContext,
    TerrainChunkSource, TerrainFluidKind, TerrainFluidState,
    TerrainMaterialClass as RenderTerrainMaterialClass, TerrainSectionLightRef, TerrainSectionView,
    LIGHT_ARRAY_BYTES,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

const CHANGED_OWNER_COUNTS: [usize; 3] = [1, 8, 32];
const SNAPSHOT_HEIGHT: usize = 64;

#[derive(Clone, Copy)]
struct DirtyWorkload {
    name: &'static str,
    chunks: usize,
    section_ys: &'static [i32],
}

const INTERIOR_SECTION: &[i32] = &[0];
const VERTICAL_BOUNDARY_SECTIONS: &[i32] = &[-1, 0];
const LIGHT_NEIGHBOR_SECTIONS: &[i32] = &[-1, 0, 1];
const FULL_HEIGHT_SECTIONS: &[i32] = &[-1, 0, 1, 2];
const CAMERA: [f32; 3] = [8.0, 72.0, 8.0];

const DIRTY_WORKLOADS: [DirtyWorkload; 11] = [
    DirtyWorkload {
        name: "unchanged",
        chunks: 0,
        section_ys: &[],
    },
    DirtyWorkload {
        name: "interior_block",
        chunks: 1,
        section_ys: INTERIOR_SECTION,
    },
    DirtyWorkload {
        name: "vertical_section_boundary",
        chunks: 1,
        section_ys: VERTICAL_BOUNDARY_SECTIONS,
    },
    DirtyWorkload {
        name: "horizontal_chunk_edge",
        chunks: 2,
        section_ys: INTERIOR_SECTION,
    },
    DirtyWorkload {
        name: "horizontal_chunk_corner",
        chunks: 4,
        section_ys: INTERIOR_SECTION,
    },
    DirtyWorkload {
        name: "standalone_light_section",
        chunks: 9,
        section_ys: LIGHT_NEIGHBOR_SECTIONS,
    },
    DirtyWorkload {
        name: "full_chunk_or_biome_load",
        chunks: 9,
        section_ys: FULL_HEIGHT_SECTIONS,
    },
    DirtyWorkload {
        name: "chunk_unload_neighbors",
        chunks: 8,
        section_ys: FULL_HEIGHT_SECTIONS,
    },
    DirtyWorkload {
        name: "interior_block_burst_8",
        chunks: 8,
        section_ys: INTERIOR_SECTION,
    },
    DirtyWorkload {
        name: "interior_block_burst_32",
        chunks: 32,
        section_ys: INTERIOR_SECTION,
    },
    DirtyWorkload {
        name: "resource_reload_32",
        chunks: 32,
        section_ys: FULL_HEIGHT_SECTIONS,
    },
];

fn representative_cells() -> Vec<TerrainCell> {
    let mut cells = vec![TerrainCell::EMPTY; 16 * SNAPSHOT_HEIGHT * 16];
    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                let index = (y * 16 + z) * 16 + x;
                cells[index] = match (x + y + z) % 19 {
                    0 => TerrainCell::with_texture(TerrainMaterialClass::Translucent, 0)
                        .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6]),
                    1 | 2 => TerrainCell::with_texture(TerrainMaterialClass::Cutout, 0),
                    _ => TerrainCell::with_texture(TerrainMaterialClass::Opaque, 0),
                };
            }
        }
    }
    cells
}

fn representative_chunk(index: usize) -> TerrainChunkFixture {
    TerrainChunkFixture::new(
        index as i32,
        0,
        -16,
        SNAPSHOT_HEIGHT,
        representative_cells(),
    )
}

fn dense_section_chunk(index: usize) -> TerrainChunkFixture {
    let mut cells = vec![TerrainCell::EMPTY; 16 * SNAPSHOT_HEIGHT * 16];
    for y in 0..SNAPSHOT_HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                let cell_index = (y * 16 + z) * 16 + x;
                cells[cell_index] = match (x + y + z) % 19 {
                    0 => TerrainCell::with_texture(TerrainMaterialClass::Translucent, 0)
                        .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6]),
                    1 | 2 => TerrainCell::with_texture(TerrainMaterialClass::Cutout, 0),
                    _ => TerrainCell::with_texture(TerrainMaterialClass::Opaque, 0),
                };
            }
        }
    }
    TerrainChunkFixture::new(index as i32, 0, -16, SNAPSHOT_HEIGHT, cells)
}

fn fixture_world(chunk: fn(usize) -> TerrainChunkFixture) -> TerrainFixtureWorld {
    let chunks = (0..*CHANGED_OWNER_COUNTS.last().unwrap())
        .map(chunk)
        .collect::<Vec<_>>();
    TerrainFixtureWorld::new(&chunks, TerrainTextureAtlas::unit())
}

/// Each chunk meshes alone, as the baseline did.
fn build_isolated(
    world: &TerrainFixtureWorld,
    snapshot: &TerrainChunkSnapshot,
    camera_position: [f32; 3],
    section_ys: &[i32],
) -> TerrainSectionMeshLayers {
    build_terrain_section_meshes(
        snapshot,
        std::iter::empty(),
        &world.table,
        camera_position,
        section_ys,
    )
}

fn benchmark_configuration() -> Criterion {
    Criterion::default()
        .without_plots()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
}

fn bench_snapshot_materialization(c: &mut Criterion) {
    let cells: Vec<_> = representative_cells()
        .into_iter()
        .map(|cell| (cell.material as u32, cell.light))
        .collect();
    let mut group = c.benchmark_group("terrain_cpu/snapshot_materialization");
    for count in CHANGED_OWNER_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let snapshots = (0..count)
                    .map(|index| {
                        TerrainChunkSnapshot::from_cells(
                            index as i32,
                            0,
                            -16,
                            SNAPSHOT_HEIGHT,
                            cells.iter().copied(),
                        )
                    })
                    .collect::<Vec<_>>();
                black_box(snapshots)
            });
        });
    }
    group.finish();
}

fn bench_changed_owner_mesh_build(c: &mut Criterion) {
    let world = fixture_world(representative_chunk);
    let mut group = c.benchmark_group("terrain_cpu/changed_owner_mesh_build");
    for count in CHANGED_OWNER_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let layers = world.snapshots[..count]
                    .iter()
                    .map(|snapshot| {
                        build_isolated(&world, snapshot, black_box(CAMERA), FULL_HEIGHT_SECTIONS)
                    })
                    .collect::<Vec<_>>();
                black_box(layers)
            });
        });
    }
    group.finish();
}

/// The ids match the whole-chunk baseline, while each case now performs the
/// exact section set selected by world invalidation. Full-height load, unload,
/// biome, and resource-reload cases deliberately retain whole-height work.
fn bench_dirty_mesh_workloads(c: &mut Criterion) {
    let world = fixture_world(dense_section_chunk);
    let retained = build_isolated(&world, &world.snapshots[0], CAMERA, FULL_HEIGHT_SECTIONS);
    let mut group = c.benchmark_group("terrain_cpu/dirty_mesh_workload");
    for workload in DIRTY_WORKLOADS {
        group.bench_function(workload.name, |b| {
            b.iter(|| {
                if workload.chunks == 0 {
                    return black_box(retained.source_sections);
                }
                let layers = world.snapshots[..workload.chunks]
                    .iter()
                    .map(|snapshot| {
                        build_isolated(&world, snapshot, black_box(CAMERA), workload.section_ys)
                    })
                    .collect::<Vec<_>>();
                black_box(layers.len())
            });
        });
    }
    group.finish();
}

fn bench_translucent_camera_resort_input(c: &mut Criterion) {
    let world = fixture_world(representative_chunk);
    let mut camera_x = 0.0_f32;
    c.bench_function("terrain_cpu/translucent_camera_resort_input/1_owner", |b| {
        b.iter(|| {
            camera_x = if camera_x == 0.0 { 32.0 } else { 0.0 };
            black_box(build_isolated(
                &world,
                &world.snapshots[0],
                black_box([camera_x, 72.0, 8.0]),
                FULL_HEIGHT_SECTIONS,
            ))
        });
    });
}

#[derive(Debug)]
struct OceanForestChunkSource {
    block_sections: Vec<PalettedContainerData>,
    biomes: PalettedContainerData,
    sky_light: [u8; LIGHT_ARRAY_BYTES],
}

impl TerrainChunkSource for OceanForestChunkSource {
    fn section(&self, section_index: usize) -> Option<TerrainSectionView<'_>> {
        let block_states = self.block_sections.get(section_index)?;
        Some(TerrainSectionView {
            block_states,
            biomes: Some(&self.biomes),
            sky_light: TerrainSectionLightRef::Packed(&self.sky_light),
            block_light: TerrainSectionLightRef::Uniform(0),
        })
    }
}

fn dense_bench_base_state_facts(
    block_state_id: i32,
) -> Option<(RenderTerrainMaterialClass, Option<TerrainFluidState>)> {
    let name = vanilla_block_state_name(block_state_id)?;
    let facts = match name {
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" => {
            (RenderTerrainMaterialClass::Empty, None)
        }
        "minecraft:water" => (
            RenderTerrainMaterialClass::Fluid,
            Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false)),
        ),
        "minecraft:lava" => (
            RenderTerrainMaterialClass::Fluid,
            Some(TerrainFluidState::new(TerrainFluidKind::Lava, 8, false)),
        ),
        "minecraft:oak_leaves" | "minecraft:short_grass" => {
            (RenderTerrainMaterialClass::Cutout, None)
        }
        _ => (RenderTerrainMaterialClass::Opaque, None),
    };
    Some(facts)
}

fn single_biome_container(biome_id: i32) -> PalettedContainerData {
    PalettedContainerData {
        domain: PaletteDomain::Biomes,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![biome_id],
        packed_data: Vec::new(),
        entry_count: 64,
    }
}

fn split_x_biome_container(left_biome_id: i32, right_biome_id: i32) -> PalettedContainerData {
    let mut packed = 0_u64;
    for y in 0..4 {
        for z in 0..4 {
            for x in 2..4 {
                let index = (y << 4) | (z << 2) | x;
                packed |= 1_u64 << index;
            }
        }
    }
    PalettedContainerData {
        domain: PaletteDomain::Biomes,
        bits_per_entry: 1,
        palette_kind: PaletteKind::Local,
        palette_global_ids: vec![left_biome_id, right_biome_id],
        packed_data: vec![packed as i64],
        entry_count: 64,
    }
}

fn build_ocean_forest_world() -> (
    TerrainBlockRenderTable,
    TerrainChunkSnapshot,
    Vec<TerrainChunkSnapshot>,
) {
    let biome_colors = BiomeColorCatalog::new([
        BiomeColorProfile {
            id: 0,
            name: "minecraft:ocean".to_string(),
            temperature: 0.5,
            temperature_modifier: BiomeTemperatureModifier::None,
            downfall: 0.5,
            has_precipitation: true,
            grass_color: None,
            foliage_color: None,
            dry_foliage_color: None,
            water_color: Some([0x3F, 0x76, 0xE4]),
            grass_color_modifier: GrassColorModifier::None,
        },
        BiomeColorProfile {
            id: 1,
            name: "minecraft:forest".to_string(),
            temperature: 0.7,
            temperature_modifier: BiomeTemperatureModifier::None,
            downfall: 0.8,
            has_precipitation: true,
            grass_color: None,
            foliage_color: None,
            dry_foliage_color: None,
            water_color: Some([0x3F, 0x76, 0xE4]),
            grass_color_modifier: GrassColorModifier::None,
        },
    ]);
    let textures =
        TerrainTextureState::with_biome_colors_for_tests(biome_colors).with_biome_blend_radius(2);
    let table = TerrainBlockRenderTable::from_resources(TerrainResourceSource::new(
        &textures,
        TerrainBlockRenderContext::new(Vec::new(), dense_bench_base_state_facts),
    ));

    let state = |name: &str| {
        vanilla_block_type_for_resource_id(name)
            .and_then(|block| vanilla_block_default_state_id(block.id))
            .unwrap_or_else(|| panic!("missing default block state for {name}")) as u32
    };
    let air = state("minecraft:air");
    let stone = state("minecraft:stone");
    let sand = state("minecraft:sand");
    let water = state("minecraft:water");
    let grass_block = state("minecraft:grass_block");
    let short_grass = state("minecraft:short_grass");
    let oak_log = state("minecraft:oak_log");
    let oak_leaves = state("minecraft:oak_leaves");

    let solid_stone = PalettedContainerData::from_block_state_ids(&[stone; 4096]);
    let sky_light = [0xff; LIGHT_ARRAY_BYTES];

    let mut center_snapshot = None;
    let mut neighbors = Vec::with_capacity(8);

    for cz in -1_i32..=1 {
        for cx in -1_i32..=1 {
            let mut sec0 = [air; 4096];
            let mut sec1 = [air; 4096];
            for y in 0..16_usize {
                for z in 0..16_usize {
                    for x in 0..16_usize {
                        let idx = (y * 16 + z) * 16 + x;
                        let is_ocean = cx < 0 || (cx == 0 && x < 8);
                        if is_ocean {
                            sec0[idx] = match y {
                                0..=2 => stone,
                                3 => sand,
                                _ => water,
                            };
                        } else {
                            sec0[idx] = match y {
                                0..=2 => stone,
                                3 => grass_block,
                                4 if x % 5 == 1 && z % 5 == 1 => oak_log,
                                4 if (x + z) % 3 == 0 => short_grass,
                                5..=10 if x % 5 == 1 && z % 5 == 1 => oak_log,
                                8..=15 if (x + y + z) % 4 != 0 => oak_leaves,
                                _ => air,
                            };
                        }
                    }
                }
            }
            for y in 0..4_usize {
                for z in 0..16_usize {
                    for x in 0..16_usize {
                        let idx = (y * 16 + z) * 16 + x;
                        let is_ocean = cx < 0 || (cx == 0 && x < 8);
                        if is_ocean && y == 0 {
                            sec1[idx] = water;
                        } else if !is_ocean && y <= 2 && (x + y + z) % 3 != 0 {
                            sec1[idx] = oak_leaves;
                        }
                    }
                }
            }

            let biomes = match cx.cmp(&0) {
                std::cmp::Ordering::Less => single_biome_container(0),
                std::cmp::Ordering::Equal => split_x_biome_container(0, 1),
                std::cmp::Ordering::Greater => single_biome_container(1),
            };
            let source = Arc::new(OceanForestChunkSource {
                block_sections: vec![
                    solid_stone.clone(),
                    PalettedContainerData::from_block_state_ids(&sec0),
                    PalettedContainerData::from_block_state_ids(&sec1),
                    PalettedContainerData::from_block_state_ids(&[air; 4096]),
                ],
                biomes,
                sky_light,
            });
            let snapshot = TerrainChunkSnapshot::from_source(
                cx,
                cz,
                -16,
                SNAPSHOT_HEIGHT,
                -1,
                source,
                TerrainCardinalLighting::Default,
            );
            if cx == 0 && cz == 0 {
                center_snapshot = Some(snapshot);
            } else {
                neighbors.push(snapshot);
            }
        }
    }
    (table, center_snapshot.unwrap(), neighbors)
}

fn bench_dense_ocean_forest_section(c: &mut Criterion) {
    let (table, center, neighbors) = build_ocean_forest_world();
    let probe = build_terrain_section_meshes(
        &center,
        neighbors.iter(),
        &table,
        CAMERA,
        FULL_HEIGHT_SECTIONS,
    );
    assert!(
        probe.opaque.len() + probe.cutout.len() > 0 && !probe.translucent.is_empty(),
        "dense ocean+forest bench fixture must emit solid/foliage and water geometry",
    );

    let mut group = c.benchmark_group("terrain_cpu/dense_ocean_forest_section");
    group.bench_function("biome_tint_and_neighbors_full_height", |b| {
        b.iter(|| {
            black_box(build_terrain_section_meshes(
                black_box(&center),
                neighbors.iter(),
                &table,
                black_box(CAMERA),
                FULL_HEIGHT_SECTIONS,
            ))
        });
    });
    group.bench_function("biome_tint_and_neighbors_surface_sections", |b| {
        b.iter(|| {
            black_box(build_terrain_section_meshes(
                black_box(&center),
                neighbors.iter(),
                &table,
                black_box(CAMERA),
                &[0, 1],
            ))
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets =
        bench_snapshot_materialization,
        bench_changed_owner_mesh_build,
        bench_dirty_mesh_workloads,
        bench_translucent_camera_resort_input,
        bench_dense_ocean_forest_section
}
criterion_main!(benches);
