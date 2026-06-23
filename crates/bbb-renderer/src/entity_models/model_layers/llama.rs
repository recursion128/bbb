use super::{
    apply_head_look, apply_quadruped_leg_swing, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, LLAMA_CREAMY,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const ADULT_LLAMA_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-2.0, -14.0, -10.0],
        size: [4.0, 4.0, 9.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-4.0, -16.0, -6.0],
        size: [8.0, 18.0, 6.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-4.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [1.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        color: LLAMA_CREAMY,
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -10.0, -7.0],
    size: [12.0, 18.0, 10.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const LLAMA_CHEST: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, 0.0],
    size: [8.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const ADULT_LLAMA_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 14.0, 4.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const ADULT_LLAMA_RIGHT_CHEST_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-8.5, 3.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
    },
    cubes: &LLAMA_CHEST,
    children: &[],
};

pub(in crate::entity_models) const ADULT_LLAMA_LEFT_CHEST_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.5, 3.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
    },
    cubes: &LLAMA_CHEST,
    children: &[],
};

// Vanilla 26.1 ModelLayers.LLAMA / TRADER_LLAMA:
// LlamaModel.createBodyLayer(CubeDeformation.NONE). Chest parts are only visible
// when LlamaRenderState.hasChest is true.
pub(in crate::entity_models) const ADULT_LLAMA_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 10.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 10.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 10.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 10.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_PARTS_WITH_CHEST: [ModelPartDesc; 8] = [
    ADULT_LLAMA_PARTS[0],
    ADULT_LLAMA_PARTS[1],
    ADULT_LLAMA_RIGHT_CHEST_PART,
    ADULT_LLAMA_LEFT_CHEST_PART,
    ADULT_LLAMA_PARTS[2],
    ADULT_LLAMA_PARTS[3],
    ADULT_LLAMA_PARTS[4],
    ADULT_LLAMA_PARTS[5],
];

pub(in crate::entity_models) const BABY_LLAMA_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -9.0, -4.0],
        size: [6.0, 11.0, 4.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-1.5, -7.0, -7.0],
        size: [3.0, 3.0, 3.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [0.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-2.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        color: LLAMA_CREAMY,
    },
];

pub(in crate::entity_models) const BABY_LLAMA_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.4, -0.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const BABY_LLAMA_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.6, -0.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(in crate::entity_models) const BABY_LLAMA_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.0, -8.5],
    size: [8.0, 6.0, 13.0],
    color: LLAMA_CREAMY,
}];

// Vanilla 26.1 ModelLayers.LLAMA_BABY / TRADER_LLAMA_BABY:
// BabyLlamaModel.createBodyLayer(CubeDeformation.NONE). The layer includes
// chest parts, but LlamaRenderer sets hasChest=false for babies.
pub(in crate::entity_models) const BABY_LLAMA_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 16.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.5, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 16.5, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 14.0, 2.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_BODY,
        children: &[],
    },
];

// Vanilla 26.1 `ModelLayers.LLAMA` / `LLAMA_BABY` (`LlamaRenderer`). The trader
// llama bakes the same `LlamaModel.createBodyLayer` mesh under
// `ModelLayers.TRADER_LLAMA[_BABY]`; the only difference is the `LlamaDecorLayer`
// trader overlay, a deferred equipment layer, so the textured base reuses these.
pub(in crate::entity_models) const MODEL_LAYER_LLAMA: &str = "minecraft:llama#main";
pub(in crate::entity_models) const MODEL_LAYER_LLAMA_BABY: &str = "minecraft:llama_baby#main";

