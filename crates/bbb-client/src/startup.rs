use std::{
    convert::Infallible, fmt, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc,
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::control::ControlState;
use anyhow::{ensure, Context, Result};
use bbb_client_assets::{KnownPacksManager, PackRoots};
use bbb_client_audio::AudioVolumeSettings;
use bbb_client_net::{ClientIdentity, ConnectionOptions, ServerPackDownloadProxy};
use bbb_client_render::{
    VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS, VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
    VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS, VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
    VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY, VANILLA_MAX_CLOUD_RANGE_CHUNKS,
    VANILLA_MAX_MENU_BACKGROUND_BLURRINESS, VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    VANILLA_MIN_CLOUD_RANGE_CHUNKS, VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
use bbb_protocol::packets::ClientInformation;
#[cfg(not(target_arch = "wasm32"))]
use clap::{ArgAction, ArgMatches, CommandFactory, FromArgMatches, Parser};
#[cfg(any(not(target_arch = "wasm32"), test))]
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
use uuid::Uuid;
#[cfg(any(not(target_arch = "wasm32"), test))]
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
#[cfg(not(target_arch = "wasm32"))]
use winit::{
    event_loop::ActiveEventLoop,
    window::{Fullscreen, Window},
};

use crate::camera_fov::{
    VANILLA_DEFAULT_CAMERA_FOV_DEGREES, VANILLA_DEFAULT_FOV_EFFECT_SCALE,
    VANILLA_MAX_CAMERA_FOV_DEGREES, VANILLA_MIN_CAMERA_FOV_DEGREES,
};
use crate::options::{
    args::{parse_stored_boolean, stored_fov_degrees, ClientOptionBinding, StoredOptions},
    key_bindings::KeyBindings,
    ClientOptionEncoding, ClientOptionHost, ClientOptions,
};
use crate::remote_debug::RemoteDebugRenderer;
use crate::window_surface;

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

macro_rules! define_startup_args {
    (
        $(
            $(#[doc = $doc:expr])*
            #[arg($($arg_meta:tt)*)]
            pub(crate) $field:ident : $ty:ty,
        )*
    ) => {
        #[derive(Debug)]
        #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
        #[cfg_attr(not(target_arch = "wasm32"), derive(Parser))]
        #[cfg_attr(not(target_arch = "wasm32"), command(name = "bbb-client"))]
        #[cfg_attr(
            not(target_arch = "wasm32"),
            command(group(clap::ArgGroup::new("control_endpoint").args(["control", "mcp_bind"]).multiple(true)))
        )]
        pub(crate) struct Args {
            $(
                $(#[doc = $doc])*
                #[cfg_attr(not(target_arch = "wasm32"), arg($($arg_meta)*))]
                pub(crate) $field : $ty,
            )*
        }
    };
}

define_startup_args! {
    /// Vanilla `--gameDir`: the directory `options.txt` is read from and
    /// written back to.
    #[arg(
        long = "game-dir",
        help_heading = "Connection & Identity",
        value_name = "PATH",
        default_value = "."
    )]
    pub(crate) game_dir: PathBuf,
    /// Managed cache root printed by `scripts/vanilla path 26.1 --kind root`.
    #[arg(
        long = "vanilla-root",
        help_heading = "Connection & Identity",
        value_name = "PATH"
    )]
    pub(crate) vanilla_root: Option<PathBuf>,
    /// Target Minecraft Java 26.1 offline server address.
    #[arg(
        long,
        help_heading = "Connection & Identity",
        default_value = "127.0.0.1:25565"
    )]
    pub(crate) server: String,
    /// Vanilla `Main --quickPlayMultiplayer`; normalized onto BBB's one
    /// external-server startup path after stored options have been bound.
    #[arg(
        long = "quick-play-multiplayer",
        help_heading = "Connection & Identity",
        value_name = "ADDR",
        conflicts_with_all = ["status_server", "probe_server"]
    )]
    pub(crate) quick_play_multiplayer: Option<String>,
    /// Offline player username.
    #[arg(
        long,
        help_heading = "Connection & Identity",
        default_value_t = default_desktop_username()
    )]
    pub(crate) username: String,
    /// Vanilla `Main --uuid`. Both 32-digit undashed and Java UUID group
    /// spellings are accepted; an invalid value warns and falls back to the
    /// username-derived offline UUID just like the official launcher owner.
    #[arg(
        long = "uuid",
        help_heading = "Connection & Identity",
        value_name = "UUID"
    )]
    pub(crate) uuid: Option<String>,
    /// Query server list ping status once and exit.
    #[arg(
        long,
        help_heading = "Connection & Identity",
        value_name = "ADDR",
        conflicts_with = "probe_server"
    )]
    pub(crate) status_server: Option<String>,
    /// Print build metadata and exit.
    #[arg(long, help_heading = "Connection & Identity")]
    pub(crate) build_info: bool,
    /// Run the headless protocol acceptance probe against `--server`.
    #[arg(long, help_heading = "Diagnostics & Probes", hide = true)]
    pub(crate) probe_server: bool,
    /// Number of chunk packets to wait for after initial login in headless probe mode.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        default_value_t = 0
    )]
    pub(crate) probe_after_first_chunk_packets: usize,
    /// Run a normalized creative chat/break/place/inventory replay after the
    /// first chunk. The target server must expose a nearby buildable surface.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        requires = "probe_server"
    )]
    pub(crate) probe_server_interactions: bool,
    /// Also run a second session that the target server already lists as an
    /// operator, and record what each command it runs answered with. The
    /// server must have been handed an `ops.json` naming the probe's operator
    /// username before it started, and the session changes server state, so
    /// only a harness that prepared the fixture and owns the world asks for it.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        requires = "probe_server_interactions"
    )]
    pub(crate) probe_server_operator: bool,
    /// Run the isolated cross-dimension server acceptance probe.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        requires = "probe_server",
        conflicts_with_all = [
            "probe_server_interactions",
            "probe_server_operator",
            "probe_after_first_chunk_packets",
            "probe_hold_seconds",
            "vanilla_root"
        ]
    )]
    pub(crate) probe_server_dimension_transitions: bool,
    /// Run the isolated staged entity-runtime acceptance replay.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        requires = "probe_server",
        conflicts_with_all = [
            "probe_server_interactions",
            "probe_server_operator",
            "probe_server_dimension_transitions",
            "probe_after_first_chunk_packets",
            "probe_hold_seconds",
            "vanilla_root"
        ]
    )]
    pub(crate) probe_server_entity_runtime: bool,
    /// Open a chest from its block, quick-move a stack through the menu and
    /// close it again, stepping a frozen server between the packets. Needs the
    /// same prepared `ops.json` the operator session needs, and the same
    /// username.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        requires = "probe_server",
        conflicts_with_all = [
            "probe_server_interactions",
            "probe_server_operator",
            "probe_server_dimension_transitions",
            "probe_server_entity_runtime",
            "probe_after_first_chunk_packets",
            "probe_hold_seconds",
            "vanilla_root"
        ]
    )]
    pub(crate) probe_server_container_menu: bool,
    /// Override the headless probe's base connection deadline; omitted keeps the default.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        value_name = "SECONDS",
        requires = "probe_server",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub(crate) probe_timeout_seconds: Option<u64>,
    /// Stay connected for this many seconds after the probe's program finishes.
    /// A load or restart driver needs sessions that overlap; without this a
    /// probe leaves as soon as it has what it came for.
    #[arg(
        long,
        help_heading = "Diagnostics & Probes",
        hide = true,
        value_name = "SECONDS",
        requires = "probe_server"
    )]
    pub(crate) probe_hold_seconds: Option<u64>,
    /// Immediately connect to `--server` on desktop launch.
    #[arg(long, help_heading = "Connection & Identity")]
    pub(crate) connect_server: bool,
    /// Serve the line-delimited JSON control API on this loopback address.
    #[arg(long, help_heading = "Automation, Control & Capture")]
    pub(crate) control: Option<SocketAddr>,
    /// Deterministic replay: the clock is virtual and only advances on the
    /// control or MCP endpoint's `sim.step`.
    #[arg(
        long,
        help_heading = "Automation, Control & Capture",
        requires = "control_endpoint"
    )]
    pub(crate) lockstep: bool,
    /// Lockstep replay: append every sound the client plays as a JSON line.
    #[arg(
        long,
        help_heading = "Automation, Control & Capture",
        value_name = "PATH",
        requires = "lockstep"
    )]
    pub(crate) lockstep_audio_log: Option<PathBuf>,
    /// Serve the MCP endpoint on this loopback address; disabled unless given.
    #[arg(
        long,
        help_heading = "Automation, Control & Capture",
        value_name = "ADDR"
    )]
    pub(crate) mcp_bind: Option<SocketAddr>,
    /// Save a PNG screenshot to PATH once in-world rendering settles.
    #[arg(long, help_heading = "Automation, Control & Capture")]
    pub(crate) screenshot: Option<PathBuf>,
    /// Exit after writing `--screenshot`.
    #[arg(long, help_heading = "Automation, Control & Capture")]
    pub(crate) exit_after_screenshot: bool,
    /// Record native H.264 video and AAC game audio to MP4 (Windows).
    #[arg(
        long,
        help_heading = "Automation, Control & Capture",
        value_name = "PATH"
    )]
    pub(crate) record_video: Option<PathBuf>,
    /// Recorded MP4 target frame rate (1..=120).
    #[arg(
        long,
        help_heading = "Automation, Control & Capture",
        default_value_t = 30,
        value_parser = clap::value_parser!(u32).range(1..=120)
    )]
    pub(crate) record_fps: u32,
    /// Recorded MP4 target video bitrate in bits per second.
    #[arg(
        long,
        help_heading = "Automation, Control & Capture",
        default_value_t = 8_000_000,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub(crate) record_bitrate: u32,
    /// Override the directory where crash reports are written.
    #[arg(long, help_heading = "Connection & Identity", value_name = "PATH")]
    pub(crate) crash_report_dir: Option<PathBuf>,
    /// Override the store file recording accepted server codes of conduct.
    #[arg(long, help_heading = "Connection & Identity", value_name = "PATH")]
    pub(crate) code_of_conduct_store: Option<PathBuf>,
    /// Path to `servers.dat` updated when `--connect-server` is active.
    #[arg(
        long,
        help_heading = "Connection & Identity",
        value_name = "PATH",
        requires_all = ["server_list_name", "connect_server"]
    )]
    pub(crate) server_list_store: Option<PathBuf>,
    /// Entry name in `--server-list-store` updated when `--connect-server` is active.
    #[arg(
        long,
        help_heading = "Connection & Identity",
        value_name = "NAME",
        requires_all = ["server_list_store", "connect_server"]
    )]
    pub(crate) server_list_name: Option<String>,
    /// Additional local resource pack root directories to load (repeatable).
    #[arg(
        long = "resource-pack-dir",
        help_heading = "Resource Packs & Network Proxy",
        value_name = "PATH"
    )]
    pub(crate) resource_pack_dirs: Vec<PathBuf>,
    /// `Options.resourcePacks`, consumed only when no explicit CLI pack roots
    /// replace the Vanilla `<gameDir>/resourcepacks` repository.
    #[arg(skip)]
    pub(crate) stored_resource_packs: Vec<String>,
    /// `Options.incompatibleResourcePacks`, paired with the stored selection
    /// during repository reconciliation.
    #[arg(skip)]
    pub(crate) stored_incompatible_resource_packs: Vec<String>,
    /// Overrides the per-server choice. Without a saved choice the client prompts.
    #[arg(
        long = "server-resource-pack-policy",
        help_heading = "Resource Packs & Network Proxy",
        value_enum
    )]
    pub(crate) server_resource_pack_policy: Option<ServerResourcePackPolicyArg>,
    /// Override cache directory for downloaded server resource packs.
    #[arg(
        long = "server-resource-pack-cache-dir",
        help_heading = "Resource Packs & Network Proxy",
        value_name = "PATH"
    )]
    pub(crate) server_resource_pack_cache_dir: Option<PathBuf>,
    /// Vanilla `Main --proxyHost`; server-pack HTTP uses an explicit SOCKS5
    /// proxy only when this is present.
    #[arg(
        long = "proxy-host",
        help_heading = "Resource Packs & Network Proxy",
        value_name = "HOST"
    )]
    pub(crate) proxy_host: Option<String>,
    /// SOCKS5 proxy port paired with `--proxy-host`.
    #[arg(
        long = "proxy-port",
        help_heading = "Resource Packs & Network Proxy",
        value_name = "PORT",
        default_value_t = 8080
    )]
    pub(crate) proxy_port: i32,
    /// Optional SOCKS5 proxy username paired with `--proxy-host`.
    #[arg(
        long = "proxy-user",
        help_heading = "Resource Packs & Network Proxy",
        value_name = "USER"
    )]
    pub(crate) proxy_user: Option<String>,
    /// Optional SOCKS5 proxy password paired with `--proxy-host`.
    #[arg(
        long = "proxy-pass",
        help_heading = "Resource Packs & Network Proxy",
        value_name = "PASSWORD"
    )]
    pub(crate) proxy_pass: Option<RedactedString>,
    /// Override cache directory for downloaded player skins.
    #[arg(
        long = "player-skin-cache-dir",
        help_heading = "Connection & Identity",
        value_name = "PATH"
    )]
    pub(crate) player_skin_cache_dir: Option<PathBuf>,
    /// Client UI language code (for example `en_us` or `zh_cn`).
    #[arg(
        long = "locale",
        help_heading = "Audio & Accessibility",
        default_value = "en_us"
    )]
    pub(crate) client_locale: String,
    /// Narrator speech mode.
    #[arg(
        long = "narrator",
        help_heading = "Audio & Accessibility",
        value_enum,
        default_value = "off"
    )]
    pub(crate) client_narrator: ClientNarratorArg,
    /// Enable the `Ctrl+B` narrator toggle hotkey.
    #[arg(
        long = "narrator-hotkey",
        help_heading = "Audio & Accessibility",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_narrator_hotkey: bool,
    /// Start the window in fullscreen mode (accepts `--fullscreen` or `--fullscreen <true|false>`).
    #[arg(
        long = "fullscreen",
        help_heading = "Window & Display",
        default_value_t = false,
        default_missing_value = "true",
        num_args = 0..=1,
        action = ArgAction::Set
    )]
    pub(crate) fullscreen: bool,
    /// Vanilla `Main --width/--height` defaults. Signed input is intentional:
    /// the native window owner clamps degenerate values to one.
    #[arg(
        long = "width",
        help_heading = "Window & Display",
        default_value_t = 854,
        allow_hyphen_values = true
    )]
    pub(crate) window_width: i32,
    /// Initial window height in pixels (clamped to at least 1).
    #[arg(
        long = "height",
        help_heading = "Window & Display",
        default_value_t = 480,
        allow_hyphen_values = true
    )]
    pub(crate) window_height: i32,
    /// Preferred fullscreen width when `--fullscreen-resolution` is unset.
    #[arg(
        long = "fullscreen-width",
        help_heading = "Window & Display",
        allow_hyphen_values = true
    )]
    pub(crate) fullscreen_width: Option<i32>,
    /// Preferred fullscreen height when `--fullscreen-resolution` is unset.
    #[arg(
        long = "fullscreen-height",
        help_heading = "Window & Display",
        allow_hyphen_values = true
    )]
    pub(crate) fullscreen_height: Option<i32>,
    /// Explicit fullscreen display mode (for example `1920x1080@60`).
    #[arg(
        long = "fullscreen-resolution",
        help_heading = "Window & Display",
        value_parser = parse_client_fullscreen_resolution
    )]
    pub(crate) client_fullscreen_resolution: Option<window_surface::PreferredVideoMode>,
    /// GUI scale factor (`0` selects automatic scaling).
    #[arg(
        long = "gui-scale",
        help_heading = "Window & Display",
        default_value_t = 0,
        value_parser = parse_client_gui_scale
    )]
    pub(crate) client_gui_scale: u32,
    /// Always render text using the GNU Unifont Unicode bitmap fallback.
    #[arg(
        long = "force-unicode-font",
        help_heading = "Audio & Accessibility",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_force_unicode_font: bool,
    /// Prefer Japanese glyph variants for unified CJK code points.
    #[arg(
        long = "japanese-glyph-variants",
        help_heading = "Audio & Accessibility",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_japanese_glyph_variants: bool,
    /// Use raw OS mouse deltas when pointer capture is active.
    #[arg(
        long = "raw-mouse-input",
        help_heading = "Controls & Gameplay",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_raw_mouse_input: bool,
    /// Allow the client to switch OS mouse cursor shapes.
    #[arg(
        long = "allow-cursor-changes",
        help_heading = "Controls & Gameplay",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_allow_cursor_changes: bool,
    /// Mouse look sensitivity multiplier.
    #[arg(
        long = "mouse-sensitivity",
        help_heading = "Controls & Gameplay",
        default_value_t = 0.5
    )]
    pub(crate) client_mouse_sensitivity: f64,
    /// Invert horizontal mouse look input.
    #[arg(
        long = "invert-mouse-x",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_invert_mouse_x: bool,
    /// Invert vertical mouse look input.
    #[arg(
        long = "invert-mouse-y",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_invert_mouse_y: bool,
    /// Treat trackpad smooth wheel deltas as discrete scroll ticks.
    #[arg(
        long = "discrete-mouse-scroll",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_discrete_mouse_scroll: bool,
    /// Mouse scroll wheel sensitivity multiplier.
    #[arg(
        long = "mouse-wheel-sensitivity",
        help_heading = "Controls & Gameplay",
        default_value_t = 1.0
    )]
    pub(crate) client_mouse_wheel_sensitivity: f64,
    /// Toggle sneaking on press instead of holding the sneak key.
    #[arg(
        long = "toggle-crouch",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_toggle_crouch: bool,
    /// Toggle sprinting on press instead of holding the sprint key.
    #[arg(
        long = "toggle-sprint",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_toggle_sprint: bool,
    /// Toggle attacking on press instead of holding the attack button.
    #[arg(
        long = "toggle-attack",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_toggle_attack: bool,
    /// Toggle item use on press instead of holding the use button.
    #[arg(
        long = "toggle-use",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_toggle_use: bool,
    /// Resolved from every `key_<name>` line at once, so it has no flag of its
    /// own the way the single-valued options above do.
    #[arg(skip)]
    pub(crate) client_key_bindings: KeyBindings,
    /// Display accessibility subtitles overlay for nearby sounds.
    #[arg(
        long = "show-subtitles",
        help_heading = "Audio & Accessibility",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_show_subtitles: bool,
    /// Empty is Vanilla's system-default sentinel; a non-empty value is the
    /// persisted audio device display name. Explicit startup configuration
    /// wins over `Options.soundDevice` because there is no in-game options UI.
    #[arg(
        long = "sound-device",
        help_heading = "Audio & Accessibility",
        value_name = "NAME",
        default_value = ""
    )]
    pub(crate) client_sound_device: String,
    /// Vanilla `Options.directionalAudio`: requests HRTF only when the
    /// verified OpenAL implementation reports a supported specifier.
    #[arg(
        long = "directional-audio",
        help_heading = "Audio & Accessibility",
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
    #[arg(
        long = "music-toast",
        help_heading = "Audio & Accessibility",
        value_enum,
        default_value = "never"
    )]
    pub(crate) client_music_toast: MusicToastDisplayStateArg,
    /// Vanilla `Options.musicFrequency`, consumed by the native
    /// `MusicManager` schedule.
    #[arg(
        long = "music-frequency",
        help_heading = "Audio & Accessibility",
        value_enum,
        default_value = "default"
    )]
    pub(crate) client_music_frequency: MusicFrequencyArg,
    /// Use high-contrast outline color for the targeted block hitbox.
    #[arg(
        long = "high-contrast-block-outline",
        help_heading = "Audio & Accessibility"
    )]
    pub(crate) high_contrast_block_outline: bool,
    /// Duration multiplier (`0.5..=10.0`) for toast notifications.
    #[arg(
        long = "notification-display-time",
        help_heading = "Audio & Accessibility",
        default_value_t = 1.0,
        value_parser = parse_notification_display_time
    )]
    pub(crate) notification_display_time: f32,
    /// External speech binary overriding the built-in OS narrator synthesizer.
    #[arg(
        long = "narrator-command",
        help_heading = "Audio & Accessibility",
        value_name = "PATH"
    )]
    pub(crate) narrator_command: Option<PathBuf>,
    /// Extra argument passed to `--narrator-command` (repeatable).
    #[arg(
        long = "narrator-command-arg",
        help_heading = "Audio & Accessibility",
        value_name = "ARG"
    )]
    pub(crate) narrator_command_args: Vec<String>,
    /// `Options.getFinalSoundSourceVolume(SoundSource.VOICE)`: voice times
    /// master, resolved from the stored `soundCategory_*` lines.
    #[arg(skip = 1.0_f32)]
    pub(crate) narrator_voice_volume: f32,
    /// Which incoming chat messages are displayed (`full`, `system`, `hidden`).
    #[arg(
        long = "chat-visibility",
        help_heading = "Chat & Multiplayer Safety",
        value_enum,
        default_value = "full"
    )]
    pub(crate) client_chat_visibility: ClientChatVisibilityArg,
    /// Preserve unsent chat input drafts across screen closes.
    #[arg(
        long = "save-chat-drafts",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_save_chat_drafts: bool,
    /// Hide chat messages without a verified cryptographic signature.
    #[arg(
        long = "only-show-secure-chat",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_only_show_secure_chat: bool,
    /// Hide chat messages matching profanity or blocked words.
    #[arg(
        long = "hide-matched-names",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_hide_matched_names: bool,
    /// Minimum delay in seconds (`0.0..=6.0`) before incoming chat messages appear.
    #[arg(
        long = "chat-delay-seconds",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 0.0,
        value_parser = parse_client_chat_delay
    )]
    pub(crate) client_chat_delay_seconds: f64,
    /// Vanilla `Options.chatScale`, startup-only.
    #[arg(
        long = "chat-scale",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 1.0,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_chat_scale: f64,
    /// Vanilla `Options.chatWidth`, startup-only.
    #[arg(
        long = "chat-width",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 1.0,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_chat_width: f64,
    /// Vanilla `Options.chatHeightFocused`, startup-only.
    #[arg(
        long = "chat-height-focused",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 1.0,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_chat_height_focused: f64,
    /// Vanilla `Options.chatHeightUnfocused`, startup-only.
    #[arg(
        long = "chat-height-unfocused",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 0.4375,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_chat_height_unfocused: f64,
    /// Vanilla `Options.chatLineSpacing`, startup-only.
    #[arg(
        long = "chat-line-spacing",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 0.0,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_chat_line_spacing: f64,
    /// Vanilla `Options.chatOpacity`, startup-only.
    #[arg(
        long = "chat-opacity",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = 1.0,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_chat_opacity: f64,
    /// UUID of a player whose chat and interactions are blocked (repeatable).
    #[arg(
        long = "blocked-player",
        help_heading = "Chat & Multiplayer Safety",
        value_name = "UUID"
    )]
    pub(crate) client_blocked_players: Vec<Uuid>,
    /// Render color codes in incoming chat messages.
    #[arg(
        long = "chat-colors",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_colors: bool,
    /// Make URLs clickable inside chat messages.
    #[arg(
        long = "chat-links",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_links: bool,
    /// Show a confirmation dialog before opening clicked chat links.
    #[arg(
        long = "chat-links-prompt",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_links_prompt: bool,
    /// Bitmask of enabled player model skin customization layers (`0..=255`).
    #[arg(
        long = "skin-parts",
        help_heading = "Controls & Gameplay",
        default_value_t = 0x7f
    )]
    pub(crate) client_skin_parts: u8,
    /// Player dominant hand advertised to the server (`left` or `right`).
    #[arg(
        long = "main-hand",
        help_heading = "Controls & Gameplay",
        value_enum,
        default_value = "right"
    )]
    pub(crate) client_main_hand: ClientMainHandArg,
    /// Enable server-side text filtering for this client session.
    #[arg(
        long = "text-filtering",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_text_filtering: bool,
    /// Allow servers to list this player in server query sample lists.
    #[arg(
        long = "allow-server-listing",
        help_heading = "Chat & Multiplayer Safety",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_allow_server_listing: bool,
    /// Particle density budget (`all`, `decreased`, `minimal`).
    #[arg(
        long = "particles",
        help_heading = "Video & Rendering",
        value_enum,
        default_value = "all"
    )]
    pub(crate) client_particles: ClientParticleStatusArg,
    /// Automatically jump up single-block steps while moving forward.
    #[arg(
        long = "auto-jump",
        help_heading = "Controls & Gameplay",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_auto_jump: bool,
    /// Hide coordinates and extended details in the F3 debug screen.
    #[arg(
        long = "reduced-debug-info",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_reduced_debug_info: bool,
    /// Enable edge-of-screen vignette darkening.
    #[arg(
        long = "vignette",
        help_heading = "Video & Rendering",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_vignette: bool,
    /// Placement of the attack cooldown indicator (`off`, `crosshair`, `hotbar`).
    #[arg(
        long = "attack-indicator",
        help_heading = "Controls & Gameplay",
        value_enum,
        default_value = "crosshair"
    )]
    pub(crate) client_attack_indicator: ClientAttackIndicatorArg,
    /// Scale multiplier for distortion/nausea screen overlays (`0.0..=1.0`).
    #[arg(
        long = "screen-effect-scale",
        help_heading = "Video & Rendering",
        default_value_t = 1.0
    )]
    pub(crate) client_screen_effect_scale: f32,
    /// Initial gameplay tutorial toast step.
    #[arg(
        long = "tutorial-step",
        help_heading = "Controls & Gameplay",
        value_enum,
        default_value = "movement"
    )]
    pub(crate) tutorial_step: TutorialStepArg,
    /// Vanilla's process-persisted first-multiplayer-server sentinel. It is a
    /// startup option because BBB intentionally has no in-game options UI.
    #[arg(
        long = "joined-first-server",
        help_heading = "Controls & Gameplay",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub(crate) client_joined_first_server: bool,
    /// Terrain render distance in chunks (`2..=32`).
    #[arg(
        long = "render-distance",
        help_heading = "Video & Rendering",
        default_value_t = VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
        value_parser = parse_render_distance
    )]
    pub(crate) render_distance_chunks: u32,
    /// Graphics quality preset (`fast`, `fancy`, `fabulous`).
    #[arg(
        long = "graphics-preset",
        help_heading = "Video & Rendering",
        value_enum,
        default_value = "fancy"
    )]
    pub(crate) graphics_preset: GraphicsPresetArg,
    /// Biome color blending radius in blocks (`0..=7`).
    #[arg(
        long = "biome-blend-radius",
        help_heading = "Video & Rendering",
        default_value_t = 2,
        value_parser = clap::value_parser!(u32).range(0..=7)
    )]
    pub(crate) biome_blend_radius: u32,
    /// Chunk rebuild priority policy (`none`, `player-affects`, `nearby`).
    #[arg(
        long = "prioritize-chunk-updates",
        help_heading = "Video & Rendering",
        value_enum,
        default_value = "none"
    )]
    pub(crate) prioritize_chunk_updates: PrioritizeChunkUpdatesArg,
    /// Client simulation distance in chunks (`5..=32`).
    #[arg(
        long = "simulation-distance",
        help_heading = "Video & Rendering",
        default_value_t = 12,
        value_parser = clap::value_parser!(u32).range(5..=32)
    )]
    pub(crate) client_simulation_distance_chunks: u32,
    /// Mipmap pyramid levels for terrain atlas textures (`0..=4`).
    #[arg(
        long = "mipmap-levels",
        help_heading = "Video & Rendering",
        default_value_t = 4,
        value_parser = clap::value_parser!(u32).range(0..=4)
    )]
    pub(crate) client_mipmap_levels: u32,
    /// Entity render distance scaling factor (`0.5..=5.0`).
    #[arg(
        long = "entity-distance-scaling",
        help_heading = "Video & Rendering",
        default_value_t = 1.0,
        value_parser = parse_client_entity_distance_scaling
    )]
    pub(crate) client_entity_distance_scaling: f64,
    /// Weather particle radius in blocks (`3..=10`).
    #[arg(
        long = "weather-radius",
        help_heading = "Video & Rendering",
        default_value_t = 10,
        value_parser = clap::value_parser!(u32).range(3..=10)
    )]
    pub(crate) client_weather_radius: u32,
    /// Cloud render distance in chunks (`2..=128`).
    #[arg(
        long = "cloud-range",
        help_heading = "Video & Rendering",
        default_value_t = VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
        value_parser = parse_cloud_range
    )]
    pub(crate) cloud_range_chunks: u32,
    /// Cloud layer rendering mode (`off`, `fast`, `fancy`).
    #[arg(
        long = "render-clouds",
        help_heading = "Video & Rendering",
        default_value = "fancy",
        value_parser = parse_stored_render_clouds
    )]
    pub(crate) render_clouds: RenderCloudsArg,
    /// Maximum frame rate cap (`10..=260`, where `260` means unlimited).
    #[arg(
        long = "framerate-limit",
        help_heading = "Window & Display",
        default_value_t = VANILLA_UNLIMITED_FRAMERATE_LIMIT,
        value_parser = parse_client_framerate_limit
    )]
    pub(crate) client_framerate_limit: u32,
    /// FPS reduction strategy when idle (`minimized` or `afk`).
    #[arg(
        long = "inactivity-fps-limit",
        help_heading = "Window & Display",
        default_value = "afk",
        value_parser = parse_stored_inactivity_fps_limit
    )]
    pub(crate) inactivity_fps_limit: InactivityFpsLimitArg,
    /// Enable vertical synchronization on swapchain presentation.
    #[arg(
        long = "vsync",
        help_heading = "Window & Display",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_vsync: bool,
    /// Terrain atlas sampling filter (`none`, `rgss`, `anisotropic`).
    #[arg(
        long = "terrain-texture-filtering",
        help_heading = "Video & Rendering",
        value_enum,
        default_value = "none"
    )]
    pub(crate) terrain_texture_filtering: TerrainTextureFilteringArg,
    /// Client-side GPU dynamic point lights tier (`off`, `fast`, `fancy`).
    #[arg(
        long = "dynamic-lights",
        help_heading = "Video & Rendering",
        default_value = "off",
        value_parser = parse_dynamic_lights
    )]
    pub(crate) dynamic_lights: DynamicLightsArg,
    /// Distant terrain LOD horizon distance in chunks (`0` disables, or `16..=256`).
    #[arg(
        long = "lod-distance",
        help_heading = "Video & Rendering",
        default_value_t = 0,
        value_parser = parse_lod_distance
    )]
    pub(crate) lod_distance_chunks: u32,
    /// Enabled BBB QoL client extensions (comma-separated slugs).
    #[arg(
        long = "ext",
        help_heading = "Automation, Control & Capture",
        value_delimiter = ',',
        value_parser = crate::qol_extensions::parse_client_extension_slug
    )]
    pub(crate) extensions: Vec<crate::qol_extensions::ClientExtension>,
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
        long = "max-anisotropy-bit",
        help_heading = "Video & Rendering",
        default_value_t = 2,
        value_parser = clap::value_parser!(u32).range(1..=3)
    )]
    pub(crate) client_max_anisotropy_bit: u32,
    /// Background opacity for chat and accessibility text boxes (`0.0..=1.0`).
    #[arg(
        long = "text-background-opacity",
        help_heading = "Audio & Accessibility",
        default_value_t = VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
        value_parser = parse_text_background_opacity
    )]
    pub(crate) text_background_opacity: f32,
    /// Apply `--text-background-opacity` to chat only rather than all text boxes.
    #[arg(
        long = "background-for-chat-only",
        help_heading = "Audio & Accessibility",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) background_for_chat_only: bool,
    /// Render leaf blocks using alpha-cutout transparency instead of opaque geometry.
    #[arg(
        long = "cutout-leaves",
        help_heading = "Video & Rendering",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) cutout_leaves: bool,
    /// Brightness/gamma lift factor (`0.0..=1.0`).
    #[arg(
        long = "gamma",
        help_heading = "Video & Rendering",
        default_value_t = VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
        value_parser = parse_client_gamma
    )]
    pub(crate) client_gamma: f32,
    /// Vertical camera field of view in degrees (`30..=110`).
    #[arg(
        long = "fov",
        help_heading = "Video & Rendering",
        default_value_t = VANILLA_DEFAULT_CAMERA_FOV_DEGREES as u16,
        value_parser = parse_client_fov
    )]
    pub(crate) client_fov: u16,
    /// Scale factor applied to sprint and status-effect FOV changes (`0.0..=1.0`).
    #[arg(
        long = "fov-effect-scale",
        help_heading = "Video & Rendering",
        default_value_t = VANILLA_DEFAULT_FOV_EFFECT_SCALE,
        value_parser = parse_client_fov_effect_scale
    )]
    pub(crate) client_fov_effect_scale: f32,
    /// Enchantment glint animation speed scale (`0.0..=1.0`).
    #[arg(
        long = "glint-speed",
        help_heading = "Video & Rendering",
        default_value_t = 0.5,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_glint_speed: f64,
    /// Enchantment glint brightness multiplier (`0.0..=1.0`).
    #[arg(
        long = "glint-strength",
        help_heading = "Video & Rendering",
        default_value_t = 0.75,
        value_parser = parse_client_unit_double
    )]
    pub(crate) client_glint_strength: f64,
    /// `Options.panoramaSpeed` loaded from `options.txt`.
    #[arg(skip = 1.0)]
    pub(crate) panorama_scroll_speed: f64,
    /// Enable ambient-occlusion smooth lighting across block vertices.
    #[arg(
        long = "smooth-lighting",
        help_heading = "Video & Rendering",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_smooth_lighting: bool,
    /// Enable walking camera view bobbing.
    #[arg(
        long = "view-bobbing",
        help_heading = "Video & Rendering",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_view_bobbing: bool,
    /// Enable sorted translucent rendering for Fabulous transparency.
    #[arg(
        long = "improved-transparency",
        help_heading = "Video & Rendering"
    )]
    pub(crate) improved_transparency: bool,
    /// Blur strength behind pause and title menu screens (`0..=10`).
    #[arg(
        long = "menu-background-blurriness",
        help_heading = "Video & Rendering",
        default_value_t = VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
        value_parser = parse_menu_background_blurriness
    )]
    pub(crate) menu_background_blurriness: u8,
    /// Camera tilt strength when taking damage (`0.0..=1.0`).
    #[arg(
        long = "damage-tilt-strength",
        help_heading = "Video & Rendering",
        default_value_t = 1.0,
        value_parser = parse_damage_tilt_strength
    )]
    pub(crate) damage_tilt_strength: f32,
    /// Render circular shadows beneath mobs and entities.
    #[arg(
        long = "entity-shadows",
        help_heading = "Video & Rendering",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) entity_shadows: bool,
    /// Suppress white sky flash during lightning strikes.
    #[arg(
        long = "hide-lightning-flash",
        help_heading = "Video & Rendering"
    )]
    pub(crate) hide_lightning_flash: bool,
    /// Show item IDs and durability numbers in tooltips.
    #[arg(
        long = "advanced-item-tooltips",
        help_heading = "Controls & Gameplay"
    )]
    pub(crate) advanced_item_tooltips: bool,
    /// Pause singleplayer or show pause behavior when the window loses focus.
    #[arg(
        long = "pause-on-lost-focus",
        help_heading = "Window & Display",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) pause_on_lost_focus: bool,
    /// F3 debug overlay metric profile.
    #[arg(
        long = "debug-profile",
        help_heading = "Diagnostics & Probes",
        hide = true,
        value_enum,
        default_value = "default"
    )]
    pub(crate) debug_profile: DebugScreenProfileArg,
    /// Override debug overlay profile persistence file path.
    #[arg(
        long = "debug-profile-store",
        help_heading = "Diagnostics & Probes",
        hide = true,
        value_name = "PATH"
    )]
    pub(crate) debug_profile_store: Option<PathBuf>,
    /// Enables the profiler-only enqueue-to-present-call software latency
    /// receipt used by the fixed Retina diagnostic workflow.
    #[arg(
        long = "renderer-input-latency-diagnostics",
        help_heading = "Diagnostics & Probes",
        hide = true
    )]
    pub(crate) renderer_input_latency_diagnostics: bool,
    /// Render bounding boxes for entities tracked on the local server.
    #[arg(
        long = "debug-show-local-server-entity-hit-boxes",
        help_heading = "Diagnostics & Probes",
        hide = true
    )]
    pub(crate) debug_show_local_server_entity_hit_boxes: bool,
    /// Enable F3+debug hotkey actions on startup.
    #[arg(
        long = "debug-hotkeys",
        help_heading = "Diagnostics & Probes",
        hide = true
    )]
    pub(crate) debug_hotkeys: bool,
    /// Print placed feature count diagnostics during debug runs.
    #[arg(
        long = "debug-feature-count",
        help_heading = "Diagnostics & Probes",
        hide = true
    )]
    pub(crate) debug_feature_count: bool,
    /// Enable one or more server-pushed remote debug renderer overlays.
    #[arg(
        long = "remote-debug-renderer",
        help_heading = "Diagnostics & Probes",
        hide = true,
        value_enum,
        value_name = "RENDERER",
        action = ArgAction::Append
    )]
    pub(crate) remote_debug_renderers: Vec<RemoteDebugRenderer>,
}

