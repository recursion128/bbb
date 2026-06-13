use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_renderer::{SelectionBox, SelectionOutline};
use bbb_world::BlockPos;

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
    pub(super) const BOTTOM_SLAB: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 0.5, 1.0],
    };
    pub(super) const TOP_SLAB: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const CARPET: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0 / 16.0, 1.0],
    };
    pub(super) const FENCE_POST: Self = Self {
        min: [6.0 / 16.0, 0.0, 6.0 / 16.0],
        max: [10.0 / 16.0, 1.0, 10.0 / 16.0],
    };
    pub(super) const FENCE_NORTH_ARM: Self = Self {
        min: [6.0 / 16.0, 0.0, 0.0],
        max: [10.0 / 16.0, 1.0, 8.0 / 16.0],
    };
    pub(super) const FENCE_EAST_ARM: Self = Self {
        min: [8.0 / 16.0, 0.0, 6.0 / 16.0],
        max: [1.0, 1.0, 10.0 / 16.0],
    };
    pub(super) const FENCE_SOUTH_ARM: Self = Self {
        min: [6.0 / 16.0, 0.0, 8.0 / 16.0],
        max: [10.0 / 16.0, 1.0, 1.0],
    };
    pub(super) const FENCE_WEST_ARM: Self = Self {
        min: [0.0, 0.0, 6.0 / 16.0],
        max: [8.0 / 16.0, 1.0, 10.0 / 16.0],
    };
    pub(super) const FENCE_GATE_Z: Self = Self {
        min: [0.0, 0.0, 6.0 / 16.0],
        max: [1.0, 1.0, 10.0 / 16.0],
    };
    pub(super) const FENCE_GATE_X: Self = Self {
        min: [6.0 / 16.0, 0.0, 0.0],
        max: [10.0 / 16.0, 1.0, 1.0],
    };
    pub(super) const FENCE_GATE_Z_IN_WALL: Self = Self {
        min: [0.0, 0.0, 6.0 / 16.0],
        max: [1.0, 13.0 / 16.0, 10.0 / 16.0],
    };
    pub(super) const FENCE_GATE_X_IN_WALL: Self = Self {
        min: [6.0 / 16.0, 0.0, 0.0],
        max: [10.0 / 16.0, 13.0 / 16.0, 1.0],
    };
    pub(super) const PANE_POST: Self = Self {
        min: [7.0 / 16.0, 0.0, 7.0 / 16.0],
        max: [9.0 / 16.0, 1.0, 9.0 / 16.0],
    };
    pub(super) const PANE_NORTH_ARM: Self = Self {
        min: [7.0 / 16.0, 0.0, 0.0],
        max: [9.0 / 16.0, 1.0, 8.0 / 16.0],
    };
    pub(super) const PANE_EAST_ARM: Self = Self {
        min: [8.0 / 16.0, 0.0, 7.0 / 16.0],
        max: [1.0, 1.0, 9.0 / 16.0],
    };
    pub(super) const PANE_SOUTH_ARM: Self = Self {
        min: [7.0 / 16.0, 0.0, 8.0 / 16.0],
        max: [9.0 / 16.0, 1.0, 1.0],
    };
    pub(super) const PANE_WEST_ARM: Self = Self {
        min: [0.0, 0.0, 7.0 / 16.0],
        max: [8.0 / 16.0, 1.0, 9.0 / 16.0],
    };
    pub(super) const WALL_POST: Self = Self {
        min: [4.0 / 16.0, 0.0, 4.0 / 16.0],
        max: [12.0 / 16.0, 1.0, 12.0 / 16.0],
    };
    pub(super) const WALL_NORTH_LOW: Self = Self {
        min: [5.0 / 16.0, 0.0, 0.0],
        max: [11.0 / 16.0, 14.0 / 16.0, 11.0 / 16.0],
    };
    pub(super) const WALL_EAST_LOW: Self = Self {
        min: [5.0 / 16.0, 0.0, 5.0 / 16.0],
        max: [1.0, 14.0 / 16.0, 11.0 / 16.0],
    };
    pub(super) const WALL_SOUTH_LOW: Self = Self {
        min: [5.0 / 16.0, 0.0, 5.0 / 16.0],
        max: [11.0 / 16.0, 14.0 / 16.0, 1.0],
    };
    pub(super) const WALL_WEST_LOW: Self = Self {
        min: [0.0, 0.0, 5.0 / 16.0],
        max: [11.0 / 16.0, 14.0 / 16.0, 11.0 / 16.0],
    };
    pub(super) const WALL_NORTH_TALL: Self = Self {
        min: [5.0 / 16.0, 0.0, 0.0],
        max: [11.0 / 16.0, 1.0, 11.0 / 16.0],
    };
    pub(super) const WALL_EAST_TALL: Self = Self {
        min: [5.0 / 16.0, 0.0, 5.0 / 16.0],
        max: [1.0, 1.0, 11.0 / 16.0],
    };
    pub(super) const WALL_SOUTH_TALL: Self = Self {
        min: [5.0 / 16.0, 0.0, 5.0 / 16.0],
        max: [11.0 / 16.0, 1.0, 1.0],
    };
    pub(super) const WALL_WEST_TALL: Self = Self {
        min: [0.0, 0.0, 5.0 / 16.0],
        max: [11.0 / 16.0, 1.0, 11.0 / 16.0],
    };
    pub(super) const TRAPDOOR_BOTTOM: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 3.0 / 16.0, 1.0],
    };
    pub(super) const TRAPDOOR_TOP: Self = Self {
        min: [0.0, 13.0 / 16.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const TRAPDOOR_NORTH_OPEN: Self = Self {
        min: [0.0, 0.0, 13.0 / 16.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const TRAPDOOR_EAST_OPEN: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [3.0 / 16.0, 1.0, 1.0],
    };
    pub(super) const TRAPDOOR_SOUTH_OPEN: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 3.0 / 16.0],
    };
    pub(super) const TRAPDOOR_WEST_OPEN: Self = Self {
        min: [13.0 / 16.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const DOOR_NORTH: Self = Self {
        min: [0.0, 0.0, 13.0 / 16.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const DOOR_EAST: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [3.0 / 16.0, 1.0, 1.0],
    };
    pub(super) const DOOR_SOUTH: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 3.0 / 16.0],
    };
    pub(super) const DOOR_WEST: Self = Self {
        min: [13.0 / 16.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const LADDER_NORTH: Self = Self::DOOR_NORTH;
    pub(super) const LADDER_EAST: Self = Self::DOOR_EAST;
    pub(super) const LADDER_SOUTH: Self = Self::DOOR_SOUTH;
    pub(super) const LADDER_WEST: Self = Self::DOOR_WEST;
    pub(super) const STAIR_NORTH_WEST_OCTET: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [0.5, 1.0, 0.5],
    };
    pub(super) const STAIR_NORTH_HALF: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [1.0, 1.0, 0.5],
    };
    pub(super) const STAIR_SOUTH_EAST_OCTET: Self = Self {
        min: [0.5, 0.5, 0.5],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const PALE_MOSS_NORTH_LOW: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 10.0 / 16.0, 1.0 / 16.0],
    };
    pub(super) const PALE_MOSS_EAST_LOW: Self = Self {
        min: [15.0 / 16.0, 0.0, 0.0],
        max: [1.0, 10.0 / 16.0, 1.0],
    };
    pub(super) const PALE_MOSS_SOUTH_LOW: Self = Self {
        min: [0.0, 0.0, 15.0 / 16.0],
        max: [1.0, 10.0 / 16.0, 1.0],
    };
    pub(super) const PALE_MOSS_WEST_LOW: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0 / 16.0, 10.0 / 16.0, 1.0],
    };
    pub(super) const PALE_MOSS_NORTH_TALL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0 / 16.0],
    };
    pub(super) const PALE_MOSS_EAST_TALL: Self = Self {
        min: [15.0 / 16.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const PALE_MOSS_SOUTH_TALL: Self = Self {
        min: [0.0, 0.0, 15.0 / 16.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const PALE_MOSS_WEST_TALL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0 / 16.0, 1.0, 1.0],
    };

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

    pub(super) fn invert_y(self) -> Self {
        Self {
            min: [self.min[0], 1.0 - self.max[1], self.min[2]],
            max: [self.max[0], 1.0 - self.min[1], self.max[2]],
        }
    }

    fn rotate_y_90(self) -> Self {
        Self {
            min: [1.0 - self.max[2], self.min[1], self.min[0]],
            max: [1.0 - self.min[2], self.max[1], self.max[0]],
        }
    }

    pub(super) fn rotate_to_direction(self, direction: HorizontalDirection) -> Self {
        let mut rotated = self;
        for _ in 0..direction.quarter_turns_from_north() {
            rotated = rotated.rotate_y_90();
        }
        rotated
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlockOutlineHit {
    pub(crate) distance: f64,
    pub(crate) face: ProtocolDirection,
    pub(crate) inside: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HorizontalDirection {
    North,
    East,
    South,
    West,
}

impl HorizontalDirection {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            _ => None,
        }
    }

    pub(super) fn clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    pub(super) fn counter_clockwise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::East => Self::North,
            Self::South => Self::East,
            Self::West => Self::South,
        }
    }

    fn quarter_turns_from_north(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }
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
