use super::*;
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;

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
    pub residency: ItemModelResidencyFrame,
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
            residency: ItemModelResidencyFrame::default(),
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
    pub(super) moving_block: Option<MovingBlockRenderContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MovingBlockRenderContext {
    pub(super) random_seed_pos: BlockPos,
    pub(super) block_pos: BlockPos,
    pub(super) biome_id: Option<i32>,
    pub(super) cardinal_lighting: TerrainCardinalLighting,
}

fn entity_render_state_shader_light(instance: &EntityModelInstance) -> [f32; 2] {
    let block = (instance.render_state.light_coords >> 4) & 0xF;
    let sky = (instance.render_state.light_coords >> 20) & 0xF;
    [block as f32 / 15.0, sky as f32 / 15.0]
}

fn terrain_light_shader(light: TerrainLight) -> [f32; 2] {
    [
        f32::from(light.block.min(15)) / 15.0,
        f32::from(light.sky.min(15)) / 15.0,
    ]
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
    match world.probe_block(falling_block_block_pos(instance, world)) {
        Some(block) => block.block_state_id != block_state_id,
        None => true,
    }
}

fn falling_block_block_pos(instance: &EntityModelInstance, world: &WorldStore) -> BlockPos {
    let position = world
        .probe_entity_transform(instance.entity_id)
        .map(|state| state.position)
        .unwrap_or_else(|| bbb_world::EntityVec3 {
            x: f64::from(instance.position[0]),
            y: f64::from(instance.position[1]),
            z: f64::from(instance.position[2]),
        });
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
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
) -> ItemModelResidencyFrame {
    let attachments = entity_block_attachments(
        instances,
        world,
        item_runtime,
        entity_partial_tick.clamp(0.0, 1.0),
    );
    if attachments.is_empty() {
        return ItemModelResidencyFrame::default();
    }

    let mut residency = ItemModelResidencyFrame::default();
    for attachment in attachments {
        if attachment.outline_only {
            continue;
        }
        let base_variant = block_item_variant_identity(&attachment.properties);
        let moving_block = attachment.moving_block.is_some();
        let (quads, variant) = if let Some(context) = attachment.moving_block {
            let Some(quads) = terrain_textures.moving_block_quads(
                attachment.block_id.as_ref(),
                &attachment.properties,
                block_render_position(context.random_seed_pos),
                block_render_position(context.block_pos),
                context.biome_id,
                context.cardinal_lighting,
            ) else {
                continue;
            };
            (
                quads,
                moving_block_variant_identity(base_variant.as_ref(), context),
            )
        } else {
            let Some(quads) = terrain_textures
                .block_item_quads(attachment.block_id.as_ref(), &attachment.properties)
            else {
                continue;
            };
            (quads, base_variant)
        };
        if !quads.is_empty() {
            let identity = ItemModelAssetIdentity::new(
                Arc::<str>::from(attachment.block_id.as_ref()),
                variant,
                0,
            );
            let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
            if quads.iter().any(|quad| !quad.translucent) {
                let first_instance = residency.instances.len();
                push_resident_item_instances(
                    &mut residency,
                    identity.clone(),
                    ItemModelAtlasKind::Blocks,
                    ItemModelGeometryClass::Solid,
                    source.clone(),
                    &[attachment.transform],
                    [1.0; 4],
                    attachment.light,
                    attachment.overlay,
                    ItemModelFoil::None,
                    ItemModelDrawTarget::World,
                );
                let phase = block_model_solid_phase(
                    moving_block,
                    attachment.block_id.as_ref(),
                    terrain_textures,
                );
                if phase != ItemModelDrawPhase::Solid {
                    for instance in &mut residency.instances[first_instance..] {
                        if instance.phase == ItemModelDrawPhase::Solid {
                            instance.phase = phase;
                        }
                    }
                }
            }
            if quads.iter().any(|quad| quad.translucent) {
                push_resident_item_instances(
                    &mut residency,
                    identity,
                    ItemModelAtlasKind::Blocks,
                    ItemModelGeometryClass::Translucent,
                    source,
                    &[attachment.transform],
                    [1.0; 4],
                    attachment.light,
                    attachment.overlay,
                    ItemModelFoil::None,
                    ItemModelDrawTarget::World,
                );
            }
        }
    }
    residency
}

pub(super) fn block_model_solid_phase(
    moving_block: bool,
    block_id: &str,
    terrain_textures: &TerrainTextureState,
) -> ItemModelDrawPhase {
    if moving_block && terrain_textures.forces_opaque_leaves(block_id) {
        ItemModelDrawPhase::SolidNoCutout
    } else {
        ItemModelDrawPhase::Solid
    }
}

fn block_render_position(pos: BlockPos) -> BlockRenderPosition {
    BlockRenderPosition {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn moving_block_variant_identity(base: &str, context: MovingBlockRenderContext) -> Arc<str> {
    let cardinal = match context.cardinal_lighting {
        TerrainCardinalLighting::Default => "default",
        TerrainCardinalLighting::Nether => "nether",
    };
    Arc::from(format!(
        "{base}|moving_seed={},{},{}|block_pos={},{},{}|biome={:?}|cardinal={cardinal}",
        context.random_seed_pos.x,
        context.random_seed_pos.y,
        context.random_seed_pos.z,
        context.block_pos.x,
        context.block_pos.y,
        context.block_pos.z,
        context.biome_id,
    ))
}

pub(super) fn block_item_variant_identity(properties: &BTreeMap<String, String>) -> Arc<str> {
    if properties.is_empty() {
        return Arc::from("default");
    }
    let mut identity = String::new();
    for (index, (name, value)) in properties.iter().enumerate() {
        if index != 0 {
            identity.push(',');
        }
        identity.push_str(name);
        identity.push('=');
        identity.push_str(value);
    }
    Arc::from(identity)
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
                moving_block: None,
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
                moving_block: None,
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
                    moving_block: None,
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
                    moving_block: None,
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
                    moving_block: None,
                });
            }
        }
        if let Some(falling) = world.falling_block_state(instance.entity_id) {
            if falling_block_should_render(instance, world, falling.block_state_id) {
                if let Some(transform) = falling_block_transform(instance) {
                    let moving_block = moving_block_render_context(
                        world,
                        falling.random_seed_pos,
                        falling.block_pos,
                    );
                    let light = world
                        .sample_block_light(falling.block_pos)
                        .map(terrain_light_shader)
                        .unwrap_or_else(|| entity_render_state_shader_light(instance));
                    attachments.push(EntityBlockAttachment {
                        block_id: Cow::Owned(falling.block.name),
                        properties: falling.block.properties,
                        transform,
                        light,
                        overlay: ITEM_MODEL_NO_OVERLAY,
                        outline_only: false,
                        moving_block: Some(moving_block),
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
                    moving_block: None,
                });
            }
        }
        if let Some(attachment) =
            copper_golem_antenna_block_attachment(instance, world, item_runtime)
        {
            attachments.push(attachment);
        }
    }
    // Vanilla `PistonHeadRenderer` submits the moving block entity's model at
    // its interpolated translation using the movement-origin position for
    // deterministic model offsets. Reuse the shared moving-block terrain bake
    // that already serves falling blocks, but keep piston state owned by world.
    for moving in world.moving_piston_render_blocks(entity_partial_tick) {
        let Some(block_state) = world.registries().block_state(moving.block_state_id) else {
            continue;
        };
        let moving_block =
            moving_block_render_context(world, moving.random_seed_pos, moving.block_pos);
        let light = world
            .sample_block_light(moving.block_pos)
            .map(terrain_light_shader)
            .unwrap_or([1.0, 1.0]);
        attachments.push(EntityBlockAttachment {
            block_id: Cow::Owned(block_state.name.clone()),
            properties: block_state.properties.clone(),
            transform: Mat4::from_translation(Vec3::from_array(moving.translation)),
            light,
            overlay: ITEM_MODEL_NO_OVERLAY,
            outline_only: false,
            moving_block: Some(moving_block),
        });
    }
    attachments
}

