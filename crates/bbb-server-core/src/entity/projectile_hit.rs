//! `ProjectileUtil.getHitResultOnMoveVector` for a server-owned projectile: the
//! collider block clip (`BlockGetter.clip`) followed by the entity sweep
//! (`ProjectileUtil.getEntityHitResult`), and the `Mth`/`AABB` arithmetic
//! both are written in.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_property,
};
use bbb_protocol::block_types;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};

use crate::entity::level::NearbyPlayer;
use crate::entity::store::{section_of_block, section_walk_order_key, EntityStore, SectionKey};
use crate::entity::{Entity, EntityData};
use bbb_server_types::geometry::{Aabb, Vec3Ops};

/// One entity selected by the shared projectile sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileTarget {
    Stored { entity_id: i32 },
    Player { uuid: uuid::Uuid, entity_id: i32 },
}

impl ProjectileTarget {
    pub const fn entity_id(self) -> i32 {
        match self {
            Self::Stored { entity_id } | Self::Player { entity_id, .. } => entity_id,
        }
    }

    pub const fn uuid(self) -> Option<uuid::Uuid> {
        match self {
            Self::Stored { .. } => None,
            Self::Player { uuid, .. } => Some(uuid),
        }
    }
}

/// The candidate projection a subtype predicate sees before the sweep.
#[derive(Debug, Clone, Copy)]
pub enum ProjectileCandidate<'a> {
    /// A stored dragon projects the identity shared by its pickable parts.
    Stored(&'a Entity),
    Player(&'a NearbyPlayer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileDeflection {
    None,
    Reverse,
    BreezeReverse,
}

impl ProjectileCandidate<'_> {
    pub const fn target(self) -> ProjectileTarget {
        match self {
            Self::Stored(entity) => ProjectileTarget::Stored {
                entity_id: entity.id(),
            },
            Self::Player(player) => ProjectileTarget::Player {
                uuid: player.uuid,
                entity_id: player.entity_id,
            },
        }
    }

    pub fn is_alive_and_pickable(self) -> bool {
        match self {
            Self::Stored(entity) => {
                entity.is_alive()
                    && (crate::entity::ender_dragon::state(entity).is_some()
                        || entity.can_be_hit_by_projectile())
            }
            Self::Player(player) => !player.spectator && !player.dead_or_dying,
        }
    }

    pub fn is_alive_and_hurting_projectile_target(self) -> bool {
        match self {
            Self::Stored(entity) => {
                entity.is_alive()
                    && (crate::entity::ender_dragon::state(entity).is_some()
                        || entity.can_be_hit_by_hurting_projectile())
            }
            Self::Player(player) => !player.spectator && !player.dead_or_dying,
        }
    }

    pub(crate) fn is_alive_and_pickable_in_level(self, level: &LevelCtx<'_, '_>) -> bool {
        match self {
            Self::Stored(entity) => {
                entity.is_alive()
                    && (crate::entity::ender_dragon::state(entity).is_some()
                        || entity.can_be_hit_by_projectile_in_level(level))
            }
            Self::Player(player) => !player.spectator && !player.dead_or_dying,
        }
    }

    pub(crate) fn is_alive_and_hurting_projectile_target_in_level(
        self,
        level: &LevelCtx<'_, '_>,
    ) -> bool {
        match self {
            Self::Stored(entity) => {
                entity.is_alive()
                    && (crate::entity::ender_dragon::state(entity).is_some()
                        || entity.can_be_hit_by_hurting_projectile_in_level(level))
            }
            Self::Player(player) => !player.spectator && !player.dead_or_dying,
        }
    }
}

/// `HitResult` reduced to the fields current server projectile owners read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitResult {
    Miss,
    Block {
        location: Vec3d,
        pos: BlockPos,
        direction: Direction,
        /// `BlockHitResult.isWorldBorderHit`.
        world_border: bool,
    },
    Entity {
        target: ProjectileTarget,
        location: Vec3d,
        deflection: ProjectileDeflection,
    },
}

