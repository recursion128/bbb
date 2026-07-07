use std::f32::consts::{FRAC_PI_2, PI};

use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla bakes the book once as `ModelLayers.BOOK` (`BookModel.createBodyLayer`); both the
// enchanting-table hovering book (`EnchantTableRenderer`) and the lectern's static open book
// (`LecternRenderer`) share this single model + the `entity/enchantment/enchanting_table_book`
// sprite, differing only in the render-state animation and the root transform.
pub(in crate::entity_models) const MODEL_LAYER_BOOK: &str = "minecraft:book";

/// The colored debug tint for the book's parchment/leather; the textured path binds the real
/// `enchanting_table_book` sprite, so this only surfaces in the colored fallback.
pub(in crate::entity_models) const BOOK_PAPER: [f32; 4] = [0.86, 0.80, 0.63, 1.0];

// Vanilla 26.1 `BookModel.createBodyLayer` (`BookModel.java:35-53`, atlas 64×32).
//
// `left_lid` texOffs(0, 0) box (-6, -5, -0.005) size (6, 10, 0.005) at PartPose.offset(0, 0, -1).
pub(in crate::entity_models) const BOOK_LEFT_LID_CUBE: ModelCube = ModelCube::new(
    [-6.0, -5.0, -0.005],
    [6.0, 10.0, 0.005],
    BOOK_PAPER,
    [6.0, 10.0, 0.005],
    [0.0, 0.0],
    false,
);
/// `right_lid` texOffs(16, 0) box (0, -5, -0.005) size (6, 10, 0.005) at PartPose.offset(0, 0, 1).
pub(in crate::entity_models) const BOOK_RIGHT_LID_CUBE: ModelCube = ModelCube::new(
    [0.0, -5.0, -0.005],
    [6.0, 10.0, 0.005],
    BOOK_PAPER,
    [6.0, 10.0, 0.005],
    [16.0, 0.0],
    false,
);
/// `seam` texOffs(12, 0) box (-1, -5, 0) size (2, 10, 0.005) at PartPose.rotation(0, π/2, 0).
pub(in crate::entity_models) const BOOK_SEAM_CUBE: ModelCube = ModelCube::new(
    [-1.0, -5.0, 0.0],
    [2.0, 10.0, 0.005],
    BOOK_PAPER,
    [2.0, 10.0, 0.005],
    [12.0, 0.0],
    false,
);
/// `left_pages` texOffs(0, 10) box (0, -4, -0.99) size (5, 8, 1) at PartPose.ZERO.
pub(in crate::entity_models) const BOOK_LEFT_PAGES_CUBE: ModelCube = ModelCube::new(
    [0.0, -4.0, -0.99],
    [5.0, 8.0, 1.0],
    BOOK_PAPER,
    [5.0, 8.0, 1.0],
    [0.0, 10.0],
    false,
);
/// `right_pages` texOffs(12, 10) box (0, -4, -0.01) size (5, 8, 1) at PartPose.ZERO.
pub(in crate::entity_models) const BOOK_RIGHT_PAGES_CUBE: ModelCube = ModelCube::new(
    [0.0, -4.0, -0.01],
    [5.0, 8.0, 1.0],
    BOOK_PAPER,
    [5.0, 8.0, 1.0],
    [12.0, 10.0],
    false,
);
/// `flip_page1` / `flip_page2` share texOffs(24, 10) box (0, -4, 0) size (5, 8, 0.005) at
/// PartPose.ZERO.
pub(in crate::entity_models) const BOOK_FLIP_PAGE_CUBE: ModelCube = ModelCube::new(
    [0.0, -4.0, 0.0],
    [5.0, 8.0, 0.005],
    BOOK_PAPER,
    [5.0, 8.0, 0.005],
    [24.0, 10.0],
    false,
);

