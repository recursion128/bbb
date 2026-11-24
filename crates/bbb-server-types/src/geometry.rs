//! Boxes, axis order and the swept-collision solver `Entity.move` runs on.
//!
//! Vanilla resolves entity movement against `VoxelShape`s, a discretized
//! bitset form built over per-axis coordinate lists. This port resolves it
//! against the axis-aligned boxes `BlockState.getCollisionShape(...).toAabbs()`
//! reports, which is what the generated 26.1 collision-shape facts hold
//! (`bbb_protocol::block_state_types::vanilla_block_state_static_collision_shape`).
//!
//! The two agree for every shape this slice meets, because `VoxelShape.collideX`
//! (`net/minecraft/world/phys/shapes/VoxelShape.java:251-306`) only ever reads
//! the coordinate immediately before or after the moving box on the swept axis
//! and only tests whether the shape has *any* filled cell overlapping the
//! moving box on the other two axes. For a shape re-expressed as its list of
//! filled boxes that is exactly a per-box test, and the discretization's
//! `1.0E-7` guard bands are transcribed rather than rounded away, so the
//! touching and marginally-overlapping cases keep vanilla's answers.
//!
//! What is *not* equivalent is a shape whose filled cells do not decompose into
//! the boxes `toAabbs` reports as one solid run each; no built-in 26.1 collision
//! shape is of that kind, since every one is authored as a union of boxes.

use bbb_facts::block_state_types::{
    vanilla_block_state_static_collision_shape, VanillaBlockCollisionShape,
};
use bbb_facts::block_types::VanillaVisualShapeOverride;
pub use bbb_facts::collision::{Axis, COLLISION_EPSILON, CURSOR_EPSILON, NORMALIZE_EPSILON};
use bbb_facts::collision::{CollisionBounds, CollisionBox};
use bbb_protocol::packets::{BlockPos, Vec3d};

mod step;
pub use step::collide_with_step;

mod box_shape;
pub use box_shape::{collision_face_full, BoxShape};

/// `Direction.axisStepOrder(Vec3)` (`Direction.java:379-381`).
pub fn axis_step_order(movement: Vec3d) -> [Axis; 3] {
    bbb_facts::collision::axis_step_order(movement.x, movement.z)
}

/// Component access and the arithmetic `Vec3` carries as methods in vanilla.
///
/// `bbb_protocol::packets::Vec3d` is the repository's one double-precision
/// triple and it is a plain data carrier, so the operations live here as an
/// extension rather than as a second vector type.
pub trait Vec3Ops: Sized {
    fn axis(self, axis: Axis) -> f64;
    fn with_axis(self, axis: Axis, value: f64) -> Self;
    /// `Vec3.add(Vec3)`.
    fn plus(self, other: Self) -> Self;
    /// `Vec3.multiply(double, double, double)`.
    fn multiply(self, x: f64, y: f64, z: f64) -> Self;
    /// `Vec3.scale(double)`.
    fn scale(self, factor: f64) -> Self;
    /// `Vec3.lengthSqr()`.
    fn length_sqr(self) -> f64;
    /// `Vec3.horizontalDistanceSqr()` (`Vec3.java:192-194`).
    fn horizontal_distance_sqr(self) -> f64;
    /// `Vec3.normalize()` (`Vec3.java:83-86`): a vector shorter than
    /// [`NORMALIZE_EPSILON`] normalises to `Vec3.ZERO` rather than blowing up.
    fn normalize(self) -> Self;
    /// `Vec3.distanceToSqr(Vec3)` (`Vec3.java:131-136`), which is also what
    /// `Entity.distanceToSqr` answers for two positions.
    fn distance_to_sqr(self, other: Self) -> f64;
}

impl Vec3Ops for Vec3d {
    fn axis(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }

    fn with_axis(self, axis: Axis, value: f64) -> Self {
        match axis {
            Axis::X => Self { x: value, ..self },
            Axis::Y => Self { y: value, ..self },
            Axis::Z => Self { z: value, ..self },
        }
    }

