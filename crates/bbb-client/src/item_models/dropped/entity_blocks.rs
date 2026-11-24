use super::*;

pub(in crate::item_models) struct EntityBlockAttachment {
    pub(in crate::item_models) block_id: Cow<'static, str>,
    pub(in crate::item_models) properties: Cow<'static, BTreeMap<String, String>>,
    pub(in crate::item_models) transform: Mat4,
    pub(in crate::item_models) light: [f32; 2],
    pub(in crate::item_models) overlay: [f32; 2],
    pub(in crate::item_models) outline_color: u32,
    pub(in crate::item_models) outline_only: bool,
    pub(in crate::item_models) moving_block: Option<MovingBlockRenderContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::item_models) struct MovingBlockRenderContext {
    pub(in crate::item_models) random_seed_pos: BlockPos,
    pub(in crate::item_models) block_pos: BlockPos,
    pub(in crate::item_models) biome_id: Option<i32>,
    pub(in crate::item_models) cardinal_lighting: TerrainCardinalLighting,
}

pub(in crate::item_models) fn entity_render_state_shader_light(
    instance: &EntityModelInstance,
) -> [f32; 2] {
    let block = (instance.render_state.light_coords >> 4) & 0xF;
    let sky = (instance.render_state.light_coords >> 20) & 0xF;
    [block as f32 / 15.0, sky as f32 / 15.0]
}

pub(super) fn terrain_light_shader(light: TerrainLight) -> [f32; 2] {
    [
        f32::from(light.block.min(15)) / 15.0,
        f32::from(light.sky.min(15)) / 15.0,
    ]
}

pub(super) fn minecart_tnt_display_block_overlay(instance: &EntityModelInstance) -> [f32; 2] {
    let fuse = instance
        .render_state
        .kind
        .minecart()
        .tnt_fuse_remaining_in_ticks;
    if fuse > -1.0 && (fuse as i32) / 5 % 2 == 0 {
        [15.0, 10.0]
    } else {
        ITEM_MODEL_NO_OVERLAY
    }
}

pub(super) fn primed_tnt_block_overlay(fuse_remaining_in_ticks: f32) -> [f32; 2] {
    if (fuse_remaining_in_ticks as i32) / 5 % 2 == 0 {
        [15.0, 10.0]
    } else {
        ITEM_MODEL_NO_OVERLAY
    }
}

pub(in crate::item_models) fn falling_block_should_render(
    instance: &EntityModelInstance,
    world: &WorldStore,
    block_state_id: i32,
) -> bool {
    match world.probe_block(falling_block_block_pos(instance, world)) {
        Some(block) => block.block_state_id != block_state_id,
        None => true,
    }
}

