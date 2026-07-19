//! Headless CPU baselines for terrain snapshot materialization and changed-owner meshing.
//!
//! GPU buffer creation is intentionally not benchmarked here: it requires a
//! concrete adapter/device and belongs to the fixed playable performance
//! fixture. These cases isolate the immutable CPU input and 1/8/32 changed
//! chunk mesh work that may later move to a bounded worker.

use std::time::Duration;

use bbb_renderer::terrain::{
    build_terrain_chunk_mesh_layers_with_atlas_and_camera, TerrainCell, TerrainChunkSnapshot,
    TerrainMaterialClass, TerrainTextureAtlas, TerrainTransparency,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const CHANGED_OWNER_COUNTS: [usize; 3] = [1, 8, 32];
const SNAPSHOT_HEIGHT: usize = 64;

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
                        build_terrain_chunk_mesh_layers_with_atlas_and_camera(
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

fn bench_translucent_camera_resort_input(c: &mut Criterion) {
    let atlas = TerrainTextureAtlas::unit();
    let snapshot = representative_snapshot(0);
    let mut camera_x = 0.0_f32;
    c.bench_function("terrain_cpu/translucent_camera_resort_input/1_owner", |b| {
        b.iter(|| {
            camera_x = if camera_x == 0.0 { 32.0 } else { 0.0 };
            black_box(build_terrain_chunk_mesh_layers_with_atlas_and_camera(
                &snapshot,
                std::iter::empty::<&TerrainChunkSnapshot>(),
                &atlas,
                black_box([camera_x, 72.0, 8.0]),
            ))
        });
    });
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets =
        bench_snapshot_materialization,
        bench_changed_owner_mesh_build,
        bench_translucent_camera_resort_input
}
criterion_main!(benches);
