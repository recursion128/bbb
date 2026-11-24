mod geometry;
mod shapes;

#[cfg(test)]
mod tests;

use bbb_render_types::SelectionOutline;

use crate::{BlockPos, BlockProbe, BlockStateInfo, BlockView, TerrainMaterialClass, WorldStore};

use geometry::{BlockOutlineBox, BlockOutlineHit, BlockOutlineShape};
use shapes::{outline_shape_for_state, outline_shape_for_state_with_registry};

#[derive(Debug, Clone)]
pub struct BlockOutlineTarget {
    material: TerrainMaterialClass,
    outline: Option<BlockOutlineShape>,
}

impl BlockOutlineTarget {
    pub fn full_block(material: TerrainMaterialClass) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox::FULL)),
        }
    }

    pub fn from_world_view(world: &WorldStore, block: &BlockView) -> Self {
        Self {
            material: block.material,
            outline: outline_shape_for_world_view(world, block),
        }
    }

    pub fn from_fluid_height(height: f64) -> Self {
        Self {
            material: TerrainMaterialClass::Translucent,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, height.clamp(0.0, 1.0), 1.0],
            })),
        }
    }

    pub fn from_world_view_with_fluid_height(
        world: &WorldStore,
        block: &BlockView,
        height: f64,
    ) -> Self {
        let mut boxes = if is_selectable_crosshair_material(block.material) {
            outline_shape_for_world_view(world, block)
                .map(|shape| {
                    shape
                        .boxes_min_max()
                        .into_iter()
                        .map(|(min, max)| BlockOutlineBox { min, max })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        boxes.push(BlockOutlineBox {
            min: [0.0, 0.0, 0.0],
            max: [1.0, height.clamp(0.0, 1.0), 1.0],
        });
        Self {
            material: TerrainMaterialClass::Translucent,
            outline: Some(BlockOutlineShape::from_boxes(boxes)),
        }
    }

    pub fn from_box(material: TerrainMaterialClass, min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox { min, max })),
        }
    }

    pub fn clip(
        self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        if !is_selectable_crosshair_material(self.material) {
            return None;
        }
        self.outline?.clip(eye, direction, max_distance, pos)
    }
}

fn outline_shape_for_world_view(
    world: &WorldStore,
    block: &BlockView,
) -> Option<BlockOutlineShape> {
    match world.local_player_contextual_outline_shape_boxes(block) {
        Some(boxes) => Some(BlockOutlineShape::from_boxes(
            boxes
                .into_iter()
                .map(|(min, max)| BlockOutlineBox { min, max })
                .collect(),
        )),
        None => outline_shape_for_state_with_registry(
            block.block_state_id,
            Some(block.pos),
            world.content_registry(),
        ),
    }
}

pub fn selection_outline_for_world_view(
    world: &WorldStore,
    block: &BlockView,
) -> Option<SelectionOutline> {
    outline_shape_for_world_view(world, block).map(|outline| outline.selection_outline(block.pos))
}

pub fn block_probe_has_full_block_shape(probe: &BlockProbe) -> bool {
    if matches!(
        probe.material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Fluid
    ) {
        return false;
    }
    outline_shape_for_state(probe.block_state_id, Some(probe.pos))
        .is_some_and(|outline| outline.is_full_block())
}

pub fn block_probe_shape_max_y(probe: &BlockProbe) -> Option<f64> {
    if matches!(probe.material, TerrainMaterialClass::Empty) {
        return Some(1.0);
    }
    outline_shape_for_state(probe.block_state_id, Some(probe.pos)).map(|outline| outline.max_y())
}

pub fn block_probe_shape_center_max_y(probe: &BlockProbe) -> Option<f64> {
    outline_shape_for_state(probe.block_state_id, Some(probe.pos))
        .map(|outline| outline.max_y_at_xz(0.5, 0.5))
}

pub fn block_state_shape_boxes(block_state: &BlockStateInfo) -> Option<Vec<([f64; 3], [f64; 3])>> {
    outline_shape_for_state(block_state.id, None).map(|outline| outline.boxes_min_max())
}

pub fn block_state_shape_boxes_at(
    block_state: &BlockStateInfo,
    pos: BlockPos,
) -> Option<Vec<([f64; 3], [f64; 3])>> {
    outline_shape_for_state(block_state.id, Some(pos)).map(|outline| outline.boxes_min_max())
}

pub fn block_probe_shape_boxes(probe: &BlockProbe) -> Option<Vec<([f64; 3], [f64; 3])>> {
    if matches!(
        probe.material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Fluid
    ) {
        return None;
    }
    outline_shape_for_state(probe.block_state_id, Some(probe.pos))
        .map(|outline| outline.boxes_min_max())
}

pub fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
    selection_outline_for_box(pos, BlockOutlineBox::FULL)
}

fn selection_outline_for_box(pos: BlockPos, outline: BlockOutlineBox) -> SelectionOutline {
    BlockOutlineShape::single(outline).selection_outline(pos)
}

fn is_selectable_crosshair_material(material: TerrainMaterialClass) -> bool {
    matches!(
        material,
        TerrainMaterialClass::Invisible
            | TerrainMaterialClass::Opaque
            | TerrainMaterialClass::Cutout
            | TerrainMaterialClass::Translucent
    )
}
