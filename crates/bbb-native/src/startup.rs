use std::{
    convert::Infallible, fmt, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, thread,
    time::Duration,
};

use anyhow::{ensure, Context, Result};
use bbb_control::ControlState;
use bbb_net::{ConnectionOptions, MinecraftUserApiClient, ServerResourcePackPolicy};
use bbb_pack::{KnownPacksManager, PackRoots};
use bbb_protocol::packets::{
    ClientChatVisibility, ClientInformation, ClientMainHand, ClientParticleStatus,
};
use bbb_renderer::{
    TerrainTextureFiltering, VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
    VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR, VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
    VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS, VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
    VANILLA_MAX_CLOUD_RANGE_CHUNKS, VANILLA_MAX_MENU_BACKGROUND_BLURRINESS,
    VANILLA_MAX_RENDER_DISTANCE_CHUNKS, VANILLA_MIN_CLOUD_RANGE_CHUNKS,
    VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
use bbb_world::{PlayerReportEnvironment, TutorialStep};
use clap::{ArgAction, ArgMatches, CommandFactory, FromArgMatches, Parser, ValueEnum};

use crate::client_options::{
    args::{parse_stored_boolean, stored_fov_degrees, ClientOptionBinding, StoredOptions},
    key_bindings::KeyBindings,
    ClientOptionEncoding, ClientOptionHost, ClientOptions,
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::camera_fov::{
    VANILLA_DEFAULT_CAMERA_FOV_DEGREES, VANILLA_DEFAULT_FOV_EFFECT_SCALE,
    VANILLA_MAX_CAMERA_FOV_DEGREES, VANILLA_MIN_CAMERA_FOV_DEGREES,
};
use crate::window_surface;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{
    code_of_conduct::CodeOfConductAcceptance, debug_entries::DebugScreenProfile,
    narrator_runtime::NarratorStatus, network_runtime::NetworkRuntime,
    remote_debug::RemoteDebugRenderer, simulation::commands::SimulationMailbox,
};

pub(crate) const VANILLA_UNLIMITED_FRAMERATE_LIMIT: u32 = 260;

#[derive(Debug, Parser)]
#[command(name = "bbb-native")]
pub(crate) struct Args {
    /// Vanilla `--gameDir`: the directory `options.txt` is read from and
    /// written back to.
    #[arg(long = "game-dir", value_name = "PATH", default_value = ".")]
    pub(crate) game_dir: PathBuf,
    #[arg(long, default_value = "127.0.0.1:25565")]
    pub(crate) server: String,
    #[arg(long, default_value = "bbb-client")]
    pub(crate) username: String,
    #[arg(long, value_name = "UUID", requires = "access_token")]
    pub(crate) profile_id: Option<Uuid>,
    #[arg(long, value_name = "TOKEN", requires = "profile_id")]
    pub(crate) access_token: Option<StartupAccessToken>,
    #[arg(long, value_name = "ADDR", conflicts_with = "probe_server")]
    pub(crate) status_server: Option<String>,
    #[arg(long)]
    pub(crate) build_info: bool,
    #[arg(long)]
    pub(crate) probe_server: bool,
    #[arg(long, default_value_t = 0)]
    pub(crate) probe_after_first_chunk_packets: usize,
    #[arg(long)]
    pub(crate) connect_server: bool,
    #[arg(long)]
    pub(crate) control: Option<SocketAddr>,
    #[arg(long)]
    pub(crate) screenshot: Option<PathBuf>,
    #[arg(long)]
    pub(crate) exit_after_screenshot: bool,
    #[arg(long, value_name = "PATH")]
    pub(crate) crash_report_dir: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    pub(crate) code_of_conduct_store: Option<PathBuf>,
    #[arg(
        long,
        value_name = "PATH",
        requires_all = ["server_list_name", "connect_server"]
    )]
    pub(crate) server_list_store: Option<PathBuf>,
    #[arg(
        long,
        value_name = "NAME",
        requires_all = ["server_list_store", "connect_server"]
    )]
    pub(crate) server_list_name: Option<String>,
    #[arg(long = "resource-pack-dir", value_name = "PATH")]
    pub(crate) resource_pack_dirs: Vec<PathBuf>,
    #[arg(
        long = "server-resource-pack-policy",
        value_enum,
        default_value = "decline"
    )]
    pub(crate) server_resource_pack_policy: ServerResourcePackPolicyArg,
    #[arg(long = "server-resource-pack-cache-dir", value_name = "PATH")]
    pub(crate) server_resource_pack_cache_dir: Option<PathBuf>,
    #[arg(long = "player-skin-cache-dir", value_name = "PATH")]
    pub(crate) player_skin_cache_dir: Option<PathBuf>,
    #[arg(long = "client-locale", default_value = "en_us")]
    pub(crate) client_locale: String,
    #[arg(long = "client-narrator", value_enum, default_value = "off")]
    pub(crate) client_narrator: ClientNarratorArg,
    #[arg(
        long = "client-narrator-hotkey",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_narrator_hotkey: bool,
    #[arg(long = "client-fullscreen", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_fullscreen: bool,
    #[arg(
        long = "client-fullscreen-resolution",
        value_parser = parse_client_fullscreen_resolution
    )]
    pub(crate) client_fullscreen_resolution: Option<window_surface::PreferredVideoMode>,
    #[arg(
        long = "client-gui-scale",
        default_value_t = 0,
        value_parser = parse_client_gui_scale
    )]
    pub(crate) client_gui_scale: u32,
    #[arg(
        long = "client-force-unicode-font",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_force_unicode_font: bool,
    #[arg(
        long = "client-raw-mouse-input",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_raw_mouse_input: bool,
    #[arg(
        long = "client-allow-cursor-changes",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_allow_cursor_changes: bool,
    #[arg(long = "client-mouse-sensitivity", default_value_t = 0.5)]
    pub(crate) client_mouse_sensitivity: f64,
    #[arg(long = "client-invert-mouse-x", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_invert_mouse_x: bool,
    #[arg(long = "client-invert-mouse-y", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_invert_mouse_y: bool,
    #[arg(long = "client-discrete-mouse-scroll", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_discrete_mouse_scroll: bool,
    #[arg(long = "client-mouse-wheel-sensitivity", default_value_t = 1.0)]
    pub(crate) client_mouse_wheel_sensitivity: f64,
    #[arg(long = "client-toggle-crouch", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_toggle_crouch: bool,
    #[arg(long = "client-toggle-sprint", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_toggle_sprint: bool,
    #[arg(long = "client-toggle-attack", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_toggle_attack: bool,
    #[arg(long = "client-toggle-use", default_value_t = false, action = ArgAction::Set)]
    pub(crate) client_toggle_use: bool,
    /// Resolved from every `key_<name>` line at once, so it has no flag of its
    /// own the way the single-valued options above do.
    #[arg(skip)]
    pub(crate) client_key_bindings: KeyBindings,
    #[arg(
        long = "client-show-subtitles",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_show_subtitles: bool,
    #[arg(long = "high-contrast-block-outline")]
    pub(crate) high_contrast_block_outline: bool,
    #[arg(
        long = "notification-display-time",
        default_value_t = 1.0,
        value_parser = parse_notification_display_time
    )]
    pub(crate) notification_display_time: f32,
    #[arg(long = "narrator-command", value_name = "PATH")]
    pub(crate) narrator_command: Option<PathBuf>,
    #[arg(long = "narrator-command-arg", value_name = "ARG")]
    pub(crate) narrator_command_args: Vec<String>,
    /// `Options.getFinalSoundSourceVolume(SoundSource.VOICE)`: voice times
    /// master, resolved from the stored `soundCategory_*` lines.
    #[arg(skip = 1.0_f32)]
    pub(crate) narrator_voice_volume: f32,
    #[arg(long = "client-chat-visibility", value_enum, default_value = "full")]
    pub(crate) client_chat_visibility: ClientChatVisibilityArg,
    #[arg(
        long = "client-save-chat-drafts",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_save_chat_drafts: bool,
    #[arg(
        long = "client-only-show-secure-chat",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_only_show_secure_chat: bool,
    #[arg(
        long = "client-hide-matched-names",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_hide_matched_names: bool,
    #[arg(
        long = "client-chat-delay-seconds",
        default_value_t = 0.0,
        value_parser = parse_client_chat_delay
    )]
    pub(crate) client_chat_delay_seconds: f64,
    #[arg(long = "client-blocked-player", value_name = "UUID")]
    pub(crate) client_blocked_players: Vec<Uuid>,
    #[arg(
        long = "client-chat-colors",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_colors: bool,
    #[arg(
        long = "client-chat-links",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_links: bool,
    #[arg(
        long = "client-chat-links-prompt",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_links_prompt: bool,
    #[arg(long = "client-skin-parts", default_value_t = 0x7f)]
    pub(crate) client_skin_parts: u8,
    #[arg(long = "client-main-hand", value_enum, default_value = "right")]
    pub(crate) client_main_hand: ClientMainHandArg,
    #[arg(
        long = "client-text-filtering",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_text_filtering: bool,
    #[arg(
        long = "client-allow-server-listing",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_allow_server_listing: bool,
    #[arg(long = "client-particles", value_enum, default_value = "all")]
    pub(crate) client_particles: ClientParticleStatusArg,
    #[arg(
        long = "client-auto-jump",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_auto_jump: bool,
    #[arg(
        long = "client-reduced-debug-info",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_reduced_debug_info: bool,
    #[arg(long = "client-vignette", default_value_t = true, action = ArgAction::Set)]
    pub(crate) client_vignette: bool,
    #[arg(
        long = "client-attack-indicator",
        value_enum,
        default_value = "crosshair"
    )]
    pub(crate) client_attack_indicator: ClientAttackIndicatorArg,
    #[arg(long = "client-screen-effect-scale", default_value_t = 1.0)]
    pub(crate) client_screen_effect_scale: f32,
    #[arg(long = "tutorial-step", value_enum, default_value = "movement")]
    pub(crate) tutorial_step: TutorialStepArg,
    #[arg(
        long = "render-distance",
        default_value_t = VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
        value_parser = parse_render_distance
    )]
    pub(crate) render_distance_chunks: u32,
    #[arg(
        long = "cloud-range",
        default_value_t = VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
        value_parser = parse_cloud_range
    )]
    pub(crate) cloud_range_chunks: u32,
    #[arg(
        long = "render-clouds",
        default_value = "fancy",
        value_parser = parse_stored_render_clouds
    )]
    pub(crate) render_clouds: RenderCloudsArg,
    #[arg(
        long = "client-framerate-limit",
        default_value_t = VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        value_parser = parse_client_framerate_limit
    )]
    pub(crate) client_framerate_limit: u32,
    #[arg(
        long = "inactivity-fps-limit",
        default_value = "afk",
        value_parser = parse_stored_inactivity_fps_limit
    )]
    pub(crate) inactivity_fps_limit: InactivityFpsLimitArg,
    #[arg(long = "client-vsync", default_value_t = true, action = ArgAction::Set)]
    pub(crate) client_vsync: bool,
    #[arg(long = "terrain-texture-filtering", value_enum, default_value = "none")]
    pub(crate) terrain_texture_filtering: TerrainTextureFilteringArg,
    #[arg(
        long = "text-background-opacity",
        default_value_t = VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
        value_parser = parse_text_background_opacity
    )]
    pub(crate) text_background_opacity: f32,
    #[arg(
        long = "background-for-chat-only",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) background_for_chat_only: bool,
    #[arg(
        long = "cutout-leaves",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) cutout_leaves: bool,
    #[arg(
        long = "client-gamma",
        default_value_t = VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
        value_parser = parse_client_gamma
    )]
    pub(crate) client_gamma: f32,
    #[arg(
        long = "client-fov",
        default_value_t = VANILLA_DEFAULT_CAMERA_FOV_DEGREES as u16,
        value_parser = parse_client_fov
    )]
    pub(crate) client_fov: u16,
    #[arg(
        long = "client-fov-effect-scale",
        default_value_t = VANILLA_DEFAULT_FOV_EFFECT_SCALE,
        value_parser = parse_client_fov_effect_scale
    )]
    pub(crate) client_fov_effect_scale: f32,
    #[arg(
        long = "client-glint-speed",
        default_value_t = 0.5,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_glint_speed: f64,
    #[arg(
        long = "client-glint-strength",
        default_value_t = 0.75,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_glint_strength: f64,
    #[arg(
        long = "panorama-scroll-speed",
        default_value_t = 1.0,
        value_parser = parse_client_unit_double
    )]
    pub(crate) panorama_scroll_speed: f64,
    #[arg(
        long = "client-smooth-lighting",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_smooth_lighting: bool,
    #[arg(
        long = "client-view-bobbing",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_view_bobbing: bool,
    #[arg(long = "improved-transparency")]
    pub(crate) improved_transparency: bool,
    #[arg(
        long = "menu-background-blurriness",
        default_value_t = VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
        value_parser = parse_menu_background_blurriness
    )]
    pub(crate) menu_background_blurriness: u8,
    #[arg(
        long = "damage-tilt-strength",
        default_value_t = 1.0,
        value_parser = parse_damage_tilt_strength
    )]
    pub(crate) damage_tilt_strength: f32,
    #[arg(long = "entity-shadows", default_value_t = true, action = ArgAction::Set)]
    pub(crate) entity_shadows: bool,
    #[arg(long = "hide-lightning-flash")]
    pub(crate) hide_lightning_flash: bool,
    #[arg(long = "advanced-item-tooltips")]
    pub(crate) advanced_item_tooltips: bool,
    #[arg(long = "pause-on-lost-focus", default_value_t = true, action = ArgAction::Set)]
    pub(crate) pause_on_lost_focus: bool,
    #[arg(long = "debug-profile", value_enum, default_value = "default")]
    pub(crate) debug_profile: DebugScreenProfileArg,
    #[arg(long = "debug-profile-store", value_name = "PATH")]
    pub(crate) debug_profile_store: Option<PathBuf>,
    #[arg(long = "debug-show-local-server-entity-hit-boxes")]
    pub(crate) debug_show_local_server_entity_hit_boxes: bool,
    #[arg(long = "debug-hotkeys")]
    pub(crate) debug_hotkeys: bool,
    #[arg(long = "debug-feature-count")]
    pub(crate) debug_feature_count: bool,
    #[arg(
        long = "remote-debug-renderer",
        value_enum,
        value_name = "RENDERER",
        action = ArgAction::Append
    )]
    pub(crate) remote_debug_renderers: Vec<RemoteDebugRenderer>,
}

