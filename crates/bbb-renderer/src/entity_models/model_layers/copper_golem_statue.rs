use std::f32::consts::PI;

use crate::item_models::SpecialItemCopperGolemPose;

use super::super::{
    geometry::{PartPose, PART_POSE_ZERO},
    model::{ModelCube, ModelPart},
};
use super::copper_golem_tree;
fn statue_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2], deformation: f32) -> ModelCube {
    ModelCube::new(
        min.map(|coordinate| coordinate - deformation),
        size.map(|extent| extent + deformation * 2.0),
        [1.0; 4],
        size,
        tex,
        false,
    )
}

fn statue_pose(offset: [f32; 3], rotation: [f32; 3]) -> PartPose {
    PartPose { offset, rotation }
}

fn statue_leaf(pose: PartPose, cubes: impl IntoIterator<Item = ModelCube>) -> ModelPart {
    ModelPart::leaf(pose, cubes.into_iter().collect())
}

pub(in crate::entity_models) fn copper_golem_statue_model(
    pose: SpecialItemCopperGolemPose,
) -> ModelPart {
    let mut root = match pose {
        SpecialItemCopperGolemPose::Standing => copper_golem_tree(),
        SpecialItemCopperGolemPose::Running => copper_golem_running_statue_model(),
        SpecialItemCopperGolemPose::Sitting => copper_golem_sitting_statue_model(),
        SpecialItemCopperGolemPose::Star => copper_golem_star_statue_model(),
    };
    // `CopperGolemStatueModel.setupAnim(Unit.INSTANCE)`.
    root.pose.offset[1] = 0.0;
    root.pose.rotation[2] = PI;
    root
}

