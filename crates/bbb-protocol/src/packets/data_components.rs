use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{chunks, normalize_resource_location, read_resource_location};
use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::{
        decode_component_summary_from_decoder, decode_styled_component_summary_from_decoder,
        skip_nbt_tag_from_decoder, styled_runs_summary_text, StyledTextRun,
    },
};
use uuid::Uuid;

pub(crate) const MAX_DATA_COMPONENT_PATCH_ENTRIES: usize = 1024;
pub(crate) const MAX_DATA_COMPONENT_PREDICATE_ENTRIES: usize = 1024;
const MAX_DATA_COMPONENT_LIST_ITEMS: usize = 4096;
const MAX_BLOCK_STATE_PROPERTIES: usize = 256;
const MAX_BOOK_PAGES: usize = 100;
const MAX_CONTAINER_ITEMS: usize = 256;
const MAX_FIREWORK_EXPLOSIONS: usize = 256;
const MAX_LORE_LINES: usize = 256;
const MAX_MOB_EFFECT_DETAILS_DEPTH: usize = 16;
const MAX_NBT_ARRAY_ITEMS: usize = 4096;
const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_LIST_ITEMS: usize = 4096;
const MAX_PARTIAL_DATA_COMPONENT_PREDICATES: usize = 64;
const MAX_PLAYER_NAME_CHARS: usize = 16;
const MAX_POT_DECORATIONS: usize = 4;
const MAX_PROFILE_PROPERTIES: usize = 16;
const MAX_PROFILE_PROPERTY_NAME_CHARS: usize = 64;
const MAX_PROFILE_SIGNATURE_CHARS: usize = 1024;
const MAX_STRING_CHARS: usize = 32767;
const MAX_WRITABLE_BOOK_PAGE_CHARS: usize = 1024;
const MAX_WRITTEN_BOOK_TITLE_CHARS: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataComponentPatchSummary {
    pub added: usize,
    #[serde(default)]
    pub added_type_ids: Vec<i32>,
    pub removed_type_ids: Vec<i32>,
    #[serde(default)]
    pub custom_data: Option<NbtSummaryValue>,
    #[serde(default)]
    pub max_stack_size: Option<i32>,
    #[serde(default)]
    pub max_damage: Option<i32>,
    #[serde(default)]
    pub damage: Option<i32>,
    #[serde(default)]
    pub intangible_projectile: bool,
    #[serde(default)]
    pub unbreakable: bool,
    #[serde(default)]
    pub ominous_bottle_amplifier: Option<i32>,
    #[serde(default)]
    pub custom_name: Option<String>,
    /// Styled-run projection of `custom_name` (same decode, style kept);
    /// `custom_name` is its plain-text concatenation.
    #[serde(default)]
    pub custom_name_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub item_name: Option<String>,
    /// Styled-run projection of `item_name` (same decode, style kept).
    #[serde(default)]
    pub item_name_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub item_model: Option<String>,
    #[serde(default)]
    pub lore: Vec<String>,
    /// Styled-run projection of `lore` (same decode, style kept), one run
    /// list per lore line.
    #[serde(default)]
    pub lore_styled: Vec<Vec<StyledTextRun>>,
    #[serde(default)]
    pub rarity: Option<ItemRaritySummary>,
    #[serde(default)]
    pub use_cooldown_ticks: Option<i32>,
    #[serde(default)]
    pub use_cooldown_group: Option<String>,
    #[serde(default)]
    pub use_effects: Option<UseEffectsSummary>,
    #[serde(default)]
    pub consumable: Option<ConsumableSummary>,
    #[serde(default)]
    pub attack_range: Option<AttackRangeSummary>,
    #[serde(default)]
    pub swing_animation: Option<SwingAnimationSummary>,
    #[serde(default)]
    pub custom_model_data_floats: CustomModelDataFloats,
    #[serde(default)]
    pub custom_model_data_flags: Vec<bool>,
    #[serde(default)]
    pub custom_model_data_strings: Vec<String>,
    #[serde(default)]
    pub custom_model_data_colors: Vec<i32>,
    #[serde(default)]
    pub tooltip_hide_tooltip: bool,
    #[serde(default)]
    pub tooltip_hidden_component_type_ids: Vec<i32>,
    #[serde(default)]
    pub dyed_color: Option<i32>,
    #[serde(default)]
    pub map_color: Option<i32>,
    #[serde(default)]
    pub potion_custom_color: Option<i32>,
    #[serde(default)]
    pub potion_id: Option<i32>,
    #[serde(default)]
    pub potion_custom_effect_count: Option<usize>,
    #[serde(default)]
    pub potion_custom_effects: Vec<MobEffectInstanceSummary>,
    #[serde(default)]
    pub potion_custom_name: Option<String>,
    #[serde(default)]
    pub suspicious_stew_effects: Vec<SuspiciousStewEffectSummary>,
    #[serde(default)]
    pub firework_explosion_colors: Vec<i32>,
    #[serde(default)]
    pub firework_explosion_fade_colors: Vec<i32>,
    #[serde(default)]
    pub firework_explosion_shape: Option<FireworkExplosionShapeSummary>,
    #[serde(default)]
    pub firework_explosion_has_trail: Option<bool>,
    #[serde(default)]
    pub firework_explosion_has_twinkle: Option<bool>,
    #[serde(default)]
    pub fireworks_flight_duration: Option<i32>,
    #[serde(default)]
    pub fireworks_explosions_count: Option<usize>,
    #[serde(default)]
    pub fireworks_explosions: Vec<FireworkExplosionSummary>,
    #[serde(default)]
    pub charged_projectiles_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub bundle_contents_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub bundle_contents_item_count: Option<usize>,
    #[serde(default)]
    pub container_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub container_item_count: Option<usize>,
    #[serde(default)]
    pub container_loot: bool,
    #[serde(default)]
    pub banner_pattern_layers: Vec<BannerPatternLayerSummary>,
    #[serde(default)]
    pub pot_decorations_item_ids: Vec<i32>,
    #[serde(default)]
    pub bees_present: bool,
    #[serde(default)]
    pub bees_count: usize,
    #[serde(default)]
    pub enchantments: Vec<ItemEnchantmentSummary>,
    #[serde(default)]
    pub stored_enchantments: Vec<ItemEnchantmentSummary>,
    #[serde(default)]
    pub attribute_modifiers: Vec<AttributeModifierSummary>,
    #[serde(default)]
    pub enchantment_glint_override: Option<bool>,
    #[serde(default)]
    pub armor_trim_material_id: Option<i32>,
    #[serde(default)]
    pub armor_trim_material_direct: Option<Box<TrimMaterialSummary>>,
    #[serde(default)]
    pub armor_trim_pattern_id: Option<i32>,
    #[serde(default)]
    pub armor_trim_pattern_direct: Option<Box<TrimPatternSummary>>,
    #[serde(default)]
    pub tropical_fish_pattern_id: Option<i32>,
    #[serde(default)]
    pub tropical_fish_base_color_id: Option<i32>,
    #[serde(default)]
    pub tropical_fish_pattern_color_id: Option<i32>,
    #[serde(default)]
    pub instrument_id: Option<i32>,
    #[serde(default)]
    pub instrument_description: Option<String>,
    #[serde(default)]
    pub instrument_description_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub jukebox_song_id: Option<i32>,
    #[serde(default)]
    pub jukebox_direct_song: Option<JukeboxSongSummary>,
    #[serde(default)]
    pub map_id: Option<i32>,
    #[serde(default)]
    pub map_post_processing: Option<MapPostProcessingSummary>,
    #[serde(default)]
    pub writable_book_pages: Vec<String>,
    #[serde(default)]
    pub writable_book_page_filters: Vec<Option<String>>,
    #[serde(default)]
    pub written_book: Option<WrittenBookContentSummary>,
    #[serde(default)]
    pub block_state_properties: BTreeMap<String, String>,
    #[serde(default)]
    pub profile: Option<ResolvableProfileSummary>,
    #[serde(default)]
    pub villager_variant_id: Option<i32>,
    #[serde(default)]
    pub lodestone_target: Option<LodestoneTargetSummary>,
    #[serde(default)]
    pub lodestone_tracked: Option<bool>,
    #[serde(default)]
    pub painting_variant_id: Option<i32>,
    #[serde(default)]
    pub painting_variant_direct: Option<PaintingVariantSummary>,
    #[serde(default)]
    pub block_entity_spawn_entity_type: Option<String>,
    #[serde(default)]
    pub entity_data_entity_type_id: Option<i32>,
    #[serde(default)]
    pub can_place_on: Option<AdventureModePredicateSummary>,
    #[serde(default)]
    pub can_break: Option<AdventureModePredicateSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum NbtSummaryValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtSummaryValue>),
    Compound(Vec<NbtSummaryEntry>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtSummaryEntry {
    pub name: String,
    pub value: NbtSummaryValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LodestoneTargetSummary {
    pub dimension: String,
    pub pos: chunks::BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxSongSummary {
    pub sound_event: SoundEventSummary,
    pub description: String,
    pub length_in_seconds_bits: u32,
    pub comparator_output: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundEventSummary {
    pub registry_id: Option<i32>,
    pub sound_id: Option<String>,
    pub fixed_range_bits: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrimMaterialSummary {
    pub asset_name: String,
    pub override_armor_assets: BTreeMap<String, String>,
    pub description: String,
    #[serde(default)]
    pub description_styled: Option<Box<[StyledTextRun]>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrimPatternSummary {
    pub asset_id: String,
    pub description: String,
    #[serde(default)]
    pub description_styled: Option<Box<[StyledTextRun]>>,
    pub decal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaintingVariantSummary {
    pub width: i32,
    pub height: i32,
    pub asset_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub title_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub author_styled: Option<Vec<StyledTextRun>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureModePredicateSummary {
    pub unknown: bool,
    #[serde(default)]
    pub block_registry_ids: Vec<i32>,
    #[serde(default)]
    pub unresolved_block_tag_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerPatternLayerSummary {
    pub registry_id: Option<i32>,
    pub translation_key: Option<String>,
    pub color_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectInstanceSummary {
    pub effect_id: i32,
    pub amplifier: i32,
    pub duration: i32,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[serde(default)]
    pub hidden_effect: Option<Box<MobEffectDetailsSummary>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspiciousStewEffectSummary {
    pub effect_id: i32,
    pub duration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectDetailsSummary {
    pub amplifier: i32,
    pub duration: i32,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[serde(default)]
    pub hidden_effect: Option<Box<MobEffectDetailsSummary>>,
}

/// The `floats` list of a `minecraft:custom_model_data` component, preserved so
/// the `minecraft:custom_model_data` range-dispatch item-model property can read
/// `CustomModelData.getFloat(index)` during icon resolution. Equality is bit-exact
/// (mirroring [`AttackRangeSummary`]) so the enclosing summary can keep deriving `Eq`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CustomModelDataFloats(pub Vec<f32>);

impl PartialEq for CustomModelDataFloats {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(&other.0)
                .all(|(left, right)| left.to_bits() == right.to_bits())
    }
}

impl Eq for CustomModelDataFloats {}

impl From<Vec<f32>> for CustomModelDataFloats {
    fn from(values: Vec<f32>) -> Self {
        Self(values)
    }
}

impl std::ops::Deref for CustomModelDataFloats {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AttackRangeSummary {
    pub min_reach: f32,
    pub max_reach: f32,
    pub min_creative_reach: f32,
    pub max_creative_reach: f32,
    pub hitbox_margin: f32,
    pub mob_factor: f32,
}

impl PartialEq for AttackRangeSummary {
    fn eq(&self, other: &Self) -> bool {
        self.min_reach.to_bits() == other.min_reach.to_bits()
            && self.max_reach.to_bits() == other.max_reach.to_bits()
            && self.min_creative_reach.to_bits() == other.min_creative_reach.to_bits()
            && self.max_creative_reach.to_bits() == other.max_creative_reach.to_bits()
            && self.hitbox_margin.to_bits() == other.hitbox_margin.to_bits()
            && self.mob_factor.to_bits() == other.mob_factor.to_bits()
    }
}

impl Eq for AttackRangeSummary {}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct UseEffectsSummary {
    pub can_sprint: bool,
    pub interact_vibrations: bool,
    pub speed_multiplier: f32,
}

impl PartialEq for UseEffectsSummary {
    fn eq(&self, other: &Self) -> bool {
        self.can_sprint == other.can_sprint
            && self.interact_vibrations == other.interact_vibrations
            && self.speed_multiplier.to_bits() == other.speed_multiplier.to_bits()
    }
}

impl Eq for UseEffectsSummary {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConsumableSummary {
    pub consume_seconds: f32,
    #[serde(default)]
    pub animation: ItemUseAnimationSummary,
}

impl Default for ConsumableSummary {
    fn default() -> Self {
        Self {
            consume_seconds: 0.0,
            animation: ItemUseAnimationSummary::default(),
        }
    }
}

impl PartialEq for ConsumableSummary {
    fn eq(&self, other: &Self) -> bool {
        self.consume_seconds.to_bits() == other.consume_seconds.to_bits()
            && self.animation == other.animation
    }
}

impl Eq for ConsumableSummary {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemUseAnimationSummary {
    None,
    Eat,
    Drink,
    Block,
    Bow,
    Trident,
    Crossbow,
    Spyglass,
    TootHorn,
    Brush,
    Bundle,
    Spear,
}

impl Default for ItemUseAnimationSummary {
    fn default() -> Self {
        Self::Eat
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwingAnimationSummary {
    pub animation_type: SwingAnimationTypeSummary,
    pub duration: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwingAnimationTypeSummary {
    None,
    Whack,
    Stab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireworkExplosionShapeSummary {
    SmallBall,
    LargeBall,
    Star,
    Creeper,
    Burst,
}

impl FireworkExplosionShapeSummary {
    fn from_vanilla_id(id: i32) -> Self {
        match id {
            1 => Self::LargeBall,
            2 => Self::Star,
            3 => Self::Creeper,
            4 => Self::Burst,
            _ => Self::SmallBall,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireworkExplosionSummary {
    pub shape: FireworkExplosionShapeSummary,
    pub colors: Vec<i32>,
    #[serde(default)]
    pub fade_colors: Vec<i32>,
    pub has_trail: bool,
    pub has_twinkle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemEnchantmentSummary {
    pub holder_id: i32,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeModifierSummary {
    pub attribute_id: i32,
    pub modifier_id: String,
    pub amount_bits: u64,
    pub operation_id: i32,
    pub slot_id: i32,
    #[serde(default)]
    pub display_id: i32,
    #[serde(default)]
    pub display_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrittenBookContentSummary {
    pub title: String,
    #[serde(default)]
    pub title_filter: Option<String>,
    pub author: String,
    pub generation: i32,
    pub pages: Vec<String>,
    #[serde(default)]
    pub page_filters: Vec<Option<String>>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRaritySummary {
    Common,
    Uncommon,
    Rare,
    Epic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapPostProcessingSummary {
    Lock,
    Scale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackTemplateSummary {
    pub item_id: i32,
    pub count: i32,
    pub component_patch: DataComponentPatchSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvableProfileSummary {
    pub kind: ResolvableProfileKindSummary,
    pub uuid: Option<Uuid>,
    pub name: Option<String>,
    pub properties: Vec<GameProfilePropertySummary>,
    #[serde(default)]
    pub profile_textures: Option<ProfileTexturesSummary>,
    pub skin_patch: PlayerSkinPatchSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvableProfileKindSummary {
    GameProfile,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfilePropertySummary {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSkinPatchSummary {
    pub body: Option<ResourceTextureSummary>,
    pub cape: Option<ResourceTextureSummary>,
    pub elytra: Option<ResourceTextureSummary>,
    pub model: Option<PlayerModelTypeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceTextureSummary {
    pub asset_id: String,
    pub texture_path: String,
}

pub use super::profile_textures::{
    decode_profile_textures_from_properties, PlayerModelTypeSummary, ProfileSkinTextureSummary,
    ProfileTextureSummary, ProfileTexturesSummary,
};

pub(crate) fn decode_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    let added = decoder.read_len()?;
    let removed = decoder.read_len()?;
    if added + removed > MAX_DATA_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            added + removed,
            MAX_DATA_COMPONENT_PATCH_ENTRIES,
        ));
    }

    let mut summary = decode_typed_data_component_patch_summary(decoder, added)?;
    let mut removed_type_ids = Vec::with_capacity(removed);
    for _ in 0..removed {
        removed_type_ids.push(decoder.read_var_i32()?);
    }

    summary.added = added;
    summary.removed_type_ids = removed_type_ids;
    Ok(summary)
}

pub(crate) fn decode_data_component_exact_predicate_type_ids(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<i32>> {
    let component_count = decoder.read_len()?;
    if component_count > MAX_DATA_COMPONENT_PREDICATE_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            component_count,
            MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
        ));
    }
    decode_typed_data_component_list(decoder, component_count)
}

fn decode_typed_data_component_list(decoder: &mut Decoder<'_>, count: usize) -> Result<Vec<i32>> {
    let mut type_ids = Vec::with_capacity(count);
    for _ in 0..count {
        let type_id = decoder.read_var_i32()?;
        decode_data_component_value(decoder, type_id)?;
        type_ids.push(type_id);
    }
    Ok(type_ids)
}

fn decode_typed_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
    count: usize,
) -> Result<DataComponentPatchSummary> {
    let mut summary = DataComponentPatchSummary {
        added_type_ids: Vec::with_capacity(count),
        ..DataComponentPatchSummary::default()
    };
    for _ in 0..count {
        let type_id = decoder.read_var_i32()?;
        match type_id {
            0 => {
                summary.custom_data = decode_nbt_summary_from_decoder(decoder)?;
            }
            1 => {
                summary.max_stack_size = Some(decoder.read_var_i32()?);
            }
            2 => {
                summary.max_damage = Some(decoder.read_var_i32()?);
            }
            3 => {
                summary.damage = Some(decoder.read_var_i32()?);
            }
            4 => {
                summary.unbreakable = true;
            }
            22 => {
                summary.intangible_projectile = true;
                skip_nbt_tag_from_decoder(decoder)?;
            }
            63 => {
                summary.ominous_bottle_amplifier = Some(decoder.read_var_i32()?);
            }
            6 => {
                let runs = decode_styled_component_summary_from_decoder(decoder)?;
                summary.custom_name = Some(styled_runs_summary_text(&runs));
                summary.custom_name_styled = Some(runs);
            }
            9 => {
                let runs = decode_styled_component_summary_from_decoder(decoder)?;
                summary.item_name = Some(styled_runs_summary_text(&runs));
                summary.item_name_styled = Some(runs);
            }
            10 => {
                summary.item_model = Some(read_resource_location(decoder)?);
            }
            11 => {
                summary.lore_styled = decode_lore(decoder)?;
                summary.lore = summary
                    .lore_styled
                    .iter()
                    .map(|line| styled_runs_summary_text(line))
                    .collect();
            }
            12 => {
                summary.rarity = Some(decode_item_rarity(decoder)?);
            }
            26 => {
                let cooldown = decode_use_cooldown_summary(decoder)?;
                summary.use_cooldown_ticks = Some(cooldown.ticks);
                summary.use_cooldown_group = cooldown.cooldown_group;
            }
            5 => {
                summary.use_effects = Some(decode_use_effects_summary(decoder)?);
            }
            24 => {
                summary.consumable = Some(decode_consumable_summary(decoder)?);
            }
            30 => {
                summary.attack_range = Some(decode_attack_range_summary(decoder)?);
            }
            40 => {
                summary.swing_animation = Some(decode_swing_animation(decoder)?);
            }
            17 => {
                let (floats, flags, strings, colors) = decode_custom_model_data(decoder)?;
                summary.custom_model_data_floats = floats.into();
                summary.custom_model_data_flags = flags;
                summary.custom_model_data_strings = strings;
                summary.custom_model_data_colors = colors;
            }
            18 => {
                let (hide_tooltip, hidden_components) = decode_tooltip_display_summary(decoder)?;
                summary.tooltip_hide_tooltip = hide_tooltip;
                summary.tooltip_hidden_component_type_ids = hidden_components;
            }
            44 => {
                summary.dyed_color = Some(decoder.read_i32()?);
            }
            45 => {
                summary.map_color = Some(decoder.read_i32()?);
            }
            49 => {
                summary.charged_projectiles_items =
                    decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            }
            56 => {
                let trim = decode_armor_trim(decoder)?;
                summary.armor_trim_material_id = trim.material_id;
                summary.armor_trim_material_direct = trim.material_direct.map(Box::new);
                summary.armor_trim_pattern_id = trim.pattern_id;
                summary.armor_trim_pattern_direct = trim.pattern_direct.map(Box::new);
            }
            88 => {
                summary.tropical_fish_pattern_id = Some(decoder.read_var_i32()?);
            }
            89 => {
                summary.tropical_fish_base_color_id = Some(decoder.read_var_i32()?);
            }
            90 => {
                summary.tropical_fish_pattern_color_id = Some(decoder.read_var_i32()?);
            }
            61 => {
                let instrument = decode_instrument_component_summary(decoder)?;
                summary.instrument_id = instrument.registry_id;
                summary.instrument_description = instrument.description;
                summary.instrument_description_styled = instrument.description_styled;
            }
            64 => {
                let song = decode_jukebox_song_holder(decoder)?;
                summary.jukebox_song_id = song.registry_id;
                summary.jukebox_direct_song = song.direct;
            }
            50 => {
                summary.bundle_contents_items =
                    decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
                summary.bundle_contents_item_count = Some(summary.bundle_contents_items.len());
            }
            51 => {
                let potion = decode_potion_contents(decoder)?;
                summary.potion_id = potion.potion_id;
                summary.potion_custom_color = potion.custom_color;
                summary.potion_custom_effect_count = Some(potion.custom_effects.len());
                summary.potion_custom_effects = potion.custom_effects;
                summary.potion_custom_name = potion.custom_name;
            }
            53 => {
                summary.suspicious_stew_effects = decode_suspicious_stew_effects(decoder)?;
            }
            68 => {
                let explosion = decode_firework_explosion(decoder)?;
                summary.firework_explosion_colors = explosion.colors;
                summary.firework_explosion_fade_colors = explosion.fade_colors;
                summary.firework_explosion_shape = Some(explosion.shape);
                summary.firework_explosion_has_trail = Some(explosion.has_trail);
                summary.firework_explosion_has_twinkle = Some(explosion.has_twinkle);
            }
            69 => {
                let fireworks = decode_fireworks(decoder)?;
                summary.fireworks_flight_duration = Some(fireworks.flight_duration);
                summary.fireworks_explosions_count = Some(fireworks.explosions.len());
                summary.fireworks_explosions = fireworks.explosions;
            }
            70 => {
                summary.profile = Some(decode_resolvable_profile(decoder)?);
            }
            13 => {
                summary.enchantments = decode_varint_map(decoder)?;
            }
            14 => {
                summary.can_place_on = Some(decode_adventure_mode_predicate_summary(decoder)?);
            }
            15 => {
                summary.can_break = Some(decode_adventure_mode_predicate_summary(decoder)?);
            }
            42 => {
                summary.stored_enchantments = decode_varint_map(decoder)?;
            }
            16 => {
                summary.attribute_modifiers = decode_attribute_modifiers(decoder)?;
            }
            21 => {
                summary.enchantment_glint_override = Some(decoder.read_bool()?);
            }
            46 => {
                summary.map_id = Some(decoder.read_var_i32()?);
            }
            48 => {
                summary.map_post_processing = Some(decode_map_post_processing(decoder)?);
            }
            54 => {
                let writable_book = decode_writable_book_content(decoder)?;
                summary.writable_book_pages = writable_book.pages;
                summary.writable_book_page_filters = writable_book.page_filters;
            }
            55 => {
                summary.written_book = Some(decode_written_book_content(decoder)?);
            }
            67 => {
                let (target, tracked) = decode_lodestone_tracker(decoder)?;
                summary.lodestone_target = target;
                summary.lodestone_tracked = Some(tracked);
            }
            76 => {
                summary.block_state_properties =
                    decode_string_map(decoder, MAX_BLOCK_STATE_PROPERTIES)?;
            }
            75 => {
                summary.container_items = decode_item_container_contents(decoder)?;
                summary.container_item_count = Some(summary.container_items.len());
            }
            72 => {
                summary.banner_pattern_layers = decode_banner_pattern_layers_summary(decoder)?;
            }
            74 => {
                summary.pot_decorations_item_ids = decode_pot_decorations_item_ids(decoder)?;
            }
            79 => {
                summary.container_loot = true;
                skip_nbt_tag_from_decoder(decoder)?;
            }
            77 => {
                summary.bees_present = true;
                summary.bees_count = decode_bees(decoder)?;
            }
            58 => {
                summary.entity_data_entity_type_id =
                    Some(decode_typed_entity_data_summary(decoder)?.type_id);
            }
            60 => {
                let block_entity_data = decode_typed_entity_data_summary(decoder)?;
                summary.block_entity_spawn_entity_type =
                    typed_entity_data_spawn_entity_type(block_entity_data.tag.as_ref());
            }
            83 => {
                summary.villager_variant_id = Some(decode_holder_registry_id(decoder)?);
            }
            102 => {
                let painting = decode_painting_variant_holder_summary(decoder)?;
                summary.painting_variant_id = painting.registry_id;
                summary.painting_variant_direct = painting.direct;
            }
            _ => decode_data_component_value(decoder, type_id)?,
        }
        summary.added_type_ids.push(type_id);
    }
    Ok(summary)
}

fn decode_nbt_summary_from_decoder(decoder: &mut Decoder<'_>) -> Result<Option<NbtSummaryValue>> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(None);
    }
    read_nbt_summary_payload(decoder, tag_id, 0).map(Some)
}

fn read_nbt_summary_payload(
    decoder: &mut Decoder<'_>,
    tag_id: u8,
    depth: usize,
) -> Result<NbtSummaryValue> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "data component nbt exceeded max depth".to_string(),
        ));
    }

    match tag_id {
        1 => Ok(NbtSummaryValue::Byte(decoder.read_i8()?)),
        2 => Ok(NbtSummaryValue::Short(decoder.read_i16()?)),
        3 => Ok(NbtSummaryValue::Int(decoder.read_i32()?)),
        4 => Ok(NbtSummaryValue::Long(decoder.read_i64()?)),
        5 => Ok(NbtSummaryValue::Float(decoder.read_f32()?.to_bits())),
        6 => Ok(NbtSummaryValue::Double(decoder.read_f64()?.to_bits())),
        7 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_ARRAY_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_ARRAY_ITEMS));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i8()?);
            }
            Ok(NbtSummaryValue::ByteArray(values))
        }
        8 => Ok(NbtSummaryValue::String(read_nbt_modified_utf8(decoder)?)),
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_LIST_ITEMS));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty data component nbt list has end tag element type".to_string(),
                ));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(read_nbt_summary_payload(decoder, element_type, depth + 1)?);
            }
            Ok(NbtSummaryValue::List(values))
        }
        10 => {
            let mut entries = Vec::new();
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                let name = read_nbt_modified_utf8(decoder)?;
                let value = read_nbt_summary_payload(decoder, nested_type, depth + 1)?;
                entries.push(NbtSummaryEntry { name, value });
            }
            Ok(NbtSummaryValue::Compound(entries))
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_ARRAY_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_ARRAY_ITEMS));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i32()?);
            }
            Ok(NbtSummaryValue::IntArray(values))
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_ARRAY_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_ARRAY_ITEMS));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i64()?);
            }
            Ok(NbtSummaryValue::LongArray(values))
        }
        other => Err(ProtocolError::InvalidData(format!(
            "invalid data component nbt tag id {other}"
        ))),
    }
}