fn moving_block_render_context(
    world: &WorldStore,
    random_seed_pos: BlockPos,
    block_pos: BlockPos,
) -> MovingBlockRenderContext {
    let chunk_pos = ChunkPos {
        x: block_pos.x.div_euclid(16),
        z: block_pos.z.div_euclid(16),
    };
    let biome_id =
        world
            .chunk_biome_sampler(chunk_pos)
            .biome_id_at(block_pos.x, block_pos.y, block_pos.z);
    MovingBlockRenderContext {
        random_seed_pos,
        block_pos,
        biome_id,
        cardinal_lighting: terrain_cardinal_lighting(world),
    }
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
        moving_block: None,
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
    renderer: Option<&bbb_renderer::Renderer>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    age_ticks: f32,
    trim_material_keys: Option<&[String]>,
    trim_pattern_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> DroppedItemModels {
    let block_meshes = Vec::new();
    let block_translucent_meshes = Vec::new();
    let block_glint_meshes = Vec::new();
    let block_glint_translucent_meshes = Vec::new();
    let flat_meshes = Vec::new();
    let flat_translucent_meshes = Vec::new();
    let flat_glint_meshes = Vec::new();
    let flat_glint_translucent_meshes = Vec::new();
    let mut residency = ItemModelResidencyFrame::default();
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
            residency,
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

        let resolved = item_runtime.resolved_item_model_for_stack_with_registry_context(
            &state.stack,
            BlockModelDisplayContext::Ground,
            trim_material_keys,
            trim_pattern_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
        );
        if resolved.as_ref().is_some_and(|resolved| {
            resolved
                .layers
                .iter()
                .any(|layer| matches!(layer, ResolvedItemModelLayer::Special { .. }))
        }) {
            let ground_fallback = ground(GENERATED_GROUND_FALLBACK);
            let mut projected = Vec::new();
            for layer in resolved.into_iter().flat_map(|resolved| resolved.layers) {
                match layer {
                    ResolvedItemModelLayer::Ordinary {
                        icon,
                        transformation,
                        display_transform,
                    } => projected.push(ProjectedSpecialClusterLayer::Ordinary {
                        icon,
                        transformation,
                        display_transform,
                    }),
                    ResolvedItemModelLayer::Special {
                        model,
                        transformation,
                        display_transform,
                    } => {
                        if let Some(model) = special::project_special_item_model(
                            &model,
                            &state.stack,
                            item_runtime,
                            Some(world),
                            variant_textures,
                        ) {
                            projected.push(ProjectedSpecialClusterLayer::Special {
                                model,
                                transformation,
                                display_transform,
                            });
                        }
                    }
                }
            }
            let (min_y, depth) = special_cluster_bounds(renderer, &projected, ground_fallback);
            let base = base_transform(position, age_ticks, state.bob_offset, -min_y + 1.0 / 16.0);
            let prepared = projected
                .into_iter()
                .map(|layer| match layer {
                    ProjectedSpecialClusterLayer::Ordinary {
                        icon,
                        transformation,
                        display_transform,
                    } => {
                        let leaf = special_cluster_leaf_matrix(
                            display_transform,
                            transformation,
                            ground_fallback,
                        );
                        let transforms = cluster_transforms(base, leaf, count, seed, depth);
                        let layers = item_runtime
                            .generated_item_layers_for_icon(icon)
                            .into_iter()
                            .map(|layer| PreparedGeneratedClusterLayer {
                                identity: ItemModelAssetIdentity::new(
                                    layer.asset_id,
                                    Arc::<str>::from("generated"),
                                    0,
                                ),
                                geometry_class: if layer.translucent {
                                    ItemModelGeometryClass::Translucent
                                } else {
                                    ItemModelGeometryClass::Solid
                                },
                                source: ItemModelAssetSource::Generated {
                                    mask: layer.mask,
                                    rect: layer.rect,
                                    translucent: layer.translucent,
                                },
                                tint: layer.tint,
                            })
                            .collect();
                        PreparedSpecialClusterLayer::Ordinary { layers, transforms }
                    }
                    ProjectedSpecialClusterLayer::Special {
                        model,
                        transformation,
                        display_transform,
                    } => {
                        let leaf = special_cluster_leaf_matrix(
                            display_transform,
                            transformation,
                            ground_fallback,
                        );
                        PreparedSpecialClusterLayer::Special {
                            model,
                            transforms: cluster_transforms(base, leaf, count, seed, depth),
                        }
                    }
                })
                .collect::<Vec<_>>();
            for layer in &prepared {
                if let PreparedSpecialClusterLayer::Ordinary { layers, .. } = layer {
                    residency
                        .assets
                        .extend(layers.iter().map(|layer| ItemModelAssetRequest {
                            identity: layer.identity.clone(),
                            atlas: ItemModelAtlasKind::Items,
                            geometry_class: layer.geometry_class,
                            source: layer.source.clone(),
                        }));
                }
            }
            // Vanilla submits the complete item state once per cluster copy,
            // so composite leaf order repeats inside each copy.
            for copy_index in 0..count {
                for layer in &prepared {
                    match layer {
                        PreparedSpecialClusterLayer::Ordinary { layers, transforms } => {
                            for layer in layers {
                                push_resident_item_instances_for_asset(
                                    &mut residency,
                                    layer.identity.clone(),
                                    ItemModelAtlasKind::Items,
                                    layer.geometry_class,
                                    std::slice::from_ref(&transforms[copy_index]),
                                    layer.tint,
                                    shader_light(state.light),
                                    ITEM_MODEL_NO_OVERLAY,
                                    foil,
                                    ItemModelDrawTarget::World,
                                );
                            }
                        }
                        PreparedSpecialClusterLayer::Special { model, transforms } => {
                            residency.special_items.push(SpecialItemInstanceRequest {
                                model: model.clone(),
                                transform: transforms[copy_index],
                                light: shader_light(state.light),
                                overlay: ITEM_MODEL_NO_OVERLAY,
                                foil,
                                target: ItemModelDrawTarget::World,
                                submit_order: u32::try_from(
                                    residency.instances.len() + residency.special_items.len(),
                                )
                                .expect("special item submit order overflow"),
                                scissor: None,
                            });
                        }
                    }
                }
            }
            handled_entity_ids.insert(state.entity_id);
            continue;
        }

        // Block path: the item is a block with 3D item geometry.
        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    let ground = ground(BLOCK_GROUND_FALLBACK);
                    let ground_matrix = display_matrix(&ground, false);
                    let (min_y, depth) = ground_model_bounds(&quads, ground_matrix);
                    let base =
                        base_transform(position, age_ticks, state.bob_offset, -min_y + 1.0 / 16.0);
                    let transforms = cluster_transforms(base, ground_matrix, count, seed, depth);
                    let identity = ItemModelAssetIdentity::new(
                        Arc::<str>::from(resource_id),
                        Arc::<str>::from("default"),
                        0,
                    );
                    let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
                    if quads.iter().any(|quad| !quad.translucent) {
                        push_resident_item_instances(
                            &mut residency,
                            identity.clone(),
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Solid,
                            source.clone(),
                            &transforms,
                            [1.0; 4],
                            shader_light(state.light),
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::World,
                        );
                    }
                    if quads.iter().any(|quad| quad.translucent) {
                        push_resident_item_instances(
                            &mut residency,
                            identity,
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Translucent,
                            source,
                            &transforms,
                            [1.0; 4],
                            shader_light(state.light),
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::World,
                        );
                    }
                    handled_entity_ids.insert(state.entity_id);
                    continue;
                }
            }
        }

        // Flat path: extrude the item's sprite layers into a slab.
        let layers = item_runtime.generated_item_layers_for_stack_with_registry_context(
            &state.stack,
            BlockModelDisplayContext::Ground,
            trim_material_keys,
            trim_pattern_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
        );
        if layers.is_empty() {
            continue;
        }
        let ground = ground(GENERATED_GROUND_FALLBACK);
        let ground_matrix = display_matrix(&ground, false);
        let (min_y, depth) = generated_ground_model_bounds(ground_matrix);
        let base = base_transform(position, age_ticks, state.bob_offset, -min_y + 1.0 / 16.0);
        let transforms = cluster_transforms(base, ground_matrix, count, seed, depth);
        for layer in layers {
            let identity =
                ItemModelAssetIdentity::new(layer.asset_id, Arc::<str>::from("generated"), 0);
            let geometry_class = if layer.translucent {
                ItemModelGeometryClass::Translucent
            } else {
                ItemModelGeometryClass::Solid
            };
            push_resident_item_instances(
                &mut residency,
                identity,
                ItemModelAtlasKind::Items,
                geometry_class,
                ItemModelAssetSource::Generated {
                    mask: layer.mask,
                    rect: layer.rect,
                    translucent: layer.translucent,
                },
                &transforms,
                layer.tint,
                shader_light(state.light),
                ITEM_MODEL_NO_OVERLAY,
                foil,
                ItemModelDrawTarget::World,
            );
        }
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
        residency,
        handled_entity_ids,
    }
}

