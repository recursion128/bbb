use std::cell::OnceCell;

use bbb_client_render::item_model::ItemModelTimeContext;

use super::*;

const ITEM_DURABILITY_BAR_MAX_WIDTH: i32 = 13;
// Vanilla `BundleItem.java:35-36`.
const BUNDLE_BAR_COLOR: [f32; 3] = [0.44, 0.53, 1.0];
const BUNDLE_FULL_BAR_COLOR: [f32; 3] = [1.0, 0.33, 0.33];
pub(super) fn registry_key_id(keys: Option<&[String]>, key: &str, fallback: i32) -> i32 {
    keys.and_then(|keys| keys.iter().position(|candidate| candidate == key))
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(fallback)
}

pub(super) fn local_player_fishing_rod_casts_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    selected_item: bool,
) -> bool {
    selected_item
        && world.local_player_fishing_bobber_id().is_some()
        && item.item_id.is_some_and(|item_id| {
            item_runtime
                .and_then(|runtime| runtime.item_resource_id(item_id))
                .is_some_and(|resource_id| resource_id == "minecraft:fishing_rod")
        })
}

/// GUI item-model inputs vanilla reads from the local player and level
/// (`Gui.extractSlot`, `AbstractContainerScreen` slots). Built once per HUD
/// builder and shared by every slot it projects.
pub(super) struct HudItemModelContext<'w> {
    pub(super) world: &'w WorldStore,
    pub(super) item_runtime: Option<&'w NativeItemRuntime>,
    registry_keys: ItemRegistryKeySet,
    owner_main_hand_left: Option<bool>,
    context_dimension: Option<&'w str>,
    time: OnceCell<Option<ItemModelTimeContext>>,
    compass: Option<bbb_client_render::item_model::ItemModelCompassContext<'w>>,
}

/// Per-slot selector state of one GUI stack.
#[derive(Clone, Copy, Default)]
pub(super) struct HudItemSlotState {
    pub(super) bundle_selected_item_index: Option<i32>,
    pub(super) using_item: bool,
    pub(super) selected_item: bool,
    pub(super) carried_item: bool,
    pub(super) fishing_rod_cast: bool,
    pub(super) shift_down: bool,
    pub(super) keybind: ItemModelKeybindContext,
    pub(super) seed: i32,
}

/// One slot's resolved GUI model. The flat icon and the 3D model are both
/// derived from this single resolution.
pub(super) struct HudItemProjection<'a> {
    context: &'a HudItemModelContext<'a>,
    item_runtime: &'a NativeItemRuntime,
    stack: std::borrow::Cow<'a, bbb_protocol::packets::ItemStackSummary>,
    resolved: Option<ResolvedItemModel>,
}

impl<'w> HudItemModelContext<'w> {
    pub(super) fn new(world: &'w WorldStore, item_runtime: Option<&'w NativeItemRuntime>) -> Self {
        Self::with_registry_keys(world, item_runtime, ItemRegistryKeySet::from_world(world))
    }

    pub(super) fn with_registry_keys(
        world: &'w WorldStore,
        item_runtime: Option<&'w NativeItemRuntime>,
        registry_keys: ItemRegistryKeySet,
    ) -> Self {
        let context_dimension = world.level_info().map(|level| level.dimension.as_str());
        let compass_spawn = world.local_player().default_spawn.as_ref().map(|spawn| {
            bbb_client_render::item_model::ItemModelCompassTarget {
                dimension: spawn.dimension.as_str(),
                pos: [spawn.pos.x, spawn.pos.y, spawn.pos.z],
            }
        });
        let compass_recovery = world.level_info().and_then(|level| {
            level.last_death_location.as_ref().map(|target| {
                bbb_client_render::item_model::ItemModelCompassTarget {
                    dimension: target.dimension.as_str(),
                    pos: [target.pos.x, target.pos.y, target.pos.z],
                }
            })
        });
        let compass = context_dimension.and_then(|level_dimension| {
            world.local_player_pose().map(|pose| {
                bbb_client_render::item_model::ItemModelCompassContext {
                    game_time: world.world_time().map(|time| time.game_time).unwrap_or(0),
                    level_dimension,
                    owner_position: [pose.position.x, pose.position.y, pose.position.z],
                    owner_y_rot_degrees: pose.y_rot,
                    spawn: compass_spawn,
                    recovery: compass_recovery,
                }
            })
        });
        Self {
            world,
            item_runtime,
            registry_keys,
            owner_main_hand_left: world.local_player_main_arm_left(),
            context_dimension,
            time: OnceCell::new(),
            compass,
        }
    }

