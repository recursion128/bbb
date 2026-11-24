use crate::entity_models::model_layer::{BakedRoot, ModelLayer};
use std::f32::consts::PI;

// Vanilla bakes the book once as `ModelLayers.BOOK` (`BookModel.createBodyLayer`); both the
// enchanting-table hovering book (`EnchantTableRenderer`) and the lectern's static open book
// (`LecternRenderer`) share this single model + the `entity/enchantment/enchanting_table_book`
// sprite, differing only in the render-state animation and the root transform.

/// Vanilla `BookModel.State.forAnimation(progress, pageFlip1, pageFlip2, openness)`'s derived
/// `openness` field: `(Mth.sin(progress · 0.02) · 0.1 + 1.25) · openness`.
/// `progress` is the block entity's `time`, `openness` the (lerped) `open`.
pub(in crate::entity_models) fn book_state_openness(progress: f32, openness: f32) -> f32 {
    ((progress * 0.02).sin() * 0.1 + 1.25) * openness
}

/// Shared `BookModel` pose from the final radian openness.
pub(in crate::entity_models) fn book_model(
    open_angle: f32,
    page_flip_1: f32,
    page_flip_2: f32,
) -> BakedRoot {
    let mut root = ModelLayer::BOOK.bake();
    let page_x = open_angle.sin();
    root.child_mut("left_lid").pose.rotation[1] = PI + open_angle;
    root.child_mut("right_lid").pose.rotation[1] = -open_angle;

    let left_pages = root.child_mut("left_pages");
    left_pages.pose.rotation[1] = open_angle;
    left_pages.pose.offset[0] = page_x;

    let right_pages = root.child_mut("right_pages");
    right_pages.pose.rotation[1] = -open_angle;
    right_pages.pose.offset[0] = page_x;

    let flip_page1 = root.child_mut("flip_page1");
    flip_page1.pose.rotation[1] = open_angle - open_angle * 2.0 * page_flip_1;
    flip_page1.pose.offset[0] = page_x;

    let flip_page2 = root.child_mut("flip_page2");
    flip_page2.pose.rotation[1] = open_angle - open_angle * 2.0 * page_flip_2;
    flip_page2.pose.offset[0] = page_x;
    root
}
