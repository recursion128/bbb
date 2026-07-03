//! Dropped-item 3D models: turns dropped item entities into baked item-model meshes for the renderer's
//! item-model pass, replacing the flat billboard. A dropped item whose item is a block bakes its block
//! render shape over the blocks atlas (the block path); every other item extrudes its flat sprite into a
//! `1/16`-thick slab over the item atlas (the generated/flat path, vanilla `builtin/generated`). Both
//! are placed by vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the model's
//! GROUND display transform, and a stack of items renders as the vanilla cluster of `1..=5` jittered
//! copies (`submitMultipleFromCount`). Ominous item spawners reuse the same item-cluster bake with their
//! renderer-owned scale-in and spin transform.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_protocol::packets::{
    ConsumableSummary, EquipmentSlot, InteractionHand, ItemStackSummary, ItemStackTemplateSummary,
    ItemUseAnimationSummary, SwingAnimationTypeSummary,
};
use bbb_renderer::{
    allay_hand_attach_transform, bake_first_person_map_background_surface,
    bake_first_person_map_decoration_surface, bake_first_person_map_text_surface,
    bake_generated_item_quads, bake_item_frame_map_surface,
    bake_item_model_mesh_with_light_and_overlay,
    bake_item_model_meshes_with_light_and_overlay_and_foil_mode,
    copper_golem_antenna_block_transform, copper_golem_hand_attach_transform,
    custom_head_item_transforms, dolphin_carried_item_transform, enderman_carried_block_transform,
    falling_block_transform, fox_held_item_transform, humanoid_hand_attach_transforms,
    iron_golem_flower_block_transform, minecart_tnt_display_block_transform,
    mooshroom_mushroom_block_transforms, panda_held_item_transform, primed_tnt_block_transform,
    snow_golem_head_block_transform, villager_crossed_arms_item_transform,
    witch_held_item_transform, CameraPose, EntityModelInstance, EntityModelKind,
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture,
    FirstPersonPlayerArm, HumanoidModelFamily, IllagerModelFamily, ItemFrameMapDecorationSurface,
    ItemFrameMapDecorationTexture, ItemFrameMapSurface, ItemFrameMapTextSurface,
    ItemFrameMapTexture, ItemModelFoil, ItemModelMesh, ItemModelMeshSet, ItemModelQuad,
    MooshroomVariant, PiglinModelFamily, SkeletonModelFamily, SpearKineticWeapon,
    ZombieVariantModelFamily, ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::{BlockPos, TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::entity_scene::default_spear_kinetic_weapon_for_resource_id;
use crate::map_textures::map_item_texture;
use crate::terrain_runtime::TerrainTextureState;
use bbb_item_model::{ItemModelUseContext, NativeItemRuntime};
use bbb_protocol::entity_types::*;

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
const OMINOUS_ITEM_SPAWNER_SCALE_IN_TICKS: f32 = 50.0;
const OMINOUS_ITEM_SPAWNER_ROTATION_SPEED_DEGREES_PER_TICK: f32 = 40.0;
const CARVED_PUMPKIN_BLOCK_ID: &str = "minecraft:carved_pumpkin";
const POPPY_BLOCK_ID: &str = "minecraft:poppy";
const RED_MUSHROOM_BLOCK_ID: &str = "minecraft:red_mushroom";
const BROWN_MUSHROOM_BLOCK_ID: &str = "minecraft:brown_mushroom";
/// Vanilla `DataComponents.KINETIC_WEAPON` network type id, used by item-stack
/// patches to remove a spear's prototype kinetic weapon component.
const VANILLA_KINETIC_WEAPON_COMPONENT_ID: i32 = 39;
/// Vanilla `DataComponents.SWING_ANIMATION` network type id, used by item-stack
/// patches to remove prototype STAB/NONE data and fall back to
/// `SwingAnimation.DEFAULT` (WHACK, 6 ticks).
const VANILLA_SWING_ANIMATION_COMPONENT_ID: i32 = 40;
/// Vanilla `DataComponents.CONSUMABLE` network type id. `Item.getUseAnimation`
/// checks this before `BLOCKS_ATTACKS`, so consumable stacks do not use the
/// BLOCK first-person transform even when a patch also adds `blocks_attacks`.
const VANILLA_CONSUMABLE_COMPONENT_ID: i32 = 24;
/// Vanilla `DataComponents.BLOCKS_ATTACKS` network type id. Non-consumable
/// stacks carrying it use `ItemUseAnimation.BLOCK`; the vanilla shield has it as
/// a prototype component.
const VANILLA_BLOCKS_ATTACKS_COMPONENT_ID: i32 = 37;
/// Vanilla `BrushItem.USE_DURATION`. The first-person BRUSH swipe uses the
/// remaining use ticks modulo `BrushItem.ANIMATION_DURATION` (10).
const VANILLA_BRUSH_USE_DURATION_TICKS: f32 = 200.0;
/// Vanilla `BowItem.getUseDuration`.
const VANILLA_BOW_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla `CrossbowItem.getUseDuration`.
const VANILLA_CROSSBOW_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla base `CrossbowItem.getChargeDuration`: 1.25 seconds * 20 TPS.
const VANILLA_CROSSBOW_CHARGE_DURATION_TICKS: f32 = 25.0;
/// Vanilla `TridentItem.getUseDuration`.
const VANILLA_TRIDENT_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla base `Item.getUseDuration`: stacks with `DataComponents.KINETIC_WEAPON`
/// and no consumable use the long charged-use timer.
const VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla `ItemInHandRenderer.renderMap`: after the hand/map pose, the map is flipped, scaled to
/// `0.38`, centered, and converted from 128x128 map pixels into model units.
const VANILLA_FIRST_PERSON_MAP_SCALE: f32 = 0.38;
const VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE: f32 = 1.0 / 128.0;

/// The baked item-model meshes for this frame, split by which atlas they sample (block-items → blocks
/// atlas, flat items → item atlas), plus the set of dropped-item entity ids they cover (so the billboard
/// path skips those entities and does not double-render them).
pub(crate) struct DroppedItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub block_glint_meshes: Vec<ItemModelMesh>,
    pub block_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_glint_meshes: Vec<ItemModelMesh>,
    pub flat_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub handled_entity_ids: BTreeSet<i32>,
}

impl DroppedItemModels {
    fn empty() -> Self {
        Self {
            block_meshes: Vec::new(),
            block_translucent_meshes: Vec::new(),
            block_glint_meshes: Vec::new(),
            block_glint_translucent_meshes: Vec::new(),
            flat_meshes: Vec::new(),
            flat_translucent_meshes: Vec::new(),
            flat_glint_meshes: Vec::new(),
            flat_glint_translucent_meshes: Vec::new(),
            handled_entity_ids: BTreeSet::new(),
        }
    }
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

fn primed_tnt_block_overlay(fuse_remaining_in_ticks: f32) -> [f32; 2] {
    if (fuse_remaining_in_ticks as i32) / 5 % 2 == 0 {
        [15.0, 10.0]
    } else {
        ITEM_MODEL_NO_OVERLAY
    }
}

fn falling_block_should_render(
    instance: &EntityModelInstance,
    world: &WorldStore,
    block_state_id: i32,
) -> bool {
    match world.probe_block(falling_block_block_pos(instance)) {
        Some(block) => block.block_state_id != block_state_id,
        None => true,
    }
}

fn falling_block_block_pos(instance: &EntityModelInstance) -> BlockPos {
    BlockPos {
        x: instance.position[0].floor() as i32,
        y: instance.position[1].floor() as i32,
        z: instance.position[2].floor() as i32,
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
    entity_partial_tick: f32,
) -> Vec<ItemModelMesh> {
    let attachments = entity_block_attachments(
        instances,
        world,
        item_runtime,
        entity_partial_tick.clamp(0.0, 1.0),
    );
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
    entity_partial_tick: f32,
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
        if let Some(block) = world.primed_tnt_block_state(instance.entity_id) {
            let fuse = world
                .primed_tnt_fuse_remaining_in_ticks(instance.entity_id, entity_partial_tick)
                .unwrap_or(80.0);
            if let Some(transform) = primed_tnt_block_transform(instance, fuse) {
                attachments.push(EntityBlockAttachment {
                    block_id: Cow::Owned(block.name),
                    properties: block.properties,
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: primed_tnt_block_overlay(fuse),
                    outline_only: entity_block_attachment_outline_only(instance),
                });
            }
        }
        if let Some(falling) = world.falling_block_state(instance.entity_id) {
            if falling_block_should_render(instance, world, falling.block_state_id) {
                if let Some(transform) = falling_block_transform(instance) {
                    attachments.push(EntityBlockAttachment {
                        block_id: Cow::Owned(falling.block.name),
                        properties: falling.block.properties,
                        transform,
                        light: entity_render_state_shader_light(instance),
                        overlay: ITEM_MODEL_NO_OVERLAY,
                        outline_only: false,
                    });
                }
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
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> DroppedItemModels {
    let mut block_meshes = Vec::new();
    let mut block_translucent_meshes = Vec::new();
    let mut block_glint_meshes = Vec::new();
    let mut block_glint_translucent_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut flat_translucent_meshes = Vec::new();
    let mut flat_glint_meshes = Vec::new();
    let mut flat_glint_translucent_meshes = Vec::new();
    let mut handled_entity_ids = BTreeSet::new();
    let Some(item_runtime) = item_runtime else {
        return DroppedItemModels {
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
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
        let ground = |default_transform| {
            item_runtime
                .item_display_transform_for_stack(&state.stack, BlockModelDisplayContext::Ground)
                .unwrap_or(default_transform)
        };
        let foil =
            item_stack_foil_mode(&state.stack, item_runtime, BlockModelDisplayContext::Ground);

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
                            foil,
                        ),
                        &mut block_meshes,
                        &mut block_translucent_meshes,
                        &mut block_glint_meshes,
                        &mut block_glint_translucent_meshes,
                    );
                    handled_entity_ids.insert(state.entity_id);
                    continue;
                }
            }
        }

        // Flat path: extrude the item's sprite layers into a slab.
        let mut quads: Vec<ItemModelQuad> = Vec::new();
        for layer in item_runtime.generated_item_layers_for_stack_with_registry_context(
            &state.stack,
            BlockModelDisplayContext::Ground,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
        ) {
            quads.extend(bake_generated_item_quads(
                &layer.mask,
                layer.rect,
                layer.tint,
                layer.translucent,
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
                foil,
            ),
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        handled_entity_ids.insert(state.entity_id);
    }

    DroppedItemModels {
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        handled_entity_ids,
    }
}

/// Bakes the item cluster carried by every ominous item spawner entity. Vanilla
/// `OminousItemSpawnerRenderer` uses `ItemClusterRenderState` with full-bright light, scales in for the
/// first 50 ticks, spins at 40 degrees per tick, then delegates to
/// `ItemEntityRenderer.submitMultipleFromCount`.
pub(crate) fn ominous_item_spawner_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    entity_partial_tick: f32,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> DroppedItemModels {
    let mut models = DroppedItemModels::empty();
    let Some(item_runtime) = item_runtime else {
        return models;
    };

    for state in world.ominous_item_spawner_item_states_at_partial_tick(entity_partial_tick) {
        let Some(item_id) = state.stack.item_id else {
            continue;
        };
        let position = [
            state.position.x as f32,
            state.position.y as f32,
            state.position.z as f32,
        ];
        let count = rendered_amount(state.stack.count);
        // Vanilla `ItemClusterRenderState.getSeedForItemStack`: item id plus damage. The currently
        // retained stack summary keeps item id; damaged stack jitter is deferred with broader item
        // component render-state parity.
        let seed = item_id as i64;
        let ground = |default_transform| {
            item_runtime
                .item_display_transform_for_stack(&state.stack, BlockModelDisplayContext::Ground)
                .unwrap_or(default_transform)
        };
        let foil =
            item_stack_foil_mode(&state.stack, item_runtime, BlockModelDisplayContext::Ground);
        let base = ominous_item_spawner_base_transform(position, state.age_ticks);

        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    push_mesh_set(
                        item_cluster_mesh_with_base(
                            &quads,
                            base,
                            &ground(BLOCK_GROUND_FALLBACK),
                            ITEM_MODEL_FULL_BRIGHT_LIGHT,
                            count,
                            seed,
                            foil,
                        ),
                        &mut models.block_meshes,
                        &mut models.block_translucent_meshes,
                        &mut models.block_glint_meshes,
                        &mut models.block_glint_translucent_meshes,
                    );
                    models.handled_entity_ids.insert(state.entity_id);
                    continue;
                }
            }
        }

        let mut quads: Vec<ItemModelQuad> = Vec::new();
        for layer in item_runtime.generated_item_layers_for_stack_with_registry_context(
            &state.stack,
            BlockModelDisplayContext::Ground,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
        ) {
            quads.extend(bake_generated_item_quads(
                &layer.mask,
                layer.rect,
                layer.tint,
                layer.translucent,
            ));
        }
        if quads.is_empty() {
            continue;
        }
        push_mesh_set(
            item_cluster_mesh_with_base(
                &quads,
                base,
                &ground(GENERATED_GROUND_FALLBACK),
                ITEM_MODEL_FULL_BRIGHT_LIGHT,
                count,
                seed,
                foil,
            ),
            &mut models.flat_meshes,
            &mut models.flat_translucent_meshes,
            &mut models.flat_glint_meshes,
            &mut models.flat_glint_translucent_meshes,
        );
        models.handled_entity_ids.insert(state.entity_id);
    }

    models
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

/// Fallback first-person right-hand display transform for a block item
/// (`minecraft:block/block`): rotation `[0, 45, 0]°`, translation `0`, scale `0.40`.
const BLOCK_FIRST_PERSON_RIGHT_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 45.0, 0.0],
    translation: [0.0, 0.0, 0.0],
    scale: [0.40, 0.40, 0.40],
};

/// Fallback first-person left-hand display transform for a block item
/// (`minecraft:block/block`): rotation `[0, 225, 0]°`, translation `0`, scale `0.40`.
const BLOCK_FIRST_PERSON_LEFT_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 225.0, 0.0],
    translation: [0.0, 0.0, 0.0],
    scale: [0.40, 0.40, 0.40],
};

/// Fallback first-person display transform for a generated item
/// (`minecraft:item/generated`): rotation `[0, -90, 25]°`, translation
/// `[1.13, 3.2, 1.13]`/16, scale `0.68`. Vanilla copies this right-hand
/// transform into the missing generated left-hand slot before applying the
/// left-hand fix.
const GENERATED_FIRST_PERSON_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, -90.0, 25.0],
    translation: [1.13 / 16.0, 3.2 / 16.0, 1.13 / 16.0],
    scale: [0.68, 0.68, 0.68],
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

pub(crate) fn item_stack_foil_mode(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
    context: BlockModelDisplayContext,
) -> ItemModelFoil {
    if !stack.has_foil() {
        return ItemModelFoil::None;
    }
    if item_runtime.item_stack_uses_special_foil_texture(stack) {
        ItemModelFoil::Special {
            decal_pose_scale: special_foil_decal_pose_scale(context),
        }
    } else {
        ItemModelFoil::Standard
    }
}

fn special_foil_decal_pose_scale(context: BlockModelDisplayContext) -> f32 {
    match context {
        BlockModelDisplayContext::Gui => 0.5,
        BlockModelDisplayContext::FirstPersonLeftHand
        | BlockModelDisplayContext::FirstPersonRightHand => 0.75,
        BlockModelDisplayContext::ThirdPersonLeftHand
        | BlockModelDisplayContext::ThirdPersonRightHand
        | BlockModelDisplayContext::Head
        | BlockModelDisplayContext::Ground
        | BlockModelDisplayContext::Fixed
        | BlockModelDisplayContext::OnShelf => 1.0,
    }
}

/// The baked held-item meshes for this frame, split by atlas (block-items vs flat items).
pub(crate) struct HeldItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub block_glint_meshes: Vec<ItemModelMesh>,
    pub block_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_glint_meshes: Vec<ItemModelMesh>,
    pub flat_glint_translucent_meshes: Vec<ItemModelMesh>,
}

/// The baked first-person local-player hand item submissions for this frame. Ordinary items use
/// block/flat item-model meshes; filled maps use vanilla `ItemInHandRenderer.renderMap` dynamic-map
/// surfaces in the same depth-cleared hand pass.
pub(crate) struct FirstPersonItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub block_glint_meshes: Vec<ItemModelMesh>,
    pub block_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_glint_meshes: Vec<ItemModelMesh>,
    pub flat_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
    pub map_background_surfaces: Vec<FirstPersonMapBackgroundSurface>,
    pub map_textures: Vec<ItemFrameMapTexture>,
    pub map_surfaces: Vec<ItemFrameMapSurface>,
    pub map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub map_text_surfaces: Vec<ItemFrameMapTextSurface>,
}

