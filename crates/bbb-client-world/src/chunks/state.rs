use std::sync::Arc;

use bbb_protocol::packets::{inventory::ResolvableProfileSummary, EquipmentSlot};
use bbb_protocol::{NbtSummaryValue, StyledTextRun};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ChunkPos;

use super::{light::LightData, palette::PalettedContainerData};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkViewState {
    pub center: Option<ChunkPos>,
    pub radius: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkState {
    Missing,
    Received,
    Decoded,
    NeighborsReady,
    MeshPending,
    MeshBuilding,
    MeshReady,
    GpuUploading,
    GpuResidentHidden,
    Visible,
    Retiring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkColumn {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: Vec<HeightmapData>,
    #[serde(with = "arc_sections_serde")]
    pub sections: Vec<Arc<ChunkSection>>,
    #[serde(with = "arc_block_entities_serde")]
    pub block_entities: Arc<Vec<BlockEntityRecord>>,
    pub light: LightData,
}

mod arc_sections_serde {
    use std::fmt;
    use std::sync::Arc;

    use serde::{de, Deserializer, Serializer};

    use super::ChunkSection;

    pub(super) fn serialize<S>(
        sections: &[Arc<ChunkSection>],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(sections.iter().map(Arc::as_ref))
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Arc<ChunkSection>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SectionsVisitor;

        impl<'de> de::Visitor<'de> for SectionsVisitor {
            type Value = Vec<Arc<ChunkSection>>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of chunk sections")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut sections = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(4096));
                while let Some(section) = seq.next_element::<ChunkSection>()? {
                    sections.push(Arc::new(section));
                }
                Ok(sections)
            }
        }

        deserializer.deserialize_seq(SectionsVisitor)
    }
}

mod arc_block_entities_serde {
    use std::sync::Arc;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::BlockEntityRecord;

    pub(super) fn serialize<S>(
        records: &Arc<Vec<BlockEntityRecord>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        records.as_slice().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Arc<Vec<BlockEntityRecord>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<BlockEntityRecord>::deserialize(deserializer).map(Arc::new)
    }
}

impl bbb_render_types::TerrainChunkSource for ChunkColumn {
    #[inline]
    fn section(&self, section_index: usize) -> Option<bbb_render_types::TerrainSectionView<'_>> {
        let section = self.sections.get(section_index)?;
        let (sky_light, block_light) = self.light.section_refs(section_index + 1);
        Some(bbb_render_types::TerrainSectionView {
            block_states: &section.block_states,
            biomes: Some(&section.biomes),
            sky_light,
            block_light,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkProbeSummaryState {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: usize,
    pub sections: usize,
    pub block_entities: usize,
    pub sky_light_arrays: usize,
    pub block_light_arrays: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceColumnProbe {
    pub x: i32,
    pub z: i32,
    pub loaded: bool,
    pub top_y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_state_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_name: Option<String>,
    pub is_fluid: bool,
}

impl ChunkProbeSummaryState {
    pub(crate) fn from_chunk(chunk: &ChunkColumn) -> Self {
        Self {
            pos: chunk.pos,
            state: chunk.state,
            heightmaps: chunk.heightmaps.len(),
            sections: chunk.sections.len(),
            block_entities: chunk.block_entities.len(),
            sky_light_arrays: chunk.light.sky_array_count(),
            block_light_arrays: chunk.light.block_array_count(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeightmapData {
    pub kind_id: i32,
    pub data: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkSection {
    pub non_empty_block_count: i16,
    pub fluid_count: i16,
    pub block_states: PalettedContainerData,
    pub biomes: PalettedContainerData,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkullBlockEntityData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_block_sound: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ResolvableProfileSummary>,
}

/// Type-specific decoded client state for a `BlockEntityRecord`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum BlockEntityPayload {
    #[default]
    None,
    SignText(Box<SignBlockEntityTextState>),
    VaultSharedData(Box<VaultSharedDataState>),
    Campfire(Box<CampfireBlockEntityData>),
    Shelf(Box<ShelfBlockEntityData>),
    Brushable(Box<BrushableBlockEntityData>),
    ConduitDestroyTarget(Uuid),
    DecoratedPotSherds(Box<DecoratedPotSherdsState>),
    BannerPatterns(Box<BannerPatternsState>),
    EndGateway(EndGatewayBlockEntityData),
    Spawner(Box<SpawnerBlockEntityData>),
    TrialSpawner(Box<TrialSpawnerBlockEntityData>),
    CommandBlock(Box<crate::CommandBlockEntityDataState>),
    ComparatorOutput(i32),
    StructureBlock(Box<crate::StructureBlockEntityDataState>),
    JigsawBlock(Box<crate::JigsawBlockEntityDataState>),
    TestBlock(Box<crate::TestBlockEntityDataState>),
    TestInstanceBlock(Box<crate::TestInstanceBlockEntityDataState>),
    Skull(Box<SkullBlockEntityData>),
}

impl BlockEntityPayload {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn skull(
        note_block_sound: Option<String>,
        profile: Option<ResolvableProfileSummary>,
    ) -> Self {
        if note_block_sound.is_none() && profile.is_none() {
            Self::None
        } else {
            Self::Skull(Box::new(SkullBlockEntityData {
                note_block_sound,
                profile,
            }))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockEntityRecord {
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub type_id: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_nbt: Vec<u8>,
    pub nbt: Option<NbtPayloadSummary>,
    #[serde(default, skip_serializing_if = "BlockEntityPayload::is_none")]
    pub payload: BlockEntityPayload,
}

impl BlockEntityRecord {
    /// A block-entity record carrying only its position and type, the shape a
    /// chunk decode produces before any `BlockEntityData` payload arrives.
    pub fn empty(local_x: u8, y: i16, local_z: u8, type_id: i32) -> Self {
        Self {
            local_x,
            y,
            local_z,
            type_id,
            raw_nbt: Vec::new(),
            nbt: None,
            payload: BlockEntityPayload::None,
        }
    }

    pub fn with_raw_nbt(mut self, raw_nbt: Vec<u8>) -> Self {
        self.raw_nbt = raw_nbt;
        self
    }

    pub fn with_payload(mut self, payload: BlockEntityPayload) -> Self {
        self.payload = payload;
        self
    }

    pub fn sign_text(&self) -> Option<&SignBlockEntityTextState> {
        match &self.payload {
            BlockEntityPayload::SignText(data) => Some(data),
            _ => None,
        }
    }

    pub fn vault_shared_data(&self) -> Option<&VaultSharedDataState> {
        match &self.payload {
            BlockEntityPayload::VaultSharedData(data) => Some(data),
            _ => None,
        }
    }

    pub fn campfire(&self) -> Option<&CampfireBlockEntityData> {
        match &self.payload {
            BlockEntityPayload::Campfire(data) => Some(data),
            _ => None,
        }
    }

    pub fn shelf(&self) -> Option<&ShelfBlockEntityData> {
        match &self.payload {
            BlockEntityPayload::Shelf(data) => Some(data),
            _ => None,
        }
    }

    pub fn brushable(&self) -> Option<&BrushableBlockEntityData> {
        match &self.payload {
            BlockEntityPayload::Brushable(data) => Some(data),
            _ => None,
        }
    }

    pub fn conduit_destroy_target(&self) -> Option<Uuid> {
        match &self.payload {
            BlockEntityPayload::ConduitDestroyTarget(target) => Some(*target),
            _ => None,
        }
    }

    pub fn decorated_pot_sherds(&self) -> Option<&DecoratedPotSherdsState> {
        match &self.payload {
            BlockEntityPayload::DecoratedPotSherds(data) => Some(data),
            _ => None,
        }
    }

    pub fn banner_patterns(&self) -> Option<&BannerPatternsState> {
        match &self.payload {
            BlockEntityPayload::BannerPatterns(data) => Some(data),
            _ => None,
        }
    }

    pub fn end_gateway(&self) -> Option<EndGatewayBlockEntityData> {
        match &self.payload {
            BlockEntityPayload::EndGateway(data) => Some(*data),
            _ => None,
        }
    }

    pub fn spawner(&self) -> Option<&SpawnerBlockEntityData> {
        match &self.payload {
            BlockEntityPayload::Spawner(data) => Some(data),
            _ => None,
        }
    }

    pub fn trial_spawner(&self) -> Option<&TrialSpawnerBlockEntityData> {
        match &self.payload {
            BlockEntityPayload::TrialSpawner(data) => Some(data),
            _ => None,
        }
    }

    pub fn command_block(&self) -> Option<&crate::CommandBlockEntityDataState> {
        match &self.payload {
            BlockEntityPayload::CommandBlock(data) => Some(data),
            _ => None,
        }
    }

    pub fn command_block_mut(&mut self) -> Option<&mut crate::CommandBlockEntityDataState> {
        match &mut self.payload {
            BlockEntityPayload::CommandBlock(data) => Some(data),
            _ => None,
        }
    }

    pub fn comparator_output(&self) -> Option<i32> {
        match &self.payload {
            BlockEntityPayload::ComparatorOutput(output) => Some(*output),
            _ => None,
        }
    }

    pub fn structure_block(&self) -> Option<&crate::StructureBlockEntityDataState> {
        match &self.payload {
            BlockEntityPayload::StructureBlock(data) => Some(data),
            _ => None,
        }
    }

    pub fn jigsaw_block(&self) -> Option<&crate::JigsawBlockEntityDataState> {
        match &self.payload {
            BlockEntityPayload::JigsawBlock(data) => Some(data),
            _ => None,
        }
    }

    pub fn test_block(&self) -> Option<&crate::TestBlockEntityDataState> {
        match &self.payload {
            BlockEntityPayload::TestBlock(data) => Some(data),
            _ => None,
        }
    }

    pub fn test_instance_block(&self) -> Option<&crate::TestInstanceBlockEntityDataState> {
        match &self.payload {
            BlockEntityPayload::TestInstanceBlock(data) => Some(data),
            _ => None,
        }
    }

    pub fn note_block_sound(&self) -> Option<&str> {
        match &self.payload {
            BlockEntityPayload::Skull(skull) => skull.note_block_sound.as_deref(),
            _ => None,
        }
    }

    pub fn skull_profile(&self) -> Option<&ResolvableProfileSummary> {
        match &self.payload {
            BlockEntityPayload::Skull(skull) => skull.profile.as_ref(),
            _ => None,
        }
    }
}

/// The four `CampfireBlockEntity.getItems()` cooking slots exactly as
/// `getUpdateTag` sends them (`ContainerHelper.saveAllItems`). The persistent
/// `ItemStack` NBT stays unresolved here; `CampfireRenderer` resolves it
/// against the session registries, and `particleTick` only asks whether a slot
/// is occupied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampfireBlockEntityData {
    pub items: [Option<NbtSummaryValue>; 4],
}

impl CampfireBlockEntityData {
    /// Vanilla `CampfireBlockEntity.particleTick` inspects only whether each
    /// cooking slot holds a non-empty stack.
    pub fn occupied_slots(&self) -> [bool; 4] {
        std::array::from_fn(|slot| self.items[slot].is_some())
    }
}

/// The three `ShelfBlockEntity.getItems()` display slots plus the renderer's
/// `align_items_to_bottom` flag, exactly as `getUpdateTag` sends them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShelfBlockEntityData {
    pub items: [Option<NbtSummaryValue>; 3],
    pub align_items_to_bottom: bool,
}

/// The `BrushableBlockEntity.getUpdateTag` fields read by
/// `BrushableBlockRenderer`: the stack being dug out and the side the player
/// brushed from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrushableBlockEntityData {
    pub item: Option<NbtSummaryValue>,
    /// `Direction.get3DDataValue()`: down=0, up=1, north=2, south=3, west=4, east=5.
    pub hit_direction_3d: Option<u8>,
}

/// The two `TrialSpawnerStateData.getUpdateTag` fields consumed by the
/// client-side display-entity path. `spawn_entity` is the bounded persistent
/// input retained for the renderer projection; server spawn rules remain in
/// the raw block-entity NBT only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialSpawnerBlockEntityData {
    pub spawn_entity: Option<TrialSpawnerEntityNbtProjection>,
    pub next_mob_spawns_at: i64,
}

/// Fixed renderer-observable subset of the `SpawnData.entity` compound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialSpawnerEntityNbtProjection {
    pub entity_id: Option<String>,
    pub rotation_bits: [u32; 2],
    pub glowing: bool,
    pub ticks_frozen: i32,
    pub fall_flying: bool,
    pub baby: bool,
    pub left_handed: bool,
    pub equipment: Vec<TrialSpawnerEquipmentNbtProjection>,
}

/// One of the eight closed `EquipmentSlot.CODEC` keys plus the registry-aware
/// persistent `ItemStack` value that will be resolved against the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialSpawnerEquipmentNbtProjection {
    pub slot: EquipmentSlot,
    pub item: NbtSummaryValue,
}

/// The end gateway block entity fields read by `TheEndGatewayRenderer`:
/// vanilla saves `Age` in BE NBT and receives cooldown through block event
/// `1`. Decoded from chunk block-entity data and `BlockEntityData` updates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndGatewayBlockEntityData {
    pub age: i64,
}

/// The ordinary mob spawner block entity fields used by vanilla
/// `SpawnerRenderer`: the next display entity id and the client-side spin
/// delay counters. Decoded from chunk block-entity data and
/// `BlockEntityData` updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnerBlockEntityData {
    pub entity_id: Option<String>,
    pub spawn_delay: i32,
    pub min_spawn_delay: i32,
    pub required_player_range: i32,
}

/// The banner block entity's stored pattern layers — vanilla
/// `BannerPatternLayers` (the BE NBT `patterns` list,
/// `BannerBlockEntity.loadAdditional` reading `BannerPatternLayers.CODEC`).
/// The base color is not stored here: vanilla derives it from the block id
/// (`AbstractBannerBlock.getColor`). Decoded from the chunk block-entity
/// section and `BlockEntityData` updates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerPatternsState {
    pub layers: Vec<BannerPatternLayerState>,
}

/// One banner pattern layer — vanilla `BannerPatternLayers.Layer`: the
/// named or direct `BannerPattern.CODEC` holder and the `color` dye name
/// (`DyeColor.CODEC`, e.g. `lime`). Named holders preserve their resource id;
/// direct holders preserve both fields from `BannerPattern.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerPatternLayerState {
    pub pattern: BannerPatternHolderState,
    pub color: String,
}

/// Persistent `RegistryFileCodec` form used by a banner-pattern layer. The
/// untagged representation keeps named holders serialized as their original
/// string while direct values remain an exact two-field compound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BannerPatternHolderState {
    Named(String),
    Direct {
        asset_id: String,
        translation_key: bbb_protocol::JavaString,
    },
}

/// The decorated pot block entity's stored sherd faces — vanilla
/// `PotDecorations` (`back`/`left`/`right`/`front` item order, the BE NBT
/// `sherds` list). `Some(item_id)` carries the raw sherd item id; `None` is
/// an undecorated face (vanilla `Items.BRICK` or a missing list entry).
/// Decoded from the chunk block-entity section and `BlockEntityData` updates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecoratedPotSherdsState {
    pub back: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub front: Option<String>,
}

