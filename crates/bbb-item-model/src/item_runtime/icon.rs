use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ItemTimeWobblerKey {
    item_model_id: String,
    state_id: u64,
    source: TimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ItemTimeRandomKey {
    item_model_id: String,
    state_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ItemCompassWobblerKey {
    item_model_id: String,
    state_id: u64,
    target: CompassTarget,
    no_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ItemCompassRandomKey {
    item_model_id: String,
    state_id: u64,
    target: CompassTarget,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ItemNeedleWobbler {
    rotation: f32,
    delta_rotation: f32,
    last_update_tick: Option<i64>,
}

impl ItemNeedleWobbler {
    fn should_update(&self, tick: i64) -> bool {
        self.last_update_tick != Some(tick)
    }

    fn update(&mut self, tick: i64, target_rotation: f32, factor: f32) -> f32 {
        if self.should_update(tick) {
            self.last_update_tick = Some(tick);
            let temp_delta_rotation = (target_rotation - self.rotation + 0.5).rem_euclid(1.0) - 0.5;
            self.delta_rotation += temp_delta_rotation * 0.1;
            self.delta_rotation *= factor;
            self.rotation = (self.rotation + self.delta_rotation).rem_euclid(1.0);
        }
        self.rotation
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ItemLegacyRandom {
    seed: u64,
}

impl ItemLegacyRandom {
    const MASK: u64 = (1u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;

    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1u32 << 24) as f32
    }
}

pub(super) fn item_time_random_seed(item_model_id: &str, state_id: u64) -> i64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in item_model_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    (hash ^ state_id.wrapping_mul(0x9e37_79b9_7f4a_7c15)) as i64
}

pub(super) fn item_compass_random_seed(
    item_model_id: &str,
    state_id: u64,
    target: CompassTarget,
) -> i64 {
    let target_hash = match target {
        CompassTarget::None => 0x2d4f_51c3_4a2f_0b11,
        CompassTarget::Lodestone => 0x45b9_4f80_637d_94f3,
        CompassTarget::Recovery => 0xa6c8_6fd7_4a99_79d5,
        CompassTarget::Spawn => 0x7f4a_7c15_9e37_79b9,
    };
    let seed_hash = item_time_random_seed(item_model_id, state_id) as u64;
    (seed_hash ^ target_hash) as i64
}

pub(super) fn compass_seed_offset(seed: i32) -> f32 {
    seed.wrapping_mul(1_327_217_883) as f32 / 2_147_483_648.0
}

pub(super) fn item_model_id_for_stack<'a>(
    item_id: &'a str,
    component_patch: Option<&'a DataComponentPatchSummary>,
) -> Option<&'a str> {
    if component_patch.is_some_and(|patch| {
        patch
            .removed_type_ids
            .contains(&VANILLA_ITEM_MODEL_COMPONENT_ID)
    }) {
        return None;
    }
    component_patch
        .and_then(|patch| patch.item_model.as_deref())
        .or(Some(item_id))
}

pub(super) fn item_display_context_name(context: BlockModelDisplayContext) -> &'static str {
    match context {
        BlockModelDisplayContext::ThirdPersonLeftHand => "thirdperson_lefthand",
        BlockModelDisplayContext::ThirdPersonRightHand => "thirdperson_righthand",
        BlockModelDisplayContext::FirstPersonLeftHand => "firstperson_lefthand",
        BlockModelDisplayContext::FirstPersonRightHand => "firstperson_righthand",
        BlockModelDisplayContext::Head => "head",
        BlockModelDisplayContext::Gui => "gui",
        BlockModelDisplayContext::Ground => "ground",
        BlockModelDisplayContext::Fixed => "fixed",
        BlockModelDisplayContext::OnShelf => "on_shelf",
    }
}

pub(super) fn item_stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

pub(super) fn item_use_duration_ticks(
    item_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> i32 {
    match item_id {
        BOW_ITEM_ID | CROSSBOW_ITEM_ID | TRIDENT_ITEM_ID => return VANILLA_LONG_USE_DURATION_TICKS,
        BRUSH_ITEM_ID => return VANILLA_BRUSH_USE_DURATION_TICKS,
        SPYGLASS_ITEM_ID => return VANILLA_SPYGLASS_USE_DURATION_TICKS,
        ENDER_EYE_ITEM_ID => return VANILLA_ENDER_EYE_USE_DURATION_TICKS,
        _ => {}
    }
    if let Some(consumable) = component_patch.consumable {
        return consumable_use_duration_ticks(consumable);
    }
    if component_patch
        .added_type_ids
        .contains(&VANILLA_BLOCKS_ATTACKS_COMPONENT_ID)
        || component_patch
            .added_type_ids
            .contains(&VANILLA_KINETIC_WEAPON_COMPONENT_ID)
    {
        return VANILLA_LONG_USE_DURATION_TICKS;
    }
    0
}

pub(super) fn consumable_use_duration_ticks(consumable: ConsumableSummary) -> i32 {
    if !consumable.consume_seconds.is_finite() || consumable.consume_seconds <= 0.0 {
        return 0;
    }
    (consumable.consume_seconds * 20.0).min(i32::MAX as f32) as i32
}

pub(super) fn crossbow_charge_duration_ticks(
    item_id: &str,
    component_patch: &DataComponentPatchSummary,
    enchantment_keys: Option<&[String]>,
) -> Option<i32> {
    if item_id != CROSSBOW_ITEM_ID {
        return None;
    }
    let quick_charge_level = enchantment_keys
        .map(|keys| {
            component_patch
                .enchantments
                .iter()
                .filter(|enchantment| {
                    usize::try_from(enchantment.holder_id)
                        .ok()
                        .and_then(|id| keys.get(id))
                        .is_some_and(|key| key == QUICK_CHARGE_ENCHANTMENT_ID)
                })
                .map(|enchantment| enchantment.level.max(0))
                .sum::<i32>()
        })
        .unwrap_or(0);
    if quick_charge_level == 0 {
        return Some(VANILLA_CROSSBOW_CHARGE_DURATION_TICKS);
    }
    let duration_seconds = (1.25 - 0.25 * quick_charge_level as f32).max(0.0);
    Some((duration_seconds * 20.0).floor() as i32)
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ItemIconTextureLayer {
    pub(super) texture_index: u32,
    pub(super) tint: ItemIconTint,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ItemIconTextureRef {
    pub(super) texture_id: String,
    pub(super) tint: ItemIconTint,
}

#[derive(Debug, Clone)]
pub(super) struct ItemTextureState {
    pub(super) atlas: AtlasImage,
    pub(super) texture_indices: HashMap<String, u32>,
    pub(super) fallback_index: u32,
}

impl ItemTextureState {
    pub(super) fn from_images(images: Vec<SpriteImage>) -> Result<Self> {
        let packer = AtlasPacker::new(ITEM_ATLAS_MAX_WIDTH, 1)?;
        let atlas = packer.stitch(&images)?;
        let mut texture_indices = HashMap::new();
        for (index, sprite) in atlas.layout.sprites.iter().enumerate() {
            texture_indices.insert(sprite.id.clone(), index as u32);
        }
        let fallback_index = texture_indices
            .get(MISSING_TEXTURE_ID)
            .copied()
            .unwrap_or(0);
        Ok(Self {
            atlas,
            texture_indices,
            fallback_index,
        })
    }

    pub(super) fn texture_count(&self) -> usize {
        self.atlas.layout.sprites.len()
    }

    pub(super) fn atlas_size(&self) -> (u32, u32) {
        (self.atlas.layout.width, self.atlas.layout.height)
    }

    pub(super) fn atlas_rgba(&self) -> &[u8] {
        &self.atlas.rgba
    }

    pub(super) fn sprite_uvs(&self) -> Vec<ItemAtlasSpriteUv> {
        self.atlas
            .layout
            .sprites
            .iter()
            .map(|sprite| ItemAtlasSpriteUv {
                id: sprite.id.clone(),
                uv: item_uv_rect(&self.atlas.layout, sprite),
                has_translucent: sprite.transparency.has_translucent,
            })
            .collect()
    }

    pub(super) fn fallback_index(&self) -> u32 {
        self.fallback_index
    }

    pub(super) fn texture_index(&self, texture_id: &str) -> u32 {
        self.texture_indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    pub(super) fn texture_id(&self, texture_index: u32) -> Option<&str> {
        self.atlas
            .layout
            .sprites
            .get(texture_index as usize)
            .map(|sprite| sprite.id.as_str())
    }

    pub(super) fn texture_uv_rect(&self, texture_index: u32) -> Option<ItemAtlasUvRect> {
        let sprite = self.atlas.layout.sprites.get(texture_index as usize)?;
        Some(item_uv_rect(&self.atlas.layout, sprite))
    }

    /// Builds the per-pixel alpha silhouette of the sprite a UV rect covers, for generated-item
    /// extrusion. Inverts the half-texel inset [`item_uv_rect`] applies to recover the exact content
    /// pixel bounds, then reads the stitched atlas alpha (vanilla `SpriteContents.isTransparent`: a pixel
    /// is opaque iff its alpha byte is non-zero).
    pub(super) fn alpha_mask_for_uv(&self, uv: ItemAtlasUvRect) -> Option<SpriteAlphaMask> {
        let (atlas_width, atlas_height) = self.atlas_size();
        let width = atlas_width as f32;
        let height = atlas_height as f32;
        let x0 = (uv.min[0] * width - 0.5).round() as i64;
        let x1 = (uv.max[0] * width + 0.5).round() as i64;
        let y0 = (uv.min[1] * height - 0.5).round() as i64;
        let y1 = (uv.max[1] * height + 0.5).round() as i64;
        if x0 < 0 || y0 < 0 || x1 <= x0 || y1 <= y0 {
            return None;
        }
        if x1 as u32 > atlas_width || y1 as u32 > atlas_height {
            return None;
        }
        let mask_width = (x1 - x0) as u32;
        let mask_height = (y1 - y0) as u32;
        let rgba = self.atlas_rgba();
        let mut opaque = Vec::with_capacity((mask_width * mask_height) as usize);
        for py in 0..mask_height {
            for px in 0..mask_width {
                let ax = x0 as u32 + px;
                let ay = y0 as u32 + py;
                let alpha_index = ((ay * atlas_width + ax) * 4 + 3) as usize;
                opaque.push(rgba.get(alpha_index).copied().unwrap_or(0) != 0);
            }
        }
        Some(SpriteAlphaMask::new(mask_width, mask_height, opaque))
    }
}

pub(super) fn item_icon_texture_layers(
    models: &ItemCuboidModelSet,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> Vec<ItemIconTextureRef> {
    models
        .models
        .iter()
        .find_map(|model| generated_layer_texture_refs(model, model_tints, colormaps))
        .or_else(|| {
            models
                .models
                .iter()
                .find_map(first_texture_id)
                .map(|texture_id| {
                    vec![ItemIconTextureRef {
                        texture_id,
                        tint: ItemIconTint::Static(ITEM_TINT_WHITE),
                    }]
                })
        })
        .unwrap_or_default()
}

pub(super) fn generated_layer_texture_refs(
    model: &ItemCuboidModel,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> Option<Vec<ItemIconTextureRef>> {
    let tints = model_tints.get(&model.id);
    let mut layers = Vec::new();
    for layer_index in 0..ITEM_GENERATED_MAX_LAYERS {
        let Some(texture) = model.texture_slots.get(&format!("layer{layer_index}")) else {
            break;
        };
        layers.push(ItemIconTextureRef {
            texture_id: texture.id.clone(),
            tint: tints
                .and_then(|tints| tints.get(layer_index))
                .map(|tint| item_tint_source(tint, colormaps))
                .unwrap_or(ItemIconTint::Static(ITEM_TINT_WHITE)),
        });
    }
    (!layers.is_empty()).then_some(layers)
}

pub(super) fn first_texture_id(model: &ItemCuboidModel) -> Option<String> {
    model
        .texture_slots
        .values()
        .next()
        .map(|texture| texture.id.clone())
        .or_else(|| {
            model
                .face_textures
                .as_ref()
                .map(|textures| textures.textures[0].clone())
        })
}

pub(super) fn model_tints_for_definition(
    model: &ItemModelDefinition,
) -> HashMap<String, Vec<ItemTintSource>> {
    let mut tints = HashMap::new();
    collect_model_tints(model, &mut tints);
    tints
}

pub(super) fn collect_model_tints(
    model: &ItemModelDefinition,
    tints_by_model: &mut HashMap<String, Vec<ItemTintSource>>,
) {
    match model {
        ItemModelDefinition::Empty | ItemModelDefinition::BundleSelectedItem => {}
        ItemModelDefinition::Model { model, tints, .. } => {
            tints_by_model
                .entry(model.clone())
                .or_insert_with(|| tints.clone());
        }
        ItemModelDefinition::Condition {
            on_true, on_false, ..
        } => {
            collect_model_tints(on_true, tints_by_model);
            collect_model_tints(on_false, tints_by_model);
        }
        ItemModelDefinition::RangeDispatch {
            entries, fallback, ..
        } => {
            for entry in entries {
                collect_model_tints(&entry.model, tints_by_model);
            }
            if let Some(fallback) = fallback {
                collect_model_tints(fallback, tints_by_model);
            }
        }
        ItemModelDefinition::Select {
            cases, fallback, ..
        } => {
            for case in cases {
                collect_model_tints(&case.model, tints_by_model);
            }
            if let Some(fallback) = fallback {
                collect_model_tints(fallback, tints_by_model);
            }
        }
        ItemModelDefinition::Composite { models, .. } => {
            for model in models {
                collect_model_tints(model, tints_by_model);
            }
        }
        ItemModelDefinition::Special { base, .. } => {
            tints_by_model.entry(base.clone()).or_default();
        }
    }
}

pub(super) fn item_tint_source_default_color(
    tint: &ItemTintSource,
    colormaps: Option<&TerrainColorMaps>,
) -> [f32; 4] {
    match tint {
        ItemTintSource::CustomModelData { default_color, .. }
        | ItemTintSource::Dye { default_color }
        | ItemTintSource::Firework { default_color }
        | ItemTintSource::Potion { default_color }
        | ItemTintSource::MapColor { default_color }
        | ItemTintSource::Team { default_color } => rgb_i32_tint(*default_color),
        ItemTintSource::Constant { value } => rgb_i32_tint(*value),
        ItemTintSource::Grass {
            temperature,
            downfall,
        } => colormaps
            .map(|colormaps| {
                rgb_u8_tint(
                    colormaps
                        .grass
                        .sample_temperature_downfall(*temperature, *downfall),
                )
            })
            .unwrap_or_else(|| rgb_u8_tint([0x91, 0xbd, 0x59])),
    }
}

pub(super) fn item_tint_source(
    tint: &ItemTintSource,
    colormaps: Option<&TerrainColorMaps>,
) -> ItemIconTint {
    match tint {
        ItemTintSource::Constant { .. } | ItemTintSource::Grass { .. } => {
            ItemIconTint::Static(item_tint_source_default_color(tint, colormaps))
        }
        ItemTintSource::CustomModelData { .. }
        | ItemTintSource::Dye { .. }
        | ItemTintSource::Firework { .. }
        | ItemTintSource::Potion { .. }
        | ItemTintSource::MapColor { .. }
        | ItemTintSource::Team { .. } => ItemIconTint::Source(tint.clone()),
    }
}

pub(super) fn item_icon_tint_color(
    tint: &ItemIconTint,
    component_patch: Option<&DataComponentPatchSummary>,
) -> [f32; 4] {
    match tint {
        ItemIconTint::Static(color) => *color,
        ItemIconTint::Source(source) => item_tint_source_color(source, component_patch),
    }
}

pub(super) fn resolve_item_icon_texture_layer_tints(
    layers: Vec<ItemIconTextureLayer>,
    component_patch: Option<&DataComponentPatchSummary>,
) -> Vec<ItemIconTextureLayer> {
    layers
        .into_iter()
        .map(|layer| ItemIconTextureLayer {
            texture_index: layer.texture_index,
            tint: ItemIconTint::Static(item_icon_tint_color(&layer.tint, component_patch)),
        })
        .collect()
}

pub(super) fn item_tint_source_color(
    tint: &ItemTintSource,
    component_patch: Option<&DataComponentPatchSummary>,
) -> [f32; 4] {
    match tint {
        ItemTintSource::CustomModelData {
            index,
            default_color,
        } => {
            let color = component_patch
                .and_then(|patch| patch.custom_model_data_colors.get(*index as usize))
                .copied()
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Dye { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.dyed_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::MapColor { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.map_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Potion { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.potion_custom_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Firework { default_color } => {
            let color = component_patch
                .and_then(|patch| firework_explosion_tint_color(&patch.firework_explosion_colors))
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Constant { value } => rgb_i32_tint(*value),
        ItemTintSource::Grass { .. } | ItemTintSource::Team { .. } => {
            item_tint_source_default_color(tint, None)
        }
    }
}

pub(super) fn firework_explosion_tint_color(colors: &[i32]) -> Option<i32> {
    if colors.is_empty() {
        return None;
    }
    if colors.len() == 1 {
        return Some(colors[0]);
    }

    let mut red = 0u32;
    let mut green = 0u32;
    let mut blue = 0u32;
    for color in colors {
        let color = *color as u32;
        red += (color >> 16) & 0xff;
        green += (color >> 8) & 0xff;
        blue += color & 0xff;
    }
    let len = colors.len() as u32;
    Some(((red / len) << 16 | (green / len) << 8 | (blue / len)) as i32)
}

impl NativeItemRuntime {
    #[cfg(test)]
    pub(crate) fn icon_texture_index_for_protocol_id(&self, protocol_id: i32) -> Option<u32> {
        self.icon_texture_indices_for_protocol_id_with_context(
            protocol_id,
            BlockModelDisplayContext::Gui,
        )
        .and_then(|indices| indices.into_iter().next())
    }

    fn icon_texture_indices_for_protocol_id_with_context(
        &self,
        protocol_id: i32,
        display_context: BlockModelDisplayContext,
    ) -> Option<Vec<u32>> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        let default_max_stack_size_for_item =
            |item_id| self.default_max_stack_size_for_protocol_id(item_id);
        let default_max_damage_for_item =
            |item_id| self.default_max_damage_for_protocol_id(item_id);
        let default_item_name_translation_key =
            self.default_item_name_translation_key_for_resource_id(item_id);
        let default_attribute_modifiers =
            self.default_attribute_modifiers_for_resource_id(item_id, None);
        let default_attribute_modifiers_for_item =
            |item_id| self.default_attribute_modifiers_for_protocol_id(item_id, None);
        let context = IconResolveContext {
            component_patch: None,
            stack_count: 1,
            default_max_stack_size: self
                .registry
                .as_ref()
                .and_then(|registry| registry.max_stack_size(item_id)),
            default_max_damage: None,
            bundle_selected_item_index: None,
            selected_item: false,
            carried_item: false,
            view_entity: false,
            shift_down: false,
            keybind_context: ItemModelKeybindContext::default(),
            fishing_rod_cast: false,
            using_item: false,
            use_context: ItemModelUseContext::inactive(),
            cooldown_progress: 0.0,
            crossbow_charge: CrossbowChargeType::None,
            display_context: item_display_context_name(display_context),
            item_model_seed: 0,
            default_item_model_id: item_id,
            default_item_name_translation_key: &default_item_name_translation_key,
            main_hand_left: None,
            context_dimension: None,
            context_entity_type: None,
            local_time_epoch_millis: self.local_time_epoch_millis(),
            time_context: None,
            stateful_model_id: item_id,
            time_wobbler: None,
            time_random: None,
            compass_context: None,
            compass_wobbler: None,
            compass_no_target_rotation: None,
            default_max_stack_size_for_item: Some(&default_max_stack_size_for_item),
            default_max_damage_for_item: Some(&default_max_damage_for_item),
            default_attribute_modifiers: &default_attribute_modifiers,
            default_attribute_modifiers_for_item: Some(&default_attribute_modifiers_for_item),
            item_resource_ids: self
                .registry
                .as_ref()
                .map(ItemRegistryCatalog::resource_ids),
            item_tags: self.item_tags.as_ref(),
            enchantment_tags: self.enchantment_tags.as_ref(),
            trim_material_tags: self.trim_material_tags.as_ref(),
            trim_pattern_tags: self.trim_pattern_tags.as_ref(),
            jukebox_song_tags: self.jukebox_song_tags.as_ref(),
            potion_tags: self.potion_tags.as_ref(),
            attribute_tags: self.attribute_tags.as_ref(),
            villager_type_tags: self.villager_type_tags.as_ref(),
            trim_material_keys: None,
            enchantment_keys: None,
            attribute_keys: None,
        };
        let mut indices = self
            .item_icon_models
            .get(item_id)
            .map(|model| self.icon_layers_for_model(model, context, 0))
            .unwrap_or_else(|| self.fallback_icon_texture_layers())
            .into_iter()
            .map(|layer| layer.texture_index)
            .collect::<Vec<_>>();
        if indices.is_empty() {
            indices.push(self.textures.fallback_index());
        }
        Some(indices)
    }

    fn icon_texture_indices_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        display_context: BlockModelDisplayContext,
    ) -> Option<Vec<u32>> {
        if item_stack_is_empty(stack) {
            return None;
        }
        let item_id = self.registry.as_ref()?.resource_id(stack.item_id?)?;
        let item_model_id = item_model_id_for_stack(item_id, Some(&stack.component_patch))?;
        let default_max_stack_size_for_item =
            |item_id| self.default_max_stack_size_for_protocol_id(item_id);
        let default_max_damage_for_item =
            |item_id| self.default_max_damage_for_protocol_id(item_id);
        let default_item_name_translation_key =
            self.default_item_name_translation_key_for_resource_id(item_id);
        let default_attribute_modifiers =
            self.default_attribute_modifiers_for_resource_id(item_id, None);
        let default_attribute_modifiers_for_item =
            |item_id| self.default_attribute_modifiers_for_protocol_id(item_id, None);
        let context = IconResolveContext {
            component_patch: Some(&stack.component_patch),
            stack_count: stack.count,
            default_max_stack_size: self
                .registry
                .as_ref()
                .and_then(|registry| registry.max_stack_size(item_id)),
            default_max_damage: self
                .registry
                .as_ref()
                .and_then(|registry| registry.max_damage(item_id)),
            bundle_selected_item_index: None,
            selected_item: false,
            carried_item: false,
            view_entity: false,
            shift_down: false,
            keybind_context: ItemModelKeybindContext::default(),
            fishing_rod_cast: false,
            using_item: false,
            use_context: ItemModelUseContext::inactive(),
            cooldown_progress: 0.0,
            crossbow_charge: self.crossbow_charge_for(Some(&stack.component_patch)),
            display_context: item_display_context_name(display_context),
            item_model_seed: 0,
            default_item_model_id: item_id,
            default_item_name_translation_key: &default_item_name_translation_key,
            main_hand_left: None,
            context_dimension: None,
            context_entity_type: None,
            local_time_epoch_millis: self.local_time_epoch_millis(),
            time_context: None,
            stateful_model_id: item_model_id,
            time_wobbler: None,
            time_random: None,
            compass_context: None,
            compass_wobbler: None,
            compass_no_target_rotation: None,
            default_max_stack_size_for_item: Some(&default_max_stack_size_for_item),
            default_max_damage_for_item: Some(&default_max_damage_for_item),
            default_attribute_modifiers: &default_attribute_modifiers,
            default_attribute_modifiers_for_item: Some(&default_attribute_modifiers_for_item),
            item_resource_ids: self
                .registry
                .as_ref()
                .map(ItemRegistryCatalog::resource_ids),
            item_tags: self.item_tags.as_ref(),
            enchantment_tags: self.enchantment_tags.as_ref(),
            trim_material_tags: self.trim_material_tags.as_ref(),
            trim_pattern_tags: self.trim_pattern_tags.as_ref(),
            jukebox_song_tags: self.jukebox_song_tags.as_ref(),
            potion_tags: self.potion_tags.as_ref(),
            attribute_tags: self.attribute_tags.as_ref(),
            villager_type_tags: self.villager_type_tags.as_ref(),
            trim_material_keys: None,
            enchantment_keys: None,
            attribute_keys: None,
        };
        let mut indices = self
            .item_icon_models
            .get(item_model_id)
            .map(|model| self.icon_layers_for_model(model, context, 0))
            .unwrap_or_else(|| self.fallback_icon_texture_layers())
            .into_iter()
            .map(|layer| layer.texture_index)
            .collect::<Vec<_>>();
        if indices.is_empty() {
            indices.push(self.textures.fallback_index());
        }
        Some(indices)
    }

    /// Default, empty-component `BreakingItemParticle.Provider` active-layer sprite ids for vanilla
    /// `ItemDisplayContext.GROUND`. Vanilla resolves a stack into an `ItemStackRenderState` and samples
    /// one layer particle material by random index; this static map covers the common no-component stack
    /// path used by packet and level-event item particles until the native particle option path owns full
    /// component patch decoding.
    pub fn default_item_particle_sprite_ids_by_protocol_id(&self) -> BTreeMap<i32, Vec<String>> {
        let Some(registry) = &self.registry else {
            return BTreeMap::new();
        };
        registry
            .resource_ids()
            .iter()
            .enumerate()
            .filter_map(|(protocol_id, _)| {
                let protocol_id = i32::try_from(protocol_id).ok()?;
                let sprite_ids = self
                    .icon_texture_indices_for_protocol_id_with_context(
                        protocol_id,
                        BlockModelDisplayContext::Ground,
                    )?
                    .into_iter()
                    .filter_map(|texture_index| {
                        self.textures.texture_id(texture_index).map(str::to_string)
                    })
                    .collect::<Vec<_>>();
                (!sprite_ids.is_empty()).then_some((protocol_id, sprite_ids))
            })
            .collect()
    }

    /// `BreakingItemParticle.Provider` active-layer sprite ids for the concrete
    /// `ItemStackTemplate` carried by `minecraft:item` particle options. Vanilla
    /// resolves the stack with `ItemDisplayContext.GROUND` before randomly
    /// choosing one of the resulting particle materials.
    pub fn item_particle_sprite_ids_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<Vec<String>> {
        Some(
            self.icon_texture_indices_for_stack_with_context(
                stack,
                BlockModelDisplayContext::Ground,
            )?
            .into_iter()
            .filter_map(|texture_index| self.textures.texture_id(texture_index).map(str::to_string))
            .collect(),
        )
    }

    #[cfg(test)]
    pub(crate) fn icon_uv_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasUvRect> {
        self.icon_for_protocol_id(protocol_id)
            .and_then(|icon| icon.layers.first().map(|layer| layer.uv))
    }

    pub fn icon_for_stack(&self, stack: &ItemStackSummary) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_bundle_selected_item(stack, None)
    }

    /// Vanilla `CuboidItemModelWrapper.hasSpecialAnimatedTexture`: clocks and item-tagged compasses use
    /// `ItemStackRenderState.FoilType.SPECIAL`, which routes foil through `SheetedDecalTextureGenerator`
    /// instead of reusing atlas UVs.
    pub fn item_stack_uses_special_foil_texture(&self, stack: &ItemStackSummary) -> bool {
        let Some(resource_id) = stack
            .item_id
            .and_then(|item_id| self.registry.as_ref()?.resource_id(item_id))
        else {
            return false;
        };
        self.item_resource_uses_special_foil_texture(resource_id)
    }

    pub(super) fn item_resource_uses_special_foil_texture(&self, resource_id: &str) -> bool {
        if resource_id == "minecraft:clock" {
            return true;
        }
        self.item_tags
            .as_ref()
            .map(|tags| tags.contains("minecraft:compasses", resource_id))
            .unwrap_or_else(|| {
                matches!(
                    resource_id,
                    "minecraft:compass" | "minecraft:recovery_compass"
                )
            })
    }

    /// Display transform for the effective root item model on this stack. Vanilla
    /// `ItemModelResolver.appendItemLayers` reads `DataComponents.ITEM_MODEL`
    /// before `ModelRenderProperties.applyToLayer` selects the transform for
    /// the current display context.
    pub fn item_display_transform_for_stack(
        &self,
        stack: &ItemStackSummary,
        context: BlockModelDisplayContext,
    ) -> Option<BlockModelDisplayTransform> {
        let item_id = self.registry.as_ref()?.resource_id(stack.item_id?)?;
        let item_model_id = item_model_id_for_stack(item_id, Some(&stack.component_patch))?;
        Some(
            self.item_display_transforms
                .get(item_model_id)?
                .get(context),
        )
    }

    /// Generated item layers for a non-living stack consumer that still has a
    /// level-backed dynamic trim registry, such as dropped items (`GROUND`) and
    /// item frames (`FIXED`). Vanilla `TrimMaterialProperty.get` reads only the
    /// stack's `minecraft:trim` component and the trim material registry key.
    #[cfg(test)]
    pub(crate) fn generated_item_layers_for_stack_with_trim_materials(
        &self,
        stack: &ItemStackSummary,
        display_context: BlockModelDisplayContext,
        trim_material_keys: Option<&[String]>,
    ) -> Vec<GeneratedItemLayer> {
        self.generated_item_layers_for_stack_with_registry_context(
            stack,
            display_context,
            trim_material_keys,
            None,
            None,
        )
    }

    pub fn generated_item_layers_for_stack_with_registry_context(
        &self,
        stack: &ItemStackSummary,
        display_context: BlockModelDisplayContext,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
    ) -> Vec<GeneratedItemLayer> {
        self.generated_item_layers_for_stack_with_context(
            stack,
            display_context,
            None,
            false,
            ItemModelUseContext::inactive(),
            None,
            None,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
        )
    }

    /// Generated item layers for an entity-owned stack. Vanilla `MainHand.get`
    /// returns null without a living owner; held-item paths pass the owner's
    /// main arm so `minecraft:main_hand` select cases can resolve. Vanilla
    /// `IsUsingItem.get` is true only for the stack currently returned by
    /// `owner.getUseItem()`, so held-item paths also pass whether this hand is
    /// the active use hand. Vanilla `ContextEntityType.get` reads
    /// `owner.typeHolder().unwrapKey()`, so entity-owned callers may also pass
    /// the owner's entity type key. Vanilla `TrimMaterialProperty.get` reads
    /// only the stack trim component and synced trim-material registry key, so
    /// owner-backed world-level callers may pass those keys too.
    #[cfg(test)]
    pub(crate) fn generated_item_layers_for_stack_with_owner_context(
        &self,
        stack: &ItemStackSummary,
        display_context: BlockModelDisplayContext,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        trim_material_keys: Option<&[String]>,
        using_item: bool,
        use_context: ItemModelUseContext,
    ) -> Vec<GeneratedItemLayer> {
        self.generated_item_layers_for_stack_with_owner_registry_context(
            stack,
            display_context,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            trim_material_keys,
            None,
            None,
            using_item,
            use_context,
        )
    }

    pub fn generated_item_layers_for_stack_with_owner_registry_context(
        &self,
        stack: &ItemStackSummary,
        display_context: BlockModelDisplayContext,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
        using_item: bool,
        use_context: ItemModelUseContext,
    ) -> Vec<GeneratedItemLayer> {
        self.generated_item_layers_for_stack_with_context(
            stack,
            display_context,
            owner_main_hand_left,
            using_item,
            use_context,
            context_entity_type,
            context_dimension,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
        )
    }

    pub(super) fn generated_item_layers_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        display_context: BlockModelDisplayContext,
        owner_main_hand_left: Option<bool>,
        using_item: bool,
        use_context: ItemModelUseContext,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
    ) -> Vec<GeneratedItemLayer> {
        let Some(icon) = self.icon_for_stack_with_model_registry_context(
            stack,
            None,
            using_item,
            use_context,
            display_context,
            0.0,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            None,
            None,
            false,
            false,
            false,
            false,
            ItemModelKeybindContext::default(),
            false,
        ) else {
            return Vec::new();
        };
        icon.layers
            .into_iter()
            .filter_map(|layer| {
                let mask = self.textures.alpha_mask_for_uv(layer.uv)?;
                Some(GeneratedItemLayer {
                    mask,
                    rect: ItemSpriteRect {
                        min: layer.uv.min,
                        max: layer.uv.max,
                    },
                    tint: layer.tint,
                })
            })
            .collect()
    }

    pub(crate) fn icon_for_stack_with_bundle_selected_item(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_bundle_selected_item_and_using_item(
            stack,
            bundle_selected_item_index,
            false,
        )
    }

    pub fn icon_for_stack_with_bundle_selected_item_and_using_item(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context(
            stack,
            bundle_selected_item_index,
            using_item,
            0.0,
            None,
            None,
            None,
            None,
        )
    }

    /// Resolves a stack's icon with GUI/HUD context: bundle selected item,
    /// local using-item state, `minecraft:trim_material` registry keys, and an
    /// optional living-owner main arm / entity type for `minecraft:main_hand`
    /// / `minecraft:context_entity_type` plus the current dimension for
    /// `minecraft:context_dimension`.
    pub fn icon_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context_and_use_context(
            stack,
            bundle_selected_item_index,
            using_item,
            ItemModelUseContext::inactive(),
            BlockModelDisplayContext::Gui,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
        )
    }

    pub fn icon_for_stack_with_context_and_use_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context_and_use_context_and_time_context(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            None,
            None,
        )
    }

    pub fn icon_for_stack_with_context_and_use_context_and_time_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context_and_use_context_time_selected(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            false,
        )
    }

    pub fn icon_for_stack_with_context_and_use_context_time_selected(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context_and_use_context_time_state(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            false,
            false,
            false,
            ItemModelKeybindContext::default(),
        )
    }

    pub fn icon_for_stack_with_context_and_use_context_time_state(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            false,
        )
    }

    pub fn icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
        fishing_rod_cast: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_registry_context_and_seed(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            None,
            None,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast_with_registry_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
        fishing_rod_cast: bool,
        item_model_seed: i32,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_registry_context_and_seed(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            item_model_seed,
        )
    }

    pub fn icon_for_stack_with_owner_main_hand(
        &self,
        stack: &ItemStackSummary,
        owner_main_hand_left: Option<bool>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_owner_context(stack, owner_main_hand_left, false)
    }

    pub(crate) fn icon_for_stack_with_owner_context(
        &self,
        stack: &ItemStackSummary,
        owner_main_hand_left: Option<bool>,
        using_item: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_context(
            stack,
            None,
            using_item,
            ItemModelUseContext::inactive(),
            BlockModelDisplayContext::Gui,
            0.0,
            None,
            owner_main_hand_left,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            ItemModelKeybindContext::default(),
            false,
        )
    }

    pub(super) fn icon_for_stack_with_model_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
        fishing_rod_cast: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_registry_context(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            None,
            None,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            fishing_rod_cast,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn icon_for_stack_with_model_registry_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
        fishing_rod_cast: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_registry_context_and_seed(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn icon_for_stack_with_model_registry_context_and_seed(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
        fishing_rod_cast: bool,
        item_model_seed: i32,
    ) -> Option<ItemAtlasIcon> {
        let item_id = self.registry.as_ref()?.resource_id(stack.item_id?)?;
        let item_model_id = item_model_id_for_stack(item_id, Some(&stack.component_patch))?;
        self.icon_for_resource_id(
            item_id,
            item_model_id,
            stack.count,
            Some(&stack.component_patch),
            bundle_selected_item_index,
            using_item,
            use_context,
            display_context,
            cooldown_progress,
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            item_model_seed,
        )
    }

    #[cfg(test)]
    pub(crate) fn icon_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasIcon> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        self.icon_for_resource_id(
            item_id,
            item_id,
            1,
            None,
            None,
            false,
            ItemModelUseContext::inactive(),
            BlockModelDisplayContext::Gui,
            0.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            ItemModelKeybindContext::default(),
            false,
            0,
        )
    }

    pub(super) fn icon_for_resource_id(
        &self,
        item_id: &str,
        item_model_id: &str,
        stack_count: i32,
        component_patch: Option<&DataComponentPatchSummary>,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        display_context: BlockModelDisplayContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        enchantment_keys: Option<&[String]>,
        attribute_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
        time_context: Option<ItemModelTimeContext>,
        compass_context: Option<ItemModelCompassContext<'_>>,
        selected_item: bool,
        carried_item: bool,
        view_entity: bool,
        shift_down: bool,
        keybind_context: ItemModelKeybindContext,
        fishing_rod_cast: bool,
        item_model_seed: i32,
    ) -> Option<ItemAtlasIcon> {
        let default_max_damage = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_damage(item_id));
        let default_max_stack_size = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_stack_size(item_id));
        let default_max_stack_size_for_item =
            |item_id| self.default_max_stack_size_for_protocol_id(item_id);
        let default_max_damage_for_item =
            |item_id| self.default_max_damage_for_protocol_id(item_id);
        let default_item_name_translation_key =
            self.default_item_name_translation_key_for_resource_id(item_id);
        let default_attribute_modifiers =
            self.default_attribute_modifiers_for_resource_id(item_id, attribute_keys);
        let default_attribute_modifiers_for_item =
            |item_id| self.default_attribute_modifiers_for_protocol_id(item_id, attribute_keys);
        let time_wobbler = |model_id: &str,
                            state_id: u64,
                            source: TimeSource,
                            game_time: i64,
                            target_rotation: f32| {
            self.resolve_time_wobbler(model_id, state_id, source, game_time, target_rotation)
        };
        let time_random =
            |model_id: &str, state_id: u64| self.resolve_time_random(model_id, state_id);
        let compass_wobbler = |model_id: &str,
                               state_id: u64,
                               target: CompassTarget,
                               game_time: i64,
                               target_rotation: f32| {
            self.resolve_compass_wobbler(model_id, state_id, target, game_time, target_rotation)
        };
        let compass_no_target_rotation = |model_id: &str,
                                          state_id: u64,
                                          target: CompassTarget,
                                          wobble: bool,
                                          game_time: i64,
                                          seed: i32| {
            self.resolve_compass_no_target_rotation(
                model_id, state_id, target, wobble, game_time, seed,
            )
        };
        let context = IconResolveContext {
            component_patch,
            stack_count,
            default_max_stack_size,
            default_max_damage,
            bundle_selected_item_index,
            selected_item,
            carried_item,
            view_entity,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            using_item,
            use_context,
            cooldown_progress,
            crossbow_charge: self.crossbow_charge_for(component_patch),
            display_context: item_display_context_name(display_context),
            item_model_seed,
            default_item_model_id: item_id,
            default_item_name_translation_key: &default_item_name_translation_key,
            main_hand_left: owner_main_hand_left,
            context_dimension,
            context_entity_type,
            local_time_epoch_millis: self.local_time_epoch_millis(),
            time_context,
            stateful_model_id: item_model_id,
            time_wobbler: Some(&time_wobbler),
            time_random: Some(&time_random),
            compass_context,
            compass_wobbler: Some(&compass_wobbler),
            compass_no_target_rotation: Some(&compass_no_target_rotation),
            default_max_stack_size_for_item: Some(&default_max_stack_size_for_item),
            default_max_damage_for_item: Some(&default_max_damage_for_item),
            default_attribute_modifiers: &default_attribute_modifiers,
            default_attribute_modifiers_for_item: Some(&default_attribute_modifiers_for_item),
            item_resource_ids: self
                .registry
                .as_ref()
                .map(ItemRegistryCatalog::resource_ids),
            item_tags: self.item_tags.as_ref(),
            enchantment_tags: self.enchantment_tags.as_ref(),
            trim_material_tags: self.trim_material_tags.as_ref(),
            trim_pattern_tags: self.trim_pattern_tags.as_ref(),
            jukebox_song_tags: self.jukebox_song_tags.as_ref(),
            potion_tags: self.potion_tags.as_ref(),
            attribute_tags: self.attribute_tags.as_ref(),
            villager_type_tags: self.villager_type_tags.as_ref(),
            trim_material_keys,
            enchantment_keys,
            attribute_keys,
        };
        let layers = self
            .item_icon_models
            .get(item_model_id)
            .map(|model| self.icon_layers_for_model(model, context, 0))
            .unwrap_or_else(|| self.fallback_icon_texture_layers());
        let layers = layers
            .into_iter()
            .filter_map(|layer| {
                self.textures
                    .texture_uv_rect(layer.texture_index)
                    .map(|uv| ItemAtlasIconLayer {
                        uv,
                        tint: item_icon_tint_color(&layer.tint, component_patch),
                    })
            })
            .collect::<Vec<_>>();
        (!layers.is_empty()).then_some(ItemAtlasIcon { layers })
    }

    pub(super) fn icon_layers_for_model(
        &self,
        model: &ItemIconModel,
        context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        if depth >= ITEM_ICON_RECURSION_LIMIT {
            return Vec::new();
        }
        let mut resolve_bundle_selected_item =
            || self.bundle_selected_item_layers(context, depth + 1);
        model.icon_layers_with_bundle_resolver(context, &mut resolve_bundle_selected_item)
    }

    /// Vanilla `Charge.get`: `ROCKET` when any charged projectile is a
    /// `minecraft:firework_rocket`, `ARROW` when charged with anything else,
    /// else `NONE`. Projects the stack's `charged_projectiles` component.
    pub(super) fn crossbow_charge_for(
        &self,
        component_patch: Option<&DataComponentPatchSummary>,
    ) -> CrossbowChargeType {
        let Some(patch) = component_patch else {
            return CrossbowChargeType::None;
        };
        if patch.charged_projectiles_items.is_empty() {
            return CrossbowChargeType::None;
        }
        let is_rocket = patch.charged_projectiles_items.iter().any(|template| {
            self.registry
                .as_ref()
                .and_then(|registry| registry.resource_id(template.item_id))
                == Some(FIREWORK_ROCKET_ITEM_ID)
        });
        if is_rocket {
            CrossbowChargeType::Rocket
        } else {
            CrossbowChargeType::Arrow
        }
    }

    pub(super) fn resolve_time_wobbler(
        &self,
        item_model_id: &str,
        state_id: u64,
        source: TimeSource,
        game_time: i64,
        target_rotation: f32,
    ) -> f32 {
        let key = ItemTimeWobblerKey {
            item_model_id: item_model_id.to_string(),
            state_id,
            source,
        };
        self.time_wobblers
            .borrow_mut()
            .entry(key)
            .or_insert(ItemNeedleWobbler {
                rotation: 0.0,
                delta_rotation: 0.0,
                last_update_tick: None,
            })
            .update(game_time, target_rotation, 0.9)
    }

    pub(super) fn resolve_time_random(&self, item_model_id: &str, state_id: u64) -> f32 {
        let key = ItemTimeRandomKey {
            item_model_id: item_model_id.to_string(),
            state_id,
        };
        self.time_randoms
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                ItemLegacyRandom::new(item_time_random_seed(item_model_id, state_id))
            })
            .next_float()
    }

    pub(super) fn resolve_compass_wobbler(
        &self,
        item_model_id: &str,
        state_id: u64,
        target: CompassTarget,
        game_time: i64,
        target_rotation: f32,
    ) -> f32 {
        let key = ItemCompassWobblerKey {
            item_model_id: item_model_id.to_string(),
            state_id,
            target,
            no_target: false,
        };
        self.compass_wobblers
            .borrow_mut()
            .entry(key)
            .or_insert(ItemNeedleWobbler {
                rotation: 0.0,
                delta_rotation: 0.0,
                last_update_tick: None,
            })
            .update(game_time, target_rotation, 0.8)
    }

    pub(super) fn resolve_compass_no_target_rotation(
        &self,
        item_model_id: &str,
        state_id: u64,
        target: CompassTarget,
        wobble: bool,
        game_time: i64,
        seed: i32,
    ) -> f32 {
        let rotation = if wobble {
            let key = ItemCompassWobblerKey {
                item_model_id: item_model_id.to_string(),
                state_id,
                target,
                no_target: true,
            };
            let mut wobblers = self.compass_wobblers.borrow_mut();
            let wobbler = wobblers.entry(key).or_insert(ItemNeedleWobbler {
                rotation: 0.0,
                delta_rotation: 0.0,
                last_update_tick: None,
            });
            if wobbler.should_update(game_time) {
                let target_rotation = self.resolve_compass_random(item_model_id, state_id, target);
                wobbler.update(game_time, target_rotation, 0.8);
            }
            wobbler.rotation
        } else {
            self.resolve_compass_random(item_model_id, state_id, target)
        };
        (rotation + compass_seed_offset(seed)).rem_euclid(1.0)
    }

    pub(super) fn resolve_compass_random(
        &self,
        item_model_id: &str,
        state_id: u64,
        target: CompassTarget,
    ) -> f32 {
        let key = ItemCompassRandomKey {
            item_model_id: item_model_id.to_string(),
            state_id,
            target,
        };
        self.compass_randoms
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                ItemLegacyRandom::new(item_compass_random_seed(item_model_id, state_id, target))
            })
            .next_float()
    }

    pub fn item_model_use_context_for_stack(
        &self,
        stack: &ItemStackSummary,
        elapsed_ticks: u32,
    ) -> ItemModelUseContext {
        self.item_model_use_context_for_stack_with_enchantment_keys(stack, elapsed_ticks, None)
    }

    pub fn item_model_use_context_for_stack_with_enchantment_keys(
        &self,
        stack: &ItemStackSummary,
        elapsed_ticks: u32,
        enchantment_keys: Option<&[String]>,
    ) -> ItemModelUseContext {
        let Some(item_id) = stack
            .item_id
            .and_then(|protocol_id| self.registry.as_ref()?.resource_id(protocol_id))
        else {
            return ItemModelUseContext::inactive();
        };
        ItemModelUseContext::active(
            elapsed_ticks,
            item_use_duration_ticks(item_id, &stack.component_patch),
            crossbow_charge_duration_ticks(item_id, &stack.component_patch, enchantment_keys),
        )
    }

    pub(super) fn bundle_selected_item_layers(
        &self,
        context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        let Some(selected_item_index) = context
            .bundle_selected_item_index
            .filter(|index| *index >= 0)
        else {
            return Vec::new();
        };
        let Ok(selected_item_index) = usize::try_from(selected_item_index) else {
            return Vec::new();
        };
        let Some(template) = context
            .component_patch
            .and_then(|patch| patch.bundle_contents_items.get(selected_item_index))
        else {
            return Vec::new();
        };
        self.item_template_layers(template, context, depth)
    }

    pub(super) fn item_template_layers(
        &self,
        template: &ItemStackTemplateSummary,
        parent_context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        let Some(item_id) = self
            .registry
            .as_ref()
            .and_then(|registry| registry.resource_id(template.item_id))
        else {
            return Vec::new();
        };
        let default_max_damage = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_damage(item_id));
        let default_max_stack_size = parent_context
            .default_max_stack_size_for_item
            .map(|max_stack_size| max_stack_size(template.item_id));
        let default_item_name_translation_key =
            self.default_item_name_translation_key_for_resource_id(item_id);
        let default_attribute_modifiers = parent_context
            .default_attribute_modifiers_for_item
            .map(|modifiers| modifiers(template.item_id))
            .unwrap_or_default();
        let context = IconResolveContext {
            component_patch: Some(&template.component_patch),
            stack_count: template.count,
            default_max_stack_size,
            default_max_damage,
            bundle_selected_item_index: None,
            selected_item: false,
            carried_item: false,
            view_entity: false,
            shift_down: false,
            keybind_context: ItemModelKeybindContext::default(),
            fishing_rod_cast: false,
            using_item: false,
            use_context: ItemModelUseContext::inactive(),
            cooldown_progress: 0.0,
            crossbow_charge: self.crossbow_charge_for(Some(&template.component_patch)),
            display_context: parent_context.display_context,
            item_model_seed: parent_context.item_model_seed,
            default_item_model_id: item_id,
            default_item_name_translation_key: &default_item_name_translation_key,
            main_hand_left: parent_context.main_hand_left,
            context_dimension: parent_context.context_dimension,
            context_entity_type: parent_context.context_entity_type,
            local_time_epoch_millis: parent_context.local_time_epoch_millis,
            time_context: parent_context.time_context,
            stateful_model_id: item_id,
            time_wobbler: parent_context.time_wobbler,
            time_random: parent_context.time_random,
            compass_context: parent_context.compass_context,
            compass_wobbler: parent_context.compass_wobbler,
            compass_no_target_rotation: parent_context.compass_no_target_rotation,
            default_max_stack_size_for_item: parent_context.default_max_stack_size_for_item,
            default_max_damage_for_item: parent_context.default_max_damage_for_item,
            default_attribute_modifiers: &default_attribute_modifiers,
            default_attribute_modifiers_for_item: parent_context
                .default_attribute_modifiers_for_item,
            item_resource_ids: parent_context.item_resource_ids,
            item_tags: parent_context.item_tags,
            enchantment_tags: parent_context.enchantment_tags,
            trim_material_tags: parent_context.trim_material_tags,
            trim_pattern_tags: parent_context.trim_pattern_tags,
            jukebox_song_tags: parent_context.jukebox_song_tags,
            potion_tags: parent_context.potion_tags,
            attribute_tags: parent_context.attribute_tags,
            villager_type_tags: parent_context.villager_type_tags,
            trim_material_keys: parent_context.trim_material_keys,
            enchantment_keys: parent_context.enchantment_keys,
            attribute_keys: parent_context.attribute_keys,
        };
        let layers = self
            .item_icon_models
            .get(item_id)
            .map(|model| self.icon_layers_for_model(model, context, depth))
            .unwrap_or_else(|| self.fallback_icon_texture_layers());
        resolve_item_icon_texture_layer_tints(layers, Some(&template.component_patch))
    }

    pub(super) fn local_time_epoch_millis(&self) -> Option<i64> {
        self.local_time_epoch_millis_override
            .get()
            .or_else(current_epoch_millis)
    }

    pub(super) fn fallback_icon_texture_layers(&self) -> Vec<ItemIconTextureLayer> {
        vec![ItemIconTextureLayer {
            texture_index: self.textures.fallback_index(),
            tint: ItemIconTint::Static(ITEM_TINT_WHITE),
        }]
    }
}
