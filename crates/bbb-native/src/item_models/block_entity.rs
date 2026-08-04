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
use bbb_world::{
    BlockPos, BrushableItemDisplaySourceState, CampfireItemDisplaySourceState,
    ShelfItemDisplaySourceState, VaultItemDisplaySourceState, WorldStore,
};
use glam::{Mat4, Vec3};

use bbb_item_model::NativeItemRuntime;

use super::dropped::{
    bake_item_stack_at_transform_with_light_and_overlay, cluster_transforms, item_cluster_seed,
    rendered_amount,
};
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// `CampfireRenderer.submit` centres the cooking items just above the log
/// surface before spinning them into their slot direction.
const CAMPFIRE_ITEM_ORIGIN: Vec3 = Vec3::new(0.5, 0.449_218_75, 0.5);
/// The in-plane centring translation applied after the two rotations.
const CAMPFIRE_ITEM_PLANE_OFFSET: Vec3 = Vec3::new(-0.3125, -0.3125, 0.0);
/// `CampfireRenderer.SIZE`.
const CAMPFIRE_ITEM_SCALE: f32 = 0.375;
/// `ShelfRenderer.ITEM_SIZE`.
const SHELF_ITEM_SCALE: f32 = 0.25;
/// `ShelfRenderer.ALIGN_ITEMS_TO_BOTTOM`.
const SHELF_ALIGN_TO_BOTTOM_OFFSET: f32 = -0.25;
/// The per-slot spacing and the shelf board's depth offset.
const SHELF_SLOT_SPACING: f32 = 0.3125;
const SHELF_SLOT_DEPTH: f32 = -0.25;
/// `BrushableBlockRenderer.submit` lifts the dug item to the block's mid height
/// before the per-side offset and the two fixed yaw rotations.
const BRUSHABLE_ITEM_LIFT: f32 = 0.5;
const BRUSHABLE_ITEM_SCALE: f32 = 0.5;
const BRUSHABLE_BASE_Y_ROT: f32 = 75.0;
const BRUSHABLE_EAST_WEST_Y_ROT: f32 = 90.0;
const BRUSHABLE_EXTRA_Y_ROT: f32 = 11.0;
/// `VaultRenderer.submit` spins the reward above the cage floor.
const VAULT_ITEM_ORIGIN: Vec3 = Vec3::new(0.5, 0.4, 0.5);

