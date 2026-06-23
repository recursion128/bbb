use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, PART_POSE_ZERO};
use crate::entity_models::catalog::ArmorStandModelPose;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const ARMOR_STAND_WOOD: [f32; 4] = [0.55, 0.36, 0.19, 1.0];

pub(in crate::entity_models) const ARMOR_STAND_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -7.0, -1.0],
    size: [2.0, 7.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, 0.0, -1.5],
    size: [12.0, 3.0, 3.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 11.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_RIGHT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, 3.0, -1.0],
        size: [2.0, 7.0, 2.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const ARMOR_STAND_LEFT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [1.0, 3.0, -1.0],
        size: [2.0, 7.0, 2.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const ARMOR_STAND_SHOULDER_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-4.0, 10.0, -1.0],
        size: [8.0, 2.0, 2.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const ARMOR_STAND_BASE_PLATE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, 11.0, -6.0],
    size: [12.0, 1.0, 12.0],
    color: ARMOR_STAND_WOOD,
}];

// Vanilla 26.1 ArmorStandModel.createBodyLayer().
pub(in crate::entity_models) const ARMOR_STAND_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_RIGHT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_LEFT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_SHOULDER_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_BASE_PLATE,
        children: &[],
    },
];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.75, -5.25, -0.75],
    size: [1.5, 5.25, 1.5],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -0.75],
    size: [6.0, 1.5, 1.5],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_RIGHT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [1.0, 6.0, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_LEFT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, -1.0, -0.5],
        size: [1.0, 6.0, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.0, -0.5],
    size: [1.0, 5.5, 1.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_RIGHT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.5, 1.5, -0.5],
        size: [1.0, 3.5, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_LEFT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.5, 1.5, -0.5],
        size: [1.0, 3.5, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_SHOULDER_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 5.0, -0.5],
        size: [4.0, 1.0, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_BASE_PLATE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, 5.5, -3.0],
        size: [6.0, 0.5, 6.0],
        color: ARMOR_STAND_WOOD,
    }];

// Vanilla 26.1 ModelLayers.ARMOR_STAND_SMALL applies HumanoidModel.BABY_TRANSFORMER:
// head root parts are translated by y=16 then scaled 0.75; all other root parts
// are translated by y=24 then scaled 0.5.
pub(in crate::entity_models) const SMALL_ARMOR_STAND_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-0.95, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.95, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_RIGHT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEFT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_SHOULDER_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_BASE_PLATE,
        children: &[],
    },
];

/// The vanilla 26.1 `ArmorStandModel.createBodyLayer` `texOffs`/box for each part, in the
/// `ARMOR_STAND_PARTS` order. `uv_size` is the full-model box: the small layer is the same
/// mesh scaled by `HumanoidModel.BABY_TRANSFORMER`, which only moves vertices, so the small
/// cart samples the identical texture region as the full one.
#[derive(Clone, Copy)]
pub(in crate::entity_models) struct ArmorStandPartUv {
    pub tex: [f32; 2],
    pub uv_size: [f32; 3],
    pub mirror: bool,
}

const fn armor_stand_uv(tex: [f32; 2], uv_size: [f32; 3], mirror: bool) -> ArmorStandPartUv {
    ArmorStandPartUv {
        tex,
        uv_size,
        mirror,
    }
}

pub(in crate::entity_models) const ARMOR_STAND_PART_UVS: [ArmorStandPartUv; 10] = [
    armor_stand_uv([0.0, 0.0], [2.0, 7.0, 2.0], false), // head
    armor_stand_uv([0.0, 26.0], [12.0, 3.0, 3.0], false), // body
    armor_stand_uv([24.0, 0.0], [2.0, 12.0, 2.0], false), // right_arm
    armor_stand_uv([32.0, 16.0], [2.0, 12.0, 2.0], true), // left_arm (mirror)
    armor_stand_uv([8.0, 0.0], [2.0, 11.0, 2.0], false), // right_leg
    armor_stand_uv([40.0, 16.0], [2.0, 11.0, 2.0], true), // left_leg (mirror)
    armor_stand_uv([16.0, 0.0], [2.0, 7.0, 2.0], false), // right_body_stick
    armor_stand_uv([48.0, 16.0], [2.0, 7.0, 2.0], false), // left_body_stick
    armor_stand_uv([0.0, 48.0], [8.0, 2.0, 2.0], false), // shoulder_stick
    armor_stand_uv([0.0, 32.0], [12.0, 1.0, 12.0], false), // base_plate
];

