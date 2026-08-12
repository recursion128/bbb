//! Baseline benchmarks for the entity projection + client-tick hot paths.
//!
//! These exercise the three per-frame / per-tick entry points the renderer and
//! control layers drive every frame, at representative entity counts:
//!
//! - `WorldStore::entity_model_sources_at_partial_tick` — the render-model
//!   projection chain (position lerp, equipment, minecart rail interpolation).
//! - `WorldStore::entity_pick_targets_at_partial_tick` — the pick/hover bounds
//!   projection.
//! - `WorldStore::advance_entity_client_animations` — one client tick of the
//!   animation/lerp advance loop.
//!
//! Every entity is constructed through the same public `WorldStore` packet-apply
//! API that any downstream crate would use, so the benched cost matches the real
//! ingest path. Positions and the type mix are fully deterministic (index
//! derived, no randomness) so runs are comparable.

use std::{hint::black_box, time::Duration};

use bbb_protocol::{
    entity_types::{
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
        VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    },
    packets::{
        AddEntity, DataComponentPatchSummary, EquipmentSlot, EquipmentSlotUpdate, ItemStackSummary,
        MinecartStep, MoveMinecartAlongTrack, SetEquipment, Vec3d,
    },
};
use bbb_world::WorldStore;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

const ENTITY_COUNTS: [usize; 3] = [100, 500, 2000];
const VISIBLE_PERCENTAGES: [usize; 4] = [0, 10, 50, 100];

/// Deterministic type mix: mostly common mobs, with a steady minority of
/// minecarts (lerp branch) and horses/llamas (equipment branch) so the projection
/// exercises those paths.
fn entity_type_for_index(i: usize) -> i32 {
    match i % 12 {
        0 | 4 => VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        1 | 5 => VANILLA_ENTITY_TYPE_COW_ID,
        2 => VANILLA_ENTITY_TYPE_PIG_ID,
        6 => VANILLA_ENTITY_TYPE_SHEEP_ID,
        3 => VANILLA_ENTITY_TYPE_SKELETON_ID,
        7 => VANILLA_ENTITY_TYPE_CREEPER_ID,
        8 => VANILLA_ENTITY_TYPE_MINECART_ID,
        9 => VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        10 => VANILLA_ENTITY_TYPE_HORSE_ID,
        _ => VANILLA_ENTITY_TYPE_LLAMA_ID,
    }
}

fn deterministic_position(i: usize) -> Vec3d {
    Vec3d {
        x: (i % 32) as f64 * 2.0,
        y: 64.0,
        z: (i / 32) as f64 * 2.0,
    }
}

fn saddle_item() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    }
}

/// Build a `WorldStore` populated with `count` entities purely through the public
/// packet-apply API, giving minecarts lerp steps and horses/llamas equipment so
/// the projection touches those branches.
fn build_store(count: usize) -> WorldStore {
    let mut store = WorldStore::new();
    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
    for i in 0..count {
        let id = (i + 1) as i32;
        let entity_type_id = entity_type_for_index(i);
        let position = deterministic_position(i);
        store.apply_add_entity(AddEntity {
            id,
            uuid: uuid::Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0000 + id as u128),
            entity_type_id,
            position,
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: (i % 360) as f32,
            y_head_rot: (i % 360) as f32,
            data: 0,
        });

        match entity_type_id {
            VANILLA_ENTITY_TYPE_MINECART_ID | VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => {
                let step = MinecartStep {
                    position,
                    movement: Vec3d {
                        x: 0.05,
                        y: 0.0,
                        z: 0.0,
                    },
                    y_rot: (i % 360) as f32,
                    x_rot: 0.0,
                    weight: 1.0,
                };
                store.apply_move_minecart_along_track(MoveMinecartAlongTrack {
                    entity_id: id,
                    lerp_steps: vec![step, step],
                });
            }
            VANILLA_ENTITY_TYPE_HORSE_ID | VANILLA_ENTITY_TYPE_LLAMA_ID => {
                store.apply_set_equipment(SetEquipment {
                    entity_id: id,
                    slots: vec![EquipmentSlotUpdate {
                        slot: EquipmentSlot::Saddle,
                        item: saddle_item(),
                    }],
                });
            }
            _ => {}
        }
    }
    store
}

fn bench_model_sources(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_model_sources_at_partial_tick");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for &count in &ENTITY_COUNTS {
        let store = build_store(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(store.entity_model_sources_at_partial_tick(black_box(0.5))));
        });
    }
    group.finish();
}

fn bench_pick_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_pick_targets_at_partial_tick");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for &count in &ENTITY_COUNTS {
        let store = build_store(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(store.entity_pick_targets_at_partial_tick(black_box(0.5))));
        });
    }
    group.finish();
}

/// Records the current render-order cost: all entities first pay the complete
/// model-source projection and only then are non-survivors discarded. The
/// survivor predicate is deliberately trivial so the 0/10/50/100% series
/// attributes the invariant cost to projection order rather than a particular
/// future frustum implementation.
fn bench_current_projection_before_visibility(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_render/current_full_projection_then_visibility");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for &count in &ENTITY_COUNTS {
        let store = build_store(count);
        for &visible_percentage in &VISIBLE_PERCENTAGES {
            let survivor_count = count * visible_percentage / 100;
            group.bench_with_input(
                BenchmarkId::new(format!("visible_{visible_percentage}_percent"), count),
                &(count, survivor_count),
                |b, &(_, survivor_count)| {
                    b.iter(|| {
                        let mut sources =
                            store.entity_model_sources_at_partial_tick(black_box(0.5));
                        sources.retain(|source| source.entity_id <= survivor_count as i32);
                        black_box(sources)
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_visibility_first_projection(c: &mut Criterion) {
    let mut candidate_group = c.benchmark_group("entity_render/narrow_candidates");
    candidate_group.warm_up_time(Duration::from_millis(500));
    candidate_group.measurement_time(Duration::from_secs(2));
    for &count in &ENTITY_COUNTS {
        let store = build_store(count);
        let mut candidates = Vec::new();
        candidate_group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                store
                    .fill_entity_render_candidates_at_partial_tick(black_box(0.5), &mut candidates);
                black_box(candidates.len())
            });
        });
    }
    candidate_group.finish();

    let mut projection_group = c.benchmark_group("entity_render/visibility_first_projection");
    projection_group.warm_up_time(Duration::from_millis(500));
    projection_group.measurement_time(Duration::from_secs(2));
    for &count in &ENTITY_COUNTS {
        let store = build_store(count);
        for &visible_percentage in &VISIBLE_PERCENTAGES {
            let survivor_count = count * visible_percentage / 100;
            projection_group.bench_with_input(
                BenchmarkId::new(format!("visible_{visible_percentage}_percent"), count),
                &(count, survivor_count),
                |b, &(_, survivor_count)| {
                    b.iter(|| {
                        let candidates =
                            store.entity_render_candidates_at_partial_tick(black_box(0.5));
                        let survivors = candidates
                            .iter()
                            .copied()
                            .take(survivor_count)
                            .collect::<Vec<_>>();
                        black_box(store.entity_model_sources_for_candidates_at_partial_tick(
                            &survivors,
                            &candidates,
                            0.5,
                        ))
                    });
                },
            );
        }
    }
    projection_group.finish();
}

fn bench_advance_client_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("advance_entity_client_animations");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for &count in &ENTITY_COUNTS {
        let mut store = build_store(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| store.advance_entity_client_animations(black_box(1)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_model_sources,
    bench_pick_targets,
    bench_current_projection_before_visibility,
    bench_visibility_first_projection,
    bench_advance_client_tick
);
criterion_main!(benches);