// `LlamaModel.createBodyLayer` UVs, atlas 128×64. `CubeDeformation.NONE`, so every
// `uv_size` equals the geometry size. The two ears share `texOffs(17, 0)` and neither
// is mirrored (both vanilla `addBox("ear", …)` calls use the same offset).
pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-2.0, -14.0, -10.0],
        size: [4.0, 4.0, 9.0],
        uv_size: [4.0, 4.0, 9.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -16.0, -6.0],
        size: [8.0, 18.0, 6.0],
        uv_size: [8.0, 18.0, 6.0],
        tex: [0.0, 14.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-4.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        uv_size: [3.0, 3.0, 2.0],
        tex: [17.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [1.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        uv_size: [3.0, 3.0, 2.0],
        tex: [17.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        uv_size: [12.0, 18.0, 10.0],
        tex: [29.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_RIGHT_CHEST: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, 0.0, 0.0],
        size: [8.0, 8.0, 3.0],
        uv_size: [8.0, 8.0, 3.0],
        tex: [45.0, 28.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_LEFT_CHEST: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, 0.0, 0.0],
        size: [8.0, 8.0, 3.0],
        uv_size: [8.0, 8.0, 3.0],
        tex: [45.0, 41.0],
        mirror: false,
    }];

// All four adult legs share one `CubeListBuilder` (`texOffs(29, 29)`, no mirror).
pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 14.0, 4.0],
        uv_size: [4.0, 14.0, 4.0],
        tex: [29.0, 29.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[0].pose,
        cubes: &ADULT_LLAMA_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[1].pose,
        cubes: &ADULT_LLAMA_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[2].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[3].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[4].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_PARTS[5].pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_LLAMA_TEXTURED_PARTS_WITH_CHEST: [TexturedModelPartDesc;
    8] = [
    ADULT_LLAMA_TEXTURED_PARTS[0],
    ADULT_LLAMA_TEXTURED_PARTS[1],
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_RIGHT_CHEST_PART.pose,
        cubes: &ADULT_LLAMA_TEXTURED_RIGHT_CHEST,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_LLAMA_LEFT_CHEST_PART.pose,
        cubes: &ADULT_LLAMA_TEXTURED_LEFT_CHEST,
        children: &[],
    },
    ADULT_LLAMA_TEXTURED_PARTS[2],
    ADULT_LLAMA_TEXTURED_PARTS[3],
    ADULT_LLAMA_TEXTURED_PARTS[4],
    ADULT_LLAMA_TEXTURED_PARTS[5],
];

// `BabyLlamaModel.createBodyLayer` UVs, atlas 64×64. Each leg has its own `texOffs`
// (right/left, hind/front), unlike the adult layer's single shared leg.
pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-3.0, -9.0, -4.0],
        size: [6.0, 11.0, 4.0],
        uv_size: [6.0, 11.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.5, -7.0, -7.0],
        size: [3.0, 3.0, 3.0],
        uv_size: [3.0, 3.0, 3.0],
        tex: [0.0, 15.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [0.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [20.0, 4.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        uv_size: [2.0, 2.0, 2.0],
        tex: [20.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.4, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [0.0, 45.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.6, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [12.0, 45.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.4, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [0.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.6, -0.5, -1.5],
        size: [3.0, 8.0, 3.0],
        uv_size: [3.0, 8.0, 3.0],
        tex: [12.0, 34.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -3.0, -8.5],
        size: [8.0, 6.0, 13.0],
        uv_size: [8.0, 6.0, 13.0],
        tex: [0.0, 15.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_LLAMA_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[0].pose,
        cubes: &BABY_LLAMA_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[1].pose,
        cubes: &BABY_LLAMA_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[2].pose,
        cubes: &BABY_LLAMA_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[3].pose,
        cubes: &BABY_LLAMA_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[4].pose,
        cubes: &BABY_LLAMA_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_LLAMA_PARTS[5].pose,
        cubes: &BABY_LLAMA_TEXTURED_BODY,
        children: &[],
    },
];

/// The four leg part indices in the llama body layers. The adult layer lists head/body at `0`/`1`
/// then the legs at `[2, 3, 4, 5]`; the chest layer inserts the two chest parts at `2`/`3`, pushing
/// the legs to `[4, 5, 6, 7]`; the baby layer lists the head at `0`, the legs at `[1, 2, 3, 4]`, and
/// the body last. [`quadruped_leg_swing_pose`] resolves each leg's phase from its offset.
fn llama_leg_part_indices(baby: bool, has_chest: bool) -> [usize; 4] {
    if baby {
        [1, 2, 3, 4]
    } else if has_chest {
        [4, 5, 6, 7]
    } else {
        [2, 3, 4, 5]
    }
}

/// Selects the colored and textured const trees for a llama by `baby`/`has_chest`: the baby layer,
/// the adult layer, or the adult layer with the two cargo-chest parts. Zipped into the unified tree
/// by [`LlamaModel::new`].
pub(in crate::entity_models) fn llama_part_trees(
    baby: bool,
    has_chest: bool,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&BABY_LLAMA_PARTS, &BABY_LLAMA_TEXTURED_PARTS)
    } else if has_chest {
        (
            &ADULT_LLAMA_PARTS_WITH_CHEST,
            &ADULT_LLAMA_TEXTURED_PARTS_WITH_CHEST,
        )
    } else {
        (&ADULT_LLAMA_PARTS, &ADULT_LLAMA_TEXTURED_PARTS)
    }
}

/// Mutable llama model, mirroring vanilla `LlamaModel` (a `QuadrupedModel`, shared by the trader
/// llama). The unified tree is selected by `baby`/`has_chest` ([`llama_part_trees`]). `setup_anim`
/// looks the head (part `0`, [`apply_head_look`]) and swings the four legs at the standard quadruped
/// diagonal phase ([`apply_quadruped_leg_swing`]). The family/variant choose only the recolor (the
/// colored fallback) or the texture (the textured path); the chest visibility rides the tree choice.
pub(in crate::entity_models) struct LlamaModel {
    root: ModelPart,
    leg_indices: [usize; 4],
}

impl LlamaModel {
    pub(in crate::entity_models) fn new(baby: bool, has_chest: bool) -> Self {
        let (colored, textured) = llama_part_trees(baby, has_chest);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            leg_indices: llama_leg_part_indices(baby, has_chest),
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
            self.root.child_at_mut(0),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            self.leg_indices,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
