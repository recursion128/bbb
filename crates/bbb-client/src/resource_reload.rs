use std::collections::{BTreeMap, BTreeSet};

use crate::control::{AudioCounters, ClientResourceDiagnostics, ClientResourcePackDiagnostic};
use anyhow::{Context, Result};
use bbb_client_assets::{
    FurnaceFuelCatalog, FurnaceFuelSourceRule, PackRoots, PanoramaCatalog,
    PeriodicNotificationCatalog, PostEffectCatalog, SelectedClientPack,
    MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES, MAX_RESOURCE_DATA_PROCESS_LOGICAL_BYTES,
    MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES, MAX_RESOURCE_IMAGE_PROCESS_RGBA_BYTES,
};
use bbb_client_render::item_model::{
    font::{
        load_hud_font_atlas_with_options_and_max_dimension, providers::FontOptions,
        HudFontAtlasImage,
    },
    NativeItemRuntime,
};
use bbb_client_render::{ParticleSpriteUv, ParticleUvRect};
use bbb_client_world::{CartographyAdditionalItemKind, FurnaceFuelRule, WorldStore};

use crate::terrain_upload::TerrainUploadState;
use crate::{
    audio_runtime::{AudioEventSink, NativeAudioReload, NativeAudioRuntime},
    entity_assets::{
        prepare_entity_model_textures, upload_entity_model_textures, StagedEntityModelTextures,
    },
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    hud_assets::{try_load_hud_textures, upload_panorama},
    particle_runtime::{
        NativeParticleRenderRuntime, NativeParticleResources, NativeParticleRuntime,
    },
    simulation::effects::ResourceReloadEffect,
    sky_assets::try_load_sky_textures,
    win_screen_resources::WinScreenResources,
};
use bbb_client_render::scene::terrain_textures::{try_load_terrain_textures, TerrainTextureState};

enum PreparedAudioReload {
    Existing(NativeAudioReload),
    Disabled,
}

struct PreparedNativeResourceReload {
    roots: PackRoots,
    font_atlas: HudFontAtlasImage,
    audio: PreparedAudioReload,
    particles: NativeParticleResources,
    items: NativeItemRuntime,
    post_effects: PostEffectCatalog,
    panorama: PanoramaCatalog,
    periodic_notifications: PeriodicNotificationCatalog,
    win_screen_resources: WinScreenResources,
    entity_textures: StagedEntityModelTextures,
}

pub(crate) fn prepare_native_shader_resources(roots: &PackRoots) -> Result<PostEffectCatalog> {
    PostEffectCatalog::load(roots).context("prepare shader resources for reload")
}

/// World-facing results of a committed reload, applied by the next step
/// together with the resource pack status it acknowledges.
pub(crate) struct ResourceReloadSimulationCommit {
    pub(crate) item_defaults: ItemRuntimeWorldDefaults,
    pub(crate) available_command_sounds: Vec<String>,
    pub(crate) win_screen_resources: WinScreenResources,
}

pub(crate) struct ResourceReloadAck {
    pub(crate) generation: u64,
    pub(crate) diagnostics: ClientResourceDiagnostics,
    pub(crate) result: std::result::Result<ResourceReloadSimulationCommit, String>,
}

struct NativeResourceReloadFailure {
    stage: &'static str,
    error: anyhow::Error,
}

pub(crate) struct PreparedServerResourcePackReload {
    generation: u64,
    sound_events: bbb_client_assets::SoundEventRegistry,
    jukebox_songs: bbb_client_assets::JukeboxSongRegistry,
    client_locale: String,
    outcome: std::result::Result<PreparedNativeResourceReload, NativeResourceReloadFailure>,
}

