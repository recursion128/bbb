use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use bbb_control::AudioCounters;
use bbb_item_model::NativeItemRuntime;
use bbb_pack::{
    BlockDestroyProfile as PackBlockDestroyProfile, BlockDestroyProfileCatalog,
    BlockSoundProfile as PackBlockSoundProfile, BlockSoundProfileCatalog, FurnaceFuelCatalog,
    FurnaceFuelSourceRule, PackRoots,
};
use bbb_renderer::{ParticleSpriteUv, ParticleUvRect};
use bbb_world::{FurnaceFuelRule, WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldStore};

use crate::{
    audio_runtime::{NativeAudioReload, NativeAudioRuntime},
    entity_assets::try_load_entity_model_textures,
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    hud_assets::try_load_hud_textures,
    particle_runtime::NativeParticleRuntime,
    resource_pack_runtime::{ServerResourcePackReload, ServerResourcePackRuntime},
    runtime::synchronize_audio_runtime_from_world,
    sky_assets::try_load_sky_textures,
    terrain_runtime::{try_load_terrain_textures, TerrainTextureState, TerrainUploadState},
};

enum PreparedAudioReload {
    Existing(NativeAudioReload),
    Disabled,
}

struct PreparedNativeResourceReload {
    roots: PackRoots,
    audio: PreparedAudioReload,
    particles: NativeParticleRuntime,
    items: NativeItemRuntime,
    block_destroy_profiles: BTreeMap<String, WorldBlockDestroyProfile>,
    block_sound_profiles: BTreeMap<String, WorldBlockSoundProfile>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_pending_server_resource_pack_reload(
    server_packs: &mut ServerResourcePackRuntime,
    base_roots: &Option<PackRoots>,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    world: &mut WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    client_locale: &str,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    player_skin_cache_dir: &Path,
) {
    let Some(reload) = server_packs.take_reload() else {
        return;
    };
    let generation = reload.generation;
    let result = apply_native_resource_reload(
        reload,
        base_roots,
        active_roots,
        audio_runtime,
        audio_status,
        particle_runtime,
        item_runtime,
        terrain_textures,
        entity_variant_textures,
        terrain_upload,
        world,
        renderer,
        client_locale,
        particle_status,
        player_skin_cache_dir,
    );
    match result {
        Ok(()) => {
            server_packs.finish_reload(generation, true);
            tracing::info!(generation, "applied server resource pack reload");
        }
        Err(error) => {
            server_packs.finish_reload(generation, false);
            tracing::warn!(
                generation,
                error = %format!("{error:#}"),
                "server resource pack reload failed; retained previous resources"
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_native_resource_reload(
    reload: ServerResourcePackReload,
    base_roots: &Option<PackRoots>,
    active_roots: &mut Option<PackRoots>,
    audio_runtime: &mut Option<NativeAudioRuntime>,
    audio_status: &mut AudioCounters,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    item_runtime: &mut Option<NativeItemRuntime>,
    terrain_textures: &mut TerrainTextureState,
    entity_variant_textures: &mut NativeEntityVariantTextureRuntime,
    terrain_upload: &mut TerrainUploadState,
    world: &mut WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    client_locale: &str,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
    player_skin_cache_dir: &Path,
) -> Result<()> {
    let mut prepared = prepare_native_resource_reload(
        base_roots,
        reload.server_pack_roots,
        audio_runtime.is_some(),
        client_locale,
        particle_status,
    )?;
    prepared.items.enable_http_profile_resolution();
    prepared
        .items
        .enable_http_player_skin_downloads(player_skin_cache_dir.to_path_buf());

    let upload_result = upload_reload_atlases(renderer, &prepared.particles, &prepared.items)
        .and_then(|()| {
            try_load_terrain_textures(renderer, &prepared.roots)
                .context("upload terrain resources during resource reload")
        })
        .and_then(|textures| {
            try_load_hud_textures(renderer, &prepared.roots)
                .context("upload HUD resources during resource reload")?;
            try_load_entity_model_textures(renderer, &prepared.roots)
                .context("upload entity resources during resource reload")?;
            try_load_sky_textures(renderer, &prepared.roots)
                .context("upload sky resources during resource reload")?;
            Ok(textures)
        });
    let next_terrain_textures = match upload_result {
        Ok(textures) => textures,
        Err(error) => {
            recover_reload_resources(
                renderer,
                active_roots.as_ref(),
                particle_runtime.as_ref(),
                item_runtime.as_ref(),
            );
            return Err(error);
        }
    };
    prepared
        .particles
        .set_terrain_particle_sprite_ids(&next_terrain_textures);
    prepared
        .particles
        .set_default_item_particle_sprite_ids(&prepared.items);

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
        synchronize_audio_runtime_from_world(world, audio);
        *audio_status = audio.counters();
    }
    apply_item_runtime_world_defaults(&prepared.items, world);
    world.set_default_block_destroy_profiles(prepared.block_destroy_profiles);
    world.set_default_block_sound_profiles(prepared.block_sound_profiles);

    *terrain_textures = next_terrain_textures;
    *entity_variant_textures = NativeEntityVariantTextureRuntime::new(Some(prepared.roots.clone()));
    *particle_runtime = Some(prepared.particles);
    *item_runtime = Some(prepared.items);
    *active_roots = Some(prepared.roots);
    terrain_upload.request_reload_all_chunks();
    Ok(())
}

fn prepare_native_resource_reload(
    base_roots: &Option<PackRoots>,
    server_pack_roots: Vec<std::path::PathBuf>,
    has_audio_runtime: bool,
    client_locale: &str,
    particle_status: bbb_protocol::packets::ClientParticleStatus,
) -> Result<PreparedNativeResourceReload> {
    let roots = compose_server_resource_pack_roots(base_roots, server_pack_roots)?;
    let audio = if has_audio_runtime {
        PreparedAudioReload::Existing(NativeAudioRuntime::prepare_reload(&roots)?)
    } else {
        PreparedAudioReload::Disabled
    };
    let particles = NativeParticleRuntime::load(&roots, particle_status)
        .context("prepare particle resources for reload")?;
    let items = NativeItemRuntime::load_with_locale(&roots, client_locale)
        .context("prepare item, model, texture, and language resources for reload")?;
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
        block_destroy_profiles,
        block_sound_profiles,
    })
}

fn compose_server_resource_pack_roots(
    base_roots: &Option<PackRoots>,
    server_pack_roots: Vec<std::path::PathBuf>,
) -> Result<PackRoots> {
    let base_roots = base_roots
        .as_ref()
        .context("vanilla pack roots are unavailable")?;
    let mut resource_pack_dirs = base_roots.resource_pack_dirs.clone();
    resource_pack_dirs.extend(server_pack_roots);
    Ok(base_roots
        .clone()
        .with_resource_pack_dirs(resource_pack_dirs))
}

fn upload_reload_atlases(
    renderer: &mut bbb_renderer::Renderer,
    particles: &NativeParticleRuntime,
    items: &NativeItemRuntime,
) -> Result<()> {
    particles
        .upload_particle_atlas(renderer)
        .context("upload particle atlas during resource reload")?;
    upload_item_atlases(renderer, items).context("upload item atlases during resource reload")
}

fn recover_reload_resources(
    renderer: &mut bbb_renderer::Renderer,
    roots: Option<&PackRoots>,
    particles: Option<&NativeParticleRuntime>,
    items: Option<&NativeItemRuntime>,
) {
    if let Some(particles) = particles {
        if let Err(error) = particles.upload_particle_atlas(renderer) {
            tracing::warn!(?error, "failed to recover previous particle atlas");
        }
    }
    if let Some(items) = items {
        if let Err(error) = upload_item_atlases(renderer, items) {
            tracing::warn!(?error, "failed to recover previous item atlases");
        }
    }
    let Some(roots) = roots else {
        return;
    };
    if let Err(error) = try_load_terrain_textures(renderer, roots) {
        tracing::warn!(?error, "failed to recover previous terrain resources");
    }
    if let Err(error) = try_load_hud_textures(renderer, roots) {
        tracing::warn!(?error, "failed to recover previous HUD resources");
    }
    if let Err(error) = try_load_entity_model_textures(renderer, roots) {
        tracing::warn!(?error, "failed to recover previous entity resources");
    }
    if let Err(error) = try_load_sky_textures(renderer, roots) {
        tracing::warn!(?error, "failed to recover previous sky resources");
    }
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

pub(crate) fn apply_item_runtime_world_defaults(items: &NativeItemRuntime, world: &mut WorldStore) {
    world.set_default_item_max_stack_sizes(items.item_max_stack_sizes_by_protocol_id());
    world.set_default_item_max_damage(items.item_max_damage_by_protocol_id());
    world.set_default_item_crafting_remainders(items.item_crafting_remainders_by_protocol_id());
    world.set_recipe_specific_crafting_remainder_item_ids(
        items.recipe_specific_crafting_remainder_item_ids_by_protocol_id(),
    );
    world.set_default_item_equipment_slots(items.item_equipment_slots_by_protocol_id());
    world.set_item_armor_materials(items.item_armor_materials_by_protocol_id());
    world.set_default_item_wings_equipment_assets(
        items.item_wings_equipment_assets_by_protocol_id(),
    );
    world.set_default_mount_body_armor_kinds(items.mount_body_armor_kinds_by_protocol_id());
    world.set_default_llama_body_decor_colors(items.llama_body_decor_colors_by_protocol_id());
    world.set_default_happy_ghast_harness_colors(items.happy_ghast_harness_colors_by_protocol_id());
    world.set_default_nautilus_body_armor_materials(
        items.nautilus_body_armor_materials_by_protocol_id(),
    );
    world.set_default_horse_body_armor_materials(items.horse_body_armor_materials_by_protocol_id());
    world.set_default_wolf_body_armor_materials(items.wolf_body_armor_materials_by_protocol_id());
    world.set_default_item_attack_ranges(items.item_attack_ranges_by_protocol_id());
    world.set_default_item_swing_animation_durations(
        items.item_swing_animation_durations_by_protocol_id(),
    );
    world.set_default_piercing_weapon_item_ids(
        items.default_piercing_weapon_item_ids_by_protocol_id(),
    );
    world.set_default_damageable_item_ids(items.default_damageable_item_ids_by_protocol_id());
    world.set_default_item_use_effects(items.item_use_effects_by_protocol_id());
    let furnace_fuel_item_ids = items.furnace_fuel_item_ids_by_protocol_id();
    match FurnaceFuelCatalog::source_rules() {
        Ok(rules) => {
            let rules = rules
                .into_iter()
                .map(|rule| match rule {
                    FurnaceFuelSourceRule::AddItem(resource_id) => items
                        .item_protocol_id(&resource_id)
                        .map(FurnaceFuelRule::AddItem),
                    FurnaceFuelSourceRule::AddTag(tag) => Some(FurnaceFuelRule::AddTag(tag)),
                    FurnaceFuelSourceRule::RemoveTag(tag) => Some(FurnaceFuelRule::RemoveTag(tag)),
                })
                .collect::<Option<Vec<_>>>();
            if let Some(rules) = rules {
                world.set_furnace_fuel_profile(furnace_fuel_item_ids, rules);
            } else {
                tracing::warn!(
                    "continuing without session furnace-fuel refresh rules because a direct item is unresolved"
                );
                world.set_furnace_fuel_item_ids(furnace_fuel_item_ids);
            }
        }
        Err(error) => {
            tracing::warn!(
                ?error,
                "continuing without session furnace-fuel refresh rules"
            );
            world.set_furnace_fuel_item_ids(furnace_fuel_item_ids);
        }
    }
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
    use std::path::PathBuf;

    #[test]
    fn server_resource_pack_roots_follow_startup_dirs_in_push_order() {
        let base = PackRoots {
            mc_code_root: PathBuf::from("mc-code"),
            sources_dir: PathBuf::from("sources"),
            assets_dir: PathBuf::from("assets"),
            generated_assets_dir: None,
            resource_pack_dirs: vec![PathBuf::from("startup/base"), PathBuf::from("startup/top")],
        };

        let composed = compose_server_resource_pack_roots(
            &Some(base),
            vec![
                PathBuf::from("server/first"),
                PathBuf::from("server/second"),
            ],
        )
        .unwrap();

        assert_eq!(
            composed.resource_pack_dirs,
            vec![
                PathBuf::from("startup/base"),
                PathBuf::from("startup/top"),
                PathBuf::from("server/first"),
                PathBuf::from("server/second"),
            ]
        );
    }

    #[test]
    fn server_resource_pack_reload_requires_vanilla_base_roots() {
        let error = compose_server_resource_pack_roots(&None, vec![PathBuf::from("server/only")])
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("vanilla pack roots are unavailable"));
    }
}
