//! CPU baseline for the legacy world-space item/map frame path.
//!
//! The production path at this baseline resolves model-space quads, bakes every
//! visible or rejected instance into fresh world-space meshes, and later merges
//! and uploads those meshes. These cases intentionally include the repeated
//! bake and owner materialization; later residency measurements use the same
//! matrix and separately assert exact GPU upload counts in offscreen tests.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use bbb_renderer::{
    bake_item_model_meshes_with_light_and_overlay_and_foil_mode, ItemFrameMapTexture,
    ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource, ItemModelAtlasKind,
    ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass,
    ItemModelInstanceRequest, ItemModelMeshSet, ItemModelQuad, ItemModelResidencyFrame,
    ITEM_MODEL_NO_OVERLAY,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::{Mat4, Vec3};

const MAP_SIDE: usize = 128;
const MAP_BYTES: usize = MAP_SIDE * MAP_SIDE * 4;

#[derive(Clone, Copy)]
enum DynamicWork {
    Stable,
    CameraOnly(f32),
    SparseTransform,
    SparseLight,
    SparseFoil,
    DenseAnimation(f32),
    ItemFrame,
    FirstPerson,
}

fn representative_quads() -> Vec<ItemModelQuad> {
    let faces = [
        (
            [
                [0.0, 0.0, 0.0],
                [16.0, 0.0, 0.0],
                [16.0, 16.0, 0.0],
                [0.0, 16.0, 0.0],
            ],
            [0.0, 0.0, -1.0],
        ),
        (
            [
                [0.0, 0.0, 16.0],
                [0.0, 16.0, 16.0],
                [16.0, 16.0, 16.0],
                [16.0, 0.0, 16.0],
            ],
            [0.0, 0.0, 1.0],
        ),
        (
            [
                [0.0, 0.0, 0.0],
                [0.0, 16.0, 0.0],
                [0.0, 16.0, 16.0],
                [0.0, 0.0, 16.0],
            ],
            [-1.0, 0.0, 0.0],
        ),
        (
            [
                [16.0, 0.0, 0.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [16.0, 16.0, 0.0],
            ],
            [1.0, 0.0, 0.0],
        ),
        (
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 0.0, 0.0],
            ],
            [0.0, -1.0, 0.0],
        ),
        (
            [
                [0.0, 16.0, 0.0],
                [16.0, 16.0, 0.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            [0.0, 1.0, 0.0],
        ),
    ];
    faces
        .into_iter()
        .enumerate()
        .map(|(index, (corners, normal))| ItemModelQuad {
            corners,
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: if index == 2 {
                [0.7, 0.9, 0.6, 0.75]
            } else {
                [1.0; 4]
            },
            normal,
            shade: 1.0,
            translucent: index == 2,
        })
        .collect()
}

fn instance_transform(index: usize, work: DynamicWork) -> Mat4 {
    let base = Mat4::from_translation(Vec3::new(
        (index % 40) as f32 * 0.35,
        ((index / 40) % 10) as f32 * 0.2,
        (index / 400) as f32 * 0.35,
    ));
    match work {
        DynamicWork::Stable | DynamicWork::SparseLight | DynamicWork::SparseFoil => base,
        DynamicWork::CameraOnly(partial) => Mat4::from_rotation_y(partial * 0.02) * base,
        DynamicWork::SparseTransform if index == 0 => {
            base * Mat4::from_translation(Vec3::new(0.125, 0.0, 0.0))
        }
        DynamicWork::SparseTransform => base,
        DynamicWork::DenseAnimation(partial) => {
            base * Mat4::from_rotation_y(partial + index as f32 * 0.001)
        }
        DynamicWork::ItemFrame => {
            base * Mat4::from_rotation_z((index % 8) as f32 * std::f32::consts::FRAC_PI_4)
                * Mat4::from_scale(Vec3::splat(0.5))
        }
        DynamicWork::FirstPerson => {
            Mat4::from_translation(Vec3::new(0.56, -0.52, -0.72))
                * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_4)
                * Mat4::from_scale(Vec3::splat(0.75))
        }
    }
}

