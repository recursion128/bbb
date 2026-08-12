//! Headless CPU baselines for terrain snapshot materialization and dirty-owner meshing.
//!
//! GPU buffer creation is intentionally not benchmarked here: it requires a
//! concrete adapter/device and belongs to the fixed playable performance
//! fixture. These cases isolate immutable CPU input, historical 1/8/32 full
//! chunk work, and production-equivalent section-dirty workloads.

use std::{hint::black_box, time::Duration};

use bbb_renderer::terrain::{
    build_terrain_chunk_section_mesh_layers_with_atlas_and_camera,
    build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera, TerrainCell,
    TerrainChunkSnapshot, TerrainMaterialClass, TerrainTextureAtlas, TerrainTransparency,
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
                    0 => TerrainCell::with_texture(2, TerrainMaterialClass::Translucent, 0)
                        .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6]),
                    1 | 2 => TerrainCell::with_texture(3, TerrainMaterialClass::Cutout, 0),
                    _ => TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0),
                };
            }
        }
    }
    cells
}

fn representative_snapshot(index: usize) -> TerrainChunkSnapshot {
    TerrainChunkSnapshot::new(
        index as i32,
        0,
        -16,
        SNAPSHOT_HEIGHT,
        representative_cells(),
    )
}

fn dense_section_snapshot(index: usize) -> TerrainChunkSnapshot {
    let mut cells = vec![TerrainCell::EMPTY; 16 * SNAPSHOT_HEIGHT * 16];
    for y in 0..SNAPSHOT_HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                let cell_index = (y * 16 + z) * 16 + x;
                cells[cell_index] = match (x + y + z) % 19 {
                    0 => TerrainCell::with_texture(2, TerrainMaterialClass::Translucent, 0)
                        .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6]),
                    1 | 2 => TerrainCell::with_texture(3, TerrainMaterialClass::Cutout, 0),
                    _ => TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0),
                };
            }
        }
    }
    TerrainChunkSnapshot::new(index as i32, 0, -16, SNAPSHOT_HEIGHT, cells)
}

fn benchmark_configuration() -> Criterion {
    Criterion::default()
        .without_plots()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
}

fn bench_snapshot_materialization(c: &mut Criterion) {
    let cells = representative_cells();
    let mut group = c.benchmark_group("terrain_cpu/snapshot_materialization");
    for count in CHANGED_OWNER_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let snapshots = (0..count)
                    .map(|index| {
                        TerrainChunkSnapshot::new(
                            index as i32,
                            0,
                            -16,
                            SNAPSHOT_HEIGHT,
                            cells.clone(),
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
    let atlas = TerrainTextureAtlas::unit();
    let snapshots = (0..*CHANGED_OWNER_COUNTS.last().unwrap())
        .map(representative_snapshot)
        .collect::<Vec<_>>();
    let mut group = c.benchmark_group("terrain_cpu/changed_owner_mesh_build");
    for count in CHANGED_OWNER_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let layers = snapshots[..count]
                    .iter()
                    .map(|snapshot| {
                        build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
                            snapshot,
                            std::iter::empty::<&TerrainChunkSnapshot>(),
                            &atlas,
                            black_box([8.0, 72.0, 8.0]),
                        )
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
    let atlas = TerrainTextureAtlas::unit();
    let snapshots = (0..*CHANGED_OWNER_COUNTS.last().unwrap())
        .map(dense_section_snapshot)
        .collect::<Vec<_>>();
    let retained = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
        &snapshots[0],
        std::iter::empty::<&TerrainChunkSnapshot>(),
        &atlas,
        [8.0, 72.0, 8.0],
    );
    let mut group = c.benchmark_group("terrain_cpu/dirty_mesh_workload");
    for workload in DIRTY_WORKLOADS {
        group.bench_function(workload.name, |b| {
            b.iter(|| {
                if workload.chunks == 0 {
                    return black_box(retained.source_sections);
                }
                let layers = snapshots[..workload.chunks]
                    .iter()
                    .map(|snapshot| {
                        build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera(
                            snapshot,
                            std::iter::empty::<&TerrainChunkSnapshot>(),
                            &atlas,
                            black_box([8.0, 72.0, 8.0]),
                            workload.section_ys,
                        )
                    })
                    .collect::<Vec<_>>();
                black_box(layers.len())
            });
        });
    }
    group.finish();
}

fn bench_translucent_camera_resort_input(c: &mut Criterion) {
    let atlas = TerrainTextureAtlas::unit();
    let snapshot = representative_snapshot(0);
    let mut camera_x = 0.0_f32;
    c.bench_function("terrain_cpu/translucent_camera_resort_input/1_owner", |b| {
        b.iter(|| {
            camera_x = if camera_x == 0.0 { 32.0 } else { 0.0 };
            black_box(
                build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
                    &snapshot,
                    std::iter::empty::<&TerrainChunkSnapshot>(),
                    &atlas,
                    black_box([camera_x, 72.0, 8.0]),
                ),
            )
        });
    });
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets =
        bench_snapshot_materialization,
        bench_changed_owner_mesh_build,
        bench_dirty_mesh_workloads,
        bench_translucent_camera_resort_input
}
criterion_main!(benches);
