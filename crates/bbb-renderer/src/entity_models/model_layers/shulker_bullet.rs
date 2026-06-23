use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, SHULKER_BULLET_COLOR,
};

// Vanilla 26.1 `ShulkerBulletModel.createBodyLayer` (atlas 64×32). The mesh root holds a single
// `main` part at ZERO with three interlocking slabs (one flat in each of the XY / YZ / XZ planes):
// `texOffs(0, 0)` 8×8×2, `texOffs(0, 10)` 2×8×8, and `texOffs(20, 0)` 8×2×8. `setupAnim` only sets
// `main.yRot`/`main.xRot` from the bullet's facing (reproduced through `body_rot` / `head_pitch` in
// `shulker_bullet_model_root_transform`), so the geometry is complete. `ShulkerBulletRenderer` is a
// plain `EntityRenderer`; its constant `translate(0, 0.15, 0)` + `scale(-0.5, -0.5, 0.5)` are
// reproduced, while the `ageInTicks`-driven tumble (`Ry(sin(t·0.1)·180) · Rx(cos(t·0.1)·180) ·
// Rz(sin(t·0.15)·360)`) and the second translucent 1.5× outer-shell pass remain deferred. Only the
// texture-backed path is deferred too, so the colored debug path renders the slabs with one tint.

const SHULKER_BULLET_CUBES: [ModelCubeDesc; 3] = [
    cube([-4.0, -4.0, -1.0], [8.0, 8.0, 2.0], SHULKER_BULLET_COLOR),
    cube([-1.0, -4.0, -4.0], [2.0, 8.0, 8.0], SHULKER_BULLET_COLOR),
    cube([-4.0, -1.0, -4.0], [8.0, 2.0, 8.0], SHULKER_BULLET_COLOR),
];

pub(in crate::entity_models) const SHULKER_BULLET_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 0.0, 0.0], &SHULKER_BULLET_CUBES, &[])];
