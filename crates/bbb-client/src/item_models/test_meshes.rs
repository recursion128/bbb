//! Test-only projection of a residency frame into world-space meshes, so pose
//! tests can compare final geometry per atlas and phase.

use std::ops::Deref;

use bbb_client_render::{
    ItemModelAtlasKind, ItemModelDrawPhase, ItemModelGeometryClass, ItemModelMesh,
    ItemModelResidencyFrame,
};

pub(crate) struct ResidentMeshView<T> {
    models: T,
    pub(crate) block_meshes: Vec<ItemModelMesh>,
    pub(crate) block_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) block_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) block_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_glint_translucent_meshes: Vec<ItemModelMesh>,
}

impl<T> Deref for ResidentMeshView<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.models
    }
}

impl<T> ResidentMeshView<T> {
    pub(crate) fn new(models: T, residency: impl Fn(&T) -> &ItemModelResidencyFrame) -> Self {
        let mut view = Self {
            block_meshes: Vec::new(),
            block_translucent_meshes: Vec::new(),
            block_glint_meshes: Vec::new(),
            block_glint_translucent_meshes: Vec::new(),
            flat_meshes: Vec::new(),
            flat_translucent_meshes: Vec::new(),
            flat_glint_meshes: Vec::new(),
            flat_glint_translucent_meshes: Vec::new(),
            models,
        };
        let frame = residency(&view.models).clone();
        view.project(&frame);
        view
    }

    // Entity atlases and renderer-owned special phases have no mesh bucket
    // here; renderer semantic/offscreen tests cover them.
    fn project(&mut self, frame: &ItemModelResidencyFrame) {
        for instance in &frame.instances {
            let Some(asset) = frame.assets.iter().find(|asset| {
                asset.identity == instance.identity
                    && asset.atlas == instance.atlas
                    && asset.geometry_class == instance.geometry_class
            }) else {
                continue;
            };
            let target = match (instance.atlas, instance.phase) {
                (
                    ItemModelAtlasKind::Blocks,
                    ItemModelDrawPhase::Solid | ItemModelDrawPhase::SolidNoCutout,
                ) => &mut self.block_meshes,
                (ItemModelAtlasKind::Blocks, ItemModelDrawPhase::Translucent) => {
                    &mut self.block_translucent_meshes
                }
                (ItemModelAtlasKind::Blocks, ItemModelDrawPhase::Glint) => {
                    &mut self.block_glint_meshes
                }
                (ItemModelAtlasKind::Blocks, ItemModelDrawPhase::GlintTranslucent) => {
                    &mut self.block_glint_translucent_meshes
                }
                (
                    ItemModelAtlasKind::Items,
                    ItemModelDrawPhase::Solid | ItemModelDrawPhase::SolidNoCutout,
                ) => &mut self.flat_meshes,
                (ItemModelAtlasKind::Items, ItemModelDrawPhase::Translucent) => {
                    &mut self.flat_translucent_meshes
                }
                (ItemModelAtlasKind::Items, ItemModelDrawPhase::Glint) => {
                    &mut self.flat_glint_meshes
                }
                (ItemModelAtlasKind::Items, ItemModelDrawPhase::GlintTranslucent) => {
                    &mut self.flat_glint_translucent_meshes
                }
                _ => continue,
            };
            let mut quads = asset.source.clone().build();
            quads.retain(|quad| {
                quad.translucent
                    == matches!(instance.geometry_class, ItemModelGeometryClass::Translucent)
            });
            for quad in &mut quads {
                for (color, tint) in quad.tint.iter_mut().zip(instance.tint) {
                    *color *= tint;
                }
            }
            let mut mesh = ItemModelMesh::new();
            mesh.append_quads_with_light_and_overlay(
                &quads,
                instance.transform,
                instance.light,
                instance.overlay,
            );
            if !mesh.is_empty() {
                target.push(mesh);
            }
        }
    }
}
