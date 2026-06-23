use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// The silverfish fallback paints its body a stony gray.
pub(in crate::entity_models) const SILVERFISH_GRAY: [f32; 4] = [0.50, 0.50, 0.53, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_SILVERFISH: &str = "minecraft:silverfish#main";

/// The number of body segments in the silverfish body layer (parts `0..=6`); the three
/// overlay `layer` parts follow at `7..=9`.
pub(in crate::entity_models) const SILVERFISH_SEGMENT_COUNT: usize = 7;

/// The number of overlay `layer` parts in the silverfish body layer.
pub(in crate::entity_models) const SILVERFISH_LAYER_COUNT: usize = 3;

/// Vanilla `SilverfishModel.createBodyLayer` body-segment child names, in order `segment0..segment6`.
/// `child_mut` needs `&'static` names, so the procedural body draws its names from this const array.
const SILVERFISH_SEGMENT_NAMES: [&str; SILVERFISH_SEGMENT_COUNT] = [
    "segment0", "segment1", "segment2", "segment3", "segment4", "segment5", "segment6",
];

/// Vanilla `SilverfishModel.createBodyLayer` overlay `layer` child names, in order `layer0..layer2`.
const SILVERFISH_LAYER_NAMES: [&str; SILVERFISH_LAYER_COUNT] = ["layer0", "layer1", "layer2"];

const fn silverfish_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    // No deformation, so `uv_size == size`; never mirrored. Each cube carries both render paths' data:
    // the colored debug tint (`SILVERFISH_GRAY`) and the textured `uv_size` / `texOffs`.
    ModelCube::new(min, size, SILVERFISH_GRAY, size, tex, false)
}

// Vanilla 26.1 SilverfishModel.createBodyLayer: seven nested body segments
// (BODY_SIZES[i] = (sx, sy, sz), each addBox(-sx/2, 0, -sz/2, sx, sy, sz) at offset
// (0, 24 - sy, placement)) plus three wider overlay layers riding segments 2/4/1.
pub(in crate::entity_models) const SILVERFISH_SEGMENT_CUBES: [ModelCube; SILVERFISH_SEGMENT_COUNT] = [
    silverfish_cube([-1.5, 0.0, -1.0], [3.0, 2.0, 2.0], [0.0, 0.0]),
    silverfish_cube([-2.0, 0.0, -1.0], [4.0, 3.0, 2.0], [0.0, 4.0]),
    silverfish_cube([-3.0, 0.0, -1.5], [6.0, 4.0, 3.0], [0.0, 9.0]),
    silverfish_cube([-1.5, 0.0, -1.5], [3.0, 3.0, 3.0], [0.0, 16.0]),
    silverfish_cube([-1.0, 0.0, -1.5], [2.0, 2.0, 3.0], [0.0, 22.0]),
    silverfish_cube([-1.0, 0.0, -1.0], [2.0, 1.0, 2.0], [11.0, 0.0]),
    silverfish_cube([-0.5, 0.0, -1.0], [1.0, 1.0, 2.0], [13.0, 4.0]),
];

pub(in crate::entity_models) const SILVERFISH_LAYER_CUBES: [ModelCube; SILVERFISH_LAYER_COUNT] = [
    silverfish_cube([-5.0, 0.0, -1.5], [10.0, 8.0, 3.0], [20.0, 0.0]),
    silverfish_cube([-3.0, 0.0, -1.5], [6.0, 4.0, 3.0], [20.0, 11.0]),
    // Vanilla quirk: layer2 takes its z-min from BODY_SIZES[4][2] (3 => -1.5) but its z-size
    // from BODY_SIZES[1][2] (2), so it is offset asymmetrically in z.
    silverfish_cube([-3.0, 0.0, -1.5], [6.0, 5.0, 2.0], [20.0, 18.0]),
];

const fn silverfish_pose(offset: [f32; 3]) -> PartPose {
    PartPose {
        offset,
        rotation: [0.0, 0.0, 0.0],
    }
}

pub(in crate::entity_models) const SILVERFISH_SEGMENT_POSES: [PartPose; SILVERFISH_SEGMENT_COUNT] = [
    silverfish_pose([0.0, 22.0, -3.5]),
    silverfish_pose([0.0, 21.0, -1.5]),
    silverfish_pose([0.0, 20.0, 1.0]),
    silverfish_pose([0.0, 21.0, 4.0]),
    silverfish_pose([0.0, 22.0, 7.0]),
    silverfish_pose([0.0, 23.0, 9.5]),
    silverfish_pose([0.0, 23.0, 11.5]),
];

pub(in crate::entity_models) const SILVERFISH_LAYER_POSES: [PartPose; SILVERFISH_LAYER_COUNT] = [
    silverfish_pose([0.0, 16.0, 1.0]),
    silverfish_pose([0.0, 20.0, 7.0]),
    silverfish_pose([0.0, 19.0, -1.5]),
];

