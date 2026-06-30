//! Dropped-item 3D models: turns dropped item entities into baked item-model meshes for the renderer's
//! item-model pass, replacing the flat billboard. A dropped item whose item is a block bakes its block
//! render shape over the blocks atlas (the block path); every other item extrudes its flat sprite into a
//! `1/16`-thick slab over the item atlas (the generated/flat path, vanilla `builtin/generated`). Both
//! are placed by vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the model's
//! GROUND display transform, and a stack of items renders as the vanilla cluster of `1..=5` jittered
//! copies (`submitMultipleFromCount`).

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_protocol::packets::{EquipmentSlot, ItemStackSummary};
use bbb_renderer::{
    allay_hand_attach_transform, bake_generated_item_quads,
    bake_item_model_mesh_with_light_and_overlay, bake_item_model_meshes_with_light,
    copper_golem_antenna_block_transform, copper_golem_hand_attach_transform,
    custom_head_item_transforms, dolphin_carried_item_transform, enderman_carried_block_transform,
    fox_held_item_transform, humanoid_hand_attach_transforms, iron_golem_flower_block_transform,
    minecart_tnt_display_block_transform, mooshroom_mushroom_block_transforms,
    panda_held_item_transform, snow_golem_head_block_transform,
    villager_crossed_arms_item_transform, witch_held_item_transform, EntityModelInstance,
    EntityModelKind, IllagerModelFamily, ItemModelMesh, ItemModelMeshSet, ItemModelQuad,
    MooshroomVariant, ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::{TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::item_runtime::NativeItemRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// Fallback GROUND display transform for a block item (`minecraft:block/block`): rotation `0`,
/// translation `[0, 3, 0]`/16, scale `0.25`. Used only when the item's own `ground` transform was not
/// retained; otherwise the item's retained per-model ground transform drives the seating.
const BLOCK_GROUND_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 0.0, 0.0],
    translation: [0.0, 3.0 / 16.0, 0.0],
    scale: [0.25, 0.25, 0.25],
};

/// Fallback GROUND display transform for a flat item (`minecraft:item/generated`): rotation `0`,
/// translation `[0, 2, 0]`/16, scale `0.5`.
const GENERATED_GROUND_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 0.0, 0.0],
    translation: [0.0, 2.0 / 16.0, 0.0],
    scale: [0.5, 0.5, 0.5],
};

/// Vanilla `FLAT_ITEM_DEPTH_THRESHOLD` / `ITEM_MIN_HOVER_HEIGHT`: a rendered model thinner than this in Z
/// is stacked back-to-front; a thicker one is scattered in 3D.
const FLAT_ITEM_DEPTH_THRESHOLD: f32 = 0.0625;
const CARVED_PUMPKIN_BLOCK_ID: &str = "minecraft:carved_pumpkin";
const POPPY_BLOCK_ID: &str = "minecraft:poppy";
const RED_MUSHROOM_BLOCK_ID: &str = "minecraft:red_mushroom";
const BROWN_MUSHROOM_BLOCK_ID: &str = "minecraft:brown_mushroom";

/// The baked item-model meshes for this frame, split by which atlas they sample (block-items → blocks
/// atlas, flat items → item atlas), plus the set of dropped-item entity ids they cover (so the billboard
/// path skips those entities and does not double-render them).
pub(crate) struct DroppedItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
    pub handled_entity_ids: BTreeSet<i32>,
}

struct EntityBlockAttachment {
    block_id: Cow<'static, str>,
    properties: BTreeMap<String, String>,
    transform: Mat4,
    light: [f32; 2],
    overlay: [f32; 2],
    outline_only: bool,
}

fn entity_render_state_shader_light(instance: &EntityModelInstance) -> [f32; 2] {
    let block = (instance.render_state.light_coords >> 4) & 0xF;
    let sky = (instance.render_state.light_coords >> 20) & 0xF;
    [block as f32 / 15.0, sky as f32 / 15.0]
}

fn minecart_tnt_display_block_overlay(instance: &EntityModelInstance) -> [f32; 2] {
    let fuse = instance.render_state.minecart_tnt_fuse_remaining_in_ticks;
    if fuse > -1.0 && (fuse as i32) / 5 % 2 == 0 {
        [15.0, 10.0]
    } else {
        ITEM_MODEL_NO_OVERLAY
    }
}

