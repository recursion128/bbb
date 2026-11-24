use super::*;

/// Vanilla `HugeRedMushroomFeature` / `AbstractHugeMushroomFeature.place`
/// (`HugeRedMushroomFeature.java:17-69`, `AbstractHugeMushroomFeature.java:83-97`).
pub(super) fn place_huge_red_mushroom<L: WorldGenLevel>(
    config: &HugeMushroomConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    place_huge_mushroom(config, level, origin, random, HugeMushroomKind::Red)
}

/// Vanilla `HugeBrownMushroomFeature` / `AbstractHugeMushroomFeature.place`
/// (`HugeBrownMushroomFeature.java:17-61`, `AbstractHugeMushroomFeature.java:83-97`).
pub(super) fn place_huge_brown_mushroom<L: WorldGenLevel>(
    config: &HugeMushroomConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    place_huge_mushroom(config, level, origin, random, HugeMushroomKind::Brown)
}

#[derive(Clone, Copy)]
pub(super) enum HugeMushroomKind {
    Red,
    Brown,
}

pub(super) fn place_huge_mushroom<L: WorldGenLevel>(
    parsed: &HugeMushroomConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
    kind: HugeMushroomKind,
) -> Option<bool> {
    let tree_height = huge_mushroom_tree_height(random);
    if !huge_mushroom_is_valid_position(level, origin, tree_height, parsed, kind)? {
        return Some(false);
    }
    match kind {
        HugeMushroomKind::Red => make_red_mushroom_cap(level, random, origin, tree_height, parsed)?,
        HugeMushroomKind::Brown => {
            make_brown_mushroom_cap(level, random, origin, tree_height, parsed)?
        }
    }
    place_mushroom_trunk(level, random, origin, tree_height, parsed)?;
    Some(true)
}

/// Vanilla `AbstractHugeMushroomFeature.getTreeHeight`
/// (`AbstractHugeMushroomFeature.java:41-48`).
pub(super) fn huge_mushroom_tree_height(random: &mut dyn RandomSource) -> i32 {
    let mut tree_height = random.next_int(3) + 4;
    if random.next_int(12) == 0 {
        tree_height *= 2;
    }
    tree_height
}

/// Vanilla `AbstractHugeMushroomFeature.isValidPosition`
/// (`AbstractHugeMushroomFeature.java:50-80`).
pub(super) fn huge_mushroom_is_valid_position<L: WorldGenLevel>(
    level: &L,
    origin: BlockPos,
    tree_height: i32,
    config: &HugeMushroomConfig,
    kind: HugeMushroomKind,
) -> Option<bool> {
    let y = origin.y;
    if y < level.get_min_y() + 1 || y + tree_height + 1 > level.get_max_y() {
        return Some(false);
    }
    let below = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    if !block_predicates::block_predicate_test(&config.can_place_on, level, below)? {
        return Some(false);
    }
    for dy in 0..=tree_height {
        let radius = huge_mushroom_radius_for_height(kind, tree_height, config.foliage_radius, dy);
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let state = level.get_block(BlockPos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    z: origin.z + dz,
                });
                if !is_empty_block(state) && !level.block_state_in_tag(state, LEAVES_TAG) {
                    return Some(false);
                }
            }
        }
    }
    Some(true)
}

/// Vanilla `HugeBrownMushroomFeature.getTreeRadiusForHeight`
/// (`HugeBrownMushroomFeature.java:59-61`) and
/// `HugeRedMushroomFeature.getTreeRadiusForHeight`
/// (`HugeRedMushroomFeature.java:60-69`).
pub(super) fn huge_mushroom_radius_for_height(
    kind: HugeMushroomKind,
    tree_height: i32,
    leaf_radius: i32,
    yo: i32,
) -> i32 {
    match kind {
        HugeMushroomKind::Brown => {
            if yo <= 3 {
                0
            } else {
                leaf_radius
            }
        }
        HugeMushroomKind::Red => {
            if (yo < tree_height && yo >= tree_height - 3) || yo == tree_height {
                leaf_radius
            } else {
                0
            }
        }
    }
}

/// Vanilla `AbstractHugeMushroomFeature.placeTrunk`
/// (`AbstractHugeMushroomFeature.java:20-32`).
pub(super) fn place_mushroom_trunk<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    tree_height: i32,
    config: &HugeMushroomConfig,
) -> Option<()> {
    for dy in 0..tree_height {
        let pos = BlockPos {
            x: origin.x,
            y: origin.y + dy,
            z: origin.z,
        };
        let stem = config.stem_provider.get_state(level, random, origin)?;
        place_mushroom_block(level, pos, stem);
    }
    Some(())
}

/// Vanilla `AbstractHugeMushroomFeature.placeMushroomBlock`
/// (`AbstractHugeMushroomFeature.java:34-39`).
pub(super) fn place_mushroom_block<L: WorldGenLevel>(level: &mut L, pos: BlockPos, new_state: i32) {
    let current = level.get_block(pos);
    if is_empty_block(current) || level.block_state_in_tag(current, REPLACEABLE_BY_MUSHROOMS_TAG) {
        let _ = level.set_block(pos, new_state, FEATURE_SET_BLOCK);
    }
}

/// Vanilla `HugeBrownMushroomFeature.makeCap` (`HugeBrownMushroomFeature.java:17-56`).
pub(super) fn make_brown_mushroom_cap<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    tree_height: i32,
    config: &HugeMushroomConfig,
) -> Option<()> {
    let radius = config.foliage_radius;
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let min_x = dx == -radius;
            let max_x = dx == radius;
            let min_z = dz == -radius;
            let max_z = dz == radius;
            let x_edge = min_x || max_x;
            let z_edge = min_z || max_z;
            if x_edge && z_edge {
                continue;
            }
            let pos = BlockPos {
                x: origin.x + dx,
                y: origin.y + tree_height,
                z: origin.z + dz,
            };
            let west = min_x || (z_edge && dx == 1 - radius);
            let east = max_x || (z_edge && dx == radius - 1);
            let north = min_z || (x_edge && dz == 1 - radius);
            let south = max_z || (x_edge && dz == radius - 1);
            let mut state = config.cap_provider.get_state(level, random, origin)?;
            if block_has_property(state, "west")
                && block_has_property(state, "east")
                && block_has_property(state, "north")
                && block_has_property(state, "south")
            {
                state = block_set_bool(state, "west", west);
                state = block_set_bool(state, "east", east);
                state = block_set_bool(state, "north", north);
                state = block_set_bool(state, "south", south);
            }
            place_mushroom_block(level, pos, state);
        }
    }
    Some(())
}