enum ProjectedSpecialClusterLayer {
    Ordinary {
        icon: bbb_item_model::ItemAtlasIcon,
        transformation: bbb_item_model::ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
    Special {
        model: bbb_renderer::SpecialItemModel,
        transformation: bbb_item_model::ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
}

struct PreparedGeneratedClusterLayer {
    identity: ItemModelAssetIdentity,
    geometry_class: ItemModelGeometryClass,
    source: ItemModelAssetSource,
    tint: [f32; 4],
}

enum PreparedSpecialClusterLayer {
    Ordinary {
        layers: Vec<PreparedGeneratedClusterLayer>,
        transforms: Vec<Mat4>,
    },
    Special {
        model: bbb_renderer::SpecialItemModel,
        transforms: Vec<Mat4>,
    },
}

fn special_cluster_bounds(
    renderer: Option<&bbb_renderer::Renderer>,
    layers: &[ProjectedSpecialClusterLayer],
    ground_fallback: BlockModelDisplayTransform,
) -> (f32, f32) {
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for layer in layers {
        match layer {
            ProjectedSpecialClusterLayer::Ordinary {
                transformation,
                display_transform,
                ..
            } => {
                let transform = special_cluster_leaf_matrix(
                    *display_transform,
                    *transformation,
                    ground_fallback,
                );
                extend_transformed_bounds(
                    &mut min_y,
                    &mut min_z,
                    &mut max_z,
                    transform,
                    [0.0, 0.0, 7.5 / 16.0],
                    [1.0, 1.0, 8.5 / 16.0],
                );
            }
            ProjectedSpecialClusterLayer::Special {
                model,
                transformation,
                display_transform,
            } => {
                let Some(bounds) =
                    renderer.and_then(|renderer| renderer.special_item_model_bounds(model).ok())
                else {
                    continue;
                };
                extend_transformed_bounds(
                    &mut min_y,
                    &mut min_z,
                    &mut max_z,
                    special_cluster_leaf_matrix(
                        *display_transform,
                        *transformation,
                        ground_fallback,
                    ),
                    bounds.min,
                    bounds.max,
                );
            }
        }
    }
    if !min_y.is_finite() {
        return generated_ground_model_bounds(display_matrix(&ground_fallback, false));
    }
    (min_y, max_z - min_z)
}

fn special_cluster_leaf_matrix(
    display_transform: Option<BlockModelDisplayTransform>,
    transformation: bbb_item_model::ItemModelLeafTransformation,
    fallback: BlockModelDisplayTransform,
) -> Mat4 {
    display_matrix(&display_transform.unwrap_or(fallback), false)
        * special::special_item_leaf_matrix(transformation)
}

fn extend_transformed_bounds(
    min_y: &mut f32,
    min_z: &mut f32,
    max_z: &mut f32,
    transform: Mat4,
    min: [f32; 3],
    max: [f32; 3],
) {
    for x in [min[0], max[0]] {
        for y in [min[1], max[1]] {
            for z in [min[2], max[2]] {
                let point = transform.transform_point3(Vec3::new(x, y, z));
                *min_y = (*min_y).min(point.y);
                *min_z = (*min_z).min(point.z);
                *max_z = (*max_z).max(point.z);
            }
        }
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
    trim_pattern_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
                    let ground = ground(BLOCK_GROUND_FALLBACK);
                    let ground_matrix = display_matrix(&ground, false);
                    let (_, depth) = ground_model_bounds(&quads, ground_matrix);
                    let transforms = cluster_transforms(base, ground_matrix, count, seed, depth);
                    let identity = ItemModelAssetIdentity::new(
                        Arc::<str>::from(resource_id),
                        Arc::<str>::from("default"),
                        0,
                    );
                    let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
                    if quads.iter().any(|quad| !quad.translucent) {
                        push_resident_item_instances(
                            &mut models.residency,
                            identity.clone(),
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Solid,
                            source.clone(),
                            &transforms,
                            [1.0; 4],
                            ITEM_MODEL_FULL_BRIGHT_LIGHT,
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::World,
                        );
                    }
                    if quads.iter().any(|quad| quad.translucent) {
                        push_resident_item_instances(
                            &mut models.residency,
                            identity,
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Translucent,
                            source,
                            &transforms,
                            [1.0; 4],
                            ITEM_MODEL_FULL_BRIGHT_LIGHT,
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::World,
                        );
                    }
                    models.handled_entity_ids.insert(state.entity_id);
                    continue;
                }
            }
        }

