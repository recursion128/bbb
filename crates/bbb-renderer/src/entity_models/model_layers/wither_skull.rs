use super::{
    PartPose, ADULT_PIGLIN_HEAD, ADULT_PIGLIN_LEFT_EAR, ADULT_PIGLIN_LEFT_EAR_POSE,
    ADULT_PIGLIN_RIGHT_EAR, ADULT_PIGLIN_RIGHT_EAR_POSE, DRAGON_HEAD_CUBES, DRAGON_JAW_CUBES,
    DRAGON_JAW_POSE, PART_POSE_ZERO, WITHER_SKULL_GRAY,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};
use std::f32::consts::PI;

pub(in crate::entity_models) const MODEL_LAYER_CREEPER_HEAD: &str = "minecraft:creeper_head#main";
pub(in crate::entity_models) const MODEL_LAYER_DRAGON_SKULL: &str = "minecraft:dragon_skull#main";
pub(in crate::entity_models) const MODEL_LAYER_PIGLIN_HEAD: &str = "minecraft:piglin_head#main";
pub(in crate::entity_models) const MODEL_LAYER_PLAYER_HEAD: &str = "minecraft:player_head#main";
pub(in crate::entity_models) const MODEL_LAYER_SKELETON_SKULL: &str =
    "minecraft:skeleton_skull#main";
pub(in crate::entity_models) const MODEL_LAYER_WITHER_SKULL: &str = "minecraft:wither_skull#main";
pub(in crate::entity_models) const MODEL_LAYER_WITHER_SKELETON_SKULL: &str =
    "minecraft:wither_skeleton_skull#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_HEAD: &str = "minecraft:zombie_head#main";

// Vanilla 26.1 `WitherSkullRenderer.createSkullLayer` (atlas 64×64): one `head` part at ZERO with a
// single 8×8×8 box (`addBox(-4, -8, -4, 8, 8, 8)` at `texOffs(0, 35)`). `SkullModel.setupAnim` turns
// the head by the projectile's flight `yRot`/`xRot`; since the single part sits at ZERO that facing is
// folded into `wither_skull_model_root_transform` (together with the renderer's `scale(-1, -1, 1)`
// flip). `WitherSkullRenderer` is a plain `EntityRenderer`; `isDangerous` selects between `wither.png`
// and `wither_invulnerable.png`. The cube carries the colored debug tint and the textured `uv_size` /
// `texOffs(0, 35)`.

pub(in crate::entity_models) const WITHER_SKULL_CUBE: ModelCube = ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    WITHER_SKULL_GRAY,
    [8.0, 8.0, 8.0],
    [0.0, 35.0],
    false,
);

// Vanilla `SkullModel.createMobHeadLayer` used by `CustomHeadLayer` for skeleton / wither-skeleton /
// zombie / creeper skull equipment: one `head` cube at `texOffs(0, 0)` on a 64x32 texture.
pub(in crate::entity_models) const CUSTOM_HEAD_SKULL_CUBE: ModelCube = ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    WITHER_SKULL_GRAY,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
);

// Vanilla `SkullModel.createHumanoidHeadLayer` adds a `hat` child to the same base head:
// `texOffs(32, 0).addBox(-4, -8, -4, 8, 8, 8, CubeDeformation(0.25))` on a 64x64 skin texture.
pub(in crate::entity_models) const CUSTOM_HEAD_PLAYER_HAT_CUBE: ModelCube = ModelCube::new(
    [-4.25, -8.25, -4.25],
    [8.5, 8.5, 8.5],
    WITHER_SKULL_GRAY,
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
);

// Vanilla `DragonHeadModel.createHeadLayer`: the dragon skull layer reuses the ender-dragon head
// and jaw cubes, but the `head` part is at `PartPose.offset(0, -7.986666, 0).scaled(0.75)`.
pub(in crate::entity_models) const CUSTOM_HEAD_DRAGON_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -7.986666, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CUSTOM_HEAD_DRAGON_HEAD_SCALE: [f32; 3] = [0.75, 0.75, 0.75];

/// Static wither-skull model mirroring vanilla `SkullModel` at its ZERO rest pose: a single `head`
/// part holding the 8×8×8 skull box (the flight facing lives in the root transform), no `setup_anim`.
pub(in crate::entity_models) struct WitherSkullModel {
    root: ModelPart,
}