/// Vanilla `HugeRedMushroomFeature.makeCap` (`HugeRedMushroomFeature.java:17-57`).
pub(super) fn make_red_mushroom_cap<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    tree_height: i32,
    config: &HugeMushroomConfig,
) -> Option<()> {
    for dy in (tree_height - 3)..=tree_height {
        let radius = if dy < tree_height {
            config.foliage_radius
        } else {
            config.foliage_radius - 1
        };
        let center = config.foliage_radius - 2;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let min_x = dx == -radius;
                let max_x = dx == radius;
                let min_z = dz == -radius;
                let max_z = dz == radius;
                let x_edge = min_x || max_x;
                let z_edge = min_z || max_z;
                if dy < tree_height && x_edge == z_edge {
                    continue;
                }
                let pos = BlockPos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    z: origin.z + dz,
                };
                let mut state = config.cap_provider.get_state(level, random, origin)?;
                if block_has_property(state, "west")
                    && block_has_property(state, "east")
                    && block_has_property(state, "north")
                    && block_has_property(state, "south")
                    && block_has_property(state, "up")
                {
                    state = block_set_bool(state, "up", dy >= tree_height - 1);
                    state = block_set_bool(state, "west", dx < -center);
                    state = block_set_bool(state, "east", dx > center);
                    state = block_set_bool(state, "north", dz < -center);
                    state = block_set_bool(state, "south", dz > center);
                }
                place_mushroom_block(level, pos, state);
            }
        }
    }
    Some(())
}

pub(super) fn block_has_property(state: i32, name: &str) -> bool {
    bbb_facts::block_state_types::vanilla_block_state_property(state, name).is_some()
}

pub(super) fn block_set_bool(state: i32, name: &str, value: bool) -> i32 {
    block_set_property(state, name, if value { "true" } else { "false" })
}

/// Vanilla `BlockColumnFeature.place` (`BlockColumnFeature.java:15-57`).
pub(super) fn place_block_column<L: WorldGenLevel>(
    parsed: &BlockColumnConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let mut layer_heights = Vec::with_capacity(parsed.layers.len());
    let mut total_height = 0;
    for layer in &parsed.layers {
        let height = layer.height.sample(random);
        total_height += height;
        layer_heights.push(height);
    }
    if total_height == 0 {
        return Some(false);
    }
    let step = offset_block_pos(BlockPos { x: 0, y: 0, z: 0 }, parsed.direction);
    let (dx, dy, dz) = (step.x, step.y, step.z);
    let mut next = BlockPos {
        x: origin.x + dx,
        y: origin.y + dy,
        z: origin.z + dz,
    };
    for y in 0..total_height {
        if !block_predicates::block_predicate_test(&parsed.allowed_placement, level, next)? {
            truncate_block_column_layers(
                &mut layer_heights,
                total_height,
                y,
                parsed.prioritize_tip,
            );
            break;
        }
        next.x += dx;
        next.y += dy;
        next.z += dz;
    }
    let mut place = origin;
    for (layer, count) in parsed.layers.iter().zip(layer_heights) {
        if count == 0 {
            continue;
        }
        for _ in 0..count {
            let state = layer.provider.get_state(level, random, place)?;
            let _ = level.set_block(place, state, UPDATE_CLIENTS);
            place.x += dx;
            place.y += dy;
            place.z += dz;
        }
    }
    Some(true)
}

/// Vanilla `DiskFeature.place` (`DiskFeature.java:16-36`).
pub(super) fn place_disk<L: WorldGenLevel>(
    parsed: &DiskConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let radius = parsed.radius.sample(random);
    let top = origin.y + parsed.half_height;
    let bottom = origin.y - parsed.half_height - 1;
    let mut placed_any = false;
    let start = BlockPos {
        x: origin.x - radius,
        y: origin.y,
        z: origin.z - radius,
    };
    let end = BlockPos {
        x: origin.x + radius,
        y: origin.y,
        z: origin.z + radius,
    };
    for column_pos in between_closed(start, end) {
        let xd = column_pos.x - origin.x;
        let zd = column_pos.z - origin.z;
        if xd * xd + zd * zd <= radius * radius {
            placed_any |= place_disk_column(level, random, parsed, top, bottom, column_pos)?;
        }
    }
    Some(placed_any)
}

/// Vanilla `DiskFeature.placeColumn` (`DiskFeature.java:45-63`).
pub(super) fn place_disk_column<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    config: &DiskConfig,
    top: i32,
    bottom: i32,
    mut pos: BlockPos,
) -> Option<bool> {
    let mut placed_any = false;
    let mut placed_above = false;
    let mut y = top;
    while y > bottom {
        pos.y = y;
        if block_predicates::block_predicate_test(&config.target, level, pos)? {
            if let Some(state) = config
                .state_provider
                .get_optional_state(level, random, pos)?
            {
                let _ = level.set_block(pos, state, UPDATE_CLIENTS);
                if !placed_above {
                    mark_above_for_post_processing(level, pos);
                }
                placed_any = true;
                placed_above = true;
            }
        } else {
            placed_above = false;
        }
        y -= 1;
    }
    Some(placed_any)
}

/// Vanilla `Feature.markAboveForPostProcessing` (`Feature.java:206-217`).
pub(super) fn mark_above_for_post_processing<L: WorldGenLevel>(level: &mut L, place_pos: BlockPos) {
    let mut pos = place_pos;
    for _ in 0..2 {
        pos.y += 1;
        if is_empty_block(level.get_block(pos)) {
            return;
        }
        level.mark_pos_for_postprocessing(pos);
    }
}

/// Vanilla `HugeFungusFeature.place` (`HugeFungusFeature.java:24-57`).
pub(super) fn place_huge_fungus<L: WorldGenLevel>(
    parsed: &HugeFungusConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let below = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    if vanilla_block_state_block_type_id(level.get_block(below))
        != vanilla_block_state_block_type_id(parsed.valid_base_state)
    {
        return Some(false);
    }
    let mut total_height = mth_next_int(random, 4, 13);
    if random.next_int(12) == 0 {
        total_height *= 2;
    }
    if !parsed.planted {
        let gen_depth = level.get_max_y() - level.get_min_y() + 1;
        if origin.y + total_height + 1 >= gen_depth {
            return Some(false);
        }
    }
    let is_huge = !parsed.planted && random.next_float() < HUGE_FUNGUS_HUGE_PROBABILITY;
    let air = default_block_state("minecraft:air");
    let _ = level.set_block(origin, air, UPDATE_NONE);
    place_huge_fungus_stem(level, random, parsed, origin, total_height, is_huge, air);
    place_huge_fungus_hat(level, random, parsed, origin, total_height, is_huge, air);
    Some(true)
}

/// Vanilla `HugeFungusFeature.isReplaceable` (`HugeFungusFeature.java:60-68`).
pub(super) fn huge_fungus_is_replaceable<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    replaceable_blocks: &BlockPredicate,
    check_non_replaceable_plants: bool,
) -> bool {
    if can_be_replaced(level, level.get_block(pos)) {
        return true;
    }
    if check_non_replaceable_plants {
        block_predicates::block_predicate_test(replaceable_blocks, level, pos).unwrap_or(false)
    } else {
        false
    }
}

/// Vanilla `BlockStateBase.canBeReplaced()` (`BlockBehaviour.java:827-829`).
pub(super) fn can_be_replaced<L: WorldGenLevel>(level: &L, state_id: i32) -> bool {
    level.block_state_in_tag(state_id, REPLACEABLE_TAG)
}

