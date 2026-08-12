//! Headless baselines for heap-backed non-entity frame extraction owners.
//!
//! The benchmark deliberately stays below wgpu: it measures the repeated
//! materialization handed to `RendererFrame`. Runtime counters and offscreen
//! renderer tests separately attribute apply, merge, atlas, and upload work.

use std::{hint::black_box, time::Duration};

use bbb_renderer::{ItemModelMesh, ItemModelQuad};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Mat4;

const ITEM_MODEL_BUCKETS: usize = 19;
const REGISTRY_KEY_FAMILIES: usize = 4;

fn representative_quad(face: usize) -> ItemModelQuad {
    let (corners, normal) = match face {
        0 => (
            [
                [0.0, 0.0, 0.0],
                [16.0, 0.0, 0.0],
                [16.0, 16.0, 0.0],
                [0.0, 16.0, 0.0],
            ],
            [0.0, 0.0, -1.0],
        ),
        1 => (
            [
                [0.0, 0.0, 16.0],
                [0.0, 16.0, 16.0],
                [16.0, 16.0, 16.0],
                [16.0, 0.0, 16.0],
            ],
            [0.0, 0.0, 1.0],
        ),
        2 => (
            [
                [0.0, 0.0, 0.0],
                [0.0, 16.0, 0.0],
                [0.0, 16.0, 16.0],
                [0.0, 0.0, 16.0],
            ],
            [-1.0, 0.0, 0.0],
        ),
        3 => (
            [
                [16.0, 0.0, 0.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [16.0, 16.0, 0.0],
            ],
            [1.0, 0.0, 0.0],
        ),
        4 => (
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 0.0, 0.0],
            ],
            [0.0, -1.0, 0.0],
        ),
        _ => (
            [
                [0.0, 16.0, 0.0],
                [16.0, 16.0, 0.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            [0.0, 1.0, 0.0],
        ),
    };
    ItemModelQuad {
        corners,
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        tint: [1.0; 4],
        normal,
        shade: 1.0,
        translucent: false,
    }
}

fn representative_mesh() -> ItemModelMesh {
    let quads = (0..6).map(representative_quad).collect::<Vec<_>>();
    let mut mesh = ItemModelMesh::new();
    mesh.append_quads(&quads, Mat4::IDENTITY);
    mesh
}

fn fresh_item_model_buckets(template: &ItemModelMesh, owners: usize) -> Vec<Vec<ItemModelMesh>> {
    (0..ITEM_MODEL_BUCKETS)
        .map(|_| vec![template.clone(); owners])
        .collect()
}

fn fresh_registry_keys(keys: &[String]) -> Vec<Vec<String>> {
    (0..REGISTRY_KEY_FAMILIES).map(|_| keys.to_vec()).collect()
}

fn benchmark_configuration() -> Criterion {
    Criterion::default()
        .without_plots()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
}

fn bench_item_model_frame_owners(c: &mut Criterion) {
    let template = representative_mesh();
    let mut group = c.benchmark_group("render_extract/fresh_item_model_buckets");
    for owners in [0, 1, 8, 32] {
        group.bench_with_input(
            BenchmarkId::from_parameter(owners),
            &owners,
            |b, &owners| {
                b.iter(|| black_box(fresh_item_model_buckets(black_box(&template), owners)));
            },
        );
    }
    group.finish();
}

fn bench_registry_key_materialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_extract/fresh_registry_key_families");
    for count in [32, 256, 1_024] {
        let keys = (0..count)
            .map(|index| format!("minecraft:representative_{index}"))
            .collect::<Vec<_>>();
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(fresh_registry_keys(black_box(&keys))));
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets = bench_item_model_frame_owners, bench_registry_key_materialization
}
criterion_main!(benches);
