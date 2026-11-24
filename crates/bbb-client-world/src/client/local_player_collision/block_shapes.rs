use super::*;

impl BlockView {
    pub fn collision_shape_is_full_block(&self) -> bool {
        block_collision_shape_is_full_block(self, self.pos)
    }
}

pub(crate) fn block_collision_shape_is_full_block(block: &BlockView, pos: BlockPos) -> bool {
    if let Some(shape) = vanilla_block_state_static_collision_shape(block.block_state_id) {
        let mut boxes = shape.boxes();
        return boxes.len() == 1
            && boxes.next().map(WorldCollisionBox::from) == Some(WorldCollisionBox::FULL);
    }
    dynamic_block_collision_shape(block, pos).is_some_and(BlockCollisionShape::is_full_block)
}

pub(super) fn block_collision_shape_empty_context(
    block: &BlockView,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    // Dynamic branches in `block_collision_shape` mirror the no-entity empty
    // context: powder snow and moving pistons are empty, scaffolding is stable,
    // shulker boxes lack a live block entity, and offset shapes use `pos`.
    block_collision_shape(block, pos)
}

/// Mirrors `BlockState.entityCanStandOnFace(..., Direction.UP)` for a remote
/// entity whose collision context does not add a block-specific walkability
/// exception. Powder snow therefore fails closed; scaffolding's stable top
/// shape is the same support face exposed to an entity above it.
pub(crate) fn block_collision_shape_has_full_up_face(block: &BlockView, pos: BlockPos) -> bool {
    block_collision_shape_empty_context(block, pos)
        .is_some_and(|shape| shape.has_full_face(BlockDirection::Up))
}

/// Mirrors the cached `BlockStateBase.propagatesSkylightDown` value from the
/// generated nearest override owner and the default empty-context shape rule.
pub(crate) fn block_state_propagates_skylight_down(
    block: &BlockView,
    pos: BlockPos,
) -> Option<bool> {
    let block_name = block.block_name?;
    let block_type = vanilla_block_type_for_resource_id(block_name)?;
    let fluid_is_empty = block.fluid.is_none();
    Some(match block_type.propagates_skylight_down_override {
        Some(
            VanillaPropagatesSkylightDownOverride::BambooStalkBlock
            | VanillaPropagatesSkylightDownOverride::HangingMossBlock
            | VanillaPropagatesSkylightDownOverride::MossyCarpetBlock
            | VanillaPropagatesSkylightDownOverride::TransparentBlock
            | VanillaPropagatesSkylightDownOverride::VineBlock,
        ) => true,
        Some(
            VanillaPropagatesSkylightDownOverride::BarrierBlock
            | VanillaPropagatesSkylightDownOverride::CrossCollisionBlock
            | VanillaPropagatesSkylightDownOverride::GlowLichenBlock
            | VanillaPropagatesSkylightDownOverride::LightBlock
            | VanillaPropagatesSkylightDownOverride::VegetationBlock
            | VanillaPropagatesSkylightDownOverride::WallBlock,
        ) => fluid_is_empty,
        Some(
            VanillaPropagatesSkylightDownOverride::LiquidBlock
            | VanillaPropagatesSkylightDownOverride::PipeBlock
            | VanillaPropagatesSkylightDownOverride::ShulkerBoxBlock
            | VanillaPropagatesSkylightDownOverride::TintedGlassBlock,
        ) => false,
        None => fluid_is_empty && !block_collision_shape_is_full_block(block, pos),
    })
}

/// Vanilla `BlockState.getVisualShape` catalog used by camera zoom.
///
/// `BlockBehaviour` defaults to the collision shape. These branches are the
/// complete 26.1 override-owner set found under `world/level/block`.
pub(super) fn block_visual_shape_with_context(
    world: &WorldStore,
    block: &BlockView,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    let Some(name) = block.block_name else {
        return block_collision_shape_with_context(world, block, pos, context);
    };
    if matches!(name, "minecraft:soul_sand" | "minecraft:mud") {
        return Some(BlockCollisionShape::single(WorldCollisionBox::FULL));
    }
    if block_name_has_empty_camera_visual_shape(name) {
        return None;
    }
    if name == "minecraft:snow" {
        return snow_layer_visual_shape(block.block_properties);
    }
    if block_name_is_fence(name) {
        return fence_visual_shape(block.block_properties);
    }
    block_collision_shape_with_context(world, block, pos, context)
}

