use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO,
};

pub(super) const CHICKEN_WHITE: [f32; 4] = [0.94, 0.94, 0.86, 1.0];
pub(super) const CHICKEN_WING: [f32; 4] = [0.82, 0.82, 0.76, 1.0];
pub(super) const CHICKEN_BEAK: [f32; 4] = [0.95, 0.62, 0.18, 1.0];
pub(super) const CHICKEN_RED: [f32; 4] = [0.86, 0.08, 0.08, 1.0];
pub(super) const CHICKEN_LEG: [f32; 4] = [0.82, 0.48, 0.12, 1.0];
pub(super) const PLAYER_BLUE: [f32; 4] = [0.22, 0.42, 0.78, 1.0];
pub(super) const HOGLIN_RED: [f32; 4] = [0.60, 0.28, 0.24, 1.0];
pub(super) const ZOGLIN_GREEN: [f32; 4] = [0.42, 0.55, 0.39, 1.0];
pub(super) const RAVAGER_GRAY: [f32; 4] = [0.44, 0.38, 0.34, 1.0];
pub(super) const VILLAGER_ROBE: [f32; 4] = [0.48, 0.34, 0.23, 1.0];
pub(super) const ILLAGER_GRAY: [f32; 4] = [0.42, 0.45, 0.48, 1.0];
pub(super) const PIG_PINK: [f32; 4] = [0.92, 0.55, 0.62, 1.0];
pub(super) const PIG_COLD_FUR: [f32; 4] = [0.82, 0.78, 0.70, 1.0];
pub(super) const SHEEP_WOOL: [f32; 4] = [0.86, 0.86, 0.80, 1.0];
pub(super) const HORSE_BROWN: [f32; 4] = [0.44, 0.27, 0.14, 1.0];
pub(super) const DONKEY_GRAY: [f32; 4] = [0.46, 0.45, 0.42, 1.0];
pub(super) const MULE_BROWN: [f32; 4] = [0.34, 0.24, 0.17, 1.0];
pub(super) const SKELETON_HORSE_BONE: [f32; 4] = [0.78, 0.78, 0.68, 1.0];
pub(super) const ZOMBIE_HORSE_GREEN: [f32; 4] = [0.32, 0.54, 0.32, 1.0];
pub(super) const CAMEL_TAN: [f32; 4] = [0.72, 0.50, 0.31, 1.0];
pub(super) const CAMEL_HUSK_BROWN: [f32; 4] = [0.42, 0.33, 0.25, 1.0];
pub(super) const LLAMA_CREAMY: [f32; 4] = [0.78, 0.65, 0.45, 1.0];
pub(super) const LLAMA_WHITE: [f32; 4] = [0.86, 0.84, 0.76, 1.0];
pub(super) const LLAMA_BROWN: [f32; 4] = [0.43, 0.27, 0.16, 1.0];
pub(super) const LLAMA_GRAY: [f32; 4] = [0.45, 0.44, 0.40, 1.0];
pub(super) const GOAT_WHITE: [f32; 4] = [0.84, 0.80, 0.70, 1.0];
pub(super) const GOAT_HORN: [f32; 4] = [0.72, 0.66, 0.54, 1.0];
pub(super) const GOAT_BEARD: [f32; 4] = [0.48, 0.42, 0.32, 1.0];
pub(super) const POLAR_BEAR_WHITE: [f32; 4] = [0.88, 0.88, 0.82, 1.0];
pub(super) const CREEPER_GREEN: [f32; 4] = [0.24, 0.68, 0.23, 1.0];
pub(super) const SPIDER_DARK: [f32; 4] = [0.16, 0.12, 0.12, 1.0];
pub(super) const ENDERMAN_DARK: [f32; 4] = [0.08, 0.06, 0.10, 1.0];
pub(super) const IRON_GOLEM_STONE: [f32; 4] = [0.74, 0.74, 0.68, 1.0];
pub(super) const SNOW_GOLEM_WHITE: [f32; 4] = [0.90, 0.92, 0.88, 1.0];
pub(super) const WITCH_ROBE: [f32; 4] = [0.28, 0.17, 0.36, 1.0];
pub(super) const WITCH_HAT_COLOR: [f32; 4] = [0.16, 0.11, 0.20, 1.0];
pub(super) const ILLAGER_ROBE: [f32; 4] = [0.38, 0.40, 0.44, 1.0];
pub(super) const ILLAGER_HAT_COLOR: [f32; 4] = [0.30, 0.31, 0.34, 1.0];
pub(super) const MINECART_GRAY: [f32; 4] = [0.34, 0.35, 0.37, 1.0];
pub(super) const BOAT_WOOD: [f32; 4] = [0.55, 0.36, 0.18, 1.0];
pub(super) const PLACEHOLDER_COLOR: [f32; 4] = [0.80, 0.20, 0.72, 1.0];

mod armor_stand;
mod boat;
mod camel;
mod chicken;
mod cow;
mod creeper;
mod enderman;
mod equine;
mod goat;
mod hoglin;
mod illager;
mod llama;
mod pig;
mod piglin;
mod player;
mod polar_bear;
mod ravager;
mod sheep;
mod skeleton;
mod skeleton_clothing;
mod slime;
mod spider;
mod textures;
mod villager;
mod witch;
mod wolf;
mod zombie;

