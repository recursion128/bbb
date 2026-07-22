//! Baby axolotl keyframe definitions from Vanilla 26.1.
//!
//! `BabyAxolotlModel` bakes these seven definitions. The source also declares an unused dash
//! definition; it is intentionally absent because no model or renderer state consumes it.

use crate::entity_models::keyframe::{
    degree_vec, keyframe, pos_vec, scale_vec, AnimationChannel, AnimationDefinition,
    AnimationTarget, BoneAnimation, KeyframeInterpolation,
};

/// Vanilla `BabyAxolotlAnimation.BABY_AXOLOTL_IDLE_FLOOR`.
pub(in crate::entity_models) const BABY_AXOLOTL_IDLE_FLOOR: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 2.88,
        looping: true,
        bones: &[
            BoneAnimation {
                bone: "body",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.8,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "right_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(160.0, -10.0, -37.5),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.2, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_hind_leg",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[keyframe(
                        1.72,
                        degree_vec(360.0, 0.0, -45.0),
                        KeyframeInterpolation::CatmullRom,
                    )],
                }],
            },
            BoneAnimation {
                bone: "left_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(160.0, 10.0, 37.5),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(-0.2, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_hind_leg",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[keyframe(
                        0.0,
                        degree_vec(0.0, 0.0, 37.5),
                        KeyframeInterpolation::CatmullRom,
                    )],
                }],
            },
            BoneAnimation {
                bone: "tail",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 10.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.72,
                            degree_vec(0.0, -14.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            0.96,
                            degree_vec(0.0, -21.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.4,
                            degree_vec(0.0, -25.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.04,
                            degree_vec(0.0, -16.75, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.84,
                            degree_vec(0.0, 10.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "head",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.04,
                            degree_vec(0.0, 0.0, -6.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.32,
                            degree_vec(-6.45, -1.45, -6.5),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            1.48,
                            degree_vec(-6.45, -1.45, -6.5),
                            KeyframeInterpolation::Linear,
                        ),
                        keyframe(
                            2.84,
                            degree_vec(0.0, 0.0, -6.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "left_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 38.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.28,
                            degree_vec(0.0, 45.5, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.64,
                            degree_vec(0.0, 45.5, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.8,
                            degree_vec(0.0, 38.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "right_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.04,
                            degree_vec(0.0, -50.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.32,
                            degree_vec(0.0, -59.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.6,
                            degree_vec(0.0, -59.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.84,
                            degree_vec(0.0, -50.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "top_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(33.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.28,
                            degree_vec(47.4, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.64,
                            degree_vec(47.4, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.8,
                            degree_vec(33.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
        ],
    };

/// Vanilla `BabyAxolotlAnimation.IDLE_FLOOR_UNDERWATER`.
pub(in crate::entity_models) const IDLE_FLOOR_UNDERWATER: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 5.12,
        looping: true,
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
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                5.12,
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
                                pos_vec(0.0, 1.5, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                5.12,
                                pos_vec(0.0, 1.5, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            5.04,
                            degree_vec(190.0, -15.0, -30.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(0.3, 0.0, 0.5),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                5.12,
                                pos_vec(0.3, 0.0, 0.5),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(360.0, 0.0, -30.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(0.18, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(5.12, pos_vec(0.18, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            5.04,
                            degree_vec(190.0, 18.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(-0.1, 0.0, 0.5),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                5.12,
                                pos_vec(-0.1, 0.0, 0.5),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            5.04,
                            degree_vec(180.0, 0.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(0.0, pos_vec(-0.2, 0.0, 0.0), KeyframeInterpolation::Linear),
                            keyframe(5.12, pos_vec(-0.2, 0.0, 0.0), KeyframeInterpolation::Linear),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "tail",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.04,
                            degree_vec(0.0, -12.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.56,
                            degree_vec(0.0, 12.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.12,
                            degree_vec(0.0, -12.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "left_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 10.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            3.12,
                            degree_vec(0.0, -16.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.12,
                            degree_vec(0.0, 10.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "right_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, -16.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.08,
                            degree_vec(0.0, 12.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.12,
                            degree_vec(0.0, -16.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "top_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(10.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.56,
                            degree_vec(-16.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.12,
                            degree_vec(10.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
        ],
    };

/// Vanilla `BabyAxolotlAnimation.IDLE_UNDERWATER`.
pub(in crate::entity_models) const IDLE_UNDERWATER: AnimationDefinition = AnimationDefinition {
    length_seconds: 5.2,
    looping: true,
    bones: &[
        BoneAnimation {
            bone: "body",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(-8.9, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.6,
                            degree_vec(-2.6, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.2,
                            degree_vec(-8.9, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.6,
                            pos_vec(0.0, -0.5, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.2,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
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
                            degree_vec(-5.0, -20.0, -55.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.6,
                            degree_vec(1.0, -20.0, -65.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.2,
                            degree_vec(-5.0, -20.0, -55.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[keyframe(
                        0.0,
                        pos_vec(0.15, -0.3391, 0.0876),
                        KeyframeInterpolation::CatmullRom,
                    )],
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
                            degree_vec(60.0, -60.0, 15.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.6,
                            degree_vec(75.0, -60.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.2,
                            degree_vec(60.0, -60.0, 15.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[keyframe(
                        5.2,
                        pos_vec(0.0, -0.4, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    )],
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
                            degree_vec(0.0, 20.0, 50.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.6,
                            degree_vec(0.0, 20.0, 60.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.2,
                            degree_vec(0.0, 20.0, 50.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[keyframe(
                        2.68,
                        pos_vec(-0.15, -0.4, 0.3),
                        KeyframeInterpolation::CatmullRom,
                    )],
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
                            degree_vec(-135.0, -50.0, -330.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.6,
                            degree_vec(-160.0, -60.0, -295.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            5.2,
                            degree_vec(-135.0, -50.0, -330.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[keyframe(
                        5.2,
                        pos_vec(0.0, -0.5, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    )],
                },
            ],
        },
        BoneAnimation {
            bone: "tail",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, 15.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.6,
                        degree_vec(0.0, -32.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        5.2,
                        degree_vec(0.0, 15.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "head",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(11.7, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.6,
                        degree_vec(2.4, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        5.2,
                        degree_vec(11.7, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "left_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, -25.6, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        3.12,
                        degree_vec(0.0, 16.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        5.2,
                        degree_vec(0.0, -25.6, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "right_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, 23.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.08,
                        degree_vec(0.0, -26.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        5.2,
                        degree_vec(0.0, 23.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "top_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(-19.2, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.6,
                        degree_vec(12.3, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        5.2,
                        degree_vec(-19.2, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
    ],
};

/// Vanilla `BabyAxolotlAnimation.BABY_AXOLOTL_SWIM`.
pub(in crate::entity_models) const BABY_AXOLOTL_SWIM: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.0,
    looping: true,
    bones: &[
        BoneAnimation {
            bone: "body",
            channels: &[
                AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(8.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.24,
                            degree_vec(12.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.68,
                            degree_vec(-7.5, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.0,
                            degree_vec(8.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.52,
                            pos_vec(0.0, -1.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.96,
                            pos_vec(0.0, -0.05, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "right_front_leg",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(310.0, 70.0, 400.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.24,
                        degree_vec(330.0, 70.0, 420.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.52,
                        degree_vec(290.0, 65.0, 384.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.76,
                        degree_vec(290.0, 70.0, 380.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(310.0, 70.0, 400.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "right_hind_leg",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(105.0, -95.0, 180.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.28,
                        degree_vec(105.0, -85.0, 180.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.52,
                        degree_vec(105.0, -80.0, 180.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.76,
                        degree_vec(105.0, -95.0, 180.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(105.0, -95.0, 180.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "left_front_leg",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(-50.0, -70.0, -40.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.52,
                        degree_vec(-60.0, -68.0, -30.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.76,
                        degree_vec(-55.0, -70.0, -30.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(-50.0, -70.0, -40.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "left_hind_leg",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(70.0, -80.0, 15.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.28,
                        degree_vec(130.0, -80.0, -45.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.52,
                        degree_vec(120.0, -70.0, -35.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.76,
                        degree_vec(80.0, -70.0, 5.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(70.0, -80.0, 15.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "tail",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, 15.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.52,
                        degree_vec(0.0, -15.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(0.0, 13.36, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "head",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(-7.5, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.28,
                        degree_vec(-13.9, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.72,
                        degree_vec(14.23, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.0,
                        degree_vec(-7.5, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "left_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, -72.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.2,
                        degree_vec(0.0, -79.9, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.64,
                        degree_vec(0.0, -38.1, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(0.0, -72.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "right_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, 72.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.2,
                        degree_vec(0.0, 86.5, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.64,
                        degree_vec(0.0, 26.7, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(0.0, 72.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "top_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(-57.0, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.2,
                        degree_vec(-68.7, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.64,
                        degree_vec(-24.2, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        0.96,
                        degree_vec(-57.0, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
    ],
};

/// Vanilla `BabyAxolotlAnimation.AXOLOTL_WALK_FLOOR`.
pub(in crate::entity_models) const AXOLOTL_WALK_FLOOR: AnimationDefinition = AnimationDefinition {
    length_seconds: 2.72,
    looping: true,
    bones: &[
        BoneAnimation {
            bone: "body",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        KeyframeInterpolation::Linear,
                    ),
                    keyframe(
                        2.72,
                        degree_vec(0.0, 0.0, 0.0),
                        KeyframeInterpolation::Linear,
                    ),
                ],
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
                            degree_vec(2.5, 10.0, -30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.32,
                            degree_vec(2.5, -22.5, -30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.72,
                            degree_vec(2.5, 10.0, -30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[keyframe(
                        1.92,
                        pos_vec(0.5, 0.0, 0.8),
                        KeyframeInterpolation::CatmullRom,
                    )],
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
                            degree_vec(260.0, -10.0, 230.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.32,
                            degree_vec(90.0, -20.0, 50.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.72,
                            degree_vec(260.0, -10.0, 230.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(
                            0.0,
                            pos_vec(0.5, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.72,
                            pos_vec(0.5, 0.0, 0.0),
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
                            degree_vec(-2.5, 32.5, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.32,
                            degree_vec(-2.5, -12.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.72,
                            degree_vec(-2.5, 32.5, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[keyframe(
                        0.0,
                        pos_vec(-0.5, 0.0, 0.7),
                        KeyframeInterpolation::CatmullRom,
                    )],
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
                            degree_vec(-2.5, -30.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.32,
                            degree_vec(2.5, 12.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.72,
                            degree_vec(-2.5, -30.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
                AnimationChannel {
                    target: AnimationTarget::Position,
                    keyframes: &[
                        keyframe(
                            0.0,
                            pos_vec(-0.5, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.72,
                            pos_vec(-0.5, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                },
            ],
        },
        BoneAnimation {
            bone: "tail",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, -10.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.32,
                        degree_vec(0.0, 10.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.72,
                        degree_vec(0.0, -10.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "head",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, -7.5, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.32,
                        degree_vec(0.0, 6.7, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.72,
                        degree_vec(0.0, -7.5, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "left_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, 38.1, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.08,
                        degree_vec(0.0, 45.6, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.44,
                        degree_vec(0.0, 45.6, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.72,
                        degree_vec(0.0, 38.1, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "right_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(0.0, -50.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.08,
                        degree_vec(0.0, -59.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.36,
                        degree_vec(0.0, -59.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.72,
                        degree_vec(0.0, -50.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
        BoneAnimation {
            bone: "top_gills",
            channels: &[AnimationChannel {
                target: AnimationTarget::Rotation,
                keyframes: &[
                    keyframe(
                        0.0,
                        degree_vec(33.0, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.08,
                        degree_vec(47.4, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        1.44,
                        degree_vec(47.4, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                    keyframe(
                        2.72,
                        degree_vec(33.0, 0.0, 0.0),
                        KeyframeInterpolation::CatmullRom,
                    ),
                ],
            }],
        },
    ],
};

/// Vanilla `BabyAxolotlAnimation.WALK_FLOOR_UNDERWATER`.
pub(in crate::entity_models) const WALK_FLOOR_UNDERWATER: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 2.04,
        looping: true,
        bones: &[
            BoneAnimation {
                bone: "body",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[
                            keyframe(
                                0.0,
                                degree_vec(0.0, -5.0, 6.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.04,
                                degree_vec(0.0, 5.0, -4.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                degree_vec(0.0, -5.0, 6.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(0.0, 1.5, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                pos_vec(0.0, 1.5, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
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
                                degree_vec(0.0, -15.0, -30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.72,
                                degree_vec(0.0, 9.0, -30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.12,
                                degree_vec(0.0, 13.0, -24.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.44,
                                degree_vec(0.0, 9.0, -30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                degree_vec(0.0, -15.0, -30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.96,
                                pos_vec(0.4, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                pos_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
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
                                degree_vec(95.0, -20.0, 70.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.68,
                                degree_vec(255.0, -20.0, 230.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.12,
                                degree_vec(280.0, -25.0, 255.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.52,
                                degree_vec(255.0, -20.0, 230.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                degree_vec(95.0, -20.0, 70.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(0.5, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.96,
                                pos_vec(0.5, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                pos_vec(0.5, 0.0, 0.0),
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
                                degree_vec(-5.0, -20.0, 30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.72,
                                degree_vec(0.0, 10.0, 25.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.44,
                                degree_vec(0.0, 10.0, 25.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                degree_vec(-5.0, -20.0, 30.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(-0.5, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.96,
                                pos_vec(0.0, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                pos_vec(-0.5, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
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
                                degree_vec(5.0, 15.0, 25.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.72,
                                degree_vec(-10.0, -20.0, 35.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                1.44,
                                degree_vec(-10.0, -20.0, 35.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                degree_vec(5.0, 15.0, 25.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[
                            keyframe(
                                0.0,
                                pos_vec(-0.7, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                0.96,
                                pos_vec(-0.4, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                            keyframe(
                                2.04,
                                pos_vec(-0.7, 0.0, 0.0),
                                KeyframeInterpolation::CatmullRom,
                            ),
                        ],
                    },
                ],
            },
            BoneAnimation {
                bone: "tail",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 16.5, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.56,
                            degree_vec(0.0, 1.12, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.44,
                            degree_vec(0.0, 17.5, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.04,
                            degree_vec(0.0, 16.5, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "head",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 4.0, -5.8),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.04,
                            degree_vec(0.0, -4.0, 5.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.04,
                            degree_vec(0.0, 4.0, -5.8),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "left_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, -60.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.24,
                            degree_vec(0.0, -45.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.04,
                            degree_vec(0.0, -60.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "right_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(0.0, 38.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            0.64,
                            degree_vec(0.0, 53.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.04,
                            degree_vec(0.0, 38.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
            BoneAnimation {
                bone: "top_gills",
                channels: &[AnimationChannel {
                    target: AnimationTarget::Rotation,
                    keyframes: &[
                        keyframe(
                            0.0,
                            degree_vec(-34.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            1.04,
                            degree_vec(-41.5, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                        keyframe(
                            2.04,
                            degree_vec(-34.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        ),
                    ],
                }],
            },
        ],
    };

/// Vanilla `BabyAxolotlAnimation.BABY_AXOLOTL_PLAY_DEAD`.
pub(in crate::entity_models) const BABY_AXOLOTL_PLAY_DEAD: AnimationDefinition =
    AnimationDefinition {
        length_seconds: 0.04,
        looping: false,
        bones: &[
            BoneAnimation {
                bone: "body",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(0.0, 0.0, 30.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(202.35, -30.33, -40.82),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.2, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(418.0, -50.0, 35.8),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_front_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(177.0, 22.76, 15.9),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(-0.2, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_hind_leg",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(16.4287, -37.6467, 16.9822),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "tail",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(0.0, -18.67, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "head",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(-4.21, -0.95, -6.33),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "left_gills",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(0.0, 43.21, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "right_gills",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(0.0, -55.87, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "top_gills",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(43.0, 0.0, 0.0),
                            KeyframeInterpolation::CatmullRom,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
            BoneAnimation {
                bone: "root",
                channels: &[
                    AnimationChannel {
                        target: AnimationTarget::Rotation,
                        keyframes: &[keyframe(
                            0.0,
                            degree_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Position,
                        keyframes: &[keyframe(
                            0.0,
                            pos_vec(0.0, 0.0, 0.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                    AnimationChannel {
                        target: AnimationTarget::Scale,
                        keyframes: &[keyframe(
                            0.0,
                            scale_vec(1.0, 1.0, 1.0),
                            KeyframeInterpolation::Linear,
                        )],
                    },
                ],
            },
        ],
    };