fn read_nbt_modified_utf8(decoder: &mut Decoder<'_>) -> Result<String> {
    let len = decoder.read_u16()? as usize;
    let bytes = decoder.read_exact(len, "nbt string")?;
    let mut units = Vec::with_capacity(len);
    let mut cursor = 0;

    while cursor < bytes.len() {
        let b0 = bytes[cursor];
        if b0 & 0x80 == 0 {
            units.push(u16::from(b0));
            cursor += 1;
        } else if b0 & 0xe0 == 0xc0 {
            if cursor + 1 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            let b1 = bytes[cursor + 1];
            if b1 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            units.push((u16::from(b0 & 0x1f) << 6) | u16::from(b1 & 0x3f));
            cursor += 2;
        } else if b0 & 0xf0 == 0xe0 {
            if cursor + 2 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            let b1 = bytes[cursor + 1];
            let b2 = bytes[cursor + 2];
            if b1 & 0xc0 != 0x80 || b2 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            units.push(
                (u16::from(b0 & 0x0f) << 12) | (u16::from(b1 & 0x3f) << 6) | u16::from(b2 & 0x3f),
            );
            cursor += 3;
        } else {
            return Err(ProtocolError::InvalidData(
                "invalid modified utf-8 leading byte".to_string(),
            ));
        }
    }

    String::from_utf16(&units)
        .map_err(|_| ProtocolError::InvalidData("invalid modified utf-8 string".to_string()))
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

fn decode_data_component_value(decoder: &mut Decoder<'_>, type_id: i32) -> Result<()> {
    match type_id {
        // These components use DataComponentType's codec-backed stream codec,
        // which serializes one NBT tag through ByteBufCodecs.fromCodec*.
        // custom_data, intangible_projectile, map_decorations, debug_stick_state,
        // bucket_entity_data, recipes, lock, and container_loot.
        0 | 22 | 47 | 57 | 59 | 66 | 78 | 79 => skip_nbt_tag_from_decoder(decoder)?,
        // 26.1 DataComponents: max_stack_size, max_damage, damage, repair_cost,
        // additional_trade_cost, map_id, ominous_bottle_amplifier, enchantable.
        1 | 2 | 3 | 19 | 31 | 41 | 46 | 63 => {
            decoder.read_var_i32()?;
        }
        // use_effects.
        5 => decode_use_effects(decoder)?,
        // unbreakable, creative_slot_lock, glider use Unit.STREAM_CODEC.
        4 | 20 | 34 => {}
        // damage_type and holderRegistry-backed entity variants.
        8 | 81 | 82 | 83 | 93 | 94 | 95 | 96 | 97 | 98 | 99 | 100 | 105 | 106 => {
            decode_holder_registry(decoder)?
        }
        // custom_name and item_name use ComponentSerialization.STREAM_CODEC.
        6 | 9 => {
            decode_component_summary_from_decoder(decoder)?;
        }
        // lore: list(256) of ComponentSerialization.STREAM_CODEC.
        11 => {
            let _ = decode_lore(decoder)?;
        }
        // minimum_attack_charge and potion_duration_scale.
        7 | 52 => {
            decoder.read_f32()?;
        }
        // item_model, tooltip_style, note_block_sound.
        10 | 35 | 71 => {
            decode_identifier(decoder)?;
        }
        // rarity uses ByIdMap.OutOfBoundsStrategy.ZERO.
        12 => {
            let _ = decode_item_rarity(decoder)?;
        }
        // dye, animal variant enums, collars,
        // tropical fish colors, sheep_color, shulker_color.
        43 | 73 | 84 | 85 | 86 | 87 | 88 | 89 | 90 | 91 | 92 | 101 | 103 | 104 | 107 | 108
        | 109 => {
            decoder.read_var_i32()?;
        }
        // map_post_processing uses ByIdMap.OutOfBoundsStrategy.ZERO.
        48 => {
            let _ = decode_map_post_processing(decoder)?;
        }
        // enchantments and stored_enchantments: map(enchantment holder id -> level).
        13 | 42 => {
            let _ = decode_varint_map(decoder)?;
        }
        // can_place_on and can_break.
        14 | 15 => decode_adventure_mode_predicate(decoder)?,
        // attribute_modifiers.
        16 => {
            let _ = decode_attribute_modifiers(decoder)?;
        }
        // custom_model_data: floats, flags, strings, colors.
        17 => {
            let _ = decode_custom_model_data(decoder)?;
        }
        // tooltip_display: bool + collection of data component type ids.
        18 => decode_tooltip_display(decoder)?,
        // enchantment_glint_override.
        21 => {
            decoder.read_bool()?;
        }
        // food, consumable, use_remainder.
        23 => decode_food(decoder)?,
        24 => decode_consumable(decoder)?,
        25 => decode_use_remainder(decoder)?,
        // use_cooldown.
        26 => decode_use_cooldown(decoder)?,
        // tool: rules, default mining speed, damage per block, creative flag.
        28 => decode_tool(decoder)?,
        // damage_resistant and repairable are holder sets.
        27 | 33 => decode_holder_set(decoder)?,
        // weapon.
        29 => decode_weapon(decoder)?,
        // attack_range.
        30 => decode_attack_range(decoder)?,
        // equippable.
        32 => decode_equippable(decoder)?,
        // death_protection, blocks_attacks, piercing_weapon, and kinetic_weapon.
        36 => decode_death_protection(decoder)?,
        37 => decode_blocks_attacks(decoder)?,
        38 => decode_piercing_weapon(decoder)?,
        39 => decode_kinetic_weapon(decoder)?,
        // swing_animation.
        40 => {
            let _ = decode_swing_animation(decoder)?;
        }
        // dyed_color and map_color.
        44 | 45 => {
            decoder.read_i32()?;
        }
        // charged_projectiles and bundle_contents.
        49 | 50 => {
            let _ = decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
        }
        // potion_contents.
        51 => {
            decode_potion_contents(decoder)?;
        }
        // suspicious_stew_effects.
        53 => {
            let _ = decode_suspicious_stew_effects(decoder)?;
        }
        // writable_book_content and written_book_content.
        54 => {
            let _ = decode_writable_book_content(decoder)?;
        }
        55 => {
            let _ = decode_written_book_content(decoder)?;
        }
        // trim.
        56 => {
            decode_armor_trim(decoder)?;
        }
        // entity_data and block_entity_data.
        58 | 60 => decode_typed_entity_data(decoder)?,
        // instrument, trim material, jukebox playable, break sound, painting variant.
        61 => decode_instrument_component(decoder)?,
        62 => decode_trim_material_holder(decoder)?,
        64 => decode_jukebox_playable(decoder)?,
        65 => decode_holder_set(decoder)?,
        67 => {
            let _ = decode_lodestone_tracker(decoder)?;
        }
        70 => {
            let _ = decode_resolvable_profile(decoder)?;
        }
        80 => decode_sound_event_holder(decoder)?,
        102 => decode_painting_variant_holder(decoder)?,
        // firework_explosion and fireworks.
        68 => {
            decode_firework_explosion(decoder)?;
        }
        69 => {
            let _ = decode_fireworks(decoder)?;
        }
        // banner_patterns, pot_decorations, and bees.
        72 => decode_banner_pattern_layers(decoder)?,
        74 => decode_pot_decorations(decoder)?,
        77 => {
            let _ = decode_bees(decoder)?;
        }
        // block_state.
        76 => {
            let _ = decode_string_map(decoder, MAX_BLOCK_STATE_PROPERTIES)?;
        }
        // container.
        75 => {
            let _ = decode_item_container_contents(decoder)?;
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unsupported data component type id {other}"
            )))
        }
    }
    Ok(())
}

