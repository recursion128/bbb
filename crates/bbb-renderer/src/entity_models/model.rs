use glam::Mat4;

use super::geometry::{
    emit_model_cube, emit_textured_model_cube, part_pose_transform, EntityModelMesh,
    EntityModelTexturedMesh, ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc,
    TexturedModelPartDesc,
};
use super::instances::EntityModelInstance;
use super::{EntityModelTextureRef, EntityModelUvRect};

/// Vanilla child parts are addressed by name; the descs the migration zips from carry none, so a
/// zipped child is named by its index. Twenty-four covers the widest part in the entity catalog (the
/// ender dragon's nineteen root parts).
const INDEX_CHILD_NAMES: [&str; 24] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
    "17", "18", "19", "20", "21", "22", "23",
];

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

    /// A textured-only cube (an overlay layer with no colored debug variant — the tropical fish
    /// pattern, the slime outer shell, …): the geometry/UV come from the textured desc and the
    /// `color` is an unused placeholder because [`ModelPart::render_colored`] is never called for a
    /// textured-only model.
    #[allow(dead_code)]
    fn from_textured_desc(desc: &TexturedModelCubeDesc) -> Self {
        Self {
            min: desc.min,
            size: desc.size,
            color: [0.0, 0.0, 0.0, 0.0],
            uv_size: desc.uv_size,
            tex: desc.tex,
            mirror: desc.mirror,
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

    /// A colored-only part carrying both `cubes` (from baked [`ModelCubeDesc`] geometry) and
    /// `children` — the non-leaf counterpart of [`ModelPart::leaf_colored`], for a colored-only
    /// model whose tree is assembled at construction (the guardian, whose twelve spikes carry
    /// computed bind poses) rather than declared as a `&'static` desc tree. Children are named by
    /// index.
    pub(in crate::entity_models) fn colored(
        pose: PartPose,
        cubes: &[ModelCubeDesc],
        children: Vec<ModelPart>,
    ) -> Self {
        let children = children
            .into_iter()
            .enumerate()
            .map(|(index, child)| (INDEX_CHILD_NAMES[index], child))
            .collect();
        Self {
            pose,
            default_pose: pose,
            cubes: cubes.iter().map(ModelCube::from_colored_desc).collect(),
            children,
            visible: true,
        }
    }

    /// A colored-only part carrying both `cubes` (from baked [`ModelCubeDesc`] geometry) and NAMED
    /// `children` — the named-children counterpart of [`ModelPart::colored`], for a colored-only model
    /// whose cube-bearing parent (head, body, …) parents children that `setup_anim` addresses by name
    /// (the frog's `body` arms, the sniffer's `head` ears, the warden's `body` head and `head`
    /// tendrils, …). Pairs with [`ModelPart::child_mut`] instead of positional `child_at_mut`.
    pub(in crate::entity_models) fn colored_named(
        pose: PartPose,
        cubes: &[ModelCubeDesc],
        children: Vec<(&'static str, ModelPart)>,
    ) -> Self {
        Self {
            pose,
            default_pose: pose,
            cubes: cubes.iter().map(ModelCube::from_colored_desc).collect(),
            children,
            visible: true,
        }
    }

    /// Builds a unified [`ModelPart`] subtree by zipping a colored [`ModelPartDesc`] tree with its
    /// matching textured [`TexturedModelPartDesc`] tree (the two share structure and bind poses; the
    /// textured tree reuses the colored poses). Each unified cube takes its geometry/color from the
    /// colored cube and its UV from the paired textured cube. This lets a dual-path entity reuse its
    /// existing baked cube consts verbatim while collapsing its two hand-walked `emit_*` functions
    /// into one mutable tree driven by a single `setup_anim`. Children are addressed positionally via
    /// [`ModelPart::child_at_mut`] (named by index). Panics if the two trees disagree in shape.
    pub(in crate::entity_models) fn from_descs(
        colored: &ModelPartDesc,
        textured: &TexturedModelPartDesc,
    ) -> Self {
        assert_eq!(
            colored.cubes.len(),
            textured.cubes.len(),
            "colored/textured cube counts diverge"
        );
        assert_eq!(
            colored.children.len(),
            textured.children.len(),
            "colored/textured child counts diverge"
        );
        let cubes = colored
            .cubes
            .iter()
            .zip(textured.cubes.iter())
            .map(|(colored_cube, textured_cube)| ModelCube {
                min: colored_cube.min,
                size: colored_cube.size,
                color: colored_cube.color,
                uv_size: textured_cube.uv_size,
                tex: textured_cube.tex,
                mirror: textured_cube.mirror,
            })
            .collect();
        let children = colored
            .children
            .iter()
            .zip(textured.children.iter())
            .enumerate()
            .map(|(index, (colored_child, textured_child))| {
                (
                    INDEX_CHILD_NAMES[index],
                    ModelPart::from_descs(colored_child, textured_child),
                )
            })
            .collect();
        Self {
            pose: colored.pose,
            default_pose: colored.pose,
            cubes,
            children,
            visible: true,
        }
    }

    /// Builds a unified root [`ModelPart`] over a flat list of sibling colored/textured part trees
    /// (the common vanilla layout where `createBodyLayer` returns several root parts). The siblings
    /// hang off a synthetic identity root and are addressed positionally via
    /// [`ModelPart::child_at_mut`]. Retained as the generic dual-path desc-zip root builder now that
    /// every dual-path entity builds its tree imperatively with named children.
    #[allow(dead_code)]
    pub(in crate::entity_models) fn root_from_descs(
        colored: &[ModelPartDesc],
        textured: &[TexturedModelPartDesc],
    ) -> Self {
        assert_eq!(
            colored.len(),
            textured.len(),
            "colored/textured root part counts diverge"
        );
        let children = colored
            .iter()
            .zip(textured.iter())
            .enumerate()
            .map(|(index, (colored_part, textured_part))| {
                (
                    INDEX_CHILD_NAMES[index],
                    ModelPart::from_descs(colored_part, textured_part),
                )
            })
            .collect();
        Self {
            pose: super::geometry::PART_POSE_ZERO,
            default_pose: super::geometry::PART_POSE_ZERO,
            cubes: Vec::new(),
            children,
            visible: true,
        }
    }

    /// Builds a textured-only [`ModelPart`] subtree from a [`TexturedModelPartDesc`] tree, for an
    /// overlay layer that has no colored debug variant (the tropical fish pattern, the slime outer
    /// shell, …). Each cube's colored color is an unused placeholder; only [`ModelPart::render_textured`]
    /// is ever called on the result. Children are addressed positionally (named by index).
    #[allow(dead_code)]
    pub(in crate::entity_models) fn from_textured_desc(textured: &TexturedModelPartDesc) -> Self {
        let cubes = textured
            .cubes
            .iter()
            .map(ModelCube::from_textured_desc)
            .collect();
        let children = textured
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                (
                    INDEX_CHILD_NAMES[index],
                    ModelPart::from_textured_desc(child),
                )
            })
            .collect();
        Self {
            pose: textured.pose,
            default_pose: textured.pose,
            cubes,
            children,
            visible: true,
        }
    }

    /// Builds a textured-only root [`ModelPart`] over a flat list of sibling [`TexturedModelPartDesc`]
    /// trees — the textured counterpart of [`ModelPart::root_from_descs`] for an overlay layer with no
    /// colored variant. The siblings hang off a synthetic identity root, addressed positionally via
    /// [`ModelPart::child_at_mut`].
    #[allow(dead_code)]
    pub(in crate::entity_models) fn root_from_textured_descs(
        textured: &[TexturedModelPartDesc],
    ) -> Self {
        let children = textured
            .iter()
            .enumerate()
            .map(|(index, part)| {
                (
                    INDEX_CHILD_NAMES[index],
                    ModelPart::from_textured_desc(part),
                )
            })
            .collect();
        Self {
            pose: super::geometry::PART_POSE_ZERO,
            default_pose: super::geometry::PART_POSE_ZERO,
            cubes: Vec::new(),
            children,
            visible: true,
        }
    }

    /// Builds a colored-only [`ModelPart`] subtree from a [`ModelPartDesc`] tree — the colored
    /// counterpart of [`ModelPart::from_textured_desc`] for an entity with no textured path (the
    /// parrot, shulker, …). Each cube reuses its baked color; the textured UV is an unused
    /// placeholder, since only [`ModelPart::render_colored`] is ever called. Children are addressed
    /// positionally (named by index).
    pub(in crate::entity_models) fn from_colored_desc(colored: &ModelPartDesc) -> Self {
        let cubes = colored
            .cubes
            .iter()
            .map(ModelCube::from_colored_desc)
            .collect();
        let children = colored
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                (
                    INDEX_CHILD_NAMES[index],
                    ModelPart::from_colored_desc(child),
                )
            })
            .collect();
        Self {
            pose: colored.pose,
            default_pose: colored.pose,
            cubes,
            children,
            visible: true,
        }
    }

    /// Builds a colored-only root [`ModelPart`] over a flat list of sibling [`ModelPartDesc`] trees —
    /// the colored counterpart of [`ModelPart::root_from_textured_descs`] for an entity with no
    /// textured path. The siblings hang off a synthetic identity root, addressed positionally via
    /// [`ModelPart::child_at_mut`].
    pub(in crate::entity_models) fn root_from_colored_descs(colored: &[ModelPartDesc]) -> Self {
        let children = colored
            .iter()
            .enumerate()
            .map(|(index, part)| (INDEX_CHILD_NAMES[index], ModelPart::from_colored_desc(part)))
            .collect();
        Self {
            pose: super::geometry::PART_POSE_ZERO,
            default_pose: super::geometry::PART_POSE_ZERO,
            cubes: Vec::new(),
            children,
            visible: true,
        }
    }

    /// Builds a synthetic identity root over a flat list of already-built sibling parts, named by
    /// index for positional [`ModelPart::child_at_mut`] access. The runtime counterpart of
    /// [`ModelPart::root_from_descs`] for a model whose parts are computed at construction (the
    /// pufferfish puff states pick one of three part lists) rather than declared as `&'static` descs.
    pub(in crate::entity_models) fn root_from_parts(children: Vec<ModelPart>) -> Self {
        let children = children
            .into_iter()
            .enumerate()
            .map(|(index, part)| (INDEX_CHILD_NAMES[index], part))
            .collect();
        Self {
            pose: super::geometry::PART_POSE_ZERO,
            default_pose: super::geometry::PART_POSE_ZERO,
            cubes: Vec::new(),
            children,
            visible: true,
        }
    }

    /// Vanilla `ModelPart.getChild` by position: a mutable handle to the `index`-th direct child.
    /// Pairs with the entities' existing `*_PART_INDEX` constants. Panics if out of range.
    pub(in crate::entity_models) fn child_at_mut(&mut self, index: usize) -> &mut ModelPart {
        &mut self.children[index].1
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

    /// Walks the subtree like [`ModelPart::render_colored`], but overrides every cube's baked color
    /// with `color` — the colored counterpart of a textured layer's runtime tint. Used by the
    /// entities whose colored fallback recolors the whole model (the zombie variants, the squid, the
    /// dyed sheep wool, …).
    pub(in crate::entity_models) fn render_colored_with_color(
        &self,
        mesh: &mut EntityModelMesh,
        parent_transform: Mat4,
        color: [f32; 4],
    ) {
        if !self.visible {
            return;
        }
        let transform = parent_transform * part_pose_transform(self.pose);
        for cube in &self.cubes {
            let mut desc = cube.colored_desc();
            desc.color = color;
            emit_model_cube(mesh, transform, desc);
        }
        for (_, child) in &self.children {
            child.render_colored_with_color(mesh, transform, color);
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

    /// Like [`EntityModel::prepare_and_render`], but recolors every cube with `color` — for the
    /// entities whose colored fallback overrides the whole model with a single runtime color (the
    /// recolored zombie variants, the squid, the dyed sheep wool, …) rather than its baked colors.
    fn prepare_and_render_with_color(
        &mut self,
        mesh: &mut EntityModelMesh,
        instance: &EntityModelInstance,
        root_transform: Mat4,
        color: [f32; 4],
    ) {
        self.prepare(instance);
        self.root()
            .render_colored_with_color(mesh, root_transform, color);
    }
}

/// A static [`EntityModel`] over a fixed colored part tree — an entity whose vanilla `setupAnim` is a
/// no-op (the leash knot, the thrown trident, the llama spit, …) or whose animations are all
/// deferred (the arrow wobble, the evoker-fangs bite), so it always renders at its bind pose. Built
/// from the baked [`ModelPartDesc`] geometry; `setup_anim` does nothing. Each such entity passes its
/// own `&'static X_PARTS` and its renderer's root transform at the colored call site.
pub(in crate::entity_models) struct StaticModel {
    root: ModelPart,
}

impl StaticModel {
    pub(in crate::entity_models) fn new(parts: &[ModelPartDesc]) -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(parts),
        }
    }
}

impl EntityModel for StaticModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
