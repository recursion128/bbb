mod hud;
use hud::{extract_hud_and_screens, ExtractedHudAndScreens};

use super::*;
use crate::camera_pose::FrameCamera;
use crate::input::Screen;
use crate::narrator_runtime::{ScreenNarrationFrames, ScreenNarrationTarget};
use crate::simulation::commands::CommandQueue;
use bbb_client_render::{
    DisplayTextSurface, EntityFlameSurface, EntityNameTagSurface, EntityShadowSurface,
    FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture, FirstPersonPlayerArm,
    HudActionBarText, HudBossBar, HudMapPreviewSurface, HudNowPlayingToast, HudTitleText, HudToast,
    HudVignette, ItemEntityBillboard, ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture,
    ItemFrameMapSurface, ItemFrameMapTextSurface, ItemFrameMapTexture, ItemModelResidencyFrame,
    SignTextOutlineContext, SignTextSurface, WorldBorderRenderState,
};

pub(crate) fn update_renderer(
    view: &SimulationView<'_>,
    mut context: SimulationRenderContext<'_>,
) -> NativeRenderProfile {
    let profile_started_at = Instant::now();
    let basics = prepare_frame_basics(view, &mut context);
    let hud = extract_hud_and_screens(view, &mut context, &basics);
    let item_pre_started_at = Instant::now();
    let camera_and_env = extract_camera_and_env(view, &mut context, &basics, &hud);
    let world_scene = extract_world_scene(
        view,
        &mut context,
        &basics,
        &hud,
        &camera_and_env,
        item_pre_started_at,
    );
    let mut hud_duration = hud.hud_duration;
    let toasts_duration = hud.toasts_duration;
    let world_effects_and_debug = extract_world_effects_and_debug(
        view,
        &mut context,
        &basics,
        &camera_and_env,
        &world_scene,
        &mut hud_duration,
    );
    submit_renderer_frame(
        view,
        &mut context,
        &basics,
        hud,
        camera_and_env,
        world_scene,
        world_effects_and_debug,
        hud_duration,
    );
    NativeRenderProfile {
        extract: profile_started_at.elapsed().saturating_sub(toasts_duration),
        toasts: toasts_duration,
    }
}

struct FrameBasics {
    now: Instant,
    advanced_ticks: u32,
    lightmap_environment: LightmapEnvironment,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
    cloud_environment: CloudEnvironment,
    profiling_enabled: bool,
    render_distance_chunks: u32,
    depth_far: f32,
    stable_world_unchanged: bool,
    physical_surface_size: PhysicalSize<u32>,
    gui_surface_size: PhysicalSize<u32>,
    render_started_at: Instant,
    entity_partial_tick: f32,
    hud_frame_delta_game_time: f32,
    level_loading_screen_is_up: bool,
    idle_hint_screen_is_up: bool,
    panorama_rotation_degrees: f32,
    terrain_textures_epoch: u64,
    resource_generation: u64,
    resources_loaded: bool,
    hud_cursor_position: Option<(i32, i32)>,
    item_model_keybind_context: ItemModelKeybindContext,
}

fn prepare_frame_basics(
    view: &SimulationView<'_>,
    context: &mut SimulationRenderContext<'_>,
) -> FrameBasics {
    context
        .renderer
        .set_level_frame_route(LevelFrameRoute::for_options(
            context.improved_transparency,
            false,
        ));
    let state = view.frame;
    let now = state.now;
    let advanced_ticks = match context.rendered_client_tick.replace(view.client_tick_index) {
        Some(last) => {
            u32::try_from(view.client_tick_index.saturating_sub(last)).unwrap_or(u32::MAX)
        }
        None => state.advanced_ticks,
    };
    let cloud_environment = state
        .cloud_environment
        .with_range_chunks(context.cloud_range_chunks);
    let input = view.input;
    let profiling_enabled =
        input.debug_profiler_chart_visible() || input.debug_profiling_recording();
    let world = view.world;
    // `Options.getEffectiveRenderDistance`: every consumer below reads the
    // distance the server actually streams, not the configured one, so a client
    // set beyond the server's `SetChunkCacheRadius` does not render past it.
    let render_distance_chunks =
        effective_render_distance(context.render_distance_chunks, world.chunk_cache_radius());
    let depth_far = (render_distance_chunks * 4)
        .max(context.cloud_range_chunks)
        .max(context.lod_distance_chunks.saturating_add(4)) as f32
        * 16.0;
    let stable_world_unchanged = context
        .non_entity_render_extraction
        .begin_stable_world_frame(view.world_revision);
    if context
        .non_entity_render_extraction
        .synchronize_map_texture_level(world.client_level_generation())
    {
        context.renderer.invalidate_map_texture_residency();
    }
    // Physical framebuffer facts (world camera, culling, screenshots, line
    // width) read the physical surface; every HUD/screen layout, toast,
    // narration helper and GUI hit test reads the projected logical GUI
    // surface (`Minecraft.resizeGui` / `Window.setGuiScale`).
    let physical_surface_size = view.input_runtime.surface_size;
    let gui_surface_size = PhysicalSize::new(
        view.input_runtime.gui_scaled_width,
        view.input_runtime.gui_scaled_height,
    );
    let render_started_at = bbb_client_clock::now();
    let entity_partial_tick = view.partial_tick_at(render_started_at);
    let hud_frame_delta = context
        .hud_render_runtime
        .frame_delta_ticks(render_started_at, view.tick_interval);
    // Vanilla `Screen.panoramaShouldSpin` defaults to true, so both the
    // disconnected screen and `LevelLoadingScreen` keep the cubemap spinning.
    let level_loading_screen_is_up = world.level_load().screen_is_up();
    let idle_hint_screen_is_up = view.idle_mode
        && input.current_screen().is_none()
        && !level_loading_screen_is_up
        && world.level_info().is_none();
    let panorama_rotation_degrees = context.hud_render_runtime.panorama_rotation_degrees(
        input.disconnected_screen_runtime().is_some()
            || level_loading_screen_is_up
            || idle_hint_screen_is_up,
        hud_frame_delta,
    );
    if let Some((generation, delta_ticks)) = HudRenderRuntime::win_screen_realtime_delta_ticks(
        input.win_screen_generation(),
        hud_frame_delta,
    ) {
        context.commands.push(
            crate::simulation::commands::SimulationCommand::GameEventScreenRenderDelta {
                generation,
                delta_ticks,
            },
        );
    }
    context
        .hud_debug_fps_sampler
        .record_frame(render_started_at);
    context.entity_variant_textures.synchronize_session(
        view.net_counters.connection_generation,
        view.net_counters.connected,
    );
    context
        .entity_variant_textures
        .synchronize(world, context.renderer);
    let terrain_textures_epoch = context
        .hud_render_runtime
        .bump_terrain_textures_epoch(view.terrain_textures_changed);
    if view.terrain_textures_changed {
        context.terrain_upload.request_reload_all_chunks();
    }
    let resource_generation = context
        .item_runtime
        .map_or(0, NativeItemRuntime::resource_generation);
    let resources_loaded = context.item_runtime.is_some();
    let hud_cursor_position = view.input_runtime.cursor_position.and_then(|position| {
        (position.x.is_finite() && position.y.is_finite())
            .then(|| (position.x.floor() as i32, position.y.floor() as i32))
    });
    let item_model_keybind_context = input.item_model_keybind_context();
    FrameBasics {
        now,
        advanced_ticks,
        lightmap_environment: state.lightmap_environment,
        water_vision: state.water_vision,
        rain_fog_multiplier: state.rain_fog_multiplier,
        hide_lightning_flash: state.hide_lightning_flash,
        cloud_environment,
        profiling_enabled,
        render_distance_chunks,
        depth_far,
        stable_world_unchanged,
        physical_surface_size,
        gui_surface_size,
        render_started_at,
        entity_partial_tick,
        hud_frame_delta_game_time: hud_frame_delta.game_time,
        level_loading_screen_is_up,
        idle_hint_screen_is_up,
        panorama_rotation_degrees,
        terrain_textures_epoch,
        resource_generation,
        resources_loaded,
        hud_cursor_position,
        item_model_keybind_context,
    }
}