fn decode_holder_registry(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    Ok(())
}

fn decode_holder_registry_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    Ok(id)
}

fn decode_holder_with_direct(
    decoder: &mut Decoder<'_>,
    decode_direct: fn(&mut Decoder<'_>) -> Result<()>,
) -> Result<()> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        decode_direct(decoder)?;
    }
    Ok(())
}

fn decode_holder_set(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_holder_set_summary(decoder)?;
    Ok(())
}

fn decode_holder_set_summary(decoder: &mut Decoder<'_>) -> Result<HolderSetSummary> {
    let encoded_count = decoder.read_var_i32()?;
    if encoded_count < 0 {
        return Err(ProtocolError::NegativeLength(encoded_count));
    }
    if encoded_count == 0 {
        decode_identifier(decoder)?;
        return Ok(HolderSetSummary {
            registry_ids: Vec::new(),
            unresolved_tag_count: 1,
        });
    }

    let count = (encoded_count - 1) as usize;
    if count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    let mut registry_ids = Vec::with_capacity(count);
    for _ in 0..count {
        registry_ids.push(decode_holder_registry_id(decoder)?);
    }
    Ok(HolderSetSummary {
        registry_ids,
        unresolved_tag_count: 0,
    })
}

fn decode_identifier(decoder: &mut Decoder<'_>) -> Result<()> {
    read_resource_location(decoder)?;
    Ok(())
}

fn decode_optional_identifier(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_identifier(decoder)?;
    }
    Ok(())
}

fn decode_optional_identifier_value(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return read_resource_location(decoder).map(Some);
    }
    Ok(None)
}

fn decode_optional_i32_value(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_i32()?));
    }
    Ok(None)
}

fn decode_optional_f32(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_f32()?;
    }
    Ok(())
}

fn decode_optional_bool(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_bool()?;
    }
    Ok(())
}

fn decode_optional_holder_set(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_holder_set(decoder)?;
    }
    Ok(())
}

fn decode_optional_sound_event_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_sound_event_holder(decoder)?;
    }
    Ok(())
}

fn decode_optional_global_pos(decoder: &mut Decoder<'_>) -> Result<Option<LodestoneTargetSummary>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    decode_global_pos(decoder).map(Some)
}

fn decode_global_pos(decoder: &mut Decoder<'_>) -> Result<LodestoneTargetSummary> {
    Ok(LodestoneTargetSummary {
        dimension: read_resource_location(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
    })
}

fn decode_lore(decoder: &mut Decoder<'_>) -> Result<Vec<Vec<StyledTextRun>>> {
    let line_count = read_bounded_len(decoder, MAX_LORE_LINES)?;
    let mut lines = Vec::with_capacity(line_count);
    for _ in 0..line_count {
        lines.push(decode_styled_component_summary_from_decoder(decoder)?);
    }
    Ok(lines)
}

fn decode_item_rarity(decoder: &mut Decoder<'_>) -> Result<ItemRaritySummary> {
    Ok(match decoder.read_var_i32()? {
        1 => ItemRaritySummary::Uncommon,
        2 => ItemRaritySummary::Rare,
        3 => ItemRaritySummary::Epic,
        _ => ItemRaritySummary::Common,
    })
}

fn decode_varint_map(decoder: &mut Decoder<'_>) -> Result<Vec<ItemEnchantmentSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(ItemEnchantmentSummary {
            holder_id: decoder.read_var_i32()?,
            level: decoder.read_var_i32()?,
        });
    }
    Ok(entries)
}

fn decode_map_post_processing(decoder: &mut Decoder<'_>) -> Result<MapPostProcessingSummary> {
    Ok(match decoder.read_var_i32()? {
        1 => MapPostProcessingSummary::Scale,
        _ => MapPostProcessingSummary::Lock,
    })
}

fn decode_adventure_mode_predicate(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_adventure_mode_predicate_summary(decoder)?;
    Ok(())
}

fn decode_adventure_mode_predicate_summary(
    decoder: &mut Decoder<'_>,
) -> Result<AdventureModePredicateSummary> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut summary = AdventureModePredicateSummary::default();
    for _ in 0..count {
        let predicate = decode_block_predicate_summary(decoder)?;
        let Some(blocks) = predicate.blocks else {
            summary.unknown = true;
            continue;
        };
        summary.unresolved_block_tag_count += blocks.unresolved_tag_count;
        for registry_id in blocks.registry_ids {
            if !summary.block_registry_ids.contains(&registry_id) {
                summary.block_registry_ids.push(registry_id);
            }
        }
    }
    Ok(summary)
}

struct BlockPredicateSummary {
    blocks: Option<HolderSetSummary>,
}

#[derive(Default)]
struct HolderSetSummary {
    registry_ids: Vec<i32>,
    unresolved_tag_count: usize,
}

fn decode_block_predicate_summary(decoder: &mut Decoder<'_>) -> Result<BlockPredicateSummary> {
    let blocks = if decoder.read_bool()? {
        Some(decode_holder_set_summary(decoder)?)
    } else {
        None
    };
    if decoder.read_bool()? {
        decode_state_properties_predicate(decoder)?;
    }
    if decoder.read_bool()? {
        skip_nbt_tag_from_decoder(decoder)?;
    }
    decode_data_component_matchers(decoder)?;
    Ok(BlockPredicateSummary { blocks })
}

fn decode_state_properties_predicate(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..count {
        decoder.read_string(MAX_STRING_CHARS)?;
        decode_state_property_value_matcher(decoder)?;
    }
    Ok(())
}

fn decode_state_property_value_matcher(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_string(MAX_STRING_CHARS)?;
    } else {
        decode_optional_string(decoder, MAX_STRING_CHARS)?;
        decode_optional_string(decoder, MAX_STRING_CHARS)?;
    }
    Ok(())
}

fn decode_data_component_matchers(decoder: &mut Decoder<'_>) -> Result<()> {
    let exact_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_PREDICATE_ENTRIES)?;
    decode_typed_data_component_list(decoder, exact_count)?;

    let partial_count = read_bounded_len(decoder, MAX_PARTIAL_DATA_COMPONENT_PREDICATES)?;
    for _ in 0..partial_count {
        decoder.read_bool()?;
        decoder.read_var_i32()?;
        skip_nbt_tag_from_decoder(decoder)?;
    }
    Ok(())
}

fn decode_attribute_modifiers(decoder: &mut Decoder<'_>) -> Result<Vec<AttributeModifierSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut modifiers = Vec::with_capacity(count);
    for _ in 0..count {
        let attribute_id = decode_holder_registry_id(decoder)?;
        let modifier_id = read_resource_location(decoder)?;
        let amount_bits = decoder.read_f64()?.to_bits();
        let operation_id = decode_attribute_modifier_operation_id(decoder)?;
        let slot_id = decode_equipment_slot_group_id(decoder)?;
        let (display_id, display_text) = decode_attribute_modifier_display(decoder)?;
        modifiers.push(AttributeModifierSummary {
            attribute_id,
            modifier_id,
            amount_bits,
            operation_id,
            slot_id,
            display_id,
            display_text,
        });
    }
    Ok(modifiers)
}

fn decode_attribute_modifier_operation_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    Ok(if (0..=2).contains(&id) { id } else { 0 })
}

fn decode_equipment_slot_group_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    Ok(if (0..=10).contains(&id) { id } else { 0 })
}

fn decode_attribute_modifier_display(decoder: &mut Decoder<'_>) -> Result<(i32, Option<String>)> {
    let display_id = decoder.read_var_i32()?;
    match display_id {
        2 => {
            let display_text = decode_component_summary_from_decoder(decoder)?;
            Ok((2, Some(display_text)))
        }
        0 | 1 => Ok((display_id, None)),
        _ => Ok((0, None)),
    }
}