pub(super) fn block_name_has_empty_camera_visual_shape(name: &str) -> bool {
    (name != "minecraft:barrier" && block_name_is_impermeable(name))
        || name == "minecraft:powder_snow"
        || block_name_is_copper_grate(name)
        || block_name_is_bars_or_pane(name)
}

pub(super) fn visit_block_collision_boxes_with_registry(
    pos: BlockPos,
    state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    mut visit: impl FnMut(WorldCollisionBox),
) {
    if let Some(shape) = vanilla_block_state_static_collision_shape(state_id) {
        shape.boxes().map(WorldCollisionBox::from).for_each(visit);
        return;
    }
    if !content_registry.is_vanilla_only() {
        if let Some((spec, _)) = content_registry.block_by_state_id(state_id) {
            if spec.full_cube_collision {
                visit(WorldCollisionBox::FULL);
            }
            return;
        }
    }
    if let Some(shape) = dynamic_block_collision_shape(&BlockView::of_state(pos, state_id), pos) {
        shape.boxes().for_each(visit);
    }
}

pub(super) fn block_collision_shape_with_registry(
    block: &BlockView,
    pos: BlockPos,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<BlockCollisionShape> {
    if let Some(vanilla_shape) = vanilla_block_state_static_collision_shape(block.block_state_id) {
        let mut builder = BlockCollisionShapeBuilder::new();
        let boxes = vanilla_shape.boxes();
        debug_assert!(boxes.len() <= MAX_COLLISION_BOXES);
        for shape_box in boxes {
            builder.push(shape_box.into());
        }
        return (builder.next_index != 0).then(|| builder.build());
    }
    if !content_registry.is_vanilla_only() {
        if let Some((spec, _)) = content_registry.block_by_state_id(block.block_state_id) {
            return spec
                .full_cube_collision
                .then(|| BlockCollisionShape::single(WorldCollisionBox::FULL));
        }
    }
    dynamic_block_collision_shape(block, pos)
}

pub(super) fn block_collision_shape(
    block: &BlockView,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    block_collision_shape_with_registry(
        block,
        pos,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn dynamic_block_collision_shape(block: &BlockView, pos: BlockPos) -> Option<BlockCollisionShape> {
    // Only `hasDynamicShape` blocks and ids outside the built-in registry
    // reach this point.
    match block.block_name {
        Some("minecraft:bamboo") => Some(bamboo_collision_shape(pos)),
        Some("minecraft:pointed_dripstone") => {
            pointed_dripstone_collision_shape(block.block_properties, pos)
        }
        Some("minecraft:scaffolding") => Some(scaffolding_stable_collision_shape()),
        // `EmptyBlockGetter` has no block entity, so Vanilla returns a full
        // cube. Live world collision uses the tracked lid instead.
        Some(name) if is_shulker_box_block_name(name) => {
            Some(BlockCollisionShape::single(WorldCollisionBox::FULL))
        }
        // Moving pistons and powder snow are empty without an entity context.
        Some(_) => None,
        None => Some(BlockCollisionShape::single(WorldCollisionBox::FULL)),
    }
}

pub(super) fn is_leaves_block_name(block_name: &str) -> bool {
    block_name_is_leaves(block_name)
}

pub(super) fn debug_climbable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:ladder"
            | "minecraft:vine"
            | "minecraft:scaffolding"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
    )
}

pub(super) fn debug_can_glide_through_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:vine"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
    )
}

pub(super) fn is_soul_sand_or_mud_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:soul_sand" | "minecraft:mud")
}

pub(super) fn is_hanging_sign_block_name(block_name: &str) -> bool {
    block_name_is_hanging_sign(block_name)
}

pub(super) fn is_shulker_box_block_name(block_name: &str) -> bool {
    block_name_is_shulker_box(block_name)
}

pub(super) fn is_fence_gate_block_name(block_name: &str) -> bool {
    block_name_is_fence_gate(block_name)
}

pub(super) fn is_fence_block_name(block_name: &str) -> bool {
    block_name_is_fence(block_name)
}

pub(super) fn is_wall_block_name(block_name: &str) -> bool {
    block_name_is_wall(block_name)
}

pub(super) fn snow_layer_visual_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    Some(BlockCollisionShape::single(WorldCollisionBox::column(
        0.0,
        0.0,
        1.0,
        f64::from(layers) / 8.0,
        1.0,
    )))
}

pub(super) fn bamboo_collision_shape(pos: BlockPos) -> BlockCollisionShape {
    let (offset_x, offset_z) = vanilla_xz_offset(pos, 0.25);
    BlockCollisionShape::single(WorldCollisionBox::centered_column(3.0, 3.0, 0.0, 16.0))
        .offset(offset_x, 0.0, offset_z)
}

