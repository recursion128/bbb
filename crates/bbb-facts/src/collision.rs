//! Shared swept-AABB collision primitives and cursor traversal for 26.1 voxel shapes.

mod box_shape;

pub use box_shape::{collision_face_full, VoxelBoxShape};

use crate::block_state_types::VanillaBlockCollisionBox;

/// `VoxelShape.collideX` (`VoxelShape.java:256`, `:266-271`, `:279`, `:294`)
/// and `Shapes.collide` (`Shapes.java:233`) guard band.
pub const COLLISION_EPSILON: f64 = 1.0e-7;

/// `BlockCollisions` (`BlockCollisions.java:54-59`) block cursor guard band.
pub const CURSOR_EPSILON: f64 = 1.0e-7;

/// `Vec3.normalize()` (`Vec3.java:85`) zero-length threshold (`1.0E-5F` widened to `f64`).
pub const NORMALIZE_EPSILON: f64 = 1.0e-5_f32 as f64;

/// `Direction.Axis` (`Direction.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

/// `Direction` (`Direction.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl BlockFace {
    pub const ALL: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

    pub const HORIZONTAL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    #[inline]
    pub const fn step(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }

    #[inline]
    pub const fn axis(self) -> Axis {
        match self {
            Self::West | Self::East => Axis::X,
            Self::Down | Self::Up => Axis::Y,
            Self::North | Self::South => Axis::Z,
        }
    }

    #[inline]
    pub const fn is_positive(self) -> bool {
        matches!(self, Self::Up | Self::South | Self::East)
    }
}

/// `Direction.axisStepOrder(Vec3)` (`Direction.java:379-381`).
#[inline]
pub fn axis_step_order(dx: f64, dz: f64) -> [Axis; 3] {
    if dx.abs() < dz.abs() {
        [Axis::Y, Axis::Z, Axis::X]
    } else {
        [Axis::Y, Axis::X, Axis::Z]
    }
}

/// Protocol-independent axis-aligned bounding box (`AABB.java`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionBox {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl CollisionBox {
    pub const UNIT_CUBE: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };

    #[inline]
    pub const fn from_arrays(min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            min_x: min[0],
            min_y: min[1],
            min_z: min[2],
            max_x: max[0],
            max_y: max[1],
            max_z: max[2],
        }
    }

    #[inline]
    pub const fn to_arrays(self) -> ([f64; 3], [f64; 3]) {
        (
            [self.min_x, self.min_y, self.min_z],
            [self.max_x, self.max_y, self.max_z],
        )
    }

    #[inline]
    pub const fn of_block_xyz(x: i32, y: i32, z: i32) -> Self {
        Self {
            min_x: x as f64,
            min_y: y as f64,
            min_z: z as f64,
            max_x: x as f64 + 1.0,
            max_y: y as f64 + 1.0,
            max_z: z as f64 + 1.0,
        }
    }

    /// `AABB.minmax` (`AABB.java:209-217`), preserving NaN and signed zero ordering.
    pub fn minmax(self, other: Self) -> Self {
        fn minimum(left: f64, right: f64) -> f64 {
            if left.is_nan() {
                left
            } else if right.is_nan() {
                right
            } else if left.total_cmp(&right).is_le() {
                left
            } else {
                right
            }
        }
        fn maximum(left: f64, right: f64) -> f64 {
            if left.is_nan() {
                left
            } else if right.is_nan() {
                right
            } else if left.total_cmp(&right).is_ge() {
                left
            } else {
                right
            }
        }
        Self {
            min_x: minimum(self.min_x, other.min_x),
            min_y: minimum(self.min_y, other.min_y),
            min_z: minimum(self.min_z, other.min_z),
            max_x: maximum(self.max_x, other.max_x),
            max_y: maximum(self.max_y, other.max_y),
            max_z: maximum(self.max_z, other.max_z),
        }
    }

    #[inline]
    pub fn expand_towards(self, delta: [f64; 3]) -> Self {
        Self {
            min_x: self.min_x + delta[0].min(0.0),
            min_y: self.min_y + delta[1].min(0.0),
            min_z: self.min_z + delta[2].min(0.0),
            max_x: self.max_x + delta[0].max(0.0),
            max_y: self.max_y + delta[1].max(0.0),
            max_z: self.max_z + delta[2].max(0.0),
        }
    }

    #[inline]
    pub fn inflate(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: self.min_x - x,
            min_y: self.min_y - y,
            min_z: self.min_z - z,
            max_x: self.max_x + x,
            max_y: self.max_y + y,
            max_z: self.max_z + z,
        }
    }

    #[inline]
    pub fn deflated(self, amount: f64) -> Self {
        self.inflate(-amount, -amount, -amount)
    }

    #[inline]
    pub fn contains_half_open(self, point: [f64; 3]) -> bool {
        point[0] >= self.min_x
            && point[0] < self.max_x
            && point[1] >= self.min_y
            && point[1] < self.max_y
            && point[2] >= self.min_z
            && point[2] < self.max_z
    }

    #[inline]
    pub fn moved(self, delta: [f64; 3]) -> Self {
        Self {
            min_x: self.min_x + delta[0],
            min_y: self.min_y + delta[1],
            min_z: self.min_z + delta[2],
            max_x: self.max_x + delta[0],
            max_y: self.max_y + delta[1],
            max_z: self.max_z + delta[2],
        }
    }

    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
            && self.min_z < other.max_z
            && self.max_z > other.min_z
    }

    #[inline]
    pub fn is_unit_cube(self) -> bool {
        self == Self::UNIT_CUBE
    }
}

