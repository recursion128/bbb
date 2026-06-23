use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc, BOAT_WOOD,
};
use crate::entity_models::catalog::BoatModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ACACIA_BOAT: &str = "minecraft:boat/acacia#main";
pub(in crate::entity_models) const MODEL_LAYER_ACACIA_CHEST_BOAT: &str =
    "minecraft:chest_boat/acacia#main";
pub(in crate::entity_models) const MODEL_LAYER_BAMBOO_RAFT: &str = "minecraft:boat/bamboo#main";
pub(in crate::entity_models) const MODEL_LAYER_BAMBOO_CHEST_RAFT: &str =
    "minecraft:chest_boat/bamboo#main";
pub(in crate::entity_models) const MODEL_LAYER_BIRCH_BOAT: &str = "minecraft:boat/birch#main";
pub(in crate::entity_models) const MODEL_LAYER_BIRCH_CHEST_BOAT: &str =
    "minecraft:chest_boat/birch#main";
pub(in crate::entity_models) const MODEL_LAYER_CHERRY_BOAT: &str = "minecraft:boat/cherry#main";
pub(in crate::entity_models) const MODEL_LAYER_CHERRY_CHEST_BOAT: &str =
    "minecraft:chest_boat/cherry#main";
pub(in crate::entity_models) const MODEL_LAYER_DARK_OAK_BOAT: &str = "minecraft:boat/dark_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_DARK_OAK_CHEST_BOAT: &str =
    "minecraft:chest_boat/dark_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_JUNGLE_BOAT: &str = "minecraft:boat/jungle#main";
pub(in crate::entity_models) const MODEL_LAYER_JUNGLE_CHEST_BOAT: &str =
    "minecraft:chest_boat/jungle#main";
pub(in crate::entity_models) const MODEL_LAYER_MANGROVE_BOAT: &str = "minecraft:boat/mangrove#main";
pub(in crate::entity_models) const MODEL_LAYER_MANGROVE_CHEST_BOAT: &str =
    "minecraft:chest_boat/mangrove#main";
pub(in crate::entity_models) const MODEL_LAYER_OAK_BOAT: &str = "minecraft:boat/oak#main";
pub(in crate::entity_models) const MODEL_LAYER_OAK_CHEST_BOAT: &str =
    "minecraft:chest_boat/oak#main";
pub(in crate::entity_models) const MODEL_LAYER_PALE_OAK_BOAT: &str = "minecraft:boat/pale_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_PALE_OAK_CHEST_BOAT: &str =
    "minecraft:chest_boat/pale_oak#main";
pub(in crate::entity_models) const MODEL_LAYER_SPRUCE_BOAT: &str = "minecraft:boat/spruce#main";
pub(in crate::entity_models) const MODEL_LAYER_SPRUCE_CHEST_BOAT: &str =
    "minecraft:chest_boat/spruce#main";

pub(in crate::entity_models) const BOAT_BOTTOM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-14.0, -9.0, -3.0],
    size: [28.0, 16.0, 3.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_BACK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, -7.0, -1.0],
    size: [18.0, 6.0, 2.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_FRONT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -7.0, -1.0],
    size: [16.0, 6.0, 2.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_SIDE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-14.0, -7.0, -1.0],
    size: [28.0, 6.0, 2.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_LEFT_PADDLE: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        color: BOAT_WOOD,
    },
    ModelCubeDesc {
        min: [-1.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        color: BOAT_WOOD,
    },
];

pub(in crate::entity_models) const BOAT_RIGHT_PADDLE: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        color: BOAT_WOOD,
    },
    ModelCubeDesc {
        min: [0.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        color: BOAT_WOOD,
    },
];

pub(in crate::entity_models) const BOAT_CHEST_BOTTOM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [12.0, 8.0, 12.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_CHEST_LID: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [12.0, 4.0, 12.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const BOAT_CHEST_LOCK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [2.0, 4.0, 1.0],
    color: BOAT_WOOD,
}];

pub(in crate::entity_models) const RAFT_BOTTOM: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-14.0, -11.0, -4.0],
        size: [28.0, 20.0, 4.0],
        color: BOAT_WOOD,
    },
    ModelCubeDesc {
        min: [-14.0, -9.0, -8.0],
        size: [28.0, 16.0, 4.0],
        color: BOAT_WOOD,
    },
];

