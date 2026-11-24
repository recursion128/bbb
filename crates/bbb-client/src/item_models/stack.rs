//! One ordered item-model leaf bake shared by every world item consumer.

use super::*;
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;
use bbb_client_render::item_model::{ItemAtlasIcon, ItemModelContext, ItemModelRegistryKeys};

impl ItemRegistryKeySet {
    pub(crate) fn model_keys(&self) -> ItemModelRegistryKeys<'_> {
        ItemModelRegistryKeys {
            trim_materials: self.trim_materials.as_deref(),
            trim_patterns: self.trim_patterns.as_deref(),
            enchantments: self.enchantments.as_deref(),
            attributes: self.attributes.as_deref(),
        }
    }
}

/// Frame inputs shared by every world-space item bake.
#[derive(Clone, Copy)]
pub(crate) struct ItemBakeEnv<'a> {
    pub(crate) world: &'a WorldStore,
    pub(crate) item_runtime: &'a NativeItemRuntime,
    pub(crate) variant_textures: Option<&'a NativeEntityVariantTextureRuntime>,
    pub(crate) registry_keys: ItemModelRegistryKeys<'a>,
}

impl<'a> ItemBakeEnv<'a> {
    /// Vanilla `updateForNonLiving` / `updateForLiving` always pass the
    /// owner's client level.
    pub(crate) fn model_context(
        &self,
        display_context: BlockModelDisplayContext,
    ) -> ItemModelContext<'a> {
        ItemModelContext {
            registry_access: Some((
                self.world.registry_access_revision(),
                self.world.registries(),
            )),
            item_profiles: Some(self.world.item_profiles().component_defaults_view()),
            registry_keys: self.registry_keys,
            context_dimension: self
                .world
                .level_info()
                .map(|level| level.dimension.as_str()),
            ..ItemModelContext::new(display_context)
        }
    }
}

/// The living entity holding the stack, when there is one.
#[derive(Clone, Copy, Default)]
pub(crate) struct ItemOwner<'a> {
    pub(crate) entity_type: Option<&'a str>,
    pub(crate) main_hand_left: Option<bool>,
    pub(crate) team_color_rgb: Option<u32>,
    pub(crate) using_item: bool,
    pub(crate) using_item_ticks: f32,
}

/// Where and how one baked stack is submitted.
#[derive(Clone, Copy)]
pub(crate) struct ItemPlacement {
    pub(crate) attach: Mat4,
    pub(crate) left_hand: bool,
    pub(crate) block_fallback: BlockModelDisplayTransform,
    pub(crate) generated_fallback: BlockModelDisplayTransform,
    pub(crate) light: [f32; 2],
    pub(crate) target: ItemModelDrawTarget,
    pub(crate) outline_color: Option<u32>,
}

impl ItemPlacement {
    /// First-person contexts draw at full brightness in the hand pass.
    pub(crate) fn attached(
        attach: Mat4,
        context: BlockModelDisplayContext,
        left_hand: bool,
        block_fallback: BlockModelDisplayTransform,
        generated_fallback: BlockModelDisplayTransform,
    ) -> Self {
        Self {
            attach,
            left_hand,
            block_fallback,
            generated_fallback,
            light: ITEM_MODEL_FULL_BRIGHT_LIGHT,
            target: if matches!(
                context,
                BlockModelDisplayContext::FirstPersonLeftHand
                    | BlockModelDisplayContext::FirstPersonRightHand
            ) {
                ItemModelDrawTarget::FirstPerson
            } else {
                ItemModelDrawTarget::World
            },
            outline_color: None,
        }
    }

    pub(crate) fn with_light(mut self, light: [f32; 2]) -> Self {
        self.light = light;
        self
    }

