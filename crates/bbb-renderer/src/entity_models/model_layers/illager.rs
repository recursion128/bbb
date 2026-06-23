use super::{
    apply_half_amplitude_leg_swing_named, apply_head_look, humanoid_arm_swing_pose, PartPose,
    ILLAGER_HAT_COLOR, ILLAGER_ROBE, PART_POSE_ZERO,
};
use crate::entity_models::catalog::IllagerModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_EVOKER: &str = "minecraft:evoker#main";
pub(in crate::entity_models) const MODEL_LAYER_ILLUSIONER: &str = "minecraft:illusioner#main";
pub(in crate::entity_models) const MODEL_LAYER_PILLAGER: &str = "minecraft:pillager#main";
pub(in crate::entity_models) const MODEL_LAYER_VINDICATOR: &str = "minecraft:vindicator#main";

// Vanilla 26.1 IllagerModel.createBodyLayer(), with LayerDefinitions' MeshTransformer.scaling(0.9375F)
// applied by the emitter root transform. The deformed cubes (the hat, the body's robe overlay) inflate
// their geometry but keep the base box as `uv_size`, exactly like `CubeDeformation` in vanilla `addBox`.
// Each cube carries both render paths' data: the colored debug tint and the textured `uv_size`/`texOffs`/
// `mirror`.
pub(in crate::entity_models) const ILLAGER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -10.0, -4.0],
    [8.0, 10.0, 8.0],
    ILLAGER_ROBE,
    [8.0, 10.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ILLAGER_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.45, -10.45, -4.45],
    [8.9, 12.9, 8.9],
    ILLAGER_HAT_COLOR,
    [8.0, 12.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ILLAGER_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -6.0],
    [2.0, 4.0, 2.0],
    ILLAGER_ROBE,
    [2.0, 4.0, 2.0],
    [24.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ILLAGER_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, 0.0, -3.0],
        [8.0, 12.0, 6.0],
        ILLAGER_ROBE,
        [8.0, 12.0, 6.0],
        [16.0, 20.0],
        false,
    ),
    ModelCube::new(
        [-4.5, -0.5, -3.5],
        [9.0, 21.0, 7.0],
        ILLAGER_ROBE,
        [8.0, 20.0, 6.0],
        [0.0, 38.0],
        false,
    ),
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARMS: [ModelCube; 2] = [
    ModelCube::new(
        [-8.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        ILLAGER_ROBE,
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        false,
    ),
    ModelCube::new(
        [-4.0, 2.0, -2.0],
        [8.0, 4.0, 4.0],
        ILLAGER_ROBE,
        [8.0, 4.0, 4.0],
        [40.0, 38.0],
        false,
    ),
];

pub(in crate::entity_models) const ILLAGER_LEFT_SHOULDER: [ModelCube; 1] = [ModelCube::new(
    [4.0, -2.0, -2.0],
    [4.0, 8.0, 4.0],
    ILLAGER_ROBE,
    [4.0, 8.0, 4.0],
    [44.0, 22.0],
    true,
)];

pub(in crate::entity_models) const ILLAGER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    ILLAGER_ROBE,
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    false,
)];

pub(in crate::entity_models) const ILLAGER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    ILLAGER_ROBE,
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    true,
)];

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    ILLAGER_ROBE,
    [4.0, 12.0, 4.0],
    [40.0, 46.0],
    false,
)];

pub(in crate::entity_models) const ILLAGER_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    ILLAGER_ROBE,
    [4.0, 12.0, 4.0],
    [40.0, 46.0],
    true,
)];

/// The crossed `arms` part pose (folded forward, vanilla `IllagerModel.createBodyLayer`).
pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_POSE: PartPose = PartPose {
    offset: [0.0, 3.0, -1.0],
    rotation: [-0.75, 0.0, 0.0],
};

