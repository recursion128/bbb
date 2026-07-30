//! Resident item-model extraction for Smithing's armor-stand picture-in-picture preview.

use bbb_pack::BlockModelDisplayContext;
use bbb_protocol::menu_types::VANILLA_MENU_TYPE_SMITHING_ID;
use bbb_renderer::{
    custom_head_item_transforms, humanoid_hand_attach_transforms,
    HudEntityPreviewItemDisplayContext, HudEntityPreviewItemSlot, HudInventoryScreen,
    ItemModelDrawTarget, ItemModelMesh, ItemModelResidencyFrame, ITEM_MODEL_FULL_BRIGHT_LIGHT,
    ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::WorldStore;

use super::dropped::bake_item_stack_at_transform_with_light_and_overlay;
use super::{
    NativeItemRuntime, TerrainTextureState, BLOCK_HEAD_FALLBACK, BLOCK_THIRD_PERSON_FALLBACK,
    GENERATED_HEAD_FALLBACK, GENERATED_THIRD_PERSON_FALLBACK,
};
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;

const SMITHING_RESULT_SLOT: i16 = 3;

#[allow(clippy::too_many_arguments)]
pub(crate) fn smithing_entity_preview_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    screen: Option<&HudInventoryScreen>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    trim_material_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> ItemModelResidencyFrame {
    let Some(item_runtime) = item_runtime else {
        return ItemModelResidencyFrame::default();
    };
    let Some(container) = world
        .inventory()
        .open_container
        .as_ref()
        .filter(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID))
    else {
        return ItemModelResidencyFrame::default();
    };
    let Some(stack) = container
        .slots
        .iter()
        .find(|slot| slot.slot == SMITHING_RESULT_SLOT)
        .map(|slot| &slot.item)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
    else {
        return ItemModelResidencyFrame::default();
    };
    let Some(screen) = screen else {
        return ItemModelResidencyFrame::default();
    };

    let mut residency = ItemModelResidencyFrame::default();
    for (preview_index, preview) in screen.entity_previews.iter().enumerate() {
        let Ok(preview_index) = u32::try_from(preview_index) else {
            break;
        };
        let target = ItemModelDrawTarget::EntityPreview(preview_index);
        let mut ordered_layers = preview.item_layers.iter().enumerate().collect::<Vec<_>>();
        ordered_layers
            .sort_by_key(|(ordinal, layer)| (layer.order, layer.submit_sequence, *ordinal));
        for (_, layer) in ordered_layers {
            if stack.item_id != Some(layer.item_id) || layer.count <= 0 {
                continue;
            }
            let (context, left_hand, transforms, block_fallback, generated_fallback) =
                match (layer.slot, layer.display_context) {
                    (
                        HudEntityPreviewItemSlot::LeftHand,
                        HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
                    ) => (
                        BlockModelDisplayContext::ThirdPersonLeftHand,
                        true,
                        humanoid_hand_attach_transforms(&preview.entity, true),
                        BLOCK_THIRD_PERSON_FALLBACK,
                        GENERATED_THIRD_PERSON_FALLBACK,
                    ),
                    (HudEntityPreviewItemSlot::Head, HudEntityPreviewItemDisplayContext::Head) => (
                        BlockModelDisplayContext::Head,
                        false,
                        custom_head_item_transforms(&preview.entity),
                        BLOCK_HEAD_FALLBACK,
                        GENERATED_HEAD_FALLBACK,
                    ),
                    _ => continue,
                };
            for transform in transforms {
                let mut unused_block = Vec::<ItemModelMesh>::new();
                let mut unused_block_translucent = Vec::<ItemModelMesh>::new();
                let mut unused_block_glint = Vec::<ItemModelMesh>::new();
                let mut unused_block_glint_translucent = Vec::<ItemModelMesh>::new();
                let mut unused_flat = Vec::<ItemModelMesh>::new();
                let mut unused_flat_translucent = Vec::<ItemModelMesh>::new();
                let mut unused_flat_glint = Vec::<ItemModelMesh>::new();
                let mut unused_flat_glint_translucent = Vec::<ItemModelMesh>::new();
                bake_item_stack_at_transform_with_light_and_overlay(
                    stack,
                    world,
                    transform,
                    context,
                    left_hand,
                    None,
                    None,
                    None,
                    None,
                    false,
                    0.0,
                    enchantment_keys,
                    damage_type_keys,
                    trim_material_keys,
                    attribute_keys,
                    None,
                    block_fallback,
                    generated_fallback,
                    item_runtime,
                    terrain_textures,
                    variant_textures,
                    &mut unused_block,
                    &mut unused_block_translucent,
                    &mut unused_block_glint,
                    &mut unused_block_glint_translucent,
                    &mut unused_flat,
                    &mut unused_flat_translucent,
                    &mut unused_flat_glint,
                    &mut unused_flat_glint_translucent,
                    &mut residency,
                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                    ITEM_MODEL_NO_OVERLAY,
                    target,
                );
            }
        }
    }
    residency
}
