use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Context, Result};
use bbb_control::{AudioCounters, ClientResourceDiagnostics, ClientResourcePackDiagnostic};
use bbb_item_model::NativeItemRuntime;
use bbb_pack::{
    BlockDestroyProfile as PackBlockDestroyProfile, BlockDestroyProfileCatalog,
    BlockSoundProfile as PackBlockSoundProfile, BlockSoundProfileCatalog, FurnaceFuelCatalog,
    FurnaceFuelSourceRule, PackRoots, PanoramaCatalog, PeriodicNotificationCatalog,
    PostEffectCatalog, RegistryDependentItemComponentInitializer, SelectedClientPack,
    MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES, MAX_RESOURCE_DATA_PROCESS_LOGICAL_BYTES,
    MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES, MAX_RESOURCE_IMAGE_PROCESS_RGBA_BYTES,
};
use bbb_renderer::{ParticleSpriteUv, ParticleUvRect};
use bbb_world::{
    CartographyAdditionalItemKind, EquipmentAssetId as WorldEquipmentAssetId, FurnaceFuelRule,
    ItemAttackRange, ItemEquipmentSlot, ItemFoodProperties, ItemUseEffects, MountArmorSlotKind,
    WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldItemMiningProfile, WorldStore,
};

use crate::{
    audio_runtime::{AudioEventSink, NativeAudioReload, NativeAudioRuntime},
    entity_assets::try_load_entity_model_textures,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    hud_assets::{try_load_hud_textures, upload_panorama},
    particle_runtime::{
        NativeParticleRenderRuntime, NativeParticleResources, NativeParticleRuntime,
    },
    resource_pack_runtime::ServerResourcePackReload,
    simulation::effects::ResourceReloadEffect,
    sky_assets::try_load_sky_textures,
    terrain_runtime::{try_load_terrain_textures, TerrainTextureState, TerrainUploadState},
    win_screen_resources::WinScreenResources,
};

enum PreparedAudioReload {
    Existing(NativeAudioReload),
    Disabled,
}

struct PreparedNativeResourceReload {
    roots: PackRoots,
    audio: PreparedAudioReload,
    particles: NativeParticleResources,
    items: NativeItemRuntime,
    post_effects: PostEffectCatalog,
    panorama: PanoramaCatalog,
    periodic_notifications: PeriodicNotificationCatalog,
    block_destroy_profiles: BTreeMap<String, WorldBlockDestroyProfile>,
    block_sound_profiles: BTreeMap<String, WorldBlockSoundProfile>,
}

pub(crate) fn prepare_native_shader_resources(roots: &PackRoots) -> Result<PostEffectCatalog> {
    PostEffectCatalog::load(roots).context("prepare shader resources for reload")
}