impl FirstPersonItemModels {
    fn empty() -> Self {
        Self {
            block_meshes: Vec::new(),
            block_translucent_meshes: Vec::new(),
            block_glint_meshes: Vec::new(),
            block_glint_translucent_meshes: Vec::new(),
            flat_meshes: Vec::new(),
            flat_translucent_meshes: Vec::new(),
            flat_glint_meshes: Vec::new(),
            flat_glint_translucent_meshes: Vec::new(),
            map_background_textures: Vec::new(),
            map_background_surfaces: Vec::new(),
            map_textures: Vec::new(),
            map_surfaces: Vec::new(),
            map_decoration_textures: Vec::new(),
            map_decoration_surfaces: Vec::new(),
            map_text_surfaces: Vec::new(),
        }
    }
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
    let mut block_glint_meshes = Vec::new();
    let mut block_glint_translucent_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut flat_translucent_meshes = Vec::new();
    let mut flat_glint_meshes = Vec::new();
    let mut flat_glint_translucent_meshes = Vec::new();
    let Some(item_runtime) = item_runtime else {
        return HeldItemModels {
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
        };
    };
    let enchantment_keys = world_enchantment_keys(world);
    let trim_material_keys = world_trim_material_keys(world);
    let attribute_keys = world_attribute_keys(world);
    let context_dimension = world.level_info().map(|level| level.dimension.as_str());

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
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_held_hand(
            instance,
            true,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_fox_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_dolphin_carried_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_witch_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_copper_golem_held_items(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_allay_held_items(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_villager_crossed_arms_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_panda_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
        bake_custom_head_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            &mut block_meshes,
            &mut block_translucent_meshes,
            &mut block_glint_meshes,
            &mut block_glint_translucent_meshes,
            &mut flat_meshes,
            &mut flat_translucent_meshes,
            &mut flat_glint_meshes,
            &mut flat_glint_translucent_meshes,
        );
    }

    HeldItemModels {
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
    }
}

pub(crate) fn first_person_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    partial_ticks: f32,
) -> FirstPersonItemModels {
    let mut models = FirstPersonItemModels::empty();
    let (Some(item_runtime), Some(camera_pose)) = (item_runtime, camera_pose) else {
        return models;
    };
    if !world.local_player().camera.follows_player || world.local_player_is_spectator() {
        return models;
    }
    if local_player_is_scoping(world, item_runtime) {
        return models;
    }

    let main_stack = world.local_item_in_hand(InteractionHand::MainHand);
    let off_stack = world.local_item_in_hand(InteractionHand::OffHand);
    let using_hand = world.local_player().interaction.using_item.then(|| {
        world
            .local_player()
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand)
    });
    if let Some(hand) = using_hand {
        let stack = match hand {
            InteractionHand::MainHand => main_stack,
            InteractionHand::OffHand => off_stack,
        };
        if !stack.is_some_and(|stack| {
            first_person_stack_supported_use_animation(stack, item_runtime).is_some()
        }) {
            return models;
        }
    }
    if [main_stack, off_stack]
        .into_iter()
        .flatten()
        .any(|stack| !supported_first_person_item_stack(stack, item_runtime))
    {
        return models;
    }

    let enchantment_keys = world_enchantment_keys(world);
    let trim_material_keys = world_trim_material_keys(world);
    let attribute_keys = world_attribute_keys(world);
    let context_dimension = world.level_info().map(|level| level.dimension.as_str());
    let mut map_textures = BTreeMap::new();
    let owner_main_hand_left = world.local_player_main_arm_left().unwrap_or(false);
    let camera_x_rot = camera_pose.x_rot;
    let camera_world = first_person_camera_world_transform(camera_pose);
    let attack_swing = world.local_player_attack_swing(partial_ticks);
    let only_render_using_hand =
        first_person_only_render_using_hand(item_runtime, main_stack, off_stack, using_hand);

    for (hand, stack) in [
        (InteractionHand::MainHand, main_stack),
        (InteractionHand::OffHand, off_stack),
    ] {
        if only_render_using_hand.is_some_and(|only_hand| only_hand != hand) {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        let arm_left = match hand {
            InteractionHand::MainHand => owner_main_hand_left,
            InteractionHand::OffHand => !owner_main_hand_left,
        };
        let context = if arm_left {
            BlockModelDisplayContext::FirstPersonLeftHand
        } else {
            BlockModelDisplayContext::FirstPersonRightHand
        };
        let attack = attack_swing
            .filter(|swing| swing.off_hand == (hand == InteractionHand::OffHand))
            .map_or(0.0, |swing| swing.attack_anim);
        if let Some(map_id) = stack.component_patch.map_id {
            let map = world.map_item(map_id);
            let map_transform = if hand == InteractionHand::MainHand && off_stack.is_none() {
                first_person_two_handed_map_transform(camera_world, camera_x_rot, 0.0, attack)
            } else {
                first_person_one_handed_map_transform(camera_world, arm_left, 0.0, attack)
            };
            let background_kind = if map.is_some() {
                FirstPersonMapBackgroundKind::Checkerboard
            } else {
                FirstPersonMapBackgroundKind::Plain
            };
            models
                .map_background_surfaces
                .push(bake_first_person_map_background_surface(
                    background_kind,
                    map_transform,
                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                ));
            if let Some(map) = map {
                map_textures
                    .entry(map.id)
                    .or_insert_with(|| map_item_texture(map));
                models.map_surfaces.push(bake_item_frame_map_surface(
                    map.id,
                    map_transform,
                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                ));
                let mut visible_decoration_index = 0;
                let mut text_submit_sequence = 0;
                for decoration in &map.decorations {
                    if let Some(surface) = bake_first_person_map_decoration_surface(
                        decoration.type_id,
                        decoration.x,
                        decoration.y,
                        decoration.rot,
                        visible_decoration_index,
                        map_transform,
                        ITEM_MODEL_FULL_BRIGHT_LIGHT,
                        visible_decoration_index + 1,
                    ) {
                        models.map_decoration_surfaces.push(surface);
                        if let Some(name) = decoration.name.as_ref() {
                            if let Some(glyphs) = item_runtime.map_text_glyphs() {
                                if let Some(text_surface) = bake_first_person_map_text_surface(
                                    decoration.type_id,
                                    name.as_str(),
                                    decoration.x,
                                    decoration.y,
                                    visible_decoration_index,
                                    map_transform,
                                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                                    text_submit_sequence,
                                    glyphs,
                                ) {
                                    models.map_text_surfaces.push(text_surface);
                                    text_submit_sequence += 1;
                                }
                            }
                        }
                        visible_decoration_index += 1;
                    }
                }
            }
            continue;
        }
        let mut attach =
            first_person_item_arm_transform_from_camera_world(camera_world, arm_left, 0.0);
        let using_this_hand = using_hand == Some(hand);
        if let Some(use_animation) = using_this_hand
            .then(|| first_person_stack_supported_use_animation(stack, item_runtime))
            .flatten()
        {
            match use_animation {
                FirstPersonUseAnimation::None => {}
                FirstPersonUseAnimation::EatDrink { use_duration_ticks } => {
                    attach = first_person_apply_eat_drink_use_transform(
                        camera_world,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Block(kind) => {
                    if kind == FirstPersonBlockUseKind::NonShieldBlock {
                        attach = first_person_apply_block_use_transform(attach, arm_left);
                    }
                }
                FirstPersonUseAnimation::Brush { use_duration_ticks } => {
                    attach = first_person_apply_brush_use_transform(
                        attach,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Bundle => {
                    attach = first_person_apply_whack_swing(attach, arm_left, attack);
                }
                FirstPersonUseAnimation::Trident { use_duration_ticks } => {
                    attach = first_person_apply_trident_use_transform(
                        attach,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Bow { use_duration_ticks } => {
                    attach = first_person_apply_bow_use_transform(
                        attach,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Crossbow {
                    use_duration_ticks,
                    charge_duration_ticks,
                } => {
                    if first_person_stack_is_charged_crossbow(stack, item_runtime) {
                        attach = first_person_apply_whack_swing(attach, arm_left, attack);
                        if hand == InteractionHand::MainHand && attack < 0.001 {
                            attach = first_person_apply_charged_crossbow_idle_transform(
                                attach, arm_left,
                            );
                        }
                    } else {
                        attach = first_person_apply_crossbow_use_transform(
                            attach,
                            arm_left,
                            use_duration_ticks,
                            charge_duration_ticks,
                            world.local_player().interaction.using_item_ticks as f32,
                            partial_ticks,
                        );
                    }
                }
                FirstPersonUseAnimation::Spear {
                    kinetic_weapon,
                    use_duration_ticks,
                } => {
                    attach = first_person_apply_spear_use_transform(
                        camera_world,
                        arm_left,
                        kinetic_weapon,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                        world.local_player_ticks_since_kinetic_hit_feedback(partial_ticks),
                    );
                }
            }
        } else {
            match first_person_stack_swing_animation(stack, item_runtime) {
                FirstPersonSwingAnimation::None => {}
                FirstPersonSwingAnimation::Whack => {
                    attach = first_person_apply_whack_swing(attach, arm_left, attack);
                }
                FirstPersonSwingAnimation::Stab => {
                    attach = first_person_apply_stab_swing(attach, arm_left, attack);
                }
            }
            if first_person_stack_is_charged_crossbow(stack, item_runtime)
                && hand == InteractionHand::MainHand
                && attack < 0.001
            {
                attach = first_person_apply_charged_crossbow_idle_transform(attach, arm_left);
            }
        }
        bake_item_stack_at_transform(
            stack,
            attach,
            context,
            arm_left,
            Some(owner_main_hand_left),
            Some("minecraft:player"),
            false,
            0.0,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            first_person_block_fallback(context),
            GENERATED_FIRST_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            &mut models.block_meshes,
            &mut models.block_translucent_meshes,
            &mut models.block_glint_meshes,
            &mut models.block_glint_translucent_meshes,
            &mut models.flat_meshes,
            &mut models.flat_translucent_meshes,
            &mut models.flat_glint_meshes,
            &mut models.flat_glint_translucent_meshes,
        );
    }

    if !models.map_background_surfaces.is_empty() {
        models.map_background_textures = item_runtime.map_background_textures().to_vec();
    }
    models.map_textures = map_textures.into_values().collect();
    if !models.map_decoration_surfaces.is_empty() {
        models.map_decoration_textures = item_runtime.map_decoration_textures().to_vec();
    }
    models
}

pub(crate) fn first_person_player_arms(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_player_instance: Option<&EntityModelInstance>,
    camera_pose: Option<CameraPose>,
    partial_ticks: f32,
) -> Vec<FirstPersonPlayerArm> {
    let (Some(local_player_instance), Some(camera_pose)) = (local_player_instance, camera_pose)
    else {
        return Vec::new();
    };
    if !world.local_player().camera.follows_player || world.local_player_is_spectator() {
        return Vec::new();
    }
    if item_runtime.is_some_and(|item_runtime| local_player_is_scoping(world, item_runtime)) {
        return Vec::new();
    }
    if local_player_instance.render_state.invisible {
        return Vec::new();
    }
    let EntityModelKind::Player { skin, parts } = local_player_instance.kind else {
        return Vec::new();
    };

    let main_stack = world.local_item_in_hand(InteractionHand::MainHand);
    let off_stack = world.local_item_in_hand(InteractionHand::OffHand);
    let using_hand = world.local_player().interaction.using_item.then(|| {
        world
            .local_player()
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand)
    });
    let only_render_using_hand = item_runtime.and_then(|item_runtime| {
        first_person_only_render_using_hand(item_runtime, main_stack, off_stack, using_hand)
    });
    let owner_main_hand_left = world.local_player_main_arm_left().unwrap_or(false);
    let camera_world = first_person_camera_world_transform(camera_pose);
    let attack_swing = world.local_player_attack_swing(partial_ticks);
    let hand_attack = |hand: InteractionHand| {
        attack_swing
            .filter(|swing| swing.off_hand == (hand == InteractionHand::OffHand))
            .map_or(0.0, |swing| swing.attack_anim)
    };
    let light = first_person_player_arm_light(local_player_instance);
    let mut arms = Vec::new();

    if only_render_using_hand.is_none_or(|hand| hand == InteractionHand::MainHand)
        && main_stack.is_none()
    {
        let arm_left = owner_main_hand_left;
        let attack = hand_attack(InteractionHand::MainHand);
        arms.push(FirstPersonPlayerArm {
            left: arm_left,
            skin,
            sleeve_visible: if arm_left {
                parts.left_sleeve
            } else {
                parts.right_sleeve
            },
            transform: first_person_render_player_arm_transform(
                camera_world,
                arm_left,
                0.0,
                attack,
            ),
            light,
        });
    }

    for (hand, stack) in [
        (InteractionHand::MainHand, main_stack),
        (InteractionHand::OffHand, off_stack),
    ] {
        if only_render_using_hand.is_some_and(|only_hand| only_hand != hand) {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        if stack.component_patch.map_id.is_none() {
            continue;
        }
        let attack = hand_attack(hand);
        if hand == InteractionHand::MainHand && off_stack.is_none() {
            for arm_left in [false, true] {
                arms.push(FirstPersonPlayerArm {
                    left: arm_left,
                    skin,
                    sleeve_visible: if arm_left {
                        parts.left_sleeve
                    } else {
                        parts.right_sleeve
                    },
                    transform: first_person_two_handed_map_arm_transform(
                        camera_world,
                        camera_pose.x_rot,
                        0.0,
                        attack,
                        arm_left,
                    ),
                    light,
                });
            }
        } else {
            let arm_left = match hand {
                InteractionHand::MainHand => owner_main_hand_left,
                InteractionHand::OffHand => !owner_main_hand_left,
            };
            arms.push(FirstPersonPlayerArm {
                left: arm_left,
                skin,
                sleeve_visible: if arm_left {
                    parts.left_sleeve
                } else {
                    parts.right_sleeve
                },
                transform: first_person_one_handed_map_arm_transform(
                    camera_world,
                    arm_left,
                    0.0,
                    attack,
                ),
                light,
            });
        }
    }

    arms
}

fn first_person_only_render_using_hand(
    item_runtime: &NativeItemRuntime,
    main_stack: Option<&ItemStackSummary>,
    off_stack: Option<&ItemStackSummary>,
    using_hand: Option<InteractionHand>,
) -> Option<InteractionHand> {
    using_hand.filter(|hand| {
        let stack = match hand {
            InteractionHand::MainHand => main_stack,
            InteractionHand::OffHand => off_stack,
        };
        stack
            .and_then(|stack| first_person_stack_resource_id(stack, item_runtime))
            .is_some_and(|resource_id| {
                matches!(resource_id, "minecraft:bow" | "minecraft:crossbow")
            })
    })
}

fn first_person_player_arm_light(instance: &EntityModelInstance) -> [f32; 2] {
    let block = (instance.render_state.light_coords >> 4) & 0xF;
    let sky = (instance.render_state.light_coords >> 20) & 0xF;
    [block as f32 / 15.0, sky as f32 / 15.0]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstPersonSwingAnimation {
    None,
    Whack,
    Stab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstPersonBlockUseKind {
    Shield,
    NonShieldBlock,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FirstPersonUseAnimation {
    None,
    EatDrink {
        use_duration_ticks: f32,
    },
    Block(FirstPersonBlockUseKind),
    Brush {
        use_duration_ticks: f32,
    },
    Bundle,
    Trident {
        use_duration_ticks: f32,
    },
    Bow {
        use_duration_ticks: f32,
    },
    Crossbow {
        use_duration_ticks: f32,
        charge_duration_ticks: f32,
    },
    Spear {
        kinetic_weapon: SpearKineticWeapon,
        use_duration_ticks: f32,
    },
}

fn supported_first_person_item_stack(
    stack: &ItemStackSummary,
    _item_runtime: &NativeItemRuntime,
) -> bool {
    stack.item_id.is_some()
}

fn first_person_stack_block_use_kind(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<FirstPersonBlockUseKind> {
    let Some(item_id) = stack.item_id else {
        return None;
    };
    if first_person_stack_has_added_component(stack, VANILLA_CONSUMABLE_COMPONENT_ID) {
        return None;
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_BLOCKS_ATTACKS_COMPONENT_ID)
    {
        return None;
    }
    let is_shield = item_runtime.item_resource_id(item_id) == Some("minecraft:shield");
    if is_shield {
        return Some(FirstPersonBlockUseKind::Shield);
    }
    first_person_stack_has_added_component(stack, VANILLA_BLOCKS_ATTACKS_COMPONENT_ID)
        .then_some(FirstPersonBlockUseKind::NonShieldBlock)
}

fn first_person_stack_supported_use_animation(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<FirstPersonUseAnimation> {
    match first_person_stack_resource_id(stack, item_runtime) {
        Some("minecraft:goat_horn") => return Some(FirstPersonUseAnimation::None),
        Some("minecraft:brush") => {
            return Some(FirstPersonUseAnimation::Brush {
                use_duration_ticks: VANILLA_BRUSH_USE_DURATION_TICKS,
            });
        }
        Some("minecraft:bundle") => return Some(FirstPersonUseAnimation::Bundle),
        Some("minecraft:trident") => {
            return Some(FirstPersonUseAnimation::Trident {
                use_duration_ticks: VANILLA_TRIDENT_USE_DURATION_TICKS,
            });
        }
        Some("minecraft:bow") => {
            return Some(FirstPersonUseAnimation::Bow {
                use_duration_ticks: VANILLA_BOW_USE_DURATION_TICKS,
            });
        }
        Some("minecraft:crossbow") => {
            return Some(FirstPersonUseAnimation::Crossbow {
                use_duration_ticks: VANILLA_CROSSBOW_USE_DURATION_TICKS,
                charge_duration_ticks: VANILLA_CROSSBOW_CHARGE_DURATION_TICKS,
            });
        }
        _ => {}
    }
    if let Some(consumable) = first_person_stack_consumable_summary(stack, item_runtime) {
        let use_duration_ticks = first_person_consumable_use_duration_ticks(consumable);
        return match consumable.animation {
            ItemUseAnimationSummary::None => Some(FirstPersonUseAnimation::None),
            ItemUseAnimationSummary::Eat | ItemUseAnimationSummary::Drink => {
                Some(FirstPersonUseAnimation::EatDrink { use_duration_ticks })
            }
            ItemUseAnimationSummary::Block => Some(FirstPersonUseAnimation::Block(
                first_person_stack_block_kind_for_animation(stack, item_runtime)?,
            )),
            ItemUseAnimationSummary::Bow => {
                Some(FirstPersonUseAnimation::Bow { use_duration_ticks })
            }
            ItemUseAnimationSummary::Trident => {
                Some(FirstPersonUseAnimation::Trident { use_duration_ticks })
            }
            ItemUseAnimationSummary::Crossbow
            | ItemUseAnimationSummary::Spyglass
            | ItemUseAnimationSummary::TootHorn => Some(FirstPersonUseAnimation::None),
            ItemUseAnimationSummary::Brush => {
                Some(FirstPersonUseAnimation::Brush { use_duration_ticks })
            }
            ItemUseAnimationSummary::Bundle => Some(FirstPersonUseAnimation::Bundle),
            ItemUseAnimationSummary::Spear => Some(FirstPersonUseAnimation::Spear {
                kinetic_weapon: first_person_stack_spear_kinetic_weapon(stack, item_runtime)?,
                use_duration_ticks,
            }),
        };
    }
    if let Some(kinetic_weapon) = first_person_stack_spear_kinetic_weapon(stack, item_runtime) {
        return Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon,
            use_duration_ticks: VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
        });
    }
    first_person_stack_block_use_kind(stack, item_runtime).map(FirstPersonUseAnimation::Block)
}

fn first_person_stack_resource_id<'a>(
    stack: &ItemStackSummary,
    item_runtime: &'a NativeItemRuntime,
) -> Option<&'a str> {
    stack
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
}

fn first_person_stack_is_charged_crossbow(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> bool {
    first_person_stack_resource_id(stack, item_runtime) == Some("minecraft:crossbow")
        && !stack.component_patch.charged_projectiles_items.is_empty()
}

fn first_person_stack_consumable_summary(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<ConsumableSummary> {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_CONSUMABLE_COMPONENT_ID)
    {
        return None;
    }
    stack.component_patch.consumable.or_else(|| {
        first_person_stack_has_added_component(stack, VANILLA_CONSUMABLE_COMPONENT_ID)
            .then(ConsumableSummary::default)
            .or_else(|| {
                stack
                    .item_id
                    .and_then(|item_id| item_runtime.item_default_consumable(item_id))
            })
    })
}

fn first_person_stack_block_kind_for_animation(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<FirstPersonBlockUseKind> {
    let item_id = stack.item_id?;
    if item_runtime.item_resource_id(item_id) == Some("minecraft:shield") {
        Some(FirstPersonBlockUseKind::Shield)
    } else {
        Some(FirstPersonBlockUseKind::NonShieldBlock)
    }
}

fn first_person_consumable_use_duration_ticks(consumable: ConsumableSummary) -> f32 {
    (consumable.consume_seconds * 20.0) as i32 as f32
}

fn first_person_stack_has_added_component(stack: &ItemStackSummary, component_id: i32) -> bool {
    stack.component_patch.added_type_ids.contains(&component_id)
        && !stack
            .component_patch
            .removed_type_ids
            .contains(&component_id)
}

fn first_person_stack_swing_animation(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> FirstPersonSwingAnimation {
    if first_person_stack_removes_swing_animation(stack) {
        return FirstPersonSwingAnimation::Whack;
    }
    if let Some(swing_animation) = stack.component_patch.swing_animation {
        return match swing_animation.animation_type {
            SwingAnimationTypeSummary::None => FirstPersonSwingAnimation::None,
            SwingAnimationTypeSummary::Whack => FirstPersonSwingAnimation::Whack,
            SwingAnimationTypeSummary::Stab => FirstPersonSwingAnimation::Stab,
        };
    }
    stack
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        .filter(|resource_id| first_person_resource_id_is_spear(resource_id))
        .map_or(FirstPersonSwingAnimation::Whack, |_| {
            FirstPersonSwingAnimation::Stab
        })
}

fn first_person_stack_spear_kinetic_weapon(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<SpearKineticWeapon> {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_KINETIC_WEAPON_COMPONENT_ID)
    {
        return None;
    }
    let resource_id = first_person_stack_resource_id(stack, item_runtime)?;
    default_spear_kinetic_weapon_for_resource_id(resource_id)
}

fn first_person_resource_id_is_spear(resource_id: &str) -> bool {
    matches!(
        resource_id,
        "minecraft:wooden_spear"
            | "minecraft:stone_spear"
            | "minecraft:copper_spear"
            | "minecraft:iron_spear"
            | "minecraft:golden_spear"
            | "minecraft:diamond_spear"
            | "minecraft:netherite_spear"
    )
}

fn first_person_stack_removes_swing_animation(stack: &ItemStackSummary) -> bool {
    stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_SWING_ANIMATION_COMPONENT_ID)
}

fn local_player_is_scoping(world: &WorldStore, item_runtime: &NativeItemRuntime) -> bool {
    world
        .local_using_item_item_id()
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        == Some("minecraft:spyglass")
}

fn first_person_block_fallback(context: BlockModelDisplayContext) -> BlockModelDisplayTransform {
    match context {
        BlockModelDisplayContext::FirstPersonLeftHand => BLOCK_FIRST_PERSON_LEFT_FALLBACK,
        BlockModelDisplayContext::FirstPersonRightHand => BLOCK_FIRST_PERSON_RIGHT_FALLBACK,
        _ => BLOCK_FIRST_PERSON_RIGHT_FALLBACK,
    }
}

fn first_person_item_arm_transform_from_camera_world(
    camera_world: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    camera_world
        * Mat4::from_translation(Vec3::new(
            invert * 0.56,
            -0.52 + inverse_arm_height * -0.6,
            -0.72,
        ))
}

fn first_person_one_handed_map_transform(
    camera_world: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    let attack = attack.clamp(0.0, 1.0);
    let sqrt_attack = attack.sqrt();
    let x_swing = (sqrt_attack * std::f32::consts::PI).sin();
    let x_swing_position = -0.5 * x_swing;
    let y_swing_position = 0.4 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.3 * (attack * std::f32::consts::PI).sin();
    let map_pose = camera_world
        * Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
        * Mat4::from_translation(Vec3::new(
            invert * 0.51,
            -0.08 + inverse_arm_height * -1.2,
            -0.75,
        ))
        * Mat4::from_translation(Vec3::new(
            invert * x_swing_position,
            y_swing_position - 0.3 * x_swing,
            z_swing_position,
        ))
        * Mat4::from_rotation_x((x_swing * -45.0).to_radians())
        * Mat4::from_rotation_y((invert * x_swing * -30.0).to_radians());
    first_person_apply_map_render_transform(map_pose)
}

fn first_person_two_handed_map_transform(
    camera_world: Mat4,
    x_rot: f32,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let sqrt_attack = attack.sqrt();
    let y_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let z_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let map_tilt = first_person_map_tilt(x_rot);
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
    let map_pose = camera_world
        * Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
            -0.72,
        ))
        * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * 20.0).to_radians())
        * Mat4::from_scale(Vec3::splat(2.0));
    first_person_apply_map_render_transform(map_pose)
}

fn first_person_render_player_arm_transform(
    parent_transform: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let sqrt_attack = attack.sqrt();
    let x_swing_position = -0.3 * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position = 0.4 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.4 * (attack * std::f32::consts::PI).sin();
    let z_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
    let y_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();

    parent_transform
        * Mat4::from_translation(Vec3::new(
            invert * (x_swing_position + 0.64000005),
            y_swing_position - 0.6 + inverse_arm_height * -0.6,
            z_swing_position - 0.71999997,
        ))
        * Mat4::from_rotation_y((invert * 45.0_f32).to_radians())
        * Mat4::from_rotation_y((invert * y_swing_rotation * 70.0).to_radians())
        * Mat4::from_rotation_z((invert * z_swing_rotation * -20.0).to_radians())
        * Mat4::from_translation(Vec3::new(invert * -1.0, 3.6, 3.5))
        * Mat4::from_rotation_z((invert * 120.0_f32).to_radians())
        * Mat4::from_rotation_x(200.0_f32.to_radians())
        * Mat4::from_rotation_y((invert * -135.0_f32).to_radians())
        * Mat4::from_translation(Vec3::new(invert * 5.6, 0.0, 0.0))
}

fn first_person_one_handed_map_arm_transform(
    camera_world: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    first_person_render_player_arm_transform(
        camera_world
            * Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
            * Mat4::from_rotation_z((invert * 10.0_f32).to_radians()),
        arm_left,
        inverse_arm_height,
        attack,
    )
}

fn first_person_two_handed_map_arm_transform(
    camera_world: Mat4,
    x_rot: f32,
    inverse_arm_height: f32,
    attack: f32,
    arm_left: bool,
) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let sqrt_attack = attack.sqrt();
    let y_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let z_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let map_tilt = first_person_map_tilt(x_rot);
    let invert = if arm_left { -1.0 } else { 1.0 };

    camera_world
        * Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
            -0.72,
        ))
        * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
        * Mat4::from_rotation_y(90.0_f32.to_radians())
        * Mat4::from_rotation_y(92.0_f32.to_radians())
        * Mat4::from_rotation_x(45.0_f32.to_radians())
        * Mat4::from_rotation_z((invert * -41.0_f32).to_radians())
        * Mat4::from_translation(Vec3::new(invert * 0.3, -1.1, 0.45))
}

fn first_person_apply_map_render_transform(transform: Mat4) -> Mat4 {
    transform
        * Mat4::from_rotation_y(180.0_f32.to_radians())
        * Mat4::from_rotation_z(180.0_f32.to_radians())
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_SCALE))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE))
}

