use super::*;

/// Fallback GROUND display transform for a block item (`minecraft:block/block`): rotation `0`,
/// translation `[0, 3, 0]`/16, scale `0.25`. Used only when the item's own `ground` transform was not
/// retained; otherwise the item's retained per-model ground transform drives the seating.
pub(super) const BLOCK_GROUND_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 0.0, 0.0],
    translation: [0.0, 3.0 / 16.0, 0.0],
    scale: [0.25, 0.25, 0.25],
};

/// Fallback GROUND display transform for a flat item (`minecraft:item/generated`): rotation `0`,
/// translation `[0, 2, 0]`/16, scale `0.5`.
pub(super) const GENERATED_GROUND_FALLBACK: BlockModelDisplayTransform =
    BlockModelDisplayTransform {
        rotation: [0.0, 0.0, 0.0],
        translation: [0.0, 2.0 / 16.0, 0.0],
        scale: [0.5, 0.5, 0.5],
    };

/// Fallback third-person right-hand display transform for a block item (`minecraft:block/block`):
/// rotation `[75, 45, 0]°`, translation `[0, 2.5, 0]`/16, scale `0.375`. Used only when the item's own
/// model transform was not retained (e.g. a missing model).
pub(super) const BLOCK_THIRD_PERSON_FALLBACK: BlockModelDisplayTransform =
    BlockModelDisplayTransform {
        rotation: [75.0, 45.0, 0.0],
        translation: [0.0, 2.5 / 16.0, 0.0],
        scale: [0.375, 0.375, 0.375],
    };

/// Fallback third-person right-hand display transform for a flat item (`minecraft:item/generated`):
/// rotation `[0, 0, 0]°`, translation `[0, 3, 1]`/16, scale `0.55`. Handheld tools have their own angled
/// transform (`item/handheld`), retained per-item; this is only the no-model fallback.
pub(super) const GENERATED_THIRD_PERSON_FALLBACK: BlockModelDisplayTransform =
    BlockModelDisplayTransform {
        rotation: [0.0, 0.0, 0.0],
        translation: [0.0, 3.0 / 16.0, 1.0 / 16.0],
        scale: [0.55, 0.55, 0.55],
    };

/// Fallback first-person right-hand display transform for a block item
/// (`minecraft:block/block`): rotation `[0, 45, 0]°`, translation `0`, scale `0.40`.
pub(super) const BLOCK_FIRST_PERSON_RIGHT_FALLBACK: BlockModelDisplayTransform =
    BlockModelDisplayTransform {
        rotation: [0.0, 45.0, 0.0],
        translation: [0.0, 0.0, 0.0],
        scale: [0.40, 0.40, 0.40],
    };

/// Fallback first-person left-hand display transform for a block item
/// (`minecraft:block/block`): rotation `[0, 225, 0]°`, translation `0`, scale `0.40`.
pub(super) const BLOCK_FIRST_PERSON_LEFT_FALLBACK: BlockModelDisplayTransform =
    BlockModelDisplayTransform {
        rotation: [0.0, 225.0, 0.0],
        translation: [0.0, 0.0, 0.0],
        scale: [0.40, 0.40, 0.40],
    };

/// Fallback first-person display transform for a generated item
/// (`minecraft:item/generated`): rotation `[0, -90, 25]°`, translation
/// `[1.13, 3.2, 1.13]`/16, scale `0.68`. Vanilla copies this right-hand
/// transform into the missing generated left-hand slot before applying the
/// left-hand fix.
pub(super) const GENERATED_FIRST_PERSON_FALLBACK: BlockModelDisplayTransform =
    BlockModelDisplayTransform {
        rotation: [0.0, -90.0, 25.0],
        translation: [1.13 / 16.0, 3.2 / 16.0, 1.13 / 16.0],
        scale: [0.68, 0.68, 0.68],
    };

/// Fallback HEAD display transform for a block item whose model does not define one. Vanilla's
/// `block/block` parent has no `head` entry, so the item transform is the identity centered on the
/// model by [`display_matrix`].
pub(super) const BLOCK_HEAD_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 0.0, 0.0],
    translation: [0.0, 0.0, 0.0],
    scale: [1.0, 1.0, 1.0],
};

/// Fallback HEAD display transform for `builtin/generated` items: vanilla `item/generated.json`
/// rotation `[0, 180, 0]`, translation `[0, 13, 7]` pixels, scale `1`.
pub(super) const GENERATED_HEAD_FALLBACK: BlockModelDisplayTransform = BlockModelDisplayTransform {
    rotation: [0.0, 180.0, 0.0],
    translation: [0.0, 13.0 / 16.0, 7.0 / 16.0],
    scale: [1.0, 1.0, 1.0],
};