/// Vanilla `PartPose.offset(0, 0, -1)` — the left cover pivot.
pub(in crate::entity_models) const BOOK_LEFT_LID_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Vanilla `PartPose.offset(0, 0, 1)` — the right cover pivot.
pub(in crate::entity_models) const BOOK_RIGHT_LID_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Vanilla `PartPose.rotation(0, π/2, 0)` — the static spine seam.
pub(in crate::entity_models) const BOOK_SEAM_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, FRAC_PI_2, 0.0],
};

/// Vanilla `BookModel.State.forAnimation(progress, pageFlip1, pageFlip2, openness)`'s derived
/// `openness` field: `(Mth.sin(progress · 0.02) · 0.1 + 1.25) · openness` (`BookModel.java:71-73`).
/// `progress` is the block entity's `time`, `openness` the (lerped) `open`.
pub(in crate::entity_models) fn book_state_openness(progress: f32, openness: f32) -> f32 {
    ((progress * 0.02).sin() * 0.1 + 1.25) * openness
}

/// Shared `BookModel` (`ModelLayers.BOOK`): the two covers, the two page halves, the two flip
/// pages, and the static spine seam. `setup_anim` transcribes `BookModel.setupAnim`
/// (`BookModel.java:55-68`) over the projected `book_openness` / `book_page_flip_1` /
/// `book_page_flip_2` render state.
pub(in crate::entity_models) struct BookModel {
    root: ModelPart,
}

impl BookModel {
    pub(in crate::entity_models) fn new() -> Self {
        let children: Vec<(&'static str, ModelPart)> = vec![
            (
                "left_lid",
                ModelPart::leaf(BOOK_LEFT_LID_POSE, vec![BOOK_LEFT_LID_CUBE]),
            ),
            (
                "right_lid",
                ModelPart::leaf(BOOK_RIGHT_LID_POSE, vec![BOOK_RIGHT_LID_CUBE]),
            ),
            (
                "seam",
                ModelPart::leaf(BOOK_SEAM_POSE, vec![BOOK_SEAM_CUBE]),
            ),
            (
                "left_pages",
                ModelPart::leaf(PART_POSE_ZERO, vec![BOOK_LEFT_PAGES_CUBE]),
            ),
            (
                "right_pages",
                ModelPart::leaf(PART_POSE_ZERO, vec![BOOK_RIGHT_PAGES_CUBE]),
            ),
            (
                "flip_page1",
                ModelPart::leaf(PART_POSE_ZERO, vec![BOOK_FLIP_PAGE_CUBE]),
            ),
            (
                "flip_page2",
                ModelPart::leaf(PART_POSE_ZERO, vec![BOOK_FLIP_PAGE_CUBE]),
            ),
        ];
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for BookModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // `BookModel.State.forAnimation` derives the openness from the block
        // entity's animation progress and its raw openness; `setupAnim` then
        // poses the parts from that single value.
        let openness = book_state_openness(
            instance.render_state.book_progress,
            instance.render_state.book_open,
        );
        let page_flip_1 = instance.render_state.book_page_flip_1;
        let page_flip_2 = instance.render_state.book_page_flip_2;
        let page_x = openness.sin();

        self.root.child_mut("left_lid").pose.rotation[1] = PI + openness;
        self.root.child_mut("right_lid").pose.rotation[1] = -openness;

        let left_pages = self.root.child_mut("left_pages");
        left_pages.pose.rotation[1] = openness;
        left_pages.pose.offset[0] = page_x;

        let right_pages = self.root.child_mut("right_pages");
        right_pages.pose.rotation[1] = -openness;
        right_pages.pose.offset[0] = page_x;

        let flip_page1 = self.root.child_mut("flip_page1");
        flip_page1.pose.rotation[1] = openness - openness * 2.0 * page_flip_1;
        flip_page1.pose.offset[0] = page_x;

        let flip_page2 = self.root.child_mut("flip_page2");
        flip_page2.pose.rotation[1] = openness - openness * 2.0 * page_flip_2;
        flip_page2.pose.offset[0] = page_x;
    }
}