/// The sign block entity's stored text — vanilla `SignBlockEntity`'s
/// `frontText` / `backText` `SignText` pair plus the `is_waxed` flag. Decoded
/// from the chunk block-entity section and `BlockEntityData` updates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignBlockEntityTextState {
    pub front: SignTextSideState,
    pub back: SignTextSideState,
    pub is_waxed: bool,
}

impl SignBlockEntityTextState {
    pub fn side(&self, is_front_text: bool) -> &SignTextSideState {
        if is_front_text {
            &self.front
        } else {
            &self.back
        }
    }
}

/// One sign face's text — vanilla `SignText`: four raw lines plus an optional
/// filtered four-line variant of styled component runs (the shared
/// `StyledTextRun` flattening), the face's `DyeColor`, and the
/// `has_glowing_text` flag.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignTextSideState {
    pub lines: [Vec<StyledTextRun>; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filtered_lines: Option<[Vec<StyledTextRun>; 4]>,
    pub color: SignTextDyeColor,
    pub has_glowing_text: bool,
}

impl SignTextSideState {
    /// The effective styled lines selected by Vanilla's client text-filter
    /// setting. Missing/malformed filtered NBT falls back to the raw lines.
    pub fn lines_for_filter(&self, should_filter: bool) -> &[Vec<StyledTextRun>; 4] {
        if should_filter {
            self.filtered_lines.as_ref().unwrap_or(&self.lines)
        } else {
            &self.lines
        }
    }

