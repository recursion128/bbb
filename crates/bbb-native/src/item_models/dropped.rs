use super::*;

/// Vanilla `FLAT_ITEM_DEPTH_THRESHOLD` / `ITEM_MIN_HOVER_HEIGHT`: a rendered model thinner than this in Z
/// is stacked back-to-front; a thicker one is scattered in 3D.
pub(super) const FLAT_ITEM_DEPTH_THRESHOLD: f32 = 0.0625;
const OMINOUS_ITEM_SPAWNER_SCALE_IN_TICKS: f32 = 50.0;
const OMINOUS_ITEM_SPAWNER_ROTATION_SPEED_DEGREES_PER_TICK: f32 = 40.0;
pub(super) const CARVED_PUMPKIN_BLOCK_ID: &str = "minecraft:carved_pumpkin";
pub(super) const POPPY_BLOCK_ID: &str = "minecraft:poppy";
pub(super) const RED_MUSHROOM_BLOCK_ID: &str = "minecraft:red_mushroom";
pub(super) const BROWN_MUSHROOM_BLOCK_ID: &str = "minecraft:brown_mushroom";

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

pub(super) struct EntityBlockAttachment {
    pub(super) block_id: Cow<'static, str>,
    pub(super) properties: BTreeMap<String, String>,
    pub(super) transform: Mat4,
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) outline_only: bool,
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

pub(super) fn falling_block_should_render(
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

pub(super) fn entity_block_attachments(
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

pub(super) fn copper_golem_antenna_block_attachment_from_stack(
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
pub(super) fn carved_pumpkin_default_properties() -> BTreeMap<String, String> {
    BTreeMap::from([("facing".to_string(), "north".to_string())])
}

/// Vanilla `Blocks.POPPY.defaultBlockState()` has no properties; the iron-golem flower layer uses that
/// exact blockstate while `offerFlowerTick > 0`.
pub(super) fn poppy_default_properties() -> BTreeMap<String, String> {
    BTreeMap::new()
}

pub(super) fn mooshroom_mushroom_block_id(variant: MooshroomVariant) -> &'static str {
    match variant {
        MooshroomVariant::Red => RED_MUSHROOM_BLOCK_ID,
        MooshroomVariant::Brown => BROWN_MUSHROOM_BLOCK_ID,
    }
}

/// Vanilla `MushroomCow.Variant` stores `Blocks.RED_MUSHROOM.defaultBlockState()` or
/// `Blocks.BROWN_MUSHROOM.defaultBlockState()`, both property-less states.
pub(super) fn mooshroom_mushroom_default_properties() -> BTreeMap<String, String> {
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

/// Bakes ordinary item-stack render states carried by vanilla
/// `ItemPickupParticleGroup`. The particle runtime owns the target interpolation;
/// this helper reuses the same item-cluster bake as dropped item entities at the
/// extracted particle position.
pub(crate) fn item_pickup_particle_item_models(
    states: &[ItemPickupParticleRenderState],
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> DroppedItemModels {
    let mut models = DroppedItemModels::empty();
    let Some(item_runtime) = item_runtime else {
        return models;
    };

    for state in states {
        if state.item.component_patch_len != 0 || state.item.item_id < 0 || state.item.count <= 0 {
            continue;
        }
        let stack = ItemStackSummary {
            item_id: Some(state.item.item_id),
            count: state.item.count,
            component_patch: Default::default(),
        };
        let count = rendered_amount(stack.count);
        let seed = state.item.item_id as i64;
        let ground = |default_transform| {
            item_runtime
                .item_display_transform_for_stack(&stack, BlockModelDisplayContext::Ground)
                .unwrap_or(default_transform)
        };
        let foil = item_stack_foil_mode(&stack, item_runtime, BlockModelDisplayContext::Ground);

        if let Some(resource_id) = item_runtime.item_resource_id(state.item.item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    push_mesh_set(
                        stacked_item_mesh(
                            &quads,
                            state.position,
                            state.age_ticks,
                            state.source_entity_id,
                            &ground(BLOCK_GROUND_FALLBACK),
                            state.light,
                            count,
                            seed,
                            foil,
                        ),
                        &mut models.block_meshes,
                        &mut models.block_translucent_meshes,
                        &mut models.block_glint_meshes,
                        &mut models.block_glint_translucent_meshes,
                    );
                    continue;
                }
            }
        }

        let mut quads: Vec<ItemModelQuad> = Vec::new();
        for layer in item_runtime.generated_item_layers_for_stack_with_registry_context(
            &stack,
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
                state.position,
                state.age_ticks,
                state.source_entity_id,
                &ground(GENERATED_GROUND_FALLBACK),
                state.light,
                count,
                seed,
                foil,
            ),
            &mut models.flat_meshes,
            &mut models.flat_translucent_meshes,
            &mut models.flat_glint_meshes,
            &mut models.flat_glint_translucent_meshes,
        );
    }

    models
}

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

pub(super) fn world_enchantment_keys(world: &WorldStore) -> Option<Vec<String>> {
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

pub(super) fn world_trim_material_keys(world: &WorldStore) -> Option<Vec<String>> {
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

pub(super) fn world_attribute_keys(world: &WorldStore) -> Option<Vec<String>> {
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

pub(super) fn is_custom_head_skull_item_id(resource_id: &str) -> bool {
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

pub(super) fn entity_model_context_entity_type(kind: EntityModelKind) -> Option<&'static str> {
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
pub(super) fn bake_item_stack_at_transform(
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
pub(super) fn rendered_amount(stack_count: i32) -> usize {
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
pub(super) fn stacked_item_mesh(
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
pub(super) fn base_transform(
    position: [f32; 3],
    age_ticks: f32,
    entity_id: i32,
    min_offset_y: f32,
) -> Mat4 {
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

pub(super) fn ominous_item_spawner_base_transform(position: [f32; 3], age_ticks: f32) -> Mat4 {
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
pub(super) fn ground_model_bounds(quads: &[ItemModelQuad], ground_matrix: Mat4) -> (f32, f32) {
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
pub(super) fn bob_offset(entity_id: i32) -> f32 {
    let mixed = (entity_id as u32).wrapping_mul(2_654_435_761);
    (mixed as f32 / u32::MAX as f32) * std::f32::consts::TAU
}

const RNG_MULTIPLIER: u64 = 25_214_903_917;
const RNG_INCREMENT: u64 = 11;
const RNG_MASK: u64 = (1_u64 << 48) - 1;

/// The Java `Random` / vanilla `LegacyRandomSource` LCG, enough to reproduce `nextFloat()` for the
/// cluster jitter so it matches vanilla.
pub(super) struct LegacyRandom {
    seed: u64,
}

impl LegacyRandom {
    pub(super) fn new(seed: i64) -> Self {
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

    pub(super) fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
    }
}
