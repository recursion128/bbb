//! Host-free block facts and shape queries.

use crate::directions::opposite;
use crate::fluid::block_type_ids_in_tag;
use bbb_protocol::block_state_types::vanilla_block_state_render_facts;
use bbb_protocol::block_state_types::vanilla_falling_dust_static_facts;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_property,
    vanilla_block_state_static_collision_shape, vanilla_block_state_support_facts,
    VanillaBlockCollisionBox, VanillaBlockCollisionShape, VanillaBlockStateSupportFacts,
    VanillaBlockSupportDirection, VanillaBlockSupportType as SupportType,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_BAMBOO_ID,
    VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_POWDER_SNOW_ID,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, Direction};
use std::collections::HashSet;
use std::sync::OnceLock;

/// `BlockState`'s generated `leaves_block` class bit.
#[must_use]
pub fn is_leaves(block_state_id: i32) -> bool {
    vanilla_block_state_render_facts(block_state_id).is_some_and(|facts| facts.leaves_block())
}

/// `BlockState.getBlock()` as a `BuiltInRegistries.BLOCK` id.
pub fn block_type_of(block_state_id: i32) -> Option<i32> {
    vanilla_block_state_block_type_id(block_state_id)
}

/// `ContentRegistry`-aware variant of [`block_type_of`].
pub fn block_type_of_with_registry(block_state_id: i32, registry: &ContentRegistry) -> Option<i32> {
    if let Some(id) = vanilla_block_state_block_type_id(block_state_id) {
        return Some(id);
    }
    registry
        .block_by_state_id(block_state_id)
        .map(|(spec, _)| spec.block_id)
}

/// `BlockState.getValue(IntegerProperty)`; `None` when the state has no such
/// property, which is the branch `getValueOrElse` answers with its default
/// (`StateHolder.java:100-102`).
pub fn int_property(block_state_id: i32, property: &str) -> Option<i32> {
    vanilla_block_state_property(block_state_id, property)?
        .parse()
        .ok()
}

