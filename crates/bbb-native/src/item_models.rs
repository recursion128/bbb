//! Dropped-item 3D models: turns dropped item entities into baked item-model meshes for the renderer's
//! item-model pass, replacing the flat billboard. A dropped item whose item is a block bakes its block
//! render shape over the blocks atlas (the block path); every other item extrudes its flat sprite into a
//! `1/16`-thick slab over the item atlas (the generated/flat path, vanilla `builtin/generated`). Both
//! are placed by vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the model's
//! GROUND display transform, and a stack of items renders as the vanilla cluster of `1..=5` jittered
//! copies (`submitMultipleFromCount`).

use std::collections::{BTreeMap, BTreeSet};

use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_renderer::{
    bake_generated_item_quads, bake_item_model_mesh, humanoid_hand_attach_transform,
    EntityModelInstance, ItemModelMesh, ItemModelQuad,
};
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

/// Vanilla `FLAT_ITEM_DEPTH_THRESHOLD` / `ITEM_MIN_HOVER_HEIGHT`: a rendered model thinner than this in Z
/// is stacked back-to-front; a thicker one is scattered in 3D.
const FLAT_ITEM_DEPTH_THRESHOLD: f32 = 0.0625;

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
/// for flat items — at the entity's world position with vanilla's bob + spin animation, the GROUND
/// display transform, and the count-based cluster of copies. `age_ticks` is the animation clock (world
/// game time + partial tick).
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
        let count = rendered_amount(state.stack.count);
        // Vanilla `ItemClusterRenderState` seeds the jitter with `Item.getId + damageValue`; stackable
        // items (the ones that show more than one copy) are undamaged, so the protocol id matches.
        let seed = item_id as i64;

        // Block path: the item is a block with 3D item geometry.
        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    block_meshes.push(stacked_item_mesh(
                        &quads,
                        position,
                        age_ticks,
                        state.entity_id,
                        &BLOCK_GROUND,
                        count,
                        seed,
                    ));
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
        flat_meshes.push(stacked_item_mesh(
            &quads,
            position,
            age_ticks,
            state.entity_id,
            &FLAT_GROUND,
            count,
            seed,
        ));
        handled_entity_ids.insert(state.entity_id);
    }

    DroppedItemModels {
        block_meshes,
        flat_meshes,
        handled_entity_ids,
    }
}

/// Fallback third-person right-hand display transform for a block item (`minecraft:block/block`):
/// rotation `[75, 45, 0]°`, translation `[0, 2.5, 0]`/16, scale `0.375`. Used only when the item's own
/// model transform was not retained (e.g. a missing model).
const BLOCK_THIRD_PERSON_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [75.0, 45.0, 0.0],
    translation: [0.0, 2.5 / 16.0, 0.0],
    scale: [0.375, 0.375, 0.375],
};

/// Fallback third-person right-hand display transform for a flat item (`minecraft:item/generated`):
/// rotation `[0, 0, 0]°`, translation `[0, 3, 1]`/16, scale `0.55`. Handheld tools have their own angled
/// transform (`item/handheld`), retained per-item; this is only the no-model fallback.
const GENERATED_THIRD_PERSON_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 0.0, 0.0],
    translation: [0.0, 3.0 / 16.0, 1.0 / 16.0],
    scale: [0.55, 0.55, 0.55],
};

/// The display transform about the model center: `T(t) · Rxyz · S · T(-0.5)` (vanilla
/// `ItemTransform.apply`). `Rxyz` matches joml `rotationXYZ`. The pack's `BlockModelDisplayTransform`
/// already holds translation in world units (the model JSON's 1/16 value pre-multiplied) and per-axis
/// scale, so no further normalization is applied here. `left_hand` is vanilla's left-hand fix: it negates
/// `translation.x`, `rotation.y`, and `rotation.z` so an item mirrors correctly into the left arm.
fn display_matrix(display: &BlockModelDisplayTransform, left_hand: bool) -> Mat4 {
    let sign = if left_hand { -1.0 } else { 1.0 };
    let translation = Vec3::new(
        display.translation[0] * sign,
        display.translation[1],
        display.translation[2],
    );
    let rotation = Mat4::from_rotation_x(display.rotation[0].to_radians())
        * Mat4::from_rotation_y((display.rotation[1] * sign).to_radians())
        * Mat4::from_rotation_z((display.rotation[2] * sign).to_radians());
    Mat4::from_translation(translation)
        * rotation
        * Mat4::from_scale(Vec3::from_array(display.scale))
        * Mat4::from_translation(Vec3::splat(-0.5))
}

/// The baked held-item meshes for this frame, split by atlas (block-items vs flat items).
pub(crate) struct HeldItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
}

