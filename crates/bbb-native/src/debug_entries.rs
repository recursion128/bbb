use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use bbb_protocol::MC_DATA_VERSION;
use serde::{Deserialize, Serialize};

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
    ThreeDimensionalCrosshair,
    GameVersion,
    Fps,
    Tps,
    Memory,
    DetailedMemory,
    SystemSpecs,
    PlayerPosition,
    PlayerSectionPosition,
    DayCount,
    LightLevels,
    Heightmap,
    Biome,
    LocalDifficulty,
    EntitySpawnCounts,
    LookingAtBlockState,
    LookingAtBlockTags,
    LookingAtFluidState,
    LookingAtFluidTags,
    LookingAtEntity,
    LookingAtEntityTags,
    ChunkRenderStats,
    ChunkGenerationStats,
    EntityRenderStats,
    ParticleRenderStats,
    ChunkSourceStats,
    SoundCache,
    SimplePerformanceImpactors,
    EntityHitboxes,
    ChunkBorders,
    GpuUtilization,
}

impl DebugScreenEntryId {
    fn vanilla_id(self) -> &'static str {
        match self {
            Self::ThreeDimensionalCrosshair => "minecraft:3d_crosshair",
            Self::GameVersion => "minecraft:game_version",
            Self::Fps => "minecraft:fps",
            Self::Tps => "minecraft:tps",
            Self::Memory => "minecraft:memory",
            Self::DetailedMemory => "minecraft:detailed_memory",
            Self::SystemSpecs => "minecraft:system_specs",
            Self::PlayerPosition => "minecraft:player_position",
            Self::PlayerSectionPosition => "minecraft:player_section_position",
            Self::DayCount => "minecraft:day_count",
            Self::LightLevels => "minecraft:light_levels",
            Self::Heightmap => "minecraft:heightmap",
            Self::Biome => "minecraft:biome",
            Self::LocalDifficulty => "minecraft:local_difficulty",
            Self::EntitySpawnCounts => "minecraft:entity_spawn_counts",
            Self::LookingAtBlockState => "minecraft:looking_at_block_state",
            Self::LookingAtBlockTags => "minecraft:looking_at_block_tags",
            Self::LookingAtFluidState => "minecraft:looking_at_fluid_state",
            Self::LookingAtFluidTags => "minecraft:looking_at_fluid_tags",
            Self::LookingAtEntity => "minecraft:looking_at_entity",
            Self::LookingAtEntityTags => "minecraft:looking_at_entity_tags",
            Self::ChunkRenderStats => "minecraft:chunk_render_stats",
            Self::ChunkGenerationStats => "minecraft:chunk_generation_stats",
            Self::EntityRenderStats => "minecraft:entity_render_stats",
            Self::ParticleRenderStats => "minecraft:particle_render_stats",
            Self::ChunkSourceStats => "minecraft:chunk_source_stats",
            Self::SoundCache => "minecraft:sound_cache",
            Self::SimplePerformanceImpactors => "minecraft:simple_performance_impactors",
            Self::EntityHitboxes => "minecraft:entity_hitboxes",
            Self::ChunkBorders => "minecraft:chunk_borders",
            Self::GpuUtilization => "minecraft:gpu_utilization",
        }
    }

    fn from_vanilla_id(id: &str) -> Option<Self> {
        match id {
            "minecraft:3d_crosshair" => Some(Self::ThreeDimensionalCrosshair),
            "minecraft:game_version" => Some(Self::GameVersion),
            "minecraft:fps" => Some(Self::Fps),
            "minecraft:tps" => Some(Self::Tps),
            "minecraft:memory" => Some(Self::Memory),
            "minecraft:detailed_memory" => Some(Self::DetailedMemory),
            "minecraft:system_specs" => Some(Self::SystemSpecs),
            "minecraft:player_position" => Some(Self::PlayerPosition),
            "minecraft:player_section_position" => Some(Self::PlayerSectionPosition),
            "minecraft:day_count" => Some(Self::DayCount),
            "minecraft:light_levels" => Some(Self::LightLevels),
            "minecraft:heightmap" => Some(Self::Heightmap),
            "minecraft:biome" => Some(Self::Biome),
            "minecraft:local_difficulty" => Some(Self::LocalDifficulty),
            "minecraft:entity_spawn_counts" => Some(Self::EntitySpawnCounts),
            "minecraft:looking_at_block" | "minecraft:looking_at_block_state" => {
                Some(Self::LookingAtBlockState)
            }
            "minecraft:looking_at_block_tags" => Some(Self::LookingAtBlockTags),
            "minecraft:looking_at_fluid" | "minecraft:looking_at_fluid_state" => {
                Some(Self::LookingAtFluidState)
            }
            "minecraft:looking_at_fluid_tags" => Some(Self::LookingAtFluidTags),
            "minecraft:looking_at_entity" => Some(Self::LookingAtEntity),
            "minecraft:looking_at_entity_tags" => Some(Self::LookingAtEntityTags),
            "minecraft:chunk_render_stats" => Some(Self::ChunkRenderStats),
            "minecraft:chunk_generation_stats" => Some(Self::ChunkGenerationStats),
            "minecraft:entity_render_stats" => Some(Self::EntityRenderStats),
            "minecraft:particle_render_stats" => Some(Self::ParticleRenderStats),
            "minecraft:chunk_source_stats" => Some(Self::ChunkSourceStats),
            "minecraft:sound_cache" => Some(Self::SoundCache),
            "minecraft:simple_performance_impactors" => Some(Self::SimplePerformanceImpactors),
            "minecraft:entity_hitboxes" => Some(Self::EntityHitboxes),
            "minecraft:chunk_borders" => Some(Self::ChunkBorders),
            "minecraft:gpu_utilization" => Some(Self::GpuUtilization),
            _ => None,
        }
    }

    pub(crate) fn is_allowed(self, reduced_debug_info: bool) -> bool {
        !reduced_debug_info
            || matches!(
                self,
                Self::GameVersion
                    | Self::Fps
                    | Self::Tps
                    | Self::Memory
                    | Self::DetailedMemory
                    | Self::SystemSpecs
                    | Self::PlayerSectionPosition
                    | Self::ChunkRenderStats
                    | Self::EntityRenderStats
                    | Self::ChunkSourceStats
                    | Self::SoundCache
                    | Self::SimplePerformanceImpactors
                    | Self::GpuUtilization
            )
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

    #[cfg(test)]
    fn is_using_profile(&self, profile: DebugScreenProfile) -> bool {
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
        let enabled = match self.status(entry) {
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
        };
        enabled
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
            custom.insert(
                id.vanilla_id().to_string(),
                status.vanilla_name().to_string(),
            );
        }

        SerializedDebugProfileOut {
            data_version: MC_DATA_VERSION,
            profile: None,
            custom: Some(custom),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SerializedDebugProfile {
    #[serde(rename = "DataVersion")]
    _data_version: Option<i32>,
    profile: Option<String>,
    custom: Option<BTreeMap<String, String>>,
}

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
}