    fn plus(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn normalize(self) -> Self {
        let length = self.length_sqr().sqrt();
        if length < NORMALIZE_EPSILON {
            Self {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        } else {
            Self {
                x: self.x / length,
                y: self.y / length,
                z: self.z / length,
            }
        }
    }

    fn distance_to_sqr(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    fn multiply(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            x: self.x * x,
            y: self.y * y,
            z: self.z * z,
        }
    }

    fn scale(self, factor: f64) -> Self {
        self.multiply(factor, factor, factor)
    }

    fn length_sqr(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    fn horizontal_distance_sqr(self) -> f64 {
        self.x * self.x + self.z * self.z
    }
}

/// `LongJumpUtil.calculateJumpVectorForAngle` (`LongJumpUtil.java:11-63`).
pub fn long_jump_velocity(
    origin: Vec3d,
    target: Vec3d,
    max_speed: f64,
    angle: i32,
    gravity: f64,
    transition_clear: Option<&mut dyn FnMut(Vec3d, Vec3d) -> bool>,
) -> Option<Vec3d> {
    let plane = Vec3d {
        x: target.x - origin.x,
        y: 0.0,
        z: target.z - origin.z,
    };
    let normalized_plane = plane.normalize();
    let offset = Vec3d {
        x: normalized_plane.x * 0.5,
        y: normalized_plane.y * 0.5,
        z: normalized_plane.z * 0.5,
    };
    let target_position = Vec3d {
        x: target.x - offset.x,
        y: target.y - offset.y,
        z: target.z - offset.z,
    };
    let direction = Vec3d {
        x: target_position.x - origin.x,
        y: target_position.y - origin.y,
        z: target_position.z - origin.z,
    };
    // `float angrad = angle * (float)Math.PI / 180.0F` (`:18`).
    let angrad = angle as f32 * (std::f32::consts::PI / 180.0);
    let xz_angle = direction.z.atan2(direction.x);
    let r2 = direction.x * direction.x + direction.z * direction.z;
    let r = r2.sqrt();
    let y = direction.y;
    let sin_2ang = f64::from(2.0_f32 * angrad).sin();
    let cos_ang = f64::from(angrad.cos());
    let cos_ang_sqr = cos_ang.powf(2.0);
    let sin_ang = f64::from(angrad.sin());
    let denominator = r * sin_2ang - 2.0 * y * cos_ang_sqr;
    let v0_sqr = r2 * gravity / denominator;
    if v0_sqr.is_nan() || v0_sqr < 0.0 {
        return None;
    }
    let v0 = v0_sqr.sqrt();
    if v0 > max_speed {
        return None;
    }
    let v0r = v0 * cos_ang;
    let v0y = v0 * sin_ang;
    if let Some(transition_clear) = transition_clear {
        let samples = (r / v0r).ceil() as i32 * 2;
        let mut ri = 0.0;
        let mut previous: Option<Vec3d> = None;
        for _ in 0..samples - 1 {
            ri += r / f64::from(samples);
            let yi = sin_ang / cos_ang * ri - ri.powf(2.0) * gravity / (2.0 * v0_sqr * cos_ang_sqr);
            let xi = ri * xz_angle.cos();
            let zi = ri * xz_angle.sin();
            let sample = Vec3d {
                x: origin.x + xi,
                y: origin.y + yi,
                z: origin.z + zi,
            };
            if let Some(previous) = previous {
                if !transition_clear(previous, sample) {
                    return None;
                }
            }
            previous = Some(sample);
        }
    }
    let scale = f64::from(0.95_f32);
    Some(Vec3d {
        x: v0r * xz_angle.cos() * scale,
        y: v0y * scale,
        z: v0r * xz_angle.sin() * scale,
    })
}

/// `LongJumpUtil.isClearTransition` (`LongJumpUtil.java:65-80`).
pub fn long_jump_transition_clear(
    from: Vec3d,
    to: Vec3d,
    width: f64,
    height: f64,
    mut can_fit: impl FnMut(Aabb) -> bool,
) -> bool {
    let direction = Vec3d {
        x: to.x - from.x,
        y: to.y - from.y,
        z: to.z - from.z,
    };
    let min_dimension = width.min(height);
    let checks = (direction.length_sqr().sqrt() / min_dimension).ceil() as i32;
    let normalized = direction.normalize();
    let step = min_dimension * 0.9;
    let mut next = from;
    for index in 0..checks {
        next = if index == checks - 1 {
            to
        } else {
            Vec3d {
                x: next.x + normalized.x * step,
                y: next.y + normalized.y * step,
                z: next.z + normalized.z * step,
            }
        };
        let area = Aabb {
            min_x: next.x - width / 2.0,
            min_y: next.y,
            min_z: next.z - width / 2.0,
            max_x: next.x + width / 2.0,
            max_y: next.y + height,
            max_z: next.z + width / 2.0,
        };
        if !can_fit(area) {
            return false;
        }
    }
    true
}

/// `AABB` (`net/minecraft/world/phys/AABB.java`).
///
/// The six-field shape matches both vanilla's record and the generated
/// collision-box facts, so neither end needs a conversion that could reorder
/// the corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl Aabb {
    /// `EntityDimensions.makeBoundingBox(double, double, double)`
    /// (`net/minecraft/world/entity/EntityDimensions.java:19-23`): the entity
    /// stands centred on its position with the box growing upwards only.
    ///
    /// Vanilla halves the width in `float` and widens to `double` afterwards,
    /// which is why the halving happens before the conversion here as well.
    /// `AABB.ofSize(Vec3, double, double, double)` (`AABB.java:47-54`).
    pub fn of_size(center: Vec3d, x_size: f64, y_size: f64, z_size: f64) -> Self {
        Self {
            min_x: center.x - x_size / 2.0,
            min_y: center.y - y_size / 2.0,
            min_z: center.z - z_size / 2.0,
            max_x: center.x + x_size / 2.0,
            max_y: center.y + y_size / 2.0,
            max_z: center.z + z_size / 2.0,
        }
    }

    /// `AABB.getCenter()` (`AABB.java:378-380`).
    pub fn center(self) -> Vec3d {
        Vec3d {
            x: (self.min_x + self.max_x) / 2.0,
            y: (self.min_y + self.max_y) / 2.0,
            z: (self.min_z + self.max_z) / 2.0,
        }
    }

    pub fn of_entity(position: Vec3d, width: f32, height: f32) -> Self {
        let half_width = f64::from(width / 2.0f32);
        Self {
            min_x: position.x - half_width,
            min_y: position.y,
            min_z: position.z - half_width,
            max_x: position.x + half_width,
            max_y: position.y + f64::from(height),
            max_z: position.z + half_width,
        }
    }

    /// The unit cube a block shape is offset into (`VoxelShape.move(BlockPos)`).
    pub fn of_block(pos: BlockPos) -> Self {
        Self {
            min_x: f64::from(pos.x),
            min_y: f64::from(pos.y),
            min_z: f64::from(pos.z),
            max_x: f64::from(pos.x) + 1.0,
            max_y: f64::from(pos.y) + 1.0,
            max_z: f64::from(pos.z) + 1.0,
        }
    }

    /// `AABB.distanceToSqr(Vec3)` (AABB.java:415-420): zero inside the box, and
    /// the squared gap to the nearest face outside it.
    pub fn distance_to_sqr(&self, point: Vec3d) -> f64 {
        let dx = (self.min_x - point.x).max(point.x - self.max_x).max(0.0);
        let dy = (self.min_y - point.y).max(point.y - self.max_y).max(0.0);
        let dz = (self.min_z - point.z).max(point.z - self.max_z).max(0.0);
        dx * dx + dy * dy + dz * dz
    }

    /// AABB.java:209-217.
    pub fn minmax(self, other: Self) -> Self {
        Self::from(CollisionBox::from(self).minmax(CollisionBox::from(other)))
    }

    /// `AABB.expandTowards(double, double, double)` (`AABB.java:157-183`).
    pub fn expand_towards(self, delta: Vec3d) -> Self {
        Self::from(CollisionBox::from(self).expand_towards([delta.x, delta.y, delta.z]))
    }

    /// `AABB.inflate(double, double, double)` (`AABB.java:185-193`).
    pub fn inflate(self, x: f64, y: f64, z: f64) -> Self {
        Self::from(CollisionBox::from(self).inflate(x, y, z))
    }

    /// `AABB.move(Vec3)` (`AABB.java:219-231`).
    pub fn moved(self, delta: Vec3d) -> Self {
        Self::from(CollisionBox::from(self).moved([delta.x, delta.y, delta.z]))
    }

    /// `AABB.intersects(AABB)` (`AABB.java:237-239`).
    pub fn intersects(self, other: Self) -> bool {
        CollisionBox::from(self).intersects(CollisionBox::from(other))
    }

    /// `AABB.min(Direction.Axis)`.
    pub fn min(self, axis: Axis) -> f64 {
        self.min_on(axis)
    }

    /// `AABB.max(Direction.Axis)`.
    pub fn max(self, axis: Axis) -> f64 {
        self.max_on(axis)
    }

    /// Whether this is exactly `Shapes.block()`, the full unit cube (`BlockCollisions.java:94-97`).
    pub fn is_unit_cube(self) -> bool {
        CollisionBox::from(self).is_unit_cube()
    }
}

impl From<CollisionBox> for Aabb {
    #[inline]
    fn from(value: CollisionBox) -> Self {
        Self {
            min_x: value.min_x,
            min_y: value.min_y,
            min_z: value.min_z,
            max_x: value.max_x,
            max_y: value.max_y,
            max_z: value.max_z,
        }
    }
}

impl From<Aabb> for CollisionBox {
    #[inline]
    fn from(value: Aabb) -> Self {
        Self {
            min_x: value.min_x,
            min_y: value.min_y,
            min_z: value.min_z,
            max_x: value.max_x,
            max_y: value.max_y,
            max_z: value.max_z,
        }
    }
}

impl CollisionBounds for Aabb {
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

/// `BlockBehaviour.BlockStateBase.Cache.largeCollisionShape` (`BlockBehaviour.java:930-931`).
pub fn is_large_collision_shape(boxes: &[Aabb]) -> bool {
    bbb_facts::collision::is_large_collision_shape(boxes)
}

/// The empty-context cached collision shape of a built-in 26.1 block state, as
/// its list of filled boxes in block-local coordinates.
///
/// `None` marks a state whose shape vanilla recomputes from the live world
/// (`BlockBehaviour.Properties.dynamicShape`); the generated facts have no
/// cached answer for those, so the decision of what to do about them belongs to
/// whoever owns the live world. See `bbb_server_core::entity::EntityLevel::block_collision_boxes`.
/// `BlockState.getShape(level, pos)` in the empty context, which is the shape
/// `ClipContext.Block.OUTLINE` reads (`ClipContext.java:56-60`). `None` is a
/// dynamic state whose shape needs a live level, position or collision
/// context.
pub fn static_block_outline_boxes(block_state_id: i32) -> Option<Vec<Aabb>> {
    let shape =
        bbb_protocol::block_state_types::vanilla_block_state_static_outline_shape(block_state_id)?;
    Some(static_shape_boxes(shape).collect())
}

pub fn static_block_collision_boxes(block_state_id: i32) -> Option<Vec<Aabb>> {
    Some(static_block_collision_box_iter(block_state_id)?.collect())
}

pub fn push_static_block_collision_boxes(block_state_id: i32, out: &mut Vec<Aabb>) -> Option<()> {
    out.extend(static_block_collision_box_iter(block_state_id)?);
    Some(())
}

/// `BlockState.getCollisionShape(level, pos)` (`CollisionContext.empty()`) for
/// every built-in 26.1 block state without block-entity access (`PowderSnowBlock.java:116-132`,
/// `ScaffoldingBlock.java:137-146`, `ShulkerBoxBlock.java:152-156`,
/// `MovingPistonBlock.java:106-113`, `BambooStalkBlock.java:73-75`,
/// `PointedDripstoneBlock.java:260-279`).
pub fn empty_context_block_collision_boxes(
    block_state_id: i32,
    pos: BlockPos,
) -> Option<Vec<Aabb>> {
    use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_POWDER_SNOW_ID, VANILLA_BLOCK_SCAFFOLDING_ID,
    };
    match vanilla_block_state_block_type_id(block_state_id)? {
        VANILLA_BLOCK_MOVING_PISTON_ID | VANILLA_BLOCK_POWDER_SNOW_ID => Some(Vec::new()),
        VANILLA_BLOCK_SCAFFOLDING_ID => {
            scaffolding::collision_boxes(block_state_id, pos, scaffolding::Context::Empty)
        }
        _ if crate::block_queries::is_shulker_box(block_state_id) => {
            Some(vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
        }
        _ => positioned_shapes::collision_boxes(block_state_id, pos),
    }
}

pub fn static_block_collision_box_iter(
    block_state_id: i32,
) -> Option<impl ExactSizeIterator<Item = Aabb>> {
    Some(static_shape_boxes(
        vanilla_block_state_static_collision_shape(block_state_id)?,
    ))
}

fn static_shape_boxes(shape: VanillaBlockCollisionShape) -> impl ExactSizeIterator<Item = Aabb> {
    shape.boxes().map(|shape| Aabb {
        min_x: shape.min_x,
        min_y: shape.min_y,
        min_z: shape.min_z,
        max_x: shape.max_x,
        max_y: shape.max_y,
        max_z: shape.max_z,
    })
}

/// `BlockBehaviour.getVisualShape` (`BlockBehaviour.java:345-347`) for a
/// built-in 26.1 block state, which is the shape `ClipContext.Block.VISUAL`
/// reads (`ClipContext.java:59`).
///
/// Only seven 26.1 classes override it. `TransparentBlock` and `IronBarsBlock`
/// (glass, stained glass, tinted glass, copper grates, bars and panes) and
/// `PowderSnowBlock` answer the empty shape; `SnowLayerBlock` and `FenceBlock`
/// answer `getShape`, which is the outline palette; `MudBlock` and
/// `SoulSandBlock` answer a full block, which is *taller* than their collision
/// shape. Everyone else answers the collision shape.
///
/// The empty answer is a real answer, not a missing one: `Ok(vec![])` says the
/// ray passes through, while `None` says this state's shape needs a live world
/// and the caller has to decide. The three empty owners are level-independent,
/// so they answer `Some` even for a state whose collision shape is dynamic.
pub fn static_block_visual_boxes(block_state_id: i32) -> Option<Vec<Aabb>> {
    let owner = bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .and_then(|entry| entry.visual_shape_override);
    match owner {
        Some(
            VanillaVisualShapeOverride::TransparentBlock
            | VanillaVisualShapeOverride::IronBarsBlock
            | VanillaVisualShapeOverride::PowderSnowBlock,
        ) => Some(Vec::new()),
        Some(
            VanillaVisualShapeOverride::SnowLayerBlock | VanillaVisualShapeOverride::FenceBlock,
        ) => static_block_outline_boxes(block_state_id),
        Some(VanillaVisualShapeOverride::MudBlock | VanillaVisualShapeOverride::SoulSandBlock) => {
            Some(vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
        }
        None => static_block_collision_boxes(block_state_id),
    }
}

/// BlockCollisions.java:85-102: non-piston shapes use the same cursor.
pub fn collect_block_collisions(
    query: Aabb,
    boxes_at: impl FnMut(BlockPos) -> Vec<Aabb>,
) -> Vec<(BlockPos, Aabb)> {
    collect_block_collisions_with_moving_pistons(query, |_| false, boxes_at)
}

/// BlockCollisions.java:85-102: current moving-piston identity admits edges before shape lookup.
pub fn collect_block_collisions_with_moving_pistons(
    query: Aabb,
    is_moving_piston_at: impl FnMut(BlockPos) -> bool,
    mut boxes_at: impl FnMut(BlockPos) -> Vec<Aabb>,
) -> Vec<(BlockPos, Aabb)> {
    let mut collected = Vec::new();
    let _ = visit_block_collisions_with_scratch(
        query,
        is_moving_piston_at,
        |pos, out| out.extend(boxes_at(pos)),
        |pos, shape| {
            collected.push((pos, shape));
            true
        },
    );
    collected
}

/// Zero-allocation cursor walk yielding every world-space collision box intersecting `query`.
pub fn visit_block_collisions_with_scratch(
    query: Aabb,
    mut is_moving_piston_at: impl FnMut(BlockPos) -> bool,
    mut boxes_at: impl FnMut(BlockPos, &mut Vec<Aabb>),
    mut visit: impl FnMut(BlockPos, Aabb) -> bool,
) -> bool {
    visit_block_collision_shapes_with_scratch(
        query,
        |pos| Some(is_moving_piston_at(pos)),
        |pos, out| {
            boxes_at(pos, out);
            Some(())
        },
        |pos, boxes| {
            for shape in intersecting_boxes(query, pos, boxes) {
                if !visit(pos, shape) {
                    return Some(false);
                }
            }
            Some(true)
        },
    )
    .unwrap_or(true)
}

/// CollisionGetter.java:59-67: stop at the first block collision, before later unknown reads.
pub fn no_block_collision_with_moving_pistons(
    query: Aabb,
    is_moving_piston_at: impl FnMut(BlockPos) -> Option<bool>,
    mut boxes_at: impl FnMut(BlockPos) -> Option<Vec<Aabb>>,
) -> Option<bool> {
    no_block_collision_with_scratch(query, is_moving_piston_at, |pos, out| {
        out.extend(boxes_at(pos)?);
        Some(())
    })
}

/// Zero-allocation `CollisionGetter.noBlockCollision` reusing one scratch buffer across cells.
pub fn no_block_collision_with_scratch(
    query: Aabb,
    is_moving_piston_at: impl FnMut(BlockPos) -> Option<bool>,
    boxes_at: impl FnMut(BlockPos, &mut Vec<Aabb>) -> Option<()>,
) -> Option<bool> {
    visit_block_collision_shapes_with_scratch(query, is_moving_piston_at, boxes_at, |pos, boxes| {
        Some(intersecting_boxes(query, pos, boxes).next().is_none())
    })
}

// BlockCollisions.java:94-105: a hit yields the entire shape, including boxes outside the query.
pub fn collect_block_collision_shapes_checked(
    query: Aabb,
    is_moving_piston_at: impl FnMut(BlockPos) -> Option<bool>,
    boxes_at: impl FnMut(BlockPos) -> Option<Vec<Aabb>>,
) -> Option<Vec<(BlockPos, BoxShape)>> {
    let mut collected = Vec::new();
    visit_block_collision_shapes_checked(query, is_moving_piston_at, boxes_at, |pos, shape| {
        collected.push((pos, shape));
        Some(true)
    })?;
    Some(collected)
}

pub fn visit_block_collision_shapes_checked(
    query: Aabb,
    is_moving_piston_at: impl FnMut(BlockPos) -> Option<bool>,
    mut boxes_at: impl FnMut(BlockPos) -> Option<Vec<Aabb>>,
    visit: impl FnMut(BlockPos, BoxShape) -> Option<bool>,
) -> Option<bool> {
    visit_block_collision_shapes_checked_with_scratch(
        query,
        is_moving_piston_at,
        |pos, out| {
            out.extend(boxes_at(pos)?);
            Some(())
        },
        visit,
    )
}

pub fn visit_block_collision_shapes_checked_with_scratch(
    query: Aabb,
    is_moving_piston_at: impl FnMut(BlockPos) -> Option<bool>,
    boxes_at: impl FnMut(BlockPos, &mut Vec<Aabb>) -> Option<()>,
    mut visit: impl FnMut(BlockPos, BoxShape) -> Option<bool>,
) -> Option<bool> {
    let query_shape = BoxShape::from_box(query)?;
    let minima = [query.min_x, query.min_y, query.min_z];
    let maxima = [query.max_x, query.max_y, query.max_z];
    let mut cells = 1_u64;
    for axis in 0..3 {
        let min = (minima[axis] - CURSOR_EPSILON).floor();
        let max = (maxima[axis] + CURSOR_EPSILON).floor();
        if min <= f64::from(i32::MIN) || max >= f64::from(i32::MAX) {
            return None;
        }
        cells = cells.checked_mul((max - min + 3.0) as u64)?;
        if cells > 1 << 20 {
            return None;
        }
    }
    visit_block_collision_shapes_with_scratch(query, is_moving_piston_at, boxes_at, |pos, boxes| {
        let cube = boxes.len() == 1 && boxes[0].is_unit_cube();
        let shape = BoxShape::from_boxes(boxes)?;
        let shape = shape.moved(Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        })?;
        let hit = if cube {
            query.intersects(Aabb::of_block(pos))
        } else {
            shape.intersects(&query_shape)?
        };
        if hit {
            visit(pos, shape)
        } else {
            Some(true)
        }
    })
}

fn intersecting_boxes(
    query: Aabb,
    pos: BlockPos,
    boxes: &[Aabb],
) -> impl Iterator<Item = Aabb> + '_ {
    let offset = Vec3d {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
        z: f64::from(pos.z),
    };
    boxes
        .iter()
        .copied()
        .map(move |shape| shape.moved(offset))
        .filter(move |shape| query.intersects(*shape))
}

fn visit_block_collision_shapes_with_scratch(
    query: Aabb,
    mut is_moving_piston_at: impl FnMut(BlockPos) -> Option<bool>,
    mut boxes_at: impl FnMut(BlockPos, &mut Vec<Aabb>) -> Option<()>,
    mut visit: impl FnMut(BlockPos, &[Aabb]) -> Option<bool>,
) -> Option<bool> {
    bbb_facts::collision::visit_block_collision_cursor_with_scratch(
        query,
        |[x, y, z]| is_moving_piston_at(BlockPos { x, y, z }),
        |[x, y, z], scratch| boxes_at(BlockPos { x, y, z }, scratch),
        |[x, y, z], boxes| visit(BlockPos { x, y, z }, boxes),
    )
}

/// `Entity.collideWithShapes(Vec3, AABB, List<VoxelShape>)` (`Entity.java:1159-1175`).
pub fn collide_with_boxes(movement: Vec3d, bounding_box: Aabb, boxes: &[Aabb]) -> Vec3d {
    let [x, y, z] = bbb_facts::collision::collide_with_boxes(
        [movement.x, movement.y, movement.z],
        bounding_box,
        boxes,
    );
    Vec3d { x, y, z }
}

/// `Shapes.collide` (`Shapes.java:231-241`) over `VoxelShape.collideX` (`VoxelShape.java:251-306`).
pub fn collide_axis(axis: Axis, moving: Aabb, boxes: &[Aabb], distance: f64) -> f64 {
    bbb_facts::collision::collide_axis(axis, moving, boxes, distance)
}

/// `VoxelShape.collideX` (`VoxelShape.java:268-271`) face-overlap test.
pub fn overlaps_on(moving: Aabb, shape: Aabb, axis: Axis) -> bool {
    bbb_facts::collision::overlaps_on(moving, shape, axis)
}

/// `Mth.sin(double)` (`net/minecraft/util/Mth.java:49-51`) over the 65536-entry
/// table (`:34-38`): the table entry is the answer, not `f64::sin`.
pub fn mth_sin(radians: f64) -> f32 {
    sin_table()[((radians * SIN_SCALE) as i64 & 0xFFFF) as usize]
}

/// `Mth.cos(double)` (`Mth.java:53-55`).
pub fn mth_cos(radians: f64) -> f32 {
    sin_table()[((radians * SIN_SCALE + 16384.0) as i64 & 0xFFFF) as usize]
}

/// `Mth.SIN_SCALE` (`Mth.java:33`).
pub const SIN_SCALE: f64 = 10430.378350470453;

pub fn sin_table() -> &'static [f32; 65536] {
    static TABLE: std::sync::OnceLock<Box<[f32; 65536]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Box::new([0.0_f32; 65536]);
        for (index, entry) in table.iter_mut().enumerate() {
            *entry = (index as f64 / SIN_SCALE).sin() as f32;
        }
        table
    })
}

#[cfg(test)]
mod tests;

pub mod clip;
pub mod positioned_shapes;
pub mod scaffolding;