/// `BlockState.getValue(BooleanProperty)`.
pub fn bool_property(block_state_id: i32, property: &str) -> Option<bool> {
    match vanilla_block_state_property(block_state_id, property)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `BlockState.isFaceSturdy(level, pos, direction, supportType)`
/// (`BlockBehaviour.java:883-885`), from the generated per-state support
/// facts.
///
/// Vanilla answers this from `BlockStateBase.Cache`, which is built once per
/// state by running `SupportType.isSupporting` (`SupportType.java:11-33`)
/// against `EmptyBlockGetter.INSTANCE` and `BlockPos.ZERO`
/// (`BlockBehaviour.java:932-938`) — no level and no position. The checked-in
/// probe calls `BlockState.isFaceSturdy` on exactly those arguments, so for
/// every state that has a cache the generated bit *is* vanilla's answer,
/// whatever level and position the caller holds.
///
/// A state whose block declares `Properties.dynamicShape()` has no cache
/// (`BlockBehaviour.java:504-509`) and vanilla evaluates it live against the
/// real level; the probe's `EmptyBlockGetter` answer is not that, so this
/// answers `None` for the five blocks and one family that declare it —
/// `moving_piston` (`Blocks.java:895-905`), `bamboo` (`:5214-5227`),
/// `scaffolding` (`:5308-5318`), `powder_snow` (`:6051-6061`),
/// `pointed_dripstone` (`:6509-6522`) and the shulker boxes
/// (`:6973-6981`) — rather than a shape read out of an empty world.
pub fn face_sturdy(
    block_state_id: i32,
    direction: Direction,
    support_type: SupportType,
) -> Option<bool> {
    face_sturdy_with_registry(
        block_state_id,
        direction,
        support_type,
        ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`face_sturdy`].
pub fn face_sturdy_with_registry(
    block_state_id: i32,
    direction: Direction,
    support_type: SupportType,
    registry: &ContentRegistry,
) -> Option<bool> {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if facts.dynamic_shape() {
            return None;
        }
        return Some(facts.face_sturdy(support_direction(direction), support_type));
    }
    registry
        .block_by_state_id(block_state_id)
        .map(|(spec, _)| spec.full_cube_collision)
}

/// `Block.isFaceFull(state.getCollisionShape(level, pos), direction)`
/// (`Block.java:325-327`), off the same cached facts [`face_sturdy`] reads.
pub fn collision_face_full(block_state_id: i32, direction: Direction) -> Option<bool> {
    collision_face_full_with_registry(block_state_id, direction, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`collision_face_full`].
pub fn collision_face_full_with_registry(
    block_state_id: i32,
    direction: Direction,
    registry: &ContentRegistry,
) -> Option<bool> {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if facts.dynamic_shape() {
            return None;
        }
        return Some(facts.collision_face_full(support_direction(direction)));
    }
    registry
        .block_by_state_id(block_state_id)
        .map(|(spec, _)| spec.full_cube_collision)
}

/// Whether `state.getCollisionShape(level, pos).getFaceShape(direction)` is
/// non-empty (`SeaPickleBlock.java:65`).
pub fn collision_face_nonempty(block_state_id: i32, direction: Direction) -> Option<bool> {
    collision_face_nonempty_with_registry(block_state_id, direction, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`collision_face_nonempty`].
pub fn collision_face_nonempty_with_registry(
    block_state_id: i32,
    direction: Direction,
    registry: &ContentRegistry,
) -> Option<bool> {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if facts.dynamic_shape() {
            return None;
        }
        return Some(facts.collision_face_nonempty(support_direction(direction)));
    }
    registry
        .block_by_state_id(block_state_id)
        .map(|(spec, _)| spec.full_cube_collision)
}

/// `MultifaceBlock.canAttachTo` (`MultifaceBlock.java:256-260`).
pub fn multiface_can_attach_to(
    block_state_id: i32,
    direction_towards_neighbour: Direction,
) -> Option<bool> {
    multiface_can_attach_to_with_registry(
        block_state_id,
        direction_towards_neighbour,
        ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`multiface_can_attach_to`].
pub fn multiface_can_attach_to_with_registry(
    block_state_id: i32,
    direction_towards_neighbour: Direction,
    registry: &ContentRegistry,
) -> Option<bool> {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if facts.dynamic_shape() {
            return None;
        }
        let face = support_direction(opposite(direction_towards_neighbour));
        return Some(facts.face_sturdy(face, SupportType::Full) || facts.collision_face_full(face));
    }
    registry
        .block_by_state_id(block_state_id)
        .map(|(spec, _)| spec.full_cube_collision)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyContextShape {
    Empty,
    Full,
    Sampled {
        facts: VanillaBlockStateSupportFacts,
        vertical_center: Option<bool>,
        collision_empty: bool,
    },
}

impl EmptyContextShape {
    fn face_nonempty(self, direction: Direction) -> bool {
        match self {
            Self::Empty => false,
            Self::Full => true,
            Self::Sampled { facts, .. } => {
                facts.collision_face_nonempty(support_direction(direction))
            }
        }
    }

    fn face_full(self, direction: Direction) -> bool {
        match self {
            Self::Empty => false,
            Self::Full => true,
            Self::Sampled { facts, .. } => facts.collision_face_full(support_direction(direction)),
        }
    }

    fn face_sturdy(self, direction: Direction, support_type: SupportType) -> bool {
        match self {
            Self::Empty => false,
            Self::Full => true,
            Self::Sampled {
                facts,
                vertical_center,
                ..
            } => {
                if support_type == SupportType::Center
                    && matches!(direction, Direction::Down | Direction::Up)
                {
                    if let Some(answer) = vertical_center {
                        return answer;
                    }
                }
                facts.face_sturdy(support_direction(direction), support_type)
            }
        }
    }
}

/// A dynamic-shape state's empty-context support and collision shapes; the
/// transient moving-piston shape and an opening shulker lid are not modelled.
pub fn empty_context_shape(
    block_state_id: i32,
    pos: BlockPos,
    facts: VanillaBlockStateSupportFacts,
) -> EmptyContextShape {
    match block_type_of(block_state_id) {
        // `BambooStalkBlock.java:73-75`; the XZ offset is
        // `BlockBehaviour.java:1228-1238` over `Mth.java:314-318`.
        Some(VANILLA_BLOCK_BAMBOO_ID) => EmptyContextShape::Sampled {
            facts,
            vertical_center: Some(bamboo_center_supported(pos)),
            collision_empty: false,
        },
        // `PowderSnowBlock.java:116-131`.
        Some(VANILLA_BLOCK_POWDER_SNOW_ID) => EmptyContextShape::Sampled {
            facts,
            vertical_center: None,
            collision_empty: true,
        },
        // `MovingPistonBlock.java:106-109` is empty without its block entity.
        Some(VANILLA_BLOCK_MOVING_PISTON_ID) => EmptyContextShape::Empty,
        // `ShulkerBoxBlock.java:140-155` uses a full cube without a BE.
        _ if is_shulker_box(block_state_id) => EmptyContextShape::Full,
        // Scaffolding and pointed dripstone: the generated empty-context
        // sample is exact (`ScaffoldingBlock.java:137-146`,
        // `PointedDripstoneBlock.java:260-268`).
        _ => EmptyContextShape::Sampled {
            facts,
            vertical_center: None,
            collision_empty: false,
        },
    }
}

fn bamboo_center_supported(pos: BlockPos) -> bool {
    let seed =
        i64::from(pos.x.wrapping_mul(3_129_871)) ^ i64::from(pos.z).wrapping_mul(116_129_781);
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16;
    matches!(seed & 15, 7 | 8) && matches!((seed >> 8) & 15, 7 | 8)
}

/// Live equivalent of [`face_sturdy`], preserving its static fast path.
pub fn face_sturdy_at(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    support_type: SupportType,
) -> bool {
    face_sturdy_at_with_registry(
        block_state_id,
        pos,
        direction,
        support_type,
        ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`face_sturdy_at`].
pub fn face_sturdy_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    support_type: SupportType,
    registry: &ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if !facts.dynamic_shape() {
            return facts.face_sturdy(support_direction(direction), support_type);
        }
        let support = empty_context_shape(block_state_id, pos, facts);
        return support.face_sturdy(direction, support_type);
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}

/// Live `Block.isFaceFull(state.getCollisionShape(...), direction)`.
pub fn collision_face_full_at(block_state_id: i32, pos: BlockPos, direction: Direction) -> bool {
    collision_face_full_at_with_registry(block_state_id, pos, direction, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`collision_face_full_at`].
pub fn collision_face_full_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    registry: &ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if !facts.dynamic_shape() {
            return facts.collision_face_full(support_direction(direction));
        }
        let collision = empty_context_shape(block_state_id, pos, facts);
        return collision.face_full(direction);
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}

/// Live empty-context collision-face non-emptiness.
pub fn collision_face_nonempty_at(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
) -> bool {
    collision_face_nonempty_at_with_registry(
        block_state_id,
        pos,
        direction,
        ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`collision_face_nonempty_at`].
pub fn collision_face_nonempty_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    registry: &ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if !facts.dynamic_shape() {
            return facts.collision_face_nonempty(support_direction(direction));
        }
        let collision = empty_context_shape(block_state_id, pos, facts);
        return collision.face_nonempty(direction);
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}

/// Live empty-context equivalent of `state.getCollisionShape(...).isEmpty()`.
pub fn collision_shape_empty_at(block_state_id: i32, pos: BlockPos) -> bool {
    collision_shape_empty_at_with_registry(block_state_id, pos, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`collision_shape_empty_at`].
pub fn collision_shape_empty_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    registry: &ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if !facts.dynamic_shape() {
            return vanilla_block_state_static_collision_shape(block_state_id)
                .is_none_or(|shape| shape.boxes().len() == 0);
        }
        let collision = empty_context_shape(block_state_id, pos, facts);
        return match collision {
            EmptyContextShape::Empty => true,
            EmptyContextShape::Full => false,
            EmptyContextShape::Sampled {
                collision_empty, ..
            } => collision_empty,
        };
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| !spec.full_cube_collision)
}

/// `state.isCollisionShapeFullBlock(level, pos)`
/// (`BlockBehaviour.java:341-343`) for a state whose collision shape ignores
/// the level; `None` says the shape is dynamic and only
/// `collision_shape_full_block_at` can answer it.
#[must_use]
pub fn collision_shape_full_block(block_state_id: i32) -> Option<bool> {
    collision_shape_full_block_with_registry(block_state_id, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`collision_shape_full_block`].
#[must_use]
pub fn collision_shape_full_block_with_registry(
    block_state_id: i32,
    registry: &ContentRegistry,
) -> Option<bool> {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        if facts.dynamic_shape() {
            return None;
        }
        return Some(static_collision_shape_is_full_block(
            vanilla_block_state_static_collision_shape(block_state_id)?,
        ));
    }
    registry
        .block_by_state_id(block_state_id)
        .map(|(spec, _)| spec.full_cube_collision)
}

pub fn static_collision_shape_is_full_block(shape: VanillaBlockCollisionShape) -> bool {
    let mut iter = shape.boxes();
    let Some(first) = iter.next() else {
        return false;
    };
    // Fast path for the overwhelmingly common single-box shape (unit cube or slab):
    // avoids all Vec allocations.
    let Some(second) = iter.next() else {
        return first.min_x == 0.0
            && first.min_y == 0.0
            && first.min_z == 0.0
            && first.max_x == 1.0
            && first.max_y == 1.0
            && first.max_z == 1.0;
    };
    let mut boxes = vec![first, second];
    boxes.extend(iter);
    if boxes.iter().any(|bounds| {
        bounds.min_x < 0.0
            || bounds.min_y < 0.0
            || bounds.min_z < 0.0
            || bounds.max_x > 1.0
            || bounds.max_y > 1.0
            || bounds.max_z > 1.0
    }) {
        return false;
    }
    let boundaries = |axis: fn(&VanillaBlockCollisionBox) -> (f64, f64)| {
        let mut values = vec![0.0, 1.0];
        for bounds in &boxes {
            let (min, max) = axis(bounds);
            values.extend([min, max]);
        }
        values.sort_by(f64::total_cmp);
        values.dedup();
        values
    };
    let xs = boundaries(|bounds| (bounds.min_x, bounds.max_x));
    let ys = boundaries(|bounds| (bounds.min_y, bounds.max_y));
    let zs = boundaries(|bounds| (bounds.min_z, bounds.max_z));
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            for z in zs.windows(2) {
                let point = (
                    (x[0] + x[1]) * 0.5,
                    (y[0] + y[1]) * 0.5,
                    (z[0] + z[1]) * 0.5,
                );
                if !boxes.iter().any(|bounds| {
                    point.0 >= bounds.min_x
                        && point.0 <= bounds.max_x
                        && point.1 >= bounds.min_y
                        && point.1 <= bounds.max_y
                        && point.2 >= bounds.min_z
                        && point.2 <= bounds.max_z
                }) {
                    return false;
                }
            }
        }
    }
    true
}