fn first_person_map_tilt(x_rot: f32) -> f32 {
    let tilt = (1.0 - x_rot / 45.0 + 0.1).clamp(0.0, 1.0);
    -(tilt * std::f32::consts::PI).cos() * 0.5 + 0.5
}

fn first_person_apply_whack_swing(transform: Mat4, arm_left: bool, attack: f32) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let sqrt_attack = attack.sqrt();
    let x_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position = 0.2 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let y_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();

    transform
        * Mat4::from_translation(Vec3::new(
            invert * x_swing_position,
            y_swing_position,
            z_swing_position,
        ))
        * Mat4::from_rotation_y((invert * (45.0 + y_swing_rotation * -20.0)).to_radians())
        * Mat4::from_rotation_z((invert * xz_swing_rotation * -20.0).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * -80.0).to_radians())
        * Mat4::from_rotation_y((invert * -45.0).to_radians())
}

fn first_person_apply_stab_swing(transform: Mat4, arm_left: bool, attack: f32) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let starting_amount = first_person_ease_in_out_sine(first_person_progress(attack, 0.0, 0.05));
    let middle_amount = first_person_ease_out_back(first_person_progress(attack, 0.05, 0.2));
    let ending_amount = first_person_ease_in_out_expo(first_person_progress(attack, 0.4, 1.0));

    transform
        * Mat4::from_translation(Vec3::new(
            invert * 0.1 * (starting_amount - middle_amount),
            -0.075 * (starting_amount - ending_amount),
            0.65 * (starting_amount - middle_amount),
        ))
        * Mat4::from_rotation_x((-70.0 * (starting_amount - ending_amount)).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.25 * (ending_amount - middle_amount)))
}

fn first_person_apply_block_use_transform(transform: Mat4, arm_left: bool) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    transform
        * Mat4::from_translation(Vec3::new(invert * -0.14142136, 0.08, 0.14142136))
        * Mat4::from_rotation_x((-102.25_f32).to_radians())
        * Mat4::from_rotation_y((invert * 13.365_f32).to_radians())
        * Mat4::from_rotation_z((invert * 78.05_f32).to_radians())
}

fn first_person_apply_eat_drink_use_transform(
    camera_world: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let eat_transform = first_person_eat_drink_use_transform(
        arm_left,
        use_duration_ticks,
        using_item_ticks,
        partial_ticks,
    );
    first_person_item_arm_transform_from_camera_world(camera_world * eat_transform, arm_left, 0.0)
}

fn first_person_eat_drink_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    if use_duration_ticks <= 0.0 {
        return Mat4::IDENTITY;
    }
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let curr_usage_time = remaining_ticks - partial_ticks + 1.0;
    let scaled_usage_time = curr_usage_time / use_duration_ticks;
    let extra_height_offset = if scaled_usage_time < 0.8 {
        (curr_usage_time / 4.0 * std::f32::consts::PI).cos().abs() * 0.1
    } else {
        0.0
    };
    let eat_jiggle = 1.0 - scaled_usage_time.powf(27.0);
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(0.0, extra_height_offset, 0.0))
        * Mat4::from_translation(Vec3::new(eat_jiggle * 0.6 * invert, eat_jiggle * -0.5, 0.0))
        * Mat4::from_rotation_y((invert * eat_jiggle * 90.0).to_radians())
        * Mat4::from_rotation_x((eat_jiggle * 10.0).to_radians())
        * Mat4::from_rotation_z((invert * eat_jiggle * 30.0).to_radians())
}

fn first_person_apply_brush_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_brush_use_transform(
            arm_left,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_brush_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    if use_duration_ticks <= 0.0 {
        return Mat4::IDENTITY;
    }
    let brush_animation_remaining_ticks = (use_duration_ticks - using_item_ticks).max(0.0) % 10.0;
    let delta_since_last_update = brush_animation_remaining_ticks - partial_ticks + 1.0;
    let scaled_usage_time = 1.0 - delta_since_last_update / 10.0;
    let current_swipe_angle = -15.0 + 75.0 * (scaled_usage_time * 2.0 * std::f32::consts::PI).cos();

    if arm_left {
        Mat4::from_translation(Vec3::new(0.1, 0.83, 0.35))
            * Mat4::from_rotation_x((-80.0_f32).to_radians())
            * Mat4::from_rotation_y((-90.0_f32).to_radians())
            * Mat4::from_rotation_x(current_swipe_angle.to_radians())
            * Mat4::from_translation(Vec3::new(-0.3, 0.22, 0.35))
    } else {
        Mat4::from_translation(Vec3::new(-0.25, 0.22, 0.35))
            * Mat4::from_rotation_x((-80.0_f32).to_radians())
            * Mat4::from_rotation_y(90.0_f32.to_radians())
            * Mat4::from_rotation_x(current_swipe_angle.to_radians())
    }
}

fn first_person_apply_trident_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_trident_use_transform(
            arm_left,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_trident_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let power = (time_held / 10.0).min(1.0);
    let shake = if power > 0.1 {
        ((time_held - 0.1) * 1.3).sin() * (power - 0.1)
    } else {
        0.0
    };
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(invert * -0.5, 0.7, 0.1))
        * Mat4::from_rotation_x((-55.0_f32).to_radians())
        * Mat4::from_rotation_y((invert * 35.3_f32).to_radians())
        * Mat4::from_rotation_z((invert * -9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.2))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-invert * 45.0_f32).to_radians())
}

fn first_person_apply_bow_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_bow_use_transform(
            arm_left,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_bow_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let mut power = time_held / 20.0;
    power = (power * power + power * 2.0) / 3.0;
    if power > 1.0 {
        power = 1.0;
    }
    let shake = if power > 0.1 {
        ((time_held - 0.1) * 1.3).sin() * (power - 0.1)
    } else {
        0.0
    };
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(invert * -0.2785682, 0.18344387, 0.15731531))
        * Mat4::from_rotation_x((-13.935_f32).to_radians())
        * Mat4::from_rotation_y((invert * 35.3_f32).to_radians())
        * Mat4::from_rotation_z((invert * -9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-invert * 45.0_f32).to_radians())
}

fn first_person_apply_crossbow_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    charge_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_crossbow_use_transform(
            arm_left,
            use_duration_ticks,
            charge_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_crossbow_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    charge_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let mut power = time_held / charge_duration_ticks.max(1.0);
    if power > 1.0 {
        power = 1.0;
    }
    let shake = if power > 0.1 {
        ((time_held - 0.1) * 1.3).sin() * (power - 0.1)
    } else {
        0.0
    };
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(invert * -0.4785682, -0.094387, 0.05731531))
        * Mat4::from_rotation_x((-11.935_f32).to_radians())
        * Mat4::from_rotation_y((invert * 65.3_f32).to_radians())
        * Mat4::from_rotation_z((invert * -9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-invert * 45.0_f32).to_radians())
}

fn first_person_apply_spear_use_transform(
    camera_world: Mat4,
    arm_left: bool,
    kinetic_weapon: SpearKineticWeapon,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
    ticks_since_kinetic_hit_feedback: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    camera_world
        * Mat4::from_translation(Vec3::new(invert * 0.56, -0.52, -0.72))
        * first_person_spear_use_transform(
            arm_left,
            kinetic_weapon,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
            ticks_since_kinetic_hit_feedback,
        )
}

fn first_person_spear_use_transform(
    arm_left: bool,
    kinetic_weapon: SpearKineticWeapon,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
    ticks_since_kinetic_hit_feedback: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let params = kinetic_weapon.use_params(time_held);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let x_rotation_degrees = -65.0 * first_person_ease_in_out_back(params.raise_progress)
        - 35.0 * params.lower_progress
        + 100.0 * params.raise_back_progress
        - 0.5 * params.sway_scale_fast;
    let y_negative_axis_rotation_degrees = invert
        * (-90.0 * first_person_progress(params.raise_progress, 0.5, 0.55)
            + 90.0 * params.sway_progress
            + 2.0 * params.sway_scale_slow);

    Mat4::from_translation(Vec3::new(
        invert
            * (params.raise_progress * 0.15
                + params.raise_progress_end * -0.05
                + params.sway_progress * -0.1
                + params.sway_scale_slow * 0.005),
        params.raise_progress * -0.075
            + params.raise_progress_middle * 0.075
            + params.sway_scale_fast * 0.01,
        params.raise_progress_start * 0.05
            + params.raise_progress_end * -0.05
            + params.sway_scale_slow * 0.005,
    )) * first_person_rotate_around(
        Vec3::new(0.0, 0.1, 0.0),
        Mat4::from_rotation_x(x_rotation_degrees.to_radians()),
    ) * first_person_rotate_around(
        Vec3::new(invert * 0.15, 0.0, 0.0),
        Mat4::from_rotation_y((-y_negative_axis_rotation_degrees).to_radians()),
    ) * Mat4::from_translation(Vec3::new(
        0.0,
        -first_person_spear_kinetic_hit_feedback_amount(ticks_since_kinetic_hit_feedback),
        0.0,
    ))
}

fn first_person_apply_charged_crossbow_idle_transform(transform: Mat4, arm_left: bool) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    transform
        * Mat4::from_translation(Vec3::new(invert * -0.641864, 0.0, 0.0))
        * Mat4::from_rotation_y((invert * 10.0_f32).to_radians())
}

fn first_person_rotate_around(pivot: Vec3, rotation: Mat4) -> Mat4 {
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
}

fn first_person_spear_kinetic_hit_feedback_amount(ticks_since_feedback_start: f32) -> f32 {
    0.4 * (first_person_ease_out_quart(first_person_progress(ticks_since_feedback_start, 1.0, 3.0))
        - first_person_ease_in_out_sine(first_person_progress(
            ticks_since_feedback_start,
            3.0,
            10.0,
        )))
}

fn first_person_progress(value: f32, start: f32, end: f32) -> f32 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}

fn first_person_ease_in_out_sine(value: f32) -> f32 {
    -((std::f32::consts::PI * value).cos() - 1.0) / 2.0
}

fn first_person_ease_out_quart(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(4)
}

fn first_person_ease_out_back(value: f32) -> f32 {
    1.0 + 2.70158 * (value - 1.0).powi(3) + 1.70158 * (value - 1.0).powi(2)
}

fn first_person_ease_in_out_back(value: f32) -> f32 {
    if value < 0.5 {
        4.0 * value * value * (7.189819 * value - 2.5949094) / 2.0
    } else {
        let delta = 2.0 * value - 2.0;
        (delta * delta * (3.5949094 * delta + 2.5949094) + 2.0) / 2.0
    }
}

fn first_person_ease_in_out_expo(value: f32) -> f32 {
    if value < 0.5 {
        if value == 0.0 {
            0.0
        } else {
            2.0_f32.powf(20.0 * value - 10.0) / 2.0
        }
    } else if value == 1.0 {
        1.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * value + 10.0)) / 2.0
    }
}

fn first_person_camera_world_transform(pose: CameraPose) -> Mat4 {
    first_person_camera_view_matrix(pose).inverse()
}

fn first_person_camera_view_matrix(pose: CameraPose) -> Mat4 {
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let target = eye
        + if forward.length_squared() > 0.0 {
            forward
        } else {
            Vec3::Z
        };
    Mat4::look_at_rh(eye, target, Vec3::Y)
}