    /// Third-person entity equipment inherits `EntityRenderState.lightCoords`
    /// and `outlineColor`.
    pub(crate) fn attached_to_entity(
        instance: &EntityModelInstance,
        attach: Mat4,
        left_hand: bool,
        block_fallback: BlockModelDisplayTransform,
        generated_fallback: BlockModelDisplayTransform,
    ) -> Self {
        Self {
            attach,
            left_hand,
            block_fallback,
            generated_fallback,
            light: super::dropped::entity_render_state_shader_light(instance),
            target: ItemModelDrawTarget::World,
            outline_color: (instance.render_state.outline_color != 0)
                .then_some(instance.render_state.outline_color),
        }
    }
}

/// Vanilla `ItemModelResolver` reads use duration only for the owner's
/// active use item; crossbows also read the synced charge time.
fn item_use_context(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    stack: &ItemStackSummary,
    owner: &ItemOwner<'_>,
) -> ItemModelUseContext {
    if !owner.using_item {
        return ItemModelUseContext::inactive();
    }
    let elapsed = item_model_elapsed_ticks(owner.using_item_ticks);
    let is_crossbow = stack
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        .is_some_and(|resource_id| {
            item_types::vanilla_item_resource_id_matches_type(
                resource_id,
                item_types::VANILLA_ITEM_CROSSBOW_ID,
            )
        });
    if !is_crossbow {
        return item_runtime.item_model_use_context_for_stack(stack, elapsed);
    }
    let charge_duration = super::crossbow_charge_duration_ticks(
        item_runtime,
        world,
        &stack.component_patch.enchantments,
    );
    item_runtime.item_model_use_context_for_stack_with_crossbow_charge_duration(
        stack,
        elapsed,
        Some(charge_duration),
    )
}

/// Bakes one stack at an attach transform with the stack's retained display
/// transform for `display_context`, falling back to the parent-model default
/// of the block or generated path.
pub(crate) fn bake_item_stack(
    env: &ItemBakeEnv<'_>,
    stack: &ItemStackSummary,
    display_context: BlockModelDisplayContext,
    owner: &ItemOwner<'_>,
    placement: &ItemPlacement,
    residency: &mut ItemModelResidencyFrame,
) {
    let item_runtime = env.item_runtime;
    let effective_stack = env.world.effective_item_stack(stack);
    let stack = effective_stack.as_ref();
    let context = ItemModelContext {
        owner_main_hand_left: owner.main_hand_left,
        context_entity_type: owner.entity_type,
        owner_team_color_rgb: owner.team_color_rgb,
        using_item: owner.using_item,
        use_context: item_use_context(env.world, item_runtime, stack, owner),
        ..env.model_context(display_context)
    };
    let Some(geometry) = item_runtime.resolve_item_model(stack, &context) else {
        return;
    };
    let retained = item_runtime.item_display_transform_for_stack(stack, display_context);
    let foil = item_stack_foil_mode(stack, item_runtime, display_context);
    let place = |display: Option<BlockModelDisplayTransform>, fallback| {
        placement.attach
            * display_matrix(
                &display.or(retained).unwrap_or(fallback),
                placement.left_hand,
            )
    };
    let submit = ItemSubmit {
        light: placement.light,
        overlay: ITEM_MODEL_NO_OVERLAY,
        foil,
        target: placement.target,
        outline_color: placement.outline_color,
    };
    for layer in geometry.layers {
        match layer {
            ResolvedItemModelLayer::Cuboid {
                parts,
                transformation,
                display_transform,
                ..
            } => {
                submit.cuboid_parts(
                    residency,
                    &parts,
                    &[place(display_transform, placement.block_fallback)
                        * special::special_item_leaf_matrix(transformation)],
                );
            }
            ResolvedItemModelLayer::Ordinary {
                icon,
                transformation,
                display_transform,
            } => submit.flat_icon(
                residency,
                item_runtime,
                &icon,
                &[place(display_transform, placement.generated_fallback)
                    * special::special_item_leaf_matrix(transformation)],
            ),
            ResolvedItemModelLayer::Special {
                model,
                transformation,
                display_transform,
                ..
            } => {
                let Some(model) = special::project_special_item_model(
                    &model,
                    stack,
                    item_runtime,
                    Some(env.world),
                    env.variant_textures,
                ) else {
                    continue;
                };
                submit.special(
                    residency,
                    model,
                    place(display_transform, placement.generated_fallback)
                        * special::special_item_leaf_matrix(transformation),
                );
            }
        }
    }
}