#[derive(Clone)]
pub(crate) struct StartupAccessToken(String);

impl fmt::Debug for StartupAccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl FromStr for StartupAccessToken {
    type Err = Infallible;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(value.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ClientChatVisibilityArg {
    Full,
    System,
    Hidden,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ClientMainHandArg {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ClientNarratorArg {
    Off,
    All,
    Chat,
    System,
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

/// Vanilla `CloudStatus` (`renderClouds`), legacy-encoded as `"true"`
/// (fancy), `"fast"`, and `"false"` (off); default fancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RenderCloudsArg {
    Off,
    Fast,
    Fancy,
}

/// Vanilla `InactivityFpsLimit` (`inactivityFpsLimit`), default `AFK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum InactivityFpsLimitArg {
    Minimized,
    Afk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum DebugScreenProfileArg {
    Default,
    Performance,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ClientParticleStatusArg {
    All,
    Decreased,
    Minimal,
}

/// Vanilla `AttackIndicatorStatus` (`AttackIndicatorStatus.java:8-11`), stored
/// in `options.txt` by its `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ClientAttackIndicatorArg {
    Off,
    Crosshair,
    Hotbar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum TerrainTextureFilteringArg {
    None,
    Rgss,
}

impl From<TerrainTextureFilteringArg> for TerrainTextureFiltering {
    fn from(value: TerrainTextureFilteringArg) -> Self {
        match value {
            TerrainTextureFilteringArg::None => Self::None,
            TerrainTextureFilteringArg::Rgss => Self::Rgss,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum TutorialStepArg {
    Movement,
    FindTree,
    PunchTree,
    OpenInventory,
    CraftPlanks,
    None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ServerResourcePackPolicyArg {
    Accept,
    Decline,
}

impl From<ServerResourcePackPolicyArg> for ServerResourcePackPolicy {
    fn from(value: ServerResourcePackPolicyArg) -> Self {
        match value {
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

pub(crate) fn init_tracing() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

pub(crate) fn parse_args() -> Args {
    let matches = Args::command().get_matches();
    let mut args = match Args::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(error) => error.exit(),
    };
    apply_stored_client_options(&mut args, &matches);
    args
}

/// Vanilla reads these values from `options.txt` alone, so the file is the
/// base and only an argument the user actually typed overrides it. A missing
/// or unreadable file leaves every clap default in place.
fn apply_stored_client_options(args: &mut Args, matches: &ArgMatches) {
    let path = args.game_dir.join("options.txt");
    let host = ClientOptionHost {
        locale_is_japanese: false,
        platform_is_windows: cfg!(windows),
    };
    let options = match ClientOptions::load(&path, host) {
        Ok(options) => options,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "failed to load options.txt");
            return;
        }
    };
    args.client_key_bindings = KeyBindings::from_options(&options);
    crate::client_options::key_bindings::install_bound_key_display_keys(&options);
    args.narrator_voice_volume = narrator_voice_volume(&options);
    bind_stored_client_options(args, StoredOptions::new(&options, matches));
}

/// `GameNarrator.narrateMessage` speaks at
/// `getFinalSoundSourceVolume(SoundSource.VOICE)` — the stored voice volume
/// times the stored master volume, each defaulting to 1.0 and clamped to the
/// accepted 0..=1 range; an unparsable line keeps its default.
fn narrator_voice_volume(options: &ClientOptions) -> f32 {
    let source_volume = |key: &str| {
        options
            .dynamic_raw(key)
            .and_then(|raw| raw.parse::<f32>().ok())
            .filter(|volume| volume.is_finite())
            .map(|volume| volume.clamp(0.0, 1.0))
            .unwrap_or(1.0)
    };
    source_volume("soundCategory_voice") * source_volume("soundCategory_master")
}

/// Every startup argument that mirrors an `options.txt` key. The stored line is
/// validated by the same parser the command line uses, so a bad line keeps the
/// current value instead of injecting an out-of-range one.
fn bind_stored_client_options(
    args: &mut Args,
    mut stored: StoredOptions<'_>,
) -> Vec<ClientOptionBinding> {
    macro_rules! flag {
        ($id:literal, $key:literal, $field:ident) => {
            stored.bind(
                $id,
                $key,
                ClientOptionEncoding::Boolean,
                &mut args.$field,
                parse_stored_boolean,
            )
        };
    }
    macro_rules! parsed {
        ($id:literal, $key:literal, $encoding:ident, $field:ident, $parse:expr) => {
            stored.bind(
                $id,
                $key,
                ClientOptionEncoding::$encoding,
                &mut args.$field,
                $parse,
            )
        };
    }

    flag!("client_auto_jump", "autoJump", client_auto_jump);
    flag!("client_smooth_lighting", "ao", client_smooth_lighting);
    flag!(
        "client_reduced_debug_info",
        "reducedDebugInfo",
        client_reduced_debug_info
    );
    flag!("client_vignette", "vignette", client_vignette);
    parsed!(
        "client_narrator",
        "narrator",
        EnumId,
        client_narrator,
        parse_stored_narrator
    );
    flag!(
        "client_narrator_hotkey",
        "narratorHotkey",
        client_narrator_hotkey
    );
    parsed!(
        "client_attack_indicator",
        "attackIndicator",
        EnumId,
        client_attack_indicator,
        parse_stored_attack_indicator
    );
    parsed!(
        "client_screen_effect_scale",
        "screenEffectScale",
        Double,
        client_screen_effect_scale,
        parse_client_screen_effect_scale
    );
    flag!("client_chat_colors", "chatColors", client_chat_colors);
    flag!("client_chat_links", "chatLinks", client_chat_links);
    flag!(
        "client_chat_links_prompt",
        "chatLinksPrompt",
        client_chat_links_prompt
    );
    flag!(
        "client_show_subtitles",
        "showSubtitles",
        client_show_subtitles
    );
    flag!(
        "client_save_chat_drafts",
        "saveChatDrafts",
        client_save_chat_drafts
    );
    flag!(
        "client_only_show_secure_chat",
        "onlyShowSecureChat",
        client_only_show_secure_chat
    );
    flag!(
        "client_hide_matched_names",
        "hideMatchedNames",
        client_hide_matched_names
    );
    flag!(
        "client_allow_server_listing",
        "allowServerListing",
        client_allow_server_listing
    );
    flag!(
        "high_contrast_block_outline",
        "highContrastBlockOutline",
        high_contrast_block_outline
    );
    flag!("client_view_bobbing", "bobView", client_view_bobbing);
    flag!(
        "background_for_chat_only",
        "backgroundForChatOnly",
        background_for_chat_only
    );
    flag!("cutout_leaves", "cutoutLeaves", cutout_leaves);
    flag!("entity_shadows", "entityShadows", entity_shadows);
    flag!(
        "hide_lightning_flash",
        "hideLightningFlashes",
        hide_lightning_flash
    );
    flag!(
        "advanced_item_tooltips",
        "advancedItemTooltips",
        advanced_item_tooltips
    );
    flag!(
        "pause_on_lost_focus",
        "pauseOnLostFocus",
        pause_on_lost_focus
    );
    flag!(
        "improved_transparency",
        "improvedTransparency",
        improved_transparency
    );
    flag!("client_vsync", "enableVsync", client_vsync);
    flag!("client_fullscreen", "fullscreen", client_fullscreen);
    stored.bind_fullscreen_resolution(
        "client_fullscreen_resolution",
        &mut args.client_fullscreen_resolution,
        parse_client_fullscreen_resolution,
    );
    parsed!(
        "client_gui_scale",
        "guiScale",
        Integer,
        client_gui_scale,
        parse_client_gui_scale
    );
    flag!(
        "client_force_unicode_font",
        "forceUnicodeFont",
        client_force_unicode_font
    );
    flag!(
        "client_raw_mouse_input",
        "rawMouseInput",
        client_raw_mouse_input
    );
    flag!(
        "client_allow_cursor_changes",
        "allowCursorChanges",
        client_allow_cursor_changes
    );
    parsed!(
        "client_mouse_sensitivity",
        "mouseSensitivity",
        Double,
        client_mouse_sensitivity,
        parse_client_unit_double
    );
    flag!(
        "client_invert_mouse_x",
        "invertXMouse",
        client_invert_mouse_x
    );
    flag!(
        "client_invert_mouse_y",
        "invertYMouse",
        client_invert_mouse_y
    );
    flag!(
        "client_discrete_mouse_scroll",
        "discrete_mouse_scroll",
        client_discrete_mouse_scroll
    );
    flag!("client_toggle_crouch", "toggleCrouch", client_toggle_crouch);
    flag!("client_toggle_sprint", "toggleSprint", client_toggle_sprint);
    flag!("client_toggle_attack", "toggleAttack", client_toggle_attack);
    flag!("client_toggle_use", "toggleUse", client_toggle_use);
    parsed!(
        "client_mouse_wheel_sensitivity",
        "mouseWheelSensitivity",
        Double,
        client_mouse_wheel_sensitivity,
        parse_client_mouse_wheel_sensitivity
    );

    parsed!(
        "render_distance_chunks",
        "renderDistance",
        Integer,
        render_distance_chunks,
        parse_render_distance
    );
    parsed!(
        "cloud_range_chunks",
        "cloudRange",
        Integer,
        cloud_range_chunks,
        parse_cloud_range
    );
    parsed!(
        "render_clouds",
        "renderClouds",
        EnumNameOrBoolean,
        render_clouds,
        parse_stored_render_clouds
    );
    parsed!(
        "client_framerate_limit",
        "maxFps",
        Integer,
        client_framerate_limit,
        parse_client_framerate_limit
    );
    parsed!(
        "inactivity_fps_limit",
        "inactivityFpsLimit",
        EnumName,
        inactivity_fps_limit,
        parse_stored_inactivity_fps_limit
    );
    parsed!(
        "menu_background_blurriness",
        "menuBackgroundBlurriness",
        Integer,
        menu_background_blurriness,
        parse_menu_background_blurriness
    );
    parsed!(
        "client_chat_delay_seconds",
        "chatDelay",
        Double,
        client_chat_delay_seconds,
        parse_client_chat_delay
    );
    parsed!(
        "text_background_opacity",
        "textBackgroundOpacity",
        Double,
        text_background_opacity,
        parse_text_background_opacity
    );
    parsed!(
        "client_gamma",
        "gamma",
        Double,
        client_gamma,
        parse_client_gamma
    );
    parsed!(
        "client_fov_effect_scale",
        "fovEffectScale",
        Double,
        client_fov_effect_scale,
        parse_client_fov_effect_scale
    );
    parsed!(
        "client_glint_speed",
        "glintSpeed",
        Double,
        client_glint_speed,
        parse_client_unit_double
    );
    parsed!(
        "client_glint_strength",
        "glintStrength",
        Double,
        client_glint_strength,
        parse_client_unit_double
    );
    parsed!(
        "panorama_scroll_speed",
        "panoramaScrollSpeed",
        Double,
        panorama_scroll_speed,
        parse_client_unit_double
    );
    parsed!(
        "damage_tilt_strength",
        "damageTiltStrength",
        Double,
        damage_tilt_strength,
        parse_damage_tilt_strength
    );
    parsed!(
        "notification_display_time",
        "notificationDisplayTime",
        Double,
        notification_display_time,
        parse_notification_display_time
    );
    parsed!("client_fov", "fov", Double, client_fov, stored_fov_degrees);
    parsed!(
        "client_locale",
        "lang",
        Text,
        client_locale,
        |raw: &str| Ok(raw.to_owned())
    );

    stored.bound()
}

#[derive(Debug, Serialize)]
pub(crate) struct BuildInfo {
    schema: u32,
    client: &'static str,
    client_version: &'static str,
    minecraft_version: &'static str,
    protocol_version: i32,
    resource_pack_format: String,
    data_pack_format: String,
    target_os: &'static str,
    target_arch: &'static str,
}

impl BuildInfo {
    fn current() -> Self {
        Self {
            schema: 1,
            client: "bbb-native",
            client_version: env!("CARGO_PKG_VERSION"),
            minecraft_version: bbb_protocol::MC_VERSION,
            protocol_version: bbb_protocol::PROTOCOL_VERSION,
            resource_pack_format: bbb_protocol::MC_RESOURCE_PACK_FORMAT.to_vanilla_string(),
            data_pack_format: bbb_protocol::MC_DATA_PACK_FORMAT.to_vanilla_string(),
            target_os: std::env::consts::OS,
            target_arch: std::env::consts::ARCH,
        }
    }
}

pub(crate) fn run_build_info_if_requested(args: &Args) -> Result<bool> {
    if !args.build_info {
        return Ok(false);
    }
    println!("{}", serde_json::to_string_pretty(&BuildInfo::current())?);
    Ok(true)
}

pub(crate) fn run_status_if_requested(runtime: &Runtime, args: &Args) -> Result<bool> {
    let Some(address) = args.status_server.as_deref() else {
        return Ok(false);
    };
    let status = runtime.block_on(bbb_net::status_ping(address))?;
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(true)
}

pub(crate) fn run_probe_if_requested(runtime: &Runtime, args: &Args) -> Result<bool> {
    if !args.probe_server {
        return Ok(false);
    }

    let mut options = connection_options_from_args(args)?;
    let roots = load_pack_roots(args)?;
    options.known_packs_manager = load_known_packs_manager(roots.as_ref());
    options.client_information = client_information_from_args(args)?;
    options.probe_after_first_chunk_packets = args.probe_after_first_chunk_packets;
    let report = runtime.block_on(bbb_net::run_probe(options))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(true)
}

pub(crate) fn load_pack_roots(args: &Args) -> Result<Option<PackRoots>> {
    match PackRoots::discover() {
        Ok(roots) => Ok(Some(apply_resource_pack_dirs(
            roots,
            args.resource_pack_dirs.iter().cloned(),
        )?)),
        Err(err) => {
            tracing::warn!(?err, "vanilla 26.1 pack roots unavailable");
            Ok(None)
        }
    }
}

pub(crate) fn load_known_packs_manager(
    roots: Option<&PackRoots>,
) -> Option<Arc<KnownPacksManager>> {
    let roots = roots?;
    match KnownPacksManager::load(roots) {
        Ok(manager) => Some(Arc::new(manager)),
        Err(err) => {
            tracing::warn!(?err, "local 26.1 known packs unavailable");
            None
        }
    }
}

fn apply_resource_pack_dirs(
    roots: PackRoots,
    dirs: impl IntoIterator<Item = PathBuf>,
) -> Result<PackRoots> {
    let selected = bbb_pack::SelectedClientPackStack::from_startup_dirs(&roots, dirs)?;
    roots.with_selected_client_packs(selected)
}

pub(crate) fn start_network_if_requested(
    runtime: &Runtime,
    args: &Args,
    code_of_conduct: &mut CodeOfConductAcceptance,
    known_packs_manager: Option<Arc<KnownPacksManager>>,
    simulation_mailbox: &SimulationMailbox,
) -> Result<NetworkRuntime> {
    if !args.connect_server {
        return Ok(NetworkRuntime::idle());
    }

    let mut options = connection_options_from_args(args)?;
    options.client_information = client_information_from_args(args)?;
    options.accepted_code_of_conduct_hash = code_of_conduct.accepted_hash_for_options(&options);
    options.known_packs_manager = known_packs_manager;
    code_of_conduct.set_connected_server(&options);
    Ok(NetworkRuntime::start(
        runtime,
        options,
        simulation_mailbox.clone(),
    ))
}

fn connection_options_from_args(args: &Args) -> Result<ConnectionOptions> {
    ensure!(
        args.profile_id.is_some() == args.access_token.is_some(),
        "profile id and access token must be supplied together"
    );
    let mut options = match (args.profile_id, args.access_token.as_ref()) {
        (Some(profile_id), Some(access_token)) => ConnectionOptions::authenticated(
            &args.server,
            &args.username,
            profile_id,
            access_token.0.clone(),
        ),
        (None, None) => ConnectionOptions::offline(&args.server, &args.username),
        _ => unreachable!("paired session credential presence was checked"),
    }?;
    options.server_resource_pack_policy = args.server_resource_pack_policy.into();
    Ok(options)
}

pub(crate) fn services_block_list_client_from_args(
    args: &Args,
) -> Result<Option<MinecraftUserApiClient>> {
    ensure!(
        args.profile_id.is_some() == args.access_token.is_some(),
        "profile id and access token must be supplied together"
    );
    args.access_token
        .as_ref()
        .map(|access_token| {
            MinecraftUserApiClient::authenticated(access_token.0.clone(), Duration::from_secs(5))
                .map_err(Into::into)
        })
        .transpose()
}

pub(crate) fn report_locale_from_client_locale(locale: &str) -> String {
    locale
        .split(['-', '_'])
        .enumerate()
        .map(|(index, part)| {
            if index == 0 {
                part.to_ascii_lowercase()
            } else if part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                part.to_ascii_uppercase()
            } else if part.len() == 4 && part.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                let mut chars = part.chars();
                chars
                    .next()
                    .map(|first| {
                        format!(
                            "{}{}",
                            first.to_ascii_uppercase(),
                            chars.as_str().to_ascii_lowercase()
                        )
                    })
                    .unwrap_or_default()
            } else {
                part.to_ascii_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn player_report_environment_from_args(args: &Args) -> Option<PlayerReportEnvironment> {
    args.profile_id.map(|_| {
        PlayerReportEnvironment::third_party(
            format!("{} (modded)", bbb_protocol::MC_VERSION),
            report_locale_from_client_locale(&args.client_locale),
            args.server.clone(),
        )
    })
}

pub(crate) fn client_information_from_args(args: &Args) -> Result<ClientInformation> {
    ensure!(
        args.client_locale.len() <= 16,
        "client locale must be at most 16 UTF-8 bytes"
    );
    // Vanilla builds this from `renderDistance.get()`, not a second setting,
    // so the value the server is told matches the one the client renders.
    let view_distance = i8::try_from(args.render_distance_chunks)
        .context("render distance must fit the ClientInformation byte")?;
    Ok(ClientInformation {
        language: args.client_locale.clone(),
        view_distance,
        chat_visibility: args.client_chat_visibility.into(),
        chat_colors: args.client_chat_colors,
        displayed_skin_parts: args.client_skin_parts,
        main_hand: args.client_main_hand.into(),
        text_filtering_enabled: args.client_text_filtering,
        allows_listing: args.client_allow_server_listing,
        particle_status: args.client_particles.into(),
    })
}

fn parse_client_gamma(value: &str) -> std::result::Result<f32, String> {
    let gamma = value
        .parse::<f32>()
        .map_err(|err| format!("client gamma must be a number: {err}"))?;
    if !gamma.is_finite() {
        return Err("client gamma must be finite".to_string());
    }
    if !(0.0..=1.0).contains(&gamma) {
        return Err("client gamma must be between 0.0 and 1.0".to_string());
    }
    Ok(gamma)
}

fn parse_text_background_opacity(value: &str) -> std::result::Result<f32, String> {
    let opacity = value
        .parse::<f32>()
        .map_err(|err| format!("text background opacity must be a number: {err}"))?;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err("text background opacity must be finite and between 0.0 and 1.0".to_string());
    }
    Ok(opacity)
}

fn parse_notification_display_time(value: &str) -> std::result::Result<f32, String> {
    let multiplier = value
        .parse::<f32>()
        .map_err(|err| format!("notification display time must be a number: {err}"))?;
    if !multiplier.is_finite() || !(0.5..=10.0).contains(&multiplier) {
        return Err(
            "notification display time must be finite and between 0.5 and 10.0".to_string(),
        );
    }
    Ok(multiplier)
}

fn parse_client_fov(value: &str) -> std::result::Result<u16, String> {
    let degrees = value
        .parse::<u16>()
        .map_err(|_| format!("client FOV must be an integer degree count: {value}"))?;
    if (VANILLA_MIN_CAMERA_FOV_DEGREES..=VANILLA_MAX_CAMERA_FOV_DEGREES).contains(&degrees) {
        Ok(degrees)
    } else {
        Err(format!(
            "client FOV must be between {VANILLA_MIN_CAMERA_FOV_DEGREES} and \
             {VANILLA_MAX_CAMERA_FOV_DEGREES} degrees"
        ))
    }
}

fn parse_client_fov_effect_scale(value: &str) -> std::result::Result<f32, String> {
    let scale = value
        .parse::<f32>()
        .map_err(|err| format!("client FOV effect scale must be a number: {err}"))?;
    if !scale.is_finite() {
        return Err("client FOV effect scale must be finite".to_string());
    }
    if !(0.0..=1.0).contains(&scale) {
        return Err("client FOV effect scale must be between 0.0 and 1.0".to_string());
    }
    Ok(scale)
}

fn parse_client_chat_delay(value: &str) -> std::result::Result<f64, String> {
    let delay = value
        .parse::<f64>()
        .map_err(|error| format!("invalid client chat delay: {error}"))?;
    if !delay.is_finite() || !(0.0..=6.0).contains(&delay) {
        return Err("client chat delay must be finite and between 0.0 and 6.0 seconds".to_string());
    }
    Ok(delay)
}

fn parse_damage_tilt_strength(value: &str) -> std::result::Result<f32, String> {
    let strength = value
        .parse::<f32>()
        .map_err(|err| format!("damage tilt strength must be a number: {err}"))?;
    if !strength.is_finite() {
        return Err("damage tilt strength must be finite".to_string());
    }
    if !(0.0..=1.0).contains(&strength) {
        return Err("damage tilt strength must be between 0.0 and 1.0".to_string());
    }
    Ok(strength)
}

fn parse_client_screen_effect_scale(value: &str) -> std::result::Result<f32, String> {
    let scale = value
        .parse::<f32>()
        .map_err(|err| format!("screen effect scale must be a number: {err}"))?;
    if scale.is_finite() && (0.0..=1.0).contains(&scale) {
        Ok(scale)
    } else {
        Err("screen effect scale must be between 0.0 and 1.0".to_string())
    }
}

fn parse_stored_attack_indicator(
    value: &str,
) -> std::result::Result<ClientAttackIndicatorArg, String> {
    match value {
        "0" => Ok(ClientAttackIndicatorArg::Off),
        "1" => Ok(ClientAttackIndicatorArg::Crosshair),
        "2" => Ok(ClientAttackIndicatorArg::Hotbar),
        other => Err(format!("attack indicator must be 0, 1 or 2, got {other}")),
    }
}

fn parse_client_unit_double(value: &str) -> std::result::Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("expected a number, got {value}"))?;
    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(format!("expected a value between 0 and 1, got {value}"));
    }
    Ok(parsed)
}

fn parse_client_mouse_wheel_sensitivity(value: &str) -> std::result::Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("expected a number, got {value}"))?;
    // `Options.mouseWheelSensitivity` is an `IntRange(-200, 100)` mapped through
    // `logMouse`, so the stored double never leaves `10^-2 ..= 10^1`.
    if !parsed.is_finite() || !(0.01..=10.0).contains(&parsed) {
        return Err(format!("expected a value between 0.01 and 10, got {value}"));
    }
    Ok(parsed)
}

/// `Options.guiScale` (Options.java:853-865): `0` (auto) through
/// `MAX_GUI_SCALE_INCLUSIVE`; `Window.calculateScale` stops the loop at the
/// cap, so the effective maximum is whatever the framebuffer can still fit at
/// 320x240.
fn parse_client_gui_scale(value: &str) -> std::result::Result<u32, String> {
    window_surface::parse_gui_scale(value)
}

/// The stored `fullscreenResolution` line (`Options.fullscreenVideoModeString`
/// -> `VideoMode.read`); an invalid line keeps the current (unset) value,
/// matching `Optional.empty()`.
fn parse_client_fullscreen_resolution(
    value: &str,
) -> std::result::Result<window_surface::PreferredVideoMode, String> {
    window_surface::PreferredVideoMode::parse(value)
        .ok_or_else(|| format!("invalid fullscreen resolution: {value}"))
}

fn parse_stored_narrator(value: &str) -> std::result::Result<ClientNarratorArg, String> {
    match value {
        "0" => Ok(ClientNarratorArg::Off),
        "1" => Ok(ClientNarratorArg::All),
        "2" => Ok(ClientNarratorArg::Chat),
        "3" => Ok(ClientNarratorArg::System),
        other => Err(format!("narrator must be 0, 1, 2 or 3, got {other}")),
    }
}

fn parse_render_distance(value: &str) -> std::result::Result<u32, String> {
    let chunks = value
        .parse::<u32>()
        .map_err(|_| format!("render distance must be an integer chunk count: {value}"))?;
    if (VANILLA_MIN_RENDER_DISTANCE_CHUNKS..=VANILLA_MAX_RENDER_DISTANCE_CHUNKS).contains(&chunks) {
        Ok(chunks)
    } else {
        Err(format!(
            "render distance must be between {VANILLA_MIN_RENDER_DISTANCE_CHUNKS} and {VANILLA_MAX_RENDER_DISTANCE_CHUNKS} chunks"
        ))
    }
}

fn parse_menu_background_blurriness(value: &str) -> std::result::Result<u8, String> {
    let blurriness = value
        .parse::<u8>()
        .map_err(|_| format!("menu background blurriness must be an integer: {value}"))?;
    if blurriness <= VANILLA_MAX_MENU_BACKGROUND_BLURRINESS {
        Ok(blurriness)
    } else {
        Err(format!(
            "menu background blurriness must be between 0 and {VANILLA_MAX_MENU_BACKGROUND_BLURRINESS}"
        ))
    }
}

fn parse_client_framerate_limit(value: &str) -> std::result::Result<u32, String> {
    if value.eq_ignore_ascii_case("inf") || value.eq_ignore_ascii_case("unlimited") {
        return Ok(VANILLA_UNLIMITED_FRAMERATE_LIMIT);
    }
    let limit = value
        .parse::<u32>()
        .map_err(|_| format!("client framerate limit must be an integer or inf: {value}"))?;
    // Vanilla `Options.framerateLimit` persists through `Codec.intRange(10,
    // 260)`: the slider only reaches multiples of ten, but any stored value in
    // range loads, and only `UNLIMITED_FRAMERATE_CUTOFF` (260) lifts the cap.
    if (10..=VANILLA_UNLIMITED_FRAMERATE_LIMIT).contains(&limit) {
        Ok(limit)
    } else {
        Err("client framerate limit must be 10..=260, inf, or unlimited".to_string())
    }
}

fn parse_cloud_range(value: &str) -> std::result::Result<u32, String> {
    let range = value
        .parse::<u32>()
        .map_err(|_| format!("cloud range must be an integer: {value}"))?;
    if (VANILLA_MIN_CLOUD_RANGE_CHUNKS..=VANILLA_MAX_CLOUD_RANGE_CHUNKS).contains(&range) {
        Ok(range)
    } else {
        Err(format!(
            "cloud range must be between {VANILLA_MIN_CLOUD_RANGE_CHUNKS} and {VANILLA_MAX_CLOUD_RANGE_CHUNKS} chunks"
        ))
    }
}

/// `CloudStatus.CODEC` reads the legacy serialized names; the pre-enum
/// `options.txt` shape also stored bare booleans, which
/// `EnumNameOrBoolean` still loads. The friendly names stay accepted for
/// the command line.
fn parse_stored_render_clouds(value: &str) -> std::result::Result<RenderCloudsArg, String> {
    match value.trim_matches('"') {
        "true" | "fancy" => Ok(RenderCloudsArg::Fancy),
        "fast" => Ok(RenderCloudsArg::Fast),
        "false" | "off" => Ok(RenderCloudsArg::Off),
        other => Err(format!(
            "render clouds must be true, fast, false, fancy or off, got {other}"
        )),
    }
}

fn parse_stored_inactivity_fps_limit(
    value: &str,
) -> std::result::Result<InactivityFpsLimitArg, String> {
    match value.trim_matches('"') {
        "minimized" => Ok(InactivityFpsLimitArg::Minimized),
        "afk" => Ok(InactivityFpsLimitArg::Afk),
        other => Err(format!(
            "inactivity fps limit must be minimized or afk, got {other}"
        )),
    }
}

/// Vanilla `FramerateLimitTracker.ICONIFIED_WINDOW_LIMIT` / `LONG_AFK_LIMIT`.
const VANILLA_ICONIFIED_FRAMERATE_LIMIT: u32 = 10;
/// Vanilla `FramerateLimitTracker.AFK_LIMIT`.
const VANILLA_SHORT_AFK_FRAMERATE_LIMIT: u32 = 30;
/// Vanilla `FramerateLimitTracker.OUT_OF_LEVEL_MENU_LIMIT`.
const VANILLA_OUT_OF_LEVEL_MENU_FRAMERATE_LIMIT: u32 = 60;
/// Vanilla `FramerateLimitTracker.AFK_THRESHOLD_MS` / `LONG_AFK_THRESHOLD_MS`.
const VANILLA_AFK_THRESHOLD_MS: u128 = 60_000;
const VANILLA_LONG_AFK_THRESHOLD_MS: u128 = 600_000;

/// Vanilla `FramerateLimitTracker.getFramerateLimit`: an iconified window and
/// a long AFK force 10 fps, a short AFK caps at 30, and a menu with no level
/// runs at exactly 60 — even above the configured limit. The window state is
/// checked before the AFK branches, which only fire with the option on `AFK`.
/// bbb has no vanilla `LoadingOverlay` equivalent, so the out-of-level branch
/// only tests the screen set.
pub(crate) fn effective_framerate_limit(
    framerate_limit: u32,
    inactivity_fps_limit: InactivityFpsLimitArg,
    window_iconified: bool,
    afk_millis: u128,
    level_loaded: bool,
    menu_screen_open: bool,
) -> u32 {
    if window_iconified {
        return VANILLA_ICONIFIED_FRAMERATE_LIMIT;
    }
    if inactivity_fps_limit == InactivityFpsLimitArg::Afk {
        if afk_millis > VANILLA_LONG_AFK_THRESHOLD_MS {
            return VANILLA_ICONIFIED_FRAMERATE_LIMIT;
        }
        if afk_millis > VANILLA_AFK_THRESHOLD_MS {
            return framerate_limit.min(VANILLA_SHORT_AFK_FRAMERATE_LIMIT);
        }
    }
    if !level_loaded && menu_screen_open {
        return VANILLA_OUT_OF_LEVEL_MENU_FRAMERATE_LIMIT;
    }
    framerate_limit
}

pub(crate) fn client_framerate_limit_interval(client_framerate_limit: u32) -> Option<Duration> {
    if client_framerate_limit == 0 || client_framerate_limit >= VANILLA_UNLIMITED_FRAMERATE_LIMIT {
        return None;
    }
    Some(Duration::from_nanos(
        (1_000_000_000.0 / f64::from(client_framerate_limit))
            .round()
            .clamp(1.0, u64::MAX as f64) as u64,
    ))
}

pub(crate) fn start_control_api(
    runtime: &Runtime,
    addr: Option<SocketAddr>,
    control: &ControlState,
) {
    let Some(addr) = addr else {
        return;
    };

    let control = control.clone();
    runtime.spawn(async move {
        if let Err(err) = bbb_control::serve(addr, control).await {
            tracing::error!(?err, "control API stopped");
        }
    });
}

pub(crate) fn create_event_loop() -> Result<EventLoop<()>> {
    Ok(EventLoop::new()?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "bbb-native".to_string(),
            width: 1280,
            height: 720,
        }
    }
}

impl WindowConfig {
    pub(crate) fn physical_size(&self) -> PhysicalSize<u32> {
        // `Window.allowedWindowMinSize` (Window.java:551) clamps every stored
        // window dimension to at least one, so a degenerate display size can
        // never construct a zero-width native window.
        PhysicalSize::new(
            window_surface::allowed_window_min_size(self.width),
            window_surface::allowed_window_min_size(self.height),
        )
    }
}

/// `Window.createWindow`/`createGlfwWindow` collapsed into one owner. The
/// GLFW error plumbing around it — `checkGlfwError`'s error-queue drain,
/// `defaultErrorCallback`, and the `setBootErrorCallback`/
/// `setDefaultErrorCallback` install/swap pair — has no equivalent surface:
/// winit and wgpu return typed `Result`s at each call site instead of parking
/// errors in a process-global queue behind an installable callback, so every
/// failure is handled where it occurs and flows into the guarded crash
/// report.
pub(crate) fn build_window(event_loop: &EventLoop<()>) -> Result<Window> {
    let config = WindowConfig::default();
    WindowBuilder::new()
        .with_title(config.title.clone())
        .with_inner_size(config.physical_size())
        .build(event_loop)
        .context("create native window")
}

pub(crate) fn spawn_frame_tick(event_loop: &EventLoop<()>) {
    let event_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        while event_proxy.send_event(()).is_ok() {
            thread::sleep(Duration::from_millis(16));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_forwarder_preserves_visibility_before_simulation_wake() {
        let source = include_str!("startup.rs");
        let forwarded = source
            .find("simulation_tx.send(event).await")
            .expect("ordered network event forward");
        let wake = source
            .find("network_wake.notify_external_work()")
            .expect("simulation network wake");
        assert!(forwarded < wake);
    }

    #[test]
    fn args_accept_delegated_status_server_and_reject_probe_conflict() {
        let args =
            Args::try_parse_from(["bbb-native", "--status-server", "127.0.0.1:25565"]).unwrap();
        assert_eq!(args.status_server.as_deref(), Some("127.0.0.1:25565"));
        assert!(Args::try_parse_from([
            "bbb-native",
            "--status-server",
            "127.0.0.1:25565",
            "--probe-server",
        ])
        .is_err());
    }

    #[test]
    fn build_info_is_machine_readable_and_uses_protocol_constants() {
        let args = Args::try_parse_from(["bbb-native", "--build-info"]).unwrap();
        assert!(args.build_info);

        let value = serde_json::to_value(BuildInfo::current()).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(value["client"], "bbb-native");
        assert_eq!(value["client_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["minecraft_version"], bbb_protocol::MC_VERSION);
        assert_eq!(value["protocol_version"], bbb_protocol::PROTOCOL_VERSION);
        assert_eq!(
            value["resource_pack_format"],
            bbb_protocol::MC_RESOURCE_PACK_FORMAT.to_vanilla_string()
        );
        assert_eq!(
            value["data_pack_format"],
            bbb_protocol::MC_DATA_PACK_FORMAT.to_vanilla_string()
        );
        assert_eq!(value["target_os"], std::env::consts::OS);
        assert_eq!(value["target_arch"], std::env::consts::ARCH);
    }

    #[test]
    fn tracing_keeps_machine_readable_cli_stdout_clean() {
        let source = include_str!("startup.rs");
        assert!(source.contains(".with_writer(std::io::stderr)"));
    }

    #[test]
    fn server_list_owner_requires_explicit_store_name_and_live_connection() {
        assert!(
            Args::try_parse_from(["bbb-native", "--server-list-store", "/tmp/servers.dat"])
                .is_err()
        );
        assert!(
            Args::try_parse_from(["bbb-native", "--server-list-name", "Native Server"]).is_err()
        );
        let args = Args::try_parse_from([
            "bbb-native",
            "--connect-server",
            "--server",
            "example.org:25565",
            "--server-list-store",
            "/tmp/servers.dat",
            "--server-list-name",
            "Native Server",
        ])
        .unwrap();
        assert_eq!(
            args.server_list_store,
            Some(PathBuf::from("/tmp/servers.dat"))
        );
        assert_eq!(args.server_list_name.as_deref(), Some("Native Server"));
    }

    #[test]
    fn session_credentials_are_paired_redacted_and_build_authenticated_options() {
        let profile_id = "12345678-1234-5678-90ab-cdef12345678";
        assert!(Args::try_parse_from(["bbb-native", "--profile-id", profile_id]).is_err());
        assert!(Args::try_parse_from(["bbb-native", "--access-token", "secret"]).is_err());

        let offline = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!connection_options_from_args(&offline)
            .unwrap()
            .is_authenticated());

        let authenticated = Args::try_parse_from([
            "bbb-native",
            "--profile-id",
            profile_id,
            "--access-token",
            "sensitive-access-token",
        ])
        .unwrap();
        let debug = format!("{authenticated:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sensitive-access-token"));
        let options = connection_options_from_args(&authenticated).unwrap();
        assert!(options.is_authenticated());
        assert_eq!(options.profile_id, Uuid::parse_str(profile_id).unwrap());
    }

    #[test]
    fn services_block_list_startup_client_requires_authenticated_nonempty_credentials() {
        let offline = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(services_block_list_client_from_args(&offline)
            .unwrap()
            .is_none());

        let authenticated = Args::try_parse_from([
            "bbb-native",
            "--profile-id",
            "12345678-1234-5678-90ab-cdef12345678",
            "--access-token",
            "sensitive-access-token",
        ])
        .unwrap();
        let client = services_block_list_client_from_args(&authenticated)
            .unwrap()
            .unwrap();
        let debug = format!("{client:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sensitive-access-token"));

        let empty = Args::try_parse_from([
            "bbb-native",
            "--profile-id",
            "12345678-1234-5678-90ab-cdef12345678",
            "--access-token",
            "",
        ])
        .unwrap();
        assert!(services_block_list_client_from_args(&empty).is_err());
    }

    #[test]
    fn player_reporting_locale_uses_bcp47_style_startup_language_tag() {
        assert_eq!(report_locale_from_client_locale("en_us"), "en-US");
        assert_eq!(report_locale_from_client_locale("zh_hans_cn"), "zh-Hans-CN");
        assert_eq!(report_locale_from_client_locale("pt-BR"), "pt-BR");
    }

    #[test]
    fn player_reporting_environment_requires_authenticated_startup_credentials() {
        let offline = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(player_report_environment_from_args(&offline), None);

        let authenticated = Args::try_parse_from([
            "bbb-native",
            "--server",
            "example.org:25565",
            "--client-locale",
            "zh_hans_cn",
            "--profile-id",
            "12345678-1234-5678-90ab-cdef12345678",
            "--access-token",
            "sensitive-access-token",
        ])
        .unwrap();
        assert_eq!(
            player_report_environment_from_args(&authenticated),
            Some(PlayerReportEnvironment::third_party(
                format!("{} (modded)", bbb_protocol::MC_VERSION),
                "zh-Hans-CN",
                "example.org:25565",
            ))
        );
    }

    #[test]
    fn args_accept_repeated_resource_pack_dirs() {
        let args = Args::try_parse_from([
            "bbb-native",
            "--resource-pack-dir",
            "packs/base",
            "--resource-pack-dir",
            "packs/overlay",
        ])
        .unwrap();

        assert_eq!(
            args.resource_pack_dirs,
            vec![PathBuf::from("packs/base"), PathBuf::from("packs/overlay")]
        );
    }

    #[test]
    fn social_interactions_blocked_players_are_startup_uuid_inputs() {
        let first = Uuid::from_u128(0x11111111111141118111111111111111);
        let second = Uuid::from_u128(0x22222222222242228222222222222222);
        let args = Args::try_parse_from([
            "bbb-native",
            "--client-blocked-player",
            &first.to_string(),
            "--client-blocked-player",
            &second.to_string(),
        ])
        .unwrap();

        assert_eq!(args.client_blocked_players, vec![first, second]);
        assert!(
            Args::try_parse_from(["bbb-native", "--client-blocked-player", "not-a-uuid",]).is_err()
        );
    }

    #[test]
    fn args_select_server_resource_pack_policy_and_cache_directory() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            defaults.server_resource_pack_policy,
            ServerResourcePackPolicyArg::Decline
        );
        assert_eq!(defaults.server_resource_pack_cache_dir, None);
        assert_eq!(
            connection_options_from_args(&defaults)
                .unwrap()
                .server_resource_pack_policy,
            ServerResourcePackPolicy::Decline
        );

        let accepted = Args::try_parse_from([
            "bbb-native",
            "--server-resource-pack-policy",
            "accept",
            "--server-resource-pack-cache-dir",
            "/tmp/bbb-server-packs",
        ])
        .unwrap();
        assert_eq!(
            accepted.server_resource_pack_policy,
            ServerResourcePackPolicyArg::Accept
        );
        assert_eq!(
            accepted.server_resource_pack_cache_dir,
            Some(PathBuf::from("/tmp/bbb-server-packs"))
        );
        assert_eq!(
            connection_options_from_args(&accepted)
                .unwrap()
                .server_resource_pack_policy,
            ServerResourcePackPolicy::Accept
        );
    }

    #[test]
    fn args_accept_player_skin_cache_dir() {
        let args =
            Args::try_parse_from(["bbb-native", "--player-skin-cache-dir", "/tmp/bbb-skins"])
                .unwrap();

        assert_eq!(
            args.player_skin_cache_dir,
            Some(PathBuf::from("/tmp/bbb-skins"))
        );
    }

    #[test]
    fn args_accept_explicit_crash_report_directory() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(defaults.crash_report_dir, None);

        let configured =
            Args::try_parse_from(["bbb-native", "--crash-report-dir", "/tmp/bbb-crash-reports"])
                .unwrap();
        assert_eq!(
            configured.crash_report_dir,
            Some(PathBuf::from("/tmp/bbb-crash-reports"))
        );
    }

    #[test]
    fn args_accept_narrator_status_command_and_repeated_arguments() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.client_narrator, ClientNarratorArg::Off);
        assert_eq!(default_args.narrator_command, None);
        assert!(default_args.narrator_command_args.is_empty());

