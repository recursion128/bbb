use super::{ModelCubeDesc, ModelPartDesc, PartPose, COD_TAN};

// Vanilla 26.1 `CodModel.createBodyLayer` (atlas 32×32). All offsets share the
// `yo = 22` baseline. The fins are zero-thickness planes (`right`/`left` flat in Y,
// `tail`/`top` flat in X).
pub(in crate::entity_models) const COD_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, 0.0],
    size: [2.0, 4.0, 7.0],
    color: COD_TAN,
}];

pub(in crate::entity_models) const COD_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -3.0],
    size: [2.0, 4.0, 3.0],
    color: COD_TAN,
}];

pub(in crate::entity_models) const COD_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 3.0, 1.0],
    color: COD_TAN,
}];

pub(in crate::entity_models) const COD_RIGHT_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -1.0],
    size: [2.0, 0.0, 2.0],
    color: COD_TAN,
}];

pub(in crate::entity_models) const COD_LEFT_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [2.0, 0.0, 2.0],
    color: COD_TAN,
}];

pub(in crate::entity_models) const COD_TAIL_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.0, 0.0],
    size: [0.0, 4.0, 4.0],
    color: COD_TAN,
}];

pub(in crate::entity_models) const COD_TOP_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.0, -1.0],
    size: [0.0, 1.0, 6.0],
    color: COD_TAN,
}];

/// Vanilla `CodModel.createBodyLayer` part order: body, head, nose, right fin
/// (`zRot = -π/4`), left fin (`zRot = π/4`), tail fin, top fin. The tail fin is
/// index [`COD_TAIL_FIN_PART_INDEX`]; `CodModel.setupAnim` sets its `yRot`.
pub(in crate::entity_models) const COD_PARTS: [ModelPartDesc; 7] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COD_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COD_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COD_NOSE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 23.0, 0.0],
            rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_4],
        },
        cubes: &COD_RIGHT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 23.0, 0.0],
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_4],
        },
        cubes: &COD_LEFT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COD_TAIL_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &COD_TOP_FIN,
        children: &[],
    },
];

pub(in crate::entity_models) const COD_TAIL_FIN_PART_INDEX: usize = 5;

/// Vanilla `CodModel.setupAnim`: `tailFin.yRot = -amplitude * 0.45 * sin(0.6 *
/// ageInTicks)`, with `amplitude = isInWater ? 1.0 : 1.5` (a beached cod thrashes
/// harder). The rest pose has `yRot = 0`, so this is set absolutely.
pub(in crate::entity_models) fn cod_tail_fin_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let amplitude = if in_water { 1.0 } else { 1.5 };
    -amplitude * 0.45 * (0.6 * age_in_ticks).sin()
}