    /// The plain concatenated text per line (styles dropped) — what the sign
    /// edit screen preloads (vanilla `AbstractSignEditScreen` edits raw
    /// strings).
    pub fn plain_lines(&self) -> [String; 4] {
        plain_sign_lines(&self.lines)
    }

    /// The effective plain text selected by the client text-filter setting.
    pub fn plain_lines_for_filter(&self, should_filter: bool) -> [String; 4] {
        plain_sign_lines(self.lines_for_filter(should_filter))
    }

    /// The decoded filtered text without fallback, retained so an open sign
    /// editor can snapshot both variants before later world updates arrive.
    pub fn filtered_plain_lines(&self) -> Option<[String; 4]> {
        self.filtered_lines.as_ref().map(plain_sign_lines)
    }

    /// Whether either selectable variant has text. Extraction cannot decide
    /// the command-line filtering policy, so it retains a face needed by
    /// either runtime choice.
    pub fn has_any_text(&self) -> bool {
        sign_lines_have_text(&self.lines)
            || self
                .filtered_lines
                .as_ref()
                .is_some_and(sign_lines_have_text)
    }
}

fn plain_sign_lines(lines: &[Vec<StyledTextRun>; 4]) -> [String; 4] {
    std::array::from_fn(|index| lines[index].iter().map(|run| run.text.as_str()).collect())
}

