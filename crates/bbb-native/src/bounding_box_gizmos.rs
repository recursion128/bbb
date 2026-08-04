//! Immutable render extraction for `BlockEntityWithBoundingBoxRenderer`.
//!
//! The renderer submits only gizmos: one stroked cuboid for the renderable box
//! and, in `BOX_AND_INVISIBLE_BLOCKS` mode, a small stroked cube per invisible
//! block colour-coded by what makes it invisible. It is an off-screen renderer,
//! so `block_entity_visibility` applies its `shouldRender` here rather than
//! through the section-based gate the entity-model stream uses.

use bbb_renderer::DebugLine;
use bbb_world::{
    BlockPos, BoundingBoxRenderMode, BoundingBoxRenderableSourceState, InvisibleBlockType,
    RenderableBoxState, WorldStore,
};

use crate::block_entity_visibility::{BlockEntityVisibility, OffScreenBlockEntityRenderer};

/// `ARGB.colorFromFloat(1.0F, 0.9F, 0.9F, 0.9F)`.
const BOX_COLOR: [u8; 4] = [230, 230, 230, 255];
/// `ARGB.colorFromFloat(1.0F, 0.5F, 0.5F, 1.0F)`.
const AIR_COLOR: [u8; 4] = [128, 128, 255, 255];
/// `ARGB.colorFromFloat(1.0F, 1.0F, 0.75F, 0.75F)`.
const STRUCTURE_VOID_COLOR: [u8; 4] = [255, 191, 191, 255];
/// `-65536`.
const BARRIER_COLOR: [u8; 4] = [255, 0, 0, 255];
/// `-256`.
const LIGHT_COLOR: [u8; 4] = [255, 255, 0, 255];
/// The extra inflation vanilla gives air markers so they read as the loosest
/// of the four.
const AIR_MARKER_SCALE: f32 = 0.05;

pub(crate) fn bounding_box_gizmo_lines(
    world: &WorldStore,
    visibility: BlockEntityVisibility,
) -> Vec<DebugLine> {
    let mut lines = Vec::new();
    for source in world.bounding_box_renderable_source_states() {
        if source.mode == BoundingBoxRenderMode::None || !source.renderable_box.is_drawable() {
            continue;
        }
        if !visibility.off_screen_owner_is_visible(
            OffScreenBlockEntityRenderer::for_bounding_box_owner(source.owner_is_test_instance),
            source.pos,
        ) {
            continue;
        }
        append_bounding_box(&mut lines, source.pos, source.renderable_box);
        append_invisible_block_markers(&mut lines, &source);
    }
    lines
}

/// `Gizmos.cuboid(new AABB(pos, pos.offset(size)).move(state.blockPos),
/// GizmoStyle.stroke(...), true)`.
fn append_bounding_box(
    lines: &mut Vec<DebugLine>,
    pos: BlockPos,
    renderable_box: RenderableBoxState,
) {
    let min = [
        (pos.x + renderable_box.local_pos.x) as f32,
        (pos.y + renderable_box.local_pos.y) as f32,
        (pos.z + renderable_box.local_pos.z) as f32,
    ];
    let max = std::array::from_fn(|axis| min[axis] + renderable_box.size[axis] as f32);
    lines.extend(DebugLine::cuboid_with_colored_corner(
        min,
        max,
        BOX_COLOR,
        DebugLine::DEFAULT_WIDTH,
    ));
}