/// Trait implemented by client and server AABB representations so both share one solver.
pub trait CollisionBounds: Copy {
    fn min_x(self) -> f64;
    fn min_y(self) -> f64;
    fn min_z(self) -> f64;
    fn max_x(self) -> f64;
    fn max_y(self) -> f64;
    fn max_z(self) -> f64;

    #[inline]
    fn min_on(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.min_x(),
            Axis::Y => self.min_y(),
            Axis::Z => self.min_z(),
        }
    }

    #[inline]
    fn max_on(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.max_x(),
            Axis::Y => self.max_y(),
            Axis::Z => self.max_z(),
        }
    }

    #[inline]
    fn to_collision_box(self) -> CollisionBox {
        CollisionBox {
            min_x: self.min_x(),
            min_y: self.min_y(),
            min_z: self.min_z(),
            max_x: self.max_x(),
            max_y: self.max_y(),
            max_z: self.max_z(),
        }
    }
}

impl CollisionBounds for CollisionBox {
    #[inline]
    fn min_x(self) -> f64 {
        self.min_x
    }
    #[inline]
    fn min_y(self) -> f64 {
        self.min_y
    }
    #[inline]
    fn min_z(self) -> f64 {
        self.min_z
    }
    #[inline]
    fn max_x(self) -> f64 {
        self.max_x
    }
    #[inline]
    fn max_y(self) -> f64 {
        self.max_y
    }
    #[inline]
    fn max_z(self) -> f64 {
        self.max_z
    }
}

impl CollisionBounds for VanillaBlockCollisionBox {
    #[inline]
    fn min_x(self) -> f64 {
        self.min_x
    }
    #[inline]
    fn min_y(self) -> f64 {
        self.min_y
    }
    #[inline]
    fn min_z(self) -> f64 {
        self.min_z
    }
    #[inline]
    fn max_x(self) -> f64 {
        self.max_x
    }
    #[inline]
    fn max_y(self) -> f64 {
        self.max_y
    }
    #[inline]
    fn max_z(self) -> f64 {
        self.max_z
    }
}

impl CollisionBounds for ([f64; 3], [f64; 3]) {
    #[inline]
    fn min_x(self) -> f64 {
        self.0[0]
    }
    #[inline]
    fn min_y(self) -> f64 {
        self.0[1]
    }
    #[inline]
    fn min_z(self) -> f64 {
        self.0[2]
    }
    #[inline]
    fn max_x(self) -> f64 {
        self.1[0]
    }
    #[inline]
    fn max_y(self) -> f64 {
        self.1[1]
    }
    #[inline]
    fn max_z(self) -> f64 {
        self.1[2]
    }
}

