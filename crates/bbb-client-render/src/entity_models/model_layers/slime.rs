use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `LavaSlimeModel.createBodyLayer` segment child names, `cube0..cube7`.
const MAGMA_CUBE_SEGMENT_NAMES: [&str; 8] = [
    "cube0", "cube1", "cube2", "cube3", "cube4", "cube5", "cube6", "cube7",
];

/// Mutable magma cube model, mirroring vanilla `LavaSlimeModel`. The unified tree is built once with
/// named children: eight stacked outer segments (`cube0..cube7`) plus the inner `inside_cube` core.
/// Vanilla `LavaSlimeModel.setupAnim` fans the eight segments apart vertically by the interpolated,
/// non-negative `squish` (`cubeN.y = -(4 - N) * max(0, squish) * 1.7`), reconstructed client-side from
/// the `onGround()` jump transitions and projected onto the render state. The overall non-uniform body
/// scale lives in the root transform (`magma_cube_model_root_transform`).
pub(in crate::entity_models) fn magma_cube_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::MAGMA_CUBE.bake();
    // Vanilla `LavaSlimeModel.setupAnim`: only the non-negative squish (the jump
    // stretch, never the landing splat) spreads the eight stacked segments apart
    // vertically — `cubeN.y = -(4 - N) * max(0, squish) * 1.7`. The lower segments
    // (N < 4) sink and the upper ones (N > 4) rise, opening gaps between the lava
    // slices. The overall body scale is applied separately by the root transform.
    let slime_squish = instance.render_state.kind.slime().squish.max(0.0);
    for (i, &name) in MAGMA_CUBE_SEGMENT_NAMES.iter().enumerate() {
        root.child_mut(name).pose.offset[1] = -((4 - i as i32) as f32) * slime_squish * 1.7;
    }
    root
}
