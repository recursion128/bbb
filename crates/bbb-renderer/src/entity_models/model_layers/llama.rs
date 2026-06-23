use super::{apply_head_look, apply_quadruped_leg_swing, PartPose, LLAMA_CREAMY, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 ModelLayers.LLAMA / TRADER_LLAMA: LlamaModel.createBodyLayer(CubeDeformation.NONE),
// atlas 128×64. Each cube carries both render paths' data: the colored debug tint and the textured
// `uv_size` / `texOffs` / `mirror`. The two ears share `texOffs(17, 0)` unmirrored.
pub(in crate::entity_models) const ADULT_LLAMA_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-2.0, -14.0, -10.0],
        [4.0, 4.0, 9.0],
        LLAMA_CREAMY,
        [4.0, 4.0, 9.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -16.0, -6.0],
        [8.0, 18.0, 6.0],
        LLAMA_CREAMY,
        [8.0, 18.0, 6.0],
        [0.0, 14.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -19.0, -4.0],
        [3.0, 3.0, 2.0],
        LLAMA_CREAMY,
        [3.0, 3.0, 2.0],
        [17.0, 0.0],
        false,
    ),
    ModelCube::new(
        [1.0, -19.0, -4.0],
        [3.0, 3.0, 2.0],
        LLAMA_CREAMY,
        [3.0, 3.0, 2.0],
        [17.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_LLAMA_BODY: [ModelCube; 1] = [ModelCube::new(
    [-6.0, -10.0, -7.0],
    [12.0, 18.0, 10.0],
    LLAMA_CREAMY,
    [12.0, 18.0, 10.0],
    [29.0, 0.0],
    false,
)];

pub(in crate::entity_models) const LLAMA_RIGHT_CHEST: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, 0.0],
    [8.0, 8.0, 3.0],
    LLAMA_CREAMY,
    [8.0, 8.0, 3.0],
    [45.0, 28.0],
    false,
)];

pub(in crate::entity_models) const LLAMA_LEFT_CHEST: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, 0.0],
    [8.0, 8.0, 3.0],
    LLAMA_CREAMY,
    [8.0, 8.0, 3.0],
    [45.0, 41.0],
    false,
)];

// All four adult legs share one `CubeListBuilder` (`texOffs(29, 29)`, no mirror).
pub(in crate::entity_models) const ADULT_LLAMA_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 14.0, 4.0],
    LLAMA_CREAMY,
    [4.0, 14.0, 4.0],
    [29.0, 29.0],
    false,
)];

/// The adult llama chest part poses (only present when `has_chest`).
pub(in crate::entity_models) const ADULT_LLAMA_RIGHT_CHEST_POSE: PartPose = PartPose {
    offset: [-8.5, 3.0, 3.0],
    rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
};

pub(in crate::entity_models) const ADULT_LLAMA_LEFT_CHEST_POSE: PartPose = PartPose {
    offset: [5.5, 3.0, 3.0],
    rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
};

// Vanilla 26.1 ModelLayers.LLAMA_BABY / TRADER_LLAMA_BABY:
// BabyLlamaModel.createBodyLayer(CubeDeformation.NONE), atlas 64×64. Each leg has its own `texOffs`.
pub(in crate::entity_models) const BABY_LLAMA_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-3.0, -9.0, -4.0],
        [6.0, 11.0, 4.0],
        LLAMA_CREAMY,
        [6.0, 11.0, 4.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.5, -7.0, -7.0],
        [3.0, 3.0, 3.0],
        LLAMA_CREAMY,
        [3.0, 3.0, 3.0],
        [0.0, 15.0],
        false,
    ),
    ModelCube::new(
        [0.5, -11.0, -3.0],
        [2.0, 2.0, 2.0],
        LLAMA_CREAMY,
        [2.0, 2.0, 2.0],
        [20.0, 4.0],
        false,
    ),
    ModelCube::new(
        [-2.5, -11.0, -3.0],
        [2.0, 2.0, 2.0],
        LLAMA_CREAMY,
        [2.0, 2.0, 2.0],
        [20.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_LLAMA_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.4, -0.5, -1.5],
    [3.0, 8.0, 3.0],
    LLAMA_CREAMY,
    [3.0, 8.0, 3.0],
    [0.0, 45.0],
    false,
)];

pub(in crate::entity_models) const BABY_LLAMA_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.6, -0.5, -1.5],
    [3.0, 8.0, 3.0],
    LLAMA_CREAMY,
    [3.0, 8.0, 3.0],
    [12.0, 45.0],
    false,
)];

pub(in crate::entity_models) const BABY_LLAMA_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.4, -0.5, -1.5],
    [3.0, 8.0, 3.0],
    LLAMA_CREAMY,
    [3.0, 8.0, 3.0],
    [0.0, 34.0],
    false,
)];

pub(in crate::entity_models) const BABY_LLAMA_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.6, -0.5, -1.5],
    [3.0, 8.0, 3.0],
    LLAMA_CREAMY,
    [3.0, 8.0, 3.0],
    [12.0, 34.0],
    false,
)];