/// `BlockBehaviour.BlockStateBase.Cache.largeCollisionShape` (`BlockBehaviour.java:930-931`).
pub fn is_large_collision_shape<B: CollisionBounds>(boxes: &[B]) -> bool {
    boxes.iter().any(|shape| {
        shape.min_x() < 0.0
            || shape.min_y() < 0.0
            || shape.min_z() < 0.0
            || shape.max_x() > 1.0
            || shape.max_y() > 1.0
            || shape.max_z() > 1.0
    })
}

/// `VoxelShape.collideX` (`VoxelShape.java:268-271`) asymmetric face-overlap test.
#[inline]
pub fn overlaps_on<M: CollisionBounds, B: CollisionBounds>(
    moving: M,
    shape: B,
    axis: Axis,
) -> bool {
    moving.max_on(axis) - COLLISION_EPSILON >= shape.min_on(axis)
        && moving.min_on(axis) + COLLISION_EPSILON < shape.max_on(axis)
}

/// `Shapes.collide` (`Shapes.java:231-241`) over `VoxelShape.collideX` (`VoxelShape.java:251-306`).
pub fn collide_axis<M: CollisionBounds, B: CollisionBounds>(
    axis: Axis,
    moving: M,
    boxes: &[B],
    mut distance: f64,
) -> f64 {
    let (other_a, other_b) = match axis {
        Axis::X => (Axis::Y, Axis::Z),
        Axis::Y => (Axis::Z, Axis::X),
        Axis::Z => (Axis::X, Axis::Y),
    };
    for shape in boxes {
        if distance.abs() < COLLISION_EPSILON {
            return 0.0;
        }
        if !overlaps_on(moving, *shape, other_a) || !overlaps_on(moving, *shape, other_b) {
            continue;
        }
        if distance > 0.0 {
            if moving.max_on(axis) - COLLISION_EPSILON < shape.min_on(axis) {
                let candidate = shape.min_on(axis) - moving.max_on(axis);
                if candidate >= -COLLISION_EPSILON {
                    distance = distance.min(candidate);
                }
            }
        } else if moving.min_on(axis) + COLLISION_EPSILON >= shape.max_on(axis) {
            let candidate = shape.max_on(axis) - moving.min_on(axis);
            if candidate <= COLLISION_EPSILON {
                distance = distance.max(candidate);
            }
        }
    }
    distance
}

/// `Entity.collideWithShapes` (`Entity.java:1159-1175`).
pub fn collide_with_boxes<M: CollisionBounds, B: CollisionBounds>(
    movement: [f64; 3],
    bounding_box: M,
    boxes: &[B],
) -> [f64; 3] {
    if boxes.is_empty() {
        return movement;
    }
    let bounds = bounding_box.to_collision_box();
    let mut resolved = [0.0; 3];
    for axis in axis_step_order(movement[0], movement[2]) {
        let idx = axis.index();
        let axis_movement = movement[idx];
        if axis_movement != 0.0 {
            resolved[idx] = collide_axis(axis, bounds.moved(resolved), boxes, axis_movement);
        }
    }
    resolved
}

/// `Entity.collide` step-up resolution (`Entity.java:1083-1127`).
pub fn collide_with_step<M: CollisionBounds, B: CollisionBounds>(
    bounds: M,
    delta: [f64; 3],
    clipped: [f64; 3],
    on_ground: bool,
    max_step: f32,
    mut collect: impl FnMut(CollisionBox) -> Option<Vec<B>>,
) -> Option<[f64; 3]> {
    let bounds = bounds.to_collision_box();
    let grounded_after = delta[1] != clipped[1] && delta[1] < 0.0;
    if max_step > 0.0
        && (grounded_after || on_ground)
        && (delta[0] != clipped[0] || delta[2] != clipped[2])
    {
        let grounded = if grounded_after {
            bounds.moved([0.0, clipped[1], 0.0])
        } else {
            bounds
        };
        let mut step_bounds = grounded.expand_towards([delta[0], f64::from(max_step), delta[2]]);
        if !grounded_after {
            step_bounds = step_bounds.expand_towards([0.0, -f64::from(1.0e-5_f32), 0.0]);
        }
        let boxes = collect(step_bounds)?;
        let mut candidates = Vec::new();
        for shape in &boxes {
            for coordinate in [shape.min_y(), shape.max_y()] {
                let height = (coordinate - grounded.min_y) as f32;
                if height >= 0.0
                    && height != clipped[1] as f32
                    && height <= max_step
                    && !candidates.contains(&height)
                {
                    candidates.push(height);
                }
            }
        }
        candidates.sort_unstable_by(f32::total_cmp);
        let clipped_h_sqr = clipped[0] * clipped[0] + clipped[2] * clipped[2];
        for height in candidates {
            let step =
                collide_with_boxes([delta[0], f64::from(height), delta[2]], grounded, &boxes);
            if step[0] * step[0] + step[2] * step[2] > clipped_h_sqr {
                return Some([step[0], step[1] - (bounds.min_y - grounded.min_y), step[2]]);
            }
        }
    }
    Some(clipped)
}