/// Vanilla `HugeFungusFeature.placeStem` (`HugeFungusFeature.java:82-105`).
pub(super) fn place_huge_fungus_stem<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    config: &HugeFungusConfig,
    origin: BlockPos,
    total_height: i32,
    is_huge: bool,
    air: i32,
) -> Option<()> {
    let stem_radius: i32 = if is_huge { 1 } else { 0 };
    for dx in -stem_radius..=stem_radius {
        for dz in -stem_radius..=stem_radius {
            let corner_of_huge_stem = is_huge && dx.abs() == stem_radius && dz.abs() == stem_radius;
            for dy in 0..total_height {
                let pos = BlockPos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    z: origin.z + dz,
                };
                if huge_fungus_is_replaceable(level, pos, &config.replaceable_blocks, true) {
                    if config.planted {
                        let below = BlockPos {
                            x: pos.x,
                            y: pos.y - 1,
                            z: pos.z,
                        };
                        if !is_empty_block(level.get_block(below)) {
                            let _ = level.set_block(pos, air, FEATURE_SET_BLOCK);
                        }
                        let _ = level.set_block(pos, config.stem_state, FEATURE_SET_BLOCK);
                    } else if corner_of_huge_stem {
                        if random.next_float() < 0.1 {
                            let _ = level.set_block(pos, config.stem_state, FEATURE_SET_BLOCK);
                        }
                    } else {
                        let _ = level.set_block(pos, config.stem_state, FEATURE_SET_BLOCK);
                    }
                }
            }
        }
    }
    Some(())
}

/// Vanilla `HugeFungusFeature.placeHat` (`HugeFungusFeature.java:121-157`).
pub(super) fn place_huge_fungus_hat<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    config: &HugeFungusConfig,
    origin: BlockPos,
    total_height: i32,
    is_huge: bool,
    air: i32,
) -> Option<()> {
    let place_vines = block_is(config.hat_state, "minecraft:nether_wart_block");
    let hat_height = (random.next_int(1 + total_height / 3) + 5).min(total_height);
    let hat_start_y = total_height - hat_height;
    for dy in hat_start_y..=total_height {
        let mut radius = if dy < total_height - random.next_int(3) {
            2
        } else {
            1
        };
        if hat_height > 8 && dy < hat_start_y + 4 {
            radius = 3;
        }
        if is_huge {
            radius += 1;
        }
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let is_edge_x = dx == -radius || dx == radius;
                let is_edge_z = dz == -radius || dz == radius;
                let inside = !is_edge_x && !is_edge_z && dy != total_height;
                let corner = is_edge_x && is_edge_z;
                let is_hat_bottom = dy < hat_start_y + 3;
                let pos = BlockPos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    z: origin.z + dz,
                };
                if huge_fungus_is_replaceable(level, pos, &config.replaceable_blocks, false) {
                    if config.planted
                        && !is_empty_block(level.get_block(BlockPos {
                            x: pos.x,
                            y: pos.y - 1,
                            z: pos.z,
                        }))
                    {
                        let _ = level.set_block(pos, air, FEATURE_SET_BLOCK);
                    }
                    if is_hat_bottom {
                        if !inside {
                            place_huge_fungus_hat_drop_block(
                                level,
                                random,
                                pos,
                                config.hat_state,
                                place_vines,
                            );
                        }
                    } else if inside {
                        place_huge_fungus_hat_block(
                            level,
                            random,
                            config,
                            pos,
                            0.1,
                            0.2,
                            if place_vines { 0.1 } else { 0.0 },
                        );
                    } else if corner {
                        place_huge_fungus_hat_block(
                            level,
                            random,
                            config,
                            pos,
                            0.01,
                            0.7,
                            if place_vines { 0.083 } else { 0.0 },
                        );
                    } else {
                        place_huge_fungus_hat_block(
                            level,
                            random,
                            config,
                            pos,
                            5.0e-4,
                            0.98,
                            if place_vines { 0.07 } else { 0.0 },
                        );
                    }
                }
            }
        }
    }
    Some(())
}

/// Vanilla `HugeFungusFeature.placeHatBlock` (`HugeFungusFeature.java:170-176`).
pub(super) fn place_huge_fungus_hat_block<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    config: &HugeFungusConfig,
    pos: BlockPos,
    decor_probability: f32,
    hat_probability: f32,
    vines_probability: f32,
) {
    if random.next_float() < decor_probability {
        let _ = level.set_block(pos, config.decor_state, FEATURE_SET_BLOCK);
    } else if random.next_float() < hat_probability {
        let _ = level.set_block(pos, config.hat_state, FEATURE_SET_BLOCK);
        if random.next_float() < vines_probability {
            try_place_weeping_vines(level, random, pos);
        }
    }
}

/// Vanilla `HugeFungusFeature.placeHatDropBlock` (`HugeFungusFeature.java:183-190`).
pub(super) fn place_huge_fungus_hat_drop_block<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    hat_state: i32,
    place_vines: bool,
) {
    let below = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    if vanilla_block_state_block_type_id(level.get_block(below))
        == vanilla_block_state_block_type_id(hat_state)
    {
        let _ = level.set_block(pos, hat_state, FEATURE_SET_BLOCK);
    } else if random.next_float() < 0.15 {
        let _ = level.set_block(pos, hat_state, FEATURE_SET_BLOCK);
        if place_vines && random.next_int(11) == 0 {
            try_place_weeping_vines(level, random, pos);
        }
    }
}

/// Vanilla `HugeFungusFeature.tryPlaceWeepingVines` (`HugeFungusFeature.java:193-204`).
pub(super) fn try_place_weeping_vines<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    hat_pos: BlockPos,
) {
    let mut place_pos = BlockPos {
        x: hat_pos.x,
        y: hat_pos.y - 1,
        z: hat_pos.z,
    };
    if is_empty_block(level.get_block(place_pos)) {
        let mut goal_vine_height = mth_next_int(random, 1, 5);
        if random.next_int(7) == 0 {
            goal_vine_height *= 2;
        }
        let plant = default_block_state("minecraft:weeping_vines_plant");
        place_weeping_vines_column(
            level,
            random,
            &mut place_pos,
            goal_vine_height,
            plant,
            23,
            25,
        );
    }
}

/// Vanilla `BlockPileFeature.place` (`BlockPileFeature.java:20-41`).
pub(super) fn place_block_pile<L: WorldGenLevel>(
    state_provider: &BlockStateProvider,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if origin.y < level.get_min_y() + 5 {
        return Some(false);
    }
    let xr = 2 + random.next_int(2);
    let zr = 2 + random.next_int(2);
    let start = BlockPos {
        x: origin.x - xr,
        y: origin.y,
        z: origin.z - zr,
    };
    let end = BlockPos {
        x: origin.x + xr,
        y: origin.y + 1,
        z: origin.z + zr,
    };
    for pos in between_closed(start, end) {
        let xd = origin.x - pos.x;
        let zd = origin.z - pos.z;
        if (xd * xd + zd * zd) as f32 <= random.next_float() * 10.0 - random.next_float() * 6.0
            || f64::from(random.next_float()) < 0.031
        {
            try_place_block_pile(level, pos, random, state_provider)?;
        }
    }
    Some(true)
}