/// Bakes every block-entity renderer whose submission is an item stack.
#[allow(clippy::too_many_arguments)]
pub(crate) fn block_entity_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    renderer: Option<&bbb_renderer::Renderer>,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    partial_tick: f32,
) -> ItemModelResidencyFrame {
    let mut residency = ItemModelResidencyFrame::default();
    let Some(item_runtime) = item_runtime else {
        return residency;
    };
    for source in world.campfire_item_display_source_states() {
        let light = block_entity_shader_light(world, source.pos);
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
    for source in world.shelf_item_display_source_states() {
        let light = block_entity_shader_light(world, source.pos);
        for (slot, stack) in &source.items {
            // `ShelfRenderer.submitItem` seats the item on the shelf board from
            // the model's own bounding box, so the stack is baked once in model
            // space, measured, and then placed by the slot pose.
            let mut item = ItemModelResidencyFrame::default();
            bake_block_entity_item(
                stack,
                Mat4::IDENTITY,
                BlockModelDisplayContext::OnShelf,
                light,
                world,
                item_runtime,
                terrain_textures,
                trim_material_keys,
                enchantment_keys,
                damage_type_keys,
                attribute_keys,
                variant_textures,
                &mut item,
            );
            let Some((min, max)) = residency_model_extents(&item, renderer) else {
                continue;
            };
            let mut offset_y = -min.y;
            if !source.align_items_to_bottom {
                offset_y += -(max.y - min.y) / 2.0;
            }
            place_resident_item(
                &mut item,
                shelf_item_transform(&source, *slot) * Mat4::from_translation(Vec3::Y * offset_y),
            );
            residency.append(item);
        }
    }
    for source in world.brushable_item_display_source_states() {
        // `BrushableBlockRenderer.extractRenderState` samples the light of the
        // neighbour the block was brushed from, with the brushable block's own
        // state supplying the emissive facts.
        let light_pos = relative_block_pos(source.pos, source.hit_direction_3d);
        let coords = world
            .sample_render_light_for_state(source.block_state_id, light_pos)
            .map_or(0, |light| light.packed());
        bake_block_entity_item(
            &source.item,
            brushable_item_transform(&source),
            BlockModelDisplayContext::Fixed,
            shader_light(coords),
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
    for source in world.vault_item_display_source_states() {
        let light = block_entity_shader_light(world, source.pos);
        // `ItemEntityRenderer.renderMultipleFromCount` copies the same model
        // state, so the stack is baked once and posed per cluster copy.
        let mut item = ItemModelResidencyFrame::default();
        bake_block_entity_item(
            &source.item,
            Mat4::IDENTITY,
            BlockModelDisplayContext::Ground,
            light,
            world,
            item_runtime,
            terrain_textures,
            trim_material_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
            variant_textures,
            &mut item,
        );
        let Some((min, max)) = residency_model_extents(&item, renderer) else {
            continue;
        };
        let base = vault_item_transform(&source, partial_tick);
        for offset in cluster_transforms(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            rendered_amount(source.item.count),
            item_cluster_seed(&source.item),
            max.z - min.z,
        ) {
            let mut copy = item.clone();
            place_resident_item(&mut copy, base * offset);
            residency.append(copy);
        }
    }
    residency
}

/// `BrushableBlockRenderer.submit`: `T(0, 0.5, 0)`, the per-side dust
/// translation, `Ry(75)`, the east/west `Ry(90)` plus the fixed `Ry(11)`, and
/// the `0.5` item scale, all relative to the block's minimum corner.
fn brushable_item_transform(source: &BrushableItemDisplaySourceState) -> Mat4 {
    let east_west = matches!(source.hit_direction_3d, 4 | 5);
    let y_rot = BRUSHABLE_BASE_Y_ROT
        + if east_west {
            BRUSHABLE_EAST_WEST_Y_ROT
        } else {
            0.0
        }
        + BRUSHABLE_EXTRA_Y_ROT;
    Mat4::from_translation(Vec3::new(
        source.pos.x as f32,
        source.pos.y as f32,
        source.pos.z as f32,
    )) * Mat4::from_translation(Vec3::Y * BRUSHABLE_ITEM_LIFT)
        * Mat4::from_translation(brushable_dust_translation(
            source.hit_direction_3d,
            source.dust_progress,
        ))
        * Mat4::from_rotation_y(y_rot.to_radians())
        * Mat4::from_scale(Vec3::splat(BRUSHABLE_ITEM_SCALE))
}

/// `BrushableBlockRenderer.translations`: the dug item slides out of the
/// brushed face by `dusted / 10 * 0.75`.
fn brushable_dust_translation(hit_direction_3d: u8, dust_progress: i32) -> Vec3 {
    let completion_offset = dust_progress as f32 / 10.0 * 0.75;
    let mut translation = Vec3::new(0.5, 0.0, 0.5);
    match hit_direction_3d {
        // down
        0 => translation.y = -0.23 - completion_offset,
        // up
        1 => translation.y = 0.25 + completion_offset,
        // north
        2 => translation.z = 0.25 - completion_offset,
        // south
        3 => translation.z = 0.73 + completion_offset,
        // west
        4 => translation.x = 0.25 - completion_offset,
        // east
        _ => translation.x = 0.73 + completion_offset,
    }
    translation
}

/// `VaultRenderer.submit`: `T(0.5, 0.4, 0.5)` then the interpolated display
/// spin, relative to the block's minimum corner.
fn vault_item_transform(source: &VaultItemDisplaySourceState, partial_tick: f32) -> Mat4 {
    let spin = rot_lerp(partial_tick, source.previous_spin, source.current_spin);
    Mat4::from_translation(Vec3::new(
        source.pos.x as f32,
        source.pos.y as f32,
        source.pos.z as f32,
    )) * Mat4::from_translation(VAULT_ITEM_ORIGIN)
        * Mat4::from_rotation_y(spin.to_radians())
}

/// Vanilla `Mth.rotLerp`.
fn rot_lerp(alpha: f32, from: f32, to: f32) -> f32 {
    from + alpha * wrap_degrees(to - from)
}

/// Vanilla `Mth.wrapDegrees`.
fn wrap_degrees(value: f32) -> f32 {
    let wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped - 360.0
    } else if wrapped < -180.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}

/// Vanilla `BlockPos.relative(Direction)` for a `get3DDataValue` index.
fn relative_block_pos(pos: BlockPos, direction_3d: u8) -> BlockPos {
    let (dx, dy, dz) = match direction_3d {
        0 => (0, -1, 0),
        1 => (0, 1, 0),
        2 => (0, 0, -1),
        3 => (0, 0, 1),
        4 => (-1, 0, 0),
        _ => (1, 0, 0),
    };
    BlockPos {
        x: pos.x + dx,
        y: pos.y + dy,
        z: pos.z + dz,
    }
}

/// `ShelfRenderer.submitItem` without the bounding-box seating step:
/// `T(0.5,0.5,0.5)`, the block facing's `Ry(-facing.toYRot())`, the slot offset
/// `((slot - 1) * 0.3125, alignToBottom ? -0.25 : 0, -0.25)` and the `0.25`
/// item scale, all relative to the block's minimum corner.
fn shelf_item_transform(source: &ShelfItemDisplaySourceState, slot: u8) -> Mat4 {
    let slot_offset = Vec3::new(
        (f32::from(slot) - 1.0) * SHELF_SLOT_SPACING,
        if source.align_items_to_bottom {
            SHELF_ALIGN_TO_BOTTOM_OFFSET
        } else {
            0.0
        },
        SHELF_SLOT_DEPTH,
    );
    Mat4::from_translation(Vec3::new(
        source.pos.x as f32,
        source.pos.y as f32,
        source.pos.z as f32,
    )) * Mat4::from_translation(Vec3::splat(0.5))
        * Mat4::from_rotation_y((-horizontal_direction_y_rot(source.facing_2d)).to_radians())
        * Mat4::from_translation(slot_offset)
        * Mat4::from_scale(Vec3::splat(SHELF_ITEM_SCALE))
}

/// Applies one block-local pose to a frame baked in model space. Every resident
/// instance transform is `attach * display * leaf`, so the pose composes as a
/// left factor exactly as `PoseStack` does around `ItemStackRenderState.submit`.
fn place_resident_item(frame: &mut ItemModelResidencyFrame, attach: Mat4) {
    for instance in &mut frame.instances {
        instance.transform = attach * instance.transform;
    }
    for instance in &mut frame.special_items {
        instance.transform = attach * instance.transform;
    }
}

/// `ItemStackRenderState.getModelBoundingBox()`: every resident instance's
/// source extents pushed through that instance's own transform, exactly like
/// `visitExtents`.
fn residency_model_extents(
    frame: &ItemModelResidencyFrame,
    renderer: Option<&bbb_renderer::Renderer>,
) -> Option<(Vec3, Vec3)> {
    let mut box_min = Vec3::splat(f32::INFINITY);
    let mut box_max = Vec3::splat(f32::NEG_INFINITY);
    let mut include = |transform: Mat4, min: Vec3, max: Vec3| {
        for x in [min.x, max.x] {
            for y in [min.y, max.y] {
                for z in [min.z, max.z] {
                    let point = transform.transform_point3(Vec3::new(x, y, z));
                    box_min = box_min.min(point);
                    box_max = box_max.max(point);
                }
            }
        }
    };
    for instance in &frame.instances {
        let Some(asset) = frame
            .assets
            .iter()
            .find(|asset| asset.identity == instance.identity)
        else {
            continue;
        };
        if let Some((min, max)) = asset.source.model_extents() {
            include(instance.transform, min, max);
        }
    }
    for instance in &frame.special_items {
        let Some(bounds) =
            renderer.and_then(|renderer| renderer.special_item_model_bounds(&instance.model).ok())
        else {
            continue;
        };
        include(
            instance.transform,
            Vec3::from_array(bounds.min),
            Vec3::from_array(bounds.max),
        );
    }
    box_min.is_finite().then_some((box_min, box_max))
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
fn block_entity_shader_light(world: &WorldStore, pos: BlockPos) -> [f32; 2] {
    shader_light(
        world
            .sample_render_light(pos)
            .map_or(0, |light| light.packed()),
    )
}

fn shader_light(coords: u32) -> [f32; 2] {
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

    fn shelf_source(facing_2d: u8, align_items_to_bottom: bool) -> ShelfItemDisplaySourceState {
        ShelfItemDisplaySourceState {
            pos: BlockPos { x: 2, y: 64, z: -3 },
            facing_2d,
            align_items_to_bottom,
            items: Vec::new(),
        }
    }

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

    #[test]
    fn shelf_slots_spread_along_the_board_and_face_the_block_facing() {
        let pos = Vec3::new(2.0, 64.0, -3.0);
        let origin = |facing, align, slot| {
            shelf_item_transform(&shelf_source(facing, align), slot).transform_point3(Vec3::ZERO)
        };
        // South-facing shelf: `Ry(0)` leaves the `(slot - 1) * 0.3125` spacing
        // and the `-0.25` board depth in block axes.
        assert!((origin(0, false, 0) - (pos + Vec3::new(0.1875, 0.5, 0.25))).length() < 1e-5);
        assert!((origin(0, false, 1) - (pos + Vec3::new(0.5, 0.5, 0.25))).length() < 1e-5);
        assert!((origin(0, false, 2) - (pos + Vec3::new(0.8125, 0.5, 0.25))).length() < 1e-5);
        // North-facing shelf mirrors both axes through `Ry(-180)`.
        assert!((origin(2, false, 0) - (pos + Vec3::new(0.8125, 0.5, 0.75))).length() < 1e-5);
        // `alignItemsToBottom` drops the whole slot by a quarter block.
        assert!((origin(0, true, 0) - (pos + Vec3::new(0.1875, 0.25, 0.25))).length() < 1e-5);

        let scale = shelf_item_transform(&shelf_source(0, false), 0)
            .transform_vector3(Vec3::X)
            .length();
        assert!((scale - SHELF_ITEM_SCALE).abs() < 1e-6);
    }

    #[test]
    fn resident_item_placement_stays_a_left_factor_of_every_instance_transform() {
        // `place_resident_item` relies on the shared baking path applying its
        // `attach` argument as a strict left factor, so measuring in model
        // space and posing afterwards equals baking with the pose.
        let source = include_str!("dropped.rs");
        let start = source
            .find("pub(super) fn bake_item_stack_at_transform_with_light_and_overlay(")
            .expect("shared item baking entry");
        let body = &source[start..];
        let end = body.find("\npub(").unwrap_or(body.len());
        let body = &body[..end];
        assert!(
            !body.contains("* attach") && !body.contains("attach."),
            "attach must stay the outermost left factor of every baked transform"
        );
        assert!(body.contains("attach * display_matrix"));
    }

    #[test]
    fn brushable_items_slide_out_of_the_brushed_face_by_the_dust_stage() {
        let pos = Vec3::new(2.0, 64.0, -3.0);
        let source = |hit_direction_3d, dust_progress| BrushableItemDisplaySourceState {
            pos: BlockPos { x: 2, y: 64, z: -3 },
            hit_direction_3d,
            dust_progress,
            block_state_id: 0,
            item: bbb_protocol::packets::ItemStackSummary::empty(),
        };
        let origin =
            |dir, dust| brushable_item_transform(&source(dir, dust)).transform_point3(Vec3::ZERO);
        // `translations` keeps the two unused axes centred and pushes the third
        // by `dusted / 10 * 0.75` away from the brushed face.
        assert!((origin(3, 1) - (pos + Vec3::new(0.5, 0.5, 0.73 + 0.075))).length() < 1e-5);
        assert!((origin(2, 3) - (pos + Vec3::new(0.5, 0.5, 0.25 - 0.225))).length() < 1e-5);
        assert!((origin(1, 2) - (pos + Vec3::new(0.5, 0.5 + 0.25 + 0.15, 0.5))).length() < 1e-5);
        assert!((origin(0, 1) - (pos + Vec3::new(0.5, 0.5 - 0.23 - 0.075, 0.5))).length() < 1e-5);
        assert!((origin(5, 1) - (pos + Vec3::new(0.73 + 0.075, 0.5, 0.5))).length() < 1e-5);

        // East/west faces add the extra quarter turn on top of `75 + 11`.
        let yaw = |dir| {
            let axis = brushable_item_transform(&source(dir, 1)).transform_vector3(Vec3::X);
            (-axis.z).atan2(axis.x).to_degrees()
        };
        assert!((yaw(5) - yaw(3) - BRUSHABLE_EAST_WEST_Y_ROT).abs() < 1e-3);
    }

    #[test]
    fn vault_items_spin_above_the_cage_floor_from_the_ticked_client_data() {
        let source = VaultItemDisplaySourceState {
            pos: BlockPos { x: 2, y: 64, z: -3 },
            item: bbb_protocol::packets::ItemStackSummary::empty(),
            previous_spin: 170.0,
            current_spin: -180.0,
        };
        let transform = vault_item_transform(&source, 0.5);
        assert!(
            (transform.transform_point3(Vec3::ZERO) - Vec3::new(2.5, 64.4, -2.5)).length() < 1e-5
        );
        // `Mth.rotLerp` crosses the wrap boundary instead of sweeping backwards.
        let axis = transform.transform_vector3(Vec3::X);
        let spin = (-axis.z).atan2(axis.x).to_degrees();
        assert!((wrap_degrees(spin - 175.0)).abs() < 1e-3);
    }
}