/// Hard upper bound on integer cells walked by a single 3D collision cursor scan.
const MAX_COLLISION_CURSOR_CELLS: u64 = 262_144;

#[inline]
fn collision_cursor_cell_count(
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
) -> u64 {
    if min_x > max_x || min_y > max_y || min_z > max_z {
        return 0;
    }
    let dx = (i64::from(max_x) - i64::from(min_x) + 1) as u64;
    let dy = (i64::from(max_y) - i64::from(min_y) + 1) as u64;
    let dz = (i64::from(max_z) - i64::from(min_z) + 1) as u64;
    dx.saturating_mul(dy).saturating_mul(dz)
}

/// `BlockCollisions.computeNext` (`BlockCollisions.java:85-92`) 3D cursor walk
/// reusing a single scratch `Vec<B>` across cells to avoid per-block allocations.
pub fn visit_block_collision_cursor_with_scratch<M: CollisionBounds, B: CollisionBounds>(
    query: M,
    mut is_moving_piston_at: impl FnMut([i32; 3]) -> Option<bool>,
    mut boxes_at: impl FnMut([i32; 3], &mut Vec<B>) -> Option<()>,
    mut visit: impl FnMut([i32; 3], &[B]) -> Option<bool>,
) -> Option<bool> {
    if !(query.min_x().is_finite()
        && query.max_x().is_finite()
        && query.min_y().is_finite()
        && query.max_y().is_finite()
        && query.min_z().is_finite()
        && query.max_z().is_finite())
    {
        return Some(true);
    }
    let min_x = ((query.min_x() - CURSOR_EPSILON).floor() as i32).saturating_sub(1);
    let max_x = ((query.max_x() + CURSOR_EPSILON).floor() as i32).saturating_add(1);
    let min_y = ((query.min_y() - CURSOR_EPSILON).floor() as i32).saturating_sub(1);
    let max_y = ((query.max_y() + CURSOR_EPSILON).floor() as i32).saturating_add(1);
    let min_z = ((query.min_z() - CURSOR_EPSILON).floor() as i32).saturating_sub(1);
    let max_z = ((query.max_z() + CURSOR_EPSILON).floor() as i32).saturating_add(1);
    if collision_cursor_cell_count(min_x, max_x, min_y, max_y, min_z, max_z)
        > MAX_COLLISION_CURSOR_CELLS
    {
        return Some(true);
    }

    let mut scratch = Vec::new();
    // Cursor3D.java:35-38: x fastest, then y, then z.
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let boundary_count = usize::from(x == min_x || x == max_x)
                    + usize::from(y == min_y || y == max_y)
                    + usize::from(z == min_z || z == max_z);
                if boundary_count == 3 {
                    continue;
                }
                let pos = [x, y, z];
                if boundary_count == 2 && !is_moving_piston_at(pos)? {
                    continue;
                }
                scratch.clear();
                boxes_at(pos, &mut scratch)?;
                if scratch.is_empty() {
                    continue;
                }
                if boundary_count == 1 && !is_large_collision_shape(&scratch) {
                    continue;
                }
                if !visit(pos, &scratch)? {
                    return Some(false);
                }
            }
        }
    }
    Some(true)
}