/// Live equivalent of [`multiface_can_attach_to`].
pub fn multiface_can_attach_to_at(
    block_state_id: i32,
    pos: BlockPos,
    direction_towards_neighbour: Direction,
) -> bool {
    multiface_can_attach_to_at_with_registry(
        block_state_id,
        pos,
        direction_towards_neighbour,
        ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`multiface_can_attach_to_at`].
pub fn multiface_can_attach_to_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    direction_towards_neighbour: Direction,
    registry: &ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        let face = opposite(direction_towards_neighbour);
        if !facts.dynamic_shape() {
            let face = support_direction(face);
            return facts.face_sturdy(face, SupportType::Full) || facts.collision_face_full(face);
        }
        let shape = empty_context_shape(block_state_id, pos, facts);
        return shape.face_sturdy(face, SupportType::Full) || shape.face_full(face);
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}

/// `Direction` as the generated support facts index it; the two enums are the
/// same six values in the same `Direction.values()` order
/// (`Direction.java:33-38`).
pub const fn support_direction(direction: Direction) -> VanillaBlockSupportDirection {
    match direction {
        Direction::Down => VanillaBlockSupportDirection::Down,
        Direction::Up => VanillaBlockSupportDirection::Up,
        Direction::North => VanillaBlockSupportDirection::North,
        Direction::South => VanillaBlockSupportDirection::South,
        Direction::West => VanillaBlockSupportDirection::West,
        Direction::East => VanillaBlockSupportDirection::East,
    }
}

