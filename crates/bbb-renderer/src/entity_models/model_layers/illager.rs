use super::{
    apply_crossbow_charge_pose, apply_crossbow_hold_pose, apply_half_amplitude_leg_swing,
    apply_head_look, apply_humanoid_weapon_swing_down, apply_zombie_arms_held_out_named,
    humanoid_arm_swing_pose, PartPose, CROSSBOW_CHARGE_DURATION_TICKS, ILLAGER_HAT_COLOR,
    ILLAGER_ROBE, PART_POSE_ZERO,
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

/// Builds the unified illager tree, mirroring the vanilla layer choice with the vanilla `IllagerModel`
/// child names. The idle crossed layout (`uncrossed = false`) lists `head`, `body`, the folded `arms`,
/// then the legs; the `uncrossed` layout (the pillager, and any spell-casting / bow-drawing / celebrating
/// evoker/vindicator/illusioner) lists `head`, `body`, the legs, then the separate `right_arm`/`left_arm`.
/// The illusioner keeps its hatted head in both. Vanilla declaration order is preserved so the colored
/// render order stays byte-identical, while the head look, leg swing, and arm poses resolve by name.
fn illager_tree(family: IllagerModelFamily, uncrossed: bool) -> ModelPart {
    let hatted = matches!(family, IllagerModelFamily::Illusioner);
    let mut children: Vec<(&'static str, ModelPart)> =
        vec![("head", head(hatted)), ("body", body())];
    if uncrossed {
        children.extend(legs());
        children.extend(arms());
    } else {
        children.push(("arms", crossed_arms()));
        children.extend(legs());
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

/// Vanilla `IllagerModel.setupAnim` `CELEBRATING` arm pose for one separate arm (the evoker/vindicator
/// raid victory dance). Both arms hold their bind offset (`rightArm.x = -5`/`leftArm.x = 5`, `z = 0`),
/// bob `xRot = cos(ageInTicks · 0.6662) · 0.05`, and raise outward by an asymmetric roll — the right arm
/// higher (`zRot = 2.670354`), the left to `-3π/4` — with `yRot = 0`.
pub(in crate::entity_models) fn illager_celebrate_arm_pose(
    base: PartPose,
    age_in_ticks: f32,
    is_right: bool,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            (age_in_ticks * 0.6662).cos() * 0.05,
            0.0,
            if is_right {
                2.670354
            } else {
                -std::f32::consts::PI * 3.0 / 4.0
            },
        ],
    }
}

/// Vanilla `IllagerModel.setupAnim` `BOW_AND_ARROW` arm pose (the illusioner drawing its bow). Unlike
/// the symmetric `HumanoidModel`/skeleton aim, the illager braces the off (left) hand across the bow:
/// the right arm aims down the head look (`xRot = -π/2 + head.xRot`, `yRot = -0.1 + head.yRot`, `zRot`
/// preserved), the left arm holds the bow level and rolled in (`xRot = -0.9424779 + head.xRot`,
/// `yRot = head.yRot - 0.4`, `zRot = π/2`).
fn apply_illager_bow_aim(root: &mut ModelPart, head_yaw_degrees: f32, head_pitch_degrees: f32) {
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let right = root.child_mut("right_arm");
    right.pose.rotation = [
        -std::f32::consts::FRAC_PI_2 + head_pitch,
        -0.1 + head_yaw,
        right.pose.rotation[2],
    ];
    let left = root.child_mut("left_arm");
    left.pose.rotation = [
        -0.9424779 + head_pitch,
        head_yaw - 0.4,
        std::f32::consts::FRAC_PI_2,
    ];
}

/// Whether a pillager levels its crossbow this frame (vanilla `Pillager.getArmPose` returning
/// `CROSSBOW_HOLD`): it holds a crossbow and is not mid-draw (`isChargingCrossbow()`, which instead
/// takes the higher-priority `CROSSBOW_CHARGE` pull-back pose).
fn illager_is_holding_crossbow(instance: &EntityModelInstance) -> bool {
    instance.render_state.main_hand_holds_crossbow && !instance.render_state.is_charging_crossbow
}

/// The resolved illager arm pose for a frame, mirroring each family's vanilla `getArmPose()` for the
/// poses bbb renders. `Crossed` shows the static folded `arms` part; every other pose uses the uncrossed
/// separate arms. An aggressive vindicator now chops (`Attacking`) and an aggressive illusioner draws its
/// bow (`BowAndArrow`) — `is_aggressive` is projected for both. A charging pillager pulls its crossbow
/// back (the `Swing` layout's `CROSSBOW_CHARGE` sub-pose, driven by the projected draw ticks).
#[derive(Clone, Copy, PartialEq, Eq)]
enum IllagerArmPose {
    /// Folded `arms` part — vanilla `CROSSED` (idle evoker/vindicator/illusioner).
    Crossed,
    /// Pillager separate-arm walk swing, optionally overridden by the `CROSSBOW_HOLD` level.
    Swing,
    /// Evoker/illusioner `SPELLCASTING` raised arms.
    Spellcast,
    /// Illusioner `BOW_AND_ARROW` draw.
    BowAndArrow,
    /// Evoker/vindicator `CELEBRATING` victory dance.
    Celebrating,
    /// Vindicator `ATTACKING` weapon swing-down (`AnimationUtils.swingWeaponDown`, armed main hand).
    Attacking,
}

impl IllagerArmPose {
    /// Whether this pose renders the uncrossed separate-arm layout (vanilla `crossedArms = pose ==
    /// CROSSED` toggles the `arms` part off and the separate arms on for every non-`Crossed` pose).
    fn uses_separate_arms(self) -> bool {
        self != IllagerArmPose::Crossed
    }
}

/// Resolves the illager arm pose from the projected render state, mirroring each family's vanilla
/// `getArmPose()` for the supported poses. The pillager always uses the uncrossed swing layout (vanilla
/// returns HOLD/CHARGE/ATTACKING/NEUTRAL, never CROSSED); the spellcasters cast first (priority), else the
/// evoker celebrates and the illusioner draws its bow when aggressive; the vindicator chops its axe when
/// aggressive (`ATTACKING`, priority over `CELEBRATING` per vanilla), else celebrates, else stays `Crossed`.
fn resolve_illager_arm_pose(
    instance: &EntityModelInstance,
    family: IllagerModelFamily,
) -> IllagerArmPose {
    let rs = &instance.render_state;
    match family {
        IllagerModelFamily::Pillager => IllagerArmPose::Swing,
        IllagerModelFamily::Evoker => {
            if rs.illager_spellcasting {
                IllagerArmPose::Spellcast
            } else if rs.illager_celebrating {
                IllagerArmPose::Celebrating
            } else {
                IllagerArmPose::Crossed
            }
        }
        IllagerModelFamily::Illusioner => {
            if rs.illager_spellcasting {
                IllagerArmPose::Spellcast
            } else if rs.is_aggressive {
                IllagerArmPose::BowAndArrow
            } else {
                IllagerArmPose::Crossed
            }
        }
        IllagerModelFamily::Vindicator => {
            if rs.is_aggressive {
                // Vanilla `Vindicator.getArmPose`: aggressive -> ATTACKING. `IllagerModel.setupAnim`
                // later chooses the empty-hand or armed branch from the rendered main-hand item state.
                IllagerArmPose::Attacking
            } else if rs.illager_celebrating {
                IllagerArmPose::Celebrating
            } else {
                IllagerArmPose::Crossed
            }
        }
    }
}

/// Mutable illager model, mirroring vanilla `IllagerModel`/`SpellcasterIllagerModel` shared by the
/// evoker, vindicator, illusioner, and pillager. The unified tree is built for the resolved
/// [`IllagerArmPose`] with the vanilla `IllagerModel` child names. `setup_anim` looks the head
/// ([`apply_head_look`] on `head`) and swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]), then applies the resolved arm pose: the pillager swings its
/// separate arms ([`humanoid_arm_swing_pose`]) and either pulls back a charging crossbow
/// ([`apply_crossbow_charge_pose`]) or levels a held one ([`apply_crossbow_hold_pose`]);
/// a casting evoker/illusioner raises the `SPELLCASTING` arms ([`illager_spellcast_arm_pose`]); an
/// aggressive illusioner draws its bow ([`apply_illager_bow_aim`]); an aggressive vindicator either
/// reaches with empty zombie arms ([`apply_zombie_arms_held_out_named`]) or chops its axe
/// ([`apply_humanoid_weapon_swing_down`]); a celebrating evoker/vindicator dances
/// ([`illager_celebrate_arm_pose`]). Riding illagers use vanilla's fixed seated leg pose before
/// those arm-pose branches run.
pub(in crate::entity_models) struct IllagerModel {
    root: ModelPart,
    pose: IllagerArmPose,
}

impl IllagerModel {
    pub(in crate::entity_models) fn new(
        instance: &EntityModelInstance,
        family: IllagerModelFamily,
    ) -> Self {
        let pose = resolve_illager_arm_pose(instance, family);
        Self {
            root: illager_tree(family, pose.uses_separate_arms()),
            pose,
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
        if render_state.is_riding {
            apply_illager_riding_pose(&mut self.root, self.pose.uses_separate_arms());
        } else {
            apply_half_amplitude_leg_swing(&mut self.root, limb_swing, limb_swing_amount);
        }
        let age = render_state.age_in_ticks;
        match self.pose {
            // The folded `arms` part is static — vanilla never animates it (it swings the invisible
            // separate arms), so the crossed illager only swings its legs.
            IllagerArmPose::Crossed => {}
            IllagerArmPose::Swing => {
                if !render_state.is_riding {
                    for name in ["right_arm", "left_arm"] {
                        let arm = self.root.child_mut(name);
                        arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
                    }
                }
                // Vanilla `Pillager.getArmPose`: `CROSSBOW_CHARGE` (drawing) takes priority over
                // `CROSSBOW_HOLD` (holding a level crossbow). A charging pillager pulls the crossbow back
                // ([`apply_crossbow_charge_pose`] over the projected draw ticks), overwriting the walk
                // swing; otherwise a held crossbow levels along the head look.
                if render_state.is_charging_crossbow {
                    apply_crossbow_charge_pose(
                        &mut self.root,
                        CROSSBOW_CHARGE_DURATION_TICKS,
                        render_state.crossbow_charge_ticks,
                    );
                } else if illager_is_holding_crossbow(instance) {
                    apply_crossbow_hold_pose(
                        &mut self.root,
                        render_state.head_yaw,
                        render_state.head_pitch,
                    );
                }
            }
            IllagerArmPose::Spellcast => {
                // Vanilla overwrites both separate arms' rotations with the spellcasting pose.
                let right = self.root.child_mut("right_arm");
                right.pose = illager_spellcast_arm_pose(right.pose, age, true);
                let left = self.root.child_mut("left_arm");
                left.pose = illager_spellcast_arm_pose(left.pose, age, false);
            }
            IllagerArmPose::BowAndArrow => {
                apply_illager_bow_aim(
                    &mut self.root,
                    render_state.head_yaw,
                    render_state.head_pitch,
                );
            }
            IllagerArmPose::Celebrating => {
                let right = self.root.child_mut("right_arm");
                right.pose = illager_celebrate_arm_pose(right.pose, age, true);
                let left = self.root.child_mut("left_arm");
                left.pose = illager_celebrate_arm_pose(left.pose, age, false);
            }
            IllagerArmPose::Attacking => {
                // Vindicator ATTACKING: vanilla checks the rendered main-hand item. Empty hands use the
                // same held-out `animateZombieArms` as zombies; armed hands raise the weapon overhead and
                // chop with `swingWeaponDown`. IllagerModel is not a HumanoidModel, so there is no body
                // twist in either branch.
                if render_state.illager_main_hand_empty {
                    apply_zombie_arms_held_out_named(
                        &mut self.root,
                        true,
                        render_state.attack_anim,
                        age,
                    );
                } else {
                    apply_humanoid_weapon_swing_down(&mut self.root, render_state.attack_anim, age);
                }
            }
        }
    }
}

fn apply_illager_riding_pose(root: &mut ModelPart, has_separate_arms: bool) {
    use std::f32::consts::PI;

    if has_separate_arms {
        for name in ["right_arm", "left_arm"] {
            root.child_mut(name).pose.rotation = [-PI / 5.0, 0.0, 0.0];
        }
    }
    root.child_mut("right_leg").pose.rotation = [-1.4137167, PI / 10.0, 0.07853982];
    root.child_mut("left_leg").pose.rotation = [-1.4137167, -PI / 10.0, -0.07853982];
}