/// Vanilla `ArmorStandModel.createBodyLayer` child names, in the `ARMOR_STAND_PARTS` /
/// `SMALL_ARMOR_STAND_PARTS` order. The first six are the shared `HumanoidModel` bones (`head`,
/// `body`, the arms/legs); the last four are the armor stand's wooden decorations. `setup_anim`
/// resolves each by name via `child_mut`.
pub(in crate::entity_models) const ARMOR_STAND_PART_NAMES: [&str; 10] = [
    "head",
    "body",
    "right_arm",
    "left_arm",
    "right_leg",
    "left_leg",
    "right_body_stick",
    "left_body_stick",
    "shoulder_stick",
    "base_plate",
];

/// Builds the textured cube for an armor-stand part: the geometry (`min`/`size`) comes from
/// the shared colored part (so the colored and textured meshes are identical), while the UV
/// source comes from the full-model `ArmorStandPartUv`.
pub(in crate::entity_models) fn armor_stand_textured_cube(
    part: &ModelPartDesc,
    uv: ArmorStandPartUv,
) -> TexturedModelCubeDesc {
    let cube = part.cubes[0];
    TexturedModelCubeDesc {
        min: cube.min,
        size: cube.size,
        uv_size: uv.uv_size,
        tex: uv.tex,
        mirror: uv.mirror,
    }
}

fn degrees_to_radians3(rotation: [f32; 3]) -> [f32; 3] {
    [
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    ]
}

/// Mutable armor-stand model, mirroring vanilla `ArmorStandModel`. The ten parts hang off a synthetic
/// root as named children ([`ARMOR_STAND_PART_NAMES`]); each unified cube takes its geometry/color from
/// the shared colored part and its UV from the matching [`ARMOR_STAND_PART_UVS`] row, so one tree drives
/// both render paths. `new` selects the small or full layer; `setup_anim` poses each part by name from
/// the synced [`ArmorStandModelPose`] (degrees), hides the arms / base plate by visibility (`showArms` /
/// `showBasePlate`), and yaws the base plate by `-bodyRot`. The body, both body sticks, and the shoulder
/// stick all share the body pose.
pub(in crate::entity_models) struct ArmorStandModel {
    root: ModelPart,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
}

impl ArmorStandModel {
    pub(in crate::entity_models) fn new(
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    ) -> Self {
        let parts = if small {
            &SMALL_ARMOR_STAND_PARTS
        } else {
            &ARMOR_STAND_PARTS
        };
        let children = parts
            .iter()
            .zip(ARMOR_STAND_PART_UVS.iter())
            .zip(ARMOR_STAND_PART_NAMES.iter())
            .map(|((part, uv), &name)| {
                // Geometry/color from the shared colored part; UV from the matching row (the same
                // `armor_stand_textured_cube` mapping the textured tests assert against).
                let cube = part.cubes[0];
                let textured = armor_stand_textured_cube(part, *uv);
                (
                    name,
                    ModelPart::leaf(
                        part.pose,
                        vec![ModelCube::new(
                            cube.min,
                            cube.size,
                            cube.color,
                            textured.uv_size,
                            textured.tex,
                            textured.mirror,
                        )],
                    ),
                )
            })
            .collect();
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
            show_arms,
            show_base_plate,
            pose,
        }
    }
}

impl EntityModel for ArmorStandModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let pose = self.pose;
        let body = degrees_to_radians3(pose.body);
        self.root.child_mut("head").pose.rotation = degrees_to_radians3(pose.head);
        self.root.child_mut("body").pose.rotation = body;
        if self.show_arms {
            self.root.child_mut("right_arm").pose.rotation = degrees_to_radians3(pose.right_arm);
            self.root.child_mut("left_arm").pose.rotation = degrees_to_radians3(pose.left_arm);
        } else {
            self.root.child_mut("right_arm").visible = false;
            self.root.child_mut("left_arm").visible = false;
        }
        self.root.child_mut("right_leg").pose.rotation = degrees_to_radians3(pose.right_leg);
        self.root.child_mut("left_leg").pose.rotation = degrees_to_radians3(pose.left_leg);
        // The two body sticks and the shoulder stick share the body pose.
        self.root.child_mut("right_body_stick").pose.rotation = body;
        self.root.child_mut("left_body_stick").pose.rotation = body;
        self.root.child_mut("shoulder_stick").pose.rotation = body;
        if self.show_base_plate {
            self.root.child_mut("base_plate").pose.rotation =
                [0.0, -instance.render_state.body_rot.to_radians(), 0.0];
        } else {
            self.root.child_mut("base_plate").visible = false;
        }
    }
}
