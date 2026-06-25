use super::PART_POSE_ZERO;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const SLIME_GREEN: [f32; 4] = [0.42, 0.82, 0.30, 1.0];
pub(in crate::entity_models) const SLIME_FEATURE_DARK: [f32; 4] = [0.16, 0.28, 0.10, 1.0];
pub(in crate::entity_models) const MAGMA_CUBE_ORANGE: [f32; 4] = [0.92, 0.38, 0.12, 1.0];
pub(in crate::entity_models) const MAGMA_CUBE_CORE: [f32; 4] = [0.98, 0.72, 0.22, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_SLIME: &str = "minecraft:slime#main";
pub(in crate::entity_models) const MODEL_LAYER_SLIME_OUTER: &str = "minecraft:slime#outer";
pub(in crate::entity_models) const MODEL_LAYER_MAGMA_CUBE: &str = "minecraft:magma_cube#main";

// Vanilla 26.1 `SlimeModel.createInnerBodyLayer` cubes (atlas 64×32). Each unified cube carries the
// colored tint and the textured `uv_size`/`texOffs`; all parts sit at the identity pose.
pub(in crate::entity_models) const SLIME_INNER_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 17.0, -3.0],
    [6.0, 6.0, 6.0],
    SLIME_GREEN,
    [6.0, 6.0, 6.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const SLIME_RIGHT_EYE: [ModelCube; 1] = [ModelCube::new(
    [-3.25, 18.0, -3.5],
    [2.0, 2.0, 2.0],
    SLIME_FEATURE_DARK,
    [2.0, 2.0, 2.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SLIME_LEFT_EYE: [ModelCube; 1] = [ModelCube::new(
    [1.25, 18.0, -3.5],
    [2.0, 2.0, 2.0],
    SLIME_FEATURE_DARK,
    [2.0, 2.0, 2.0],
    [32.0, 4.0],
    false,
)];

pub(in crate::entity_models) const SLIME_MOUTH: [ModelCube; 1] = [ModelCube::new(
    [0.0, 21.0, -3.5],
    [1.0, 1.0, 1.0],
    SLIME_FEATURE_DARK,
    [1.0, 1.0, 1.0],
    [32.0, 8.0],
    false,
)];

// Vanilla 26.1 `SlimeModel.createOuterBodyLayer`: the single translucent 8³ shell cube.
pub(in crate::entity_models) const SLIME_OUTER_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 16.0, -4.0],
    [8.0, 8.0, 8.0],
    SLIME_GREEN,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

/// Vanilla `LavaSlimeModel.createBodyLayer` segment `texOffs` (left/top atlas `v` ladder).
const MAGMA_CUBE_SEGMENT_TEX: [[f32; 2]; 8] = [
    [0.0, 0.0],
    [0.0, 9.0],
    [0.0, 18.0],
    [0.0, 27.0],
    [32.0, 0.0],
    [32.0, 9.0],
    [32.0, 18.0],
    [32.0, 27.0],
];

/// Segment `i` of the magma cube's eight stacked outer slices: `box(-4, 16 + i, -4, 8×1×8)`.
pub(in crate::entity_models) fn magma_cube_segment_cube(index: usize) -> ModelCube {
    ModelCube::new(
        [-4.0, 16.0 + index as f32, -4.0],
        [8.0, 1.0, 8.0],
        MAGMA_CUBE_ORANGE,
        [8.0, 1.0, 8.0],
        MAGMA_CUBE_SEGMENT_TEX[index],
        false,
    )
}

pub(in crate::entity_models) const MAGMA_CUBE_INSIDE_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 18.0, -2.0],
    [4.0, 4.0, 4.0],
    MAGMA_CUBE_CORE,
    [4.0, 4.0, 4.0],
    [24.0, 40.0],
    false,
)];

/// Vanilla `LavaSlimeModel.createBodyLayer` segment child names, `cube0..cube7`.
const MAGMA_CUBE_SEGMENT_NAMES: [&str; 8] = [
    "cube0", "cube1", "cube2", "cube3", "cube4", "cube5", "cube6", "cube7",
];