/// `BlockCollisions.computeNext` (`BlockCollisions.java:85-92`) 3D cursor walk.
pub fn visit_block_collision_cursor<M: CollisionBounds, B: CollisionBounds>(
    query: M,
    mut is_moving_piston_at: impl FnMut([i32; 3]) -> Option<bool>,
    mut boxes_at: impl FnMut([i32; 3]) -> Option<Vec<B>>,
    mut visit: impl FnMut([i32; 3], Vec<B>) -> Option<bool>,
) -> Option<bool> {
    if !(query.min_x().is_finite()
        && query.max_x().is_finite()
        && query.min_y().is_finite()
        && query.max_y().is_finite()
        && query.min_z().is_finite()
        && query.max_z().is_finite())
    {
        return Some(true);
    }
    let min_x = ((query.min_x() - CURSOR_EPSILON).floor() as i32).saturating_sub(1);
    let max_x = ((query.max_x() + CURSOR_EPSILON).floor() as i32).saturating_add(1);
    let min_y = ((query.min_y() - CURSOR_EPSILON).floor() as i32).saturating_sub(1);
    let max_y = ((query.max_y() + CURSOR_EPSILON).floor() as i32).saturating_add(1);
    let min_z = ((query.min_z() - CURSOR_EPSILON).floor() as i32).saturating_sub(1);
    let max_z = ((query.max_z() + CURSOR_EPSILON).floor() as i32).saturating_add(1);
    if collision_cursor_cell_count(min_x, max_x, min_y, max_y, min_z, max_z)
        > MAX_COLLISION_CURSOR_CELLS
    {
        return Some(true);
    }

    // Cursor3D.java:35-38: x fastest, then y, then z.
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let boundary_count = usize::from(x == min_x || x == max_x)
                    + usize::from(y == min_y || y == max_y)
                    + usize::from(z == min_z || z == max_z);
                if boundary_count == 3 {
                    continue;
                }
                let pos = [x, y, z];
                if boundary_count == 2 && !is_moving_piston_at(pos)? {
                    continue;
                }
                let boxes = boxes_at(pos)?;
                if boxes.is_empty() {
                    continue;
                }
                if boundary_count == 1 && !is_large_collision_shape(&boxes) {
                    continue;
                }
                if !visit(pos, boxes)? {
                    return Some(false);
                }
            }
        }
    }
    Some(true)
}

/// Squared Euclidean length of `[x, y, z]`.
#[inline]
pub fn vec3_length_sqr(v: [f64; 3]) -> f64 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

/// Squared Euclidean distance (`Vec3.distanceToSqr`, `Vec3.java:131-136`).
#[inline]
pub fn vec3_distance_sqr(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// `Vec3.normalize()` (`Vec3.java:83-86`), returning zero below [`NORMALIZE_EPSILON`].
#[inline]
pub fn vec3_normalize(v: [f64; 3]) -> [f64; 3] {
    let length = vec3_length_sqr(v).sqrt();
    if length < NORMALIZE_EPSILON {
        [0.0; 3]
    } else {
        [v[0] / length, v[1] / length, v[2] / length]
    }
}

/// Hit point from ray/segment clipping against a block's shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayClipHit {
    pub pos: [i32; 3],
    pub face: BlockFace,
    pub location: [f64; 3],
}

/// `Direction.getApproximateNearest` (`Direction.java:311-330`).
pub fn approximate_nearest_face(vector: [f64; 3]) -> BlockFace {
    let mut face = BlockFace::North;
    let mut highest = f32::from_bits(1);
    for (candidate, [x, y, z]) in [
        (BlockFace::Down, [0.0, -1.0, 0.0]),
        (BlockFace::Up, [0.0, 1.0, 0.0]),
        (BlockFace::North, [0.0, 0.0, -1.0]),
        (BlockFace::South, [0.0, 0.0, 1.0]),
        (BlockFace::West, [-1.0, 0.0, 0.0]),
        (BlockFace::East, [1.0, 0.0, 0.0]),
    ] {
        let dot = vector[0] as f32 * x + vector[1] as f32 * y + vector[2] as f32 * z;
        if dot > highest {
            highest = dot;
            face = candidate;
        }
    }
    face
}

