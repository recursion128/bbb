mod geometry;
mod shapes;

#[cfg(test)]
mod tests;

use bbb_renderer::SelectionOutline;
use bbb_world::{BlockPos, BlockProbe, TerrainMaterialClass};

use geometry::{BlockOutlineBox, BlockOutlineHit, BlockOutlineShape, HorizontalDirection};
use shapes::outline_shape_for_block;

#[derive(Debug, Clone)]
pub(crate) struct BlockOutlineTarget {
    material: TerrainMaterialClass,
    outline: Option<BlockOutlineShape>,
}

impl BlockOutlineTarget {
    #[cfg(test)]
    pub(crate) fn full_block(material: TerrainMaterialClass) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox::FULL)),
        }
    }

    pub(crate) fn from_probe(probe: &BlockProbe) -> Self {
        Self {
            material: probe.material,
            outline: outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_box(material: TerrainMaterialClass, min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox { min, max })),
        }
    }

    pub(crate) fn clip(
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

pub(crate) fn selection_outline_for_probe(probe: &BlockProbe) -> Option<SelectionOutline> {
    outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .map(|outline| outline.selection_outline(probe.pos))
}

pub(crate) fn block_probe_has_full_block_shape(probe: &BlockProbe) -> bool {
    if matches!(
        probe.material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Fluid
    ) {
        return false;
    }
    outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .is_some_and(|outline| outline.is_full_block())
}

pub(crate) fn block_probe_shape_max_y(probe: &BlockProbe) -> Option<f64> {
    if matches!(probe.material, TerrainMaterialClass::Empty) {
        return Some(1.0);
    }
    outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .map(|outline| outline.max_y())
}

pub(crate) fn block_probe_shape_center_max_y(probe: &BlockProbe) -> Option<f64> {
    outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .map(|outline| outline.max_y_at_xz(0.5, 0.5))
}

pub(crate) fn block_state_shape_boxes(
    block_state: &bbb_world::BlockStateInfo,
) -> Option<Vec<([f64; 3], [f64; 3])>> {
    outline_shape_for_block(Some(&block_state.name), &block_state.properties)
        .map(|outline| outline.boxes_min_max())
}

pub(crate) fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
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