/// `Direction.CODEC`'s decode side (`Direction.java:33-38`): the serialized
/// name back to the constant.
pub fn direction_from_serialized_name(name: &str) -> Option<Direction> {
    match name {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

/// `Direction.getSerializedName()`, the other half of [`direction_property`]
/// (`Direction.java:33-38`).
pub const fn direction_serialized_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

/// `ShulkerBoxBlock.useWithoutItem` (`ShulkerBoxBlock.java`) is unique
/// to that class, so the generated tag names all seventeen colour variants.
#[must_use]
pub fn is_shulker_box(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::ShulkerBoxBlock)
}

/// Alias for [`is_shulker_box`].
#[must_use]
pub fn is_shulker_box_block(state_id: i32) -> bool {
    is_shulker_box(state_id)
}

/// `ShulkerBoxBlock.getBlockSupportShape` (`ShulkerBoxBlock.java:47, 145-149`)
/// when open (`!shulker.isClosed()`): `SHAPES_OPEN_SUPPORT` is a `16x16x1` plate
/// on `FACING.getOpposite()`, supporting `Full`, `Center`, and `Rigid` only on
/// that base face.
#[must_use]
pub fn shulker_box_open_face_sturdy(block_state_id: i32, direction: Direction) -> bool {
    let facing = vanilla_block_state_property(block_state_id, "facing")
        .and_then(direction_from_serialized_name)
        .unwrap_or(Direction::Up);
    direction == opposite(facing)
}

/// `BlockState.hasBlockEntity()` (`BlockBehaviour.java:847-849`), which is
/// `getBlock() instanceof EntityBlock`, together with *which* type that block
/// registered for.
///
/// Both answers come out of the one generated column: a block carries a
/// `block_entity_type_id` exactly when some `BlockEntityType.register` call
/// named it, and `BlockEntityType.register` is the only way a 26.1 block class
/// that implements `EntityBlock` reaches the registry. The generator asserts
/// that equivalence against the resolved `EntityBlock` interface membership of
/// every registered block class, so this is not an assumption made here.
#[must_use]
pub fn block_entity_type_for_state(block_state_id: i32) -> Option<i32> {
    vanilla_block_type(vanilla_block_state_block_type_id(block_state_id)?)?.block_entity_type_id
}

/// `BlockState.hasBlockEntity()` (`BlockBehaviour.java:847-849`).
#[must_use]
pub fn block_state_has_block_entity(block_state_id: i32) -> bool {
    block_entity_type_for_state(block_state_id).is_some()
}

const DISTANCE: &str = "distance";
/// `BlockTags.PREVENTS_NEARBY_LEAF_DECAY` (`BlockTags.java:47`), whose data
/// file is the single nested reference `#minecraft:logs`.
pub fn prevents_nearby_leaf_decay() -> &'static HashSet<i32> {
    static IDS: OnceLock<HashSet<i32>> = OnceLock::new();
    IDS.get_or_init(|| block_type_ids_in_tag("minecraft:prevents_nearby_leaf_decay"))
}

