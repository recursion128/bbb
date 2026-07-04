//! Dropped-item 3D models: turns dropped item entities into baked item-model meshes for the renderer's
//! item-model pass, replacing the flat billboard. A dropped item whose item is a block bakes its block
//! render shape over the blocks atlas (the block path); every other item extrudes its flat sprite into a
//! `1/16`-thick slab over the item atlas (the generated/flat path, vanilla `builtin/generated`). Both
//! are placed by vanilla `ItemEntityRenderer.submit` (the bob lift + Y spin) composed with the model's
//! GROUND display transform, and a stack of items renders as the vanilla cluster of `1..=5` jittered
//! copies (`submitMultipleFromCount`). Ominous item spawners reuse the same item-cluster bake with their
//! renderer-owned scale-in and spin transform.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_protocol::packets::{
    ConsumableSummary, EquipmentSlot, InteractionHand, ItemStackSummary, ItemStackTemplateSummary,
    ItemUseAnimationSummary, SwingAnimationTypeSummary,
};
use bbb_renderer::{
    allay_hand_attach_transform, bake_first_person_map_background_surface,
    bake_first_person_map_decoration_surface, bake_first_person_map_text_surface,
    bake_generated_item_quads, bake_item_frame_map_surface,
    bake_item_model_mesh_with_light_and_overlay,
    bake_item_model_meshes_with_light_and_overlay_and_foil_mode,
    copper_golem_antenna_block_transform, copper_golem_hand_attach_transform,
    custom_head_item_transforms, dolphin_carried_item_transform, enderman_carried_block_transform,
    falling_block_transform, fox_held_item_transform, humanoid_hand_attach_transforms,
    iron_golem_flower_block_transform, minecart_tnt_display_block_transform,
    mooshroom_mushroom_block_transforms, panda_held_item_transform, primed_tnt_block_transform,
    snow_golem_head_block_transform, villager_crossed_arms_item_transform,
    witch_held_item_transform, CameraPose, EntityModelInstance, EntityModelKind,
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture,
    FirstPersonPlayerArm, HumanoidModelFamily, IllagerModelFamily, ItemFrameMapDecorationSurface,
    ItemFrameMapDecorationTexture, ItemFrameMapSurface, ItemFrameMapTextSurface,
    ItemFrameMapTexture, ItemModelFoil, ItemModelMesh, ItemModelMeshSet, ItemModelQuad,
    ItemPickupParticleRenderState, MooshroomVariant, PiglinModelFamily, SkeletonModelFamily,
    SpearKineticWeapon, ZombieVariantModelFamily, ITEM_MODEL_FULL_BRIGHT_LIGHT,
    ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::{BlockPos, TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::entity_scene::default_spear_kinetic_weapon_for_resource_id;
use crate::map_textures::map_item_texture;
use crate::terrain_runtime::TerrainTextureState;
use bbb_item_model::{ItemModelUseContext, NativeItemRuntime};
use bbb_protocol::entity_types::*;

mod dropped;
mod first_person;
mod transforms;

use dropped::*;
use first_person::*;
use transforms::*;

pub(crate) use dropped::{
    display_matrix, dropped_item_models, entity_block_models, held_item_models,
    item_pickup_particle_item_models, item_stack_foil_mode, ominous_item_spawner_models,
};
pub(crate) use first_person::{first_person_item_models, first_person_player_arms};

#[cfg(test)]
mod tests;
