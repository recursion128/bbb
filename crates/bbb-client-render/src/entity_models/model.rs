use std::sync::Arc;

use glam::Mat4;

use super::geometry::{
    emit_textured_model_cube, emit_untextured_model_cube, part_pose_transform, EntityModelMesh,
    EntityModelTexturedMesh, PartPose,
};
use super::model_layer::ModelCube;
use super::EntityModelUvRect;

#[derive(Debug, Clone)]
pub(in crate::entity_models) enum ModelCubes {
    Static(&'static [ModelCube]),
    Owned { id: u64, cubes: Arc<[ModelCube]> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::entity_models) enum ModelCubesKey {
    Static(usize, usize),
    Owned(u64),
}

impl ModelCubes {
    pub(in crate::entity_models) fn as_slice(&self) -> &[ModelCube] {
        match self {
            Self::Static(cubes) => cubes,
            Self::Owned { cubes, .. } => cubes,
        }
    }

    pub(in crate::entity_models) fn key(&self) -> ModelCubesKey {
        match self {
            Self::Static(cubes) => ModelCubesKey::Static(cubes.as_ptr() as usize, cubes.len()),
            Self::Owned { id, .. } => ModelCubesKey::Owned(*id),
        }
    }
}

impl PartialEq for ModelCubes {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

#[derive(Debug, Clone)]
pub(in crate::entity_models) enum PartName {
    Static(&'static str),
    Shared(Arc<str>),
}

impl PartName {
    pub(in crate::entity_models) fn as_str(&self) -> &str {
        match self {
            Self::Static(name) => name,
            Self::Shared(name) => name,
        }
    }
}

/// Vanilla `net.minecraft.client.model.geom.ModelPart`, baked from a generated layer: each frame
/// `resetPose`, then `setupAnim` mutates named parts, then the tree renders with every part's pose
/// as a parent-relative `PoseStack` transform.
pub(in crate::entity_models) struct ModelPart {
    /// Current pose (vanilla `x/y/z` + `xRot/yRot/zRot`): reset to `default_pose` each frame and
    /// mutated by `setup_anim`.
    pub(in crate::entity_models) pose: PartPose,
    /// Bind pose captured when the tree is built (vanilla `PartPose`/`loadPose`); restored by
    /// [`ModelPart::reset_pose`].
    default_pose: PartPose,
    /// This part's generated or data-driven `ModelPart.Cube`s.
    cubes: ModelCubes,
    /// The owning layer's `MaterialDefinition` texture size, the UV divisor.
    texture_size: [f32; 2],
    /// Named children in vanilla declaration / render order (vanilla `Map<String, ModelPart>`,
    /// whose iteration order is the insertion order). Names drive [`ModelPart::child_mut`].
    children: Vec<(PartName, ModelPart)>,
    /// Vanilla `ModelPart.visible`: a hidden part and its whole subtree are skipped at render.
    pub(in crate::entity_models) visible: bool,
    /// Vanilla `ModelPart.skipDraw`: skips this part's own cubes but still renders its children.
    pub(in crate::entity_models) skip_draw: bool,
    /// Vanilla `ModelPart.xScale/yScale/zScale` (each `1.0` at the bind pose): `translateAndRotate`
    /// applies them last as `poseStack.scale(...)`, scaling this part's cubes and its subtree about
    /// the part origin. Reset to `[1, 1, 1]` each frame and mutated by `setup_anim` (the keyframe
    /// `AnimationChannel.Targets.SCALE` channel folds `scaleVec` offsets onto it). Only the croaking
    /// frog and nautilus use it so far, so most parts hold `[1, 1, 1]` (an identity scale the render
    /// skips).
    pub(in crate::entity_models) scale: [f32; 3],
    /// Bind-pose scale (vanilla `PartPose.xScale/yScale/zScale`), restored by `reset_pose`.
    default_scale: [f32; 3],
}

/// A selected cube in a posed direct child, used by player stuck-in-body layers. The transform
/// reaches the selected part frame from this part's parent frame; `min`/`max` retain vanilla's
/// baked `ModelPart.Cube` bounds in model pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity_models) struct ModelCubeAttachment {
    pub(in crate::entity_models) transform: Mat4,
    pub(in crate::entity_models) min: [f32; 3],
    pub(in crate::entity_models) max: [f32; 3],
}

impl ModelPart {
    pub(in crate::entity_models) const EMPTY: Self = Self {
        pose: PartPose {
            offset: [0.0; 3],
            rotation: [0.0; 3],
        },
        default_pose: PartPose {
            offset: [0.0; 3],
            rotation: [0.0; 3],
        },
        cubes: ModelCubes::Static(&[]),
        texture_size: [1.0; 2],
        children: Vec::new(),
        visible: true,
        skip_draw: false,
        scale: [1.0; 3],
        default_scale: [1.0; 3],
    };

    /// Builds a part at `pose` carrying `cubes` and the named `children`, capturing `pose` as the
    /// bind pose that [`ModelPart::reset_pose`] restores.
    pub(in crate::entity_models) fn new(
        pose: PartPose,
        cubes: &'static [ModelCube],
        children: Vec<(&'static str, ModelPart)>,
        texture_size: [f32; 2],
    ) -> Self {
        Self::new_owned(
            pose,
            ModelCubes::Static(cubes),
            children
                .into_iter()
                .map(|(name, child)| (PartName::Static(name), child))
                .collect(),
            texture_size,
        )
    }

    pub(in crate::entity_models) fn new_owned(
        pose: PartPose,
        cubes: ModelCubes,
        children: Vec<(PartName, ModelPart)>,
        texture_size: [f32; 2],
    ) -> Self {
        Self {
            pose,
            default_pose: pose,
            cubes,
            texture_size,
            children,
            visible: true,
            skip_draw: false,
            scale: [1.0; 3],
            default_scale: [1.0; 3],
        }
    }

    /// Sets the bind-pose scale.
    pub(in crate::entity_models) fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self.default_scale = scale;
        self
    }

    /// Appends `other`'s root children, for renderers that submit several Vanilla layers as one
    /// tree.
    pub(in crate::entity_models) fn with_children_of(mut self, other: ModelPart) -> Self {
        self.children.extend(other.children);
        self
    }

    fn drawn_cubes(&self) -> &[ModelCube] {
        if self.skip_draw {
            &[]
        } else {
            self.cubes.as_slice()
        }
    }

    fn emit_cubes(
        &self,
        mesh: &mut EntityModelTexturedMesh,
        transform: Mat4,
        uv_rect: EntityModelUvRect,
        tint: [f32; 4],
    ) {
        let cubes = self.drawn_cubes();
        if cubes.is_empty() {
            return;
        }
        if mesh.retain_parts {
            super::retained::retain_model_part(
                mesh,
                self.cubes.clone(),
                self.texture_size,
                transform,
                uv_rect,
                tint,
            );
            return;
        }
        let world_transform = match mesh.part_origin {
            Some(origin) => Mat4::from_translation(glam::Vec3::from_array(origin)) * transform,
            None => transform,
        };
        let normal_matrix = transform.inverse().transpose();
        for cube in cubes {
            emit_textured_model_cube(
                mesh,
                world_transform,
                normal_matrix,
                cube,
                self.texture_size,
                uv_rect,
                tint,
            );
        }
    }

    /// Vanilla `ModelPart.resetPose` over the whole subtree: restores the bind pose and visibility
    /// so each frame's `setup_anim` starts from a clean slate.
    pub(in crate::entity_models) fn reset_pose(&mut self) {
        self.pose = self.default_pose;
        self.visible = true;
        self.skip_draw = false;
        self.scale = self.default_scale;
        for (_, child) in &mut self.children {
            child.reset_pose();
        }
    }

    /// This part's local transform: the bind/animated [`PartPose`] (translate + rotate) followed by
    /// the part scale (vanilla `translateAndRotate`'s final `poseStack.scale`), which scales this
    /// part's cubes and its subtree about the part origin. The identity scale `[1, 1, 1]` is the
    /// no-op common case.
    fn local_transform(&self) -> Mat4 {
        let transform = part_pose_transform(self.pose);
        if self.scale == [1.0; 3] {
            transform
        } else {
            transform * Mat4::from_scale(glam::Vec3::from_array(self.scale))
        }
    }

    /// The bind pose restored by [`Self::reset_pose`].
    #[cfg(test)]
    pub(in crate::entity_models) fn default_pose(&self) -> PartPose {
        self.default_pose
    }

    /// Vanilla `ModelPart.createPartLookup`: `root` is this part, any other name the first match
    /// in `addAllChildren` order (direct children before their subtrees).
    pub(in crate::entity_models) fn part_mut(&mut self, name: &str) -> Option<&mut ModelPart> {
        if name == "root" {
            Some(self)
        } else {
            self.descendant_mut(name)
        }
    }

    fn descendant_mut(&mut self, name: &str) -> Option<&mut ModelPart> {
        if let Some(index) = self
            .children
            .iter()
            .position(|(child, _)| child.as_str() == name)
        {
            return Some(&mut self.children[index].1);
        }
        self.children
            .iter_mut()
            .find_map(|(_, child)| child.descendant_mut(name))
    }

    /// The vertex range a named descendant's subtree occupies in this part's render output, for tests that compare one part across poses.
    #[cfg(test)]
    pub(in crate::entity_models) fn part_vertex_range(
        &self,
        name: &str,
    ) -> Option<std::ops::Range<usize>> {
        fn vertex_count(part: &ModelPart) -> usize {
            if !part.visible {
                return 0;
            }
            part.drawn_cubes()
                .iter()
                .map(|cube| cube.visible_faces.count_ones() as usize * 4)
                .sum::<usize>()
                + part
                    .children
                    .iter()
                    .map(|(_, child)| vertex_count(child))
                    .sum::<usize>()
        }
        fn find(part: &ModelPart, name: &str, start: usize) -> Option<std::ops::Range<usize>> {
            let mut offset = start + vertex_count(part)
                - part
                    .children
                    .iter()
                    .map(|(_, child)| vertex_count(child))
                    .sum::<usize>();
            for (child_name, child) in &part.children {
                if child_name.as_str() == name {
                    return Some(offset..offset + vertex_count(child));
                }
                if let Some(range) = find(child, name, offset) {
                    return Some(range);
                }
                offset += vertex_count(child);
            }
            None
        }
        find(self, name, 0)
    }

    /// Vanilla `ModelPart.getChild(name)`: a mutable handle to the named direct child. Panics if the
    /// child does not exist — a static structural mistake, like vanilla's `NoSuchElementException`.
    pub(in crate::entity_models) fn child_mut(&mut self, name: &str) -> &mut ModelPart {
        self.children
            .iter_mut()
            .find(|(child_name, _)| child_name.as_str() == name)
            .map(|(_, child)| child)
            .unwrap_or_else(|| panic!("model part has no child named `{name}`"))
    }

    /// Immutable counterpart to [`Self::child_mut`], returning `None` for optional render paths that
    /// need to degrade gracefully when a model lacks the requested direct child.
    pub(in crate::entity_models) fn try_child(&self, name: &str) -> Option<&ModelPart> {
        self.children
            .iter()
            .find(|(child_name, _)| child_name.as_str() == name)
            .map(|(_, child)| child)
    }

    /// The posed transform that reaches a named direct child's frame from this part's parent frame:
    /// `self.local_transform() · child.local_transform()` — vanilla `translateToHand`'s
    /// `root.translateAndRotate · arm.translateAndRotate`. Used to attach a held item to a posed arm,
    /// returning `None` when the named child is absent. The generic humanoid hand-attach dispatches over
    /// many model families and must degrade gracefully (render no held item) for any that lacks a
    /// standard arm bone.
    pub(in crate::entity_models) fn try_child_attach_transform(&self, name: &str) -> Option<Mat4> {
        let child = self
            .children
            .iter()
            .find(|(child_name, _)| child_name.as_str() == name)
            .map(|(_, child)| child)?;
        Some(self.local_transform() * child.local_transform())
    }

    /// Selects one baked cube from a named direct child and returns its posed attachment frame.
    /// This is the narrow immutable counterpart of vanilla `PlayerModel.getRandomBodyPart` followed
    /// by `ModelPart.getRandomCube` and `translateAndRotate`; the random index generation stays with
    /// the renderer layer so consuming even a one-element cube list preserves Java RNG parity.
    pub(in crate::entity_models) fn try_child_cube_attachment(
        &self,
        name: &str,
        cube_index: usize,
    ) -> Option<ModelCubeAttachment> {
        let child = self
            .children
            .iter()
            .find(|(child_name, _)| child_name.as_str() == name)
            .map(|(_, child)| child)?;
        let cube = child.cubes.as_slice().get(cube_index)?;
        Some(ModelCubeAttachment {
            transform: self.local_transform() * child.local_transform(),
            min: cube.min(),
            max: cube.max(),
        })
    }

    /// Number of baked cubes on a named direct child. `StuckInBodyLayer` feeds this exact count to
    /// Java `nextInt` before requesting the matching attachment, preserving the random-call stream.
    pub(in crate::entity_models) fn try_child_cube_count(&self, name: &str) -> Option<usize> {
        self.children
            .iter()
            .find(|(child_name, _)| child_name.as_str() == name)
            .map(|(_, child)| child.cubes.as_slice().len())
    }

    /// The posed transform that reaches a descendant path from this part's parent frame. This is the
    /// multi-part counterpart to [`Self::try_child_attach_transform`], used by item layers that walk a
    /// vanilla `PoseStack` through nested bones such as `root -> head -> nose`.
    pub(in crate::entity_models) fn try_descendant_attach_transform(
        &self,
        names: &[&str],
    ) -> Option<Mat4> {
        let mut transform = self.local_transform();
        let mut part = self;
        for name in names {
            let child = part
                .children
                .iter()
                .find(|(child_name, _)| child_name.as_str() == *name)
                .map(|(_, child)| child)?;
            transform *= child.local_transform();
            part = child;
        }
        Some(transform)
    }

    /// Copies the animated delta of each named direct child while preserving this model's own bind
    /// offsets. Baby armor models have vanilla-authored bind offsets that differ from their host body;
    /// they still need dynamic offset bobbing, rotations, and scale from the posed host model.
    pub(in crate::entity_models) fn copy_child_animation_from(
        &mut self,
        source: &ModelPart,
        names: &[&str],
    ) {
        for name in names {
            if let Some((_, src)) = source.children.iter().find(|(n, _)| n.as_str() == *name) {
                let target = self.child_mut(name);
                for axis in 0..3 {
                    target.pose.offset[axis] = target.default_pose.offset[axis]
                        + src.pose.offset[axis]
                        - src.default_pose.offset[axis];
                }
                target.pose.rotation = src.pose.rotation;
                target.scale = src.scale;
            }
        }
    }

    /// Emits the subtree's cube geometry without texture coordinates, for untextured passes such as
    /// the boat water mask.
    pub(in crate::entity_models) fn render_untextured(
        &self,
        mesh: &mut EntityModelMesh,
        parent_transform: Mat4,
    ) {
        if !self.visible {
            return;
        }
        let transform = parent_transform * self.local_transform();
        for cube in self.drawn_cubes() {
            emit_untextured_model_cube(mesh, transform, cube);
        }
        for (_, child) in &self.children {
            child.render_untextured(mesh, transform);
        }
    }

    /// Vanilla `ModelPart.render`: emits every visible cube into `mesh` with `uv_rect` / `tint`.
    pub(in crate::entity_models) fn render_textured(
        &self,
        mesh: &mut EntityModelTexturedMesh,
        mut parent_transform: Mat4,
        uv_rect: EntityModelUvRect,
        tint: [f32; 4],
    ) {
        if !self.visible {
            return;
        }
        let extracted = begin_retained_origin(mesh, &mut parent_transform);
        let transform = parent_transform * self.local_transform();
        self.emit_cubes(mesh, transform, uv_rect, tint);
        for (_, child) in &self.children {
            child.render_textured(mesh, transform, uv_rect, tint);
        }
        if extracted {
            mesh.part_origin = None;
        }
    }

    /// Walks the subtree like [`ModelPart::render_textured`], but emits only the cubes of the parts
    /// named in `retained`, mirroring vanilla `PartDefinition.retainExactParts`: a retained part draws
    /// its own cubes and its whole subtree is dropped (vanilla `clearRecursively` empties every
    /// descendant), so a retained ancestor short-circuits any retained descendants; a non-retained
    /// part draws nothing but is still traversed, keeping the pose chain to deeper retained parts. Used
    /// by the per-layer emissive overlays that vanilla bakes as part subsets (e.g. the warden's
    /// bioluminescent / pulsating-spots / heart / tendril layers). `name` is this part's name in its
    /// parent's child list (`""` for the nameless root).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::entity_models) fn render_textured_retained(
        &self,
        mesh: &mut EntityModelTexturedMesh,
        mut parent_transform: Mat4,
        uv_rect: EntityModelUvRect,
        tint: [f32; 4],
        name: &str,
        retained: &[&str],
    ) {
        if !self.visible {
            return;
        }
        let extracted = begin_retained_origin(mesh, &mut parent_transform);
        let transform = parent_transform * self.local_transform();
        if retained.contains(&name) {
            self.emit_cubes(mesh, transform, uv_rect, tint);
        } else {
            for (child_name, child) in &self.children {
                child.render_textured_retained(
                    mesh,
                    transform,
                    uv_rect,
                    tint,
                    child_name.as_str(),
                    retained,
                );
            }
        }
        if extracted {
            mesh.part_origin = None;
        }
    }

    /// Walks the subtree like [`ModelPart::render_textured`], but skips any part whose name is listed
    /// in `excluded` and drops that skipped part's whole subtree. This mirrors the vanilla no-hat
    /// villager layers, where the alternate model omits the `hat` / `hat_rim` parts but keeps the
    /// already-posed head, body, arms, and legs.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::entity_models) fn render_textured_excluding(
        &self,
        mesh: &mut EntityModelTexturedMesh,
        mut parent_transform: Mat4,
        uv_rect: EntityModelUvRect,
        tint: [f32; 4],
        name: &str,
        excluded: &[&str],
    ) {
        if !self.visible || excluded.contains(&name) {
            return;
        }
        let extracted = begin_retained_origin(mesh, &mut parent_transform);
        let transform = parent_transform * self.local_transform();
        self.emit_cubes(mesh, transform, uv_rect, tint);
        for (child_name, child) in &self.children {
            child.render_textured_excluding(
                mesh,
                transform,
                uv_rect,
                tint,
                child_name.as_str(),
                excluded,
            );
        }
        if extracted {
            mesh.part_origin = None;
        }
    }
}

fn begin_retained_origin(mesh: &mut EntityModelTexturedMesh, parent_transform: &mut Mat4) -> bool {
    if mesh.retain_parts && mesh.part_origin.is_none() {
        mesh.part_origin = Some(parent_transform.w_axis.truncate().to_array());
        parent_transform.w_axis = glam::Vec4::W;
        true
    } else {
        false
    }
}
