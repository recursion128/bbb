use super::*;
pub(super) fn protocol_ids_for_resource_ids(
    registry: &ItemRegistryCatalog,
    resource_ids: &[&str],
) -> BTreeSet<i32> {
    resource_ids
        .iter()
        .filter_map(|resource_id| registry.protocol_id(resource_id))
        .collect()
}

impl NativeItemRuntime {
    pub fn default_component_codec_values_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, BTreeMap<i32, serde_json::Value>> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_component_codec_values_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn map_background_textures(&self) -> &[FirstPersonMapBackgroundTexture] {
        &self.map_background_textures
    }

    pub fn map_decoration_textures(&self) -> &[ItemFrameMapDecorationTexture] {
        &self.map_decoration_textures
    }

    pub fn map_text_glyphs(&self) -> Option<&HudFontGlyphMap> {
        self.map_text_fonts.as_ref().map(|fonts| fonts.glyphs(None))
    }

    pub fn map_text_fonts(&self) -> Option<&HudFontGlyphBook> {
        self.map_text_fonts.as_ref()
    }

    pub fn item_crafting_remainders_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::crafting_remainders_by_protocol_id)
            .unwrap_or_default()
    }

    pub fn item_crafting_remainder_count(&self) -> usize {
        self.item_crafting_remainders_by_protocol_id().len()
    }

    /// Pack catalog used to build the generation-matched renderer equipment
    /// texture catalog during startup and atomic resource reload.
    pub fn equipment_asset_catalog(&self) -> &EquipmentAssetCatalog {
        &self.equipment_assets
    }

    /// Resolves an asset resource id to its generation-local compact identity.
    pub fn equipment_asset_id(&self, asset_id: &str) -> Option<EntityEquipmentAssetId> {
        self.equipment_assets
            .asset_index(asset_id)
            .map(|index| EntityEquipmentAssetId::new(self.resource_generation, index))
    }

    /// Normalized asset names in the same order as compact identities.
    pub fn equipment_asset_names(&self) -> Vec<String> {
        self.equipment_assets
            .indexed_assets()
            .map(|(_index, asset_id, _asset)| asset_id.to_owned())
            .collect()
    }

    pub fn furnace_fuel_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.furnace_fuel_item_ids.clone()
    }

    pub fn furnace_fuel_item_count(&self) -> usize {
        self.furnace_fuel_item_ids.len()
    }

    pub fn brewing_potion_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| protocol_ids_for_resource_ids(registry, BREWING_POTION_ITEM_IDS))
            .unwrap_or_default()
    }

    pub fn brewing_potion_item_count(&self) -> usize {
        self.brewing_potion_item_ids_by_protocol_id().len()
    }

    pub fn brewing_ingredient_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| protocol_ids_for_resource_ids(registry, BREWING_INGREDIENT_ITEM_IDS))
            .unwrap_or_default()
    }

    pub fn brewing_ingredient_item_count(&self) -> usize {
        self.brewing_ingredient_item_ids_by_protocol_id().len()
    }

    pub fn enchantment_lapis_lazuli_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| {
                protocol_ids_for_resource_ids(registry, ENCHANTMENT_LAPIS_LAZULI_ITEM_IDS)
            })
            .unwrap_or_default()
    }

    pub fn enchantment_lapis_lazuli_item_count(&self) -> usize {
        self.enchantment_lapis_lazuli_item_ids_by_protocol_id()
            .len()
    }

    pub fn cartography_additional_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        let Some(registry) = &self.registry else {
            return BTreeSet::new();
        };
        let item_ids = protocol_ids_for_resource_ids(registry, CARTOGRAPHY_ADDITIONAL_ITEM_IDS);
        if item_ids.len() == CARTOGRAPHY_ADDITIONAL_ITEM_IDS.len() {
            item_ids
        } else {
            BTreeSet::new()
        }
    }

    pub fn cartography_additional_item_count(&self) -> usize {
        self.cartography_additional_item_ids_by_protocol_id().len()
    }

    pub fn freeze_immune_wearable_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.freeze_immune_wearable_item_ids.clone()
    }

    pub fn freeze_immune_wearable_item_count(&self) -> usize {
        self.freeze_immune_wearable_item_ids.len()
    }

    pub fn powder_snow_walkable_foot_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.powder_snow_walkable_foot_item_ids.clone()
    }

    pub fn powder_snow_walkable_foot_item_count(&self) -> usize {
        self.powder_snow_walkable_foot_item_ids.len()
    }

    pub fn resolved_model_count(&self) -> usize {
        self.resolved_model_count
    }

    pub fn missing_model_count(&self) -> usize {
        self.missing_model_ids.len()
    }

    pub fn missing_texture_count(&self) -> usize {
        self.missing_texture_ids.len()
    }

    pub fn texture_count(&self) -> usize {
        self.textures.texture_count()
    }

    pub fn icon_texture_count(&self) -> usize {
        self.item_icon_models.len()
    }

    pub fn atlas_size(&self) -> (u32, u32) {
        self.textures.atlas_size()
    }

    pub fn atlas_rgba(&self) -> &[u8] {
        self.textures.atlas_rgba()
    }

    /// Releases the stitched item-atlas CPU pixel copy once uploaded to GPU.
    pub fn release_atlas_rgba(&mut self) {
        self.textures.release_atlas_rgba();
    }

    pub fn has_texture_animation(&self) -> bool {
        self.textures.has_animation()
    }

    pub fn texture_animation_atlas_patches(
        &self,
        previous_tick: u64,
        tick: u64,
    ) -> Result<Option<Vec<bbb_client_assets::AtlasMipPatch>>> {
        self.textures.animation_atlas_patches(previous_tick, tick)
    }

    pub fn atlas_sprite_uvs(&self) -> Vec<ItemAtlasSpriteUv> {
        self.textures.sprite_uvs()
    }

    pub fn texture_index(&self, texture_id: &str) -> u32 {
        self.textures.texture_index(texture_id)
    }

    /// Item-atlas index of `texture_id`, without the missing-texture fallback.
    pub fn atlas_sprite_index(&self, texture_id: &str) -> Option<u32> {
        self.textures.texture_indices.get(texture_id).copied()
    }

    pub fn item_resource_id_for_protocol_id(&self, protocol_id: i32) -> Option<&str> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.resource_id(protocol_id))
            .or_else(|| {
                self.content_registry
                    .item_by_id(protocol_id)
                    .map(|spec| spec.resource_id())
            })
    }

    pub fn default_component_codec_values_for_resource_id(
        &self,
        resource_id: &str,
    ) -> Option<&BTreeMap<i32, serde_json::Value>> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.default_component_codec_values(resource_id))
    }

    pub fn language(&self) -> &LanguageCatalog {
        &self.language
    }

    pub fn with_language(mut self, language: LanguageCatalog) -> Self {
        self.language = language;
        self
    }

    pub fn bound_key_lookup(&self) -> Option<BoundKeyLookupFn> {
        self.bound_key_lookup
    }

    pub fn enchantment_tags(&self) -> Option<&TagCatalog> {
        self.enchantment_tags.as_ref()
    }

    pub fn jukebox_songs(&self) -> &JukeboxSongRegistry {
        &self.jukebox_songs
    }

    pub fn item_registry(&self) -> Option<&ItemRegistryCatalog> {
        self.registry.as_ref()
    }
}
