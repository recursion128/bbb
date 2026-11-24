//! Resident item-model extraction for Smithing's armor-stand picture-in-picture preview.

use bbb_client_assets::BlockModelDisplayContext;
use bbb_client_render::{
    custom_head_item_transforms, humanoid_hand_attach_transforms,
    HudEntityPreviewItemDisplayContext, HudEntityPreviewItemSlot, HudInventoryScreen,
    ItemModelDrawTarget, ItemModelResidencyFrame, ITEM_MODEL_FULL_BRIGHT_LIGHT,
};
use bbb_client_world::WorldStore;
use bbb_protocol::menu_types::VANILLA_MENU_TYPE_SMITHING_ID;

use super::stack::{bake_item_stack, ItemBakeEnv, ItemOwner, ItemPlacement};
use super::{
    ItemRegistryKeySet, NativeItemRuntime, TerrainTextureState, BLOCK_HEAD_FALLBACK,
    BLOCK_THIRD_PERSON_FALLBACK, GENERATED_HEAD_FALLBACK, GENERATED_THIRD_PERSON_FALLBACK,
};
use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;

const SMITHING_RESULT_SLOT: i16 = 3;

pub(crate) fn smithing_entity_preview_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    _terrain_textures: &TerrainTextureState,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    screen: Option<&HudInventoryScreen>,
    registry_keys: &ItemRegistryKeySet,
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
    let Some(stack) = world
        .inventory()
        .container_slot(container, SMITHING_RESULT_SLOT)
        .map(|slot| slot.item)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
    else {
        return ItemModelResidencyFrame::default();
    };
    let Some(screen) = screen else {
        return ItemModelResidencyFrame::default();
    };

    let env = ItemBakeEnv {
        world,
        item_runtime,
        variant_textures,
        registry_keys: registry_keys.model_keys(),
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
                bake_item_stack(
                    &env,
                    stack,
                    context,
                    &ItemOwner::default(),
                    &ItemPlacement {
                        attach: transform,
                        left_hand,
                        block_fallback,
                        generated_fallback,
                        light: ITEM_MODEL_FULL_BRIGHT_LIGHT,
                        target,
                        outline_color: None,
                    },
                    &mut residency,
                );
            }
        }
    }
    residency
}