fn legacy_world_space_frame(
    quads: &[ItemModelQuad],
    instance_count: usize,
    visible_count: usize,
    work: DynamicWork,
) -> (Vec<ItemModelMeshSet>, u64) {
    // The legacy dropped-item path resolves/bakes all candidate item entities
    // before any equivalent visibility ratio can reject their complete mesh.
    let mut meshes = Vec::with_capacity(instance_count);
    for index in 0..instance_count {
        let light = if matches!(work, DynamicWork::SparseLight) && index == 0 {
            [0.4, 0.8]
        } else {
            [0.8, 1.0]
        };
        let foil = if matches!(work, DynamicWork::SparseFoil) && index == 0 {
            ItemModelFoil::Standard
        } else if index % 17 == 0 {
            ItemModelFoil::Special {
                decal_pose_scale: 1.0,
            }
        } else {
            ItemModelFoil::None
        };
        meshes.push(bake_item_model_meshes_with_light_and_overlay_and_foil_mode(
            quads,
            instance_transform(index, work),
            light,
            ITEM_MODEL_NO_OVERLAY,
            foil,
        ));
    }
    meshes.truncate(visible_count.min(meshes.len()));
    let bytes = meshes.iter().map(ItemModelMeshSet::logical_bytes).sum();
    (meshes, bytes)
}

fn resident_instance_frame(
    quads: &Arc<[ItemModelQuad]>,
    candidate_count: usize,
    visible_count: usize,
    work: DynamicWork,
) -> ItemModelResidencyFrame {
    let identity = ItemModelAssetIdentity::new(
        Arc::<str>::from("minecraft:benchmark_cube"),
        Arc::<str>::from("default"),
        0,
    );
    let mut frame = ItemModelResidencyFrame::default();
    if visible_count != 0 {
        for geometry_class in [
            ItemModelGeometryClass::Solid,
            ItemModelGeometryClass::Translucent,
        ] {
            frame.assets.push(ItemModelAssetRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class,
                source: ItemModelAssetSource::Quads(Arc::clone(quads)),
            });
        }
    }
    for index in 0..candidate_count {
        if index >= visible_count {
            continue;
        }
        let light = if matches!(work, DynamicWork::SparseLight) && index == 0 {
            [0.4, 0.8]
        } else {
            [0.8, 1.0]
        };
        let foil = if matches!(work, DynamicWork::SparseFoil) && index == 0 {
            ItemModelFoil::Standard
        } else {
            ItemModelFoil::None
        };
        for (geometry_class, phase) in [
            (ItemModelGeometryClass::Solid, ItemModelDrawPhase::Solid),
            (
                ItemModelGeometryClass::Translucent,
                ItemModelDrawPhase::Translucent,
            ),
        ] {
            frame.instances.push(ItemModelInstanceRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class,
                transform: instance_transform(index, work),
                tint: [1.0; 4],
                light,
                overlay: ITEM_MODEL_NO_OVERLAY,
                foil,
                target: ItemModelDrawTarget::World,
                phase,
                submit_order: frame.instances.len() as u32,
            });
        }
    }
    frame
}

fn representative_map(id: i32, revision: u8) -> ItemFrameMapTexture {
    let mut rgba = vec![0_u8; MAP_BYTES];
    for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(&[
            revision.wrapping_add(index as u8),
            id as u8,
            (index / MAP_SIDE) as u8,
            255,
        ]);
    }
    ItemFrameMapTexture { map_id: id, rgba }
}

/// Mirrors the legacy transient atlas's important cost owners: exact id
/// de-duplication, a fresh full atlas allocation, and full-image copies for all
/// currently visible maps.
fn legacy_transient_map_atlas(textures: &[ItemFrameMapTexture]) -> Vec<u8> {
    let by_id = textures
        .iter()
        .map(|texture| (texture.map_id, texture.rgba.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut atlas = Vec::with_capacity(by_id.len() * MAP_BYTES);
    for rgba in by_id.values() {
        atlas.extend_from_slice(rgba);
    }
    atlas
}

fn benchmark_configuration() -> Criterion {
    Criterion::default()
        .without_plots()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(400))
        .measurement_time(Duration::from_secs(2))
}

fn bench_instance_counts(c: &mut Criterion) {
    let quads = representative_quads();
    let mut group = c.benchmark_group("item_legacy/world_space_frame/count");
    for count in [0, 1, 100, 500, 2_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                black_box(legacy_world_space_frame(
                    black_box(&quads),
                    count,
                    count,
                    DynamicWork::Stable,
                ))
            });
        });
    }
    group.finish();
}

fn bench_visibility(c: &mut Criterion) {
    let quads = representative_quads();
    let mut group = c.benchmark_group("item_legacy/world_space_frame/visible_2000");
    for percent in [0, 10, 50, 100] {
        let visible = 2_000 * percent / 100;
        group.bench_with_input(
            BenchmarkId::new("percent", percent),
            &visible,
            |b, &visible| {
                b.iter(|| {
                    black_box(legacy_world_space_frame(
                        black_box(&quads),
                        2_000,
                        visible,
                        DynamicWork::Stable,
                    ))
                });
            },
        );
    }
    group.finish();

    let quads: Arc<[ItemModelQuad]> = quads.into();
    let mut group = c.benchmark_group("item_resident/instance_frame/visible_2000");
    for percent in [0, 10, 50, 100] {
        let visible = 2_000 * percent / 100;
        group.bench_with_input(
            BenchmarkId::new("percent", percent),
            &visible,
            |b, &visible| {
                b.iter(|| {
                    black_box(resident_instance_frame(
                        black_box(&quads),
                        2_000,
                        visible,
                        DynamicWork::Stable,
                    ))
                });
            },
        );
    }
    group.finish();
}

