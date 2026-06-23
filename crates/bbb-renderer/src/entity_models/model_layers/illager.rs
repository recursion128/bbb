use super::{
    apply_half_amplitude_leg_swing, apply_head_look, humanoid_arm_swing_pose,
    villager_head_part_index, ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc,
    TexturedModelPartDesc, ILLAGER_HAT_COLOR, ILLAGER_ROBE, PART_POSE_ZERO,
};
use crate::entity_models::catalog::IllagerModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_EVOKER: &str = "minecraft:evoker#main";
pub(in crate::entity_models) const MODEL_LAYER_ILLUSIONER: &str = "minecraft:illusioner#main";
pub(in crate::entity_models) const MODEL_LAYER_PILLAGER: &str = "minecraft:pillager#main";
pub(in crate::entity_models) const MODEL_LAYER_VINDICATOR: &str = "minecraft:vindicator#main";

pub(in crate::entity_models) const ILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.45, -10.45, -4.45],
    size: [8.9, 12.9, 8.9],
    color: ILLAGER_HAT_COLOR,
}];

pub(in crate::entity_models) const ILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        color: ILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARMS: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: ILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ILLAGER_LEFT_SHOULDER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [4.0, -2.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(in crate::entity_models) const ILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_NOSE,
    children: &[],
}];

pub(in crate::entity_models) const ILLAGER_HEAD_WITH_HAT_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_NOSE,
        children: &[],
    },
];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_LEFT_SHOULDER,
        children: &[],
    }];

pub(in crate::entity_models) const ILLAGER_CROSSED_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_CROSSED_ARMS,
    children: &ILLAGER_CROSSED_ARM_CHILDREN,
};

pub(in crate::entity_models) const ILLAGER_RIGHT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_RIGHT_ARM,
    children: &[],
};

pub(in crate::entity_models) const ILLAGER_LEFT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_LEFT_ARM,
    children: &[],
};

// Vanilla 26.1 IllagerModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(in crate::entity_models) const ILLAGER_SHARED_CROSSED_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ILLAGER_SHARED_UNCROSSED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ILLAGER_RIGHT_ARM_PART,
    ILLAGER_LEFT_ARM_PART,
];

// The illusioner's spellcasting layout: the same uncrossed (separate-arm) body as the shared
// uncrossed parts, but keeping the illusioner's hatted head. Used when `isCastingSpell` hides the
// crossed `arms` part and raises the two separate arms.
pub(in crate::entity_models) const ILLAGER_ILLUSIONER_UNCROSSED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_WITH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ILLAGER_RIGHT_ARM_PART,
    ILLAGER_LEFT_ARM_PART,
];

pub(in crate::entity_models) const ILLAGER_ILLUSIONER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_WITH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

// Textured `IllagerModel.createBodyLayer` (64x64 UVs). The deformed cubes (the hat, the body's
// robe overlay) inflate their geometry but keep the base box as `uv_size`, exactly like
// `CubeDeformation` in vanilla `addBox`. The geometry (min/size) matches the colored cubes above,
// so both render paths share the same mesh.
const fn illager_textured_cube(
    min: [f32; 3],
    size: [f32; 3],
    uv_size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min,
        size,
        uv_size,
        tex,
        mirror,
    }
}

const fn illager_textured_part(
    offset: [f32; 3],
    cubes: &'static [TexturedModelCubeDesc],
    children: &'static [TexturedModelPartDesc],
) -> TexturedModelPartDesc {
    TexturedModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children,
    }
}

const ILLAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-4.0, -10.0, -4.0],
    [8.0, 10.0, 8.0],
    [8.0, 10.0, 8.0],
    [0.0, 0.0],
    false,
)];
const ILLAGER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-4.45, -10.45, -4.45],
    [8.9, 12.9, 8.9],
    [8.0, 12.0, 8.0],
    [32.0, 0.0],
    false,
)];
const ILLAGER_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-1.0, -1.0, -6.0],
    [2.0, 4.0, 2.0],
    [2.0, 4.0, 2.0],
    [24.0, 0.0],
    false,
)];
const ILLAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    illager_textured_cube(
        [-4.0, 0.0, -3.0],
        [8.0, 12.0, 6.0],
        [8.0, 12.0, 6.0],
        [16.0, 20.0],
        false,
    ),
    illager_textured_cube(
        [-4.5, -0.5, -3.5],
        [9.0, 21.0, 7.0],
        [8.0, 20.0, 6.0],
        [0.0, 38.0],
        false,
    ),
];
const ILLAGER_TEXTURED_CROSSED_ARMS: [TexturedModelCubeDesc; 2] = [
    illager_textured_cube(
        [-8.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        false,
    ),
    illager_textured_cube(
        [-4.0, 2.0, -2.0],
        [8.0, 4.0, 4.0],
        [8.0, 4.0, 4.0],
        [40.0, 38.0],
        false,
    ),
];
const ILLAGER_TEXTURED_LEFT_SHOULDER: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [4.0, -2.0, -2.0],
    [4.0, 8.0, 4.0],
    [4.0, 8.0, 4.0],
    [44.0, 22.0],
    true,
)];
const ILLAGER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    false,
)];
const ILLAGER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    true,
)];
const ILLAGER_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 46.0],
    false,
)];
const ILLAGER_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [illager_textured_cube(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 46.0],
    true,
)];

const ILLAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] = [illager_textured_part(
    [0.0, -2.0, 0.0],
    &ILLAGER_TEXTURED_NOSE,
    &[],
)];
const ILLAGER_TEXTURED_HEAD_WITH_HAT_CHILDREN: [TexturedModelPartDesc; 2] = [
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_HAT, &[]),
    illager_textured_part([0.0, -2.0, 0.0], &ILLAGER_TEXTURED_NOSE, &[]),
];
const ILLAGER_TEXTURED_CROSSED_ARM_CHILDREN: [TexturedModelPartDesc; 1] = [illager_textured_part(
    [0.0, 0.0, 0.0],
    &ILLAGER_TEXTURED_LEFT_SHOULDER,
    &[],
)];
const ILLAGER_TEXTURED_CROSSED_ARM_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_TEXTURED_CROSSED_ARMS,
    children: &ILLAGER_TEXTURED_CROSSED_ARM_CHILDREN,
};
const ILLAGER_TEXTURED_RIGHT_ARM_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_TEXTURED_RIGHT_ARM,
    children: &[],
};
const ILLAGER_TEXTURED_LEFT_ARM_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_TEXTURED_LEFT_ARM,
    children: &[],
};

pub(in crate::entity_models) const ILLAGER_TEXTURED_CROSSED_PARTS: [TexturedModelPartDesc; 5] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    ILLAGER_TEXTURED_CROSSED_ARM_PART,
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
];

pub(in crate::entity_models) const ILLAGER_TEXTURED_ILLUSIONER_PARTS: [TexturedModelPartDesc; 5] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_WITH_HAT_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    ILLAGER_TEXTURED_CROSSED_ARM_PART,
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
];

pub(in crate::entity_models) const ILLAGER_TEXTURED_UNCROSSED_PARTS: [TexturedModelPartDesc; 6] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
    ILLAGER_TEXTURED_RIGHT_ARM_PART,
    ILLAGER_TEXTURED_LEFT_ARM_PART,
];

// The illusioner's textured spellcasting layout: the uncrossed (separate-arm) body with the
// illusioner's hatted head. Mirrors `ILLAGER_ILLUSIONER_UNCROSSED_PARTS` on the textured path.
pub(in crate::entity_models) const ILLAGER_TEXTURED_ILLUSIONER_UNCROSSED_PARTS:
    [TexturedModelPartDesc; 6] = [
    illager_textured_part(
        [0.0, 0.0, 0.0],
        &ILLAGER_TEXTURED_HEAD,
        &ILLAGER_TEXTURED_HEAD_WITH_HAT_CHILDREN,
    ),
    illager_textured_part([0.0, 0.0, 0.0], &ILLAGER_TEXTURED_BODY, &[]),
    illager_textured_part([-2.0, 12.0, 0.0], &ILLAGER_TEXTURED_RIGHT_LEG, &[]),
    illager_textured_part([2.0, 12.0, 0.0], &ILLAGER_TEXTURED_LEFT_LEG, &[]),
    ILLAGER_TEXTURED_RIGHT_ARM_PART,
    ILLAGER_TEXTURED_LEFT_ARM_PART,
];

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