/// `ProjectileUtil.getHitResultOnMoveVector(source, matching)`
/// (`ProjectileUtil.java`) over `getHitResult` (`:63-84`).
pub(crate) fn hit_result_on_move_vector(
    source: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    matching: impl Fn(ProjectileCandidate<'_>, &EntityStore, &LevelCtx<'_, '_>) -> bool,
) -> Option<HitResult> {
    let movement = source.delta_movement();
    let from = source.position();
    let mut to = from.plus(movement);
    let block_hit = clip_including_border(level, from, to);
    if let Some((location, ..)) = block_hit {
        to = location;
    }
    let mut nearest = f64::MAX;
    let mut hit = None;
    for_each_candidate_hit(
        store,
        level,
        source,
        from,
        to,
        matching,
        false,
        |target, location, _, section| {
            let dd = from.distance_to_sqr(location);
            let key = (section_walk_order_key(section), target.entity_id());
            if dd < nearest
                || (dd == nearest && hit.as_ref().is_some_and(|(previous, _, _)| key < *previous))
            {
                nearest = dd;
                hit = Some((key, target, location));
            }
        },
    );
    if let Some((_, target, location)) = hit {
        let deflection = target_deflection(source, store, level, target)?;
        return Some(HitResult::Entity {
            target,
            location,
            deflection,
        });
    }
    Some(match block_hit {
        Some((location, pos, direction, world_border)) => HitResult::Block {
            location,
            pos,
            direction,
            world_border,
        },
        None => HitResult::Miss,
    })
}

pub(crate) fn target_deflection(
    source: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    target: ProjectileTarget,
) -> Option<ProjectileDeflection> {
    let (resource_id, breeze) = match target {
        ProjectileTarget::Stored { entity_id } => {
            let entity = store.get(entity_id).or_else(|| {
                let (dragon_id, _, _) = crate::entity::ender_dragon::part_by_id(store, entity_id)?;
                store.get(dragon_id)
            })?;
            let resource_id = entity.data().entity_resource_id();
            (resource_id, resource_id == "minecraft:breeze")
        }
        ProjectileTarget::Player { .. } => ("minecraft:player", false),
    };
    if breeze
        && matches!(
            source.data(),
            EntityData::WindCharge(_) | EntityData::BreezeWindCharge(_)
        )
    {
        // `Breeze.deflection` (`Breeze.java`).
        return Some(ProjectileDeflection::None);
    }
    Some(
        if !level.registry_tag_membership(
            "minecraft:entity_type",
            "minecraft:deflects_projectiles",
            resource_id,
        ) {
            ProjectileDeflection::None
        } else if breeze {
            ProjectileDeflection::BreezeReverse
        } else {
            ProjectileDeflection::Reverse
        },
    )
}

/// `ProjectileUtil.computeMargin` (`ProjectileUtil.java`).
pub(crate) fn compute_margin(source: &Entity) -> f32 {
    0.0_f32.max(0.3_f32.min((source.tick_count() - 2) as f32 / 20.0))
}

pub(super) fn exact_or_grazing_hit(
    level: &LevelCtx<'_, '_>,
    bounds: Aabb,
    from: Vec3d,
    to: Vec3d,
    margin: f64,
) -> Option<Vec3d> {
    aabb_clip(bounds, from, to).or_else(|| {
        if margin <= 0.0 {
            return None;
        }
        let graze = aabb_clip(bounds.inflate(margin, margin, margin), from, to)?;
        let centre = bounds.center();
        let towards = clip_including_border(level, graze, centre).map_or(centre, |hit| hit.0);
        aabb_clip(bounds, graze, towards)
    })
}

/// Every stored entity other than the source and every player whose box
/// intersects the search area and passes `matching`,
/// with where the segment enters its box grown by `margin`, or with
/// `exact_first` the `getManyEntityHitResult` exact-then-graze location.
#[allow(clippy::too_many_arguments)]
pub(super) fn for_each_candidate_hit(
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    source: &Entity,
    from: Vec3d,
    to: Vec3d,
    matching: impl Fn(ProjectileCandidate<'_>, &EntityStore, &LevelCtx<'_, '_>) -> bool,
    exact_first: bool,
    mut visit: impl FnMut(ProjectileTarget, Vec3d, Vec3d, SectionKey),
) {
    let search_area = source
        .bounding_box()
        .expand_towards(source.delta_movement())
        .inflate(1.0, 1.0, 1.0);
    let margin = f64::from(compute_margin(source));
    let mut emit = |target, bounds: Aabb, position, section| {
        let location = if exact_first {
            exact_or_grazing_hit(level, bounds, from, to, margin)
        } else {
            aabb_clip(bounds.inflate(margin, margin, margin), from, to)
        };
        if let Some(location) = location {
            visit(target, location, position, section);
        }
    };
    store.for_each_box_section_id(search_area, |id| {
        if id == source.id() {
            return;
        }
        let Some(entity) = store.get(id) else {
            return;
        };
        let candidate = ProjectileCandidate::Stored(entity);
        if crate::entity::ender_dragon::state(entity).is_some()
            || !entity.bounding_box().intersects(search_area)
        {
            return;
        }
        if matching(candidate, store, level) {
            emit(
                candidate.target(),
                entity.bounding_box(),
                entity.position(),
                entity.filed_section(),
            );
        }
    });
    // Parts can extend beyond the parent's box and occupy other sections.
    for dragon in store.dragons() {
        if dragon.id() == source.id()
            || !matching(ProjectileCandidate::Stored(dragon), store, level)
        {
            continue;
        }
        let nearest = crate::entity::ender_dragon::projectile_parts(dragon)
            .filter(|(_, bounds, _)| bounds.intersects(search_area))
            .filter_map(|(id, bounds, position)| {
                let location = aabb_clip(bounds.inflate(margin, margin, margin), from, to)?;
                Some((id, bounds, position, from.distance_to_sqr(location)))
            })
            .min_by(|left, right| {
                left.3
                    .total_cmp(&right.3)
                    .then_with(|| left.0.cmp(&right.0))
            });
        if let Some((entity_id, bounding_box, position, _)) = nearest {
            emit(
                ProjectileTarget::Stored { entity_id },
                bounding_box,
                position,
                section_of_block(BlockPos {
                    x: position.x.floor() as i32,
                    y: position.y.floor() as i32,
                    z: position.z.floor() as i32,
                }),
            );
        }
    }
    for player in level.players_in_box(search_area) {
        let candidate = ProjectileCandidate::Player(&player);
        let bounding_box = player.bounding_box();
        if !bounding_box.intersects(search_area) {
            continue;
        }
        if matching(candidate, store, level) {
            emit(
                candidate.target(),
                bounding_box,
                player.position,
                section_of_block(BlockPos {
                    x: player.position.x.floor() as i32,
                    y: player.position.y.floor() as i32,
                    z: player.position.z.floor() as i32,
                }),
            );
        }
    }
}

/// `BlockGetter.clip(ClipContext)` (`BlockGetter.java`) with
/// `ClipContext.Fluid.NONE`: the fluid shape is `Shapes.empty()`, so only the
/// block shape the caller names can answer, plus the `getInteractionShape`
/// override `BlockGetter.clipWithInteractionOverride` (`:82-95`) applies to
/// every block mode.
fn clip_blocks(
    level: &LevelCtx<'_, '_>,
    from: Vec3d,
    to: Vec3d,
    shape: BlockClipShape,
) -> Option<(Vec3d, BlockPos, Direction)> {
    let mut scratch = Vec::new();
    traverse_blocks(from, to, |pos| {
        let state = level.block_state(pos);
        let (hit, mut direction) = shape.clip(level, state, pos, from, to, &mut scratch)?;
        if let Some((override_hit, override_direction)) =
            clip_block_shape(&block_interaction_boxes(state), from, to, pos)
        {
            if from.distance_to_sqr(override_hit) < from.distance_to_sqr(hit) {
                direction = override_direction;
            }
        }
        Some((hit, pos, direction))
    })
}

// ClipContext.java: projectile block clips use COLLIDER.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockClipShape {
    /// `getCollisionShape`, the mode `ProjectileUtil` and
    /// `LivingEntity.hasLineOfSight` name (`LivingEntity.java`).
    Collider,
}

impl BlockClipShape {
    fn clip(
        self,
        level: &LevelCtx<'_, '_>,
        state: i32,
        pos: BlockPos,
        from: Vec3d,
        to: Vec3d,
        scratch: &mut Vec<Aabb>,
    ) -> Option<(Vec3d, Direction)> {
        match self {
            Self::Collider => {
                if let Some(boxes) =
                    bbb_server_types::geometry::static_block_collision_box_iter(state)
                {
                    clip_block_shape_iter(boxes, from, to, pos)
                } else {
                    scratch.clear();
                    level.push_block_collision_boxes(pos, scratch);
                    clip_block_shape(scratch, from, to, pos)
                }
            }
        }
    }
}

/// `level.clip(new ClipContext(from, to, COLLIDER, NONE, source))`: where the
/// segment first enters a block's collision shape.
pub(crate) fn clip_collider(
    level: &LevelCtx<'_, '_>,
    from: Vec3d,
    to: Vec3d,
) -> Option<(Vec3d, BlockPos, Direction)> {
    clip_blocks(level, from, to, BlockClipShape::Collider)
}

/// `CollisionGetter.clipIncludingBorder` (`CollisionGetter.java:113-123`): a
/// segment that starts inside the world border and would end outside it hits
/// the border instead; the flag marks that hit.
pub(crate) fn clip_including_border(
    level: &LevelCtx<'_, '_>,
    from: Vec3d,
    to: Vec3d,
) -> Option<(Vec3d, BlockPos, Direction, bool)> {
    let hit = clip_collider(level, from, to);
    let unbordered = hit.map(|(location, pos, direction)| (location, pos, direction, false));
    let Some(border) = level.world_border() else {
        return unbordered;
    };
    let time = level.game_time();
    let (min_x, max_x) = (border.min_x_at(time), border.max_x_at(time));
    let (min_z, max_z) = (border.min_z_at(time), border.max_z_at(time));
    let within = |p: Vec3d| p.x >= min_x && p.x < max_x && p.z >= min_z && p.z < max_z;
    let location = hit.map_or(to, |(location, ..)| location);
    if !within(from) || within(location) {
        return unbordered;
    }
    let direction = approximate_nearest(location.plus(from.scale(-1.0)));
    // `WorldBorder.clampVec3ToBound` (`WorldBorder.java`).
    let margin = f64::from(1.0E-5_f32);
    let clamp_axis = |value: f64, min: f64, max: f64| {
        if !value.is_finite() || !min.is_finite() || !max.is_finite() {
            return min;
        }
        let upper = max - margin;
        if upper < min {
            min
        } else {
            value.clamp(min, upper)
        }
    };
    let clamped = Vec3d {
        x: clamp_axis(location.x, min_x, max_x),
        y: location.y,
        z: clamp_axis(location.z, min_z, max_z),
    };
    let pos = BlockPos {
        x: clamped.x.floor() as i32,
        y: clamped.y.floor() as i32,
        z: clamped.z.floor() as i32,
    };
    Some((clamped, pos, direction, true))
}

/// `Projectile.hitTargetOrDeflectSelf` (`Projectile.java:256-262`): arrows,
/// tridents and fishing hooks reverse off the world border at a fifth of the
/// reversed speed.
pub(crate) fn bounce_off_world_border(entity: &mut Entity, store: &mut EntityStore) {
    let rotation = 170.0 + store.random().next_float() * 20.0;
    entity.set_delta_movement(entity.delta_movement().scale(-0.5).scale(0.2));
    entity.set_rot(entity.y_rot() + rotation, entity.x_rot());
    entity.set_needs_sync(true);
}

pub(crate) fn collision_clip_misses(level: &LevelCtx<'_, '_>, from: Vec3d, to: Vec3d) -> bool {
    clip_blocks(level, from, to, BlockClipShape::Collider).is_none()
}

/// `BlockGetter.clip` (`BlockGetter.java`) with `ClipContext.Block.COLLIDER`
/// and `ClipContext.Fluid.ANY` (`ClipContext.java`), whose fluid shape is
/// `FlowingFluid.getShape` (`FlowingFluid.java`) read without its
/// per-state height cache.
pub(crate) fn collision_and_fluid_clip_misses(
    level: &LevelCtx<'_, '_>,
    from: Vec3d,
    to: Vec3d,
) -> bool {
    use bbb_server_types::fluid::block_state_fluid_state;
    let mut scratch = Vec::new();
    traverse_blocks(from, to, |pos| {
        let state = level.block_state(pos);
        if BlockClipShape::Collider
            .clip(level, state, pos, from, to, &mut scratch)
            .is_some()
        {
            return Some(());
        }
        let fluid = block_state_fluid_state(state);
        if fluid.is_empty() {
            return None;
        }
        let above = block_state_fluid_state(level.block_state(BlockPos {
            y: pos.y + 1,
            ..pos
        }));
        let height = if above.family().is_some() && above.family() == fluid.family() {
            1.0
        } else {
            f64::from(fluid.own_height())
        };
        let shape = Aabb {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 1.0,
            max_y: height,
            max_z: 1.0,
        };
        clip_block_shape(&[shape], from, to, pos).map(|_| ())
    })
    .is_none()
}

/// The four Vanilla 26.1 `getInteractionShape` overrides
/// (`ScaffoldingBlock.java`, `HopperBlock.java`,
/// `ComposterBlock.java`, `AbstractCauldronBlock.java`).
fn block_interaction_boxes(block_state_id: i32) -> Vec<Aabb> {
    let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) else {
        return Vec::new();
    };
    let full = Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    match block_type_id {
        block_types::VANILLA_BLOCK_SCAFFOLDING_ID | block_types::VANILLA_BLOCK_COMPOSTER_ID => {
            vec![full]
        }
        block_types::VANILLA_BLOCK_CAULDRON_ID
        | block_types::VANILLA_BLOCK_WATER_CAULDRON_ID
        | block_types::VANILLA_BLOCK_LAVA_CAULDRON_ID
        | block_types::VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID => vec![Aabb {
            min_x: 0.125,
            min_y: 0.25,
            min_z: 0.125,
            max_x: 0.875,
            max_y: 1.0,
            max_z: 0.875,
        }],
        block_types::VANILLA_BLOCK_HOPPER_ID => {
            let mut boxes = vec![Aabb {
                min_x: 0.125,
                min_y: 0.6875,
                min_z: 0.125,
                max_x: 0.875,
                max_y: 1.0,
                max_z: 0.875,
            }];
            let spout = match vanilla_block_state_property(block_state_id, "facing") {
                Some("north") => Some(Aabb {
                    min_x: 0.375,
                    min_y: 0.5,
                    min_z: 0.0,
                    max_x: 0.625,
                    max_y: 0.625,
                    max_z: 0.25,
                }),
                Some("south") => Some(Aabb {
                    min_x: 0.375,
                    min_y: 0.5,
                    min_z: 0.75,
                    max_x: 0.625,
                    max_y: 0.625,
                    max_z: 1.0,
                }),
                Some("west") => Some(Aabb {
                    min_x: 0.0,
                    min_y: 0.5,
                    min_z: 0.375,
                    max_x: 0.25,
                    max_y: 0.625,
                    max_z: 0.625,
                }),
                Some("east") => Some(Aabb {
                    min_x: 0.75,
                    min_y: 0.5,
                    min_z: 0.375,
                    max_x: 1.0,
                    max_y: 0.625,
                    max_z: 0.625,
                }),
                Some("down") | None | Some(_) => None,
            };
            boxes.extend(spout);
            boxes
        }
        _ => Vec::new(),
    }
}