/// Vanilla `BlockPileFeature.tryPlaceBlock` (`BlockPileFeature.java:50-54`).
pub(super) fn try_place_block_pile<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
    state_provider: &BlockStateProvider,
) -> Option<()> {
    if is_empty_block(level.get_block(pos)) && may_place_on_block_pile(level, pos, random) {
        let state = state_provider.get_state(level, random, pos)?;
        let _ = level.set_block(pos, state, UPDATE_NONE);
    }
    Some(())
}

/// Vanilla `BlockPileFeature.mayPlaceOn` (`BlockPileFeature.java:44-48`).
pub(super) fn may_place_on_block_pile<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) -> bool {
    let below = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    let below_state = level.get_block(below);
    if block_is(below_state, "minecraft:dirt_path") {
        return random.next_boolean();
    }
    feature_face_sturdy(
        level,
        below,
        below_state,
        Direction::Up,
        VanillaBlockSupportType::Full,
    )
}

/// Vanilla `NetherForestVegetationFeature.place`
/// (`NetherForestVegetationFeature.java:18-48`).
pub(super) fn place_nether_forest_vegetation<L: WorldGenLevel>(
    parsed: &NetherForestVegetationConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let below = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    let below_state = level.get_block(below);
    if !level.block_state_in_tag(below_state, NYLIUM_TAG) {
        return Some(false);
    }
    if origin.y < level.get_min_y() + 1 || origin.y + 1 > level.get_max_y() {
        return Some(false);
    }
    let attempts = parsed.spread_width.checked_mul(parsed.spread_width)?;
    let mut placed = 0;
    for _ in 0..attempts {
        let final_pos = BlockPos {
            x: origin.x + random.next_int(parsed.spread_width)
                - random.next_int(parsed.spread_width),
            y: origin.y + random.next_int(parsed.spread_height)
                - random.next_int(parsed.spread_height),
            z: origin.z + random.next_int(parsed.spread_width)
                - random.next_int(parsed.spread_width),
        };
        let state = parsed.state_provider.get_state(level, random, final_pos)?;
        if is_empty_block(level.get_block(final_pos))
            && final_pos.y > level.get_min_y()
            && feature_block_can_survive(level, final_pos, state)
        {
            let _ = level.set_block(final_pos, state, UPDATE_CLIENTS);
            placed += 1;
        }
    }
    Some(placed > 0)
}

/// Vanilla `BlockState.canSurvive` (`BlockBehaviour.java:831-833`).
pub(super) fn feature_block_can_survive<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    state: i32,
) -> bool {
    match survive_override(state) {
        Some(VanillaCanSurviveOverride::MossyCarpetBlock) => {
            return mossy_carpet::can_survive(level, pos, state).unwrap_or(false);
        }
        Some(VanillaCanSurviveOverride::MushroomBlock) => {
            return mushroom_can_survive(level, pos);
        }
        _ => {}
    }
    feature_owned_block_can_survive(level, pos, state)
}

/// `MushroomBlock.canSurvive` (`MushroomBlock.java:78-86`).
pub(super) fn mushroom_can_survive<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let below = level.get_block(BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    });
    if level.block_state_in_tag(below, OVERRIDES_MUSHROOM_LIGHT_REQUIREMENT_TAG) {
        return true;
    }
    let solid_render =
        vanilla_block_state_support_facts(below).is_some_and(|facts| facts.solid_render());
    level.get_raw_brightness(pos, 0) < 13 && solid_render
}

/// Vanilla `SimpleBlockFeature.place` (`SimpleBlockFeature.java:18-47`).
pub(super) fn place_simple_block<L: WorldGenLevel>(
    to_place: &BlockStateProvider,
    schedule_tick: bool,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let Some(state) = to_place.get_optional_state(level, random, origin)? else {
        return Some(false);
    };
    if !feature_block_can_survive(level, origin, state) {
        return Some(false);
    }
    let survive_override = survive_override(state);
    if survive_override == Some(VanillaCanSurviveOverride::MossyCarpetBlock) {
        mossy_carpet::place_at(level, origin, UPDATE_CLIENTS)?;
    } else if matches!(
        survive_override,
        Some(
            VanillaCanSurviveOverride::DoublePlantBlock
                | VanillaCanSurviveOverride::PitcherCropBlock
                | VanillaCanSurviveOverride::SmallDripleafBlock
                | VanillaCanSurviveOverride::TallSeagrassBlock
        )
    ) {
        let above = BlockPos {
            x: origin.x,
            y: origin.y + 1,
            z: origin.z,
        };
        if !is_empty_block(level.get_block(above)) {
            return Some(false);
        }
        place_double_plant(level, state, origin);
    } else {
        let _ = level.set_block(origin, state, UPDATE_CLIENTS);
    }
    if schedule_tick {
        level.schedule_block_tick(origin, 1);
    }
    Some(true)
}

/// Vanilla `DoublePlantBlock.placeAt` (`DoublePlantBlock.java:86-90`).
pub(super) fn place_double_plant<L: WorldGenLevel>(level: &mut L, state: i32, lower: BlockPos) {
    let upper = BlockPos {
        x: lower.x,
        y: lower.y + 1,
        z: lower.z,
    };
    let lower_state =
        copy_waterlogged_from(level, lower, block_set_property(state, "half", "lower"));
    let upper_state =
        copy_waterlogged_from(level, upper, block_set_property(state, "half", "upper"));
    let _ = level.set_block(lower, lower_state, UPDATE_CLIENTS);
    let _ = level.set_block(upper, upper_state, UPDATE_CLIENTS);
}

/// Vanilla `DoublePlantBlock.copyWaterloggedFrom` (`DoublePlantBlock.java:92-94`).
pub(super) fn copy_waterlogged_from<L: WorldGenLevel>(level: &L, pos: BlockPos, state: i32) -> i32 {
    if block_has_property(state, "waterlogged") {
        block_set_bool(state, "waterlogged", is_water_at(level.get_block(pos)))
    } else {
        state
    }
}

pub(super) fn survive_override(state: i32) -> Option<VanillaCanSurviveOverride> {
    vanilla_block_state_block_type_id(state)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.can_survive_override)
}

/// `StateHolder.trySetValue`: a state without the property is returned unchanged.
pub(super) fn block_set_property(state: i32, name: &str, value: &str) -> i32 {
    bbb_facts::block_state_types::vanilla_block_state_with_property(state, name, value)
        .unwrap_or(state)
}