fn decode_custom_model_data(
    decoder: &mut Decoder<'_>,
) -> Result<(Vec<f32>, Vec<bool>, Vec<String>, Vec<i32>)> {
    let float_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut float_values = Vec::with_capacity(float_count);
    for _ in 0..float_count {
        float_values.push(decoder.read_f32()?);
    }

    let flag_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut flag_values = Vec::with_capacity(flag_count);
    for _ in 0..flag_count {
        flag_values.push(decoder.read_bool()?);
    }

    let strings = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut string_values = Vec::with_capacity(strings);
    for _ in 0..strings {
        string_values.push(decoder.read_string(MAX_STRING_CHARS)?);
    }

    let colors = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut color_values = Vec::with_capacity(colors);
    for _ in 0..colors {
        color_values.push(decoder.read_i32()?);
    }

    Ok((float_values, flag_values, string_values, color_values))
}

fn decode_use_effects(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_use_effects_summary(decoder)?;
    Ok(())
}

fn decode_use_effects_summary(decoder: &mut Decoder<'_>) -> Result<UseEffectsSummary> {
    let summary = UseEffectsSummary {
        can_sprint: decoder.read_bool()?,
        interact_vibrations: decoder.read_bool()?,
        speed_multiplier: decoder.read_f32()?,
    };
    Ok(summary)
}

fn decode_food(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_f32()?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_consumable(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_consumable_summary(decoder)?;
    Ok(())
}

fn decode_consumable_summary(decoder: &mut Decoder<'_>) -> Result<ConsumableSummary> {
    let consume_seconds = decoder.read_f32()?;
    let animation = decode_item_use_animation_summary(decoder)?;
    decode_sound_event_holder(decoder)?;
    decoder.read_bool()?;

    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effect_count {
        decode_consume_effect(decoder)?;
    }
    Ok(ConsumableSummary {
        consume_seconds,
        animation,
    })
}

fn decode_item_use_animation_summary(decoder: &mut Decoder<'_>) -> Result<ItemUseAnimationSummary> {
    Ok(match decoder.read_var_i32()? {
        1 => ItemUseAnimationSummary::Eat,
        2 => ItemUseAnimationSummary::Drink,
        3 => ItemUseAnimationSummary::Block,
        4 => ItemUseAnimationSummary::Bow,
        5 => ItemUseAnimationSummary::Trident,
        6 => ItemUseAnimationSummary::Crossbow,
        7 => ItemUseAnimationSummary::Spyglass,
        8 => ItemUseAnimationSummary::TootHorn,
        9 => ItemUseAnimationSummary::Brush,
        10 => ItemUseAnimationSummary::Bundle,
        11 => ItemUseAnimationSummary::Spear,
        _ => ItemUseAnimationSummary::None,
    })
}

fn decode_consume_effect(decoder: &mut Decoder<'_>) -> Result<()> {
    match decoder.read_var_i32()? {
        0 => {
            let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            for _ in 0..effect_count {
                decode_mob_effect_instance(decoder)?;
            }
            decoder.read_f32()?;
        }
        1 => decode_holder_set(decoder)?,
        2 => {}
        3 => {
            decoder.read_f32()?;
        }
        4 => decode_sound_event_holder(decoder)?,
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid consume effect type id {other}"
            )));
        }
    }
    Ok(())
}

fn decode_use_remainder(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_item_stack_template(decoder)?;
    Ok(())
}

fn decode_item_stack_template(decoder: &mut Decoder<'_>) -> Result<ItemStackTemplateSummary> {
    let item_id = decoder.read_var_i32()?;
    if item_id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid item stack template item id {item_id}"
        )));
    }
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid item stack template count {count}"
        )));
    }
    let component_patch = decode_data_component_patch_summary(decoder)?;
    Ok(ItemStackTemplateSummary {
        item_id,
        count,
        component_patch,
    })
}

fn decode_item_stack_template_list(
    decoder: &mut Decoder<'_>,
    max: usize,
) -> Result<Vec<ItemStackTemplateSummary>> {
    let count = read_bounded_len(decoder, max)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(decode_item_stack_template(decoder)?);
    }
    Ok(items)
}

fn decode_optional_item_stack_template(
    decoder: &mut Decoder<'_>,
) -> Result<Option<ItemStackTemplateSummary>> {
    if decoder.read_bool()? {
        Ok(Some(decode_item_stack_template(decoder)?))
    } else {
        Ok(None)
    }
}

fn decode_item_container_contents(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<ItemStackTemplateSummary>> {
    let count = read_bounded_len(decoder, MAX_CONTAINER_ITEMS)?;
    let mut items = Vec::new();
    for _ in 0..count {
        if let Some(item) = decode_optional_item_stack_template(decoder)? {
            items.push(item);
        }
    }
    Ok(items)
}

fn decode_tool(decoder: &mut Decoder<'_>) -> Result<()> {
    let rule_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..rule_count {
        decode_holder_set(decoder)?;
        decode_optional_f32(decoder)?;
        decode_optional_bool(decoder)?;
    }
    decoder.read_f32()?;
    decoder.read_var_i32()?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_use_cooldown(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decode_optional_identifier(decoder)
}

struct UseCooldownSummary {
    ticks: i32,
    cooldown_group: Option<String>,
}

fn decode_use_cooldown_summary(decoder: &mut Decoder<'_>) -> Result<UseCooldownSummary> {
    let seconds = decoder.read_f32()?;
    Ok(UseCooldownSummary {
        ticks: (seconds * 20.0) as i32,
        cooldown_group: decode_optional_identifier_value(decoder)?,
    })
}

fn decode_weapon(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_attack_range(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_attack_range_summary(decoder)?;
    Ok(())
}

fn decode_attack_range_summary(decoder: &mut Decoder<'_>) -> Result<AttackRangeSummary> {
    Ok(AttackRangeSummary {
        min_reach: decoder.read_f32()?,
        max_reach: decoder.read_f32()?,
        min_creative_reach: decoder.read_f32()?,
        max_creative_reach: decoder.read_f32()?,
        hitbox_margin: decoder.read_f32()?,
        mob_factor: decoder.read_f32()?,
    })
}

fn decode_death_protection(decoder: &mut Decoder<'_>) -> Result<()> {
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effect_count {
        decode_consume_effect(decoder)?;
    }
    Ok(())
}

fn decode_blocks_attacks(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decoder.read_f32()?;

    let reduction_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..reduction_count {
        decode_damage_reduction(decoder)?;
    }

    decode_item_damage_function(decoder)?;
    decode_optional_holder_set(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_damage_reduction(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decode_optional_holder_set(decoder)?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_item_damage_function(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_piercing_weapon(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_bool()?;
    decoder.read_bool()?;
    decode_optional_sound_event_holder(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_kinetic_weapon(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_var_i32()?;
    decode_optional_kinetic_weapon_condition(decoder)?;
    decode_optional_kinetic_weapon_condition(decoder)?;
    decode_optional_kinetic_weapon_condition(decoder)?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    decode_optional_sound_event_holder(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_optional_kinetic_weapon_condition(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_var_i32()?;
        decoder.read_f32()?;
        decoder.read_f32()?;
    }
    Ok(())
}

fn decode_equippable(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decode_sound_event_holder(decoder)?;
    decode_optional_identifier(decoder)?;
    decode_optional_identifier(decoder)?;
    if decoder.read_bool()? {
        decode_holder_set(decoder)?;
    }
    for _ in 0..5 {
        decoder.read_bool()?;
    }
    decode_sound_event_holder(decoder)?;
    Ok(())
}

struct ArmorTrimSummary {
    material_id: Option<i32>,
    material_direct: Option<TrimMaterialSummary>,
    pattern_id: Option<i32>,
    pattern_direct: Option<TrimPatternSummary>,
}

fn decode_armor_trim(decoder: &mut Decoder<'_>) -> Result<ArmorTrimSummary> {
    let material = decode_trim_material_holder_summary(decoder)?;
    let pattern = decode_trim_pattern_holder(decoder)?;
    Ok(ArmorTrimSummary {
        material_id: material.registry_id,
        material_direct: material.direct,
        pattern_id: pattern.registry_id,
        pattern_direct: pattern.direct,
    })
}

struct TrimMaterialHolderSummary {
    registry_id: Option<i32>,
    direct: Option<TrimMaterialSummary>,
}

/// Decodes the `ArmorTrim.material()` holder, preserving registry references as
/// `holder_id - 1` so `minecraft:trim_material` can still project them through
/// the dynamic registry while exact component predicates can compare inline
/// material payloads.
fn decode_trim_material_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<TrimMaterialHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(TrimMaterialHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_trim_material_summary(decoder)?),
        })
    } else {
        Ok(TrimMaterialHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

struct TypedEntityDataSummary {
    type_id: i32,
    tag: Option<NbtSummaryValue>,
}

fn decode_typed_entity_data(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_typed_entity_data_summary(decoder)?;
    Ok(())
}

fn decode_typed_entity_data_summary(decoder: &mut Decoder<'_>) -> Result<TypedEntityDataSummary> {
    let type_id = decode_holder_registry_id(decoder)?;
    Ok(TypedEntityDataSummary {
        type_id,
        tag: decode_nbt_summary_from_decoder(decoder)?,
    })
}

fn typed_entity_data_spawn_entity_type(tag: Option<&NbtSummaryValue>) -> Option<String> {
    let spawn_data = nbt_compound_get(tag?, "SpawnData")?;
    let entity = nbt_compound_get(spawn_data, "entity")?;
    nbt_string(nbt_compound_get(entity, "id")?)
        .and_then(|entity_id| normalize_resource_location(entity_id.to_string()).ok())
}

fn nbt_compound_get<'a>(value: &'a NbtSummaryValue, key: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn nbt_string(value: &NbtSummaryValue) -> Option<&str> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    Some(value)
}

struct InstrumentComponentSummary {
    registry_id: Option<i32>,
    description: Option<String>,
    description_styled: Option<Vec<StyledTextRun>>,
}

fn decode_instrument_component(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_instrument_component_summary(decoder)?;
    Ok(())
}

fn decode_instrument_component_summary(
    decoder: &mut Decoder<'_>,
) -> Result<InstrumentComponentSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        let description_styled = decode_direct_instrument_description(decoder)?;
        Ok(InstrumentComponentSummary {
            registry_id: None,
            description: Some(styled_runs_summary_text(&description_styled)),
            description_styled: Some(description_styled),
        })
    } else {
        Ok(InstrumentComponentSummary {
            registry_id: Some(id - 1),
            description: None,
            description_styled: None,
        })
    }
}

fn decode_direct_instrument_description(decoder: &mut Decoder<'_>) -> Result<Vec<StyledTextRun>> {
    decode_sound_event_holder(decoder)?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    decode_styled_component_summary_from_decoder(decoder)
}

fn decode_trim_material_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_trim_material)
}

fn decode_direct_trim_material(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_direct_trim_material_summary(decoder)?;
    Ok(())
}

fn decode_direct_trim_material_summary(decoder: &mut Decoder<'_>) -> Result<TrimMaterialSummary> {
    let (asset_name, override_armor_assets) = decode_material_asset_group_summary(decoder)?;
    let description_styled = decode_styled_component_summary_from_decoder(decoder)?;
    let description = styled_runs_summary_text(&description_styled);
    Ok(TrimMaterialSummary {
        asset_name,
        override_armor_assets,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
    })
}

fn decode_material_asset_group_summary(
    decoder: &mut Decoder<'_>,
) -> Result<(String, BTreeMap<String, String>)> {
    let asset_name = decoder.read_string(MAX_STRING_CHARS)?;
    let overrides = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut override_armor_assets = BTreeMap::new();
    for _ in 0..overrides {
        let equipment_asset = read_resource_location(decoder)?;
        let suffix = decoder.read_string(MAX_STRING_CHARS)?;
        override_armor_assets.insert(equipment_asset, suffix);
    }
    Ok((asset_name, override_armor_assets))
}

struct TrimPatternHolderSummary {
    registry_id: Option<i32>,
    direct: Option<TrimPatternSummary>,
}

fn decode_trim_pattern_holder(decoder: &mut Decoder<'_>) -> Result<TrimPatternHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(TrimPatternHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_trim_pattern_summary(decoder)?),
        })
    } else {
        Ok(TrimPatternHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_direct_trim_pattern_summary(decoder: &mut Decoder<'_>) -> Result<TrimPatternSummary> {
    let asset_id = read_resource_location(decoder)?;
    let description_styled = decode_styled_component_summary_from_decoder(decoder)?;
    let description = styled_runs_summary_text(&description_styled);
    let decal = decoder.read_bool()?;
    Ok(TrimPatternSummary {
        asset_id,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        decal,
    })
}

fn decode_jukebox_playable(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_jukebox_song)
}

struct JukeboxSongHolderSummary {
    registry_id: Option<i32>,
    direct: Option<JukeboxSongSummary>,
}

/// Decodes the `JukeboxPlayable.song()` holder, preserving either the registry
/// reference id (`holder_id - 1`) or the inline direct song payload.
fn decode_jukebox_song_holder(decoder: &mut Decoder<'_>) -> Result<JukeboxSongHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(JukeboxSongHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_jukebox_song_summary(decoder)?),
        })
    } else {
        Ok(JukeboxSongHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_lodestone_tracker(
    decoder: &mut Decoder<'_>,
) -> Result<(Option<LodestoneTargetSummary>, bool)> {
    let target = decode_optional_global_pos(decoder)?;
    let tracked = decoder.read_bool()?;
    Ok((target, tracked))
}

fn decode_resolvable_profile(decoder: &mut Decoder<'_>) -> Result<ResolvableProfileSummary> {
    let (kind, uuid, name, properties) = if decoder.read_bool()? {
        let profile = decode_game_profile(decoder)?;
        (
            ResolvableProfileKindSummary::GameProfile,
            Some(profile.uuid),
            Some(profile.name),
            profile.properties,
        )
    } else {
        let partial = decode_partial_profile(decoder)?;
        (
            ResolvableProfileKindSummary::Partial,
            partial.uuid,
            partial.name,
            partial.properties,
        )
    };
    let profile_textures = decode_profile_textures_from_properties(
        properties
            .iter()
            .map(|property| (property.name.as_str(), property.value.as_str())),
    );
    let skin_patch = decode_player_skin_patch(decoder)?;
    Ok(ResolvableProfileSummary {
        kind,
        uuid,
        name,
        properties,
        profile_textures,
        skin_patch,
    })
}

struct DecodedGameProfile {
    uuid: Uuid,
    name: String,
    properties: Vec<GameProfilePropertySummary>,
}

fn decode_game_profile(decoder: &mut Decoder<'_>) -> Result<DecodedGameProfile> {
    Ok(DecodedGameProfile {
        uuid: decoder.read_uuid()?,
        name: decoder.read_string(MAX_PLAYER_NAME_CHARS)?,
        properties: decode_game_profile_properties(decoder)?,
    })
}

struct DecodedPartialProfile {
    name: Option<String>,
    uuid: Option<Uuid>,
    properties: Vec<GameProfilePropertySummary>,
}

fn decode_partial_profile(decoder: &mut Decoder<'_>) -> Result<DecodedPartialProfile> {
    let name = decode_optional_string_value(decoder, MAX_PLAYER_NAME_CHARS)?;
    let uuid = if decoder.read_bool()? {
        Some(decoder.read_uuid()?)
    } else {
        None
    };
    Ok(DecodedPartialProfile {
        name,
        uuid,
        properties: decode_game_profile_properties(decoder)?,
    })
}

fn decode_game_profile_properties(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<GameProfilePropertySummary>> {
    let property_count = read_bounded_len(decoder, MAX_PROFILE_PROPERTIES)?;
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        properties.push(GameProfilePropertySummary {
            name: decoder.read_string(MAX_PROFILE_PROPERTY_NAME_CHARS)?,
            value: decoder.read_string(MAX_STRING_CHARS)?,
            signature: decode_optional_string_value(decoder, MAX_PROFILE_SIGNATURE_CHARS)?,
        });
    }
    Ok(properties)
}

fn decode_player_skin_patch(decoder: &mut Decoder<'_>) -> Result<PlayerSkinPatchSummary> {
    Ok(PlayerSkinPatchSummary {
        body: decode_optional_resource_texture(decoder)?,
        cape: decode_optional_resource_texture(decoder)?,
        elytra: decode_optional_resource_texture(decoder)?,
        model: decode_optional_player_model_type(decoder)?,
    })
}

fn decode_optional_resource_texture(
    decoder: &mut Decoder<'_>,
) -> Result<Option<ResourceTextureSummary>> {
    if decoder.read_bool()? {
        let asset_id = read_resource_location(decoder)?;
        return Ok(Some(ResourceTextureSummary {
            texture_path: resource_texture_path(&asset_id),
            asset_id,
        }));
    }
    Ok(None)
}

fn resource_texture_path(asset_id: &str) -> String {
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("resource locations decoded by read_resource_location include a namespace");
    format!("{namespace}:textures/{path}.png")
}

fn decode_optional_player_model_type(
    decoder: &mut Decoder<'_>,
) -> Result<Option<PlayerModelTypeSummary>> {
    if decoder.read_bool()? {
        return Ok(Some(if decoder.read_bool()? {
            PlayerModelTypeSummary::Slim
        } else {
            PlayerModelTypeSummary::Wide
        }));
    }
    Ok(None)
}

fn decode_direct_jukebox_song(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_direct_jukebox_song_summary(decoder)?;
    Ok(())
}

fn decode_direct_jukebox_song_summary(decoder: &mut Decoder<'_>) -> Result<JukeboxSongSummary> {
    let sound_event = decode_sound_event_holder_summary(decoder)?;
    let description = decode_component_summary_from_decoder(decoder)?;
    let length_in_seconds_bits = decoder.read_f32()?.to_bits();
    let comparator_output = decoder.read_var_i32()?;
    Ok(JukeboxSongSummary {
        sound_event,
        description,
        length_in_seconds_bits,
        comparator_output,
    })
}

fn decode_sound_event_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_sound_event_holder_summary(decoder)?;
    Ok(())
}

fn decode_sound_event_holder_summary(decoder: &mut Decoder<'_>) -> Result<SoundEventSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        decode_direct_sound_event_summary(decoder)
    } else {
        Ok(SoundEventSummary {
            registry_id: Some(id - 1),
            sound_id: None,
            fixed_range_bits: None,
        })
    }
}

fn decode_direct_sound_event_summary(decoder: &mut Decoder<'_>) -> Result<SoundEventSummary> {
    let sound_id = read_resource_location(decoder)?;
    let fixed_range_bits = if decoder.read_bool()? {
        Some(decoder.read_f32()?.to_bits())
    } else {
        None
    };
    Ok(SoundEventSummary {
        registry_id: None,
        sound_id: Some(sound_id),
        fixed_range_bits,
    })
}

fn decode_banner_pattern_layers(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_banner_pattern_layers_summary(decoder)?;
    Ok(())
}

fn decode_banner_pattern_layers_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<BannerPatternLayerSummary>> {
    let layer_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut layers = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let pattern = decode_banner_pattern_holder(decoder)?;
        let color_id = decoder.read_var_i32()?;
        layers.push(BannerPatternLayerSummary {
            registry_id: pattern.registry_id,
            translation_key: pattern.translation_key,
            color_id,
        });
    }
    Ok(layers)
}

