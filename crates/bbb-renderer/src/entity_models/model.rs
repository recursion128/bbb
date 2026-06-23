use glam::Mat4;

use super::geometry::{
    emit_model_cube, emit_textured_model_cube, part_pose_transform, EntityModelMesh,
    EntityModelTexturedMesh, ModelCubeDesc, PartPose, TexturedModelCubeDesc,
};
use super::instances::EntityModelInstance;
use super::{EntityModelTextureRef, EntityModelUvRect};

/// A unified model cube carrying both render paths' data, mirroring a vanilla `ModelPart.Cube`. The
/// `min`/`size` geometry is shared; `color` drives the colored debug path and `uv_size`/`tex`/
/// `mirror` drive the textured path. Authoring one cube replaces the former parallel
/// `ModelCubeDesc` (colored) and `TexturedModelCubeDesc` (textured) consts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity_models) struct ModelCube {
    pub(in crate::entity_models) min: [f32; 3],
    pub(in crate::entity_models) size: [f32; 3],
    /// Colored debug tint.
    pub(in crate::entity_models) color: [f32; 4],
    /// Vanilla `addBox` UV box size (`width`/`height`/`depth`); equals `size` for `CubeDeformation.NONE`.
    pub(in crate::entity_models) uv_size: [f32; 3],
    /// Vanilla `texOffs` atlas origin.
    pub(in crate::entity_models) tex: [f32; 2],
    /// Vanilla `mirror`.
    pub(in crate::entity_models) mirror: bool,
}

impl ModelCube {
    /// A cube authored for both paths (colored color + textured UV).
    pub(in crate::entity_models) const fn new(
        min: [f32; 3],
        size: [f32; 3],
        color: [f32; 4],
        uv_size: [f32; 3],
        tex: [f32; 2],
        mirror: bool,
    ) -> Self {
        Self {
            min,
            size,
            color,
            uv_size,
            tex,
            mirror,
        }
    }