#[inline]
fn ray_lerp(alpha: f64, from: f64, to: f64) -> f64 {
    from + alpha * (to - from)
}

fn cell_coordinates(point: [f64; 3]) -> Option<[i32; 3]> {
    let mut result = [0; 3];
    for (axis, value) in point.into_iter().enumerate() {
        let floor = value.floor();
        if !value.is_finite() || floor < f64::from(i32::MIN) || floor > f64::from(i32::MAX) {
            return None;
        }
        result[axis] = floor as i32;
    }
    Some(result)
}

fn valid_box(shape: CollisionBox) -> bool {
    [
        shape.min_x,
        shape.min_y,
        shape.min_z,
        shape.max_x,
        shape.max_y,
        shape.max_z,
    ]
    .iter()
    .all(|value| value.is_finite())
        && shape.min_x <= shape.max_x
        && shape.min_y <= shape.max_y
        && shape.min_z <= shape.max_z
}

/// `AABB.clipPoint` (`AABB.java:370-395`).
fn clip_point(
    scale: &mut f64,
    face: &mut Option<BlockFace>,
    [da, db, dc]: [f64; 3],
    plane: f64,
    [min_b, max_b, min_c, max_c]: [f64; 4],
    [from_a, from_b, from_c]: [f64; 3],
    candidate: BlockFace,
) {
    let s = (plane - from_a) / da;
    let pb = from_b + s * db;
    let pc = from_c + s * dc;
    if 0.0 < s
        && s < *scale
        && min_b - COLLISION_EPSILON < pb
        && pb < max_b + COLLISION_EPSILON
        && min_c - COLLISION_EPSILON < pc
        && pc < max_c + COLLISION_EPSILON
    {
        *scale = s;
        *face = Some(candidate);
    }
}

/// `AABB.clip` (`AABB.java:344-367`): only the face entered along each axis can be clipped.
fn clip_box_faces(
    shape: CollisionBox,
    from: [f64; 3],
    scale: &mut f64,
    face: &mut Option<BlockFace>,
    [dx, dy, dz]: [f64; 3],
) {
    if dx > COLLISION_EPSILON {
        clip_point(
            scale,
            face,
            [dx, dy, dz],
            shape.min_x,
            [shape.min_y, shape.max_y, shape.min_z, shape.max_z],
            from,
            BlockFace::West,
        );
    } else if dx < -COLLISION_EPSILON {
        clip_point(
            scale,
            face,
            [dx, dy, dz],
            shape.max_x,
            [shape.min_y, shape.max_y, shape.min_z, shape.max_z],
            from,
            BlockFace::East,
        );
    }
    if dy > COLLISION_EPSILON {
        clip_point(
            scale,
            face,
            [dy, dz, dx],
            shape.min_y,
            [shape.min_z, shape.max_z, shape.min_x, shape.max_x],
            [from[1], from[2], from[0]],
            BlockFace::Down,
        );
    } else if dy < -COLLISION_EPSILON {
        clip_point(
            scale,
            face,
            [dy, dz, dx],
            shape.max_y,
            [shape.min_z, shape.max_z, shape.min_x, shape.max_x],
            [from[1], from[2], from[0]],
            BlockFace::Up,
        );
    }
    if dz > COLLISION_EPSILON {
        clip_point(
            scale,
            face,
            [dz, dx, dy],
            shape.min_z,
            [shape.min_x, shape.max_x, shape.min_y, shape.max_y],
            [from[2], from[0], from[1]],
            BlockFace::North,
        );
    } else if dz < -COLLISION_EPSILON {
        clip_point(
            scale,
            face,
            [dz, dx, dy],
            shape.max_z,
            [shape.min_x, shape.max_x, shape.min_y, shape.max_y],
            [from[2], from[0], from[1]],
            BlockFace::South,
        );
    }
}

