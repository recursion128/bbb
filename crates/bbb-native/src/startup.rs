use std::{
    convert::Infallible, fmt, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, thread,
    time::Duration,
};

use anyhow::{ensure, Context, Result};
use bbb_audio::AudioVolumeSettings;
use bbb_control::ControlState;
use bbb_net::{
    ClientIdentity, ConnectionOptions, ServerPackDownloadProxy, ServerResourcePackPolicy,
};
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
use bbb_world::TutorialStep;
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
    window::{Fullscreen, Window, WindowBuilder},
};

use crate::{
    code_of_conduct::CodeOfConductAcceptance, debug_entries::DebugScreenProfile,
    narrator_runtime::NarratorStatus, network_runtime::NetworkRuntime,
    remote_debug::RemoteDebugRenderer, simulation::commands::SimulationMailbox,
};

pub(crate) const VANILLA_UNLIMITED_FRAMERATE_LIMIT: u32 = 260;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RedactedString(String);

impl RedactedString {
    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RedactedString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl FromStr for RedactedString {
    type Err = Infallible;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(value.to_string()))
    }
}

#[derive(Debug, Parser)]
#[command(name = "bbb-native")]
pub(crate) struct Args {
    /// Vanilla `--gameDir`: the directory `options.txt` is read from and
    /// written back to.
    #[arg(
        long = "game-dir",
        alias = "gameDir",
        value_name = "PATH",
        default_value = "."
    )]
    pub(crate) game_dir: PathBuf,
    /// Managed cache root printed by `scripts/vanilla path 26.1 --kind root`.
    #[arg(long = "vanilla-root", value_name = "PATH")]
    pub(crate) vanilla_root: Option<PathBuf>,
    #[arg(long, default_value = "127.0.0.1:25565")]
    pub(crate) server: String,
    /// Vanilla `Main --quickPlayMultiplayer`; normalized onto BBB's one
    /// external-server startup path after stored options have been bound.
    #[arg(
        long = "quickPlayMultiplayer",
        value_name = "ADDR",
        conflicts_with_all = ["status_server", "probe_server"]
    )]
    pub(crate) quick_play_multiplayer: Option<String>,
    #[arg(long, default_value = "bbb-client")]
    pub(crate) username: String,
    /// Vanilla `Main --uuid`. Both 32-digit undashed and Java UUID group
    /// spellings are accepted; an invalid value warns and falls back to the
    /// username-derived offline UUID just like the official launcher owner.
    #[arg(long = "uuid", value_name = "UUID")]
    pub(crate) uuid: Option<String>,
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
    /// `Options.resourcePacks`, consumed only when no explicit CLI pack roots
    /// replace the Vanilla `<gameDir>/resourcepacks` repository.
    #[arg(skip)]
    pub(crate) stored_resource_packs: Vec<String>,
    /// `Options.incompatibleResourcePacks`, paired with the stored selection
    /// during repository reconciliation.
    #[arg(skip)]
    pub(crate) stored_incompatible_resource_packs: Vec<String>,
    #[arg(
        long = "server-resource-pack-policy",
        value_enum,
        default_value = "decline"
    )]
    pub(crate) server_resource_pack_policy: ServerResourcePackPolicyArg,
    #[arg(long = "server-resource-pack-cache-dir", value_name = "PATH")]
    pub(crate) server_resource_pack_cache_dir: Option<PathBuf>,
    /// Vanilla `Main --proxyHost`; server-pack HTTP uses an explicit SOCKS5
    /// proxy only when this is present.
    #[arg(long = "proxy-host", alias = "proxyHost", value_name = "HOST")]
    pub(crate) proxy_host: Option<String>,
    #[arg(
        long = "proxy-port",
        alias = "proxyPort",
        value_name = "PORT",
        default_value_t = 8080
    )]
    pub(crate) proxy_port: i32,
    #[arg(long = "proxy-user", alias = "proxyUser", value_name = "USER")]
    pub(crate) proxy_user: Option<String>,
    #[arg(long = "proxy-pass", alias = "proxyPass", value_name = "PASSWORD")]
    pub(crate) proxy_pass: Option<RedactedString>,
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
    /// Vanilla's no-argument `--fullscreen` launch switch. This is kept
    /// separate from BBB's explicit boolean option so both syntaxes retain
    /// their own argument contract.
    #[arg(long = "fullscreen", action = ArgAction::SetTrue)]
    pub(crate) vanilla_fullscreen: bool,
    /// Vanilla `Main --width/--height` defaults. Signed input is intentional:
    /// the native window owner clamps degenerate values to one.
    #[arg(long = "width", default_value_t = 854, allow_hyphen_values = true)]
    pub(crate) window_width: i32,
    #[arg(long = "height", default_value_t = 480, allow_hyphen_values = true)]
    pub(crate) window_height: i32,
    #[arg(long = "fullscreenWidth", allow_hyphen_values = true)]
    pub(crate) fullscreen_width: Option<i32>,
    #[arg(long = "fullscreenHeight", allow_hyphen_values = true)]
    pub(crate) fullscreen_height: Option<i32>,
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
        long = "client-japanese-glyph-variants",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_japanese_glyph_variants: bool,
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
    /// Empty is Vanilla's system-default sentinel; a non-empty value is the
    /// persisted audio device display name. Explicit startup configuration
    /// wins over `Options.soundDevice` because there is no in-game options UI.
    #[arg(long = "client-sound-device", value_name = "NAME", default_value = "")]
    pub(crate) client_sound_device: String,
    /// Vanilla `Options.directionalAudio`: requests HRTF only when the
    /// verified OpenAL implementation reports a supported specifier.
    #[arg(
        long = "client-directional-audio",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_directional_audio: bool,
    /// All `soundCategory_*` values are startup-only configuration in this
    /// external client. They still feed every native sound resolution just as
    /// `Options.getFinalSoundSourceVolume` does in Vanilla.
    #[arg(skip)]
    pub(crate) client_audio_volumes: AudioVolumeSettings,
    /// Vanilla `Options.musicToast`, startup-only because BBB deliberately has
    /// no in-game options UI.
    #[arg(long = "client-music-toast", value_enum, default_value = "never")]
    pub(crate) client_music_toast: MusicToastDisplayStateArg,
    /// Vanilla `Options.musicFrequency`, consumed by the native
    /// `MusicManager` schedule.
    #[arg(long = "client-music-frequency", value_enum, default_value = "default")]
    pub(crate) client_music_frequency: MusicFrequencyArg,
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
    /// Vanilla's process-persisted first-multiplayer-server sentinel. It is a
    /// startup option because BBB intentionally has no in-game options UI.
    #[arg(
        long = "client-joined-first-server",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_joined_first_server: bool,
    #[arg(
        long = "render-distance",
        default_value_t = VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
        value_parser = parse_render_distance
    )]
    pub(crate) render_distance_chunks: u32,
    #[arg(long = "graphics-preset", value_enum, default_value = "fancy")]
    pub(crate) graphics_preset: GraphicsPresetArg,
    #[arg(
        long = "biome-blend-radius",
        default_value_t = 2,
        value_parser = clap::value_parser!(u32).range(0..=7)
    )]
    pub(crate) biome_blend_radius: u32,
    #[arg(long = "prioritize-chunk-updates", value_enum, default_value = "none")]
    pub(crate) prioritize_chunk_updates: PrioritizeChunkUpdatesArg,
    #[arg(
        long = "client-simulation-distance",
        default_value_t = 12,
        value_parser = clap::value_parser!(u32).range(5..=32)
    )]
    pub(crate) client_simulation_distance_chunks: u32,
    #[arg(
        long = "client-mipmap-levels",
        default_value_t = 4,
        value_parser = clap::value_parser!(u32).range(0..=4)
    )]
    pub(crate) client_mipmap_levels: u32,
    #[arg(
        long = "client-entity-distance-scaling",
        default_value_t = 1.0,
        value_parser = parse_client_entity_distance_scaling
    )]
    pub(crate) client_entity_distance_scaling: f64,
    #[arg(
        long = "client-weather-radius",
        default_value_t = 10,
        value_parser = clap::value_parser!(u32).range(3..=10)
    )]
    pub(crate) client_weather_radius: u32,
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
    /// `GraphicsPreset.FABULOUS` chooses RGSS on an AMD renderer and
    /// anisotropic filtering otherwise. Resolution is deferred until wgpu has
    /// selected the actual adapter; an explicit filtering argument disables
    /// this automatic branch.
    #[arg(skip)]
    pub(crate) fabulous_texture_filtering: bool,
    /// Vanilla `maxAnisotropyBit` is `OptionInstance.IntRange(1, 3)` with
    /// default 2 (2^bit anisotropy); only consumed under `anisotropic`
    /// terrain texture filtering.
    #[arg(
        long = "client-max-anisotropy-bit",
        default_value_t = 2,
        value_parser = clap::value_parser!(u32).range(1..=3)
    )]
    pub(crate) client_max_anisotropy_bit: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum MusicToastDisplayStateArg {
    Never,
    Pause,
    PauseAndToast,
}

