use super::super::{CompassTarget, LocalTimeFormatterFn, TimeSource};

type TimeWobblerFn<'a> = &'a dyn Fn(&str, u64, TimeSource, i64, f32) -> f32;
type CompassWobblerFn<'a> = &'a dyn Fn(&str, u64, CompassTarget, i64, f32) -> f32;
type CompassNoTargetRotationFn<'a> = &'a dyn Fn(&str, u64, CompassTarget, bool, i64, i32) -> f32;

use super::*;

impl ItemIconModel {
    pub(crate) fn prepare_ground_extents(&mut self, inherited: Mat4) {
        match self {
            Self::Layers {
                display_transforms,
                cuboid: Some(parts),
                ..
            } => {
                // `cuboid::bake_geometry` already precomputes default ground extents for
                // `inherited == Mat4::IDENTITY`, so skip `Arc::make_mut` on shared leaves.
                if inherited == Mat4::IDENTITY {
                    return;
                }
                let display = display_transforms.get(BlockModelDisplayContext::Ground);
                let transform = super::super::item_display_matrix(&display, false) * inherited;
                for part in std::sync::Arc::make_mut(parts) {
                    part.ground_model_extents = part.geometry.transformed_extents(transform);
                }
            }
            Self::Transformed {
                transformation,
                model,
            } => {
                let local = Mat4::from_cols_array_2d(&transformation.columns);
                model.prepare_ground_extents(inherited * local);
            }
            Self::Condition {
                on_true, on_false, ..
            } => {
                on_true.prepare_ground_extents(inherited);
                on_false.prepare_ground_extents(inherited);
            }
            Self::RangeDispatch {
                entries, fallback, ..
            } => {
                for (_, model) in entries {
                    model.prepare_ground_extents(inherited);
                }
                fallback.prepare_ground_extents(inherited);
            }
            Self::Select {
                cases, fallback, ..
            } => {
                for (_, model) in cases {
                    model.prepare_ground_extents(inherited);
                }
                fallback.prepare_ground_extents(inherited);
            }
            Self::Composite(models) => {
                for model in models {
                    model.prepare_ground_extents(inherited);
                }
            }
            Self::Empty
            | Self::BundleSelectedItem
            | Self::Special { .. }
            | Self::Layers { cuboid: None, .. } => {}
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Layers { layers, .. } => layers.is_empty(),
            Self::Special { .. } => false,
            Self::Transformed { model, .. } => model.is_empty(),
            Self::BundleSelectedItem => false,
            Self::Condition {
                on_true, on_false, ..
            } => on_true.is_empty() && on_false.is_empty(),
            Self::RangeDispatch {
                entries, fallback, ..
            } => entries.iter().all(|(_, model)| model.is_empty()) && fallback.is_empty(),
            Self::Select {
                cases, fallback, ..
            } => cases.iter().all(|(_, model)| model.is_empty()) && fallback.is_empty(),
            Self::Composite(models) => models.iter().all(Self::is_empty),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ItemIconModel {
    Empty,
    Layers {
        layers: Vec<ItemIconTextureLayer>,
        display_transforms: BlockModelDisplayTransforms,
        cuboid: Option<std::sync::Arc<[super::super::cuboid::CuboidPart]>>,
    },
    Special {
        base: Box<ItemIconModel>,
        model: ItemSpecialModel,
    },
    Transformed {
        transformation: ItemModelLeafTransformation,
        model: Box<ItemIconModel>,
    },
    BundleSelectedItem,
    Condition {
        condition: ItemModelCondition,
        on_true: Box<ItemIconModel>,
        on_false: Box<ItemIconModel>,
    },
    RangeDispatch {
        property: ItemModelRangeProperty,
        state_id: Option<u64>,
        scale: f32,
        entries: Vec<(f32, Box<ItemIconModel>)>,
        fallback: Box<ItemIconModel>,
    },
    Select {
        property: ItemModelSelectProperty,
        cases: Vec<(SelectWhen, Box<ItemIconModel>)>,
        fallback: Box<ItemIconModel>,
    },
    Composite(Vec<ItemIconModel>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedItemIconModelLayer {
    Cuboid {
        layers: Vec<ItemIconTextureLayer>,
        parts: std::sync::Arc<[super::super::cuboid::CuboidPart]>,
        transformation: ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
    Ordinary {
        layers: Vec<ItemIconTextureLayer>,
        transformation: ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
    Special {
        base_layers: Vec<ItemIconTextureLayer>,
        model: ItemSpecialModel,
        transformation: ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
}

/// The per-stack inputs the GUI icon resolver projects from. Constant across one
/// item-model tree (a nested bundle template rebuilds its own context), so the
/// recursion threads it unchanged.
#[derive(Clone, Copy)]
pub(crate) struct IconResolveContext<'a> {
    pub protocol_id: i32,
    pub component_patch: Option<&'a DataComponentPatchSummary>,
    /// Active level registry owner used to swap placeholder-backed component
    /// select cases into the current holder generation.
    pub registry_access: Option<(
        RegistryAccessRevision,
        &'a dyn PersistentItemStackHolderResolver,
    )>,
    pub item_profiles: Option<ItemComponentDefaultsView<'a>>,
    /// Component values supplied by the item's registry prototype. Vanilla
    /// applies the stack patch over this prototype before evaluating item
    /// model component conditions.
    pub default_component_codec_values: Option<&'a BTreeMap<i32, serde_json::Value>>,
    pub stack_count: i32,
    pub default_max_stack_size: Option<i32>,
    pub default_max_damage: Option<i32>,
    pub bundle_selected_item_index: Option<i32>,
    /// Vanilla `IsSelected.get`: true only when the owner is the local player
    /// and this exact stack is `LocalPlayer.getInventory().getSelectedItem()`.
    pub selected_item: bool,
    /// Vanilla `IsCarried.get`: true only when the owner is the local player
    /// and this exact stack is `LocalPlayer.containerMenu.getCarried()`.
    pub carried_item: bool,
    /// Vanilla `IsViewEntity.get`: true when the item owner is the current
    /// camera entity, or the local player when there is no camera entity.
    pub view_entity: bool,
    /// Vanilla `ExtendedView.get`: true only for GUI item display context while
    /// either Shift key is held down.
    pub shift_down: bool,
    /// Vanilla `IsKeybindDown.get`: caller-projected `KeyMapping.isDown()` state
    /// for supported default key names.
    pub keybind_context: ItemModelKeybindContext,
    /// Vanilla `FishingRodCast.get`: true only when the player owner has a
    /// fishing hook and this exact stack is held by
    /// `FishingHookRenderer.getHoldingArm(player)`.
    pub fishing_rod_cast: bool,
    pub using_item: bool,
    pub use_context: ItemModelUseContext,
    /// Vanilla `Cooldown.get`: caller-projected
    /// `Player.getCooldowns().getCooldownPercent(itemStack, 0.0F)`, or `0.0`
    /// when there is no player owner / stack cooldown.
    pub cooldown_progress: f32,
    pub crossbow_charge: CrossbowChargeType,
    pub display_context: &'a str,
    pub display_context_kind: BlockModelDisplayContext,
    /// Vanilla `ItemModelResolver` seed forwarded to item-model properties.
    pub item_model_seed: i32,
    pub default_item_model_id: &'a str,
    /// Vanilla `MainHand.get`: `None` means this native call site has not
    /// threaded a `LivingEntity` owner, so select cases do not match and
    /// fallback is used. `Some(true)` is `HumanoidArm.LEFT`; `Some(false)` is
    /// RIGHT.
    pub main_hand_left: Option<bool>,
    /// Vanilla `ContextDimension.get`: `None` means this native call site has
    /// no `ClientLevel` context, so select cases do not match.
    pub context_dimension: Option<&'a str>,
    /// Vanilla `ContextEntityType.get`: `None` means this native call site has
    /// no owner entity, so select cases do not match.
    pub context_entity_type: Option<&'a str>,
    /// Vanilla item tint `TeamColor.calculate`: the living owner's scoreboard
    /// team formatting RGB. Non-living and ownerless paths leave this absent.
    pub owner_team_color_rgb: Option<u32>,
    /// Vanilla `LocalTime.get`: wall-clock millis used for the formatted date.
    pub local_time_epoch_millis: Option<i64>,
    pub local_time_formatter: Option<LocalTimeFormatterFn>,
    /// Vanilla `Time.get`: world clock values. `None` means this native call
    /// site has no `ClientLevel` context, so the property returns `0.0`.
    pub time_context: Option<ItemModelTimeContext>,
    /// Runtime-owned state for stateful item-model properties. The model id
    /// plus per-model state id approximates vanilla's baked-property identity.
    pub stateful_model_id: &'a str,
    pub time_wobbler: Option<TimeWobblerFn<'a>>,
    /// Random samples for `minecraft:time source=random`.
    pub time_random: Option<&'a dyn Fn() -> f32>,
    /// Vanilla `CompassAngle.get`: owner pose, level dimension, and known
    /// compass targets available to the GUI/HUD icon resolver.
    pub compass_context: Option<ItemModelCompassContext<'a>>,
    pub compass_wobbler: Option<CompassWobblerFn<'a>>,
    pub compass_no_target_rotation: Option<CompassNoTargetRotationFn<'a>>,
    /// Item registry keys by protocol id, used for vanilla `ItemPredicate.items`
    /// matching inside collection component predicates.
    pub item_resource_ids: Option<&'a [String]>,
    /// Modded item/block registry fallback for protocol IDs beyond vanilla.
    pub content_registry: Option<&'a bbb_protocol::content_registry::ContentRegistry>,
    /// `tags/item` catalog used for `#namespace:path` HolderSet entries in
    /// vanilla `ItemPredicate.items`.
    pub item_tags: Option<&'a TagCatalog>,
    /// `tags/enchantment` catalog used for `#namespace:path` HolderSet entries
    /// in vanilla `EnchantmentPredicate.enchantments`.
    pub enchantment_tags: Option<&'a TagCatalog>,
    /// `tags/trim_material` catalog used for `#namespace:path` HolderSet
    /// entries in vanilla `TrimPredicate.material`.
    pub trim_material_tags: Option<&'a TagCatalog>,
    /// `tags/trim_pattern` catalog used for `#namespace:path` HolderSet entries
    /// in vanilla `TrimPredicate.pattern`.
    pub trim_pattern_tags: Option<&'a TagCatalog>,
    /// `tags/jukebox_song` catalog used for `#namespace:path` HolderSet
    /// entries in vanilla `JukeboxPlayablePredicate.song`.
    pub jukebox_song_tags: Option<&'a TagCatalog>,
    /// `tags/potion` catalog used for `#namespace:path` HolderSet entries in
    /// vanilla `PotionsPredicate`.
    pub potion_tags: Option<&'a TagCatalog>,
    /// `tags/attribute` catalog used for `#namespace:path` HolderSet entries
    /// in vanilla `AttributeModifiersPredicate.EntryPredicate.attribute`.
    pub attribute_tags: Option<&'a TagCatalog>,
    /// `tags/villager_type` catalog used for `#namespace:path` HolderSet
    /// entries in vanilla `VillagerTypePredicate`.
    pub villager_type_tags: Option<&'a TagCatalog>,
    /// `minecraft:trim_material` registry keys by holder id (the dynamic
    /// registry, projected from `bbb-client-world` at the call site).
    pub trim_material_keys: Option<&'a [String]>,
    /// `minecraft:trim_pattern` registry keys by holder id (the dynamic
    /// registry, projected from `bbb-client-world` at the call site).
    pub trim_pattern_keys: Option<&'a [String]>,
    /// `minecraft:enchantment` registry keys by holder id (the dynamic
    /// registry, projected from `bbb-client-world` at the call site).
    pub enchantment_keys: Option<&'a [String]>,
    /// Built-in `minecraft:attribute` registry keys by holder id.
    pub attribute_keys: Option<&'a [String]>,
}

impl<'a> IconResolveContext<'a> {
    pub(super) fn component_source(self, type_id: i32) -> Option<&'a DataComponentPatchSummary> {
        let patch = self.component_patch?;
        Some(self.item_profiles.map_or(patch, |profiles| {
            profiles.component_source(Some(self.protocol_id), patch, type_id)
        }))
    }

    pub(super) fn effective_max_stack_size(self) -> i32 {
        effective_item_max_stack_size(self.component_patch, self.default_max_stack_size)
    }
}

pub(super) fn select_property_value(
    property: &ItemModelSelectProperty,
    ctx: IconResolveContext<'_>,
) -> Option<ItemModelScalar> {
    let string = |value: &str| ItemModelScalar::String(value.into());
    match property {
        ItemModelSelectProperty::DisplayContext => Some(string(ctx.display_context)),
        ItemModelSelectProperty::MainHand => ctx
            .main_hand_left
            .map(|left| string(if left { "left" } else { "right" })),
        ItemModelSelectProperty::ContextDimension => ctx.context_dimension.map(string),
        ItemModelSelectProperty::ContextEntityType => ctx.context_entity_type.map(string),
        ItemModelSelectProperty::LocalTime {
            pattern,
            locale,
            time_zone,
        } => ctx
            .local_time_epoch_millis
            .zip(ctx.local_time_formatter)
            .and_then(|(epoch_millis, formatter)| {
                formatter(epoch_millis, pattern, locale, time_zone.as_deref())
            })
            .map(|value| string(&value)),
        ItemModelSelectProperty::ChargeType => Some(string(ctx.crossbow_charge.when_name())),
        ItemModelSelectProperty::TrimMaterial => ctx
            .component_patch
            .and_then(|patch| patch.armor_trim_material_id)
            .and_then(|id| usize::try_from(id).ok())
            .and_then(|id| ctx.trim_material_keys.and_then(|keys| keys.get(id)))
            .map(|value| string(value)),
        ItemModelSelectProperty::BlockState { property } => ctx
            .component_patch
            .and_then(|patch| patch.block_state_properties.get(property))
            .map(|value| ItemModelScalar::String(value.clone())),
        ItemModelSelectProperty::CustomModelData { index } => ctx
            .component_patch
            .and_then(|patch| patch.custom_model_data_strings.get(*index))
            .map(|value| ItemModelScalar::String(value.clone())),
        ItemModelSelectProperty::Component { .. } | ItemModelSelectProperty::Unsupported(_) => None,
    }
}

pub(super) fn overworld_sun_angle(day_time: i64) -> f32 {
    ((day_time - 6_000) as f32 * 360.0 / 24_000.0).rem_euclid(360.0)
}

pub(super) fn moon_phase_index(day_time: i64) -> i64 {
    day_time.rem_euclid(24_000 * 8) / 24_000
}

pub(super) fn compass_rotation_to_target(
    ctx: IconResolveContext<'_>,
    compass: ItemModelCompassContext<'_>,
    target_pos: [i32; 3],
    wobble_state: Option<(CompassTarget, u64)>,
) -> Option<f32> {
    let target_x = f64::from(target_pos[0]) + 0.5;
    let target_z = f64::from(target_pos[2]) + 0.5;
    let dx = target_x - compass.owner_position[0];
    let dz = target_z - compass.owner_position[2];
    if dx * dx + dz * dz < 1.0e-5 {
        return None;
    }
    let angle_to_target = (dz.atan2(dx) / (std::f64::consts::PI * 2.0)) as f32;
    let owner_y_rotation = (compass.owner_y_rot_degrees / 360.0).rem_euclid(1.0);
    if let Some((target, state)) = wobble_state {
        let owner_rotation = 0.5 - (owner_y_rotation - 0.25);
        let rotation = ctx
            .compass_wobbler
            .map(|wobbler| {
                wobbler(
                    ctx.stateful_model_id,
                    state,
                    target,
                    compass.game_time,
                    owner_rotation,
                )
            })
            .unwrap_or(owner_rotation);
        Some((angle_to_target + rotation).rem_euclid(1.0))
    } else {
        Some((0.5 - (owner_y_rotation - 0.25 - angle_to_target)).rem_euclid(1.0))
    }
}

pub(super) fn lodestone_target_for_patch(
    patch: &DataComponentPatchSummary,
) -> Option<&bbb_protocol::packets::LodestoneTargetSummary> {
    if patch
        .removed_type_ids
        .contains(&component_id::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
    {
        return None;
    }
    patch.lodestone_target.as_ref()
}

impl ItemIconModel {
    pub(super) fn display_transform(
        &self,
        context: BlockModelDisplayContext,
    ) -> Option<BlockModelDisplayTransform> {
        match self {
            Self::Layers {
                display_transforms, ..
            } => Some(display_transforms.get(context)),
            Self::Transformed { model, .. } => model.display_transform(context),
            _ => None,
        }
    }

    pub(super) fn selected_branch(&self, ctx: IconResolveContext<'_>) -> Option<&ItemIconModel> {
        match self {
            Self::Condition {
                condition,
                on_true,
                on_false,
            } => Some(if condition_holds(condition, &ctx) {
                on_true
            } else {
                on_false
            }),
            Self::RangeDispatch {
                property,
                state_id,
                scale,
                entries,
                fallback,
            } => {
                let value = range_property_value(property, *state_id, ctx) * scale;
                Some(if value.is_nan() {
                    fallback
                } else {
                    match last_range_entry_at_or_below(entries, value) {
                        Some(index) => &entries[index].1,
                        None => fallback,
                    }
                })
            }
            Self::Select {
                property,
                cases,
                fallback,
            } => {
                let selected = select_property_value(property, ctx);
                Some(
                    cases
                        .iter()
                        .find(|(when, _)| match (when, property) {
                            (
                                SelectWhen::Component(case),
                                ItemModelSelectProperty::Component { type_id },
                            ) => component_select_matches(*type_id, case, ctx),
                            (SelectWhen::Values(values), _) => selected
                                .as_ref()
                                .is_some_and(|value| values.contains(value)),
                            _ => false,
                        })
                        .map(|(_, model)| model.as_ref())
                        .unwrap_or(fallback),
                )
            }
            _ => None,
        }
    }

    pub(crate) fn icon_layers_with_bundle_resolver(
        &self,
        ctx: IconResolveContext<'_>,
        resolve_bundle_selected_item: &mut impl FnMut() -> Vec<ItemIconTextureLayer>,
    ) -> Vec<ItemIconTextureLayer> {
        match self {
            Self::Empty => Vec::new(),
            Self::BundleSelectedItem => resolve_bundle_selected_item(),
            Self::Layers { layers, .. } => layers.clone(),
            Self::Special { base, .. } => {
                base.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
            }
            Self::Transformed { model, .. } => {
                model.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
            }
            Self::Condition { .. } | Self::RangeDispatch { .. } | Self::Select { .. } => self
                .selected_branch(ctx)
                .expect("branch model must select a child")
                .icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item),
            Self::Composite(models) => models
                .iter()
                .flat_map(|model| {
                    model.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
                })
                .collect(),
        }
    }

    pub(crate) fn resolved_layers_with_bundle_resolver(
        &self,
        ctx: IconResolveContext<'_>,
        inherited: Mat4,
        resolve_bundle_selected_item: &mut impl FnMut() -> Vec<ResolvedItemIconModelLayer>,
    ) -> Vec<ResolvedItemIconModelLayer> {
        match self {
            Self::Empty => Vec::new(),
            Self::BundleSelectedItem => resolve_bundle_selected_item(),
            Self::Layers {
                layers,
                display_transforms,
                cuboid: Some(parts),
            } => vec![ResolvedItemIconModelLayer::Cuboid {
                layers: layers.clone(),
                parts: parts.clone(),
                transformation: ItemModelLeafTransformation {
                    columns: inherited.to_cols_array_2d(),
                },
                display_transform: Some(display_transforms.get(ctx.display_context_kind)),
            }],
            Self::Layers {
                layers,
                display_transforms,
                cuboid: None,
            } => vec![ResolvedItemIconModelLayer::Ordinary {
                layers: layers.clone(),
                transformation: ItemModelLeafTransformation {
                    columns: inherited.to_cols_array_2d(),
                },
                display_transform: Some(display_transforms.get(ctx.display_context_kind)),
            }],
            Self::Special { base, model } => {
                let mut no_bundle_resolver = Vec::new;
                vec![ResolvedItemIconModelLayer::Special {
                    base_layers: base
                        .icon_layers_with_bundle_resolver(ctx, &mut no_bundle_resolver),
                    model: model.clone(),
                    transformation: ItemModelLeafTransformation {
                        columns: inherited.to_cols_array_2d(),
                    },
                    display_transform: base.display_transform(ctx.display_context_kind),
                }]
            }
            Self::Transformed {
                transformation,
                model,
            } => {
                let local = Mat4::from_cols_array_2d(&transformation.columns);
                model.resolved_layers_with_bundle_resolver(
                    ctx,
                    inherited * local,
                    resolve_bundle_selected_item,
                )
            }
            Self::Condition { .. } | Self::RangeDispatch { .. } | Self::Select { .. } => self
                .selected_branch(ctx)
                .expect("branch model must select a child")
                .resolved_layers_with_bundle_resolver(ctx, inherited, resolve_bundle_selected_item),
            Self::Composite(models) => models
                .iter()
                .flat_map(|model| {
                    model.resolved_layers_with_bundle_resolver(
                        ctx,
                        inherited,
                        resolve_bundle_selected_item,
                    )
                })
                .collect(),
        }
    }
}

/// Vanilla `ConditionalItemModelProperty.get` for the typed conditions.
fn condition_holds(condition: &ItemModelCondition, ctx: &IconResolveContext<'_>) -> bool {
    match condition {
        ItemModelCondition::Broken | ItemModelCondition::Damaged => ctx
            .component_patch
            .and_then(|patch| {
                ItemComponentDefaultsView::damage_state(
                    ctx.item_profiles,
                    Some(ctx.protocol_id),
                    patch,
                )
            })
            .is_some_and(|(damage, max_damage)| {
                if matches!(condition, ItemModelCondition::Broken) {
                    damage >= max_damage - 1
                } else {
                    damage > 0
                }
            }),
        ItemModelCondition::HasComponent {
            component,
            ignore_default,
        } => item_stack_has_component_id(
            *component,
            if *ignore_default {
                ctx.component_patch
            } else {
                ctx.component_source(*component)
            },
            ctx.default_component_codec_values,
            Some(ctx.default_item_model_id),
            *ignore_default,
        ),
        ItemModelCondition::BundleHasSelectedItem => {
            item_stack_has_selected_bundle_item(ctx.component_patch, ctx.bundle_selected_item_index)
        }
        ItemModelCondition::CustomModelData { index } => ctx.component_patch.is_some_and(|patch| {
            !patch
                .removed_type_ids
                .contains(&component_id::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID)
                && patch
                    .custom_model_data_flags
                    .get(*index)
                    .copied()
                    .unwrap_or(false)
        }),
        ItemModelCondition::Carried => ctx.carried_item,
        ItemModelCondition::Component(predicate) => {
            component_predicate_matches(predicate, &PredicateSubject::of_stack(ctx), ctx)
        }
        ItemModelCondition::Selected => ctx.selected_item,
        ItemModelCondition::UsingItem => ctx.using_item,
        ItemModelCondition::ViewEntity => ctx.view_entity,
        ItemModelCondition::ExtendedView => ctx.display_context == "gui" && ctx.shift_down,
        ItemModelCondition::KeybindDown { keybind } => ctx.keybind_context.keybind_down(keybind),
        ItemModelCondition::FishingRodCast => ctx.fishing_rod_cast,
        ItemModelCondition::Unsupported(_) => false,
    }
}

pub(crate) const MAX_ITEM_BAKED_GEOMETRY_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ITEM_MODEL_TOTAL_AST_NODES: usize = 262_144;
const ITEM_MODEL_AST_NODE_LOGICAL_BYTES: u64 = 64;

type CachedCuboidPartsEntry = (
    Vec<super::super::ItemIconTint>,
    std::sync::Arc<[super::super::cuboid::CuboidPart]>,
);

/// Cross-item baking cache shared across `NativeItemRuntime::from_loaded` so repeated
/// cuboid model references (including large `minecraft:composite` trees) share one
/// `Arc<[CuboidPart]>` and charge derived geometry + AST bytes against the pack budget.
pub(crate) struct CuboidBakeCache<'a> {
    raw_geometry:
        hashbrown::HashMap<String, std::sync::Arc<super::super::cuboid::BakedCuboidGeometry>>,
    parts_by_model: hashbrown::HashMap<String, Vec<CachedCuboidPartsEntry>>,
    retained_geometry_bytes: u64,
    total_ast_nodes: usize,
    stack: Option<&'a bbb_client_assets::PackResourceStack>,
}

impl<'a> CuboidBakeCache<'a> {
    pub(crate) fn new(stack: Option<&'a bbb_client_assets::PackResourceStack>) -> Self {
        Self {
            raw_geometry: hashbrown::HashMap::new(),
            parts_by_model: hashbrown::HashMap::new(),
            retained_geometry_bytes: 0,
            total_ast_nodes: 0,
            stack,
        }
    }

    fn admit_ast_node(&mut self) -> anyhow::Result<()> {
        self.total_ast_nodes = self.total_ast_nodes.saturating_add(1);
        if self.total_ast_nodes > MAX_ITEM_MODEL_TOTAL_AST_NODES {
            anyhow::bail!(
                "item model AST node count {} exceeds limit {MAX_ITEM_MODEL_TOTAL_AST_NODES}",
                self.total_ast_nodes
            );
        }
        if let Some(stack) = self.stack {
            stack.reserve_derived_resource_bytes(
                ITEM_MODEL_AST_NODE_LOGICAL_BYTES,
                "retained item icon model AST",
            )?;
        }
        Ok(())
    }

    fn bake_cuboid_parts(
        &mut self,
        model: &bbb_client_assets::ItemCuboidModel,
        tints: Option<&Vec<ItemTintSource>>,
        colormaps: Option<&TerrainColorMaps>,
        textures: &ItemTextureState,
    ) -> anyhow::Result<std::sync::Arc<[super::super::cuboid::CuboidPart]>> {
        let baked = if let Some(existing) = self.raw_geometry.get(&model.id) {
            std::sync::Arc::clone(existing)
        } else {
            let (baked, estimated_bytes) = super::super::cuboid::bake_geometry(model, textures);
            let added = u64::try_from(estimated_bytes).unwrap_or(u64::MAX);
            let next_total = self
                .retained_geometry_bytes
                .checked_add(added)
                .ok_or_else(|| anyhow::anyhow!("item cuboid baked geometry byte count overflow"))?;
            if next_total > MAX_ITEM_BAKED_GEOMETRY_BYTES {
                anyhow::bail!(
                    "item cuboid baked geometry {next_total} bytes exceeds {MAX_ITEM_BAKED_GEOMETRY_BYTES} byte budget"
                );
            }
            if let Some(stack) = self.stack {
                stack.reserve_derived_resource_bytes(added, "baked item cuboid geometry")?;
            }
            self.retained_geometry_bytes = next_total;
            self.raw_geometry
                .insert(model.id.clone(), std::sync::Arc::clone(&baked));
            baked
        };
        let (resolved_tints, parts) =
            super::super::cuboid::parts_from_baked_geometry(&baked, tints, colormaps);
        let variants = self.parts_by_model.entry(model.id.clone()).or_default();
        if let Some((_, existing)) = variants
            .iter()
            .find(|(cached_tints, _)| *cached_tints == resolved_tints)
        {
            return Ok(std::sync::Arc::clone(existing));
        }
        variants.push((resolved_tints, std::sync::Arc::clone(&parts)));
        Ok(parts)
    }

    #[cfg(test)]
    pub(crate) fn unique_baked_geometry_count(&self) -> usize {
        self.raw_geometry.len()
    }
}

pub(crate) fn item_icon_model_for_definition_cached(
    model: &ItemModelDefinition,
    cuboid_models: &ItemCuboidModelCatalog,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
    textures: &ItemTextureState,
    cache: &mut CuboidBakeCache<'_>,
) -> anyhow::Result<ItemIconModel> {
    let mut next_wobble_state = 0;
    let mut model = item_icon_model_for_definition_with_state(
        model,
        cuboid_models,
        model_tints,
        colormaps,
        textures,
        &mut next_wobble_state,
        cache,
    )?;
    model.prepare_ground_extents(Mat4::IDENTITY);
    Ok(model)
}

pub(super) fn item_icon_model_for_definition_with_state(
    model: &ItemModelDefinition,
    cuboid_models: &ItemCuboidModelCatalog,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
    textures: &ItemTextureState,
    next_wobble_state: &mut u64,
    cache: &mut CuboidBakeCache<'_>,
) -> anyhow::Result<ItemIconModel> {
    cache.admit_ast_node()?;
    let resolved = match model {
        ItemModelDefinition::Empty => ItemIconModel::Empty,
        ItemModelDefinition::BundleSelectedItem => ItemIconModel::BundleSelectedItem,
        ItemModelDefinition::Model { model, .. } => item_icon_model_for_model_id(
            model,
            cuboid_models,
            model_tints,
            colormaps,
            textures,
            cache,
        )?,
        ItemModelDefinition::Special { base, special, .. } => ItemIconModel::Special {
            base: Box::new(item_icon_model_for_model_id(
                base,
                cuboid_models,
                model_tints,
                colormaps,
                textures,
                cache,
            )?),
            model: special.clone(),
        },
        ItemModelDefinition::Condition {
            condition,
            on_true,
            on_false,
            ..
        } => {
            let on_true = item_icon_model_for_definition_with_state(
                on_true,
                cuboid_models,
                model_tints,
                colormaps,
                textures,
                next_wobble_state,
                cache,
            )?;
            let on_false = item_icon_model_for_definition_with_state(
                on_false,
                cuboid_models,
                model_tints,
                colormaps,
                textures,
                next_wobble_state,
                cache,
            )?;
            if !matches!(condition, ItemModelCondition::Unsupported(_)) {
                ItemIconModel::Condition {
                    condition: condition.clone(),
                    on_true: Box::new(on_true),
                    on_false: Box::new(on_false),
                }
            } else if !on_false.is_empty() {
                on_false
            } else {
                on_true
            }
        }
        ItemModelDefinition::RangeDispatch {
            property,
            scale,
            entries,
            fallback,
            ..
        } => {
            if !matches!(property, ItemModelRangeProperty::Unsupported(_)) {
                let state_id = range_state_id(property, next_wobble_state);
                let mut resolved_entries: Vec<(f32, Box<ItemIconModel>)> =
                    Vec::with_capacity(entries.len());
                for entry in entries {
                    resolved_entries.push((
                        entry.threshold,
                        Box::new(item_icon_model_for_definition_with_state(
                            &entry.model,
                            cuboid_models,
                            model_tints,
                            colormaps,
                            textures,
                            next_wobble_state,
                            cache,
                        )?),
                    ));
                }
                resolved_entries.sort_by(|a, b| a.0.total_cmp(&b.0));
                let fallback = match fallback.as_deref() {
                    Some(model) => item_icon_model_for_definition_with_state(
                        model,
                        cuboid_models,
                        model_tints,
                        colormaps,
                        textures,
                        next_wobble_state,
                        cache,
                    )?,
                    None => ItemIconModel::Empty,
                };
                ItemIconModel::RangeDispatch {
                    property: property.clone(),
                    state_id,
                    scale: *scale,
                    entries: resolved_entries,
                    fallback: Box::new(fallback),
                }
            } else {
                match fallback
                    .as_deref()
                    .or_else(|| entries.first().map(|entry| entry.model.as_ref()))
                {
                    Some(model) => item_icon_model_for_definition_with_state(
                        model,
                        cuboid_models,
                        model_tints,
                        colormaps,
                        textures,
                        next_wobble_state,
                        cache,
                    )?,
                    None => ItemIconModel::Empty,
                }
            }
        }
        ItemModelDefinition::Select {
            property,
            cases,
            fallback,
            ..
        } => {
            if !matches!(property, ItemModelSelectProperty::Unsupported(_)) {
                let mut resolved_cases: Vec<(SelectWhen, Box<ItemIconModel>)> =
                    Vec::with_capacity(cases.len());
                for case in cases {
                    resolved_cases.push((
                        SelectWhen::from_definition(&case.when, property),
                        Box::new(item_icon_model_for_definition_with_state(
                            &case.model,
                            cuboid_models,
                            model_tints,
                            colormaps,
                            textures,
                            next_wobble_state,
                            cache,
                        )?),
                    ));
                }
                let fallback = match fallback.as_deref() {
                    Some(model) => item_icon_model_for_definition_with_state(
                        model,
                        cuboid_models,
                        model_tints,
                        colormaps,
                        textures,
                        next_wobble_state,
                        cache,
                    )?,
                    None => ItemIconModel::Empty,
                };
                ItemIconModel::Select {
                    property: property.clone(),
                    cases: resolved_cases,
                    fallback: Box::new(fallback),
                }
            } else {
                match fallback
                    .as_deref()
                    .or_else(|| cases.first().map(|case| case.model.as_ref()))
                {
                    Some(model) => item_icon_model_for_definition_with_state(
                        model,
                        cuboid_models,
                        model_tints,
                        colormaps,
                        textures,
                        next_wobble_state,
                        cache,
                    )?,
                    None => ItemIconModel::Empty,
                }
            }
        }
        ItemModelDefinition::Composite { models, .. } => {
            let mut resolved_models = Vec::with_capacity(models.len());
            for model in models {
                resolved_models.push(item_icon_model_for_definition_with_state(
                    model,
                    cuboid_models,
                    model_tints,
                    colormaps,
                    textures,
                    next_wobble_state,
                    cache,
                )?);
            }
            ItemIconModel::Composite(resolved_models)
        }
    };
    let transformation = match model {
        ItemModelDefinition::Model { transformation, .. }
        | ItemModelDefinition::Condition { transformation, .. }
        | ItemModelDefinition::RangeDispatch { transformation, .. }
        | ItemModelDefinition::Select { transformation, .. }
        | ItemModelDefinition::Composite { transformation, .. }
        | ItemModelDefinition::Special { transformation, .. } => transformation.as_ref(),
        ItemModelDefinition::Empty | ItemModelDefinition::BundleSelectedItem => None,
    };
    Ok(match transformation {
        Some(transformation) => ItemIconModel::Transformed {
            transformation: ItemModelLeafTransformation {
                columns: transformation.to_cols_array_2d(),
            },
            model: Box::new(resolved),
        },
        None => resolved,
    })
}

pub(super) fn item_icon_model_for_model_id(
    model_id: &str,
    cuboid_models: &ItemCuboidModelCatalog,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
    textures: &ItemTextureState,
    cache: &mut CuboidBakeCache<'_>,
) -> anyhow::Result<ItemIconModel> {
    let Some(model) = cuboid_models.model(model_id) else {
        return Ok(ItemIconModel::Empty);
    };
    let generated = generated_icon_layers(&model, model_tints, colormaps, textures);
    let cuboid = if !model.generated {
        Some(cache.bake_cuboid_parts(&model, model_tints.get(&model.id), colormaps, textures)?)
    } else {
        None
    };
    Ok(ItemIconModel::Layers {
        cuboid,
        layers: generated
            .or_else(|| first_icon_layer(&model, textures).map(|texture| vec![texture]))
            .unwrap_or_default(),
        display_transforms: model.display_transforms.clone(),
    })
}

pub(super) fn item_stack_has_selected_bundle_item(
    component_patch: Option<&DataComponentPatchSummary>,
    selected_item_index: Option<i32>,
) -> bool {
    let Some(selected_item_index) =
        selected_item_index.and_then(|index| usize::try_from(index).ok())
    else {
        return false;
    };
    component_patch.is_some_and(|patch| selected_item_index < patch.bundle_contents_items.len())
}