struct CameraAndEnvironment<'a> {
    item_model_age_ticks: f32,
    shader_game_time_ticks: f64,
    frame_camera: Option<FrameCamera>,
    camera_entity_pose: Option<CameraPose>,
    camera_pose: Option<CameraPose>,
    sign_text_outline_context: SignTextOutlineContext,
    first_person_camera_pose: Option<CameraPose>,
    screen_effect_quads: RendererFrameUpdate<Vec<bbb_client_render::ScreenEffectQuad>>,
    hud_vignette: RendererFrameUpdate<Option<HudVignette>>,
    hud_spyglass_scope: RendererFrameUpdate<Option<HudSpyglassScope>>,
    hud_confusion_overlay: RendererFrameUpdate<Option<HudConfusionOverlay>>,
    hud_portal_overlay: RendererFrameUpdate<Option<HudPortalOverlay>>,
    camera_hurt_effect: Option<bbb_client_render::CameraHurtEffect>,
    camera_view_bob_effect: Option<bbb_client_render::CameraViewBobEffect>,
    camera_spinning_effect: Option<bbb_client_render::CameraSpinningEffect>,
    clear_color: ClearColor,
    fog_environment: FogEnvironment,
    sky_environment: SkyEnvironment,
    camera_fov_world_degrees: f32,
    camera_fov_hud_degrees: f32,
    camera_fov_culling_degrees: f32,
    hud_contextual_bar: RendererFrameUpdate<HudContextualBar>,
    current_post_effect_id: Option<&'a str>,
    terrain_visibility_context: TerrainVisibilityContext,
}

fn extract_camera_and_env<'a>(
    view: &'a SimulationView<'a>,
    context: &mut SimulationRenderContext<'_>,
    basics: &FrameBasics,
    hud: &ExtractedHudAndScreens,
) -> CameraAndEnvironment<'a> {
    let state = view.frame;
    let input = view.input;
    let world = view.world;
    // Vanilla extracts item/entity render state after `Minecraft.tick` has advanced keybinds,
    // `gameRenderer.tick`, and `level.tickEntities`; these projections read the post-tick snapshot.
    // Dropped block-items render as 3D block-item models (replacing their billboard); the animation
    // clock is the world game time plus the partial tick.
    let item_model_age_ticks = world
        .world_time()
        .map(|time| time.game_time as f32)
        .unwrap_or(0.0)
        + basics.entity_partial_tick;
    let shader_game_time_ticks = world
        .world_time()
        .map(|time| time.game_time as f64)
        .unwrap_or(0.0)
        + f64::from(basics.entity_partial_tick);
    let frame_camera = frame_camera_from_world_at_partial_tick(
        world,
        basics.entity_partial_tick,
        input.camera_type(),
        view.camera_eye_height_ticks,
    );
    let camera_entity_pose = frame_camera.map(|camera| camera.entity_pose);
    let camera_pose = frame_camera.map(|camera| camera.pose);
    let sign_text_outline_context = sign_text_outline_context(
        camera_entity_pose,
        input.camera_type().is_first_person(),
        world.local_player_is_scoping(),
    );
    let first_person_camera_pose = input
        .camera_type()
        .is_first_person()
        .then_some(camera_entity_pose)
        .flatten();
    let screen_effect_quads = if basics.stable_world_unchanged {
        RendererFrameUpdate::Unchanged
    } else {
        RendererFrameUpdate::Replace(crate::screen_effects::screen_effect_overlay_quads(
            world,
            context.terrain_textures,
            input.camera_type().is_first_person(),
            state.sky_darken,
        ))
    };
    if basics.advanced_ticks > 0 {
        context.hud_render_runtime.vignette.advance(
            basics.advanced_ticks,
            world,
            state.sky_darken,
            render_camera_pose_from_world_at_partial_tick(
                world,
                1.0,
                input.camera_type(),
                view.camera_eye_height_ticks,
            )
            .map(camera_eye_position),
        );
    }
    let raw_vignette = context.hud_render_runtime.vignette.extract(
        world,
        camera_pose.map(camera_eye_position),
        input.hide_gui(),
        context.vignette,
    );
    let hud_vignette = context.hud_render_runtime.vignette_update(raw_vignette);
    let raw_spyglass_scope = context.hud_render_runtime.spyglass_scope(
        basics.hud_frame_delta_game_time,
        input.hide_gui(),
        input.camera_type().is_first_person(),
        world.local_player_is_scoping(),
    );
    let hud_spyglass_scope = context
        .hud_render_runtime
        .spyglass_scope_update(raw_spyglass_scope);
    let hud_confusion_overlay =
        context
            .hud_render_runtime
            .confusion_overlay_update(hud_confusion_overlay(
                world,
                input.hide_gui(),
                context.screen_effect_scale,
                basics.entity_partial_tick,
            ));
    let hud_portal_overlay = context
        .hud_render_runtime
        .portal_overlay_update(hud_portal_overlay(
            world,
            context.terrain_textures,
            input.hide_gui(),
            basics.entity_partial_tick,
        ));
    let camera_hurt_effect = camera_hurt_effect_from_world(
        world,
        basics.entity_partial_tick,
        context.damage_tilt_strength,
    );
    let camera_view_bob_effect =
        camera_view_bob_effect_from_world(world, basics.entity_partial_tick, context.view_bobbing);
    let camera_spinning_effect = state
        .camera_spinning_ticks
        .effect_for_world(world, basics.entity_partial_tick);
    // Vanilla `GameRenderer.extract` samples the current interpolated camera
    // once per rendered frame, after the client tick. FOV, color, fog, and sky
    // must consume this same `Camera.getFluidInCamera` result.
    let camera_fluid = camera_pose.map_or(CameraFluid::None, |camera| {
        camera_fluid_in_camera(
            world,
            camera,
            [
                basics.physical_surface_size.width,
                basics.physical_surface_size.height,
            ],
            view.camera_fov_ticks.base_degrees(),
        )
    });
    let camera_environment_colors = camera_environment_colors_from_cached_sample(
        context.camera_environment_samples,
        world,
        world.client_level_generation(),
        view.client_tick_index,
        basics.entity_partial_tick,
        camera_pose,
        camera_fluid,
    );
    let clear_color = clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        camera_environment_colors,
        basics.render_distance_chunks,
        basics.water_vision,
        basics.hide_lightning_flash,
    );
    let fog_environment = if input.debug_fog_enabled() {
        fog_environment_for_world_with_environment_colors(
            world,
            camera_environment_colors,
            basics.render_distance_chunks,
            context.cloud_range_chunks,
            basics.water_vision,
            basics.rain_fog_multiplier,
            basics.hide_lightning_flash,
        )
    } else {
        FogEnvironment::disabled()
    };
    let sky_environment = sky_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors,
        basics.hide_lightning_flash,
    );
    let camera_fov =
        view.camera_fov_ticks
            .frame(camera_hurt_effect, camera_fluid, basics.entity_partial_tick);
    let hud_locator_bar = hud_locator_bar_from_world(
        world,
        camera_pose,
        camera_fov.world_degrees,
        basics.entity_partial_tick,
    );
    let raw_contextual_bar = context.hud_render_runtime.contextual.select(
        world,
        world.client_level_generation(),
        view.client_tick_index,
        hud_locator_bar,
        hud.hud_jump_bar,
        hud.hud_experience_progress,
    );
    let hud_contextual_bar = context
        .hud_render_runtime
        .contextual_bar_update(raw_contextual_bar);
    // Selection and active state are one immutable simulation publication:
    // F5 may clear/reselect independently of the canonical camera entity, while
    // a later SetCamera refresh may select again even in third person.
    let current_post_effect_id = input.camera_post_effect_id();
    let terrain_visibility_context = TerrainVisibilityContext::new(
        camera_pose,
        [
            basics.physical_surface_size.width,
            basics.physical_surface_size.height,
        ],
        camera_fov.culling_degrees,
        basics.render_distance_chunks,
        input.debug_smart_cull_enabled(),
    );
    // Vanilla `LevelRenderer.cullTerrain` applies the asynchronously published
    // graph when it changes or camera rotation crosses a two-degree bucket.
    // The terrain owner additionally invalidates on viewport/FOV changes.
    context
        .terrain_upload
        .refresh_frame_visibility(terrain_visibility_context);
    CameraAndEnvironment {
        item_model_age_ticks,
        shader_game_time_ticks,
        frame_camera,
        camera_entity_pose,
        camera_pose,
        sign_text_outline_context,
        first_person_camera_pose,
        screen_effect_quads,
        hud_vignette,
        hud_spyglass_scope,
        hud_confusion_overlay,
        hud_portal_overlay,
        camera_hurt_effect,
        camera_view_bob_effect,
        camera_spinning_effect,
        clear_color,
        fog_environment,
        sky_environment,
        camera_fov_world_degrees: camera_fov.world_degrees,
        camera_fov_hud_degrees: camera_fov.hud_degrees,
        camera_fov_culling_degrees: camera_fov.culling_degrees,
        hud_contextual_bar,
        current_post_effect_id,
        terrain_visibility_context,
    }
}

