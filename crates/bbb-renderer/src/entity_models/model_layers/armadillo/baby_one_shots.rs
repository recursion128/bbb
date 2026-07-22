//! Baby armadillo one-shot animation definitions from Vanilla 26.1.
//!
//! The baby model uses distinct roll-up, roll-out, and peek definitions. In particular,
//! roll-out and peek target the baby-only ear topology as well as the shell ball.

use crate::entity_models::keyframe::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, KeyframeInterpolation,
};

/// Vanilla `BabyArmadilloAnimation.ARMADILLO_BABY_ROLL_UP`.
pub(in crate::entity_models) const ARMADILLO_BABY_ROLL_UP: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 0.5,
        looping: false,
        bones: &[
            BoneAnimation {
                bone: "body",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 5.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(0.0, 6.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 6.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(0.0, -1.0, -1.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "tail",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(0.0, 1.0, -3.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 1.0, -3.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(0.0, 2.0, -3.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "head",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(17.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(-72.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, -1.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(0.0, 0.0, 1.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 0.0, 1.0), KeyframeInterpolation::Linear),
                            keyframe(0.3, pos_vec(0.0, 0.0, 5.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(0.0, 0.0, 5.0), KeyframeInterpolation::Linear),
                            keyframe(0.5, pos_vec(0.0, -0.25, 5.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(-35.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 5.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(0.0, 8.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 8.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(1.0, 3.0, -2.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(-45.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 5.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(0.0, 8.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 8.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(-1.0, 3.0, -3.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(-27.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.2,
                                degree_vec(-32.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(-85.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 5.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(-1.0, 8.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(
                                0.25,
                                pos_vec(-1.0, 9.0, -1.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(0.4, pos_vec(-1.0, 2.0, 3.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(-12.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.2,
                                degree_vec(-35.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(-85.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 5.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(1.0, 8.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(1.0, 9.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(1.0, 2.0, 3.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "cube",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.4,
                                degree_vec(-2.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.45,
                                degree_vec(5.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.5,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 3.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 8.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.2, pos_vec(0.0, 7.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 7.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.4, pos_vec(0.0, 1.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.45, pos_vec(0.0, 0.6, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.5, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
        ],
    };

/// Vanilla `BabyArmadilloAnimation.ARMADILLO_BABY_ROLL_OUT`.
pub(in crate::entity_models) const ARMADILLO_BABY_ROLL_OUT: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 1.5,
        looping: false,
        bones: &[
            BoneAnimation {
                bone: "head",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.1,
                                degree_vec(-50.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.4,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.65,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.7,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.85,
                                degree_vec(-2.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.9,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.95,
                                degree_vec(-7.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.05,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.1,
                                degree_vec(7.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, -1.9, 5.0), KeyframeInterpolation::Linear),
                            keyframe(0.05, pos_vec(0.0, -1.0, 0.2), KeyframeInterpolation::Linear),
                            keyframe(0.1, pos_vec(0.0, 0.0, 0.2), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 2.1, 1.2), KeyframeInterpolation::Linear),
                            keyframe(
                                0.25,
                                pos_vec(0.0, 0.03, 0.13),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(0.4, pos_vec(0.0, 0.03, 0.13), KeyframeInterpolation::Linear),
                            keyframe(
                                0.65,
                                pos_vec(0.0, 0.03, 0.13),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(0.7, pos_vec(0.0, 0.1, 0.2), KeyframeInterpolation::Linear),
                            keyframe(0.75, pos_vec(0.0, 3.1, 2.2), KeyframeInterpolation::Linear),
                            keyframe(0.85, pos_vec(0.0, 4.1, 2.2), KeyframeInterpolation::Linear),
                            keyframe(0.9, pos_vec(0.0, 0.1, 0.2), KeyframeInterpolation::Linear),
                            keyframe(0.95, pos_vec(0.0, 0.9, -0.8), KeyframeInterpolation::Linear),
                            keyframe(1.05, pos_vec(0.0, 0.9, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.1, pos_vec(0.0, 2.6, 0.2), KeyframeInterpolation::Linear),
                            keyframe(1.15, pos_vec(0.0, 2.4, 0.2), KeyframeInterpolation::Linear),
                            keyframe(1.2, pos_vec(0.0, 0.0, 0.2), KeyframeInterpolation::Linear),
                            keyframe(1.25, pos_vec(0.0, 0.0, 0.2), KeyframeInterpolation::Linear),
                            keyframe(1.3, pos_vec(0.0, 0.0, 0.2), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                1.1,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.3,
                                degree_vec(0.0, 0.0, 30.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.4,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.45,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.5,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(1.1, pos_vec(0.0, 3.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(1.2, pos_vec(0.0, 8.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(1.3, pos_vec(-1.0, 3.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.4, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.45, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.5, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                1.1,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.3,
                                degree_vec(0.0, 0.0, -30.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.4,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.45,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.5,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(1.1, pos_vec(0.0, 3.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(1.2, pos_vec(0.0, 8.0, -2.0), KeyframeInterpolation::Linear),
                            keyframe(1.3, pos_vec(1.0, 3.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.35, pos_vec(1.0, 3.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.4, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.45, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.5, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(-90.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.05,
                                degree_vec(-90.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(-45.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.55,
                                degree_vec(-45.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.6,
                                degree_vec(-92.5, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.1,
                                degree_vec(-90.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.3,
                                degree_vec(0.0, 0.0, 30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.4,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.45,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.5,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(-1.0, 2.0, 2.0), KeyframeInterpolation::Linear),
                            keyframe(
                                0.05,
                                pos_vec(-1.0, 2.0, 2.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(0.25, pos_vec(0.0, 2.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(0.55, pos_vec(0.0, 2.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(
                                0.7,
                                pos_vec(-1.0, 2.0, 2.63),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(1.1, pos_vec(-1.0, 2.0, 2.0), KeyframeInterpolation::Linear),
                            keyframe(1.2, pos_vec(-1.0, 7.0, 2.0), KeyframeInterpolation::Linear),
                            keyframe(1.3, pos_vec(-1.0, 3.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.4, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.45, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.5, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(-90.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.05,
                                degree_vec(-90.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(-45.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.55,
                                degree_vec(-45.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.6,
                                degree_vec(-87.5, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.1,
                                degree_vec(-90.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.3,
                                degree_vec(0.0, 0.0, -30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.4,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.45,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.5,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(1.0, 2.0, 2.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.05,
                                pos_vec(1.0, 2.0, 2.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.15,
                                pos_vec(1.0, 2.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(0.25, pos_vec(0.0, 2.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(0.55, pos_vec(0.0, 2.0, -1.0), KeyframeInterpolation::Linear),
                            keyframe(
                                0.7,
                                pos_vec(1.0, 2.0, 1.88),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.75,
                                pos_vec(1.0, 2.0, 2.67),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.1,
                                pos_vec(1.0, 2.0, 2.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.2,
                                pos_vec(1.0, 8.0, 2.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.25,
                                pos_vec(1.06, 5.06, 1.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.3,
                                pos_vec(1.0, 3.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.4,
                                pos_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(1.45, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(
                                1.5,
                                pos_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "body",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(1.1, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(1.2, pos_vec(0.0, 4.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(1.25, pos_vec(0.0, 5.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(1.3, pos_vec(0.0, 4.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(1.4, pos_vec(0.0, -1.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(1.5, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                    ],
                }],
            },
            BoneAnimation {
                bone: "left_ear",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(
                                0.15,
                                pos_vec(0.0, -1.4129, 0.0617),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(0.25, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_ear",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(
                                0.15,
                                pos_vec(0.0, -1.4129, 0.0617),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(0.25, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "cube",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.05,
                                degree_vec(15.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.15,
                                degree_vec(-7.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.25,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.7,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.75,
                                degree_vec(-17.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.85,
                                degree_vec(-25.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.9,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                0.95,
                                degree_vec(12.5, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.05,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                            keyframe(
                                1.1,
                                degree_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::Linear,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.05, pos_vec(0.0, 1.6, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.15, pos_vec(0.0, 0.5, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.25, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.7, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.75, pos_vec(0.0, 1.2, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.85, pos_vec(0.0, 1.7, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.9, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(0.95, pos_vec(0.0, 1.3, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.05, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.2, pos_vec(0.0, 5.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.25, pos_vec(0.0, 8.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(1.5, pos_vec(0.0, 1.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
        ],
    };

/// Vanilla `BabyArmadilloAnimation.ARMADILLO_BABY_PEEK`.
pub(in crate::entity_models) const ARMADILLO_BABY_PEEK: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.5,
    looping: false,
    bones: &[
        BoneAnimation {
            bone: "head",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(-70.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.15,
                            degree_vec(-65.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.4,
                            degree_vec(-50.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.5,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.7,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.9,
                            degree_vec(-7.5, 0.0, 45.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.15,
                            degree_vec(-7.5, 0.0, 45.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.3,
                            degree_vec(-0.8639, -1.4959, -39.1287),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.6,
                            degree_vec(-0.8639, -1.4959, -39.1287),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.75,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.8,
                            degree_vec(-25.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.85,
                            degree_vec(-70.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(0.0, pos_vec(0.0, -1.25, 4.0), KeyframeInterpolation::Linear),
                        keyframe(0.1, pos_vec(0.0, -1.0, 4.0), KeyframeInterpolation::Linear),
                        keyframe(0.15, pos_vec(0.0, -1.0, 4.0), KeyframeInterpolation::Linear),
                        keyframe(0.25, pos_vec(0.0, -1.0, 5.0), KeyframeInterpolation::Linear),
                        keyframe(0.35, pos_vec(0.0, 0.0, 0.2), KeyframeInterpolation::Linear),
                        keyframe(0.4, pos_vec(0.0, 0.0, 0.2), KeyframeInterpolation::Linear),
                        keyframe(0.45, pos_vec(0.0, 0.05, 0.7), KeyframeInterpolation::Linear),
                        keyframe(0.5, pos_vec(0.0, 1.1, 1.2), KeyframeInterpolation::Linear),
                        keyframe(0.6, pos_vec(0.0, 0.1, 0.8), KeyframeInterpolation::Linear),
                        keyframe(0.7, pos_vec(0.0, 0.1, 0.7), KeyframeInterpolation::Linear),
                        keyframe(1.75, pos_vec(0.0, 0.1, 0.7), KeyframeInterpolation::Linear),
                        keyframe(1.8, pos_vec(0.0, 0.1, 0.2), KeyframeInterpolation::Linear),
                        keyframe(1.9, pos_vec(0.0, -1.2, 3.53), KeyframeInterpolation::Linear),
                        keyframe(
                            1.95,
                            pos_vec(0.0, -1.2, 3.53),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(2.0, pos_vec(0.0, -1.2, 3.53), KeyframeInterpolation::Linear),
                        keyframe(2.15, pos_vec(0.0, -0.9, 4.2), KeyframeInterpolation::Linear),
                        keyframe(2.3, pos_vec(0.0, -0.9, 3.4), KeyframeInterpolation::Linear),
                        keyframe(2.5, pos_vec(0.0, -1.9, 4.1), KeyframeInterpolation::Linear),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "right_hind_leg",
            channels: &[AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &[keyframe(
                    0.0,
                    pos_vec(0.0, 3.0, -2.0),
                    KeyframeInterpolation::Linear,
                )],
            }],
        },
        BoneAnimation {
            bone: "left_hind_leg",
            channels: &[AnimationChannel {
                target: AnimationTarget::Position,
                keyframes: &[keyframe(
                    0.0,
                    pos_vec(0.0, 3.0, -2.0),
                    KeyframeInterpolation::Linear,
                )],
            }],
        },
        BoneAnimation {
            bone: "right_front_leg",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.6,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.85,
                            degree_vec(-45.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.0,
                            degree_vec(-45.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.75,
                            degree_vec(-45.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.8,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.95,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(0.0, pos_vec(-1.0, 2.0, 2.0), KeyframeInterpolation::Linear),
                        keyframe(
                            0.6,
                            pos_vec(-1.0, 2.0, 2.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.65,
                            pos_vec(-1.0, 2.0, 1.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.85,
                            pos_vec(0.0, 1.0, -0.5),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.0,
                            pos_vec(0.0, 1.0, -0.5),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.75,
                            pos_vec(0.0, 1.0, -0.5),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.95,
                            pos_vec(0.0, 1.0, 3.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.0,
                            pos_vec(-1.0, 2.0, 3.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.15,
                            pos_vec(-1.0, 3.0, 4.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "left_front_leg",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.65,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.85,
                            degree_vec(-45.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.0,
                            degree_vec(-45.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.75,
                            degree_vec(-45.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.8,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.95,
                            degree_vec(-90.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(0.0, pos_vec(1.0, 2.0, 2.0), KeyframeInterpolation::Linear),
                        keyframe(
                            0.65,
                            pos_vec(1.0, 2.0, 2.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.85,
                            pos_vec(0.0, 1.0, -0.4),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.0,
                            pos_vec(0.0, 1.0, -0.4),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.75,
                            pos_vec(0.0, 1.0, -0.4),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.95,
                            pos_vec(1.0, 1.0, 2.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.0,
                            pos_vec(1.0, 2.0, 3.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.15,
                            pos_vec(1.0, 3.0, 4.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "left_ear",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.4,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.5,
                            degree_vec(-15.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.6,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.8,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.85,
                            degree_vec(-10.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.5,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(
                            0.4,
                            pos_vec(0.0, -1.13, 1.05),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.5,
                            pos_vec(0.0, -1.4129, 0.0617),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(0.6, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "right_ear",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.4,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.5,
                            degree_vec(-15.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.6,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.8,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.85,
                            degree_vec(-10.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.5,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(0.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(
                            0.4,
                            pos_vec(0.0, -1.13, 1.05),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.5,
                            pos_vec(0.0, -1.4129, 0.0617),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(0.6, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "cube",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.25,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.35,
                            degree_vec(15.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.5,
                            degree_vec(-7.5, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.6,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.0,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.05,
                            degree_vec(-17.5, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.15,
                            degree_vec(-25.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.25,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.3,
                            degree_vec(12.5, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.5,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(0.25, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(0.35, pos_vec(0.0, 1.6, 0.0), KeyframeInterpolation::Linear),
                        keyframe(0.5, pos_vec(0.0, 0.5, 0.0), KeyframeInterpolation::Linear),
                        keyframe(0.6, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(2.0, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(2.05, pos_vec(0.0, 1.2, 0.0), KeyframeInterpolation::Linear),
                        keyframe(2.15, pos_vec(0.0, 1.7, 0.0), KeyframeInterpolation::Linear),
                        keyframe(2.25, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                        keyframe(2.3, pos_vec(0.0, 1.3, 0.0), KeyframeInterpolation::Linear),
                        keyframe(2.5, pos_vec(0.0, 0.0, 0.0), KeyframeInterpolation::Linear),
                    ],
                },
            ],
        },
    ],
};