pub(in crate::entity_models) const BABY_LLAMA_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -3.0, -8.5],
    [8.0, 6.0, 13.0],
    LLAMA_CREAMY,
    [8.0, 6.0, 13.0],
    [0.0, 15.0],
    false,
)];

// Vanilla 26.1 `ModelLayers.LLAMA` / `LLAMA_BABY` (`LlamaRenderer`). The trader llama bakes the same
// `LlamaModel.createBodyLayer` mesh; the only difference is the deferred `LlamaDecorLayer` overlay.
pub(in crate::entity_models) const MODEL_LAYER_LLAMA: &str = "minecraft:llama#main";
pub(in crate::entity_models) const MODEL_LAYER_LLAMA_BABY: &str = "minecraft:llama_baby#main";

/// The adult llama head/body part poses.
const ADULT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 7.0, -6.0],
    rotation: [0.0, 0.0, 0.0],
};

const ADULT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 5.0, 2.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

/// Builds a leaf part at `offset` (no rotation) carrying `cubes`.
fn leg(offset: [f32; 3], cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes.to_vec(),
    )
}

/// Builds the four adult-llama legs (hind-first, vanilla order) under the vanilla `QuadrupedModel`
/// child names. All four share one leg cube.
fn adult_legs() -> Vec<(&'static str, ModelPart)> {
    vec![
        ("right_hind_leg", leg([-3.5, 10.0, 6.0], &ADULT_LLAMA_LEG)),
        ("left_hind_leg", leg([3.5, 10.0, 6.0], &ADULT_LLAMA_LEG)),
        ("right_front_leg", leg([-3.5, 10.0, -5.0], &ADULT_LLAMA_LEG)),
        ("left_front_leg", leg([3.5, 10.0, -5.0], &ADULT_LLAMA_LEG)),
    ]
}

/// Builds a unified llama root for `baby`/`has_chest`, keeping the vanilla declaration order so the
/// render/atlas vertex layout stays byte-identical: adult is head, body, [chests,] legs; baby is
/// head, legs, body. Uses the vanilla `LlamaModel` child names (`head`, `body`, `right_chest`/
/// `left_chest`, and the four legs).
fn llama_tree(baby: bool, has_chest: bool) -> ModelPart {
    if baby {
        let children = vec![
            (
                "head",
                ModelPart::leaf(
                    PartPose {
                        offset: [0.0, 12.0, -4.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    BABY_LLAMA_HEAD.to_vec(),
                ),
            ),
            (
                "right_hind_leg",
                leg([-2.5, 16.5, 4.5], &BABY_LLAMA_RIGHT_HIND_LEG),
            ),
            (
                "left_hind_leg",
                leg([2.5, 16.5, 4.5], &BABY_LLAMA_LEFT_HIND_LEG),
            ),
            (
                "right_front_leg",
                leg([-2.5, 16.5, -3.5], &BABY_LLAMA_RIGHT_FRONT_LEG),
            ),
            (
                "left_front_leg",
                leg([2.5, 16.5, -3.5], &BABY_LLAMA_LEFT_FRONT_LEG),
            ),
            ("body", leg([0.0, 14.0, 2.5], &BABY_LLAMA_BODY)),
        ];
        return ModelPart::new(PART_POSE_ZERO, Vec::new(), children);
    }
    let mut children = vec![
        (
            "head",
            ModelPart::leaf(ADULT_HEAD_POSE, ADULT_LLAMA_HEAD.to_vec()),
        ),
        (
            "body",
            ModelPart::leaf(ADULT_BODY_POSE, ADULT_LLAMA_BODY.to_vec()),
        ),
    ];
    if has_chest {
        children.push((
            "right_chest",
            ModelPart::leaf(ADULT_LLAMA_RIGHT_CHEST_POSE, LLAMA_RIGHT_CHEST.to_vec()),
        ));
        children.push((
            "left_chest",
            ModelPart::leaf(ADULT_LLAMA_LEFT_CHEST_POSE, LLAMA_LEFT_CHEST.to_vec()),
        ));
    }
    children.extend(adult_legs());
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable llama model, mirroring vanilla `LlamaModel` (a `QuadrupedModel`, shared by the trader
/// llama). The unified tree is built for `baby`/`has_chest` ([`llama_tree`]) with the vanilla child
/// names. `setup_anim` looks the head ([`apply_head_look`] on `head`) and swings the four legs
/// ([`apply_quadruped_leg_swing`]). The family/variant choose only the recolor or texture; the
/// chest visibility rides the tree choice.
pub(in crate::entity_models) struct LlamaModel {
    root: ModelPart,
}

impl LlamaModel {
    pub(in crate::entity_models) fn new(baby: bool, has_chest: bool) -> Self {
        Self {
            root: llama_tree(baby, has_chest),
        }
    }
}

impl EntityModel for LlamaModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