impl MusicToastDisplayStateArg {
    pub(crate) fn renders_pause(self) -> bool {
        matches!(self, Self::Pause | Self::PauseAndToast)
    }

    pub(crate) fn renders_toast(self) -> bool {
        self == Self::PauseAndToast
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum MusicFrequencyArg {
    Default,
    Frequent,
    Constant,
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

/// Vanilla 26.1 `GraphicsPreset`; `CUSTOM` deliberately performs no writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum GraphicsPresetArg {
    Fast,
    Fancy,
    Fabulous,
    Custom,
}

/// Vanilla 26.1 `PrioritizeChunkUpdates`, persisted through its wrapped
/// integer id codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum PrioritizeChunkUpdatesArg {
    None,
    PlayerAffected,
    Nearby,
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
    Anisotropic,
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
fn terrain_atlas_base_padding(
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

/// `Main.main` names the GUI owner `Render thread` immediately before native
/// renderer initialization. Linux `perf` and `/proc` consume the pthread name,
/// so use that same OS-visible identity rather than a Rust-only label.
#[cfg(target_os = "linux")]
pub(crate) fn name_current_render_thread() -> Result<()> {
    let name = c"Render thread";
    // SAFETY: `pthread_self` identifies the current live thread, `name` is a
    // static NUL-terminated string, and its 13 bytes fit Linux's 16-byte limit.
    let result = unsafe { libc::pthread_setname_np(libc::pthread_self(), name.as_ptr()) };
    ensure!(result == 0, "pthread_setname_np failed with errno {result}");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn name_current_render_thread() -> Result<()> {
    Ok(())
}

/// `Main.hasValidUuid` / `UndashedUuid.fromStringLenient` followed by
/// `UUIDUtil.createOfflinePlayerUUID`. Keep this as the only startup identity
/// decision so login, world state, and server-pack headers share one UUID.
pub(crate) fn client_identity_from_args(args: &Args) -> ClientIdentity {
    if let Some(value) = args.uuid.as_deref() {
        if let Some(profile_id) = parse_vanilla_launcher_uuid(value) {
            return ClientIdentity::with_profile_id(args.username.clone(), profile_id);
        }
        tracing::warn!(uuid = value, "Invalid UUID; using offline player UUID");
    }
    ClientIdentity::offline(args.username.clone())
}

/// Authlib 7.0.63 `UndashedUuid.fromStringLenient` first inserts the canonical
/// four dashes into a 32-character Java `\\w` value, then delegates to
/// `java.util.UUID.fromString`. JDK 25 also accepts shorter non-empty hex
/// groups and masks them to 32/16/16/16/48 bits, so reproduce that behavior
/// instead of relying on Rust UUID's stricter presentation parser.
fn parse_vanilla_launcher_uuid(value: &str) -> Option<Uuid> {
    let normalized = if value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        format!(
            "{}-{}-{}-{}-{}",
            &value[0..8],
            &value[8..12],
            &value[12..16],
            &value[16..20],
            &value[20..32]
        )
    } else {
        value.to_owned()
    };
    if normalized.len() > 36 {
        return None;
    }
    let groups = normalized.split('-').collect::<Vec<_>>();
    if groups.len() != 5 || groups.iter().any(|group| group.is_empty()) {
        return None;
    }
    let parse = |group: &str| {
        i64::from_str_radix(group.strip_prefix('+').unwrap_or(group), 16)
            .ok()
            .map(|value| value as u64)
    };
    let most = ((parse(groups[0])? & 0xffff_ffff) << 32)
        | ((parse(groups[1])? & 0xffff) << 16)
        | (parse(groups[2])? & 0xffff);
    let least = ((parse(groups[3])? & 0xffff) << 48) | (parse(groups[4])? & 0xffff_ffff_ffff);
    Some(Uuid::from_u128(
        (u128::from(most) << 64) | u128::from(least),
    ))
}

pub(crate) fn parse_args() -> Args {
    let matches = Args::command().get_matches();
    let mut args = match Args::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(error) => error.exit(),
    };
    apply_stored_client_options(&mut args, &matches);
    if let Err(message) = apply_vanilla_bootstrap_arguments(&mut args) {
        Args::command()
            .error(clap::error::ErrorKind::InvalidValue, message)
            .exit();
    }
    args
}

/// Collapse Vanilla's launcher spellings onto BBB's existing single-owner
/// external multiplayer startup contract. Stored fullscreen is the base;
/// the no-argument launch switch and quick-play address are explicit startup
/// overrides, matching `Main.main`'s argument-derived `GameConfig`.
fn apply_vanilla_bootstrap_arguments(args: &mut Args) -> std::result::Result<(), String> {
    if args.vanilla_fullscreen {
        args.client_fullscreen = true;
    }
    if let Some(server) = args.quick_play_multiplayer.as_ref() {
        args.server = unescape_java_argument(server)?;
        args.connect_server = true;
    }
    if args.client_fullscreen_resolution.is_none() {
        if let (Some(width), Some(height)) = (args.fullscreen_width, args.fullscreen_height) {
            // `Window.<init>` constructs this fallback only when both launch
            // dimensions exist; a stored `fullscreenResolution` wins first.
            args.client_fullscreen_resolution = Some(window_surface::PreferredVideoMode {
                width: width.max(0) as u32,
                height: height.max(0) as u32,
                refresh_rate_hz: 60,
                total_bits: 24,
            });
        }
    }
    Ok(())
}

/// Apache Commons Lang 3.19 `StringEscapeUtils.unescapeJava`, used by
/// `Main.getQuickPlayVariant`: Java control escapes, Unicode escapes (including
/// repeated `u`/optional `+`), octal escapes, and the Java rule that removes
/// the slash from an otherwise unknown escape.
fn unescape_java_argument(input: &str) -> std::result::Result<String, String> {
    let mut output = Vec::<u16>::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut units = [0_u16; 2];
            output.extend_from_slice(ch.encode_utf16(&mut units));
            continue;
        }
        let Some(next) = chars.next() else {
            break;
        };
        match next {
            'b' => output.push(u16::from(b'\x08')),
            'n' => output.push(u16::from(b'\n')),
            't' => output.push(u16::from(b'\t')),
            'f' => output.push(u16::from(b'\x0c')),
            'r' => output.push(u16::from(b'\r')),
            '\\' => output.push(u16::from(b'\\')),
            '"' => output.push(u16::from(b'"')),
            '\'' => output.push(u16::from(b'\'')),
            'u' => {
                while chars.next_if_eq(&'u').is_some() {}
                let _ = chars.next_if_eq(&'+');
                let digits: String = chars.by_ref().take(4).collect();
                if digits.chars().count() != 4 {
                    return Err("incomplete Unicode escape in --quickPlayMultiplayer".to_string());
                }
                let value = u16::from_str_radix(&digits, 16).map_err(|_| {
                    format!("invalid Unicode escape \\u{digits} in --quickPlayMultiplayer")
                })?;
                output.push(value);
            }
            digit @ '0'..='7' => {
                let mut digits = String::from(digit);
                let maximum = if digit <= '3' { 3 } else { 2 };
                while digits.len() < maximum {
                    let Some(candidate @ '0'..='7') = chars.peek().copied() else {
                        break;
                    };
                    digits.push(candidate);
                    chars.next();
                }
                output.push(u16::from_str_radix(&digits, 8).expect("octal digits are validated"));
            }
            other => {
                let mut units = [0_u16; 2];
                output.extend_from_slice(other.encode_utf16(&mut units));
            }
        }
    }
    String::from_utf16(&output)
        .map_err(|_| "unpaired UTF-16 surrogate in --quickPlayMultiplayer".to_string())
}

