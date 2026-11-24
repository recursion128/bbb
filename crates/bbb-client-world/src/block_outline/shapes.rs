use crate::BlockPos;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_property,
    vanilla_block_state_static_outline_shape,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VANILLA_BLOCK_BAMBOO_ID, VANILLA_BLOCK_MOVING_PISTON_ID,
    VANILLA_BLOCK_POINTED_DRIPSTONE_ID,
};

use super::{BlockOutlineBox, BlockOutlineShape};

/// Blocks whose `getShape` moves a static shape by `state.getOffset(pos)`
/// (`FlowerBlock.java:46-48`, `MangrovePropaguleBlock.java:67-70`,
/// `BambooSaplingBlock.java:35-37`), all `OffsetType.XZ` with the default
/// 0.25 maximum. The generated table was sampled at `BlockPos.ZERO`.
const XZ_OFFSET_OUTLINE_BLOCKS: &[&str] = &[
    "minecraft:allium",
    "minecraft:azure_bluet",
    "minecraft:bamboo_sapling",
    "minecraft:blue_orchid",
    "minecraft:closed_eyeblossom",
    "minecraft:cornflower",
    "minecraft:dandelion",
    "minecraft:golden_dandelion",
    "minecraft:lily_of_the_valley",
    "minecraft:mangrove_propagule",
    "minecraft:open_eyeblossom",
    "minecraft:orange_tulip",
    "minecraft:oxeye_daisy",
    "minecraft:pink_tulip",
    "minecraft:poppy",
    "minecraft:red_tulip",
    "minecraft:torchflower",
    "minecraft:white_tulip",
    "minecraft:wither_rose",
];

/// Vanilla `BlockState.getShape(level, pos)` in the empty context, with a
/// full-cube default for registered `ContentRegistry` custom mod blocks.
/// `pos` selects the offset of offset-shape blocks; `None` leaves them unshifted.
/// An empty shape is `None`: nothing to outline or hit.
pub(super) fn outline_shape_for_state(
    state_id: i32,
    pos: Option<BlockPos>,
) -> Option<BlockOutlineShape> {
    outline_shape_for_state_with_registry(
        state_id,
        pos,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(super) fn outline_shape_for_state_with_registry(
    state_id: i32,
    pos: Option<BlockPos>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<BlockOutlineShape> {
    let Some(block_type_id) = vanilla_block_state_block_type_id(state_id) else {
        return (!content_registry.is_vanilla_only()
            && content_registry.block_by_state_id(state_id).is_some())
        .then(|| BlockOutlineShape::single(BlockOutlineBox::FULL));
    };
    let Some(shape) = vanilla_block_state_static_outline_shape(state_id) else {
        return dynamic_outline_shape(state_id, block_type_id, pos);
    };
    let mut boxes: Vec<_> = shape
        .boxes()
        .map(|shape_box| BlockOutlineBox {
            min: [shape_box.min_x, shape_box.min_y, shape_box.min_z],
            max: [shape_box.max_x, shape_box.max_y, shape_box.max_z],
        })
        .collect();
    if boxes.is_empty() {
        return None;
    }
    let name = vanilla_block_type(block_type_id)?.resource_id;
    if XZ_OFFSET_OUTLINE_BLOCKS.contains(&name) {
        let zero = xz_offset(BlockPos { x: 0, y: 0, z: 0 }, 0.25);
        let at = pos.map_or([0.0; 2], |pos| xz_offset(pos, 0.25));
        for shape_box in &mut boxes {
            shape_box.translate_xz(at[0] - zero[0], at[1] - zero[1]);
        }
    }
    Some(BlockOutlineShape::from_boxes(boxes))
}

/// `hasDynamicShape` blocks without a cached shape.
fn dynamic_outline_shape(
    state_id: i32,
    block_type_id: i32,
    pos: Option<BlockPos>,
) -> Option<BlockOutlineShape> {
    let offset = |max_offset| pos.map_or([0.0; 2], |pos| xz_offset(pos, max_offset));
    let column = |width: f64, min_y: f64, max_y: f64, max_offset: f64| {
        let [x, z] = offset(max_offset);
        let mut shape_box = BlockOutlineBox::centered_column(width, width, min_y, max_y);
        shape_box.translate_xz(x, z);
        Some(BlockOutlineShape::single(shape_box))
    };
    match block_type_id {
        // MovingPistonBlock answers its block entity's shape; the outline
        // owner has none, so a moving piston is not targetable.
        VANILLA_BLOCK_MOVING_PISTON_ID => None,
        // BambooStalkBlock.java:62-65.
        VANILLA_BLOCK_BAMBOO_ID => {
            let width = match vanilla_block_state_property(state_id, "leaves")? {
                "large" => 10.0,
                _ => 6.0,
            };
            column(width, 0.0, 16.0, 0.25)
        }
        // PointedDripstoneBlock.java:69-76, 260-279.
        VANILLA_BLOCK_POINTED_DRIPSTONE_ID => {
            let down = vanilla_block_state_property(state_id, "vertical_direction")? == "down";
            let (width, min_y, max_y) = match vanilla_block_state_property(state_id, "thickness")? {
                "tip_merge" => (6.0, 0.0, 16.0),
                "tip" if down => (6.0, 5.0, 16.0),
                "tip" => (6.0, 0.0, 11.0),
                "frustum" => (8.0, 0.0, 16.0),
                "middle" => (10.0, 0.0, 16.0),
                _ => (12.0, 0.0, 16.0),
            };
            column(width, min_y, max_y, 0.125)
        }
        // Shulker boxes without a block entity and powder snow are full
        // cubes; scaffolding has no held-item context here and stays full.
        _ => Some(BlockOutlineShape::single(BlockOutlineBox::FULL)),
    }
}

// BlockBehaviour.java:1230-1237; Mth.java:314-318.
fn xz_offset(pos: BlockPos, max_offset: f64) -> [f64; 2] {
    let [x, _, z] = bbb_protocol::block_state_types::vanilla_block_model_offset(
        pos.x,
        pos.z,
        [max_offset as f32, 0.0],
    );
    [x, z]
}
