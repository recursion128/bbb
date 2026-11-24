use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ItemTimeWobblerKey {
    item_model_id: String,
    state_id: u64,
    source: TimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ItemCompassWobblerKey {
    item_model_id: String,
    state_id: u64,
    target: CompassTarget,
    no_target: bool,
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
            .contains(&VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID)
    }) {
        return None;
    }
    component_patch
        .and_then(|patch| patch.item_model.as_deref())
        .or(Some(item_id))
}

pub(super) fn item_display_context_name(context: BlockModelDisplayContext) -> &'static str {
    match context {
        BlockModelDisplayContext::None => "none",
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

fn effective_component_is_present(
    patch: &DataComponentPatchSummary,
    component_id: i32,
    present_by_default: bool,
) -> bool {
    if patch.removed_type_ids.contains(&component_id) {
        false
    } else if patch.added_type_ids.contains(&component_id) {
        true
    } else {
        present_by_default
    }
}

fn component_patch_without_damage(patch: &DataComponentPatchSummary) -> DataComponentPatchSummary {
    let mut normalized = patch.clone();
    normalized
        .added_type_ids
        .retain(|component_id| *component_id != VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    normalized
        .removed_type_ids
        .retain(|component_id| *component_id != VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    normalized
        .codec_backed_nbt
        .remove(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    normalized.added_type_ids.sort_unstable();
    normalized.removed_type_ids.sort_unstable();
    normalized.damage = None;
    normalized
}

pub(super) fn item_use_duration_ticks(
    item_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> i32 {
    if [
        item_types::VANILLA_ITEM_BOW_ID,
        item_types::VANILLA_ITEM_CROSSBOW_ID,
        item_types::VANILLA_ITEM_TRIDENT_ID,
    ]
    .into_iter()
    .any(|item_type_id| item_types::vanilla_item_resource_id_matches_type(item_id, item_type_id))
    {
        return VANILLA_LONG_USE_DURATION_TICKS;
    }
    if item_types::vanilla_item_resource_id_matches_type(item_id, item_types::VANILLA_ITEM_BRUSH_ID)
    {
        return VANILLA_BRUSH_USE_DURATION_TICKS;
    }
    if item_types::vanilla_item_resource_id_matches_type(
        item_id,
        item_types::VANILLA_ITEM_SPYGLASS_ID,
    ) {
        return VANILLA_SPYGLASS_USE_DURATION_TICKS;
    }
    if item_types::vanilla_item_resource_id_matches_type(
        item_id,
        item_types::VANILLA_ITEM_ENDER_EYE_ID,
    ) {
        return VANILLA_ENDER_EYE_USE_DURATION_TICKS;
    }
    if let Some(consumable) = component_patch.consumable {
        return consumable_use_duration_ticks(consumable);
    }
    if component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
        || component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
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
    synchronized_duration_ticks: Option<i32>,
) -> Option<i32> {
    if !item_types::vanilla_item_resource_id_matches_type(
        item_id,
        item_types::VANILLA_ITEM_CROSSBOW_ID,
    ) {
        return None;
    }
    Some(
        synchronized_duration_ticks
            .unwrap_or(VANILLA_CROSSBOW_CHARGE_DURATION_TICKS)
            .max(0),
    )
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ItemIconTextureLayer {
    pub(super) texture_index: u32,
    pub(super) tint: ItemIconTint,
    pub(super) translucent: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ItemTextureState {
    cuboid_atlas: std::sync::OnceLock<crate::terrain::TerrainTextureAtlas>,
    pub(super) atlas: AtlasImage,
    pub(super) texture_indices: HashMap<String, u32>,
    pub(super) texture_ids: Vec<Arc<str>>,
    pub(super) alpha_masks: Vec<SpriteAlphaMask>,
    pub(super) fallback_index: u32,
    animation: Option<ItemTextureAnimation>,
}

#[derive(Debug, Clone)]
struct ItemTextureAnimation {
    frames: bbb_client_assets::AtlasAnimationMipCache,
}

impl ItemTextureAnimation {
    fn new(layout: &AtlasLayout, images: &[&SpriteImage]) -> Result<Option<Self>> {
        if !images.iter().any(|image| {
            image
                .animation
                .as_ref()
                .is_some_and(|animation| animation.frames.len() > 1)
        }) {
            return Ok(None);
        }
        Ok(Some(Self {
            frames: layout.prepare_animation_mip_cache_refs(images, 0)?,
        }))
    }

    fn atlas_patches(
        &self,
        layout: &AtlasLayout,
        previous_tick: u64,
        tick: u64,
    ) -> Result<Vec<bbb_client_assets::AtlasMipPatch>> {
        self.frames.frame_mip_patches(layout, previous_tick, tick)
    }
}

impl ItemTextureState {
    pub(super) fn from_images(images: Vec<SpriteImage>) -> Result<Self> {
        let refs = images.iter().collect::<Vec<_>>();
        Self::from_image_refs(&refs)
    }

    pub(super) fn from_image_refs(images: &[&SpriteImage]) -> Result<Self> {
        Self::from_image_refs_with_stack(images, None)
    }

    pub(super) fn from_image_refs_with_stack(
        images: &[&SpriteImage],
        stack: Option<&PackResourceStack>,
    ) -> Result<Self> {
        let packer = AtlasPacker::new(ITEM_ATLAS_MAX_WIDTH, 1)?;
        let atlas = match stack {
            Some(stack) => packer.stitch_refs_with_stack(images, stack)?,
            None => packer.stitch_refs(images)?,
        };
        let mut texture_indices = HashMap::new();
        for (index, sprite) in atlas.layout.sprites.iter().enumerate() {
            texture_indices.insert(sprite.id.clone(), index as u32);
        }
        let fallback_index = texture_indices
            .get(MISSING_TEXTURE_ID)
            .copied()
            .unwrap_or(0);
        let texture_ids = atlas
            .layout
            .sprites
            .iter()
            .map(|sprite| Arc::<str>::from(sprite.id.as_str()))
            .collect();
        let alpha_masks = atlas
            .layout
            .sprites
            .iter()
            .zip(images)
            .map(|(sprite, image)| sprite_alpha_mask(sprite, image))
            .collect::<Result<Vec<_>>>()?;
        let animation = ItemTextureAnimation::new(&atlas.layout, images)?;
        Ok(Self {
            cuboid_atlas: std::sync::OnceLock::new(),
            atlas,
            texture_indices,
            texture_ids,
            alpha_masks,
            fallback_index,
            animation,
        })
    }

    pub(super) fn cuboid_atlas(&self) -> &crate::terrain::TerrainTextureAtlas {
        self.cuboid_atlas
            .get_or_init(|| crate::terrain::TerrainTextureAtlas {
                rects: (0..self.texture_ids.len())
                    .map(|i| {
                        let rect = self.texture_uv_rect(i as u32).unwrap();
                        crate::terrain::TerrainUvRect {
                            min: rect.min,
                            max: rect.max,
                        }
                    })
                    .collect(),
                fallback_index: self.fallback_index,
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

    pub(super) fn release_atlas_rgba(&mut self) {
        self.atlas.rgba = Vec::new();
    }

    pub(super) fn has_animation(&self) -> bool {
        self.animation.is_some()
    }

    pub(super) fn animation_atlas_patches(
        &self,
        previous_tick: u64,
        tick: u64,
    ) -> Result<Option<Vec<bbb_client_assets::AtlasMipPatch>>> {
        self.animation
            .as_ref()
            .map(|animation| animation.atlas_patches(&self.atlas.layout, previous_tick, tick))
            .transpose()
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

    pub(super) fn texture_has_translucent(&self, texture_index: u32) -> bool {
        self.atlas
            .layout
            .sprites
            .get(texture_index as usize)
            .is_some_and(|sprite| sprite.transparency.has_translucent)
    }

    pub(super) fn texture_id(&self, texture_index: u32) -> Option<&str> {
        self.texture_ids
            .get(texture_index as usize)
            .map(AsRef::as_ref)
    }

    pub(super) fn texture_asset_id(&self, texture_index: u32) -> Option<Arc<str>> {
        self.texture_ids.get(texture_index as usize).cloned()
    }

    pub(super) fn texture_uv_rect(&self, texture_index: u32) -> Option<ItemAtlasUvRect> {
        let sprite = self.atlas.layout.sprites.get(texture_index as usize)?;
        Some(item_uv_rect(&self.atlas.layout, sprite))
    }

    /// Returns the immutable alpha silhouette built once with the atlas.
    pub(super) fn alpha_mask(&self, texture_index: u32) -> Option<SpriteAlphaMask> {
        self.alpha_masks.get(texture_index as usize).cloned()
    }
}

fn sprite_alpha_mask(sprite: &AtlasSprite, image: &SpriteImage) -> Result<SpriteAlphaMask> {
    let mut opaque = Vec::with_capacity((sprite.content.width * sprite.content.height) as usize);
    opaque.resize(
        (sprite.content.width * sprite.content.height) as usize,
        false,
    );
    let frame_indices = image
        .animation
        .as_ref()
        .map(|animation| {
            animation
                .frames
                .iter()
                .map(|frame| frame.index)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_else(|| BTreeSet::from([0]));
    for frame_index in frame_indices {
        let rgba = image.frame_rgba(frame_index).ok_or_else(|| {
            anyhow::anyhow!(
                "item texture {} is missing animation frame {}",
                image.id,
                frame_index
            )
        })?;
        for (pixel, alpha) in opaque
            .iter_mut()
            .zip(rgba.as_chunks::<4>().0.iter().map(|pixel| pixel[3]))
        {
            *pixel |= alpha != 0;
        }
    }
    Ok(SpriteAlphaMask::new(
        sprite.content.width,
        sprite.content.height,
        opaque,
    ))
}

pub(super) fn generated_icon_layers(
    model: &ItemCuboidModel,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
    textures: &ItemTextureState,
) -> Option<Vec<ItemIconTextureLayer>> {
    let tints = model_tints.get(&model.id);
    let mut layers = Vec::new();
    for layer_index in 0..ITEM_GENERATED_MAX_LAYERS {
        let Some(texture) = model.texture_slots.get(&format!("layer{layer_index}")) else {
            break;
        };
        let texture_index = textures.texture_index(&texture.id);
        layers.push(ItemIconTextureLayer {
            texture_index,
            translucent: texture.force_translucent
                || textures.texture_has_translucent(texture_index),
            tint: tints
                .and_then(|tints| tints.get(layer_index))
                .map(|tint| item_tint_source(tint, colormaps))
                .unwrap_or(ItemIconTint::Static(ITEM_TINT_WHITE)),
        });
    }
    (!layers.is_empty()).then_some(layers)
}

pub(super) fn first_icon_layer(
    model: &ItemCuboidModel,
    textures: &ItemTextureState,
) -> Option<ItemIconTextureLayer> {
    let (texture_id, force_translucent) = model
        .texture_slots
        .values()
        .next()
        .map(|texture| (texture.id.as_str(), texture.force_translucent))
        .or_else(|| {
            model
                .face_textures
                .as_ref()
                .map(|faces| (faces.textures[0].as_str(), false))
        })?;
    let texture_index = textures.texture_index(texture_id);
    Some(ItemIconTextureLayer {
        texture_index,
        translucent: force_translucent || textures.texture_has_translucent(texture_index),
        tint: ItemIconTint::Static(ITEM_TINT_WHITE),
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
    owner_team_color_rgb: Option<u32>,
) -> [f32; 4] {
    match tint {
        ItemIconTint::Static(color) => *color,
        ItemIconTint::Source(source) => {
            item_tint_source_color(source, component_patch, owner_team_color_rgb)
        }
    }
}

pub(super) fn resolve_item_icon_texture_layer_tints(
    layers: Vec<ItemIconTextureLayer>,
    component_patch: Option<&DataComponentPatchSummary>,
    owner_team_color_rgb: Option<u32>,
) -> Vec<ItemIconTextureLayer> {
    layers
        .into_iter()
        .map(|layer| ItemIconTextureLayer {
            texture_index: layer.texture_index,
            tint: ItemIconTint::Static(item_icon_tint_color(
                &layer.tint,
                component_patch,
                owner_team_color_rgb,
            )),
            translucent: layer.translucent,
        })
        .collect()
}

pub(super) fn item_tint_source_color(
    tint: &ItemTintSource,
    component_patch: Option<&DataComponentPatchSummary>,
    owner_team_color_rgb: Option<u32>,
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
                .and_then(potion_contents_tint_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Firework { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.firework_explosion.as_ref())
                .and_then(|explosion| firework_explosion_tint_color(&explosion.colors))
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Constant { value } => rgb_i32_tint(*value),
        ItemTintSource::Grass { .. } => item_tint_source_default_color(tint, None),
        ItemTintSource::Team { default_color } => {
            rgb_i32_tint(owner_team_color_rgb.map_or(*default_color, |color| color as i32))
        }
    }
}

fn potion_contents_tint_color(component_patch: &DataComponentPatchSummary) -> Option<i32> {
    if let Some(custom_color) = component_patch.potion_contents.custom_color {
        return Some(custom_color);
    }

    let mut red = 0u32;
    let mut green = 0u32;
    let mut blue = 0u32;
    let mut total_weight = 0u32;
    let mut add_effect = |effect_id: i32, amplifier: i32, visible: bool| {
        if !visible {
            return;
        }
        let Some(effect) = bbb_protocol::mob_effect_types::vanilla_mob_effect_type(effect_id)
        else {
            return;
        };
        let weight = u32::try_from(amplifier.saturating_add(1)).unwrap_or(0);
        let color = effect.color;
        red += weight * ((color >> 16) & 0xff);
        green += weight * ((color >> 8) & 0xff);
        blue += weight * (color & 0xff);
        total_weight += weight;
    };

    if let Some(potion) = component_patch
        .potion_contents
        .potion_id
        .and_then(bbb_protocol::potion_types::vanilla_potion)
    {
        for effect in potion.effects {
            add_effect(effect.effect_id, effect.amplifier, true);
        }
    }
    for effect in &component_patch.potion_contents.custom_effects {
        add_effect(effect.effect_id, effect.amplifier, effect.show_particles);
    }

    (total_weight != 0).then(|| {
        let red = red / total_weight;
        let green = green / total_weight;
        let blue = blue / total_weight;
        ((red << 16) | (green << 8) | blue) as i32
    })
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

#[cfg(test)]
impl NativeItemRuntime {
    pub(crate) fn icon_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasIcon> {
        let stack = ItemStackSummary {
            item_id: Some(protocol_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        self.resolve_item_model(
            &stack,
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )?
        .flat_icon()
    }

    pub(crate) fn icon_texture_index_for_protocol_id(&self, protocol_id: i32) -> Option<u32> {
        self.item_resource_id_for_protocol_id(protocol_id)?;
        Some(
            self.icon_for_protocol_id(protocol_id)
                .map_or(self.textures.fallback_index(), |icon| {
                    icon.layers[0].texture_index
                }),
        )
    }

    pub(crate) fn icon_uv_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasUvRect> {
        self.icon_for_protocol_id(protocol_id)
            .map(|icon| icon.layers[0].uv)
    }
}

impl NativeItemRuntime {
    /// Default, empty-component `BreakingItemParticle.Provider` item-atlas
    /// sprites for `ItemDisplayContext.GROUND`, indexed by item protocol id
    /// (empty when the item has none).
    pub fn default_item_particle_sprites(&self) -> Vec<Arc<[u32]>> {
        let base_count = self
            .registry
            .as_ref()
            .map_or(0, |registry| registry.resource_ids().len());
        let mod_max = self
            .content_registry
            .items()
            .iter()
            .filter_map(|spec| usize::try_from(spec.id).ok())
            .max()
            .map_or(0, |id| id + 1);
        let total = base_count.max(mod_max);
        (0..total)
            .map(|protocol_id| {
                let stack = ItemStackSummary {
                    item_id: i32::try_from(protocol_id).ok(),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                };
                self.item_particle_sprites_for_stack(&stack)
                    .unwrap_or_else(|| Arc::from([]))
            })
            .collect()
    }

    /// `BreakingItemParticle.Provider` item-atlas sprites for the concrete
    /// `ItemStackTemplate` carried by `minecraft:item` particle options. Vanilla
    /// resolves the stack with `ItemDisplayContext.GROUND` before randomly
    /// choosing one of the resulting particle materials.
    pub fn item_particle_sprites_for_stack(&self, stack: &ItemStackSummary) -> Option<Arc<[u32]>> {
        if item_stack_is_empty(stack) {
            return None;
        }
        let mut indices: Vec<u32> = self
            .resolve_item_model(
                stack,
                &ItemModelContext::new(BlockModelDisplayContext::Ground),
            )
            .and_then(|model| model.flat_icon())
            .map(|icon| {
                icon.layers
                    .into_iter()
                    .map(|layer| layer.texture_index)
                    .collect()
            })
            .unwrap_or_default();
        if indices.is_empty() {
            indices.push(self.textures.fallback_index());
        }
        Some(
            indices
                .into_iter()
                .filter(|&index| self.textures.texture_id(index).is_some())
                .collect(),
        )
    }

    /// Vanilla `CuboidItemModelWrapper.hasSpecialAnimatedTexture`: clocks and item-tagged compasses use
    /// `ItemStackRenderState.FoilType.SPECIAL`, which routes foil through `SheetedDecalTextureGenerator`
    /// instead of reusing atlas UVs.
    pub fn item_stack_uses_special_foil_texture(&self, stack: &ItemStackSummary) -> bool {
        let Some(resource_id) = stack
            .item_id
            .and_then(|item_id| self.item_resource_id_for_protocol_id(item_id))
        else {
            return false;
        };
        self.item_resource_uses_special_foil_texture(resource_id)
    }

    pub(super) fn item_resource_uses_special_foil_texture(&self, resource_id: &str) -> bool {
        if item_types::vanilla_item_resource_id_matches_type(
            resource_id,
            item_types::VANILLA_ITEM_CLOCK_ID,
        ) {
            return true;
        }
        self.item_tags
            .as_ref()
            .map(|tags| tags.contains("minecraft:compasses", resource_id))
            .unwrap_or_else(|| {
                item_types::vanilla_item_resource_id_matches_type(
                    resource_id,
                    item_types::VANILLA_ITEM_COMPASS_ID,
                ) || item_types::vanilla_item_resource_id_matches_type(
                    resource_id,
                    item_types::VANILLA_ITEM_RECOVERY_COMPASS_ID,
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
        let item_model_id = self.item_model_id_for_stack(stack)?;
        let transforms = self
            .item_display_transforms
            .get(item_model_id)
            .or_else(|| self.mod_item_display_transforms.get(item_model_id))?;
        Some(transforms.get(context))
    }

    /// Effective client-item model key selected by
    /// `ItemModelResolver.getClientItem`. The stack's `ITEM_MODEL` patch
    /// overrides the item prototype; an explicit removal has no client item.
    pub fn item_model_id_for_stack<'a>(&'a self, stack: &'a ItemStackSummary) -> Option<&'a str> {
        let item_id = self.item_resource_id_for_protocol_id(stack.item_id?)?;
        item_model_id_for_stack(item_id, Some(&stack.component_patch))
    }

    /// Vanilla `ItemModelResolver.shouldPlaySwapAnimation`: missing client-item
    /// definitions keep the default `true` behavior.
    pub fn hand_animation_on_swap_for_stack(&self, stack: &ItemStackSummary) -> bool {
        self.item_model_id_for_stack(stack)
            .and_then(|item_model_id| self.client_item_properties.get(item_model_id))
            .is_none_or(|properties| properties.hand_animation_on_swap)
    }

    /// Vanilla `ItemModelResolver.swapAnimationScale`: missing client-item
    /// definitions keep the default full-height scale.
    pub fn swap_animation_scale_for_stack(&self, stack: &ItemStackSummary) -> f32 {
        self.item_model_id_for_stack(stack)
            .and_then(|item_model_id| self.client_item_properties.get(item_model_id))
            .map_or(1.0, |properties| properties.swap_animation_scale)
    }

    /// Summary-level equivalent of
    /// `ItemStack.matchesIgnoringComponents(...,
    /// DataComponentType::ignoreSwapAnimation)`. Vanilla 26.1 marks only
    /// `DAMAGE` with `ignoreSwapAnimation`; component membership still has to
    /// match, while differing damage values do not.
    pub fn stacks_match_ignoring_swap_animation(
        &self,
        visible: &ItemStackSummary,
        expected: &ItemStackSummary,
    ) -> bool {
        if visible == expected {
            return true;
        }
        if visible.count != expected.count || visible.item_id != expected.item_id {
            return false;
        }
        if item_stack_is_empty(visible) && item_stack_is_empty(expected) {
            return true;
        }
        let Some(item_id) = visible.item_id else {
            return false;
        };
        let Some(resource_id) = self.item_resource_id_for_protocol_id(item_id) else {
            return visible == expected;
        };
        let default_damage = self
            .default_component_codec_values_for_resource_id(resource_id)
            .is_some_and(|values| values.contains_key(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID));
        if effective_component_is_present(
            &visible.component_patch,
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            default_damage,
        ) != effective_component_is_present(
            &expected.component_patch,
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            default_damage,
        ) {
            return false;
        }

        component_patch_without_damage(&visible.component_patch)
            == component_patch_without_damage(&expected.component_patch)
    }

    /// Converts an already-selected ordinary leaf icon into the existing
    /// generated geometry payload without running the item selector again.
    pub fn generated_item_layers_for_icon(&self, icon: &ItemAtlasIcon) -> Vec<GeneratedItemLayer> {
        icon.layers
            .iter()
            .filter_map(|layer| {
                let mask = self.textures.alpha_mask(layer.texture_index)?;
                let asset_id = self.textures.texture_asset_id(layer.texture_index)?;
                Some(GeneratedItemLayer {
                    asset_id,
                    mask,
                    rect: ItemSpriteRect {
                        min: layer.uv.min,
                        max: layer.uv.max,
                    },
                    tint: layer.tint,
                    translucent: layer.translucent,
                })
            })
            .collect()
    }

    /// The single client-item resolver: selects the stack's model tree for
    /// `context` and returns its ordered leaves, keeping `minecraft:special`
    /// leaves (vanilla `ItemModelResolver.appendItemLayers`).
    pub fn resolve_item_model(
        &self,
        stack: &ItemStackSummary,
        context: &ItemModelContext<'_>,
    ) -> Option<ResolvedItemModel> {
        let protocol_id = stack.item_id?;
        let item_id = self.item_resource_id_for_protocol_id(protocol_id)?;
        let item_model_id = item_model_id_for_stack(item_id, Some(&stack.component_patch))?;
        let component_patch = Some(&*stack.component_patch);
        let keys = context.registry_keys;
        let attribute_keys = keys.attributes;
        let owner_team_color_rgb = context.owner_team_color_rgb;
        let default_max_damage = context
            .item_profiles
            .and_then(|profiles| profiles.item_max_damage_for_protocol_id(protocol_id));
        let default_max_stack_size = context
            .item_profiles
            .map(|profiles| {
                profiles.item_max_stack_size_for_protocol_id_with_registry(
                    protocol_id,
                    &self.content_registry,
                )
            })
            .or_else(|| {
                self.content_registry
                    .item_by_id(protocol_id)
                    .map(|spec| i32::from(spec.max_stack_size))
            });
        let time_wobbler = |model_id: &str,
                            state_id: u64,
                            source: TimeSource,
                            game_time: i64,
                            target_rotation: f32| {
            self.resolve_time_wobbler(model_id, state_id, source, game_time, target_rotation)
        };
        let time_random = || self.presentation_random.borrow_mut().next_float();
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
        let resolve_context = IconResolveContext {
            protocol_id,
            component_patch,
            registry_access: context.registry_access,
            item_profiles: context.item_profiles,
            default_component_codec_values: self
                .default_component_codec_values_for_resource_id(item_id),
            stack_count: stack.count,
            default_max_stack_size,
            default_max_damage,
            bundle_selected_item_index: context.bundle_selected_item_index,
            selected_item: context.selected_item,
            carried_item: context.carried_item,
            view_entity: context.view_entity,
            shift_down: context.shift_down,
            keybind_context: context.keybind,
            fishing_rod_cast: context.fishing_rod_cast,
            using_item: context.using_item,
            use_context: context.use_context,
            cooldown_progress: context.cooldown_progress,
            crossbow_charge: self.crossbow_charge_for(component_patch),
            display_context: item_display_context_name(context.display_context),
            display_context_kind: context.display_context,
            item_model_seed: context.seed,
            default_item_model_id: item_id,
            main_hand_left: context.owner_main_hand_left,
            context_dimension: context.context_dimension,
            context_entity_type: context.context_entity_type,
            owner_team_color_rgb,
            local_time_epoch_millis: self.local_time_epoch_millis(),
            local_time_formatter: self.local_time_formatter,
            time_context: context.time,
            stateful_model_id: item_model_id,
            time_wobbler: Some(&time_wobbler),
            time_random: Some(&time_random),
            compass_context: context.compass,
            compass_wobbler: Some(&compass_wobbler),
            compass_no_target_rotation: Some(&compass_no_target_rotation),
            item_resource_ids: self
                .registry
                .as_ref()
                .map(ItemRegistryCatalog::resource_ids),
            content_registry: Some(&self.content_registry),
            item_tags: self.item_tags.as_ref(),
            enchantment_tags: self.enchantment_tags.as_ref(),
            trim_material_tags: self.trim_material_tags.as_ref(),
            trim_pattern_tags: self.trim_pattern_tags.as_ref(),
            jukebox_song_tags: self.jukebox_song_tags.as_ref(),
            potion_tags: self.potion_tags.as_ref(),
            attribute_tags: self.attribute_tags.as_ref(),
            villager_type_tags: self.villager_type_tags.as_ref(),
            trim_material_keys: keys.trim_materials,
            trim_pattern_keys: keys.trim_patterns,
            enchantment_keys: keys.enchantments,
            attribute_keys,
        };
        let resolved_layers = self
            .item_icon_models
            .get(item_model_id)
            .or_else(|| self.mod_item_icon_models.get(item_model_id))
            .map(|model| self.resolved_layers_for_model(model, resolve_context, 0))
            .unwrap_or_else(|| {
                vec![ResolvedItemIconModelLayer::Ordinary {
                    layers: self.fallback_icon_texture_layers(),
                    transformation: ItemModelLeafTransformation::IDENTITY,
                    display_transform: None,
                }]
            });
        let layers = resolved_layers
            .into_iter()
            .filter_map(|layer| match layer {
                ResolvedItemIconModelLayer::Cuboid {
                    layers,
                    parts,
                    transformation,
                    display_transform,
                } => Some(ResolvedItemModelLayer::Cuboid {
                    icon: self.item_atlas_icon_for_layers(
                        layers,
                        component_patch,
                        owner_team_color_rgb,
                    )?,
                    parts: parts
                        .iter()
                        .map(|part| ResolvedItemCuboidPart {
                            model_id: part.model_id.clone(),
                            part: part.part,
                            geometry: part.geometry.clone(),
                            ground_model_extents: part.ground_model_extents,
                            tint: item_icon_tint_color(
                                &part.tint,
                                component_patch,
                                owner_team_color_rgb,
                            ),
                        })
                        .collect::<Vec<_>>()
                        .into(),
                    transformation,
                    display_transform,
                }),
                ResolvedItemIconModelLayer::Ordinary {
                    layers,
                    transformation,
                    display_transform,
                } => Some(ResolvedItemModelLayer::Ordinary {
                    icon: self.item_atlas_icon_for_layers(
                        layers,
                        component_patch,
                        owner_team_color_rgb,
                    )?,
                    transformation,
                    display_transform,
                }),
                ResolvedItemIconModelLayer::Special {
                    base_layers,
                    model,
                    transformation,
                    display_transform,
                } => Some(ResolvedItemModelLayer::Special {
                    model,
                    transformation,
                    display_transform,
                    base_icon: self.item_atlas_icon_for_layers(
                        base_layers,
                        component_patch,
                        owner_team_color_rgb,
                    ),
                }),
            })
            .collect::<Vec<_>>();
        (!layers.is_empty()).then_some(ResolvedItemModel { layers })
    }

    fn item_atlas_icon_for_layers(
        &self,
        layers: Vec<ItemIconTextureLayer>,
        component_patch: Option<&DataComponentPatchSummary>,
        owner_team_color_rgb: Option<u32>,
    ) -> Option<ItemAtlasIcon> {
        let layers = layers
            .into_iter()
            .filter_map(|layer| {
                self.textures
                    .texture_uv_rect(layer.texture_index)
                    .map(|uv| ItemAtlasIconLayer {
                        texture_index: layer.texture_index,
                        uv,
                        tint: item_icon_tint_color(
                            &layer.tint,
                            component_patch,
                            owner_team_color_rgb,
                        ),
                        translucent: layer.translucent,
                    })
            })
            .collect::<Vec<_>>();
        (!layers.is_empty()).then_some(ItemAtlasIcon { layers })
    }

    pub(super) fn resolved_layers_for_model(
        &self,
        model: &ItemIconModel,
        context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ResolvedItemIconModelLayer> {
        if depth >= ITEM_ICON_RECURSION_LIMIT {
            return Vec::new();
        }
        let mut resolve_bundle_selected_item =
            || self.bundle_selected_item_resolved_layers(context, depth + 1);
        model.resolved_layers_with_bundle_resolver(
            context,
            glam::Mat4::IDENTITY,
            &mut resolve_bundle_selected_item,
        )
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
            self.item_resource_id_for_protocol_id(template.item_id)
                .is_some_and(|resource_id| {
                    item_types::vanilla_item_resource_id_matches_type(
                        resource_id,
                        item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID,
                    )
                })
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
                let target_rotation = self.presentation_random.borrow_mut().next_float();
                wobbler.update(game_time, target_rotation, 0.8);
            }
            wobbler.rotation
        } else {
            self.presentation_random.borrow_mut().next_float()
        };
        (rotation + compass_seed_offset(seed)).rem_euclid(1.0)
    }

    pub fn item_model_use_context_for_stack(
        &self,
        stack: &ItemStackSummary,
        elapsed_ticks: u32,
    ) -> ItemModelUseContext {
        self.item_model_use_context_for_stack_with_crossbow_charge_duration(
            stack,
            elapsed_ticks,
            None,
        )
    }

    /// Builds the use context with a caller-projected synchronized
    /// `minecraft:crossbow_charge_time` result. Callers without a world
    /// registry may pass `None` for vanilla's 25-tick base duration.
    pub fn item_model_use_context_for_stack_with_crossbow_charge_duration(
        &self,
        stack: &ItemStackSummary,
        elapsed_ticks: u32,
        synchronized_crossbow_charge_duration_ticks: Option<i32>,
    ) -> ItemModelUseContext {
        let Some(item_id) = stack
            .item_id
            .and_then(|protocol_id| self.item_resource_id_for_protocol_id(protocol_id))
        else {
            return ItemModelUseContext::inactive();
        };
        ItemModelUseContext::active(
            elapsed_ticks,
            item_use_duration_ticks(item_id, &stack.component_patch),
            crossbow_charge_duration_ticks(item_id, synchronized_crossbow_charge_duration_ticks),
        )
    }

    pub(super) fn bundle_selected_item_resolved_layers(
        &self,
        context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ResolvedItemIconModelLayer> {
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
        self.item_template_resolved_layers(template, context, depth)
    }

    pub(super) fn item_template_resolved_layers(
        &self,
        template: &ItemStackTemplateSummary,
        parent_context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ResolvedItemIconModelLayer> {
        let Some(item_id) = self.item_resource_id_for_protocol_id(template.item_id) else {
            return Vec::new();
        };
        let Some(item_model_id) = item_model_id_for_stack(item_id, Some(&template.component_patch))
        else {
            return Vec::new();
        };
        let default_max_damage = parent_context
            .item_profiles
            .and_then(|profiles| profiles.item_max_damage_for_protocol_id(template.item_id));
        let default_max_stack_size = parent_context
            .item_profiles
            .map(|profiles| {
                profiles.item_max_stack_size_for_protocol_id_with_registry(
                    template.item_id,
                    &self.content_registry,
                )
            })
            .or_else(|| {
                self.content_registry
                    .item_by_id(template.item_id)
                    .map(|spec| i32::from(spec.max_stack_size))
            });
        let context = IconResolveContext {
            protocol_id: template.item_id,
            component_patch: Some(&template.component_patch),
            registry_access: parent_context.registry_access,
            item_profiles: parent_context.item_profiles,
            default_component_codec_values: self
                .default_component_codec_values_for_resource_id(item_id),
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
            display_context_kind: parent_context.display_context_kind,
            item_model_seed: parent_context.item_model_seed,
            default_item_model_id: item_id,
            main_hand_left: parent_context.main_hand_left,
            context_dimension: parent_context.context_dimension,
            context_entity_type: parent_context.context_entity_type,
            owner_team_color_rgb: parent_context.owner_team_color_rgb,
            local_time_epoch_millis: parent_context.local_time_epoch_millis,
            local_time_formatter: parent_context.local_time_formatter,
            time_context: parent_context.time_context,
            stateful_model_id: item_model_id,
            time_wobbler: parent_context.time_wobbler,
            time_random: parent_context.time_random,
            compass_context: parent_context.compass_context,
            compass_wobbler: parent_context.compass_wobbler,
            compass_no_target_rotation: parent_context.compass_no_target_rotation,
            item_resource_ids: parent_context.item_resource_ids,
            content_registry: parent_context.content_registry,
            item_tags: parent_context.item_tags,
            enchantment_tags: parent_context.enchantment_tags,
            trim_material_tags: parent_context.trim_material_tags,
            trim_pattern_tags: parent_context.trim_pattern_tags,
            jukebox_song_tags: parent_context.jukebox_song_tags,
            potion_tags: parent_context.potion_tags,
            attribute_tags: parent_context.attribute_tags,
            villager_type_tags: parent_context.villager_type_tags,
            trim_material_keys: parent_context.trim_material_keys,
            trim_pattern_keys: parent_context.trim_pattern_keys,
            enchantment_keys: parent_context.enchantment_keys,
            attribute_keys: parent_context.attribute_keys,
        };
        let layers = self
            .item_icon_models
            .get(item_model_id)
            .or_else(|| self.mod_item_icon_models.get(item_model_id))
            .map(|model| self.resolved_layers_for_model(model, context, depth))
            .unwrap_or_else(|| {
                vec![ResolvedItemIconModelLayer::Ordinary {
                    layers: self.fallback_icon_texture_layers(),
                    transformation: ItemModelLeafTransformation::IDENTITY,
                    display_transform: None,
                }]
            });
        layers
            .into_iter()
            .map(|layer| match layer {
                ResolvedItemIconModelLayer::Cuboid {
                    layers,
                    parts,
                    transformation,
                    display_transform,
                } => ResolvedItemIconModelLayer::Cuboid {
                    layers: resolve_item_icon_texture_layer_tints(
                        layers,
                        Some(&template.component_patch),
                        parent_context.owner_team_color_rgb,
                    ),
                    parts: parts
                        .iter()
                        .map(|part| super::cuboid::CuboidPart {
                            tint: ItemIconTint::Static(item_icon_tint_color(
                                &part.tint,
                                Some(&template.component_patch),
                                parent_context.owner_team_color_rgb,
                            )),
                            ..part.clone()
                        })
                        .collect::<Vec<_>>()
                        .into(),
                    transformation,
                    display_transform,
                },
                ResolvedItemIconModelLayer::Ordinary {
                    layers,
                    transformation,
                    display_transform,
                } => ResolvedItemIconModelLayer::Ordinary {
                    layers: resolve_item_icon_texture_layer_tints(
                        layers,
                        Some(&template.component_patch),
                        parent_context.owner_team_color_rgb,
                    ),
                    transformation,
                    display_transform,
                },
                ResolvedItemIconModelLayer::Special {
                    base_layers,
                    model,
                    transformation,
                    display_transform,
                } => ResolvedItemIconModelLayer::Special {
                    base_layers: resolve_item_icon_texture_layer_tints(
                        base_layers,
                        Some(&template.component_patch),
                        parent_context.owner_team_color_rgb,
                    ),
                    model,
                    transformation,
                    display_transform,
                },
            })
            .collect()
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
            translucent: self
                .textures
                .texture_has_translucent(self.textures.fallback_index()),
        }]
    }
}