fn world_enchantment_keys(world: &WorldStore) -> Option<Vec<String>> {
    world
        .registry_content("minecraft:enchantment")
        .map(|registry| {
            registry
                .entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
}

fn world_trim_material_keys(world: &WorldStore) -> Option<Vec<String>> {
    world
        .registry_content("minecraft:trim_material")
        .map(|registry| {
            registry
                .entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
}

fn world_attribute_keys(world: &WorldStore) -> Option<Vec<String>> {
    world
        .registry_content("minecraft:attribute")
        .map(|registry| {
            registry
                .entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
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
    let using_item = entity_hand_using_item(instance, off_hand);
    let using_item_ticks = entity_use_elapsed_ticks(instance, using_item);
    for hand in humanoid_hand_attach_transforms(instance, left_arm) {
        bake_item_stack_at_transform(
            &stack,
            hand,
            context,
            left_arm,
            Some(instance.render_state.main_arm_left),
            entity_model_context_entity_type(instance.kind),
            using_item,
            using_item_ticks,
            enchantment_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = fox_held_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        entity_model_context_entity_type(instance.kind),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = dolphin_carried_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        entity_model_context_entity_type(instance.kind),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = witch_held_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        entity_model_context_entity_type(instance.kind),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
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
        let using_item = entity_hand_using_item(instance, left_hand);
        bake_item_stack_at_transform(
            &stack,
            hand,
            context,
            left_hand,
            Some(instance.render_state.main_arm_left),
            entity_model_context_entity_type(instance.kind),
            using_item,
            entity_use_elapsed_ticks(instance, using_item),
            enchantment_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
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
        let using_item = entity_hand_using_item(instance, left_hand);
        bake_item_stack_at_transform(
            &stack,
            hand,
            context,
            left_hand,
            Some(instance.render_state.main_arm_left),
            entity_model_context_entity_type(instance.kind),
            using_item,
            entity_use_elapsed_ticks(instance, using_item),
            enchantment_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = villager_crossed_arms_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        entity_model_context_entity_type(instance.kind),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(stack) = world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = panda_held_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack_at_transform(
        &stack,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        entity_model_context_entity_type(instance.kind),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
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
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
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
            Some(instance.render_state.main_arm_left),
            entity_model_context_entity_type(instance.kind),
            false,
            0.0,
            enchantment_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_HEAD_FALLBACK,
            GENERATED_HEAD_FALLBACK,
            item_runtime,
            terrain_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
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

fn entity_hand_using_item(instance: &EntityModelInstance, off_hand: bool) -> bool {
    instance.render_state.is_using_item && instance.render_state.use_item_off_hand == off_hand
}

fn entity_use_elapsed_ticks(instance: &EntityModelInstance, using_item: bool) -> f32 {
    if using_item {
        instance.render_state.crossbow_charge_ticks
    } else {
        0.0
    }
}

fn entity_model_context_entity_type(kind: EntityModelKind) -> Option<&'static str> {
    match kind {
        EntityModelKind::Player { .. } => Some("minecraft:player"),
        EntityModelKind::Humanoid { family, .. } => match family {
            HumanoidModelFamily::Player => Some("minecraft:player"),
            HumanoidModelFamily::Zombie => Some("minecraft:zombie"),
            HumanoidModelFamily::Skeleton => Some("minecraft:skeleton"),
            HumanoidModelFamily::Villager => Some("minecraft:villager"),
            HumanoidModelFamily::Illager => None,
            HumanoidModelFamily::ArmorStand => Some("minecraft:armor_stand"),
        },
        EntityModelKind::ArmorStand { .. } => Some("minecraft:armor_stand"),
        EntityModelKind::Zombie { .. } => Some("minecraft:zombie"),
        EntityModelKind::ZombieVariant { family, .. } => match family {
            ZombieVariantModelFamily::Husk => Some("minecraft:husk"),
            ZombieVariantModelFamily::Drowned => Some("minecraft:drowned"),
            ZombieVariantModelFamily::ZombieVillager => Some("minecraft:zombie_villager"),
        },
        EntityModelKind::Skeleton => Some("minecraft:skeleton"),
        EntityModelKind::SkeletonVariant { family } => match family {
            SkeletonModelFamily::Stray => Some("minecraft:stray"),
            SkeletonModelFamily::Parched => Some("minecraft:parched"),
            SkeletonModelFamily::WitherSkeleton => Some("minecraft:wither_skeleton"),
            SkeletonModelFamily::Bogged { .. } => Some("minecraft:bogged"),
        },
        EntityModelKind::Piglin { family, .. } => match family {
            PiglinModelFamily::Piglin => Some("minecraft:piglin"),
            PiglinModelFamily::PiglinBrute => Some("minecraft:piglin_brute"),
            PiglinModelFamily::ZombifiedPiglin => Some("minecraft:zombified_piglin"),
        },
        EntityModelKind::Giant => Some("minecraft:giant"),
        EntityModelKind::Vex { .. } => Some("minecraft:vex"),
        EntityModelKind::Allay => Some("minecraft:allay"),
        EntityModelKind::Dolphin { .. } => Some("minecraft:dolphin"),
        EntityModelKind::Villager { .. } => Some("minecraft:villager"),
        EntityModelKind::WanderingTrader => Some("minecraft:wandering_trader"),
        EntityModelKind::Fox { .. } => Some("minecraft:fox"),
        EntityModelKind::Panda { .. } => Some("minecraft:panda"),
        EntityModelKind::CopperGolem { .. } => Some("minecraft:copper_golem"),
        EntityModelKind::Witch => Some("minecraft:witch"),
        EntityModelKind::Illager { family } => match family {
            IllagerModelFamily::Evoker => Some("minecraft:evoker"),
            IllagerModelFamily::Illusioner => Some("minecraft:illusioner"),
            IllagerModelFamily::Pillager => Some("minecraft:pillager"),
            IllagerModelFamily::Vindicator => Some("minecraft:vindicator"),
        },
        _ => None,
    }
}

fn item_model_elapsed_ticks(ticks: f32) -> u32 {
    if ticks.is_finite() {
        ticks.max(0.0).floor() as u32
    } else {
        0
    }
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
    owner_main_hand_left: Option<bool>,
    context_entity_type: Option<&str>,
    using_item: bool,
    using_item_ticks: f32,
    enchantment_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_fallback: BlockModelDisplayTransform,
    generated_fallback: BlockModelDisplayTransform,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    block_meshes: &mut Vec<ItemModelMesh>,
    block_translucent_meshes: &mut Vec<ItemModelMesh>,
    block_glint_meshes: &mut Vec<ItemModelMesh>,
    block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_meshes: &mut Vec<ItemModelMesh>,
    flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_meshes: &mut Vec<ItemModelMesh>,
    flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
) {
    let Some(item_id) = stack.item_id else {
        return;
    };
    let retained = item_runtime.item_display_transform_for_stack(stack, context);

    // Block path.
    if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
        if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
            if !quads.is_empty() {
                let display = retained.unwrap_or(block_fallback);
                let transform = attach * display_matrix(&display, left_hand);
                push_mesh_set(
                    bake_item_model_meshes_with_light_and_overlay_and_foil_mode(
                        &quads,
                        transform,
                        ITEM_MODEL_FULL_BRIGHT_LIGHT,
                        ITEM_MODEL_NO_OVERLAY,
                        item_stack_foil_mode(stack, item_runtime, context),
                    ),
                    block_meshes,
                    block_translucent_meshes,
                    block_glint_meshes,
                    block_glint_translucent_meshes,
                );
                return;
            }
        }
    }

    // Flat path.
    let use_context = if using_item {
        item_runtime.item_model_use_context_for_stack_with_enchantment_keys(
            stack,
            item_model_elapsed_ticks(using_item_ticks),
            enchantment_keys,
        )
    } else {
        ItemModelUseContext::inactive()
    };
    let mut quads: Vec<ItemModelQuad> = Vec::new();
    for layer in item_runtime.generated_item_layers_for_stack_with_owner_registry_context(
        stack,
        context,
        owner_main_hand_left,
        context_entity_type,
        context_dimension,
        trim_material_keys,
        enchantment_keys,
        attribute_keys,
        using_item,
        use_context,
    ) {
        quads.extend(bake_generated_item_quads(
            &layer.mask,
            layer.rect,
            layer.tint,
            layer.translucent,
        ));
    }
    if quads.is_empty() {
        return;
    }
    let display = retained.unwrap_or(generated_fallback);
    let transform = attach * display_matrix(&display, left_hand);
    push_mesh_set(
        bake_item_model_meshes_with_light_and_overlay_and_foil_mode(
            &quads,
            transform,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
            ITEM_MODEL_NO_OVERLAY,
            item_stack_foil_mode(stack, item_runtime, context),
        ),
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
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
    foil: ItemModelFoil,
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
        foil,
    );
    mesh
}

fn item_cluster_mesh_with_base(
    quads: &[ItemModelQuad],
    base: Mat4,
    ground: &BlockModelDisplayTransform,
    light: [f32; 2],
    count: usize,
    seed: i64,
    foil: ItemModelFoil,
) -> ItemModelMeshSet {
    let ground_matrix = display_matrix(ground, false);
    let (_, depth) = ground_model_bounds(quads, ground_matrix);
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
        foil,
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

fn ominous_item_spawner_base_transform(position: [f32; 3], age_ticks: f32) -> Mat4 {
    let scale = if age_ticks <= OMINOUS_ITEM_SPAWNER_SCALE_IN_TICKS {
        age_ticks.clamp(0.0, OMINOUS_ITEM_SPAWNER_SCALE_IN_TICKS)
            / OMINOUS_ITEM_SPAWNER_SCALE_IN_TICKS
    } else {
        1.0
    };
    let spin_degrees =
        (age_ticks * OMINOUS_ITEM_SPAWNER_ROTATION_SPEED_DEGREES_PER_TICK).rem_euclid(360.0);
    Mat4::from_translation(Vec3::from_array(position))
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_rotation_y(spin_degrees.to_radians())
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
    glint: &mut Vec<ItemModelMesh>,
    glint_translucent: &mut Vec<ItemModelMesh>,
) {
    if !meshes.solid.is_empty() {
        solid.push(meshes.solid);
    }
    if !meshes.translucent.is_empty() {
        translucent.push(meshes.translucent);
    }
    if !meshes.glint.is_empty() {
        glint.push(meshes.glint);
    }
    if !meshes.glint_translucent.is_empty() {
        glint_translucent.push(meshes.glint_translucent);
    }
}

fn append_quads_to_mesh_set(
    meshes: &mut ItemModelMeshSet,
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
    foil: ItemModelFoil,
) {
    meshes.append_quads_with_light_and_overlay_and_foil(
        quads,
        transform,
        light,
        ITEM_MODEL_NO_OVERLAY,
        foil,
    );
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
    foil: ItemModelFoil,
) {
    let mut random = LegacyRandom::new(seed);
    if depth > FLAT_ITEM_DEPTH_THRESHOLD {
        append_quads_to_mesh_set(mesh, quads, base * ground, light, foil);
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let zo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let offset = Mat4::from_translation(Vec3::new(xo, yo, zo));
            append_quads_to_mesh_set(mesh, quads, base * offset * ground, light, foil);
        }
    } else {
        let offset_z = depth * 1.5;
        let mut z = -(offset_z * (count as f32 - 1.0) / 2.0);
        append_quads_to_mesh_set(
            mesh,
            quads,
            base * Mat4::from_translation(Vec3::new(0.0, 0.0, z)) * ground,
            light,
            foil,
        );
        z += offset_z;
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let offset = Mat4::from_translation(Vec3::new(xo, yo, z));
            append_quads_to_mesh_set(mesh, quads, base * offset * ground, light, foil);
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
    use crate::map_textures::map_color_rgba8;
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, BlockUpdate, CommonPlayerSpawnInfo,
        DataComponentPatchSummary, EntityAnimation, EntityDataValue, EntityDataValueKind,
        EntityEvent, EquipmentSlotUpdate, MapColorPatch, MapItemData, PlayLogin, SetEntityData,
        SetEquipment, SetPlayerInventory, SwingAnimationSummary, Vec3d,
    };
    use bbb_renderer::{EntityDefaultPlayerSkin, EntityPlayerSkin, PlayerModelPartVisibility};
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    const ENDERMAN_CARRY_STATE_DATA_ID: u8 = 16;
    const BLOCK_STATE_SERIALIZER_ID: i32 = 14;
    const OPTIONAL_BLOCK_STATE_SERIALIZER_ID: i32 = 15;
    const AIR_BLOCK_STATE_ID: i32 = 0;
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

    fn world_with_level_dimension(dimension: &str) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 1,
            hardcore: false,
            levels: vec![dimension.to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: dimension.to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world
    }

    fn first_person_test_map_stack(
        item_runtime: &NativeItemRuntime,
        map_id: i32,
    ) -> ItemStackSummary {
        let mut item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:filled_map"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        item.component_patch.map_id = Some(map_id);
        item
    }

    fn first_person_test_consumable_stack(
        item_runtime: &NativeItemRuntime,
        item_id: &str,
        animation: ItemUseAnimationSummary,
        consume_seconds: f32,
    ) -> ItemStackSummary {
        let mut item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        item.component_patch.added = 1;
        item.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        item.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds,
            animation,
        });
        item
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

    fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 1,
            value: EntityDataValueKind::Int(value),
        }
    }

    fn protocol_block_state_data(data_id: u8, block_state: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: BLOCK_STATE_SERIALIZER_ID,
            value: EntityDataValueKind::BlockState(block_state),
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

    fn world_with_primed_tnt(entity_id: i32, fuse: i32, block_state: Option<i32>) -> WorldStore {
        const PRIMED_TNT_FUSE_DATA_ID: u8 = 8;
        const PRIMED_TNT_BLOCK_STATE_DATA_ID: u8 = 9;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(entity_id, VANILLA_ENTITY_TYPE_TNT_ID));
        let mut values = vec![protocol_int_data(PRIMED_TNT_FUSE_DATA_ID, fuse)];
        if let Some(block_state) = block_state {
            values.push(protocol_block_state_data(
                PRIMED_TNT_BLOCK_STATE_DATA_ID,
                block_state,
            ));
        }
        assert!(world.apply_set_entity_data(SetEntityData {
            id: entity_id,
            values,
        }));
        world
    }

    fn world_with_falling_block(
        entity_id: i32,
        block_state_id: i32,
        position: [f64; 3],
    ) -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        let mut add = protocol_add_entity(entity_id, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID);
        add.data = block_state_id;
        add.position = Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        };
        world.apply_add_entity(add);
        world
    }

    fn insert_entity_block_test_chunk(world: &mut WorldStore) {
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_entity_block_test_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
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
            1.0,
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

        let attachments = entity_block_attachments(&[snow_golem, mooshroom], &world, None, 1.0);

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

        let attachments = entity_block_attachments(&[enderman], &world, None, 1.0);

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

        let attachments = entity_block_attachments(&[minecart], &world, None, 1.0);

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
        assert!(entity_block_attachments(&[hidden], &world, None, 1.0).is_empty());

        let outline = hidden.with_appears_glowing(true);
        let outline_attachments = entity_block_attachments(&[outline], &world, None, 1.0);
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

        let attachments = entity_block_attachments(&[primed], &world, None, 1.0);

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
        let unprimed_attachment = entity_block_attachments(&[unprimed], &world, None, 1.0)
            .pop()
            .expect("unprimed tnt attachment");
        assert_eq!(unprimed_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
        assert_ne!(unprimed_attachment.transform, attachments[0].transform);

        let odd_strobe = primed.with_minecart_tnt_fuse_remaining_in_ticks(6.0);
        let odd_strobe_attachment = entity_block_attachments(&[odd_strobe], &world, None, 1.0)
            .pop()
            .expect("odd strobe tnt attachment");
        assert_eq!(odd_strobe_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
    }

    #[test]
    fn entity_block_attachments_apply_primed_tnt_block_state_fuse_and_outline() {
        let entity_id = 134;
        let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
        let world = world_with_primed_tnt(entity_id, 4, Some(GRASS_BLOCK_STATE_ID));
        let light_coords = (5_u32 << 4) | (12_u32 << 20);
        let tnt = EntityModelInstance::no_render(entity_id, [0.0, 64.0, 0.0], 0.0)
            .with_light_coords(light_coords);

        let attachments = entity_block_attachments(&[tnt], &world, None, 0.5);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
        assert_eq!(attachments[0].properties, grass_props);
        assert_eq!(attachments[0].light, [5.0 / 15.0, 12.0 / 15.0]);
        assert_eq!(attachments[0].overlay, [15.0, 10.0]);
        assert_eq!(
            attachments[0].transform,
            primed_tnt_block_transform(&tnt, 4.5).unwrap()
        );
        assert!(!attachments[0].outline_only);

        let hidden = tnt.with_invisible(true);
        assert!(entity_block_attachments(&[hidden], &world, None, 0.5).is_empty());
        let outline = hidden.with_appears_glowing(true);
        let outline_attachments = entity_block_attachments(&[outline], &world, None, 0.5);
        assert_eq!(outline_attachments.len(), 1);
        assert!(outline_attachments[0].outline_only);

        let default_world = world_with_primed_tnt(entity_id, 6, None);
        let default_attachment = entity_block_attachments(&[tnt], &default_world, None, 0.0)
            .pop()
            .expect("default primed tnt attachment");
        assert_eq!(default_attachment.block_id.as_ref(), "minecraft:tnt");
        assert_eq!(
            default_attachment.properties,
            BTreeMap::from([("unstable".to_string(), "false".to_string())])
        );
        assert_eq!(default_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
    }

    #[test]
    fn entity_block_attachments_apply_falling_block_state_and_should_render_gate() {
        let entity_id = 135;
        let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
        let mut world = world_with_falling_block(entity_id, GRASS_BLOCK_STATE_ID, [1.5, 4.0, 2.5]);
        let light_coords = (7_u32 << 4) | (11_u32 << 20);
        let falling = EntityModelInstance::no_render(entity_id, [1.5, 4.0, 2.5], 90.0)
            .with_light_coords(light_coords);

        let attachments = entity_block_attachments(&[falling], &world, None, 1.0);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
        assert_eq!(attachments[0].properties, grass_props);
        assert_eq!(attachments[0].light, [7.0 / 15.0, 11.0 / 15.0]);
        assert_eq!(attachments[0].overlay, ITEM_MODEL_NO_OVERLAY);
        assert_eq!(
            attachments[0].transform,
            falling_block_transform(&falling).unwrap()
        );
        assert!(!attachments[0].outline_only);

        let hidden_outline = falling.with_invisible(true).with_appears_glowing(true);
        let hidden_attachments = entity_block_attachments(&[hidden_outline], &world, None, 1.0);
        assert_eq!(hidden_attachments.len(), 1);
        assert!(
            !hidden_attachments[0].outline_only,
            "vanilla FallingBlockRenderer.submit does not gate the body on isInvisible"
        );

        assert!(falling_block_should_render(
            &falling,
            &world,
            GRASS_BLOCK_STATE_ID
        ));
        insert_entity_block_test_chunk(&mut world);
        let feet = BlockPos { x: 1, y: 4, z: 2 };
        set_entity_block_test_block(&mut world, feet, GRASS_BLOCK_STATE_ID);
        assert!(!falling_block_should_render(
            &falling,
            &world,
            GRASS_BLOCK_STATE_ID
        ));
        assert!(entity_block_attachments(&[falling], &world, None, 1.0).is_empty());

        set_entity_block_test_block(&mut world, feet, AIR_BLOCK_STATE_ID);
        assert!(falling_block_should_render(
            &falling,
            &world,
            GRASS_BLOCK_STATE_ID
        ));
        assert_eq!(
            entity_block_attachments(&[falling], &world, None, 1.0).len(),
            1
        );
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
    fn first_person_item_models_bake_ordinary_local_hand_stack() {
        let root = unique_item_model_temp_dir("first-person-basic-item");
        write_flat_item_runtime_fixture(&root, &["hand_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

        let models = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            Some(CameraPose {
                position: [4.0, 64.0, -2.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: CameraPose::STANDING_EYE_HEIGHT,
            }),
            1.0,
        );

        assert!(models.block_meshes.is_empty());
        assert_eq!(models.flat_meshes.len(), 1);
        assert!(!models.flat_meshes[0].is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_local_player_whack_swing() {
        let root = unique_item_model_temp_dir("first-person-whack-swing");
        write_flat_item_runtime_fixture(&root, &["hand_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });
        let mut world = world_with_level_dimension("minecraft:overworld");
        world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

        let still = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(still.flat_meshes.len(), 1);

        assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
        world.advance_entity_client_animations(2);
        let swinging = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(swinging.flat_meshes.len(), 1);
        assert_ne!(
            still.flat_meshes[0], swinging.flat_meshes[0],
            "ordinary first-person WHACK items should receive vanilla swingArm transform"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_skip_unsupported_using_and_missing_map_data() {
        let root = unique_item_model_temp_dir("first-person-special-skip");
        write_flat_item_runtime_fixture(&root, &["hand_item", "filled_map"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let hand_item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut filled_map = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:filled_map"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        filled_map.component_patch.map_id = Some(7);
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut using_world = WorldStore::new();
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: hand_item,
        });
        using_world.set_local_using_item(true);
        assert!(first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        )
        .flat_meshes
        .is_empty());

        let mut special_world = WorldStore::new();
        special_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: filled_map,
        });
        assert!(first_person_item_models(
            &special_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        )
        .flat_meshes
        .is_empty());
        assert!(first_person_item_models(
            &special_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        )
        .map_surfaces
        .is_empty());
        let missing_map = first_person_item_models(
            &special_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(missing_map.map_background_surfaces.len(), 1);
        assert_eq!(
            missing_map.map_background_surfaces[0].submission.kind,
            FirstPersonMapBackgroundKind::Plain
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_render_two_handed_filled_map_surface() {
        let root = unique_item_model_temp_dir("first-person-map-two-handed");
        write_flat_item_runtime_fixture(&root, &["filled_map"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let packed_grass_high = (1 << 2) | 2;
        let map = first_person_test_map_stack(&item_runtime, 7);
        let pose = CameraPose {
            position: [2.0, 65.0, -4.0],
            y_rot: 35.0,
            x_rot: -15.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: map });
        assert!(world.apply_map_item_data(MapItemData {
            map_id: 7,
            scale: 0,
            locked: false,
            decorations: Some(Vec::new()),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![packed_grass_high],
            }),
        }));

        let models = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            Some(pose),
            1.0,
        );

        assert!(models.block_meshes.is_empty());
        assert!(models.flat_meshes.is_empty());
        assert_eq!(models.map_background_surfaces.len(), 1);
        assert_eq!(
            models.map_background_surfaces[0].submission.kind,
            FirstPersonMapBackgroundKind::Checkerboard
        );
        assert_eq!(
            models.map_background_surfaces[0].submission.transform,
            first_person_two_handed_map_transform(
                first_person_camera_world_transform(pose),
                pose.x_rot,
                0.0,
                0.0,
            )
        );
        assert_eq!(models.map_textures.len(), 1);
        assert_eq!(models.map_textures[0].map_id, 7);
        assert_eq!(
            &models.map_textures[0].rgba[0..4],
            &map_color_rgba8(packed_grass_high)
        );
        assert_eq!(models.map_surfaces.len(), 1);
        let surface = &models.map_surfaces[0];
        assert!(!surface.is_empty());
        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(surface.submission.map_id, 7);
        assert_eq!(surface.submission.texture.vanilla_path(), "minecraft:map/7");
        assert_eq!(surface.submission.light, ITEM_MODEL_FULL_BRIGHT_LIGHT);
        assert_eq!(
            surface.submission.transform,
            first_person_two_handed_map_transform(
                first_person_camera_world_transform(pose),
                pose.x_rot,
                0.0,
                0.0,
            )
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_render_one_handed_offhand_filled_map_surface() {
        let root = unique_item_model_temp_dir("first-person-map-one-handed");
        write_flat_item_runtime_fixture(&root, &["filled_map"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let map = first_person_test_map_stack(&item_runtime, 8);
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 20.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: map,
        });
        assert!(world.apply_map_item_data(MapItemData {
            map_id: 8,
            scale: 0,
            locked: false,
            decorations: Some(Vec::new()),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![(2 << 2) | 1],
            }),
        }));

        let models = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            Some(pose),
            1.0,
        );

        assert!(models.flat_meshes.is_empty());
        assert_eq!(models.map_background_surfaces.len(), 1);
        assert_eq!(
            models.map_background_surfaces[0].submission.kind,
            FirstPersonMapBackgroundKind::Checkerboard
        );
        assert_eq!(models.map_surfaces.len(), 1);
        let expected = first_person_one_handed_map_transform(
            first_person_camera_world_transform(pose),
            true,
            0.0,
            0.0,
        );
        assert_eq!(models.map_surfaces[0].submission.transform, expected);
        assert_ne!(
            models.map_surfaces[0].submission.transform,
            first_person_two_handed_map_transform(
                first_person_camera_world_transform(pose),
                pose.x_rot,
                0.0,
                0.0,
            ),
            "offhand maps use vanilla's one-handed map branch"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_player_arms_render_empty_main_hand_with_player_skin_light_and_parts() {
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 20.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let world = WorldStore::new();
        let parts = PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
        );
        let skin = EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve);
        let player = EntityModelInstance::player_with_skin(1, [0.0, 64.0, 0.0], 0.0, skin, parts)
            .with_light_coords((5_u32 << 4) | (11_u32 << 20));

        let arms = first_person_player_arms(&world, None, Some(&player), Some(pose), 1.0);

        assert_eq!(arms.len(), 1);
        assert!(!arms[0].left);
        assert_eq!(arms[0].skin, skin);
        assert!(arms[0].sleeve_visible);
        assert_eq!(arms[0].light, [5.0 / 15.0, 11.0 / 15.0]);
        assert!(arms[0].transform.abs_diff_eq(
            first_person_render_player_arm_transform(
                first_person_camera_world_transform(pose),
                false,
                0.0,
                0.0,
            ),
            1.0e-5,
        ));

        assert!(
            first_person_player_arms(
                &world,
                None,
                Some(&player.with_invisible(true)),
                Some(pose),
                1.0,
            )
            .is_empty(),
            "vanilla renderArmWithItem skips empty-hand player arms while invisible",
        );
        assert!(first_person_player_arms(&world, None, Some(&player), None, 1.0).is_empty());
    }

    #[test]
    fn first_person_player_arms_render_two_handed_and_one_handed_map_arms() {
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 20.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let camera_world = first_person_camera_world_transform(pose);
        let parts = PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::LEFT_SLEEVE_MASK
                | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
        );
        let player = EntityModelInstance::player_with_parts(1, [0.0, 64.0, 0.0], 0.0, false, parts);
        let mut map = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        map.component_patch.map_id = Some(7);

        let mut main_map_world = WorldStore::new();
        main_map_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: map.clone(),
        });
        let main_map_arms =
            first_person_player_arms(&main_map_world, None, Some(&player), Some(pose), 1.0);
        assert_eq!(main_map_arms.len(), 2);
        assert_eq!(
            main_map_arms.iter().map(|arm| arm.left).collect::<Vec<_>>(),
            vec![false, true],
        );
        assert!(main_map_arms[0].sleeve_visible);
        assert!(main_map_arms[1].sleeve_visible);
        assert!(main_map_arms[0].transform.abs_diff_eq(
            first_person_two_handed_map_arm_transform(camera_world, pose.x_rot, 0.0, 0.0, false),
            1.0e-5,
        ));
        assert!(main_map_arms[1].transform.abs_diff_eq(
            first_person_two_handed_map_arm_transform(camera_world, pose.x_rot, 0.0, 0.0, true),
            1.0e-5,
        ));

        let mut offhand_map_world = WorldStore::new();
        offhand_map_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: map,
        });
        let offhand_map_arms =
            first_person_player_arms(&offhand_map_world, None, Some(&player), Some(pose), 1.0);
        assert_eq!(
            offhand_map_arms
                .iter()
                .map(|arm| arm.left)
                .collect::<Vec<_>>(),
            vec![false, true],
            "offhand maps leave the empty main hand visible and render one map hand",
        );
        assert!(offhand_map_arms[0].transform.abs_diff_eq(
            first_person_render_player_arm_transform(camera_world, false, 0.0, 0.0),
            1.0e-5,
        ));
        assert!(offhand_map_arms[1].transform.abs_diff_eq(
            first_person_one_handed_map_arm_transform(camera_world, true, 0.0, 0.0),
            1.0e-5,
        ));
    }

    #[test]
    fn first_person_item_models_render_filled_map_decorations_and_text() {
        let item_runtime = NativeItemRuntime::empty_for_test();
        let mut map = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        map.component_patch.map_id = Some(9);
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: map });
        assert!(world.apply_map_item_data(MapItemData {
            map_id: 9,
            scale: 0,
            locked: false,
            decorations: Some(vec![
                bbb_protocol::packets::MapDecoration {
                    type_id: 0,
                    x: 0,
                    y: 0,
                    rot: 0,
                    name: Some("Player".to_string()),
                },
                bbb_protocol::packets::MapDecoration {
                    type_id: 1,
                    x: -20,
                    y: 30,
                    rot: 7,
                    name: Some("Frame".to_string()),
                },
            ]),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![(1 << 2) | 2],
            }),
        }));

        let models = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            Some(pose),
            1.0,
        );

        assert_eq!(models.map_surfaces.len(), 1);
        assert_eq!(models.map_background_surfaces.len(), 1);
        assert_eq!(
            models.map_background_surfaces[0].submission.kind,
            FirstPersonMapBackgroundKind::Checkerboard
        );
        assert_eq!(models.map_decoration_surfaces.len(), 2);
        assert_eq!(models.map_text_surfaces.len(), 2);
        let player = &models.map_decoration_surfaces[0];
        assert_eq!(player.submission.type_id, 0);
        assert_eq!(
            player.submission.texture.vanilla_sprite_id(),
            "minecraft:player"
        );
        assert_eq!(
            (player.submission.order, player.submission.submit_sequence),
            (0, 1)
        );
        assert_eq!(player.submission.decoration_index, 0);
        let frame = &models.map_decoration_surfaces[1];
        assert_eq!(frame.submission.type_id, 1);
        assert_eq!(
            frame.submission.texture.vanilla_sprite_id(),
            "minecraft:frame"
        );
        assert_eq!(
            (frame.submission.order, frame.submission.submit_sequence),
            (0, 2)
        );
        assert_eq!(frame.submission.decoration_index, 1);
        assert_eq!(models.map_text_surfaces[0].submission.type_id, 0);
        assert_eq!(models.map_text_surfaces[0].submission.text, "Player");
        assert_eq!(
            (
                models.map_text_surfaces[0].submission.order,
                models.map_text_surfaces[0].submission.submit_sequence
            ),
            (1, 0)
        );
        assert_eq!(models.map_text_surfaces[1].submission.type_id, 1);
        assert_eq!(models.map_text_surfaces[1].submission.text, "Frame");
        assert_eq!(
            (
                models.map_text_surfaces[1].submission.order,
                models.map_text_surfaces[1].submission.submit_sequence
            ),
            (1, 1)
        );
    }

    #[test]
    fn first_person_item_models_render_idle_spyglass_and_hide_when_scoping() {
        let root = unique_item_model_temp_dir("first-person-spyglass-use");
        write_flat_item_runtime_fixture(&root, &["spyglass", "off_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let spyglass = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:spyglass"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let off_item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:off_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: spyglass.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut scoping_world = WorldStore::new();
        scoping_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: spyglass,
        });
        scoping_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: off_item,
        });
        scoping_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        assert!(
            first_person_item_models(
                &scoping_world,
                Some(&item_runtime),
                &TerrainTextureState::default(),
                camera,
                1.0,
            )
            .flat_meshes
            .is_empty(),
            "vanilla ItemInHandRenderer skips hands/items while player.isScoping"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_block_use_pose_for_shield_and_blocks_attacks() {
        let root = unique_item_model_temp_dir("first-person-block-use");
        write_flat_item_runtime_fixture(&root, &["shield", "guard_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut shield = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:shield"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        shield.component_patch.added = 1;
        shield.component_patch.added_type_ids = vec![VANILLA_SWING_ANIMATION_COMPONENT_ID];
        shield.component_patch.swing_animation = Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::None,
            duration: 0,
        });
        let mut guard_item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:guard_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        guard_item.component_patch.added = 1;
        guard_item.component_patch.added_type_ids = vec![VANILLA_BLOCKS_ATTACKS_COMPONENT_ID];
        let mut consumable_guard = guard_item.clone();
        consumable_guard
            .component_patch
            .added_type_ids
            .push(VANILLA_CONSUMABLE_COMPONENT_ID);
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut shield_world = WorldStore::new();
        shield_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: shield,
        });
        let idle_shield = first_person_item_models(
            &shield_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(idle_shield.flat_meshes.len(), 1);
        shield_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        let blocking_shield = first_person_item_models(
            &shield_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(blocking_shield.flat_meshes.len(), 1);
        assert_eq!(
            idle_shield.flat_meshes[0], blocking_shield.flat_meshes[0],
            "vanilla ShieldItem BLOCK use keeps only the base arm transform"
        );

        let mut idle_guard_world = WorldStore::new();
        idle_guard_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: guard_item.clone(),
        });
        let idle_guard = first_person_item_models(
            &idle_guard_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(idle_guard.flat_meshes.len(), 1);

        let mut blocking_guard_world = WorldStore::new();
        blocking_guard_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: guard_item,
        });
        blocking_guard_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        let blocking_guard = first_person_item_models(
            &blocking_guard_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(blocking_guard.flat_meshes.len(), 1);
        assert_ne!(
            idle_guard.flat_meshes[0], blocking_guard.flat_meshes[0],
            "non-shield BLOCK use applies ItemInHandRenderer's fixed BLOCK transform"
        );

        assert_eq!(
            first_person_stack_block_use_kind(&consumable_guard, &item_runtime),
            None,
            "CONSUMABLE wins over BLOCKS_ATTACKS in Item.getUseAnimation"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_consumable_eat_and_drink_use_pose() {
        let root = unique_item_model_temp_dir("first-person-consumable-use");
        write_flat_item_runtime_fixture(&root, &["snack"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut snack = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:snack"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        snack.component_patch.added = 1;
        snack.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        snack.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        });
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: snack.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut eating_world = WorldStore::new();
        eating_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: snack.clone(),
        });
        eating_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        eating_world.advance_local_using_item_ticks(12);
        let eating = first_person_item_models(
            &eating_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(eating.flat_meshes.len(), 1);
        assert_ne!(
            idle.flat_meshes[0], eating.flat_meshes[0],
            "consumable EAT applies ItemInHandRenderer.applyEatTransform before the arm transform"
        );

        let mut drink = snack;
        drink.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Drink,
        });
        let mut drinking_world = WorldStore::new();
        drinking_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: drink,
        });
        drinking_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        drinking_world.advance_local_using_item_ticks(12);
        let drinking = first_person_item_models(
            &drinking_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(drinking.flat_meshes.len(), 1);
        assert_eq!(
            eating.flat_meshes[0], drinking.flat_meshes[0],
            "vanilla EAT and DRINK share the first-person applyEatTransform matrix"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_default_consumable_use_pose() {
        let root = unique_item_model_temp_dir("first-person-default-consumable-use");
        write_default_consumable_item_runtime_fixture(&root, "snack");
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let snack = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:snack"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        assert_eq!(
            snack
                .item_id
                .and_then(|item_id| item_runtime.item_default_consumable(item_id)),
            Some(ConsumableSummary {
                consume_seconds: 1.6,
                animation: ItemUseAnimationSummary::Eat,
            })
        );
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: snack.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut eating_world = WorldStore::new();
        eating_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: snack,
        });
        eating_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        eating_world.advance_local_using_item_ticks(12);
        let eating = first_person_item_models(
            &eating_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(eating.flat_meshes.len(), 1);
        assert_ne!(
            idle.flat_meshes[0], eating.flat_meshes[0],
            "default item prototype CONSUMABLE applies vanilla EAT first-person use pose"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_custom_consumable_non_eat_use_animations() {
        let root = unique_item_model_temp_dir("first-person-custom-consumable-use");
        write_flat_item_runtime_fixture(&root, &["snack", "off_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let snack_bow = first_person_test_consumable_stack(
            &item_runtime,
            "minecraft:snack",
            ItemUseAnimationSummary::Bow,
            1.6,
        );
        assert_eq!(
            first_person_stack_supported_use_animation(&snack_bow, &item_runtime),
            Some(FirstPersonUseAnimation::Bow {
                use_duration_ticks: 32.0,
            })
        );
        assert_eq!(
            first_person_stack_supported_use_animation(
                &first_person_test_consumable_stack(
                    &item_runtime,
                    "minecraft:snack",
                    ItemUseAnimationSummary::Trident,
                    1.6,
                ),
                &item_runtime,
            ),
            Some(FirstPersonUseAnimation::Trident {
                use_duration_ticks: 32.0,
            })
        );
        assert_eq!(
            first_person_stack_supported_use_animation(
                &first_person_test_consumable_stack(
                    &item_runtime,
                    "minecraft:snack",
                    ItemUseAnimationSummary::Brush,
                    1.6,
                ),
                &item_runtime,
            ),
            Some(FirstPersonUseAnimation::Brush {
                use_duration_ticks: 32.0,
            })
        );
        assert_eq!(
            first_person_stack_supported_use_animation(
                &first_person_test_consumable_stack(
                    &item_runtime,
                    "minecraft:snack",
                    ItemUseAnimationSummary::Bundle,
                    1.6,
                ),
                &item_runtime,
            ),
            Some(FirstPersonUseAnimation::Bundle)
        );
        for animation in [
            ItemUseAnimationSummary::None,
            ItemUseAnimationSummary::Crossbow,
            ItemUseAnimationSummary::Spyglass,
            ItemUseAnimationSummary::TootHorn,
        ] {
            assert_eq!(
                first_person_stack_supported_use_animation(
                    &first_person_test_consumable_stack(
                        &item_runtime,
                        "minecraft:snack",
                        animation,
                        1.6,
                    ),
                    &item_runtime,
                ),
                Some(FirstPersonUseAnimation::None),
                "generic {animation:?} has no ItemInHandRenderer switch case"
            );
        }
        assert_eq!(
            first_person_stack_supported_use_animation(
                &first_person_test_consumable_stack(
                    &item_runtime,
                    "minecraft:snack",
                    ItemUseAnimationSummary::Spear,
                    1.6,
                ),
                &item_runtime,
            ),
            None,
            "SPEAR needs the kinetic SpearAnimations.firstPersonUse path"
        );

        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });
        let off_item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:off_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: snack_bow.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut using_world = WorldStore::new();
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: snack_bow,
        });
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: off_item,
        });
        using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        using_world.advance_local_using_item_ticks(12);
        let using = first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(
            using.flat_meshes.len(),
            2,
            "generic BOW animation is not an Items.BOW/CROSSBOW hand-selection special case"
        );
        assert_ne!(
            idle.flat_meshes[0], using.flat_meshes[0],
            "generic BOW consumables still apply the vanilla draw transform"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_goat_horn_base_use_pose() {
        let root = unique_item_model_temp_dir("first-person-goat-horn-use");
        write_flat_item_runtime_fixture(&root, &["goat_horn"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut goat_horn = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:goat_horn"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        goat_horn.component_patch.added = 1;
        goat_horn.component_patch.added_type_ids = vec![VANILLA_SWING_ANIMATION_COMPONENT_ID];
        goat_horn.component_patch.swing_animation = Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::None,
            duration: 0,
        });
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: goat_horn.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut tooting_world = WorldStore::new();
        tooting_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: goat_horn,
        });
        tooting_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        let tooting = first_person_item_models(
            &tooting_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(tooting.flat_meshes.len(), 1);
        assert_eq!(
            idle.flat_meshes[0], tooting.flat_meshes[0],
            "vanilla TOOT_HORN first-person use keeps only the base arm transform"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_brush_use_pose() {
        let root = unique_item_model_temp_dir("first-person-brush-use");
        write_flat_item_runtime_fixture(&root, &["brush"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let brush = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:brush"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut edible_brush = brush.clone();
        edible_brush.component_patch.added = 1;
        edible_brush.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        edible_brush.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        });
        assert_eq!(
            first_person_stack_supported_use_animation(&edible_brush, &item_runtime),
            Some(FirstPersonUseAnimation::Brush {
                use_duration_ticks: VANILLA_BRUSH_USE_DURATION_TICKS,
            }),
            "BrushItem.getUseAnimation overrides stack CONSUMABLE data"
        );
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: brush.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut brushing_world = WorldStore::new();
        brushing_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: brush,
        });
        brushing_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        brushing_world.advance_local_using_item_ticks(12);
        let brushing = first_person_item_models(
            &brushing_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(brushing.flat_meshes.len(), 1);
        assert_ne!(
            idle.flat_meshes[0], brushing.flat_meshes[0],
            "BRUSH use applies ItemInHandRenderer.applyBrushTransform after the arm transform"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_bundle_use_swing() {
        let root = unique_item_model_temp_dir("first-person-bundle-use");
        write_flat_item_runtime_fixture(&root, &["bundle"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let bundle = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:bundle"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut edible_bundle = bundle.clone();
        edible_bundle.component_patch.added = 1;
        edible_bundle.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        edible_bundle.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        });
        assert_eq!(
            first_person_stack_supported_use_animation(&edible_bundle, &item_runtime),
            Some(FirstPersonUseAnimation::Bundle),
            "BundleItem.getUseAnimation overrides stack CONSUMABLE data"
        );
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut world = world_with_level_dimension("minecraft:overworld");
        world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: bundle,
        });
        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        let still = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(still.flat_meshes.len(), 1);

        assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
        world.advance_entity_client_animations(2);
        let swinging = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(swinging.flat_meshes.len(), 1);
        assert_ne!(
            still.flat_meshes[0], swinging.flat_meshes[0],
            "BUNDLE use applies ItemInHandRenderer.swingArm while the item is being used"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_trident_use_pose() {
        let root = unique_item_model_temp_dir("first-person-trident-use");
        write_flat_item_runtime_fixture(&root, &["trident"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let trident = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:trident"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut edible_trident = trident.clone();
        edible_trident.component_patch.added = 1;
        edible_trident.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        edible_trident.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        });
        assert_eq!(
            first_person_stack_supported_use_animation(&edible_trident, &item_runtime),
            Some(FirstPersonUseAnimation::Trident {
                use_duration_ticks: VANILLA_TRIDENT_USE_DURATION_TICKS,
            }),
            "TridentItem.getUseAnimation overrides stack CONSUMABLE data"
        );
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: trident.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut using_world = WorldStore::new();
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: trident,
        });
        using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        using_world.advance_local_using_item_ticks(12);
        let using = first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(using.flat_meshes.len(), 1);
        assert_ne!(
            idle.flat_meshes[0], using.flat_meshes[0],
            "TRIDENT use applies ItemInHandRenderer's throw-charge transform"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_bow_use_pose_and_hand_selection() {
        let root = unique_item_model_temp_dir("first-person-bow-use");
        write_flat_item_runtime_fixture(&root, &["bow", "off_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let bow = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:bow"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let off_item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:off_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut edible_bow = bow.clone();
        edible_bow.component_patch.added = 1;
        edible_bow.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        edible_bow.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        });
        assert_eq!(
            first_person_stack_supported_use_animation(&edible_bow, &item_runtime),
            Some(FirstPersonUseAnimation::Bow {
                use_duration_ticks: VANILLA_BOW_USE_DURATION_TICKS,
            }),
            "BowItem.getUseAnimation overrides stack CONSUMABLE data"
        );
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: bow.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut using_world = WorldStore::new();
        using_world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: bow });
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: off_item,
        });
        using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        using_world.advance_local_using_item_ticks(12);
        let using = first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(
            using.flat_meshes.len(),
            1,
            "vanilla renders only the used hand while drawing a bow"
        );
        assert_ne!(
            idle.flat_meshes[0], using.flat_meshes[0],
            "BOW use applies ItemInHandRenderer's draw transform"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_crossbow_use_and_charged_pose() {
        let root = unique_item_model_temp_dir("first-person-crossbow-use");
        write_flat_item_runtime_fixture(&root, &["crossbow", "off_item"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let crossbow = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:crossbow"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let off_item = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:off_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut edible_crossbow = crossbow.clone();
        edible_crossbow.component_patch.added = 1;
        edible_crossbow.component_patch.added_type_ids = vec![VANILLA_CONSUMABLE_COMPONENT_ID];
        edible_crossbow.component_patch.consumable = Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        });
        assert_eq!(
            first_person_stack_supported_use_animation(&edible_crossbow, &item_runtime),
            Some(FirstPersonUseAnimation::Crossbow {
                use_duration_ticks: VANILLA_CROSSBOW_USE_DURATION_TICKS,
                charge_duration_ticks: VANILLA_CROSSBOW_CHARGE_DURATION_TICKS,
            }),
            "CrossbowItem.getUseAnimation overrides stack CONSUMABLE data"
        );
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });

        let mut idle_world = WorldStore::new();
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: crossbow.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut using_world = WorldStore::new();
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: crossbow.clone(),
        });
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 40,
            item: off_item,
        });
        using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        using_world.advance_local_using_item_ticks(12);
        let using = first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(
            using.flat_meshes.len(),
            1,
            "vanilla renders only the used hand while drawing a crossbow"
        );
        assert_ne!(
            idle.flat_meshes[0], using.flat_meshes[0],
            "uncharged CROSSBOW use applies ItemInHandRenderer's draw transform"
        );

        let mut charged_crossbow = crossbow;
        charged_crossbow.component_patch.charged_projectiles_items =
            vec![ItemStackTemplateSummary {
                item_id: 99,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }];
        let mut charged_world = WorldStore::new();
        charged_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: charged_crossbow,
        });
        let charged = first_person_item_models(
            &charged_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(charged.flat_meshes.len(), 1);
        assert_ne!(
            idle.flat_meshes[0], charged.flat_meshes[0],
            "charged main-hand crossbows apply the vanilla idle hold offset"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_stack_supported_use_animation_resolves_spear_kinetic_weapon() {
        let root = unique_item_model_temp_dir("first-person-spear-use-animation");
        write_flat_item_runtime_fixture(&root, &["wooden_spear", "snack"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let kinetic_weapon = SpearKineticWeapon {
            delay_ticks: 15.0,
            dismount_duration_ticks: 100.0,
            knockback_duration_ticks: 200.0,
            damage_duration_ticks: 300.0,
            forward_movement: 0.38,
        };
        let wooden_spear = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        assert_eq!(
            first_person_stack_supported_use_animation(&wooden_spear, &item_runtime),
            Some(FirstPersonUseAnimation::Spear {
                kinetic_weapon,
                use_duration_ticks: VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
            })
        );

        let mut removed_kinetic = wooden_spear.clone();
        removed_kinetic
            .component_patch
            .removed_type_ids
            .push(VANILLA_KINETIC_WEAPON_COMPONENT_ID);
        assert_eq!(
            first_person_stack_supported_use_animation(&removed_kinetic, &item_runtime),
            None,
            "removing the prototype KINETIC_WEAPON component disables vanilla SPEAR use"
        );

        let snack_spear = first_person_test_consumable_stack(
            &item_runtime,
            "minecraft:snack",
            ItemUseAnimationSummary::Spear,
            1.6,
        );
        assert_eq!(
            first_person_stack_supported_use_animation(&snack_spear, &item_runtime),
            None,
            "generic SPEAR consumables need readable kinetic weapon data"
        );

        let wooden_spear_consumable = first_person_test_consumable_stack(
            &item_runtime,
            "minecraft:wooden_spear",
            ItemUseAnimationSummary::Spear,
            1.6,
        );
        assert_eq!(
            first_person_stack_supported_use_animation(&wooden_spear_consumable, &item_runtime),
            Some(FirstPersonUseAnimation::Spear {
                kinetic_weapon,
                use_duration_ticks: 32.0,
            }),
            "consumable SPEAR keeps the consumable use duration while using the item kinetic component"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_spear_kinetic_use_pose_and_feedback() {
        let root = unique_item_model_temp_dir("first-person-spear-kinetic-use");
        write_flat_item_runtime_fixture(&root, &["wooden_spear"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let wooden_spear = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });
        let mut idle_world = world_with_level_dimension("minecraft:overworld");
        idle_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        idle_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: wooden_spear.clone(),
        });
        let idle = first_person_item_models(
            &idle_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(idle.flat_meshes.len(), 1);

        let mut using_world = world_with_level_dimension("minecraft:overworld");
        using_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        using_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: wooden_spear.clone(),
        });
        using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        using_world.advance_local_using_item_ticks(20);
        let using = first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(using.flat_meshes.len(), 1);
        assert_ne!(
            idle.flat_meshes[0], using.flat_meshes[0],
            "SPEAR use skips applyItemArmTransform and applies SpearAnimations.firstPersonUse"
        );

        let mut feedback_world = world_with_level_dimension("minecraft:overworld");
        feedback_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        feedback_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: wooden_spear,
        });
        feedback_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        feedback_world.advance_local_using_item_ticks(20);
        assert!(feedback_world.apply_entity_event(EntityEvent {
            entity_id: 1,
            event_id: 2,
        }));
        feedback_world.advance_entity_client_animations(2);
        let feedback = first_person_item_models(
            &feedback_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(feedback.flat_meshes.len(), 1);
        assert_eq!(
            feedback_world.local_player_ticks_since_kinetic_hit_feedback(0.5),
            2.5
        );
        assert_ne!(
            using.flat_meshes[0], feedback.flat_meshes[0],
            "local kinetic hit feedback contributes the vanilla first-person spear kick"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_item_models_apply_stab_swing_for_spear_and_stack_patch() {
        let root = unique_item_model_temp_dir("first-person-stab-swing");
        write_flat_item_runtime_fixture(&root, &["stab_item", "wooden_spear"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut patched_stab = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:stab_item"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        patched_stab.component_patch.swing_animation = Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::Stab,
            duration: 13,
        });
        let default_spear = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let camera = Some(CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        });
        let assert_stab_moves = |item: ItemStackSummary, label: &str| {
            let mut world = world_with_level_dimension("minecraft:overworld");
            world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
            world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

            let still = first_person_item_models(
                &world,
                Some(&item_runtime),
                &TerrainTextureState::default(),
                camera,
                1.0,
            );
            assert_eq!(still.flat_meshes.len(), 1, "{label} should render");

            assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
            world.advance_entity_client_animations(2);
            let swinging = first_person_item_models(
                &world,
                Some(&item_runtime),
                &TerrainTextureState::default(),
                camera,
                1.0,
            );
            assert_eq!(
                swinging.flat_meshes.len(),
                1,
                "{label} should keep rendering"
            );
            assert_ne!(
                still.flat_meshes[0], swinging.flat_meshes[0],
                "{label} should receive vanilla SpearAnimations.firstPersonAttack"
            );
        };
        assert_stab_moves(patched_stab, "stack-patch STAB");
        assert_stab_moves(default_spear, "default spear STAB");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_person_arm_transform_matches_vanilla_apply_item_arm_transform() {
        let pose = CameraPose {
            position: [10.0, 64.0, -5.0],
            y_rot: 35.0,
            x_rot: -15.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let view = first_person_camera_view_matrix(pose);
        let camera_world = first_person_camera_world_transform(pose);
        let right =
            view * first_person_item_arm_transform_from_camera_world(camera_world, false, 0.0);
        let left =
            view * first_person_item_arm_transform_from_camera_world(camera_world, true, 0.0);
        let right_origin = right.transform_point3(Vec3::ZERO);
        let left_origin = left.transform_point3(Vec3::ZERO);

        assert!(
            right_origin.abs_diff_eq(Vec3::new(0.56, -0.52, -0.72), 1.0e-4),
            "right origin {right_origin:?}"
        );
        assert!(
            left_origin.abs_diff_eq(Vec3::new(-0.56, -0.52, -0.72), 1.0e-4),
            "left origin {left_origin:?}"
        );
    }

    #[test]
    fn first_person_player_arm_transform_matches_vanilla_render_player_arm() {
        let attack = 0.25_f32;
        let inverse_arm_height = 0.2_f32;
        let expected = |arm_left: bool| {
            let invert = if arm_left { -1.0 } else { 1.0 };
            let sqrt_attack = attack.sqrt();
            let x_swing_position = -0.3 * (sqrt_attack * std::f32::consts::PI).sin();
            let y_swing_position = 0.4 * (sqrt_attack * std::f32::consts::TAU).sin();
            let z_swing_position = -0.4 * (attack * std::f32::consts::PI).sin();
            let z_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
            let y_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();

            Mat4::from_translation(Vec3::new(
                invert * (x_swing_position + 0.64000005),
                y_swing_position - 0.6 + inverse_arm_height * -0.6,
                z_swing_position - 0.71999997,
            )) * Mat4::from_rotation_y((invert * 45.0_f32).to_radians())
                * Mat4::from_rotation_y((invert * y_swing_rotation * 70.0).to_radians())
                * Mat4::from_rotation_z((invert * z_swing_rotation * -20.0).to_radians())
                * Mat4::from_translation(Vec3::new(invert * -1.0, 3.6, 3.5))
                * Mat4::from_rotation_z((invert * 120.0_f32).to_radians())
                * Mat4::from_rotation_x(200.0_f32.to_radians())
                * Mat4::from_rotation_y((invert * -135.0_f32).to_radians())
                * Mat4::from_translation(Vec3::new(invert * 5.6, 0.0, 0.0))
        };

        assert!(first_person_render_player_arm_transform(
            Mat4::IDENTITY,
            false,
            inverse_arm_height,
            attack,
        )
        .abs_diff_eq(expected(false), 1.0e-5));
        assert!(first_person_render_player_arm_transform(
            Mat4::IDENTITY,
            true,
            inverse_arm_height,
            attack,
        )
        .abs_diff_eq(expected(true), 1.0e-5));
    }

    #[test]
    fn first_person_map_arm_transform_matches_vanilla_render_map_hand() {
        let attack = 0.25_f32;
        let x_rot = 20.0_f32;
        let inverse_arm_height = 0.2_f32;
        let sqrt_attack = attack.sqrt();
        let y_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
        let z_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
        let map_tilt = first_person_map_tilt(x_rot);
        let base =
            Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
                * Mat4::from_translation(Vec3::new(
                    0.0,
                    0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
                    -0.72,
                ))
                * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
                * Mat4::from_rotation_y(90.0_f32.to_radians());
        let expected_map_hand = |arm_left: bool| {
            let invert = if arm_left { -1.0 } else { 1.0 };
            base * Mat4::from_rotation_y(92.0_f32.to_radians())
                * Mat4::from_rotation_x(45.0_f32.to_radians())
                * Mat4::from_rotation_z((invert * -41.0_f32).to_radians())
                * Mat4::from_translation(Vec3::new(invert * 0.3, -1.1, 0.45))
        };

        assert!(first_person_two_handed_map_arm_transform(
            Mat4::IDENTITY,
            x_rot,
            inverse_arm_height,
            attack,
            false,
        )
        .abs_diff_eq(expected_map_hand(false), 1.0e-5));
        assert!(first_person_two_handed_map_arm_transform(
            Mat4::IDENTITY,
            x_rot,
            inverse_arm_height,
            attack,
            true,
        )
        .abs_diff_eq(expected_map_hand(true), 1.0e-5));
    }

    #[test]
    fn first_person_map_tilt_matches_vanilla_calculate_map_tilt() {
        assert!((first_person_map_tilt(0.0) - 1.0).abs() < 1.0e-6);
        let mid_tilt = 1.0 - 45.0 / 45.0 + 0.1;
        let expected_mid = -(mid_tilt * std::f32::consts::PI).cos() * 0.5 + 0.5;
        assert!((first_person_map_tilt(45.0) - expected_mid).abs() < 1.0e-6);
        assert!((first_person_map_tilt(90.0) - 0.0).abs() < 1.0e-6);
    }

    #[test]
    fn first_person_one_handed_map_transform_matches_vanilla_render_one_handed_map() {
        let attack = 0.25_f32;
        let invert = -1.0_f32;
        let sqrt_attack = attack.sqrt();
        let x_swing = (sqrt_attack * std::f32::consts::PI).sin();
        let expected = Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
            * Mat4::from_translation(Vec3::new(invert * 0.51, -0.08, -0.75))
            * Mat4::from_translation(Vec3::new(
                invert * -0.5 * x_swing,
                0.4 * (sqrt_attack * std::f32::consts::TAU).sin() - 0.3 * x_swing,
                -0.3 * (attack * std::f32::consts::PI).sin(),
            ))
            * Mat4::from_rotation_x((x_swing * -45.0).to_radians())
            * Mat4::from_rotation_y((invert * x_swing * -30.0).to_radians())
            * Mat4::from_rotation_y(180.0_f32.to_radians())
            * Mat4::from_rotation_z(180.0_f32.to_radians())
            * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_SCALE))
            * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
            * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE));
        let transformed = first_person_one_handed_map_transform(Mat4::IDENTITY, true, 0.0, attack);
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));
    }

    #[test]
    fn first_person_two_handed_map_transform_matches_vanilla_render_two_handed_map() {
        let attack = 0.25_f32;
        let x_rot = 20.0_f32;
        let sqrt_attack = attack.sqrt();
        let map_tilt = first_person_map_tilt(x_rot);
        let expected = Mat4::from_translation(Vec3::new(
            0.0,
            -(-0.2 * (attack * std::f32::consts::PI).sin()) / 2.0,
            -0.4 * (sqrt_attack * std::f32::consts::PI).sin(),
        )) * Mat4::from_translation(Vec3::new(0.0, 0.04 + map_tilt * -0.5, -0.72))
            * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
            * Mat4::from_rotation_x(
                ((sqrt_attack * std::f32::consts::PI).sin() * 20.0).to_radians(),
            )
            * Mat4::from_scale(Vec3::splat(2.0))
            * Mat4::from_rotation_y(180.0_f32.to_radians())
            * Mat4::from_rotation_z(180.0_f32.to_radians())
            * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_SCALE))
            * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
            * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE));
        let transformed = first_person_two_handed_map_transform(Mat4::IDENTITY, x_rot, 0.0, attack);
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));
    }

    #[test]
    fn first_person_whack_swing_transform_matches_vanilla_swing_arm() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let attack = 0.25_f32;
        let swung = first_person_apply_whack_swing(base, false, attack);
        let origin = swung.transform_point3(Vec3::ZERO);
        let sqrt_attack = attack.sqrt();
        let x_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
        let y_swing_position = 0.2 * (sqrt_attack * std::f32::consts::TAU).sin();
        let z_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
        let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
        let expected_origin = Vec3::new(
            0.56 + x_swing_position,
            -0.52 + y_swing_position,
            -0.72 + z_swing_position,
        );
        assert!(
            origin.abs_diff_eq(expected_origin, 1.0e-4),
            "origin {origin:?}, expected {expected_origin:?}"
        );

        let expected = base
            * Mat4::from_translation(expected_origin - Vec3::new(0.56, -0.52, -0.72))
            * Mat4::from_rotation_y(
                (45.0 + (attack * attack * std::f32::consts::PI).sin() * -20.0).to_radians(),
            )
            * Mat4::from_rotation_z((xz_swing_rotation * -20.0).to_radians())
            * Mat4::from_rotation_x((xz_swing_rotation * -80.0).to_radians())
            * Mat4::from_rotation_y((-45.0_f32).to_radians());
        assert!(swung.abs_diff_eq(expected, 1.0e-5));

        let runtime = NativeItemRuntime::empty_for_test();
        let mut none_stack = ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        none_stack.component_patch.swing_animation = Some(SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::None,
            duration: 6,
        });
        assert_eq!(
            first_person_stack_swing_animation(&none_stack, &runtime),
            FirstPersonSwingAnimation::None
        );

        none_stack.component_patch.removed_type_ids = vec![VANILLA_SWING_ANIMATION_COMPONENT_ID];
        assert_eq!(
            first_person_stack_swing_animation(&none_stack, &runtime),
            FirstPersonSwingAnimation::Whack,
            "removing swing_animation falls back to vanilla SwingAnimation.DEFAULT WHACK",
        );
    }

    #[test]
    fn first_person_stab_swing_transform_matches_vanilla_first_person_attack() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let attack = 0.1_f32;
        let swung = first_person_apply_stab_swing(base, false, attack);
        let starting_amount =
            -((std::f32::consts::PI * first_person_progress(attack, 0.0, 0.05)).cos() - 1.0) / 2.0;
        let middle_progress = first_person_progress(attack, 0.05, 0.2);
        let middle_amount = 1.0
            + 2.70158 * (middle_progress - 1.0).powi(3)
            + 1.70158 * (middle_progress - 1.0).powi(2);
        let ending_amount = 0.0;

        let expected = base
            * Mat4::from_translation(Vec3::new(
                0.1 * (starting_amount - middle_amount),
                -0.075 * (starting_amount - ending_amount),
                0.65 * (starting_amount - middle_amount),
            ))
            * Mat4::from_rotation_x((-70.0 * (starting_amount - ending_amount)).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.25 * (ending_amount - middle_amount)));
        assert!(swung.abs_diff_eq(expected, 1.0e-5));

        let left = first_person_apply_stab_swing(
            first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, true, 0.0),
            true,
            attack,
        );
        let right_origin = swung.transform_point3(Vec3::ZERO);
        let left_origin = left.transform_point3(Vec3::ZERO);
        assert!(
            (right_origin.x + left_origin.x).abs() < 1.0e-4,
            "STAB x translation mirrors between arms: {right_origin:?} vs {left_origin:?}"
        );
    }

    #[test]
    fn first_person_block_use_transform_matches_vanilla_non_shield_block_case() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let transformed = first_person_apply_block_use_transform(base, false);
        let expected = base
            * Mat4::from_translation(Vec3::new(-0.14142136, 0.08, 0.14142136))
            * Mat4::from_rotation_x((-102.25_f32).to_radians())
            * Mat4::from_rotation_y(13.365_f32.to_radians())
            * Mat4::from_rotation_z(78.05_f32.to_radians());
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));

        let left = first_person_apply_block_use_transform(
            first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, true, 0.0),
            true,
        );
        let right_origin = transformed.transform_point3(Vec3::ZERO);
        let left_origin = left.transform_point3(Vec3::ZERO);
        assert!(
            (right_origin.x + left_origin.x).abs() < 1.0e-4,
            "BLOCK use x translation mirrors between arms: {right_origin:?} vs {left_origin:?}"
        );
    }

    #[test]
    fn first_person_eat_drink_use_transform_matches_vanilla_apply_eat_transform() {
        let transformed =
            first_person_apply_eat_drink_use_transform(Mat4::IDENTITY, false, 32.0, 12.0, 0.5);
        let curr_usage_time = 32.0 - 12.0 - 0.5 + 1.0;
        let scaled_usage_time = curr_usage_time / 32.0;
        let extra_height_offset = (curr_usage_time / 4.0 * std::f32::consts::PI).cos().abs() * 0.1;
        let eat_jiggle = 1.0_f32 - scaled_usage_time.powf(27.0);
        let expected_eat = Mat4::from_translation(Vec3::new(0.0, extra_height_offset, 0.0))
            * Mat4::from_translation(Vec3::new(eat_jiggle * 0.6, eat_jiggle * -0.5, 0.0))
            * Mat4::from_rotation_y((eat_jiggle * 90.0).to_radians())
            * Mat4::from_rotation_x((eat_jiggle * 10.0).to_radians())
            * Mat4::from_rotation_z((eat_jiggle * 30.0).to_radians());
        let expected = first_person_item_arm_transform_from_camera_world(expected_eat, false, 0.0);
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));

        let left =
            first_person_apply_eat_drink_use_transform(Mat4::IDENTITY, true, 32.0, 12.0, 0.5);
        let right_origin = transformed.transform_point3(Vec3::ZERO);
        let left_origin = left.transform_point3(Vec3::ZERO);
        assert!(
            (right_origin.x + left_origin.x).abs() < 1.0e-4,
            "EAT/DRINK use x movement mirrors between arms: {right_origin:?} vs {left_origin:?}"
        );
    }

    #[test]
    fn first_person_brush_use_transform_matches_vanilla_apply_brush_transform() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let transformed = first_person_apply_brush_use_transform(base, false, 200.0, 12.0, 0.5);
        let remaining = (200.0_f32 - 12.0).max(0.0) % 10.0;
        let delta_since_last_update = remaining - 0.5 + 1.0;
        let scaled_usage_time = 1.0 - delta_since_last_update / 10.0;
        let current_swipe_angle =
            -15.0 + 75.0 * (scaled_usage_time * 2.0 * std::f32::consts::PI).cos();
        let right_brush = Mat4::from_translation(Vec3::new(-0.25, 0.22, 0.35))
            * Mat4::from_rotation_x((-80.0_f32).to_radians())
            * Mat4::from_rotation_y(90.0_f32.to_radians())
            * Mat4::from_rotation_x(current_swipe_angle.to_radians());
        assert!(transformed.abs_diff_eq(base * right_brush, 1.0e-5));

        let left_base =
            first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, true, 0.0);
        let left = first_person_apply_brush_use_transform(left_base, true, 200.0, 12.0, 0.5);
        let left_brush = Mat4::from_translation(Vec3::new(0.1, 0.83, 0.35))
            * Mat4::from_rotation_x((-80.0_f32).to_radians())
            * Mat4::from_rotation_y((-90.0_f32).to_radians())
            * Mat4::from_rotation_x(current_swipe_angle.to_radians())
            * Mat4::from_translation(Vec3::new(-0.3, 0.22, 0.35));
        assert!(left.abs_diff_eq(left_base * left_brush, 1.0e-5));
    }

    #[test]
    fn first_person_trident_use_transform_matches_vanilla_throw_charge_transform() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let transformed = first_person_apply_trident_use_transform(
            base,
            false,
            VANILLA_TRIDENT_USE_DURATION_TICKS,
            12.0,
            0.5,
        );
        let remaining_ticks = VANILLA_TRIDENT_USE_DURATION_TICKS - 12.0;
        let time_held = VANILLA_TRIDENT_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
        let power = (time_held / 10.0).min(1.0);
        let shake = ((time_held - 0.1) * 1.3).sin() * (power - 0.1);
        let expected = base
            * Mat4::from_translation(Vec3::new(-0.5, 0.7, 0.1))
            * Mat4::from_rotation_x((-55.0_f32).to_radians())
            * Mat4::from_rotation_y(35.3_f32.to_radians())
            * Mat4::from_rotation_z((-9.785_f32).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
            * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.2))
            * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
            * Mat4::from_rotation_y((-45.0_f32).to_radians());
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));
    }

    #[test]
    fn first_person_bow_use_transform_matches_vanilla_draw_transform() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let transformed = first_person_apply_bow_use_transform(
            base,
            false,
            VANILLA_BOW_USE_DURATION_TICKS,
            12.0,
            0.5,
        );
        let remaining_ticks = VANILLA_BOW_USE_DURATION_TICKS - 12.0;
        let time_held = VANILLA_BOW_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
        let mut power = time_held / 20.0;
        power = (power * power + power * 2.0) / 3.0;
        if power > 1.0 {
            power = 1.0;
        }
        let shake = ((time_held - 0.1) * 1.3).sin() * (power - 0.1);
        let expected = base
            * Mat4::from_translation(Vec3::new(-0.2785682, 0.18344387, 0.15731531))
            * Mat4::from_rotation_x((-13.935_f32).to_radians())
            * Mat4::from_rotation_y(35.3_f32.to_radians())
            * Mat4::from_rotation_z((-9.785_f32).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
            * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
            * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
            * Mat4::from_rotation_y((-45.0_f32).to_radians());
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));
    }

    #[test]
    fn first_person_crossbow_transforms_match_vanilla_draw_and_charged_hold() {
        let base = first_person_item_arm_transform_from_camera_world(Mat4::IDENTITY, false, 0.0);
        let transformed = first_person_apply_crossbow_use_transform(
            base,
            false,
            VANILLA_CROSSBOW_USE_DURATION_TICKS,
            VANILLA_CROSSBOW_CHARGE_DURATION_TICKS,
            12.0,
            0.5,
        );
        let remaining_ticks = VANILLA_CROSSBOW_USE_DURATION_TICKS - 12.0;
        let time_held = VANILLA_CROSSBOW_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
        let power = time_held / VANILLA_CROSSBOW_CHARGE_DURATION_TICKS;
        let shake = ((time_held - 0.1) * 1.3).sin() * (power - 0.1);
        let expected_draw = base
            * Mat4::from_translation(Vec3::new(-0.4785682, -0.094387, 0.05731531))
            * Mat4::from_rotation_x((-11.935_f32).to_radians())
            * Mat4::from_rotation_y(65.3_f32.to_radians())
            * Mat4::from_rotation_z((-9.785_f32).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
            * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
            * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
            * Mat4::from_rotation_y((-45.0_f32).to_radians());
        assert!(transformed.abs_diff_eq(expected_draw, 1.0e-5));

        let charged = first_person_apply_charged_crossbow_idle_transform(base, false);
        let expected_charged = base
            * Mat4::from_translation(Vec3::new(-0.641864, 0.0, 0.0))
            * Mat4::from_rotation_y(10.0_f32.to_radians());
        assert!(charged.abs_diff_eq(expected_charged, 1.0e-5));
    }

    #[test]
    fn first_person_spear_use_transform_matches_vanilla_kinetic_use() {
        let kinetic_weapon = SpearKineticWeapon {
            delay_ticks: 15.0,
            dismount_duration_ticks: 100.0,
            knockback_duration_ticks: 200.0,
            damage_duration_ticks: 300.0,
            forward_movement: 0.38,
        };
        let transformed = first_person_apply_spear_use_transform(
            Mat4::IDENTITY,
            false,
            kinetic_weapon,
            VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
            20.0,
            0.5,
            2.5,
        );
        let remaining_ticks = VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS - 20.0;
        let time_held = VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS - (remaining_ticks - 0.5 + 1.0);
        let params = kinetic_weapon.use_params(time_held);
        let x_rotation_degrees = -65.0 * first_person_ease_in_out_back(params.raise_progress)
            - 35.0 * params.lower_progress
            + 100.0 * params.raise_back_progress
            - 0.5 * params.sway_scale_fast;
        let y_negative_axis_rotation_degrees = -90.0
            * first_person_progress(params.raise_progress, 0.5, 0.55)
            + 90.0 * params.sway_progress
            + 2.0 * params.sway_scale_slow;
        let expected = Mat4::from_translation(Vec3::new(0.56, -0.52, -0.72))
            * Mat4::from_translation(Vec3::new(
                params.raise_progress * 0.15
                    + params.raise_progress_end * -0.05
                    + params.sway_progress * -0.1
                    + params.sway_scale_slow * 0.005,
                params.raise_progress * -0.075
                    + params.raise_progress_middle * 0.075
                    + params.sway_scale_fast * 0.01,
                params.raise_progress_start * 0.05
                    + params.raise_progress_end * -0.05
                    + params.sway_scale_slow * 0.005,
            ))
            * first_person_rotate_around(
                Vec3::new(0.0, 0.1, 0.0),
                Mat4::from_rotation_x(x_rotation_degrees.to_radians()),
            )
            * first_person_rotate_around(
                Vec3::new(0.15, 0.0, 0.0),
                Mat4::from_rotation_y((-y_negative_axis_rotation_degrees).to_radians()),
            )
            * Mat4::from_translation(Vec3::new(
                0.0,
                -first_person_spear_kinetic_hit_feedback_amount(2.5),
                0.0,
            ));
        assert!(transformed.abs_diff_eq(expected, 1.0e-5));
    }

    #[test]
    fn held_generated_item_main_hand_select_uses_owner_main_arm_context() {
        // Vanilla `MainHand.get` returns `owner.getMainArm()`. Keep the attach
        // transform and physical hand fixed here so the mesh difference proves
        // the selected generated-item texture branch changed.
        let root = unique_item_model_temp_dir("held-main-hand-select");
        write_main_hand_select_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let terrain_textures = TerrainTextureState::default();
        let stack = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let bake = |owner_main_hand_left| {
            let mut block_meshes = Vec::new();
            let mut block_translucent_meshes = Vec::new();
            let mut block_glint_meshes = Vec::new();
            let mut block_glint_translucent_meshes = Vec::new();
            let mut flat_meshes = Vec::new();
            let mut flat_translucent_meshes = Vec::new();
            let mut flat_glint_meshes = Vec::new();
            let mut flat_glint_translucent_meshes = Vec::new();
            bake_item_stack_at_transform(
                &stack,
                Mat4::IDENTITY,
                BlockModelDisplayContext::ThirdPersonRightHand,
                false,
                owner_main_hand_left,
                None,
                false,
                0.0,
                None,
                None,
                None,
                None,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
                &item_runtime,
                &terrain_textures,
                &mut block_meshes,
                &mut block_translucent_meshes,
                &mut block_glint_meshes,
                &mut block_glint_translucent_meshes,
                &mut flat_meshes,
                &mut flat_translucent_meshes,
                &mut flat_glint_meshes,
                &mut flat_glint_translucent_meshes,
            );
            assert!(block_meshes.is_empty());
            assert!(block_translucent_meshes.is_empty());
            assert!(block_glint_meshes.is_empty());
            assert!(block_glint_translucent_meshes.is_empty());
            assert!(flat_translucent_meshes.is_empty());
            assert!(flat_glint_meshes.is_empty());
            assert!(flat_glint_translucent_meshes.is_empty());
            assert_eq!(flat_meshes.len(), 1);
            flat_meshes.remove(0)
        };

        let fallback = bake(None);
        let right = bake(Some(false));
        let left = bake(Some(true));

        assert_ne!(fallback, right);
        assert_ne!(fallback, left);
        assert_ne!(right, left);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn held_generated_item_context_entity_type_select_uses_owner_kind() {
        // Vanilla `ContextEntityType.get` returns the living owner entity type
        // key. Keep the transform fixed so mesh changes prove the generated
        // item texture branch changed, not the hand pose.
        let root = unique_item_model_temp_dir("held-context-entity-type-select");
        write_context_entity_type_select_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let terrain_textures = TerrainTextureState::default();
        let stack = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let bake = |context_entity_type| {
            let mut block_meshes = Vec::new();
            let mut block_translucent_meshes = Vec::new();
            let mut block_glint_meshes = Vec::new();
            let mut block_glint_translucent_meshes = Vec::new();
            let mut flat_meshes = Vec::new();
            let mut flat_translucent_meshes = Vec::new();
            let mut flat_glint_meshes = Vec::new();
            let mut flat_glint_translucent_meshes = Vec::new();
            bake_item_stack_at_transform(
                &stack,
                Mat4::IDENTITY,
                BlockModelDisplayContext::ThirdPersonRightHand,
                false,
                Some(false),
                context_entity_type,
                false,
                0.0,
                None,
                None,
                None,
                None,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
                &item_runtime,
                &terrain_textures,
                &mut block_meshes,
                &mut block_translucent_meshes,
                &mut block_glint_meshes,
                &mut block_glint_translucent_meshes,
                &mut flat_meshes,
                &mut flat_translucent_meshes,
                &mut flat_glint_meshes,
                &mut flat_glint_translucent_meshes,
            );
            assert!(block_meshes.is_empty());
            assert!(block_translucent_meshes.is_empty());
            assert!(block_glint_meshes.is_empty());
            assert!(block_glint_translucent_meshes.is_empty());
            assert!(flat_translucent_meshes.is_empty());
            assert!(flat_glint_meshes.is_empty());
            assert!(flat_glint_translucent_meshes.is_empty());
            assert_eq!(flat_meshes.len(), 1);
            flat_meshes.remove(0)
        };

        let player_kind = EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Player,
            baby: false,
        };
        assert_eq!(
            entity_model_context_entity_type(player_kind),
            Some("minecraft:player")
        );
        assert_eq!(
            entity_model_context_entity_type(EntityModelKind::Witch),
            Some("minecraft:witch")
        );

        let fallback = bake(None);
        let player = bake(entity_model_context_entity_type(player_kind));
        let witch = bake(entity_model_context_entity_type(EntityModelKind::Witch));

        assert_ne!(fallback, player);
        assert_ne!(fallback, witch);
        assert_ne!(player, witch);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn held_generated_item_context_dimension_select_uses_world_level() {
        // Vanilla `ContextDimension.get` returns `level.dimension()` when the
        // item resolver is called with a `ClientLevel`. The held-item path uses
        // the real world level, so the same stack can select different
        // generated textures in different dimensions.
        let root = unique_item_model_temp_dir("held-context-dimension-select");
        write_context_dimension_select_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let terrain_textures = TerrainTextureState::default();
        const ENTITY_ID: i32 = 610;
        const PLAYER_ENTITY_TYPE_ID: i32 = 155;
        let bake = |dimension: Option<&str>| {
            let mut world = dimension.map_or_else(WorldStore::new, world_with_level_dimension);
            world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
            assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
            let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
            let models =
                held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
            assert_eq!(models.flat_meshes.len(), 1);
            models.flat_meshes[0].clone()
        };

        let fallback = bake(None);
        let overworld = bake(Some("minecraft:overworld"));
        let nether = bake(Some("minecraft:the_nether"));

        assert_ne!(fallback, overworld);
        assert_ne!(fallback, nether);
        assert_ne!(overworld, nether);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn held_generated_item_trim_material_select_uses_world_registry() {
        // Vanilla `TrimMaterialProperty.get` reads the stack's TRIM component
        // and unwraps the synced trim-material registry key. Held generated
        // items therefore need the same world registry context as GUI/dropped
        // item consumers.
        let root = unique_item_model_temp_dir("held-trim-material-select");
        write_trim_material_select_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let terrain_textures = TerrainTextureState::default();
        const ENTITY_ID: i32 = 611;
        const PLAYER_ENTITY_TYPE_ID: i32 = 155;
        let bake = |with_registry: bool, material_id: Option<i32>| {
            let mut world = WorldStore::new();
            if with_registry {
                record_trim_material_registry(&mut world);
            }
            world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
            assert!(world.apply_set_equipment(SetEquipment {
                entity_id: ENTITY_ID,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::MainHand,
                    item: ItemStackSummary {
                        item_id: Some(0),
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            armor_trim_material_id: material_id,
                            ..DataComponentPatchSummary::default()
                        },
                    },
                }],
            }));
            let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
            let models =
                held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
            assert_eq!(models.flat_meshes.len(), 1);
            models.flat_meshes[0].clone()
        };

        let fallback = bake(false, Some(0));
        let quartz = bake(true, Some(0));
        let diamond = bake(true, Some(2));

        assert_ne!(fallback, quartz);
        assert_ne!(fallback, diamond);
        assert_ne!(quartz, diamond);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn humanoid_held_generated_item_using_item_condition_matches_used_hand() {
        // Vanilla `IsUsingItem.get` is true only when the submitted stack is the
        // owner's active `getUseItem()` stack, not merely because the owner is
        // using some item in the other hand.
        let root = unique_item_model_temp_dir("held-using-item-condition");
        write_using_item_condition_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 607;
        const PLAYER_ENTITY_TYPE_ID: i32 = 155;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        let terrain_textures = TerrainTextureState::default();
        let base = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);

        let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
        let main_using_models = held_item_models(
            &[base.with_is_using_item(true).with_use_item_off_hand(false)],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );
        let off_using_models = held_item_models(
            &[base.with_is_using_item(true).with_use_item_off_hand(true)],
            &world,
            Some(&item_runtime),
            &terrain_textures,
        );

        assert_eq!(idle_models.flat_meshes.len(), 1);
        assert_eq!(main_using_models.flat_meshes.len(), 1);
        assert_eq!(off_using_models.flat_meshes.len(), 1);
        assert_ne!(idle_models.flat_meshes[0], main_using_models.flat_meshes[0]);
        assert_eq!(idle_models.flat_meshes[0], off_using_models.flat_meshes[0]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn humanoid_held_generated_item_use_duration_reads_owner_use_ticks() {
        // Vanilla `ItemModelResolver.updateForLiving` passes the owning living
        // entity into the item model; `UseDuration.get` then reads the active
        // `getUseItem()` stack's elapsed use ticks.
        let root = unique_item_model_temp_dir("held-use-duration-range-dispatch");
        write_bow_use_duration_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        const ENTITY_ID: i32 = 608;
        const PLAYER_ENTITY_TYPE_ID: i32 = 155;
        world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        let terrain_textures = TerrainTextureState::default();
        let base = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);

        let bake = |instance: EntityModelInstance| {
            let models =
                held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
            assert_eq!(models.flat_meshes.len(), 1);
            models.flat_meshes[0].clone()
        };

        let idle = bake(base);
        let using_start = bake(
            base.with_is_using_item(true)
                .with_use_item_off_hand(false)
                .with_crossbow_charge_ticks(0.0),
        );
        let using_pulling = bake(
            base.with_is_using_item(true)
                .with_use_item_off_hand(false)
                .with_crossbow_charge_ticks(13.5),
        );
        let offhand_using = bake(
            base.with_is_using_item(true)
                .with_use_item_off_hand(true)
                .with_crossbow_charge_ticks(13.5),
        );

        assert_ne!(idle, using_start);
        assert_ne!(using_start, using_pulling);
        assert_eq!(idle, offhand_using);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn humanoid_held_generated_crossbow_pull_applies_quick_charge_registry() {
        // Vanilla `CrossbowPull.get` divides elapsed use ticks by
        // `CrossbowItem.getChargeDuration`, which applies Quick Charge through
        // the synced enchantment registry before choosing the item-model
        // texture.
        let root = unique_item_model_temp_dir("held-crossbow-quick-charge-range-dispatch");
        write_crossbow_pull_item_runtime_fixture(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        const ENTITY_ID: i32 = 609;
        const PLAYER_ENTITY_TYPE_ID: i32 = 155;
        let quick_charge_crossbow = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 2,
                }],
                ..DataComponentPatchSummary::default()
            },
        };
        let mut default_world = WorldStore::new();
        default_world.apply_add_entity(protocol_add_entity(ENTITY_ID, PLAYER_ENTITY_TYPE_ID));
        assert!(default_world.apply_set_equipment(SetEquipment {
            entity_id: ENTITY_ID,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: quick_charge_crossbow,
            }],
        }));
        let mut quick_charge_world = default_world.clone();
        record_enchantment_registry(&mut quick_charge_world);
        let terrain_textures = TerrainTextureState::default();
        let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false)
            .with_is_using_item(true)
            .with_use_item_off_hand(false)
            .with_crossbow_charge_ticks(10.0);

        let default_models = held_item_models(
            &[instance],
            &default_world,
            Some(&item_runtime),
            &terrain_textures,
        );
        let quick_charge_models = held_item_models(
            &[instance],
            &quick_charge_world,
            Some(&item_runtime),
            &terrain_textures,
        );

        assert_eq!(default_models.flat_meshes.len(), 1);
        assert_eq!(quick_charge_models.flat_meshes.len(), 1);
        assert!(!default_models.flat_meshes[0].is_empty());
        assert!(!quick_charge_models.flat_meshes[0].is_empty());
        assert_ne!(
            default_models.flat_meshes[0],
            quick_charge_models.flat_meshes[0]
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
            ItemModelFoil::None,
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
            ItemModelFoil::None,
        );
        assert!(!single.is_empty());
        assert!(!cluster.is_empty());
    }

    #[test]
    fn foiled_cluster_mirrors_geometry_to_item_glint_buckets() {
        let mut quads = unit_block_quads();
        let mut translucent = quads[0].clone();
        translucent.translucent = true;
        translucent.tint = [0.7, 0.8, 1.0, 0.5];
        quads.push(translucent);
        let plain = stacked_item_mesh(
            &quads,
            [0.0, 0.0, 0.0],
            0.0,
            0,
            &BLOCK_GROUND_FALLBACK,
            [1.0, 1.0],
            1,
            7,
            ItemModelFoil::None,
        );
        let foiled = stacked_item_mesh(
            &quads,
            [0.0, 0.0, 0.0],
            0.0,
            0,
            &BLOCK_GROUND_FALLBACK,
            [1.0, 1.0],
            1,
            7,
            ItemModelFoil::Standard,
        );

        assert!(plain.glint.is_empty());
        assert!(plain.glint_translucent.is_empty());
        assert_eq!(foiled.glint, foiled.solid);
        assert_eq!(foiled.glint_translucent, foiled.translucent);
        assert!(!foiled.translucent.is_empty());
    }

    #[test]
    fn ominous_item_spawner_base_transform_scales_in_and_spins() {
        let early = ominous_item_spawner_base_transform([1.0, 2.0, 3.0], 9.0);
        assert!((early.transform_point3(Vec3::ZERO) - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
        assert!((early.transform_vector3(Vec3::X) - Vec3::new(0.18, 0.0, 0.0)).length() < 1e-6);

        let full_size = ominous_item_spawner_base_transform([0.0, 0.0, 0.0], 60.0);
        assert!((full_size.transform_vector3(Vec3::X).length() - 1.0).abs() < 1e-6);
        assert!(
            (full_size.transform_vector3(Vec3::X) - Vec3::X).length() > 1.0,
            "60 ticks spins by 240 degrees, not identity"
        );
    }

    #[test]
    fn ominous_item_spawner_models_emit_item_cluster_meshes() {
        const OMINOUS_ITEM_SPAWNER_DATA_ITEM_ID: u8 = 8;

        let root = unique_item_model_temp_dir("ominous-item-spawner");
        write_flat_item_runtime_fixture(&root, &["omen_drop"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let item_id = item_runtime
            .item_protocol_id("minecraft:omen_drop")
            .expect("fixture item registered");
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            801,
            VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 801,
            values: vec![EntityDataValue {
                data_id: OMINOUS_ITEM_SPAWNER_DATA_ITEM_ID,
                serializer_id: 7,
                value: EntityDataValueKind::ItemStack(ItemStackSummary {
                    item_id: Some(item_id),
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }),
            }],
        }));
        world.advance_entity_client_animations(25);

        let models = ominous_item_spawner_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            0.5,
            None,
            None,
            None,
        );

        assert!(models.block_meshes.is_empty());
        assert_eq!(models.flat_meshes.len(), 1);
        assert!(!models.flat_meshes[0].is_empty());
        assert!(models.handled_entity_ids.contains(&801));
        std::fs::remove_dir_all(root).unwrap();
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
    fn item_stack_foil_mode_projects_vanilla_special_context_scale() {
        let root = unique_item_model_temp_dir("special-foil-mode");
        write_flat_item_runtime_fixture(&root, &["clock", "spyglass"]);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let foiled_stack = |resource_id: &str| ItemStackSummary {
            item_id: item_runtime.item_protocol_id(resource_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                enchantment_glint_override: Some(true),
                ..DataComponentPatchSummary::default()
            },
        };
        let plain_clock = ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:clock"),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };

        assert_eq!(
            item_stack_foil_mode(
                &foiled_stack("minecraft:clock"),
                &item_runtime,
                BlockModelDisplayContext::Ground
            ),
            ItemModelFoil::Special {
                decal_pose_scale: 1.0
            }
        );
        assert_eq!(
            item_stack_foil_mode(
                &foiled_stack("minecraft:clock"),
                &item_runtime,
                BlockModelDisplayContext::Gui
            ),
            ItemModelFoil::Special {
                decal_pose_scale: 0.5
            }
        );
        assert_eq!(
            item_stack_foil_mode(
                &foiled_stack("minecraft:clock"),
                &item_runtime,
                BlockModelDisplayContext::FirstPersonRightHand
            ),
            ItemModelFoil::Special {
                decal_pose_scale: 0.75
            }
        );
        assert_eq!(
            item_stack_foil_mode(
                &foiled_stack("minecraft:spyglass"),
                &item_runtime,
                BlockModelDisplayContext::Ground
            ),
            ItemModelFoil::Standard
        );
        assert_eq!(
            item_stack_foil_mode(
                &plain_clock,
                &item_runtime,
                BlockModelDisplayContext::Ground
            ),
            ItemModelFoil::None
        );

        std::fs::remove_dir_all(root).unwrap();
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

    fn record_enchantment_registry(world: &mut WorldStore) {
        world.record_registry_data(bbb_protocol::packets::RegistryData {
            registry: "minecraft:enchantment".to_string(),
            entries: vec![
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:power".to_string(),
                    raw_data: None,
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:quick_charge".to_string(),
                    raw_data: None,
                },
            ],
            raw_payload_len: 0,
        });
    }

    fn record_trim_material_registry(world: &mut WorldStore) {
        world.record_registry_data(bbb_protocol::packets::RegistryData {
            registry: "minecraft:trim_material".to_string(),
            entries: vec![
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:quartz".to_string(),
                    raw_data: None,
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:iron".to_string(),
                    raw_data: None,
                },
                bbb_protocol::packets::RegistryDataEntry {
                    id: "minecraft:diamond".to_string(),
                    raw_data: None,
                },
            ],
            raw_payload_len: 0,
        });
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

    fn write_default_consumable_item_runtime_fixture(root: &Path, item_id: &str) {
        write_flat_item_runtime_fixture(root, &[item_id]);
        let constant = item_id.to_ascii_uppercase();
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
                public static final Item {constant} = registerItem("{item_id}", new Item.Properties().food(Foods.{constant}));
            }}"#,
            ),
        );
    }

    fn write_main_hand_select_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["hand_selector"]);
        write_json(
            &assets.join("items").join("hand_selector.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:main_hand",
                    "cases": [
                        {
                            "when": "left",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_left" }
                        },
                        {
                            "when": "right",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_right" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/hand_selector" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "hand_selector", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "hand_selector_left", &[120, 40, 80, 255]);
        write_flat_item_model_and_texture(&assets, "hand_selector_right", &[80, 120, 40, 255]);
    }

    fn write_context_entity_type_select_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["entity_selector"]);
        write_json(
            &assets.join("items").join("entity_selector.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:context_entity_type",
                    "cases": [
                        {
                            "when": "minecraft:player",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_player" }
                        },
                        {
                            "when": "minecraft:witch",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/entity_selector_witch" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/entity_selector" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "entity_selector", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "entity_selector_player", &[120, 40, 80, 255]);
        write_flat_item_model_and_texture(&assets, "entity_selector_witch", &[80, 120, 40, 255]);
    }

    fn write_context_dimension_select_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["dimension_selector"]);
        write_json(
            &assets.join("items").join("dimension_selector.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:context_dimension",
                    "cases": [
                        {
                            "when": "minecraft:overworld",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_overworld" }
                        },
                        {
                            "when": "minecraft:the_nether",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector_nether" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/dimension_selector" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "dimension_selector", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(
            &assets,
            "dimension_selector_overworld",
            &[120, 40, 80, 255],
        );
        write_flat_item_model_and_texture(
            &assets,
            "dimension_selector_nether",
            &[80, 120, 40, 255],
        );
    }

    fn write_trim_material_select_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["iron_chestplate"]);
        write_json(
            &assets.join("items").join("iron_chestplate.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:trim_material",
                    "cases": [
                        {
                            "when": "minecraft:quartz",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_quartz_trim" }
                        },
                        {
                            "when": "minecraft:diamond",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_diamond_trim" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "iron_chestplate", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(
            &assets,
            "iron_chestplate_quartz_trim",
            &[200, 200, 190, 255],
        );
        write_flat_item_model_and_texture(
            &assets,
            "iron_chestplate_diamond_trim",
            &[120, 200, 210, 255],
        );
    }

    fn write_using_item_condition_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["use_selector"]);
        write_json(
            &assets.join("items").join("use_selector.json"),
            r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/use_selector" },
                    "on_true": { "type": "minecraft:model", "model": "minecraft:item/use_selector_using" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "use_selector", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "use_selector_using", &[120, 80, 40, 255]);
    }

    fn write_bow_use_duration_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["bow"]);
        write_json(
            &assets.join("items").join("bow.json"),
            r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/bow" },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:use_duration",
                        "scale": 0.05,
                        "entries": [
                            {
                                "threshold": 0.65,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_1" }
                            },
                            {
                                "threshold": 0.9,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_2" }
                            }
                        ],
                        "fallback": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_0" }
                    }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "bow", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "bow_pulling_0", &[120, 80, 40, 255]);
        write_flat_item_model_and_texture(&assets, "bow_pulling_1", &[80, 120, 40, 255]);
        write_flat_item_model_and_texture(&assets, "bow_pulling_2", &[120, 40, 80, 255]);
    }

    fn write_crossbow_pull_item_runtime_fixture(root: &Path) {
        let assets = item_model_assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["crossbow"]);
        write_json(
            &assets.join("items").join("crossbow.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:charge_type",
                    "cases": [],
                    "fallback": {
                        "type": "minecraft:condition",
                        "property": "minecraft:using_item",
                        "on_false": { "type": "minecraft:model", "model": "minecraft:item/crossbow" },
                        "on_true": {
                            "type": "minecraft:range_dispatch",
                            "property": "minecraft:crossbow/pull",
                            "entries": [
                                {
                                    "threshold": 0.58,
                                    "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_1" }
                                },
                                {
                                    "threshold": 1.0,
                                    "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_2" }
                                }
                            ],
                            "fallback": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_0" }
                        }
                    }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_pulling_0", &[70, 100, 130, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_pulling_1", &[100, 130, 70, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_pulling_2", &[130, 70, 100, 255]);
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
