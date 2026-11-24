use bbb_render_types::{SelectionBox, SelectionOutline};

use crate::BlockPos;
use bbb_protocol::packets::Direction as ProtocolDirection;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum BlockOutlineShape {
    Single(BlockOutlineBox),
    Multi(Vec<BlockOutlineBox>),
}

impl BlockOutlineShape {
    pub(super) fn single(outline: BlockOutlineBox) -> Self {
        Self::Single(outline)
    }

    pub(super) fn from_boxes(mut boxes: Vec<BlockOutlineBox>) -> Self {
        if boxes.len() == 1 {
            Self::Single(boxes.pop().expect("one outline box"))
        } else {
            Self::Multi(boxes)
        }
    }

    fn boxes(&self) -> &[BlockOutlineBox] {
        match self {
            Self::Single(outline) => std::slice::from_ref(outline),
            Self::Multi(boxes) => boxes,
        }
    }

    pub(super) fn clip(
        &self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        self.boxes()
            .iter()
            .filter_map(|outline| outline.clip(eye, direction, max_distance, pos))
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    pub(super) fn selection_outline(&self, pos: BlockPos) -> SelectionOutline {
        SelectionOutline::from_boxes(
            self.boxes()
                .iter()
                .map(|outline| selection_box_for_outline_box(pos, *outline)),
        )
    }

    pub(super) fn is_full_block(&self) -> bool {
        matches!(self, Self::Single(outline) if *outline == BlockOutlineBox::FULL)
    }

    pub(super) fn max_y(&self) -> f64 {
        self.boxes()
            .iter()
            .map(|outline| outline.max[1])
            .fold(0.0, f64::max)
    }

    pub(super) fn max_y_at_xz(&self, x: f64, z: f64) -> f64 {
        self.boxes()
            .iter()
            .filter(|outline| outline.contains_xz(x, z))
            .map(|outline| outline.max[1])
            .fold(0.0, f64::max)
    }

    pub(super) fn boxes_min_max(&self) -> Vec<([f64; 3], [f64; 3])> {
        self.boxes()
            .iter()
            .map(|outline| (outline.min, outline.max))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BlockOutlineBox {
    pub(super) min: [f64; 3],
    pub(super) max: [f64; 3],
}

impl BlockOutlineBox {
    pub(super) const FULL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) fn centered_column(size_x: f64, size_z: f64, min_y: f64, max_y: f64) -> Self {
        let half_x = size_x / 2.0;
        let half_z = size_z / 2.0;
        Self {
            min: [(8.0 - half_x) / 16.0, min_y / 16.0, (8.0 - half_z) / 16.0],
            max: [(8.0 + half_x) / 16.0, max_y / 16.0, (8.0 + half_z) / 16.0],
        }
    }

    pub(super) fn translate_xz(&mut self, x: f64, z: f64) {
        self.min[0] += x;
        self.max[0] += x;
        self.min[2] += z;
        self.max[2] += z;
    }

    fn contains_xz(self, x: f64, z: f64) -> bool {
        x >= self.min[0] && x <= self.max[0] && z >= self.min[2] && z <= self.max[2]
    }

    fn clip(
        self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        let min = [
            f64::from(pos.x) + self.min[0],
            f64::from(pos.y) + self.min[1],
            f64::from(pos.z) + self.min[2],
        ];
        let max = [
            f64::from(pos.x) + self.max[0],
            f64::from(pos.y) + self.max[1],
            f64::from(pos.z) + self.max[2],
        ];

        if contains_point(min, max, eye) {
            return Some(BlockOutlineHit {
                distance: 0.0,
                face: face_opposing_dominant_direction(direction),
                inside: true,
            });
        }

        let mut entry = f64::NEG_INFINITY;
        let mut exit = f64::INFINITY;
        let mut face = face_opposing_dominant_direction(direction);

        for axis in 0..3 {
            if direction[axis] == 0.0 {
                if eye[axis] < min[axis] || eye[axis] > max[axis] {
                    return None;
                }
                continue;
            }

            let t0 = (min[axis] - eye[axis]) / direction[axis];
            let t1 = (max[axis] - eye[axis]) / direction[axis];
            let (near, far, near_face) = if t0 <= t1 {
                (t0, t1, face_for_axis_delta(axis as u8, 1))
            } else {
                (t1, t0, face_for_axis_delta(axis as u8, -1))
            };

            if near > entry {
                entry = near;
                face = near_face;
            }
            exit = exit.min(far);
            if entry > exit {
                return None;
            }
        }

        if entry < 0.0 || entry > max_distance {
            return None;
        }

        Some(BlockOutlineHit {
            distance: entry,
            face,
            inside: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockOutlineHit {
    pub distance: f64,
    pub face: ProtocolDirection,
    pub inside: bool,
}

fn selection_box_for_outline_box(pos: BlockPos, outline: BlockOutlineBox) -> SelectionBox {
    SelectionBox {
        min: [
            pos.x as f32 + outline.min[0] as f32,
            pos.y as f32 + outline.min[1] as f32,
            pos.z as f32 + outline.min[2] as f32,
        ],
        max: [
            pos.x as f32 + outline.max[0] as f32,
            pos.y as f32 + outline.max[1] as f32,
            pos.z as f32 + outline.max[2] as f32,
        ],
    }
}

fn contains_point(min: [f64; 3], max: [f64; 3], point: [f64; 3]) -> bool {
    (0..3).all(|axis| point[axis] >= min[axis] && point[axis] <= max[axis])
}

fn face_for_axis_delta(axis: u8, delta: i32) -> ProtocolDirection {
    match (axis, delta.signum()) {
        (0, 1) => ProtocolDirection::West,
        (0, -1) => ProtocolDirection::East,
        (1, 1) => ProtocolDirection::Down,
        (1, -1) => ProtocolDirection::Up,
        (2, 1) => ProtocolDirection::North,
        (2, -1) => ProtocolDirection::South,
        _ => ProtocolDirection::North,
    }
}

fn face_opposing_dominant_direction(direction: [f64; 3]) -> ProtocolDirection {
    let ax = direction[0].abs();
    let ay = direction[1].abs();
    let az = direction[2].abs();
    if ax >= ay && ax >= az {
        if direction[0] >= 0.0 {
            ProtocolDirection::West
        } else {
            ProtocolDirection::East
        }
    } else if ay >= az {
        if direction[1] >= 0.0 {
            ProtocolDirection::Down
        } else {
            ProtocolDirection::Up
        }
    } else if direction[2] >= 0.0 {
        ProtocolDirection::North
    } else {
        ProtocolDirection::South
    }
}