/// Bakes entity-attached block-model layers that sample the blocks atlas. This is the block-model
/// counterpart to held items: the renderer supplies the posed entity-bone transform, and native resolves
/// the block model through the terrain atlas state.
pub(crate) fn entity_block_models(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> Vec<ItemModelMesh> {
    let attachments = entity_block_attachments(instances, world, item_runtime);
    if attachments.is_empty() {
        return Vec::new();
    }

    let mut meshes = Vec::new();
    for attachment in attachments {
        if attachment.outline_only {
            continue;
        }
        let Some(quads) =
            terrain_textures.block_item_quads(attachment.block_id.as_ref(), &attachment.properties)
        else {
            continue;
        };
        if !quads.is_empty() {
            meshes.push(bake_item_model_mesh_with_light_and_overlay(
                &quads,
                attachment.transform,
                attachment.light,
                attachment.overlay,
            ));
        }
    }
    meshes
}

fn entity_block_attachments(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<EntityBlockAttachment> {
    let mut attachments = Vec::new();
    for instance in instances {
        if let Some(transform) = snow_golem_head_block_transform(instance) {
            attachments.push(EntityBlockAttachment {
                block_id: Cow::Borrowed(CARVED_PUMPKIN_BLOCK_ID),
                properties: carved_pumpkin_default_properties(),
                transform,
                light: entity_render_state_shader_light(instance),
                overlay: ITEM_MODEL_NO_OVERLAY,
                outline_only: entity_block_attachment_outline_only(instance),
            });
        }
        if let Some(transform) = iron_golem_flower_block_transform(instance) {
            attachments.push(EntityBlockAttachment {
                block_id: Cow::Borrowed(POPPY_BLOCK_ID),
                properties: poppy_default_properties(),
                transform,
                light: entity_render_state_shader_light(instance),
                overlay: ITEM_MODEL_NO_OVERLAY,
                outline_only: false,
            });
        }
        if let Some(transform) = enderman_carried_block_transform(instance) {
            if let Some(block_state) = world.enderman_carried_block_state(instance.entity_id) {
                attachments.push(EntityBlockAttachment {
                    block_id: Cow::Owned(block_state.name),
                    properties: block_state.properties,
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: ITEM_MODEL_NO_OVERLAY,
                    outline_only: false,
                });
            }
        }
        if let Some(display) = world.minecart_display_block_state(instance.entity_id) {
            if let Some(transform) =
                minecart_tnt_display_block_transform(instance, display.display_offset)
            {
                attachments.push(EntityBlockAttachment {
                    block_id: Cow::Owned(display.block.name),
                    properties: display.block.properties,
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: minecart_tnt_display_block_overlay(instance),
                    outline_only: entity_block_attachment_outline_only(instance),
                });
            }
        }
        if let Some((variant, transforms)) = mooshroom_mushroom_block_transforms(instance) {
            for transform in transforms {
                attachments.push(EntityBlockAttachment {
                    block_id: Cow::Borrowed(mooshroom_mushroom_block_id(variant)),
                    properties: mooshroom_mushroom_default_properties(),
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: ITEM_MODEL_NO_OVERLAY,
                    outline_only: entity_block_attachment_outline_only(instance),
                });
            }
        }
        if let Some(attachment) =
            copper_golem_antenna_block_attachment(instance, world, item_runtime)
        {
            attachments.push(attachment);
        }
    }
    attachments
}

fn entity_block_attachment_outline_only(instance: &EntityModelInstance) -> bool {
    instance.render_state.invisible && instance.render_state.appears_glowing
}

fn copper_golem_antenna_block_attachment(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<EntityBlockAttachment> {
    let item_runtime = item_runtime?;
    let stack = world.equipment_item(instance.entity_id, EquipmentSlot::Saddle)?;
    let item_id = stack.item_id?;
    let block_id = item_runtime.item_resource_id(item_id)?;
    copper_golem_antenna_block_attachment_from_stack(instance, &stack, block_id)
}

fn copper_golem_antenna_block_attachment_from_stack(
    instance: &EntityModelInstance,
    stack: &ItemStackSummary,
    block_id: &str,
) -> Option<EntityBlockAttachment> {
    let transform = copper_golem_antenna_block_transform(instance)?;
    Some(EntityBlockAttachment {
        block_id: Cow::Owned(block_id.to_string()),
        properties: stack.component_patch.block_state_properties.clone(),
        transform,
        light: entity_render_state_shader_light(instance),
        overlay: ITEM_MODEL_NO_OVERLAY,
        outline_only: false,
    })
}

/// Vanilla `Blocks.CARVED_PUMPKIN.defaultBlockState()` sets `FACING = NORTH`; the snow-golem head layer
/// uses that exact blockstate.
fn carved_pumpkin_default_properties() -> BTreeMap<String, String> {
    BTreeMap::from([("facing".to_string(), "north".to_string())])
}

/// Vanilla `Blocks.POPPY.defaultBlockState()` has no properties; the iron-golem flower layer uses that
/// exact blockstate while `offerFlowerTick > 0`.
fn poppy_default_properties() -> BTreeMap<String, String> {
    BTreeMap::new()
}

fn mooshroom_mushroom_block_id(variant: MooshroomVariant) -> &'static str {
    match variant {
        MooshroomVariant::Red => RED_MUSHROOM_BLOCK_ID,
        MooshroomVariant::Brown => BROWN_MUSHROOM_BLOCK_ID,
    }
}

/// Vanilla `MushroomCow.Variant` stores `Blocks.RED_MUSHROOM.defaultBlockState()` or
/// `Blocks.BROWN_MUSHROOM.defaultBlockState()`, both property-less states.
fn mooshroom_mushroom_default_properties() -> BTreeMap<String, String> {
    BTreeMap::new()
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
    let mut block_translucent_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut flat_translucent_meshes = Vec::new();
    let mut handled_entity_ids = BTreeSet::new();
    let Some(item_runtime) = item_runtime else {
        return DroppedItemModels {
            block_meshes,
            block_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
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

        // The item's own retained GROUND display transform (custom ground rotation / scale / offset for
        // items that define one); falls back to the vanilla `block/block` or `item/generated` default.
        let ground = |fallback| {
            item_runtime
                .item_display_transform(item_id, BlockModelDisplayContext::Ground)
                .unwrap_or(fallback)
        };

        // Block path: the item is a block with 3D item geometry.
        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    push_mesh_set(
                        stacked_item_mesh(
                            &quads,
                            position,
                            age_ticks,
                            state.entity_id,
                            &ground(BLOCK_GROUND_FALLBACK),
                            shader_light(state.light),
                            count,
                            seed,
                        ),
                        &mut block_meshes,
                        &mut block_translucent_meshes,
                    );
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
        push_mesh_set(
            stacked_item_mesh(
                &quads,
                position,
                age_ticks,
                state.entity_id,
                &ground(GENERATED_GROUND_FALLBACK),
                shader_light(state.light),
                count,
                seed,
            ),
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        handled_entity_ids.insert(state.entity_id);
    }

    DroppedItemModels {
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
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

/// Fallback HEAD display transform for a block item whose model does not define one. Vanilla's
/// `block/block` parent has no `head` entry, so the item transform is the identity centered on the
/// model by [`display_matrix`].
const BLOCK_HEAD_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 0.0, 0.0],
    translation: [0.0, 0.0, 0.0],
    scale: [1.0, 1.0, 1.0],
};

/// Fallback HEAD display transform for `builtin/generated` items: vanilla `item/generated.json`
/// rotation `[0, 180, 0]`, translation `[0, 13, 7]` pixels, scale `1`.
const GENERATED_HEAD_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 180.0, 0.0],
    translation: [0.0, 13.0 / 16.0, 7.0 / 16.0],
    scale: [1.0, 1.0, 1.0],
};

/// The display transform about the model center: `T(t) · Rxyz · S · T(-0.5)` (vanilla
/// `ItemTransform.apply`). `Rxyz` matches joml `rotationXYZ`. The pack's `BlockModelDisplayTransform`
/// already holds translation in world units (the model JSON's 1/16 value pre-multiplied) and per-axis
/// scale, so no further normalization is applied here. `left_hand` is vanilla's left-hand fix: it negates
/// `translation.x`, `rotation.y`, and `rotation.z` so an item mirrors correctly into the left arm.
pub(crate) fn display_matrix(display: &BlockModelDisplayTransform, left_hand: bool) -> Mat4 {
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
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
}

/// Bakes the third-person main- and off-hand held items for every humanoid entity that holds one
/// (players and the weapon-holding mobs — zombies, giants, skeletons, piglins, illagers; vanilla
/// `ItemInHandLayer`) plus non-humanoid `ArmedModel` users such as allays and copper golems. The hand
/// attach transform comes from the renderer's posed model; native resolves the item to quads (block or
/// flat) and applies the item's third-person display transform.
pub(crate) fn held_item_models(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> HeldItemModels {
    let mut block_meshes = Vec::new();
    let mut block_translucent_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut flat_translucent_meshes = Vec::new();
    let Some(item_runtime) = item_runtime else {
        return HeldItemModels {
            block_meshes,
            block_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
        };
    };

    for instance in instances {
        // Vanilla `ItemInHandLayer.submit` submits the right arm first and the left arm second. Each arm's
        // stack comes from `LivingEntity.getItemHeldByArm(arm)`, so a left-main-arm entity swaps which
        // logical hand slot is attached to the right/left arm.
        bake_held_hand(
            instance,
            false,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_held_hand(
            instance,
            true,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_fox_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_dolphin_carried_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_witch_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_copper_golem_held_items(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_allay_held_items(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_villager_crossed_arms_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_panda_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
        bake_custom_head_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
        );
    }

    HeldItemModels {
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
    }
}

/// Bakes one arm's held item onto its arm bone with that arm's own
/// `thirdperson_{left,right}hand` display transform. The logical hand slot is
/// resolved from the entity's main arm, mirroring vanilla `getItemHeldByArm`.
#[allow(clippy::too_many_arguments)]
fn bake_held_hand(
    instance: &EntityModelInstance,
    left_arm: bool,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let off_hand = left_arm != instance.render_state.main_arm_left;
    let Some(stack) = world.held_item(instance.entity_id, off_hand) else {
        return;
    };
    if !humanoid_item_in_hand_layer_visible(instance) {
        return;
    }

    // The item's own retained third-person transform for this arm (handheld tools angle the model,
    // blocks tilt it, generated items lay it flat); falls back to the parent-model default per path.
    let context = if left_arm {
        BlockModelDisplayContext::ThirdPersonLeftHand
    } else {
        BlockModelDisplayContext::ThirdPersonRightHand
    };
    for hand in humanoid_hand_attach_transforms(instance, left_arm) {
        bake_item_stack_at_transform(
            &stack,
            hand,
            context,
            left_arm,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
        );
    }
}

fn humanoid_item_in_hand_layer_visible(instance: &EntityModelInstance) -> bool {
    !matches!(
        instance.kind,
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
    ) || instance.render_state.illager_spellcasting
        || instance.render_state.is_aggressive
}

/// Bakes a fox's main-hand stack in its mouth. Vanilla `FoxHeldItemLayer` is backed by
/// `HoldingEntityRenderState.extractHoldingEntityRenderState`, which resolves `entity.getMainHandItem()`
/// with `ItemDisplayContext.GROUND` before applying the fox-head layer transform.
#[allow(clippy::too_many_arguments)]
fn bake_fox_held_item(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = fox_held_item_transform(instance) else {
        return;
    };
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
    );
}

/// Bakes a dolphin's carried main-hand stack. Vanilla `DolphinCarryingItemLayer` shares
/// `HoldingEntityRenderState` with foxes, so it reads `entity.getMainHandItem()` resolved in the
/// `GROUND` display context and places it at a pitch-adjusted entity-root offset.
#[allow(clippy::too_many_arguments)]
fn bake_dolphin_carried_item(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = dolphin_carried_item_transform(instance) else {
        return;
    };
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
    );
}

/// Bakes a witch's main-hand stack through `WitchItemLayer`: potions attach to the posed nose, while all
/// other held items sit in the crossed arms. Vanilla resolves the stack through `HoldingEntityRenderState`
/// with `ItemDisplayContext.GROUND`.
#[allow(clippy::too_many_arguments)]
fn bake_witch_held_item(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = witch_held_item_transform(instance) else {
        return;
    };
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
    );
}

/// Bakes a copper golem's standard `ItemInHandLayer` hand items. Vanilla uses the normal third-person
/// left/right contexts, with `CopperGolemModel.translateToHand` supplying the posed arm transform.
#[allow(clippy::too_many_arguments)]
fn bake_copper_golem_held_items(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    for left_hand in [false, true] {
        let Some(stack) = world.held_item(instance.entity_id, left_hand) else {
            continue;
        };
        let Some(hand) = copper_golem_hand_attach_transform(instance, left_hand) else {
            continue;
        };
        let context = if left_hand {
            BlockModelDisplayContext::ThirdPersonLeftHand
        } else {
            BlockModelDisplayContext::ThirdPersonRightHand
        };
        bake_item_stack_at_transform(
            &stack,
            hand,
            context,
            left_hand,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
        );
    }
}

/// Bakes an allay's standard `ItemInHandLayer` hand items. Vanilla uses
/// `ArmedEntityRenderState.extractArmedEntityRenderState`, so the right/left item states use
/// third-person right/left hand contexts even though `AllayModel.translateToHand` itself ignores the
/// arm parameter before the shared layer offset.
#[allow(clippy::too_many_arguments)]
fn bake_allay_held_items(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    for left_hand in [false, true] {
        let Some(stack) = world.held_item(instance.entity_id, left_hand) else {
            continue;
        };
        let Some(hand) = allay_hand_attach_transform(instance, left_hand) else {
            continue;
        };
        let context = if left_hand {
            BlockModelDisplayContext::ThirdPersonLeftHand
        } else {
            BlockModelDisplayContext::ThirdPersonRightHand
        };
        bake_item_stack_at_transform(
            &stack,
            hand,
            context,
            left_hand,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
        );
    }
}

/// Bakes the main-hand stack rendered by vanilla `CrossedArmsItemLayer` for villagers and wandering
/// traders. `HoldingEntityRenderState` resolves this layer with `ItemDisplayContext.GROUND`.
#[allow(clippy::too_many_arguments)]
fn bake_villager_crossed_arms_item(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = villager_crossed_arms_item_transform(instance) else {
        return;
    };
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
    );
}

/// Bakes the main-hand stack rendered by vanilla `PandaHoldsItemLayer`. `HoldingEntityRenderState`
/// resolves the stack in `ItemDisplayContext.GROUND`; the renderer transform already gates sitting/scared
/// and applies the eating bob.
#[allow(clippy::too_many_arguments)]
fn bake_panda_held_item(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = panda_held_item_transform(instance) else {
        return;
    };
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
    );
}

/// Bakes the non-skull, non-armor item rendered by vanilla `CustomHeadLayer` from the HEAD equipment
/// slot. Skull block items use `SkullBlockRenderer` in vanilla and stay deferred to the dedicated skull
/// path; humanoid armor with an equipment asset is already emitted by the armor overlay.
#[allow(clippy::too_many_arguments)]
fn bake_custom_head_item(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.equipment_item(instance.entity_id, EquipmentSlot::Head) else {
        return;
    };
    let Some(item_id) = stack.item_id else {
        return;
    };
    if item_runtime.item_has_humanoid_armor_asset(item_id) {
        return;
    }
    if item_runtime
        .item_resource_id(item_id)
        .is_some_and(is_custom_head_skull_item_id)
    {
        return;
    }
    for transform in custom_head_item_transforms(instance) {
        bake_item_stack_at_transform(
            &stack,
            transform,
            BlockModelDisplayContext::Head,
            false,
            BLOCK_HEAD_FALLBACK,
            GENERATED_HEAD_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
        );
    }
}

fn is_custom_head_skull_item_id(resource_id: &str) -> bool {
    matches!(
        resource_id,
        "minecraft:skeleton_skull"
            | "minecraft:wither_skeleton_skull"
            | "minecraft:player_head"
            | "minecraft:zombie_head"
            | "minecraft:creeper_head"
            | "minecraft:dragon_head"
            | "minecraft:piglin_head"
    )
}

/// Bakes one item stack at an entity-supplied attach transform, applying the stack's retained item
/// display transform for the selected vanilla context and falling back to the parent-model default for
/// block vs generated items.
#[allow(clippy::too_many_arguments)]
fn bake_item_stack_at_transform(
    stack: &ItemStackSummary,
    attach: Mat4,
    context: BlockModelDisplayContext,
    left_hand: bool,
    block_fallback: BlockModelDisplayTransform,
    generated_fallback: BlockModelDisplayTransform,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(item_id) = stack.item_id else {
        return;
    };
    let retained = item_runtime.item_display_transform(item_id, context);

    // Block path.
    if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
        if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
            if !quads.is_empty() {
                let display = retained.unwrap_or(block_fallback);
                let transform = attach * display_matrix(&display, left_hand);
                push_mesh_set(
                    bake_item_model_meshes_with_light(
                        &quads,
                        transform,
                        ITEM_MODEL_FULL_BRIGHT_LIGHT,
                    ),
                    block_meshes,
                    block_translucent_meshes,
                );
                return;
            }
        }
    }

    // Flat path.
    let mut quads: Vec<ItemModelQuad> = Vec::new();
    for layer in item_runtime.generated_item_layers_for_stack(stack) {
        quads.extend(bake_generated_item_quads(
            &layer.mask,
            layer.rect,
            layer.tint,
        ));
    }
    if quads.is_empty() {
        return;
    }
    let display = retained.unwrap_or(generated_fallback);
    let transform = attach * display_matrix(&display, left_hand);
    push_mesh_set(
        bake_item_model_meshes_with_light(&quads, transform, ITEM_MODEL_FULL_BRIGHT_LIGHT),
        flat_meshes,
        flat_translucent_meshes,
    );
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