/// Returns `(scale, entry_face)` for the segment `from -> to` entering `bounds` (`AABB.java:344-395`).
pub fn segment_box_entry<B: CollisionBounds>(
    bounds: B,
    from: [f64; 3],
    to: [f64; 3],
) -> Option<(f64, BlockFace)> {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let mut scale = 1.0;
    let mut face = None;
    clip_box_faces(
        bounds.to_collision_box(),
        from,
        &mut scale,
        &mut face,
        delta,
    );
    face.map(|face| (scale, face))
}

/// Returns only the segment scale `[0, 1]` where `from -> to` enters `bounds`.
#[inline]
pub fn segment_box_entry_scale<B: CollisionBounds>(
    bounds: B,
    from: [f64; 3],
    to: [f64; 3],
) -> Option<f64> {
    segment_box_entry(bounds, from, to).map(|(scale, _)| scale)
}

/// `VoxelShape.clip` (`VoxelShape.java:158-177`; `AABB.java:310-327`) for block-local boxes.
pub fn shape_clip<B: CollisionBounds>(
    boxes: &[B],
    from: [f64; 3],
    to: [f64; 3],
    pos: [i32; 3],
) -> Option<RayClipHit> {
    if boxes.is_empty()
        || boxes
            .iter()
            .any(|shape| !valid_box(shape.to_collision_box()))
    {
        return None;
    }
    let diff = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    if from.iter().chain(to.iter()).any(|value| !value.is_finite())
        || !vec3_length_sqr(diff).is_finite()
        || vec3_length_sqr(diff) < COLLISION_EPSILON
    {
        return None;
    }
    let test_point = [
        from[0] + diff[0] * 0.001,
        from[1] + diff[1] * 0.001,
        from[2] + diff[2] * 0.001,
    ];
    let local = [
        test_point[0] - f64::from(pos[0]),
        test_point[1] - f64::from(pos[1]),
        test_point[2] - f64::from(pos[2]),
    ];
    // `DiscreteVoxelShape.java:66-72`: coordinate cell is half-open on its upper faces.
    if boxes
        .iter()
        .any(|shape| shape.to_collision_box().contains_half_open(local))
    {
        return Some(RayClipHit {
            pos,
            face: approximate_nearest_face(diff).opposite(),
            location: test_point,
        });
    }
    let mut scale = 1.0;
    let mut face = None;
    let offset = [f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2])];
    for shape in boxes {
        clip_box_faces(
            shape.to_collision_box().moved(offset),
            from,
            &mut scale,
            &mut face,
            diff,
        );
    }
    let hit_face = face?;
    Some(RayClipHit {
        pos,
        face: hit_face,
        location: [
            from[0] + diff[0] * scale,
            from[1] + diff[1] * scale,
            from[2] + diff[2] * scale,
        ],
    })
}