    fn time(&self) -> Option<ItemModelTimeContext> {
        *self.time.get_or_init(|| {
            let world = self.world;
            let time = world.world_time()?;
            let environment_attributes = world
                .local_player_pose()
                .map_or_else(
                    || world.sampled_client_environment_attributes(),
                    |pose| {
                        world.sampled_client_environment_attributes_at(
                            [pose.position.x, pose.position.y, pose.position.z],
                            EnvironmentBiomeSampling::Direct,
                        )
                    },
                )
                .ok();
            let has_authoritative_dimension_type = world
                .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
                .is_some();
            Some(ItemModelTimeContext {
                game_time: time.game_time,
                day_time: time.day_time,
                sun_angle_degrees: environment_attributes
                    .as_ref()
                    .and_then(|attributes| attributes.sun_angle_degrees)
                    .or_else(|| has_authoritative_dimension_type.then_some(0.0)),
                moon_phase_index: environment_attributes
                    .as_ref()
                    .and_then(|attributes| attributes.moon_phase)
                    .map(client_moon_phase_index)
                    .or_else(|| has_authoritative_dimension_type.then_some(0)),
            })
        })
    }

    /// Resolves `item` as the local player's GUI stack. `owner_team_color_rgb`
    /// is `None` for fake stacks that have no owning player.
    pub(super) fn project<'a>(
        &'a self,
        item: &'a bbb_protocol::packets::ItemStackSummary,
        slot: HudItemSlotState,
        owner_team_color_rgb: Option<u32>,
    ) -> Option<HudItemProjection<'a>> {
        let item_runtime = self.item_runtime?;
        item.item_id?;
        let world = self.world;
        let stack = world.effective_item_stack(item);
        // Vanilla `Cooldown.get` uses `getCooldownPercent(itemStack, 0.0F)`;
        // the HUD overlay itself samples the render partial tick.
        let cooldown_progress =
            item_cooldown_percent_for_stack(world, Some(item_runtime), &stack, 0.0).unwrap_or(0.0);
        let context = bbb_client_render::item_model::ItemModelContext {
            registry_access: Some((world.registry_access_revision(), world.registries())),
            item_profiles: Some(world.item_profiles().component_defaults_view()),
            registry_keys: self.registry_keys.model_keys(),
            context_dimension: self.context_dimension,
            time: self.time(),
            compass: self.compass,
            owner_main_hand_left: self.owner_main_hand_left,
            context_entity_type: Some("minecraft:player"),
            owner_team_color_rgb,
            using_item: slot.using_item,
            use_context: hud_item_model_use_context(world, item_runtime, &stack, slot.using_item),
            cooldown_progress,
            bundle_selected_item_index: slot.bundle_selected_item_index,
            selected_item: slot.selected_item,
            carried_item: slot.carried_item,
            view_entity: true,
            shift_down: slot.shift_down,
            keybind: slot.keybind,
            fishing_rod_cast: slot.fishing_rod_cast,
            seed: slot.seed,
            ..bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            )
        };
        let resolved = item_runtime.resolve_item_model(&stack, &context);
        Some(HudItemProjection {
            context: self,
            item_runtime,
            stack,
            resolved,
        })
    }
}

pub(super) fn hud_item_icon(
    models: &HudItemModelContext<'_>,
    item: &bbb_protocol::packets::ItemStackSummary,
    slot: HudItemSlotState,
    owner_team_color_rgb: Option<u32>,
    partial_tick: f32,
) -> Option<HudItemIcon> {
    models
        .project(item, slot, owner_team_color_rgb)?
        .icon(partial_tick)
}

pub(super) fn hud_block_item_model(
    models: &HudItemModelContext<'_>,
    item: &bbb_protocol::packets::ItemStackSummary,
    slot: HudItemSlotState,
    owner_team_color_rgb: Option<u32>,
    terrain_textures: &TerrainTextureState,
) -> Option<HudBlockItemModel> {
    models
        .project(item, slot, owner_team_color_rgb)?
        .block_model(terrain_textures)
}