impl Args {
    pub(crate) fn extension_set(&self) -> crate::qol_extensions::ClientExtensionSet {
        crate::qol_extensions::ClientExtensionSet::from_extensions(self.extensions.iter().copied())
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl Args {
    pub(crate) fn default_values() -> Self {
        Self {
            game_dir: PathBuf::from("."),
            vanilla_root: None,
            server: "127.0.0.1:25565".to_string(),
            quick_play_multiplayer: None,
            username: "bbb-client".to_string(),
            uuid: None,
            status_server: None,
            build_info: false,
            probe_server: false,
            probe_after_first_chunk_packets: 0,
            probe_server_interactions: false,
            probe_server_operator: false,
            probe_server_dimension_transitions: false,
            probe_server_entity_runtime: false,
            probe_server_container_menu: false,
            probe_timeout_seconds: None,
            probe_hold_seconds: None,
            connect_server: false,
            control: None,
            lockstep: false,
            lockstep_audio_log: None,
            mcp_bind: None,
            screenshot: None,
            exit_after_screenshot: false,
            record_video: None,
            record_fps: 30,
            record_bitrate: 8_000_000,
            crash_report_dir: None,
            code_of_conduct_store: None,
            server_list_store: None,
            server_list_name: None,
            resource_pack_dirs: Vec::new(),
            stored_resource_packs: Vec::new(),
            stored_incompatible_resource_packs: Vec::new(),
            server_resource_pack_policy: None,
            server_resource_pack_cache_dir: None,
            proxy_host: None,
            proxy_port: 8080,
            proxy_user: None,
            proxy_pass: None,
            player_skin_cache_dir: None,
            client_locale: "en_us".to_string(),
            client_narrator: ClientNarratorArg::Off,
            client_narrator_hotkey: true,
            fullscreen: false,
            window_width: 854,
            window_height: 480,
            fullscreen_width: None,
            fullscreen_height: None,
            client_fullscreen_resolution: None,
            client_gui_scale: 0,
            client_force_unicode_font: false,
            client_japanese_glyph_variants: false,
            client_raw_mouse_input: true,
            client_allow_cursor_changes: true,
            client_mouse_sensitivity: 0.5,
            client_invert_mouse_x: false,
            client_invert_mouse_y: false,
            client_discrete_mouse_scroll: false,
            client_mouse_wheel_sensitivity: 1.0,
            client_toggle_crouch: false,
            client_toggle_sprint: false,
            client_toggle_attack: false,
            client_toggle_use: false,
            client_key_bindings: KeyBindings::default(),
            client_show_subtitles: false,
            client_sound_device: String::new(),
            client_directional_audio: false,
            client_audio_volumes: AudioVolumeSettings::default(),
            client_music_toast: MusicToastDisplayStateArg::Never,
            client_music_frequency: MusicFrequencyArg::Default,
            high_contrast_block_outline: false,
            notification_display_time: 1.0,
            narrator_command: None,
            narrator_command_args: Vec::new(),
            narrator_voice_volume: 1.0,
            client_chat_visibility: ClientChatVisibilityArg::Full,
            client_save_chat_drafts: false,
            client_only_show_secure_chat: false,
            client_hide_matched_names: true,
            client_chat_delay_seconds: 0.0,
            client_chat_scale: 1.0,
            client_chat_width: 1.0,
            client_chat_height_focused: 1.0,
            client_chat_height_unfocused: 0.4375,
            client_chat_line_spacing: 0.0,
            client_chat_opacity: 1.0,
            client_blocked_players: Vec::new(),
            client_chat_colors: true,
            client_chat_links: true,
            client_chat_links_prompt: true,
            client_skin_parts: 0x7f,
            client_main_hand: ClientMainHandArg::Right,
            client_text_filtering: false,
            client_allow_server_listing: false,
            client_particles: ClientParticleStatusArg::All,
            client_auto_jump: true,
            client_reduced_debug_info: false,
            client_vignette: true,
            client_attack_indicator: ClientAttackIndicatorArg::Crosshair,
            client_screen_effect_scale: 1.0,
            tutorial_step: TutorialStepArg::Movement,
            client_joined_first_server: false,
            render_distance_chunks: VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
            graphics_preset: GraphicsPresetArg::Fancy,
            biome_blend_radius: 2,
            prioritize_chunk_updates: PrioritizeChunkUpdatesArg::None,
            client_simulation_distance_chunks: 12,
            client_mipmap_levels: 4,
            client_entity_distance_scaling: 1.0,
            client_weather_radius: 10,
            cloud_range_chunks: VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
            render_clouds: RenderCloudsArg::Fancy,
            client_framerate_limit: VANILLA_UNLIMITED_FRAMERATE_LIMIT,
            inactivity_fps_limit: InactivityFpsLimitArg::Afk,
            client_vsync: true,
            terrain_texture_filtering: TerrainTextureFilteringArg::None,
            dynamic_lights: DynamicLightsArg::Off,
            lod_distance_chunks: 0,
            extensions: Vec::new(),
            fabulous_texture_filtering: false,
            client_max_anisotropy_bit: 2,
            text_background_opacity: VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
            background_for_chat_only: true,
            cutout_leaves: true,
            client_gamma: VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
            client_fov: VANILLA_DEFAULT_CAMERA_FOV_DEGREES as u16,
            client_fov_effect_scale: VANILLA_DEFAULT_FOV_EFFECT_SCALE,
            client_glint_speed: 0.5,
            client_glint_strength: 0.75,
            panorama_scroll_speed: 1.0,
            client_smooth_lighting: true,
            client_view_bobbing: true,
            improved_transparency: false,
            menu_background_blurriness: VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
            damage_tilt_strength: 1.0,
            entity_shadows: true,
            hide_lightning_flash: false,
            advanced_item_tooltips: false,
            pause_on_lost_focus: true,
            debug_profile: DebugScreenProfileArg::Default,
            debug_profile_store: None,
            renderer_input_latency_diagnostics: false,
            debug_show_local_server_entity_hit_boxes: false,
            debug_hotkeys: false,
            debug_feature_count: false,
            remote_debug_renderers: Vec::new(),
        }
    }
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

#[cfg(not(target_arch = "wasm32"))]
fn default_desktop_username() -> String {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    let digits = (hasher.finish() % 900_000) + 100_000;
    format!("Player{digits}")
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn parse_args() -> Args {
    let matches = Args::command().get_matches();
    match parse_args_from_matches(&matches) {
        Ok(args) => args,
        Err(message) => Args::command()
            .error(clap::error::ErrorKind::InvalidValue, message)
            .exit(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn parse_args_from_matches(matches: &ArgMatches) -> std::result::Result<Args, String> {
    let mut args = match Args::from_arg_matches(matches) {
        Ok(args) => args,
        Err(error) => error.exit(),
    };
    apply_stored_client_options(&mut args, matches);
    apply_vanilla_bootstrap_arguments(&mut args)?;
    if matches.value_source("username") != Some(clap::parser::ValueSource::CommandLine) {
        args.username = default_desktop_username();
    }
    // Supplying `--server <ADDR>` explicitly should start connecting without
    // requiring `--connect-server`, except for headless `--status-server` and
    // `--probe-server` modes.
    if matches.value_source("server") == Some(clap::parser::ValueSource::CommandLine)
        && args.status_server.is_none()
        && !args.probe_server
    {
        args.connect_server = true;
    }
    Ok(args)
}

/// Collapse Vanilla's launcher spellings onto BBB's existing single-owner
/// external multiplayer startup contract. Stored fullscreen is the base;
/// the no-argument launch switch and quick-play address are explicit startup
/// overrides, matching `Main.main`'s argument-derived `GameConfig`.
#[cfg(not(target_arch = "wasm32"))]
fn apply_vanilla_bootstrap_arguments(args: &mut Args) -> std::result::Result<(), String> {
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
#[cfg(not(target_arch = "wasm32"))]
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
                    return Err("incomplete Unicode escape in --quick-play-multiplayer".to_string());
                }
                let value = u16::from_str_radix(&digits, 16).map_err(|_| {
                    format!("invalid Unicode escape \\u{digits} in --quick-play-multiplayer")
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
        .map_err(|_| "unpaired UTF-16 surrogate in --quick-play-multiplayer".to_string())
}

/// Vanilla reads these values from `options.txt` alone, so the file is the
/// base and only an argument the user actually typed overrides it. A missing
/// or unreadable file leaves every default in place.
#[cfg(not(target_arch = "wasm32"))]
fn apply_stored_client_options(args: &mut Args, matches: &ArgMatches) {
    let host_locale = sys_locale::get_locale();
    let host = ClientOptionHost {
        locale_is_japanese: crate::options::locale_is_japanese(host_locale.as_deref()),
        platform_is_windows: crate::platform::HOST_IS_WINDOWS,
    };
    let path = args.game_dir.join("options.txt");
    let options = match ClientOptions::load(&path, host) {
        Ok(options) => options,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "failed to load options.txt");
            return;
        }
    };
    apply_loaded_client_options_to_args(args, &options, |arg_id| {
        matches.value_source(arg_id) == Some(clap::parser::ValueSource::CommandLine)
    });
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

fn connection_options_from_args(
    args: &Args,
    identity: &ClientIdentity,
) -> Result<ConnectionOptions> {
    let mut options = ConnectionOptions::offline_with_identity(&args.server, identity)?;
    options.server_resource_pack_policy = args
        .server_resource_pack_policy
        .map(Into::into)
        .unwrap_or_default();
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

pub(crate) fn hud_chat_options_from_args(args: &Args) -> bbb_client_render::HudChatOptions {
    bbb_client_render::HudChatOptions {
        scale: args.client_chat_scale,
        width: args.client_chat_width,
        height_focused: args.client_chat_height_focused,
        height_unfocused: args.client_chat_height_unfocused,
        line_spacing: args.client_chat_line_spacing,
        opacity: args.client_chat_opacity,
    }
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

mod arg_types;
mod options_and_parsers;
mod platform;
#[cfg(any(target_arch = "wasm32", test))]
mod web;
mod window_and_runtime;

#[cfg(test)]
use arg_types::terrain_atlas_base_padding;
pub(crate) use arg_types::*;
pub(crate) use options_and_parsers::apply_loaded_client_options_to_args;
use options_and_parsers::*;
pub(crate) use platform::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::build_web_args_from_config_and_query;
#[cfg(test)]
pub(crate) use web::parse_web_bootstrap_from_config_and_query;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use web::WebBootstrapConfig;
#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) use window_and_runtime::WindowConfig;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use window_and_runtime::{auto_discover_vanilla_root, build_window, start_control_api};
pub(crate) use window_and_runtime::{
    client_framerate_limit_interval, create_event_loop, effective_framerate_limit,
};

#[cfg(test)]
mod tests;