fn sign_lines_have_text(lines: &[Vec<StyledTextRun>; 4]) -> bool {
    lines
        .iter()
        .any(|line| line.iter().any(|run| !run.text.is_empty()))
}

/// Vanilla `DyeColor` for sign text (`SignText.color`, serialized by name via
/// `DyeColor.CODEC`). `text_color()` is `DyeColor.getTextColor()` — the
/// `textColor` constructor argument of each enum constant (`DyeColor.java`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignTextDyeColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    #[default]
    Black,
}

impl SignTextDyeColor {
    /// The `DyeColor.CODEC` name mapping; unknown or missing names fall back
    /// to black (vanilla `fieldOf("color").orElse(DyeColor.BLACK)`).
    pub fn from_name(name: &str) -> Self {
        match name {
            "white" => Self::White,
            "orange" => Self::Orange,
            "magenta" => Self::Magenta,
            "light_blue" => Self::LightBlue,
            "yellow" => Self::Yellow,
            "lime" => Self::Lime,
            "pink" => Self::Pink,
            "gray" => Self::Gray,
            "light_gray" => Self::LightGray,
            "cyan" => Self::Cyan,
            "purple" => Self::Purple,
            "blue" => Self::Blue,
            "brown" => Self::Brown,
            "green" => Self::Green,
            "red" => Self::Red,
            _ => Self::Black,
        }
    }