/// Each overlay `layer` part copies one body segment's animation: `(source_segment,
/// copy_x)`. Vanilla sets `layer0.yRot = segment2.yRot` (x untouched), `layer1.yRot =
/// segment4.yRot` and `layer1.x = segment4.x`, `layer2.yRot = segment1.yRot` and `layer2.x =
/// segment1.x`.
pub(in crate::entity_models) const SILVERFISH_LAYER_RULES: [(usize, bool); SILVERFISH_LAYER_COUNT] =
    [(2, false), (4, true), (1, true)];

/// Vanilla `SilverfishModel.setupAnim` segment wiggle for segment `index`, driven purely by
/// `ageInTicks` (`super.setupAnim` first resets every part to its rest pose). With `phase =
/// ageInTicks * 0.9 + index * 0.15 * π` and `dist = |index - 2|`, vanilla *sets*
/// `segment.yRot = cos(phase) * π * 0.05 * (1 + dist)` and `segment.x = sin(phase) * π * 0.2
/// * dist` — the same shape as the endermite, but with the larger `0.05`/`0.2` amplitudes of
/// the silverfish's faster scuttle. Only `offset.x` and `rotation.yRot` change; the rest
/// `offset.y`/`offset.z` and the zero `xRot`/`zRot` are preserved. `ageInTicks` advances
/// every frame and the rest phase already carries nonzero `cos`/`sin` terms, so the
/// silverfish never sits at its layer pose.
pub(in crate::entity_models) fn silverfish_segment_pose(
    base: PartPose,
    index: usize,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let phase = age_in_ticks * 0.9 + index as f32 * 0.15 * PI;
    let dist = (index as i32 - 2).abs() as f32;
    let y_rot = phase.cos() * PI * 0.05 * (1.0 + dist);
    let x = phase.sin() * PI * 0.2 * dist;
    PartPose {
        offset: [x, base.offset[1], base.offset[2]],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Applies a [`SILVERFISH_LAYER_RULES`] copy to one overlay `layer` part: its `yRot` always
/// follows the source segment's animated `yRot`, and its `offset.x` follows the source's `x`
/// only when `copy_x` (layer0 leaves its `x` at the layer rest). The rest `offset.y`/`z` and
/// `xRot`/`zRot` are preserved.
pub(in crate::entity_models) fn silverfish_layer_pose(
    base: PartPose,
    source: PartPose,
    copy_x: bool,
) -> PartPose {
    PartPose {
        offset: [
            if copy_x {
                source.offset[0]
            } else {
                base.offset[0]
            },
            base.offset[1],
            base.offset[2],
        ],
        rotation: [base.rotation[0], source.rotation[1], base.rotation[2]],
    }
}

/// Mutable silverfish model, mirroring vanilla `SilverfishModel`. The unified tree is built once with
/// named children: the [`SILVERFISH_SEGMENT_COUNT`] body segments (`segment0..segment6`) followed by
/// the [`SILVERFISH_LAYER_COUNT`] overlay parts (`layer0..layer2`). `setup_anim` first wiggles every
/// segment from `ageInTicks` ([`silverfish_segment_pose`]), then copies each overlay's pose from its
/// already-animated source segment per [`SILVERFISH_LAYER_RULES`] ([`silverfish_layer_pose`]). There is
/// no head look or walk swing, and no `MeshTransformer` scaling.
pub(in crate::entity_models) struct SilverfishModel {
    root: ModelPart,
}

impl SilverfishModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut children: Vec<(&'static str, ModelPart)> =
            Vec::with_capacity(SILVERFISH_SEGMENT_COUNT + SILVERFISH_LAYER_COUNT);
        for (i, &name) in SILVERFISH_SEGMENT_NAMES.iter().enumerate() {
            children.push((
                name,
                ModelPart::leaf(
                    SILVERFISH_SEGMENT_POSES[i],
                    vec![SILVERFISH_SEGMENT_CUBES[i]],
                ),
            ));
        }
        for (i, &name) in SILVERFISH_LAYER_NAMES.iter().enumerate() {
            children.push((
                name,
                ModelPart::leaf(SILVERFISH_LAYER_POSES[i], vec![SILVERFISH_LAYER_CUBES[i]]),
            ));
        }
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for SilverfishModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let age_in_ticks = instance.render_state.age_in_ticks;
        for (index, &name) in SILVERFISH_SEGMENT_NAMES.iter().enumerate() {
            let segment = self.root.child_mut(name);
            segment.pose = silverfish_segment_pose(segment.pose, index, age_in_ticks);
        }
        for (&layer_name, &(source, copy_x)) in SILVERFISH_LAYER_NAMES
            .iter()
            .zip(SILVERFISH_LAYER_RULES.iter())
        {
            let source_pose = self.root.child_mut(SILVERFISH_SEGMENT_NAMES[source]).pose;
            let part = self.root.child_mut(layer_name);
            part.pose = silverfish_layer_pose(part.pose, source_pose, copy_x);
        }
    }
}