/// `BlockGetter.traverseBlocks` (`BlockGetter.java`) with the miss
/// factory answering `None`.
fn traverse_blocks<T>(
    from: Vec3d,
    to: Vec3d,
    mut consumer: impl FnMut(BlockPos) -> Option<T>,
) -> Option<T> {
    if from == to {
        return None;
    }
    let to_x = mth_lerp(-1.0e-7, to.x, from.x);
    let to_y = mth_lerp(-1.0e-7, to.y, from.y);
    let to_z = mth_lerp(-1.0e-7, to.z, from.z);
    let from_x = mth_lerp(-1.0e-7, from.x, to.x);
    let from_y = mth_lerp(-1.0e-7, from.y, to.y);
    let from_z = mth_lerp(-1.0e-7, from.z, to.z);
    let mut current_x = from_x.floor() as i32;
    let mut current_y = from_y.floor() as i32;
    let mut current_z = from_z.floor() as i32;
    if let Some(first) = consumer(BlockPos {
        x: current_x,
        y: current_y,
        z: current_z,
    }) {
        return Some(first);
    }
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let dz = to_z - from_z;
    let sign_x = mth_sign(dx);
    let sign_y = mth_sign(dy);
    let sign_z = mth_sign(dz);
    let t_delta_x = if sign_x == 0 {
        f64::MAX
    } else {
        f64::from(sign_x) / dx
    };
    let t_delta_y = if sign_y == 0 {
        f64::MAX
    } else {
        f64::from(sign_y) / dy
    };
    let t_delta_z = if sign_z == 0 {
        f64::MAX
    } else {
        f64::from(sign_z) / dz
    };
    let mut t_x = t_delta_x
        * if sign_x > 0 {
            1.0 - mth_frac(from_x)
        } else {
            mth_frac(from_x)
        };
    let mut t_y = t_delta_y
        * if sign_y > 0 {
            1.0 - mth_frac(from_y)
        } else {
            mth_frac(from_y)
        };
    let mut t_z = t_delta_z
        * if sign_z > 0 {
            1.0 - mth_frac(from_z)
        } else {
            mth_frac(from_z)
        };
    while t_x <= 1.0 || t_y <= 1.0 || t_z <= 1.0 {
        if t_x < t_y {
            if t_x < t_z {
                current_x += sign_x;
                t_x += t_delta_x;
            } else {
                current_z += sign_z;
                t_z += t_delta_z;
            }
        } else if t_y < t_z {
            current_y += sign_y;
            t_y += t_delta_y;
        } else {
            current_z += sign_z;
            t_z += t_delta_z;
        }
        if let Some(result) = consumer(BlockPos {
            x: current_x,
            y: current_y,
            z: current_z,
        }) {
            return Some(result);
        }
    }
    None
}

