//! Item submissions owned by block-entity renderers.
//!
//! Vanilla `BlockEntityRenderers` registers several renderers whose whole
//! submission is one or more `ItemStackRenderState`s placed by a block-local
//! pose (`CampfireRenderer`, `ShelfRenderer`, `VaultRenderer`,
//! `BrushableBlockRenderer`). bbb keeps the world projection in `bbb-world` and
//! bakes the stacks through the single item-model path used by dropped, framed
//! and held items, so these renderers add placement only — never a second item
//! baking or submission owner.

use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_renderer::{
    ItemModelDrawTarget, ItemModelMesh, ItemModelResidencyFrame, ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::{CampfireItemDisplaySourceState, WorldStore};
use glam::{Mat4, Vec3};

use bbb_item_model::NativeItemRuntime;

use super::dropped::bake_item_stack_at_transform_with_light_and_overlay;
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// `CampfireRenderer.submit` centres the cooking items just above the log
/// surface before spinning them into their slot direction.
const CAMPFIRE_ITEM_ORIGIN: Vec3 = Vec3::new(0.5, 0.449_218_75, 0.5);
/// The in-plane centring translation applied after the two rotations.
const CAMPFIRE_ITEM_PLANE_OFFSET: Vec3 = Vec3::new(-0.3125, -0.3125, 0.0);
/// `CampfireRenderer.SIZE`.
const CAMPFIRE_ITEM_SCALE: f32 = 0.375;

/// Bakes every block-entity renderer whose submission is an item stack.
#[allow(clippy::too_many_arguments)]
pub(crate) fn block_entity_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
) -> ItemModelResidencyFrame {
    let mut residency = ItemModelResidencyFrame::default();
    let Some(item_runtime) = item_runtime else {
        return residency;
    };
    for source in world.campfire_item_display_source_states() {
        let light = block_entity_shader_light(world, &source);
        for (slot, stack) in &source.items {
            bake_block_entity_item(
                stack,
                campfire_item_transform(&source, *slot),
                BlockModelDisplayContext::Fixed,
                light,
                world,
                item_runtime,
                terrain_textures,
                trim_material_keys,
                enchantment_keys,
                damage_type_keys,
                attribute_keys,
                variant_textures,
                &mut residency,
            );
        }
    }
    residency
}

/// `CampfireRenderer.submit`: `T(0.5, 0.44921875, 0.5)`, the slot's
/// `Direction.from2DDataValue((slot + facing.get2DDataValue()) % 4)` yaw as
/// `Ry(-direction.toYRot())`, `Rx(90°)`, `T(-0.3125, -0.3125, 0)` and the
/// `0.375` item scale, all relative to the block's minimum corner.
fn campfire_item_transform(source: &CampfireItemDisplaySourceState, slot: u8) -> Mat4 {
    let direction_2d = (u32::from(slot) + u32::from(source.facing_2d)) % 4;
    let y_rot = -horizontal_direction_y_rot(direction_2d as u8);
    Mat4::from_translation(Vec3::new(
        source.pos.x as f32,
        source.pos.y as f32,
        source.pos.z as f32,
    )) * Mat4::from_translation(CAMPFIRE_ITEM_ORIGIN)
        * Mat4::from_rotation_y(y_rot.to_radians())
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
        * Mat4::from_translation(CAMPFIRE_ITEM_PLANE_OFFSET)
        * Mat4::from_scale(Vec3::splat(CAMPFIRE_ITEM_SCALE))
}

/// Vanilla `Direction.from2DDataValue(value).toYRot()`.
fn horizontal_direction_y_rot(direction_2d: u8) -> f32 {
    match direction_2d {
        0 => 0.0,
        1 => 90.0,
        2 => 180.0,
        _ => 270.0,
    }
}

/// `BlockEntityRenderState.extractBase` samples the light coords at the block
/// entity's own position.
fn block_entity_shader_light(
    world: &WorldStore,
    source: &CampfireItemDisplaySourceState,
) -> [f32; 2] {
    let coords = world
        .sample_render_light(source.pos)
        .map_or(0, |light| light.packed());
    [
        ((coords >> 4) & 0xF) as f32 / 15.0,
        ((coords >> 20) & 0xF) as f32 / 15.0,
    ]
}

