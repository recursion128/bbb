use std::{net::SocketAddr, path::PathBuf, sync::Arc, thread, time::Duration};

use anyhow::{Context, Result};
use bbb_control::{shared_snapshot, NetCounters};
use bbb_net::{ConnectionOptions, NetEvent};
use bbb_platform::WindowConfig;
use bbb_world::WorldStore;
use clap::Parser;
use tokio::sync::mpsc;
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod biome_tint;
mod crosshair;
mod hud_assets;
mod input;
mod runtime;
mod terrain_runtime;

use hud_assets::load_hud_textures;
use input::{
    handle_focus_change, handle_key_input, handle_mouse_input, handle_mouse_motion,
    ClientInputState,
};
use runtime::{
    clear_color_for_world, publish_snapshot, pump_network_and_terrain, request_net_disconnect,
    snapshot_is_running, take_control_screenshot,
};
use terrain_runtime::{load_terrain_textures, TerrainUploadState};

#[derive(Debug, Parser)]
#[command(name = "bbb-native")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:25565")]
    server: String,
    #[arg(long, default_value = "bbb-client")]
    username: String,
    #[arg(long)]
    probe_server: bool,
    #[arg(long)]
    connect_server: bool,
    #[arg(long)]
    control: Option<SocketAddr>,
    #[arg(long)]
    screenshot: Option<PathBuf>,
    #[arg(long)]
    exit_after_screenshot: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let runtime = tokio::runtime::Runtime::new()?;

    if args.probe_server {
        let options = ConnectionOptions::offline(&args.server, &args.username)?;
        let report = runtime.block_on(bbb_net::run_offline_probe(options))?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let snapshot = shared_snapshot(format!(
        "bbb-native {} / protocol {}",
        bbb_protocol::MC_VERSION,
        bbb_protocol::PROTOCOL_VERSION
    ));
    let mut world = WorldStore::new();
    let mut net_counters = NetCounters::default();
    let mut net_events = None;
    let mut net_commands = None;

    if args.connect_server {
        let options = ConnectionOptions::offline(&args.server, &args.username)?;
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
        net_events = Some(rx);
        net_commands = Some(command_tx);
    }

    if let Some(addr) = args.control {
        let snapshot = Arc::clone(&snapshot);
        runtime.spawn(async move {
            if let Err(err) = bbb_control::serve(addr, snapshot).await {
                tracing::error!(?err, "control API stopped");
            }
        });
    }

    let event_loop = EventLoop::new()?;
    let config = WindowConfig::default();
    let window = WindowBuilder::new()
        .with_title(config.title.clone())
        .with_inner_size(config.physical_size())
        .build(&event_loop)
        .context("create native window")?;
    let mut input = ClientInputState::new(window.has_focus());
    let event_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        while event_proxy.send_event(()).is_ok() {
            thread::sleep(Duration::from_millis(16));
        }
    });

    let mut renderer = pollster::block_on(bbb_renderer::Renderer::new(&window))?;
    let terrain_textures = load_terrain_textures(&mut renderer);
    load_hud_textures(&mut renderer);
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let mut net_disconnect_requested = false;

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(size) => renderer.resize(size),
                WindowEvent::Focused(focused) => {
                    handle_focus_change(&mut input, &mut net_counters, &net_commands, focused)
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    handle_key_input(
                        &mut input,
                        &mut net_counters,
                        &net_commands,
                        event.physical_key,
                        event.state,
                    );
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    handle_mouse_input(
                        &mut input,
                        &world,
                        &mut net_counters,
                        &net_commands,
                        button,
                        state,
                    );
                }
                WindowEvent::RedrawRequested => {
                    if !pump_network_and_terrain(
                        &mut net_events,
                        &net_commands,
                        &mut input,
                        &mut world,
                        &mut renderer,
                        &mut net_counters,
                        &mut terrain_upload,
                        &terrain_textures,
                        &snapshot,
                    ) {
                        target.exit();
                        return;
                    }
                    renderer.set_clear_color(clear_color_for_world(&net_counters));

                    let terrain_ready_for_screenshot =
                        !screenshot_after_terrain || terrain_upload.has_uploaded_chunks();
                    let cli_screenshot_path = screenshot
                        .as_deref()
                        .filter(|_| terrain_ready_for_screenshot);
                    let control_screenshot_path = if cli_screenshot_path.is_none() {
                        take_control_screenshot(&snapshot)
                    } else {
                        None
                    };
                    let render_path = cli_screenshot_path.or(control_screenshot_path.as_deref());
                    let wrote_cli_screenshot = cli_screenshot_path.is_some();

                    if let Err(err) = renderer.render(render_path) {
                        tracing::error!(?err, "render failed");
                        target.exit();
                        return;
                    }

                    if wrote_cli_screenshot {
                        screenshot = None;
                        if exit_after_screenshot {
                            target.exit();
                        }
                    }

                    if !publish_snapshot(&snapshot, renderer.counters(), &net_counters, &world) {
                        target.exit();
                    }
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                handle_mouse_motion(&mut input, delta);
            }
            Event::UserEvent(()) => {
                if !snapshot_is_running(&snapshot) {
                    request_net_disconnect(&net_commands, &mut net_disconnect_requested);
                    target.exit();
                    return;
                }
                window.request_redraw();
            }
            Event::AboutToWait => {
                if !snapshot_is_running(&snapshot) {
                    request_net_disconnect(&net_commands, &mut net_disconnect_requested);
                    target.exit();
                    return;
                }
                if !pump_network_and_terrain(
                    &mut net_events,
                    &net_commands,
                    &mut input,
                    &mut world,
                    &mut renderer,
                    &mut net_counters,
                    &mut terrain_upload,
                    &terrain_textures,
                    &snapshot,
                ) {
                    target.exit();
                    return;
                }
                window.request_redraw();
            }
            Event::LoopExiting => {
                request_net_disconnect(&net_commands, &mut net_disconnect_requested);
                if let Ok(mut guard) = snapshot.write() {
                    guard.app.running = false;
                    guard.net.connected = false;
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}
