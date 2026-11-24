use super::*;
use crate::block_entity_visibility::{BlockEntityVisibility, PISTON_HEAD_VIEW_DISTANCE};
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;
use crate::terrain_upload::TerrainUploadState;

mod entity_blocks;
mod held;
pub(crate) use entity_blocks::*;
pub(crate) use held::*;

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
    pub residency: ItemModelResidencyFrame,
    pub handled_entity_ids: BTreeSet<i32>,
}

impl DroppedItemModels {
    fn empty() -> Self {
        Self {
            residency: ItemModelResidencyFrame::default(),
            handled_entity_ids: BTreeSet::new(),
        }
    }
}

/// Bakes a 3D model for every dropped item entity at the entity's world position with
/// vanilla's bob + spin animation, the GROUND display transform, and the count-based cluster
/// of copies. `age_ticks` is the animation clock (world game time + partial tick).
pub(crate) fn dropped_item_models(
    world: &WorldStore,
    states: &[bbb_client_world::ItemEntityStackState<'_>],
    item_runtime: Option<&NativeItemRuntime>,
    renderer: Option<&bbb_client_render::Renderer>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    age_ticks: f32,
    registry_keys: &ItemRegistryKeySet,
) -> DroppedItemModels {
    let mut models = DroppedItemModels::empty();
    let Some(item_runtime) = item_runtime else {
        return models;
    };
    let env = ItemBakeEnv {
        world,
        item_runtime,
        variant_textures,
        registry_keys: registry_keys.model_keys(),
    };
    for state in states {
        let position = [
            state.position.x as f32,
            state.position.y as f32,
            state.position.z as f32,
        ];
        if bake_item_cluster(
            &env,
            renderer,
            state.stack,
            shader_light(state.light),
            ItemModelDrawTarget::World,
            |min_y| base_transform(position, age_ticks, state.bob_offset, -min_y + 1.0 / 16.0),
            &mut models.residency,
        ) {
            models.handled_entity_ids.insert(state.entity_id);
        }
    }
    models
}

/// Vanilla `ItemEntityRenderer.submitMultipleFromCount`: the stack's GROUND model repeated
/// `rendered_amount` times. `base` receives the model floor (`modelBoundingBox.minY`) so item
/// entities can seat the cluster on the ground. Returns whether anything was submitted.
fn bake_item_cluster(
    env: &ItemBakeEnv<'_>,
    renderer: Option<&bbb_client_render::Renderer>,
    stack: &ItemStackSummary,
    light: [f32; 2],
    target: ItemModelDrawTarget,
    base: impl Fn(f32) -> Mat4,
    residency: &mut ItemModelResidencyFrame,
) -> bool {
    let item_runtime = env.item_runtime;
    let effective_stack = env.world.effective_item_stack(stack);
    let stack = effective_stack.as_ref();
    if stack.item_id.is_none() {
        return false;
    }
    let context = env.model_context(BlockModelDisplayContext::Ground);
    let Some(geometry) = item_runtime.resolve_item_model(stack, &context) else {
        return false;
    };
    let count = rendered_amount(stack.count);
    let seed = item_cluster_seed(stack);
    // The item's own retained GROUND display transform; falls back to the vanilla `block/block` or
    // `item/generated` default.
    let ground = |default_transform| {
        item_runtime
            .item_display_transform_for_stack(stack, BlockModelDisplayContext::Ground)
            .unwrap_or(default_transform)
    };
    let submit = ItemSubmit {
        light,
        overlay: ITEM_MODEL_NO_OVERLAY,
        foil: item_stack_foil_mode(stack, item_runtime, BlockModelDisplayContext::Ground),
        target,
        outline_color: None,
    };
    let ground_fallback = ground(GENERATED_GROUND_FALLBACK);
    let projected = geometry
        .layers
        .into_iter()
        .filter_map(|layer| match layer {
            ResolvedItemModelLayer::Cuboid {
                parts,
                transformation,
                display_transform,
                ..
            } => Some(ProjectedItemClusterLayer::Cuboid {
                parts,
                transformation,
                display_transform,
            }),
            ResolvedItemModelLayer::Ordinary {
                icon,
                transformation,
                display_transform,
            } => Some(ProjectedItemClusterLayer::Ordinary {
                icon,
                transformation,
                display_transform,
            }),
            ResolvedItemModelLayer::Special {
                model,
                transformation,
                display_transform,
                ..
            } => special::project_special_item_model(
                &model,
                stack,
                item_runtime,
                Some(env.world),
                env.variant_textures,
            )
            .map(|model| ProjectedItemClusterLayer::Special {
                model,
                transformation,
                display_transform,
            }),
        })
        .collect::<Vec<_>>();
    let (min_y, depth) = item_cluster_bounds(renderer, &projected, ground_fallback);
    let base = base(min_y);
    let prepared = projected
        .into_iter()
        .map(|layer| match layer {
            ProjectedItemClusterLayer::Cuboid {
                parts,
                transformation,
                display_transform,
            } => {
                let leaf =
                    item_cluster_leaf_matrix(display_transform, transformation, ground_fallback);
                let transforms = cluster_transforms(base, leaf, count, seed, depth);
                let mut layers = Vec::new();
                for part in parts.iter() {
                    for geometry_class in [
                        ItemModelGeometryClass::Solid,
                        ItemModelGeometryClass::Translucent,
                    ] {
                        if !part.geometry.has_geometry_class(geometry_class) {
                            continue;
                        }
                        layers.push(PreparedItemClusterPart {
                            identity: ItemModelAssetIdentity::new(
                                part.model_id.clone(),
                                "cuboid",
                                part.part,
                            ),
                            geometry_class,
                            source: ItemModelAssetSource::Model(part.geometry.clone()),
                            tint: part.tint,
                        });
                    }
                }
                PreparedItemClusterLayer::Ordinary { layers, transforms }
            }
            ProjectedItemClusterLayer::Ordinary {
                icon,
                transformation,
                display_transform,
            } => {
                let leaf =
                    item_cluster_leaf_matrix(display_transform, transformation, ground_fallback);
                let transforms = cluster_transforms(base, leaf, count, seed, depth);
                let layers = item_runtime
                    .generated_item_layers_for_icon(&icon)
                    .into_iter()
                    .map(|layer| PreparedItemClusterPart {
                        identity: ItemModelAssetIdentity::new(layer.asset_id, "generated", 0),
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
                PreparedItemClusterLayer::Ordinary { layers, transforms }
            }
            ProjectedItemClusterLayer::Special {
                model,
                transformation,
                display_transform,
            } => {
                let leaf =
                    item_cluster_leaf_matrix(display_transform, transformation, ground_fallback);
                PreparedItemClusterLayer::Special {
                    model,
                    transforms: cluster_transforms(base, leaf, count, seed, depth),
                }
            }
        })
        .collect::<Vec<_>>();
    for layer in &prepared {
        if let PreparedItemClusterLayer::Ordinary { layers, .. } = layer {
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
                PreparedItemClusterLayer::Ordinary { layers, transforms } => {
                    for layer in layers {
                        push_resident_item_instances_for_asset(
                            residency,
                            layer.identity.clone(),
                            ItemModelAtlasKind::Items,
                            layer.geometry_class,
                            std::slice::from_ref(&transforms[copy_index]),
                            layer.tint,
                            submit.light,
                            submit.overlay,
                            submit.foil,
                            submit.target,
                        );
                    }
                }
                PreparedItemClusterLayer::Special { model, transforms } => {
                    submit.special(residency, model.clone(), transforms[copy_index]);
                }
            }
        }
    }
    true
}

enum ProjectedItemClusterLayer {
    Cuboid {
        parts: Arc<[bbb_client_render::item_model::ResolvedItemCuboidPart]>,
        transformation: bbb_client_render::item_model::ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
    Ordinary {
        icon: bbb_client_render::item_model::ItemAtlasIcon,
        transformation: bbb_client_render::item_model::ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
    Special {
        model: bbb_client_render::SpecialItemModel,
        transformation: bbb_client_render::item_model::ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
}

struct PreparedItemClusterPart {
    identity: ItemModelAssetIdentity,
    geometry_class: ItemModelGeometryClass,
    source: ItemModelAssetSource,
    tint: [f32; 4],
}

enum PreparedItemClusterLayer {
    Ordinary {
        layers: Vec<PreparedItemClusterPart>,
        transforms: Vec<Mat4>,
    },
    Special {
        model: bbb_client_render::SpecialItemModel,
        transforms: Vec<Mat4>,
    },
}

fn item_cluster_bounds(
    renderer: Option<&bbb_client_render::Renderer>,
    layers: &[ProjectedItemClusterLayer],
    ground_fallback: BlockModelDisplayTransform,
) -> (f32, f32) {
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for layer in layers {
        match layer {
            ProjectedItemClusterLayer::Cuboid { parts, .. } => {
                for part in parts.iter() {
                    if let Some((min, max)) = part.ground_model_extents {
                        min_y = min_y.min(min.y);
                        min_z = min_z.min(min.z);
                        max_z = max_z.max(max.z);
                    }
                }
            }
            ProjectedItemClusterLayer::Ordinary {
                transformation,
                display_transform,
                ..
            } => {
                let transform =
                    item_cluster_leaf_matrix(*display_transform, *transformation, ground_fallback);
                extend_transformed_bounds(
                    &mut min_y,
                    &mut min_z,
                    &mut max_z,
                    transform,
                    [0.0, 0.0, 7.5 / 16.0],
                    [1.0, 1.0, 8.5 / 16.0],
                );
            }
            ProjectedItemClusterLayer::Special {
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
                    item_cluster_leaf_matrix(*display_transform, *transformation, ground_fallback),
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

fn item_cluster_leaf_matrix(
    display_transform: Option<BlockModelDisplayTransform>,
    transformation: bbb_client_render::item_model::ItemModelLeafTransformation,
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
    _terrain_textures: &TerrainTextureState,
    renderer: Option<&bbb_client_render::Renderer>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    entity_partial_tick: f32,
    registry_keys: &ItemRegistryKeySet,
) -> DroppedItemModels {
    let mut models = DroppedItemModels::empty();
    let Some(item_runtime) = item_runtime else {
        return models;
    };
    let env = ItemBakeEnv {
        world,
        item_runtime,
        variant_textures,
        registry_keys: registry_keys.model_keys(),
    };
    for state in world.ominous_item_spawner_item_states_at_partial_tick(entity_partial_tick) {
        let base = ominous_item_spawner_base_transform(
            [
                state.position.x as f32,
                state.position.y as f32,
                state.position.z as f32,
            ],
            state.age_ticks,
        );
        if bake_item_cluster(
            &env,
            renderer,
            state.stack,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
            ItemModelDrawTarget::World,
            |_| base,
            &mut models.residency,
        ) {
            models.handled_entity_ids.insert(state.entity_id);
        }
    }
    models
}

/// Bakes the item-stack render states carried by vanilla `ItemPickupParticleGroup`. The
/// particle runtime owns the target interpolation; this reuses the dropped-item cluster bake at
/// the extracted particle position.
pub(crate) fn item_pickup_particle_item_models(
    world: &WorldStore,
    states: &[ItemPickupParticleRenderState],
    item_runtime: Option<&NativeItemRuntime>,
    _terrain_textures: &TerrainTextureState,
    renderer: Option<&bbb_client_render::Renderer>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    registry_keys: &ItemRegistryKeySet,
) -> DroppedItemModels {
    let mut models = DroppedItemModels::empty();
    let Some(item_runtime) = item_runtime else {
        return models;
    };
    let env = ItemBakeEnv {
        world,
        item_runtime,
        variant_textures,
        registry_keys: registry_keys.model_keys(),
    };
    for state in states {
        if state.item.item_id < 0 || state.item.count <= 0 {
            continue;
        }
        let component_patch = state
            .component_patch
            .as_deref()
            .cloned()
            .unwrap_or_default();
        let stack = ItemStackSummary {
            item_id: Some(state.item.item_id),
            count: state.item.count,
            component_patch: component_patch.into(),
        };
        bake_item_cluster(
            &env,
            renderer,
            &stack,
            state.light,
            ItemModelDrawTarget::Particle,
            |min_y| {
                base_transform(
                    state.position,
                    state.age_ticks,
                    state.bob_offset,
                    -min_y + 1.0 / 16.0,
                )
            },
            &mut models.residency,
        );
    }
    models
}

pub(crate) use bbb_client_render::item_model::item_display_matrix as display_matrix;

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

/// Vanilla `ItemClusterRenderState.getRenderedAmount`: the number of copies rendered for a stack size.
/// Vanilla `ItemClusterRenderState.getSeedForItemStack`: `Item.getId(item)`
/// plus the stack's damage value.
pub(super) fn item_cluster_seed(stack: &ItemStackSummary) -> i64 {
    i64::from(stack.item_id.unwrap_or(0)) + i64::from(stack.component_patch.damage.unwrap_or(0))
}

pub(super) fn rendered_amount(stack_count: i32) -> usize {
    match stack_count {
        i32::MIN..=1 => 1,
        2..=16 => 2,
        17..=32 => 3,
        33..=48 => 4,
        _ => 5,
    }
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
#[cfg(test)]
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

pub(super) fn cluster_transforms(
    base: Mat4,
    ground: Mat4,
    count: usize,
    seed: i64,
    depth: f32,
) -> Vec<Mat4> {
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
pub(in crate::item_models) fn push_resident_item_outline_instances_for_asset(
    frame: &mut ItemModelResidencyFrame,
    identity: ItemModelAssetIdentity,
    atlas: ItemModelAtlasKind,
    geometry_class: ItemModelGeometryClass,
    transforms: &[Mat4],
    outline_color: u32,
    light: [f32; 2],
    overlay: [f32; 2],
) {
    let tint = argb_outline_tint(outline_color);
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
            ItemModelFoil::None,
            ItemModelDrawTarget::EntityOutline,
            ItemModelDrawPhase::Outline,
        );
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