pub(in crate::entity_models) const BOAT_COMMON_PARTS: [ModelPartDesc; 7] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, 1.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &BOAT_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-15.0, 4.0, 4.0],
            rotation: [0.0, std::f32::consts::PI * 1.5, 0.0],
        },
        cubes: &BOAT_BACK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [15.0, 4.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_FRONT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -9.0],
            rotation: [0.0, std::f32::consts::PI, 0.0],
        },
        cubes: &BOAT_SIDE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 9.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BOAT_SIDE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -5.0, 9.0],
            rotation: [0.0, 0.0, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_LEFT_PADDLE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -5.0, -9.0],
            rotation: [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const BOAT_CHEST_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -5.0, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -9.0, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LID,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, -6.0, -1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LOCK,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_COMMON_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.1, 1.0],
            rotation: [1.5708, 0.0, 0.0],
        },
        cubes: &RAFT_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -4.0, 9.0],
            rotation: [0.0, 0.0, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_LEFT_PADDLE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, -4.0, -9.0],
            rotation: [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        },
        cubes: &BOAT_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_CHEST_PARTS: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -10.1, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_BOTTOM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -14.1, -6.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LID,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, -11.1, -1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &BOAT_CHEST_LOCK,
        children: &[],
    },
];

pub(in crate::entity_models) const BOAT_TEXTURED_BOTTOM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-14.0, -9.0, -3.0],
        size: [28.0, 16.0, 3.0],
        uv_size: [28.0, 16.0, 3.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_BACK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-13.0, -7.0, -1.0],
        size: [18.0, 6.0, 2.0],
        uv_size: [18.0, 6.0, 2.0],
        tex: [0.0, 19.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_FRONT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, -7.0, -1.0],
        size: [16.0, 6.0, 2.0],
        uv_size: [16.0, 6.0, 2.0],
        tex: [0.0, 27.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_RIGHT_SIDE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-14.0, -7.0, -1.0],
        size: [28.0, 6.0, 2.0],
        uv_size: [28.0, 6.0, 2.0],
        tex: [0.0, 35.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_LEFT_SIDE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-14.0, -7.0, -1.0],
        size: [28.0, 6.0, 2.0],
        uv_size: [28.0, 6.0, 2.0],
        tex: [0.0, 43.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_LEFT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [62.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [62.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BOAT_TEXTURED_RIGHT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [62.0, 20.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [62.0, 20.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_BOTTOM: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-14.0, -11.0, -4.0],
        size: [28.0, 20.0, 4.0],
        uv_size: [28.0, 20.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-14.0, -9.0, -8.0],
        size: [28.0, 16.0, 4.0],
        uv_size: [28.0, 16.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_LEFT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [0.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [0.0, 24.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_RIGHT_PADDLE: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -5.0],
        size: [2.0, 2.0, 18.0],
        uv_size: [2.0, 2.0, 18.0],
        tex: [40.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.001, -3.0, 8.0],
        size: [1.0, 6.0, 7.0],
        uv_size: [1.0, 6.0, 7.0],
        tex: [40.0, 24.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BOAT_TEXTURED_CHEST_BOTTOM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [12.0, 8.0, 12.0],
        uv_size: [12.0, 8.0, 12.0],
        tex: [0.0, 76.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_CHEST_LID: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [12.0, 4.0, 12.0],
        uv_size: [12.0, 4.0, 12.0],
        tex: [0.0, 59.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_CHEST_LOCK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [2.0, 4.0, 1.0],
        uv_size: [2.0, 4.0, 1.0],
        tex: [0.0, 59.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BOAT_TEXTURED_PARTS: [TexturedModelPartDesc; 7] = [
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[0].pose,
        cubes: &BOAT_TEXTURED_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[1].pose,
        cubes: &BOAT_TEXTURED_BACK,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[2].pose,
        cubes: &BOAT_TEXTURED_FRONT,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[3].pose,
        cubes: &BOAT_TEXTURED_RIGHT_SIDE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[4].pose,
        cubes: &BOAT_TEXTURED_LEFT_SIDE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[5].pose,
        cubes: &BOAT_TEXTURED_LEFT_PADDLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_COMMON_PARTS[6].pose,
        cubes: &BOAT_TEXTURED_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const BOAT_CHEST_TEXTURED_PARTS: [TexturedModelPartDesc; 10] = [
    BOAT_TEXTURED_PARTS[0],
    BOAT_TEXTURED_PARTS[1],
    BOAT_TEXTURED_PARTS[2],
    BOAT_TEXTURED_PARTS[3],
    BOAT_TEXTURED_PARTS[4],
    BOAT_TEXTURED_PARTS[5],
    BOAT_TEXTURED_PARTS[6],
    TexturedModelPartDesc {
        pose: BOAT_CHEST_PARTS[0].pose,
        cubes: &BOAT_TEXTURED_CHEST_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_CHEST_PARTS[1].pose,
        cubes: &BOAT_TEXTURED_CHEST_LID,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BOAT_CHEST_PARTS[2].pose,
        cubes: &BOAT_TEXTURED_CHEST_LOCK,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_TEXTURED_PARTS: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: RAFT_COMMON_PARTS[0].pose,
        cubes: &RAFT_TEXTURED_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_COMMON_PARTS[1].pose,
        cubes: &RAFT_TEXTURED_LEFT_PADDLE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_COMMON_PARTS[2].pose,
        cubes: &RAFT_TEXTURED_RIGHT_PADDLE,
        children: &[],
    },
];

pub(in crate::entity_models) const RAFT_CHEST_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    RAFT_TEXTURED_PARTS[0],
    RAFT_TEXTURED_PARTS[1],
    RAFT_TEXTURED_PARTS[2],
    TexturedModelPartDesc {
        pose: RAFT_CHEST_PARTS[0].pose,
        cubes: &BOAT_TEXTURED_CHEST_BOTTOM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_CHEST_PARTS[1].pose,
        cubes: &BOAT_TEXTURED_CHEST_LID,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: RAFT_CHEST_PARTS[2].pose,
        cubes: &BOAT_TEXTURED_CHEST_LOCK,
        children: &[],
    },
];

/// Selects the colored common / chest part lists and the matching textured combined tree for a boat or
/// raft, with or without a chest. The bamboo family is the raft; every other family is the plain boat.
/// The textured chest trees ([`BOAT_CHEST_TEXTURED_PARTS`] / [`RAFT_CHEST_TEXTURED_PARTS`]) already
/// concatenate the common and chest parts, so they zip 1:1 with the chained colored parts.
fn boat_part_trees(
    family: BoatModelFamily,
    chest: bool,
) -> (
    &'static [ModelPartDesc],
    &'static [ModelPartDesc],
    &'static [TexturedModelPartDesc],
) {
    let raft = family == BoatModelFamily::Bamboo;
    match (raft, chest) {
        (true, false) => (&RAFT_COMMON_PARTS, &[], &RAFT_TEXTURED_PARTS),
        (true, true) => (
            &RAFT_COMMON_PARTS,
            &RAFT_CHEST_PARTS,
            &RAFT_CHEST_TEXTURED_PARTS,
        ),
        (false, false) => (&BOAT_COMMON_PARTS, &[], &BOAT_TEXTURED_PARTS),
        (false, true) => (
            &BOAT_COMMON_PARTS,
            &BOAT_CHEST_PARTS,
            &BOAT_CHEST_TEXTURED_PARTS,
        ),
    }
}

/// Mutable boat model, mirroring vanilla `BoatModel` / `RaftModel` (+ their chest variants). The flat
/// parts (hull pieces, the two paddles, and — with a chest — the chest bottom/lid/lock) hang off a
/// synthetic root; each unified cube takes its geometry/color from the colored part and its UV from the
/// matching textured part, so one tree drives both render paths. The two hull sides share the colored
/// `BOAT_SIDE` box but carry distinct left/right UVs. `new` selects the boat / raft / chest tree; the
/// boat has no per-frame animation (the vanilla paddle swing is deferred entity-side state), so
/// `setup_anim` is a no-op. The colored fallback uses the baked wood color; the textured path uses the
/// per-family boat texture.
pub(in crate::entity_models) struct BoatModel {
    root: ModelPart,
}

impl BoatModel {
    pub(in crate::entity_models) fn new(family: BoatModelFamily, chest: bool) -> Self {
        let (colored_common, colored_chest, textured) = boat_part_trees(family, chest);
        let children = colored_common
            .iter()
            .chain(colored_chest.iter())
            .zip(textured.iter())
            .map(|(colored, textured)| ModelPart::from_descs(colored, textured))
            .collect();
        Self {
            root: ModelPart::root_from_parts(children),
        }
    }
}

impl EntityModel for BoatModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {
        // The boat renders at its bind pose; the vanilla `BoatModel.setupAnim` paddle swing (driven by
        // the un-projected `rowingTime`) is deferred entity-side state.
    }
}
