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

use std::time::Duration;

use bbb_protocol::packets::{
    AddEntity, DataComponentPatchSummary, EquipmentSlot, EquipmentSlotUpdate, ItemStackSummary,
    MinecartStep, MoveMinecartAlongTrack, SetEquipment, Vec3d,
};
use bbb_world::WorldStore;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const ENTITY_COUNTS: [usize; 3] = [100, 500, 2000];

// Vanilla 26.1 entity type ids (see `crates/bbb-world/src/entities.rs`).
const TYPE_ZOMBIE: i32 = 150;
const TYPE_COW: i32 = 30;
const TYPE_PIG: i32 = 100;
const TYPE_SHEEP: i32 = 111;
const TYPE_SKELETON: i32 = 115;
const TYPE_CREEPER: i32 = 32;
const TYPE_MINECART: i32 = 85;
const TYPE_CHEST_MINECART: i32 = 25;
const TYPE_HORSE: i32 = 66;
const TYPE_LLAMA: i32 = 78;

/// Deterministic type mix: mostly common mobs, with a steady minority of
/// minecarts (lerp branch) and horses/llamas (equipment branch) so the projection
/// exercises those paths.
fn entity_type_for_index(i: usize) -> i32 {
    match i % 12 {
        0 | 4 => TYPE_ZOMBIE,
        1 | 5 => TYPE_COW,
        2 => TYPE_PIG,
        6 => TYPE_SHEEP,
        3 => TYPE_SKELETON,
        7 => TYPE_CREEPER,
        8 => TYPE_MINECART,
        9 => TYPE_CHEST_MINECART,
        10 => TYPE_HORSE,
        _ => TYPE_LLAMA,
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
            TYPE_MINECART | TYPE_CHEST_MINECART => {
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
            TYPE_HORSE | TYPE_LLAMA => {
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
    bench_advance_client_tick
);
criterion_main!(benches);
