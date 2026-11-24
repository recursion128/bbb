//! Vanilla 26.1 `LayerDefinitions.createRoots()` model layers, baked from the generated
//! [`super::model_data_26_1`] tables.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use super::geometry::PartPose;
use super::model::ModelPart;
use super::model_data_26_1::{CHILDREN, CUBES, LAYERS, PARTS};

/// A Vanilla `CubeDefinition`: `origin`/`dimensions` before deformation, the `CubeDeformation`
/// grow, `texOffs`, the per-cube `texScale` and the visible face bits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity_models) struct ModelCube {
    pub(in crate::entity_models) origin: [f32; 3],
    pub(in crate::entity_models) dimensions: [f32; 3],
    pub(in crate::entity_models) grow: [f32; 3],
    pub(in crate::entity_models) tex: [f32; 2],
    pub(in crate::entity_models) tex_scale: [f32; 2],
    pub(in crate::entity_models) mirror: bool,
    pub(in crate::entity_models) visible_faces: u8,
}

pub(in crate::entity_models) const G0: [f32; 3] = [0.0; 3];

pub(in crate::entity_models) const fn cube(
    origin: [f32; 3],
    dimensions: [f32; 3],
    grow: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
    visible_faces: u8,
) -> ModelCube {
    cube_scaled(
        origin,
        dimensions,
        grow,
        tex,
        mirror,
        visible_faces,
        [1.0, 1.0],
    )
}

pub(in crate::entity_models) const fn cube_scaled(
    origin: [f32; 3],
    dimensions: [f32; 3],
    grow: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
    visible_faces: u8,
    tex_scale: [f32; 2],
) -> ModelCube {
    ModelCube {
        origin,
        dimensions,
        grow,
        tex,
        tex_scale,
        mirror,
        visible_faces,
    }
}

impl ModelCube {
    /// Vanilla `ModelPart.Cube` `minX/minY/minZ`: `origin - grow`.
    pub(in crate::entity_models) fn min(&self) -> [f32; 3] {
        std::array::from_fn(|axis| self.origin[axis] - self.grow[axis])
    }

    /// Vanilla `ModelPart.Cube` `maxX/maxY/maxZ`: `origin + dimensions + grow`.
    pub(in crate::entity_models) fn max(&self) -> [f32; 3] {
        std::array::from_fn(|axis| self.origin[axis] + self.dimensions[axis] + self.grow[axis])
    }
}

/// One `LayerDefinitions.createRoots()` entry.
pub(in crate::entity_models) struct LayerDef {
    pub(in crate::entity_models) texture_size: [u32; 2],
    pub(in crate::entity_models) root: u16,
}

/// One baked `PartDefinition`; cubes and children index the generated tables.
pub(in crate::entity_models) struct PartDef {
    name: &'static str,
    offset: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    cube_start: u16,
    cube_len: u16,
    child_start: u16,
    child_len: u16,
}

impl PartDef {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::entity_models) const fn new(
        name: &'static str,
        offset: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
        cube_start: u16,
        cube_len: u16,
        child_start: u16,
        child_len: u16,
    ) -> Self {
        Self {
            name,
            offset,
            rotation,
            scale,
            cube_start,
            cube_len,
            child_start,
            child_len,
        }
    }

    fn cubes(&self) -> &'static [ModelCube] {
        &CUBES[self.cube_start as usize..][..self.cube_len as usize]
    }

    fn children(&self) -> impl Iterator<Item = &'static PartDef> {
        CHILDREN[self.child_start as usize..][..self.child_len as usize]
            .iter()
            .map(|&index| &PARTS[index as usize])
    }

    fn bake(&self, texture_size: [f32; 2]) -> ModelPart {
        ModelPart::new(
            PartPose {
                offset: self.offset,
                rotation: self.rotation,
            },
            self.cubes(),
            self.children()
                .map(|child| (child.name, child.bake(texture_size)))
                .collect(),
            texture_size,
        )
        .with_scale(self.scale)
    }
}

/// A Vanilla `ModelLayerLocation` from `LayerDefinitions.createRoots()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::entity_models) struct ModelLayer(pub(in crate::entity_models) u16);

impl ModelLayer {
    fn definition(self) -> &'static LayerDef {
        &LAYERS[self.0 as usize]
    }

    fn bake_tree(self) -> ModelPart {
        let [width, height] = self.definition().texture_size;
        PARTS[self.definition().root as usize].bake([width as f32, height as f32])
    }

    /// Vanilla `LayerDefinition.bakeRoot()`, reusing a tree baked by an earlier frame.
    pub(in crate::entity_models) fn bake(self) -> BakedRoot {
        BakedRoot::take(u32::from(self.0), || self.bake_tree())
    }

    /// Bakes `self` with `other`'s root children appended, for renderers that submit several
    /// Vanilla layers as one tree.
    pub(in crate::entity_models) fn bake_with_children_of(self, other: ModelLayer) -> BakedRoot {
        let key = 1 << 31 | u32::from(self.0) << 16 | u32::from(other.0);
        BakedRoot::take(key, || self.bake_tree().with_children_of(other.bake_tree()))
    }
}

#[derive(Default)]
struct BakedTreePool {
    layers: Vec<Vec<ModelPart>>,
    combined: HashMap<u32, Vec<ModelPart>>,
}

thread_local! {
    static BAKED_TREES: RefCell<BakedTreePool> = RefCell::new(BakedTreePool::default());
}

/// A baked layer tree on loan from a per-thread pool: models are rebuilt every frame, so dropping
/// one returns its tree and the next frame reuses it instead of baking again.
pub(in crate::entity_models) struct BakedRoot {
    key: u32,
    root: ModelPart,
}

impl BakedRoot {
    fn take(key: u32, bake: impl FnOnce() -> ModelPart) -> Self {
        let pooled = BAKED_TREES.with(|trees| {
            let mut pool = trees.borrow_mut();
            if key & (1 << 31) == 0 {
                pool.layers.get_mut(key as usize).and_then(Vec::pop)
            } else {
                pool.combined.get_mut(&key).and_then(Vec::pop)
            }
        });
        let root = match pooled {
            Some(mut root) => {
                root.reset_pose();
                root
            }
            None => bake(),
        };
        Self { key, root }
    }
}

impl Drop for BakedRoot {
    fn drop(&mut self) {
        let root = std::mem::replace(&mut self.root, ModelPart::EMPTY);
        // A thread being torn down has no pool to return to.
        let _ = BAKED_TREES.try_with(|trees| {
            let mut pool = trees.borrow_mut();
            if self.key & (1 << 31) == 0 {
                let index = self.key as usize;
                if pool.layers.len() <= index {
                    pool.layers.resize_with(index + 1, Vec::new);
                }
                pool.layers[index].push(root);
            } else {
                pool.combined.entry(self.key).or_default().push(root);
            }
        });
    }
}

impl Deref for BakedRoot {
    type Target = ModelPart;

    fn deref(&self) -> &ModelPart {
        &self.root
    }
}

impl DerefMut for BakedRoot {
    fn deref_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }
}
