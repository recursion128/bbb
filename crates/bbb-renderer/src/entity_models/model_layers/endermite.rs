use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// The endermite fallback paints its four chitin segments a dark End purple.
pub(in crate::entity_models) const ENDERMITE_PURPLE: [f32; 4] = [0.18, 0.10, 0.24, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_ENDERMITE: &str = "minecraft:endermite#main";

/// The number of body segments in the endermite body layer.
pub(in crate::entity_models) const ENDERMITE_SEGMENT_COUNT: usize = 4;

/// Vanilla `EndermiteModel.createBodyLayer` body-segment child names, in order `segment0..segment3`.
/// `child_mut` needs `&'static` names, so the procedural body draws its names from this const array.
const ENDERMITE_SEGMENT_NAMES: [&str; ENDERMITE_SEGMENT_COUNT] =
    ["segment0", "segment1", "segment2", "segment3"];

const fn endermite_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    // No deformation, so `uv_size == size`; never mirrored. Each cube carries both render paths' data:
    // the colored debug tint (`ENDERMITE_PURPLE`) and the textured `uv_size` / `texOffs`.
    ModelCube::new(min, size, ENDERMITE_PURPLE, size, tex, false)
}

// Vanilla 26.1 EndermiteModel.createBodyLayer: four nested chitin segments. Each segment
// `i` is a box of BODY_SIZES[i] = (sx, sy, sz) drawn from texOffs(BODY_TEXS[i]) at
// addBox(-sx/2, 0, -sz/2, sx, sy, sz), posed at offset (0, 24 - sy, placement) where
// `placement` walks back to front by half the summed depths of adjacent segments.
pub(in crate::entity_models) const ENDERMITE_SEGMENT_CUBES: [ModelCube; ENDERMITE_SEGMENT_COUNT] = [
    endermite_cube([-2.0, 0.0, -1.0], [4.0, 3.0, 2.0], [0.0, 0.0]),
    endermite_cube([-3.0, 0.0, -2.5], [6.0, 4.0, 5.0], [0.0, 5.0]),
    endermite_cube([-1.5, 0.0, -0.5], [3.0, 3.0, 1.0], [0.0, 14.0]),
    endermite_cube([-0.5, 0.0, -0.5], [1.0, 2.0, 1.0], [0.0, 18.0]),
];

const fn endermite_pose(offset: [f32; 3]) -> PartPose {
    PartPose {
        offset,
        rotation: [0.0, 0.0, 0.0],
    }
}

pub(in crate::entity_models) const ENDERMITE_SEGMENT_POSES: [PartPose; ENDERMITE_SEGMENT_COUNT] = [
    endermite_pose([0.0, 21.0, -3.5]),
    endermite_pose([0.0, 20.0, 0.0]),
    endermite_pose([0.0, 21.0, 3.0]),
    endermite_pose([0.0, 22.0, 4.0]),
];

/// Vanilla `EndermiteModel.setupAnim` segment wiggle for segment `index`, driven purely by
/// `ageInTicks` (`super.setupAnim` first resets every part to its rest pose). With `phase =
/// ageInTicks * 0.9 + index * 0.15 * π` and `dist = |index - 2|`, vanilla *sets*
/// `segment.yRot = cos(phase) * π * 0.01 * (1 + dist)` and `segment.x = sin(phase) * π *
/// 0.1 * dist`. The middle segment (`index == 2`, `dist == 0`) never shifts in `x` and only
/// wags `yRot` by the smallest amount; the head and tail (`dist == 2`) swing the most. Only
/// `offset.x` and `rotation.yRot` change; the rest `offset.y`/`offset.z` and the zero
/// `xRot`/`zRot` are preserved. Because `ageInTicks` advances every frame and the rest phase
/// already carries nonzero `cos`/`sin` terms, the endermite never sits at its layer pose.
pub(in crate::entity_models) fn endermite_segment_pose(
    base: PartPose,
    index: usize,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let phase = age_in_ticks * 0.9 + index as f32 * 0.15 * PI;
    let dist = (index as i32 - 2).abs() as f32;
    let y_rot = phase.cos() * PI * 0.01 * (1.0 + dist);
    let x = phase.sin() * PI * 0.1 * dist;
    PartPose {
        offset: [x, base.offset[1], base.offset[2]],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Mutable endermite model, mirroring vanilla `EndermiteModel`. The unified tree is built once with
/// named children (the four `segment0..segment3` chitin segments); `setup_anim` wiggles all four from
/// `ageInTicks` ([`endermite_segment_pose`]). There is no head look or walk swing, and no
/// `MeshTransformer` scaling (unit model root).
pub(in crate::entity_models) struct EndermiteModel {
    root: ModelPart,
}

impl EndermiteModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut children: Vec<(&'static str, ModelPart)> =
            Vec::with_capacity(ENDERMITE_SEGMENT_COUNT);
        for (i, &name) in ENDERMITE_SEGMENT_NAMES.iter().enumerate() {
            children.push((
                name,
                ModelPart::leaf(ENDERMITE_SEGMENT_POSES[i], vec![ENDERMITE_SEGMENT_CUBES[i]]),
            ));
        }
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for EndermiteModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let age_in_ticks = instance.render_state.age_in_ticks;
        for (index, &name) in ENDERMITE_SEGMENT_NAMES.iter().enumerate() {
            let segment = self.root.child_mut(name);
            segment.pose = endermite_segment_pose(segment.pose, index, age_in_ticks);
        }
    }
}