/// Right/left leg part indices in an illager body layer. The crossed-arms layouts (idle
/// evoker/vindicator/illusioner) carry the combined crossed `arms` part at slot `2` and list the
/// legs at `[3, 4]`; the uncrossed pillager / spellcasting layout lists the legs at `[2, 3]` before
/// its two separate arms at `[4, 5]`. [`half_amplitude_leg_swing_pose`] resolves each leg's phase
/// from its offset, so only the slot positions differ.
fn illager_leg_part_indices(crossed: bool) -> [usize; 2] {
    if crossed {
        [3, 4]
    } else {
        [2, 3]
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

/// Selects the colored and textured const trees for an illager by `family` and whether it is mid
/// spell-cast. Idle evoker/vindicator show the static crossed `arms` layout; the pillager (and any
/// spellcasting evoker/illusioner) use the uncrossed separate-arm layout. The illusioner keeps its
/// hatted head in both. Zipped into the unified tree by [`IllagerModel::new`].
pub(in crate::entity_models) fn illager_part_trees(
    family: IllagerModelFamily,
    spellcasting: bool,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if spellcasting {
        return match family {
            IllagerModelFamily::Illusioner => (
                &ILLAGER_ILLUSIONER_UNCROSSED_PARTS,
                &ILLAGER_TEXTURED_ILLUSIONER_UNCROSSED_PARTS,
            ),
            _ => (
                &ILLAGER_SHARED_UNCROSSED_PARTS,
                &ILLAGER_TEXTURED_UNCROSSED_PARTS,
            ),
        };
    }
    match family {
        IllagerModelFamily::Evoker | IllagerModelFamily::Vindicator => (
            &ILLAGER_SHARED_CROSSED_PARTS,
            &ILLAGER_TEXTURED_CROSSED_PARTS,
        ),
        IllagerModelFamily::Illusioner => (
            &ILLAGER_ILLUSIONER_PARTS,
            &ILLAGER_TEXTURED_ILLUSIONER_PARTS,
        ),
        IllagerModelFamily::Pillager => (
            &ILLAGER_SHARED_UNCROSSED_PARTS,
            &ILLAGER_TEXTURED_UNCROSSED_PARTS,
        ),
    }
}

/// Mutable illager model, mirroring vanilla `IllagerModel`/`SpellcasterIllagerModel` shared by the
/// evoker, vindicator, illusioner, and pillager. The unified tree is zipped from the colored and
/// textured const trees selected by `family`/`spellcasting` ([`illager_part_trees`]). `setup_anim`
/// looks the head ([`apply_head_look`]) and swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]); the pillager additionally swings its separate arms at the
/// `HumanoidModel` amplitude ([`humanoid_arm_swing_pose`]), while a spellcasting evoker/illusioner
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
        let (colored, textured) = illager_part_trees(family, spellcasting);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
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
            self.root.child_at_mut(villager_head_part_index(false)),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        // The idle crossed-arms layout (evoker/vindicator/illusioner) lists the legs at `[3, 4]`; the
        // uncrossed pillager / spellcasting layout lists them at `[2, 3]`.
        let crossed = !self.spellcasting && !matches!(self.family, IllagerModelFamily::Pillager);
        apply_half_amplitude_leg_swing(
            &mut self.root,
            illager_leg_part_indices(crossed),
            limb_swing,
            limb_swing_amount,
        );
        if self.spellcasting {
            // Vanilla overwrites both separate arms' rotations with the spellcasting pose (arms
            // `[4, 5]` are right then left), so it overrides even at rest.
            let age = render_state.age_in_ticks;
            let right = self.root.child_at_mut(4);
            right.pose = illager_spellcast_arm_pose(right.pose, age, true);
            let left = self.root.child_at_mut(5);
            left.pose = illager_spellcast_arm_pose(left.pose, age, false);
        } else if matches!(self.family, IllagerModelFamily::Pillager) {
            // Only the pillager renders the uncrossed (separate, swinging) arms at `[4, 5]`; the
            // idle evoker/vindicator/illusioner show the static crossed `arms` part instead.
            for index in [4, 5] {
                let arm = self.root.child_at_mut(index);
                arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
            }
        }
    }
}