struct ExtractedWorldScene {
    entity_instances: Vec<EntityModelInstance>,
    item_entity_billboards: Vec<ItemEntityBillboard>,
    item_model_residency: ItemModelResidencyFrame,
    first_person_player_arms: Vec<FirstPersonPlayerArm>,
    first_person_map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
    first_person_map_background_surfaces: Vec<FirstPersonMapBackgroundSurface>,
    first_person_map_textures: Vec<ItemFrameMapTexture>,
    first_person_map_surfaces: Vec<ItemFrameMapSurface>,
    first_person_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    first_person_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    first_person_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    hud_map_preview_textures: Vec<ItemFrameMapTexture>,
    hud_map_preview_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapSurface>>,
    hud_map_preview_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    hud_map_preview_decoration_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapDecorationSurface>>,
    hud_map_preview_text_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapTextSurface>>,
    item_frame_map_textures: Vec<ItemFrameMapTexture>,
    item_frame_map_surfaces: Vec<ItemFrameMapSurface>,
    item_frame_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    item_frame_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    item_frame_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    sign_text_surfaces: RendererFrameUpdate<Vec<SignTextSurface>>,
    visible_sign_text_positions: Vec<[i32; 3]>,
    display_text_surfaces: Vec<DisplayTextSurface>,
    entity_name_tag_surfaces: Vec<EntityNameTagSurface>,
    entity_flame_surfaces: Vec<EntityFlameSurface>,
    entity_shadow_surfaces: Vec<EntityShadowSurface>,
    hud_crosshair_attack_indicator: RendererFrameUpdate<HudCrosshairAttackIndicator>,
    hud_hotbar_attack_indicator: RendererFrameUpdate<HudHotbarAttackIndicator>,
    hud_crosshair_visible: RendererFrameUpdate<bool>,
    crosshair_pick: crate::crosshair::CrosshairPick,
    targeted_entity_id: Option<i32>,
    block_entity_visibility: BlockEntityVisibility,
    block_destroy_stages: Vec<(BlockPos, u8)>,
    block_entity_duration: Duration,
    item_duration: Duration,
}