/// The nose child part pose (under the head).
pub(in crate::entity_models) const ILLAGER_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// The right/left leg part poses.
pub(in crate::entity_models) const ILLAGER_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const ILLAGER_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// The separate (uncrossed) right/left arm part poses.
pub(in crate::entity_models) const ILLAGER_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const ILLAGER_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a leaf part at `pose` carrying `cubes`.
fn leaf(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds the illager `head` part. The illusioner keeps its hat (re-enabled in vanilla via
/// `getHat().visible = true`); the others list only the nose. Both list the nose as a head child.
fn head(hatted: bool) -> ModelPart {
    let mut children = Vec::new();
    if hatted {
        children.push(("hat", leaf(PART_POSE_ZERO, &ILLAGER_HAT)));
    }
    children.push(("nose", leaf(ILLAGER_NOSE_POSE, &ILLAGER_NOSE)));
    ModelPart::new(PART_POSE_ZERO, ILLAGER_HEAD.to_vec(), children)
}

/// Builds the illager `body` part (carrying the robe overlay cube as a second cube, no children).
fn body() -> ModelPart {
    leaf(PART_POSE_ZERO, &ILLAGER_BODY)
}

/// Builds the crossed `arms` part: the folded arms cube parents the left shoulder.
fn crossed_arms() -> ModelPart {
    ModelPart::new(
        ILLAGER_CROSSED_ARM_POSE,
        ILLAGER_CROSSED_ARMS.to_vec(),
        vec![(
            "left_shoulder",
            leaf(PART_POSE_ZERO, &ILLAGER_LEFT_SHOULDER),
        )],
    )
}

/// Builds the two named leg children in vanilla declaration order.
fn legs() -> Vec<(&'static str, ModelPart)> {
    vec![
        (
            "right_leg",
            leaf(ILLAGER_RIGHT_LEG_POSE, &ILLAGER_RIGHT_LEG),
        ),
        ("left_leg", leaf(ILLAGER_LEFT_LEG_POSE, &ILLAGER_LEFT_LEG)),
    ]
}

/// Builds the two named separate-arm children in vanilla declaration order.
fn arms() -> Vec<(&'static str, ModelPart)> {
    vec![
        (
            "right_arm",
            leaf(ILLAGER_RIGHT_ARM_POSE, &ILLAGER_RIGHT_ARM),
        ),
        ("left_arm", leaf(ILLAGER_LEFT_ARM_POSE, &ILLAGER_LEFT_ARM)),
    ]
}

/// Builds the unified illager tree for `family`/`spellcasting`, mirroring the vanilla layer choice
/// with the vanilla `IllagerModel` child names. The idle crossed layout lists `head`, `body`, the
/// folded `arms`, then the legs; the uncrossed (pillager / spellcasting) layout lists `head`, `body`,
/// the legs, then the separate `right_arm`/`left_arm`. The illusioner keeps its hatted head in both.
/// Vanilla declaration order is preserved so the colored render order stays byte-identical, while the
/// head look, leg swing, and arm poses resolve their parts by name.
fn illager_tree(family: IllagerModelFamily, spellcasting: bool) -> ModelPart {
    let hatted = matches!(family, IllagerModelFamily::Illusioner);
    // The idle crossed layout applies to the evoker/vindicator/illusioner that are not spell-casting;
    // the pillager (and any spell-casting evoker/illusioner) use the uncrossed separate-arm layout.
    let crossed = !spellcasting && !matches!(family, IllagerModelFamily::Pillager);
    let mut children: Vec<(&'static str, ModelPart)> =
        vec![("head", head(hatted)), ("body", body())];
    if crossed {
        children.push(("arms", crossed_arms()));
        children.extend(legs());
    } else {
        children.extend(legs());
        children.extend(arms());
    }
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Vanilla `IllagerModel.setupAnim` SPELLCASTING arm pose for one separate arm. The arm holds its
/// base offset (`rightArm.x = -5`/`leftArm.x = 5`, `z = 0` — both already the bind offset), pitches
/// `xRot = cos(ageInTicks · 0.6662) · 0.25`, and rolls outward `zRot = ±3π/4` (right `+`, left `−`),
/// with `yRot = 0`. Reused by both the colored and textured illager emits.
pub(in crate::entity_models) fn illager_spellcast_arm_pose(
    base: PartPose,
    age_in_ticks: f32,
    is_right: bool,
) -> PartPose {
    let three_quarter_pi = std::f32::consts::PI * 3.0 / 4.0;
    PartPose {
        offset: base.offset,
        rotation: [
            (age_in_ticks * 0.6662).cos() * 0.25,
            0.0,
            if is_right {
                three_quarter_pi
            } else {
                -three_quarter_pi
            },
        ],
    }
}

/// Whether an illager is mid spell-cast — only the evoker and illusioner cast, and only then do they
/// swap from the static crossed `arms` layout to the uncrossed separate-arm layout.
fn illager_is_spellcasting(instance: &EntityModelInstance, family: IllagerModelFamily) -> bool {
    instance.render_state.illager_spellcasting
        && matches!(
            family,
            IllagerModelFamily::Evoker | IllagerModelFamily::Illusioner
        )
}

/// Mutable illager model, mirroring vanilla `IllagerModel`/`SpellcasterIllagerModel` shared by the
/// evoker, vindicator, illusioner, and pillager. The unified tree is built for `family`/`spellcasting`
/// with the vanilla child names. `setup_anim` looks the head ([`apply_head_look`] on `head`) and swings
/// the legs at the villager-family half amplitude ([`apply_half_amplitude_leg_swing_named`]); the
/// pillager additionally swings its separate arms at the `HumanoidModel` amplitude
/// ([`humanoid_arm_swing_pose`] on `right_arm`/`left_arm`), while a spellcasting evoker/illusioner
/// instead raises both arms into the `SPELLCASTING` pose ([`illager_spellcast_arm_pose`]). The
/// attack/bow/crossbow/celebrate arm overrides and the riding sit pose defer.
pub(in crate::entity_models) struct IllagerModel {
    root: ModelPart,
    family: IllagerModelFamily,
    spellcasting: bool,
}

impl IllagerModel {
    pub(in crate::entity_models) fn new(
        instance: &EntityModelInstance,
        family: IllagerModelFamily,
    ) -> Self {
        let spellcasting = illager_is_spellcasting(instance, family);
        Self {
            root: illager_tree(family, spellcasting),
            family,
            spellcasting,
        }
    }
}

impl EntityModel for IllagerModel {
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
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        apply_half_amplitude_leg_swing_named(&mut self.root, limb_swing, limb_swing_amount);
        if self.spellcasting {
            // Vanilla overwrites both separate arms' rotations with the spellcasting pose, so it
            // overrides even at rest.
            let age = render_state.age_in_ticks;
            let right = self.root.child_mut("right_arm");
            right.pose = illager_spellcast_arm_pose(right.pose, age, true);
            let left = self.root.child_mut("left_arm");
            left.pose = illager_spellcast_arm_pose(left.pose, age, false);
        } else if matches!(self.family, IllagerModelFamily::Pillager) {
            // Only the pillager renders the uncrossed (separate, swinging) arms; the idle
            // evoker/vindicator/illusioner show the static crossed `arms` part instead.
            for name in ["right_arm", "left_arm"] {
                let arm = self.root.child_mut(name);
                arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
            }
        }
    }
}