impl HudItemProjection<'_> {
    pub(super) fn icon(&self, partial_tick: f32) -> Option<HudItemIcon> {
        let icon = self.resolved.as_ref()?.flat_icon()?;
        let world = self.context.world;
        let item = self.stack.as_ref();
        Some(HudItemIcon {
            pop: 0.0,
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: icon
                .layers
                .into_iter()
                .map(|layer| {
                    HudIconLayer::new(
                        HudUvRect {
                            min: layer.uv.min,
                            max: layer.uv.max,
                        },
                        layer.tint,
                    )
                })
                .collect(),
            foil: hud_item_foil_for_stack(self.item_runtime, item),
            count_label: hud_item_count_label_for_stack(item),
            durability_bar: hud_item_durability_bar_for_stack(world, item),
            cooldown_progress: hud_item_cooldown_progress_for_stack(
                world,
                Some(self.item_runtime),
                item,
                partial_tick,
            ),
        })
    }

    /// Vanilla 3D inventory item rendering: special and block items draw
    /// their model with the `gui` display transform; flat items keep the 2D
    /// sprite (`None`).
    pub(super) fn block_model(
        &self,
        _terrain_textures: &TerrainTextureState,
    ) -> Option<HudBlockItemModel> {
        let item_runtime = self.item_runtime;
        let item = self.stack.as_ref();
        let gui = item_runtime
            .item_display_transform_for_stack(
                item,
                bbb_client_assets::BlockModelDisplayContext::Gui,
            )
            .unwrap_or_default();
        let resolved = self.resolved.clone()?;
        if !resolved.layers.iter().any(|layer| {
            matches!(
                layer,
                ResolvedItemModelLayer::Cuboid { .. } | ResolvedItemModelLayer::Special { .. }
            )
        }) {
            return None;
        }
        let layers = hud_special_item_layers(item_runtime, item, resolved.layers, gui);
        (!layers.is_empty()).then(|| HudBlockItemModel {
            layers,
            lighting: GuiItemLightingEntry::Items3d,
            foil: item.has_foil(),
        })
    }
}

fn hud_special_item_layers(
    item_runtime: &NativeItemRuntime,
    item: &bbb_protocol::packets::ItemStackSummary,
    layers: Vec<ResolvedItemModelLayer>,
    gui: bbb_client_assets::BlockModelDisplayTransform,
) -> Arc<[HudResidentItemLayer]> {
    let leaf = |display_transform: Option<bbb_client_assets::BlockModelDisplayTransform>,
                transformation| {
        crate::item_models::display_matrix(&display_transform.unwrap_or(gui), false)
            * crate::item_models::special_item_leaf_matrix(transformation)
    };
    let mut out = Vec::new();
    for layer in layers {
        match layer {
            ResolvedItemModelLayer::Cuboid {
                parts,
                transformation,
                display_transform,
                ..
            } => {
                let transform = leaf(display_transform, transformation);
                for part in parts.iter() {
                    for geometry_class in [
                        ItemModelGeometryClass::Solid,
                        ItemModelGeometryClass::Translucent,
                    ] {
                        if !part.geometry.has_geometry_class(geometry_class) {
                            continue;
                        }
                        out.push(HudResidentItemLayer::Geometry {
                            atlas: ItemModelAtlasKind::Items,
                            identity: ItemModelAssetIdentity::new(
                                part.model_id.clone(),
                                "cuboid",
                                part.part,
                            ),
                            geometry_class,
                            source: ItemModelAssetSource::Model(part.geometry.clone()),
                            transform,
                            tint: part.tint,
                        });
                    }
                }
            }
            ResolvedItemModelLayer::Ordinary {
                icon,
                transformation,
                display_transform,
            } => {
                let transform = leaf(display_transform, transformation);
                out.extend(
                    item_runtime
                        .generated_item_layers_for_icon(&icon)
                        .into_iter()
                        .map(|layer| HudResidentItemLayer::Geometry {
                            atlas: ItemModelAtlasKind::Items,
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
                            transform,
                            tint: layer.tint,
                        }),
                );
            }
            ResolvedItemModelLayer::Special {
                model,
                transformation,
                display_transform,
                ..
            } => {
                if let Some(model) = crate::item_models::project_special_item_model(
                    &model,
                    item,
                    item_runtime,
                    None,
                    None,
                ) {
                    out.push(HudResidentItemLayer::Special {
                        model,
                        transform: leaf(display_transform, transformation),
                    });
                }
            }
        }
    }
    out.into()
}