/// Per-stack submission state shared by every leaf of one item.
#[derive(Clone, Copy)]
pub(crate) struct ItemSubmit {
    pub(crate) light: [f32; 2],
    pub(crate) overlay: [f32; 2],
    pub(crate) foil: ItemModelFoil,
    pub(crate) target: ItemModelDrawTarget,
    pub(crate) outline_color: Option<u32>,
}

impl ItemSubmit {
    pub(crate) fn cuboid_parts(
        &self,
        residency: &mut ItemModelResidencyFrame,
        parts: &[bbb_client_render::item_model::ResolvedItemCuboidPart],
        transforms: &[Mat4],
    ) {
        for part in parts {
            for geometry_class in [
                ItemModelGeometryClass::Solid,
                ItemModelGeometryClass::Translucent,
            ] {
                if !part.geometry.has_geometry_class(geometry_class) {
                    continue;
                }
                let identity =
                    ItemModelAssetIdentity::new(part.model_id.clone(), "cuboid", part.part);
                push_resident_item_instances(
                    residency,
                    identity.clone(),
                    ItemModelAtlasKind::Items,
                    geometry_class,
                    ItemModelAssetSource::Model(part.geometry.clone()),
                    transforms,
                    part.tint,
                    self.light,
                    self.overlay,
                    self.foil,
                    self.target,
                );
                if let Some(outline_color) = self.outline_color {
                    super::dropped::push_resident_item_outline_instances_for_asset(
                        residency,
                        identity,
                        ItemModelAtlasKind::Items,
                        geometry_class,
                        transforms,
                        outline_color,
                        self.light,
                        self.overlay,
                    );
                }
            }
        }
    }

    pub(crate) fn flat_icon(
        &self,
        residency: &mut ItemModelResidencyFrame,
        item_runtime: &NativeItemRuntime,
        icon: &ItemAtlasIcon,
        transforms: &[Mat4],
    ) {
        for layer in item_runtime.generated_item_layers_for_icon(icon) {
            let identity = ItemModelAssetIdentity::new(layer.asset_id, "generated", 0);
            let geometry_class = if layer.translucent {
                ItemModelGeometryClass::Translucent
            } else {
                ItemModelGeometryClass::Solid
            };
            push_resident_item_instances(
                residency,
                identity.clone(),
                ItemModelAtlasKind::Items,
                geometry_class,
                ItemModelAssetSource::Generated {
                    mask: layer.mask,
                    rect: layer.rect,
                    translucent: layer.translucent,
                },
                transforms,
                layer.tint,
                self.light,
                self.overlay,
                self.foil,
                self.target,
            );
            if let Some(outline_color) = self.outline_color {
                super::dropped::push_resident_item_outline_instances_for_asset(
                    residency,
                    identity,
                    ItemModelAtlasKind::Items,
                    geometry_class,
                    transforms,
                    outline_color,
                    self.light,
                    self.overlay,
                );
            }
        }
    }

    pub(crate) fn special(
        &self,
        residency: &mut ItemModelResidencyFrame,
        model: bbb_client_render::SpecialItemModel,
        transform: Mat4,
    ) {
        residency.special_items.push(SpecialItemInstanceRequest {
            model,
            transform,
            light: self.light,
            overlay: self.overlay,
            foil: self.foil,
            target: self.target,
            outline_color: self.outline_color,
            submit_order: u32::try_from(residency.instances.len() + residency.special_items.len())
                .expect("special item submit order overflow"),
            scissor: None,
        });
    }
}

fn item_model_elapsed_ticks(ticks: f32) -> u32 {
    if ticks.is_finite() {
        ticks.max(0.0).floor() as u32
    } else {
        0
    }
}