        let layers = item_runtime.generated_item_layers_for_stack_with_registry_context(
            &state.stack,
            BlockModelDisplayContext::Ground,
            trim_material_keys,
            trim_pattern_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
        );
        if layers.is_empty() {
            continue;
        }
        let ground = ground(GENERATED_GROUND_FALLBACK);
        let ground_matrix = display_matrix(&ground, false);
        let (_, depth) = generated_ground_model_bounds(ground_matrix);
        let transforms = cluster_transforms(base, ground_matrix, count, seed, depth);
        for layer in layers {
            push_resident_item_instances(
                &mut models.residency,
                ItemModelAssetIdentity::new(layer.asset_id, Arc::<str>::from("generated"), 0),
                ItemModelAtlasKind::Items,
                if layer.translucent {
                    ItemModelGeometryClass::Translucent
                } else {
                    ItemModelGeometryClass::Solid
                },
                ItemModelAssetSource::Generated {
                    mask: layer.mask,
                    rect: layer.rect,
                    translucent: layer.translucent,
                },
                &transforms,
                layer.tint,
                ITEM_MODEL_FULL_BRIGHT_LIGHT,
                ITEM_MODEL_NO_OVERLAY,
                foil,
                ItemModelDrawTarget::World,
            );
        }
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
    trim_pattern_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> DroppedItemModels {
    let mut models = DroppedItemModels::empty();
    let Some(item_runtime) = item_runtime else {
        return models;
    };

    for state in states {
        if state.item.item_id < 0 || state.item.count <= 0 {
            continue;
        }
        // Rebuild the component-rich stack from the opaque patch the pickup
        // channel round-tripped through the renderer. The bytes are the
        // `DataComponentPatchSummary` that `ClientboundTakeItemEntity` already
        // decoded (serialized in `pickup_item_component_patch_bytes`); this is a
        // serde round-trip, not a second wire decode. `None` means the empty
        // default patch, matching the dropped-item bake for a plain stack.
        let component_patch = state
            .component_patch
            .as_deref()
            .and_then(|bytes| serde_json::from_slice::<DataComponentPatchSummary>(bytes).ok())
            .unwrap_or_default();
        let stack = ItemStackSummary {
            item_id: Some(state.item.item_id),
            count: state.item.count,
            component_patch,
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
                    let ground = ground(BLOCK_GROUND_FALLBACK);
                    let ground_matrix = display_matrix(&ground, false);
                    let (min_y, depth) = ground_model_bounds(&quads, ground_matrix);
                    let base = base_transform(
                        state.position,
                        state.age_ticks,
                        state.bob_offset,
                        -min_y + 1.0 / 16.0,
                    );
                    let transforms = cluster_transforms(base, ground_matrix, count, seed, depth);
                    let identity = ItemModelAssetIdentity::new(
                        Arc::<str>::from(resource_id),
                        Arc::<str>::from("default"),
                        0,
                    );
                    let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
                    if quads.iter().any(|quad| !quad.translucent) {
                        push_resident_item_instances(
                            &mut models.residency,
                            identity.clone(),
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Solid,
                            source.clone(),
                            &transforms,
                            [1.0; 4],
                            state.light,
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::Particle,
                        );
                    }
                    if quads.iter().any(|quad| quad.translucent) {
                        push_resident_item_instances(
                            &mut models.residency,
                            identity,
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Translucent,
                            source,
                            &transforms,
                            [1.0; 4],
                            state.light,
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::Particle,
                        );
                    }
                    continue;
                }
            }
        }