/// `VoxelShape.clip(from, to, pos)` (`VoxelShape.java`) for a shape
/// given as the boxes `toAabbs` reports, in block-local coordinates.
///
/// `isFullWide` over `findIndex` asks whether the cell holding the test point
/// is filled, and the cells are the half-open intervals between the shape's
/// coordinate lists, so for a union of boxes it is the half-open
/// `AABB.contains` test against each box.
fn clip_block_shape(
    boxes: &[Aabb],
    from: Vec3d,
    to: Vec3d,
    pos: BlockPos,
) -> Option<(Vec3d, Direction)> {
    clip_block_shape_iter(boxes.iter().copied(), from, to, pos)
}

fn clip_block_shape_iter<I>(
    boxes: I,
    from: Vec3d,
    to: Vec3d,
    pos: BlockPos,
) -> Option<(Vec3d, Direction)>
where
    I: ExactSizeIterator<Item = Aabb>,
{
    if boxes.len() == 0 {
        return None;
    }
    let diff = Vec3d {
        x: to.x - from.x,
        y: to.y - from.y,
        z: to.z - from.z,
    };
    if diff.length_sqr() < 1.0e-7 {
        return None;
    }
    let test_point = from.plus(diff.scale(0.001));
    let local = Vec3d {
        x: test_point.x - f64::from(pos.x),
        y: test_point.y - f64::from(pos.y),
        z: test_point.z - f64::from(pos.z),
    };
    // `AABB.clip(Iterable<AABB>, from, to, pos)` (`AABB.java`).
    let mut scale = 1.0;
    let mut direction = None;
    let offset = Vec3d {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
        z: f64::from(pos.z),
    };
    for shape in boxes {
        if aabb_contains(shape, local) {
            return Some((test_point, opposite(approximate_nearest(diff))));
        }
        let moved = shape.moved(offset);
        direction = get_direction(moved, from, &mut scale, direction, diff.x, diff.y, diff.z);
    }
    Some((from.plus(diff.scale(scale)), direction?))
}