/// Vanilla reads these values from `options.txt` alone, so the file is the
/// base and only an argument the user actually typed overrides it. A missing
/// or unreadable file leaves every clap default in place.
fn apply_stored_client_options(args: &mut Args, matches: &ArgMatches) {
    let path = args.game_dir.join("options.txt");
    let host = ClientOptionHost {
        locale_is_japanese: crate::client_options::locale_is_japanese(
            sys_locale::get_locale().as_deref(),
        ),
        platform_is_windows: crate::platform::HOST_IS_WINDOWS,
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
    args.client_audio_volumes = stored_audio_volume_settings(&options);
    args.narrator_voice_volume = args
        .client_audio_volumes
        .final_source_volume(&bbb_audio::AudioCategory::Voice);
    args.stored_resource_packs = stored_text_list(&options, "resourcePacks");
    args.stored_incompatible_resource_packs =
        stored_text_list(&options, "incompatibleResourcePacks");
    bind_stored_client_options(args, StoredOptions::new(&options, matches));
    apply_stored_window_override(args, &options);
}

/// `Minecraft.<init>` applies `overrideWidth`/`overrideHeight` after Main's
/// display arguments and only when both are positive. These diagnostic launch
/// overrides therefore intentionally beat an explicit `--width/--height`,
/// unlike ordinary options mirrored by BBB startup flags.
fn apply_stored_window_override(args: &mut Args, options: &ClientOptions) {
    let override_width = options
        .raw("overrideWidth")
        .and_then(|raw| raw.parse::<i32>().ok());
    let override_height = options
        .raw("overrideHeight")
        .and_then(|raw| raw.parse::<i32>().ok());
    if let (Some(width @ 1..), Some(height @ 1..)) = (override_width, override_height) {
        args.window_width = width;
        args.window_height = height;
    }
}

fn stored_text_list(options: &ClientOptions, key: &'static str) -> Vec<String> {
    let Some(raw) = options.raw(key) else {
        return Vec::new();
    };
    match serde_json::from_str::<Option<Vec<String>>>(raw) {
        Ok(values) => values.unwrap_or_default(),
        Err(error) => {
            tracing::warn!(key, raw, %error, "ignoring invalid options.txt text list");
            Vec::new()
        }
    }
}

/// `soundDevice` is written through `Codec.STRING`, so unlike the raw `lang`
/// field its text is JSON quoted in `options.txt`. The empty decoded string is
/// the exact `Options.isSoundDeviceDefault` sentinel.
fn parse_stored_sound_device(value: &str) -> std::result::Result<String, String> {
    serde_json::from_str::<String>(value)
        .map_err(|error| format!("expected a JSON string: {error}"))
}

/// `GameNarrator.narrateMessage` speaks at
/// `getFinalSoundSourceVolume(SoundSource.VOICE)` — the stored voice volume
/// times the stored master volume, each defaulting to 1.0 and clamped to the
/// accepted 0..=1 range; an unparsable line keeps its default.
fn stored_audio_volume_settings(options: &ClientOptions) -> AudioVolumeSettings {
    let source_volume = |key: &str| {
        options
            .dynamic_raw(key)
            .and_then(|raw| raw.parse::<f32>().ok())
            .filter(|volume| volume.is_finite())
            .map(|volume| volume.clamp(0.0, 1.0))
            .unwrap_or(1.0)
    };
    AudioVolumeSettings {
        master: source_volume("soundCategory_master"),
        music: source_volume("soundCategory_music"),
        records: source_volume("soundCategory_record"),
        weather: source_volume("soundCategory_weather"),
        blocks: source_volume("soundCategory_block"),
        hostile: source_volume("soundCategory_hostile"),
        neutral: source_volume("soundCategory_neutral"),
        players: source_volume("soundCategory_player"),
        ambient: source_volume("soundCategory_ambient"),
        voice: source_volume("soundCategory_voice"),
        ui: source_volume("soundCategory_ui"),
    }
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
    flag!(
        "client_joined_first_server",
        "joinedFirstServer",
        client_joined_first_server
    );
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
    parsed!(
        "client_sound_device",
        "soundDevice",
        Text,
        client_sound_device,
        parse_stored_sound_device
    );
    flag!(
        "client_directional_audio",
        "directionalAudio",
        client_directional_audio
    );
    parsed!(
        "client_music_toast",
        "musicToast",
        EnumName,
        client_music_toast,
        parse_stored_music_toast
    );
    parsed!(
        "client_music_frequency",
        "musicFrequency",
        EnumName,
        client_music_frequency,
        parse_stored_music_frequency
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
        "client_japanese_glyph_variants",
        "japaneseGlyphVariants",
        client_japanese_glyph_variants
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
        "graphics_preset",
        "graphicsPreset",
        EnumName,
        graphics_preset,
        parse_stored_graphics_preset
    );
    parsed!(
        "biome_blend_radius",
        "biomeBlendRadius",
        Integer,
        biome_blend_radius,
        parse_biome_blend_radius
    );
    parsed!(
        "prioritize_chunk_updates",
        "prioritizeChunkUpdates",
        EnumId,
        prioritize_chunk_updates,
        parse_stored_prioritize_chunk_updates
    );
    parsed!(
        "client_simulation_distance_chunks",
        "simulationDistance",
        Integer,
        client_simulation_distance_chunks,
        parse_client_simulation_distance
    );
    parsed!(
        "client_mipmap_levels",
        "mipmapLevels",
        Integer,
        client_mipmap_levels,
        parse_client_mipmap_levels
    );
    parsed!(
        "client_entity_distance_scaling",
        "entityDistanceScaling",
        Double,
        client_entity_distance_scaling,
        parse_client_entity_distance_scaling
    );
    parsed!(
        "client_weather_radius",
        "weatherRadius",
        Integer,
        client_weather_radius,
        parse_client_weather_radius
    );
    parsed!(
        "client_particles",
        "particles",
        EnumId,
        client_particles,
        parse_stored_particle_status
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
        "terrain_texture_filtering",
        "textureFiltering",
        EnumId,
        terrain_texture_filtering,
        parse_stored_terrain_texture_filtering
    );
    parsed!(
        "client_max_anisotropy_bit",
        "maxAnisotropyBit",
        Integer,
        client_max_anisotropy_bit,
        parse_client_max_anisotropy_bit
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

    apply_graphics_preset(args, &stored);

    stored.bound()
}

/// `Minecraft` invokes `Options.applyGraphicsPreset` after the window and
/// input owners exist. During the earlier `Options.load`, `Minecraft.running`
/// is still false, so `OptionInstance.set` stores raw values without invoking
/// callbacks; this final application therefore intentionally overwrites every
/// preset-owned stored value. BBB's startup-only CLI is an additional source,
/// and an argument the user actually typed remains the final override.
fn apply_graphics_preset(args: &mut Args, stored: &StoredOptions<'_>) {
    macro_rules! preset_value {
        ($arg_id:literal, $field:ident, $value:expr) => {
            if !stored.is_explicit($arg_id) {
                args.$field = $value;
            }
        };
    }

    let preset = args.graphics_preset;
    if preset == GraphicsPresetArg::Custom {
        args.fabulous_texture_filtering = false;
        return;
    }

    match preset {
        GraphicsPresetArg::Fast => {
            preset_value!("biome_blend_radius", biome_blend_radius, 1);
            preset_value!("render_distance_chunks", render_distance_chunks, 8);
            preset_value!(
                "prioritize_chunk_updates",
                prioritize_chunk_updates,
                PrioritizeChunkUpdatesArg::None
            );
            preset_value!(
                "client_simulation_distance_chunks",
                client_simulation_distance_chunks,
                6
            );
            preset_value!("client_smooth_lighting", client_smooth_lighting, false);
            preset_value!("render_clouds", render_clouds, RenderCloudsArg::Fast);
            preset_value!(
                "client_particles",
                client_particles,
                ClientParticleStatusArg::Decreased
            );
            preset_value!("client_mipmap_levels", client_mipmap_levels, 2);
            preset_value!("entity_shadows", entity_shadows, false);
            preset_value!(
                "client_entity_distance_scaling",
                client_entity_distance_scaling,
                0.75
            );
            preset_value!("menu_background_blurriness", menu_background_blurriness, 2);
            preset_value!("cloud_range_chunks", cloud_range_chunks, 32);
            preset_value!("cutout_leaves", cutout_leaves, false);
            preset_value!("improved_transparency", improved_transparency, false);
            preset_value!("client_weather_radius", client_weather_radius, 5);
            preset_value!("client_max_anisotropy_bit", client_max_anisotropy_bit, 1);
            preset_value!(
                "terrain_texture_filtering",
                terrain_texture_filtering,
                TerrainTextureFilteringArg::None
            );
            args.fabulous_texture_filtering = false;
        }
        GraphicsPresetArg::Fancy => {
            preset_value!("biome_blend_radius", biome_blend_radius, 2);
            preset_value!("render_distance_chunks", render_distance_chunks, 16);
            preset_value!(
                "prioritize_chunk_updates",
                prioritize_chunk_updates,
                PrioritizeChunkUpdatesArg::PlayerAffected
            );
            preset_value!(
                "client_simulation_distance_chunks",
                client_simulation_distance_chunks,
                12
            );
            preset_value!("client_smooth_lighting", client_smooth_lighting, true);
            preset_value!("render_clouds", render_clouds, RenderCloudsArg::Fancy);
            preset_value!(
                "client_particles",
                client_particles,
                ClientParticleStatusArg::All
            );
            preset_value!("client_mipmap_levels", client_mipmap_levels, 4);
            preset_value!("entity_shadows", entity_shadows, true);
            preset_value!(
                "client_entity_distance_scaling",
                client_entity_distance_scaling,
                1.0
            );
            preset_value!("menu_background_blurriness", menu_background_blurriness, 5);
            preset_value!("cloud_range_chunks", cloud_range_chunks, 64);
            preset_value!("cutout_leaves", cutout_leaves, true);
            preset_value!("improved_transparency", improved_transparency, false);
            preset_value!("client_weather_radius", client_weather_radius, 10);
            preset_value!("client_max_anisotropy_bit", client_max_anisotropy_bit, 1);
            preset_value!(
                "terrain_texture_filtering",
                terrain_texture_filtering,
                TerrainTextureFilteringArg::Rgss
            );
            args.fabulous_texture_filtering = false;
        }
        GraphicsPresetArg::Fabulous => {
            preset_value!("biome_blend_radius", biome_blend_radius, 2);
            preset_value!("render_distance_chunks", render_distance_chunks, 32);
            preset_value!(
                "prioritize_chunk_updates",
                prioritize_chunk_updates,
                PrioritizeChunkUpdatesArg::PlayerAffected
            );
            preset_value!(
                "client_simulation_distance_chunks",
                client_simulation_distance_chunks,
                12
            );
            preset_value!("client_smooth_lighting", client_smooth_lighting, true);
            preset_value!("render_clouds", render_clouds, RenderCloudsArg::Fancy);
            preset_value!(
                "client_particles",
                client_particles,
                ClientParticleStatusArg::All
            );
            preset_value!("client_mipmap_levels", client_mipmap_levels, 4);
            preset_value!("entity_shadows", entity_shadows, true);
            preset_value!(
                "client_entity_distance_scaling",
                client_entity_distance_scaling,
                1.25
            );
            preset_value!("menu_background_blurriness", menu_background_blurriness, 5);
            preset_value!("cloud_range_chunks", cloud_range_chunks, 128);
            preset_value!("cutout_leaves", cutout_leaves, true);
            preset_value!(
                "improved_transparency",
                improved_transparency,
                !cfg!(target_os = "macos")
            );
            preset_value!("client_weather_radius", client_weather_radius, 10);
            preset_value!("client_max_anisotropy_bit", client_max_anisotropy_bit, 2);
            preset_value!(
                "terrain_texture_filtering",
                terrain_texture_filtering,
                TerrainTextureFilteringArg::Anisotropic
            );
            args.fabulous_texture_filtering = !stored.is_explicit("terrain_texture_filtering");
        }
        GraphicsPresetArg::Custom => unreachable!("handled above"),
    }
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

    let identity = client_identity_from_args(args);
    let mut options = connection_options_from_args(args, &identity)?;
    let roots = load_pack_roots(args)?;
    options.known_packs_manager = load_known_packs_manager(roots.as_ref());
    options.client_information = client_information_from_args(args)?;
    options.probe_after_first_chunk_packets = args.probe_after_first_chunk_packets;
    let report = runtime.block_on(bbb_net::run_probe(options))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(true)
}

pub(crate) fn load_pack_roots(args: &Args) -> Result<Option<PackRoots>> {
    Ok(load_pack_roots_with_options(args)?.roots)
}

pub(crate) struct LoadedPackRoots {
    pub(crate) roots: Option<PackRoots>,
    pub(crate) options_reconciliation: Option<bbb_pack::ClientPackOptionsReconciliation>,
}

pub(crate) fn load_pack_roots_with_options(args: &Args) -> Result<LoadedPackRoots> {
    let Some(vanilla_root) = args.vanilla_root.as_ref() else {
        tracing::warn!("--vanilla-root was not supplied; Vanilla resources are unavailable");
        return Ok(LoadedPackRoots {
            roots: None,
            options_reconciliation: None,
        });
    };
    match PackRoots::from_root(vanilla_root) {
        Ok(roots) if !args.resource_pack_dirs.is_empty() => Ok(LoadedPackRoots {
            roots: Some(apply_resource_pack_dirs(
                roots,
                args.resource_pack_dirs.iter().cloned(),
            )?),
            options_reconciliation: None,
        }),
        Ok(roots) => {
            let reconciliation = bbb_pack::SelectedClientPackStack::from_options_repository(
                &roots,
                &args.game_dir.join("resourcepacks"),
                &args.stored_resource_packs,
                &args.stored_incompatible_resource_packs,
            )?;
            for id in &reconciliation.removed_missing {
                tracing::warn!(pack_id = id, "removed missing resource pack from options");
            }
            for id in &reconciliation.removed_incompatible {
                tracing::warn!(
                    pack_id = id,
                    "removed newly incompatible resource pack from options"
                );
            }
            for id in &reconciliation.became_compatible {
                tracing::info!(
                    pack_id = id,
                    "removed resource pack from incompatibility list"
                );
            }
            let selected = reconciliation.selected_packs.clone();
            Ok(LoadedPackRoots {
                roots: Some(roots.with_selected_client_packs(selected)?),
                options_reconciliation: Some(reconciliation),
            })
        }
        Err(err) => {
            tracing::warn!(?err, "vanilla 26.1 pack roots unavailable");
            Ok(LoadedPackRoots {
                roots: None,
                options_reconciliation: None,
            })
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
    identity: &ClientIdentity,
    code_of_conduct: &mut CodeOfConductAcceptance,
    known_packs_manager: Option<Arc<KnownPacksManager>>,
    simulation_mailbox: &SimulationMailbox,
) -> Result<NetworkRuntime> {
    if !args.connect_server {
        return Ok(NetworkRuntime::idle());
    }

    let mut options = connection_options_from_args(args, identity)?;
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

fn connection_options_from_args(
    args: &Args,
    identity: &ClientIdentity,
) -> Result<ConnectionOptions> {
    let mut options = ConnectionOptions::offline_with_identity(&args.server, identity)?;
    options.server_resource_pack_policy = args.server_resource_pack_policy.into();
    Ok(options)
}

pub(crate) fn server_resource_pack_proxy_from_args(args: &Args) -> Option<ServerPackDownloadProxy> {
    let host = args.proxy_host.as_ref()?;
    ServerPackDownloadProxy::from_vanilla_args(
        host.clone(),
        args.proxy_port,
        args.proxy_user.clone(),
        args.proxy_pass
            .as_ref()
            .map(|password| password.expose().to_string()),
    )
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

fn parse_stored_music_toast(value: &str) -> std::result::Result<MusicToastDisplayStateArg, String> {
    match value.trim_matches('"') {
        "never" => Ok(MusicToastDisplayStateArg::Never),
        "pause" => Ok(MusicToastDisplayStateArg::Pause),
        "pause_and_toast" => Ok(MusicToastDisplayStateArg::PauseAndToast),
        other => Err(format!(
            "music toast must be never, pause or pause_and_toast, got {other}"
        )),
    }
}

fn parse_stored_music_frequency(value: &str) -> std::result::Result<MusicFrequencyArg, String> {
    match value.trim_matches('"') {
        "DEFAULT" => Ok(MusicFrequencyArg::Default),
        "FREQUENT" => Ok(MusicFrequencyArg::Frequent),
        "CONSTANT" => Ok(MusicFrequencyArg::Constant),
        other => Err(format!(
            "music frequency must be DEFAULT, FREQUENT or CONSTANT, got {other}"
        )),
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

fn parse_client_entity_distance_scaling(value: &str) -> std::result::Result<f64, String> {
    let scaling = value
        .parse::<f64>()
        .map_err(|_| format!("entity distance scaling must be a number: {value}"))?;
    if scaling.is_finite() && (0.5..=5.0).contains(&scaling) {
        Ok(scaling)
    } else {
        Err("entity distance scaling must be between 0.5 and 5.0".to_owned())
    }
}

fn parse_stored_graphics_preset(value: &str) -> std::result::Result<GraphicsPresetArg, String> {
    match value.trim_matches('"') {
        "fast" => Ok(GraphicsPresetArg::Fast),
        "fancy" => Ok(GraphicsPresetArg::Fancy),
        "fabulous" => Ok(GraphicsPresetArg::Fabulous),
        "custom" => Ok(GraphicsPresetArg::Custom),
        other => Err(format!(
            "graphics preset must be fast, fancy, fabulous or custom, got {other}"
        )),
    }
}

fn parse_stored_prioritize_chunk_updates(
    value: &str,
) -> std::result::Result<PrioritizeChunkUpdatesArg, String> {
    let id = value
        .trim_matches('"')
        .parse::<i64>()
        .map_err(|_| format!("chunk update priority must be an integer id, got {value}"))?;
    Ok(match id.rem_euclid(3) {
        0 => PrioritizeChunkUpdatesArg::None,
        1 => PrioritizeChunkUpdatesArg::PlayerAffected,
        _ => PrioritizeChunkUpdatesArg::Nearby,
    })
}

fn parse_stored_particle_status(
    value: &str,
) -> std::result::Result<ClientParticleStatusArg, String> {
    let id = value
        .trim_matches('"')
        .parse::<i64>()
        .map_err(|_| format!("particle status must be an integer id, got {value}"))?;
    Ok(match id.rem_euclid(3) {
        0 => ClientParticleStatusArg::All,
        1 => ClientParticleStatusArg::Decreased,
        _ => ClientParticleStatusArg::Minimal,
    })
}

fn parse_biome_blend_radius(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 0, 7, "biome blend radius")
}

fn parse_client_simulation_distance(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 5, 32, "client simulation distance")
}

fn parse_client_mipmap_levels(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 0, 4, "mipmap levels")
}

fn parse_client_weather_radius(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 3, 10, "weather radius")
}

fn parse_bounded_u32(
    value: &str,
    minimum: u32,
    maximum: u32,
    label: &str,
) -> std::result::Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{label} must be an integer: {value}"))?;
    if (minimum..=maximum).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!(
            "{label} must be between {minimum} and {maximum}, got {parsed}"
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
fn parse_stored_terrain_texture_filtering(
    value: &str,
) -> std::result::Result<TerrainTextureFilteringArg, String> {
    // Vanilla `TextureFilteringMethod.LEGACY_CODEC` reads the stored int via
    // `ByIdMap.continuous(..., OutOfBoundsStrategy.WRAP)`, so out-of-range ids
    // wrap modulo the three variants instead of failing.
    let id = value
        .trim_matches('"')
        .parse::<i64>()
        .map_err(|_| format!("texture filtering must be an integer id, got {value}"))?;
    Ok(match id.rem_euclid(3) {
        0 => TerrainTextureFilteringArg::None,
        1 => TerrainTextureFilteringArg::Rgss,
        _ => TerrainTextureFilteringArg::Anisotropic,
    })
}

fn parse_client_max_anisotropy_bit(value: &str) -> std::result::Result<u32, String> {
    // Vanilla `maxAnisotropyBit` is `Codec.intRange(1, 3)` via
    // `OptionInstance.IntRange`; out-of-range stored values fail codec
    // validation and keep the default.
    let bit = value
        .parse::<u32>()
        .map_err(|_| format!("max anisotropy bit must be an integer: {value}"))?;
    if (1..=3).contains(&bit) {
        Ok(bit)
    } else {
        Err(format!("max anisotropy bit must be 1, 2 or 3, got {bit}"))
    }
}

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
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "bbb-native".to_string(),
            width: 854,
            height: 480,
        }
    }
}

impl WindowConfig {
    pub(crate) fn from_args(args: &Args) -> Self {
        Self {
            width: args.window_width,
            height: args.window_height,
            ..Self::default()
        }
    }

    pub(crate) fn physical_size(&self) -> PhysicalSize<u32> {
        // `Window.allowedWindowMinSize` (Window.java:551) clamps every stored
        // window dimension to at least one, so a degenerate display size can
        // never construct a zero-width native window.
        PhysicalSize::new(
            crate::window_surface::allowed_window_min_size(self.width.max(0) as u32),
            crate::window_surface::allowed_window_min_size(self.height.max(0) as u32),
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
pub(crate) fn build_window(
    event_loop: &EventLoop<()>,
    config: &WindowConfig,
    initial_fullscreen: Option<Fullscreen>,
) -> Result<Window> {
    WindowBuilder::new()
        .with_title(config.title.clone())
        .with_inner_size(config.physical_size())
        .with_fullscreen(initial_fullscreen)
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

    #[cfg(target_os = "linux")]
    #[test]
    fn render_thread_name_is_visible_to_linux_profilers() {
        name_current_render_thread().unwrap();
        assert_eq!(
            std::fs::read_to_string("/proc/thread-self/comm")
                .unwrap()
                .trim_end(),
            "Render thread"
        );
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
        let default_identity = client_identity_from_args(&defaults);
        assert_eq!(
            defaults.server_resource_pack_policy,
            ServerResourcePackPolicyArg::Decline
        );
        assert_eq!(defaults.server_resource_pack_cache_dir, None);
        assert_eq!(defaults.proxy_host, None);
        assert_eq!(defaults.proxy_port, 8080);
        assert!(server_resource_pack_proxy_from_args(&defaults).is_none());
        assert_eq!(
            connection_options_from_args(&defaults, &default_identity)
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
        let accepted_identity = client_identity_from_args(&accepted);
        assert_eq!(
            accepted.server_resource_pack_policy,
            ServerResourcePackPolicyArg::Accept
        );
        assert_eq!(
            accepted.server_resource_pack_cache_dir,
            Some(PathBuf::from("/tmp/bbb-server-packs"))
        );
        assert_eq!(
            connection_options_from_args(&accepted, &accepted_identity)
                .unwrap()
                .server_resource_pack_policy,
            ServerResourcePackPolicy::Accept
        );
    }

    #[test]
    fn args_project_vanilla_socks_proxy_without_exposing_password() {
        let args = Args::try_parse_from([
            "bbb-native",
            "--proxyHost",
            "proxy.example",
            "--proxyPort",
            "1080",
            "--proxyUser",
            "proxy-user",
            "--proxyPass",
            "proxy-password",
        ])
        .unwrap();

        assert_eq!(args.proxy_host.as_deref(), Some("proxy.example"));
        assert_eq!(args.proxy_port, 1080);
        assert!(server_resource_pack_proxy_from_args(&args).is_some());
        let debug = format!("{args:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("proxy-password"));

        let invalid_port = Args::try_parse_from([
            "bbb-native",
            "--proxy-host",
            "proxy.example",
            "--proxy-port",
            "65536",
        ])
        .unwrap();
        assert!(server_resource_pack_proxy_from_args(&invalid_port).is_none());
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
    fn startup_music_options_use_vanilla_defaults_and_cli_names() {
        let defaults = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(
            defaults.client_music_toast,
            MusicToastDisplayStateArg::Never
        );
        assert_eq!(defaults.client_music_frequency, MusicFrequencyArg::Default);

        let configured = Args::try_parse_from([
            "bbb-native",
            "--client-music-toast",
            "pause-and-toast",
            "--client-music-frequency",
            "constant",
        ])
        .unwrap();
        assert_eq!(
            configured.client_music_toast,
            MusicToastDisplayStateArg::PauseAndToast
        );
        assert_eq!(
            configured.client_music_frequency,
            MusicFrequencyArg::Constant
        );
    }

    #[test]
    fn stored_music_options_reach_startup_and_explicit_cli_wins() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("musicToast", r#""pause_and_toast""#);
        options.set_raw_for_test("musicFrequency", r#""FREQUENT""#);

        let mut stored_args = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(
            &mut stored_args,
            StoredOptions::without_command_line(&options),
        );
        assert_eq!(
            stored_args.client_music_toast,
            MusicToastDisplayStateArg::PauseAndToast
        );
        assert_eq!(
            stored_args.client_music_frequency,
            MusicFrequencyArg::Frequent
        );

        let matches = Args::command().get_matches_from([
            "bbb-native",
            "--client-music-toast",
            "pause",
            "--client-music-frequency",
            "constant",
        ]);
        let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
        bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
        assert_eq!(
            explicit_args.client_music_toast,
            MusicToastDisplayStateArg::Pause
        );
        assert_eq!(
            explicit_args.client_music_frequency,
            MusicFrequencyArg::Constant
        );
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
    fn joined_first_server_is_consumed_at_startup_and_explicit_cli_wins() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("joinedFirstServer", "true");

        let mut stored_args = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(
            &mut stored_args,
            StoredOptions::without_command_line(&options),
        );
        assert!(stored_args.client_joined_first_server);

        let matches = Args::command().get_matches_from([
            "bbb-native",
            "--client-joined-first-server",
            "false",
        ]);
        let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
        bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
        assert!(!explicit_args.client_joined_first_server);
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

        let anisotropic =
            Args::try_parse_from(["bbb-native", "--terrain-texture-filtering", "anisotropic"])
                .unwrap();
        assert_eq!(
            TerrainTextureFiltering::from(anisotropic.terrain_texture_filtering),
            TerrainTextureFiltering::Anisotropic
        );
    }

    #[test]
    fn max_anisotropy_bit_matches_the_vanilla_int_range() {
        // Vanilla `maxAnisotropyBit` is `OptionInstance.IntRange(1, 3)` with
        // default 2.
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert_eq!(default_args.client_max_anisotropy_bit, 2);
        let three =
            Args::try_parse_from(["bbb-native", "--client-max-anisotropy-bit", "3"]).unwrap();
        assert_eq!(three.client_max_anisotropy_bit, 3);
        assert!(Args::try_parse_from(["bbb-native", "--client-max-anisotropy-bit", "0"]).is_err());
        assert!(Args::try_parse_from(["bbb-native", "--client-max-anisotropy-bit", "4"]).is_err());

        // Stored `maxAnisotropyBit` outside the codec range fails validation
        // and keeps the default, like vanilla's `OptionInstance` read.
        assert_eq!(parse_client_max_anisotropy_bit("1"), Ok(1));
        assert!(parse_client_max_anisotropy_bit("0").is_err());
        assert!(parse_client_max_anisotropy_bit("4").is_err());
        assert!(parse_client_max_anisotropy_bit("x").is_err());
    }

    #[test]
    fn stored_texture_filtering_wraps_ids_like_the_vanilla_legacy_codec() {
        // `TextureFilteringMethod.LEGACY_CODEC` resolves ids through
        // `ByIdMap.continuous(..., OutOfBoundsStrategy.WRAP)` (floorMod).
        assert_eq!(
            parse_stored_terrain_texture_filtering("0"),
            Ok(TerrainTextureFilteringArg::None)
        );
        assert_eq!(
            parse_stored_terrain_texture_filtering("1"),
            Ok(TerrainTextureFilteringArg::Rgss)
        );
        assert_eq!(
            parse_stored_terrain_texture_filtering("2"),
            Ok(TerrainTextureFilteringArg::Anisotropic)
        );
        assert_eq!(
            parse_stored_terrain_texture_filtering("3"),
            Ok(TerrainTextureFilteringArg::None)
        );
        assert_eq!(
            parse_stored_terrain_texture_filtering("-1"),
            Ok(TerrainTextureFilteringArg::Anisotropic)
        );
        assert!(parse_stored_terrain_texture_filtering("fancy").is_err());
    }

    #[test]
    fn terrain_atlas_base_padding_matches_the_vanilla_stitcher_formula() {
        // Vanilla `Stitcher`: padding = `1 << mipLevel << clamp(bit - 1, 0,
        // 4)` with bit 0 outside `ANISOTROPIC`; the packer multiplies the
        // base by `1 << mipLevel`, so the base is the anisotropy factor.
        assert_eq!(
            terrain_atlas_base_padding(TerrainTextureFilteringArg::None, 3),
            1
        );
        assert_eq!(
            terrain_atlas_base_padding(TerrainTextureFilteringArg::Rgss, 3),
            1
        );
        assert_eq!(
            terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 1),
            1
        );
        assert_eq!(
            terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 2),
            2
        );
        assert_eq!(
            terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 3),
            4
        );
        assert_eq!(
            terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 9),
            16
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
    fn stored_resource_pack_lists_reconcile_the_game_directory_repository() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!(
            "bbb-startup-options-resource-packs-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(temp.join("vanilla/sources/26.1/assets/minecraft")).unwrap();
        let selected = temp.join("game/resourcepacks/selected");
        let rejected = temp.join("game/resourcepacks/rejected");
        for (pack, format) in [(&selected, 84), (&rejected, 63)] {
            std::fs::create_dir_all(pack).unwrap();
            let metadata = if format > 64 {
                format!(
                    r#"{{"pack":{{"description":"test","min_format":[{format},0],"max_format":[{format},0]}}}}"#
                )
            } else {
                format!(r#"{{"pack":{{"description":"test","pack_format":{format}}}}}"#)
            };
            std::fs::write(pack.join("pack.mcmeta"), metadata).unwrap();
        }
        let mut args = Args::parse_from(["bbb-native"]);
        args.vanilla_root = Some(temp.join("vanilla"));
        args.game_dir = temp.join("game");
        args.stored_resource_packs = vec!["selected".to_owned(), "file/rejected".to_owned()];

        let loaded = load_pack_roots_with_options(&args).unwrap();

        assert_eq!(
            loaded
                .roots
                .as_ref()
                .unwrap()
                .selected_client_packs()
                .unwrap()
                .packs()
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["vanilla", "file/selected"]
        );
        let reconciliation = loaded.options_reconciliation.unwrap();
        assert_eq!(reconciliation.resource_packs, vec!["selected"]);
        assert_eq!(reconciliation.removed_incompatible, vec!["file/rejected"]);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn stored_text_lists_decode_json_strings_and_fail_closed() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("resourcePacks", r#"["one","file/two"]"#);
        options.set_raw_for_test("incompatibleResourcePacks", "null");

        assert_eq!(
            stored_text_list(&options, "resourcePacks"),
            vec!["one", "file/two"]
        );
        assert!(stored_text_list(&options, "incompatibleResourcePacks").is_empty());

        options.set_raw_for_test("resourcePacks", r#"["one",7]"#);
        assert!(stored_text_list(&options, "resourcePacks").is_empty());
    }

    #[test]
    fn stored_sound_device_decodes_codec_string_and_explicit_cli_wins() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("soundDevice", r#""OpenAL Soft on USB Headset""#);

        let mut stored_args = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(
            &mut stored_args,
            StoredOptions::without_command_line(&options),
        );
        assert_eq!(
            stored_args.client_sound_device,
            "OpenAL Soft on USB Headset"
        );

        let matches = Args::command().get_matches_from([
            "bbb-native",
            "--client-sound-device",
            "Built-in Audio",
        ]);
        let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
        bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
        assert_eq!(explicit_args.client_sound_device, "Built-in Audio");
    }

    #[test]
    fn invalid_stored_sound_device_keeps_the_default_sentinel() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("soundDevice", "not-json");
        let mut args = Args::parse_from(["bbb-native"]);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert!(args.client_sound_device.is_empty());
    }

    #[test]
    fn stored_directional_audio_reaches_startup_and_explicit_cli_wins() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("directionalAudio", "true");
        let mut stored_args = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(
            &mut stored_args,
            StoredOptions::without_command_line(&options),
        );
        assert!(stored_args.client_directional_audio);

        let matches =
            Args::command().get_matches_from(["bbb-native", "--client-directional-audio", "false"]);
        let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
        bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
        assert!(!explicit_args.client_directional_audio);
    }

    #[test]
    fn stored_audio_volumes_cover_every_vanilla_source_with_safe_defaults() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        assert_eq!(
            stored_audio_volume_settings(&options),
            AudioVolumeSettings::default()
        );

        options.set_dynamic_raw_for_test("soundCategory_voice", "0.5");
        options.set_dynamic_raw_for_test("soundCategory_master", "0.5");
        options.set_dynamic_raw_for_test("soundCategory_music", "0.25");
        options.set_dynamic_raw_for_test("soundCategory_record", "0.75");
        options.set_dynamic_raw_for_test("soundCategory_weather", "0.1");
        options.set_dynamic_raw_for_test("soundCategory_block", "0.2");
        options.set_dynamic_raw_for_test("soundCategory_hostile", "0.3");
        options.set_dynamic_raw_for_test("soundCategory_neutral", "0.4");
        options.set_dynamic_raw_for_test("soundCategory_player", "0.6");
        options.set_dynamic_raw_for_test("soundCategory_ambient", "0.7");
        options.set_dynamic_raw_for_test("soundCategory_ui", "0.8");
        assert_eq!(
            stored_audio_volume_settings(&options),
            AudioVolumeSettings {
                master: 0.5,
                music: 0.25,
                records: 0.75,
                weather: 0.1,
                blocks: 0.2,
                hostile: 0.3,
                neutral: 0.4,
                players: 0.6,
                ambient: 0.7,
                voice: 0.5,
                ui: 0.8,
            }
        );

        options.set_dynamic_raw_for_test("soundCategory_master", "nonsense");
        assert_eq!(stored_audio_volume_settings(&options).master, 1.0);

        options.set_dynamic_raw_for_test("soundCategory_voice", "7.5");
        options.set_dynamic_raw_for_test("soundCategory_master", "0.5");
        let settings = stored_audio_volume_settings(&options);
        assert_eq!(settings.voice, 1.0);
        assert_eq!(
            settings.final_source_volume(&bbb_audio::AudioCategory::Voice),
            0.5
        );
    }

    #[test]
    fn stored_options_fill_arguments_the_user_did_not_type() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("graphicsPreset", "\"custom\"");
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

    fn graphics_options_snapshot(args: &Args) -> String {
        format!(
            "{:?}|{}|{}|{:?}|{}|{}|{:?}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}",
            args.graphics_preset,
            args.biome_blend_radius,
            args.render_distance_chunks,
            args.prioritize_chunk_updates,
            args.client_simulation_distance_chunks,
            args.client_smooth_lighting,
            args.render_clouds,
            args.client_particles,
            args.client_mipmap_levels,
            args.entity_shadows,
            args.client_entity_distance_scaling,
            args.menu_background_blurriness,
            args.cloud_range_chunks,
            args.cutout_leaves,
            args.improved_transparency,
            args.client_weather_radius,
            args.terrain_texture_filtering,
            args.client_max_anisotropy_bit,
        )
    }

    #[test]
    fn stored_graphics_presets_overwrite_every_owned_value_after_load() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());

        options.set_raw_for_test("graphicsPreset", "\"fast\"");
        let mut fast = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(&mut fast, StoredOptions::without_command_line(&options));
        assert_eq!(
            graphics_options_snapshot(&fast),
            "Fast|1|8|None|6|false|Fast|Decreased|2|false|0.75|2|32|false|false|5|None|1"
        );
        assert!(!fast.fabulous_texture_filtering);

        options.set_raw_for_test("graphicsPreset", "\"fancy\"");
        let mut fancy = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(&mut fancy, StoredOptions::without_command_line(&options));
        assert_eq!(
            graphics_options_snapshot(&fancy),
            "Fancy|2|16|PlayerAffected|12|true|Fancy|All|4|true|1|5|64|true|false|10|Rgss|1"
        );
        assert!(!fancy.fabulous_texture_filtering);

        options.set_raw_for_test("graphicsPreset", "\"fabulous\"");
        let mut fabulous = Args::parse_from(["bbb-native"]);
        bind_stored_client_options(&mut fabulous, StoredOptions::without_command_line(&options));
        assert_eq!(
            graphics_options_snapshot(&fabulous),
            format!(
                "Fabulous|2|32|PlayerAffected|12|true|Fancy|All|4|true|1.25|5|128|true|{}|10|Anisotropic|2",
                !cfg!(target_os = "macos")
            )
        );
        assert!(fabulous.fabulous_texture_filtering);
    }

    #[test]
    fn custom_graphics_preset_preserves_the_individually_stored_values() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("graphicsPreset", "\"custom\"");
        options.set_raw_for_test("renderDistance", "20");
        options.set_raw_for_test("particles", "2");
        options.set_raw_for_test("textureFiltering", "2");
        let mut args = Args::parse_from(["bbb-native"]);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert_eq!(args.graphics_preset, GraphicsPresetArg::Custom);
        assert_eq!(args.render_distance_chunks, 20);
        assert_eq!(args.client_particles, ClientParticleStatusArg::Minimal);
        assert_eq!(
            args.terrain_texture_filtering,
            TerrainTextureFilteringArg::Anisotropic
        );
        assert!(!args.fabulous_texture_filtering);
    }

    #[test]
    fn typed_graphics_values_override_a_typed_fabulous_preset() {
        let options = ClientOptions::defaults(ClientOptionHost::default());
        let matches = Args::command().get_matches_from([
            "bbb-native",
            "--graphics-preset",
            "fabulous",
            "--render-distance",
            "20",
            "--terrain-texture-filtering",
            "none",
        ]);
        let mut args = Args::from_arg_matches(&matches).unwrap();

        bind_stored_client_options(&mut args, StoredOptions::new(&options, &matches));

        assert_eq!(args.graphics_preset, GraphicsPresetArg::Fabulous);
        assert_eq!(args.render_distance_chunks, 20);
        assert_eq!(
            args.terrain_texture_filtering,
            TerrainTextureFilteringArg::None
        );
        assert_eq!(args.cloud_range_chunks, 128);
        assert_eq!(args.client_entity_distance_scaling, 1.25);
        assert!(!args.fabulous_texture_filtering);
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
        options.set_raw_for_test("japaneseGlyphVariants", "true");
        options.set_raw_for_test("rawMouseInput", "false");
        options.set_raw_for_test("allowCursorChanges", "false");
        options.set_raw_for_test("fullscreenResolution", "1920x1080@144:24");
        let mut args = Args::parse_from(["bbb-native"]);
        assert!(args.client_fullscreen_resolution.is_none());
        assert_eq!(args.client_gui_scale, 0);
        assert!(!args.client_force_unicode_font);
        assert!(!args.client_japanese_glyph_variants);
        assert!(args.client_raw_mouse_input);
        assert!(args.client_allow_cursor_changes);

        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

        assert_eq!(args.client_gui_scale, 3);
        assert!(args.client_force_unicode_font);
        assert!(args.client_japanese_glyph_variants);
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
        config.height = -10;
        assert_eq!(config.physical_size(), PhysicalSize::new(1, 1));

        let config = WindowConfig::default();
        assert_eq!(config.physical_size(), PhysicalSize::new(854, 480));
    }

    #[test]
    fn vanilla_bootstrap_arguments_feed_the_single_external_startup_path() {
        let mut args = Args::parse_from([
            "bbb-native",
            "--gameDir",
            "/tmp/vanilla-game",
            "--width",
            "1280",
            "--height",
            "720",
            "--fullscreen",
            "--quickPlayMultiplayer",
            "127.0.0.1:25570",
        ]);
        apply_vanilla_bootstrap_arguments(&mut args).unwrap();

        assert_eq!(args.game_dir, PathBuf::from("/tmp/vanilla-game"));
        assert_eq!(args.server, "127.0.0.1:25570");
        assert!(args.connect_server);
        assert!(args.client_fullscreen);
        assert_eq!(
            WindowConfig::from_args(&args).physical_size(),
            PhysicalSize::new(1280, 720)
        );
    }

    #[test]
    fn vanilla_launcher_uuid_is_lenient_and_one_identity_feeds_login() {
        let expected = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
        for value in [
            "00112233-4455-6677-8899-aabbccddeeff",
            "00112233445566778899aabbccddeeff",
        ] {
            let args =
                Args::parse_from(["bbb-native", "--username", "identity-test", "--uuid", value]);
            let identity = client_identity_from_args(&args);
            assert_eq!(identity.username, "identity-test");
            assert_eq!(identity.profile_id, expected);
            let connection = connection_options_from_args(&args, &identity).unwrap();
            assert_eq!(connection.username, identity.username);
            assert_eq!(connection.profile_id, identity.profile_id);
        }

        assert_eq!(
            parse_vanilla_launcher_uuid("1-2-3-4-5"),
            Some(Uuid::from_u128(0x00000001_0002_0003_0004_000000000005))
        );
    }

    #[test]
    fn invalid_launcher_uuid_falls_back_to_the_username_offline_identity() {
        let args = Args::parse_from([
            "bbb-native",
            "--username",
            "identity-test",
            "--uuid",
            "not-a-uuid",
        ]);
        let identity = client_identity_from_args(&args);

        assert_eq!(identity, ClientIdentity::offline("identity-test"));
        assert_eq!(parse_vanilla_launcher_uuid("not-a-uuid"), None);
        assert_eq!(parse_vanilla_launcher_uuid("_".repeat(32).as_str()), None);
    }

    #[test]
    fn vanilla_quick_play_unescapes_java_and_cli_fullscreen_dimensions_are_fallback_only() {
        assert_eq!(unescape_java_argument(r"\uu+0041\q").unwrap(), "Aq");
        assert_eq!(unescape_java_argument(r"\uD83D\uDE00").unwrap(), "😀");
        assert!(unescape_java_argument(r"\u12").is_err());
        let mut args = Args::parse_from([
            "bbb-native",
            "--quickPlayMultiplayer",
            r"example\u002eorg\07225565",
            "--fullscreenWidth",
            "1920",
            "--fullscreenHeight",
            "1080",
        ]);
        apply_vanilla_bootstrap_arguments(&mut args).unwrap();
        assert_eq!(args.server, "example.org:25565");
        assert_eq!(
            args.client_fullscreen_resolution.map(|mode| mode.write()),
            Some("1920x1080@60:24".to_string())
        );

        args.client_fullscreen_resolution =
            window_surface::PreferredVideoMode::parse("2560x1440@144:30");
        apply_vanilla_bootstrap_arguments(&mut args).unwrap();
        assert_eq!(
            args.client_fullscreen_resolution.map(|mode| mode.write()),
            Some("2560x1440@144:30".to_string())
        );
    }

    #[test]
    fn stored_positive_window_override_beats_main_display_arguments_only_as_a_pair() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("overrideWidth", "1600");
        options.set_raw_for_test("overrideHeight", "900");
        let mut args = Args::parse_from(["bbb-native", "--width", "1280", "--height", "720"]);

        apply_stored_window_override(&mut args, &options);
        assert_eq!(
            WindowConfig::from_args(&args).physical_size(),
            PhysicalSize::new(1600, 900)
        );

        options.set_raw_for_test("overrideHeight", "0");
        args.window_width = 1280;
        args.window_height = 720;
        apply_stored_window_override(&mut args, &options);
        assert_eq!(
            WindowConfig::from_args(&args).physical_size(),
            PhysicalSize::new(1280, 720)
        );
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
        options.set_raw_for_test("graphicsPreset", "\"custom\"");
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
