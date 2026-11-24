use crate::entity_models::catalog::SignModelAttachment;
use crate::entity_models::model_layer::ModelLayer;

/// Vanilla 26.1 sign meshes: `StandingSignRenderer.createSignLayer` for the
/// standing/wall plain sign and `HangingSignRenderer.createHangingSignLayer`
/// for the three hanging attachments. No `setupAnim` — the sign model is
/// static; facing/rotation ride the root transform.
pub(in crate::entity_models) fn sign_model_layer(attachment: SignModelAttachment) -> ModelLayer {
    // The mesh is identical across woods; only the bound sprite differs.
    match attachment {
        SignModelAttachment::Standing => ModelLayer::SIGN_STANDING_OAK,
        SignModelAttachment::Wall => ModelLayer::SIGN_WALL_OAK,
        SignModelAttachment::HangingCeiling => ModelLayer::HANGING_SIGN_OAK_CEILING,
        SignModelAttachment::HangingCeilingMiddle => ModelLayer::HANGING_SIGN_OAK_CEILING_MIDDLE,
        SignModelAttachment::HangingWall => ModelLayer::HANGING_SIGN_OAK_WALL,
    }
}