fn approximate_nearest(vector: Vec3d) -> Direction {
    let x = vector.x.abs();
    let y = vector.y.abs();
    let z = vector.z.abs();
    if y >= x && y >= z {
        if vector.y >= 0.0 {
            Direction::Up
        } else {
            Direction::Down
        }
    } else if z >= x {
        if vector.z >= 0.0 {
            Direction::South
        } else {
            Direction::North
        }
    } else if vector.x >= 0.0 {
        Direction::East
    } else {
        Direction::West
    }
}

const fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

/// `AABB.contains(double, double, double)` (`AABB.java`).
pub(super) fn aabb_contains(aabb: Aabb, point: Vec3d) -> bool {
    point.x >= aabb.min_x
        && point.x < aabb.max_x
        && point.y >= aabb.min_y
        && point.y < aabb.max_y
        && point.z >= aabb.min_z
        && point.z < aabb.max_z
}

/// `AABB.clip(Vec3, Vec3)` (`AABB.java`).
pub fn aabb_clip(aabb: Aabb, from: Vec3d, to: Vec3d) -> Option<Vec3d> {
    let mut scale = 1.0;
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let dz = to.z - from.z;
    get_direction(aabb, from, &mut scale, None, dx, dy, dz)?;
    Some(Vec3d {
        x: from.x + scale * dx,
        y: from.y + scale * dy,
        z: from.z + scale * dz,
    })
}