/// Mutable magma cube model, mirroring vanilla `LavaSlimeModel`. The unified tree is built once with
/// named children: eight stacked outer segments (`cube0..cube7`) plus the inner `inside_cube` core.
/// Vanilla `LavaSlimeModel.setupAnim` fans the eight segments apart vertically by the interpolated,
/// non-negative `squish` (`cubeN.y = -(4 - N) * max(0, squish) * 1.7`), reconstructed client-side from
/// the `onGround()` jump transitions and projected onto the render state. The overall non-uniform body
/// scale lives in the root transform (`magma_cube_model_root_transform`).
pub(in crate::entity_models) struct MagmaCubeModel {
    root: ModelPart,
}

impl MagmaCubeModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(9);
        for (i, &name) in MAGMA_CUBE_SEGMENT_NAMES.iter().enumerate() {
            children.push((
                name,
                ModelPart::leaf(PART_POSE_ZERO, vec![magma_cube_segment_cube(i)]),
            ));
        }
        children.push((
            "inside_cube",
            ModelPart::leaf(PART_POSE_ZERO, MAGMA_CUBE_INSIDE_CUBE.to_vec()),
        ));
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for MagmaCubeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `LavaSlimeModel.setupAnim`: only the non-negative squish (the jump
        // stretch, never the landing splat) spreads the eight stacked segments apart
        // vertically — `cubeN.y = -(4 - N) * max(0, squish) * 1.7`. The lower segments
        // (N < 4) sink and the upper ones (N > 4) rise, opening gaps between the lava
        // slices. The overall body scale is applied separately by the root transform.
        let slime_squish = instance.render_state.slime_squish.max(0.0);
        for (i, &name) in MAGMA_CUBE_SEGMENT_NAMES.iter().enumerate() {
            self.root.child_mut(name).pose.offset[1] =
                -((4 - i as i32) as f32) * slime_squish * 1.7;
        }
    }
}

/// Mutable slime inner-body model, mirroring vanilla `SlimeModel` (`ModelLayers.SLIME`,
/// `createInnerBodyLayer`): the inner `cube` plus the two eyes and the `mouth`. The unified tree is
/// built with named children. Vanilla `SlimeModel` is typed on the base `EntityRenderState` (it never
/// sees the squish) and has no `setupAnim` override, so `setup_anim` is a genuine no-op; the whole
/// squish + per-size stretch lives in `SlimeRenderer.scale` (`slime_model_root_transform`), unlike the
/// magma cube whose `LavaSlimeModel.setupAnim` additionally fans its segments apart. The inner body is
/// the opaque cutout pass; [`SlimeOuterModel`] is the translucent shell.
pub(in crate::entity_models) struct SlimeModel {
    root: ModelPart,
}

impl SlimeModel {
    pub(in crate::entity_models) fn new() -> Self {
        let children: Vec<(&'static str, ModelPart)> = vec![
            (
                "cube",
                ModelPart::leaf(PART_POSE_ZERO, SLIME_INNER_CUBE.to_vec()),
            ),
            (
                "right_eye",
                ModelPart::leaf(PART_POSE_ZERO, SLIME_RIGHT_EYE.to_vec()),
            ),
            (
                "left_eye",
                ModelPart::leaf(PART_POSE_ZERO, SLIME_LEFT_EYE.to_vec()),
            ),
            (
                "mouth",
                ModelPart::leaf(PART_POSE_ZERO, SLIME_MOUTH.to_vec()),
            ),
        ];
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for SlimeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}

/// Mutable slime outer-shell model, mirroring vanilla `SlimeModel`'s `ModelLayers.SLIME_OUTER`
/// (`createOuterBodyLayer`): the single translucent 8³ shell cube. The unified tree is the lone `cube`
/// child; `setup_anim` is a no-op for the same reason as [`SlimeModel`]. Rendered on the translucent
/// pass over the inner body (and in the colored fallback both layers draw, reproducing the combined
/// slime mesh).
pub(in crate::entity_models) struct SlimeOuterModel {
    root: ModelPart,
}

impl SlimeOuterModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "cube",
                    ModelPart::leaf(PART_POSE_ZERO, SLIME_OUTER_CUBE.to_vec()),
                )],
            ),
        }
    }
}

impl EntityModel for SlimeOuterModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