fn extract_world_scene(
    view: &SimulationView<'_>,
    context: &mut SimulationRenderContext<'_>,
    basics: &FrameBasics,
    hud: &ExtractedHudAndScreens,
    camera: &CameraAndEnvironment<'_>,
    item_pre_started_at: Instant,
) -> ExtractedWorldScene {
    let input = view.input;
    let world = view.world;
    let item_runtime = context.item_runtime;
    let entity_extraction_started_at = Instant::now();
    let mut entity_instances =
        visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick(
            world,
            item_runtime,
            context.entity_variant_textures,
            basics.entity_partial_tick,
            EntityRenderVisibilityContext {
                world_revision: view.world_revision,
                camera_pose: camera.camera_pose,
                camera_detached: !input.camera_type().is_first_person(),
                surface_size: [
                    basics.physical_surface_size.width,
                    basics.physical_surface_size.height,
                ],
                camera_fov_degrees: camera.camera_fov_culling_degrees,
                base_camera_fov_degrees: view.camera_fov_ticks.base_degrees(),
                render_distance_chunks: basics.render_distance_chunks,
                entity_distance_scaling: context.entity_distance_scaling,
                terrain_upload: context.terrain_upload,
                entity_shadows: context.entity_shadows,
                shadow_sky_darken: view.frame.sky_darken,
            },
            context.entity_render_extraction,
        );
    let entity_extraction_duration = entity_extraction_started_at.elapsed();
    let dropped_item_states = context
        .entity_render_extraction
        .render_candidates()
        .iter()
        .filter(|candidate| {
            candidate.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID
        })
        .filter_map(|candidate| world.entity_item_stack(candidate.entity_id))
        .collect::<Vec<_>>();
    let dropped_item_models = dropped_item_models(
        world,
        &dropped_item_states,
        item_runtime,
        Some(context.renderer),
        Some(context.entity_variant_textures),
        camera.item_model_age_ticks,
        &hud.item_registry_keys,
    );
    let ominous_item_spawner_models = context
        .entity_render_extraction
        .render_candidates()
        .iter()
        .any(|candidate| {
            candidate.entity_type_id
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID
        })
        .then(|| {
            ominous_item_spawner_models(
                world,
                item_runtime,
                context.terrain_textures,
                Some(context.renderer),
                Some(context.entity_variant_textures),
                basics.entity_partial_tick,
                &hud.item_registry_keys,
            )
        });
    let item_pickup_particle_states =
        crate::particle_runtime::extract::item_pickup_particle_render_states(
            view.particles,
            basics.entity_partial_tick,
        );
    let item_pickup_particle_models = item_pickup_particle_item_models(
        world,
        &item_pickup_particle_states,
        item_runtime,
        context.terrain_textures,
        Some(context.renderer),
        Some(context.entity_variant_textures),
        &hud.item_registry_keys,
    );
    let item_entity_billboards = item_entity_billboards_from_world(
        world,
        context.entity_render_extraction.render_candidates(),
        &dropped_item_states,
        item_runtime,
        &dropped_item_models.handled_entity_ids,
        &hud.item_registry_keys,
    );
    let item_pre_duration = item_pre_started_at
        .elapsed()
        .saturating_sub(entity_extraction_duration);
    // Vanilla `LevelRenderer.extractVisibleEntities` submits generic name
    // displays from the same culled entity generation immediately after each
    // entity renderer extracts its model state. Camera distance/crosshair and
    // `Options.hideGui` are frame/input facts, so native resolves them here.
    let crosshair_pick = context.non_entity_render_extraction.crosshair_pick(
        world,
        view.world_revision,
        camera.camera_entity_pose,
    );
    let crosshair_target = crosshair_pick.target;
    let targeted_entity_id = match crosshair_target {
        Some(CrosshairTarget::Entity(hit)) => Some(hit.entity_id),
        _ => None,
    };
    let crosshair_allowed_for_game_mode = !world.local_player_is_spectator()
        || can_render_crosshair_for_spectator(world, crosshair_target);
    let attack_indicator_state =
        world.local_player_crosshair_attack_indicator_state(targeted_entity_id);
    let hud_crosshair_attack_indicator = context
        .hud_render_runtime
        .crosshair_attack_indicator_update(hud_crosshair_attack_indicator(
            input.camera_type().is_first_person()
                && crosshair_allowed_for_game_mode
                && context.attack_indicator == AttackIndicatorOption::Crosshair,
            attack_indicator_state,
        ));
    let hud_hotbar_attack_indicator =
        context
            .hud_render_runtime
            .hotbar_attack_indicator_update(hud_hotbar_attack_indicator(
                context.attack_indicator == AttackIndicatorOption::Hotbar,
                attack_indicator_state,
                world.local_player_main_arm_left().unwrap_or(false),
            ));
    let hud_crosshair_visible = context.hud_render_runtime.crosshair_visible_update(
        input.camera_type().is_first_person() && crosshair_allowed_for_game_mode,
    );
    let entity_name_tag_surfaces = entity_name_tag_surfaces(
        visible_entity_name_tag_render_sources(context.entity_render_extraction),
        &entity_instances,
        item_runtime,
        camera.camera_pose,
        targeted_entity_id,
        world.camera_entity_id(),
        !input.hide_gui(),
        bbb_client_render::text_obfuscation_seed(),
    );
    let entity_flame_surfaces = entity_flame_surfaces(
        &entity_instances,
        context.terrain_textures,
        camera.camera_pose,
    );
    let entity_shadow_surfaces = entity_shadow_surfaces(
        visible_entity_shadow_render_sources(context.entity_render_extraction),
        &entity_instances,
    );
    let display_entity_item_models = display_entity_item_models(
        visible_display_entity_render_sources_at_partial_tick(
            world,
            basics.entity_partial_tick,
            context.entity_render_extraction,
        ),
        world,
        item_runtime,
        context.terrain_textures,
        camera.camera_pose,
        bbb_client_render::text_obfuscation_seed(),
        &hud.item_registry_keys,
        Some(context.entity_variant_textures),
        context
            .non_entity_render_extraction
            .christmas_chest_textures()
            .enabled(),
    );
    let display_text_surfaces = display_entity_item_models.text_surfaces;
    let item_mid_started_at = Instant::now();
    let mut held_item_models = held_item_models_with_variant_textures(
        &entity_instances,
        world,
        item_runtime,
        context.terrain_textures,
        Some(context.entity_variant_textures),
        &hud.item_registry_keys,
    );
    let item_mid_duration = item_mid_started_at.elapsed();
    let block_entities = append_block_entity_instances(
        view,
        context,
        basics,
        camera,
        &hud.item_registry_keys,
        &mut entity_instances,
        &mut held_item_models.residency,
    );
    let item_tail_started_at = Instant::now();
    let active_inventory_screen = match &hud.hud_screens {
        RendererFrameUpdate::Replace(screens) => screens.hud_inventory_screen.as_ref(),
        RendererFrameUpdate::Unchanged => context.hud_render_runtime.cached_inventory_screen(),
    };
    let map_texture_cache = context
        .non_entity_render_extraction
        .begin_map_texture_frame();
    let hud_map_preview_models = hud_map_preview_models(
        world,
        item_runtime,
        active_inventory_screen,
        basics.gui_surface_size.width,
        basics.gui_surface_size.height,
        map_texture_cache,
    );
    // Vanilla `GameRenderer.renderItemInHand` samples the local player's visible arm state,
    // held stacks, frame light, and partial-tick attack swing during the first-person hand pass,
    // after the client tick has advanced input/use-item/entity animation state.
    let local_player_model_instance = context
        .entity_render_extraction
        .local_player_instance(world, &entity_instances);
    // Vanilla `ScreenEffectRenderer.renderScreenEffect` (`ScreenEffectRenderer.java:80-82`)
    // renders item-activation in any camera mode/sleep state when `!hideGui`, anchored to
    // the active render camera (`frame_camera.pose`).
    let activation_camera_pose = (!input.hide_gui()).then_some(camera.camera_pose).flatten();
    // Vanilla `renderItemActivationAnimation` spreads the totem's horizontal
    // offset by the framebuffer aspect (`windowRenderState.width / height`).
    let first_person_item_models = first_person_item_models_for_frame(
        world,
        view.first_person_hand_presentation,
        item_runtime,
        context.terrain_textures,
        Some(context.entity_variant_textures),
        &hud.item_registry_keys,
        map_texture_cache,
        camera.first_person_camera_pose,
        activation_camera_pose,
        local_player_model_instance,
        basics.entity_partial_tick,
        basics.physical_surface_size.width as f32
            / (basics.physical_surface_size.height.max(1)) as f32,
    );
    let first_person_player_arms = first_person_player_arms(
        world,
        view.first_person_hand_presentation,
        item_runtime,
        local_player_model_instance,
        camera.first_person_camera_pose,
        basics.entity_partial_tick,
    );
    // Item frames render their wooden border + framed item into the same two atlas draws.
    let item_frame_models = item_frame_models_with_map_cache(
        world,
        context.entity_render_extraction.render_candidates(),
        item_runtime,
        context.terrain_textures,
        map_texture_cache,
        &hud.item_registry_keys,
        Some(context.entity_variant_textures),
    );
    let entity_block_models = entity_block_models(
        &entity_instances,
        world,
        item_runtime,
        context.terrain_textures,
        basics.entity_partial_tick,
        MovingPistonVisibility::new(block_entities.visibility, context.terrain_upload),
        block_entities.christmas_chest_textures_enabled,
    );
    let smithing_preview_item_models = smithing_entity_preview_item_models(
        world,
        item_runtime,
        context.terrain_textures,
        Some(context.entity_variant_textures),
        active_inventory_screen,
        &hud.item_registry_keys,
    );
    let mut item_model_residency = dropped_item_models.residency;
    item_model_residency.append(display_entity_item_models.residency);
    if let Some(ominous_item_spawner_models) = ominous_item_spawner_models {
        item_model_residency.append(ominous_item_spawner_models.residency);
    }
    item_model_residency.append(item_pickup_particle_models.residency);
    item_model_residency.append(held_item_models.residency);
    item_model_residency.append(first_person_item_models.residency);
    item_model_residency.append(item_frame_models.residency);
    item_model_residency.append(entity_block_models);
    item_model_residency.append(smithing_preview_item_models);
    if block_entities.has_item_displays {
        let block_entity_visibility = block_entities.visibility;
        let terrain_upload = &*context.terrain_upload;
        item_model_residency.append(block_entity_item_models(
            world,
            item_runtime,
            Some(context.renderer),
            &hud.item_registry_keys,
            Some(context.entity_variant_textures),
            basics.entity_partial_tick,
            |pos, view_distance| {
                block_entity_visibility.block_position_is_visible(
                    pos,
                    view_distance,
                    world,
                    terrain_upload,
                )
            },
        ));
    }
    let item_duration = item_pre_duration + item_mid_duration + item_tail_started_at.elapsed();
    ExtractedWorldScene {
        entity_instances,
        item_entity_billboards,
        item_model_residency,
        first_person_player_arms,
        first_person_map_background_textures: first_person_item_models.map_background_textures,
        first_person_map_background_surfaces: first_person_item_models.map_background_surfaces,
        first_person_map_textures: first_person_item_models.map_textures,
        first_person_map_surfaces: first_person_item_models.map_surfaces,
        first_person_map_decoration_textures: first_person_item_models.map_decoration_textures,
        first_person_map_decoration_surfaces: first_person_item_models.map_decoration_surfaces,
        first_person_map_text_surfaces: first_person_item_models.map_text_surfaces,
        hud_map_preview_textures: hud_map_preview_models.textures,
        hud_map_preview_surfaces: hud_map_preview_models.surfaces,
        hud_map_preview_decoration_textures: hud_map_preview_models.decoration_textures,
        hud_map_preview_decoration_surfaces: hud_map_preview_models.decoration_surfaces,
        hud_map_preview_text_surfaces: hud_map_preview_models.text_surfaces,
        item_frame_map_textures: item_frame_models.map_textures,
        item_frame_map_surfaces: item_frame_models.map_surfaces,
        item_frame_map_decoration_textures: item_frame_models.map_decoration_textures,
        item_frame_map_decoration_surfaces: item_frame_models.map_decoration_surfaces,
        item_frame_map_text_surfaces: item_frame_models.map_text_surfaces,
        sign_text_surfaces: block_entities.sign_text_surfaces,
        visible_sign_text_positions: block_entities.visible_sign_text_positions,
        display_text_surfaces,
        entity_name_tag_surfaces,
        entity_flame_surfaces,
        entity_shadow_surfaces,
        hud_crosshair_attack_indicator,
        hud_hotbar_attack_indicator,
        hud_crosshair_visible,
        crosshair_pick,
        targeted_entity_id,
        block_entity_visibility: block_entities.visibility,
        block_destroy_stages: block_entities.block_destroy_stages,
        block_entity_duration: block_entities.duration,
        item_duration,
    }
}