pub(super) use armor_stand::*;
pub(super) use boat::*;
pub(super) use camel::*;
pub(super) use chicken::*;
pub(super) use cow::*;
pub(super) use creeper::*;
pub(super) use enderman::*;
pub(super) use equine::*;
pub(super) use goat::*;
pub(super) use hoglin::*;
pub(super) use illager::*;
pub(super) use llama::*;
pub(super) use pig::*;
pub(super) use piglin::*;
pub(super) use player::*;
pub(super) use polar_bear::*;
pub(super) use ravager::*;
pub(super) use sheep::*;
pub(super) use skeleton::*;
pub(super) use skeleton_clothing::*;
pub(super) use slime::*;
pub(super) use spider::*;
pub(super) use textures::*;
pub use textures::{
    boat_entity_texture_refs, chicken_entity_texture_refs, cow_entity_texture_refs,
    creeper_entity_texture_refs, enderman_entity_texture_refs, entity_model_texture_refs,
    goat_entity_texture_refs, hoglin_entity_texture_refs, pig_entity_texture_refs,
    player_entity_texture_refs, polar_bear_entity_texture_refs, ravager_entity_texture_refs,
    sheep_entity_texture_refs, skeleton_entity_texture_refs, slime_entity_texture_refs,
    spider_entity_texture_refs, villager_entity_texture_refs, wolf_entity_texture_refs,
};
pub(super) use villager::*;
pub(super) use witch::*;
pub(super) use wolf::*;
pub(super) use zombie::*;

pub(super) const IRON_GOLEM_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -12.0, -5.5],
        size: [8.0, 10.0, 8.0],
        color: IRON_GOLEM_STONE,
    },
    ModelCubeDesc {
        min: [-1.0, -5.0, -7.5],
        size: [2.0, 4.0, 2.0],
        color: IRON_GOLEM_STONE,
    },
];

pub(super) const IRON_GOLEM_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-9.0, -2.0, -6.0],
        size: [18.0, 12.0, 11.0],
        color: IRON_GOLEM_STONE,
    },
    ModelCubeDesc {
        min: [-5.0, 9.5, -3.5],
        size: [10.0, 6.0, 7.0],
        color: IRON_GOLEM_STONE,
    },
];

pub(super) const IRON_GOLEM_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

pub(super) const IRON_GOLEM_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [9.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

pub(super) const IRON_GOLEM_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

pub(super) const IRON_GOLEM_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

// Vanilla 26.1 IronGolemModel.createBodyLayer().
pub(super) const IRON_GOLEM_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 11.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 11.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const MODEL_LAYER_IRON_GOLEM: &str = "minecraft:iron_golem#main";

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.0, -12.0, -5.5],
        size: [8.0, 10.0, 8.0],
        uv_size: [8.0, 10.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.0, -5.0, -7.5],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [24.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-9.0, -2.0, -6.0],
        size: [18.0, 12.0, 11.0],
        uv_size: [18.0, 12.0, 11.0],
        tex: [0.0, 40.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-5.0, 9.5, -3.5],
        size: [10.0, 6.0, 7.0],
        uv_size: [9.0, 5.0, 6.0],
        tex: [0.0, 70.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-13.0, -2.5, -3.0],
        size: [4.0, 30.0, 6.0],
        uv_size: [4.0, 30.0, 6.0],
        tex: [60.0, 21.0],
        mirror: false,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [9.0, -2.5, -3.0],
        size: [4.0, 30.0, 6.0],
        uv_size: [4.0, 30.0, 6.0],
        tex: [60.0, 58.0],
        mirror: false,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [6.0, 16.0, 5.0],
        uv_size: [6.0, 16.0, 5.0],
        tex: [37.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [6.0, 16.0, 5.0],
        uv_size: [6.0, 16.0, 5.0],
        tex: [60.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[0].pose,
        cubes: &IRON_GOLEM_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[1].pose,
        cubes: &IRON_GOLEM_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[2].pose,
        cubes: &IRON_GOLEM_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[3].pose,
        cubes: &IRON_GOLEM_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[4].pose,
        cubes: &IRON_GOLEM_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[5].pose,
        cubes: &IRON_GOLEM_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(super) const SNOW_GOLEM_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(super) const SNOW_GOLEM_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -0.5],
    size: [11.0, 1.0, 1.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(super) const SNOW_GOLEM_UPPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -9.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(super) const SNOW_GOLEM_LOWER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.5, -11.5, -5.5],
    size: [11.0, 11.0, 11.0],
    color: SNOW_GOLEM_WHITE,
}];

// Vanilla 26.1 SnowGolemModel.createBodyLayer().
pub(super) const SNOW_GOLEM_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 6.0, 1.0],
            rotation: [0.0, 0.0, 1.0],
        },
        cubes: &SNOW_GOLEM_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 6.0, -1.0],
            rotation: [0.0, std::f32::consts::PI, -1.0],
        },
        cubes: &SNOW_GOLEM_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_UPPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_LOWER_BODY,
        children: &[],
    },
];

pub(in crate::entity_models) const MODEL_LAYER_SNOW_GOLEM: &str = "minecraft:snow_golem#main";

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -7.5, -3.5],
        size: [7.0, 7.0, 7.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.5, 0.5, -0.5],
        size: [11.0, 1.0, 1.0],
        uv_size: [12.0, 2.0, 2.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_UPPER_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -9.5, -4.5],
        size: [9.0, 9.0, 9.0],
        uv_size: [10.0, 10.0, 10.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_LOWER_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.5, -11.5, -5.5],
        size: [11.0, 11.0, 11.0],
        uv_size: [12.0, 12.0, 12.0],
        tex: [0.0, 36.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[0].pose,
        cubes: &SNOW_GOLEM_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[1].pose,
        cubes: &SNOW_GOLEM_TEXTURED_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[2].pose,
        cubes: &SNOW_GOLEM_TEXTURED_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[3].pose,
        cubes: &SNOW_GOLEM_TEXTURED_UPPER_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[4].pose,
        cubes: &SNOW_GOLEM_TEXTURED_LOWER_BODY,
        children: &[],
    },
];