/// `renderInvisibleBlocks`: a `0.45`-to-`0.55` cube per classified position,
/// inflated by `0.05` for air.
fn append_invisible_block_markers(
    lines: &mut Vec<DebugLine>,
    source: &BoundingBoxRenderableSourceState,
) {
    let [size_x, size_y, _] = source.renderable_box.size;
    let start = BlockPos {
        x: source.pos.x + source.renderable_box.local_pos.x,
        y: source.pos.y + source.renderable_box.local_pos.y,
        z: source.pos.z + source.renderable_box.local_pos.z,
    };
    for (index, invisible) in source.invisible_blocks.iter().enumerate() {
        let Some(invisible) = invisible else {
            continue;
        };
        let Ok(index) = i32::try_from(index) else {
            continue;
        };
        let plane = size_x * size_y;
        if plane <= 0 || size_x <= 0 {
            continue;
        }
        let z = index / plane;
        let y = (index % plane) / size_x;
        let x = index % size_x;
        let scale = if *invisible == InvisibleBlockType::Air {
            AIR_MARKER_SCALE
        } else {
            0.0
        };
        let corner = [
            (start.x + x) as f32,
            (start.y + y) as f32,
            (start.z + z) as f32,
        ];
        let min = corner.map(|coordinate| coordinate + 0.45 - scale);
        let max = corner.map(|coordinate| coordinate + 0.55 + scale);
        lines.extend(DebugLine::cuboid(
            min,
            max,
            invisible_block_color(*invisible),
            DebugLine::DEFAULT_WIDTH,
        ));
    }
}

fn invisible_block_color(invisible: InvisibleBlockType) -> [u8; 4] {
    match invisible {
        InvisibleBlockType::Air => AIR_COLOR,
        InvisibleBlockType::StructureVoid => STRUCTURE_VOID_COLOR,
        InvisibleBlockType::Barrier => BARRIER_COLOR,
        InvisibleBlockType::Light => LIGHT_COLOR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        mode: BoundingBoxRenderMode,
        invisible_blocks: Vec<Option<InvisibleBlockType>>,
    ) -> BoundingBoxRenderableSourceState {
        BoundingBoxRenderableSourceState {
            pos: BlockPos { x: 10, y: 5, z: 20 },
            owner_is_test_instance: false,
            mode,
            renderable_box: RenderableBoxState {
                local_pos: BlockPos { x: 0, y: 1, z: 0 },
                size: [2, 1, 1],
            },
            invisible_blocks,
        }
    }

    #[test]
    fn the_bounding_box_marks_its_three_axis_edges_at_the_min_corner() {
        let mut lines = Vec::new();
        let source = source(BoundingBoxRenderMode::Box, Vec::new());
        append_bounding_box(&mut lines, source.pos, source.renderable_box);

        assert_eq!(lines.len(), 12);
        assert_eq!(lines[0].from, [10.0, 6.0, 20.0]);
        assert_eq!(lines[0].to, [12.0, 6.0, 20.0]);
        // `ARGB.multiply(0xFFE6E6E6, <axis tint>)`: 230 * 0x77 / 255 = 107.
        assert_eq!(lines[0].color, [230, 107, 107, 255]);
        assert_eq!(lines[1].color, [107, 230, 107, 255]);
        assert_eq!(lines[2].color, [107, 107, 230, 255]);
        // Every other edge keeps the plain stroke colour.
        assert!(lines[3..].iter().all(|line| line.color == BOX_COLOR));
    }

    #[test]
    fn invisible_block_markers_use_the_vanilla_colors_and_air_inflation() {
        let mut lines = Vec::new();
        append_invisible_block_markers(
            &mut lines,
            &source(
                BoundingBoxRenderMode::BoxAndInvisibleBlocks,
                vec![
                    Some(InvisibleBlockType::Air),
                    Some(InvisibleBlockType::Barrier),
                ],
            ),
        );

        assert_eq!(lines.len(), 24);
        // Air at local (0, 0, 0) → world (10, 6, 20), inflated by 0.05.
        assert_eq!(lines[0].color, AIR_COLOR);
        assert_close(lines[0].from, [10.4, 6.4, 20.4]);
        // Barrier at local (1, 0, 0) → world (11, 6, 20), no inflation.
        assert_eq!(lines[12].color, BARRIER_COLOR);
        assert_close(lines[12].from, [11.45, 6.45, 20.45]);
    }

    #[track_caller]
    fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
        assert!(
            (0..3).all(|axis| (actual[axis] - expected[axis]).abs() < 1e-4),
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn unclassified_positions_emit_nothing() {
        let mut lines = Vec::new();
        append_invisible_block_markers(
            &mut lines,
            &source(
                BoundingBoxRenderMode::BoxAndInvisibleBlocks,
                vec![None, None],
            ),
        );
        assert!(lines.is_empty());
    }
}
