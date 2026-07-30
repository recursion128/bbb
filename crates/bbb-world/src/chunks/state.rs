use bbb_protocol::packets::inventory::ResolvableProfileSummary;
use bbb_protocol::StyledTextRun;
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
    pub sections: Vec<ChunkSection>,
    pub block_entities: Vec<BlockEntityRecord>,
    pub light: LightData,
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

impl ChunkProbeSummaryState {
    pub(crate) fn from_chunk(chunk: &ChunkColumn) -> Self {
        Self {
            pos: chunk.pos,
            state: chunk.state,
            heightmaps: chunk.heightmaps.len(),
            sections: chunk.sections.len(),
            block_entities: chunk.block_entities.len(),
            sky_light_arrays: chunk.light.sky_updates.len(),
            block_light_arrays: chunk.light.block_updates.len(),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockEntityRecord {
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub type_id: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_nbt: Vec<u8>,
    pub nbt: Option<NbtPayloadSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_text: Option<SignBlockEntityTextState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_shared_data: Option<VaultSharedDataState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campfire: Option<CampfireBlockEntityData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conduit_destroy_target: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decorated_pot_sherds: Option<DecoratedPotSherdsState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_patterns: Option<BannerPatternsState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_gateway: Option<EndGatewayBlockEntityData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawner: Option<SpawnerBlockEntityData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_block: Option<crate::CommandBlockEntityDataState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparator_output: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure_block: Option<crate::StructureBlockEntityDataState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jigsaw_block: Option<crate::JigsawBlockEntityDataState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_block: Option<crate::TestBlockEntityDataState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_instance_block: Option<crate::TestInstanceBlockEntityDataState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_block_sound: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skull_profile: Option<ResolvableProfileSummary>,
}

impl BlockEntityRecord {
    pub(crate) fn empty(local_x: u8, y: i16, local_z: u8, type_id: i32) -> Self {
        Self {
            local_x,
            y,
            local_z,
            type_id,
            raw_nbt: Vec::new(),
            nbt: None,
            sign_text: None,
            vault_shared_data: None,
            campfire: None,
            conduit_destroy_target: None,
            decorated_pot_sherds: None,
            banner_patterns: None,
            end_gateway: None,
            spawner: None,
            command_block: None,
            comparator_output: None,
            structure_block: None,
            jigsaw_block: None,
            test_block: None,
            test_instance_block: None,
            note_block_sound: None,
            skull_profile: None,
        }
    }
}

/// Vanilla `CampfireBlockEntity.particleTick` only inspects whether each of
/// the four cooking slots is empty. The full registry-coupled `ItemStack`
/// remains outside this narrow client-effect projection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampfireBlockEntityData {
    pub occupied_slots: [bool; 4],
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
        translation_key: String,
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
    #[serde(default)]
    pub has_display_item: bool,
    pub connected_players: Vec<Uuid>,
    pub connected_particles_range: f64,
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
