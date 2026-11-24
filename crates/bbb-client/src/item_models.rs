//! Item-model extraction for dropped, held, first-person and GUI consumers.

#[cfg(test)]
use bbb_client_render::ItemModelQuad;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use bbb_client_assets::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_client_render::terrain::TerrainCardinalLighting;
use bbb_client_render::{
    allay_hand_attach_transform, bake_first_person_map_background_surface,
    bake_first_person_map_decoration_surface, bake_first_person_map_text_surface,
    bake_item_frame_map_surface, copper_golem_antenna_block_transform,
    copper_golem_hand_attach_transform, custom_head_item_transforms,
    dolphin_carried_item_transform, enderman_carried_block_transform, falling_block_transform,
    fox_held_item_transform, humanoid_hand_attach_transforms, iron_golem_flower_block_transform,
    minecart_tnt_display_block_transform, mooshroom_mushroom_block_transforms,
    panda_held_item_transform, player_held_to_eye_transform, primed_tnt_block_transform,
    snow_golem_head_block_transform, villager_crossed_arms_item_transform,
    witch_held_item_transform, CameraPose, EntityModelInstance, EntityModelKind,
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture,
    FirstPersonPlayerArm, IllagerModelFamily, ItemFrameMapDecorationSurface,
    ItemFrameMapDecorationTexture, ItemFrameMapSurface, ItemFrameMapTextSurface,
    ItemFrameMapTexture, ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource,
    ItemModelAtlasKind, ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil,
    ItemModelGeometryClass, ItemModelInstanceRequest, ItemModelResidencyFrame,
    ItemPickupParticleRenderState, MooshroomVariant, PiglinModelFamily, SkeletonModelFamily,
    SpearKineticWeapon, SpecialItemInstanceRequest, ZombieVariantModelFamily,
    ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
use bbb_client_world::{BlockPos, ChunkPos, TerrainLight, WorldStore};
#[cfg(test)]
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
};
use bbb_protocol::item_types;
#[cfg(test)]
use bbb_protocol::packets::ItemStackTemplateSummary;
use bbb_protocol::packets::{
    ConsumableSummary, EquipmentSlot, InteractionHand, ItemStackSummary, ItemUseAnimationSummary,
    SwingAnimationTypeSummary,
};
use glam::{Mat4, Vec3};

use crate::map_textures::MapTextureExtractionCache;
use crate::terrain_upload::terrain_cardinal_lighting;
use bbb_client_render::item_model::{
    ItemModelUseContext, NativeItemRuntime, ResolvedItemModelLayer,
};
use bbb_client_render::scene::terrain_textures::TerrainTextureState;
#[cfg(test)]
use bbb_protocol::entity_types::*;

mod block_entity;
mod display;
mod dropped;
mod first_person;
mod hand_presentation;
mod smithing_preview;
mod special;
mod special_blocks;
mod stack;
#[cfg(test)]
mod test_meshes;
pub(crate) mod tooltip;
mod transforms;

#[cfg(test)]
use dropped::*;
#[cfg(test)]
use first_person::*;
use special_blocks::push_special_block_model;
use transforms::*;

pub(crate) use block_entity::block_entity_item_models;
pub(crate) use display::display_entity_item_models;
#[cfg(test)]
pub(crate) use dropped::held_item_models;
pub(crate) use dropped::{
    display_matrix, dropped_item_models, entity_block_models,
    held_item_models_with_variant_textures, item_pickup_particle_item_models, item_stack_foil_mode,
    ominous_item_spawner_models, push_resident_item_instances, MovingPistonVisibility,
};
#[cfg(test)]
pub(crate) use first_person::first_person_item_models;
#[cfg(test)]
pub(crate) use first_person::first_person_item_models_with_map_cache;
pub(crate) use first_person::{first_person_item_models_for_frame, first_person_player_arms};
pub(crate) use hand_presentation::FirstPersonHandPresentation;
pub(crate) use smithing_preview::smithing_entity_preview_item_models;
pub(crate) use special::{
    project_special_item_model, special_item_leaf_matrix, special_item_pack_texture_descriptors,
};
pub(crate) use stack::{bake_item_stack, ItemBakeEnv, ItemOwner, ItemPlacement, ItemSubmit};

/// Immutable holder-order keys shared by every item-model consumer in one
/// registry generation. The render extraction cache owns lifetime/revision;
/// held and first-person paths only borrow these Arc-backed slices.
#[derive(Debug, Clone, Default)]
pub(crate) struct ItemRegistryKeySet {
    pub(crate) trim_materials: Option<Arc<[String]>>,
    pub(crate) trim_patterns: Option<Arc<[String]>>,
    pub(crate) enchantments: Option<Arc<[String]>>,
    pub(crate) attributes: Option<Arc<[String]>>,
}