/// A scaffolding distance must not make it a source for nearby leaves.
#[must_use]
pub fn distance(block_state_id: i32) -> Option<i32> {
    if !is_leaves(block_state_id) {
        return None;
    }
    int_property(block_state_id, DISTANCE)
}

/// `LeavesBlock.getOptionalDistanceAt` (`LeavesBlock.java:130-136`).
///
/// A log answers 0 — it is the source every distance is measured from — and
/// any other block that does not carry `LeavesBlock.DISTANCE` answers `None`,
/// which the caller turns into `DECAY_DISTANCE`.
///
/// Note the tag test comes **first**: a block that is both in
/// `#minecraft:prevents_nearby_leaf_decay` and carries `DISTANCE` would answer
/// 0, not its own property.
#[must_use]
pub fn optional_distance_at(block_state_id: i32) -> Option<i32> {
    if state_in_tag(block_state_id, prevents_nearby_leaf_decay()) {
        return Some(0);
    }
    distance(block_state_id)
}

/// `BlockState.is(TagKey<Block>)` against an already-resolved tag set.
pub fn state_in_tag(block_state_id: i32, tag_members: &HashSet<i32>) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| tag_members.contains(&block_type_id))
}

/// `ContentRegistry`-aware variant of [`state_in_tag`].
pub fn state_in_tag_with_registry(
    block_state_id: i32,
    tag_members: &HashSet<i32>,
    registry: &ContentRegistry,
) -> bool {
    block_type_of_with_registry(block_state_id, registry)
        .is_some_and(|block_type_id| tag_members.contains(&block_type_id))
}