pub(super) fn scaffolding_collision_shape(
    properties: &BTreeMap<String, String>,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    if context.is_above(pos, 1.0) && !context.descending {
        return Some(scaffolding_stable_collision_shape());
    }

    let distance = properties.get("distance")?.parse::<i32>().ok()?;
    if distance != 0 && bool_property(properties, "bottom")? && context.is_above(pos, 0.0) {
        return Some(BlockCollisionShape::single(
            WorldCollisionBox::SCAFFOLDING_UNSTABLE_BOTTOM,
        ));
    }

    None
}

pub(super) fn shulker_box_collision_shape(
    world: &WorldStore,
    block: &BlockView,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    let facing = BlockDirection::parse(block.block_properties.get("facing")?)?;
    let extension = 0.5 * f64::from(world.shulker_box_progress_at(pos, 1.0)).clamp(0.0, 1.0);
    let mut shape_box = WorldCollisionBox::FULL;
    match facing {
        BlockDirection::North => shape_box.min[2] -= extension,
        BlockDirection::East => shape_box.max[0] += extension,
        BlockDirection::South => shape_box.max[2] += extension,
        BlockDirection::West => shape_box.min[0] -= extension,
        BlockDirection::Up => shape_box.max[1] += extension,
        BlockDirection::Down => shape_box.min[1] -= extension,
    }
    Some(BlockCollisionShape::single(shape_box))
}

pub(super) fn scaffolding_outline_shape(bottom: bool) -> BlockCollisionShape {
    // ScaffoldingBlock.SHAPE_UNSTABLE's boxZ strips lie inside its bottom slab.
    let mut builder = BlockCollisionShapeBuilder::new();
    if bottom {
        builder.push(WorldCollisionBox::SCAFFOLDING_UNSTABLE_BOTTOM);
    }
    for shape_box in scaffolding_stable_collision_shape().boxes() {
        builder.push(shape_box);
    }
    builder.build()
}

pub(super) fn scaffolding_stable_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(WorldCollisionBox::SCAFFOLDING_TOP),
        Some(WorldCollisionBox::SCAFFOLDING_NORTH_WEST_POST),
        Some(WorldCollisionBox::SCAFFOLDING_NORTH_EAST_POST),
        Some(WorldCollisionBox::SCAFFOLDING_SOUTH_EAST_POST),
        Some(WorldCollisionBox::SCAFFOLDING_SOUTH_WEST_POST),
    ])
}

pub(super) fn powder_snow_collision_shape(
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    if context.fall_distance > LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_FALL_DISTANCE {
        return Some(BlockCollisionShape::single(
            WorldCollisionBox::POWDER_SNOW_FALLING,
        ));
    }
    if context.is_falling_block {
        return Some(BlockCollisionShape::single(WorldCollisionBox::FULL));
    }
    if context.can_walk_on_powder_snow && context.is_above(pos, 1.0) && !context.descending {
        return Some(BlockCollisionShape::single(WorldCollisionBox::FULL));
    }
    None
}

pub(super) fn pointed_dripstone_collision_shape(
    properties: &BTreeMap<String, String>,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    let shape_box = match properties.get("thickness").map(String::as_str)? {
        "tip_merge" => WorldCollisionBox::centered_column(6.0, 6.0, 0.0, 16.0),
        "tip" => match properties.get("vertical_direction").map(String::as_str)? {
            "down" => WorldCollisionBox::centered_column(6.0, 6.0, 5.0, 16.0),
            "up" => WorldCollisionBox::centered_column(6.0, 6.0, 0.0, 11.0),
            _ => return None,
        },
        "frustum" => WorldCollisionBox::centered_column(8.0, 8.0, 0.0, 16.0),
        "middle" => WorldCollisionBox::centered_column(10.0, 10.0, 0.0, 16.0),
        "base" => WorldCollisionBox::centered_column(12.0, 12.0, 0.0, 16.0),
        _ => return None,
    };
    let (offset_x, offset_z) = pointed_dripstone_xz_offset(pos);
    Some(BlockCollisionShape::single(shape_box).offset(offset_x, 0.0, offset_z))
}

pub(super) fn pointed_dripstone_xz_offset(pos: BlockPos) -> (f64, f64) {
    vanilla_xz_offset(pos, 2.0 * PX)
}

