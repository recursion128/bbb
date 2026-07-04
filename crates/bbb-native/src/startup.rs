use std::{net::SocketAddr, path::PathBuf, thread, time::Duration};

use anyhow::{ensure, Context, Result};
use bbb_control::ControlState;
use bbb_net::{ConnectionOptions, NetCommand, NetEvent};
use bbb_pack::PackRoots;
use bbb_protocol::packets::{
    ClientChatVisibility, ClientInformation, ClientMainHand, ClientParticleStatus,
};
use bbb_renderer::{
    VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR, VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
    VANILLA_MAX_RENDER_DISTANCE_CHUNKS, VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
use clap::{ArgAction, Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tokio::{runtime::Runtime, sync::mpsc};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::code_of_conduct::CodeOfConductAcceptance;

#[derive(Debug, Parser)]
#[command(name = "bbb-native")]
pub(crate) struct Args {
    #[arg(long, default_value = "127.0.0.1:25565")]
    pub(crate) server: String,
    #[arg(long, default_value = "bbb-client")]
    pub(crate) username: String,
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
    pub(crate) code_of_conduct_store: Option<PathBuf>,
    #[arg(long = "resource-pack-dir", value_name = "PATH")]
    pub(crate) resource_pack_dirs: Vec<PathBuf>,
    #[arg(long = "player-skin-cache-dir", value_name = "PATH")]
    pub(crate) player_skin_cache_dir: Option<PathBuf>,
    #[arg(long = "client-locale", default_value = "en_us")]
    pub(crate) client_locale: String,
    #[arg(long = "client-view-distance", default_value_t = 10)]
    pub(crate) client_view_distance: i8,
    #[arg(long = "client-chat-visibility", value_enum, default_value = "full")]
    pub(crate) client_chat_visibility: ClientChatVisibilityArg,
    #[arg(
        long = "client-chat-colors",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) client_chat_colors: bool,
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
        long = "render-distance",
        default_value_t = VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
        value_parser = parse_render_distance
    )]
    pub(crate) render_distance_chunks: u32,
    #[arg(
        long = "client-gamma",
        default_value_t = VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
        value_parser = parse_client_gamma
    )]
    pub(crate) client_gamma: f32,
    #[arg(long = "hide-lightning-flash")]
    pub(crate) hide_lightning_flash: bool,
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

impl From<ClientParticleStatusArg> for ClientParticleStatus {
    fn from(value: ClientParticleStatusArg) -> Self {
        match value {
            ClientParticleStatusArg::All => Self::All,
            ClientParticleStatusArg::Decreased => Self::Decreased,
            ClientParticleStatusArg::Minimal => Self::Minimal,
        }
    }
}

pub(crate) struct NetworkHandles {
    pub(crate) events: Option<mpsc::Receiver<NetEvent>>,
    pub(crate) commands: Option<mpsc::Sender<NetCommand>>,
}

pub(crate) fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

pub(crate) fn parse_args() -> Args {
    Args::parse()
}

pub(crate) fn run_probe_if_requested(runtime: &Runtime, args: &Args) -> Result<bool> {
    if !args.probe_server {
        return Ok(false);
    }

    let mut options = ConnectionOptions::offline(&args.server, &args.username)?;
    options.client_information = client_information_from_args(args)?;
    options.probe_after_first_chunk_packets = args.probe_after_first_chunk_packets;
    let report = runtime.block_on(bbb_net::run_offline_probe(options))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(true)
}

pub(crate) fn load_pack_roots(args: &Args) -> Option<PackRoots> {
    match PackRoots::discover() {
        Ok(roots) => Some(apply_resource_pack_dirs(
            roots,
            args.resource_pack_dirs.iter().cloned(),
        )),
        Err(err) => {
            tracing::warn!(?err, "vanilla 26.1 pack roots unavailable");
            None
        }
    }
}

fn apply_resource_pack_dirs(
    roots: PackRoots,
    dirs: impl IntoIterator<Item = PathBuf>,
) -> PackRoots {
    roots.with_resource_pack_dirs(dirs)
}

pub(crate) fn start_network_if_requested(
    runtime: &Runtime,
    args: &Args,
    code_of_conduct: &mut CodeOfConductAcceptance,
) -> Result<NetworkHandles> {
    if !args.connect_server {
        return Ok(NetworkHandles {
            events: None,
            commands: None,
        });
    }

    let mut options = ConnectionOptions::offline(&args.server, &args.username)?;
    options.client_information = client_information_from_args(args)?;
    options.accepted_code_of_conduct_hash = code_of_conduct.accepted_hash_for_options(&options);
    code_of_conduct.set_connected_server(&options);
    let (tx, rx) = mpsc::channel(8192);
    let (command_tx, command_rx) = mpsc::channel(256);
    let disconnect_tx = tx.clone();
    runtime.spawn(async move {
        let reason = match bbb_net::run_offline_event_stream(options, tx, command_rx).await {
            Ok(()) => None,
            Err(err) => Some(err.to_string()),
        };
        let _ = disconnect_tx.send(NetEvent::Disconnected { reason }).await;
    });

    Ok(NetworkHandles {
        events: Some(rx),
        commands: Some(command_tx),
    })
}

pub(crate) fn client_information_from_args(args: &Args) -> Result<ClientInformation> {
    ensure!(
        args.client_locale.len() <= 16,
        "client locale must be at most 16 UTF-8 bytes"
    );
    Ok(ClientInformation {
        language: args.client_locale.clone(),
        view_distance: args.client_view_distance,
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
        PhysicalSize::new(self.width, self.height)
    }
}

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
    fn args_build_client_information_from_startup_options() {
        let args = Args::try_parse_from([
            "bbb-native",
            "--client-locale",
            "zh_cn",
            "--client-view-distance",
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
        assert_eq!(information.view_distance, 12);
        assert_eq!(information.chat_visibility, ClientChatVisibility::System);
        assert!(!information.chat_colors);
        assert_eq!(information.displayed_skin_parts, 21);
        assert_eq!(information.main_hand, ClientMainHand::Left);
        assert!(information.text_filtering_enabled);
        assert!(information.allows_listing);
        assert_eq!(information.particle_status, ClientParticleStatus::Minimal);
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
    fn args_accept_hide_lightning_flash_startup_option() {
        let default_args = Args::try_parse_from(["bbb-native"]).unwrap();
        assert!(!default_args.hide_lightning_flash);

        let args = Args::try_parse_from(["bbb-native", "--hide-lightning-flash"]).unwrap();
        assert!(args.hide_lightning_flash);
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
        let roots = PackRoots {
            mc_code_root: PathBuf::from("/mc"),
            sources_dir: PathBuf::from("/mc/sources/26.1"),
            assets_dir: PathBuf::from("/mc/sources/26.1/assets/minecraft"),
            generated_assets_dir: Some(PathBuf::from("/generated/assets-26.1")),
            resource_pack_dirs: Vec::new(),
        };

        let roots = apply_resource_pack_dirs(
            roots,
            [PathBuf::from("packs/base"), PathBuf::from("packs/overlay")],
        );

        assert_eq!(
            roots.resource_pack_dirs,
            vec![PathBuf::from("packs/base"), PathBuf::from("packs/overlay")]
        );
        assert_eq!(
            roots.generated_assets_dir,
            Some(PathBuf::from("/generated/assets-26.1"))
        );
    }
}