/// Vanilla `FallenTreeFeature.place` (`FallenTreeFeature.java:32-45`).
pub(super) fn place_fallen_tree<L: WorldGenLevel>(
    parsed: &FallenTreeConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let stump = place_fallen_tree_stump(level, random, origin, &parsed.trunk_provider, None)?;
    let mut stump_logs = BlockPosSet::new();
    stump_logs.add(stump);
    place_log_decorators(&parsed.stump_decorators, level, random, &stump_logs);
    let (dx, _, dz) = HORIZONTAL_OFFSETS[random.next_int(4) as usize];
    let axis = if dx != 0 { "x" } else { "z" };
    let log_length = parsed.log_length.sample(random) - 2;
    let start_offset = 2 + random.next_int(2);
    let mut log_start = BlockPos {
        x: origin.x + dx * start_offset,
        y: origin.y,
        z: origin.z + dz * start_offset,
    };
    set_ground_height_for_fallen_log(level, &mut log_start);
    if can_place_entire_fallen_log(level, log_length, log_start, dx, dz) {
        place_fallen_log(
            level,
            random,
            &parsed.trunk_provider,
            log_length,
            log_start,
            dx,
            dz,
            axis,
            &parsed.log_decorators,
        )?;
    }
    Some(true)
}

/// Vanilla `FallenTreeFeature.setGroundHeightForFallenLogStartPos`
/// (`FallenTreeFeature.java:47-56`).
pub(super) fn set_ground_height_for_fallen_log<L: WorldGenLevel>(
    level: &L,
    log_start: &mut BlockPos,
) {
    log_start.y += 1;
    for _ in 0..6 {
        if fallen_tree_may_place_on(level, *log_start) {
            return;
        }
        log_start.y -= 1;
    }
}

/// Vanilla `FallenTreeFeature.canPlaceEntireFallenLog`
/// (`FallenTreeFeature.java:64-86`).
pub(super) fn can_place_entire_fallen_log<L: WorldGenLevel>(
    level: &L,
    log_length: i32,
    start: BlockPos,
    dx: i32,
    dz: i32,
) -> bool {
    if log_length <= 0 {
        return true;
    }
    let mut cursor = start;
    let mut gap_in_ground = 0;
    for _ in 0..log_length {
        if !valid_tree_pos(level, cursor) {
            return false;
        }
        if !fallen_tree_is_over_solid_ground(level, cursor) {
            gap_in_ground += 1;
            if gap_in_ground > 2 {
                return false;
            }
        } else {
            gap_in_ground = 0;
        }
        cursor.x += dx;
        cursor.z += dz;
    }
    true
}

/// Vanilla `FallenTreeFeature.placeFallenLog` (`FallenTreeFeature.java:89-105`).
#[allow(clippy::too_many_arguments)]
pub(super) fn place_fallen_log<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    trunk_provider: &BlockStateProvider,
    log_length: i32,
    mut log_start: BlockPos,
    dx: i32,
    dz: i32,
    axis: &str,
    decorators: &[TreeDecorator],
) -> Option<()> {
    let mut logs = BlockPosSet::new();
    for _ in 0..log_length {
        let log = place_fallen_tree_stump(level, random, log_start, trunk_provider, Some(axis))?;
        logs.add(log);
        log_start.x += dx;
        log_start.z += dz;
    }
    place_log_decorators(decorators, level, random, &logs);
    Some(())
}

/// Vanilla `FallenTreeFeature.placeLogBlock` (`FallenTreeFeature.java:115-125`).
pub(super) fn place_fallen_tree_stump<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    trunk_provider: &BlockStateProvider,
    axis: Option<&str>,
) -> Option<BlockPos> {
    let mut state = trunk_provider.get_state(level, random, pos)?;
    if let Some(axis) = axis {
        if block_has_property(state, "axis") {
            state = block_set_property(state, "axis", axis);
        }
    }
    let _ = level.set_block(pos, state, FEATURE_SET_BLOCK);
    mark_above_for_post_processing(level, pos);
    Some(pos)
}

/// Vanilla `TreeFeature.validTreePos` (`TreeFeature.java:53-55`).
pub(super) fn valid_tree_pos<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let state = level.get_block(pos);
    if is_empty_block(state) {
        return true;
    }
    level.block_state_in_tag(state, REPLACEABLE_BY_TREES_TAG)
}

/// Vanilla `FallenTreeFeature.mayPlaceOn` (`FallenTreeFeature.java:107-109`).
pub(super) fn fallen_tree_may_place_on<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    valid_tree_pos(level, pos) && fallen_tree_is_over_solid_ground(level, pos)
}

/// Vanilla `FallenTreeFeature.isOverSolidGround` (`FallenTreeFeature.java:111-113`).
pub(super) fn fallen_tree_is_over_solid_ground<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let below = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    feature_face_sturdy(
        level,
        below,
        level.get_block(below),
        Direction::Up,
        VanillaBlockSupportType::Full,
    )
}

/// Vanilla `BambooFeature.place` (`BambooFeature.java:32-74`).
pub(super) fn place_bamboo<L: WorldGenLevel>(
    probability: f32,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_empty_block(level.get_block(origin)) {
        return Some(false);
    }
    let bamboo = bamboo_state("0", "none", "0");
    if feature_block_can_survive(level, origin, bamboo) {
        let height = random.next_int(12) + 5;
        if random.next_float() < probability {
            let radius = random.next_int(4) + 1;
            let podzol = vanilla_block_default_state_id(block_type_id("minecraft:podzol"))?;
            for xx in (origin.x - radius)..=(origin.x + radius) {
                for zz in (origin.z - radius)..=(origin.z + radius) {
                    let xd = xx - origin.x;
                    let zd = zz - origin.z;
                    if xd * xd + zd * zd <= radius * radius {
                        let podzol_pos = BlockPos {
                            x: xx,
                            y: world_surface_height(level, xx, zz) - 1,
                            z: zz,
                        };
                        let podzol_state = level.get_block(podzol_pos);
                        if level
                            .block_state_in_tag(podzol_state, BENEATH_BAMBOO_PODZOL_REPLACEABLE_TAG)
                        {
                            let _ = level.set_block(podzol_pos, podzol, UPDATE_CLIENTS);
                        }
                    }
                }
            }
        }
        let trunk = bamboo_state("1", "none", "0");
        let mut bamboo_pos = origin;
        for _ in 0..height {
            if !is_empty_block(level.get_block(bamboo_pos)) {
                break;
            }
            let _ = level.set_block(bamboo_pos, trunk, UPDATE_CLIENTS);
            bamboo_pos.y += 1;
        }
        if bamboo_pos.y - origin.y >= 3 {
            let _ = level.set_block(bamboo_pos, bamboo_state("1", "large", "1"), UPDATE_CLIENTS);
            bamboo_pos.y -= 1;
            let _ = level.set_block(bamboo_pos, bamboo_state("1", "large", "0"), UPDATE_CLIENTS);
            bamboo_pos.y -= 1;
            let _ = level.set_block(bamboo_pos, bamboo_state("1", "small", "0"), UPDATE_CLIENTS);
        }
    }
    Some(true)
}

