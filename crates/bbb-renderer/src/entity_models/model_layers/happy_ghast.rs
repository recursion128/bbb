use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};

// Happy ghasts are a warm cream jelly; the colored fallback paints every cube the same pale
// cream so the silhouette reads even without the texture.
pub(in crate::entity_models) const HAPPY_GHAST_CREAM: [f32; 4] = [0.96, 0.92, 0.74, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST: &str = "minecraft:happy_ghast#main";

/// The nine happy-ghast tentacle lengths, baked verbatim from vanilla 26.1
/// `HappyGhastModel.createBodyLayer` (each `addBox(-1, 0, -1, 2, len, 2)`). Unlike the regular
/// ghast (random lengths), the happy ghast hard-codes them.
pub(in crate::entity_models) const HAPPY_GHAST_TENTACLE_LENGTHS: [f32; 9] =
    [5.0, 7.0, 4.0, 5.0, 5.0, 7.0, 8.0, 8.0, 5.0];

/// The nine tentacle root offsets `[xo, 23.0, yo]`. Vanilla parents the tentacles under the
/// body (`PartPose.offset(0, 16, 0)`) at `PartPose.offset(xo, 7.0, yo)`, so the world-space
/// offset is `[xo, 16 + 7, yo]`. The body carries no rotation (and, for an unharnessed happy
/// ghast, no scale), so flattening the tentacles to absolute offsets is exact.
pub(in crate::entity_models) const HAPPY_GHAST_TENTACLE_OFFSETS: [[f32; 3]; 9] = [
    [-3.75, 23.0, -5.0],
    [1.25, 23.0, -5.0],
    [6.25, 23.0, -5.0],
    [-6.25, 23.0, 0.0],
    [-1.25, 23.0, 0.0],
    [3.75, 23.0, 0.0],
    [-3.75, 23.0, 5.0],
    [1.25, 23.0, 5.0],
    [6.25, 23.0, 5.0],
];

pub(in crate::entity_models) const HAPPY_GHAST_BODY_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -8.0, -8.0],
    size: [16.0, 16.0, 16.0],
    color: HAPPY_GHAST_CREAM,
}];

const HAPPY_GHAST_TENTACLE_0: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(0)];
const HAPPY_GHAST_TENTACLE_1: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(1)];
const HAPPY_GHAST_TENTACLE_2: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(2)];
const HAPPY_GHAST_TENTACLE_3: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(3)];
const HAPPY_GHAST_TENTACLE_4: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(4)];
const HAPPY_GHAST_TENTACLE_5: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(5)];
const HAPPY_GHAST_TENTACLE_6: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(6)];
const HAPPY_GHAST_TENTACLE_7: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(7)];
const HAPPY_GHAST_TENTACLE_8: [ModelCubeDesc; 1] = [happy_ghast_tentacle_cube(8)];

const fn happy_ghast_tentacle_cube(index: usize) -> ModelCubeDesc {
    ModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0],
        color: HAPPY_GHAST_CREAM,
    }
}

const fn happy_ghast_tentacle_part(index: usize, cubes: &'static [ModelCubeDesc]) -> ModelPartDesc {
    ModelPartDesc {
        pose: PartPose {
            offset: HAPPY_GHAST_TENTACLE_OFFSETS[index],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children: &[],
    }
}

// Vanilla 26.1 ModelLayers.HAPPY_GHAST: HappyGhastModel.createBodyLayer(false, NONE). The body
// sits at y 16 and the nine tentacles hang from y 23; the whole layer is scaled 4.0x by
// MeshTransformer.scaling at the model-root transform.
pub(in crate::entity_models) const HAPPY_GHAST_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &HAPPY_GHAST_BODY_CUBE,
        children: &[],
    },
    happy_ghast_tentacle_part(0, &HAPPY_GHAST_TENTACLE_0),
    happy_ghast_tentacle_part(1, &HAPPY_GHAST_TENTACLE_1),
    happy_ghast_tentacle_part(2, &HAPPY_GHAST_TENTACLE_2),
    happy_ghast_tentacle_part(3, &HAPPY_GHAST_TENTACLE_3),
    happy_ghast_tentacle_part(4, &HAPPY_GHAST_TENTACLE_4),
    happy_ghast_tentacle_part(5, &HAPPY_GHAST_TENTACLE_5),
    happy_ghast_tentacle_part(6, &HAPPY_GHAST_TENTACLE_6),
    happy_ghast_tentacle_part(7, &HAPPY_GHAST_TENTACLE_7),
    happy_ghast_tentacle_part(8, &HAPPY_GHAST_TENTACLE_8),
];

pub(in crate::entity_models) const HAPPY_GHAST_TEXTURED_BODY_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-8.0, -8.0, -8.0],
        size: [16.0, 16.0, 16.0],
        uv_size: [16.0, 16.0, 16.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

const HAPPY_GHAST_TEXTURED_TENTACLE_0: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(0)];
const HAPPY_GHAST_TEXTURED_TENTACLE_1: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(1)];
const HAPPY_GHAST_TEXTURED_TENTACLE_2: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(2)];
const HAPPY_GHAST_TEXTURED_TENTACLE_3: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(3)];
const HAPPY_GHAST_TEXTURED_TENTACLE_4: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(4)];
const HAPPY_GHAST_TEXTURED_TENTACLE_5: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(5)];
const HAPPY_GHAST_TEXTURED_TENTACLE_6: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(6)];
const HAPPY_GHAST_TEXTURED_TENTACLE_7: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(7)];
const HAPPY_GHAST_TEXTURED_TENTACLE_8: [TexturedModelCubeDesc; 1] =
    [happy_ghast_textured_tentacle_cube(8)];

// Vanilla reuses `texOffs(0, 0)` for the body and every tentacle, so all of them sample the
// same top-left region of the 64x64 texture.
const fn happy_ghast_textured_tentacle_cube(index: usize) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0],
        uv_size: [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0],
        tex: [0.0, 0.0],
        mirror: false,
    }
}

const fn happy_ghast_textured_tentacle_part(
    index: usize,
    cubes: &'static [TexturedModelCubeDesc],
) -> TexturedModelPartDesc {
    TexturedModelPartDesc {
        pose: PartPose {
            offset: HAPPY_GHAST_TENTACLE_OFFSETS[index],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children: &[],
    }
}

pub(in crate::entity_models) const HAPPY_GHAST_TEXTURED_PARTS: [TexturedModelPartDesc; 10] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &HAPPY_GHAST_TEXTURED_BODY_CUBE,
        children: &[],
    },
    happy_ghast_textured_tentacle_part(0, &HAPPY_GHAST_TEXTURED_TENTACLE_0),
    happy_ghast_textured_tentacle_part(1, &HAPPY_GHAST_TEXTURED_TENTACLE_1),
    happy_ghast_textured_tentacle_part(2, &HAPPY_GHAST_TEXTURED_TENTACLE_2),
    happy_ghast_textured_tentacle_part(3, &HAPPY_GHAST_TEXTURED_TENTACLE_3),
    happy_ghast_textured_tentacle_part(4, &HAPPY_GHAST_TEXTURED_TENTACLE_4),
    happy_ghast_textured_tentacle_part(5, &HAPPY_GHAST_TEXTURED_TENTACLE_5),
    happy_ghast_textured_tentacle_part(6, &HAPPY_GHAST_TEXTURED_TENTACLE_6),
    happy_ghast_textured_tentacle_part(7, &HAPPY_GHAST_TEXTURED_TENTACLE_7),
    happy_ghast_textured_tentacle_part(8, &HAPPY_GHAST_TEXTURED_TENTACLE_8),
];