#[allow(clippy::too_many_arguments)]
fn bake_block_entity_item(
    stack: &bbb_protocol::packets::ItemStackSummary,
    transform: Mat4,
    context: BlockModelDisplayContext,
    light: [f32; 2],
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    residency: &mut ItemModelResidencyFrame,
) {
    let mut unused_block = Vec::<ItemModelMesh>::new();
    let mut unused_block_translucent = Vec::<ItemModelMesh>::new();
    let mut unused_block_glint = Vec::<ItemModelMesh>::new();
    let mut unused_block_glint_translucent = Vec::<ItemModelMesh>::new();
    let mut unused_flat = Vec::<ItemModelMesh>::new();
    let mut unused_flat_translucent = Vec::<ItemModelMesh>::new();
    let mut unused_flat_glint = Vec::<ItemModelMesh>::new();
    let mut unused_flat_glint_translucent = Vec::<ItemModelMesh>::new();
    bake_item_stack_at_transform_with_light_and_overlay(
        stack,
        world,
        transform,
        context,
        false,
        None,
        None,
        None,
        None,
        false,
        0.0,
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        world.level_info().map(|level| level.dimension.as_str()),
        BlockModelDisplayTransform::default(),
        BlockModelDisplayTransform::default(),
        item_runtime,
        terrain_textures,
        variant_textures,
        &mut unused_block,
        &mut unused_block_translucent,
        &mut unused_block_glint,
        &mut unused_block_glint_translucent,
        &mut unused_flat,
        &mut unused_flat_translucent,
        &mut unused_flat_glint,
        &mut unused_flat_glint_translucent,
        residency,
        light,
        ITEM_MODEL_NO_OVERLAY,
        ItemModelDrawTarget::World,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_world::BlockPos;

    fn source(facing_2d: u8) -> CampfireItemDisplaySourceState {
        CampfireItemDisplaySourceState {
            pos: BlockPos { x: 2, y: 64, z: -3 },
            facing_2d,
            items: Vec::new(),
        }
    }

    #[test]
    fn campfire_slots_rotate_through_the_block_facing_data_value() {
        // `Direction.from2DDataValue((slot + facing) % 4)`: with a north-facing
        // (2) campfire, slot 0 points north and slot 2 wraps back to south.
        let north = source(2);
        let slot_zero = campfire_item_transform(&north, 0);
        let slot_two = campfire_item_transform(&north, 2);
        let south_slot_zero = campfire_item_transform(&source(0), 0);
        assert!(slot_zero.abs_diff_eq(campfire_item_transform(&source(0), 2), 1e-5));
        assert!(slot_two.abs_diff_eq(south_slot_zero, 1e-5));
    }

    #[test]
    fn campfire_items_lie_flat_in_their_quadrant_at_the_vanilla_scale() {
        let pos = Vec3::new(2.0, 64.0, -3.0);
        // The item model's own centre, `0.375` scaled and pushed by the
        // `-0.3125` in-plane offset, lands `0.125` from the block centre —
        // one quadrant of the campfire per slot.
        let centre = |slot| {
            campfire_item_transform(&source(0), slot).transform_point3(Vec3::new(0.5, 0.5, 0.0))
        };
        assert!((centre(0) - (pos + Vec3::new(0.375, 0.449_218_75, 0.375))).length() < 1e-5);
        assert!((centre(1) - (pos + Vec3::new(0.625, 0.449_218_75, 0.375))).length() < 1e-5);
        assert!((centre(2) - (pos + Vec3::new(0.625, 0.449_218_75, 0.625))).length() < 1e-5);
        assert!((centre(3) - (pos + Vec3::new(0.375, 0.449_218_75, 0.625))).length() < 1e-5);

        let transform = campfire_item_transform(&source(0), 0);
        let scale = transform.transform_vector3(Vec3::X).length();
        assert!((scale - CAMPFIRE_ITEM_SCALE).abs() < 1e-6);
        // `Axis.XP.rotationDegrees(90)` lays the item plane down.
        let plane_normal = transform.transform_vector3(Vec3::Z).normalize();
        assert!((plane_normal - Vec3::NEG_Y).length() < 1e-5);
    }
}
