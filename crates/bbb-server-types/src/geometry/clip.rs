use bbb_facts::collision::{BlockFace, RayClipHit};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};

use super::Aabb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipHit {
    pub pos: BlockPos,
    pub direction: Direction,
    pub location: Vec3d,
}

#[cfg(test)]
mod tests;

fn to_direction(face: BlockFace) -> Direction {
    match face {
        BlockFace::Down => Direction::Down,
        BlockFace::Up => Direction::Up,
        BlockFace::North => Direction::North,
        BlockFace::South => Direction::South,
        BlockFace::West => Direction::West,
        BlockFace::East => Direction::East,
    }
}

fn from_ray_clip_hit(hit: RayClipHit) -> ClipHit {
    ClipHit {
        pos: BlockPos {
            x: hit.pos[0],
            y: hit.pos[1],
            z: hit.pos[2],
        },
        direction: to_direction(hit.face),
        location: Vec3d {
            x: hit.location[0],
            y: hit.location[1],
            z: hit.location[2],
        },
    }
}

// BlockGetter.java:63-80,103-171.
pub fn clip_with<'a>(
    from: Vec3d,
    to: Vec3d,
    mut cell: impl FnMut(BlockPos) -> Option<(Vec<Aabb>, Option<&'a Aabb>)>,
) -> Option<Option<ClipHit>> {
    let hit = bbb_facts::collision::clip_ray_with(
        [from.x, from.y, from.z],
        [to.x, to.y, to.z],
        |[x, y, z]| {
            let (blocks, fluid) = cell(BlockPos { x, y, z })?;
            Some((blocks, fluid.copied()))
        },
    )?;
    Some(hit.map(from_ray_clip_hit))
}

// VoxelShape.java:158-177; AABB.java:310-327.
pub fn shape_clip(boxes: &[Aabb], from: Vec3d, to: Vec3d, pos: BlockPos) -> Option<ClipHit> {
    bbb_facts::collision::shape_clip(
        boxes,
        [from.x, from.y, from.z],
        [to.x, to.y, to.z],
        [pos.x, pos.y, pos.z],
    )
    .map(from_ray_clip_hit)
}
