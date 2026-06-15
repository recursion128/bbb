use anyhow::Result;
use bbb_control::{shared_snapshot, NetCounters};
use bbb_world::WorldStore;
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::ControlFlow,
};

mod audio_runtime;
mod biome_tint;
mod block_outline;
mod camera_pose;
mod code_of_conduct;
mod crosshair;
mod hud_assets;
mod input;
mod particle_registry;
mod particle_runtime;
mod runtime;
mod startup;
mod terrain_runtime;

use audio_runtime::{AudioEventSink, NativeAudioRuntime};
use code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance};
use hud_assets::load_hud_textures;
use input::{
    handle_focus_change, handle_key_input, handle_mouse_input, handle_mouse_motion,
    handle_mouse_wheel, ClientInputState,
};
use particle_runtime::{NativeParticleRuntime, ParticleEventSink};
use runtime::{
    clear_color_for_world, publish_snapshot, pump_network_and_terrain, request_net_disconnect,
    snapshot_is_running, take_control_screenshot, ClientAnimationTickState,
};
use startup::{
    build_window, create_event_loop, init_tracing, load_pack_roots, parse_args,
    run_probe_if_requested, spawn_frame_tick, start_control_api, start_network_if_requested,
    NetworkHandles,
};
use terrain_runtime::{load_terrain_textures, TerrainUploadState};

fn main() -> Result<()> {
    init_tracing();
    let args = parse_args();
    let runtime = tokio::runtime::Runtime::new()?;

    if run_probe_if_requested(&runtime, &args)? {
        return Ok(());
    }

    let pack_roots = load_pack_roots();
    let snapshot = shared_snapshot(format!(
        "bbb-native {} / protocol {}",
        bbb_protocol::MC_VERSION,
        bbb_protocol::PROTOCOL_VERSION
    ));
    let mut world = WorldStore::new();
    let mut net_counters = NetCounters::default();
    let code_of_conduct_store_path = args
        .code_of_conduct_store
        .clone()
        .unwrap_or_else(default_code_of_conduct_store_path);
    let mut code_of_conduct_acceptance =
        match CodeOfConductAcceptance::load(code_of_conduct_store_path.clone()) {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!(
                    ?err,
                    path = %code_of_conduct_store_path.display(),
                    "continuing with empty code-of-conduct acceptance store"
                );
                CodeOfConductAcceptance::empty(code_of_conduct_store_path)
            }
        };
    let NetworkHandles {
        events: mut net_events,
        commands: net_commands,
    } = start_network_if_requested(&runtime, &args, &mut code_of_conduct_acceptance)?;
    start_control_api(&runtime, args.control, &snapshot);
    let mut audio_runtime = pack_roots.as_ref().and_then(|roots| {
        NativeAudioRuntime::load(roots)
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native audio runtime");
                err
            })
            .ok()
    });
    let mut particle_runtime = pack_roots.as_ref().and_then(|roots| {
        NativeParticleRuntime::load(roots)
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native particle runtime");
                err
            })
            .ok()
    });

    let event_loop = create_event_loop()?;
    let window = build_window(&event_loop)?;
    let mut input = ClientInputState::new(window.has_focus());
    spawn_frame_tick(&event_loop);

    let mut renderer = pollster::block_on(bbb_renderer::Renderer::new(&window))?;
    let terrain_textures = load_terrain_textures(&mut renderer, pack_roots.as_ref());
    load_hud_textures(&mut renderer, pack_roots.as_ref());
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let mut client_animation_ticks = ClientAnimationTickState::default();
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
                        &mut world,
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
                WindowEvent::MouseWheel { delta, .. } => {
                    handle_mouse_wheel(
                        &mut input,
                        &mut world,
                        &mut net_counters,
                        &net_commands,
                        delta,
                    );
                }
                WindowEvent::RedrawRequested => {
                    if !pump_network_and_terrain(
                        &mut net_events,
                        &net_commands,
                        audio_runtime
                            .as_mut()
                            .map(|runtime| runtime as &mut dyn AudioEventSink),
                        particle_runtime
                            .as_mut()
                            .map(|runtime| runtime as &mut dyn ParticleEventSink),
                        &mut input,
                        &mut world,
                        &mut renderer,
                        &mut net_counters,
                        &mut client_animation_ticks,
                        &mut terrain_upload,
                        &terrain_textures,
                        &snapshot,
                        Some(&mut code_of_conduct_acceptance),
                    ) {
                        target.exit();
                        return;
                    }
                    renderer.set_clear_color(clear_color_for_world(&world));

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
                    audio_runtime
                        .as_mut()
                        .map(|runtime| runtime as &mut dyn AudioEventSink),
                    particle_runtime
                        .as_mut()
                        .map(|runtime| runtime as &mut dyn ParticleEventSink),
                    &mut input,
                    &mut world,
                    &mut renderer,
                    &mut net_counters,
                    &mut client_animation_ticks,
                    &mut terrain_upload,
                    &terrain_textures,
                    &snapshot,
                    Some(&mut code_of_conduct_acceptance),
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
