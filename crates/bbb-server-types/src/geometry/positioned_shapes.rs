//! Block-local shapes whose horizontal offset depends on the block position.

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_BAMBOO_ID, VANILLA_BLOCK_POINTED_DRIPSTONE_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::{
    push_static_block_collision_boxes, static_block_collision_boxes, static_block_outline_boxes,
    static_block_visual_boxes, Aabb,
};

pub fn collision_boxes(state: i32, pos: BlockPos) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_collision_boxes(state, pos, &mut boxes)?;
    Some(boxes)
}

pub fn push_collision_boxes(state: i32, pos: BlockPos, out: &mut Vec<Aabb>) -> Option<()> {
    match positioned_box(state, pos, ShapeKind::Collision)? {
        Some(shape) => {
            out.push(shape);
            Some(())
        }
        None => push_static_block_collision_boxes(state, out),
    }
}

pub fn outline_boxes(state: i32, pos: BlockPos) -> Option<Vec<Aabb>> {
    boxes(state, pos, ShapeKind::Outline)
}

pub fn visual_boxes(state: i32, pos: BlockPos) -> Option<Vec<Aabb>> {
    boxes(state, pos, ShapeKind::Visual)
}

#[derive(Clone, Copy)]
enum ShapeKind {
    Collision,
    Outline,
    Visual,
}

fn boxes(state: i32, pos: BlockPos, kind: ShapeKind) -> Option<Vec<Aabb>> {
    if let Some(shape) = positioned_box(state, pos, kind)? {
        return Some(vec![shape]);
    }
    match kind {
        ShapeKind::Collision => static_block_collision_boxes(state),
        ShapeKind::Outline => static_block_outline_boxes(state),
        ShapeKind::Visual => static_block_visual_boxes(state),
    }
}

fn positioned_box(state: i32, pos: BlockPos, kind: ShapeKind) -> Option<Option<Aabb>> {
    let block_type = vanilla_block_state_block_type_id(state)?;
    let (shape, max_offset) = match block_type {
        VANILLA_BLOCK_BAMBOO_ID => {
            let identity = vanilla_block_state_identity(state)?;
            let outline_width = match identity.properties.get("leaves")?.as_str() {
                "large" => 10.0,
                "none" | "small" => 6.0,
                _ => return None,
            };
            // BambooStalkBlock.java:29-31; BambooStalkBlock.java:62-74.
            // BlockBehaviour.java:345-347.
            let width = match kind {
                ShapeKind::Outline => outline_width,
                ShapeKind::Collision | ShapeKind::Visual => 3.0,
            };
            (column(width, 0.0, 16.0), 0.25_f32)
        }
        VANILLA_BLOCK_POINTED_DRIPSTONE_ID => {
            let identity = vanilla_block_state_identity(state)?;
            let down = match identity.properties.get("vertical_direction")?.as_str() {
                "down" => true,
                "up" => false,
                _ => return None,
            };
            // PointedDripstoneBlock.java:69-76; PointedDripstoneBlock.java:260-279.
            let shape = match identity.properties.get("thickness")?.as_str() {
                "tip_merge" => column(6.0, 0.0, 16.0),
                "tip" if down => column(6.0, 5.0, 16.0),
                "tip" => column(6.0, 0.0, 11.0),
                "frustum" => column(8.0, 0.0, 16.0),
                "middle" => column(10.0, 0.0, 16.0),
                "base" => column(12.0, 0.0, 16.0),
                _ => return None,
            };
            (shape, 0.125_f32)
        }
        _ => return Some(None),
    };
    Some(Some(shape.moved(xz_offset(pos, max_offset))))
}

// Block.java:164-166; Block.java:181-189.
fn column(width: f64, min_y: f64, max_y: f64) -> Aabb {
    let half_width = width / 2.0;
    Aabb {
        min_x: (8.0 - half_width) / 16.0,
        min_y: min_y / 16.0,
        min_z: (8.0 - half_width) / 16.0,
        max_x: (8.0 + half_width) / 16.0,
        max_y: max_y / 16.0,
        max_z: (8.0 + half_width) / 16.0,
    }
}

// BlockBehaviour.java:1230-1237; Mth.java:314-318.
fn xz_offset(pos: BlockPos, max_offset: f32) -> Vec3d {
    let [x, _, z] = bbb_protocol::block_state_types::vanilla_block_model_offset(
        pos.x,
        pos.z,
        [max_offset, 0.0],
    );
    Vec3d { x, y: 0.0, z }
}

#[cfg(test)]
mod tests;
