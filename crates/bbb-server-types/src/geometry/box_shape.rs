//! Box-shape boolean operations delegating to `bbb_facts::collision::VoxelBoxShape`.

use super::Aabb;
use bbb_facts::block_state_types::VanillaBlockSupportDirection;
use bbb_facts::collision::VoxelBoxShape;
use bbb_protocol::packets::{Direction, Vec3d};

#[derive(Clone, Debug)]
pub struct BoxShape(VoxelBoxShape);

impl BoxShape {
    pub fn empty() -> Self {
        Self(VoxelBoxShape::empty())
    }

    pub fn from_box(bounds: Aabb) -> Option<Self> {
        VoxelBoxShape::from_box(bounds).map(Self)
    }

    pub fn from_boxes(boxes: &[Aabb]) -> Option<Self> {
        VoxelBoxShape::from_boxes(boxes).map(Self)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn union(&self, other: &Self) -> Option<Self> {
        self.0.union(&other.0).map(Self)
    }

    pub fn only_first(&self, other: &Self) -> Option<Self> {
        self.0.only_first(&other.0).map(Self)
    }

    pub fn optimized(&self) -> Option<Self> {
        self.0.optimized().map(Self)
    }

    pub fn intersects(&self, other: &Self) -> Option<bool> {
        self.0.intersects(&other.0)
    }

    pub fn moved(&self, offset: Vec3d) -> Option<Self> {
        self.0.moved([offset.x, offset.y, offset.z]).map(Self)
    }

    pub fn bounds(&self) -> Option<Aabb> {
        self.0.bounds().map(Aabb::from)
    }

    pub fn boxes(&self) -> Vec<Aabb> {
        self.0.boxes().into_iter().map(Aabb::from).collect()
    }

    pub fn closest_point(&self, point: Vec3d) -> Option<Vec3d> {
        self.0
            .closest_point([point.x, point.y, point.z])
            .map(|[x, y, z]| Vec3d { x, y, z })
    }
}

pub fn collision_face_full(boxes: &[Aabb], face: Direction) -> Option<bool> {
    let support = match face {
        Direction::Down => VanillaBlockSupportDirection::Down,
        Direction::Up => VanillaBlockSupportDirection::Up,
        Direction::North => VanillaBlockSupportDirection::North,
        Direction::South => VanillaBlockSupportDirection::South,
        Direction::West => VanillaBlockSupportDirection::West,
        Direction::East => VanillaBlockSupportDirection::East,
    };
    bbb_facts::collision::collision_face_full(boxes, support)
}