impl ItemRegistryKeySet {
    pub(crate) fn from_world(world: &WorldStore) -> Self {
        Self {
            trim_materials: world_trim_material_keys(world),
            trim_patterns: world_trim_pattern_keys(world),
            enchantments: world_enchantment_keys(world),
            attributes: world_attribute_keys(world),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TooltipRegistryCacheKey {
    world_ptr: usize,
    level_generation: u64,
    revision: bbb_client_world::RegistryAccessRevision,
    header_fingerprint: u64,
}

type TooltipKeyedContext<T> = Option<(Arc<[String]>, Arc<[Option<T>]>)>;

#[derive(Default)]
struct WorldTooltipRegistryContext {
    enchantment_keys: std::sync::OnceLock<Option<Arc<[String]>>>,
    trim_material_keys: std::sync::OnceLock<Option<Arc<[String]>>>,
    trim_pattern_keys: std::sync::OnceLock<Option<Arc<[String]>>>,
    attribute_keys: std::sync::OnceLock<Option<Arc<[String]>>>,
    instrument_context:
        std::sync::OnceLock<TooltipKeyedContext<bbb_client_world::InstrumentPresentation>>,
    trim_material_context:
        std::sync::OnceLock<TooltipKeyedContext<bbb_client_world::TrimMaterialPresentation>>,
    trim_pattern_context:
        std::sync::OnceLock<TooltipKeyedContext<bbb_client_world::TrimPatternPresentation>>,
    jukebox_song_context:
        std::sync::OnceLock<TooltipKeyedContext<bbb_client_world::JukeboxSongPresentation>>,
    banner_pattern_context:
        std::sync::OnceLock<TooltipKeyedContext<bbb_client_world::BannerPatternElement>>,
    painting_variant_context:
        std::sync::OnceLock<Option<Arc<[Option<bbb_client_world::PaintingVariantPresentation>]>>>,
    enchantment_definitions:
        std::sync::OnceLock<Option<Arc<[Option<bbb_client_world::EnchantmentTooltipDefinition>]>>>,
}

fn tooltip_registry_header_fingerprint(world: &WorldStore) -> u64 {
    use std::hash::{Hash, Hasher};
    const TOOLTIP_REGISTRIES: [&str; 8] = [
        bbb_client_world::ENCHANTMENT_REGISTRY,
        "minecraft:attribute",
        bbb_client_world::TRIM_MATERIAL_REGISTRY,
        bbb_client_world::TRIM_PATTERN_REGISTRY,
        bbb_client_world::INSTRUMENT_REGISTRY,
        bbb_client_world::JUKEBOX_SONG_REGISTRY,
        bbb_client_world::BANNER_PATTERN_REGISTRY,
        bbb_client_world::PAINTING_VARIANT_REGISTRY,
    ];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for registry in TOOLTIP_REGISTRIES {
        if let Some(content) = world.registry_content(registry) {
            true.hash(&mut hasher);
            content.entries.len().hash(&mut hasher);
            content.entries.as_ptr().hash(&mut hasher);
            if let Some(first) = content.entries.first() {
                first.id.hash(&mut hasher);
                first.has_data.hash(&mut hasher);
                first.raw_data_len.hash(&mut hasher);
            }
            if let Some(last) = content.entries.last() {
                last.id.hash(&mut hasher);
                last.has_data.hash(&mut hasher);
                last.raw_data_len.hash(&mut hasher);
            }
        } else {
            false.hash(&mut hasher);
        }
    }
    if let Some(tags) = world.registry_tags(bbb_client_world::ENCHANTMENT_REGISTRY) {
        tags.tags.len().hash(&mut hasher);
    }
    hasher.finish()
}

fn cached_world_tooltip_registry_context(world: &WorldStore) -> Arc<WorldTooltipRegistryContext> {
    thread_local! {
        static TOOLTIP_REGISTRY_CACHE: std::cell::RefCell<
            Option<(TooltipRegistryCacheKey, Arc<WorldTooltipRegistryContext>)>,
        > = const { std::cell::RefCell::new(None) };
    }
    let key = TooltipRegistryCacheKey {
        world_ptr: std::ptr::from_ref(world) as usize,
        level_generation: world.client_level_generation(),
        revision: world.registry_access_revision(),
        header_fingerprint: tooltip_registry_header_fingerprint(world),
    };
    TOOLTIP_REGISTRY_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();
        if let Some((cached_key, cached_context)) = cache.as_ref() {
            if *cached_key == key {
                return Arc::clone(cached_context);
            }
        }
        let context = Arc::new(WorldTooltipRegistryContext::default());
        *cache = Some((key, Arc::clone(&context)));
        context
    })
}

/// Item-model registry keys indexed by network holder id. Materialized once per
/// registry revision and shared via Arc slices across tooltip and model consumers.
pub(crate) fn world_enchantment_keys(world: &WorldStore) -> Option<Arc<[String]>> {
    cached_world_tooltip_registry_context(world)
        .enchantment_keys
        .get_or_init(|| world_registry_keys(world, "minecraft:enchantment"))
        .clone()
}

pub(crate) fn world_trim_material_keys(world: &WorldStore) -> Option<Arc<[String]>> {
    cached_world_tooltip_registry_context(world)
        .trim_material_keys
        .get_or_init(|| world_registry_keys(world, "minecraft:trim_material"))
        .clone()
}

pub(crate) fn world_trim_pattern_keys(world: &WorldStore) -> Option<Arc<[String]>> {
    cached_world_tooltip_registry_context(world)
        .trim_pattern_keys
        .get_or_init(|| world_registry_keys(world, "minecraft:trim_pattern"))
        .clone()
}

pub(crate) fn world_instrument_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::InstrumentPresentation> {
    cached_world_tooltip_registry_context(world)
        .instrument_context
        .get_or_init(|| build_world_instrument_tooltip_context(world))
        .clone()
}

fn build_world_instrument_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::InstrumentPresentation> {
    let resource_ids =
        world.registry_resource_ids_in_holder_order(bbb_client_world::INSTRUMENT_REGISTRY)?;
    let mut keys = Vec::new();
    let mut presentations = Vec::new();
    for (index, resource_id) in resource_ids.enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection =
            holder_id.map(|holder_id| world.synchronized_instrument_presentation(holder_id));
        let data_less_stub = holder_id
            .and_then(|holder_id| {
                world.registry_entry_by_holder_id(bbb_client_world::INSTRUMENT_REGISTRY, holder_id)
            })
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Some(presentation)) => {
                keys.push(resource_id.to_string());
                presentations.push(Some(presentation.clone()));
            }
            Some(None)
                if data_less_stub
                    && bbb_client_world::VanillaInstrumentIdentity::for_resource_id(
                        resource_id,
                    )
                    .is_some() =>
            {
                keys.push(resource_id.to_string());
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((Arc::from(keys), Arc::from(presentations)))
}

pub(crate) fn world_trim_material_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::TrimMaterialPresentation> {
    cached_world_tooltip_registry_context(world)
        .trim_material_context
        .get_or_init(|| build_world_trim_material_tooltip_context(world))
        .clone()
}

fn build_world_trim_material_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::TrimMaterialPresentation> {
    let resource_ids =
        world.registry_resource_ids_in_holder_order(bbb_client_world::TRIM_MATERIAL_REGISTRY)?;
    let mut keys = Vec::new();
    let mut presentations = Vec::new();
    for (index, resource_id) in resource_ids.enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection = holder_id.map(|id| world.synchronized_trim_material_presentation(id));
        let data_less_stub = holder_id
            .and_then(|id| {
                world.registry_entry_by_holder_id(bbb_client_world::TRIM_MATERIAL_REGISTRY, id)
            })
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Some(presentation)) => {
                keys.push(resource_id.to_string());
                presentations.push(Some(presentation.clone()));
            }
            Some(None)
                if data_less_stub
                    && bbb_client_world::VanillaTrimMaterialIdentity::for_resource_id(
                        resource_id,
                    )
                    .is_some() =>
            {
                keys.push(resource_id.to_string());
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((Arc::from(keys), Arc::from(presentations)))
}

pub(crate) fn world_trim_pattern_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::TrimPatternPresentation> {
    cached_world_tooltip_registry_context(world)
        .trim_pattern_context
        .get_or_init(|| build_world_trim_pattern_tooltip_context(world))
        .clone()
}

fn build_world_trim_pattern_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::TrimPatternPresentation> {
    let resource_ids =
        world.registry_resource_ids_in_holder_order(bbb_client_world::TRIM_PATTERN_REGISTRY)?;
    let mut keys = Vec::new();
    let mut presentations = Vec::new();
    for (index, resource_id) in resource_ids.enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection = holder_id.map(|id| world.synchronized_trim_pattern_presentation(id));
        let data_less_stub = holder_id
            .and_then(|id| {
                world.registry_entry_by_holder_id(bbb_client_world::TRIM_PATTERN_REGISTRY, id)
            })
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Some(presentation)) => {
                keys.push(resource_id.to_string());
                presentations.push(Some(presentation.clone()));
            }
            Some(None)
                if data_less_stub
                    && bbb_client_world::VanillaTrimPatternIdentity::for_resource_id(
                        resource_id,
                    )
                    .is_some() =>
            {
                keys.push(resource_id.to_string());
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((Arc::from(keys), Arc::from(presentations)))
}

pub(crate) fn world_jukebox_song_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::JukeboxSongPresentation> {
    cached_world_tooltip_registry_context(world)
        .jukebox_song_context
        .get_or_init(|| build_world_jukebox_song_tooltip_context(world))
        .clone()
}

fn build_world_jukebox_song_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::JukeboxSongPresentation> {
    let resource_ids =
        world.registry_resource_ids_in_holder_order(bbb_client_world::JUKEBOX_SONG_REGISTRY)?;
    let mut keys = Vec::new();
    let mut presentations = Vec::new();
    for (index, resource_id) in resource_ids.enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection =
            holder_id.map(|holder_id| world.synchronized_jukebox_song_presentation(holder_id));
        let data_less_stub = holder_id
            .and_then(|holder_id| {
                world
                    .registry_entry_by_holder_id(bbb_client_world::JUKEBOX_SONG_REGISTRY, holder_id)
            })
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Some(presentation)) => {
                keys.push(resource_id.to_string());
                presentations.push(Some(presentation.clone()));
            }
            Some(None)
                if data_less_stub
                    && bbb_client_assets::JukeboxSongRegistry::vanilla_26_1_sound_event_id(
                        resource_id,
                    )
                    .is_some() =>
            {
                keys.push(resource_id.to_string());
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((Arc::from(keys), Arc::from(presentations)))
}

pub(crate) fn world_banner_pattern_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::BannerPatternElement> {
    cached_world_tooltip_registry_context(world)
        .banner_pattern_context
        .get_or_init(|| build_world_banner_pattern_tooltip_context(world))
        .clone()
}

fn build_world_banner_pattern_tooltip_context(
    world: &WorldStore,
) -> TooltipKeyedContext<bbb_client_world::BannerPatternElement> {
    let resource_ids =
        world.registry_resource_ids_in_holder_order(bbb_client_world::BANNER_PATTERN_REGISTRY)?;
    let mut keys = Vec::new();
    let mut presentations = Vec::new();
    for (index, resource_id) in resource_ids.enumerate() {
        let projection = i32::try_from(index)
            .ok()
            .map(|holder_id| world.synchronized_banner_pattern_presentation(holder_id));
        match projection {
            Some(Some(presentation)) => {
                keys.push(resource_id.to_string());
                presentations.push(Some(presentation.into_owned()));
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((Arc::from(keys), Arc::from(presentations)))
}

pub(crate) fn world_painting_variant_tooltip_context(
    world: &WorldStore,
) -> Option<Arc<[Option<bbb_client_world::PaintingVariantPresentation>]>> {
    cached_world_tooltip_registry_context(world)
        .painting_variant_context
        .get_or_init(|| build_world_painting_variant_tooltip_context(world))
        .clone()
}

fn build_world_painting_variant_tooltip_context(
    world: &WorldStore,
) -> Option<Arc<[Option<bbb_client_world::PaintingVariantPresentation>]>> {
    let resource_ids =
        world.registry_resource_ids_in_holder_order(bbb_client_world::PAINTING_VARIANT_REGISTRY)?;
    let presentations: Vec<_> = resource_ids
        .enumerate()
        .map(|(index, _)| {
            i32::try_from(index).ok().and_then(|holder_id| {
                world
                    .painting_variant_presentation(holder_id)
                    .map(|presentation| presentation.into_owned())
            })
        })
        .collect();
    Some(Arc::from(presentations))
}

pub(crate) fn world_enchantment_tooltip_definitions(
    world: &WorldStore,
) -> Option<Arc<[Option<bbb_client_world::EnchantmentTooltipDefinition>]>> {
    cached_world_tooltip_registry_context(world)
        .enchantment_definitions
        .get_or_init(|| {
            world
                .enchantment_tooltip_definitions_in_holder_order()
                .map(Arc::from)
        })
        .clone()
}

pub(crate) fn world_attribute_keys(world: &WorldStore) -> Option<Arc<[String]>> {
    cached_world_tooltip_registry_context(world)
        .attribute_keys
        .get_or_init(|| world_registry_keys(world, "minecraft:attribute"))
        .clone()
}

fn world_registry_keys(world: &WorldStore, registry: &str) -> Option<Arc<[String]>> {
    world
        .registry_resource_ids_in_holder_order(registry)
        .map(|resource_ids| Arc::from(resource_ids.map(str::to_string).collect::<Vec<_>>()))
}

pub(crate) fn crossbow_charge_duration_ticks(
    item_runtime: &NativeItemRuntime,
    world: &WorldStore,
    enchantments: &[bbb_protocol::packets::ItemEnchantmentSummary],
) -> i32 {
    item_runtime.with_presentation_random(|random| {
        world.crossbow_charge_duration_ticks_with_random(enchantments, random)
    })
}

#[cfg(test)]
mod tests;
