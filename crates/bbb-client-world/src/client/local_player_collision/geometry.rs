use super::*;

pub(in crate::client) type CollisionAxis = bbb_facts::collision::Axis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockDirection {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl BlockDirection {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    pub(super) fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

pub(super) fn block_direction(direction: ProtocolDirection) -> BlockDirection {
    match direction {
        ProtocolDirection::Down => BlockDirection::Down,
        ProtocolDirection::Up => BlockDirection::Up,
        ProtocolDirection::North => BlockDirection::North,
        ProtocolDirection::South => BlockDirection::South,
        ProtocolDirection::West => BlockDirection::West,
        ProtocolDirection::East => BlockDirection::East,
    }
}

pub(super) fn vanilla_support_direction(
    direction: ProtocolDirection,
) -> VanillaBlockSupportDirection {
    match direction {
        ProtocolDirection::Down => VanillaBlockSupportDirection::Down,
        ProtocolDirection::Up => VanillaBlockSupportDirection::Up,
        ProtocolDirection::North => VanillaBlockSupportDirection::North,
        ProtocolDirection::South => VanillaBlockSupportDirection::South,
        ProtocolDirection::West => VanillaBlockSupportDirection::West,
        ProtocolDirection::East => VanillaBlockSupportDirection::East,
    }
}

pub(super) fn protocol_direction_index(direction: ProtocolDirection) -> usize {
    match direction {
        ProtocolDirection::Down => 0,
        ProtocolDirection::Up => 1,
        ProtocolDirection::North => 2,
        ProtocolDirection::South => 3,
        ProtocolDirection::West => 4,
        ProtocolDirection::East => 5,
    }
}

pub(super) const VANILLA_PROTOCOL_DIRECTIONS: [ProtocolDirection; 6] = [
    ProtocolDirection::Down,
    ProtocolDirection::Up,
    ProtocolDirection::North,
    ProtocolDirection::South,
    ProtocolDirection::West,
    ProtocolDirection::East,
];

pub(super) const DEBUG_SOLID_FACE_DIRECTIONS: [BlockDirection; 6] = [
    BlockDirection::West,
    BlockDirection::South,
    BlockDirection::East,
    BlockDirection::North,
    BlockDirection::Down,
    BlockDirection::Up,
];

#[derive(Debug, Clone, Copy)]
pub(in crate::client) struct LocalPlayerCollisionContext {
    pub(super) entity_bottom: f64,
    pub(super) descending: bool,
    pub(super) fall_distance: f64,
    pub(super) can_walk_on_powder_snow: bool,
    pub(super) can_stand_on_lava: bool,
    pub(super) is_falling_block: bool,
}

impl LocalPlayerCollisionContext {
    pub(in crate::client) fn new(entity_bottom: f64, descending: bool) -> Self {
        Self {
            entity_bottom,
            descending,
            fall_distance: 0.0,
            can_walk_on_powder_snow: false,
            can_stand_on_lava: false,
            is_falling_block: false,
        }
    }

    pub(in crate::client) fn for_pose(
        world: &WorldStore,
        pose: LocalPlayerPoseState,
        descending: bool,
    ) -> Self {
        Self {
            entity_bottom: pose.position.y,
            descending,
            fall_distance: pose.fall_distance,
            can_walk_on_powder_snow: world.local_player_can_walk_on_powder_snow(),
            can_stand_on_lava: false,
            is_falling_block: false,
        }
    }

    pub(super) fn for_remote_entity(
        entity_bottom: f64,
        descending: bool,
        can_walk_on_powder_snow: bool,
        is_falling_block: bool,
    ) -> Self {
        Self {
            entity_bottom,
            descending,
            fall_distance: 0.0,
            can_walk_on_powder_snow,
            can_stand_on_lava: false,
            is_falling_block,
        }
    }

    pub(super) fn with_can_stand_on_lava(mut self, can_stand_on_lava: bool) -> Self {
        self.can_stand_on_lava = can_stand_on_lava;
        self
    }

    pub(super) fn for_bounds(bounds: WorldCollisionBox) -> Self {
        Self::new(bounds.min[1], false)
    }

    pub(super) fn is_above(self, pos: BlockPos, shape_max_y: f64) -> bool {
        self.entity_bottom
            > f64::from(pos.y) + shape_max_y - VANILLA_COLLISION_CONTEXT_ABOVE_EPSILON
    }
}

impl bbb_facts::collision::CollisionBounds for WorldCollisionBox {
    #[inline]
    fn min_x(self) -> f64 {
        self.min[0]
    }
    #[inline]
    fn min_y(self) -> f64 {
        self.min[1]
    }
    #[inline]
    fn min_z(self) -> f64 {
        self.min[2]
    }
    #[inline]
    fn max_x(self) -> f64 {
        self.max[0]
    }
    #[inline]
    fn max_y(self) -> f64 {
        self.max[1]
    }
    #[inline]
    fn max_z(self) -> f64 {
        self.max[2]
    }
}

impl From<bbb_facts::collision::CollisionBox> for WorldCollisionBox {
    fn from(bounds: bbb_facts::collision::CollisionBox) -> Self {
        let (min, max) = bounds.to_arrays();
        Self { min, max }
    }
}

impl From<bbb_protocol::block_state_types::VanillaBlockCollisionBox> for WorldCollisionBox {
    fn from(bounds: bbb_protocol::block_state_types::VanillaBlockCollisionBox) -> Self {
        Self {
            min: [bounds.min_x, bounds.min_y, bounds.min_z],
            max: [bounds.max_x, bounds.max_y, bounds.max_z],
        }
    }
}

impl WorldCollisionBox {
    pub(super) fn inflated(self, amount: f64) -> Self {
        Self {
            min: [
                self.min[0] - amount,
                self.min[1] - amount,
                self.min[2] - amount,
            ],
            max: [
                self.max[0] + amount,
                self.max[1] + amount,
                self.max[2] + amount,
            ],
        }
    }

    pub(in crate::client) fn moved(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min: [self.min[0] + x, self.min[1] + y, self.min[2] + z],
            max: [self.max[0] + x, self.max[1] + y, self.max[2] + z],
        }
    }

    pub(in crate::client) fn deflated(self, amount: f64) -> Self {
        Self {
            min: [
                self.min[0] + amount,
                self.min[1] + amount,
                self.min[2] + amount,
            ],
            max: [
                self.max[0] - amount,
                self.max[1] - amount,
                self.max[2] - amount,
            ],
        }
    }

    pub(in crate::client) fn edge_support_probe(self, min_height: f64) -> Self {
        Self {
            min: [
                self.min[0] + COLLISION_EPSILON,
                self.min[1] - min_height - COLLISION_EPSILON,
                self.min[2] + COLLISION_EPSILON,
            ],
            max: [
                self.max[0] - COLLISION_EPSILON,
                self.min[1],
                self.max[2] - COLLISION_EPSILON,
            ],
        }
    }

    pub(super) fn moved_axis(self, axis: CollisionAxis, amount: f64) -> Self {
        match axis {
            CollisionAxis::X => self.moved(amount, 0.0, 0.0),
            CollisionAxis::Y => self.moved(0.0, amount, 0.0),
            CollisionAxis::Z => self.moved(0.0, 0.0, amount),
        }
    }

    pub(in crate::client) fn swept_axis(self, axis: CollisionAxis, amount: f64) -> Self {
        let moved = self.moved_axis(axis, amount);
        match axis {
            CollisionAxis::X => Self {
                min: [self.min[0].min(moved.min[0]), self.min[1], self.min[2]],
                max: [self.max[0].max(moved.max[0]), self.max[1], self.max[2]],
            },
            CollisionAxis::Y => Self {
                min: [self.min[0], self.min[1].min(moved.min[1]), self.min[2]],
                max: [self.max[0], self.max[1].max(moved.max[1]), self.max[2]],
            },
            CollisionAxis::Z => Self {
                min: [self.min[0], self.min[1], self.min[2].min(moved.min[2])],
                max: [self.max[0], self.max[1], self.max[2].max(moved.max[2])],
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BlockCollisionShape {
    pub(super) boxes: [Option<WorldCollisionBox>; MAX_COLLISION_BOXES],
}

impl BlockCollisionShape {
    pub(super) fn single(shape_box: WorldCollisionBox) -> Self {
        let mut boxes = [None; MAX_COLLISION_BOXES];
        boxes[0] = Some(shape_box);
        Self { boxes }
    }

    pub(super) fn from_boxes<const N: usize>(boxes: [Option<WorldCollisionBox>; N]) -> Self {
        let mut out = [None; MAX_COLLISION_BOXES];
        for (slot, shape_box) in out.iter_mut().zip(boxes.into_iter().flatten()) {
            *slot = Some(shape_box);
        }
        Self { boxes: out }
    }

    pub(super) fn boxes(self) -> impl Iterator<Item = WorldCollisionBox> {
        self.boxes.into_iter().flatten()
    }

    /// Mirrors `VoxelShape.toAabbs()`: build the coordinate-compressed union,
    /// then apply `BitSetDiscreteVoxelShape.forAllBoxes(..., true)`'s z-strip,
    /// x-neighbor, and y-neighbor merge order without allocating for 0/1-box
    /// shapes.
    pub(super) fn visit_aabbs(self, mut visit: impl FnMut(WorldCollisionBox)) {
        let mut boxes = self.boxes();
        let Some(first) = boxes.next() else {
            return;
        };
        let Some(second) = boxes.next() else {
            visit(first);
            return;
        };
        let mut collected = [WorldCollisionBox::FULL; MAX_COLLISION_BOXES];
        collected[0] = first;
        collected[1] = second;
        let mut len = 2;
        for shape_box in boxes {
            collected[len] = shape_box;
            len += 1;
        }
        for shape_box in bbb_facts::collision::VoxelBoxShape::from_boxes(&collected[..len])
            .expect("valid block collision boxes")
            .boxes()
        {
            visit(WorldCollisionBox::from(shape_box));
        }
    }

    pub(super) fn to_aabbs(self) -> Vec<WorldCollisionBox> {
        let mut out = Vec::new();
        self.visit_aabbs(|shape_box| out.push(shape_box));
        out
    }

    pub(super) fn is_full_block(self) -> bool {
        let mut boxes = self.boxes();
        matches!(boxes.next(), Some(WorldCollisionBox::FULL)) && boxes.next().is_none()
    }

    pub(super) fn full_face_flags(self) -> [bool; 6] {
        DEBUG_SOLID_FACE_DIRECTIONS.map(|direction| self.has_full_face(direction))
    }

    /// Mirrors `Block.isFaceFull(shape, direction)`: sample the boundary
    /// slice selected by `VoxelShape.getFaceShape`, then require its union to
    /// cover the complete unit square.
    pub(super) fn has_full_face(self, direction: BlockDirection) -> bool {
        self.covers_face_rectangle(direction, [0.0, 1.0, 0.0, 1.0])
    }

    pub(super) fn has_center_face(self, direction: BlockDirection) -> bool {
        self.covers_face_rectangle(direction, [7.0 * PX, 9.0 * PX, 7.0 * PX, 9.0 * PX])
    }

    pub(super) fn has_rigid_face(self, direction: BlockDirection) -> bool {
        [
            [0.0, 2.0 * PX, 0.0, 1.0],
            [14.0 * PX, 1.0, 0.0, 1.0],
            [2.0 * PX, 14.0 * PX, 0.0, 2.0 * PX],
            [2.0 * PX, 14.0 * PX, 14.0 * PX, 1.0],
        ]
        .into_iter()
        .all(|target| self.covers_face_rectangle(direction, target))
    }

    pub(super) fn has_nonempty_face(self, direction: BlockDirection) -> bool {
        self.boxes()
            .any(|shape_box| block_collision_box_reaches_face(shape_box, direction))
    }

    pub(super) fn covers_face_rectangle(self, direction: BlockDirection, target: [f64; 4]) -> bool {
        let [target_u0, target_u1, target_v0, target_v1] = target;
        if target_u0 >= target_u1 || target_v0 >= target_v1 {
            return false;
        }
        let face_boxes = self
            .boxes()
            .filter(|shape_box| block_collision_box_reaches_face(*shape_box, direction))
            .collect::<Vec<_>>();
        if face_boxes.is_empty() {
            return false;
        }

        let projection = |shape_box: WorldCollisionBox| match direction {
            BlockDirection::West | BlockDirection::East => [
                shape_box.min[1],
                shape_box.max[1],
                shape_box.min[2],
                shape_box.max[2],
            ],
            BlockDirection::South | BlockDirection::North => [
                shape_box.min[0],
                shape_box.max[0],
                shape_box.min[1],
                shape_box.max[1],
            ],
            BlockDirection::Down | BlockDirection::Up => [
                shape_box.min[0],
                shape_box.max[0],
                shape_box.min[2],
                shape_box.max[2],
            ],
        };
        let mut u_coords = vec![target_u0, target_u1];
        let mut v_coords = vec![target_v0, target_v1];
        for shape_box in &face_boxes {
            let [u0, u1, v0, v1] = projection(*shape_box);
            u_coords.extend([
                u0.clamp(target_u0, target_u1),
                u1.clamp(target_u0, target_u1),
            ]);
            v_coords.extend([
                v0.clamp(target_v0, target_v1),
                v1.clamp(target_v0, target_v1),
            ]);
        }
        u_coords.sort_by(f64::total_cmp);
        u_coords.dedup_by(|left, right| *left == *right);
        v_coords.sort_by(f64::total_cmp);
        v_coords.dedup_by(|left, right| *left == *right);

        u_coords.windows(2).all(|u| {
            v_coords.windows(2).all(|v| {
                let midpoint_u = (u[0] + u[1]) * 0.5;
                let midpoint_v = (v[0] + v[1]) * 0.5;
                face_boxes.iter().any(|shape_box| {
                    let [u0, u1, v0, v1] = projection(*shape_box);
                    midpoint_u >= u0 && midpoint_u < u1 && midpoint_v >= v0 && midpoint_v < v1
                })
            })
        })
    }

    pub(super) fn max_y_at(self, local_x: f64, local_z: f64) -> f64 {
        self.boxes()
            .filter(|shape_box| shape_box.contains_xz(local_x, local_z))
            .map(|shape_box| shape_box.max[1])
            .fold(0.0, f64::max)
    }

    pub(super) fn environment_animate_bounds(self) -> Option<EnvironmentAnimateCollisionBounds> {
        let mut boxes = self.boxes();
        let first = boxes.next()?;
        Some(boxes.fold(
            EnvironmentAnimateCollisionBounds {
                min_x: first.min[0],
                min_y: first.min[1],
                min_z: first.min[2],
                max_x: first.max[0],
                max_y: first.max[1],
                max_z: first.max[2],
            },
            |bounds, shape_box| EnvironmentAnimateCollisionBounds {
                min_x: bounds.min_x.min(shape_box.min[0]),
                min_y: bounds.min_y.min(shape_box.min[1]),
                min_z: bounds.min_z.min(shape_box.min[2]),
                max_x: bounds.max_x.max(shape_box.max[0]),
                max_y: bounds.max_y.max(shape_box.max[1]),
                max_z: bounds.max_z.max(shape_box.max[2]),
            },
        ))
    }

    pub(super) fn offset(self, x: f64, y: f64, z: f64) -> Self {
        self.map_boxes(|shape_box| shape_box.moved(x, y, z))
    }

    pub(super) fn map_boxes(
        self,
        mut f: impl FnMut(WorldCollisionBox) -> WorldCollisionBox,
    ) -> Self {
        Self {
            boxes: self.boxes.map(|shape_box| shape_box.map(&mut f)),
        }
    }
}

pub(super) fn block_collision_box_reaches_face(
    shape_box: WorldCollisionBox,
    direction: BlockDirection,
) -> bool {
    const NEGATIVE_FACE_SAMPLE: f64 = 1.0e-7;
    const POSITIVE_FACE_SAMPLE: f64 = 0.999_999_9;

    let (min, max, sample) = match direction {
        BlockDirection::West => (shape_box.min[0], shape_box.max[0], NEGATIVE_FACE_SAMPLE),
        BlockDirection::South => (shape_box.min[2], shape_box.max[2], POSITIVE_FACE_SAMPLE),
        BlockDirection::East => (shape_box.min[0], shape_box.max[0], POSITIVE_FACE_SAMPLE),
        BlockDirection::North => (shape_box.min[2], shape_box.max[2], NEGATIVE_FACE_SAMPLE),
        BlockDirection::Down => (shape_box.min[1], shape_box.max[1], NEGATIVE_FACE_SAMPLE),
        BlockDirection::Up => (shape_box.min[1], shape_box.max[1], POSITIVE_FACE_SAMPLE),
    };
    min <= sample && sample < max
}

pub(super) struct BlockCollisionShapeBuilder {
    pub(super) boxes: [Option<WorldCollisionBox>; MAX_COLLISION_BOXES],
    pub(super) next_index: usize,
}

impl BlockCollisionShapeBuilder {
    pub(super) fn new() -> Self {
        Self {
            boxes: [None; MAX_COLLISION_BOXES],
            next_index: 0,
        }
    }

    pub(super) fn push(&mut self, shape_box: WorldCollisionBox) {
        if self.next_index < MAX_COLLISION_BOXES {
            self.boxes[self.next_index] = Some(shape_box);
            self.next_index += 1;
        }
    }

    pub(super) fn extend(&mut self, shape: BlockCollisionShape) {
        for shape_box in shape.boxes() {
            self.push(shape_box);
        }
    }

    pub(super) fn build(self) -> BlockCollisionShape {
        BlockCollisionShape { boxes: self.boxes }
    }
}

impl WorldCollisionBox {
    pub(super) const FULL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const SCAFFOLDING_TOP: Self = Self {
        min: [0.0, 14.0 * PX, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const SCAFFOLDING_UNSTABLE_BOTTOM: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 2.0 * PX, 1.0],
    };
    pub(super) const POWDER_SNOW_FALLING: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_HEIGHT, 1.0],
    };
    pub(super) const SCAFFOLDING_NORTH_WEST_POST: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [2.0 * PX, 1.0, 2.0 * PX],
    };
    pub(super) const SCAFFOLDING_NORTH_EAST_POST: Self = Self {
        min: [14.0 * PX, 0.0, 0.0],
        max: [1.0, 1.0, 2.0 * PX],
    };
    pub(super) const SCAFFOLDING_SOUTH_EAST_POST: Self = Self {
        min: [14.0 * PX, 0.0, 14.0 * PX],
        max: [1.0, 1.0, 1.0],
    };
    pub(super) const SCAFFOLDING_SOUTH_WEST_POST: Self = Self {
        min: [0.0, 0.0, 14.0 * PX],
        max: [2.0 * PX, 1.0, 1.0],
    };
    pub(super) const BOTTOM_SLAB: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 0.5, 1.0],
    };
    pub(super) const FENCE_POST: Self = Self {
        min: [6.0 * PX, 0.0, 6.0 * PX],
        max: [10.0 * PX, 1.5, 10.0 * PX],
    };
    pub(super) const FENCE_NORTH_ARM: Self = Self {
        min: [6.0 * PX, 0.0, 0.0],
        max: [10.0 * PX, 1.5, 8.0 * PX],
    };
    pub(super) const FENCE_EAST_ARM: Self = Self {
        min: [8.0 * PX, 0.0, 6.0 * PX],
        max: [1.0, 1.5, 10.0 * PX],
    };
    pub(super) const FENCE_SOUTH_ARM: Self = Self {
        min: [6.0 * PX, 0.0, 8.0 * PX],
        max: [10.0 * PX, 1.5, 1.0],
    };
    pub(super) const FENCE_WEST_ARM: Self = Self {
        min: [0.0, 0.0, 6.0 * PX],
        max: [8.0 * PX, 1.5, 10.0 * PX],
    };
    pub(super) fn cuboid(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> Self {
        Self {
            min: [min_x, min_y, min_z],
            max: [max_x, max_y, max_z],
        }
    }

    pub(super) fn from_pixels(min: [f64; 3], max: [f64; 3]) -> Self {
        Self::cuboid(
            min[0] * PX,
            min[1] * PX,
            min[2] * PX,
            max[0] * PX,
            max[1] * PX,
            max[2] * PX,
        )
    }

    pub(super) fn column(min_x: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min: [min_x, 0.0, min_z],
            max: [max_x, max_y, max_z],
        }
    }

    pub(super) fn centered_column(
        width_x_px: f64,
        width_z_px: f64,
        min_y_px: f64,
        max_y_px: f64,
    ) -> Self {
        Self::from_pixels(
            [
                (16.0 - width_x_px) * 0.5,
                min_y_px,
                (16.0 - width_z_px) * 0.5,
            ],
            [
                (16.0 + width_x_px) * 0.5,
                max_y_px,
                (16.0 + width_z_px) * 0.5,
            ],
        )
    }

    pub(super) fn contains_xz(self, local_x: f64, local_z: f64) -> bool {
        local_x >= self.min[0]
            && local_x < self.max[0]
            && local_z >= self.min[2]
            && local_z < self.max[2]
    }
}
