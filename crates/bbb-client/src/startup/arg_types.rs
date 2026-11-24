use bbb_client_net::ServerResourcePackPolicy;
use bbb_client_render::TerrainTextureFiltering;
use bbb_client_world::TutorialStep;
use bbb_protocol::packets::{ClientChatVisibility, ClientMainHand, ClientParticleStatus};
#[cfg(not(target_arch = "wasm32"))]
use clap::ValueEnum;

use crate::{debug_entries::DebugScreenProfile, narrator_runtime::NarratorStatus};

macro_rules! cli_value_enum {
    (
        $(#[doc = $doc:expr])*
        pub(crate) enum $name:ident {
            $($variant:ident),+ $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
        #[cfg_attr(not(target_arch = "wasm32"), derive(ValueEnum))]
        pub(crate) enum $name {
            $($variant,)+
        }
    };
}

cli_value_enum! {
    pub(crate) enum ClientChatVisibilityArg {
    Full,
    System,
    Hidden,
    }
}

impl From<ClientChatVisibilityArg> for ClientChatVisibility {
    fn from(value: ClientChatVisibilityArg) -> Self {
        match value {
            ClientChatVisibilityArg::Full => Self::Full,
            ClientChatVisibilityArg::System => Self::System,
            ClientChatVisibilityArg::Hidden => Self::Hidden,
        }
    }
}

cli_value_enum! {
    pub(crate) enum ClientMainHandArg {
    Left,
    Right,
    }
}

cli_value_enum! {
    pub(crate) enum ClientNarratorArg {
    Off,
    All,
    Chat,
    System,
    }
}

cli_value_enum! {
    pub(crate) enum MusicToastDisplayStateArg {
    Never,
    Pause,
    PauseAndToast,
    }
}

impl MusicToastDisplayStateArg {
    pub(crate) fn renders_pause(self) -> bool {
        matches!(self, Self::Pause | Self::PauseAndToast)
    }

    pub(crate) fn renders_toast(self) -> bool {
        self == Self::PauseAndToast
    }
}

cli_value_enum! {
    pub(crate) enum MusicFrequencyArg {
    Default,
    Frequent,
    Constant,
    }
}

impl MusicFrequencyArg {
    pub(crate) const fn max_frequency_ticks(self) -> i32 {
        match self {
            Self::Default => 20 * 1_200,
            Self::Frequent => 10 * 1_200,
            Self::Constant => 0,
        }
    }
}

impl From<ClientNarratorArg> for NarratorStatus {
    fn from(value: ClientNarratorArg) -> Self {
        match value {
            ClientNarratorArg::Off => Self::Off,
            ClientNarratorArg::All => Self::All,
            ClientNarratorArg::Chat => Self::Chat,
            ClientNarratorArg::System => Self::System,
        }
    }
}

cli_value_enum! {
/// Vanilla `CloudStatus` (`renderClouds`), legacy-encoded as `"true"`
/// (fancy), `"fast"`, and `"false"` (off); default fancy.
    pub(crate) enum RenderCloudsArg {
    Off,
    Fast,
    Fancy,
    }
}

cli_value_enum! {
/// Client-side GPU dynamic point lights tier (`--dynamic-lights`), default `Off`.
    pub(crate) enum DynamicLightsArg {
    Off,
    Fast,
    Fancy,
    }
}

impl From<DynamicLightsArg> for bbb_client_render::DynamicLightsMode {
    fn from(value: DynamicLightsArg) -> Self {
        match value {
            DynamicLightsArg::Off => Self::Off,
            DynamicLightsArg::Fast => Self::Fast,
            DynamicLightsArg::Fancy => Self::Fancy,
        }
    }
}

cli_value_enum! {
/// Vanilla `InactivityFpsLimit` (`inactivityFpsLimit`), default `AFK`.
    pub(crate) enum InactivityFpsLimitArg {
    Minimized,
    Afk,
    }
}

cli_value_enum! {
    pub(crate) enum DebugScreenProfileArg {
    Default,
    Performance,
    }
}

impl From<DebugScreenProfileArg> for DebugScreenProfile {
    fn from(value: DebugScreenProfileArg) -> Self {
        match value {
            DebugScreenProfileArg::Default => Self::Default,
            DebugScreenProfileArg::Performance => Self::Performance,
        }
    }
}

impl From<ClientMainHandArg> for ClientMainHand {
    fn from(value: ClientMainHandArg) -> Self {
        match value {
            ClientMainHandArg::Left => Self::Left,
            ClientMainHandArg::Right => Self::Right,
        }
    }
}

cli_value_enum! {
    pub(crate) enum ClientParticleStatusArg {
    All,
    Decreased,
    Minimal,
    }
}

cli_value_enum! {
/// Vanilla 26.1 `GraphicsPreset`; `CUSTOM` deliberately performs no writes.
    pub(crate) enum GraphicsPresetArg {
    Fast,
    Fancy,
    Fabulous,
    Custom,
    }
}

cli_value_enum! {
/// Vanilla 26.1 `PrioritizeChunkUpdates`, persisted through its wrapped
/// integer id codec.
    pub(crate) enum PrioritizeChunkUpdatesArg {
    None,
    PlayerAffected,
    Nearby,
    }
}

cli_value_enum! {
/// Vanilla `AttackIndicatorStatus`, stored
/// in `options.txt` by its `id`.
    pub(crate) enum ClientAttackIndicatorArg {
    Off,
    Crosshair,
    Hotbar,
    }
}

cli_value_enum! {
    pub(crate) enum TerrainTextureFilteringArg {
    None,
    Rgss,
    Anisotropic,
    }
}

impl From<TerrainTextureFilteringArg> for TerrainTextureFiltering {
    fn from(value: TerrainTextureFilteringArg) -> Self {
        match value {
            TerrainTextureFilteringArg::None => Self::None,
            TerrainTextureFilteringArg::Rgss => Self::Rgss,
            TerrainTextureFilteringArg::Anisotropic => Self::Anisotropic,
        }
    }
}

/// Vanilla `Stitcher` pads sprites by `1 << mipLevel << clamp(anisotropyBit -
/// 1, 0, 4)` and `SpriteLoader` passes bit 0 unless `textureFiltering ==
/// ANISOTROPIC`; the atlas packer multiplies its base padding by `1 <<
/// mipLevel`, so the base is the anisotropy factor alone.
#[cfg(test)]
pub(super) fn terrain_atlas_base_padding(
    filtering: TerrainTextureFilteringArg,
    max_anisotropy_bit: u32,
) -> u32 {
    terrain_atlas_base_padding_for_filtering(filtering.into(), max_anisotropy_bit)
}

pub(crate) fn terrain_atlas_base_padding_for_filtering(
    filtering: TerrainTextureFiltering,
    max_anisotropy_bit: u32,
) -> u32 {
    if filtering != TerrainTextureFiltering::Anisotropic {
        return 1;
    }
    1 << max_anisotropy_bit.saturating_sub(1).min(4)
}

cli_value_enum! {
    pub(crate) enum TutorialStepArg {
    Movement,
    FindTree,
    PunchTree,
    OpenInventory,
    CraftPlanks,
    None,
    }
}

impl From<TutorialStepArg> for TutorialStep {
    fn from(value: TutorialStepArg) -> Self {
        match value {
            TutorialStepArg::Movement => Self::Movement,
            TutorialStepArg::FindTree => Self::FindTree,
            TutorialStepArg::PunchTree => Self::PunchTree,
            TutorialStepArg::OpenInventory => Self::OpenInventory,
            TutorialStepArg::CraftPlanks => Self::CraftPlanks,
            TutorialStepArg::None => Self::None,
        }
    }
}

cli_value_enum! {
    pub(crate) enum ServerResourcePackPolicyArg {
    Prompt,
    Accept,
    Decline,
    }
}

impl From<ServerResourcePackPolicyArg> for ServerResourcePackPolicy {
    fn from(value: ServerResourcePackPolicyArg) -> Self {
        match value {
            ServerResourcePackPolicyArg::Prompt => Self::Prompt,
            ServerResourcePackPolicyArg::Accept => Self::Accept,
            ServerResourcePackPolicyArg::Decline => Self::Decline,
        }
    }
}

impl From<ClientAttackIndicatorArg> for crate::simulation::AttackIndicatorOption {
    fn from(value: ClientAttackIndicatorArg) -> Self {
        match value {
            ClientAttackIndicatorArg::Off => Self::Off,
            ClientAttackIndicatorArg::Crosshair => Self::Crosshair,
            ClientAttackIndicatorArg::Hotbar => Self::Hotbar,
        }
    }
}

impl From<ClientParticleStatusArg> for ClientParticleStatus {
    fn from(value: ClientParticleStatusArg) -> Self {
        match value {
            ClientParticleStatusArg::All => Self::All,
            ClientParticleStatusArg::Decreased => Self::Decreased,
            ClientParticleStatusArg::Minimal => Self::Minimal,
        }
    }
}