        let args = Args::try_parse_from([
            "bbb-native",
            "--client-narrator",
            "system",
            "--narrator-command",
            "/usr/bin/spd-say",
            "--narrator-command-arg=--wait",
            "--narrator-command-arg={text}",
        ])
        .unwrap();
        assert_eq!(args.client_narrator, ClientNarratorArg::System);
        assert_eq!(
            args.narrator_command,
            Some(PathBuf::from("/usr/bin/spd-say"))
        );
        assert_eq!(args.narrator_command_args, ["--wait", "{text}"]);
        assert_eq!(
            NarratorStatus::from(args.client_narrator),
            NarratorStatus::System
        );
    }

    #[test]
    fn startup_sound_subtitle_options_use_vanilla_defaults_and_bounds() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!defaults.client_show_subtitles);
        assert_eq!(defaults.notification_display_time, 1.0);

        let configured = Args::try_parse_from([
            "bbb-native",
            "--client-show-subtitles=true",
            "--notification-display-time",
            "10.0",
        ])
        .unwrap();
        assert!(configured.client_show_subtitles);
        assert_eq!(configured.notification_display_time, 10.0);

        for invalid in ["0.49", "10.01", "NaN", "inf"] {
            assert!(
                Args::try_parse_from(["bbb-native", "--notification-display-time", invalid,])
                    .is_err()
            );
        }
    }

    #[test]
    fn tutorial_step_is_startup_configured_and_defaults_to_movement() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.tutorial_step, TutorialStepArg::Movement);
        assert_eq!(
            TutorialStep::from(default_args.tutorial_step),
            TutorialStep::Movement
        );

        let args = Args::try_parse_from(["bbb-native", "--tutorial-step", "craft-planks"]).unwrap();
        assert_eq!(args.tutorial_step, TutorialStepArg::CraftPlanks);
        assert_eq!(
            TutorialStep::from(args.tutorial_step),
            TutorialStep::CraftPlanks
        );
    }

    #[test]
    fn args_accept_vanilla_debug_framerate_limits() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            default_args.client_framerate_limit,
            VANILLA_UNLIMITED_FRAMERATE_LIMIT
        );

        let capped =
            Args::try_parse_from(["bbb-native", "--client-framerate-limit", "120"]).unwrap();
        assert_eq!(capped.client_framerate_limit, 120);

        let unlimited =
            Args::try_parse_from(["bbb-native", "--client-framerate-limit", "inf"]).unwrap();
        assert_eq!(
            unlimited.client_framerate_limit,
            VANILLA_UNLIMITED_FRAMERATE_LIMIT
        );

        // `Codec.intRange(10, 260)` loads every in-range value; the slider
        // granularity never restricts the stored one.
        let odd = Args::try_parse_from(["bbb-native", "--client-framerate-limit", "251"]).unwrap();
        assert_eq!(odd.client_framerate_limit, 251);
        assert!(Args::try_parse_from(["bbb-native", "--client-framerate-limit", "9"]).is_err());
    }

    #[test]
    fn cloud_range_matches_vanilla_option_bounds() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            default_args.cloud_range_chunks,
            VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS
        );

        let fancy = Args::try_parse_from(["bbb-native", "--cloud-range", "64"]).unwrap();
        assert_eq!(fancy.cloud_range_chunks, 64);
        assert!(Args::try_parse_from(["bbb-native", "--cloud-range", "1"]).is_err());
        assert!(Args::try_parse_from(["bbb-native", "--cloud-range", "129"]).is_err());
    }

    #[test]
    fn args_accept_vanilla_default_vsync_toggle() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.client_vsync);

        let disabled = Args::try_parse_from(["bbb-native", "--client-vsync", "false"]).unwrap();
        assert!(!disabled.client_vsync);
    }

    #[test]
    fn args_accept_explicit_vanilla_terrain_texture_filtering() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            default_args.terrain_texture_filtering,
            TerrainTextureFilteringArg::None
        );

        let rgss =
            Args::try_parse_from(["bbb-native", "--terrain-texture-filtering", "rgss"]).unwrap();
        assert_eq!(
            TerrainTextureFiltering::from(rgss.terrain_texture_filtering),
            TerrainTextureFiltering::Rgss
        );
    }

    #[test]
    fn startup_client_auto_jump_uses_vanilla_default_and_explicit_toggle() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.client_auto_jump);

        let disabled = Args::try_parse_from(["bbb-native", "--client-auto-jump", "false"]).unwrap();
        assert!(!disabled.client_auto_jump);
    }

    #[test]
    fn args_accept_vanilla_default_cutout_leaves_toggle() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.cutout_leaves);

        let disabled = Args::try_parse_from(["bbb-native", "--cutout-leaves", "false"]).unwrap();
        assert!(!disabled.cutout_leaves);
    }

    #[test]
    fn args_accept_vanilla_chat_link_toggles() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.client_chat_links);
        assert!(default_args.client_chat_links_prompt);

        let disabled = Args::try_parse_from([
            "bbb-native",
            "--client-chat-links",
            "false",
            "--client-chat-links-prompt",
            "false",
        ])
        .unwrap();
        assert!(!disabled.client_chat_links);
        assert!(!disabled.client_chat_links_prompt);
    }

    #[test]
    fn client_framerate_limit_interval_matches_unlimited_and_finite_modes() {
        assert_eq!(
            client_framerate_limit_interval(VANILLA_UNLIMITED_FRAMERATE_LIMIT),
            None
        );
        assert_eq!(
            client_framerate_limit_interval(120),
            Some(Duration::from_nanos(8_333_333))
        );
        // 251-259 load through `Codec.intRange(10, 260)` as literal caps; only
        // the 260 cutoff itself lifts the limiter.
        assert_eq!(
            client_framerate_limit_interval(259),
            Some(Duration::from_nanos(3_861_004))
        );
    }

    #[test]
    fn client_framerate_limit_accepts_the_vanilla_codec_range() {
        assert_eq!(parse_client_framerate_limit("255"), Ok(255));
        assert_eq!(parse_client_framerate_limit("10"), Ok(10));
        assert_eq!(
            parse_client_framerate_limit("260"),
            Ok(VANILLA_UNLIMITED_FRAMERATE_LIMIT)
        );
        assert!(parse_client_framerate_limit("9").is_err());
        assert!(parse_client_framerate_limit("261").is_err());
    }

    #[test]
    fn effective_framerate_limit_matches_the_vanilla_tracker() {
        // Vanilla `FramerateLimitTracker.getFramerateLimit`: NONE keeps the
        // option, iconified/long-AFK force 10, short AFK caps at 30, and an
        // out-of-level menu runs at exactly 60 even above the configured cap.
        let base = 120;
        let afk = InactivityFpsLimitArg::Afk;
        assert_eq!(
            effective_framerate_limit(base, afk, false, 0, true, false),
            120
        );
        assert_eq!(
            effective_framerate_limit(base, afk, true, 0, true, false),
            10
        );
        assert_eq!(
            effective_framerate_limit(base, afk, false, 600_001, true, false),
            10
        );
        assert_eq!(
            effective_framerate_limit(base, afk, false, 60_001, true, false),
            30
        );
        assert_eq!(
            effective_framerate_limit(20, afk, false, 60_001, true, false),
            20,
            "a short AFK takes min(limit, 30)"
        );
        assert_eq!(
            effective_framerate_limit(20, afk, false, 0, false, true),
            60,
            "the out-of-level menu limit overrides even a lower option"
        );
        assert_eq!(
            effective_framerate_limit(base, afk, false, 0, false, false),
            120
        );
        // `inactivityFpsLimit: minimized` skips the AFK branches but keeps the
        // iconified clamp.
        let minimized = InactivityFpsLimitArg::Minimized;
        assert_eq!(
            effective_framerate_limit(base, minimized, false, 600_001, true, false),
            120
        );
        assert_eq!(
            effective_framerate_limit(base, minimized, true, 0, true, false),
            10
        );
    }

    #[test]
    fn stored_render_clouds_reads_legacy_names_and_booleans() {
        // `CloudStatus` legacy names: "true" fancy, "fast", "false" off; the
        // EnumNameOrBoolean shape also loads bare booleans.
        assert_eq!(
            parse_stored_render_clouds("\"true\""),
            Ok(RenderCloudsArg::Fancy)
        );
        assert_eq!(
            parse_stored_render_clouds("true"),
            Ok(RenderCloudsArg::Fancy)
        );
        assert_eq!(
            parse_stored_render_clouds("\"fast\""),
            Ok(RenderCloudsArg::Fast)
        );
        assert_eq!(
            parse_stored_render_clouds("false"),
            Ok(RenderCloudsArg::Off)
        );
        assert_eq!(parse_stored_render_clouds("off"), Ok(RenderCloudsArg::Off));
        assert!(parse_stored_render_clouds("\"sometimes\"").is_err());
    }

    #[test]
    fn stored_inactivity_fps_limit_reads_quoted_enum_names() {
        assert_eq!(
            parse_stored_inactivity_fps_limit("\"minimized\""),
            Ok(InactivityFpsLimitArg::Minimized)
        );
        assert_eq!(
            parse_stored_inactivity_fps_limit("afk"),
            Ok(InactivityFpsLimitArg::Afk)
        );
        assert!(parse_stored_inactivity_fps_limit("\"other\"").is_err());
    }

    #[test]
    fn args_build_client_information_from_startup_options() {
        let args = Args::try_parse_from([
            "bbb-native",
            "--client-locale",
            "zh_cn",
            "--render-distance",
            "12",
            "--client-chat-visibility",
            "system",
            "--client-chat-colors",
            "false",
            "--client-skin-parts",
            "21",
            "--client-main-hand",
            "left",
            "--client-text-filtering",
            "true",
            "--client-allow-server-listing",
            "true",
            "--client-particles",
            "minimal",
        ])
        .unwrap();

        let information = client_information_from_args(&args).unwrap();

        assert_eq!(information.language, "zh_cn");
        // Vanilla reports `renderDistance.get()`, so there is no second
        // view-distance setting that could disagree with what is rendered.
        assert_eq!(information.view_distance, 12);
        assert_eq!(
            i64::from(information.view_distance),
            i64::from(args.render_distance_chunks)
        );
        assert_eq!(information.chat_visibility, ClientChatVisibility::System);
        assert!(!information.chat_colors);
        assert_eq!(information.displayed_skin_parts, 21);
        assert_eq!(information.main_hand, ClientMainHand::Left);
        assert!(information.text_filtering_enabled);
        assert!(information.allows_listing);
        assert_eq!(information.particle_status, ClientParticleStatus::Minimal);
    }

    #[test]
    fn args_accept_chat_listener_presentation_options() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!defaults.client_only_show_secure_chat);
        assert!(defaults.client_hide_matched_names);
        assert_eq!(defaults.client_chat_delay_seconds, 0.0);
        assert!(!defaults.client_save_chat_drafts);

        let configured = Args::try_parse_from([
            "bbb-native",
            "--client-only-show-secure-chat",
            "true",
            "--client-hide-matched-names",
            "false",
            "--client-chat-delay-seconds",
            "6.0",
            "--client-save-chat-drafts",
            "true",
        ])
        .unwrap();
        assert!(configured.client_only_show_secure_chat);
        assert!(!configured.client_hide_matched_names);
        assert_eq!(configured.client_chat_delay_seconds, 6.0);
        assert!(configured.client_save_chat_drafts);
        assert!(
            Args::try_parse_from(["bbb-native", "--client-chat-delay-seconds", "6.1",]).is_err()
        );
    }

    #[test]
    fn args_accept_client_gamma_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            default_args.client_gamma,
            VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
        );

        let args = Args::try_parse_from(["bbb-native", "--client-gamma", "0.75"]).unwrap();
        assert_eq!(args.client_gamma, 0.75);
    }

    #[test]
    fn args_accept_vanilla_text_background_options() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            defaults.text_background_opacity,
            VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY
        );
        assert!(defaults.background_for_chat_only);

        let configured = Args::try_parse_from([
            "bbb-native",
            "--text-background-opacity",
            "0.75",
            "--background-for-chat-only",
            "false",
        ])
        .unwrap();
        assert_eq!(configured.text_background_opacity, 0.75);
        assert!(!configured.background_for_chat_only);
    }

    #[test]
    fn args_reject_text_background_opacity_outside_unit_range() {
        for invalid in [
            "--text-background-opacity=-0.1",
            "--text-background-opacity=1.1",
            "--text-background-opacity=NaN",
        ] {
            let error = Args::try_parse_from(["bbb-native", invalid]).unwrap_err();
            assert!(error.to_string().contains("between 0.0 and 1.0"));
        }
    }

    #[test]
    fn args_accept_render_distance_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            default_args.render_distance_chunks,
            VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS
        );

        let args = Args::try_parse_from(["bbb-native", "--render-distance", "20"]).unwrap();
        assert_eq!(args.render_distance_chunks, 20);
    }

    #[test]
    fn args_accept_vanilla_menu_background_blurriness_range() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            default_args.menu_background_blurriness,
            VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS
        );

        let disabled =
            Args::try_parse_from(["bbb-native", "--menu-background-blurriness", "0"]).unwrap();
        assert_eq!(disabled.menu_background_blurriness, 0);

        let maximum =
            Args::try_parse_from(["bbb-native", "--menu-background-blurriness", "10"]).unwrap();
        assert_eq!(maximum.menu_background_blurriness, 10);
    }

    #[test]
    fn args_reject_menu_background_blurriness_outside_vanilla_range() {
        let err =
            Args::try_parse_from(["bbb-native", "--menu-background-blurriness", "11"]).unwrap_err();
        assert!(err.to_string().contains("between 0 and 10"));
    }

    #[test]
    fn args_reject_render_distance_outside_vanilla_range() {
        let err = Args::try_parse_from(["bbb-native", "--render-distance", "1"]).unwrap_err();
        assert!(err.to_string().contains("between 2 and 32"));

        let err = Args::try_parse_from(["bbb-native", "--render-distance", "33"]).unwrap_err();
        assert!(err.to_string().contains("between 2 and 32"));

        let err = Args::try_parse_from(["bbb-native", "--render-distance", "NaN"]).unwrap_err();
        assert!(err.to_string().contains("integer chunk count"));
    }

    #[test]
    fn args_reject_client_gamma_outside_unit_range() {
        let err = Args::try_parse_from(["bbb-native", "--client-gamma", "1.25"]).unwrap_err();
        assert!(err.to_string().contains("between 0.0 and 1.0"));

        let err = Args::try_parse_from(["bbb-native", "--client-gamma", "NaN"]).unwrap_err();
        assert!(err.to_string().contains("must be finite"));
    }

    #[test]
    fn args_accept_fov_startup_options() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.client_fov, 70);
        assert_eq!(default_args.client_fov_effect_scale, 1.0);

        let args = Args::try_parse_from([
            "bbb-native",
            "--client-fov",
            "95",
            "--client-fov-effect-scale",
            "0.25",
        ])
        .unwrap();
        assert_eq!(args.client_fov, 95);
        assert_eq!(args.client_fov_effect_scale, 0.25);
    }

    #[test]
    fn args_accept_client_view_bobbing_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.client_view_bobbing);

        let args = Args::try_parse_from(["bbb-native", "--client-view-bobbing=false"]).unwrap();
        assert!(!args.client_view_bobbing);
    }

    #[test]
    fn args_use_vanilla_fancy_transparency_default_and_accept_explicit_improved_route() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!defaults.improved_transparency);

        let configured = Args::try_parse_from(["bbb-native", "--improved-transparency"]).unwrap();
        assert!(configured.improved_transparency);
    }

    #[test]
    fn args_accept_high_contrast_block_outline_startup_option() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!defaults.high_contrast_block_outline);

        let configured =
            Args::try_parse_from(["bbb-native", "--high-contrast-block-outline"]).unwrap();
        assert!(configured.high_contrast_block_outline);
    }

    #[test]
    fn args_reject_fov_startup_options_outside_vanilla_ranges() {
        let err = Args::try_parse_from(["bbb-native", "--client-fov", "29"]).unwrap_err();
        assert!(err.to_string().contains("between 30 and 110"));

        let err = Args::try_parse_from(["bbb-native", "--client-fov", "111"]).unwrap_err();
        assert!(err.to_string().contains("between 30 and 110"));

        let err =
            Args::try_parse_from(["bbb-native", "--client-fov-effect-scale", "1.25"]).unwrap_err();
        assert!(err.to_string().contains("between 0.0 and 1.0"));

        let err =
            Args::try_parse_from(["bbb-native", "--client-fov-effect-scale", "NaN"]).unwrap_err();
        assert!(err.to_string().contains("must be finite"));
    }

    #[test]
    fn args_accept_damage_tilt_strength_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.damage_tilt_strength, 1.0);

        let args = Args::try_parse_from(["bbb-native", "--damage-tilt-strength", "0.25"]).unwrap();
        assert_eq!(args.damage_tilt_strength, 0.25);
    }

    #[test]
    fn args_accept_entity_shadows_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.entity_shadows);

        let args = Args::try_parse_from(["bbb-native", "--entity-shadows=false"]).unwrap();
        assert!(!args.entity_shadows);
    }

    #[test]
    fn args_reject_damage_tilt_strength_outside_unit_range() {
        let err =
            Args::try_parse_from(["bbb-native", "--damage-tilt-strength", "1.25"]).unwrap_err();
        assert!(err.to_string().contains("between 0.0 and 1.0"));

        let err =
            Args::try_parse_from(["bbb-native", "--damage-tilt-strength", "NaN"]).unwrap_err();
        assert!(err.to_string().contains("must be finite"));
    }

    #[test]
    fn args_accept_hide_lightning_flash_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!default_args.hide_lightning_flash);

        let args = Args::try_parse_from(["bbb-native", "--hide-lightning-flash"]).unwrap();
        assert!(args.hide_lightning_flash);
    }

    #[test]
    fn args_accept_advanced_item_tooltips_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!default_args.advanced_item_tooltips);

        let args = Args::try_parse_from(["bbb-native", "--advanced-item-tooltips"]).unwrap();
        assert!(args.advanced_item_tooltips);
    }

    #[test]
    fn args_accept_debug_profile_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.debug_profile, DebugScreenProfileArg::Default);

        let args = Args::try_parse_from(["bbb-native", "--debug-profile", "performance"]).unwrap();
        assert_eq!(args.debug_profile, DebugScreenProfileArg::Performance);
    }

    #[test]
    fn args_accept_debug_profile_store_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.debug_profile_store, None);

        let args = Args::try_parse_from([
            "bbb-native",
            "--debug-profile-store",
            "/tmp/debug-profile.json",
        ])
        .unwrap();
        assert_eq!(
            args.debug_profile_store,
            Some(PathBuf::from("/tmp/debug-profile.json"))
        );
    }

    #[test]
    fn args_accept_debug_show_local_server_entity_hit_boxes_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!default_args.debug_show_local_server_entity_hit_boxes);

        let args =
            Args::try_parse_from(["bbb-native", "--debug-show-local-server-entity-hit-boxes"])
                .unwrap();
        assert!(args.debug_show_local_server_entity_hit_boxes);
    }

    #[test]
    fn args_accept_sharedconstants_debug_flag_startup_options() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!default_args.debug_hotkeys);
        assert!(!default_args.debug_feature_count);

        let args = Args::try_parse_from(["bbb-native", "--debug-hotkeys", "--debug-feature-count"])
            .unwrap();
        assert!(args.debug_hotkeys);
        assert!(args.debug_feature_count);
    }

    #[test]
    fn args_accept_repeatable_remote_debug_renderer_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(default_args.remote_debug_renderers.is_empty());

        let args = Args::try_parse_from([
            "bbb-native",
            "--remote-debug-renderer",
            "bees",
            "--remote-debug-renderer",
            "game-event-listeners",
        ])
        .unwrap();
        assert_eq!(
            args.remote_debug_renderers,
            vec![
                RemoteDebugRenderer::Bees,
                RemoteDebugRenderer::GameEventListeners,
            ]
        );
    }

    #[test]
    fn client_locale_rejects_more_than_sixteen_utf8_bytes() {
        let args =
            Args::try_parse_from(["bbb-native", "--client-locale", "abcdefghijklmnopq"]).unwrap();

        let err = client_information_from_args(&args).unwrap_err();

        assert!(err.to_string().contains("at most 16 UTF-8 bytes"));
    }

    #[test]
    fn apply_resource_pack_dirs_updates_pack_roots() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!(
            "bbb-startup-resource-packs-{}-{nonce}",
            std::process::id()
        ));
        let base = temp.join("packs/base");
        let overlay = temp.join("packs/overlay");
        for pack in [&base, &overlay] {
            std::fs::create_dir_all(pack).unwrap();
            std::fs::write(
                pack.join("pack.mcmeta"),
                r#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap();
        }
        let roots = PackRoots::from_paths(
            &temp,
            temp.join("sources/26.1"),
            temp.join("sources/26.1/assets/minecraft"),
        );

        let roots = apply_resource_pack_dirs(roots, [base.clone(), overlay.clone()]).unwrap();

        assert_eq!(roots.resource_pack_dirs(), vec![base, overlay]);
        assert_eq!(
            roots
                .selected_client_packs()
                .unwrap()
                .packs()
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["vanilla", "startup/0", "startup/1"]
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn narrator_voice_volume_multiplies_voice_and_master_with_safe_defaults() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        assert_eq!(narrator_voice_volume(&options), 1.0);

        options.set_dynamic_raw_for_test("soundCategory_voice", "0.5");
        options.set_dynamic_raw_for_test("soundCategory_master", "0.5");
        assert_eq!(narrator_voice_volume(&options), 0.25);

        options.set_dynamic_raw_for_test("soundCategory_master", "nonsense");
        assert_eq!(narrator_voice_volume(&options), 0.5);

        options.set_dynamic_raw_for_test("soundCategory_voice", "7.5");
        options.set_dynamic_raw_for_test("soundCategory_master", "0.5");
        assert_eq!(narrator_voice_volume(&options), 0.5);
    }

    #[test]
    fn stored_options_fill_arguments_the_user_did_not_type() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("renderDistance", "16");
        options.set_raw_for_test("entityShadows", "false");
        options.set_raw_for_test("lang", "zh_cn");
        options.set_raw_for_test("fov", "1.0");
        let mut args = Args::parse_from(["bbb-native"]);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert_eq!(args.render_distance_chunks, 16);
        assert!(!args.entity_shadows);
        assert_eq!(args.client_locale, "zh_cn");
        // `fov` is stored scaled; 1.0 reads back as the 110-degree maximum.
        assert_eq!(args.client_fov, 110);
    }

    #[test]
    fn a_stored_debug_toggle_comes_back_as_the_session_starting_value() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("advancedItemTooltips", "true");
        options.set_raw_for_test("pauseOnLostFocus", "false");
        let mut args = Args::parse_from(["bbb-native"]);
        assert!(!args.advanced_item_tooltips);
        assert!(args.pause_on_lost_focus);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        // Both are what F3+H and F3+P write back, so the write is only useful
        // if the next run starts from it.
        assert!(args.advanced_item_tooltips);
        assert!(!args.pause_on_lost_focus);
    }

    #[test]
    fn the_stored_window_family_options_feed_the_window_owner() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("guiScale", "3");
        options.set_raw_for_test("forceUnicodeFont", "true");
        options.set_raw_for_test("rawMouseInput", "false");
        options.set_raw_for_test("allowCursorChanges", "false");
        options.set_raw_for_test("fullscreenResolution", "1920x1080@144:24");
        let mut args = Args::parse_from(["bbb-native"]);
        assert!(args.client_fullscreen_resolution.is_none());
        assert_eq!(args.client_gui_scale, 0);
        assert!(!args.client_force_unicode_font);
        assert!(args.client_raw_mouse_input);
        assert!(args.client_allow_cursor_changes);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert_eq!(args.client_gui_scale, 3);
        assert!(args.client_force_unicode_font);
        assert!(!args.client_raw_mouse_input);
        assert!(!args.client_allow_cursor_changes);
        assert_eq!(
            args.client_fullscreen_resolution.map(|mode| mode.write()),
            Some("1920x1080@144:24".to_owned())
        );
    }

    #[test]
    fn a_typed_window_family_argument_wins_over_the_stored_line() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("guiScale", "3");
        options.set_raw_for_test("rawMouseInput", "false");
        let matches = Args::command().get_matches_from(["bbb-native", "--client-gui-scale", "2"]);
        let mut args = Args::from_arg_matches(&matches).unwrap();

        bind_stored_client_options(&mut args, StoredOptions::new(&options, &matches));

        assert_eq!(args.client_gui_scale, 2);
        assert!(!args.client_raw_mouse_input);
    }

    #[test]
    fn a_typed_fullscreen_resolution_wins_over_the_stored_line() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("fullscreenResolution", "1920x1080@144:24");
        let matches = Args::command().get_matches_from([
            "bbb-native",
            "--client-fullscreen-resolution",
            "1280x720@60",
        ]);
        let mut args = Args::from_arg_matches(&matches).unwrap();
        assert_eq!(
            args.client_fullscreen_resolution.map(|mode| mode.write()),
            Some("1280x720@60:24".to_owned())
        );

        bind_stored_client_options(&mut args, StoredOptions::new(&options, &matches));

        assert_eq!(
            args.client_fullscreen_resolution.map(|mode| mode.write()),
            Some("1280x720@60:24".to_owned())
        );
    }

    #[test]
    fn an_invalid_stored_fullscreen_resolution_keeps_the_current_value() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("fullscreenResolution", "not a video mode");
        let mut args = Args::parse_from(["bbb-native"]);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert!(args.client_fullscreen_resolution.is_none());
    }

    #[test]
    fn client_gui_scale_rejects_values_outside_the_vanilla_bound() {
        assert!(parse_client_gui_scale("0").is_ok());
        assert!(parse_client_gui_scale("4").is_ok());
        assert!(parse_client_gui_scale("2147483646").is_ok());
        assert!(parse_client_gui_scale("2147483647").is_err());
        assert!(parse_client_gui_scale("-1").is_err());
        assert!(parse_client_gui_scale("auto").is_err());
    }

    #[test]
    fn client_fullscreen_resolution_rejects_invalid_forms_at_parse_time() {
        assert_eq!(
            parse_client_fullscreen_resolution("1920x1080@144:24")
                .unwrap()
                .write(),
            "1920x1080@144:24"
        );
        assert!(parse_client_fullscreen_resolution("").is_err());
        assert!(parse_client_fullscreen_resolution("1920x1080@").is_err());
        assert!(parse_client_fullscreen_resolution("1920x1080@144:").is_err());
    }

    #[test]
    fn window_config_clamps_dimensions_to_minimum_nonzero() {
        let mut config = WindowConfig::default();
        config.width = 0;
        config.height = 0;
        assert_eq!(config.physical_size(), PhysicalSize::new(1, 1));

        let config = WindowConfig::default();
        assert_eq!(config.physical_size(), PhysicalSize::new(1280, 720));
    }

    #[test]
    fn the_stored_reduced_debug_info_option_becomes_the_session_starting_value() {
        // `Minecraft.showOnlyReducedInfo` (Minecraft.java:2666) reads
        // `options.reducedDebugInfo()`, which only ever comes from
        // `options.txt` for an external client.
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("reducedDebugInfo", "true");
        let mut args = Args::parse_from(["bbb-native"]);
        assert!(!args.client_reduced_debug_info);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert!(args.client_reduced_debug_info);
    }

    #[test]
    fn the_stored_vignette_option_defaults_on_and_comes_back_off() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("vignette", "false");
        let mut args = Args::parse_from(["bbb-native"]);
        assert!(args.client_vignette);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert!(!args.client_vignette);
    }

    #[test]
    fn the_stored_screen_effect_scale_defaults_full_and_rejects_out_of_range_lines() {
        let mut args = Args::parse_from(["bbb-native"]);
        assert_eq!(args.client_screen_effect_scale, 1.0);

        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("screenEffectScale", "0.25");
        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
        assert_eq!(args.client_screen_effect_scale, 0.25);

        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("screenEffectScale", "2.0");
        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
        assert_eq!(args.client_screen_effect_scale, 0.25);
    }

    #[test]
    fn the_stored_attack_indicator_option_reads_back_the_vanilla_enum_ids() {
        let mut args = Args::parse_from(["bbb-native"]);
        assert_eq!(
            args.client_attack_indicator,
            ClientAttackIndicatorArg::Crosshair
        );

        for (stored, expected) in [
            ("0", ClientAttackIndicatorArg::Off),
            ("2", ClientAttackIndicatorArg::Hotbar),
            ("1", ClientAttackIndicatorArg::Crosshair),
        ] {
            let mut options = ClientOptions::defaults(ClientOptionHost::default());
            options.set_raw_for_test("attackIndicator", stored);
            let mut args = Args::parse_from(["bbb-native"]);
            bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
            assert_eq!(args.client_attack_indicator, expected);
        }

        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("attackIndicator", "9");
        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
        assert_eq!(
            args.client_attack_indicator,
            ClientAttackIndicatorArg::Crosshair
        );
    }

    #[test]
    fn an_out_of_range_stored_line_keeps_the_argument_value() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("renderDistance", "9999");
        let mut args = Args::parse_from(["bbb-native"]);
        let default_distance = args.render_distance_chunks;

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        // The command line's own parser rejects it, so the value stands —
        // `FieldAccess.process` keeps the current value the same way.
        assert_eq!(args.render_distance_chunks, default_distance);
    }

    #[test]
    fn every_binding_names_a_real_argument_and_a_catalogued_key() {
        let mut args = Args::parse_from(["bbb-native"]);
        let options = ClientOptions::defaults(ClientOptionHost::default());

        let bindings =
            bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert!(!bindings.is_empty());
        let command = Args::command();
        for binding in bindings {
            assert!(
                command
                    .get_arguments()
                    .any(|arg| arg.get_id() == binding.arg_id),
                "no argument named {}",
                binding.arg_id
            );
        }
    }
}