struct BlockEntityExtraction {
    visibility: BlockEntityVisibility,
    block_destroy_stages: Vec<(BlockPos, u8)>,
    sign_text_surfaces: RendererFrameUpdate<Vec<SignTextSurface>>,
    visible_sign_text_positions: Vec<[i32; 3]>,
    has_item_displays: bool,
    christmas_chest_textures_enabled: bool,
    duration: Duration,
}

fn append_block_entity_instances(
    view: &SimulationView<'_>,
    context: &mut SimulationRenderContext<'_>,
    basics: &FrameBasics,
    camera: &CameraAndEnvironment<'_>,
    item_registry_keys: &ItemRegistryKeySet,
    entity_instances: &mut Vec<EntityModelInstance>,
    held_item_residency: &mut ItemModelResidencyFrame,
) -> BlockEntityExtraction {
    let world = view.world;
    let input = view.input;
    let item_runtime = context.item_runtime;
    let terrain_upload = &*context.terrain_upload;
    let block_entity_started_at = Instant::now();
    let block_destroy_stages = world.block_destruction_render_stages();
    let block_entity_model_start = entity_instances.len();
    let block_entity_visibility = BlockEntityVisibility::new(
        camera.camera_pose,
        [
            basics.physical_surface_size.width,
            basics.physical_surface_size.height,
        ],
        camera.camera_fov_culling_degrees,
        basics.render_distance_chunks,
    );
    let block_entity_source_is_visible = |pos, view_distance| {
        block_entity_visibility.block_position_is_visible(pos, view_distance, world, terrain_upload)
    };
    let christmas_chest_textures = context
        .non_entity_render_extraction
        .christmas_chest_textures();
    let christmas_chest_textures_enabled = christmas_chest_textures.enabled();
    let block_entity_revision = world.stable_block_entity_render_revision();
    let dynamic_block_entities = context
        .non_entity_render_extraction
        .dynamic_block_entity_presence(world, block_entity_revision, camera.camera_pose);
    let StableBlockEntityFrame {
        sign_instances,
        bed_instances,
        lectern_instances,
        sign_text_surfaces,
    } = context
        .non_entity_render_extraction
        .stable_block_entity_frame(
            StableBlockEntityCacheKey {
                revision: block_entity_revision,
                text_filtering_enabled: input.client_text_filtering_enabled(),
            },
            || {
                let sign_scene = sign_scene_from_world(
                    world,
                    item_runtime,
                    input.client_text_filtering_enabled(),
                );
                (
                    sign_scene.instances,
                    sign_scene.text_surfaces,
                    bed_model_instances_from_world(world),
                    lectern_book_model_instances_from_world(world),
                )
            },
        );
    let visible_sign_text_positions = sign_instances
        .iter()
        .filter_map(|instance| {
            block_entity_visibility.visible_block_position(instance, world, terrain_upload)
        })
        .collect();
    // Chest block-entity models join the same single entity-model submission
    // stream after held-item baking (they have no hands to bake).
    if !dynamic_block_entities.chest_columns.is_empty() {
        entity_instances.extend(chest_model_instances_from_world_at_partial_tick(
            world,
            &dynamic_block_entities.chest_columns,
            basics.entity_partial_tick,
            christmas_chest_textures,
            block_entity_visibility,
            terrain_upload,
        ));
    }
    // Sign block-entity models ride the same stream; their face text bakes
    // into world-space glyph quads drawn with the map-label font atlas.
    entity_instances.extend(
        sign_instances
            .iter()
            .filter(|instance| block_entity_visibility.is_visible(instance, world, terrain_upload))
            .copied(),
    );
    // Bed and bell block-entity models ride the same stream: beds are static
    // per-half meshes, bells carry the shake ticks for the renderer-side
    // `BellModel.setupAnim` swing.
    entity_instances.extend(
        bed_instances
            .iter()
            .filter(|instance| block_entity_visibility.is_visible(instance, world, terrain_upload))
            .copied(),
    );
    if dynamic_block_entities.bell {
        entity_instances.extend(bell_model_instances_from_world_at_partial_tick(
            world,
            basics.entity_partial_tick,
            block_entity_source_is_visible,
        ));
    }
    // Shulker box and decorated pot block-entity models ride the same stream:
    // boxes carry the lerped lid progress, pots their sherd patterns and the
    // wobble style/progress.
    if dynamic_block_entities.shulker_box {
        entity_instances.extend(shulker_box_model_instances_from_world_at_partial_tick(
            world,
            basics.entity_partial_tick,
            block_entity_source_is_visible,
        ));
    }
    if dynamic_block_entities.decorated_pot {
        entity_instances.extend(decorated_pot_model_instances_from_world_at_partial_tick(
            world,
            basics.entity_partial_tick,
            block_entity_visibility,
            terrain_upload,
        ));
    }
    // Banners carry their base color, tinted pattern stack, and flag swing
    // phase.
    if dynamic_block_entities.banner {
        entity_instances.extend(banner_model_instances_from_world_at_partial_tick(
            world,
            basics.entity_partial_tick,
            Some(context.entity_variant_textures),
            block_entity_source_is_visible,
        ));
    }
    // The enchanting-table hovering book (animated, nearest-player-facing) and
    // the lectern's static open book share the same `BookModel` in the stream.
    if dynamic_block_entities.enchanting_table {
        entity_instances.extend(
            enchanting_table_book_model_instances_from_world_at_partial_tick(
                world,
                basics.entity_partial_tick,
                block_entity_source_is_visible,
            ),
        );
    }
    entity_instances.extend(
        lectern_instances
            .iter()
            .filter(|instance| block_entity_visibility.is_visible(instance, world, terrain_upload))
            .copied(),
    );
    if dynamic_block_entities.conduit {
        entity_instances.extend(conduit_model_instances_from_world_at_partial_tick(
            world,
            camera.camera_pose,
            basics.entity_partial_tick,
            block_entity_source_is_visible,
        ));
    }
    if dynamic_block_entities.copper_golem_statue {
        entity_instances.extend(copper_golem_statue_model_instances_from_world(
            world,
            block_entity_source_is_visible,
        ));
    }
    entity_instances.extend(beacon_beam_model_instances_from_world_at_partial_tick(
        world,
        camera.camera_pose,
        block_entity_visibility,
        basics.entity_partial_tick,
    ));
    if dynamic_block_entities.end_portal {
        entity_instances.extend(end_portal_model_instances_from_world_at_partial_tick(
            world,
            basics.entity_partial_tick,
            block_entity_source_is_visible,
        ));
    }
    if dynamic_block_entities.skull {
        entity_instances.extend(skull_model_instances_from_world_at_partial_tick(
            world,
            basics.entity_partial_tick,
            item_runtime,
            block_entity_source_is_visible,
        ));
    }
    if dynamic_block_entities.spawner {
        for submission in spawner_display_entity_submissions_from_world_at_partial_tick(
            world,
            &dynamic_block_entities.spawner_columns,
            item_runtime,
            Some(context.entity_variant_textures),
            basics.entity_partial_tick,
            block_entity_visibility,
            terrain_upload,
        ) {
            if let Some(entity_world) = submission.entity_world.as_ref() {
                held_item_residency.append(
                    held_item_models_with_variant_textures(
                        std::slice::from_ref(&submission.instance),
                        entity_world,
                        item_runtime,
                        context.terrain_textures,
                        Some(context.entity_variant_textures),
                        item_registry_keys,
                    )
                    .residency,
                );
            }
            entity_instances.push(submission.instance);
        }
    }
    retain_visible_block_entity_instances(
        entity_instances,
        block_entity_model_start,
        block_entity_visibility,
        world,
        terrain_upload,
    );
    apply_block_entity_crumbling(
        &mut entity_instances[block_entity_model_start..],
        &block_destroy_stages,
        context.terrain_textures,
    );
    let duration = block_entity_started_at.elapsed();
    BlockEntityExtraction {
        visibility: block_entity_visibility,
        block_destroy_stages,
        sign_text_surfaces,
        visible_sign_text_positions,
        has_item_displays: dynamic_block_entities.item_displays,
        christmas_chest_textures_enabled,
        duration,
    }
}

