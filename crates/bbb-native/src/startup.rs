use std::{net::SocketAddr, path::PathBuf, thread, time::Duration};

use anyhow::{Context, Result};
use bbb_control::SharedSnapshot;
use bbb_net::{ConnectionOptions, NetCommand, NetEvent};
use bbb_pack::PackRoots;
use bbb_platform::WindowConfig;
use clap::Parser;
use tokio::{runtime::Runtime, sync::mpsc};
use winit::{
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

pub(crate) fn start_control_api(
    runtime: &Runtime,
    addr: Option<SocketAddr>,
    snapshot: &SharedSnapshot,
) {
    let Some(addr) = addr else {
        return;
    };

    let snapshot = snapshot.clone();
    runtime.spawn(async move {
        if let Err(err) = bbb_control::serve(addr, snapshot).await {
            tracing::error!(?err, "control API stopped");
        }
    });
}

pub(crate) fn create_event_loop() -> Result<EventLoop<()>> {
    Ok(EventLoop::new()?)
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