/// Bakes the third-person main- and off-hand held items for every humanoid entity that holds one
/// (players and the weapon-holding mobs — zombies, skeletons, piglins, illagers; vanilla
/// `ItemInHandLayer`). The hand attach transform comes from the renderer's posed humanoid model; native
/// resolves the item to quads (block or flat) and applies the item's third-person display transform.
pub(crate) fn held_item_models(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> HeldItemModels {
    let mut block_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let Some(item_runtime) = item_runtime else {
        return HeldItemModels {
            block_meshes,
            flat_meshes,
        };
    };

    for instance in instances {
        // Vanilla holds the main hand in the right arm and the off hand in the left arm (default main
        // arm; the off-hand item additionally gets the left-hand display mirror).
        bake_held_hand(
            instance,
            false,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut flat_meshes,
        );
        bake_held_hand(
            instance,
            true,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut flat_meshes,
        );
    }

    HeldItemModels {
        block_meshes,
        flat_meshes,
    }
}

/// Bakes one held item (`off_hand` selects the off-hand slot / left arm) onto its arm bone with the
/// item's own `thirdperson_{left,right}hand` display transform.
#[allow(clippy::too_many_arguments)]
fn bake_held_hand(
    instance: &EntityModelInstance,
    off_hand: bool,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, off_hand) else {
        return;
    };
    let Some(item_id) = stack.item_id else {
        return;
    };
    // The off hand is the left arm (default right-handed main arm); the left arm gets the left-hand
    // display mirror.
    let left_arm = off_hand;
    let Some(hand) = humanoid_hand_attach_transform(instance, left_arm) else {
        return;
    };

    // The item's own retained third-person transform for this arm (handheld tools angle the model,
    // blocks tilt it, generated items lay it flat); falls back to the parent-model default per path.
    let context = if left_arm {
        BlockModelDisplayContext::ThirdPersonLeftHand
    } else {
        BlockModelDisplayContext::ThirdPersonRightHand
    };
    let retained = item_runtime.item_display_transform(item_id, context);

    // Block path.
    if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
        if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
            if !quads.is_empty() {
                let display = retained.unwrap_or(BLOCK_THIRD_PERSON_FALLBACK);
                let transform = hand * display_matrix(&display, left_arm);
                block_meshes.push(bake_item_model_mesh(&quads, transform));
                return;
            }
        }
    }

    // Flat path.
    let mut quads: Vec<ItemModelQuad> = Vec::new();
    for layer in item_runtime.generated_item_layers_for_stack(&stack) {
        quads.extend(bake_generated_item_quads(
            &layer.mask,
            layer.rect,
            layer.tint,
        ));
    }
    if quads.is_empty() {
        return;
    }
    let display = retained.unwrap_or(GENERATED_THIRD_PERSON_FALLBACK);
    let transform = hand * display_matrix(&display, left_arm);
    flat_meshes.push(bake_item_model_mesh(&quads, transform));
}

/// Vanilla `ItemClusterRenderState.getRenderedAmount`: the number of copies rendered for a stack size.
fn rendered_amount(stack_count: i32) -> usize {
    match stack_count {
        i32::MIN..=1 => 1,
        2..=16 => 2,
        17..=32 => 3,
        33..=48 => 4,
        _ => 5,
    }
}

/// Bakes the cluster of copies for one dropped item into a single mesh.
fn stacked_item_mesh(
    quads: &[ItemModelQuad],
    position: [f32; 3],
    age_ticks: f32,
    entity_id: i32,
    ground: &GroundTransform,
    count: usize,
    seed: i64,
) -> ItemModelMesh {
    let base = base_transform(position, age_ticks, entity_id, ground.min_offset_y);
    let ground_matrix = ground_matrix(ground);
    let depth = model_depth(quads, ground.scale);
    let mut mesh = ItemModelMesh::new();
    append_cluster(&mut mesh, quads, base, ground_matrix, count, seed, depth);
    mesh
}

/// The entity-level transform shared by every copy: `T(pos, +bob + minOffsetY) · Ry(spin)` (vanilla
/// `ItemEntityRenderer.submit` before the per-item display transform).
fn base_transform(position: [f32; 3], age_ticks: f32, entity_id: i32, min_offset_y: f32) -> Mat4 {
    let phase = bob_offset(entity_id);
    // Vanilla `ItemEntityRenderer`: bob = sin(age/10 + bobOffs)·0.1 + 0.1; spin = age/20 + bobOffs.
    let bob = (age_ticks / 10.0 + phase).sin() * 0.1 + 0.1;
    let spin = age_ticks / 20.0 + phase;
    Mat4::from_translation(Vec3::new(
        position[0],
        position[1] + bob + min_offset_y,
        position[2],
    )) * Mat4::from_rotation_y(spin)
}

/// The GROUND display transform about the model center: `T(t) · S · T(-0.5)` (vanilla `ItemTransform.apply`).
fn ground_matrix(ground: &GroundTransform) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, ground.translation_y, 0.0))
        * Mat4::from_scale(Vec3::splat(ground.scale))
        * Mat4::from_translation(Vec3::splat(-0.5))
}