/// `BlockGetter.clip` / `traverseBlocks` (`BlockGetter.java:63-80, 103-171`).
pub fn clip_ray_with<B, I, F>(
    from: [f64; 3],
    to: [f64; 3],
    mut cell: impl FnMut([i32; 3]) -> Option<(I, Option<F>)>,
) -> Option<Option<RayClipHit>>
where
    B: CollisionBounds,
    I: AsRef<[B]>,
    F: CollisionBounds,
{
    cell_coordinates(from)?;
    cell_coordinates(to)?;
    if from == to {
        return Some(None);
    }
    let nudged_from: [f64; 3] =
        std::array::from_fn(|axis| ray_lerp(-COLLISION_EPSILON, from[axis], to[axis]));
    let nudged_to: [f64; 3] =
        std::array::from_fn(|axis| ray_lerp(-COLLISION_EPSILON, to[axis], from[axis]));
    let mut current = cell_coordinates(nudged_from)?;
    cell_coordinates(nudged_to)?;
    let delta: [f64; 3] = std::array::from_fn(|axis| nudged_to[axis] - nudged_from[axis]);
    let sign: [i32; 3] = std::array::from_fn(|axis| {
        if delta[axis] > 0.0 {
            1
        } else if delta[axis] < 0.0 {
            -1
        } else {
            0
        }
    });
    let step: [f64; 3] = std::array::from_fn(|axis| {
        if sign[axis] == 0 {
            f64::MAX
        } else {
            f64::from(sign[axis]) / delta[axis]
        }
    });
    if step.iter().any(|step| !step.is_finite()) {
        return None;
    }
    let mut next: [f64; 3] = std::array::from_fn(|axis| {
        let fraction = nudged_from[axis] - nudged_from[axis].floor();
        step[axis]
            * if sign[axis] > 0 {
                1.0 - fraction
            } else {
                fraction
            }
    });
    const MAX_RAY_CLIP_STEPS: usize = 16_384;
    let mut steps = 0usize;
    loop {
        if steps >= MAX_RAY_CLIP_STEPS {
            return None;
        }
        steps += 1;
        let (block, fluid) = cell(current)?;
        let block_boxes = block.as_ref();
        if block_boxes
            .iter()
            .any(|shape| !valid_box(shape.to_collision_box()))
            || fluid
                .as_ref()
                .is_some_and(|shape| !valid_box(shape.to_collision_box()))
        {
            return None;
        }
        let block_hit = shape_clip(block_boxes, from, to, current);
        let fluid_hit = fluid.and_then(|shape| shape_clip(&[shape], from, to, current));
        let block_distance =
            block_hit.map_or(f64::MAX, |hit| vec3_distance_sqr(from, hit.location));
        let fluid_distance =
            fluid_hit.map_or(f64::MAX, |hit| vec3_distance_sqr(from, hit.location));
        let hit = if block_distance <= fluid_distance {
            block_hit
        } else {
            fluid_hit
        };
        if hit.is_some() {
            return Some(hit);
        }
        if next.iter().all(|next| *next > 1.0) {
            return Some(None);
        }
        let axis = if next[0] < next[1] {
            if next[0] < next[2] {
                0
            } else {
                2
            }
        } else if next[1] < next[2] {
            1
        } else {
            2
        };
        current[axis] = current[axis].checked_add(sign[axis])?;
        let advanced = next[axis] + step[axis];
        if !advanced.is_finite() || advanced <= next[axis] {
            return None;
        }
        next[axis] = advanced;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swept_box_slides_along_wall_with_vanilla_axis_order() {
        let wall = [CollisionBox::of_block_xyz(1, 0, 0)];
        let player = CollisionBox::from_arrays([0.2, 0.0, 0.2], [0.8, 1.8, 0.8]);
        let resolved = collide_with_boxes([0.5, 0.0, 0.25], player, &wall);
        assert!((resolved[0] - 0.2).abs() < 1e-12);
        assert!((resolved[2] - 0.25).abs() < 1e-12);
    }

    #[test]
    fn step_up_climbs_slab_when_horizontal_progress_increases() {
        let slab = CollisionBox::from_arrays([1.0, 0.0, 0.0], [2.0, 0.5, 1.0]);
        let player = CollisionBox::from_arrays([0.2, 0.0, 0.2], [0.8, 1.8, 0.8]);
        let delta = [0.5, 0.0, 0.0];
        let clipped = collide_with_boxes(delta, player, &[slab]);
        let stepped = collide_with_step(player, delta, clipped, true, 0.6, |_| Some(vec![slab]))
            .expect("step query succeeds");
        assert!((stepped[0] - 0.5).abs() < 1e-12);
        assert!((stepped[1] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn ray_clip_and_segment_entry_hit_nearest_face() {
        let unit = [CollisionBox::UNIT_CUBE];
        let hit = clip_ray_with([0.5, 0.5, -1.0], [0.5, 0.5, 2.0], |pos| {
            let blocks: &[CollisionBox] = if pos == [0, 0, 0] { &unit } else { &[] };
            Some((blocks, None::<CollisionBox>))
        })
        .expect("finite ray")
        .expect("hits block at origin");
        assert_eq!(hit.pos, [0, 0, 0]);
        assert_eq!(hit.face, BlockFace::North);
        assert!((hit.location[2] - 0.0).abs() < 1e-12);

        let scale =
            segment_box_entry_scale(CollisionBox::UNIT_CUBE, [-1.0, 0.5, 0.5], [1.0, 0.5, 0.5]);
        assert_eq!(scale, Some(0.5));
    }
}