/// `AABB.getDirection` (`AABB.java`).
fn get_direction(
    aabb: Aabb,
    from: Vec3d,
    scale: &mut f64,
    mut direction: Option<Direction>,
    dx: f64,
    dy: f64,
    dz: f64,
) -> Option<Direction> {
    let Aabb {
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    } = aabb;
    if dx > 1.0e-7 {
        direction = clip_point(
            scale,
            direction,
            [dx, dy, dz],
            min_x,
            [min_y, max_y, min_z, max_z],
            Direction::West,
            [from.x, from.y, from.z],
        );
    } else if dx < -1.0e-7 {
        direction = clip_point(
            scale,
            direction,
            [dx, dy, dz],
            max_x,
            [min_y, max_y, min_z, max_z],
            Direction::East,
            [from.x, from.y, from.z],
        );
    }
    if dy > 1.0e-7 {
        direction = clip_point(
            scale,
            direction,
            [dy, dz, dx],
            min_y,
            [min_z, max_z, min_x, max_x],
            Direction::Down,
            [from.y, from.z, from.x],
        );
    } else if dy < -1.0e-7 {
        direction = clip_point(
            scale,
            direction,
            [dy, dz, dx],
            max_y,
            [min_z, max_z, min_x, max_x],
            Direction::Up,
            [from.y, from.z, from.x],
        );
    }
    if dz > 1.0e-7 {
        direction = clip_point(
            scale,
            direction,
            [dz, dx, dy],
            min_z,
            [min_x, max_x, min_y, max_y],
            Direction::North,
            [from.z, from.x, from.y],
        );
    } else if dz < -1.0e-7 {
        direction = clip_point(
            scale,
            direction,
            [dz, dx, dy],
            max_z,
            [min_x, max_x, min_y, max_y],
            Direction::South,
            [from.z, from.x, from.y],
        );
    }
    direction
}