fn bench_dynamic_mutations(c: &mut Criterion) {
    let quads = representative_quads();
    let cases = [
        ("unchanged", DynamicWork::Stable),
        ("partial_tick_camera_only", DynamicWork::CameraOnly(0.75)),
        ("single_transform", DynamicWork::SparseTransform),
        ("single_light", DynamicWork::SparseLight),
        ("single_foil_equipment", DynamicWork::SparseFoil),
        ("dense_animation", DynamicWork::DenseAnimation(0.75)),
        ("item_frame", DynamicWork::ItemFrame),
        ("first_person", DynamicWork::FirstPerson),
    ];
    let mut group = c.benchmark_group("item_legacy/world_space_frame/mutation_500");
    for (name, work) in cases {
        group.bench_function(name, |b| {
            b.iter(|| black_box(legacy_world_space_frame(black_box(&quads), 500, 500, work)));
        });
    }
    group.finish();

    let resident_quads: Arc<[ItemModelQuad]> = quads.clone().into();
    let mut group = c.benchmark_group("item_resident/instance_frame/mutation_500");
    for (name, work) in cases {
        group.bench_function(name, |b| {
            b.iter(|| {
                black_box(resident_instance_frame(
                    black_box(&resident_quads),
                    500,
                    500,
                    work,
                ))
            });
        });
    }
    group.finish();

    c.bench_function("item_legacy/spawn_remove_churn/500_250", |b| {
        let mut dense = true;
        b.iter(|| {
            dense = !dense;
            let count = if dense { 500 } else { 250 };
            black_box(legacy_world_space_frame(
                black_box(&quads),
                count,
                count,
                DynamicWork::DenseAnimation(count as f32),
            ))
        });
    });

    c.bench_function("item_legacy/retained_old_frame/next_generation_500", |b| {
        let retained = legacy_world_space_frame(&quads, 500, 500, DynamicWork::Stable).0;
        b.iter(|| {
            black_box(&retained);
            black_box(legacy_world_space_frame(
                black_box(&quads),
                500,
                500,
                DynamicWork::SparseTransform,
            ))
        });
    });
}

fn bench_map_frames(c: &mut Criterion) {
    for count in [1, 8, 32] {
        let maps = (0..count)
            .map(|id| representative_map(id, 0))
            .collect::<Vec<_>>();
        c.bench_with_input(
            BenchmarkId::new("map_legacy/transient_atlas/unchanged", count),
            &maps,
            |b, maps| b.iter(|| black_box(legacy_transient_map_atlas(black_box(maps)))),
        );

        let mut patched = maps.clone();
        patched[0].rgba[0..4].copy_from_slice(&[255, 0, 0, 255]);
        c.bench_with_input(
            BenchmarkId::new("map_legacy/transient_atlas/single_patch", count),
            &patched,
            |b, maps| b.iter(|| black_box(legacy_transient_map_atlas(black_box(maps)))),
        );
    }

    let maps = (0..32)
        .map(|id| representative_map(id, id as u8))
        .collect::<Vec<_>>();
    c.bench_function("map_legacy/transient_atlas/unload_one_of_32", |b| {
        b.iter(|| black_box(legacy_transient_map_atlas(black_box(&maps[1..]))));
    });
    c.bench_function("map_legacy/transient_atlas/resource_reload_32", |b| {
        b.iter(|| {
            let reloaded = maps.clone();
            black_box(legacy_transient_map_atlas(black_box(&reloaded)))
        });
    });
}

fn bench_steady_fps_batches(c: &mut Criterion) {
    let quads = representative_quads();
    let mut group = c.benchmark_group("item_legacy/steady_100_instances/one_second_batch");
    for frames in [60, 144, 260, 520] {
        group.bench_with_input(
            BenchmarkId::from_parameter(frames),
            &frames,
            |b, &frames| {
                b.iter(|| {
                    for frame in 0..frames {
                        black_box(legacy_world_space_frame(
                            black_box(&quads),
                            100,
                            100,
                            DynamicWork::CameraOnly(frame as f32 / frames as f32),
                        ));
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets =
        bench_instance_counts,
        bench_visibility,
        bench_dynamic_mutations,
        bench_map_frames,
        bench_steady_fps_batches
}
criterion_main!(benches);