struct BannerPatternHolderSummary {
    registry_id: Option<i32>,
    translation_key: Option<String>,
}

fn decode_banner_pattern_holder(decoder: &mut Decoder<'_>) -> Result<BannerPatternHolderSummary> {
    let holder_id = decoder.read_var_i32()?;
    if holder_id == 0 {
        let _asset_id = decode_identifier(decoder)?;
        let translation_key = decoder.read_string(MAX_STRING_CHARS)?;
        Ok(BannerPatternHolderSummary {
            registry_id: None,
            translation_key: Some(translation_key),
        })
    } else {
        Ok(BannerPatternHolderSummary {
            registry_id: Some(holder_id - 1),
            translation_key: None,
        })
    }
}

fn decode_pot_decorations(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_pot_decorations_item_ids(decoder)?;
    Ok(())
}

fn decode_pot_decorations_item_ids(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    let item_count = read_bounded_len(decoder, MAX_POT_DECORATIONS)?;
    let mut item_ids = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        item_ids.push(decoder.read_var_i32()?);
    }
    Ok(item_ids)
}

fn decode_bees(decoder: &mut Decoder<'_>) -> Result<usize> {
    let bee_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..bee_count {
        decoder.read_var_i32()?;
        skip_nbt_tag_from_decoder(decoder)?;
        decoder.read_var_i32()?;
        decoder.read_var_i32()?;
    }
    Ok(bee_count)
}

fn decode_painting_variant_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_painting_variant_holder_summary(decoder)?;
    Ok(())
}

struct PaintingVariantHolderSummary {
    registry_id: Option<i32>,
    direct: Option<PaintingVariantSummary>,
}

fn decode_painting_variant_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<PaintingVariantHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(PaintingVariantHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_painting_variant_summary(decoder)?),
        })
    } else {
        Ok(PaintingVariantHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_direct_painting_variant_summary(
    decoder: &mut Decoder<'_>,
) -> Result<PaintingVariantSummary> {
    let width = decoder.read_var_i32()?;
    let height = decoder.read_var_i32()?;
    let asset_id = read_resource_location(decoder)?;
    let title_styled = decode_optional_component_summary(decoder)?;
    let author_styled = decode_optional_component_summary(decoder)?;
    Ok(PaintingVariantSummary {
        width,
        height,
        asset_id,
        title: title_styled
            .as_ref()
            .map(|runs| styled_runs_summary_text(runs)),
        title_styled,
        author: author_styled
            .as_ref()
            .map(|runs| styled_runs_summary_text(runs)),
        author_styled,
    })
}

fn decode_swing_animation(decoder: &mut Decoder<'_>) -> Result<SwingAnimationSummary> {
    let animation_type = match decoder.read_var_i32()? {
        1 => SwingAnimationTypeSummary::Whack,
        2 => SwingAnimationTypeSummary::Stab,
        _ => SwingAnimationTypeSummary::None,
    };
    Ok(SwingAnimationSummary {
        animation_type,
        duration: decoder.read_var_i32()?,
    })
}

struct PotionContentsSummary {
    potion_id: Option<i32>,
    custom_color: Option<i32>,
    custom_effects: Vec<MobEffectInstanceSummary>,
    custom_name: Option<String>,
}

fn decode_potion_contents(decoder: &mut Decoder<'_>) -> Result<PotionContentsSummary> {
    let potion_id = if decoder.read_bool()? {
        Some(decoder.read_var_i32()?)
    } else {
        None
    };
    let custom_color = decode_optional_i32_value(decoder)?;
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut custom_effects = Vec::with_capacity(effect_count);
    for _ in 0..effect_count {
        custom_effects.push(decode_mob_effect_instance(decoder)?);
    }
    let custom_name = decode_optional_string_value(decoder, MAX_STRING_CHARS)?;
    Ok(PotionContentsSummary {
        potion_id,
        custom_color,
        custom_effects,
        custom_name,
    })
}

fn decode_suspicious_stew_effects(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<SuspiciousStewEffectSummary>> {
    let effects = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut summaries = Vec::with_capacity(effects);
    for _ in 0..effects {
        summaries.push(SuspiciousStewEffectSummary {
            effect_id: decode_holder_registry_id(decoder)?,
            duration: decoder.read_var_i32()?,
        });
    }
    Ok(summaries)
}

fn decode_mob_effect_instance(decoder: &mut Decoder<'_>) -> Result<MobEffectInstanceSummary> {
    let effect_id = decode_holder_registry_id(decoder)?;
    let details = decode_mob_effect_details(decoder, 0)?;
    Ok(MobEffectInstanceSummary {
        effect_id,
        amplifier: details.amplifier,
        duration: details.duration,
        ambient: details.ambient,
        show_particles: details.show_particles,
        show_icon: details.show_icon,
        hidden_effect: details.hidden_effect,
    })
}

fn decode_mob_effect_details(
    decoder: &mut Decoder<'_>,
    depth: usize,
) -> Result<MobEffectDetailsSummary> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(ProtocolError::InvalidData(
            "mob effect details exceeded max depth".to_string(),
        ));
    }
    let amplifier = decoder.read_var_i32()?;
    let duration = decoder.read_var_i32()?;
    let ambient = decoder.read_bool()?;
    let show_particles = decoder.read_bool()?;
    let show_icon = decoder.read_bool()?;
    let hidden_effect = if decoder.read_bool()? {
        Some(Box::new(decode_mob_effect_details(decoder, depth + 1)?))
    } else {
        None
    };
    Ok(MobEffectDetailsSummary {
        amplifier,
        duration,
        ambient,
        show_particles,
        show_icon,
        hidden_effect,
    })
}

struct WritableBookContentSummary {
    pages: Vec<String>,
    page_filters: Vec<Option<String>>,
}

fn decode_writable_book_content(decoder: &mut Decoder<'_>) -> Result<WritableBookContentSummary> {
    let pages = read_bounded_len(decoder, MAX_BOOK_PAGES)?;
    let mut out = WritableBookContentSummary {
        pages: Vec::with_capacity(pages),
        page_filters: Vec::with_capacity(pages),
    };
    for _ in 0..pages {
        let page = decode_filterable_string_summary(decoder, MAX_WRITABLE_BOOK_PAGE_CHARS)?;
        out.pages.push(page.raw);
        out.page_filters.push(page.filtered);
    }
    Ok(out)
}

fn decode_written_book_content(decoder: &mut Decoder<'_>) -> Result<WrittenBookContentSummary> {
    let title = decode_filterable_string_summary(decoder, MAX_WRITTEN_BOOK_TITLE_CHARS)?;
    let author = decoder.read_string(MAX_STRING_CHARS)?;
    let generation = decoder.read_var_i32()?;
    let pages = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut page_values = Vec::with_capacity(pages);
    let mut page_filters = Vec::with_capacity(pages);
    for _ in 0..pages {
        let page = decode_filterable_component_summary(decoder)?;
        page_values.push(page.raw);
        page_filters.push(page.filtered);
    }
    let resolved = decoder.read_bool()?;
    Ok(WrittenBookContentSummary {
        title: title.raw,
        title_filter: title.filtered,
        author,
        generation,
        pages: page_values,
        page_filters,
        resolved,
    })
}

struct FilterableStringSummary {
    raw: String,
    filtered: Option<String>,
}

fn decode_filterable_string_summary(
    decoder: &mut Decoder<'_>,
    max_chars: usize,
) -> Result<FilterableStringSummary> {
    let raw = decoder.read_string(max_chars)?;
    let filtered = decode_optional_string_value(decoder, max_chars)?;
    Ok(FilterableStringSummary { raw, filtered })
}

fn decode_filterable_component_summary(
    decoder: &mut Decoder<'_>,
) -> Result<FilterableStringSummary> {
    let raw = decode_component_summary_from_decoder(decoder)?;
    let filtered = if decoder.read_bool()? {
        Some(decode_component_summary_from_decoder(decoder)?)
    } else {
        None
    };
    Ok(FilterableStringSummary { raw, filtered })
}

fn decode_optional_component_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Option<Vec<StyledTextRun>>> {
    if decoder.read_bool()? {
        return decode_styled_component_summary_from_decoder(decoder).map(Some);
    }
    Ok(None)
}

struct FireworksComponentSummary {
    flight_duration: i32,
    explosions: Vec<FireworkExplosionSummary>,
}

fn decode_fireworks(decoder: &mut Decoder<'_>) -> Result<FireworksComponentSummary> {
    let flight_duration = decoder.read_var_i32()?;
    let explosion_count = read_bounded_len(decoder, MAX_FIREWORK_EXPLOSIONS)?;
    let mut explosions = Vec::with_capacity(explosion_count);
    for _ in 0..explosion_count {
        explosions.push(decode_firework_explosion(decoder)?);
    }
    Ok(FireworksComponentSummary {
        flight_duration,
        explosions,
    })
}

fn decode_firework_explosion(decoder: &mut Decoder<'_>) -> Result<FireworkExplosionSummary> {
    let shape = FireworkExplosionShapeSummary::from_vanilla_id(decoder.read_var_i32()?);
    let colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let fade_colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let has_trail = decoder.read_bool()?;
    let has_twinkle = decoder.read_bool()?;
    Ok(FireworkExplosionSummary {
        shape,
        colors,
        fade_colors,
        has_trail,
        has_twinkle,
    })
}

fn decode_int_list(decoder: &mut Decoder<'_>, max: usize) -> Result<Vec<i32>> {
    let count = read_bounded_len(decoder, max)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decoder.read_i32()?);
    }
    Ok(values)
}

