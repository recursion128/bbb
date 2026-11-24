use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use bbb_protocol::MC_DATA_VERSION;
#[cfg(any(not(target_arch = "wasm32"), test))]
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugScreenEntryStatus {
    AlwaysOn,
    InOverlay,
    Never,
}

impl DebugScreenEntryStatus {
    fn vanilla_name(self) -> &'static str {
        match self {
            Self::AlwaysOn => "alwaysOn",
            Self::InOverlay => "inOverlay",
            Self::Never => "never",
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn from_vanilla_name(name: &str) -> Option<Self> {
        match name {
            "alwaysOn" => Some(Self::AlwaysOn),
            "inOverlay" | "inF3" => Some(Self::InOverlay),
            "never" => Some(Self::Never),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugScreenProfile {
    Default,
    Performance,
}

impl DebugScreenProfile {
    fn vanilla_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Performance => "performance",
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn from_vanilla_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::Default),
            "performance" => Some(Self::Performance),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugScreenEntryId {
    Biome,
    ChunkGenerationStats,
    ChunkRenderStats,
    ChunkSourceStats,
    DayCount,
    DetailedMemory,
    EntityRenderStats,
    EntitySpawnCounts,
    Fps,
    GameVersion,
    GpuUtilization,
    Heightmap,
    LightLevels,
    LocalDifficulty,
    LookingAtBlockState,
    LookingAtBlockTags,
    LookingAtEntity,
    LookingAtEntityTags,
    LookingAtFluidState,
    LookingAtFluidTags,
    Memory,
    ParticleRenderStats,
    PlayerPosition,
    PlayerSectionPosition,
    PostEffect,
    SimplePerformanceImpactors,
    SoundCache,
    SoundMood,
    SystemSpecs,
    Tps,
    ThreeDimensionalCrosshair,
    ChunkBorders,
    ChunkSectionOctree,
    ChunkSectionPaths,
    ChunkSectionVisibility,
    EntityHitboxes,
    VisualizeBlockLightLevels,
    VisualizeChunksOnServer,
    VisualizeCollisionBoxes,
    VisualizeEntitySupportingBlocks,
    VisualizeHeightmap,
    VisualizeSkyLightLevels,
    VisualizeSkyLightSections,
    VisualizeSolidFaces,
    VisualizeWaterLevels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugScreenEntryCategory {
    Text,
    Renderer,
}

use DebugScreenEntryCategory as C;
use DebugScreenEntryId as E;

type DebugScreenEntryRow = (
    DebugScreenEntryId,
    &'static str,
    DebugScreenEntryCategory,
    bool,
);

/// `DebugScreenEntries` registrations in debug options screen order (category, then id), with
/// whether the entry stays allowed under reduced debug info (`DebugScreenEntry.isAllowed`).
#[rustfmt::skip]
const DEBUG_SCREEN_ENTRIES: [DebugScreenEntryRow; 45] = [
    (E::Biome, "biome", C::Text, false),
    (E::ChunkGenerationStats, "chunk_generation_stats", C::Text, false),
    (E::ChunkRenderStats, "chunk_render_stats", C::Text, true),
    (E::ChunkSourceStats, "chunk_source_stats", C::Text, true),
    (E::DayCount, "day_count", C::Text, false),
    (E::DetailedMemory, "detailed_memory", C::Text, true),
    (E::EntityRenderStats, "entity_render_stats", C::Text, true),
    (E::EntitySpawnCounts, "entity_spawn_counts", C::Text, false),
    (E::Fps, "fps", C::Text, true),
    (E::GameVersion, "game_version", C::Text, true),
    (E::GpuUtilization, "gpu_utilization", C::Text, true),
    (E::Heightmap, "heightmap", C::Text, false),
    (E::LightLevels, "light_levels", C::Text, false),
    (E::LocalDifficulty, "local_difficulty", C::Text, false),
    (E::LookingAtBlockState, "looking_at_block_state", C::Text, false),
    (E::LookingAtBlockTags, "looking_at_block_tags", C::Text, false),
    (E::LookingAtEntity, "looking_at_entity", C::Text, false),
    (E::LookingAtEntityTags, "looking_at_entity_tags", C::Text, false),
    (E::LookingAtFluidState, "looking_at_fluid_state", C::Text, false),
    (E::LookingAtFluidTags, "looking_at_fluid_tags", C::Text, false),
    (E::Memory, "memory", C::Text, true),
    (E::ParticleRenderStats, "particle_render_stats", C::Text, false),
    (E::PlayerPosition, "player_position", C::Text, false),
    (E::PlayerSectionPosition, "player_section_position", C::Text, true),
    (E::PostEffect, "post_effect", C::Text, false),
    (E::SimplePerformanceImpactors, "simple_performance_impactors", C::Text, true),
    (E::SoundCache, "sound_cache", C::Text, true),
    (E::SoundMood, "sound_mood", C::Text, false),
    (E::SystemSpecs, "system_specs", C::Text, true),
    (E::Tps, "tps", C::Text, true),
    (E::ThreeDimensionalCrosshair, "3d_crosshair", C::Renderer, false),
    (E::ChunkBorders, "chunk_borders", C::Renderer, false),
    (E::ChunkSectionOctree, "chunk_section_octree", C::Renderer, false),
    (E::ChunkSectionPaths, "chunk_section_paths", C::Renderer, false),
    (E::ChunkSectionVisibility, "chunk_section_visibility", C::Renderer, false),
    (E::EntityHitboxes, "entity_hitboxes", C::Renderer, false),
    (E::VisualizeBlockLightLevels, "visualize_block_light_levels", C::Renderer, false),
    (E::VisualizeChunksOnServer, "visualize_chunks_on_server", C::Renderer, false),
    (E::VisualizeCollisionBoxes, "visualize_collision_boxes", C::Renderer, false),
    (E::VisualizeEntitySupportingBlocks, "visualize_entity_supporting_blocks", C::Renderer, false),
    (E::VisualizeHeightmap, "visualize_heightmap", C::Renderer, false),
    (E::VisualizeSkyLightLevels, "visualize_sky_light_levels", C::Renderer, false),
    (E::VisualizeSkyLightSections, "visualize_sky_light_sections", C::Renderer, false),
    (E::VisualizeSolidFaces, "visualize_solid_faces", C::Renderer, false),
    (E::VisualizeWaterLevels, "visualize_water_levels", C::Renderer, false),
];

// `registration` indexes the table by discriminant.
const _: () = {
    let mut index = 0;
    while index < DEBUG_SCREEN_ENTRIES.len() {
        assert!(DEBUG_SCREEN_ENTRIES[index].0 as usize == index);
        index += 1;
    }
};

impl DebugScreenEntryId {
    pub(crate) fn all_debug_options_ordered() -> impl Iterator<Item = Self> {
        DEBUG_SCREEN_ENTRIES.iter().map(|&(entry, ..)| entry)
    }

    fn registration(self) -> &'static DebugScreenEntryRow {
        &DEBUG_SCREEN_ENTRIES[self as usize]
    }

    pub(crate) fn path(self) -> &'static str {
        self.registration().1
    }

    pub(crate) fn category_label(self) -> &'static str {
        match self.registration().2 {
            C::Text => "Debug Screen Text",
            C::Renderer => "Debug Renderers",
        }
    }

    fn vanilla_id(self) -> String {
        format!("minecraft:{}", self.path())
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn from_vanilla_id(id: &str) -> Option<Self> {
        let path = match id.strip_prefix("minecraft:")? {
            // `DebugProfileLookingAtSplitFix`
            "looking_at_block" => "looking_at_block_state",
            "looking_at_fluid" => "looking_at_fluid_state",
            path => path,
        };
        DEBUG_SCREEN_ENTRIES
            .iter()
            .find(|row| row.1 == path)
            .map(|row| row.0)
    }

    pub(crate) fn is_allowed(self, reduced_debug_info: bool) -> bool {
        !reduced_debug_info || self.registration().3
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DebugScreenEntryList {
    overlay_visible: bool,
    profile: Option<DebugScreenProfile>,
    statuses: Vec<(DebugScreenEntryId, DebugScreenEntryStatus)>,
    unknown_statuses: BTreeMap<String, DebugScreenEntryStatus>,
}

impl Default for DebugScreenEntryList {
    fn default() -> Self {
        Self::from_profile(DebugScreenProfile::Default)
    }
}

impl DebugScreenEntryList {
    fn from_profile(profile: DebugScreenProfile) -> Self {
        let mut entries = Self {
            overlay_visible: false,
            profile: None,
            statuses: Vec::new(),
            unknown_statuses: BTreeMap::new(),
        };
        entries.load_profile(profile);
        entries
    }

    pub(crate) fn is_overlay_visible(&self) -> bool {
        self.overlay_visible
    }

    pub(crate) fn set_overlay_visible(&mut self, visible: bool) {
        if self.overlay_visible != visible {
            self.overlay_visible = visible;
        }
    }

    pub(crate) fn toggle_overlay(&mut self) {
        self.set_overlay_visible(!self.overlay_visible);
    }

    pub(crate) fn load_profile(&mut self, profile: DebugScreenProfile) {
        self.profile = Some(profile);
        self.statuses.clear();
        self.unknown_statuses.clear();
        self.statuses
            .extend_from_slice(debug_screen_profile_entries(profile));
    }

    pub(crate) fn is_using_profile(&self, profile: DebugScreenProfile) -> bool {
        self.profile == Some(profile)
    }

    pub(crate) fn status(&self, entry: DebugScreenEntryId) -> DebugScreenEntryStatus {
        self.statuses
            .iter()
            .find_map(|(id, status)| (*id == entry).then_some(*status))
            .unwrap_or(DebugScreenEntryStatus::Never)
    }

    pub(crate) fn set_status(&mut self, entry: DebugScreenEntryId, status: DebugScreenEntryStatus) {
        self.profile = None;
        if let Some((_, existing)) = self.statuses.iter_mut().find(|(id, _)| *id == entry) {
            *existing = status;
        } else {
            self.statuses.push((entry, status));
        }
    }

    pub(crate) fn toggle_status(&mut self, entry: DebugScreenEntryId) -> bool {
        match self.status(entry) {
            DebugScreenEntryStatus::AlwaysOn => {
                self.set_status(entry, DebugScreenEntryStatus::Never);
                false
            }
            DebugScreenEntryStatus::InOverlay => {
                if self.overlay_visible {
                    self.set_status(entry, DebugScreenEntryStatus::Never);
                    false
                } else {
                    self.set_status(entry, DebugScreenEntryStatus::AlwaysOn);
                    true
                }
            }
            DebugScreenEntryStatus::Never => {
                if self.overlay_visible {
                    self.set_status(entry, DebugScreenEntryStatus::InOverlay);
                } else {
                    self.set_status(entry, DebugScreenEntryStatus::AlwaysOn);
                }
                true
            }
        }
    }

    pub(crate) fn is_currently_enabled(
        &self,
        entry: DebugScreenEntryId,
        reduced_debug_info: bool,
    ) -> bool {
        let enabled_by_status = self.status(entry) == DebugScreenEntryStatus::AlwaysOn
            || (self.overlay_visible && self.status(entry) == DebugScreenEntryStatus::InOverlay);
        enabled_by_status && entry.is_allowed(reduced_debug_info)
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn load_from_debug_profile_file(
        path: &Path,
        fallback_profile: DebugScreenProfile,
    ) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self::from_profile(fallback_profile));
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("read debug profile file {}", path.display()))?;
        let serialized: SerializedDebugProfile = serde_json::from_str(&raw)
            .with_context(|| format!("parse debug profile file {}", path.display()))?;
        serialized.into_entries()
    }

    pub(crate) fn save_to_debug_profile_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create debug profile directory {}", parent.display()))?;
        }
        let serialized = self.serialized();
        let raw = serde_json::to_vec_pretty(&serialized).context("serialize debug profile")?;
        fs::write(path, raw).with_context(|| format!("write debug profile file {}", path.display()))
    }

    fn serialized(&self) -> SerializedDebugProfileOut {
        if let Some(profile) = self.profile {
            return SerializedDebugProfileOut {
                data_version: MC_DATA_VERSION,
                profile: Some(profile.vanilla_name().to_string()),
                custom: None,
            };
        }

        let mut custom = self
            .unknown_statuses
            .iter()
            .map(|(id, status)| (id.clone(), status.vanilla_name().to_string()))
            .collect::<BTreeMap<_, _>>();
        for (id, status) in &self.statuses {
            custom.insert(id.vanilla_id(), status.vanilla_name().to_string());
        }

        SerializedDebugProfileOut {
            data_version: MC_DATA_VERSION,
            profile: None,
            custom: Some(custom),
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, Deserialize)]
struct SerializedDebugProfile {
    #[serde(rename = "DataVersion")]
    _data_version: Option<i32>,
    profile: Option<String>,
    custom: Option<BTreeMap<String, String>>,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl SerializedDebugProfile {
    fn into_entries(self) -> Result<DebugScreenEntryList> {
        if let Some(profile) = self.profile {
            let profile = DebugScreenProfile::from_vanilla_name(&profile)
                .with_context(|| format!("unknown debug screen profile {profile:?}"))?;
            return Ok(DebugScreenEntryList::from_profile(profile));
        }

        let mut entries = DebugScreenEntryList {
            overlay_visible: false,
            profile: None,
            statuses: Vec::new(),
            unknown_statuses: BTreeMap::new(),
        };
        for (id, status) in self.custom.unwrap_or_default() {
            let status = DebugScreenEntryStatus::from_vanilla_name(&status)
                .with_context(|| format!("unknown debug screen entry status {status:?}"))?;
            if let Some(entry) = DebugScreenEntryId::from_vanilla_id(&id) {
                entries.set_status(entry, status);
            } else {
                entries.unknown_statuses.insert(id, status);
            }
        }

        Ok(entries)
    }
}

#[derive(Debug, Serialize)]
struct SerializedDebugProfileOut {
    #[serde(rename = "DataVersion")]
    data_version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom: Option<BTreeMap<String, String>>,
}

fn debug_screen_profile_entries(
    profile: DebugScreenProfile,
) -> &'static [(DebugScreenEntryId, DebugScreenEntryStatus)] {
    match profile {
        DebugScreenProfile::Default => &[
            (
                DebugScreenEntryId::ThreeDimensionalCrosshair,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::GameVersion,
                DebugScreenEntryStatus::InOverlay,
            ),
            (DebugScreenEntryId::Tps, DebugScreenEntryStatus::InOverlay),
            (DebugScreenEntryId::Fps, DebugScreenEntryStatus::InOverlay),
            (
                DebugScreenEntryId::Memory,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::SystemSpecs,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::PlayerPosition,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::PlayerSectionPosition,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::SimplePerformanceImpactors,
                DebugScreenEntryStatus::InOverlay,
            ),
        ],
        DebugScreenProfile::Performance => &[
            (DebugScreenEntryId::Tps, DebugScreenEntryStatus::InOverlay),
            (DebugScreenEntryId::Fps, DebugScreenEntryStatus::AlwaysOn),
            (
                DebugScreenEntryId::GpuUtilization,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::Memory,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::SimplePerformanceImpactors,
                DebugScreenEntryStatus::InOverlay,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn default_profile_matches_vanilla_entry_statuses() {
        let entries = DebugScreenEntryList::default();

        for entry in [
            DebugScreenEntryId::ThreeDimensionalCrosshair,
            DebugScreenEntryId::GameVersion,
            DebugScreenEntryId::Tps,
            DebugScreenEntryId::Fps,
            DebugScreenEntryId::Memory,
            DebugScreenEntryId::SystemSpecs,
            DebugScreenEntryId::PlayerPosition,
            DebugScreenEntryId::PlayerSectionPosition,
            DebugScreenEntryId::SimplePerformanceImpactors,
        ] {
            assert_eq!(entries.status(entry), DebugScreenEntryStatus::InOverlay);
        }
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::DetailedMemory),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::DayCount),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LightLevels),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::Heightmap),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::Biome),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LocalDifficulty),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::EntitySpawnCounts),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LookingAtBlockState),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LookingAtBlockTags),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LookingAtFluidState),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LookingAtFluidTags),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LookingAtEntity),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LookingAtEntityTags),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::ChunkRenderStats),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::ChunkGenerationStats),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityRenderStats),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::ParticleRenderStats),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::ChunkSourceStats),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::SoundCache),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::SoundMood),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::PostEffect),
            DebugScreenEntryStatus::Never
        );
        for entry in VANILLA_RENDERER_NOOP_NON_PROFILE_ENTRIES {
            assert_eq!(entries.status(entry), DebugScreenEntryStatus::Never);
        }
        assert!(entries.is_using_profile(DebugScreenProfile::Default));
    }

    #[test]
    fn performance_profile_keeps_fps_always_on() {
        let mut entries = DebugScreenEntryList::default();

        entries.load_profile(DebugScreenProfile::Performance);

        assert_eq!(
            entries.status(DebugScreenEntryId::Fps),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::GpuUtilization),
            DebugScreenEntryStatus::InOverlay
        );
        assert!(entries.is_using_profile(DebugScreenProfile::Performance));
    }

    #[test]
    fn toggle_status_follows_overlay_visibility() {
        let mut entries = DebugScreenEntryList::default();

        assert!(entries.toggle_status(DebugScreenEntryId::EntityHitboxes));
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert!(!entries.toggle_status(DebugScreenEntryId::EntityHitboxes));
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::Never
        );

        entries.set_overlay_visible(true);
        assert!(entries.toggle_status(DebugScreenEntryId::EntityHitboxes));
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::InOverlay
        );
        entries.set_overlay_visible(false);
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::EntityHitboxes, false));
    }

    #[test]
    fn reduced_debug_info_filters_disallowed_entries() {
        let mut entries = DebugScreenEntryList::default();
        entries.set_overlay_visible(true);

        assert!(entries.is_currently_enabled(DebugScreenEntryId::GameVersion, true));
        assert!(entries.is_currently_enabled(DebugScreenEntryId::PlayerSectionPosition, true));
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::PlayerPosition, true));
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::ThreeDimensionalCrosshair, true));
        entries.set_status(
            DebugScreenEntryId::DayCount,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::DayCount, true));
        entries.set_status(
            DebugScreenEntryId::DetailedMemory,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(entries.is_currently_enabled(DebugScreenEntryId::DetailedMemory, true));
        entries.set_status(
            DebugScreenEntryId::LightLevels,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LightLevels, true));
        entries.set_status(
            DebugScreenEntryId::Heightmap,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::Heightmap, true));
        entries.set_status(DebugScreenEntryId::Biome, DebugScreenEntryStatus::AlwaysOn);
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::Biome, true));
        entries.set_status(
            DebugScreenEntryId::LocalDifficulty,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LocalDifficulty, true));
        entries.set_status(
            DebugScreenEntryId::EntitySpawnCounts,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::EntitySpawnCounts, true));
        entries.set_status(
            DebugScreenEntryId::LookingAtBlockState,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LookingAtBlockState, true));
        entries.set_status(
            DebugScreenEntryId::LookingAtBlockTags,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LookingAtBlockTags, true));
        entries.set_status(
            DebugScreenEntryId::LookingAtFluidState,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LookingAtFluidState, true));
        entries.set_status(
            DebugScreenEntryId::LookingAtFluidTags,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LookingAtFluidTags, true));
        entries.set_status(
            DebugScreenEntryId::LookingAtEntity,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LookingAtEntity, true));
        entries.set_status(
            DebugScreenEntryId::LookingAtEntityTags,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LookingAtEntityTags, true));
        entries.set_status(
            DebugScreenEntryId::ChunkRenderStats,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(entries.is_currently_enabled(DebugScreenEntryId::ChunkRenderStats, true));
        entries.set_status(
            DebugScreenEntryId::ChunkGenerationStats,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::ChunkGenerationStats, true));
        entries.set_status(
            DebugScreenEntryId::EntityRenderStats,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(entries.is_currently_enabled(DebugScreenEntryId::EntityRenderStats, true));
        entries.set_status(
            DebugScreenEntryId::ParticleRenderStats,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::ParticleRenderStats, true));
        entries.set_status(
            DebugScreenEntryId::ChunkSourceStats,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(entries.is_currently_enabled(DebugScreenEntryId::ChunkSourceStats, true));
        entries.set_status(
            DebugScreenEntryId::SoundCache,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(entries.is_currently_enabled(DebugScreenEntryId::SoundCache, true));
        entries.set_status(
            DebugScreenEntryId::SoundMood,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::SoundMood, true));
        entries.set_status(
            DebugScreenEntryId::PostEffect,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::PostEffect, true));
        for entry in VANILLA_RENDERER_NOOP_ENTRIES {
            entries.set_status(entry, DebugScreenEntryStatus::AlwaysOn);
            assert!(!entries.is_currently_enabled(entry, true));
        }
    }

    #[test]
    fn debug_profile_file_roundtrips_vanilla_profile_shape() {
        let path = unique_debug_profile_path("profile");
        let mut entries = DebugScreenEntryList::default();
        entries.load_profile(DebugScreenProfile::Performance);

        entries.save_to_debug_profile_file(&path).unwrap();

        let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .expect("debug profile json parses");
        assert_eq!(value["DataVersion"].as_i64(), Some(MC_DATA_VERSION.into()));
        assert_eq!(value["profile"].as_str(), Some("performance"));
        assert!(value.get("custom").is_none());

        let loaded =
            DebugScreenEntryList::load_from_debug_profile_file(&path, DebugScreenProfile::Default)
                .unwrap();
        assert_eq!(
            loaded.status(DebugScreenEntryId::Fps),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert!(loaded.is_using_profile(DebugScreenProfile::Performance));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn custom_debug_profile_file_preserves_unknown_entries_and_vanilla_fixups() {
        let path = unique_debug_profile_path("custom");
        fs::write(
            &path,
            r#"{
  "DataVersion": 4649,
  "custom": {
    "minecraft:chunk_generation_stats": "alwaysOn",
    "minecraft:local_difficulty": "never",
    "minecraft:entity_spawn_counts": "alwaysOn",
    "minecraft:sound_mood": "alwaysOn",
    "minecraft:post_effect": "alwaysOn",
    "minecraft:entity_hitboxes": "inF3",
    "minecraft:looking_at_block": "never",
    "minecraft:looking_at_fluid": "alwaysOn"
  }
}"#,
        )
        .unwrap();

        let loaded =
            DebugScreenEntryList::load_from_debug_profile_file(&path, DebugScreenProfile::Default)
                .unwrap();
        assert_eq!(
            loaded.status(DebugScreenEntryId::ChunkGenerationStats),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::LocalDifficulty),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::EntitySpawnCounts),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::SoundMood),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::PostEffect),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::InOverlay
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::LookingAtBlockState),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            loaded.status(DebugScreenEntryId::LookingAtFluidState),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert!(!loaded.is_using_profile(DebugScreenProfile::Default));

        loaded.save_to_debug_profile_file(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .expect("debug profile json parses");
        let custom = value["custom"].as_object().unwrap();
        assert_eq!(
            custom["minecraft:chunk_generation_stats"].as_str(),
            Some("alwaysOn")
        );
        assert_eq!(custom["minecraft:local_difficulty"].as_str(), Some("never"));
        assert_eq!(
            custom["minecraft:entity_spawn_counts"].as_str(),
            Some("alwaysOn")
        );
        assert_eq!(custom["minecraft:sound_mood"].as_str(), Some("alwaysOn"));
        assert_eq!(custom["minecraft:post_effect"].as_str(), Some("alwaysOn"));
        assert_eq!(
            custom["minecraft:entity_hitboxes"].as_str(),
            Some("inOverlay")
        );
        assert_eq!(
            custom["minecraft:looking_at_fluid_state"].as_str(),
            Some("alwaysOn")
        );
        assert!(custom.get("minecraft:looking_at_fluid").is_none());
        let _ = fs::remove_file(path);
    }

    fn unique_debug_profile_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "bbb-debug-profile-{name}-{}-{nanos}.json",
            process::id()
        ))
    }

    const VANILLA_RENDERER_NOOP_ENTRIES: [DebugScreenEntryId; 15] = [
        DebugScreenEntryId::EntityHitboxes,
        DebugScreenEntryId::ChunkBorders,
        DebugScreenEntryId::ThreeDimensionalCrosshair,
        DebugScreenEntryId::ChunkSectionPaths,
        DebugScreenEntryId::ChunkSectionOctree,
        DebugScreenEntryId::VisualizeWaterLevels,
        DebugScreenEntryId::VisualizeHeightmap,
        DebugScreenEntryId::VisualizeCollisionBoxes,
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        DebugScreenEntryId::VisualizeBlockLightLevels,
        DebugScreenEntryId::VisualizeSkyLightLevels,
        DebugScreenEntryId::VisualizeSolidFaces,
        DebugScreenEntryId::VisualizeChunksOnServer,
        DebugScreenEntryId::VisualizeSkyLightSections,
        DebugScreenEntryId::ChunkSectionVisibility,
    ];

    const VANILLA_RENDERER_NOOP_NON_PROFILE_ENTRIES: [DebugScreenEntryId; 14] = [
        DebugScreenEntryId::EntityHitboxes,
        DebugScreenEntryId::ChunkBorders,
        DebugScreenEntryId::ChunkSectionPaths,
        DebugScreenEntryId::ChunkSectionOctree,
        DebugScreenEntryId::VisualizeWaterLevels,
        DebugScreenEntryId::VisualizeHeightmap,
        DebugScreenEntryId::VisualizeCollisionBoxes,
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        DebugScreenEntryId::VisualizeBlockLightLevels,
        DebugScreenEntryId::VisualizeSkyLightLevels,
        DebugScreenEntryId::VisualizeSolidFaces,
        DebugScreenEntryId::VisualizeChunksOnServer,
        DebugScreenEntryId::VisualizeSkyLightSections,
        DebugScreenEntryId::ChunkSectionVisibility,
    ];
}
