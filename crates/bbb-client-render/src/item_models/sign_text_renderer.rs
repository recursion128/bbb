use crate::item_render::item_models::sign_text::*;
use crate::Renderer;

impl Renderer {
    pub fn set_sign_text_surfaces(&mut self, surfaces: Vec<SignTextSurface>) {
        self.sign_text_surfaces = if self.pack.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
        self.pack.sign_text.dirty = true;
    }

    /// Replaces the per-frame visible sign-block positions while retaining the
    /// stable baked text surfaces across camera-only changes.
    pub fn set_visible_sign_text_positions(&mut self, mut positions: Vec<[i32; 3]>) {
        positions.sort_unstable();
        positions.dedup();
        if self.visible_sign_text_positions != positions {
            self.visible_sign_text_positions = positions;
            self.pack.sign_text.dirty = true;
        }
    }

    pub fn set_sign_text_outline_context(&mut self, mut context: SignTextOutlineContext) {
        if context
            .camera_entity_position
            .is_some_and(|position| position.iter().any(|coordinate| !coordinate.is_finite()))
        {
            context.camera_entity_position = None;
        }
        if self.sign_text_outline_context != context
            && self.sign_text_surfaces.iter().any(|surface| {
                surface.submission.has_glowing_text
                    && self
                        .visible_sign_text_positions
                        .binary_search(&sign_text_block_position(surface))
                        .is_ok()
                    && sign_text_draws_outline(surface, self.sign_text_outline_context)
                        != sign_text_draws_outline(surface, context)
            })
        {
            self.pack.sign_text.dirty = true;
        }
        self.sign_text_outline_context = context;
    }

    pub(crate) fn prepare_sign_text(&mut self) {
        if let Some(current_dims) = self
            .pack
            .hud_font_book
            .dynamic_atlas()
            .map(|atlas| atlas.atlas_dimensions())
        {
            let mut rebaked = false;
            for surface in &mut self.sign_text_surfaces {
                if surface
                    .dynamic_atlas_dims()
                    .is_some_and(|dims| dims != current_dims)
                {
                    surface.rebake(&self.pack.hud_font_book);
                    rebaked = true;
                }
            }
            if rebaked {
                self.pack.sign_text.dirty = true;
            }
        }
        let state = &mut self.pack.sign_text;
        if !state.dirty && !state.obfuscated {
            return;
        }
        if state.dirty {
            state.obfuscated = self.sign_text_surfaces.iter().any(|surface| {
                surface.is_obfuscated()
                    && self
                        .visible_sign_text_positions
                        .binary_search(&sign_text_block_position(surface))
                        .is_ok()
            });
        }
        let geometry = merge_sign_text_surfaces(
            &self.sign_text_surfaces,
            &self.visible_sign_text_positions,
            &self.pack.hud_font_book,
            crate::text_obfuscation_seed(),
            self.sign_text_outline_context,
        );
        for (buffers, mesh) in
            state
                .buffers
                .iter_mut()
                .zip([&geometry.outline, &geometry.primary, &geometry.effects])
        {
            buffers.upload(&self.device, &self.queue, &mesh.vertices, &mesh.indices);
        }
        state.dirty = false;
    }
}
