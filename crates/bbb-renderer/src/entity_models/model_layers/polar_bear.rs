use super::{
    apply_head_look, apply_quadruped_leg_swing, polar_bear_head_part_index, ModelCubeDesc,
    ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc, POLAR_BEAR_WHITE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_POLAR_BEAR: &str = "minecraft:polar_bear#main";
pub(in crate::entity_models) const MODEL_LAYER_POLAR_BEAR_BABY: &str =
    "minecraft:polar_bear_baby#main";

pub(in crate::entity_models) const ADULT_POLAR_BEAR_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [7.0, 7.0, 7.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-2.5, 1.0, -6.0],
        size: [5.0, 3.0, 3.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [2.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [7.0, 7.0, 7.0],
        uv_size: [7.0, 7.0, 7.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, 1.0, -6.0],
        size: [5.0, 3.0, 3.0],
        uv_size: [5.0, 3.0, 3.0],
        tex: [0.0, 44.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [26.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [2.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [26.0, 0.0],
        mirror: true,
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-5.0, -13.0, -7.0],
        size: [14.0, 14.0, 11.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.0, -25.0, -7.0],
        size: [12.0, 12.0, 10.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-5.0, -13.0, -7.0],
        size: [14.0, 14.0, 11.0],
        uv_size: [14.0, 14.0, 11.0],
        tex: [0.0, 19.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -25.0, -7.0],
        size: [12.0, 12.0, 10.0],
        uv_size: [12.0, 12.0, 10.0],
        tex: [39.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 10.0, 8.0],
        color: POLAR_BEAR_WHITE,
    }];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_TEXTURED_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 10.0, 8.0],
        uv_size: [4.0, 10.0, 8.0],
        tex: [50.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 10.0, 6.0],
        color: POLAR_BEAR_WHITE,
    }];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_TEXTURED_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 10.0, 6.0],
        uv_size: [4.0, 10.0, 6.0],
        tex: [50.0, 40.0],
        mirror: false,
    }];

// Vanilla 26.1 ModelLayers.POLAR_BEAR: PolarBearModel.createBodyLayer()
// with LayerDefinition MeshTransformer.scaling(1.2F) applied at emission.
pub(in crate::entity_models) const ADULT_POLAR_BEAR_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 10.0, -16.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 9.0, 12.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 10.0, -16.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 9.0, 12.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_TEXTURED_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_TEXTURED_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_TEXTURED_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_TEXTURED_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.5, -6.0],
    size: [8.0, 7.0, 12.0],
    color: POLAR_BEAR_WHITE,
}];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -3.5, -6.0],
        size: [8.0, 7.0, 12.0],
        uv_size: [8.0, 7.0, 12.0],
        tex: [0.0, 9.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_POLAR_BEAR_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -2.625, -4.25],
        size: [6.0, 5.0, 4.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-2.0, 0.375, -6.25],
        size: [4.0, 2.0, 2.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [2.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-3.0, -2.625, -4.25],
        size: [6.0, 5.0, 4.0],
        uv_size: [6.0, 5.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, 0.375, -6.25],
        size: [4.0, 2.0, 2.0],
        uv_size: [4.0, 2.0, 2.0],
        tex: [20.0, 3.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [20.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [2.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [26.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    color: POLAR_BEAR_WHITE,
}];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_RIGHT_HIND_LEG:
    [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    uv_size: [3.0, 3.0, 3.0],
    tex: [0.0, 34.0],
    mirror: false,
}];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    uv_size: [3.0, 3.0, 3.0],
    tex: [12.0, 34.0],
    mirror: false,
}];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_RIGHT_FRONT_LEG:
    [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    uv_size: [3.0, 3.0, 3.0],
    tex: [0.0, 28.0],
    mirror: false,
}];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_LEFT_FRONT_LEG:
    [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    uv_size: [3.0, 3.0, 3.0],
    tex: [12.0, 28.0],
    mirror: false,
}];

// Vanilla 26.1 ModelLayers.POLAR_BEAR_BABY: BabyPolarBearModel.createBodyLayer().
pub(in crate::entity_models) const BABY_POLAR_BEAR_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.625, -5.75],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.625, -5.75],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
];

/// Which polar bear model part a standing-pose delta applies to. Vanilla
/// `PolarBearModel.setupAnim` rears the bear by moving the head, body, and both
/// front legs; the hind legs stay put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum PolarBearStandPart {
    Head,
    Body,
    FrontLeg,
}