/// Vanilla `LevelReader.getHeight(Heightmap.Types.WORLD_SURFACE)`
/// (`Heightmap.java:30`, `Heightmap.java:146`).
pub(super) fn world_surface_height<L: WorldGenLevel>(level: &L, x: i32, z: i32) -> i32 {
    let min_y = level.get_min_y();
    for y in (min_y..=level.get_max_y()).rev() {
        if !is_empty_block(level.get_block(BlockPos { x, y, z })) {
            return y + 1;
        }
    }
    min_y
}

/// Vanilla `SeagrassFeature.place` (`SeagrassFeature.java:21-49`).
pub(super) fn place_seagrass<L: WorldGenLevel>(
    probability: f32,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let x = random.next_int(8) - random.next_int(8);
    let z = random.next_int(8) - random.next_int(8);
    let grass_pos = BlockPos {
        x: origin.x + x,
        y: ocean_floor_height(level, origin.x + x, origin.z + z),
        z: origin.z + z,
    };
    if !block_is(level.get_block(grass_pos), "minecraft:water") {
        return Some(false);
    }
    let is_tall = random.next_double() < f64::from(probability);
    let seagrass = default_block_state("minecraft:seagrass");
    // `TallSeagrassBlock.canSurvive` for LOWER (`TallSeagrassBlock.java:68-76`)
    // is `DoublePlantBlock` super + a full water fluid. Super reaches the same
    // `mayPlaceOn` as `SeagrassBlock.java:46-48`, already owned as
    // `VegetationSupport::Seagrass`. Feature.place already required
    // `Blocks.WATER` at `grassPos`.
    if !feature_block_can_survive(level, grass_pos, seagrass) {
        return Some(false);
    }
    if is_tall {
        let above = BlockPos {
            x: grass_pos.x,
            y: grass_pos.y + 1,
            z: grass_pos.z,
        };
        if block_is(level.get_block(above), "minecraft:water") {
            let tall = vanilla_block_default_state_id(block_type_id("minecraft:tall_seagrass"))?;
            let upper = block_set_property(tall, "half", "upper");
            let _ = level.set_block(grass_pos, tall, UPDATE_CLIENTS);
            let _ = level.set_block(above, upper, UPDATE_CLIENTS);
        }
    } else {
        let _ = level.set_block(grass_pos, seagrass, UPDATE_CLIENTS);
    }
    Some(true)
}

/// Vanilla `LevelReader.getHeight(Heightmap.Types.OCEAN_FLOOR)` as
/// `WorldGenRegion` evaluates it (`WorldGenRegion.java:391-392`,
/// `ChunkAccess.java:182-193`, `Heightmap.java:31`, `Heightmap.java:148`).
pub(super) fn ocean_floor_height<L: WorldGenLevel>(level: &L, x: i32, z: i32) -> i32 {
    let min_y = level.get_min_y();
    for y in (min_y..=level.get_max_y()).rev() {
        if block_state_blocks_motion(level.get_block(BlockPos { x, y, z })) {
            return y + 1;
        }
    }
    min_y
}

/// Vanilla `SeaPickleFeature.place` (`SeaPickleFeature.java:20-38`).
pub(super) fn place_sea_pickle<L: WorldGenLevel>(
    count: &IntProvider,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let count = count.sample(random);
    let default_pickle = vanilla_block_default_state_id(block_type_id("minecraft:sea_pickle"))?;
    let mut placed = 0;
    for _ in 0..count {
        let x = random.next_int(8) - random.next_int(8);
        let z = random.next_int(8) - random.next_int(8);
        let pickle_pos = BlockPos {
            x: origin.x + x,
            y: ocean_floor_height(level, origin.x + x, origin.z + z),
            z: origin.z + z,
        };
        const PICKLE_COUNTS: [&str; 4] = ["1", "2", "3", "4"];
        let pickle = block_set_property(
            default_pickle,
            "pickles",
            PICKLE_COUNTS[random.next_int(4) as usize],
        );
        if block_is(level.get_block(pickle_pos), "minecraft:water")
            && sea_pickle_can_survive(level, pickle_pos)
        {
            let _ = level.set_block(pickle_pos, pickle, UPDATE_CLIENTS);
            placed += 1;
        }
    }
    Some(placed > 0)
}

/// Vanilla `SeaPickleBlock.canSurvive` (`SeaPickleBlock.java:69-72`) via
/// `mayPlaceOn` (`SeaPickleBlock.java:64-66`).
pub(super) fn sea_pickle_can_survive<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let below_pos = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    let below = level.get_block(below_pos);

    feature_collision_face_nonempty(level, below_pos, below, Direction::Up)
        || feature_face_sturdy(
            level,
            below_pos,
            below,
            Direction::Up,
            VanillaBlockSupportType::Full,
        )
}

/// Vanilla `Direction.values()` except `DOWN` (`Direction.java:33-38`), the
/// order `VinesFeature` walks (`VinesFeature.java:25-29`). Each entry is the
/// vine face property and the neighbour face `canAttachTo` tests
/// (`MultifaceBlock.java:257-261`).
pub(super) const VINE_ATTACH_FACES: [(&str, Direction); 5] = [
    ("up", Direction::Down),
    ("north", Direction::South),
    ("south", Direction::North),
    ("west", Direction::East),
    ("east", Direction::West),
];

/// Vanilla `VinesFeature.place` (`VinesFeature.java:18-32`).
pub(super) fn place_vines<L: WorldGenLevel>(level: &mut L, origin: BlockPos) -> Option<bool> {
    if !is_empty_block(level.get_block(origin)) {
        return Some(false);
    }
    let vine = vanilla_block_default_state_id(block_type_id("minecraft:vine"))?;
    for &(face, opposite) in &VINE_ATTACH_FACES {
        let (dx, dy, dz) = direction_offset(face)?;
        let neighbour = BlockPos {
            x: origin.x + dx,
            y: origin.y + dy,
            z: origin.z + dz,
        };
        if vine_can_attach_to(level, neighbour, opposite) {
            let state = block_set_bool(vine, face, true);
            let _ = level.set_block(origin, state, UPDATE_CLIENTS);
            return Some(true);
        }
    }
    Some(false)
}

/// Vanilla `VineBlock.isAcceptableNeighbour` (`VineBlock.java:118-120`) via
/// `MultifaceBlock.canAttachTo` (`MultifaceBlock.java:257-261`).
pub(super) fn vine_can_attach_to<L: WorldGenLevel>(
    level: &L,
    neighbour_pos: BlockPos,
    neighbour_face: Direction,
) -> bool {
    feature_multiface_can_attach_to(
        level,
        neighbour_pos,
        level.get_block(neighbour_pos),
        direction_opposite(neighbour_face),
    )
}

/// Vanilla `ChorusPlantFeature.place` (`ChorusPlantFeature.java:18-26`).
pub(super) fn place_chorus_plant<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_empty_block(level.get_block(origin)) {
        return Some(false);
    }
    let below = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    let below_state = level.get_block(below);
    if !level.block_state_in_tag(below_state, SUPPORTS_CHORUS_PLANT_TAG) {
        return Some(false);
    }
    generate_chorus_plant(level, origin, random, CHORUS_MAX_HORIZONTAL_SPREAD)?;
    Some(true)
}