/// Bakes the cluster of copies for one dropped item into a single mesh. `ground` is the item's GROUND
/// display transform; the seating lift and cluster layout are derived from the resulting model bounds,
/// exactly as vanilla derives them from `modelBoundingBox`.
fn stacked_item_mesh(
    quads: &[ItemModelQuad],
    position: [f32; 3],
    age_ticks: f32,
    entity_id: i32,
    ground: &BlockModelDisplayTransform,
    light: [f32; 2],
    count: usize,
    seed: i64,
) -> ItemModelMeshSet {
    let ground_matrix = display_matrix(ground, false);
    // Vanilla `ItemEntityRenderer.submit`: `minOffsetY = -modelBoundingBox.minY + 1/16` seats the
    // rendered model on the ground; `getZsize()` picks the cluster layout (3D scatter vs back-to-front).
    let (min_y, depth) = ground_model_bounds(quads, ground_matrix);
    let min_offset_y = -min_y + 1.0 / 16.0;
    let base = base_transform(position, age_ticks, entity_id, min_offset_y);
    let mut mesh = ItemModelMeshSet::default();
    append_cluster(
        &mut mesh,
        quads,
        base,
        ground_matrix,
        light,
        count,
        seed,
        depth,
    );
    mesh
}

fn shader_light(light: TerrainLight) -> [f32; 2] {
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
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

/// The rendered model's floor and Z thickness (vanilla `modelBoundingBox.minY` / `getZsize()`): each quad
/// corner normalized to the unit cube (`/16`) and pushed through the GROUND display matrix, reduced to the
/// minimum world Y and the Z extent. The full display matrix (rotation + per-axis scale + offset) is
/// applied, so a custom ground transform seats and clusters exactly as vanilla's `modelBoundingBox` does.
fn ground_model_bounds(quads: &[ItemModelQuad], ground_matrix: Mat4) -> (f32, f32) {
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for quad in quads {
        for corner in quad.corners {
            let point = ground_matrix.transform_point3(Vec3::from_array(corner) / 16.0);
            min_y = min_y.min(point.y);
            min_z = min_z.min(point.z);
            max_z = max_z.max(point.z);
        }
    }
    if !min_y.is_finite() {
        return (0.0, 0.0);
    }
    (min_y, max_z - min_z)
}

/// Vanilla `ItemEntityRenderer.submitMultipleFromCount`: bake `count` copies of the model. A model
/// thicker than [`FLAT_ITEM_DEPTH_THRESHOLD`] scatters its copies in 3D; a thin (flat) one stacks them
/// back-to-front along Z with small in-plane jitter. The jitter draws from a Java-seeded RNG so the
/// cluster arrangement matches vanilla.
fn push_mesh_set(
    meshes: ItemModelMeshSet,
    solid: &mut Vec<ItemModelMesh>,
    translucent: &mut Vec<ItemModelMesh>,
) {
    if !meshes.solid.is_empty() {
        solid.push(meshes.solid);
    }
    if !meshes.translucent.is_empty() {
        translucent.push(meshes.translucent);
    }
}

fn append_quads_to_mesh_set(
    meshes: &mut ItemModelMeshSet,
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
) {
    for quad in quads {
        let mesh = if quad.translucent {
            &mut meshes.translucent
        } else {
            &mut meshes.solid
        };
        mesh.append_quads_with_light(std::slice::from_ref(quad), transform, light);
    }
}

fn append_cluster(
    mesh: &mut ItemModelMeshSet,
    quads: &[ItemModelQuad],
    base: Mat4,
    ground: Mat4,
    light: [f32; 2],
    count: usize,
    seed: i64,
    depth: f32,
) {
    let mut random = LegacyRandom::new(seed);
    if depth > FLAT_ITEM_DEPTH_THRESHOLD {
        append_quads_to_mesh_set(mesh, quads, base * ground, light);
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let zo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let offset = Mat4::from_translation(Vec3::new(xo, yo, zo));
            append_quads_to_mesh_set(mesh, quads, base * offset * ground, light);
        }
    } else {
        let offset_z = depth * 1.5;
        let mut z = -(offset_z * (count as f32 - 1.0) / 2.0);
        append_quads_to_mesh_set(
            mesh,
            quads,
            base * Mat4::from_translation(Vec3::new(0.0, 0.0, z)) * ground,
            light,
        );
        z += offset_z;
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let offset = Mat4::from_translation(Vec3::new(xo, yo, z));
            append_quads_to_mesh_set(mesh, quads, base * offset * ground, light);
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
    use bbb_protocol::packets::{
        AddEntity, DataComponentPatchSummary, EntityDataValue, EntityDataValueKind,
        EquipmentSlotUpdate, SetEntityData, SetEquipment, Vec3d,
    };
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_ENDERMAN_ID: i32 = 41;
    const VANILLA_ENTITY_TYPE_CHEST_MINECART_ID: i32 = 25;
    const VANILLA_ENTITY_TYPE_TNT_MINECART_ID: i32 = 133;
    const ENDERMAN_CARRY_STATE_DATA_ID: u8 = 16;
    const OPTIONAL_BLOCK_STATE_SERIALIZER_ID: i32 = 15;
    const GRASS_BLOCK_STATE_ID: i32 = 9;

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
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        }]
    }

    fn generated_slab_quads() -> Vec<ItemModelQuad> {
        // A generated-item slab: full 0..16 in X/Y, thin Z 7.5..8.5 (the extruded sprite face).
        vec![ItemModelQuad {
            corners: [
                [0.0, 0.0, 7.5],
                [16.0, 0.0, 7.5],
                [16.0, 16.0, 8.5],
                [0.0, 16.0, 8.5],
            ],
            uvs: [[0.0, 0.0]; 4],
            tint: [1.0; 4],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        }]
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
            entity_type_id,
            position: Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn protocol_optional_block_state_data(
        data_id: u8,
        block_state: Option<i32>,
    ) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: OPTIONAL_BLOCK_STATE_SERIALIZER_ID,
            value: EntityDataValueKind::OptionalBlockState(block_state),
        }
    }

    fn world_with_enderman_carried_grass_block(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            entity_id,
            VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: entity_id,
            values: vec![protocol_optional_block_state_data(
                ENDERMAN_CARRY_STATE_DATA_ID,
                Some(GRASS_BLOCK_STATE_ID),
            )],
        }));
        world
    }

    fn world_with_chest_minecart(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            entity_id,
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        ));
        world
    }

    fn world_with_tnt_minecart(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            entity_id,
            VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        ));
        world
    }

    #[test]
    fn carved_pumpkin_layer_uses_vanilla_default_block_state() {
        assert_eq!(
            carved_pumpkin_default_properties(),
            BTreeMap::from([("facing".to_string(), "north".to_string())])
        );
    }

    #[test]
    fn poppy_layer_uses_vanilla_default_block_state() {
        assert_eq!(poppy_default_properties(), BTreeMap::new());
    }

    #[test]
    fn mooshroom_mushroom_layers_use_vanilla_default_block_states() {
        assert_eq!(
            mooshroom_mushroom_block_id(MooshroomVariant::Red),
            RED_MUSHROOM_BLOCK_ID
        );
        assert_eq!(
            mooshroom_mushroom_block_id(MooshroomVariant::Brown),
            BROWN_MUSHROOM_BLOCK_ID
        );
        assert_eq!(mooshroom_mushroom_default_properties(), BTreeMap::new());
    }

    #[test]
    fn entity_block_attachments_collect_snow_golem_iron_golem_and_mooshroom_layers() {
        let world = WorldStore::new();
        let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
            .with_snow_golem_pumpkin(true);
        let iron_golem = EntityModelInstance::iron_golem(74, [1.0, 64.0, 0.0], 0.0)
            .with_iron_golem_offer_flower_tick(400);
        let idle_iron_golem = EntityModelInstance::iron_golem(75, [2.0, 64.0, 0.0], 0.0);
        let mooshroom = EntityModelInstance::mooshroom(86, [3.0, 64.0, 0.0], 0.0, false);
        let baby_mooshroom = EntityModelInstance::mooshroom(87, [4.0, 64.0, 0.0], 0.0, true);

        let attachments = entity_block_attachments(
            &[
                snow_golem,
                iron_golem,
                idle_iron_golem,
                mooshroom,
                baby_mooshroom,
            ],
            &world,
            None,
        );

        assert_eq!(attachments.len(), 5);
        assert!(attachments
            .iter()
            .all(|attachment| !attachment.outline_only));
        assert_eq!(attachments[0].block_id.as_ref(), CARVED_PUMPKIN_BLOCK_ID);
        assert_eq!(
            attachments[0].properties,
            carved_pumpkin_default_properties()
        );
        assert_eq!(attachments[1].block_id.as_ref(), POPPY_BLOCK_ID);
        assert_eq!(attachments[1].properties, poppy_default_properties());
        for attachment in &attachments[2..] {
            assert_eq!(attachment.block_id.as_ref(), RED_MUSHROOM_BLOCK_ID);
            assert_eq!(
                attachment.properties,
                mooshroom_mushroom_default_properties()
            );
        }
    }

    #[test]
    fn entity_block_attachments_record_invisible_glowing_outline_only_layers() {
        let world = WorldStore::new();
        let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
            .with_snow_golem_pumpkin(true)
            .with_invisible(true)
            .with_appears_glowing(true);
        let mooshroom = EntityModelInstance::mooshroom(86, [3.0, 64.0, 0.0], 0.0, false)
            .with_invisible(true)
            .with_appears_glowing(true);

        let attachments = entity_block_attachments(&[snow_golem, mooshroom], &world, None);

        assert_eq!(attachments.len(), 4);
        assert!(attachments.iter().all(|attachment| attachment.outline_only));
        assert_eq!(attachments[0].block_id.as_ref(), CARVED_PUMPKIN_BLOCK_ID);
        assert_eq!(
            attachments[0].properties,
            carved_pumpkin_default_properties()
        );
        for attachment in &attachments[1..] {
            assert_eq!(attachment.block_id.as_ref(), RED_MUSHROOM_BLOCK_ID);
            assert_eq!(
                attachment.properties,
                mooshroom_mushroom_default_properties()
            );
        }
    }

    #[test]
    fn entity_block_attachments_collect_enderman_carried_block_from_world() {
        let entity_id = 41;
        let world = world_with_enderman_carried_grass_block(entity_id);
        let light_coords = (6_u32 << 4) | (10_u32 << 20);
        let enderman = EntityModelInstance::enderman(entity_id, [0.0, 64.0, 0.0], 0.0)
            .with_enderman_carrying(true)
            .with_light_coords(light_coords);

        let attachments = entity_block_attachments(&[enderman], &world, None);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
        assert_eq!(
            attachments[0].properties,
            BTreeMap::from([("snowy".to_string(), "false".to_string())])
        );
        assert_eq!(attachments[0].light, [6.0 / 15.0, 10.0 / 15.0]);
    }

    #[test]
    fn entity_block_attachments_collect_minecart_display_block_from_world() {
        let entity_id = 85;
        let world = world_with_chest_minecart(entity_id);
        let light_coords = (6_u32 << 4) | (10_u32 << 20);
        let minecart = EntityModelInstance::minecart(entity_id, [0.0, 64.0, 0.0], 45.0)
            .with_light_coords(light_coords);

        let attachments = entity_block_attachments(&[minecart], &world, None);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].block_id.as_ref(), "minecraft:chest");
        assert_eq!(
            attachments[0].properties,
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), "single".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ])
        );
        assert_eq!(attachments[0].light, [6.0 / 15.0, 10.0 / 15.0]);
        assert_eq!(attachments[0].overlay, ITEM_MODEL_NO_OVERLAY);
        assert!(!attachments[0].outline_only);
        assert!(attachments[0]
            .transform
            .transform_point3(Vec3::splat(0.5))
            .is_finite());

        let hidden = minecart.with_invisible(true);
        assert!(entity_block_attachments(&[hidden], &world, None).is_empty());

        let outline = hidden.with_appears_glowing(true);
        let outline_attachments = entity_block_attachments(&[outline], &world, None);
        assert_eq!(outline_attachments.len(), 1);
        assert!(outline_attachments[0].outline_only);
    }

    #[test]
    fn entity_block_attachments_apply_tnt_minecart_fuse_scale_and_white_overlay() {
        let entity_id = 133;
        let world = world_with_tnt_minecart(entity_id);
        let light_coords = (5_u32 << 4) | (12_u32 << 20);
        let primed = EntityModelInstance::minecart(entity_id, [0.0, 64.0, 0.0], 0.0)
            .with_light_coords(light_coords)
            .with_minecart_tnt_fuse_remaining_in_ticks(4.5);

        let attachments = entity_block_attachments(&[primed], &world, None);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].block_id.as_ref(), "minecraft:tnt");
        assert_eq!(
            attachments[0].properties,
            BTreeMap::from([("unstable".to_string(), "false".to_string())])
        );
        assert_eq!(attachments[0].light, [5.0 / 15.0, 12.0 / 15.0]);
        assert_eq!(attachments[0].overlay, [15.0, 10.0]);
        assert_eq!(
            attachments[0].transform,
            minecart_tnt_display_block_transform(&primed, 6).unwrap()
        );

        let unprimed = primed.with_minecart_tnt_fuse_remaining_in_ticks(-1.0);
        let unprimed_attachment = entity_block_attachments(&[unprimed], &world, None)
            .pop()
            .expect("unprimed tnt attachment");
        assert_eq!(unprimed_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
        assert_ne!(unprimed_attachment.transform, attachments[0].transform);

        let odd_strobe = primed.with_minecart_tnt_fuse_remaining_in_ticks(6.0);
        let odd_strobe_attachment = entity_block_attachments(&[odd_strobe], &world, None)
            .pop()
            .expect("odd strobe tnt attachment");
        assert_eq!(odd_strobe_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
    }

    #[test]
    fn copper_golem_antenna_attachment_uses_block_state_component_properties() {
        let copper_golem = EntityModelInstance::new(
            28,
            bbb_renderer::EntityModelKind::CopperGolem {
                weathering: bbb_renderer::CopperGolemWeathering::Unaffected,
            },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let stack = ItemStackSummary {
            item_id: Some(901),
            count: 1,
            component_patch: DataComponentPatchSummary {
                block_state_properties: BTreeMap::from([
                    ("facing".to_string(), "south".to_string()),
                    ("powered".to_string(), "true".to_string()),
                ]),
                ..DataComponentPatchSummary::default()
            },
        };

        let attachment = copper_golem_antenna_block_attachment_from_stack(
            &copper_golem,
            &stack,
            "minecraft:oak_button",
        )
        .unwrap();

        assert_eq!(attachment.block_id.as_ref(), "minecraft:oak_button");
        assert_eq!(
            attachment.properties,
            BTreeMap::from([
                ("facing".to_string(), "south".to_string()),
                ("powered".to_string(), "true".to_string()),
            ])
        );
        assert!(attachment
            .transform
            .transform_point3(Vec3::splat(0.5))
            .is_finite());
        assert!(copper_golem_antenna_block_attachment_from_stack(
            &EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0),
            &stack,
            "minecraft:oak_button",
        )
        .is_none());
    }

    #[test]
    fn block_ground_transform_seats_a_unit_block_just_above_the_entity_origin() {
        // The 0.25-scaled block centered then lifted +3/16 sits at y in [0.0625, 0.3125], plus the bob.
        let ground_matrix = display_matrix(&BLOCK_GROUND_FALLBACK, false);
        let (min_y, _) = ground_model_bounds(&unit_block_quads(), ground_matrix);
        let min_offset_y = -min_y + 1.0 / 16.0;
        // A full unit block already rests on the ground under the default transform (no extra lift).
        assert!(min_offset_y.abs() < 1e-4, "block needs no lift");
        let transform = base_transform([0.0, 64.0, 0.0], 0.0, 0, min_offset_y) * ground_matrix;
        let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.0));
        let top = transform.transform_point3(Vec3::new(1.0, 1.0, 1.0));
        assert!((bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4, "bottom y");
        assert!((top.y - (64.0 + bob + 0.3125)).abs() < 1e-4, "top y");
    }

    #[test]
    fn flat_ground_transform_lifts_the_slab_to_rest_on_the_ground() {
        let ground_matrix = display_matrix(&GENERATED_GROUND_FALLBACK, false);
        let (min_y, _) = ground_model_bounds(&generated_slab_quads(), ground_matrix);
        let min_offset_y = -min_y + 1.0 / 16.0;
        // The 0.5-scaled slab's bottom sits at -0.125, so vanilla lifts it 0.1875 to rest on the ground.
        assert!((min_offset_y - 0.1875).abs() < 1e-4, "flat lift");
        let transform = base_transform([0.0, 64.0, 0.0], 0.0, 0, min_offset_y) * ground_matrix;
        let bob = (bob_offset(0)).sin() * 0.1 + 0.1;
        let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.5));
        assert!(
            (bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4,
            "flat bottom y"
        );
    }

    #[test]
    fn custom_ground_transform_reseats_from_its_own_bounds() {
        // A ground transform scaling the block to 0.5 (vs the default 0.25) drops its bottom to -0.25,
        // so vanilla's `-minY + 1/16` lift grows to 0.3125 to keep it resting on the ground — proving the
        // seating is derived per-model from the actual transform, not a hardcoded constant.
        let custom = BlockModelDisplayTransform {
            rotation: [0.0, 0.0, 0.0],
            translation: [0.0, 0.0, 0.0],
            scale: [0.5, 0.5, 0.5],
        };
        let (min_y, _) = ground_model_bounds(&unit_block_quads(), display_matrix(&custom, false));
        assert!(
            (min_y + 0.25).abs() < 1e-4,
            "0.5-scaled block bottom at -0.25"
        );
        let min_offset_y = -min_y + 1.0 / 16.0;
        assert!(
            (min_offset_y - 0.3125).abs() < 1e-4,
            "lift compensates the lower bottom"
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
    fn ground_model_bounds_classify_block_vs_flat_depth() {
        // A cube face spanning Z 0..16, scaled 0.25, is 0.25 deep → scatter branch.
        let (_, block_depth) = ground_model_bounds(
            &unit_block_quads(),
            display_matrix(&BLOCK_GROUND_FALLBACK, false),
        );
        assert!((block_depth - 0.25).abs() < 1e-6);
        assert!(block_depth > FLAT_ITEM_DEPTH_THRESHOLD);
        // A generated slab spans Z 7.5..8.5 (depth 1), scaled 0.5 → 0.03125 deep → stack branch.
        let (_, flat_depth) = ground_model_bounds(
            &generated_slab_quads(),
            display_matrix(&GENERATED_GROUND_FALLBACK, false),
        );
        assert!((flat_depth - 0.031_25).abs() < 1e-6);
        assert!(flat_depth <= FLAT_ITEM_DEPTH_THRESHOLD);
    }

    #[test]
    fn custom_head_skull_items_are_reserved_for_the_skull_renderer() {
        for resource_id in [
            "minecraft:skeleton_skull",
            "minecraft:wither_skeleton_skull",
            "minecraft:player_head",
            "minecraft:zombie_head",
            "minecraft:creeper_head",
            "minecraft:dragon_head",
            "minecraft:piglin_head",
        ] {
            assert!(
                is_custom_head_skull_item_id(resource_id),
                "{resource_id} should not use the generic CustomHeadLayer item path"
            );
        }

        assert!(!is_custom_head_skull_item_id("minecraft:carved_pumpkin"));
        assert!(!is_custom_head_skull_item_id("minecraft:white_banner"));
    }

    #[test]
    fn armor_stand_invisible_marker_keeps_held_and_custom_head_item_models() {
        // Vanilla `LivingEntityRenderer.submit` still runs armor-stand `ItemInHandLayer` and the
        // generic item branch of `CustomHeadLayer` when the marker base body has no render type.
        let root = unique_item_model_temp_dir("armor-stand-invisible-held-items");
        write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 601;
        const ARMOR_STAND_ENTITY_TYPE_ID: i32 = 5;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, ARMOR_STAND_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 1)));

        let visible = EntityModelInstance::armor_stand_with_marker(
            ENTITY_ID,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            true,
            true,
            bbb_renderer::DEFAULT_ARMOR_STAND_MODEL_POSE,
        );
        let hidden_glowing = visible.with_invisible(true).with_outline_color(0xff55_aa11);
        let terrain_textures = TerrainTextureState::default();

        let visible_models =
            held_item_models(&[visible], &world, Some(&item_runtime), &terrain_textures);
        let hidden_models = held_item_models(
            &[hidden_glowing],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );

        assert!(visible_models.block_meshes.is_empty());
        assert!(hidden_models.block_meshes.is_empty());
        assert_eq!(visible_models.flat_meshes.len(), 2);
        assert_eq!(hidden_models.flat_meshes.len(), 2);
        assert!(visible_models
            .flat_meshes
            .iter()
            .all(|mesh| !mesh.is_empty()));
        assert!(hidden_models
            .flat_meshes
            .iter()
            .all(|mesh| !mesh.is_empty()));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn humanoid_invisible_keeps_held_and_custom_head_item_models() {
        // Vanilla `ItemInHandLayer` and the generic item branch of `CustomHeadLayer` have no
        // `state.isInvisible` gate, so ordinary humanoid mobs keep both item-model attachments.
        let root = unique_item_model_temp_dir("zombie-invisible-held-items");
        write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 602;
        const ZOMBIE_ENTITY_TYPE_ID: i32 = 150;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, ZOMBIE_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 1)));

        let visible = EntityModelInstance::zombie(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
        let hidden = visible.with_invisible(true);
        let hidden_glowing = visible.with_invisible(true).with_outline_color(0xff55_aa11);
        let terrain_textures = TerrainTextureState::default();

        let visible_models =
            held_item_models(&[visible], &world, Some(&item_runtime), &terrain_textures);
        let hidden_models =
            held_item_models(&[hidden], &world, Some(&item_runtime), &terrain_textures);
        let glowing_models = held_item_models(
            &[hidden_glowing],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );

        for models in [&visible_models, &hidden_models, &glowing_models] {
            assert!(models.block_meshes.is_empty());
            assert_eq!(models.flat_meshes.len(), 2);
            assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn humanoid_item_in_hand_layer_uses_main_arm_for_hand_slot_mapping() {
        // Vanilla `ItemInHandLayer.submit` renders RIGHT then LEFT, but each arm resolves its stack via
        // `LivingEntity.getItemHeldByArm(arm)`. A left-main-arm player therefore renders the main-hand
        // stack on the left arm and the off-hand stack on the right arm.
        let root = unique_item_model_temp_dir("humanoid-main-arm-held-items");
        write_flat_item_runtime_fixture(&root, &["main_item", "off_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        const ENTITY_ID: i32 = 606;
        const PLAYER_ENTITY_TYPE_ID: i32 = 155;
        let terrain_textures = TerrainTextureState::default();

        let mut main_world = WorldStore::new();
        main_world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(main_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        let right_main = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
        let left_main = right_main.with_main_arm_left(true);
        let right_main_models = held_item_models(
            &[right_main],
            &main_world,
            Some(&item_runtime),
            &terrain_textures,
        );
        let left_main_models = held_item_models(
            &[left_main],
            &main_world,
            Some(&item_runtime),
            &terrain_textures,
        );
        assert_eq!(right_main_models.flat_meshes.len(), 1);
        assert_eq!(left_main_models.flat_meshes.len(), 1);
        assert_ne!(
            right_main_models.flat_meshes[0], left_main_models.flat_meshes[0],
            "main-hand item moves from the right arm to the left arm when mainArm is LEFT"
        );

        let mut off_world = WorldStore::new();
        off_world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(off_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::OffHand, 1)));
        let right_off_models = held_item_models(
            &[right_main],
            &off_world,
            Some(&item_runtime),
            &terrain_textures,
        );
        let left_off_models = held_item_models(
            &[left_main],
            &off_world,
            Some(&item_runtime),
            &terrain_textures,
        );
        assert_eq!(right_off_models.flat_meshes.len(), 1);
        assert_eq!(left_off_models.flat_meshes.len(), 1);
        assert_ne!(
            right_off_models.flat_meshes[0], left_off_models.flat_meshes[0],
            "off-hand item moves from the left arm to the right arm when mainArm is LEFT"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn illusioner_item_in_hand_layer_uses_vanilla_visibility_and_clones() {
        // Vanilla `IllusionerRenderer` installs a conditional `ItemInHandLayer`: idle illusioners do not
        // submit held items, while casting/aggressive illusioners submit inside the invisible clone loop.
        let root = unique_item_model_temp_dir("illusioner-held-item-clones");
        write_flat_item_runtime_fixture(&root, &["bow"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 604;
        const ILLUSIONER_ENTITY_TYPE_ID: i32 = 68;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, ILLUSIONER_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));

        let offsets = [
            [1.25, 0.0, -0.75],
            [-1.0, 0.5, 0.5],
            [0.5, 0.0, 1.0],
            [-1.5, 0.0, -1.25],
        ];
        let base = EntityModelInstance::illager(
            ENTITY_ID,
            [4.0, 64.0, -2.0],
            30.0,
            IllagerModelFamily::Illusioner,
        )
        .with_age_in_ticks(8.0)
        .with_illusioner_clone_offsets(offsets);
        let terrain_textures = TerrainTextureState::default();

        let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
        let aggressive_models = held_item_models(
            &[base.with_is_aggressive(true)],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );
        let invisible_aggressive_models = held_item_models(
            &[base.with_is_aggressive(true).with_invisible(true)],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );
        let invisible_casting_models = held_item_models(
            &[base.with_illager_spellcasting(true).with_invisible(true)],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );

        assert!(idle_models.block_meshes.is_empty());
        assert!(idle_models.flat_meshes.is_empty());
        assert_eq!(aggressive_models.flat_meshes.len(), 1);
        assert_eq!(invisible_aggressive_models.flat_meshes.len(), 4);
        assert_eq!(invisible_casting_models.flat_meshes.len(), 4);
        for models in [
            &aggressive_models,
            &invisible_aggressive_models,
            &invisible_casting_models,
        ] {
            assert!(models.block_meshes.is_empty());
            assert!(models.block_translucent_meshes.is_empty());
            assert!(models.flat_translucent_meshes.is_empty());
            assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn illusioner_custom_head_item_branch_submits_per_invisible_clone() {
        // The generic item branch of vanilla `CustomHeadLayer` is inside `LivingEntityRenderer.submit`,
        // so `IllusionerRenderer`'s invisible clone loop runs it once for each clone root.
        let root = unique_item_model_temp_dir("illusioner-custom-head-item-clones");
        write_flat_item_runtime_fixture(&root, &["head_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 605;
        const ILLUSIONER_ENTITY_TYPE_ID: i32 = 68;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, ILLUSIONER_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 0)));

        let offsets = [
            [1.25, 0.0, -0.75],
            [-1.0, 0.5, 0.5],
            [0.5, 0.0, 1.0],
            [-1.5, 0.0, -1.25],
        ];
        let base = EntityModelInstance::illager(
            ENTITY_ID,
            [4.0, 64.0, -2.0],
            30.0,
            IllagerModelFamily::Illusioner,
        )
        .with_age_in_ticks(8.0)
        .with_illusioner_clone_offsets(offsets);
        let terrain_textures = TerrainTextureState::default();

        let visible_models =
            held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
        let invisible_models = held_item_models(
            &[base.with_invisible(true)],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );

        assert!(visible_models.block_meshes.is_empty());
        assert!(invisible_models.block_meshes.is_empty());
        assert!(visible_models.flat_translucent_meshes.is_empty());
        assert!(invisible_models.flat_translucent_meshes.is_empty());
        assert_eq!(visible_models.flat_meshes.len(), 1);
        assert_eq!(invisible_models.flat_meshes.len(), 4);
        assert!(visible_models
            .flat_meshes
            .iter()
            .all(|mesh| !mesh.is_empty()));
        assert!(invisible_models
            .flat_meshes
            .iter()
            .all(|mesh| !mesh.is_empty()));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allay_item_in_hand_layer_bakes_main_and_offhand_items() {
        // Vanilla `AllayRenderer` adds the standard `ItemInHandLayer`. Its `AllayModel.translateToHand`
        // supplies a special allay hand transform, but the item states still use the ordinary third-person
        // right/left hand display contexts from `ArmedEntityRenderState`.
        let root = unique_item_model_temp_dir("allay-held-items");
        write_flat_item_runtime_fixture(&root, &["main_item", "off_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 603;
        const ALLAY_ENTITY_TYPE_ID: i32 = 2;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, ALLAY_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::OffHand, 1)));

        let base =
            EntityModelInstance::allay(ENTITY_ID, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(7.0);
        let holding = base.with_allay_holding_item_progress(1.0);
        let terrain_textures = TerrainTextureState::default();

        let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
        let holding_models =
            held_item_models(&[holding], &world, Some(&item_runtime), &terrain_textures);

        for models in [&idle_models, &holding_models] {
            assert!(models.block_meshes.is_empty());
            assert!(models.block_translucent_meshes.is_empty());
            assert_eq!(models.flat_meshes.len(), 2);
            assert!(models.flat_translucent_meshes.is_empty());
            assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
        }
        assert_ne!(
            idle_models.flat_meshes[0], holding_models.flat_meshes[0],
            "holdingAnimationProgress feeds AllayModel.translateToHand through right_arm.xRot"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cluster_count_drives_geometry_and_is_non_empty() {
        let quads = unit_block_quads();
        // One copy and four copies both produce geometry; the four-copy mesh holds four times as much.
        let single = stacked_item_mesh(
            &quads,
            [0.0, 0.0, 0.0],
            0.0,
            0,
            &BLOCK_GROUND_FALLBACK,
            [4.0 / 15.0, 11.0 / 15.0],
            1,
            7,
        );
        let cluster = stacked_item_mesh(
            &quads,
            [0.0, 0.0, 0.0],
            0.0,
            0,
            &BLOCK_GROUND_FALLBACK,
            [4.0 / 15.0, 11.0 / 15.0],
            4,
            7,
        );
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

        let generated_head =
            display_matrix(&GENERATED_HEAD_FALLBACK, false).transform_point3(Vec3::splat(0.5));
        assert!((generated_head - Vec3::new(0.0, 13.0 / 16.0, 7.0 / 16.0)).length() < 1e-6);
        let block_head =
            display_matrix(&BLOCK_HEAD_FALLBACK, false).transform_point3(Vec3::splat(0.5));
        assert!((block_head - Vec3::ZERO).length() < 1e-6);
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

    fn equipment(entity_id: i32, slot: EquipmentSlot, item_id: i32) -> SetEquipment {
        SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        }
    }

    fn write_flat_item_runtime_fixture(root: &Path, item_ids: &[&str]) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, item_ids);
        for (index, item_id) in item_ids.iter().enumerate() {
            write_json(
                &assets.join("items").join(format!("{item_id}.json")),
                &format!(
                    r#"{{
                    "model": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{item_id}"
                    }}
                }}"#
                ),
            );
            write_flat_item_model_and_texture(
                &assets,
                item_id,
                &[(64 + index * 40) as u8, 80, 120, 255],
            );
        }
    }

    fn item_model_assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn write_item_atlases(assets: &Path) {
        write_json(
            &assets.join("atlases").join("items.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "item/",
                        "source": "item"
                    }
                ]
            }"#,
        );
        write_json(
            &assets.join("atlases").join("blocks.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "block/",
                        "source": "block"
                    }
                ]
            }"#,
        );
    }

    fn write_item_registry_source(root: &Path, item_ids: &[&str]) {
        let declarations = item_ids
            .iter()
            .map(|item_id| {
                let constant = item_id.to_ascii_uppercase();
                format!("public static final Item {constant} = registerItem(\"{item_id}\");")
            })
            .collect::<Vec<_>>()
            .join("\n");
        write_json(
            &root
                .join("sources")
                .join(bbb_pack::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            &format!(
                r#"public class Items {{
                {declarations}
            }}"#,
            ),
        );
    }

    fn write_flat_item_model_and_texture(assets: &Path, model_id: &str, rgba: &[u8]) {
        write_json(
            &assets
                .join("models")
                .join("item")
                .join(format!("{model_id}.json")),
            &format!(
                r#"{{
                "textures": {{
                    "layer0": "minecraft:item/{model_id}"
                }}
            }}"#
            ),
        );
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("item")
                .join(format!("{model_id}.png")),
            rgba,
        );
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_test_rgba_png(path: &Path, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
    }

    fn unique_item_model_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-native-item-models-{label}-{nanos}"))
    }
}
