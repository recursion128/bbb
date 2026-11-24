//! Headless baselines for heap-backed non-entity frame extraction owners.
//!
//! The benchmark deliberately stays below wgpu: it measures the repeated
//! materialization handed to `RendererFrame`. Runtime counters and offscreen
//! renderer tests separately attribute apply, merge, atlas, and upload work.

use std::{hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

const REGISTRY_KEY_FAMILIES: usize = 4;

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
    targets = bench_registry_key_materialization
}
criterion_main!(benches);
