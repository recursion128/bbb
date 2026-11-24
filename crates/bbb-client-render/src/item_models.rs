pub use crate::entity_models::{
    bake_entity_name_tag_surfaces, entity_name_tag_root_position, EntityNameTagLayerSubmission,
    EntityNameTagLineKind, EntityNameTagRenderType, EntityNameTagSubmission, EntityNameTagSurface,
    ENTITY_NAME_TAG_BACKGROUND_OPACITY, ENTITY_NAME_TAG_FONT_TEXTURE, ENTITY_NAME_TAG_SCALE,
};
pub use crate::item_render::item_models::*;

mod display_text_renderer;
mod map;
mod residency_renderer;
mod sign_text_renderer;
pub(crate) mod special_geometry;
pub use map::{
    bake_first_person_map_background_surface, bake_first_person_map_decoration_surface,
    bake_first_person_map_text_surface, bake_hud_map_decoration_surface, bake_hud_map_text_surface,
    bake_item_frame_map_decoration_surface, bake_item_frame_map_surface,
    bake_item_frame_map_text_surface, item_frame_map_decoration_type, item_frame_map_text_width,
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundSubmission,
    FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture,
    FirstPersonMapBackgroundTextureRef, HudMapPreviewSurface, ItemFrameMapDecorationSubmission,
    ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture, ItemFrameMapDecorationTextureRef,
    ItemFrameMapDecorationType, ItemFrameMapRenderType, ItemFrameMapSubmission,
    ItemFrameMapSurface, ItemFrameMapTextSubmission, ItemFrameMapTextSurface,
    ItemFrameMapTextTextureRef, ItemFrameMapTexture, ItemFrameMapTexturePatch,
    ItemFrameMapTextureRef,
};
pub(crate) use map::{
    FirstPersonMapBackgroundAtlasGpu, ItemFrameMapAtlasGpu, ItemFrameMapDecorationAtlasGpu,
    ItemFrameMapTextFontAtlasGpu, MapSurfaceResidency, MapSurfaceTarget,
};

use crate::{Renderer, StandaloneTextureSampling};
use anyhow::Result;

impl Renderer {
    /// Uploads vanilla `textures/misc/enchanted_glint_item.png` for item foil draws. The same
    /// texture is bound with the world, first-person HUD-3D, and GUI-item cameras because
    /// `RenderTypes.glint` is used in scene item features, held items, and inventory icons.
    pub fn upload_item_glint_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.upload_item_glint_texture_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::default(),
        )
    }

    /// Uploads the item/entity glint owner with selected-pack standalone
    /// sampling metadata.
    pub fn upload_item_glint_texture_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.pack.item_glint_texture = Some(create_glint_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
            width,
            height,
            rgba,
            sampling,
            "item",
        )?);
        Ok(())
    }

    /// Uploads vanilla `textures/misc/enchanted_glint_armor.png` for
    /// `armorEntityGlint` draws.
    pub fn upload_armor_glint_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.upload_armor_glint_texture_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::default(),
        )
    }

    /// Uploads the armor glint owner with selected-pack standalone sampling
    /// metadata.
    pub fn upload_armor_glint_texture_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.pack.armor_glint_texture = Some(create_glint_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
            width,
            height,
            rgba,
            sampling,
            "armor",
        )?);
        Ok(())
    }

    /// Sets this frame's 3D block items for the hotbar slots (`None` for an empty slot or a flat item,
    /// which keeps its 2D sprite). Each is the block's model quads plus its `gui` display transform; the
    /// renderer seats them in their slot pixel rects and draws them in the GUI item pass (vanilla 3D
    /// inventory icons). Index `i` is hotbar slot `i`.
    pub fn set_hud_hotbar_block_item_models(&mut self, models: Vec<Option<HudBlockItemModel>>) {
        self.hud_hotbar_block_item_models = models;
    }
}