fn decode_string_map(decoder: &mut Decoder<'_>, max: usize) -> Result<BTreeMap<String, String>> {
    let count = read_bounded_len(decoder, max)?;
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        let key = decoder.read_string(MAX_STRING_CHARS)?;
        let value = decoder.read_string(MAX_STRING_CHARS)?;
        entries.insert(key, value);
    }
    Ok(entries)
}

fn decode_optional_string(decoder: &mut Decoder<'_>, max_chars: usize) -> Result<()> {
    let _ = decode_optional_string_value(decoder, max_chars)?;
    Ok(())
}

fn decode_optional_string_value(
    decoder: &mut Decoder<'_>,
    max_chars: usize,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return decoder.read_string(max_chars).map(Some);
    }
    Ok(None)
}

fn decode_tooltip_display(decoder: &mut Decoder<'_>) -> Result<()> {
    let _ = decode_tooltip_display_summary(decoder)?;
    Ok(())
}

fn decode_tooltip_display_summary(decoder: &mut Decoder<'_>) -> Result<(bool, Vec<i32>)> {
    let hide_tooltip = decoder.read_bool()?;
    let hidden_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut hidden_components = Vec::with_capacity(hidden_count);
    for _ in 0..hidden_count {
        hidden_components.push(decoder.read_var_i32()?);
    }
    Ok((hide_tooltip, hidden_components))
}