/// Maps each standing-pose part to its index in the adult/baby body layer. The
/// adult layer (`PolarBearModel.createBodyLayer`) lists the head first then the
/// body; the baby layer (`BabyPolarBearModel.createBodyLayer`) lists the body
/// first then the head. Both front legs are indices 4 and 5.
pub(in crate::entity_models) const fn polar_bear_standing_part_roles(
    baby: bool,
) -> [(usize, PolarBearStandPart); 4] {
    if baby {
        [
            (1, PolarBearStandPart::Head),
            (0, PolarBearStandPart::Body),
            (4, PolarBearStandPart::FrontLeg),
            (5, PolarBearStandPart::FrontLeg),
        ]
    } else {
        [
            (0, PolarBearStandPart::Head),
            (1, PolarBearStandPart::Body),
            (4, PolarBearStandPart::FrontLeg),
            (5, PolarBearStandPart::FrontLeg),
        ]
    }
}

/// Applies the vanilla `PolarBearModel.setupAnim` standing delta to one part
/// pose. `stand_scale` is `PolarBear.getStandingAnimationScale`; vanilla squares
/// it into `standScale`. `bodyAgeScale` (`state.ageScale`, 1.0 adult / 0.5 baby)
/// scales only the body and front-leg translation terms. Both front legs share
/// the same base y/z, so applying the delta to each matches vanilla's
/// `leftFrontLeg.y = rightFrontLeg.y` / `leftFrontLeg.z = rightFrontLeg.z`.
pub(in crate::entity_models) fn apply_polar_bear_standing_pose(
    pose: &mut PartPose,
    part: PolarBearStandPart,
    baby: bool,
    stand_scale: f32,
) {
    let scale = stand_scale * stand_scale;
    let body_age_scale = if baby { 0.5 } else { 1.0 };
    match part {
        PolarBearStandPart::Body => {
            pose.rotation[0] -= scale * std::f32::consts::PI * 0.35;
            pose.offset[1] += scale * body_age_scale * 2.0;
        }
        PolarBearStandPart::FrontLeg => {
            pose.offset[1] -= scale * body_age_scale * 20.0;
            pose.offset[2] += scale * body_age_scale * 4.0;
            pose.rotation[0] -= scale * std::f32::consts::PI * 0.45;
        }
        PolarBearStandPart::Head => {
            pose.offset[1] -= scale * 24.0;
            pose.offset[2] += scale * 13.0;
            pose.rotation[0] += scale * std::f32::consts::PI * 0.15;
        }
    }
}

/// Selects the unified polar bear part-tree pair (colored + textured) for `baby`.
fn polar_bear_part_trees(
    baby: bool,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&BABY_POLAR_BEAR_PARTS, &BABY_POLAR_BEAR_TEXTURED_PARTS)
    } else {
        (&ADULT_POLAR_BEAR_PARTS, &ADULT_POLAR_BEAR_TEXTURED_PARTS)
    }
}

/// Mutable polar bear model, mirroring vanilla `PolarBearModel` (a `QuadrupedModel`). The unified tree
/// is zipped from the baked colored and textured trees for the selected `baby` layout
/// ([`polar_bear_part_trees`]). `setup_anim` runs the `QuadrupedModel` head look ([`apply_head_look`])
/// and four-leg swing ([`apply_quadruped_leg_swing`]), then — when standing — adds the rear-up deltas on
/// top ([`apply_polar_bear_standing_pose`] over [`polar_bear_standing_part_roles`]), driven by the
/// projected `standScale`. The per-size scale (adult only) lives in the root transform.
pub(in crate::entity_models) struct PolarBearModel {
    root: ModelPart,
    baby: bool,
}

impl PolarBearModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let (colored, textured) = polar_bear_part_trees(baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            baby,
        }
    }
}

impl EntityModel for PolarBearModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root
                .child_at_mut(polar_bear_head_part_index(self.baby)),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            [2, 3, 4, 5],
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
        let stand_scale = render_state.polar_bear_stand_scale;
        if stand_scale != 0.0 {
            for (index, part) in polar_bear_standing_part_roles(self.baby) {
                apply_polar_bear_standing_pose(
                    &mut self.root.child_at_mut(index).pose,
                    part,
                    self.baby,
                    stand_scale,
                );
            }
        }
    }
}