/// Vanilla `ChorusFlowerBlock.generatePlant` (`ChorusFlowerBlock.java:200-203`).
pub(super) fn generate_chorus_plant<L: WorldGenLevel>(
    level: &mut L,
    target: BlockPos,
    random: &mut dyn RandomSource,
    max_horizontal_spread: i32,
) -> Option<()> {
    let state = chorus_plant_state_with_connections(level, target)?;
    let _ = level.set_block(target, state, UPDATE_CLIENTS);
    grow_chorus_tree_recursive(level, target, random, target, max_horizontal_spread, 0)
}

/// Vanilla `ChorusFlowerBlock.growTreeRecursive` (`ChorusFlowerBlock.java:205-254`).
pub(super) fn grow_chorus_tree_recursive<L: WorldGenLevel>(
    level: &mut L,
    current: BlockPos,
    random: &mut dyn RandomSource,
    start_pos: BlockPos,
    max_horizontal_spread: i32,
    depth: i32,
) -> Option<()> {
    let mut height = random.next_int(4) + 1;
    if depth == 0 {
        height += 1;
    }
    for i in 0..height {
        let target = BlockPos {
            x: current.x,
            y: current.y + i + 1,
            z: current.z,
        };
        if !chorus_all_neighbors_empty(level, target, None) {
            return Some(());
        }
        let plant = chorus_plant_state_with_connections(level, target)?;
        let _ = level.set_block(target, plant, UPDATE_CLIENTS);
        let below = BlockPos {
            x: target.x,
            y: target.y - 1,
            z: target.z,
        };
        let below_state = chorus_plant_state_with_connections(level, below)?;
        let _ = level.set_block(below, below_state, UPDATE_CLIENTS);
    }
    let mut placed_stem = false;
    if depth < 4 {
        let mut stems = random.next_int(4);
        if depth == 0 {
            stems += 1;
        }
        for _ in 0..stems {
            let direction = HORIZONTAL_FACES[random.next_int(4) as usize];
            let target = offset_block_pos(
                BlockPos {
                    x: current.x,
                    y: current.y + height,
                    z: current.z,
                },
                direction,
            );
            if (target.x - start_pos.x).abs() < max_horizontal_spread
                && (target.z - start_pos.z).abs() < max_horizontal_spread
                && is_empty_block(level.get_block(target))
                && is_empty_block(level.get_block(offset_block_pos(target, Direction::Down)))
                && chorus_all_neighbors_empty(level, target, Some(direction_opposite(direction)))
            {
                placed_stem = true;
                let plant = chorus_plant_state_with_connections(level, target)?;
                let _ = level.set_block(target, plant, UPDATE_CLIENTS);
                let back = offset_block_pos(target, direction_opposite(direction));
                let back_state = chorus_plant_state_with_connections(level, back)?;
                let _ = level.set_block(back, back_state, UPDATE_CLIENTS);
                grow_chorus_tree_recursive(
                    level,
                    target,
                    random,
                    start_pos,
                    max_horizontal_spread,
                    depth + 1,
                )?;
            }
        }
    }
    if !placed_stem {
        let flower_pos = BlockPos {
            x: current.x,
            y: current.y + height,
            z: current.z,
        };
        let _ = level.set_block(flower_pos, chorus_flower_age(5)?, UPDATE_CLIENTS);
    }
    Some(())
}

/// Vanilla `ChorusFlowerBlock.allNeighborsEmpty` (`ChorusFlowerBlock.java:138-146`).
pub(super) fn chorus_all_neighbors_empty<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    ignore: Option<Direction>,
) -> bool {
    HORIZONTAL_FACES.iter().all(|&face| {
        Some(face) == ignore || is_empty_block(level.get_block(offset_block_pos(pos, face)))
    })
}

/// Vanilla `ChorusPlantBlock.getStateWithConnections` (`ChorusPlantBlock.java:45-59`).
pub(super) fn chorus_plant_state_with_connections<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
) -> Option<i32> {
    let mut state = vanilla_block_default_state_id(block_type_id("minecraft:chorus_plant"))?;
    let down = level.get_block(offset_block_pos(pos, Direction::Down));
    let up = level.get_block(offset_block_pos(pos, Direction::Up));
    let north = level.get_block(offset_block_pos(pos, Direction::North));
    let east = level.get_block(offset_block_pos(pos, Direction::East));
    let south = level.get_block(offset_block_pos(pos, Direction::South));
    let west = level.get_block(offset_block_pos(pos, Direction::West));
    state = block_set_bool(state, "down", chorus_plant_connects(level, down, true));
    state = block_set_bool(state, "up", chorus_plant_connects(level, up, false));
    state = block_set_bool(state, "north", chorus_plant_connects(level, north, false));
    state = block_set_bool(state, "east", chorus_plant_connects(level, east, false));
    state = block_set_bool(state, "south", chorus_plant_connects(level, south, false));
    state = block_set_bool(state, "west", chorus_plant_connects(level, west, false));
    Some(state)
}

pub(super) fn chorus_plant_connects<L: WorldGenLevel>(level: &L, state: i32, down: bool) -> bool {
    if block_is(state, "minecraft:chorus_plant") || block_is(state, "minecraft:chorus_flower") {
        return true;
    }
    down && level.block_state_in_tag(state, SUPPORTS_CHORUS_PLANT_TAG)
}

pub(super) fn chorus_flower_age(age: i32) -> Option<i32> {
    let flower = vanilla_block_default_state_id(block_type_id("minecraft:chorus_flower"))?;
    let age_token = match age {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        _ => return Some(flower),
    };
    Some(block_set_property(flower, "age", age_token))
}

pub(super) fn offset_block_pos(pos: BlockPos, direction: Direction) -> BlockPos {
    let (dx, dy, dz) = match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    };
    BlockPos {
        x: pos.x + dx,
        y: pos.y + dy,
        z: pos.z + dz,
    }
}

pub(super) fn direction_opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

/// Vanilla `KelpFeature.place` (`KelpFeature.java:20-52`).
pub(super) fn place_kelp<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let mut kelp_pos = BlockPos {
        x: origin.x,
        y: ocean_floor_height(level, origin.x, origin.z),
        z: origin.z,
    };
    if !block_is(level.get_block(kelp_pos), "minecraft:water") {
        return Some(false);
    }
    let kelp_plant = default_block_state("minecraft:kelp_plant");
    let height = 1 + random.next_int(10);
    let mut placed = 0;
    for h in 0..=height {
        let above = BlockPos {
            x: kelp_pos.x,
            y: kelp_pos.y + 1,
            z: kelp_pos.z,
        };
        if block_is(level.get_block(kelp_pos), "minecraft:water")
            && block_is(level.get_block(above), "minecraft:water")
            && kelp_can_survive(level, kelp_pos)
        {
            if h == height {
                let _ = level.set_block(
                    kelp_pos,
                    growing_plant_head_state("minecraft:kelp", random.next_int(4) + 20),
                    UPDATE_CLIENTS,
                );
                placed += 1;
            } else {
                let _ = level.set_block(kelp_pos, kelp_plant, UPDATE_CLIENTS);
            }
        } else if h > 0 {
            let below = BlockPos {
                x: kelp_pos.x,
                y: kelp_pos.y - 1,
                z: kelp_pos.z,
            };
            let below_below = BlockPos {
                x: below.x,
                y: below.y - 1,
                z: below.z,
            };
            if kelp_can_survive(level, below)
                && !block_is(level.get_block(below_below), "minecraft:kelp")
            {
                let _ = level.set_block(
                    below,
                    growing_plant_head_state("minecraft:kelp", random.next_int(4) + 20),
                    UPDATE_CLIENTS,
                );
                placed += 1;
            }
            break;
        }
        kelp_pos.y += 1;
    }
    Some(placed > 0)
}

