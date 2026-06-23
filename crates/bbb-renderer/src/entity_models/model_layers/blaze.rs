use super::{head_look_at_rest, head_look_pose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// The blaze fallback paints the head and rods a single fiery orange.
pub(in crate::entity_models) const BLAZE_ORANGE: [f32; 4] = [0.94, 0.55, 0.10, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_BLAZE: &str = "minecraft:blaze#main";

// Vanilla 26.1 ModelLayers.BLAZE: BlazeModel.createBodyLayer() — an 8x8x8 head plus twelve rods, all
// positioned by `BlazeModel.setupAnim` from `ageInTicks`. Each cube carries both render paths' data:
// the colored debug tint (`BLAZE_ORANGE`) and the textured `uv_size` / `texOffs`.
//
// Head: texOffs(0, 0), 8x8x8.
pub(in crate::entity_models) const BLAZE_HEAD_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -4.0, -4.0],
    [8.0, 8.0, 8.0],
    BLAZE_ORANGE,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

// Vanilla reuses one `rod` CubeListBuilder for all twelve rods: `texOffs(0, 16)` addBox(0, 0, 0, 2, 8, 2).
pub(in crate::entity_models) const BLAZE_ROD_CUBE: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [2.0, 8.0, 2.0],
    BLAZE_ORANGE,
    [2.0, 8.0, 2.0],
    [0.0, 16.0],
    false,
)];

/// The number of rods in the blaze body layer (parts `1..=12`; part `0` is the head).
pub(in crate::entity_models) const BLAZE_ROD_COUNT: usize = 12;

/// Vanilla `BlazeModel.createBodyLayer` rod child names, in order `rod0..rod11`. `child_mut` needs
/// `&'static` names, so the twelve rods draw their names from this const array.
const BLAZE_ROD_NAMES: [&str; BLAZE_ROD_COUNT] = [
    "rod0", "rod1", "rod2", "rod3", "rod4", "rod5", "rod6", "rod7", "rod8", "rod9", "rod10",
    "rod11",
];

/// Vanilla `BlazeModel.setupAnim` rod placement: the twelve rods orbit in three rings of
/// four, their `x`/`y`/`z` offsets SET every frame from `ageInTicks`. Ring 0 (rods 0..4) at
/// radius 9, ring 1 (4..8) at radius 7, ring 2 (8..12) at radius 5; each ring spins at its
/// own rate and the rods bob in `y`. Returns the part offset for rod `index`.
pub(in crate::entity_models) fn blaze_rod_offset(index: usize, age_in_ticks: f32) -> [f32; 3] {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
    let i = index as f32;
    let (radius, y, base_angle) = if index < 4 {
        (
            9.0,
            -2.0 + ((2.0 * i + age_in_ticks) * 0.25).cos(),
            age_in_ticks * PI * -0.1,
        )
    } else if index < 8 {
        (
            7.0,
            2.0 + ((2.0 * i + age_in_ticks) * 0.25).cos(),
            FRAC_PI_4 + age_in_ticks * PI * 0.03,
        )
    } else {
        (
            5.0,
            11.0 + ((1.5 * i + age_in_ticks) * 0.5).cos(),
            0.47123894 + age_in_ticks * PI * -0.05,
        )
    };
    let angle = base_angle + (index % 4) as f32 * FRAC_PI_2;
    [angle.cos() * radius, y, angle.sin() * radius]
}

/// Mutable blaze model, mirroring vanilla `BlazeModel`. The unified tree is built once with named
/// children: the `head` plus the twelve orbiting `rod0..rod11`. `setup_anim` follows the head look
/// angles ([`head_look_pose`]) and SETs every rod offset from `ageInTicks` ([`blaze_rod_offset`]). A
/// blaze floats, so there is no walk swing, and there is no `MeshTransformer` scaling (unit model
/// root). The rod layer rest offsets are irrelevant (`PART_POSE_ZERO`); vanilla never displays the
/// un-posed rods since `setup_anim` overwrites every offset each frame.
pub(in crate::entity_models) struct BlazeModel {
    root: ModelPart,
}

impl BlazeModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(1 + BLAZE_ROD_COUNT);
        children.push((
            "head",
            ModelPart::leaf(PART_POSE_ZERO, BLAZE_HEAD_CUBE.to_vec()),
        ));
        for &name in &BLAZE_ROD_NAMES {
            children.push((
                name,
                ModelPart::leaf(PART_POSE_ZERO, BLAZE_ROD_CUBE.to_vec()),
            ));
        }
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for BlazeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let age_in_ticks = instance.render_state.age_in_ticks;
        let head_yaw = instance.render_state.head_yaw;
        let head_pitch = instance.render_state.head_pitch;
        if !head_look_at_rest(head_yaw, head_pitch) {
            let head = self.root.child_mut("head");
            head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
        }
        for (index, &name) in BLAZE_ROD_NAMES.iter().enumerate() {
            self.root.child_mut(name).pose.offset = blaze_rod_offset(index, age_in_ticks);
        }
    }
}