/// `AABB.clipPoint` (`AABB.java`): `[da, db, dc]`, the face plane,
/// `[minB, maxB, minC, maxC]`, the face and `[fromA, fromB, fromC]`.
fn clip_point(
    scale: &mut f64,
    direction: Option<Direction>,
    [da, db, dc]: [f64; 3],
    point: f64,
    [min_b, max_b, min_c, max_c]: [f64; 4],
    new_direction: Direction,
    [from_a, from_b, from_c]: [f64; 3],
) -> Option<Direction> {
    let s = (point - from_a) / da;
    let pb = from_b + s * db;
    let pc = from_c + s * dc;
    if 0.0 < s
        && s < *scale
        && min_b - 1.0e-7 < pb
        && pb < max_b + 1.0e-7
        && min_c - 1.0e-7 < pc
        && pc < max_c + 1.0e-7
    {
        *scale = s;
        Some(new_direction)
    } else {
        direction
    }
}

/// `Mth.lerp(double, double, double)` (`Mth.java`).
fn mth_lerp(alpha: f64, p0: f64, p1: f64) -> f64 {
    p0 + alpha * (p1 - p0)
}

/// `Mth.sign(double)` (`Mth.java`).
fn mth_sign(number: f64) -> i32 {
    if number == 0.0 {
        0
    } else if number > 0.0 {
        1
    } else {
        -1
    }
}

/// `Mth.frac(double)` (`Mth.java`) over `lfloor` (`:69-71`).
fn mth_frac(number: f64) -> f64 {
    number - number.floor()
}

/// `Mth.atan2(double, double)` (`Mth.java`).
pub use crate::entity::projectile_shoot::mth_atan2;