/// Vanilla `GrowingPlantBlock.canSurvive` (`GrowingPlantBlock.java:47-55`) for
/// kelp / kelp plant. Both reach `KelpBlock.canAttachTo`
/// (`KelpBlock.java:47-48`, `KelpPlantBlock.java:40-42`).
pub(super) fn kelp_can_survive<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let attached = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    let attached_state = level.get_block(attached);
    if level.block_state_in_tag(attached_state, CANNOT_SUPPORT_KELP_TAG) {
        return false;
    }
    if block_is(attached_state, "minecraft:kelp")
        || block_is(attached_state, "minecraft:kelp_plant")
    {
        return true;
    }
    feature_face_sturdy(
        level,
        attached,
        attached_state,
        Direction::Up,
        VanillaBlockSupportType::Full,
    )
}

pub(super) fn bamboo_state(age: &str, leaves: &str, stage: &str) -> i32 {
    let state = catalog_default_state("minecraft:bamboo");
    let state = bbb_facts::block_state_types::vanilla_block_state_with_property(state, "age", age)
        .expect("vanilla bamboo age property");
    let state =
        bbb_facts::block_state_types::vanilla_block_state_with_property(state, "leaves", leaves)
            .expect("vanilla bamboo leaves property");
    bbb_facts::block_state_types::vanilla_block_state_with_property(state, "stage", stage)
        .expect("vanilla bamboo stage property")
}

/// Vanilla `Direction` step vectors (`Direction.java:33-38`).
pub(super) fn direction_offset(name: &str) -> Option<(i32, i32, i32)> {
    match name {
        "down" => Some((0, -1, 0)),
        "up" => Some((0, 1, 0)),
        "north" => Some((0, 0, -1)),
        "south" => Some((0, 0, 1)),
        "west" => Some((-1, 0, 0)),
        "east" => Some((1, 0, 0)),
        _ => None,
    }
}

/// Vanilla `BlockColumnFeature.truncate` (`BlockColumnFeature.java:59-71`).
pub(super) fn truncate_block_column_layers(
    layer_heights: &mut [i32],
    total_height: i32,
    new_height: i32,
    prioritize_tip: bool,
) {
    let mut amount_to_remove = total_height - new_height;
    if prioritize_tip {
        for height in layer_heights.iter_mut() {
            if amount_to_remove <= 0 {
                break;
            }
            let to_remove = (*height).min(amount_to_remove);
            amount_to_remove -= to_remove;
            *height -= to_remove;
        }
    } else {
        for height in layer_heights.iter_mut().rev() {
            if amount_to_remove <= 0 {
                break;
            }
            let to_remove = (*height).min(amount_to_remove);
            amount_to_remove -= to_remove;
            *height -= to_remove;
        }
    }
}

/// Vanilla `RuleTest.test(state, random)`.
pub(super) fn rule_test_matches<L: WorldGenLevel>(
    level: &L,
    test: &RuleTest,
    state: i32,
    random: &mut dyn RandomSource,
) -> bool {
    match test {
        RuleTest::AlwaysTrue => true,
        RuleTest::BlockMatch(block) => block_is(state, block),
        RuleTest::BlockStateMatch(want) => state == *want,
        RuleTest::TagMatch(tag) => level.block_state_in_tag(state, tag),
        RuleTest::RandomBlockMatch { block, probability } => {
            block_is(state, block) && random.next_float() < *probability
        }
        RuleTest::RandomBlockStateMatch {
            state: want,
            probability,
        } => state == *want && random.next_float() < *probability,
    }
}

/// Vanilla `Mth.randomBetween(float)` (`Mth.java:668-670`).
pub(super) fn mth_random_between_f32(
    random: &mut dyn RandomSource,
    min: f32,
    max_exclusive: f32,
) -> f32 {
    random.next_float() * (max_exclusive - min) + min
}

/// Vanilla `Mth.randomBetweenInclusive` (`Mth.java:664-666`).
pub(super) fn mth_random_between_inclusive(
    random: &mut dyn RandomSource,
    min: i32,
    max_inclusive: i32,
) -> i32 {
    random.next_int(max_inclusive - min + 1) + min
}

/// Vanilla `Mth.clamp(int)` (`Mth.java:93-95`).
pub(super) fn mth_clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

/// Vanilla `Vec3i.distManhattan` (`Vec3i.java:223-227`).
pub(super) fn dist_manhattan(a: BlockPos, b: BlockPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()
}

/// Vanilla `BlockPos.withinManhattan` (`BlockPos.java:296-347`).
pub(super) fn within_manhattan(
    origin: BlockPos,
    reach_x: i32,
    reach_y: i32,
    reach_z: i32,
) -> Vec<BlockPos> {
    let max_depth = reach_x + reach_y + reach_z;
    let mut out = Vec::new();
    let mut current_depth = 0;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut x = 0;
    let mut y = 0;
    let mut z_mirror = false;
    let mut cursor = origin;
    loop {
        if z_mirror {
            z_mirror = false;
            cursor.z = origin.z - (cursor.z - origin.z);
            out.push(cursor);
            continue;
        }
        let found = loop {
            if y > max_y {
                x += 1;
                if x > max_x {
                    current_depth += 1;
                    if current_depth > max_depth {
                        return out;
                    }
                    max_x = reach_x.min(current_depth);
                    x = -max_x;
                }
                max_y = reach_y.min(current_depth - x.abs());
                y = -max_y;
            }
            let xx = x;
            let yy = y;
            let zz = current_depth - xx.abs() - yy.abs();
            y += 1;
            if zz <= reach_z {
                z_mirror = zz != 0;
                cursor = BlockPos {
                    x: origin.x + xx,
                    y: origin.y + yy,
                    z: origin.z + zz,
                };
                break cursor;
            }
        };
        out.push(found);
    }
}

/// Vanilla `Mth.nextInt` (`Mth.java:145-147`).
pub(super) fn mth_next_int(
    random: &mut dyn RandomSource,
    min_inclusive: i32,
    max_inclusive: i32,
) -> i32 {
    if min_inclusive >= max_inclusive {
        min_inclusive
    } else {
        random.next_int(max_inclusive - min_inclusive + 1) + min_inclusive
    }
}