pub(super) fn vanilla_xz_offset(pos: BlockPos, max_horizontal_offset: f64) -> (f64, f64) {
    let seed = vanilla_block_seed(pos.x, 0, pos.z);
    let x = (((seed & 15) as f64 / 15.0) - 0.5) * 0.5;
    let z = ((((seed >> 8) & 15) as f64 / 15.0) - 0.5) * 0.5;
    (
        x.clamp(-max_horizontal_offset, max_horizontal_offset),
        z.clamp(-max_horizontal_offset, max_horizontal_offset),
    )
}

pub(super) fn vanilla_block_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut seed = i64::from(x).wrapping_mul(3_129_871)
        ^ i64::from(z).wrapping_mul(116_129_781)
        ^ i64::from(y);
    seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}

pub(super) fn fence_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    cross_collision_shape(
        properties,
        WorldCollisionBox::FENCE_POST,
        WorldCollisionBox::FENCE_NORTH_ARM,
        WorldCollisionBox::FENCE_EAST_ARM,
        WorldCollisionBox::FENCE_SOUTH_ARM,
        WorldCollisionBox::FENCE_WEST_ARM,
    )
}

pub(super) fn fence_visual_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let mut shape = fence_collision_shape(properties)?;
    for shape_box in shape.boxes.iter_mut().flatten() {
        shape_box.max[1] = 1.0;
    }
    Some(shape)
}

pub(super) fn cross_collision_shape(
    properties: &BTreeMap<String, String>,
    post: WorldCollisionBox,
    north_arm: WorldCollisionBox,
    east_arm: WorldCollisionBox,
    south_arm: WorldCollisionBox,
    west_arm: WorldCollisionBox,
) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(post);
    if bool_property(properties, "north")? {
        builder.push(north_arm);
    }
    if bool_property(properties, "east")? {
        builder.push(east_arm);
    }
    if bool_property(properties, "south")? {
        builder.push(south_arm);
    }
    if bool_property(properties, "west")? {
        builder.push(west_arm);
    }
    Some(builder.build())
}

pub(super) fn bool_property(properties: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    match properties.get(key).map(String::as_str)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub(super) fn bounds_intersects_block_shape(
    bounds: WorldCollisionBox,
    pos: BlockPos,
    shape: BlockCollisionShape,
) -> bool {
    shape
        .boxes()
        .any(|shape_box| bounds_intersects_block_box(bounds, pos, shape_box))
}

/// Vanilla `AABB.intersects(Vec3, Vec3)`: strict overlap with the endpoint
/// bounds, not a ray/segment clip.
pub(in crate::client) fn world_collision_box_intersects_segment_bounds(
    collision_box: WorldCollisionBox,
    from: [f64; 3],
    to: [f64; 3],
) -> bool {
    (0..3).all(|axis| {
        collision_box.min[axis] < from[axis].max(to[axis])
            && collision_box.max[axis] > from[axis].min(to[axis])
    })
}

pub(super) fn world_collision_box(pos: BlockPos, shape: WorldCollisionBox) -> WorldCollisionBox {
    WorldCollisionBox {
        min: [
            f64::from(pos.x) + shape.min[0],
            f64::from(pos.y) + shape.min[1],
            f64::from(pos.z) + shape.min[2],
        ],
        max: [
            f64::from(pos.x) + shape.max[0],
            f64::from(pos.y) + shape.max[1],
            f64::from(pos.z) + shape.max[2],
        ],
    }
}

pub(super) fn bounds_intersects_block_box(
    bounds: WorldCollisionBox,
    pos: BlockPos,
    shape: WorldCollisionBox,
) -> bool {
    let block_x = f64::from(pos.x);
    let min_x = block_x + shape.min[0];
    let max_x = block_x + shape.max[0];
    let min_y = f64::from(pos.y) + shape.min[1];
    let max_y = f64::from(pos.y) + shape.max[1];
    let block_z = f64::from(pos.z);
    let min_z = block_z + shape.min[2];
    let max_z = block_z + shape.max[2];

    bounds.max[0] > min_x + COLLISION_EPSILON
        && bounds.min[0] < max_x - COLLISION_EPSILON
        && bounds.max[1] > min_y + COLLISION_EPSILON
        && bounds.min[1] < max_y - COLLISION_EPSILON
        && bounds.max[2] > min_z + COLLISION_EPSILON
        && bounds.min[2] < max_z - COLLISION_EPSILON
}

pub(super) fn block_floor(value: f64) -> i32 {
    value.floor() as i32
}
