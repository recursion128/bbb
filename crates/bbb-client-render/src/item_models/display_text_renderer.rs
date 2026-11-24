use std::collections::HashMap;

use crate::item_render::item_models::display_text::*;
use crate::Renderer;

impl Renderer {
    pub fn set_display_text_surfaces(&mut self, surfaces: Vec<DisplayTextSurface>) {
        let mut surfaces: Vec<_> = if self.pack.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
        if self.display_text_surfaces.len() != surfaces.len()
            || !self
                .display_text_surfaces
                .iter()
                .zip(&surfaces)
                .all(|(old, new)| old.same_draw(new))
        {
            let mut previous = std::mem::take(&mut self.display_text_surfaces)
                .into_iter()
                .map(|surface| (surface.submission.entity_id, surface))
                .collect::<HashMap<_, _>>();
            for surface in &mut surfaces {
                if let Some(previous) = previous.get_mut(&surface.submission.entity_id) {
                    surface.retain_world_meshes(previous);
                }
            }
            self.display_text_surfaces = surfaces;
            self.pack.display_text.dirty = true;
        }
    }

    pub(crate) fn prepare_display_text(&mut self) {
        if let Some(current_dims) = self
            .pack
            .hud_font_book
            .dynamic_atlas()
            .map(|atlas| atlas.atlas_dimensions())
        {
            let mut rescaled = false;
            for surface in &mut self.display_text_surfaces {
                rescaled |= surface.rescale_for_atlas_dims(current_dims);
            }
            if rescaled {
                self.pack.display_text.dirty = true;
            }
        }
        let camera = self.camera_sort_position();
        let state = &mut self.pack.display_text;
        if state.dirty {
            let geometry = merge_display_text_surfaces(&self.display_text_surfaces, camera);
            for ((buffers, (vertices, indices)), (previous, _)) in state
                .buffers
                .iter_mut()
                .zip(geometry.meshes())
                .zip(state.geometry.meshes())
            {
                buffers.upload_changed_vertices(
                    &self.device,
                    &self.queue,
                    vertices,
                    previous,
                    indices,
                );
            }
            state.geometry = geometry;
            state.dirty = false;
        } else if state.camera != camera {
            sort_display_text_geometry(&mut state.geometry, camera);
            // See-through glyphs preserve submission order, so only the other lanes change.
            for index in [0, 2, 3, 4] {
                state.buffers[index].upload_indices(
                    &self.device,
                    &self.queue,
                    &state.geometry.meshes()[index].1,
                );
            }
        }
        state.camera = camera;
    }
}
