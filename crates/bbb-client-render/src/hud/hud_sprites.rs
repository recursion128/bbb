use super::*;

impl Renderer {
    pub(crate) fn hud_item_atlas(&self) -> Option<&HudSpriteGpu> {
        self.pack.item_entity_atlas.as_ref().map(|atlas| &atlas.hud)
    }

    pub fn upload_hud_font_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        fonts: impl Into<HudFontGlyphBook>,
    ) -> Result<()> {
        let fonts = fonts.into();
        let glyphs = fonts.glyphs(None).clone();
        let (atlas, dynamic_layout_gen, dynamic_upload_rev, dynamic_dims) =
            if let Some(dynamic) = fonts.dynamic_atlas() {
                let snapshot = dynamic.snapshot();
                let atlas = crate::hud::gpu::create_dynamic_hud_font_atlas_gpu(
                    &self.device,
                    &self.queue,
                    &self.hud_bind_group_layout,
                    &snapshot,
                )?;
                (
                    atlas,
                    snapshot.layout_generation,
                    snapshot.upload_revision,
                    Some((snapshot.width, snapshot.height)),
                )
            } else {
                (self.upload_hud_sprite(width, height, rgba)?, 0, 0, None)
            };
        self.bind_world_text_font_atlas(&atlas.gpu.texture);
        if let Some(previous) = self.pack.hud_font_atlas.replace(atlas) {
            if let Ok(gpu) = std::sync::Arc::try_unwrap(previous.gpu) {
                crate::frame_buffers::retire_gpu_textures(&self.queue, [gpu.texture]);
            }
        }
        self.pack
            .map_surface_residency
            .set_text_atlas_dims(dynamic_dims);
        self.pack.hud_font_dynamic_layout_generation = dynamic_layout_gen;
        self.pack.hud_font_dynamic_upload_revision = dynamic_upload_rev;
        // Rebuild the advance-grouped obfuscated pool once here, not per frame.
        self.pack.hud_obfuscated_glyph_pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
        self.pack.hud_digit_glyphs =
            std::array::from_fn(|i| glyphs.resolve((b'0' + i as u8) as char));
        self.pack.hud_font_glyphs = glyphs;
        self.pack.hud_font_book = fonts;
        self.rebuild_chat_lines();
        self.mark_hud_dirty();
        // Digits 0..=9 may have lazily realized glyphs; sync them immediately.
        self.flush_dynamic_font_atlas();
        Ok(())
    }

    /// Uploads any lazily rasterized Unihex/TrueType glyphs into the active GPU
    /// font atlas. Returns `true` if the backing texture resized and bind groups
    /// were recreated.
    pub(crate) fn flush_dynamic_font_atlas(&mut self) -> bool {
        let Some(dynamic) = self.pack.hud_font_book.dynamic_atlas() else {
            return false;
        };
        let synced_layout = self.pack.hud_font_dynamic_layout_generation;
        let synced_upload = self.pack.hud_font_dynamic_upload_revision;
        if dynamic.layout_generation() == synced_layout
            && dynamic.upload_revision() == synced_upload
        {
            return false;
        }
        match dynamic.poll_updates(synced_layout, synced_upload) {
            bbb_render_types::HudDynamicAtlasUpdate::Unchanged => false,
            bbb_render_types::HudDynamicAtlasUpdate::Uploads {
                upload_revision,
                uploads,
            } => {
                if let Some(atlas) = self.pack.hud_font_atlas.as_ref() {
                    for glyph in uploads {
                        if glyph.width > 0 && glyph.height > 0 && !glyph.rgba.is_empty() {
                            crate::hud::gpu::write_hud_texture_subrect(
                                &self.queue,
                                &atlas.gpu.texture,
                                glyph.x,
                                glyph.y,
                                glyph.width,
                                glyph.height,
                                &glyph.rgba,
                            );
                        }
                    }
                    self.pack.hud_font_dynamic_upload_revision = upload_revision;
                }
                false
            }
            bbb_render_types::HudDynamicAtlasUpdate::Resized(snapshot) => {
                match crate::hud::gpu::create_dynamic_hud_font_atlas_gpu(
                    &self.device,
                    &self.queue,
                    &self.hud_bind_group_layout,
                    &snapshot,
                ) {
                    Ok(atlas) => {
                        self.bind_world_text_font_atlas(&atlas.gpu.texture);
                        if let Some(previous) = self.pack.hud_font_atlas.replace(atlas) {
                            if let Ok(gpu) = std::sync::Arc::try_unwrap(previous.gpu) {
                                // Defer destruction until after this frame's `queue.submit` so
                                // already-recorded world passes referencing the old texture stay valid.
                                self.frame_uploads.retire_texture(gpu.texture);
                            }
                        }
                        self.pack.hud_font_dynamic_layout_generation = snapshot.layout_generation;
                        self.pack.hud_font_dynamic_upload_revision = snapshot.upload_revision;
                        self.pack.hud_digit_glyphs = std::array::from_fn(|i| {
                            self.pack.hud_font_glyphs.resolve((b'0' + i as u8) as char)
                        });
                        self.pack.sign_text.dirty = true;
                        self.pack.display_text.dirty = true;
                        self.pack.entity_name_tags.dirty = true;
                        // Defer UV rescaling of `map_surface_residency.text_buffer` so a resize
                        // during HUD passes does not mutate the buffer via `queue.write_buffer`
                        // before already-recorded world/first-person passes execute on GPU.
                        self.pack
                            .map_surface_residency
                            .queue_text_atlas_rescale((snapshot.width, snapshot.height));
                        self.rebuild_chat_lines();
                        self.mark_hud_dirty();
                        true
                    }
                    Err(error) => {
                        tracing::warn!(%error, "failed resizing dynamic HUD font atlas");
                        // Record the attempted generation so a device limit or budget cap
                        // does not trigger a full CPU re-rasterization on every subsequent frame.
                        self.pack.hud_font_dynamic_layout_generation = snapshot.layout_generation;
                        self.pack.hud_font_dynamic_upload_revision = snapshot.upload_revision;
                        false
                    }
                }
            }
        }
    }

    pub(super) fn upload_hud_sprite(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<HudSpriteGpu> {
        create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
        )
    }
}