fn hud_item_model_use_context(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    item: &bbb_protocol::packets::ItemStackSummary,
    using_item: bool,
) -> bbb_client_render::item_model::ItemModelUseContext {
    let is_crossbow = item
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        .is_some_and(|resource_id| {
            bbb_protocol::item_types::vanilla_item_resource_id_matches_type(
                resource_id,
                bbb_protocol::item_types::VANILLA_ITEM_CROSSBOW_ID,
            )
        });
    if using_item && is_crossbow {
        let charge_duration = crate::item_models::crossbow_charge_duration_ticks(
            item_runtime,
            world,
            &item.component_patch.enchantments,
        );
        item_runtime.item_model_use_context_for_stack_with_crossbow_charge_duration(
            item,
            world.local_player().interaction.using_item_ticks,
            Some(charge_duration),
        )
    } else if using_item {
        item_runtime.item_model_use_context_for_stack(
            item,
            world.local_player().interaction.using_item_ticks,
        )
    } else {
        bbb_client_render::item_model::ItemModelUseContext::inactive()
    }
}

pub(super) fn hud_item_foil_for_stack(
    item_runtime: &NativeItemRuntime,
    item: &bbb_protocol::packets::ItemStackSummary,
) -> HudItemFoil {
    if !item.has_foil() {
        return HudItemFoil::None;
    }
    if item_runtime.item_stack_uses_special_foil_texture(item) {
        HudItemFoil::Special
    } else {
        HudItemFoil::Standard
    }
}

pub(super) fn hud_item_count_label_for_stack(
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<HudItemCountLabel> {
    (!item_stack_is_empty(item) && item.count != 1)
        .then(|| HudItemCountLabel::new(item.count.to_string()))
}

pub(super) fn item_stack_is_empty(item: &bbb_protocol::packets::ItemStackSummary) -> bool {
    item.item_id.is_none() || item.count <= 0
}

pub(super) fn hud_item_cooldown_progress_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    partial_tick: f32,
) -> Option<f32> {
    let progress = item_cooldown_percent_for_stack(world, item_runtime, item, partial_tick)?;
    (progress > 0.0).then_some(progress)
}

fn item_cooldown_percent_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    partial_tick: f32,
) -> Option<f32> {
    let cooldown_group = item_cooldown_group(item_runtime, item)?;
    Some(world.item_cooldown_percent(&cooldown_group, partial_tick))
}