impl WitherSkullModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "head",
                    ModelPart::leaf(PART_POSE_ZERO, vec![WITHER_SKULL_CUBE]),
                )],
            ),
        }
    }
}

impl EntityModel for WitherSkullModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}

/// Static skull model for `CustomHeadLayer` skull equipment. Mob heads are only the base `head` cube;
/// profileless player heads use the humanoid head layer with the inflated hat child. The host model
/// already supplied the posed head transform, so this skull tree remains at its baked ZERO pose.
pub(in crate::entity_models) struct CustomHeadSkullModel {
    root: ModelPart,
}

impl CustomHeadSkullModel {
    pub(in crate::entity_models) fn new(include_player_hat: bool) -> Self {
        let head = if include_player_hat {
            ModelPart::new(
                PART_POSE_ZERO,
                vec![CUSTOM_HEAD_SKULL_CUBE],
                vec![(
                    "hat",
                    ModelPart::leaf(PART_POSE_ZERO, vec![CUSTOM_HEAD_PLAYER_HAT_CUBE]),
                )],
            )
        } else {
            ModelPart::leaf(PART_POSE_ZERO, vec![CUSTOM_HEAD_SKULL_CUBE])
        };
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("head", head)]),
        }
    }
}

impl EntityModel for CustomHeadSkullModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}

/// Dragon skull model for `CustomHeadLayer` skull equipment. Vanilla `DragonHeadModel` uses the
/// ender-dragon head and jaw cubes, a scaled `head` part, and opens the jaw from
/// `SkullModelBase.State.animationPos`.
pub(in crate::entity_models) struct CustomHeadDragonSkullModel {
    root: ModelPart,
}

impl CustomHeadDragonSkullModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "head",
                    ModelPart::new(
                        CUSTOM_HEAD_DRAGON_HEAD_POSE,
                        DRAGON_HEAD_CUBES.to_vec(),
                        vec![(
                            "jaw",
                            ModelPart::leaf(DRAGON_JAW_POSE, DRAGON_JAW_CUBES.to_vec()),
                        )],
                    ),
                )],
            ),
        }
    }
}

impl EntityModel for CustomHeadDragonSkullModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let animation_pos = instance.render_state.worn_head_animation_pos;
        let head = self.root.child_mut("head");
        head.scale = CUSTOM_HEAD_DRAGON_HEAD_SCALE;
        head.child_mut("jaw").pose.rotation[0] = ((animation_pos * PI * 0.2).sin() + 1.0) * 0.2;
    }
}

/// Piglin skull model for `CustomHeadLayer` skull equipment. Vanilla `PiglinHeadModel` reuses
/// `AbstractPiglinModel.addHead(CubeDeformation.NONE)`: adult piglin head cubes plus two ear
/// children, then drives only the ears from `SkullModelBase.State.animationPos`.
pub(in crate::entity_models) struct CustomHeadPiglinSkullModel {
    root: ModelPart,
}

impl CustomHeadPiglinSkullModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "head",
                    ModelPart::new(
                        PART_POSE_ZERO,
                        ADULT_PIGLIN_HEAD.to_vec(),
                        vec![
                            (
                                "left_ear",
                                ModelPart::leaf(
                                    ADULT_PIGLIN_LEFT_EAR_POSE,
                                    ADULT_PIGLIN_LEFT_EAR.to_vec(),
                                ),
                            ),
                            (
                                "right_ear",
                                ModelPart::leaf(
                                    ADULT_PIGLIN_RIGHT_EAR_POSE,
                                    ADULT_PIGLIN_RIGHT_EAR.to_vec(),
                                ),
                            ),
                        ],
                    ),
                )],
            ),
        }
    }
}

impl EntityModel for CustomHeadPiglinSkullModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let animation_pos = instance.render_state.worn_head_animation_pos;
        let head = self.root.child_mut("head");
        head.child_mut("left_ear").pose.rotation[2] =
            -((animation_pos * PI * 0.2 * 1.2).cos() + 2.5) * 0.2;
        head.child_mut("right_ear").pose.rotation[2] =
            ((animation_pos * PI * 0.2).cos() + 2.5) * 0.2;
    }
}
