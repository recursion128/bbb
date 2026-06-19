use anyhow::Result;
use bbb_control::{shared_snapshot, AudioCounters, NetCounters};
use bbb_pack::{
    BlockDestroyProfile as PackBlockDestroyProfile, BlockDestroyProfileCatalog,
    BlockSoundProfile as PackBlockSoundProfile, BlockSoundProfileCatalog,
};
use bbb_world::{WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldStore};
use std::collections::BTreeMap;
use winit::{
    event::{DeviceEvent, ElementState, Event, Ime, WindowEvent},
    event_loop::ControlFlow,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

mod audio_runtime;
mod biome_tint;
mod block_outline;
mod camera_pose;
mod code_of_conduct;
mod code_of_conduct_overlay;
mod crosshair;
mod entity_scene;
mod hud_assets;
mod input;
mod item_entities;
mod item_runtime;
mod particle_registry;
mod particle_runtime;
mod runtime;
mod startup;
mod terrain_runtime;

use audio_runtime::{AudioEventSink, NativeAudioRuntime};
use code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance};
use code_of_conduct_overlay::CodeOfConductOverlayState;
use hud_assets::load_hud_textures;
use input::{
    handle_focus_change, handle_inventory_cursor_moved, handle_inventory_mouse_input,
    handle_inventory_mouse_wheel, handle_key_input_with_item_runtime,
    handle_mouse_input_at_partial_tick, handle_mouse_motion, handle_mouse_wheel,
    handle_text_input_with_item_runtime, release_active_input, ClientInputState,
};
use item_runtime::NativeItemRuntime;
use particle_runtime::{NativeParticleRuntime, ParticleEventSink};
use runtime::{
    clear_color_for_world, publish_snapshot, pump_network_and_terrain, request_net_disconnect,
    snapshot_is_running, take_control_screenshot, ClientAnimationTickState,
    LevelEventSoundRandomState,
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

    let pack_roots = load_pack_roots(&args);
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
    let (mut audio_runtime, audio_status) = match pack_roots.as_ref() {
        Some(roots) => match NativeAudioRuntime::load(roots) {
            Ok(runtime) => {
                let status = runtime.counters();
                (Some(runtime), status)
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without native audio runtime");
                (None, AudioCounters::disabled(err.to_string()))
            }
        },
        None => (
            None,
            AudioCounters::disabled("pack roots unavailable for native audio runtime"),
        ),
    };
    let mut particle_runtime = pack_roots.as_ref().and_then(|roots| {
        NativeParticleRuntime::load(roots)
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native particle runtime");
                err
            })
            .ok()
    });
    let item_runtime = pack_roots.as_ref().and_then(|roots| {
        NativeItemRuntime::load_with_locale(roots, &args.client_locale)
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native item asset runtime");
                err
            })
            .ok()
    });
    if let Some(items) = &item_runtime {
        world.set_default_item_max_stack_sizes(items.item_max_stack_sizes_by_protocol_id());
        world.set_default_item_equipment_slots(items.item_equipment_slots_by_protocol_id());
        world.set_default_item_attack_ranges(items.item_attack_ranges_by_protocol_id());
        world.set_default_piercing_weapon_item_ids(
            items.default_piercing_weapon_item_ids_by_protocol_id(),
        );
        world.set_furnace_fuel_item_ids(items.furnace_fuel_item_ids_by_protocol_id());
        world.set_freeze_immune_wearable_item_ids(
            items.freeze_immune_wearable_item_ids_by_protocol_id(),
        );
        world.set_powder_snow_walkable_foot_item_ids(
            items.powder_snow_walkable_foot_item_ids_by_protocol_id(),
        );
        world.set_default_item_mining_profiles(items.item_mining_profiles_by_protocol_id());
        let (atlas_width, atlas_height) = items.atlas_size();
        let missingno_index = items.texture_index("minecraft:missingno");
        tracing::info!(
            item_definitions = items.item_definition_count(),
            item_registry_entries = items.item_registry_count(),
            resolved_models = items.resolved_model_count(),
            textures = items.texture_count(),
            icon_textures = items.icon_texture_count(),
            item_equipment_slots = items.item_equipment_slot_count(),
            default_attack_range_items = items.item_attack_range_count(),
            default_piercing_weapon_items = items.default_piercing_weapon_item_count(),
            item_mining_profiles = items.item_mining_profile_count(),
            furnace_fuel_items = items.furnace_fuel_item_count(),
            freeze_immune_wearable_items = items.freeze_immune_wearable_item_count(),
            powder_snow_walkable_foot_items = items.powder_snow_walkable_foot_item_count(),
            missing_models = items.missing_model_count(),
            missing_textures = items.missing_texture_count(),
            missingno_index,
            atlas_width,
            atlas_height,
            "loaded native item assets"
        );
    }
    if let Some(roots) = &pack_roots {
        match roots.load_block_destroy_profile_catalog() {
            Ok(catalog) => {
                world
                    .set_default_block_destroy_profiles(block_destroy_profiles_for_world(&catalog));
                tracing::info!(
                    block_destroy_profiles = catalog.len(),
                    "loaded block destroy profiles"
                );
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without block destroy profiles");
            }
        }
        match roots.load_block_sound_profile_catalog() {
            Ok(catalog) => {
                world.set_default_block_sound_profiles(block_sound_profiles_for_world(&catalog));
                tracing::info!(
                    block_sound_profiles = catalog.len(),
                    "loaded block sound profiles"
                );
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without block sound profiles");
            }
        }
    }

    let event_loop = create_event_loop()?;
    let window = build_window(&event_loop)?;
    window.set_ime_allowed(true);
    let mut input = ClientInputState::new(window.has_focus());
    spawn_frame_tick(&event_loop);

    let mut renderer = pollster::block_on(bbb_renderer::Renderer::new(&window))?;
    let terrain_textures = load_terrain_textures(&mut renderer, pack_roots.as_ref());
    load_hud_textures(&mut renderer, pack_roots.as_ref());
    if let Some(particles) = &particle_runtime {
        if let Err(err) = particles.upload_particle_atlas(&mut renderer) {
            tracing::warn!(?err, "continuing without native particle atlas");
        }
    }
    if let Some(items) = &item_runtime {
        let (atlas_width, atlas_height) = items.atlas_size();
        if let Err(err) =
            renderer.upload_hud_item_atlas(atlas_width, atlas_height, items.atlas_rgba())
        {
            tracing::warn!(?err, "continuing without native item HUD atlas");
        }
        if let Err(err) =
            renderer.upload_item_entity_atlas(atlas_width, atlas_height, items.atlas_rgba())
        {
            tracing::warn!(?err, "continuing without native item entity atlas");
        }
    }
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let mut client_animation_ticks = ClientAnimationTickState::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::default();
    let mut net_disconnect_requested = false;
    let mut code_of_conduct_overlay = CodeOfConductOverlayState::default();
    let mut cursor_position = None;
    let mut cursor_captured = false;

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(size) => renderer.resize(size),
                WindowEvent::CursorMoved { position, .. } => {
                    cursor_position = Some(position);
                    handle_inventory_cursor_moved(
                        &mut input,
                        &mut world,
                        &mut net_counters,
                        &net_commands,
                        cursor_position,
                        window.inner_size(),
                    );
                }
                WindowEvent::Focused(focused) => {
                    if !focused {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                    }
                    handle_focus_change(
                        &mut input,
                        &mut world,
                        &mut net_counters,
                        &net_commands,
                        focused,
                    );
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if code_of_conduct_overlay.is_visible(&world) {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    let container_open = world.open_container_id().is_some();
                    let sign_editor_open = input.sign_editor_is_active_or_pending(&world);
                    if matches!(event.state, ElementState::Pressed)
                        && matches!(event.physical_key, PhysicalKey::Code(KeyCode::Escape))
                        && cursor_captured
                        && !input.command_entry_is_active()
                        && !sign_editor_open
                        && !world_wants_cursor(&world)
                    {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        release_active_input(
                            &mut input,
                            &mut world,
                            &mut net_counters,
                            &net_commands,
                        );
                        return;
                    }
                    if sign_editor_open {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                    }
                    if !cursor_captured && !container_open && !sign_editor_open {
                        return;
                    }
                    handle_key_input_with_item_runtime(
                        &mut input,
                        &mut net_counters,
                        &mut world,
                        &net_commands,
                        item_runtime.as_ref(),
                        event.physical_key,
                        event.state,
                    );
                }
                WindowEvent::Ime(Ime::Commit(text)) => {
                    if code_of_conduct_overlay.is_visible(&world) {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    let sign_editor_open = input.sign_editor_is_active_or_pending(&world);
                    let container_open = world.open_container_id().is_some();
                    if world.current_dialog().is_some() {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    if sign_editor_open || container_open {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                    }
                    if !cursor_captured && !sign_editor_open && !container_open {
                        return;
                    }
                    handle_text_input_with_item_runtime(
                        &mut input,
                        &mut net_counters,
                        &mut world,
                        &net_commands,
                        item_runtime.as_ref(),
                        &text,
                    );
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if code_of_conduct_overlay.is_visible(&world) {
                        if code_of_conduct_overlay.handle_mouse_input(
                            &world,
                            &snapshot,
                            button,
                            state,
                            cursor_position,
                            window.inner_size(),
                        ) {
                            code_of_conduct_overlay.update_renderer(
                                &mut renderer,
                                &world,
                                code_of_conduct_acceptance.current_world_acceptance_matches(&world),
                            );
                        }
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    if input.sign_editor_is_active_or_pending(&world) {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    if world.open_container_id().is_some() {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        handle_inventory_mouse_input(
                            &mut input,
                            &mut world,
                            &mut net_counters,
                            &net_commands,
                            button,
                            state,
                            cursor_position,
                            window.inner_size(),
                        );
                        return;
                    }
                    if matches!(state, ElementState::Pressed) && !cursor_captured {
                        if runtime_wants_cursor(&input, &world) {
                            set_cursor_capture(&window, &mut cursor_captured, false);
                            return;
                        }
                        set_cursor_capture(&window, &mut cursor_captured, true);
                        return;
                    }
                    if !cursor_captured {
                        return;
                    }
                    handle_mouse_input_at_partial_tick(
                        &mut input,
                        &mut world,
                        &mut net_counters,
                        &net_commands,
                        button,
                        state,
                        client_animation_ticks.entity_partial_tick(std::time::Instant::now()),
                    );
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if code_of_conduct_overlay.is_visible(&world) {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    if input.sign_editor_is_active_or_pending(&world) {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    if world.open_container_id().is_some() {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        handle_inventory_mouse_wheel(
                            &mut input,
                            &mut world,
                            &mut net_counters,
                            &net_commands,
                            delta,
                            cursor_position,
                            window.inner_size(),
                        );
                        return;
                    }
                    if runtime_wants_cursor(&input, &world) {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        return;
                    }
                    if !cursor_captured {
                        return;
                    }
                    handle_mouse_wheel(
                        &mut input,
                        &mut world,
                        &mut net_counters,
                        &net_commands,
                        delta,
                    );
                }
                WindowEvent::RedrawRequested => {
                    if code_of_conduct_overlay.is_visible(&world)
                        || runtime_wants_cursor(&input, &world)
                    {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        release_active_input(
                            &mut input,
                            &mut world,
                            &mut net_counters,
                            &net_commands,
                        );
                    }
                    if !pump_network_and_terrain(
                        &mut net_events,
                        &net_commands,
                        audio_runtime
                            .as_mut()
                            .map(|runtime| runtime as &mut dyn AudioEventSink),
                        &audio_status,
                        particle_runtime
                            .as_mut()
                            .map(|runtime| runtime as &mut dyn ParticleEventSink),
                        &mut input,
                        &mut world,
                        &mut renderer,
                        &mut net_counters,
                        &mut client_animation_ticks,
                        &mut level_event_sound_random,
                        &mut terrain_upload,
                        &terrain_textures,
                        item_runtime.as_ref(),
                        &snapshot,
                        Some(&mut code_of_conduct_acceptance),
                    ) {
                        target.exit();
                        return;
                    }
                    renderer.set_clear_color(clear_color_for_world(&world));
                    code_of_conduct_overlay.update_renderer(
                        &mut renderer,
                        &world,
                        code_of_conduct_acceptance.current_world_acceptance_matches(&world),
                    );
                    if code_of_conduct_overlay.is_visible(&world)
                        || runtime_wants_cursor(&input, &world)
                    {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        release_active_input(
                            &mut input,
                            &mut world,
                            &mut net_counters,
                            &net_commands,
                        );
                    }

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

                    let audio_counters = audio_runtime
                        .as_ref()
                        .map(NativeAudioRuntime::counters)
                        .unwrap_or_else(|| audio_status.clone());
                    if !publish_snapshot(
                        &snapshot,
                        renderer.counters(),
                        &net_counters,
                        &audio_counters,
                        &world,
                    ) {
                        target.exit();
                    }
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if code_of_conduct_overlay.is_visible(&world)
                    || runtime_wants_cursor(&input, &world)
                {
                    set_cursor_capture(&window, &mut cursor_captured, false);
                    return;
                }
                if !cursor_captured {
                    return;
                }
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
                if code_of_conduct_overlay.is_visible(&world)
                    || runtime_wants_cursor(&input, &world)
                {
                    set_cursor_capture(&window, &mut cursor_captured, false);
                    release_active_input(&mut input, &mut world, &mut net_counters, &net_commands);
                }
                if !pump_network_and_terrain(
                    &mut net_events,
                    &net_commands,
                    audio_runtime
                        .as_mut()
                        .map(|runtime| runtime as &mut dyn AudioEventSink),
                    &audio_status,
                    particle_runtime
                        .as_mut()
                        .map(|runtime| runtime as &mut dyn ParticleEventSink),
                    &mut input,
                    &mut world,
                    &mut renderer,
                    &mut net_counters,
                    &mut client_animation_ticks,
                    &mut level_event_sound_random,
                    &mut terrain_upload,
                    &terrain_textures,
                    item_runtime.as_ref(),
                    &snapshot,
                    Some(&mut code_of_conduct_acceptance),
                ) {
                    target.exit();
                    return;
                }
                code_of_conduct_overlay.update_renderer(
                    &mut renderer,
                    &world,
                    code_of_conduct_acceptance.current_world_acceptance_matches(&world),
                );
                if code_of_conduct_overlay.is_visible(&world)
                    || runtime_wants_cursor(&input, &world)
                {
                    set_cursor_capture(&window, &mut cursor_captured, false);
                    release_active_input(&mut input, &mut world, &mut net_counters, &net_commands);
                }
                window.request_redraw();
            }
            Event::LoopExiting => {
                set_cursor_capture(&window, &mut cursor_captured, false);
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

fn block_destroy_profiles_for_world(
    catalog: &BlockDestroyProfileCatalog,
) -> BTreeMap<String, WorldBlockDestroyProfile> {
    catalog
        .profiles()
        .iter()
        .map(|(block_name, profile)| (block_name.clone(), world_block_destroy_profile(profile)))
        .collect()
}

fn world_block_destroy_profile(profile: &PackBlockDestroyProfile) -> WorldBlockDestroyProfile {
    WorldBlockDestroyProfile {
        destroy_time_tenths: profile.destroy_time_tenths,
        requires_correct_tool: profile.requires_correct_tool,
    }
}

fn block_sound_profiles_for_world(
    catalog: &BlockSoundProfileCatalog,
) -> BTreeMap<String, WorldBlockSoundProfile> {
    catalog
        .profiles()
        .iter()
        .map(|(block_name, profile)| (block_name.clone(), world_block_sound_profile(profile)))
        .collect()
}

fn world_block_sound_profile(profile: &PackBlockSoundProfile) -> WorldBlockSoundProfile {
    WorldBlockSoundProfile {
        break_sound: profile.break_sound.clone(),
        hit_sound: profile.hit_sound.clone(),
        volume: profile.volume,
        pitch: profile.pitch,
    }
}

fn set_cursor_capture(window: &Window, captured: &mut bool, capture: bool) {
    if *captured == capture {
        return;
    }

    if capture {
        match window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        {
            Ok(()) => {
                window.set_cursor_visible(false);
                *captured = true;
            }
            Err(err) => {
                tracing::warn!(?err, "unable to capture cursor for first-person input");
            }
        }
    } else {
        if let Err(err) = window.set_cursor_grab(CursorGrabMode::None) {
            tracing::warn!(?err, "unable to release cursor capture");
        }
        window.set_cursor_visible(true);
        *captured = false;
    }
}

fn world_wants_cursor(world: &WorldStore) -> bool {
    world.open_container_id().is_some() || world.current_dialog().is_some()
}

fn runtime_wants_cursor(input: &ClientInputState, world: &WorldStore) -> bool {
    world_wants_cursor(world) || input.sign_editor_is_active_or_pending(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, OpenSignEditor};

    #[test]
    fn runtime_wants_cursor_for_pending_sign_editor() {
        let input = ClientInputState::new(true);
        let mut world = WorldStore::new();

        assert!(!runtime_wants_cursor(&input, &world));

        world.apply_open_sign_editor(OpenSignEditor {
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            is_front_text: true,
        });

        assert!(runtime_wants_cursor(&input, &world));
    }
}
