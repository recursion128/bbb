use glam::Mat4;

use super::geometry::{
    emit_model_cube, part_pose_transform, EntityModelMesh, ModelCubeDesc, PartPose,
};
use super::instances::EntityModelInstance;

/// A mutable model part, mirroring vanilla `net.minecraft.client.model.geom.ModelPart`. A model is
/// a tree of these built once from its baked layer geometry. Each frame vanilla restores the bind
/// pose (`resetPose`), the entity's `setupAnim` mutates the named parts' `pose` fields from the
/// projected render state, and `render` walks the tree applying every part's pose as a parent-
/// relative `PoseStack` transform. This replaces the per-entity hand-walked `emit_*` functions:
/// instead of threading `Mat4`s and rebuilding `ModelPartDesc` copies by hand, an entity model
/// mutates this tree and renders it once, exactly as vanilla does.
pub(in crate::entity_models) struct ModelPart {
    /// Current pose (vanilla `x/y/z` + `xRot/yRot/zRot`): reset to `default_pose` each frame and
    /// mutated by `setup_anim`.
    pub(in crate::entity_models) pose: PartPose,
    /// Bind pose captured when the tree is built (vanilla `PartPose`/`loadPose`); restored by
    /// [`ModelPart::reset_pose`].
    default_pose: PartPose,
    /// This part's cubes. Cube geometry never animates, so the baked `&'static` data is shared
    /// rather than copied.
    cubes: &'static [ModelCubeDesc],
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
        cubes: &'static [ModelCubeDesc],
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
    pub(in crate::entity_models) fn leaf(pose: PartPose, cubes: &'static [ModelCubeDesc]) -> Self {
        Self::new(pose, cubes, Vec::new())
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

    /// Walks the subtree, emitting every visible cube into `mesh` with the cube's baked color.
    /// Mirrors vanilla `ModelPart.render`: apply this part's pose, draw the cubes, recurse.
    pub(in crate::entity_models) fn render(
        &self,
        mesh: &mut EntityModelMesh,
        parent_transform: Mat4,
    ) {
        if !self.visible {
            return;
        }
        let transform = parent_transform * part_pose_transform(self.pose);
        for cube in self.cubes {
            emit_model_cube(mesh, transform, *cube);
        }
        for (_, child) in &self.children {
            child.render(mesh, transform);
        }
    }
}

/// A mutable entity model, mirroring vanilla `EntityModel`: own a [`ModelPart`] tree, reset it to
/// the bind pose, mutate it from the projected render state in `setup_anim`, then render it. An
/// implementor supplies only the root accessors and the animation; [`EntityModel::prepare_and_render`]
/// wires the vanilla `resetPose → setupAnim → render` order so the call site stays a one-liner.
pub(in crate::entity_models) trait EntityModel {
    /// The model's root [`ModelPart`] (vanilla `ModelPart root`).
    fn root(&self) -> &ModelPart;
    /// Mutable access to the root, used to reset and address parts during `setup_anim`.
    fn root_mut(&mut self) -> &mut ModelPart;
    /// Vanilla `EntityModel.setupAnim`: mutate the (already reset) tree from the render state.
    fn setup_anim(&mut self, instance: &EntityModelInstance);

    /// Vanilla per-frame flow: restore the bind pose, run `setup_anim`, then render the tree under
    /// `root_transform` (the model→world transform from `LivingEntityRenderer.setupRotations`).
    fn prepare_and_render(
        &mut self,
        mesh: &mut EntityModelMesh,
        instance: &EntityModelInstance,
        root_transform: Mat4,
    ) {
        self.root_mut().reset_pose();
        self.setup_anim(instance);
        self.root().render(mesh, root_transform);
    }
}