fn copper_golem_running_statue_model() -> ModelPart {
    let body = ModelPart::new(
        statue_pose([-1.064, -5.0, 0.0], [0.0; 3]),
        Vec::new(),
        vec![
            (
                "body_r1",
                statue_leaf(
                    statue_pose([1.1, 0.1, 0.7], [0.1204, -0.0064, -0.0779]),
                    [statue_cube(
                        [-4.02, -6.116, -3.5],
                        [8.0, 6.0, 6.0],
                        [0.0, 15.0],
                        0.0,
                    )],
                ),
            ),
            (
                "head",
                statue_leaf(
                    statue_pose([0.7, -5.6, -1.8], [0.0; 3]),
                    [
                        statue_cube([-4.0, -5.1, -5.0], [8.0, 5.0, 10.0], [0.0, 0.0], 0.0),
                        statue_cube([-1.02, -2.1, -6.0], [2.0, 3.0, 2.0], [56.0, 0.0], 0.0),
                        statue_cube([-1.02, -9.1, -1.0], [2.0, 4.0, 2.0], [37.0, 8.0], -0.015),
                        statue_cube([-2.0, -13.1, -2.0], [4.0, 4.0, 4.0], [37.0, 0.0], -0.015),
                    ],
                ),
            ),
            (
                "right_arm",
                ModelPart::new(
                    statue_pose([-4.0, -6.0, 0.0], [0.0; 3]),
                    Vec::new(),
                    vec![(
                        "right_arm_r1",
                        statue_leaf(
                            statue_pose([0.7, -0.248, -1.62], [1.0036, 0.0, 0.0]),
                            [statue_cube(
                                [-3.052, -1.11, -2.036],
                                [3.0, 10.0, 4.0],
                                [36.0, 16.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
            (
                "left_arm",
                ModelPart::new(
                    statue_pose([4.0, -6.0, 0.0], [0.0; 3]),
                    Vec::new(),
                    vec![(
                        "left_arm_r1",
                        statue_leaf(
                            statue_pose([0.732, 0.0, 0.0], [-0.8715, -0.0535, -0.0449]),
                            [statue_cube(
                                [0.032, -1.1, -2.0],
                                [3.0, 10.0, 4.0],
                                [50.0, 16.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_leg",
                ModelPart::new(
                    statue_pose([-3.064, -5.0, 0.0], [0.0; 3]),
                    Vec::new(),
                    vec![(
                        "right_leg_r1",
                        statue_leaf(
                            statue_pose([1.048, 0.0, -0.9], [-0.8727, 0.0, 0.0]),
                            [statue_cube(
                                [-1.856, -0.1, -1.09],
                                [4.0, 5.0, 4.0],
                                [0.0, 27.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
            (
                "left_leg",
                ModelPart::new(
                    statue_pose([0.936, -5.0, 0.0], [0.0; 3]),
                    Vec::new(),
                    vec![(
                        "left_leg_r1",
                        statue_leaf(
                            statue_pose([1.0, 0.0, 0.0], [0.7854, 0.0, 0.0]),
                            [statue_cube(
                                [-2.088, -0.1, -2.0],
                                [4.0, 5.0, 4.0],
                                [16.0, 27.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
        ],
    )
}

fn copper_golem_sitting_statue_model() -> ModelPart {
    let body = ModelPart::new(
        statue_pose([0.0, -3.0, 2.325], [0.0; 3]),
        vec![
            statue_cube([-3.0, -4.0, -4.525], [6.0, 1.0, 6.0], [3.0, 19.0], 0.0),
            statue_cube([-4.0, -3.0, -3.525], [8.0, 6.0, 6.0], [0.0, 15.0], 0.0),
        ],
        vec![
            (
                "body_r1",
                statue_leaf(
                    statue_pose([0.0, -1.0, -4.325], [0.0, 0.0, -3.1416]),
                    [statue_cube(
                        [-4.0, -3.0, -2.2],
                        [8.0, 6.0, 3.0],
                        [3.0, 18.0],
                        0.0,
                    )],
                ),
            ),
            (
                "head",
                statue_leaf(
                    statue_pose([0.0, -6.0, -0.2], [0.0; 3]),
                    [
                        statue_cube([-1.0, -7.0, -3.3], [2.0, 4.0, 2.0], [37.0, 8.0], -0.015),
                        statue_cube([-2.0, -11.0, -4.3], [4.0, 4.0, 4.0], [37.0, 0.0], -0.015),
                        statue_cube([-4.0, -3.0, -7.325], [8.0, 5.0, 10.0], [0.0, 0.0], 0.0),
                        statue_cube([-1.0, 0.0, -8.325], [2.0, 3.0, 2.0], [56.0, 0.0], 0.0),
                    ],
                ),
            ),
            (
                "right_arm",
                ModelPart::new(
                    statue_pose([-4.0, -5.6, -1.8], [0.4363, 0.0, 0.0]),
                    Vec::new(),
                    vec![(
                        "right_arm_r1",
                        statue_leaf(
                            statue_pose([0.0, 0.0893, 0.1198], [-1.0472, 0.0, 0.0]),
                            [statue_cube(
                                [-3.075, -0.9733, -1.9966],
                                [3.0, 10.0, 4.0],
                                [36.0, 16.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
            (
                "left_arm",
                ModelPart::new(
                    statue_pose([4.0, -5.6, -1.7], [0.4363, 0.0, 0.0]),
                    Vec::new(),
                    vec![(
                        "left_arm_r1",
                        statue_leaf(
                            statue_pose([0.0, -0.0015, -0.0808], [-1.0472, 0.0, 0.0]),
                            [statue_cube(
                                [0.075, -1.0443, -1.8997],
                                [3.0, 10.0, 4.0],
                                [50.0, 16.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
        ],
    );
    let leg = |name, offset: [f32; 3], inner_offset: [f32; 3], tex| {
        (
            name,
            ModelPart::new(
                statue_pose(offset, [0.0; 3]),
                Vec::new(),
                vec![(
                    if name == "right_leg" {
                        "right_leg_r1"
                    } else {
                        "left_leg_r1"
                    },
                    statue_leaf(
                        statue_pose(inner_offset, [-1.5708, 0.0, 0.0]),
                        [statue_cube([-2.0, 0.975, 0.0], [4.0, 5.0, 4.0], tex, 0.0)],
                    ),
                )],
            ),
        )
    };
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("body", body),
            leg(
                "right_leg",
                [-2.1, -2.1, -2.075],
                [0.05, -1.9, 1.075],
                [0.0, 27.0],
            ),
            leg(
                "left_leg",
                [2.0, -2.0, -2.075],
                [0.05, -2.0, 1.075],
                [16.0, 27.0],
            ),
        ],
    )
}

fn copper_golem_star_statue_model() -> ModelPart {
    let body = ModelPart::new(
        statue_pose([0.0, -5.0, 0.0], [0.0; 3]),
        vec![statue_cube(
            [-4.0, -6.0, -3.0],
            [8.0, 6.0, 6.0],
            [0.0, 15.0],
            0.0,
        )],
        vec![
            (
                "head",
                statue_leaf(
                    statue_pose([0.0, -6.0, 0.0], [0.0; 3]),
                    [
                        statue_cube([-4.0, -5.0, -5.0], [8.0, 5.0, 10.0], [0.0, 0.0], 0.0),
                        statue_cube([-1.0, -2.0, -6.0], [2.0, 3.0, 2.0], [56.0, 0.0], 0.0),
                        statue_cube([-1.0, -9.0, -1.0], [2.0, 4.0, 2.0], [37.0, 8.0], -0.015),
                        statue_cube([-2.0, -13.0, -2.0], [4.0, 4.0, 4.0], [37.0, 0.0], -0.015),
                    ],
                ),
            ),
            (
                "right_arm",
                ModelPart::new(
                    statue_pose([-4.0, -6.0, 0.0], [0.0; 3]),
                    Vec::new(),
                    vec![(
                        "right_arm_r1",
                        statue_leaf(
                            statue_pose([1.0, 1.0, 0.0], [0.0, 0.0, 1.9199]),
                            [statue_cube(
                                [-1.5, -5.0, -2.0],
                                [3.0, 10.0, 4.0],
                                [36.0, 16.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
            (
                "left_arm",
                ModelPart::new(
                    statue_pose([4.0, -6.0, 0.0], [0.0; 3]),
                    Vec::new(),
                    vec![(
                        "left_arm_r1",
                        statue_leaf(
                            statue_pose([-1.0, 1.0, 0.0], [0.0, 0.0, -1.9199]),
                            [statue_cube(
                                [-1.5, -5.0, -2.0],
                                [3.0, 10.0, 4.0],
                                [50.0, 16.0],
                                0.0,
                            )],
                        ),
                    )],
                ),
            ),
        ],
    );
    let leg = |name, offset: [f32; 3], inner_offset: [f32; 3], z_rot, tex| {
        (
            name,
            ModelPart::new(
                statue_pose(offset, [0.0; 3]),
                Vec::new(),
                vec![(
                    if name == "right_leg" {
                        "right_leg_r1"
                    } else {
                        "left_leg_r1"
                    },
                    statue_leaf(
                        statue_pose(inner_offset, [0.0, 0.0, z_rot]),
                        [statue_cube([-2.0, -2.5, -2.0], [4.0, 5.0, 4.0], tex, 0.0)],
                    ),
                )],
            ),
        )
    };
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("body", body),
            leg(
                "right_leg",
                [-3.0, -5.0, 0.0],
                [0.35, 2.0, 0.01],
                0.2618,
                [0.0, 27.0],
            ),
            leg(
                "left_leg",
                [1.0, -5.0, 0.0],
                [1.65, 2.0, 0.0],
                -0.2618,
                [16.0, 27.0],
            ),
        ],
    )
}
