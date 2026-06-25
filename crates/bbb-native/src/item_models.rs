//! Dropped-item 3D models: turns dropped item entities into baked item-model meshes for the renderer's
//! item-model pass, replacing the flat billboard. A dropped item whose item is a block bakes its block
//! render shape over the blocks atlas (the block path); every other item extrudes its flat sprite into a
//! `1/16`-thick slab over the item atlas (the generated/flat path, vanilla `builtin/generated`). Both
//! are placed by vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the model's
//! GROUND display transform (different defaults for blocks vs flat items).

use std::collections::{BTreeMap, BTreeSet};

use bbb_renderer::{bake_generated_item_quads, bake_item_model_mesh, ItemModelMesh, ItemModelQuad};
use bbb_world::WorldStore;
use glam::{Mat4, Vec3};

use crate::item_runtime::NativeItemRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// Vanilla GROUND display transform for a block item (`minecraft:block/block`): translation `[0, 3, 0]`
/// in 1/16 units, scale `0.25`. A centered unit cube under it seats just above the ground, so no extra
/// lift is needed.
const BLOCK_GROUND: GroundTransform = GroundTransform {
    translation_y: 3.0 / 16.0,
    scale: 0.25,
    min_offset_y: 0.0,
};

/// Vanilla GROUND display transform for a flat item (`minecraft:item/generated`): translation `[0, 2, 0]`
/// in 1/16 units, scale `0.5`. The `0.5`-scaled, centered slab's bottom sits at `-0.125`, so vanilla's
/// `minOffsetY = -minY + 1/16` lifts it by `0.1875` to rest on the ground.
const FLAT_GROUND: GroundTransform = GroundTransform {
    translation_y: 2.0 / 16.0,
    scale: 0.5,
    min_offset_y: 0.1875,
};

/// A model's GROUND display transform plus vanilla's ground-seating lift (`minOffsetY`).
struct GroundTransform {
    translation_y: f32,
    scale: f32,
    min_offset_y: f32,
}

/// The baked item-model meshes for this frame, split by which atlas they sample (block-items → blocks
/// atlas, flat items → item atlas), plus the set of dropped-item entity ids they cover (so the billboard
/// path skips those entities and does not double-render them).
pub(crate) struct DroppedItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub handled_entity_ids: BTreeSet<i32>,
}

/// Bakes a 3D model for every dropped item entity — a block model for block items, an extruded sprite
/// for flat items — at the entity's world position with vanilla's bob + spin animation and the GROUND
/// display transform. `age_ticks` is the animation clock (world game time + partial tick).
pub(crate) fn dropped_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    age_ticks: f32,
) -> DroppedItemModels {
    let mut block_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut handled_entity_ids = BTreeSet::new();
    let Some(item_runtime) = item_runtime else {
        return DroppedItemModels {
            block_meshes,
            flat_meshes,
            handled_entity_ids,
        };
    };

    for state in world.item_entity_stacks() {
        let Some(item_id) = state.stack.item_id else {
            continue;
        };
        let position = [
            state.position.x as f32,
            state.position.y as f32,
            state.position.z as f32,
        ];

        // Block path: the item is a block with 3D item geometry.
        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    let transform =
                        dropped_item_transform(position, age_ticks, state.entity_id, &BLOCK_GROUND);
                    block_meshes.push(bake_item_model_mesh(&quads, transform));
                    handled_entity_ids.insert(state.entity_id);
                    continue;
                }
            }
        }

        // Flat path: extrude the item's sprite layers into a slab.
        let mut quads: Vec<ItemModelQuad> = Vec::new();
        for layer in item_runtime.generated_item_layers_for_stack(&state.stack) {
            quads.extend(bake_generated_item_quads(
                &layer.mask,
                layer.rect,
                layer.tint,
            ));
        }
        if quads.is_empty() {
            continue;
        }
        let transform = dropped_item_transform(position, age_ticks, state.entity_id, &FLAT_GROUND);
        flat_meshes.push(bake_item_model_mesh(&quads, transform));
        handled_entity_ids.insert(state.entity_id);
    }

    DroppedItemModels {
        block_meshes,
        flat_meshes,
        handled_entity_ids,
    }
}

/// The model→world transform for one dropped item: `T(pos, +bob+minOffsetY) · Ry(spin) · GROUND`, where
/// `GROUND` is the model's ground display transform applied about the model center (`T(t) · S · T(-0.5)`,
/// vanilla `ItemTransform.apply`). The unit (`0..1`) model is normalized by the renderer at bake time.
fn dropped_item_transform(
    position: [f32; 3],
    age_ticks: f32,
    entity_id: i32,
    ground: &GroundTransform,
) -> Mat4 {
    let phase = bob_offset(entity_id);
    // Vanilla `ItemEntityRenderer`: bob = sin(age/10 + bobOffs)·0.1 + 0.1; spin = age/20 + bobOffs.
    let bob = (age_ticks / 10.0 + phase).sin() * 0.1 + 0.1;
    let spin = age_ticks / 20.0 + phase;

    let world = Mat4::from_translation(Vec3::new(
        position[0],
        position[1] + bob + ground.min_offset_y,
        position[2],
    ));
    let rotation = Mat4::from_rotation_y(spin);
    let ground = Mat4::from_translation(Vec3::new(0.0, ground.translation_y, 0.0))
        * Mat4::from_scale(Vec3::splat(ground.scale))
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
    fn block_ground_transform_seats_a_unit_block_just_above_the_entity_origin() {
        // A unit-cube corner (0,0,0) → ground bottom, (1,1,1) → top: the 0.25-scaled block centered then
        // lifted +3/16 sits at y in [0.0625, 0.3125] (vanilla GROUND), plus the bob.
        let transform = dropped_item_transform([0.0, 64.0, 0.0], 0.0, 0, &BLOCK_GROUND);
        let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.0));
        let top = transform.transform_point3(Vec3::new(1.0, 1.0, 1.0));
        assert!((bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4, "bottom y");
        assert!((top.y - (64.0 + bob + 0.3125)).abs() < 1e-4, "top y");
    }

    #[test]
    fn flat_ground_transform_lifts_the_slab_to_rest_on_the_ground() {
        // The flat slab's bottom (model y=0) sits at -0.125 before minOffsetY; the 0.1875 lift seats it
        // at +0.0625 (+ bob), matching the block item's ground clearance.
        let transform = dropped_item_transform([0.0, 64.0, 0.0], 0.0, 0, &FLAT_GROUND);
        let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.5));
        assert!(
            (bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4,
            "flat bottom y"
        );
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