    /// Vanilla `DyeColor.getTextColor()` as `0xRRGGBB`.
    pub fn text_color(self) -> u32 {
        match self {
            Self::White => 0xFF_FF_FF,
            Self::Orange => 0xFF_68_1F,
            Self::Magenta => 0xFF_00_FF,
            Self::LightBlue => 0x9A_C0_CD,
            Self::Yellow => 0xFF_FF_00,
            Self::Lime => 0xBF_FF_00,
            Self::Pink => 0xFF_69_B4,
            Self::Gray => 0x80_80_80,
            Self::LightGray => 0xD3_D3_D3,
            Self::Cyan => 0x00_FF_FF,
            Self::Purple => 0xA0_20_F0,
            Self::Blue => 0x00_00_FF,
            Self::Brown => 0x8B_45_13,
            Self::Green => 0x00_FF_00,
            Self::Red => 0xFF_00_00,
            Self::Black => 0x00_00_00,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultSharedDataState {
    /// The persistent `VaultSharedData.displayItem` stack NBT, retained only
    /// when it is a renderable non-empty stack; `VaultRenderer` resolves it
    /// against the session registries.
    #[serde(default)]
    pub display_item: Option<NbtSummaryValue>,
    pub connected_players: Vec<Uuid>,
    pub connected_particles_range: f64,
}

impl VaultSharedDataState {
    /// Vanilla `VaultSharedData.hasDisplayItem`.
    pub fn has_display_item(&self) -> bool {
        self.display_item.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultConnectionParticleState {
    pub origin: [f64; 3],
    pub targets: Vec<VaultConnectionParticleTargetState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VaultConnectionParticleTargetState {
    pub entity_id: i32,
    pub uuid: Uuid,
    pub target_position: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtPayloadSummary {
    pub root_type: u8,
    pub byte_len: usize,
}
