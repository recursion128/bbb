use anyhow::Result;
use bbb_control::{shared_snapshot, AudioCounters, NetCounters};
use bbb_pack::{
    BlockDestroyProfile as PackBlockDestroyProfile, BlockDestroyProfileCatalog,
    BlockSoundProfile as PackBlockSoundProfile, BlockSoundProfileCatalog,
};
use bbb_renderer::{ParticleSpriteUv, ParticleUvRect};
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
mod entity_assets;
mod entity_scene;
mod hud_assets;
mod input;
mod item_entities;
mod item_frames;
mod item_models;
mod particle_registry;
mod particle_runtime;
mod runtime;
mod sky_assets;
mod startup;
mod terrain_runtime;

use audio_runtime::{AudioEventSink, NativeAudioRuntime};
use bbb_item_model::default_player_skin_cache_dir;
use bbb_item_model::NativeItemRuntime;
use code_of_conduct::{default_code_of_conduct_store_path, CodeOfConductAcceptance};
use code_of_conduct_overlay::CodeOfConductOverlayState;
use entity_assets::load_entity_model_textures;
use hud_assets::load_hud_textures;
use input::{
    handle_book_screen_mouse_input, handle_focus_change, handle_inventory_cursor_moved,
    handle_inventory_mouse_input, handle_inventory_mouse_wheel, handle_key_input_with_item_runtime,
    handle_mouse_input_at_partial_tick, handle_mouse_motion, handle_mouse_wheel,
    handle_text_input_with_item_runtime, release_active_input, ClientInputState,
};
use particle_runtime::{NativeParticleRuntime, ParticleEventSink};
use runtime::{
    control_renderer_counters, publish_snapshot, pump_network_and_terrain, request_net_disconnect,
    snapshot_is_running, take_control_screenshot, ClientAnimationTickState,
    LevelEventSoundRandomState, LightmapTickState,
};
use sky_assets::load_sky_textures;
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
        NativeParticleRuntime::load(roots, args.client_particles.into())
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
        items.enable_http_profile_resolution();
        let player_skin_cache_dir = args
            .player_skin_cache_dir
            .clone()
            .unwrap_or_else(default_player_skin_cache_dir);
        items.enable_http_player_skin_downloads(player_skin_cache_dir);
        world.set_default_item_max_stack_sizes(items.item_max_stack_sizes_by_protocol_id());
        world.set_default_item_max_damage(items.item_max_damage_by_protocol_id());
        world.set_default_item_crafting_remainders(items.item_crafting_remainders_by_protocol_id());
        world.set_recipe_specific_crafting_remainder_item_ids(
            items.recipe_specific_crafting_remainder_item_ids_by_protocol_id(),
        );
        world.set_default_item_equipment_slots(items.item_equipment_slots_by_protocol_id());
        world.set_item_armor_materials(items.item_armor_materials_by_protocol_id());
        world.set_default_mount_body_armor_kinds(items.mount_body_armor_kinds_by_protocol_id());
        world.set_default_llama_body_decor_colors(items.llama_body_decor_colors_by_protocol_id());
        world.set_default_nautilus_body_armor_materials(
            items.nautilus_body_armor_materials_by_protocol_id(),
        );
        world.set_default_horse_body_armor_materials(
            items.horse_body_armor_materials_by_protocol_id(),
        );
        world.set_default_wolf_body_armor_materials(
            items.wolf_body_armor_materials_by_protocol_id(),
        );
        world.set_default_item_attack_ranges(items.item_attack_ranges_by_protocol_id());
        world.set_default_item_swing_animation_durations(
            items.item_swing_animation_durations_by_protocol_id(),
        );
        world.set_default_piercing_weapon_item_ids(
            items.default_piercing_weapon_item_ids_by_protocol_id(),
        );
        world.set_default_damageable_item_ids(items.default_damageable_item_ids_by_protocol_id());
        world.set_default_item_use_effects(items.item_use_effects_by_protocol_id());
        world.set_furnace_fuel_item_ids(items.furnace_fuel_item_ids_by_protocol_id());
        world.set_brewing_potion_item_ids(items.brewing_potion_item_ids_by_protocol_id());
        world.set_brewing_ingredient_item_ids(items.brewing_ingredient_item_ids_by_protocol_id());
        world.set_enchantment_lapis_lazuli_item_ids(
            items.enchantment_lapis_lazuli_item_ids_by_protocol_id(),
        );
        world.set_cartography_additional_item_ids(
            items.cartography_additional_item_ids_by_protocol_id(),
        );
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
            mount_body_armor_kinds = items.mount_body_armor_kind_count(),
            nautilus_body_armor_materials = items.nautilus_body_armor_material_count(),
            horse_body_armor_materials = items.horse_body_armor_material_count(),
            wolf_body_armor_materials = items.wolf_body_armor_material_count(),
            default_attack_range_items = items.item_attack_range_count(),
            default_swing_animation_items = items.item_swing_animation_duration_count(),
            default_piercing_weapon_items = items.default_piercing_weapon_item_count(),
            default_item_max_damage = items.item_max_damage_count(),
            default_damageable_items = items.default_damageable_item_count(),
            item_crafting_remainders = items.item_crafting_remainder_count(),
            recipe_specific_crafting_remainder_items =
                items.recipe_specific_crafting_remainder_item_count(),
            default_use_effect_items = items.item_use_effect_count(),
            item_mining_profiles = items.item_mining_profile_count(),
            furnace_fuel_items = items.furnace_fuel_item_count(),
            brewing_potion_items = items.brewing_potion_item_count(),
            brewing_ingredient_items = items.brewing_ingredient_item_count(),
            enchantment_lapis_lazuli_items = items.enchantment_lapis_lazuli_item_count(),
            cartography_additional_items = items.cartography_additional_item_count(),
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
    renderer.set_lightmap_brightness_factor(args.client_gamma);
    let terrain_textures = load_terrain_textures(&mut renderer, pack_roots.as_ref());
    if let Some(particles) = particle_runtime.as_mut() {
        particles.set_terrain_particle_sprite_ids(&terrain_textures);
    }
    load_hud_textures(&mut renderer, pack_roots.as_ref());
    load_entity_model_textures(&mut renderer, pack_roots.as_ref());
    load_sky_textures(&mut renderer, pack_roots.as_ref());
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
        match renderer.upload_item_entity_atlas(atlas_width, atlas_height, items.atlas_rgba()) {
            Ok(()) => renderer.set_item_particle_sprite_uvs(item_particle_sprite_uvs(items)),
            Err(err) => {
                tracing::warn!(?err, "continuing without native item entity atlas");
            }
        }
    }
    let mut screenshot = args.screenshot;
    let screenshot_after_terrain = args.connect_server;
    let exit_after_screenshot = args.exit_after_screenshot;
    let mut terrain_upload = TerrainUploadState::default();
    let mut client_animation_ticks = ClientAnimationTickState::default();
    let mut lightmap_ticks = LightmapTickState::with_brightness_factor_and_hide_lightning_flash(
        args.client_gamma,
        args.hide_lightning_flash,
    );
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
                    let book_open = world.current_book().is_some();
                    if matches!(event.state, ElementState::Pressed)
                        && matches!(event.physical_key, PhysicalKey::Code(KeyCode::Escape))
                        && cursor_captured
                        && !input.command_entry_is_active()
                        && !sign_editor_open
                        && !book_open
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
                    if sign_editor_open || book_open {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                    }
                    if !cursor_captured && !container_open && !sign_editor_open && !book_open {
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
                    let book_open = world.current_book().is_some();
                    if world.current_dialog().is_some() || book_open {
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
                    if world.current_book().is_some() {
                        set_cursor_capture(&window, &mut cursor_captured, false);
                        handle_book_screen_mouse_input(
                            &mut world,
                            button,
                            state,
                            cursor_position,
                            window.inner_size(),
                        );
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
                    if world.current_book().is_some() {
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
                    if let Some(items) = item_runtime.as_ref() {
                        items.drain_profile_resolution_results();
                        drain_dynamic_player_skin_downloads(items, &mut renderer);
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
                        &mut lightmap_ticks,
                        &mut level_event_sound_random,
                        &mut terrain_upload,
                        &terrain_textures,
                        item_runtime.as_ref(),
                        &snapshot,
                        Some(&mut code_of_conduct_acceptance),
                        args.render_distance_chunks,
                        args.hide_lightning_flash,
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
                        control_renderer_counters(renderer.counters()),
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
                if let Some(items) = item_runtime.as_ref() {
                    items.drain_profile_resolution_results();
                    drain_dynamic_player_skin_downloads(items, &mut renderer);
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
                    &mut lightmap_ticks,
                    &mut level_event_sound_random,
                    &mut terrain_upload,
                    &terrain_textures,
                    item_runtime.as_ref(),
                    &snapshot,
                    Some(&mut code_of_conduct_acceptance),
                    args.render_distance_chunks,
                    args.hide_lightning_flash,
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

fn item_particle_sprite_uvs(items: &NativeItemRuntime) -> Vec<ParticleSpriteUv> {
    items
        .atlas_sprite_uvs()
        .into_iter()
        .map(|sprite| ParticleSpriteUv {
            id: sprite.id,
            uv: ParticleUvRect {
                min: sprite.uv.min,
                max: sprite.uv.max,
            },
        })
        .collect()
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
    world.open_container_id().is_some()
        || world.current_dialog().is_some()
        || world.current_book().is_some()
}

fn runtime_wants_cursor(input: &ClientInputState, world: &WorldStore) -> bool {
    world_wants_cursor(world) || input.sign_editor_is_active_or_pending(world)
}

fn drain_dynamic_player_skin_downloads(
    items: &NativeItemRuntime,
    renderer: &mut bbb_renderer::Renderer,
) {
    for download in items.drain_dynamic_player_skin_download_results() {
        let Some(skin) = download.skin else {
            continue;
        };
        let handle = skin.handle;
        if let Err(err) = renderer.upload_dynamic_player_skin(skin) {
            tracing::warn!(
                ?err,
                url = %download.url,
                "failed to upload dynamic player skin"
            );
            items.mark_profile_skin_failed(&download.url);
            continue;
        }
        items.mark_profile_skin_resolved(&download.url, handle);
    }

    for download in items.drain_dynamic_player_texture_download_results() {
        match download.texture {
            Some(texture) => {
                let handle = texture.handle;
                let size = texture.size;
                if let Err(err) = renderer.upload_dynamic_player_texture(texture) {
                    tracing::warn!(
                        ?err,
                        kind = ?download.kind,
                        url = %download.url,
                        handle,
                        width = size[0],
                        height = size[1],
                        "failed to upload dynamic player profile texture"
                    );
                    continue;
                }
                tracing::debug!(
                    kind = ?download.kind,
                    url = %download.url,
                    handle,
                    width = size[0],
                    height = size[1],
                    "uploaded dynamic player profile texture"
                );
            }
            None => {
                tracing::warn!(
                    kind = ?download.kind,
                    url = %download.url,
                    "failed to download dynamic player profile texture"
                );
            }
        }
    }
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

    #[test]
    fn item_particle_sprite_uvs_reuse_item_atlas_rects() {
        let runtime = NativeItemRuntime::empty_for_test();
        let item_uvs = runtime.atlas_sprite_uvs();
        let particle_uvs = item_particle_sprite_uvs(&runtime);

        assert_eq!(particle_uvs.len(), item_uvs.len());
        assert_eq!(particle_uvs[0].id, item_uvs[0].id);
        assert_eq!(particle_uvs[0].uv.min, item_uvs[0].uv.min);
        assert_eq!(particle_uvs[0].uv.max, item_uvs[0].uv.max);
    }
}