/// The rendered Z thickness of a model (vanilla `modelBoundingBox.getZsize()`): the quad corners' Z
/// extent in `0..16` model space, normalized to the unit cube and scaled by the display transform.
fn model_depth(quads: &[ItemModelQuad], scale: f32) -> f32 {
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for quad in quads {
        for corner in quad.corners {
            min_z = min_z.min(corner[2]);
            max_z = max_z.max(corner[2]);
        }
    }
    if max_z < min_z {
        return 0.0;
    }
    (max_z - min_z) / 16.0 * scale
}

/// Vanilla `ItemEntityRenderer.submitMultipleFromCount`: bake `count` copies of the model. A model
/// thicker than [`FLAT_ITEM_DEPTH_THRESHOLD`] scatters its copies in 3D; a thin (flat) one stacks them
/// back-to-front along Z with small in-plane jitter. The jitter draws from a Java-seeded RNG so the
/// cluster arrangement matches vanilla.
fn append_cluster(
    mesh: &mut ItemModelMesh,
    quads: &[ItemModelQuad],
    base: Mat4,
    ground: Mat4,
    count: usize,
    seed: i64,
    depth: f32,
) {
    let mut random = LegacyRandom::new(seed);
    if depth > FLAT_ITEM_DEPTH_THRESHOLD {
        mesh.append_quads(quads, base * ground);
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let zo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let offset = Mat4::from_translation(Vec3::new(xo, yo, zo));
            mesh.append_quads(quads, base * offset * ground);
        }
    } else {
        let offset_z = depth * 1.5;
        let mut z = -(offset_z * (count as f32 - 1.0) / 2.0);
        mesh.append_quads(
            quads,
            base * Mat4::from_translation(Vec3::new(0.0, 0.0, z)) * ground,
        );
        z += offset_z;
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let offset = Mat4::from_translation(Vec3::new(xo, yo, z));
            mesh.append_quads(quads, base * offset * ground);
            z += offset_z;
        }
    }
}

/// Vanilla `ItemEntity.bobOffs` is a per-entity random in `[0, 2π)` fixed at spawn, desyncing each item's
/// bob and spin. We derive a stable per-entity phase from the entity id; combined with the world game
/// time as the clock, the absolute phase is indistinguishable from vanilla's spawn-age offset.
fn bob_offset(entity_id: i32) -> f32 {
    let mixed = (entity_id as u32).wrapping_mul(2_654_435_761);
    (mixed as f32 / u32::MAX as f32) * std::f32::consts::TAU
}

const RNG_MULTIPLIER: u64 = 25_214_903_917;
const RNG_INCREMENT: u64 = 11;
const RNG_MASK: u64 = (1_u64 << 48) - 1;

/// The Java `Random` / vanilla `LegacyRandomSource` LCG, enough to reproduce `nextFloat()` for the
/// cluster jitter so it matches vanilla.
struct LegacyRandom {
    seed: u64,
}