struct ExtractedWorldEffectsAndDebug {
    cloud_frame: CloudFrame,
    weather_render_state: WeatherRenderState,
    world_border_render_state: WorldBorderRenderState,
    block_selection: Option<BlockSelectionRenderState>,
    chunk_border_outline: Option<SelectionOutline>,
    entity_scene_outline: Option<SelectionOutline>,
    debug_filled_boxes: Vec<DebugFilledBox>,
    debug_filled_rects: Vec<DebugFilledRect>,
    debug_text_billboards: Vec<DebugTextBillboard>,
    debug_lines: Vec<DebugLine>,
    game_test_highlight_boxes: RendererFrameUpdate<Vec<DebugFilledBox>>,
    game_test_highlight_text_billboards: RendererFrameUpdate<Vec<DebugTextBillboard>>,
    block_destroy_overlays: RendererFrameUpdate<Vec<BlockDestroyOverlay>>,
    hud_debug_overlay: RendererFrameUpdate<Option<HudDebugOverlay>>,
    world_debug_duration: Duration,
}

fn extract_world_effects_and_debug(
    view: &SimulationView<'_>,
    context: &mut SimulationRenderContext<'_>,
    basics: &FrameBasics,
    camera: &CameraAndEnvironment<'_>,
    scene: &ExtractedWorldScene,
    hud_duration: &mut Duration,
) -> ExtractedWorldEffectsAndDebug {
    let input = view.input;
    let world = view.world;
    let world_debug_started_at = Instant::now();
    // Vanilla `LevelRenderer.renderLevel` samples `level.getGameTime()`, camera position, and the
    // frame partial tick for the cloud pass after the client tick has advanced the level clock.
    let cloud_frame = cloud_frame_for_world(world, camera.camera_pose, basics.entity_partial_tick);
    // Vanilla `LevelRenderer.extractLevel` calls `WeatherEffectRenderer.extractRenderState` with
    // level ticks, `deltaPartialTick`, and camera position during render extraction.
    let weather_render_state = weather_render_state_for_world_with_radius(
        world,
        context.terrain_textures,
        camera.camera_pose,
        basics.entity_partial_tick,
        context.weather_radius,
    );
    // Vanilla `LevelRenderer` extraction runs `worldBorderRenderer.extract` in
    // the "border" profiler section right after the weather/sky extraction
    // after `ClientLevel.tick` has advanced the
    // border lerp above; the forcefield UV scroll samples `Util.getMillis()`
    let world_border_render_state = world_border_render_state_for_world(
        world,
        camera.camera_pose,
        basics.render_distance_chunks,
        basics.depth_far,
        basics.entity_partial_tick,
        wall_clock_millis(),
    );
    // Vanilla `Minecraft.renderFrame` calls `pick(partialTicks)` before
    // `GameRenderer.extract`; block/entity outline extraction reads that post-input camera state.
    let block_selection = (!input.hide_gui())
        .then(|| block_selection_from_hit(world, scene.crosshair_pick.block_hit))
        .flatten()
        .map(|selection| {
            block_selection_render_state(
                world,
                context.terrain_textures,
                selection,
                basics.physical_surface_size.width,
                context.high_contrast_block_outline,
            )
        });
    let chunk_border_outline = debug_chunk_border_outline(input, world, camera.camera_pose);
    let entity_scene_outline = debug_entity_scene_outline(input, world, basics.entity_partial_tick);
    let remote_debug_selection = input.remote_debug_renderers();
    let selected_remote_debug_entity = context
        .remote_debug_selection_cache
        .selected_for_generation(world.client_level_generation());
    let targeted_remote_debug_entity = (remote_debug_selection
        .contains(crate::remote_debug::RemoteDebugRenderer::Brain)
        && !world.local_player_is_spectator())
    .then(|| {
        debug_targeted_entity_from_camera(world, camera.camera_entity_pose)
            .and_then(|id| world.probe_entity_transform(id))
            .map(|entity| entity.uuid)
    })
    .flatten();
    let remote_debug_gizmos = remote_debug_gizmos(
        world,
        remote_debug_selection,
        camera.camera_pose,
        selected_remote_debug_entity,
    );
    context.remote_debug_selection_cache.finish_frame(
        remote_debug_selection,
        world.local_player_is_spectator(),
        targeted_remote_debug_entity,
    );
    let (game_test_highlight_boxes, game_test_highlight_text_billboards) = context
        .non_entity_render_extraction
        .game_test_highlight_update(
            world.client_level_generation(),
            world.game_test_highlight_render_revision(),
            || game_test_highlight_gizmos(world),
        );
    let reduced_debug_info = input.show_only_reduced_info(world);
    let chunk_section_debug_gizmos = context.terrain_upload.chunk_section_debug_gizmos(
        camera.terrain_visibility_context,
        input.debug_screen_entry_enabled(DebugScreenEntryId::ChunkSectionPaths, reduced_debug_info),
        input
            .debug_screen_entry_enabled(DebugScreenEntryId::ChunkSectionOctree, reduced_debug_info),
        input.debug_screen_entry_enabled(
            DebugScreenEntryId::ChunkSectionVisibility,
            reduced_debug_info,
        ),
    );
    let mut debug_buffers = context.non_entity_render_extraction.begin_debug_frame();
    // `BlockEntityWithBoundingBoxRenderer` submits its gizmos from the
    // block-entity pass, which vanilla runs before `DebugRenderer.render`.
    debug_buffers.lines.extend(bounding_box_gizmo_lines(
        world,
        scene.block_entity_visibility,
    ));
    let test_instance_error_gizmos = test_instance_error_gizmos(world);
    debug_buffers
        .filled_boxes
        .extend(test_instance_error_gizmos.filled_boxes);
    debug_buffers
        .text_billboards
        .extend(test_instance_error_gizmos.text_billboards);
    // `DebugRenderer.refreshRendererList` registers `OctreeDebugRenderer`
    // before water/heightmap/collision renderers.
    debug_buffers
        .lines
        .extend(chunk_section_debug_gizmos.octree_lines);
    debug_buffers
        .text_billboards
        .extend(chunk_section_debug_gizmos.octree_text_billboards);
    // Vanilla `DebugRenderer.refreshRendererList` registers WaterDebugRenderer
    // before HeightMapRenderer, so their shared debugFilledBox submissions keep
    // that renderer order when both custom entries are enabled.
    let debug_water_samples = debug_water_samples(input, world);
    debug_buffers
        .filled_boxes
        .extend(debug_water_boxes(&debug_water_samples));
    debug_buffers
        .filled_boxes
        .extend(debug_heightmap_boxes(input, world, camera.camera_pose));
    debug_buffers
        .filled_boxes
        .extend(remote_debug_gizmos.filled_boxes);
    debug_buffers
        .filled_rects
        .extend(debug_solid_face_rects(input, world, camera.camera_pose));
    debug_buffers
        .text_billboards
        .extend(debug_water_amount_billboards(&debug_water_samples));
    debug_buffers
        .text_billboards
        .extend(debug_light_level_billboards(
            input,
            world,
            camera.camera_pose,
        ));
    debug_buffers
        .text_billboards
        .extend(debug_selection_text_billboards(
            entity_scene_outline.as_ref(),
        ));
    debug_buffers
        .text_billboards
        .extend(remote_debug_gizmos.text_billboards);
    debug_buffers.lines.extend(
        context
            .debug_collision_cache
            .lines(input, world, basics.now),
    );
    debug_buffers.lines.extend(
        context
            .debug_supporting_block_cache
            .lines(input, world, basics.now),
    );
    let (debug_sky_light_section_rects, debug_sky_light_section_lines) = context
        .debug_sky_light_section_cache
        .gizmos(input, world, basics.now);
    debug_buffers
        .filled_rects
        .extend(debug_sky_light_section_rects);
    debug_buffers.lines.extend(debug_sky_light_section_lines);
    debug_buffers.lines.extend(remote_debug_gizmos.lines);
    // `ChunkCullingDebugRenderer` is always the final Vanilla debug renderer.
    debug_buffers
        .filled_boxes
        .extend(chunk_section_debug_gizmos.culling_filled_boxes);
    debug_buffers
        .lines
        .extend(chunk_section_debug_gizmos.culling_lines);
    // Vanilla `LevelRenderer.extractBlockDestroyAnimation` reads block-breaking state during
    // render extract, after the client tick; local destroy overlay ticks are advanced above.
    let block_destroy_overlays = if basics.stable_world_unchanged {
        RendererFrameUpdate::Unchanged
    } else {
        RendererFrameUpdate::Replace(block_destroy_overlays_from_stages(
            world,
            context.terrain_textures,
            &scene.block_destroy_stages,
            camera.camera_pose,
        ))
    };
    let world_debug_duration = world_debug_started_at.elapsed();
    context
        .non_entity_render_extraction
        .record_debug_capacity_growths(&debug_buffers);
    // Vanilla `GameRenderer.extract` completes the full `LevelRenderer.extractLevel` projection
    // before GUI extraction. The entity statistics entry therefore reads the exact survivor count
    // from this frame, not the total count or a previous-frame cache.
    let hud_debug_started_at = Instant::now();
    let hud_debug_profiler_chart = context.hud_debug_profiler_chart.take();
    let hud_info_enabled = context.extensions.hud_info && !input.hide_gui();
    let debug_can_be_cached_empty = !input.debug_overlay_visible()
        && !hud_info_enabled
        && hud_debug_profiler_chart.is_none()
        && !matches!(input.current_screen(), Some(Screen::GameModeSwitcher(_)));
    let hud_debug_overlay = context.hud_render_runtime.debug_overlay_update(
        debug_can_be_cached_empty,
        (view.frame.step_generation, view.world_revision),
        || {
            let hud_audio_counters = context
                .audio_events
                .as_deref()
                .filter(|_| {
                    input.debug_screen_entry_enabled(
                        DebugScreenEntryId::SoundCache,
                        reduced_debug_info,
                    ) || input.debug_screen_entry_enabled(
                        DebugScreenEntryId::SoundMood,
                        reduced_debug_info,
                    )
                })
                .map(AudioEventSink::counters);
            hud_debug_overlay(HudDebugOverlayInputs {
                input,
                world,
                visible_entity_render_state_count: context
                    .entity_render_extraction
                    .visible_entity_render_state_count(),
                camera_pose: camera.camera_pose,
                interaction_camera_pose: camera.camera_entity_pose,
                targeted_entity_id: scene.targeted_entity_id,
                entity_partial_tick: basics.entity_partial_tick,
                item_runtime: context.item_runtime,
                terrain_textures: context.terrain_textures,
                renderer_counters: context.renderer.counters_ref(),
                gpu_utilization: context.renderer.gpu_utilization(),
                improved_transparency: context.improved_transparency,
                cloud_shape: context.renderer.cloud_shape(),
                texture_filtering: context.renderer.terrain_texture_filtering(),
                max_anisotropy: context.renderer.terrain_max_anisotropy(),
                render_distance_chunks: basics.render_distance_chunks,
                surface_size: basics.gui_surface_size,
                physical_surface_size: basics.physical_surface_size,
                fps_sampler: context.hud_debug_fps_sampler,
                client_framerate_limit: context.client_framerate_limit,
                client_vsync: context.client_vsync,
                network_sampler: view.hud_debug_network_sampler,
                tps_sampler: view.hud_debug_tps_sampler,
                net_counters: view.net_counters,
                audio_counters: hud_audio_counters.as_ref().unwrap_or(view.audio_counters),
                current_post_effect_id: camera.current_post_effect_id,
                profiler_chart: input
                    .debug_profiler_chart_visible()
                    .then_some(hud_debug_profiler_chart)
                    .flatten(),
                hud_info_enabled,
            })
        },
    );
    *hud_duration += hud_debug_started_at.elapsed();
    let DebugRenderExtractionBuffers {
        filled_boxes: debug_filled_boxes,
        filled_rects: debug_filled_rects,
        text_billboards: debug_text_billboards,
        lines: debug_lines,
        ..
    } = debug_buffers;
    ExtractedWorldEffectsAndDebug {
        cloud_frame,
        weather_render_state,
        world_border_render_state,
        block_selection,
        chunk_border_outline,
        entity_scene_outline,
        debug_filled_boxes,
        debug_filled_rects,
        debug_text_billboards,
        debug_lines,
        game_test_highlight_boxes,
        game_test_highlight_text_billboards,
        block_destroy_overlays,
        hud_debug_overlay,
        world_debug_duration,
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_renderer_frame(
    view: &SimulationView<'_>,
    context: &mut SimulationRenderContext<'_>,
    basics: &FrameBasics,
    hud: ExtractedHudAndScreens,
    camera: CameraAndEnvironment<'_>,
    scene: ExtractedWorldScene,
    effects: ExtractedWorldEffectsAndDebug,
    hud_duration: Duration,
) {
    let world = view.world;
    let input = view.input;
    let camera_chunk = camera.camera_pose.map(|pose| ChunkPos {
        x: (pose.position[0].floor() as i32).div_euclid(16),
        z: (pose.position[2].floor() as i32).div_euclid(16),
    });
    context.lod_terrain.update(
        crate::lod_terrain::LodTerrainView {
            lod_distance_chunks: context.lod_distance_chunks,
            render_distance_chunks: basics.render_distance_chunks,
            camera_chunk,
            terrain_textures_epoch: basics.terrain_textures_epoch,
        },
        world,
        context.terrain_textures,
        context.terrain_upload,
        context.renderer,
    );
    let dynamic_lights = crate::dynamic_lights::extract_dynamic_point_lights_with_occluded(
        context.dynamic_lights_mode,
        world,
        context.item_runtime,
        &scene.entity_instances,
        context.entity_render_extraction.occluded_candidates(),
        camera.camera_pose,
    );
    context
        .renderer
        .set_dynamic_lights(context.dynamic_lights_mode, dynamic_lights);
    let apply_started_at = Instant::now();
    let recycled_debug_buffers = apply_renderer_frame(
        context.renderer,
        RendererFrame {
            input_latency: context
                .input_latency
                .map(|input_latency| RendererInputLatencyFrame {
                    sequence: input_latency.sequence,
                    enqueued_at: input_latency.enqueued_at,
                    simulation_applied_at: input_latency.simulation_applied_at,
                    superseded_events: input_latency.superseded_events,
                }),
            lightmap_environment: basics.lightmap_environment,
            clear_color: camera.clear_color,
            fog_environment: camera.fog_environment,
            sky_environment: camera.sky_environment,
            cloud_environment: basics.cloud_environment,
            hud_static_ui: hud.hud_static_ui,
            hud_crosshair_visible: scene.hud_crosshair_visible,
            hud_crosshair_attack_indicator: scene.hud_crosshair_attack_indicator,
            hud_player_health: hud.hud_player_health,
            hud_vignette: camera.hud_vignette,
            screen_effect_quads: camera.screen_effect_quads,
            hud_spyglass_scope: camera.hud_spyglass_scope,
            hud_portal_overlay: camera.hud_portal_overlay,
            hud_confusion_overlay: camera.hud_confusion_overlay,
            hud_contextual_bar: camera.hud_contextual_bar,
            hud_hotbar_mode: hud.hud_hotbar_mode,
            hud_selected_item_name: hud.hud_selected_item_name,
            hud_hotbar_items: hud.hud_hotbar_items,
            hud_offhand_block_item_model: hud.hud_offhand_block_item_model,
            hud_hotbar_attack_indicator: scene.hud_hotbar_attack_indicator,
            hud_hotbar_block_item_models: hud.hud_hotbar_block_item_models,
            hud_screens: hud.hud_screens,
            hud_toasts: hud.hud_toasts,
            hud_now_playing_toast: hud.hud_now_playing_toast,
            hud_action_bar_text: hud.hud_action_bar_text,
            hud_title_text: hud.hud_title_text,
            hud_scoreboard_sidebar: hud.hud_scoreboard_sidebar,
            hud_player_list: hud.hud_player_list,
            hud_player_list_tick_count: hud.hud_player_list_tick_count,
            hud_chat_messages: hud.hud_chat_messages,
            hud_debug_overlay: effects.hud_debug_overlay,
            hud_boss_bars: hud.hud_boss_bars,
            item_entity_billboards: scene.item_entity_billboards,
            item_model_residency: scene.item_model_residency,
            first_person_player_arms: scene.first_person_player_arms,
            first_person_map_background_textures: scene.first_person_map_background_textures,
            first_person_map_background_surfaces: scene.first_person_map_background_surfaces,
            first_person_map_textures: scene.first_person_map_textures,
            first_person_map_surfaces: scene.first_person_map_surfaces,
            first_person_map_decoration_textures: scene.first_person_map_decoration_textures,
            first_person_map_decoration_surfaces: scene.first_person_map_decoration_surfaces,
            first_person_map_text_surfaces: scene.first_person_map_text_surfaces,
            hud_map_preview_textures: scene.hud_map_preview_textures,
            hud_map_preview_surfaces: scene.hud_map_preview_surfaces,
            hud_map_preview_decoration_textures: scene.hud_map_preview_decoration_textures,
            hud_map_preview_decoration_surfaces: scene.hud_map_preview_decoration_surfaces,
            hud_map_preview_text_surfaces: scene.hud_map_preview_text_surfaces,
            item_frame_map_textures: scene.item_frame_map_textures,
            item_frame_map_surfaces: scene.item_frame_map_surfaces,
            item_frame_map_decoration_textures: scene.item_frame_map_decoration_textures,
            item_frame_map_decoration_surfaces: scene.item_frame_map_decoration_surfaces,
            item_frame_map_text_surfaces: scene.item_frame_map_text_surfaces,
            sign_text_surfaces: scene.sign_text_surfaces,
            visible_sign_text_positions: scene.visible_sign_text_positions,
            sign_text_outline_context: camera.sign_text_outline_context,
            display_text_surfaces: scene.display_text_surfaces,
            entity_name_tag_surfaces: scene.entity_name_tag_surfaces,
            entity_flame_surfaces: scene.entity_flame_surfaces,
            entity_shadow_surfaces: scene.entity_shadow_surfaces,
            entity_model_instances: scene.entity_instances,
            // Vanilla consumes the asynchronously published
            // `SectionOcclusionGraph.currentGraph` during frame visibility
            // extraction (`SectionOcclusionGraph.addSectionsInFrustum`).
            terrain_sections_layout_revision: context
                .terrain_upload
                .renderer_sections_layout_revision(),
            terrain_sections_revision: context.terrain_upload.renderer_sections_revision(),
            terrain_sections: context.terrain_upload.renderer_sections(),
            terrain_fading_sections: context.terrain_upload.renderer_fading_sections(),
            camera_pose: camera.camera_pose,
            camera_hurt_effect: camera.camera_hurt_effect,
            camera_view_bob_effect: camera.camera_view_bob_effect,
            camera_spinning_effect: camera.camera_spinning_effect,
            camera_fov_degrees: camera.camera_fov_world_degrees,
            camera_hud_fov_degrees: camera.camera_fov_hud_degrees,
            current_post_effect_id: camera.current_post_effect_id,
            camera_post_effect_active: input.camera_post_effect_active(),
            shader_game_time_ticks: camera.shader_game_time_ticks,
            cloud_frame: effects.cloud_frame,
            weather_render_state: effects.weather_render_state,
            world_border_render_state: effects.world_border_render_state,
            block_selection: effects.block_selection,
            chunk_border_outline: effects.chunk_border_outline,
            entity_scene_outline: effects.entity_scene_outline,
            debug_filled_boxes: effects.debug_filled_boxes,
            debug_filled_rects: effects.debug_filled_rects,
            debug_text_billboards: effects.debug_text_billboards,
            debug_lines: effects.debug_lines,
            game_test_highlight_boxes: effects.game_test_highlight_boxes,
            game_test_highlight_text_billboards: effects.game_test_highlight_text_billboards,
            block_destroy_overlays: effects.block_destroy_overlays,
        },
        context.entity_render_extraction,
    );
    context
        .non_entity_render_extraction
        .recycle_debug_buffers(recycled_debug_buffers);
    // `LevelRenderer.extractLevel` → `ParticleEngine.extract` with the frame's
    // camera and partial tick.
    let particle_occluder = camera.frame_camera.map(|frame_camera| {
        crate::occlusion_culling::OpaqueTerrainOccluder::new(
            world,
            context.terrain_upload,
            frame_camera.eye,
        )
    });
    crate::particle_runtime::extract::extract_particles_to_renderer(
        context.renderer,
        view.particles,
        view.particle_runtime,
        view.world_revision,
        context.terrain_upload.renderer_sections_revision(),
        basics.entity_partial_tick,
        camera
            .frame_camera
            .map(|frame_camera| (frame_camera.pose, frame_camera.eye)),
        camera.camera_fov_culling_degrees,
        basics.depth_far,
        |min, max| {
            particle_occluder
                .as_ref()
                .is_some_and(|occluder| occluder.is_aabb_occluded(min, max))
        },
        |position| particle_light_for_world(world, position),
    );
    let chat_scroll_page = context.renderer.hud_chat_scroll_page();
    if context
        .hud_render_runtime
        .chat_scroll_page
        .replace(chat_scroll_page)
        != Some(chat_scroll_page)
    {
        context.commands.push(
            crate::simulation::commands::SimulationCommand::HudChatScrollPage(chat_scroll_page),
        );
    }
    if let Some(appended) = context.renderer.take_hud_chat_appended_lines() {
        context
            .commands
            .push(crate::simulation::commands::SimulationCommand::HudChatAppendedLines(appended));
    }
    context.non_entity_render_extraction.record_frame(
        hud_duration,
        scene.item_duration,
        scene.block_entity_duration,
        effects.world_debug_duration,
        apply_started_at.elapsed(),
    );
    maybe_upload_terrain_texture_animation(
        context.renderer,
        context.terrain_upload,
        context.terrain_textures,
    );
    maybe_upload_decoded_terrain(
        world,
        context.renderer,
        context.terrain_upload,
        context.terrain_textures,
        camera.terrain_visibility_context,
        context.prioritize_chunk_updates,
        terrain_camera_position(world),
    );
    let player_section_compiled_and_visible = context
        .terrain_upload
        .is_player_section_compiled_and_visible(world);
    if let Some((level_generation, level_load_start, player_section_compiled_and_visible)) =
        context.terrain_upload.take_readiness_edge(
            world.client_level_generation(),
            world.level_load().starts,
            player_section_compiled_and_visible,
        )
    {
        context.commands.push(
            crate::simulation::commands::SimulationCommand::TerrainReadiness {
                level_generation,
                level_load_start,
                player_section_compiled_and_visible,
            },
        );
    }
}
