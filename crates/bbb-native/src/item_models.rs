//! Item-model extraction: resolves dropped, held, first-person, item-frame, particle, and attachment
//! consumers into stable model identities plus narrow resident instances. A dropped item whose item is a block resolves its block
//! render shape over the blocks atlas (the block path); every other item extrudes its flat sprite into a
//! `1/16`-thick slab source over the item atlas (the generated/flat path, vanilla `builtin/generated`). Both
//! are placed by vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the model's
//! GROUND display transform, and a stack of items renders as the vanilla cluster of `1..=5` jittered
//! copies (`submitMultipleFromCount`). Ominous item spawners reuse the same item-cluster transforms with their
//! renderer-owned scale-in and spin transform.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
};
use bbb_protocol::item_types;
#[cfg(test)]
use bbb_protocol::packets::ItemStackTemplateSummary;
use bbb_protocol::packets::{
    ConsumableSummary, DataComponentPatchSummary, EquipmentSlot, InteractionHand, ItemStackSummary,
    ItemUseAnimationSummary, SwingAnimationTypeSummary,
};
use bbb_renderer::terrain::TerrainCardinalLighting;
use bbb_renderer::{
    allay_hand_attach_transform, bake_first_person_map_background_surface,
    bake_first_person_map_decoration_surface, bake_first_person_map_text_surface,
    bake_item_frame_map_surface, copper_golem_antenna_block_transform,
    copper_golem_hand_attach_transform, custom_head_item_transforms,
    dolphin_carried_item_transform, enderman_carried_block_transform, falling_block_transform,
    fox_held_item_transform, humanoid_hand_attach_transforms, iron_golem_flower_block_transform,
    minecart_tnt_display_block_transform, mooshroom_mushroom_block_transforms,
    panda_held_item_transform, primed_tnt_block_transform, snow_golem_head_block_transform,
    villager_crossed_arms_item_transform, witch_held_item_transform, CameraPose,
    EntityModelInstance, EntityModelKind, FirstPersonMapBackgroundKind,
    FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture, FirstPersonPlayerArm,
    HumanoidModelFamily, IllagerModelFamily, ItemFrameMapDecorationSurface,
    ItemFrameMapDecorationTexture, ItemFrameMapSurface, ItemFrameMapTextSurface,
    ItemFrameMapTexture, ItemModelAssetIdentity, ItemModelAssetRequest, ItemModelAssetSource,
    ItemModelAtlasKind, ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil,
    ItemModelGeometryClass, ItemModelInstanceRequest, ItemModelMesh, ItemModelQuad,
    ItemModelResidencyFrame, ItemPickupParticleRenderState, MooshroomVariant, PiglinModelFamily,
    SkeletonModelFamily, SpearKineticWeapon, SpecialItemInstanceRequest, ZombieVariantModelFamily,
    ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
#[cfg(test)]
use bbb_renderer::{
    bake_generated_item_quads, bake_item_model_meshes_with_light_and_overlay_and_foil_mode,
    ItemModelMeshSet,
};
use bbb_world::{BlockPos, ChunkPos, TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::entity_scene::default_spear_kinetic_weapon_for_resource_id;
use crate::map_textures::MapTextureExtractionCache;
use crate::terrain_runtime::{terrain_cardinal_lighting, BlockRenderPosition, TerrainTextureState};
use bbb_item_model::{ItemModelUseContext, NativeItemRuntime, ResolvedItemModelLayer};
#[cfg(test)]
use bbb_protocol::entity_types::*;

mod block_entity;
mod display;
mod dropped;
mod first_person;
mod hand_presentation;
mod smithing_preview;
mod special;
mod transforms;

#[cfg(test)]
use dropped::*;
#[cfg(test)]
use first_person::*;
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
pub(crate) use first_person::{first_person_item_models_with_map_cache, first_person_player_arms};
pub(crate) use hand_presentation::FirstPersonHandPresentation;
pub(crate) use smithing_preview::smithing_entity_preview_item_models;
pub(crate) use special::{
    project_special_item_model, special_item_leaf_matrix, special_item_pack_texture_descriptors,
};

/// Item-model registry keys indexed by network holder id. The checked world
/// registry API owns wire-order identity; these wrappers only materialize the
/// owned slices required by the item-model runtime.
pub(crate) fn world_enchantment_keys(world: &WorldStore) -> Option<Vec<String>> {
    world_registry_keys(world, "minecraft:enchantment")
}

pub(crate) fn world_damage_type_keys(world: &WorldStore) -> Option<Vec<String>> {
    world_registry_keys(world, "minecraft:damage_type")
}

pub(crate) fn world_trim_material_keys(world: &WorldStore) -> Option<Vec<String>> {
    world_registry_keys(world, "minecraft:trim_material")
}

pub(crate) fn world_trim_pattern_keys(world: &WorldStore) -> Option<Vec<String>> {
    world_registry_keys(world, "minecraft:trim_pattern")
}

pub(crate) fn world_instrument_tooltip_context(
    world: &WorldStore,
) -> Option<(Vec<String>, Vec<Option<bbb_world::InstrumentPresentation>>)> {
    let resource_ids = world_registry_keys(world, bbb_world::INSTRUMENT_REGISTRY)?;
    let mut keys = Vec::with_capacity(resource_ids.len());
    let mut presentations = Vec::with_capacity(resource_ids.len());
    for (index, resource_id) in resource_ids.into_iter().enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection =
            holder_id.map(|holder_id| world.synchronized_instrument_presentation(holder_id));
        let data_less_stub = holder_id
            .and_then(|holder_id| {
                world.registry_entry_by_holder_id(bbb_world::INSTRUMENT_REGISTRY, holder_id)
            })
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Ok(Some(presentation))) => {
                keys.push(resource_id);
                presentations.push(Some(presentation));
            }
            Some(Ok(None))
                if data_less_stub
                    && bbb_world::VanillaInstrumentIdentity::for_resource_id(&resource_id)
                        .is_some() =>
            {
                keys.push(resource_id);
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((keys, presentations))
}

pub(crate) fn world_trim_material_tooltip_context(
    world: &WorldStore,
) -> Option<(
    Vec<String>,
    Vec<Option<bbb_world::TrimMaterialPresentation>>,
)> {
    let resource_ids = world_trim_material_keys(world)?;
    let mut keys = Vec::with_capacity(resource_ids.len());
    let mut presentations = Vec::with_capacity(resource_ids.len());
    for (index, resource_id) in resource_ids.into_iter().enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection = holder_id.map(|id| world.synchronized_trim_material_presentation(id));
        let data_less_stub = holder_id
            .and_then(|id| world.registry_entry_by_holder_id(bbb_world::TRIM_MATERIAL_REGISTRY, id))
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Ok(Some(presentation))) => {
                keys.push(resource_id);
                presentations.push(Some(presentation));
            }
            Some(Ok(None))
                if data_less_stub
                    && bbb_world::VanillaTrimMaterialIdentity::for_resource_id(&resource_id)
                        .is_some() =>
            {
                keys.push(resource_id);
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((keys, presentations))
}

pub(crate) fn world_trim_pattern_tooltip_context(
    world: &WorldStore,
) -> Option<(Vec<String>, Vec<Option<bbb_world::TrimPatternPresentation>>)> {
    let resource_ids = world_trim_pattern_keys(world)?;
    let mut keys = Vec::with_capacity(resource_ids.len());
    let mut presentations = Vec::with_capacity(resource_ids.len());
    for (index, resource_id) in resource_ids.into_iter().enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection = holder_id.map(|id| world.synchronized_trim_pattern_presentation(id));
        let data_less_stub = holder_id
            .and_then(|id| world.registry_entry_by_holder_id(bbb_world::TRIM_PATTERN_REGISTRY, id))
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Ok(Some(presentation))) => {
                keys.push(resource_id);
                presentations.push(Some(presentation));
            }
            Some(Ok(None))
                if data_less_stub
                    && bbb_world::VanillaTrimPatternIdentity::for_resource_id(&resource_id)
                        .is_some() =>
            {
                keys.push(resource_id);
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((keys, presentations))
}

pub(crate) fn world_jukebox_song_tooltip_context(
    world: &WorldStore,
) -> Option<(Vec<String>, Vec<Option<bbb_world::JukeboxSongPresentation>>)> {
    let resource_ids = world_registry_keys(world, bbb_world::JUKEBOX_SONG_REGISTRY)?;
    let mut keys = Vec::with_capacity(resource_ids.len());
    let mut presentations = Vec::with_capacity(resource_ids.len());
    for (index, resource_id) in resource_ids.into_iter().enumerate() {
        let holder_id = i32::try_from(index).ok();
        let projection =
            holder_id.map(|holder_id| world.synchronized_jukebox_song_presentation(holder_id));
        let data_less_stub = holder_id
            .and_then(|holder_id| {
                world.registry_entry_by_holder_id(bbb_world::JUKEBOX_SONG_REGISTRY, holder_id)
            })
            .is_some_and(|entry| !entry.has_data);
        match projection {
            Some(Ok(Some(presentation))) => {
                keys.push(resource_id);
                presentations.push(Some(presentation));
            }
            Some(Ok(None))
                if data_less_stub
                    && bbb_pack::JukeboxSongRegistry::vanilla_26_1_sound_event_id(&resource_id)
                        .is_some() =>
            {
                keys.push(resource_id);
                presentations.push(None);
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((keys, presentations))
}

pub(crate) fn world_banner_pattern_tooltip_context(
    world: &WorldStore,
) -> Option<(
    Vec<String>,
    Vec<Option<bbb_world::BannerPatternPresentation>>,
)> {
    let resource_ids = world_registry_keys(world, bbb_world::BANNER_PATTERN_REGISTRY)?;
    let mut keys = Vec::with_capacity(resource_ids.len());
    let mut presentations = Vec::with_capacity(resource_ids.len());
    for (index, resource_id) in resource_ids.into_iter().enumerate() {
        let projection = i32::try_from(index)
            .ok()
            .map(|holder_id| world.synchronized_banner_pattern_presentation(holder_id));
        match projection {
            Some(Ok(Some(presentation))) => {
                keys.push(resource_id);
                presentations.push(Some(presentation));
            }
            _ => {
                keys.push(String::new());
                presentations.push(None);
            }
        }
    }
    Some((keys, presentations))
}

pub(crate) fn world_painting_variant_tooltip_context(
    world: &WorldStore,
) -> Option<Vec<Option<bbb_world::PaintingVariantPresentation>>> {
    let resource_ids = world_registry_keys(world, bbb_world::PAINTING_VARIANT_REGISTRY)?;
    Some(
        resource_ids
            .iter()
            .enumerate()
            .map(|(index, _)| {
                i32::try_from(index).ok().and_then(|holder_id| {
                    world
                        .painting_variant_presentation(holder_id)
                        .ok()
                        .flatten()
                })
            })
            .collect(),
    )
}

pub(crate) fn world_attribute_keys(world: &WorldStore) -> Option<Vec<String>> {
    world_registry_keys(world, "minecraft:attribute")
}

fn world_registry_keys(world: &WorldStore, registry: &str) -> Option<Vec<String>> {
    world
        .registry_resource_ids_in_holder_order(registry)
        .map(|resource_ids| resource_ids.map(str::to_string).collect())
}

#[cfg(test)]
mod tests;