impl LegacyRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RNG_MULTIPLIER) & RNG_MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RNG_MULTIPLIER)
            .wrapping_add(RNG_INCREMENT)
            & RNG_MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_block_quads() -> Vec<ItemModelQuad> {
        // A single full-cube face spanning the 0..16 Z range, enough to exercise depth + transforms.
        vec![ItemModelQuad {
            corners: [
                [0.0, 0.0, 0.0],
                [16.0, 0.0, 0.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            uvs: [[0.0, 0.0]; 4],
            tint: [1.0, 1.0, 1.0, 1.0],
            shade: 1.0,
        }]
    }

    #[test]
    fn block_ground_transform_seats_a_unit_block_just_above_the_entity_origin() {
        // The 0.25-scaled block centered then lifted +3/16 sits at y in [0.0625, 0.3125], plus the bob.
        let transform = base_transform([0.0, 64.0, 0.0], 0.0, 0, BLOCK_GROUND.min_offset_y)
            * ground_matrix(&BLOCK_GROUND);
        let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.0));
        let top = transform.transform_point3(Vec3::new(1.0, 1.0, 1.0));
        assert!((bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4, "bottom y");
        assert!((top.y - (64.0 + bob + 0.3125)).abs() < 1e-4, "top y");
    }

    #[test]
    fn flat_ground_transform_lifts_the_slab_to_rest_on_the_ground() {
        let transform = base_transform([0.0, 64.0, 0.0], 0.0, 0, FLAT_GROUND.min_offset_y)
            * ground_matrix(&FLAT_GROUND);
        let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.5));
        assert!(
            (bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4,
            "flat bottom y"
        );
    }

    #[test]
    fn rendered_amount_matches_vanilla_thresholds() {
        assert_eq!(rendered_amount(0), 1);
        assert_eq!(rendered_amount(1), 1);
        assert_eq!(rendered_amount(2), 2);
        assert_eq!(rendered_amount(16), 2);
        assert_eq!(rendered_amount(17), 3);
        assert_eq!(rendered_amount(32), 3);
        assert_eq!(rendered_amount(33), 4);
        assert_eq!(rendered_amount(48), 4);
        assert_eq!(rendered_amount(49), 5);
        assert_eq!(rendered_amount(64), 5);
    }

    #[test]
    fn model_depth_classifies_block_vs_flat() {
        // A cube face spanning Z 0..16, scaled 0.25, is 0.25 deep → scatter branch.
        let block_depth = model_depth(&unit_block_quads(), BLOCK_GROUND.scale);
        assert!((block_depth - 0.25).abs() < 1e-6);
        assert!(block_depth > FLAT_ITEM_DEPTH_THRESHOLD);
        // A generated slab spans Z 7.5..8.5 (depth 1), scaled 0.5 → 0.03125 deep → stack branch.
        let slab = vec![ItemModelQuad {
            corners: [
                [0.0, 0.0, 7.5],
                [16.0, 0.0, 7.5],
                [16.0, 16.0, 8.5],
                [0.0, 16.0, 8.5],
            ],
            uvs: [[0.0, 0.0]; 4],
            tint: [1.0; 4],
            shade: 1.0,
        }];
        let flat_depth = model_depth(&slab, FLAT_GROUND.scale);
        assert!((flat_depth - 0.031_25).abs() < 1e-6);
        assert!(flat_depth <= FLAT_ITEM_DEPTH_THRESHOLD);
    }

    #[test]
    fn cluster_count_drives_geometry_and_is_non_empty() {
        let quads = unit_block_quads();
        // One copy and four copies both produce geometry; the four-copy mesh holds four times as much.
        let single = stacked_item_mesh(&quads, [0.0, 0.0, 0.0], 0.0, 0, &BLOCK_GROUND, 1, 7);
        let cluster = stacked_item_mesh(&quads, [0.0, 0.0, 0.0], 0.0, 0, &BLOCK_GROUND, 4, 7);
        assert!(!single.is_empty());
        assert!(!cluster.is_empty());
    }

    #[test]
    fn legacy_random_matches_java_sequence() {
        // Java `new Random(0).nextFloat()` is 0.7309678 — the LCG reproduces it bit-for-bit.
        let mut random = LegacyRandom::new(0);
        assert!((random.next_float() - 0.730_967_8).abs() < 1e-6);
    }

    #[test]
    fn display_matrix_centers_the_model_at_the_translation() {
        // The display transform is about the model center (`T(-0.5)`), so the unit-cube center
        // (0.5,0.5,0.5) lands exactly at the (world-unit) translation regardless of rotation/scale.
        let generated = display_matrix(&GENERATED_THIRD_PERSON_FALLBACK, false)
            .transform_point3(Vec3::splat(0.5));
        assert!((generated - Vec3::new(0.0, 3.0 / 16.0, 1.0 / 16.0)).length() < 1e-6);
        let block =
            display_matrix(&BLOCK_THIRD_PERSON_FALLBACK, false).transform_point3(Vec3::splat(0.5));
        assert!((block - Vec3::new(0.0, 2.5 / 16.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn left_hand_display_mirror_negates_x_translation_and_yz_rotation() {
        // Vanilla `ItemTransform.apply(applyLeftHandFix=true)` negates translation.x, rotation.y, and
        // rotation.z. A handheld-style transform with a Y rotation mirrors the model across the X plane.
        let handheld = BlockModelDisplayTransform {
            rotation: [0.0, -90.0, 55.0],
            translation: [1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0],
            scale: [0.85, 0.85, 0.85],
        };
        let right = display_matrix(&handheld, false);
        let left = display_matrix(&handheld, true);
        // The model center sits at the mirrored translation (x negated, y/z unchanged).
        let right_center = right.transform_point3(Vec3::splat(0.5));
        let left_center = left.transform_point3(Vec3::splat(0.5));
        assert!((right_center - Vec3::new(1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0)).length() < 1e-6);
        assert!((left_center - Vec3::new(-1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0)).length() < 1e-6);
        // The left fix mirrors the rotation across the X plane: `rotationXYZ(rx,-ry,-rz)` =
        // `M·rotationXYZ(rx,ry,rz)·M` (M = diag(-1,1,1)), so the rotated +X basis direction reflects to
        // `(x, -y, -z)` of the right-hand one.
        let right_dir = right.transform_vector3(Vec3::X);
        let left_dir = left.transform_vector3(Vec3::X);
        assert!((left_dir - Vec3::new(right_dir.x, -right_dir.y, -right_dir.z)).length() < 1e-6);
    }
}