pub(crate) struct ResourceReloadSimulationCommit {
    pub(crate) particle_runtime: NativeParticleRuntime,
    pub(crate) item_runtime: NativeItemRuntime,
    pub(crate) terrain_textures: TerrainTextureState,
    pub(crate) item_defaults: ItemRuntimeWorldDefaults,
    pub(crate) available_command_sounds: Vec<String>,
    pub(crate) block_destroy_profiles: BTreeMap<String, WorldBlockDestroyProfile>,
    pub(crate) block_sound_profiles: BTreeMap<String, WorldBlockSoundProfile>,
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

impl ResourceReloadAck {
    pub(crate) fn mailbox_retained_logical_bytes(&self) -> usize {
        let selected_pack_bytes = self
            .diagnostics
            .selected_packs
            .capacity()
            .saturating_mul(std::mem::size_of::<ClientResourcePackDiagnostic>())
            .saturating_add(
                self.diagnostics
                    .selected_packs
                    .iter()
                    .map(|pack| {
                        pack.id
                            .capacity()
                            .saturating_add(pack.source.capacity())
                            .saturating_add(pack.compatibility.capacity())
                    })
                    .fold(0usize, usize::saturating_add),
            );
        let diagnostic_strings = [
            self.diagnostics.indexed_asset_index_id.as_ref(),
            self.diagnostics.selected_language.as_ref(),
            self.diagnostics.selected_language_probe_key.as_ref(),
            self.diagnostics.selected_language_probe_value.as_ref(),
            self.diagnostics.last_reload_failure_stage.as_ref(),
            self.diagnostics.last_reload_error.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(String::capacity)
        .fold(0usize, usize::saturating_add);
        let generation_bytes = self
            .diagnostics
            .data_logical_charged_bytes
            .saturating_add(self.diagnostics.image_rgba_charged_bytes)
            .try_into()
            .unwrap_or(usize::MAX);
        let result_error_bytes = self.result.as_ref().err().map_or(0, String::capacity);
        selected_pack_bytes
            .saturating_add(diagnostic_strings)
            .saturating_add(generation_bytes)
            .saturating_add(result_error_bytes)
            .saturating_add(std::mem::size_of::<Self>())
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_server_resource_pack_reload_effect(
    effect: ResourceReloadEffect,
    base_roots: &Option<PackRoots>,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    hud_render_runtime: &mut crate::runtime::HudRenderRuntime,
    renderer: &mut bbb_renderer::Renderer,
    client_locale: &str,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    player_skin_cache_dir: &Path,
    username: &str,
) -> ResourceReloadAck {
    let generation = effect.request.generation;
    let result = apply_native_resource_reload(
        effect.request,
        effect.sound_events,
        effect.jukebox_songs,
        base_roots,
        active_roots,
        audio_runtime,
        audio_status,
        particle_render_runtime,
        item_runtime,
        terrain_textures,
        entity_variant_textures,
        terrain_upload,
        hud_render_runtime,
        renderer,
        client_locale,
        particle_status,
        player_skin_cache_dir,
        username,
    );
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
        Some(client_locale),
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
fn apply_native_resource_reload(
    reload: ServerResourcePackReload,
    sound_events: bbb_pack::SoundEventRegistry,
    jukebox_songs: bbb_pack::JukeboxSongRegistry,
    base_roots: &Option<PackRoots>,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_render_runtime: &mut Option<NativeParticleRenderRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    hud_render_runtime: &mut crate::runtime::HudRenderRuntime,
    renderer: &mut bbb_renderer::Renderer,
    client_locale: &str,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    player_skin_cache_dir: &Path,
    username: &str,
) -> std::result::Result<ResourceReloadSimulationCommit, NativeResourceReloadFailure> {
    let mut prepared = prepare_native_resource_reload(
        base_roots,
        active_roots,
        reload.generation,
        reload.server_packs,
        audio_runtime.is_some(),
        client_locale,
        particle_status,
    )
    .map_err(|error| NativeResourceReloadFailure {
        stage: "prepare",
        error,
    })?;
    prepared
        .roots
        .ensure_resource_data_admitted()
        .context("admit prepared resource data generation")
        .map_err(|error| NativeResourceReloadFailure {
            stage: "prepare",
            error,
        })?;
    let periodic_notification_generation = prepared.periodic_notifications.generation();
    let periodic_notification_schedule =
        crate::runtime::HudRenderRuntime::prepare_periodic_notifications(
            &prepared.periodic_notifications,
        )
        .map_err(|error| NativeResourceReloadFailure {
            stage: "prepare",
            error,
        })?;

    let cutout_leaves = terrain_textures.cutout_leaves();
    let smooth_lighting = terrain_textures.smooth_lighting();
    let atlas_base_padding = terrain_textures.atlas_base_padding();
    let shader_resources = prepared.post_effects.renderer_projection();
    let (next_terrain_textures, win_screen_resources) = renderer
        .transactional_shader_resource_reload(&shader_resources, false, |renderer| {
            upload_reload_atlases(renderer, &prepared.particles.render, &prepared.items)?;
            let textures = try_load_terrain_textures(
                renderer,
                &prepared.roots,
                cutout_leaves,
                smooth_lighting,
                atlas_base_padding,
            )
            .context("upload terrain resources during resource reload")?;
            try_load_hud_textures(renderer, &prepared.roots)
                .context("upload HUD resources during resource reload")?;
            upload_panorama(renderer, &prepared.panorama)
                .context("upload panorama resources during resource reload")?;
            renderer.set_default_text_right_to_left(prepared.items.language_bidirectional());
            let win_screen_resources = WinScreenResources::load(
                &prepared.roots,
                username,
                &renderer.hud_text_measurements(),
            )
            .context("prepare win-screen resources during resource reload")?;
            try_load_entity_model_textures(
                renderer,
                &prepared.roots,
                prepared.items.equipment_asset_catalog(),
                prepared.items.resource_generation(),
            )
            .context("upload entity resources during resource reload")?;
            try_load_sky_textures(renderer, &prepared.roots)
                .context("upload sky resources during resource reload")?;
            match prepared.audio {
                PreparedAudioReload::Existing(reload) => {
                    let audio = audio_runtime
                        .as_mut()
                        .context("existing native audio runtime disappeared during reload")?;
                    audio.commit_reload(reload);
                }
                PreparedAudioReload::Disabled => {}
            }
            if let Some(audio) = audio_runtime.as_mut() {
                audio.set_sound_event_registry(sound_events);
                audio.set_jukebox_song_registry(jukebox_songs);
                *audio_status = audio.counters();
            }
            // Map pixels remain canonical world data, while every map
            // atlas/bind group is resource-generation-derived. Do this
            // only after every fallible upload has succeeded, inside the
            // same render-thread transaction.
            renderer.invalidate_map_texture_residency();
            renderer.invalidate_item_model_resources();
            Ok((textures, win_screen_resources))
        })
        .map_err(|error| NativeResourceReloadFailure {
            stage: "commit",
            error,
        })?;
    hud_render_runtime.install_periodic_notifications(
        periodic_notification_generation,
        periodic_notification_schedule,
        std::time::Instant::now(),
    );
    prepared
        .particles
        .simulation
        .set_terrain_particle_sprite_ids(&next_terrain_textures);
    prepared
        .particles
        .simulation
        .set_default_item_particle_sprite_ids(&prepared.items);

    // Every fallible reload step has committed. Move the process-owned skin
    // caches only now so any earlier failure leaves the previous generation
    // complete and untouched; retained decoded images will be re-published
    // after the renderer observes the new resource generation.
    if let Some(previous) = item_runtime.as_mut() {
        prepared.items.adopt_profile_texture_runtime(previous);
    }
    prepared.items.enable_http_profile_resolution();
    prepared
        .items
        .enable_http_player_skin_downloads(player_skin_cache_dir.to_path_buf());

    let NativeParticleResources { simulation, render } = prepared.particles;
    let item_defaults = ItemRuntimeWorldDefaults::from_runtime(&prepared.items);
    let simulation_items = prepared.items.independent_simulation_projection();
    let available_command_sounds = audio_runtime
        .as_ref()
        .map(NativeAudioRuntime::available_sound_ids)
        .unwrap_or_default();
    let simulation_terrain_textures = next_terrain_textures.clone();
    *terrain_textures = next_terrain_textures;
    *entity_variant_textures = NativeEntityVariantTextureRuntime::new(Some(prepared.roots.clone()));
    *particle_render_runtime = Some(render);
    *item_runtime = Some(prepared.items);
    *active_roots = Some(prepared.roots);
    terrain_upload.request_reload_all_chunks();
    Ok(ResourceReloadSimulationCommit {
        particle_runtime: simulation,
        item_runtime: simulation_items,
        terrain_textures: simulation_terrain_textures,
        item_defaults,
        available_command_sounds,
        block_destroy_profiles: prepared.block_destroy_profiles,
        block_sound_profiles: prepared.block_sound_profiles,
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
                    bbb_pack::ClientPackSource::Vanilla => "vanilla".to_string(),
                    bbb_pack::ClientPackSource::Startup { argument_index } => {
                        format!("startup:{argument_index}")
                    }
                    bbb_pack::ClientPackSource::Server {
                        pack_id,
                        push_sequence,
                    } => format!("server:{pack_id}@{push_sequence}"),
                    bbb_pack::ClientPackSource::Unmanaged => "unmanaged".to_string(),
                },
                compatibility: match pack.compatibility {
                    bbb_pack::ClientPackCompatibility::Compatible => "compatible",
                    bbb_pack::ClientPackCompatibility::TooOld => "too_old",
                    bbb_pack::ClientPackCompatibility::TooNew => "too_new",
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

fn prepare_native_resource_reload(
    base_roots: &Option<PackRoots>,
    active_roots: &Option<PackRoots>,
    generation: u64,
    server_packs: Vec<SelectedClientPack>,
    has_audio_runtime: bool,
    client_locale: &str,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
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
    let items = NativeItemRuntime::load_with_locale(&roots, client_locale)
        .context("prepare item, model, texture, and language resources for reload")?;
    let post_effects = prepare_native_shader_resources(&roots)?;
    let panorama = roots
        .load_panorama_catalog()
        .context("prepare panorama resources for reload")?;
    let periodic_notifications = roots
        .load_periodic_notification_catalog()
        .context("prepare periodic notifications for reload")?;
    let block_destroy_profiles = block_destroy_profiles_for_world(
        &roots
            .load_block_destroy_profile_catalog()
            .context("prepare block destroy profiles for reload")?,
    );
    let block_sound_profiles = block_sound_profiles_for_world(
        &roots
            .load_block_sound_profile_catalog()
            .context("prepare block sound profiles for reload")?,
    );
    Ok(PreparedNativeResourceReload {
        roots,
        audio,
        particles,
        items,
        post_effects,
        panorama,
        periodic_notifications,
        block_destroy_profiles,
        block_sound_profiles,
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
    renderer: &mut bbb_renderer::Renderer,
    particles: &NativeParticleRenderRuntime,
    items: &NativeItemRuntime,
) -> Result<()> {
    particles
        .upload_particle_atlas(renderer)
        .context("upload particle atlas during resource reload")?;
    upload_item_atlases(renderer, items).context("upload item atlases during resource reload")
}

fn upload_item_atlases(
    renderer: &mut bbb_renderer::Renderer,
    items: &NativeItemRuntime,
) -> Result<()> {
    let (atlas_width, atlas_height) = items.atlas_size();
    renderer.upload_hud_item_atlas(atlas_width, atlas_height, items.atlas_rgba())?;
    renderer.upload_item_entity_atlas(atlas_width, atlas_height, items.atlas_rgba())?;
    renderer.set_item_particle_sprite_uvs(item_particle_sprite_uvs(items));
    Ok(())
}

pub(crate) struct ItemRuntimeWorldDefaults {
    default_component_type_ids: BTreeMap<i32, BTreeSet<i32>>,
    default_component_codec_values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    default_dye_color_ids: BTreeMap<i32, i32>,
    default_potion_duration_scale_bits: BTreeMap<i32, u32>,
    default_use_cooldown_seconds_bits: BTreeMap<i32, u32>,
    registry_dependent_component_initializers:
        BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>>,
    max_stack_sizes: BTreeMap<i32, i32>,
    max_damage: BTreeMap<i32, i32>,
    crafting_remainders: BTreeMap<i32, i32>,
    equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    swappable_equipment_item_ids: BTreeSet<i32>,
    player_swappable_equipment_item_ids: BTreeSet<i32>,
    shearable_equipment_item_ids: BTreeSet<i32>,
    equipment_asset_generation: u64,
    equipment_asset_names: Vec<String>,
    equipment_assets: BTreeMap<i32, WorldEquipmentAssetId>,
    mount_body_armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    attack_ranges: BTreeMap<i32, ItemAttackRange>,
    minimum_attack_charges: BTreeMap<i32, f32>,
    swing_animations: BTreeMap<i32, bbb_protocol::packets::SwingAnimationSummary>,
    piercing_weapon_item_ids: BTreeSet<i32>,
    damageable_item_ids: BTreeSet<i32>,
    use_effects: BTreeMap<i32, ItemUseEffects>,
    consumables: BTreeMap<i32, bbb_protocol::packets::ConsumableSummary>,
    food_properties: BTreeMap<i32, ItemFoodProperties>,
    furnace_fuel_item_ids: BTreeSet<i32>,
    furnace_fuel_rules: Option<Vec<FurnaceFuelRule>>,
    brewing_potion_item_ids: BTreeSet<i32>,
    brewing_ingredient_item_ids: BTreeSet<i32>,
    enchantment_lapis_lazuli_item_ids: BTreeSet<i32>,
    cartography_additional_items: BTreeMap<i32, CartographyAdditionalItemKind>,
    freeze_immune_wearable_item_ids: BTreeSet<i32>,
    powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
    mining_profiles: BTreeMap<i32, WorldItemMiningProfile>,
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
            default_component_type_ids: items.default_component_type_ids_by_protocol_id(),
            default_component_codec_values: items.default_component_codec_values_by_protocol_id(),
            default_dye_color_ids: items.default_dye_color_ids_by_protocol_id(),
            default_potion_duration_scale_bits: items
                .default_potion_duration_scale_bits_by_protocol_id(),
            default_use_cooldown_seconds_bits: items
                .default_use_cooldown_seconds_bits_by_protocol_id(),
            registry_dependent_component_initializers: items
                .registry_dependent_component_initializers_by_protocol_id(),
            max_stack_sizes: items.item_max_stack_sizes_by_protocol_id(),
            max_damage: items.item_max_damage_by_protocol_id(),
            crafting_remainders: items.item_crafting_remainders_by_protocol_id(),
            equipment_slots: items.item_equipment_slots_by_protocol_id(),
            swappable_equipment_item_ids: items
                .default_swappable_equipment_item_ids_by_protocol_id(),
            player_swappable_equipment_item_ids: items
                .default_player_swappable_equipment_item_ids_by_protocol_id(),
            shearable_equipment_item_ids: items
                .default_shearable_equipment_item_ids_by_protocol_id(),
            equipment_asset_generation: items.resource_generation(),
            equipment_asset_names: items.equipment_asset_names(),
            equipment_assets: items
                .item_equipment_asset_ids_by_protocol_id()
                .into_iter()
                .map(|(item_id, asset_id)| {
                    (
                        item_id,
                        WorldEquipmentAssetId::new(asset_id.generation, asset_id.index),
                    )
                })
                .collect(),
            mount_body_armor_kinds: items.mount_body_armor_kinds_by_protocol_id(),
            attack_ranges: items.item_attack_ranges_by_protocol_id(),
            minimum_attack_charges: items.item_minimum_attack_charges_by_protocol_id(),
            swing_animations: items.item_swing_animations_by_protocol_id(),
            piercing_weapon_item_ids: items.default_piercing_weapon_item_ids_by_protocol_id(),
            damageable_item_ids: items.default_damageable_item_ids_by_protocol_id(),
            use_effects: items.item_use_effects_by_protocol_id(),
            consumables: items.item_default_consumables_by_protocol_id(),
            food_properties: items.item_food_properties_by_protocol_id(),
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
            mining_profiles: items.item_mining_profiles_by_protocol_id(),
        }
    }

    pub(crate) fn apply(self, world: &mut WorldStore) {
        world.set_default_item_component_type_ids(self.default_component_type_ids);
        world.set_default_item_component_codec_values(self.default_component_codec_values);
        world.set_default_item_dye_color_ids(self.default_dye_color_ids);
        world.set_default_item_potion_duration_scale_bits(self.default_potion_duration_scale_bits);
        world.set_default_item_use_cooldown_seconds_bits(self.default_use_cooldown_seconds_bits);
        world.set_registry_dependent_item_component_initializers(
            self.registry_dependent_component_initializers,
        );
        if let Err(error) = world.rebuild_item_component_defaults() {
            tracing::debug!(
                ?error,
                "item prototype defaults await a complete registry/tag revision"
            );
        }
        world.set_default_item_max_stack_sizes(self.max_stack_sizes);
        world.set_default_item_max_damage(self.max_damage);
        world.set_default_item_crafting_remainders(self.crafting_remainders);
        world.set_default_item_equipment_slots(self.equipment_slots);
        world.set_default_swappable_equipment_item_ids(self.swappable_equipment_item_ids);
        world.set_default_player_swappable_equipment_item_ids(
            self.player_swappable_equipment_item_ids,
        );
        world.set_default_shearable_equipment_item_ids(self.shearable_equipment_item_ids);
        world.set_default_item_equipment_assets(
            self.equipment_asset_generation,
            self.equipment_asset_names,
            self.equipment_assets,
        );
        world.set_default_mount_body_armor_kinds(self.mount_body_armor_kinds);
        world.set_default_item_attack_ranges(self.attack_ranges);
        world.set_default_item_minimum_attack_charges(self.minimum_attack_charges);
        world.set_default_item_swing_animations(self.swing_animations);
        world.set_default_piercing_weapon_item_ids(self.piercing_weapon_item_ids);
        world.set_default_damageable_item_ids(self.damageable_item_ids);
        world.set_default_item_use_effects(self.use_effects);
        world.set_default_item_consumables(self.consumables);
        world.set_default_item_food_properties(self.food_properties);
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
        world.set_default_item_mining_profiles(self.mining_profiles);
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

pub(crate) fn apply_item_runtime_world_defaults(items: &NativeItemRuntime, world: &mut WorldStore) {
    ItemRuntimeWorldDefaults::from_runtime(items).apply(world);
}

pub(crate) fn block_destroy_profiles_for_world(
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

pub(crate) fn block_sound_profiles_for_world(
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
        place_sound: profile.place_sound.clone(),
        hit_sound: profile.hit_sound.clone(),
        volume: profile.volume,
        pitch: profile.pitch,
    }
}

pub(crate) fn item_particle_sprite_uvs(items: &NativeItemRuntime) -> Vec<ParticleSpriteUv> {
    items
        .atlas_sprite_uvs()
        .into_iter()
        .map(|sprite| ParticleSpriteUv {
            id: sprite.id,
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
    use bbb_pack::RegistryDependentItemComponentInitializer;
    use bbb_protocol::{
        builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID,
        data_component_types::{
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
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

        assert_eq!(defaults.default_component_type_ids.len(), 1506);
        assert_eq!(defaults.default_dye_color_ids.len(), 16);
        assert_eq!(defaults.default_potion_duration_scale_bits.len(), 2);
        assert_eq!(defaults.default_use_cooldown_seconds_bits.len(), 3);
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
            defaults.default_use_cooldown_seconds_bits[&ender_pearl_id],
            1.0_f32.to_bits()
        );
        assert_eq!(
            defaults.default_use_cooldown_seconds_bits[&wind_charge_id],
            0.5_f32.to_bits()
        );
        assert_eq!(defaults.food_properties.len(), 44);
        assert_eq!(defaults.swappable_equipment_item_ids.len(), 75);
        assert_eq!(defaults.player_swappable_equipment_item_ids.len(), 30);
        assert!(defaults.swappable_equipment_item_ids.contains(&saddle_id));
        assert!(defaults.swappable_equipment_item_ids.contains(&elytra_id));
        assert!(!defaults.swappable_equipment_item_ids.contains(&shield_id));
        assert!(defaults
            .player_swappable_equipment_item_ids
            .contains(&elytra_id));
        assert!(!defaults
            .player_swappable_equipment_item_ids
            .contains(&saddle_id));
        assert_eq!(defaults.shearable_equipment_item_ids.len(), 45);
        assert!(defaults.shearable_equipment_item_ids.contains(&saddle_id));
        assert_eq!(
            defaults.consumables[&apple_id].consume_seconds.to_bits(),
            1.6_f32.to_bits()
        );
        assert_eq!(
            defaults.food_properties[&apple_id],
            ItemFoodProperties {
                nutrition: 4,
                saturation_bits: 2.4_f32.to_bits(),
                can_always_eat: false,
            }
        );
        assert!(defaults.food_properties[&chorus_fruit_id].can_always_eat);
        assert_eq!(
            defaults.default_potion_duration_scale_bits[&tipped_arrow_id],
            0.125_f32.to_bits()
        );
        assert!(
            defaults.registry_dependent_component_initializers[&diamond_id].contains(
                &RegistryDependentItemComponentInitializer::ProvidesTrimMaterial {
                    material: "minecraft:diamond".to_string(),
                }
            )
        );

        let mut world = WorldStore::new();
        defaults.apply(&mut world);
        let red_dye = ItemStackSummary {
            item_id: Some(red_dye_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
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
                component_patch: DataComponentPatchSummary::default(),
            },
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID
        ));
        let tipped_arrow = ItemStackSummary {
            item_id: Some(tipped_arrow_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
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
            component_patch: DataComponentPatchSummary::default(),
        };
        let effective_ender_pearl = world.effective_item_stack(&ender_pearl);
        assert_eq!(
            effective_ender_pearl
                .component_patch
                .use_cooldown_seconds_bits,
            Some(1.0_f32.to_bits())
        );
        assert_eq!(
            effective_ender_pearl.component_patch.use_cooldown_ticks,
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
            bbb_world::LocalItemUsePrediction::Result(bbb_world::LocalInteractionResult::Success(
                _
            ))
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
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(elytra_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        });
        assert!(matches!(
            world.predict_local_item_use(bbb_protocol::packets::InteractionHand::MainHand),
            bbb_world::LocalItemUsePrediction::Result(bbb_world::LocalInteractionResult::Success(
                bbb_world::LocalInteractionSuccess {
                    starts_using_item: false,
                    ..
                }
            ))
        ));
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(saddle_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        });
        assert_eq!(
            world.predict_local_item_use(bbb_protocol::packets::InteractionHand::MainHand),
            bbb_world::LocalItemUsePrediction::Result(bbb_world::LocalInteractionResult::Pass)
        );
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(shield_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        });
        assert!(matches!(
            world.predict_local_item_use(bbb_protocol::packets::InteractionHand::MainHand),
            bbb_world::LocalItemUsePrediction::Result(bbb_world::LocalInteractionResult::Success(
                bbb_world::LocalInteractionSuccess {
                    starts_using_item: true,
                    ..
                }
            ))
        ));
        let apple = ItemStackSummary {
            item_id: Some(apple_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let effective_apple = world.effective_item_stack(&apple);
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
            bbb_world::LocalItemUsePrediction::Result(bbb_world::LocalInteractionResult::Success(
                bbb_world::LocalInteractionSuccess {
                    starts_using_item: true,
                    ..
                }
            ))
        ));
    }

    #[test]
    fn minimum_attack_charge_defaults_reload_into_world() {
        let items = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
        let wooden_spear_id = items.item_protocol_id("minecraft:wooden_spear").unwrap();
        let defaults = ItemRuntimeWorldDefaults::from_runtime(&items);
        assert_eq!(
            defaults
                .minimum_attack_charges
                .get(&wooden_spear_id)
                .map(|charge| charge.to_bits()),
            Some(1.0_f32.to_bits())
        );

        let mut world = WorldStore::new();
        defaults.apply(&mut world);
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(wooden_spear_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
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
            defaults.swing_animations.get(&wooden_spear_id),
            Some(&SwingAnimationSummary {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 13,
            })
        );

        let stack = ItemStackSummary {
            item_id: Some(wooden_spear_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut world = WorldStore::new();
        defaults.apply(&mut world);
        assert_eq!(
            world.item_stack_swing_animation(&stack),
            SwingAnimationSummary {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 13,
            }
        );

        ItemRuntimeWorldDefaults::from_runtime(&NativeItemRuntime::empty_for_test())
            .apply(&mut world);
        assert_eq!(
            world.item_stack_swing_animation(&stack),
            SwingAnimationSummary {
                animation_type: SwingAnimationTypeSummary::Whack,
                duration: 6,
            },
            "a later runtime generation atomically replaces the old prototype table"
        );
    }

    #[test]
    fn renderer_resource_owners_commit_inside_one_reload_barrier() {
        let source = include_str!("resource_reload.rs");
        let production = source
            .split_once("\n#[cfg(test)]")
            .expect("resource reload production/test boundary")
            .0;
        let transaction = source
            .find(".transactional_shader_resource_reload(&shader_resources, false, |renderer|")
            .expect("reload should commit one shader generation with accepted renderer owners");
        let atlas_commit = source
            .find("upload_reload_atlases(renderer, &prepared.particles.render, &prepared.items)")
            .expect("reload should install generation-matched atlases");
        let terrain_commit = source
            .find("let textures = try_load_terrain_textures")
            .expect("reload should upload terrain material metadata inside the transaction");
        let hud_commit = source
            .find("try_load_hud_textures(renderer, &prepared.roots)")
            .expect("reload should install generation-matched HUD resources");
        let panorama_commit = source
            .find("upload_panorama(renderer, &prepared.panorama)")
            .expect("reload should install the generation-matched panorama owner");
        let language_direction_commit = source
            .find("set_default_text_right_to_left(prepared.items.language_bidirectional())")
            .expect("reload should install the prepared language direction");
        let win_screen_commit = source
            .find("let win_screen_resources = WinScreenResources::load(")
            .expect("reload should measure win-screen text with the prepared direction");
        let residency_barrier = source
            .find("renderer.invalidate_item_model_resources();")
            .expect("successful reload should advance the item residency generation");
        let audio_commit = source
            .find("audio.commit_reload(reload);")
            .expect("audio should commit inside the renderer transaction barrier");
        let simulation_commit = source
            .find("    prepared\n        .particles\n        .simulation")
            .expect("reload should commit the matching simulation resources");
        let transaction_result = source
            .find("stage: \"commit\"")
            .expect("fallible renderer transaction result");
        let skin_owner_adoption = source
            .find("prepared.items.adopt_profile_texture_runtime(previous)")
            .expect("successful reload should move the process skin owner once");
        let item_publish = source
            .find("*item_runtime = Some(prepared.items);")
            .expect("prepared item generation publish");
        let terrain_publish = source
            .find("*terrain_textures = next_terrain_textures;")
            .expect("prepared terrain material generation publish");

        assert!(transaction < atlas_commit);
        assert!(atlas_commit < terrain_commit);
        assert!(terrain_commit < hud_commit);
        assert!(hud_commit < panorama_commit);
        assert!(panorama_commit < language_direction_commit);
        assert!(language_direction_commit < win_screen_commit);
        assert!(atlas_commit < audio_commit);
        assert!(audio_commit < residency_barrier);
        assert!(residency_barrier < simulation_commit);
        assert!(transaction_result < skin_owner_adoption);
        assert!(transaction_result < terrain_publish);
        assert!(skin_owner_adoption < item_publish);
        assert_eq!(
            production
                .matches("adopt_profile_texture_runtime(previous)")
                .count(),
            1,
            "resource reload must have one post-commit SkinManager owner transfer"
        );
        let shader_prepare = source
            .find("PostEffectCatalog::load(roots)")
            .expect("shader sources and post effects should share one CPU generation owner");
        let periodic_prepare = source
            .find(".load_periodic_notification_catalog()")
            .expect("regional notifications should be prepared on the CPU boundary");
        let panorama_prepare = source
            .find(".load_panorama_catalog()")
            .expect("panorama images should be prepared on the CPU boundary");
        let prepared_generation = source
            .find("        post_effects,\n")
            .expect("prepared reload should retain the post-effect catalog");
        let prepared_panorama = source
            .find("        panorama,\n")
            .expect("prepared reload should retain the panorama catalog");
        assert!(shader_prepare < prepared_generation);
        assert!(panorama_prepare < prepared_panorama);
        assert!(periodic_prepare < prepared_generation);
        assert_eq!(production.matches("upload_panorama(").count(), 1);
        assert!(!production.contains("VanillaAssetIndex"));
        let removed_index_resolver = ["resource_with_indexed_vanilla_", "fall", "back"].concat();
        assert!(!production.contains(&removed_index_resolver));
        assert!(!production.contains("ShaderSourceCatalog"));
        let legacy_recovery = ["recover", "reload", "resources"].join("_");
        assert!(!source.contains(&legacy_recovery));
    }

    #[test]
    fn shader_prepare_and_generation_commit_have_one_native_path() {
        let reload = include_str!("resource_reload.rs");
        let main = include_str!("main.rs");
        let reload_production = reload
            .split_once("\n#[cfg(test)]")
            .expect("resource reload production/test boundary")
            .0;
        let main_production = main;
        assert_eq!(
            reload_production
                .matches("PostEffectCatalog::load(")
                .count()
                + main_production.matches("PostEffectCatalog::load(").count(),
            1,
            "startup and reload must share one shader preparation owner"
        );
        assert_eq!(
            reload_production
                .matches(".transactional_shader_resource_reload(")
                .count(),
            1,
            "server-pack reload must commit one shader generation"
        );
        assert_eq!(
            main_production
                .matches(".transactional_shader_resource_reload(")
                .count(),
            1,
            "startup must commit one shader generation"
        );
        for legacy in [".reload_core_shaders(", ".reload_post_effect_graphs("] {
            assert_eq!(reload_production.matches(legacy).count(), 0);
            assert_eq!(main_production.matches(legacy).count(), 0);
        }
        let transaction = reload
            .find(".transactional_shader_resource_reload(&shader_resources, false, |renderer|")
            .expect("unified renderer transaction");
        let atlas = reload
            .find("upload_reload_atlases(renderer, &prepared.particles.render, &prepared.items)")
            .expect("generation-matched atlas commit");
        let transaction_result = reload
            .find("stage: \"commit\"")
            .expect("renderer transaction result");
        let periodic = reload
            .find("hud_render_runtime.install_periodic_notifications(")
            .expect("periodic schedule commit");
        assert!(transaction < atlas && atlas < transaction_result);
        assert!(
            transaction_result < periodic,
            "the non-fallible periodic schedule commit must follow a successful GPU/audio barrier"
        );
        assert!(!reload_production.contains("shader_sources:"));
        assert!(!reload_production.contains("_periodic_notifications:"));
    }
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn server_resource_pack_roots_follow_startup_dirs_in_push_order() {
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
        let first_metadata = bbb_pack::read_client_resource_pack_metadata(&server_first).unwrap();
        let second_metadata = bbb_pack::read_client_resource_pack_metadata(&server_second).unwrap();

        let composed = compose_server_resource_pack_roots(
            &Some(base),
            &None,
            7,
            vec![
                SelectedClientPack::server("first", 1, &server_first, first_metadata).unwrap(),
                SelectedClientPack::server("second", 2, &server_second, second_metadata).unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(
            composed.resource_pack_dirs(),
            vec![
                startup_base,
                startup_top,
                server_first,
                server_second.clone(),
            ]
        );
        let location = bbb_pack::ResourceLocation::parse("minecraft:test/value.txt").unwrap();
        let resolved = composed.resource_stack().get_resource(&location).unwrap();
        assert_eq!(std::fs::read(&resolved.path).unwrap(), b"second");
        assert_eq!(resolved.pack_id, "server/00000000000000000002/second");
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
                        id: "server/00000000000000000001/first".to_string(),
                        source: "server:first@1".to_string(),
                        compatibility: "compatible".to_string(),
                        required: true,
                        fixed_position: true,
                    },
                    ClientResourcePackDiagnostic {
                        priority: 4,
                        id: "server/00000000000000000002/second".to_string(),
                        source: "server:second@2".to_string(),
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
            "bbb-native-resource-reload-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
