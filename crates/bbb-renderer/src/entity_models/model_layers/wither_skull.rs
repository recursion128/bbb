use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, WITHER_SKULL_GRAY,
};

// Vanilla 26.1 `WitherSkullRenderer.createSkullLayer` (atlas 64×64): one `head` part at ZERO with a
// single 8×8×8 box (`addBox(-4, -8, -4, 8, 8, 8)` at `texOffs(0, 35)`). `SkullModel.setupAnim` turns
// the head by the projectile's flight `yRot`/`xRot` (`head.yRot = yRot`, `head.xRot = xRot`); since the
// single part sits at ZERO that facing is folded into `wither_skull_model_root_transform` (together with
// the renderer's `scale(-1, -1, 1)` flip). `WitherSkullRenderer` is a plain `EntityRenderer`; the
// `wither.png` / `wither_invulnerable.png` textures (and the `isDangerous` swap between them) are
// deferred, so the colored debug path renders the skull as one dark tint.

const WITHER_SKULL_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], WITHER_SKULL_GRAY)];

pub(in crate::entity_models) const WITHER_SKULL_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 0.0, 0.0], &WITHER_SKULL_CUBES, &[])];
