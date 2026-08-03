use crate::post_effect_graph::VanillaPostEffect;

const CREEPER_DRAW_CALLS: u64 = 2;
const SPIDER_DRAW_CALLS: u64 = 10;
const INVERT_DRAW_CALLS: u64 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct PostUniform {
    p0: [f32; 4],
    p1: [f32; 4],
    p2: [f32; 4],
    p3: [f32; 4],
}

impl PostUniform {
    const fn new(p0: [f32; 4], p1: [f32; 4], p2: [f32; 4], p3: [f32; 4]) -> Self {
        Self { p0, p1, p2, p3 }
    }
}

const ZERO: [f32; 4] = [0.0; 4];
pub(super) const CREEPER_COLOR_UNIFORM: PostUniform = PostUniform::new(
    [0.0, 0.0, 0.0, 0.0],
    [0.3, 0.59, 0.11, 0.0],
    [0.0, 0.0, 0.0, 0.0],
    ZERO,
);
pub(super) const CREEPER_BITS_UNIFORM: PostUniform =
    PostUniform::new([16.0, 4.0, 0.0, 0.0], ZERO, ZERO, ZERO);
pub(super) const INVERT_UNIFORM: PostUniform =
    PostUniform::new([0.8, 0.0, 0.0, 0.0], ZERO, ZERO, ZERO);
pub(super) const IDENTITY_BLIT_UNIFORM: PostUniform =
    PostUniform::new([1.0, 1.0, 1.0, 1.0], ZERO, ZERO, ZERO);
pub(super) const SPIDER_BLIT_UNIFORM: PostUniform =
    PostUniform::new([1.0, 0.8, 0.8, 1.0], ZERO, ZERO, ZERO);
pub(super) const SPIDER_PASS_UNIFORMS: [PostUniform; 5] = [
    PostUniform::new(
        [1.25, 2.0, -0.125, -0.1],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.1, 0.1, 0.9, 0.9],
    ),
    PostUniform::new(
        [2.35, 4.2, -1.1, -1.5],
        [-45.0, 0.0, 0.0, 0.0],
        [0.21, 0.0, 0.79, 1.0],
        [0.31, 0.1, 0.69, 0.9],
    ),
    PostUniform::new(
        [2.35, 4.2, 0.45, -4.45],
        [45.0, 0.0, 0.0, 0.0],
        [0.21, 0.0, 0.79, 1.0],
        [0.31, 0.1, 0.69, 0.9],
    ),
    PostUniform::new(
        [2.35, 2.35, -0.385, -1.29],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.31, 0.1, 0.69, 0.9],
    ),
    PostUniform::new(
        [2.35, 2.35, -0.965, -1.29],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.31, 0.1, 0.69, 0.9],
    ),
];

pub(super) fn camera_post_effect_draw_calls(effect: VanillaPostEffect) -> u64 {
    match effect {
        VanillaPostEffect::Creeper => CREEPER_DRAW_CALLS,
        VanillaPostEffect::Spider => SPIDER_DRAW_CALLS,
        VanillaPostEffect::Invert => INVERT_DRAW_CALLS,
        _ => unreachable!("camera post-effect resolver only returns camera effects"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_post_effect_graph_draw_counts_are_exact() {
        assert_eq!(camera_post_effect_draw_calls(VanillaPostEffect::Creeper), 2);
        assert_eq!(camera_post_effect_draw_calls(VanillaPostEffect::Spider), 10);
        assert_eq!(camera_post_effect_draw_calls(VanillaPostEffect::Invert), 2);
    }

    #[test]
    fn camera_post_effect_uniforms_match_vanilla_26_1_graphs() {
        assert_eq!(CREEPER_COLOR_UNIFORM.p0, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(CREEPER_COLOR_UNIFORM.p1, [0.3, 0.59, 0.11, 0.0]);
        assert_eq!(CREEPER_COLOR_UNIFORM.p2, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(CREEPER_BITS_UNIFORM.p0[..2], [16.0, 4.0]);
        assert_eq!(INVERT_UNIFORM.p0[0], 0.8);
        assert_eq!(SPIDER_BLIT_UNIFORM.p0, [1.0, 0.8, 0.8, 1.0]);
        assert_eq!(
            SPIDER_PASS_UNIFORMS,
            [
                PostUniform::new(
                    [1.25, 2.0, -0.125, -0.1],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 1.0],
                    [0.1, 0.1, 0.9, 0.9],
                ),
                PostUniform::new(
                    [2.35, 4.2, -1.1, -1.5],
                    [-45.0, 0.0, 0.0, 0.0],
                    [0.21, 0.0, 0.79, 1.0],
                    [0.31, 0.1, 0.69, 0.9],
                ),
                PostUniform::new(
                    [2.35, 4.2, 0.45, -4.45],
                    [45.0, 0.0, 0.0, 0.0],
                    [0.21, 0.0, 0.79, 1.0],
                    [0.31, 0.1, 0.69, 0.9],
                ),
                PostUniform::new(
                    [2.35, 2.35, -0.385, -1.29],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 1.0],
                    [0.31, 0.1, 0.69, 0.9],
                ),
                PostUniform::new(
                    [2.35, 2.35, -0.965, -1.29],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 1.0],
                    [0.31, 0.1, 0.69, 0.9],
                ),
            ]
        );
    }
}