        let layers = item_runtime.generated_item_layers_for_stack_with_registry_context(
            &stack,
            BlockModelDisplayContext::Ground,
            trim_material_keys,
            trim_pattern_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
        );
        if layers.is_empty() {
            continue;
        }
        let ground = ground(GENERATED_GROUND_FALLBACK);
        let ground_matrix = display_matrix(&ground, false);
        let (min_y, depth) = generated_ground_model_bounds(ground_matrix);
        let base = base_transform(
            state.position,
            state.age_ticks,
            state.bob_offset,
            -min_y + 1.0 / 16.0,
        );
        let transforms = cluster_transforms(base, ground_matrix, count, seed, depth);
        for layer in layers {
            push_resident_item_instances(
                &mut models.residency,
                ItemModelAssetIdentity::new(layer.asset_id, Arc::<str>::from("generated"), 0),
                ItemModelAtlasKind::Items,
                if layer.translucent {
                    ItemModelGeometryClass::Translucent
                } else {
                    ItemModelGeometryClass::Solid
                },
                ItemModelAssetSource::Generated {
                    mask: layer.mask,
                    rect: layer.rect,
                    translucent: layer.translucent,
                },
                &transforms,
                layer.tint,
                state.light,
                ITEM_MODEL_NO_OVERLAY,
                foil,
                ItemModelDrawTarget::Particle,
            );
        }
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
        | BlockModelDisplayContext::None
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
    pub residency: ItemModelResidencyFrame,
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct ResidentItemTestMeshes {
    pub(super) block: Vec<ItemModelMesh>,
    pub(super) block_translucent: Vec<ItemModelMesh>,
    pub(super) block_glint: Vec<ItemModelMesh>,
    pub(super) block_glint_translucent: Vec<ItemModelMesh>,
    pub(super) flat: Vec<ItemModelMesh>,
    pub(super) flat_translucent: Vec<ItemModelMesh>,
    pub(super) flat_glint: Vec<ItemModelMesh>,
    pub(super) flat_glint_translucent: Vec<ItemModelMesh>,
}

/// Test-only compatibility projection: rebuilds the pre-residency meshes from
/// the new asset/instance contract so the existing vanilla-pose tests continue
/// to compare final geometry. Production never evaluates this path.
#[cfg(test)]
pub(super) fn resident_item_test_meshes(frame: &ItemModelResidencyFrame) -> ResidentItemTestMeshes {
    let mut meshes = ResidentItemTestMeshes::default();
    for instance in &frame.instances {
        let Some(asset) = frame.assets.iter().find(|asset| {
            asset.identity == instance.identity
                && asset.atlas == instance.atlas
                && asset.geometry_class == instance.geometry_class
        }) else {
            continue;
        };
        let mut quads = match &asset.source {
            ItemModelAssetSource::Quads(quads) => quads.as_ref().to_vec(),
            ItemModelAssetSource::Generated {
                mask,
                rect,
                translucent,
            } => bake_generated_item_quads(mask, *rect, [1.0; 4], *translucent),
        };
        quads.retain(|quad| {
            quad.translucent
                == matches!(instance.geometry_class, ItemModelGeometryClass::Translucent)
        });
        for quad in &mut quads {
            for (color, tint) in quad.tint.iter_mut().zip(instance.tint) {
                *color *= tint;
            }
        }
        let foil = if matches!(
            instance.phase,
            ItemModelDrawPhase::Glint | ItemModelDrawPhase::GlintTranslucent
        ) {
            instance.foil
        } else {
            ItemModelFoil::None
        };
        let baked = bake_item_model_meshes_with_light_and_overlay_and_foil_mode(
            &quads,
            instance.transform,
            instance.light,
            instance.overlay,
            foil,
        );
        // This helper compares only the legacy block/item buckets. Renderer-
        // owned special phases and entity atlases have no pre-residency mesh
        // counterpart and are covered by renderer semantic/offscreen tests.
        let target = match (instance.atlas, instance.phase) {
            (
                ItemModelAtlasKind::Blocks,
                ItemModelDrawPhase::Solid | ItemModelDrawPhase::SolidNoCutout,
            ) => &mut meshes.block,
            (ItemModelAtlasKind::Blocks, ItemModelDrawPhase::Translucent) => {
                &mut meshes.block_translucent
            }
            (ItemModelAtlasKind::Blocks, ItemModelDrawPhase::Glint) => &mut meshes.block_glint,
            (ItemModelAtlasKind::Blocks, ItemModelDrawPhase::GlintTranslucent) => {
                &mut meshes.block_glint_translucent
            }
            (
                ItemModelAtlasKind::Items,
                ItemModelDrawPhase::Solid | ItemModelDrawPhase::SolidNoCutout,
            ) => &mut meshes.flat,
            (ItemModelAtlasKind::Items, ItemModelDrawPhase::Translucent) => {
                &mut meshes.flat_translucent
            }
            (ItemModelAtlasKind::Items, ItemModelDrawPhase::Glint) => &mut meshes.flat_glint,
            (ItemModelAtlasKind::Items, ItemModelDrawPhase::GlintTranslucent) => {
                &mut meshes.flat_glint_translucent
            }
            (
                ItemModelAtlasKind::Entity
                | ItemModelAtlasKind::DynamicPlayerSkin
                | ItemModelAtlasKind::DynamicEntity,
                _,
            )
            | (
                _,
                ItemModelDrawPhase::SolidZOffsetForward
                | ItemModelDrawPhase::CutoutNoCull
                | ItemModelDrawPhase::CutoutZOffset
                | ItemModelDrawPhase::Pattern
                | ItemModelDrawPhase::TranslucentNoCull
                | ItemModelDrawPhase::EndPortal
                | ItemModelDrawPhase::EndGateway,
            ) => continue,
        };
        let mesh = match instance.phase {
            ItemModelDrawPhase::Solid | ItemModelDrawPhase::SolidNoCutout => baked.solid,
            ItemModelDrawPhase::Translucent => baked.translucent,
            ItemModelDrawPhase::Glint => baked.glint,
            ItemModelDrawPhase::GlintTranslucent => baked.glint_translucent,
            ItemModelDrawPhase::SolidZOffsetForward
            | ItemModelDrawPhase::CutoutNoCull
            | ItemModelDrawPhase::CutoutZOffset
            | ItemModelDrawPhase::Pattern
            | ItemModelDrawPhase::TranslucentNoCull
            | ItemModelDrawPhase::EndPortal
            | ItemModelDrawPhase::EndGateway => continue,
        };
        if !mesh.is_empty() {
            target.push(mesh);
        }
    }
    meshes
}

/// Bakes the third-person main- and off-hand held items for every humanoid entity that holds one
/// (players and the weapon-holding mobs — zombies, giants, skeletons, piglins, illagers; vanilla
/// `ItemInHandLayer`) plus non-humanoid `ArmedModel` users such as allays and copper golems. The hand
/// attach transform comes from the renderer's posed model; native resolves the item to quads (block or
/// flat) and applies the item's third-person display transform.
#[cfg(test)]
pub(crate) fn held_item_models(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> HeldItemModels {
    held_item_models_with_variant_textures(instances, world, item_runtime, terrain_textures, None)
}

pub(crate) fn held_item_models_with_variant_textures(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
) -> HeldItemModels {
    let mut block_meshes = Vec::new();
    let mut block_translucent_meshes = Vec::new();
    let mut block_glint_meshes = Vec::new();
    let mut block_glint_translucent_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut flat_translucent_meshes = Vec::new();
    let mut flat_glint_meshes = Vec::new();
    let mut flat_glint_translucent_meshes = Vec::new();
    let mut residency = ItemModelResidencyFrame::default();
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
            residency,
        };
    };
    let enchantment_keys = world_enchantment_keys(world);
    let damage_type_keys = world_damage_type_keys(world);
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
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_held_hand(
            instance,
            true,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_fox_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_dolphin_carried_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_witch_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_copper_golem_held_items(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_allay_held_items(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_villager_crossed_arms_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_panda_held_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
        bake_custom_head_item(
            instance,
            world,
            item_runtime,
            terrain_textures,
            variant_textures,
            enchantment_keys.as_deref(),
            damage_type_keys.as_deref(),
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
            &mut residency,
        );
    }

    #[cfg(test)]
    {
        let test_meshes = resident_item_test_meshes(&residency);
        block_meshes.extend(test_meshes.block);
        block_translucent_meshes.extend(test_meshes.block_translucent);
        block_glint_meshes.extend(test_meshes.block_glint);
        block_glint_translucent_meshes.extend(test_meshes.block_glint_translucent);
        flat_meshes.extend(test_meshes.flat);
        flat_translucent_meshes.extend(test_meshes.flat_translucent);
        flat_glint_meshes.extend(test_meshes.flat_glint);
        flat_glint_translucent_meshes.extend(test_meshes.flat_glint_translucent);
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
        residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
            world,
            hand,
            context,
            left_arm,
            Some(instance.render_state.main_arm_left),
            Some(instance.entity_id),
            entity_model_context_entity_type(instance.kind),
            world.entity_team_color_rgb(instance.entity_id),
            using_item,
            using_item_ticks,
            enchantment_keys,
            damage_type_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            variant_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
            residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
        world,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        Some(instance.entity_id),
        entity_model_context_entity_type(instance.kind),
        world.entity_team_color_rgb(instance.entity_id),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        variant_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
        world,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        Some(instance.entity_id),
        entity_model_context_entity_type(instance.kind),
        world.entity_team_color_rgb(instance.entity_id),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        variant_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
        world,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        Some(instance.entity_id),
        entity_model_context_entity_type(instance.kind),
        world.entity_team_color_rgb(instance.entity_id),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        variant_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
            world,
            hand,
            context,
            left_hand,
            Some(instance.render_state.main_arm_left),
            Some(instance.entity_id),
            entity_model_context_entity_type(instance.kind),
            world.entity_team_color_rgb(instance.entity_id),
            using_item,
            entity_use_elapsed_ticks(instance, using_item),
            enchantment_keys,
            damage_type_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            variant_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
            residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
            world,
            hand,
            context,
            left_hand,
            Some(instance.render_state.main_arm_left),
            Some(instance.entity_id),
            entity_model_context_entity_type(instance.kind),
            world.entity_team_color_rgb(instance.entity_id),
            using_item,
            entity_use_elapsed_ticks(instance, using_item),
            enchantment_keys,
            damage_type_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            variant_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
            residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
        world,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        Some(instance.entity_id),
        entity_model_context_entity_type(instance.kind),
        world.entity_team_color_rgb(instance.entity_id),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        variant_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
        world,
        transform,
        BlockModelDisplayContext::Ground,
        false,
        Some(instance.render_state.main_arm_left),
        Some(instance.entity_id),
        entity_model_context_entity_type(instance.kind),
        world.entity_team_color_rgb(instance.entity_id),
        using_item,
        entity_use_elapsed_ticks(instance, using_item),
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        BLOCK_GROUND_FALLBACK,
        GENERATED_GROUND_FALLBACK,
        item_runtime,
        terrain_textures,
        variant_textures,
        block_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        residency,
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
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
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
    residency: &mut ItemModelResidencyFrame,
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
        .is_some_and(bbb_world::block_name_is_floor_skull)
    {
        return;
    }
    for transform in custom_head_item_transforms(instance) {
        bake_item_stack_at_transform(
            &stack,
            world,
            transform,
            BlockModelDisplayContext::Head,
            false,
            Some(instance.render_state.main_arm_left),
            Some(instance.entity_id),
            entity_model_context_entity_type(instance.kind),
            world.entity_team_color_rgb(instance.entity_id),
            false,
            0.0,
            enchantment_keys,
            damage_type_keys,
            trim_material_keys,
            attribute_keys,
            context_dimension,
            BLOCK_HEAD_FALLBACK,
            GENERATED_HEAD_FALLBACK,
            item_runtime,
            terrain_textures,
            variant_textures,
            block_meshes,
            block_translucent_meshes,
            block_glint_meshes,
            block_glint_translucent_meshes,
            flat_meshes,
            flat_translucent_meshes,
            flat_glint_meshes,
            flat_glint_translucent_meshes,
            residency,
        );
    }
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
    world: &WorldStore,
    attach: Mat4,
    context: BlockModelDisplayContext,
    left_hand: bool,
    owner_main_hand_left: Option<bool>,
    living_owner_entity_id: Option<i32>,
    context_entity_type: Option<&str>,
    owner_team_color_rgb: Option<u32>,
    using_item: bool,
    using_item_ticks: f32,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_fallback: BlockModelDisplayTransform,
    generated_fallback: BlockModelDisplayTransform,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    _block_meshes: &mut Vec<ItemModelMesh>,
    _block_translucent_meshes: &mut Vec<ItemModelMesh>,
    _block_glint_meshes: &mut Vec<ItemModelMesh>,
    _block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    _flat_meshes: &mut Vec<ItemModelMesh>,
    _flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    _flat_glint_meshes: &mut Vec<ItemModelMesh>,
    _flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    residency: &mut ItemModelResidencyFrame,
) {
    bake_item_stack_at_transform_with_light_and_overlay(
        stack,
        world,
        attach,
        context,
        left_hand,
        owner_main_hand_left,
        living_owner_entity_id,
        context_entity_type,
        owner_team_color_rgb,
        using_item,
        using_item_ticks,
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        context_dimension,
        block_fallback,
        generated_fallback,
        item_runtime,
        terrain_textures,
        variant_textures,
        _block_meshes,
        _block_translucent_meshes,
        _block_glint_meshes,
        _block_glint_translucent_meshes,
        _flat_meshes,
        _flat_translucent_meshes,
        _flat_glint_meshes,
        _flat_glint_translucent_meshes,
        residency,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
        ITEM_MODEL_NO_OVERLAY,
    );
}

/// Display entities reuse the complete item-model resolution path but carry
/// `EntityRenderer.getPackedLightCoords` instead of the historical full-bright
/// attachment default.
#[allow(clippy::too_many_arguments)]
pub(super) fn bake_item_stack_at_transform_with_light_and_overlay(
    stack: &ItemStackSummary,
    world: &WorldStore,
    attach: Mat4,
    context: BlockModelDisplayContext,
    left_hand: bool,
    owner_main_hand_left: Option<bool>,
    living_owner_entity_id: Option<i32>,
    context_entity_type: Option<&str>,
    owner_team_color_rgb: Option<u32>,
    using_item: bool,
    using_item_ticks: f32,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    context_dimension: Option<&str>,
    block_fallback: BlockModelDisplayTransform,
    generated_fallback: BlockModelDisplayTransform,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    _block_meshes: &mut Vec<ItemModelMesh>,
    _block_translucent_meshes: &mut Vec<ItemModelMesh>,
    _block_glint_meshes: &mut Vec<ItemModelMesh>,
    _block_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    _flat_meshes: &mut Vec<ItemModelMesh>,
    _flat_translucent_meshes: &mut Vec<ItemModelMesh>,
    _flat_glint_meshes: &mut Vec<ItemModelMesh>,
    _flat_glint_translucent_meshes: &mut Vec<ItemModelMesh>,
    residency: &mut ItemModelResidencyFrame,
    light: [f32; 2],
    overlay: [f32; 2],
) {
    let Some(item_id) = stack.item_id else {
        return;
    };
    let retained = item_runtime.item_display_transform_for_stack(stack, context);
    let foil = item_stack_foil_mode(stack, item_runtime, context);
    let target = if matches!(
        context,
        BlockModelDisplayContext::FirstPersonLeftHand
            | BlockModelDisplayContext::FirstPersonRightHand
    ) {
        ItemModelDrawTarget::FirstPerson
    } else {
        ItemModelDrawTarget::World
    };

    let is_crossbow = item_runtime
        .item_resource_id(item_id)
        .is_some_and(|resource_id| {
            item_types::vanilla_item_resource_id_matches_type(
                resource_id,
                item_types::VANILLA_ITEM_CROSSBOW_ID,
            )
        });
    let use_context = if using_item && is_crossbow {
        let charge_duration = living_owner_entity_id
            .and_then(|entity_id| {
                item_runtime.crossbow_charge_duration_ticks_for_living_owner(
                    world,
                    &stack.component_patch.enchantments,
                    entity_id,
                )
            })
            .unwrap_or_else(|| {
                world.crossbow_charge_duration_ticks(&stack.component_patch.enchantments)
            });
        item_runtime.item_model_use_context_for_stack_with_crossbow_charge_duration(
            stack,
            item_model_elapsed_ticks(using_item_ticks),
            Some(charge_duration),
        )
    } else if using_item {
        item_runtime
            .item_model_use_context_for_stack(stack, item_model_elapsed_ticks(using_item_ticks))
    } else {
        ItemModelUseContext::inactive()
    };
    let trim_pattern_keys = world_trim_pattern_keys(world);
    let resolved = item_runtime.resolved_item_model_for_stack_with_owner_registry_context(
        stack,
        context,
        owner_main_hand_left,
        context_entity_type,
        owner_team_color_rgb,
        context_dimension,
        trim_material_keys,
        trim_pattern_keys.as_deref(),
        enchantment_keys,
        damage_type_keys,
        attribute_keys,
        using_item,
        use_context,
    );
    if resolved.as_ref().is_some_and(|resolved| {
        resolved
            .layers
            .iter()
            .any(|layer| matches!(layer, ResolvedItemModelLayer::Special { .. }))
    }) {
        for layer in resolved.into_iter().flat_map(|resolved| resolved.layers) {
            match layer {
                ResolvedItemModelLayer::Ordinary {
                    icon,
                    transformation,
                    display_transform,
                } => {
                    let display = display_transform.or(retained).unwrap_or(generated_fallback);
                    let transform = attach
                        * display_matrix(&display, left_hand)
                        * special::special_item_leaf_matrix(transformation);
                    for layer in item_runtime.generated_item_layers_for_icon(icon) {
                        push_resident_item_instances(
                            residency,
                            ItemModelAssetIdentity::new(
                                layer.asset_id,
                                Arc::<str>::from("generated"),
                                0,
                            ),
                            ItemModelAtlasKind::Items,
                            if layer.translucent {
                                ItemModelGeometryClass::Translucent
                            } else {
                                ItemModelGeometryClass::Solid
                            },
                            ItemModelAssetSource::Generated {
                                mask: layer.mask,
                                rect: layer.rect,
                                translucent: layer.translucent,
                            },
                            &[transform],
                            layer.tint,
                            light,
                            overlay,
                            foil,
                            target,
                        );
                    }
                }
                ResolvedItemModelLayer::Special {
                    model,
                    transformation,
                    display_transform,
                } => {
                    let Some(model) = special::project_special_item_model(
                        &model,
                        stack,
                        item_runtime,
                        Some(world),
                        variant_textures,
                    ) else {
                        continue;
                    };
                    let display = display_transform.or(retained).unwrap_or(generated_fallback);
                    residency.special_items.push(SpecialItemInstanceRequest {
                        model,
                        transform: attach
                            * display_matrix(&display, left_hand)
                            * special::special_item_leaf_matrix(transformation),
                        light,
                        overlay,
                        foil,
                        target,
                        submit_order: u32::try_from(
                            residency.instances.len() + residency.special_items.len(),
                        )
                        .expect("special item submit order overflow"),
                        scissor: None,
                    });
                }
            }
        }
        return;
    }

    // Block path.
    if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
        if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
            if !quads.is_empty() {
                let display = retained.unwrap_or(block_fallback);
                let transform = attach * display_matrix(&display, left_hand);
                let identity = ItemModelAssetIdentity::new(
                    Arc::<str>::from(resource_id),
                    Arc::<str>::from("default"),
                    0,
                );
                let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
                if quads.iter().any(|quad| !quad.translucent) {
                    push_resident_item_instances(
                        residency,
                        identity.clone(),
                        ItemModelAtlasKind::Blocks,
                        ItemModelGeometryClass::Solid,
                        source.clone(),
                        &[transform],
                        [1.0; 4],
                        light,
                        overlay,
                        foil,
                        target,
                    );
                }
                if quads.iter().any(|quad| quad.translucent) {
                    push_resident_item_instances(
                        residency,
                        identity,
                        ItemModelAtlasKind::Blocks,
                        ItemModelGeometryClass::Translucent,
                        source,
                        &[transform],
                        [1.0; 4],
                        light,
                        overlay,
                        foil,
                        target,
                    );
                }
                return;
            }
        }
    }

    // Flat path.
    let layers = item_runtime.generated_item_layers_for_stack_with_owner_registry_context(
        stack,
        context,
        owner_main_hand_left,
        context_entity_type,
        owner_team_color_rgb,
        context_dimension,
        trim_material_keys,
        trim_pattern_keys.as_deref(),
        enchantment_keys,
        damage_type_keys,
        attribute_keys,
        using_item,
        use_context,
    );
    if layers.is_empty() {
        return;
    }
    let display = retained.unwrap_or(generated_fallback);
    let transform = attach * display_matrix(&display, left_hand);
    for layer in layers {
        push_resident_item_instances(
            residency,
            ItemModelAssetIdentity::new(layer.asset_id, Arc::<str>::from("generated"), 0),
            ItemModelAtlasKind::Items,
            if layer.translucent {
                ItemModelGeometryClass::Translucent
            } else {
                ItemModelGeometryClass::Solid
            },
            ItemModelAssetSource::Generated {
                mask: layer.mask,
                rect: layer.rect,
                translucent: layer.translucent,
            },
            &[transform],
            layer.tint,
            light,
            overlay,
            foil,
            target,
        );
    }
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
#[cfg(test)]
pub(super) fn stacked_item_mesh(
    quads: &[ItemModelQuad],
    position: [f32; 3],
    age_ticks: f32,
    bob_offset: f32,
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
    let base = base_transform(position, age_ticks, bob_offset, min_offset_y);
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
    bob_offset: f32,
    min_offset_y: f32,
) -> Mat4 {
    // Vanilla `ItemEntityRenderer`: bob = sin(age/10 + bobOffs)·0.1 + 0.1; spin = age/20 + bobOffs.
    let bob = (age_ticks / 10.0 + bob_offset).sin() * 0.1 + 0.1;
    let spin = age_ticks / 20.0 + bob_offset;
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

fn generated_ground_model_bounds(ground_matrix: Mat4) -> (f32, f32) {
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for x in [0.0, 16.0] {
        for y in [0.0, 16.0] {
            for z in [7.5, 8.5] {
                let point = ground_matrix.transform_point3(Vec3::new(x, y, z) / 16.0);
                min_y = min_y.min(point.y);
                min_z = min_z.min(point.z);
                max_z = max_z.max(point.z);
            }
        }
    }
    (min_y, max_z - min_z)
}

fn cluster_transforms(base: Mat4, ground: Mat4, count: usize, seed: i64, depth: f32) -> Vec<Mat4> {
    let mut transforms = Vec::with_capacity(count);
    let mut random = LegacyRandom::new(seed);
    if depth > FLAT_ITEM_DEPTH_THRESHOLD {
        transforms.push(base * ground);
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15;
            let zo = (random.next_float() * 2.0 - 1.0) * 0.15;
            transforms.push(Mat4::from_translation(Vec3::new(xo, yo, zo)) * ground);
            let last = transforms.last_mut().expect("just pushed");
            *last = base * *last;
        }
    } else {
        let offset_z = depth * 1.5;
        let mut z = -(offset_z * (count as f32 - 1.0) / 2.0);
        transforms.push(base * Mat4::from_translation(Vec3::new(0.0, 0.0, z)) * ground);
        z += offset_z;
        for _ in 1..count {
            let xo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            let yo = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
            transforms.push(base * Mat4::from_translation(Vec3::new(xo, yo, z)) * ground);
            z += offset_z;
        }
    }
    transforms
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_resident_item_instances(
    frame: &mut ItemModelResidencyFrame,
    identity: ItemModelAssetIdentity,
    atlas: ItemModelAtlasKind,
    geometry_class: ItemModelGeometryClass,
    source: ItemModelAssetSource,
    transforms: &[Mat4],
    tint: [f32; 4],
    light: [f32; 2],
    overlay: [f32; 2],
    foil: ItemModelFoil,
    target: ItemModelDrawTarget,
) {
    frame.assets.push(ItemModelAssetRequest {
        identity: identity.clone(),
        atlas,
        geometry_class,
        source,
    });
    push_resident_item_instances_for_asset(
        frame,
        identity,
        atlas,
        geometry_class,
        transforms,
        tint,
        light,
        overlay,
        foil,
        target,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_resident_item_instances_for_asset(
    frame: &mut ItemModelResidencyFrame,
    identity: ItemModelAssetIdentity,
    atlas: ItemModelAtlasKind,
    geometry_class: ItemModelGeometryClass,
    transforms: &[Mat4],
    tint: [f32; 4],
    light: [f32; 2],
    overlay: [f32; 2],
    foil: ItemModelFoil,
    target: ItemModelDrawTarget,
) {
    let base_phase = match geometry_class {
        ItemModelGeometryClass::Solid => ItemModelDrawPhase::Solid,
        ItemModelGeometryClass::Translucent => ItemModelDrawPhase::Translucent,
    };
    for &transform in transforms {
        push_resident_item_instance(
            frame,
            identity.clone(),
            atlas,
            geometry_class,
            transform,
            tint,
            light,
            overlay,
            foil,
            target,
            base_phase,
        );
    }
    if foil != ItemModelFoil::None {
        let glint_phase = match geometry_class {
            ItemModelGeometryClass::Solid => ItemModelDrawPhase::Glint,
            ItemModelGeometryClass::Translucent => ItemModelDrawPhase::GlintTranslucent,
        };
        for &transform in transforms {
            push_resident_item_instance(
                frame,
                identity.clone(),
                atlas,
                geometry_class,
                transform,
                tint,
                light,
                overlay,
                foil,
                target,
                glint_phase,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_resident_item_instance(
    frame: &mut ItemModelResidencyFrame,
    identity: ItemModelAssetIdentity,
    atlas: ItemModelAtlasKind,
    geometry_class: ItemModelGeometryClass,
    transform: Mat4,
    tint: [f32; 4],
    light: [f32; 2],
    overlay: [f32; 2],
    foil: ItemModelFoil,
    target: ItemModelDrawTarget,
    phase: ItemModelDrawPhase,
) {
    let submit_order = u32::try_from(frame.instances.len() + frame.special_items.len())
        .expect("item instance order overflow");
    frame.instances.push(ItemModelInstanceRequest {
        identity,
        atlas,
        geometry_class,
        transform,
        tint,
        light,
        overlay,
        foil,
        target,
        phase,
        submit_order,
        scissor: None,
    });
}

/// Vanilla `ItemEntityRenderer.submitMultipleFromCount`: bake `count` copies of the model. A model
/// thicker than [`FLAT_ITEM_DEPTH_THRESHOLD`] scatters its copies in 3D; a thin (flat) one stacks them
/// back-to-front along Z with small in-plane jitter. The jitter draws from a Java-seeded RNG so the
/// cluster arrangement matches vanilla.
#[cfg(test)]
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

#[cfg(test)]
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
    for transform in cluster_transforms(base, ground, count, seed, depth) {
        append_quads_to_mesh_set(mesh, quads, transform, light, foil);
    }
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