/// `BlockPos.offset(dx, dy, dz)`; `BlockPos` in this repo is a bare coordinate
/// triple, and Java's `int` addition wraps.
pub fn offset(pos: BlockPos, dx: i32, dy: i32, dz: i32) -> BlockPos {
    BlockPos {
        x: pos.x.wrapping_add(dx),
        y: pos.y.wrapping_add(dy),
        z: pos.z.wrapping_add(dz),
    }
}

/// Shape-view equivalent of [`can_support_center`].
pub fn can_support_center_at(block_state_id: i32, pos: BlockPos, direction: Direction) -> bool {
    can_support_center_at_with_registry(block_state_id, pos, direction, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`can_support_center_at`].
pub fn can_support_center_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    registry: &ContentRegistry,
) -> bool {
    can_support_center_at_with_membership_and_registry(
        block_state_id,
        pos,
        direction,
        direction == Direction::Down
            && state_in_tag_with_registry(block_state_id, unstable_bottom_center(), registry),
        registry,
    )
}

pub fn can_support_center_at_with_membership(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    unstable_bottom_center: bool,
) -> bool {
    can_support_center_at_with_membership_and_registry(
        block_state_id,
        pos,
        direction,
        unstable_bottom_center,
        ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`can_support_center_at_with_membership`].
pub fn can_support_center_at_with_membership_and_registry(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    unstable_bottom_center: bool,
    registry: &ContentRegistry,
) -> bool {
    if direction == Direction::Down && unstable_bottom_center {
        return false;
    }
    face_sturdy_at_with_registry(
        block_state_id,
        pos,
        direction,
        SupportType::Center,
        registry,
    )
}

/// Live equivalent of [`can_support_rigid_block`].
pub fn can_support_rigid_block_at(block_state_id: i32, pos: BlockPos) -> bool {
    face_sturdy_at(block_state_id, pos, Direction::Up, SupportType::Rigid)
}

/// `ContentRegistry`-aware variant of [`can_support_rigid_block_at`].
pub fn can_support_rigid_block_at_with_registry(
    block_state_id: i32,
    pos: BlockPos,
    registry: &ContentRegistry,
) -> bool {
    face_sturdy_at_with_registry(
        block_state_id,
        pos,
        Direction::Up,
        SupportType::Rigid,
        registry,
    )
}

/// `Block.canSupportCenter(level, belowPos, direction)` (`Block.java:343-348`).
///
/// Takes the state that stands at `belowPos` rather than a level, because
/// `level.getBlockState(belowPos)` (`:344`) is the only read it performs. The
/// `UNSTABLE_BOTTOM_CENTER` escape only fires for `Direction.DOWN`, so a
/// caller asking about the face above a block never reaches it; it is
/// transcribed anyway so the next caller does not have to re-derive that.
pub fn can_support_center(block_state_id: i32, direction: Direction) -> Option<bool> {
    can_support_center_with_registry(block_state_id, direction, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`can_support_center`].
pub fn can_support_center_with_registry(
    block_state_id: i32,
    direction: Direction,
    registry: &ContentRegistry,
) -> Option<bool> {
    if direction == Direction::Down
        && state_in_tag_with_registry(block_state_id, unstable_bottom_center(), registry)
    {
        return Some(false);
    }
    face_sturdy_with_registry(block_state_id, direction, SupportType::Center, registry)
}

/// `Block.canSupportRigidBlock(level, below)` (`Block.java:339-341`), the
/// stricter half of the two questions a pressure plate asks its floor.
pub fn can_support_rigid_block(block_state_id: i32) -> Option<bool> {
    face_sturdy(block_state_id, Direction::Up, SupportType::Rigid)
}

/// `ContentRegistry`-aware variant of [`can_support_rigid_block`].
pub fn can_support_rigid_block_with_registry(
    block_state_id: i32,
    registry: &ContentRegistry,
) -> Option<bool> {
    face_sturdy_with_registry(block_state_id, Direction::Up, SupportType::Rigid, registry)
}

/// `BlockTags.UNSTABLE_BOTTOM_CENTER` (`BlockTags.java:125`).
fn unstable_bottom_center() -> &'static HashSet<i32> {
    static IDS: OnceLock<HashSet<i32>> = OnceLock::new();
    IDS.get_or_init(|| block_type_ids_in_tag("minecraft:unstable_bottom_center"))
}

/// Whether the state's block is a `FallingBlock`.
///
/// Derived rather than listed: `FallingBlock.getDustColor` is abstract
/// (`FallingBlock.java:77`), so a state has a falling-dust colour in the
/// generated 26.1 facts exactly when its block implements that method, which is
/// exactly when it extends `FallingBlock`. `FallingDustParticle.Provider`
/// selects on the same `instanceof FallingBlock`
/// (`net/minecraft/client/particle/FallingDustParticle.java:92`), which is
/// where the generator reads it from.
pub fn is_falling_block(block_state_id: i32) -> bool {
    vanilla_falling_dust_static_facts(block_state_id)
        .is_some_and(|facts| facts.falling_block_dust_color_rgb().is_some())
}

/// `Direction.values()` in enum-declaration order (`Direction.java:33-38`).
pub const DIRECTION_VALUES: [Direction; 6] = [
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

pub fn game_type_may_build(game_type: GameType) -> bool {
    !matches!(game_type, GameType::Adventure | GameType::Spectator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use std::collections::BTreeMap;

    fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
        vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|&(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    }

    #[test]
    fn shulker_box_resting_and_open_support_faces() {
        let up_shulker = state("minecraft:shulker_box", &[("facing", "up")]);
        let north_shulker = state("minecraft:red_shulker_box", &[("facing", "north")]);
        let stone = state("minecraft:stone", &[]);

        assert!(is_shulker_box_block(up_shulker));
        assert!(is_shulker_box_block(north_shulker));
        assert!(!is_shulker_box_block(stone));

        let pos = BlockPos { x: 0, y: 64, z: 0 };
        let facts = vanilla_block_state_support_facts(up_shulker).unwrap();
        assert_eq!(
            empty_context_shape(up_shulker, pos, facts),
            EmptyContextShape::Full
        );

        assert!(shulker_box_open_face_sturdy(up_shulker, Direction::Down));
        assert!(!shulker_box_open_face_sturdy(up_shulker, Direction::Up));
        assert!(!shulker_box_open_face_sturdy(up_shulker, Direction::North));

        assert!(shulker_box_open_face_sturdy(
            north_shulker,
            Direction::South
        ));
        assert!(!shulker_box_open_face_sturdy(
            north_shulker,
            Direction::North
        ));
        assert!(!shulker_box_open_face_sturdy(north_shulker, Direction::Up));
    }

    #[test]
    fn mod_blocks_resolve_support_solid_and_command_parsing() {
        use crate::block_state_parse::{
            block_predicate_property_matches_with_registry, read_block_state_with_registry,
            MOD_STATE_OFFSET_PROPERTY,
        };
        use crate::fluid::{
            block_state_blocks_motion_with_registry, block_state_is_solid_with_registry,
        };
        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "modtest:pillar".to_string(),
            state_count: 3,
            light_emission: 0,
            destroy_speed: 2.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "modtest:mist".to_string(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 0.0,
            full_cube_collision: false,
            dynamic_light_rgb: None,
        });
        let registry = builder.freeze_for_session().unwrap();
        let pillar = registry.block_by_resource_id("modtest:pillar").unwrap();
        let mist = registry.block_by_resource_id("modtest:mist").unwrap();

        assert_eq!(
            face_sturdy_with_registry(
                pillar.first_state_id,
                Direction::Up,
                SupportType::Full,
                &registry
            ),
            Some(true)
        );
        assert!(block_state_is_solid_with_registry(
            pillar.first_state_id,
            &registry
        ));
        assert!(block_state_blocks_motion_with_registry(
            pillar.first_state_id,
            &registry
        ));
        assert!(!block_state_is_solid_with_registry(
            mist.first_state_id,
            &registry
        ));
        assert!(collision_shape_empty_at_with_registry(
            mist.first_state_id,
            BlockPos { x: 0, y: 0, z: 0 },
            &registry
        ));

        let (_, parsed) =
            read_block_state_with_registry("modtest:pillar[bbb_state_offset=2]", 0, &registry)
                .unwrap();
        assert_eq!(parsed.state, pillar.first_state_id + 2);
        let prop_u16: Vec<u16> = MOD_STATE_OFFSET_PROPERTY.encode_utf16().collect();
        let val_u16: Vec<u16> = "2".encode_utf16().collect();
        assert!(block_predicate_property_matches_with_registry(
            parsed.state,
            &prop_u16,
            &val_u16,
            &registry
        ));
    }
}
