use std::collections::BTreeMap;

use bbb_item_model::NativeItemRuntime;
use bbb_renderer::{
    bake_hud_map_decoration_surface, bake_hud_map_text_surface, bake_item_frame_map_surface,
    HudInventoryScreen, HudMapPreviewSurface, ItemFrameMapDecorationSurface,
    ItemFrameMapDecorationTexture, ItemFrameMapSurface, ItemFrameMapTextSurface,
    ItemFrameMapTexture, ITEM_MODEL_FULL_BRIGHT_LIGHT,
};
use bbb_world::WorldStore;
use glam::{Mat4, Vec3};

use crate::map_textures::MapTextureExtractionCache;

/// Resident render inputs for all dynamic maps embedded in the current GUI.
///
/// Preview-index wrappers preserve duplicate-map placement and strata while
/// the texture map deduplicates identical map ids before renderer upload.
#[derive(Debug, Default)]
pub(crate) struct HudMapPreviewModels {
    pub(crate) textures: Vec<ItemFrameMapTexture>,
    pub(crate) surfaces: Vec<HudMapPreviewSurface<ItemFrameMapSurface>>,
    pub(crate) decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) decoration_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapDecorationSurface>>,
    pub(crate) text_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapTextSurface>>,
}

pub(crate) fn hud_map_preview_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    screen: Option<&HudInventoryScreen>,
    surface_width: u32,
    surface_height: u32,
    map_texture_cache: &mut MapTextureExtractionCache,
) -> HudMapPreviewModels {
    let Some(screen) = screen else {
        return HudMapPreviewModels::default();
    };
    let origin_x = (surface_width.max(1) as f32 - screen.width as f32) * 0.5;
    let origin_y = (surface_height.max(1) as f32 - screen.height as f32) * 0.5;
    let mut textures = BTreeMap::new();
    let mut models = HudMapPreviewModels::default();

    for (preview_index, preview) in screen.map_previews.iter().enumerate() {
        let Ok(preview_index) = u8::try_from(preview_index) else {
            break;
        };
        let Some(map) = world.map_item(preview.map_id) else {
            continue;
        };
        textures
            .entry(map.id)
            .or_insert_with(|| map_texture_cache.texture(map));
        let map_transform = Mat4::from_translation(Vec3::new(
            origin_x + preview.x as f32,
            origin_y + preview.y as f32,
            0.0,
        )) * Mat4::from_scale(Vec3::new(preview.scale, preview.scale, 1.0));
        models.surfaces.push(HudMapPreviewSurface {
            preview_index,
            surface: bake_item_frame_map_surface(
                map.id,
                map_transform,
                ITEM_MODEL_FULL_BRIGHT_LIGHT,
            ),
        });

        let mut visible_decoration_index = 0;
        let mut text_submit_sequence = 0;
        for decoration in &map.decorations {
            let Some(surface) = bake_hud_map_decoration_surface(
                decoration.type_id,
                decoration.x,
                decoration.y,
                decoration.rot,
                visible_decoration_index,
                map_transform,
                ITEM_MODEL_FULL_BRIGHT_LIGHT,
                visible_decoration_index + 1,
            ) else {
                continue;
            };
            models.decoration_surfaces.push(HudMapPreviewSurface {
                preview_index,
                surface,
            });
            if let (Some(name), Some(glyphs)) = (
                decoration.name.as_ref(),
                item_runtime.and_then(NativeItemRuntime::map_text_glyphs),
            ) {
                if let Some(surface) = bake_hud_map_text_surface(
                    decoration.type_id,
                    name.text(),
                    decoration.x,
                    decoration.y,
                    visible_decoration_index,
                    map_transform,
                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                    text_submit_sequence,
                    glyphs,
                ) {
                    models.text_surfaces.push(HudMapPreviewSurface {
                        preview_index,
                        surface,
                    });
                    text_submit_sequence += 1;
                }
            }
            visible_decoration_index += 1;
        }
    }

    models.textures = textures.into_values().collect();
    if !models.decoration_surfaces.is_empty() {
        models.decoration_textures = item_runtime
            .map(NativeItemRuntime::map_decoration_textures)
            .unwrap_or_default()
            .to_vec();
    }
    models
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        packets::{MapColorPatch, MapDecoration, MapItemData},
        RichTextComponent,
    };
    use bbb_renderer::{HudInventoryMapPreview, HudInventoryScreen};

    fn screen_with_previews(map_previews: Vec<HudInventoryMapPreview>) -> HudInventoryScreen {
        HudInventoryScreen {
            width: 176,
            height: 166,
            background_layers: Vec::new(),
            foreground_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: Vec::new(),
            foreground_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            map_previews,
            banner_pattern_previews: Vec::new(),
            text_labels: Vec::new(),
            scroll_area: None,
            preedit_overlay: None,
            hovered_slot_id: None,
            tooltip: None,
        }
    }

    #[test]
    fn duplicate_gui_previews_share_texture_and_keep_complete_targeted_stacks() {
        let mut world = WorldStore::new();
        assert!(world.apply_map_item_data(MapItemData {
            map_id: 9,
            scale: 0,
            locked: false,
            decorations: Some(vec![
                MapDecoration {
                    type_id: 0,
                    x: -8,
                    y: 6,
                    rot: 3,
                    name: Some(RichTextComponent::plain("Player")),
                },
                MapDecoration {
                    type_id: 1,
                    x: 20,
                    y: -12,
                    rot: 7,
                    name: None,
                },
            ]),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![6],
            }),
        }));
        let screen = screen_with_previews(vec![
            HudInventoryMapPreview {
                map_id: 9,
                x: 86,
                y: 16,
                scale: 0.34,
                after_background_layer: 0,
            },
            HudInventoryMapPreview {
                map_id: 9,
                x: 70,
                y: 32,
                scale: 0.34,
                after_background_layer: 1,
            },
        ]);
        let runtime = NativeItemRuntime::empty_for_test();
        let mut cache = MapTextureExtractionCache::default();
        cache.begin_frame();

        let models =
            hud_map_preview_models(&world, Some(&runtime), Some(&screen), 320, 240, &mut cache);

        assert_eq!(models.textures.len(), 1);
        assert_eq!(models.surfaces.len(), 2);
        assert_eq!(
            models
                .surfaces
                .iter()
                .map(|surface| surface.preview_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            models.surfaces[0].surface.submission.transform,
            Mat4::from_translation(Vec3::new(158.0, 53.0, 0.0))
                * Mat4::from_scale(Vec3::new(0.34, 0.34, 1.0))
        );
        assert_eq!(
            models.surfaces[1].surface.submission.transform,
            Mat4::from_translation(Vec3::new(142.0, 69.0, 0.0))
                * Mat4::from_scale(Vec3::new(0.34, 0.34, 1.0))
        );
        assert_eq!(models.decoration_surfaces.len(), 4);
        assert_eq!(
            models
                .decoration_surfaces
                .iter()
                .map(|surface| surface.preview_index)
                .collect::<Vec<_>>(),
            vec![0, 0, 1, 1]
        );
        assert_eq!(models.text_surfaces.len(), 2);
        assert_eq!(models.text_surfaces[0].preview_index, 0);
        assert_eq!(models.text_surfaces[1].preview_index, 1);
        assert_eq!(models.text_surfaces[0].surface.submission.text, "Player");
    }

    #[test]
    fn missing_map_state_emits_no_resident_preview_inputs() {
        let screen = screen_with_previews(vec![HudInventoryMapPreview {
            map_id: 12,
            x: 71,
            y: 17,
            scale: 0.45,
            after_background_layer: 0,
        }]);
        let mut cache = MapTextureExtractionCache::default();
        cache.begin_frame();

        let models = hud_map_preview_models(
            &WorldStore::new(),
            None,
            Some(&screen),
            320,
            240,
            &mut cache,
        );

        assert!(models.textures.is_empty());
        assert!(models.surfaces.is_empty());
        assert!(models.decoration_surfaces.is_empty());
        assert!(models.text_surfaces.is_empty());
    }
}
