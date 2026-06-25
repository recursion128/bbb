//! Dropped-item 3D models: turns dropped item entities whose item is a block into baked block-item
//! meshes for the renderer's item-model pass, replacing the flat billboard for those entities. Mirrors
//! vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the block model's GROUND
//! display transform, and the terrain block-item baking (the blocks atlas). Items that are not blocks
//! (apple, stick) — or blocks with no 3D item geometry (a `Cross` foliage block, which renders flat) —
//! return nothing here and keep their billboard.

use std::collections::{BTreeMap, BTreeSet};

use bbb_renderer::{bake_item_model_mesh, ItemModelMesh};
use bbb_world::WorldStore;
use glam::{Mat4, Vec3};

use crate::item_runtime::NativeItemRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// Vanilla `minecraft:block/block` GROUND display transform: translation `[0, 3, 0]` in 1/16 units,
/// scale `0.25`, no rotation. Standard block items inherit it (custom per-item ground transforms are not
/// yet retained, so this default is used for every block item).
const GROUND_TRANSLATION_Y: f32 = 3.0 / 16.0;
const GROUND_SCALE: f32 = 0.25;

/// The baked block-item meshes for this frame plus the set of dropped-item entity ids they cover (so the
/// billboard path can skip those entities and not double-render them).
pub(crate) struct DroppedItemModels {
    pub meshes: Vec<ItemModelMesh>,
    pub handled_entity_ids: BTreeSet<i32>,
}

/// Bakes a 3D block-item model for every dropped item entity whose item is a block, at the entity's
/// world position with vanilla's bob + spin animation and the GROUND display transform. `age_ticks` is
/// the animation clock (world game time + partial tick).
pub(crate) fn dropped_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    age_ticks: f32,
) -> DroppedItemModels {
    let mut meshes = Vec::new();
    let mut handled_entity_ids = BTreeSet::new();
    let Some(item_runtime) = item_runtime else {
        return DroppedItemModels {
            meshes,
            handled_entity_ids,
        };
    };

    for state in world.item_entity_stacks() {
        let Some(item_id) = state.stack.item_id else {
            continue;
        };
        let Some(resource_id) = item_runtime.item_resource_id(item_id) else {
            continue;
        };
        let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) else {
            continue;
        };
        if quads.is_empty() {
            // A block with no 3D item geometry (e.g. a `Cross` foliage block) renders as a flat item.
            continue;
        }
        let position = [
            state.position.x as f32,
            state.position.y as f32,
            state.position.z as f32,
        ];
        let transform = dropped_item_transform(position, age_ticks, state.entity_id);
        meshes.push(bake_item_model_mesh(&quads, transform));
        handled_entity_ids.insert(state.entity_id);
    }

    DroppedItemModels {
        meshes,
        handled_entity_ids,
    }
}

/// The model→world transform for one dropped item: `T(pos, +bob) · Ry(spin) · GROUND`, where `GROUND` is
/// the block ground display transform applied about the model center (`T(t) · S · T(-0.5)`, vanilla
/// `ItemTransform.apply`). The unit (`0..1`) model is normalized by the renderer at bake time.
fn dropped_item_transform(position: [f32; 3], age_ticks: f32, entity_id: i32) -> Mat4 {
    let phase = bob_offset(entity_id);
    // Vanilla `ItemEntityRenderer`: bob = sin(age/10 + bobOffs)·0.1 + 0.1; spin = age/20 + bobOffs.
    let bob = (age_ticks / 10.0 + phase).sin() * 0.1 + 0.1;
    let spin = age_ticks / 20.0 + phase;

    let world = Mat4::from_translation(Vec3::new(position[0], position[1] + bob, position[2]));
    let rotation = Mat4::from_rotation_y(spin);
    let ground = Mat4::from_translation(Vec3::new(0.0, GROUND_TRANSLATION_Y, 0.0))
        * Mat4::from_scale(Vec3::splat(GROUND_SCALE))
        * Mat4::from_translation(Vec3::splat(-0.5));
    world * rotation * ground
}

/// Vanilla `ItemEntity.bobOffs` is a per-entity random in `[0, 2π)` fixed at spawn, desyncing each item's
/// bob and spin. We derive a stable per-entity phase from the entity id; combined with the world game
/// time as the clock, the absolute phase is indistinguishable from vanilla's spawn-age offset.
fn bob_offset(entity_id: i32) -> f32 {
    let mixed = (entity_id as u32).wrapping_mul(2_654_435_761);
    (mixed as f32 / u32::MAX as f32) * std::f32::consts::TAU
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_transform_seats_a_unit_block_just_above_the_entity_origin() {
        // With age 0 and phase folded out, a unit-cube corner (0,0,0)→ ground bottom, (1,1,1)→ top: the
        // 0.25-scaled block centered then lifted +3/16 sits at y in [0.0625, 0.3125] (vanilla GROUND).
        let transform = dropped_item_transform([0.0, 64.0, 0.0], 0.0, 0);
        let bob = bob_offset(0);
        let bob = (bob).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.0));
        let top = transform.transform_point3(Vec3::new(1.0, 1.0, 1.0));
        // x/z stay centered on the entity (the spin is about Y through the center).
        assert!((bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4, "bottom y");
        assert!((top.y - (64.0 + bob + 0.3125)).abs() < 1e-4, "top y");
    }

    #[test]
    fn bob_offset_is_stable_and_in_range() {
        for id in [-7, 0, 1, 42, 99_999] {
            let phase = bob_offset(id);
            assert!((0.0..std::f32::consts::TAU).contains(&phase));
            assert_eq!(phase, bob_offset(id));
        }
    }
}