pub(super) fn item_cooldown_group(
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<String> {
    if item_stack_is_empty(item) {
        return None;
    }
    if let Some(group) = item.component_patch.use_cooldown_group.as_ref() {
        return Some(group.clone());
    }
    let item_id = item_runtime?.item_resource_id_for_protocol_id(item.item_id?)?;
    Some(item_id.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BundleWeightFraction {
    num: i128,
    den: i128,
}

impl BundleWeightFraction {
    const ZERO: Self = Self { num: 0, den: 1 };
    const ONE: Self = Self { num: 1, den: 1 };

    fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let mut a = num.unsigned_abs();
        let mut b = den.unsigned_abs();
        while b != 0 {
            let next = a % b;
            a = b;
            b = next;
        }
        let divisor = i128::try_from(a.max(1)).ok()?;
        Some(Self {
            num: num / divisor,
            den: den / divisor,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        Self::new(
            self.num
                .checked_mul(other.den)?
                .checked_add(other.num.checked_mul(self.den)?)?,
            self.den.checked_mul(other.den)?,
        )
    }

    fn times(self, count: i32) -> Option<Self> {
        Self::new(self.num.checked_mul(i128::from(count))?, self.den)
    }
}

pub(super) fn is_bundle_item_stack(stack: &bbb_protocol::packets::ItemStackSummary) -> bool {
    !item_stack_is_empty(stack)
        && stack.item_id.is_some_and(|item_id| {
            bbb_protocol::item_types::vanilla_item_type(item_id)
                .and_then(|entry| entry.use_override)
                == Some(bbb_protocol::item_types::VanillaItemUseOverride::BundleItem)
        })
}

pub(super) fn effective_bundle_contents(
    stack: &bbb_protocol::packets::ItemStackSummary,
) -> Option<&[bbb_protocol::packets::ItemStackTemplateSummary]> {
    if stack.component_patch.removed_type_ids.contains(
        &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    ) {
        return None;
    }
    if let Some(contents) = stack.component_patch.explicit_bundle_contents() {
        return Some(contents);
    }
    is_bundle_item_stack(stack).then_some(&[])
}

/// Vanilla `BundleContents.computeContentWeight` (`BundleContents.java:50-77`).
pub(super) fn bundle_contents_weight_fraction(
    world: &WorldStore,
    items: &[bbb_protocol::packets::ItemStackTemplateSummary],
) -> Option<(i128, i128)> {
    let fraction = bundle_contents_weight_fraction_inner(world, items, 0)?;
    Some((fraction.num, fraction.den))
}

fn bundle_contents_weight_fraction_inner(
    world: &WorldStore,
    items: &[bbb_protocol::packets::ItemStackTemplateSummary],
    depth: usize,
) -> Option<BundleWeightFraction> {
    if depth > bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        return None;
    }
    let profiles = world.item_profiles();
    items
        .iter()
        .try_fold(BundleWeightFraction::ZERO, |weight, item| {
            let stack = bbb_protocol::packets::ItemStackSummary {
                item_id: Some(item.item_id),
                count: item.count,
                component_patch: item.component_patch.clone(),
            };
            let effective = world.effective_item_stack(&stack);
            let item_weight =
                if let Some(nested) = effective_bundle_contents(effective.as_ref()) {
                    bundle_contents_weight_fraction_inner(world, nested, depth + 1)?
                        .add(BundleWeightFraction::new(1, 16)?)?
                } else if !effective.component_patch.removed_type_ids.contains(
                    &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
                ) && effective.component_patch.bees_present
                    && !effective.component_patch.bees.is_empty()
                {
                    BundleWeightFraction::ONE
                } else {
                    let max_stack = profiles.item_stack_max_stack_size(effective.as_ref());
                    if max_stack <= 0 {
                        return None;
                    }
                    BundleWeightFraction::new(1, i128::from(max_stack))?
                };
            weight.add(item_weight.times(item.count.max(0))?)
        })
}

pub(super) fn hud_item_durability_bar_for_stack(
    world: &WorldStore,
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<HudItemDurabilityBar> {
    if item_stack_is_empty(item) {
        return None;
    }
    if let Some((damage, max_damage)) = bbb_client_world::ItemProfiles::damage_state(
        Some(world.item_profiles()),
        item.item_id,
        &item.component_patch,
    ) {
        if damage > 0 {
            let width = (ITEM_DURABILITY_BAR_MAX_WIDTH as f32
                - damage as f32 * ITEM_DURABILITY_BAR_MAX_WIDTH as f32 / max_damage as f32)
                .round() as i32;
            let health_percentage = ((max_damage - damage) as f32 / max_damage as f32).max(0.0);
            return Some(HudItemDurabilityBar::new(
                width.clamp(0, ITEM_DURABILITY_BAR_MAX_WIDTH) as u32,
                vanilla_hsv_to_rgb_unit(health_percentage / 3.0, 1.0, 1.0),
            ));
        }
    }

    // Vanilla `BundleItem.isBarVisible/getBarWidth/getBarColor` (`BundleItem.java:153-168`).
    if is_bundle_item_stack(item) {
        let effective = world.effective_item_stack(item);
        let contents = effective_bundle_contents(effective.as_ref())?;
        let (num, den) = bundle_contents_weight_fraction(world, contents).unwrap_or((1, 1));
        if num <= 0 || den <= 0 {
            return None;
        }
        let width = 1 + num.saturating_mul(12).div_euclid(den);
        let color = if num >= den {
            BUNDLE_FULL_BAR_COLOR
        } else {
            BUNDLE_BAR_COLOR
        };
        return Some(HudItemDurabilityBar::new(
            width.clamp(0, i128::from(ITEM_DURABILITY_BAR_MAX_WIDTH)) as u32,
            color,
        ));
    }

    None
}

pub(super) fn vanilla_hsv_to_rgb_unit(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let h = ((hue * 6.0) as i32) % 6;
    let f = hue * 6.0 - h as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (red, green, blue) = match h {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        5 => (value, p, q),
        _ => (value, t, p),
    };
    [
        vanilla_hsv_color_component(red),
        vanilla_hsv_color_component(green),
        vanilla_hsv_color_component(blue),
    ]
}

fn vanilla_hsv_color_component(component: f32) -> f32 {
    ((component * 255.0) as i32).clamp(0, 255) as f32 / 255.0
}