impl std::fmt::Debug for ResourceReloadAck {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResourceReloadAck")
            .field("generation", &self.generation)
            .field("failure_stage", &self.diagnostics.last_reload_failure_stage)
            .field("success", &self.result.is_ok())
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_server_resource_pack_reload(
    effect: ResourceReloadEffect,
    base_roots: Option<PackRoots>,
    active_roots: Option<PackRoots>,
    has_audio_runtime: bool,
    client_locale: String,
    font_options: FontOptions,
    max_texture_dimension_2d: u32,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    username: String,
) -> PreparedServerResourcePackReload {
    let generation = effect.request.generation;
    let outcome = prepare_native_resource_reload(
        &base_roots,
        &active_roots,
        generation,
        effect.request.server_packs,
        has_audio_runtime,
        &client_locale,
        font_options,
        max_texture_dimension_2d,
        particle_status,
        &username,
    )
    .and_then(|prepared| {
        prepared
            .roots
            .ensure_resource_data_admitted()
            .context("admit prepared resource data generation")?;
        Ok(prepared)
    })
    .map_err(|error| NativeResourceReloadFailure {
        stage: "prepare",
        error,
    });
    PreparedServerResourcePackReload {
        generation,
        sound_events: effect.sound_events,
        jukebox_songs: effect.jukebox_songs,
        client_locale,
        outcome,
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) fn failed_server_resource_pack_reload_ack(
    generation: u64,
    active_roots: Option<&PackRoots>,
    item_runtime: Option<&NativeItemRuntime>,
    client_locale: &str,
    stage: &'static str,
    error: String,
) -> ResourceReloadAck {
    let diagnostics = client_resource_reload_diagnostics(
        active_roots,
        item_runtime.is_some_and(NativeItemRuntime::language_bidirectional),
        Some(client_locale),
        item_runtime.map(|runtime| runtime.localized_text(SELECTED_LANGUAGE_PROBE_KEY)),
        generation,
        Some((stage, error.as_str())),
    );
    ResourceReloadAck {
        generation,
        diagnostics,
        result: Err(error),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_prepared_server_resource_pack_reload(
    prepared_reload: PreparedServerResourcePackReload,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    hud_render_runtime: &mut crate::runtime::HudRenderRuntime,
    renderer: &mut bbb_client_render::Renderer,
) -> ResourceReloadAck {
    let generation = prepared_reload.generation;
    let client_locale = prepared_reload.client_locale;
    let result = prepared_reload.outcome.and_then(|prepared| {
        commit_native_resource_reload(
            prepared,
            prepared_reload.sound_events,
            prepared_reload.jukebox_songs,
            active_roots,
            audio_runtime,
            audio_status,
            particle_runtime,
            particle_render_runtime,
            item_runtime,
            terrain_textures,
            entity_variant_textures,
            terrain_upload,
            hud_render_runtime,
            renderer,
        )
    });
    let (result, failure) = match result {
        Ok(commit) => (Ok(commit), None),
        Err(failure) => {
            let error = format!("{:#}", failure.error);
            (Err(error.clone()), Some((failure.stage, error)))
        }
    };
    let diagnostics = client_resource_reload_diagnostics(
        active_roots.as_ref(),
        item_runtime
            .as_ref()
            .is_some_and(NativeItemRuntime::language_bidirectional),
        Some(&client_locale),
        item_runtime
            .as_ref()
            .map(|runtime| runtime.localized_text(SELECTED_LANGUAGE_PROBE_KEY)),
        generation,
        failure
            .as_ref()
            .map(|(stage, error)| (*stage, error.as_str())),
    );
    ResourceReloadAck {
        generation,
        diagnostics,
        result,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_server_resource_pack_reload_effect(
    effect: ResourceReloadEffect,
    base_roots: &Option<PackRoots>,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    hud_render_runtime: &mut crate::runtime::HudRenderRuntime,
    renderer: &mut bbb_client_render::Renderer,
    client_locale: &str,
    font_options: FontOptions,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    username: &str,
) -> ResourceReloadAck {
    let prepared = prepare_server_resource_pack_reload(
        effect,
        base_roots.clone(),
        active_roots.clone(),
        audio_runtime.is_some(),
        client_locale.to_owned(),
        font_options,
        renderer.max_texture_dimension_2d(),
        particle_status,
        username.to_owned(),
    );
    commit_prepared_server_resource_pack_reload(
        prepared,
        active_roots,
        audio_runtime,
        audio_status,
        particle_runtime,
        particle_render_runtime,
        item_runtime,
        terrain_textures,
        entity_variant_textures,
        terrain_upload,
        hud_render_runtime,
        renderer,
    )
}

#[allow(clippy::too_many_arguments)]
fn commit_native_resource_reload(
    mut prepared: PreparedNativeResourceReload,
    sound_events: bbb_client_assets::SoundEventRegistry,
    jukebox_songs: bbb_client_assets::JukeboxSongRegistry,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    hud_render_runtime: &mut crate::runtime::HudRenderRuntime,
    renderer: &mut bbb_client_render::Renderer,
) -> std::result::Result<ResourceReloadSimulationCommit, NativeResourceReloadFailure> {
    let periodic_notification_generation = prepared.periodic_notifications.generation();
    let periodic_notification_schedule =
        crate::runtime::HudRenderRuntime::prepare_periodic_notifications(
            &prepared.periodic_notifications,
        )
        .map_err(|error| NativeResourceReloadFailure {
            stage: "prepare",
            error,
        })?;

    if matches!(prepared.audio, PreparedAudioReload::Existing(_)) && audio_runtime.is_none() {
        return Err(NativeResourceReloadFailure {
            stage: "commit",
            error: anyhow::anyhow!("existing native audio runtime disappeared during reload"),
        });
    }

    let cutout_leaves = terrain_textures.cutout_leaves();
    let smooth_lighting = terrain_textures.smooth_lighting();
    let biome_blend_radius = terrain_textures.biome_blend_radius();
    let max_mipmap_levels = terrain_textures.max_mipmap_levels();
    let atlas_base_padding = terrain_textures.atlas_base_padding();
    let shared_quad_indices = renderer.uses_embedded_terrain_shader();
    let shader_resources = prepared.post_effects.renderer_projection();
    let win_screen_resources = prepared.win_screen_resources;
    let entity_textures = prepared.entity_textures;
    let (mut next_terrain_textures, prepared_entity_textures) = renderer
        .transactional_shader_resource_reload(&shader_resources, false, |renderer| {
            upload_reload_atlases(renderer, &prepared.particles.render, &mut prepared.items)?;
            let textures = try_load_terrain_textures(
                renderer,
                &prepared.roots,
                cutout_leaves,
                smooth_lighting,
                biome_blend_radius,
                max_mipmap_levels,
                atlas_base_padding,
            )
            .context("upload terrain resources during resource reload")?;
            // Both item and terrain atlases have finished stitching from the
            // shared `"blocks"` PNG cache for this reload generation.
            prepared.roots.release_cached_block_texture_images();
            try_load_hud_textures(renderer, &prepared.roots, prepared.font_atlas)
                .context("upload HUD resources during resource reload")?;
            upload_panorama(renderer, &prepared.panorama)
                .context("upload panorama resources during resource reload")?;
            renderer.set_default_text_right_to_left(prepared.items.language_bidirectional());
            let prepared_entity_textures = upload_entity_model_textures(renderer, entity_textures)
                .context("upload entity resources during resource reload")?;
            try_load_sky_textures(renderer, &prepared.roots)
                .context("upload sky resources during resource reload")?;
            // Map pixels remain canonical world data, while every map
            // atlas/bind group is resource-generation-derived. Do this
            // only after every fallible upload has succeeded, inside the
            // same render-thread transaction.
            renderer.invalidate_map_texture_residency();
            renderer.invalidate_item_model_resources();
            Ok((textures, prepared_entity_textures))
        })
        .map_err(|error| NativeResourceReloadFailure {
            stage: "commit",
            error,
        })?;
    // Commit irreversible audio generation reload only after the renderer
    // shader/resource transaction has committed.
    match prepared.audio {
        PreparedAudioReload::Existing(reload) => {
            if let Some(audio) = audio_runtime.as_mut() {
                audio.commit_reload(reload);
            }
        }
        PreparedAudioReload::Disabled => {}
    }
    if let Some(audio) = audio_runtime.as_mut() {
        audio.set_sound_event_registry(sound_events);
        audio.set_jukebox_song_registry(jukebox_songs);
        *audio_status = audio.counters();
    }
    crate::narrator_runtime::update_localized_status_labels(|key| {
        prepared.items.localized_text(key)
    });
    let reuse_terrain_meshes = shared_quad_indices == renderer.uses_embedded_terrain_shader()
        && next_terrain_textures.preserve_mesh_inputs_from(terrain_textures);
    hud_render_runtime.install_periodic_notifications(
        periodic_notification_generation,
        periodic_notification_schedule,
        bbb_client_clock::now(),
    );
    prepared
        .particles
        .simulation
        .set_terrain_textures(&next_terrain_textures);
    prepared
        .particles
        .simulation
        .set_item_runtime(&prepared.items);

    // Every fallible reload step has committed. Move the process-owned skin
    // caches only now so any earlier failure leaves the previous generation
    // complete and untouched; retained decoded images will be re-published
    // after the renderer observes the new resource generation.
    if let Some(previous) = item_runtime.as_mut() {
        prepared.items.adopt_profile_texture_runtime(previous);
    }
    prepared.items.enable_profile_network_queues();

    let NativeParticleResources { simulation, render } = prepared.particles;
    let item_defaults = ItemRuntimeWorldDefaults::from_runtime(&prepared.items);
    let available_command_sounds = audio_runtime
        .as_ref()
        .map(NativeAudioRuntime::available_sound_ids)
        .unwrap_or_default();
    let rig_catalog = std::sync::Arc::new(
        bbb_client_assets::EntityRigCatalog::load(Some(&prepared.roots))
            .unwrap_or_else(|_| bbb_client_assets::EntityRigCatalog::builtin_default()),
    );
    renderer.set_data_driven_entity_rigs(rig_catalog.rigs());

    *terrain_textures = next_terrain_textures;
    *particle_runtime = Some(simulation);
    *entity_variant_textures = NativeEntityVariantTextureRuntime::new(
        Some(prepared.roots.clone()),
        prepared_entity_textures,
    )
    .with_rig_catalog(rig_catalog);
    *particle_render_runtime = Some(render);
    *item_runtime = Some(prepared.items);
    *active_roots = Some(prepared.roots);
    terrain_upload.replace_mesh_resources(!reuse_terrain_meshes);
    Ok(ResourceReloadSimulationCommit {
        item_defaults,
        available_command_sounds,
        win_screen_resources,
    })
}

pub(crate) fn client_resource_pack_diagnostics(
    roots: Option<&PackRoots>,
    language_bidirectional: bool,
    selected_language: Option<&str>,
    selected_language_probe_value: Option<&str>,
) -> ClientResourceDiagnostics {
    let Some(roots) = roots else {
        return ClientResourceDiagnostics::default();
    };
    let Some(generation) = roots.client_resource_generation() else {
        return ClientResourceDiagnostics::default();
    };
    let image_memory = roots.resource_image_memory_observation();
    let data_memory = roots.resource_data_memory_observation();
    let indexed_assets = roots.vanilla_asset_index_observation();
    ClientResourceDiagnostics {
        generation: generation.generation(),
        selected_packs: generation
            .selected_packs()
            .packs()
            .iter()
            .enumerate()
            .map(|(priority, pack)| ClientResourcePackDiagnostic {
                priority,
                id: pack.id.clone(),
                source: match &pack.source {
                    bbb_client_assets::ClientPackSource::Vanilla => "vanilla".to_string(),
                    bbb_client_assets::ClientPackSource::BuiltIn { id } => {
                        format!("builtin/{id}")
                    }
                    bbb_client_assets::ClientPackSource::Startup { argument_index } => {
                        format!("startup:{argument_index}")
                    }
                    bbb_client_assets::ClientPackSource::OptionsRepository { file_name } => {
                        format!("options:file/{file_name}")
                    }
                    bbb_client_assets::ClientPackSource::Server {
                        pack_id,
                        push_sequence,
                    } => format!("server:{pack_id}@{push_sequence}"),
                    bbb_client_assets::ClientPackSource::Unmanaged => "unmanaged".to_string(),
                },
                compatibility: match pack.compatibility {
                    bbb_client_assets::ClientPackCompatibility::Compatible => "compatible",
                    bbb_client_assets::ClientPackCompatibility::TooOld => "too_old",
                    bbb_client_assets::ClientPackCompatibility::TooNew => "too_new",
                }
                .to_string(),
                required: pack.selection.required,
                fixed_position: pack.selection.fixed_position,
            })
            .collect(),
        indexed_asset_index_id: indexed_assets
            .as_ref()
            .map(|observation| observation.index_id.clone()),
        indexed_asset_object_count: indexed_assets
            .as_ref()
            .map_or(0, |observation| observation.object_count),
        indexed_asset_exposed_resource_count: indexed_assets
            .as_ref()
            .map_or(0, |observation| observation.exposed_resource_count),
        indexed_asset_root_resource_count: indexed_assets
            .as_ref()
            .map_or(0, |observation| observation.root_resource_count),
        indexed_asset_retained_logical_bytes: indexed_assets
            .as_ref()
            .map_or(0, |observation| observation.retained_logical_bytes),
        selected_language: selected_language.map(str::to_owned),
        selected_language_probe_key: selected_language_probe_value
            .map(|_| SELECTED_LANGUAGE_PROBE_KEY.to_string()),
        selected_language_probe_value: selected_language_probe_value.map(str::to_owned),
        language_bidirectional,
        image_rgba_limit_bytes: image_memory.max_rgba_bytes,
        image_rgba_charged_bytes: image_memory.charged_rgba_bytes,
        image_rgba_peak_charged_bytes: image_memory.peak_charged_rgba_bytes,
        data_logical_limit_bytes: data_memory.max_logical_bytes,
        data_logical_charged_bytes: data_memory.charged_logical_bytes,
        data_logical_peak_charged_bytes: data_memory.peak_charged_logical_bytes,
        data_json_source_bytes: data_memory.json_source_bytes,
        data_json_documents: data_memory.json_documents,
        data_json_nodes: data_memory.json_nodes,
        data_json_string_bytes: data_memory.json_string_bytes,
        data_listing_entries: data_memory.listing_entries,
        data_derived_logical_bytes: data_memory.derived_logical_bytes,
        data_rejections: data_memory.rejections,
        last_reload_generation: None,
        last_reload_failure_stage: None,
        last_reload_error: None,
    }
}

fn client_resource_reload_diagnostics(
    roots: Option<&PackRoots>,
    language_bidirectional: bool,
    selected_language: Option<&str>,
    selected_language_probe_value: Option<&str>,
    reload_generation: u64,
    failure: Option<(&str, &str)>,
) -> ClientResourceDiagnostics {
    let mut diagnostics = client_resource_pack_diagnostics(
        roots,
        language_bidirectional,
        selected_language,
        selected_language_probe_value,
    );
    diagnostics.last_reload_generation = Some(reload_generation);
    if let Some((stage, error)) = failure {
        diagnostics.last_reload_failure_stage = Some(stage.to_string());
        diagnostics.last_reload_error = Some(error.to_string());
    }
    diagnostics
}

pub(crate) const SELECTED_LANGUAGE_PROBE_KEY: &str = "menu.multiplayer";

#[allow(clippy::too_many_arguments)]
fn prepare_native_resource_reload(
    base_roots: &Option<PackRoots>,
    active_roots: &Option<PackRoots>,
    generation: u64,
    server_packs: Vec<SelectedClientPack>,
    has_audio_runtime: bool,
    client_locale: &str,
    font_options: FontOptions,
    max_texture_dimension_2d: u32,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    username: &str,
) -> Result<PreparedNativeResourceReload> {
    let roots =
        compose_server_resource_pack_roots(base_roots, active_roots, generation, server_packs)?;
    let audio = if has_audio_runtime {
        PreparedAudioReload::Existing(NativeAudioRuntime::prepare_reload(&roots)?)
    } else {
        PreparedAudioReload::Disabled
    };
    let particles = NativeParticleResources::load(&roots, particle_status)
        .context("prepare particle resources for reload")?;
    let font_atlas = load_hud_font_atlas_with_options_and_max_dimension(
        &roots,
        font_options,
        max_texture_dimension_2d,
    )
    .context("prepare shared font resources for reload")?;
    let items = NativeItemRuntime::load_with_locale_and_fonts(
        &roots,
        client_locale,
        Some(font_atlas.fonts.clone()),
    )
    .context("prepare item, model, texture, and language resources for reload")?
    .with_bound_key_lookup(crate::options::key_bindings::bound_key_display_key_units)
    .with_local_time_formatter(bbb_client_world::local_time_select_value);
    let post_effects = prepare_native_shader_resources(&roots)?;
    let panorama = roots
        .load_panorama_catalog()
        .context("prepare panorama resources for reload")?;
    let periodic_notifications = roots
        .load_periodic_notification_catalog()
        .context("prepare periodic notifications for reload")?;
    let win_screen_resources = WinScreenResources::load(&roots, username, &font_atlas.glyphs)
        .context("prepare win-screen resources for reload")?;
    let entity_textures = prepare_entity_model_textures(
        &roots,
        items.equipment_asset_catalog(),
        items.resource_generation(),
    )
    .context("prepare entity resources for reload")?;
    Ok(PreparedNativeResourceReload {
        roots,
        font_atlas,
        audio,
        particles,
        items,
        post_effects,
        panorama,
        periodic_notifications,
        win_screen_resources,
        entity_textures,
    })
}

fn compose_server_resource_pack_roots(
    base_roots: &Option<PackRoots>,
    active_roots: &Option<PackRoots>,
    generation: u64,
    server_packs: Vec<SelectedClientPack>,
) -> Result<PackRoots> {
    let base_roots = base_roots
        .as_ref()
        .context("vanilla pack roots are unavailable")?;
    let selected = base_roots
        .selected_client_packs_or_unmanaged()
        .with_server_packs(server_packs)?;
    let accepted_image_rgba_bytes = active_roots
        .as_ref()
        .map(|roots| roots.resource_image_memory_observation().charged_rgba_bytes)
        .unwrap_or(0);
    let staged_image_limit = resource_image_reload_allowance(
        accepted_image_rgba_bytes,
        MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
        MAX_RESOURCE_IMAGE_PROCESS_RGBA_BYTES,
    )?;
    let accepted_data_logical_bytes = active_roots
        .as_ref()
        .map(|roots| {
            roots
                .resource_data_memory_observation()
                .charged_logical_bytes
        })
        .unwrap_or(0);
    let staged_data_limit = resource_data_reload_allowance(
        accepted_data_logical_bytes,
        MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
        MAX_RESOURCE_DATA_PROCESS_LOGICAL_BYTES,
    )?;
    base_roots.clone().with_client_resource_generation_limits(
        generation,
        selected,
        staged_image_limit,
        staged_data_limit,
    )
}

fn resource_image_reload_allowance(
    accepted_rgba_bytes: u64,
    generation_limit_bytes: u64,
    process_limit_bytes: u64,
) -> Result<u64> {
    let process_allowance = process_limit_bytes
        .checked_sub(accepted_rgba_bytes)
        .with_context(|| {
            format!(
                "accepted resource image generation already charges {accepted_rgba_bytes} RGBA bytes, exceeding the {process_limit_bytes} byte process limit"
            )
        })?;
    Ok(generation_limit_bytes.min(process_allowance))
}

fn resource_data_reload_allowance(
    accepted_logical_bytes: u64,
    generation_limit_bytes: u64,
    process_limit_bytes: u64,
) -> Result<u64> {
    let process_allowance = process_limit_bytes
        .checked_sub(accepted_logical_bytes)
        .with_context(|| {
            format!(
                "accepted resource data generation already charges {accepted_logical_bytes} logical bytes, exceeding the {process_limit_bytes} byte process limit"
            )
        })?;
    Ok(generation_limit_bytes.min(process_allowance))
}

fn upload_reload_atlases(
    renderer: &mut bbb_client_render::Renderer,
    particles: &NativeParticleRenderRuntime,
    items: &mut NativeItemRuntime,
) -> Result<()> {
    particles
        .upload_particle_atlas(renderer)
        .context("upload particle atlas during resource reload")?;
    upload_item_atlas(renderer, items).context("upload item atlas during resource reload")
}

pub(crate) fn upload_item_atlas(
    renderer: &mut bbb_client_render::Renderer,
    items: &mut NativeItemRuntime,
) -> Result<()> {
    let (atlas_width, atlas_height) = items.atlas_size();
    renderer.upload_item_atlas(atlas_width, atlas_height, items.atlas_rgba())?;
    items.release_atlas_rgba();
    renderer.set_item_particle_sprite_uvs(item_particle_sprite_uvs(items));
    Ok(())
}

pub(crate) struct ItemRuntimeWorldDefaults {
    default_component_codec_values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    crafting_remainders: BTreeMap<i32, i32>,
    equipment_asset_generation: u64,
    equipment_asset_names: Vec<String>,
    furnace_fuel_item_ids: BTreeSet<i32>,
    furnace_fuel_rules: Option<Vec<FurnaceFuelRule>>,
    brewing_potion_item_ids: BTreeSet<i32>,
    brewing_ingredient_item_ids: BTreeSet<i32>,
    enchantment_lapis_lazuli_item_ids: BTreeSet<i32>,
    cartography_additional_items: BTreeMap<i32, CartographyAdditionalItemKind>,
    freeze_immune_wearable_item_ids: BTreeSet<i32>,
    powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
}

impl ItemRuntimeWorldDefaults {
    fn from_runtime(items: &NativeItemRuntime) -> Self {
        let furnace_fuel_rules = match FurnaceFuelCatalog::source_rules() {
            Ok(rules) => {
                let rules = rules
                    .into_iter()
                    .map(|rule| match rule {
                        FurnaceFuelSourceRule::AddItem(resource_id) => items
                            .item_protocol_id(&resource_id)
                            .map(FurnaceFuelRule::AddItem),
                        FurnaceFuelSourceRule::AddTag(tag) => Some(FurnaceFuelRule::AddTag(tag)),
                        FurnaceFuelSourceRule::RemoveTag(tag) => {
                            Some(FurnaceFuelRule::RemoveTag(tag))
                        }
                    })
                    .collect::<Option<Vec<_>>>();
                if rules.is_none() {
                    tracing::warn!(
                        "continuing without session furnace-fuel refresh rules because a direct item is unresolved"
                    );
                }
                rules
            }
            Err(error) => {
                tracing::warn!(
                    ?error,
                    "continuing without session furnace-fuel refresh rules"
                );
                None
            }
        };
        Self {
            default_component_codec_values: items.default_component_codec_values_by_protocol_id(),
            crafting_remainders: items.item_crafting_remainders_by_protocol_id(),
            equipment_asset_generation: items.resource_generation(),
            equipment_asset_names: items.equipment_asset_names(),
            furnace_fuel_item_ids: items.furnace_fuel_item_ids_by_protocol_id(),
            furnace_fuel_rules,
            brewing_potion_item_ids: items.brewing_potion_item_ids_by_protocol_id(),
            brewing_ingredient_item_ids: items.brewing_ingredient_item_ids_by_protocol_id(),
            enchantment_lapis_lazuli_item_ids: items
                .enchantment_lapis_lazuli_item_ids_by_protocol_id(),
            cartography_additional_items: cartography_additional_items(items),
            freeze_immune_wearable_item_ids: items.freeze_immune_wearable_item_ids_by_protocol_id(),
            powder_snow_walkable_foot_item_ids: items
                .powder_snow_walkable_foot_item_ids_by_protocol_id(),
        }
    }

    pub(crate) fn apply(self, world: &mut WorldStore) -> Result<()> {
        world.replace_item_component_defaults(
            self.default_component_codec_values,
            self.equipment_asset_generation,
            self.equipment_asset_names,
        )?;
        world.set_default_item_crafting_remainders(self.crafting_remainders);
        if let Some(rules) = self.furnace_fuel_rules {
            world.set_furnace_fuel_profile(self.furnace_fuel_item_ids, rules);
        } else {
            world.set_furnace_fuel_item_ids(self.furnace_fuel_item_ids);
        }
        world.set_brewing_potion_item_ids(self.brewing_potion_item_ids);
        world.set_brewing_ingredient_item_ids(self.brewing_ingredient_item_ids);
        world.set_enchantment_lapis_lazuli_item_ids(self.enchantment_lapis_lazuli_item_ids);
        world.set_cartography_additional_items(self.cartography_additional_items);
        world.set_freeze_immune_wearable_item_ids(self.freeze_immune_wearable_item_ids);
        world.set_powder_snow_walkable_foot_item_ids(self.powder_snow_walkable_foot_item_ids);
        Ok(())
    }
}

fn cartography_additional_items(
    items: &NativeItemRuntime,
) -> BTreeMap<i32, CartographyAdditionalItemKind> {
    let classified = [
        ("minecraft:paper", CartographyAdditionalItemKind::Paper),
        ("minecraft:map", CartographyAdditionalItemKind::EmptyMap),
        (
            "minecraft:glass_pane",
            CartographyAdditionalItemKind::GlassPane,
        ),
    ]
    .into_iter()
    .filter_map(|(resource_id, kind)| {
        items
            .item_protocol_id(resource_id)
            .map(|item_id| (item_id, kind))
    })
    .collect::<BTreeMap<_, _>>();
    debug_assert_eq!(
        classified.keys().copied().collect::<BTreeSet<_>>(),
        items.cartography_additional_item_ids_by_protocol_id(),
        "typed cartography classifications must cover the complete item-runtime membership catalog"
    );
    classified
}

pub(crate) fn apply_item_runtime_world_defaults(
    items: &NativeItemRuntime,
    world: &mut WorldStore,
) -> Result<()> {
    ItemRuntimeWorldDefaults::from_runtime(items).apply(world)
}

pub(crate) fn item_particle_sprite_uvs(items: &NativeItemRuntime) -> Vec<ParticleSpriteUv> {
    items
        .atlas_sprite_uvs()
        .into_iter()
        .map(|sprite| ParticleSpriteUv {
            uv: ParticleUvRect {
                min: sprite.uv.min,
                max: sprite.uv.max,
            },
            has_translucent: sprite.has_translucent,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID,
        data_component_types::{
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        },
        packets::{
            DataComponentPatchSummary, ItemStackSummary, SwingAnimationSummary,
            SwingAnimationTypeSummary,
        },
    };

    #[test]
    fn resource_reload_ack_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ResourceReloadAck>();
        assert_send::<PreparedServerResourcePackReload>();
    }

    #[test]
    fn upload_item_atlas_releases_cpu_rgba_after_gpu_upload() {
        let Some(mut renderer) = bbb_client_render::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut items = NativeItemRuntime::empty_for_test();
        assert!(!items.atlas_rgba().is_empty());
        let expected_size = items.atlas_size();
        let expected_uvs = items.atlas_sprite_uvs().len();

        upload_item_atlas(&mut renderer, &mut items).unwrap();

        assert!(items.atlas_rgba().is_empty());
        assert_eq!(items.atlas_size(), expected_size);
        assert_eq!(items.atlas_sprite_uvs().len(), expected_uvs);
    }

    #[test]
    fn item_runtime_projects_complete_component_scalar_and_trim_defaults_into_world() {
        let items = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
        let red_dye_id = items.item_protocol_id("minecraft:red_dye").unwrap();
        let diamond_id = items.item_protocol_id("minecraft:diamond").unwrap();
        let apple_id = items.item_protocol_id("minecraft:apple").unwrap();
        let chorus_fruit_id = items.item_protocol_id("minecraft:chorus_fruit").unwrap();
        let tipped_arrow_id = items.item_protocol_id("minecraft:tipped_arrow").unwrap();
        let ender_pearl_id = items.item_protocol_id("minecraft:ender_pearl").unwrap();
        let wind_charge_id = items.item_protocol_id("minecraft:wind_charge").unwrap();
        let saddle_id = items.item_protocol_id("minecraft:saddle").unwrap();
        let shield_id = items.item_protocol_id("minecraft:shield").unwrap();
        let elytra_id = items.item_protocol_id("minecraft:elytra").unwrap();
        let defaults = ItemRuntimeWorldDefaults::from_runtime(&items);

        assert_eq!(defaults.default_component_codec_values.len(), 1506);
        for (type_id, count) in [
            (VANILLA_DATA_COMPONENT_DYE_TYPE_ID, 16),
            (VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID, 2),
            (VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID, 3),
        ] {
            assert_eq!(
                defaults
                    .default_component_codec_values
                    .values()
                    .filter(|values| values.contains_key(&type_id))
                    .count(),
                count
            );
        }
        assert_eq!(
            defaults
                .cartography_additional_items
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            items.cartography_additional_item_ids_by_protocol_id()
        );
        assert_eq!(
            defaults.cartography_additional_items
                [&items.item_protocol_id("minecraft:paper").unwrap()],
            CartographyAdditionalItemKind::Paper
        );
        assert_eq!(
            defaults.cartography_additional_items
                [&items.item_protocol_id("minecraft:map").unwrap()],
            CartographyAdditionalItemKind::EmptyMap
        );
        assert_eq!(
            defaults.cartography_additional_items
                [&items.item_protocol_id("minecraft:glass_pane").unwrap()],
            CartographyAdditionalItemKind::GlassPane
        );
        assert_eq!(
            defaults.default_component_codec_values[&ender_pearl_id]
                [&VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID]["seconds"],
            serde_json::json!(1.0)
        );
        assert_eq!(
            defaults.default_component_codec_values[&wind_charge_id]
                [&VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID]["seconds"],
            serde_json::json!(0.5)
        );
        assert_eq!(
            defaults
                .default_component_codec_values
                .values()
                .filter(|values| values.contains_key(&VANILLA_DATA_COMPONENT_FOOD_TYPE_ID))
                .count(),
            44
        );
        let equippable = |item_id| {
            &defaults.default_component_codec_values[&item_id]
                [&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]
        };
        assert_eq!(equippable(saddle_id)["slot"], serde_json::json!("saddle"));
        assert_eq!(
            equippable(saddle_id)["can_be_sheared"],
            serde_json::json!(true)
        );
        assert!(!equippable(saddle_id)["allowed_entities"].is_null());
        assert!(equippable(elytra_id)["allowed_entities"].is_null());
        assert_eq!(equippable(shield_id)["swappable"], serde_json::json!(false));
        assert_eq!(
            defaults.default_component_codec_values[&apple_id]
                [&VANILLA_DATA_COMPONENT_FOOD_TYPE_ID],
            serde_json::json!({"nutrition": 4, "saturation": 2.4})
        );
        assert_eq!(
            defaults.default_component_codec_values[&chorus_fruit_id]
                [&VANILLA_DATA_COMPONENT_FOOD_TYPE_ID]["can_always_eat"],
            serde_json::json!(true)
        );
        assert_eq!(
            defaults.default_component_codec_values[&tipped_arrow_id]
                [&VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID],
            serde_json::json!(0.125)
        );
        assert_eq!(
            defaults.default_component_codec_values[&diamond_id]
                [&VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID],
            serde_json::json!("minecraft:diamond")
        );

        let mut world = WorldStore::new();
        defaults.apply(&mut world).unwrap();
        let red_dye = ItemStackSummary {
            item_id: Some(red_dye_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let effective_red_dye = world.effective_item_stack(&red_dye);
        assert_eq!(
            effective_red_dye.component_patch.dye_color_id,
            Some(VANILLA_DYE_COLOR_RED_ID)
        );
        assert!(
            world.item_stack_has_effective_component(&red_dye, VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
        );
        assert!(world.item_stack_has_effective_component(
            &ItemStackSummary {
                item_id: Some(diamond_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID
        ));
        let tipped_arrow = ItemStackSummary {
            item_id: Some(tipped_arrow_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let effective_tipped_arrow = world.effective_item_stack(&tipped_arrow);
        assert_eq!(
            effective_tipped_arrow
                .component_patch
                .potion_duration_scale_bits,
            Some(0.125_f32.to_bits())
        );
        assert!(world.item_stack_has_effective_component(
            &tipped_arrow,
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID
        ));
        let ender_pearl = ItemStackSummary {
            item_id: Some(ender_pearl_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let effective_ender_pearl = world.effective_item_stack(&ender_pearl);
        assert_eq!(
            effective_ender_pearl
                .component_patch
                .use_cooldown_seconds_bits,
            Some(1.0_f32.to_bits())
        );
        assert_eq!(
            effective_ender_pearl.component_patch.use_cooldown_ticks(),
            Some(20)
        );
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..ender_pearl
            },
        });
        assert!(matches!(
            world.predict_local_item_use(bbb_protocol::packets::InteractionHand::MainHand),
            bbb_client_world::LocalItemUsePrediction::Result(
                bbb_client_world::LocalInteractionResult::Success(_)
            )
        ));
        assert_eq!(
            world
                .cooldown("minecraft:ender_pearl")
                .map(|cooldown| cooldown.duration),
            Some(20)
        );
        assert_eq!(
            world
                .local_item_in_hand(bbb_protocol::packets::InteractionHand::MainHand)
                .map(|stack| stack.count),
            Some(1)
        );
        let apple = ItemStackSummary {
            item_id: Some(apple_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let effective_apple = world.effective_item_stack(&apple);
        assert_eq!(
            effective_apple
                .component_patch
                .consumable
                .unwrap()
                .consume_seconds
                .to_bits(),
            1.6_f32.to_bits()
        );
        assert_eq!(effective_apple.component_patch.food_nutrition, Some(4));
        assert_eq!(
            effective_apple.component_patch.food_saturation_bits,
            Some(2.4_f32.to_bits())
        );
        assert!(!effective_apple.component_patch.food_can_always_eat);
        assert!(
            world.item_stack_has_effective_component(&apple, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID)
        );
        world.apply_player_health(bbb_protocol::packets::PlayerHealth {
            health: 20.0,
            food: 10,
            saturation: 5.0,
        });
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: apple,
        });
        assert!(matches!(
            world.predict_local_item_use(bbb_protocol::packets::InteractionHand::MainHand),
            bbb_client_world::LocalItemUsePrediction::Result(
                bbb_client_world::LocalInteractionResult::Success(
                    bbb_client_world::LocalInteractionSuccess {
                        starts_using_item: true,
                        ..
                    }
                )
            )
        ));
    }

    #[test]
    fn minimum_attack_charge_defaults_reload_into_world() {
        let items = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
        let wooden_spear_id = items.item_protocol_id("minecraft:wooden_spear").unwrap();
        let defaults = ItemRuntimeWorldDefaults::from_runtime(&items);
        assert_eq!(
            defaults.default_component_codec_values[&wooden_spear_id]
                [&VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
            serde_json::json!(1.0)
        );

        let mut world = WorldStore::new();
        defaults.apply(&mut world).unwrap();
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(wooden_spear_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        });
        assert_eq!(
            world
                .local_selected_main_hand_minimum_attack_charge()
                .to_bits(),
            1.0_f32.to_bits()
        );
    }

    #[test]
    fn swing_animation_defaults_reload_and_replace_the_world_generation() {
        let items = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
        let wooden_spear_id = items.item_protocol_id("minecraft:wooden_spear").unwrap();
        let defaults = ItemRuntimeWorldDefaults::from_runtime(&items);
        assert_eq!(
            defaults.default_component_codec_values[&wooden_spear_id]
                [&VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID],
            serde_json::json!({"type": "stab", "duration": 13})
        );

        let stack = ItemStackSummary {
            item_id: Some(wooden_spear_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let mut world = WorldStore::new();
        defaults.apply(&mut world).unwrap();
        assert_eq!(
            world.item_stack_swing_animation(&stack),
            SwingAnimationSummary {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 13,
            }
        );

        ItemRuntimeWorldDefaults::from_runtime(&NativeItemRuntime::empty_for_test())
            .apply(&mut world)
            .unwrap();
        assert_eq!(
            world.item_stack_swing_animation(&stack),
            SwingAnimationSummary {
                animation_type: SwingAnimationTypeSummary::Whack,
                duration: 6,
            },
            "a later runtime generation atomically replaces the old prototype table"
        );
    }
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn server_resource_pack_roots_follow_vanilla_repository_order() {
        let temp = unique_temp_dir("pack-priority");
        let sources = temp.join("sources");
        let startup_base = temp.join("startup/base");
        let startup_top = temp.join("startup/top");
        let server_first = temp.join("server/first");
        let server_second = temp.join("server/second");
        for root in [
            &sources,
            &startup_base,
            &startup_top,
            &server_first,
            &server_second,
        ] {
            std::fs::create_dir_all(root).unwrap();
        }
        write_resource(&server_first, b"first");
        write_resource(&server_second, b"second");
        let base = PackRoots::from_paths(&temp, &sources, sources.join("assets/minecraft"))
            .with_resource_pack_dirs([startup_base.clone(), startup_top.clone()]);
        let first_metadata =
            bbb_client_assets::read_client_resource_pack_metadata(&server_first).unwrap();
        let second_metadata =
            bbb_client_assets::read_client_resource_pack_metadata(&server_second).unwrap();

        let composed = compose_server_resource_pack_roots(
            &Some(base),
            &None,
            7,
            vec![
                SelectedClientPack::server("second", 2, 0, &server_second, second_metadata)
                    .unwrap(),
                SelectedClientPack::server("first", 1, 1, &server_first, first_metadata).unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(
            composed.resource_pack_dirs(),
            vec![
                startup_base,
                startup_top,
                server_second,
                server_first.clone(),
            ]
        );
        let location =
            bbb_client_assets::ResourceLocation::parse("minecraft:test/value.txt").unwrap();
        let resolved = composed.resource_stack().get_resource(&location).unwrap();
        assert_eq!(std::fs::read(&resolved.path).unwrap(), b"first");
        assert_eq!(resolved.pack_id, "server/00000001/first");
        assert_eq!(
            composed.client_resource_generation().unwrap().generation(),
            7
        );
        assert_eq!(
            client_resource_pack_diagnostics(
                Some(&composed),
                true,
                Some("zh_cn"),
                Some("多人游戏"),
            ),
            ClientResourceDiagnostics {
                generation: 7,
                selected_packs: vec![
                    ClientResourcePackDiagnostic {
                        priority: 0,
                        id: "vanilla".to_string(),
                        source: "vanilla".to_string(),
                        compatibility: "compatible".to_string(),
                        required: true,
                        fixed_position: false,
                    },
                    ClientResourcePackDiagnostic {
                        priority: 1,
                        id: "unmanaged/0".to_string(),
                        source: "unmanaged".to_string(),
                        compatibility: "compatible".to_string(),
                        required: false,
                        fixed_position: false,
                    },
                    ClientResourcePackDiagnostic {
                        priority: 2,
                        id: "unmanaged/1".to_string(),
                        source: "unmanaged".to_string(),
                        compatibility: "compatible".to_string(),
                        required: false,
                        fixed_position: false,
                    },
                    ClientResourcePackDiagnostic {
                        priority: 3,
                        id: "server/00000000/second".to_string(),
                        source: "server:second@2".to_string(),
                        compatibility: "compatible".to_string(),
                        required: true,
                        fixed_position: true,
                    },
                    ClientResourcePackDiagnostic {
                        priority: 4,
                        id: "server/00000001/first".to_string(),
                        source: "server:first@1".to_string(),
                        compatibility: "compatible".to_string(),
                        required: true,
                        fixed_position: true,
                    },
                ],
                indexed_asset_index_id: None,
                indexed_asset_object_count: 0,
                indexed_asset_exposed_resource_count: 0,
                indexed_asset_root_resource_count: 0,
                indexed_asset_retained_logical_bytes: 0,
                selected_language: Some("zh_cn".to_string()),
                selected_language_probe_key: Some("menu.multiplayer".to_string()),
                selected_language_probe_value: Some("多人游戏".to_string()),
                language_bidirectional: true,
                image_rgba_limit_bytes: MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
                image_rgba_charged_bytes: 0,
                image_rgba_peak_charged_bytes: 0,
                data_logical_limit_bytes: composed
                    .resource_data_memory_observation()
                    .max_logical_bytes,
                data_logical_charged_bytes: composed
                    .resource_data_memory_observation()
                    .charged_logical_bytes,
                data_logical_peak_charged_bytes: composed
                    .resource_data_memory_observation()
                    .peak_charged_logical_bytes,
                data_json_source_bytes: composed
                    .resource_data_memory_observation()
                    .json_source_bytes,
                data_json_documents: composed.resource_data_memory_observation().json_documents,
                data_json_nodes: composed.resource_data_memory_observation().json_nodes,
                data_json_string_bytes: composed
                    .resource_data_memory_observation()
                    .json_string_bytes,
                data_listing_entries: composed.resource_data_memory_observation().listing_entries,
                data_derived_logical_bytes: composed
                    .resource_data_memory_observation()
                    .derived_logical_bytes,
                data_rejections: 0,
                last_reload_generation: None,
                last_reload_failure_stage: None,
                last_reload_error: None,
            }
        );
        let failed = client_resource_reload_diagnostics(
            Some(&composed),
            true,
            None,
            None,
            8,
            Some(("prepare", "invalid custom model")),
        );
        assert_eq!(failed.generation, 7);
        assert_eq!(failed.last_reload_generation, Some(8));
        assert_eq!(failed.last_reload_failure_stage.as_deref(), Some("prepare"));
        assert_eq!(
            failed.last_reload_error.as_deref(),
            Some("invalid custom model")
        );
        assert_eq!(failed.selected_packs.len(), 5);
        assert!(failed.language_bidirectional);

        let succeeded =
            client_resource_reload_diagnostics(Some(&composed), false, None, None, 9, None);
        assert_eq!(succeeded.generation, 7);
        assert!(!succeeded.language_bidirectional);
        assert_eq!(succeeded.last_reload_generation, Some(9));
        assert_eq!(succeeded.last_reload_failure_stage, None);
        assert_eq!(succeeded.last_reload_error, None);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn server_resource_pack_reload_requires_vanilla_base_roots() {
        let error = compose_server_resource_pack_roots(&None, &None, 1, Vec::new()).unwrap_err();

        assert!(error
            .to_string()
            .contains("vanilla pack roots are unavailable"));
    }

    #[test]
    fn staged_resource_image_allowance_respects_generation_and_process_limits() {
        assert_eq!(resource_image_reload_allowance(0, 512, 1_024).unwrap(), 512);
        assert_eq!(
            resource_image_reload_allowance(700, 512, 1_024).unwrap(),
            324
        );
        let error = resource_image_reload_allowance(1_025, 512, 1_024)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("accepted resource image generation"),
            "{error}"
        );
    }

    #[test]
    fn staged_resource_data_allowance_respects_generation_and_process_limits() {
        assert_eq!(resource_data_reload_allowance(0, 512, 1_024).unwrap(), 512);
        assert_eq!(
            resource_data_reload_allowance(700, 512, 1_024).unwrap(),
            324
        );
        let error = resource_data_reload_allowance(1_025, 512, 1_024)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("accepted resource data generation"),
            "{error}"
        );
    }

    #[test]
    fn staged_resource_generation_does_not_mutate_accepted_image_charge() {
        let temp = unique_temp_dir("staged-image-budget");
        let sources = temp.join("sources");
        std::fs::create_dir_all(sources.join("assets/minecraft")).unwrap();
        write_png(&sources.join("assets/minecraft/textures/item/widget.png"));
        let base = PackRoots::from_paths(&temp, &sources, sources.join("assets/minecraft"));
        let selected = base.selected_client_packs_or_unmanaged();
        let accepted = base
            .clone()
            .with_client_resource_generation_image_limit(4, selected, 64)
            .unwrap();

        let staged = compose_server_resource_pack_roots(
            &Some(base.clone()),
            &Some(accepted.clone()),
            5,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            staged.resource_image_memory_observation().max_rgba_bytes,
            MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES
        );
        assert!(staged
            .load_resource_texture_image("minecraft:item/widget")
            .is_ok());
        assert_eq!(
            staged
                .resource_image_memory_observation()
                .charged_rgba_bytes,
            4
        );
        let staged_diagnostics = client_resource_pack_diagnostics(Some(&staged), false, None, None);
        assert_eq!(
            staged_diagnostics.image_rgba_limit_bytes,
            MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES
        );
        assert_eq!(staged_diagnostics.image_rgba_charged_bytes, 4);
        assert_eq!(staged_diagnostics.image_rgba_peak_charged_bytes, 4);
        assert_eq!(
            accepted
                .resource_image_memory_observation()
                .charged_rgba_bytes,
            0
        );
        drop(staged);

        let retry = compose_server_resource_pack_roots(&Some(base), &Some(accepted), 6, Vec::new())
            .unwrap();
        assert_eq!(
            retry.resource_image_memory_observation().charged_rgba_bytes,
            0
        );
        assert_eq!(
            retry.resource_image_memory_observation().max_rgba_bytes,
            MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES
        );

        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn staged_resource_generation_has_independent_data_charge_and_diagnostics() {
        let temp = unique_temp_dir("staged-data-budget");
        let sources = temp.join("sources");
        let json_path = sources.join("assets/minecraft/test/value.json");
        std::fs::create_dir_all(json_path.parent().unwrap()).unwrap();
        std::fs::write(&json_path, br#"{"value":"accepted"}"#).unwrap();
        let base = PackRoots::from_paths(&temp, &sources, sources.join("assets/minecraft"));
        let selected = base.selected_client_packs_or_unmanaged();
        let accepted = base
            .clone()
            .with_client_resource_generation_limits(4, selected, 64, 4_096)
            .unwrap();
        let _: serde_json::Value = accepted
            .resource_stack()
            .read_json_file(&json_path, "accepted fixture")
            .unwrap();
        let accepted_charge = accepted
            .resource_data_memory_observation()
            .charged_logical_bytes;
        assert!(accepted_charge > 0);

        let staged =
            compose_server_resource_pack_roots(&Some(base), &Some(accepted.clone()), 5, Vec::new())
                .unwrap();
        let _: serde_json::Value = staged
            .resource_stack()
            .read_json_file(&json_path, "staged fixture")
            .unwrap();
        let staged_memory = staged.resource_data_memory_observation();
        assert_eq!(
            staged_memory.max_logical_bytes,
            MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES
        );
        assert_eq!(
            accepted
                .resource_data_memory_observation()
                .charged_logical_bytes,
            accepted_charge
        );
        let diagnostics = client_resource_pack_diagnostics(Some(&staged), false, None, None);
        assert_eq!(
            diagnostics.data_logical_charged_bytes,
            staged_memory.charged_logical_bytes
        );
        assert_eq!(diagnostics.data_json_documents, 1);
        assert_eq!(diagnostics.data_rejections, 0);

        std::fs::remove_dir_all(temp).unwrap();
    }

    fn write_resource(root: &std::path::Path, payload: &[u8]) {
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
        )
        .unwrap();
        let path = root.join("assets/minecraft/test/value.txt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, payload).unwrap();
    }

    fn write_png(path: &std::path::Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(path, &[1, 2, 3, 255], 1, 1, image::ColorType::Rgba8).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-client-resource-reload-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