fn read_bounded_len(decoder: &mut Decoder<'_>, max: usize) -> Result<usize> {
    let len = decoder.read_len()?;
    if len > max {
        return Err(ProtocolError::PacketTooLarge(len, max));
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Encoder;
    use uuid::Uuid;

    #[test]
    fn decodes_supported_data_component_patch_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(11);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(2);
        payload.write_var_i32(432);
        payload.write_var_i32(3);
        payload.write_var_i32(431);
        payload.write_var_i32(4);
        payload.write_var_i32(6);
        payload.write_bytes(&nbt_string_root("Named"));
        payload.write_var_i32(10);
        payload.write_string("minecraft:diamond_sword");
        payload.write_var_i32(21);
        payload.write_bool(true);
        payload.write_var_i32(22);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(26);
        payload.write_f32(1.5);
        payload.write_bool(true);
        payload.write_string("minecraft:ender_pearl");
        payload.write_var_i32(63);
        payload.write_var_i32(3);
        payload.write_var_i32(79);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(3);
        payload.write_var_i32(12);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 11,
                added_type_ids: vec![1, 2, 3, 4, 6, 10, 21, 22, 26, 63, 79],
                removed_type_ids: vec![3, 12],
                max_stack_size: Some(64),
                max_damage: Some(432),
                damage: Some(431),
                unbreakable: true,
                custom_name: Some("Named".to_string()),
                custom_name_styled: Some(plain_runs("Named")),
                item_model: Some("minecraft:diamond_sword".to_string()),
                enchantment_glint_override: Some(true),
                intangible_projectile: true,
                use_cooldown_ticks: Some(30),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                ominous_bottle_amplifier: Some(3),
                container_loot: true,
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_custom_data_component_nbt_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_bytes(&custom_data_nbt_root());

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch.custom_data,
            Some(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "owner".to_string(),
                    value: NbtSummaryValue::String("Alex".to_string()),
                },
                NbtSummaryEntry {
                    name: "level".to_string(),
                    value: NbtSummaryValue::Int(7),
                },
                NbtSummaryEntry {
                    name: "nested".to_string(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "flag".to_string(),
                        value: NbtSummaryValue::Byte(1),
                    }]),
                },
                NbtSummaryEntry {
                    name: "lore".to_string(),
                    value: NbtSummaryValue::List(vec![
                        NbtSummaryValue::String("one".to_string()),
                        NbtSummaryValue::String("two".to_string()),
                    ]),
                },
            ]))
        );
        assert_eq!(patch.added_type_ids, vec![0]);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_suspicious_stew_effects_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(53);
        payload.write_var_i32(2);
        payload.write_var_i32(18);
        payload.write_var_i32(160);
        payload.write_var_i32(9);
        payload.write_var_i32(200);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(patch.added_type_ids, vec![53]);
        assert_eq!(
            patch.suspicious_stew_effects,
            vec![
                SuspiciousStewEffectSummary {
                    effect_id: 18,
                    duration: 160,
                },
                SuspiciousStewEffectSummary {
                    effect_id: 9,
                    duration: 200,
                },
            ]
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_lodestone_tracker_target_component() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(67);
        payload.write_bool(true);
        payload.write_string("minecraft:overworld");
        payload.write_i64(chunks::encode_block_pos(chunks::BlockPos {
            x: 10,
            y: 64,
            z: -4,
        }));
        payload.write_bool(false);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![67],
                removed_type_ids: Vec::new(),
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos: chunks::BlockPos {
                        x: 10,
                        y: 64,
                        z: -4,
                    },
                }),
                lodestone_tracked: Some(false),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_jukebox_playable_song_holder_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(3);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![64],
                jukebox_song_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_instrument_holder_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(61);
        payload.write_var_i32(6);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![61],
                instrument_id: Some(5),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_instrument_description() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(61);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_f32(7.0);
        payload.write_f32(256.0);
        payload.write_bytes(&nbt_string_root("Custom horn"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![61],
                instrument_description: Some("Custom horn".to_string()),
                instrument_description_styled: Some(plain_runs("Custom horn")),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_tropical_fish_pattern_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);
        payload.write_var_i32(0);
        payload.write_var_i32(88);
        payload.write_var_i32(257);
        payload.write_var_i32(89);
        payload.write_var_i32(1);
        payload.write_var_i32(90);
        payload.write_var_i32(7);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![88, 89, 90],
                tropical_fish_pattern_id: Some(257),
                tropical_fish_base_color_id: Some(1),
                tropical_fish_pattern_color_id: Some(7),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_jukebox_playable_song_payload() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(0);
        write_direct_sound_event(&mut payload, "minecraft:test.song", Some(16.0));
        payload.write_bytes(&nbt_string_root("Test song"));
        payload.write_f32(3.5);
        payload.write_var_i32(7);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![64],
                jukebox_direct_song: Some(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:test.song".to_string()),
                        fixed_range_bits: Some(16.0f32.to_bits()),
                    },
                    description: "Test song".to_string(),
                    length_in_seconds_bits: 3.5f32.to_bits(),
                    comparator_output: 7,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_jukebox_song_registry_sound_event_payload() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(0);
        payload.write_var_i32(287);
        payload.write_bytes(&nbt_string_root("Registry sound song"));
        payload.write_f32(4.25);
        payload.write_var_i32(9);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![64],
                jukebox_direct_song: Some(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(286),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    description: "Registry sound song".to_string(),
                    length_in_seconds_bits: 4.25f32.to_bits(),
                    comparator_output: 9,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_trim_pattern_payload() {
        let mut styled_pattern = vec![10u8];
        write_named_nbt_string(&mut styled_pattern, "text", "Test pattern");
        write_named_nbt_byte(&mut styled_pattern, "bold", 1);
        styled_pattern.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(56);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_string("minecraft:test_pattern");
        payload.write_bytes(&styled_pattern);
        payload.write_bool(true);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                    asset_id: "minecraft:test_pattern".to_string(),
                    description: "Test pattern".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test pattern".to_string(),
                            style: crate::ComponentStyle {
                                bold: Some(true),
                                ..Default::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    decal: true,
                })),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_trim_material_payload() {
        let mut styled_material = vec![10u8];
        write_named_nbt_string(&mut styled_material, "text", "Test material");
        write_named_nbt_byte(&mut styled_material, "italic", 1);
        write_named_nbt_string(&mut styled_material, "color", "gold");
        styled_material.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(56);
        payload.write_var_i32(0);
        payload.write_string("test_material");
        payload.write_var_i32(1);
        payload.write_string("minecraft:iron");
        payload.write_string("test_material_iron");
        payload.write_bytes(&styled_material);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![56],
                armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "test_material".to_string(),
                    override_armor_assets: BTreeMap::from([(
                        "minecraft:iron".to_string(),
                        "test_material_iron".to_string()
                    )]),
                    description: "Test material".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test material".to_string(),
                            style: crate::ComponentStyle {
                                italic: Some(true),
                                color: Some(0xFF_AA_00),
                                ..Default::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                })),
                armor_trim_pattern_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_block_state_component_properties() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(76);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_string("south");
        payload.write_string("powered");
        payload.write_string("true");

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![76],
                removed_type_ids: Vec::new(),
                block_state_properties: BTreeMap::from([
                    ("facing".to_string(), "south".to_string()),
                    ("powered".to_string(), "true".to_string()),
                ]),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_pot_decorations_item_ids() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(74);
        payload.write_var_i32(4);
        payload.write_var_i32(10);
        payload.write_var_i32(11);
        payload.write_var_i32(12);
        payload.write_var_i32(13);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![74],
                pot_decorations_item_ids: vec![10, 11, 12, 13],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_hover_text_component_summaries() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(6);
        payload.write_bytes(&nbt_string_root("Custom Name"));
        payload.write_var_i32(9);
        payload.write_bytes(&nbt_string_root("Item Name"));
        payload.write_var_i32(11);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Lore one"));
        payload.write_bytes(&nbt_string_root("Lore two"));
        payload.write_var_i32(12);
        payload.write_var_i32(2);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![6, 9, 11, 12],
                removed_type_ids: Vec::new(),
                custom_name: Some("Custom Name".to_string()),
                custom_name_styled: Some(plain_runs("Custom Name")),
                item_name: Some("Item Name".to_string()),
                item_name_styled: Some(plain_runs("Item Name")),
                lore: vec!["Lore one".to_string(), "Lore two".to_string()],
                lore_styled: vec![plain_runs("Lore one"), plain_runs("Lore two")],
                rarity: Some(ItemRaritySummary::Rare),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_styled_name_and_lore_component_runs() {
        // custom_name = {text:"Sword", bold:1b, color:"gold"};
        // lore line = {text:"Old blade", italic:0b}.
        let mut styled_name = vec![10u8];
        write_named_nbt_string(&mut styled_name, "text", "Sword");
        write_named_nbt_byte(&mut styled_name, "bold", 1);
        write_named_nbt_string(&mut styled_name, "color", "gold");
        styled_name.push(0);
        let mut styled_lore = vec![10u8];
        write_named_nbt_string(&mut styled_lore, "text", "Old blade");
        write_named_nbt_byte(&mut styled_lore, "italic", 0);
        styled_lore.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(6);
        payload.write_bytes(&styled_name);
        payload.write_var_i32(11);
        payload.write_var_i32(1);
        payload.write_bytes(&styled_lore);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert!(decoder.is_empty());

        // Plain fields stay the concatenated text (old API delegation).
        assert_eq!(patch.custom_name.as_deref(), Some("Sword"));
        assert_eq!(patch.lore, vec!["Old blade".to_string()]);
        assert_eq!(
            patch.custom_name_styled,
            Some(vec![StyledTextRun {
                text: "Sword".to_string(),
                style: crate::ComponentStyle {
                    bold: Some(true),
                    color: Some(0xFF_AA_00),
                    ..Default::default()
                },
            }])
        );
        assert_eq!(
            patch.lore_styled,
            vec![vec![StyledTextRun {
                text: "Old blade".to_string(),
                style: crate::ComponentStyle {
                    italic: Some(false),
                    ..Default::default()
                },
            }]]
        );
    }

    #[test]
    fn decodes_villager_variant_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(83);
        payload.write_var_i32(2);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![83],
                villager_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_attack_range_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(30);
        payload.write_f32(2.0);
        payload.write_f32(4.5);
        payload.write_f32(2.0);
        payload.write_f32(6.5);
        payload.write_f32(0.125);
        payload.write_f32(0.5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![30],
                removed_type_ids: Vec::new(),
                attack_range: Some(AttackRangeSummary {
                    min_reach: 2.0,
                    max_reach: 4.5,
                    min_creative_reach: 2.0,
                    max_creative_reach: 6.5,
                    hitbox_margin: 0.125,
                    mob_factor: 0.5,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_swing_animation_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(40);
        payload.write_var_i32(2);
        payload.write_var_i32(17);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![40],
                removed_type_ids: Vec::new(),
                swing_animation: Some(SwingAnimationSummary {
                    animation_type: SwingAnimationTypeSummary::Stab,
                    duration: 17,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_use_effects_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_f32(1.0);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![5],
                removed_type_ids: Vec::new(),
                use_effects: Some(UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: 1.0,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_consumable_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(24);
        payload.write_f32(0.8);
        payload.write_var_i32(2);
        write_direct_sound_event(&mut payload, "minecraft:entity.generic.drink", None);
        payload.write_bool(true);
        payload.write_var_i32(0);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![24],
                removed_type_ids: Vec::new(),
                consumable: Some(ConsumableSummary {
                    consume_seconds: 0.8,
                    animation: ItemUseAnimationSummary::Drink,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_item_rarity_out_of_bounds_as_common() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(12);
        payload.write_var_i32(99);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![12],
                removed_type_ids: Vec::new(),
                rarity: Some(ItemRaritySummary::Common),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_enchantments_component_summary_in_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(13);
        payload.write_var_i32(3);
        payload.write_var_i32(37);
        payload.write_var_i32(4);
        payload.write_var_i32(12);
        payload.write_var_i32(1);
        payload.write_var_i32(300);
        payload.write_var_i32(5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![13],
                removed_type_ids: Vec::new(),
                enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 37,
                        level: 4,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 300,
                        level: 5,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_stored_enchantments_component_summary_in_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(42);
        payload.write_var_i32(2);
        payload.write_var_i32(8);
        payload.write_var_i32(3);
        payload.write_var_i32(22);
        payload.write_var_i32(5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![42],
                removed_type_ids: Vec::new(),
                stored_enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 8,
                        level: 3,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 22,
                        level: 5,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_map_component_summary_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(46);
        payload.write_var_i32(123);
        payload.write_var_i32(48);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![46, 48],
                removed_type_ids: Vec::new(),
                map_id: Some(123),
                map_post_processing: Some(MapPostProcessingSummary::Scale),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_map_post_processing_out_of_bounds_as_lock() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(48);
        payload.write_var_i32(99);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch.map_post_processing,
            Some(MapPostProcessingSummary::Lock)
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_common_complex_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(11);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Line one"));
        payload.write_bytes(&nbt_string_root("Line two"));

        payload.write_var_i32(13);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_var_i32(3);
        payload.write_var_i32(9);
        payload.write_var_i32(1);

        payload.write_var_i32(17);
        payload.write_var_i32(2);
        payload.write_f32(1.0);
        payload.write_f32(2.5);
        payload.write_var_i32(2);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_var_i32(1);
        payload.write_string("variant");
        payload.write_var_i32(2);
        payload.write_i32(0x112233);
        payload.write_i32(0x445566);

        payload.write_var_i32(18);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_var_i32(11);
        payload.write_var_i32(13);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![11, 13, 17, 18],
                removed_type_ids: Vec::new(),
                enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 5,
                        level: 3,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 9,
                        level: 1,
                    },
                ],
                custom_model_data_floats: vec![1.0, 2.5].into(),
                custom_model_data_flags: vec![true, false],
                custom_model_data_strings: vec!["variant".to_string()],
                custom_model_data_colors: vec![0x112233, 0x445566],
                tooltip_hide_tooltip: true,
                tooltip_hidden_component_type_ids: vec![11, 13],
                lore: vec!["Line one".to_string(), "Line two".to_string()],
                lore_styled: vec![plain_runs("Line one"), plain_runs("Line two")],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_interaction_and_attribute_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);
        payload.write_var_i32(0);

        payload.write_var_i32(14);
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_bool(true);
        payload.write_string("north");
        payload.write_string("age");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("1");
        payload.write_bool(true);
        payload.write_string("3");
        payload.write_bool(true);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(1);
        payload.write_bool(false);
        payload.write_var_i32(6);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(15);
        payload.write_var_i32(1);
        write_empty_block_predicate(&mut payload);

        payload.write_var_i32(16);
        payload.write_var_i32(3);
        write_attribute_modifier_entry(&mut payload, "minecraft:test/default", 0, None);
        write_attribute_modifier_entry(&mut payload, "minecraft:test/hidden", 1, None);
        write_attribute_modifier_entry(
            &mut payload,
            "minecraft:test/override",
            2,
            Some("Override"),
        );

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![14, 15, 16],
                removed_type_ids: Vec::new(),
                can_place_on: Some(AdventureModePredicateSummary {
                    unknown: false,
                    block_registry_ids: vec![1],
                    unresolved_block_tag_count: 0,
                }),
                can_break: Some(AdventureModePredicateSummary {
                    unknown: true,
                    block_registry_ids: Vec::new(),
                    unresolved_block_tag_count: 0,
                }),
                attribute_modifiers: vec![
                    AttributeModifierSummary {
                        attribute_id: 7,
                        modifier_id: "minecraft:test/default".to_string(),
                        amount_bits: 1.5f64.to_bits(),
                        operation_id: 0,
                        slot_id: 1,
                        display_id: 0,
                        display_text: None,
                    },
                    AttributeModifierSummary {
                        attribute_id: 7,
                        modifier_id: "minecraft:test/hidden".to_string(),
                        amount_bits: 1.5f64.to_bits(),
                        operation_id: 0,
                        slot_id: 1,
                        display_id: 1,
                        display_text: None,
                    },
                    AttributeModifierSummary {
                        attribute_id: 7,
                        modifier_id: "minecraft:test/override".to_string(),
                        amount_bits: 1.5f64.to_bits(),
                        operation_id: 0,
                        slot_id: 1,
                        display_id: 2,
                        display_text: Some("Override".to_string()),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_adventure_mode_predicate_tooltip_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(14);
        payload.write_var_i32(3);
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_ids(payload, &[1, 193]);
        });
        write_empty_block_predicate(&mut payload);
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_ids(payload, &[1, 8]);
        });

        payload.write_var_i32(15);
        payload.write_var_i32(2);
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_tag(payload, "minecraft:mineable/pickaxe");
        });
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_ids(payload, &[9]);
        });

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch.can_place_on,
            Some(AdventureModePredicateSummary {
                unknown: true,
                block_registry_ids: vec![1, 193, 8],
                unresolved_block_tag_count: 0,
            })
        );
        assert_eq!(
            patch.can_break,
            Some(AdventureModePredicateSummary {
                unknown: false,
                block_registry_ids: vec![9],
                unresolved_block_tag_count: 1,
            })
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_combat_item_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(36);
        payload.write_var_i32(2);
        payload.write_var_i32(2);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.totem.use", None);

        payload.write_var_i32(37);
        payload.write_f32(0.25);
        payload.write_f32(1.5);
        payload.write_var_i32(1);
        payload.write_f32(90.0);
        payload.write_bool(true);
        write_holder_set_tag(&mut payload, "minecraft:bypasses_shield");
        payload.write_f32(1.0);
        payload.write_f32(0.5);
        payload.write_f32(1.0);
        payload.write_f32(0.0);
        payload.write_f32(1.0);
        payload.write_bool(true);
        write_holder_set_ids(&mut payload, &[3]);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.shield.block"));
        write_optional_direct_sound_event(&mut payload, None);

        payload.write_var_i32(38);
        payload.write_bool(true);
        payload.write_bool(true);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.mace.smash_air"));
        write_optional_direct_sound_event(&mut payload, None);

        payload.write_var_i32(39);
        payload.write_var_i32(10);
        payload.write_var_i32(2);
        payload.write_bool(true);
        write_kinetic_weapon_condition(&mut payload, 20, 0.25, 0.5);
        payload.write_bool(false);
        payload.write_bool(true);
        write_kinetic_weapon_condition(&mut payload, 30, 1.0, 1.5);
        payload.write_f32(0.2);
        payload.write_f32(2.0);
        write_optional_direct_sound_event(&mut payload, None);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.mace.smash_ground"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![36, 37, 38, 39],
                removed_type_ids: Vec::new(),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_animal_variant_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [85, 86, 87, 88, 91, 92, 101, 103, 104];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        for (index, component_id) in component_ids.iter().enumerate() {
            payload.write_var_i32(*component_id);
            payload.write_var_i32(index as i32);
        }

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                tropical_fish_pattern_id: Some(3),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_banner_pattern_layers_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(72);
        payload.write_var_i32(2);
        payload.write_var_i32(6);
        payload.write_var_i32(14);
        payload.write_var_i32(0);
        payload.write_string("example:custom_banner/pattern");
        payload.write_string("block.example.banner.custom");
        payload.write_var_i32(999);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![72],
                removed_type_ids: Vec::new(),
                banner_pattern_layers: vec![
                    BannerPatternLayerSummary {
                        registry_id: Some(5),
                        translation_key: None,
                        color_id: 14,
                    },
                    BannerPatternLayerSummary {
                        registry_id: None,
                        translation_key: Some("block.example.banner.custom".to_string()),
                        color_id: 999,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_profile_and_decorative_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [65, 67, 70, 72, 74, 77];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(65);
        write_holder_set_tag(&mut payload, "minecraft:no_item_required");

        payload.write_var_i32(67);
        payload.write_bool(true);
        payload.write_string("minecraft:overworld");
        payload.write_i64(0);
        payload.write_bool(true);

        payload.write_var_i32(70);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("Steve");
        payload.write_bool(true);
        payload.write_uuid(Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678));
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("skin-value");
        payload.write_bool(true);
        payload.write_string("skin-signature");
        payload.write_bool(true);
        payload.write_string("minecraft:entity/player/wide/steve");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("minecraft:entity/player/elytra");
        payload.write_bool(true);
        payload.write_bool(true);

        payload.write_var_i32(72);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_var_i32(14);
        payload.write_var_i32(0);
        payload.write_string("minecraft:stripe_bottom");
        payload.write_string("block.minecraft.banner.stripe_bottom");
        payload.write_var_i32(11);

        payload.write_var_i32(74);
        payload.write_var_i32(4);
        for item_id in [1, 2, 3, 4] {
            payload.write_var_i32(item_id);
        }

        payload.write_var_i32(77);
        payload.write_var_i32(1);
        payload.write_var_i32(3);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(40);
        payload.write_var_i32(2400);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                banner_pattern_layers: vec![
                    BannerPatternLayerSummary {
                        registry_id: Some(4),
                        translation_key: None,
                        color_id: 14,
                    },
                    BannerPatternLayerSummary {
                        registry_id: None,
                        translation_key: Some("block.minecraft.banner.stripe_bottom".to_string()),
                        color_id: 11,
                    },
                ],
                pot_decorations_item_ids: vec![1, 2, 3, 4],
                bees_present: true,
                bees_count: 1,
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos: chunks::BlockPos { x: 0, y: 0, z: 0 },
                }),
                lodestone_tracked: Some(true),
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::Partial,
                    uuid: Some(Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678)),
                    name: Some("Steve".to_string()),
                    properties: vec![GameProfilePropertySummary {
                        name: "textures".to_string(),
                        value: "skin-value".to_string(),
                        signature: Some("skin-signature".to_string()),
                    }],
                    profile_textures: None,
                    skin_patch: PlayerSkinPatchSummary {
                        body: Some(ResourceTextureSummary {
                            asset_id: "minecraft:entity/player/wide/steve".to_string(),
                            texture_path: "minecraft:textures/entity/player/wide/steve.png"
                                .to_string(),
                        }),
                        cape: None,
                        elytra: Some(ResourceTextureSummary {
                            asset_id: "minecraft:entity/player/elytra".to_string(),
                            texture_path: "minecraft:textures/entity/player/elytra.png".to_string(),
                        }),
                        model: Some(PlayerModelTypeSummary::Slim),
                    },
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_empty_bees_data_component_presence() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(77);
        payload.write_var_i32(0);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![77],
                removed_type_ids: Vec::new(),
                bees_present: true,
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_game_profile_data_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        let profile_id = Uuid::from_u128(0xabcdef01_2345_6789_0011_223344556677);
        payload.write_var_i32(70);
        payload.write_bool(true);
        payload.write_uuid(profile_id);
        payload.write_string("Alex");
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=");
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![70],
                removed_type_ids: Vec::new(),
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::GameProfile,
                    uuid: Some(profile_id),
                    name: Some("Alex".to_string()),
                    properties: vec![GameProfilePropertySummary {
                        name: "textures".to_string(),
                        value: "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=".to_string(),
                        signature: None,
                    }],
                    profile_textures: Some(ProfileTexturesSummary {
                        skin: Some(ProfileSkinTextureSummary {
                            url: "https://textures.minecraft.net/texture/skinhash".to_string(),
                            model: PlayerModelTypeSummary::Slim,
                        }),
                        cape: Some(ProfileTextureSummary {
                            url: "https://textures.minecraft.net/texture/capehash".to_string(),
                        }),
                        elytra: Some(ProfileTextureSummary {
                            url: "https://textures.minecraft.net/texture/elytrahash".to_string(),
                        }),
                    }),
                    skin_patch: PlayerSkinPatchSummary {
                        body: None,
                        cape: None,
                        elytra: None,
                        model: Some(PlayerModelTypeSummary::Wide),
                    },
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_entity_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [58, 59, 60];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(58);
        payload.write_var_i32(1);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(59);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(60);
        payload.write_var_i32(2);
        payload.write_bytes(&spawner_block_entity_nbt_root("minecraft:zombie"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                entity_data_entity_type_id: Some(1),
                block_entity_spawn_entity_type: Some("minecraft:zombie".to_string()),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_additional_item_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [
            0, 5, 23, 24, 25, 26, 28, 29, 30, 32, 40, 44, 45, 49, 50, 51, 53, 54, 55, 56, 61, 64,
            68, 69, 75, 76, 80, 102,
        ];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(0);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_f32(0.5);

        payload.write_var_i32(23);
        payload.write_var_i32(6);
        payload.write_f32(7.2);
        payload.write_bool(true);

        payload.write_var_i32(24);
        payload.write_f32(1.6);
        payload.write_var_i32(2);
        write_direct_sound_event(&mut payload, "minecraft:entity.generic.drink", None);
        payload.write_bool(true);
        payload.write_var_i32(5);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(5);
        write_mob_effect_details(&mut payload, false);
        payload.write_f32(0.75);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(6);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_f32(16.0);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.honey_bottle.drink", None);

        payload.write_var_i32(25);
        write_item_stack_template(&mut payload, 42, 1);

        payload.write_var_i32(26);
        payload.write_f32(1.25);
        payload.write_bool(true);
        payload.write_string("minecraft:ender_pearl");

        payload.write_var_i32(28);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_f32(8.0);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_f32(1.0);
        payload.write_var_i32(1);
        payload.write_bool(true);

        payload.write_var_i32(29);
        payload.write_var_i32(1);
        payload.write_f32(0.5);

        payload.write_var_i32(30);
        for value in [0.0, 3.0, 0.0, 5.0, 0.3, 1.0] {
            payload.write_f32(value);
        }

        payload.write_var_i32(32);
        payload.write_var_i32(5);
        write_direct_sound_event(&mut payload, "minecraft:item.armor.equip_generic", None);
        payload.write_bool(true);
        payload.write_string("minecraft:diamond");
        payload.write_bool(true);
        payload.write_string("minecraft:misc/pumpkinblur");
        payload.write_bool(true);
        payload.write_var_i32(0);
        payload.write_string("minecraft:skeletons");
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_bool(true);
        write_direct_sound_event(&mut payload, "minecraft:item.shears.snip", None);

        payload.write_var_i32(40);
        payload.write_var_i32(0);
        payload.write_var_i32(6);

        payload.write_var_i32(44);
        payload.write_i32(0x112233);
        payload.write_var_i32(45);
        payload.write_i32(0x445566);

        payload.write_var_i32(49);
        payload.write_var_i32(2);
        write_item_stack_template(&mut payload, 50, 1);
        write_item_stack_template(&mut payload, 51, 2);

        payload.write_var_i32(50);
        payload.write_var_i32(1);
        write_item_stack_template(&mut payload, 52, 3);

        payload.write_var_i32(51);
        payload.write_bool(true);
        payload.write_var_i32(3);
        payload.write_bool(true);
        payload.write_i32(0x778899);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        write_mob_effect_details(&mut payload, false);
        payload.write_bool(true);
        payload.write_string("healing");

        payload.write_var_i32(53);
        payload.write_var_i32(1);
        payload.write_var_i32(4);
        payload.write_var_i32(160);

        payload.write_var_i32(54);
        payload.write_var_i32(1);
        write_filterable_string(&mut payload, "raw page", Some("filtered page"));

        payload.write_var_i32(55);
        write_filterable_string(&mut payload, "Title", None);
        payload.write_string("Author");
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_bytes(&nbt_string_root("Page"));
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Filtered"));
        payload.write_bool(true);

        payload.write_var_i32(56);
        payload.write_var_i32(2);
        payload.write_var_i32(3);

        payload.write_var_i32(61);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_f32(1.0);
        payload.write_f32(16.0);
        payload.write_bytes(&nbt_string_root("Instrument"));

        payload.write_var_i32(64);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_bytes(&nbt_string_root("Song"));
        payload.write_f32(120.0);
        payload.write_var_i32(15);

        payload.write_var_i32(68);
        write_firework_explosion(&mut payload, 2);

        payload.write_var_i32(69);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        write_firework_explosion(&mut payload, 0);

        payload.write_var_i32(75);
        payload.write_var_i32(3);
        payload.write_bool(false);
        payload.write_bool(true);
        write_item_stack_template(&mut payload, 53, 4);
        payload.write_bool(false);

        payload.write_var_i32(76);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_string("north");
        payload.write_string("lit");
        payload.write_string("true");

        payload.write_var_i32(80);
        write_direct_sound_event(&mut payload, "minecraft:block.note_block.harp", Some(16.0));

        payload.write_var_i32(102);
        payload.write_var_i32(0);
        payload.write_var_i32(16);
        payload.write_var_i32(16);
        payload.write_string("minecraft:kebab");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Painter"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                custom_data: Some(NbtSummaryValue::Compound(Vec::new())),
                dyed_color: Some(0x112233),
                map_color: Some(0x445566),
                use_cooldown_ticks: Some(25),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                use_effects: Some(UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: 0.5,
                }),
                consumable: Some(ConsumableSummary {
                    consume_seconds: 1.6,
                    animation: ItemUseAnimationSummary::Drink,
                }),
                attack_range: Some(AttackRangeSummary {
                    min_reach: 0.0,
                    max_reach: 3.0,
                    min_creative_reach: 0.0,
                    max_creative_reach: 5.0,
                    hitbox_margin: 0.3,
                    mob_factor: 1.0,
                }),
                swing_animation: Some(SwingAnimationSummary {
                    animation_type: SwingAnimationTypeSummary::None,
                    duration: 6,
                }),
                potion_custom_color: Some(0x778899),
                potion_id: Some(3),
                potion_custom_effect_count: Some(1),
                potion_custom_effects: vec![MobEffectInstanceSummary {
                    effect_id: 2,
                    amplifier: 1,
                    duration: 200,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    hidden_effect: None,
                }],
                potion_custom_name: Some("healing".to_string()),
                suspicious_stew_effects: vec![SuspiciousStewEffectSummary {
                    effect_id: 4,
                    duration: 160,
                }],
                firework_explosion_colors: vec![0x010203, 0x040506],
                firework_explosion_fade_colors: vec![0x070809],
                firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
                firework_explosion_has_trail: Some(true),
                firework_explosion_has_twinkle: Some(false),
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(1),
                fireworks_explosions: vec![FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::SmallBall,
                    colors: vec![0x010203, 0x040506],
                    fade_colors: vec![0x070809],
                    has_trail: true,
                    has_twinkle: false,
                }],
                writable_book_pages: vec!["raw page".to_string()],
                writable_book_page_filters: vec![Some("filtered page".to_string())],
                written_book: Some(WrittenBookContentSummary {
                    title: "Title".to_string(),
                    title_filter: None,
                    author: "Author".to_string(),
                    generation: 1,
                    pages: vec!["Page".to_string()],
                    page_filters: vec![Some("Filtered".to_string())],
                    resolved: true,
                }),
                charged_projectiles_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 50,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 51,
                        count: 2,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 52,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                bundle_contents_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 53,
                    count: 4,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                container_item_count: Some(1),
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(2),
                instrument_description: Some("Instrument".to_string()),
                instrument_description_styled: Some(plain_runs("Instrument")),
                jukebox_direct_song: Some(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(0),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    description: "Song".to_string(),
                    length_in_seconds_bits: 120.0f32.to_bits(),
                    comparator_output: 15,
                }),
                block_state_properties: BTreeMap::from([
                    ("facing".to_string(), "north".to_string()),
                    ("lit".to_string(), "true".to_string()),
                ]),
                painting_variant_direct: Some(PaintingVariantSummary {
                    width: 16,
                    height: 16,
                    asset_id: "minecraft:kebab".to_string(),
                    title: None,
                    title_styled: None,
                    author: Some("Painter".to_string()),
                    author_styled: Some(plain_runs("Painter")),
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_book_content_component_summaries() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(54);
        payload.write_var_i32(2);
        write_filterable_string(&mut payload, "first page", None);
        write_filterable_string(&mut payload, "second raw", Some("second filtered"));

        payload.write_var_i32(55);
        write_filterable_string(&mut payload, "Guide", Some("Filtered Guide"));
        payload.write_string("Alex");
        payload.write_var_i32(2);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Chapter one"));
        payload.write_bool(false);
        payload.write_bytes(&nbt_string_root("Raw chapter two"));
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Filtered chapter two"));
        payload.write_bool(true);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![54, 55],
                removed_type_ids: Vec::new(),
                writable_book_pages: vec!["first page".to_string(), "second raw".to_string()],
                writable_book_page_filters: vec![None, Some("second filtered".to_string())],
                written_book: Some(WrittenBookContentSummary {
                    title: "Guide".to_string(),
                    title_filter: Some("Filtered Guide".to_string()),
                    author: "Alex".to_string(),
                    generation: 2,
                    pages: vec!["Chapter one".to_string(), "Raw chapter two".to_string()],
                    page_filters: vec![None, Some("Filtered chapter two".to_string())],
                    resolved: true,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_bundle_contents_item_count_from_component_patch() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(50);
        payload.write_var_i32(2);
        write_item_stack_template_with_patch(&mut payload, 12, 1, |payload| {
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(44);
            payload.write_i32(0x224466);
        });
        write_item_stack_template_with_patch(&mut payload, 34, 3, |payload| {
            payload.write_var_i32(2);
            payload.write_var_i32(1);
            payload.write_var_i32(2);
            payload.write_var_i32(512);
            payload.write_var_i32(3);
            payload.write_var_i32(17);
            payload.write_var_i32(45);
        });

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![50],
                removed_type_ids: Vec::new(),
                bundle_contents_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 12,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added: 1,
                            added_type_ids: vec![44],
                            removed_type_ids: Vec::new(),
                            dyed_color: Some(0x224466),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 34,
                        count: 3,
                        component_patch: DataComponentPatchSummary {
                            added: 2,
                            added_type_ids: vec![2, 3],
                            removed_type_ids: vec![45],
                            max_damage: Some(512),
                            damage: Some(17),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                bundle_contents_item_count: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn rejects_invalid_identifier_data_component_values() {
        assert_invalid_data_component_identifier(10, |payload| {
            payload.write_string("minecraft:DiamondSword");
        });
        assert_invalid_data_component_identifier(35, |payload| {
            payload.write_string("minecraft:Tooltip");
        });
        assert_invalid_data_component_identifier(71, |payload| {
            payload.write_string("minecraft:NoteBlock");
        });
        assert_invalid_data_component_identifier(26, |payload| {
            payload.write_f32(1.0);
            payload.write_bool(true);
            payload.write_string("minecraft:EnderPearl");
        });
        assert_invalid_data_component_identifier(65, |payload| {
            write_holder_set_tag(payload, "minecraft:NoItemRequired");
        });
        assert_invalid_data_component_identifier(67, |payload| {
            payload.write_bool(true);
            payload.write_string("minecraft:Overworld");
            payload.write_i64(0);
        });
        assert_invalid_data_component_identifier(32, |payload| {
            payload.write_var_i32(5);
            write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
            payload.write_bool(true);
            payload.write_string("minecraft:Diamond");
        });
        assert_invalid_data_component_identifier(32, |payload| {
            payload.write_var_i32(5);
            write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
            payload.write_bool(false);
            payload.write_bool(true);
            payload.write_string("minecraft:Misc/Pumpkinblur");
        });
        assert_invalid_data_component_identifier(80, |payload| {
            payload.write_var_i32(0);
            payload.write_string("minecraft:Block.NoteBlock.Harp");
        });
        assert_invalid_data_component_identifier(102, |payload| {
            payload.write_var_i32(0);
            payload.write_var_i32(16);
            payload.write_var_i32(16);
            payload.write_string("minecraft:Kebab");
        });
    }

    #[test]
    fn rejects_unknown_data_component_type_without_consuming_payload_guesswork() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(110);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported data component type id 110"));
    }

    fn assert_invalid_data_component_identifier(
        type_id: i32,
        write_value: impl FnOnce(&mut Encoder),
    ) {
        let payload = single_data_component_payload(type_id, write_value);
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(
            err.to_string().contains("invalid resource location"),
            "component {type_id} produced unexpected error: {err}"
        );
    }

    fn single_data_component_payload(
        type_id: i32,
        write_value: impl FnOnce(&mut Encoder),
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(type_id);
        write_value(&mut payload);
        payload.into_inner()
    }

    pub(super) fn nbt_string_root(value: &str) -> Vec<u8> {
        let mut out = vec![8];
        write_mutf8(&mut out, value);
        out
    }

    fn plain_runs(text: &str) -> Vec<StyledTextRun> {
        vec![StyledTextRun {
            text: text.to_string(),
            style: Default::default(),
        }]
    }

    fn empty_nbt_compound_root() -> Vec<u8> {
        vec![10, 0]
    }

    fn custom_data_nbt_root() -> Vec<u8> {
        let mut out = vec![10];
        write_named_nbt_string(&mut out, "owner", "Alex");
        write_named_nbt_int(&mut out, "level", 7);
        out.push(10);
        write_mutf8(&mut out, "nested");
        write_named_nbt_byte(&mut out, "flag", 1);
        out.push(0);
        out.push(9);
        write_mutf8(&mut out, "lore");
        out.push(8);
        out.extend_from_slice(&2i32.to_be_bytes());
        write_mutf8(&mut out, "one");
        write_mutf8(&mut out, "two");
        out.push(0);
        out
    }

    fn spawner_block_entity_nbt_root(entity_id: &str) -> Vec<u8> {
        let mut out = vec![10];
        out.push(10);
        write_mutf8(&mut out, "SpawnData");
        out.push(10);
        write_mutf8(&mut out, "entity");
        write_named_nbt_string(&mut out, "id", entity_id);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn write_named_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_mutf8(out, name);
        write_mutf8(out, value);
    }

    fn write_named_nbt_int(out: &mut Vec<u8>, name: &str, value: i32) {
        out.push(3);
        write_mutf8(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_named_nbt_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        out.push(1);
        write_mutf8(out, name);
        out.push(value as u8);
    }

    fn write_filterable_string(payload: &mut Encoder, raw: &str, filtered: Option<&str>) {
        payload.write_string(raw);
        match filtered {
            Some(filtered) => {
                payload.write_bool(true);
                payload.write_string(filtered);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_mob_effect_details(payload: &mut Encoder, hidden: bool) {
        payload.write_var_i32(1);
        payload.write_var_i32(200);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_bool(hidden);
        if hidden {
            write_mob_effect_details(payload, false);
        }
    }

    fn write_firework_explosion(payload: &mut Encoder, shape: i32) {
        payload.write_var_i32(shape);
        payload.write_var_i32(2);
        payload.write_i32(0x010203);
        payload.write_i32(0x040506);
        payload.write_var_i32(1);
        payload.write_i32(0x070809);
        payload.write_bool(true);
        payload.write_bool(false);
    }

    fn write_item_stack_template(payload: &mut Encoder, item_id: i32, count: i32) {
        write_item_stack_template_with_patch(payload, item_id, count, |payload| {
            payload.write_var_i32(0);
            payload.write_var_i32(0);
        });
    }

    fn write_item_stack_template_with_patch(
        payload: &mut Encoder,
        item_id: i32,
        count: i32,
        write_patch: impl FnOnce(&mut Encoder),
    ) {
        payload.write_var_i32(item_id);
        payload.write_var_i32(count);
        write_patch(payload);
    }

    fn write_empty_block_predicate(payload: &mut Encoder) {
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    }

    fn write_block_predicate_with_holder_set(
        payload: &mut Encoder,
        write_holder_set: impl FnOnce(&mut Encoder),
    ) {
        payload.write_bool(true);
        write_holder_set(payload);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    }

    fn write_attribute_modifier_entry(
        payload: &mut Encoder,
        id: &str,
        display_type: i32,
        display_text: Option<&str>,
    ) {
        payload.write_var_i32(7);
        payload.write_string(id);
        payload.write_f64(1.5);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(display_type);
        if let Some(text) = display_text {
            payload.write_bytes(&nbt_string_root(text));
        }
    }

    fn write_holder_set_tag(payload: &mut Encoder, tag: &str) {
        payload.write_var_i32(0);
        payload.write_string(tag);
    }

    fn write_holder_set_ids(payload: &mut Encoder, ids: &[i32]) {
        payload.write_var_i32(ids.len() as i32 + 1);
        for id in ids {
            payload.write_var_i32(*id);
        }
    }

    fn write_optional_direct_sound_event(payload: &mut Encoder, id: Option<&str>) {
        match id {
            Some(id) => {
                payload.write_bool(true);
                write_direct_sound_event(payload, id, None);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_kinetic_weapon_condition(
        payload: &mut Encoder,
        max_duration_ticks: i32,
        min_speed: f32,
        min_relative_speed: f32,
    ) {
        payload.write_var_i32(max_duration_ticks);
        payload.write_f32(min_speed);
        payload.write_f32(min_relative_speed);
    }

    fn write_direct_sound_event(payload: &mut Encoder, id: &str, fixed_range: Option<f32>) {
        payload.write_var_i32(0);
        payload.write_string(id);
        match fixed_range {
            Some(range) => {
                payload.write_bool(true);
                payload.write_f32(range);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