pub(super) fn falling_block_block_pos(
    instance: &EntityModelInstance,
    world: &WorldStore,
) -> BlockPos {
    let position = world
        .probe_entity_transform(instance.entity_id)
        .map(|state| state.position)
        .unwrap_or_else(|| bbb_client_world::EntityVec3 {
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
    moving_piston_visibility: MovingPistonVisibility<'_>,
    christmas_chests: bool,
) -> ItemModelResidencyFrame {
    let attachments = entity_block_attachments(
        instances,
        world,
        item_runtime,
        entity_partial_tick.clamp(0.0, 1.0),
        moving_piston_visibility,
    );
    if attachments.is_empty() {
        return ItemModelResidencyFrame::default();
    }

    let mut residency = ItemModelResidencyFrame::default();
    for attachment in attachments {
        if attachment.outline_only && attachment.outline_color == 0 {
            continue;
        }
        let base_variant = block_item_variant_identity(&attachment.properties);
        let moving_block = attachment.moving_block.is_some();
        let quads = if let Some(context) = attachment.moving_block {
            terrain_textures
                .moving_block_quads(
                    attachment.block_id.as_ref(),
                    &attachment.properties,
                    context.random_seed_pos,
                    context.block_pos,
                    context.biome_id,
                    context.cardinal_lighting,
                )
                .map(|quads| {
                    (
                        quads,
                        moving_block_variant_identity(base_variant.as_ref(), context),
                    )
                })
        } else {
            terrain_textures
                .block_item_quads(attachment.block_id.as_ref(), &attachment.properties)
                .map(|quads| (quads, base_variant))
        };
        if let Some((quads, variant)) = quads.filter(|(quads, _)| !quads.is_empty()) {
            let identity = ItemModelAssetIdentity::new(
                Arc::<str>::from(attachment.block_id.as_ref()),
                variant,
                0,
            );
            let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
            if quads.iter().any(|quad| !quad.translucent) {
                push_entity_block_geometry(
                    &mut residency,
                    identity.clone(),
                    ItemModelGeometryClass::Solid,
                    source.clone(),
                    &attachment,
                    block_model_solid_phase(
                        moving_block,
                        attachment.block_id.as_ref(),
                        terrain_textures,
                    ),
                );
            }
            if quads.iter().any(|quad| quad.translucent) {
                push_entity_block_geometry(
                    &mut residency,
                    identity,
                    ItemModelGeometryClass::Translucent,
                    source,
                    &attachment,
                    ItemModelDrawPhase::Translucent,
                );
            }
        }
        // Moving blocks (falling blocks, piston heads) submit quads only.
        if let Some(item_runtime) =
            item_runtime.filter(|_| !moving_block && !attachment.outline_only)
        {
            push_special_block_model(
                &mut residency,
                item_runtime,
                world,
                attachment.block_id.as_ref(),
                &attachment.properties,
                christmas_chests,
                attachment.transform,
                attachment.light,
                attachment.overlay,
                (attachment.outline_color != 0).then_some(attachment.outline_color),
            );
        }
    }
    residency
}

pub(super) fn push_entity_block_geometry(
    residency: &mut ItemModelResidencyFrame,
    identity: ItemModelAssetIdentity,
    geometry_class: ItemModelGeometryClass,
    source: ItemModelAssetSource,
    attachment: &EntityBlockAttachment,
    ordinary_phase: ItemModelDrawPhase,
) {
    residency.assets.push(ItemModelAssetRequest {
        identity: identity.clone(),
        atlas: ItemModelAtlasKind::Blocks,
        geometry_class,
        source,
    });
    if !attachment.outline_only {
        let first_instance = residency.instances.len();
        push_resident_item_instances_for_asset(
            residency,
            identity.clone(),
            ItemModelAtlasKind::Blocks,
            geometry_class,
            std::slice::from_ref(&attachment.transform),
            [1.0; 4],
            attachment.light,
            attachment.overlay,
            ItemModelFoil::None,
            ItemModelDrawTarget::BlockFeature,
        );
        for instance in &mut residency.instances[first_instance..] {
            instance.phase = ordinary_phase;
        }
    }
    if attachment.outline_color != 0 {
        push_resident_item_outline_instances_for_asset(
            residency,
            identity,
            ItemModelAtlasKind::Blocks,
            geometry_class,
            std::slice::from_ref(&attachment.transform),
            attachment.outline_color,
            attachment.light,
            attachment.overlay,
        );
    }
}

pub(in crate::item_models) fn argb_outline_tint(color: u32) -> [f32; 4] {
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        ((color >> 24) & 0xff) as f32 / 255.0,
    ]
}

pub(in crate::item_models) fn block_model_solid_phase(
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

pub(super) fn moving_block_variant_identity(
    base: &str,
    context: MovingBlockRenderContext,
) -> Arc<str> {
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

pub(in crate::item_models) fn block_item_variant_identity(
    properties: &BTreeMap<String, String>,
) -> Arc<str> {
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

/// `PistonHeadRenderer` is an ordinary block-entity renderer, so its moving
/// blocks pass the same visible-section + `getViewDistance()` gate as any other
/// block-entity submission. The gate is absent in unit tests, where every
/// moving block is submitted.
#[derive(Clone, Copy, Default)]
pub(crate) struct MovingPistonVisibility<'a> {
    pub(super) gate: Option<(BlockEntityVisibility, &'a TerrainUploadState)>,
}

impl<'a> MovingPistonVisibility<'a> {
    pub(crate) fn new(
        visibility: BlockEntityVisibility,
        terrain_upload: &'a TerrainUploadState,
    ) -> Self {
        Self {
            gate: Some((visibility, terrain_upload)),
        }
    }

    pub(super) fn allows(self, world: &WorldStore, pos: BlockPos) -> bool {
        let Some((visibility, terrain_upload)) = self.gate else {
            return true;
        };
        visibility.block_position_is_visible(pos, PISTON_HEAD_VIEW_DISTANCE, world, terrain_upload)
    }
}

pub(in crate::item_models) fn entity_block_attachments(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
    moving_piston_visibility: MovingPistonVisibility<'_>,
) -> Vec<EntityBlockAttachment> {
    let mut attachments = Vec::new();
    for instance in instances {
        if let Some(transform) = snow_golem_head_block_transform(instance) {
            attachments.push(EntityBlockAttachment {
                block_id: Cow::Borrowed(CARVED_PUMPKIN_BLOCK_ID),
                properties: Cow::Owned(carved_pumpkin_default_properties()),
                transform,
                light: entity_render_state_shader_light(instance),
                overlay: ITEM_MODEL_NO_OVERLAY,
                outline_color: instance.render_state.outline_color,
                outline_only: entity_block_attachment_outline_only(instance),
                moving_block: None,
            });
        }
        if let Some(transform) = iron_golem_flower_block_transform(instance) {
            attachments.push(EntityBlockAttachment {
                block_id: Cow::Borrowed(POPPY_BLOCK_ID),
                properties: Cow::Owned(poppy_default_properties()),
                transform,
                light: entity_render_state_shader_light(instance),
                overlay: ITEM_MODEL_NO_OVERLAY,
                outline_color: instance.render_state.outline_color,
                outline_only: false,
                moving_block: None,
            });
        }
        if let Some(transform) = enderman_carried_block_transform(instance) {
            if let Some(block_state) = world.enderman_carried_block_state(instance.entity_id) {
                attachments.push(EntityBlockAttachment {
                    block_id: Cow::Borrowed(&block_state.name),
                    properties: Cow::Borrowed(&block_state.properties),
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: ITEM_MODEL_NO_OVERLAY,
                    outline_color: instance.render_state.outline_color,
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
                    block_id: Cow::Borrowed(&display.block.name),
                    properties: Cow::Borrowed(&display.block.properties),
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: minecart_tnt_display_block_overlay(instance),
                    outline_color: instance.render_state.outline_color,
                    outline_only: false,
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
                    block_id: Cow::Borrowed(&block.name),
                    properties: Cow::Borrowed(&block.properties),
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: primed_tnt_block_overlay(fuse),
                    outline_color: instance.render_state.outline_color,
                    outline_only: false,
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
                        block_id: Cow::Borrowed(&falling.block.name),
                        properties: Cow::Borrowed(&falling.block.properties),
                        transform,
                        light,
                        overlay: ITEM_MODEL_NO_OVERLAY,
                        outline_color: 0,
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
                    properties: Cow::Owned(mooshroom_mushroom_default_properties()),
                    transform,
                    light: entity_render_state_shader_light(instance),
                    overlay: ITEM_MODEL_NO_OVERLAY,
                    outline_color: instance.render_state.outline_color,
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
        if !moving_piston_visibility.allows(world, moving.piston_pos) {
            continue;
        }
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
            block_id: Cow::Borrowed(&block_state.name),
            properties: Cow::Borrowed(&block_state.properties),
            transform: Mat4::from_translation(Vec3::from_array(moving.translation)),
            light,
            overlay: ITEM_MODEL_NO_OVERLAY,
            outline_color: 0,
            outline_only: false,
            moving_block: Some(moving_block),
        });
    }
    attachments
}

pub(super) fn moving_block_render_context(
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

pub(super) fn entity_block_attachment_outline_only(instance: &EntityModelInstance) -> bool {
    instance.render_state.invisible && instance.render_state.appears_glowing
}

pub(super) fn copper_golem_antenna_block_attachment(
    instance: &EntityModelInstance,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<EntityBlockAttachment> {
    let item_runtime = item_runtime?;
    let stack = world.equipment_item(instance.entity_id, EquipmentSlot::Saddle)?;
    let item_id = stack.item_id?;
    let block_id = item_runtime.item_resource_id(item_id)?;
    copper_golem_antenna_block_attachment_from_stack(instance, stack, block_id)
}

pub(in crate::item_models) fn copper_golem_antenna_block_attachment_from_stack(
    instance: &EntityModelInstance,
    stack: &ItemStackSummary,
    block_id: &str,
) -> Option<EntityBlockAttachment> {
    let transform = copper_golem_antenna_block_transform(instance)?;
    Some(EntityBlockAttachment {
        block_id: Cow::Owned(block_id.to_string()),
        properties: Cow::Owned(
            stack
                .component_patch
                .block_state_properties
                .iter()
                .filter_map(|(name, value)| {
                    Some((
                        String::from_utf16(name.units()).ok()?,
                        String::from_utf16(value.units()).ok()?,
                    ))
                })
                .collect(),
        ),
        transform,
        light: entity_render_state_shader_light(instance),
        overlay: ITEM_MODEL_NO_OVERLAY,
        outline_color: instance.render_state.outline_color,
        outline_only: false,
        moving_block: None,
    })
}

/// Vanilla `Blocks.CARVED_PUMPKIN.defaultBlockState()` sets `FACING = NORTH`; the snow-golem head layer
/// uses that exact blockstate.
pub(in crate::item_models) fn carved_pumpkin_default_properties() -> BTreeMap<String, String> {
    BTreeMap::from([("facing".to_string(), "north".to_string())])
}

/// Vanilla `Blocks.POPPY.defaultBlockState()` has no properties; the iron-golem flower layer uses that
/// exact blockstate while `offerFlowerTick > 0`.
pub(in crate::item_models) fn poppy_default_properties() -> BTreeMap<String, String> {
    BTreeMap::new()
}

pub(in crate::item_models) fn mooshroom_mushroom_block_id(
    variant: MooshroomVariant,
) -> &'static str {
    match variant {
        MooshroomVariant::Red => RED_MUSHROOM_BLOCK_ID,
        MooshroomVariant::Brown => BROWN_MUSHROOM_BLOCK_ID,
    }
}

/// Vanilla `MushroomCow.Variant` stores `Blocks.RED_MUSHROOM.defaultBlockState()` or
/// `Blocks.BROWN_MUSHROOM.defaultBlockState()`, both property-less states.
pub(in crate::item_models) fn mooshroom_mushroom_default_properties() -> BTreeMap<String, String> {
    BTreeMap::new()
}