    /// A colored-only cube (no textured path yet): the UV fields are unused because
    /// [`ModelPart::render_textured`] is never called for a colored-only model.
    fn from_colored_desc(desc: &ModelCubeDesc) -> Self {
        Self {
            min: desc.min,
            size: desc.size,
            color: desc.color,
            uv_size: [0.0, 0.0, 0.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    }

    fn colored_desc(&self) -> ModelCubeDesc {
        ModelCubeDesc {
            min: self.min,
            size: self.size,
            color: self.color,
        }
    }

    fn textured_desc(&self) -> TexturedModelCubeDesc {
        TexturedModelCubeDesc {
            min: self.min,
            size: self.size,
            uv_size: self.uv_size,
            tex: self.tex,
            mirror: self.mirror,
        }
    }
}

/// A mutable model part, mirroring vanilla `net.minecraft.client.model.geom.ModelPart`. A model is
/// a tree of these built once from its baked layer geometry. Each frame vanilla restores the bind
/// pose (`resetPose`), the entity's `setupAnim` mutates the named parts' `pose` fields from the
/// projected render state, and the tree is rendered by walking it and applying every part's pose as
/// a parent-relative `PoseStack` transform. The same posed tree drives BOTH the colored debug path
/// ([`ModelPart::render_colored`]) and the textured path ([`ModelPart::render_textured`]), so an
/// entity's `setup_anim` runs once instead of being duplicated across two hand-walked `emit_*`
/// functions.
pub(in crate::entity_models) struct ModelPart {
    /// Current pose (vanilla `x/y/z` + `xRot/yRot/zRot`): reset to `default_pose` each frame and
    /// mutated by `setup_anim`.
    pub(in crate::entity_models) pose: PartPose,
    /// Bind pose captured when the tree is built (vanilla `PartPose`/`loadPose`); restored by
    /// [`ModelPart::reset_pose`].
    default_pose: PartPose,
    /// This part's cubes (both render paths' data).
    cubes: Vec<ModelCube>,
    /// Named children in vanilla declaration / render order (vanilla `Map<String, ModelPart>`,
    /// whose iteration order is the insertion order). Names drive [`ModelPart::child_mut`].
    children: Vec<(&'static str, ModelPart)>,
    /// Vanilla `ModelPart.visible`: a hidden part and its whole subtree are skipped at render.
    pub(in crate::entity_models) visible: bool,
}

impl ModelPart {
    /// Builds a part at `pose` carrying `cubes` and the named `children`, capturing `pose` as the
    /// bind pose that [`ModelPart::reset_pose`] restores.
    pub(in crate::entity_models) fn new(
        pose: PartPose,
        cubes: Vec<ModelCube>,
        children: Vec<(&'static str, ModelPart)>,
    ) -> Self {
        Self {
            pose,
            default_pose: pose,
            cubes,
            children,
            visible: true,
        }
    }

    /// A leaf part (no children), the common case for a single bone.
    pub(in crate::entity_models) fn leaf(pose: PartPose, cubes: Vec<ModelCube>) -> Self {
        Self::new(pose, cubes, Vec::new())
    }

    /// A colored-only leaf built from the existing baked [`ModelCubeDesc`] geometry. Lets a
    /// colored-only entity (no textured path yet) reuse its `&'static` cube consts verbatim.
    pub(in crate::entity_models) fn leaf_colored(pose: PartPose, cubes: &[ModelCubeDesc]) -> Self {
        Self::leaf(
            pose,
            cubes.iter().map(ModelCube::from_colored_desc).collect(),
        )
    }

    /// Vanilla `ModelPart.resetPose` over the whole subtree: restores the bind pose and visibility
    /// so each frame's `setup_anim` starts from a clean slate.
    pub(in crate::entity_models) fn reset_pose(&mut self) {
        self.pose = self.default_pose;
        self.visible = true;
        for (_, child) in &mut self.children {
            child.reset_pose();
        }
    }

    /// Vanilla `ModelPart.getChild(name)`: a mutable handle to the named direct child. Panics if the
    /// child does not exist — a static structural mistake, like vanilla's `NoSuchElementException`.
    pub(in crate::entity_models) fn child_mut(&mut self, name: &str) -> &mut ModelPart {
        self.children
            .iter_mut()
            .find(|(child_name, _)| *child_name == name)
            .map(|(_, child)| child)
            .unwrap_or_else(|| panic!("model part has no child named `{name}`"))
    }

    /// Walks the subtree, emitting every visible cube into the colored `mesh` with the cube's baked
    /// color. Mirrors vanilla `ModelPart.render`: apply this part's pose, draw the cubes, recurse.
    pub(in crate::entity_models) fn render_colored(
        &self,
        mesh: &mut EntityModelMesh,
        parent_transform: Mat4,
    ) {
        if !self.visible {
            return;
        }
        let transform = parent_transform * part_pose_transform(self.pose);
        for cube in &self.cubes {
            emit_model_cube(mesh, transform, cube.colored_desc());
        }
        for (_, child) in &self.children {
            child.render_colored(mesh, transform);
        }
    }

    /// Walks the subtree, emitting every visible cube into the textured `mesh` with `texture` /
    /// `uv_rect` / `tint`. The textured counterpart of [`ModelPart::render_colored`], reading the
    /// same posed tree so one `setup_anim` drives both paths.
    pub(in crate::entity_models) fn render_textured(
        &self,
        mesh: &mut EntityModelTexturedMesh,
        parent_transform: Mat4,
        texture: EntityModelTextureRef,
        uv_rect: EntityModelUvRect,
        tint: [f32; 4],
    ) {
        if !self.visible {
            return;
        }
        let transform = parent_transform * part_pose_transform(self.pose);
        for cube in &self.cubes {
            emit_textured_model_cube(
                mesh,
                transform,
                cube.textured_desc(),
                texture,
                uv_rect,
                tint,
            );
        }
        for (_, child) in &self.children {
            child.render_textured(mesh, transform, texture, uv_rect, tint);
        }
    }
}

/// A mutable entity model, mirroring vanilla `EntityModel`: own a [`ModelPart`] tree, reset it to
/// the bind pose, mutate it from the projected render state in `setup_anim`, then render it. An
/// implementor supplies only the root accessors and the animation; [`EntityModel::prepare`] wires
/// the vanilla `resetPose → setupAnim` order, and [`EntityModel::prepare_and_render`] adds the
/// colored render so the colored call site stays a one-liner. The textured call site uses `prepare`
/// then `root().render_textured(...)` per layer pass.
pub(in crate::entity_models) trait EntityModel {
    /// The model's root [`ModelPart`] (vanilla `ModelPart root`).
    fn root(&self) -> &ModelPart;
    /// Mutable access to the root, used to reset and address parts during `setup_anim`.
    fn root_mut(&mut self) -> &mut ModelPart;
    /// Vanilla `EntityModel.setupAnim`: mutate the (already reset) tree from the render state.
    fn setup_anim(&mut self, instance: &EntityModelInstance);

    /// Vanilla `resetPose → setupAnim`: restore the bind pose, then run `setup_anim`. Leaves the
    /// tree posed and ready for either render path.
    fn prepare(&mut self, instance: &EntityModelInstance) {
        self.root_mut().reset_pose();
        self.setup_anim(instance);
    }

    /// Colored per-frame flow: `prepare`, then render the tree under `root_transform` (the
    /// model→world transform from `LivingEntityRenderer.setupRotations`).
    fn prepare_and_render(
        &mut self,
        mesh: &mut EntityModelMesh,
        instance: &EntityModelInstance,
        root_transform: Mat4,
    ) {
        self.prepare(instance);
        self.root().render_colored(mesh, root_transform);
    }
}
