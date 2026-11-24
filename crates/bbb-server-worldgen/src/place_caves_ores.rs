use super::*;

/// Vanilla `BlockTags.BASE_STONE_OVERWORLD` (`BlockTags.java:132`).
pub(super) const BASE_STONE_OVERWORLD_TAG: &str = "minecraft:base_stone_overworld";

/// Floor/ceiling pair from Vanilla `Column` (`Column.java:11-200`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorldGenColumn {
    pub(super) floor: Option<i32>,
    pub(super) ceiling: Option<i32>,
}

impl WorldGenColumn {
    pub(super) fn create(floor: Option<i32>, ceiling: Option<i32>) -> Option<Self> {
        if let (Some(floor), Some(ceiling)) = (floor, ceiling) {
            // Vanilla `Column.Range` rejects a negative height
            // (`Column.java:134-137`).
            if ceiling - floor - 1 < 0 {
                return None;
            }
        }
        Some(Self { floor, ceiling })
    }

    pub(super) fn height(self) -> Option<i32> {
        match (self.floor, self.ceiling) {
            (Some(floor), Some(ceiling)) => Some(ceiling - floor - 1),
            _ => None,
        }
    }

    pub(super) fn with_floor(self, floor: Option<i32>) -> Option<Self> {
        Self::create(floor, self.ceiling)
    }
}

/// Vanilla `Column.scan` (`Column.java:64-80`).
pub(super) fn scan_worldgen_column<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    search_range: i32,
    inside_column: impl Fn(i32) -> Option<bool>,
    valid_edge: impl Fn(i32) -> Option<bool>,
) -> Option<Option<WorldGenColumn>> {
    if !inside_column(level.get_block(pos))? {
        return Some(None);
    }
    let ceiling = scan_column_direction(level, search_range, &inside_column, &valid_edge, pos, 1)?;
    let floor = scan_column_direction(level, search_range, &inside_column, &valid_edge, pos, -1)?;
    Some(WorldGenColumn::create(floor, ceiling))
}

/// Vanilla `Column.scanDirection` (`Column.java:82-98`).
pub(super) fn scan_column_direction<L: WorldGenLevel>(
    level: &L,
    search_range: i32,
    inside_column: impl Fn(i32) -> Option<bool>,
    valid_edge: impl Fn(i32) -> Option<bool>,
    origin: BlockPos,
    dy: i32,
) -> Option<Option<i32>> {
    let mut y = origin.y;
    let mut i = 1;
    while i < search_range
        && inside_column(level.get_block(BlockPos {
            x: origin.x,
            y,
            z: origin.z,
        }))?
    {
        y += dy;
        i += 1;
    }
    let edge = level.get_block(BlockPos {
        x: origin.x,
        y,
        z: origin.z,
    });
    Some(valid_edge(edge)?.then_some(y))
}

/// Vanilla `DripstoneClusterFeature.place` (`DripstoneClusterFeature.java:27-51`).
pub(super) fn place_dripstone_cluster<L: WorldGenLevel>(
    config: &DripstoneClusterConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_empty_or_water(level.get_block(origin)) {
        return Some(false);
    }
    let height = config.height.sample(random);
    let wetness = sample_float_provider(&config.wetness, random);
    let density = sample_float_provider(&config.density, random);
    let x_radius = config.radius.sample(random);
    let z_radius = config.radius.sample(random);
    for dx in -x_radius..=x_radius {
        for dz in -z_radius..=z_radius {
            let chance = dripstone_cluster_column_chance(x_radius, z_radius, dx, dz, config);
            let pos = BlockPos {
                x: origin.x + dx,
                y: origin.y,
                z: origin.z + dz,
            };
            place_dripstone_cluster_column(
                level, random, pos, dx, dz, wetness, chance, height, density, config,
            )?;
        }
    }
    Some(true)
}

/// Vanilla `DripstoneClusterFeature.placeColumn` (`DripstoneClusterFeature.java:53-147`).
#[allow(clippy::too_many_arguments)]
pub(super) fn place_dripstone_cluster_column<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    dx: i32,
    dz: i32,
    chance_of_water: f32,
    chance_of_stalagmite_or_stalactite: f64,
    cluster_height: i32,
    density: f32,
    config: &DripstoneClusterConfig,
) -> Option<()> {
    let Some(base_column) = scan_worldgen_column(
        level,
        pos,
        config.floor_to_ceiling_search_range,
        |state| Some(is_empty_or_water(state)),
        |state| Some(is_neither_empty_nor_water(state)),
    )?
    else {
        return Some(());
    };
    if base_column.ceiling.is_none() && base_column.floor.is_none() {
        return Some(());
    }
    let want_pool = random.next_float() < chance_of_water;
    let pool_floor = match base_column.floor {
        Some(base_floor_y) if want_pool => dripstone_cluster_can_place_pool(
            level,
            BlockPos {
                x: pos.x,
                y: base_floor_y,
                z: pos.z,
            },
        ),
        _ => false,
    };
    let column = match base_column.floor {
        Some(base_floor_y) if pool_floor => {
            let water = vanilla_block_default_state_id(block_type_id("minecraft:water"))?;
            let _ = level.set_block(
                BlockPos {
                    x: pos.x,
                    y: base_floor_y,
                    z: pos.z,
                },
                water,
                UPDATE_CLIENTS,
            );
            base_column
                .with_floor(Some(base_floor_y - 1))
                .unwrap_or(base_column)
        }
        _ => base_column,
    };
    let floor = column.floor;
    let ceiling = column.ceiling;

    let want_stalactite = random.next_double() < chance_of_stalagmite_or_stalactite;
    let stalactite_height = if let Some(ceiling_y) = ceiling {
        if want_stalactite
            && !block_is(
                level.get_block(BlockPos {
                    x: pos.x,
                    y: ceiling_y,
                    z: pos.z,
                }),
                "minecraft:lava",
            )
        {
            let ceiling_thickness = config.dripstone_block_layer_thickness.sample(random);
            replace_blocks_with_dripstone_blocks(
                level,
                BlockPos {
                    x: pos.x,
                    y: ceiling_y,
                    z: pos.z,
                },
                ceiling_thickness,
                1,
            );
            let max_height_for_this_column = match floor {
                Some(floor_y) => cluster_height.min(ceiling_y - floor_y),
                None => cluster_height,
            };
            dripstone_cluster_height(random, dx, dz, density, max_height_for_this_column, config)
        } else {
            0
        }
    } else {
        0
    };

    let want_stalagmite = random.next_double() < chance_of_stalagmite_or_stalactite;
    let stalagmite_height = if let Some(floor_y) = floor {
        if want_stalagmite
            && !block_is(
                level.get_block(BlockPos {
                    x: pos.x,
                    y: floor_y,
                    z: pos.z,
                }),
                "minecraft:lava",
            )
        {
            let floor_thickness = config.dripstone_block_layer_thickness.sample(random);
            replace_blocks_with_dripstone_blocks(
                level,
                BlockPos {
                    x: pos.x,
                    y: floor_y,
                    z: pos.z,
                },
                floor_thickness,
                -1,
            );
            if ceiling.is_some() {
                (stalactite_height
                    + mth_random_between_inclusive(
                        random,
                        -config.max_stalagmite_stalactite_height_diff,
                        config.max_stalagmite_stalactite_height_diff,
                    ))
                .max(0)
            } else {
                dripstone_cluster_height(random, dx, dz, density, cluster_height, config)
            }
        } else {
            0
        }
    } else {
        0
    };

    let (actual_stalactite_height, actual_stalagmite_height) =
        if let (Some(ceiling_y), Some(floor_y)) = (ceiling, floor) {
            if ceiling_y - stalactite_height <= floor_y + stalagmite_height {
                let lowest_stalactite_bottom = (ceiling_y - stalactite_height).max(floor_y + 1);
                let highest_stalagmite_top = (floor_y + stalagmite_height).min(ceiling_y - 1);
                let actual_stalactite_bottom = mth_random_between_inclusive(
                    random,
                    lowest_stalactite_bottom,
                    highest_stalagmite_top + 1,
                );
                let actual_stalagmite_top = actual_stalactite_bottom - 1;
                (
                    ceiling_y - actual_stalactite_bottom,
                    actual_stalagmite_top - floor_y,
                )
            } else {
                (stalactite_height, stalagmite_height)
            }
        } else {
            (stalactite_height, stalagmite_height)
        };

    let merge_tips = random.next_boolean()
        && actual_stalactite_height > 0
        && actual_stalagmite_height > 0
        && column
            .height()
            .is_some_and(|height| actual_stalactite_height + actual_stalagmite_height == height);
    if let Some(ceiling_y) = ceiling {
        grow_pointed_dripstone(
            level,
            BlockPos {
                x: pos.x,
                y: ceiling_y - 1,
                z: pos.z,
            },
            -1,
            actual_stalactite_height,
            merge_tips,
        );
    }
    if let Some(floor_y) = floor {
        grow_pointed_dripstone(
            level,
            BlockPos {
                x: pos.x,
                y: floor_y + 1,
                z: pos.z,
            },
            1,
            actual_stalagmite_height,
            merge_tips,
        );
    }
    Some(())
}

/// Vanilla `DripstoneClusterFeature.getDripstoneHeight`
/// (`DripstoneClusterFeature.java:153-163`).
pub(super) fn dripstone_cluster_height(
    random: &mut dyn RandomSource,
    dx: i32,
    dz: i32,
    density: f32,
    max_height: i32,
    config: &DripstoneClusterConfig,
) -> i32 {
    if random.next_float() > density {
        return 0;
    }
    let distance_from_center = dx.abs() + dz.abs();
    let height_mean = mth_clamped_map_f64(
        f64::from(distance_from_center),
        0.0,
        f64::from(config.max_distance_from_center_affecting_height_bias),
        f64::from(max_height) / 2.0,
        0.0,
    ) as f32;
    clamped_normal_float_sample(
        random,
        height_mean,
        config.height_deviation as f32,
        0.0,
        max_height as f32,
    ) as i32
}

/// Vanilla `DripstoneClusterFeature.canPlacePool` (`DripstoneClusterFeature.java:165-182`).
pub(super) fn dripstone_cluster_can_place_pool<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let state = level.get_block(pos);
    if block_is(state, "minecraft:water")
        || block_is(state, "minecraft:dripstone_block")
        || block_is(state, "minecraft:pointed_dripstone")
    {
        return false;
    }
    let above = BlockPos {
        x: pos.x,
        y: pos.y + 1,
        z: pos.z,
    };
    if block_fluid_is_water(level.get_block(above)) {
        return false;
    }
    for (dx, dy, dz) in HORIZONTAL_OFFSETS {
        let neighbour = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        if !dripstone_cluster_can_be_adjacent_to_water(level, neighbour) {
            return false;
        }
    }
    dripstone_cluster_can_be_adjacent_to_water(
        level,
        BlockPos {
            x: pos.x,
            y: pos.y - 1,
            z: pos.z,
        },
    )
}

/// Vanilla `DripstoneClusterFeature.canBeAdjacentToWater`
/// (`DripstoneClusterFeature.java:184-187`).
pub(super) fn dripstone_cluster_can_be_adjacent_to_water<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
) -> bool {
    let state = level.get_block(pos);

    level.block_state_in_tag(state, BASE_STONE_OVERWORLD_TAG) || block_fluid_is_water(state)
}

/// Vanilla `DripstoneClusterFeature.replaceBlocksWithDripstoneBlocks`
/// (`DripstoneClusterFeature.java:189-199`).
pub(super) fn replace_blocks_with_dripstone_blocks<L: WorldGenLevel>(
    level: &mut L,
    first: BlockPos,
    max_count: i32,
    dy: i32,
) {
    let mut pos = first;
    for _ in 0..max_count {
        if !place_dripstone_block_if_possible(level, pos) {
            return;
        }
        pos.y += dy;
    }
}

/// Vanilla `DripstoneClusterFeature.getChanceOfStalagmiteOrStalactite`
/// (`DripstoneClusterFeature.java:201-210`).
pub(super) fn dripstone_cluster_column_chance(
    x_radius: i32,
    z_radius: i32,
    dx: i32,
    dz: i32,
    config: &DripstoneClusterConfig,
) -> f64 {
    let x_distance_from_edge = x_radius - dx.abs();
    let z_distance_from_edge = z_radius - dz.abs();
    let distance_from_edge = x_distance_from_edge.min(z_distance_from_edge);
    f64::from(mth_clamped_map_f32(
        distance_from_edge as f32,
        0.0,
        config.max_distance_from_edge_affecting_chance_of_dripstone_column as f32,
        config.chance_of_dripstone_column_at_max_distance_from_center,
        1.0,
    ))
}

/// Vanilla `ClampedNormalFloat.sample` (`ClampedNormalFloat.java:31-33`).
pub(super) fn clamped_normal_float_sample(
    random: &mut dyn RandomSource,
    mean: f32,
    deviation: f32,
    min: f32,
    max: f32,
) -> f32 {
    mth_clamp_f32(mean + random.next_gaussian() as f32 * deviation, min, max)
}

/// Vanilla `Mth.clampedMap(float)` (`Mth.java:640-642`).
pub(super) fn mth_clamped_map_f32(
    value: f32,
    from_min: f32,
    from_max: f32,
    to_min: f32,
    to_max: f32,
) -> f32 {
    mth_clamped_lerp_f32(
        mth_inverse_lerp_f32(value, from_min, from_max),
        to_min,
        to_max,
    )
}

/// Vanilla `Mth.clampedMap(double)` (`Mth.java:636-638`).
pub(super) fn mth_clamped_map_f64(
    value: f64,
    from_min: f64,
    from_max: f64,
    to_min: f64,
    to_max: f64,
) -> f64 {
    mth_clamped_lerp_f64(
        mth_inverse_lerp_f64(value, from_min, from_max),
        to_min,
        to_max,
    )
}

/// Vanilla `Mth.inverseLerp(float)` (`Mth.java:330-332`).
pub(super) fn mth_inverse_lerp_f32(value: f32, min: f32, max: f32) -> f32 {
    (value - min) / (max - min)
}

/// Vanilla `Mth.inverseLerp(double)` (`Mth.java:326-328`).
pub(super) fn mth_inverse_lerp_f64(value: f64, min: f64, max: f64) -> f64 {
    (value - min) / (max - min)
}

/// Vanilla `Mth.clampedLerp(float)` (`Mth.java:117-122`).
pub(super) fn mth_clamped_lerp_f32(factor: f32, min: f32, max: f32) -> f32 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

/// Vanilla `Mth.clampedLerp(double)` (`Mth.java:109-115`).
pub(super) fn mth_clamped_lerp_f64(factor: f64, min: f64, max: f64) -> f64 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

pub(super) struct LargeDripstone {
    pub(super) root: BlockPos,
    pub(super) pointing_up: bool,
    pub(super) radius: i32,
    pub(super) bluntness: f64,
    pub(super) scale: f64,
}

pub(super) struct WindOffsetter {
    pub(super) origin_y: i32,
    pub(super) wind_speed: Option<(f64, f64)>,
}

/// Vanilla `LargeDripstoneFeature.place` (`LargeDripstoneFeature.java:27-79`).
pub(super) fn place_large_dripstone<L: WorldGenLevel>(
    config: &LargeDripstoneConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_empty_or_water(level.get_block(origin)) {
        return Some(false);
    }
    let Some(column) = scan_worldgen_column(
        &*level,
        origin,
        config.floor_to_ceiling_search_range,
        |state| Some(is_empty_or_water(state)),
        |state| Some(is_dripstone_base_or_lava(&*level, state)),
    )?
    else {
        return Some(false);
    };
    let (Some(floor), Some(ceiling)) = (column.floor, column.ceiling) else {
        return Some(false);
    };
    let Some(column_height) = column.height() else {
        return Some(false);
    };
    if column_height < 4 {
        return Some(false);
    }
    let (radius_min, radius_max) = (
        config.column_radius.min_inclusive(),
        config.column_radius.max_inclusive(),
    );
    let max_column_radius_based_on_height =
        (column_height as f32 * config.max_column_radius_to_cave_height_ratio) as i32;
    let max_column_radius =
        mth_clamp_i32(max_column_radius_based_on_height, radius_min, radius_max);
    let radius = mth_random_between_inclusive(random, radius_min, max_column_radius);
    let mut stalactite = make_large_dripstone(
        BlockPos {
            x: origin.x,
            y: ceiling - 1,
            z: origin.z,
        },
        false,
        random,
        radius,
        &config.stalactite_bluntness,
        &config.height_scale,
    );
    let mut stalagmite = make_large_dripstone(
        BlockPos {
            x: origin.x,
            y: floor + 1,
            z: origin.z,
        },
        true,
        random,
        radius,
        &config.stalagmite_bluntness,
        &config.height_scale,
    );
    let wind = if stalactite.is_suitable_for_wind(config) && stalagmite.is_suitable_for_wind(config)
    {
        WindOffsetter::from_speed(origin.y, random, &config.wind_speed)
    } else {
        WindOffsetter::no_wind()
    };
    if large_dripstone_move_back_until_embedded(level, &mut stalactite, &wind) {
        large_dripstone_place_blocks(level, random, &stalactite, &wind);
    }
    if large_dripstone_move_back_until_embedded(level, &mut stalagmite, &wind) {
        large_dripstone_place_blocks(level, random, &stalagmite, &wind);
    }
    Some(true)
}

/// Vanilla `LargeDripstoneFeature.makeDripstone` (`LargeDripstoneFeature.java:81-90`).
pub(super) fn make_large_dripstone(
    root: BlockPos,
    pointing_up: bool,
    random: &mut dyn RandomSource,
    radius: i32,
    bluntness: &FloatProvider,
    height_scale: &FloatProvider,
) -> LargeDripstone {
    LargeDripstone {
        root,
        pointing_up,
        radius,
        bluntness: f64::from(sample_float_provider(bluntness, random)),
        scale: f64::from(sample_float_provider(height_scale, random)),
    }
}

impl LargeDripstone {
    pub(super) fn height(&self) -> i32 {
        self.height_at_radius(0.0)
    }

    pub(super) fn height_at_radius(&self, check_radius: f32) -> i32 {
        dripstone_utils_height(
            f64::from(check_radius),
            f64::from(self.radius),
            self.scale,
            self.bluntness,
        ) as i32
    }

    pub(super) fn is_suitable_for_wind(&self, config: &LargeDripstoneConfig) -> bool {
        self.radius >= config.min_radius_for_wind
            && self.bluntness >= f64::from(config.min_bluntness_for_wind)
    }
}

impl WindOffsetter {
    pub(super) fn from_speed(
        origin_y: i32,
        random: &mut dyn RandomSource,
        wind_speed: &FloatProvider,
    ) -> Self {
        let speed = sample_float_provider(wind_speed, random);
        let direction = mth_random_between_f32(random, 0.0, std::f32::consts::PI);
        Self {
            origin_y,
            wind_speed: Some((
                f64::from(mth_cos(f64::from(direction)) * speed),
                f64::from(mth_sin(f64::from(direction)) * speed),
            )),
        }
    }

    pub(super) fn no_wind() -> Self {
        Self {
            origin_y: 0,
            wind_speed: None,
        }
    }

    pub(super) fn offset(&self, pos: BlockPos) -> BlockPos {
        let Some((wx, wz)) = self.wind_speed else {
            return pos;
        };
        let dy = f64::from(self.origin_y - pos.y);
        BlockPos {
            x: pos.x + mth_floor_f64(wx * dy),
            y: pos.y,
            z: pos.z + mth_floor_f64(wz * dy),
        }
    }
}

/// Vanilla `DripstoneUtils.getDripstoneHeight` (`DripstoneUtils.java:16-29`).
pub(super) fn dripstone_utils_height(
    mut xz_distance_from_center: f64,
    dripstone_radius: f64,
    scale: f64,
    bluntness: f64,
) -> f64 {
    if xz_distance_from_center < bluntness {
        xz_distance_from_center = bluntness;
    }
    let r = xz_distance_from_center / dripstone_radius * 0.384;
    let part1 = 0.75 * r.powf(1.333_333_333_333_333_3);
    let part2 = r.powf(0.666_666_666_666_666_6);
    let part3 = 0.333_333_333_333_333_3 * r.ln();
    let height_relative = (scale * (part1 - part2 - part3)).max(0.0);
    height_relative / 0.384 * dripstone_radius
}

/// Vanilla `DripstoneUtils.isCircleMostlyEmbeddedInStone`
/// (`DripstoneUtils.java:31-48`).
pub(super) fn is_circle_mostly_embedded_in_stone<L: WorldGenLevel>(
    level: &L,
    center: BlockPos,
    xz_radius: i32,
) -> bool {
    if is_empty_or_water_or_lava(level.get_block(center)) {
        return false;
    }
    let angle_increment = 6.0 / xz_radius as f32;
    let mut angle = 0.0f32;
    while angle < (std::f64::consts::PI * 2.0) as f32 {
        let dx = (mth_cos(f64::from(angle)) * xz_radius as f32) as i32;
        let dz = (mth_sin(f64::from(angle)) * xz_radius as f32) as i32;
        if is_empty_or_water_or_lava(level.get_block(BlockPos {
            x: center.x + dx,
            y: center.y,
            z: center.z + dz,
        })) {
            return false;
        }
        angle += angle_increment;
    }
    true
}

/// Vanilla `LargeDripstone.moveBackUntilBaseIsInsideStoneAndShrinkRadiusIfNecessary`
/// (`LargeDripstoneFeature.java:131-153`).
pub(super) fn large_dripstone_move_back_until_embedded<L: WorldGenLevel>(
    level: &L,
    dripstone: &mut LargeDripstone,
    wind: &WindOffsetter,
) -> bool {
    while dripstone.radius > 1 {
        let mut new_root = dripstone.root;
        let max_tries = 10.min(dripstone.height());
        for _ in 0..max_tries {
            if block_is(level.get_block(new_root), "minecraft:lava") {
                return false;
            }
            if is_circle_mostly_embedded_in_stone(level, wind.offset(new_root), dripstone.radius) {
                dripstone.root = new_root;
                return true;
            }
            new_root.y += if dripstone.pointing_up { -1 } else { 1 };
        }
        dripstone.radius /= 2;
    }
    false
}

/// Vanilla `LargeDripstone.placeBlocks` (`LargeDripstoneFeature.java:159-190`).
pub(super) fn large_dripstone_place_blocks<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    dripstone: &LargeDripstone,
    wind: &WindOffsetter,
) {
    let dripstone_block = default_block_state("minecraft:dripstone_block");
    for dx in -dripstone.radius..=dripstone.radius {
        for dz in -dripstone.radius..=dripstone.radius {
            let current_radius = mth_sqrt_f32((dx * dx + dz * dz) as f32);
            if current_radius > dripstone.radius as f32 {
                continue;
            }
            let mut height = dripstone.height_at_radius(current_radius);
            if height <= 0 {
                continue;
            }
            if random.next_float() < 0.2 {
                height = (height as f32 * mth_random_between_f32(random, 0.8, 1.0)) as i32;
            }
            let mut pos = BlockPos {
                x: dripstone.root.x + dx,
                y: dripstone.root.y,
                z: dripstone.root.z + dz,
            };
            let mut has_been_out_of_stone = false;
            let max_y = if dripstone.pointing_up {
                level.get_world_surface_wg_height(pos.x, pos.z)
            } else {
                i32::MAX
            };
            for _ in 0..height {
                if pos.y >= max_y {
                    break;
                }
                let wind_adjusted = wind.offset(pos);
                let wind_state = level.get_block(wind_adjusted);
                if is_empty_or_water_or_lava(wind_state) {
                    has_been_out_of_stone = true;
                    let _ = level.set_block(wind_adjusted, dripstone_block, UPDATE_CLIENTS);
                } else if has_been_out_of_stone
                    && level.block_state_in_tag(wind_state, BASE_STONE_OVERWORLD_TAG)
                {
                    break;
                }
                pos.y += if dripstone.pointing_up { 1 } else { -1 };
            }
        }
    }
}

/// Vanilla `UnderwaterMagmaFeature.place` (`UnderwaterMagmaFeature.java:28-50`).
pub(super) fn place_underwater_magma<L: WorldGenLevel>(
    config: &UnderwaterMagmaConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let Some(floor_y) = underwater_magma_floor_y(level, origin, config.floor_search_range)? else {
        return Some(false);
    };
    let floor_pos = BlockPos {
        x: origin.x,
        y: floor_y,
        z: origin.z,
    };
    let radius = config.placement_radius_around_floor;
    let magma = default_block_state("minecraft:magma_block");
    let mut placed = false;
    for pos in between_closed(
        BlockPos {
            x: floor_pos.x - radius,
            y: floor_pos.y - radius,
            z: floor_pos.z - radius,
        },
        BlockPos {
            x: floor_pos.x + radius,
            y: floor_pos.y + radius,
            z: floor_pos.z + radius,
        },
    ) {
        if random.next_float() >= config.placement_probability_per_valid_position {
            continue;
        }
        if !underwater_magma_is_valid_placement(level, pos) {
            continue;
        }
        let _ = level.set_block(pos, magma, UPDATE_CLIENTS);
        placed = true;
    }
    Some(placed)
}

/// Vanilla `UnderwaterMagmaFeature.getFloorY` (`UnderwaterMagmaFeature.java:52-57`).
pub(super) fn underwater_magma_floor_y<L: WorldGenLevel>(
    level: &L,
    origin: BlockPos,
    floor_search_range: i32,
) -> Option<Option<i32>> {
    scan_worldgen_column(
        level,
        origin,
        floor_search_range,
        |state| Some(block_is(state, "minecraft:water")),
        |state| Some(!block_is(state, "minecraft:water")),
    )
    .map(|column| column.and_then(|column| column.floor))
}

/// Vanilla `UnderwaterMagmaFeature.isValidPlacement` (`UnderwaterMagmaFeature.java:59-71`).
pub(super) fn underwater_magma_is_valid_placement<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
) -> bool {
    if is_water_or_air(level.get_block(pos))
        || underwater_magma_visible_from_outside(
            level,
            BlockPos {
                x: pos.x,
                y: pos.y - 1,
                z: pos.z,
            },
            Direction::Up,
        )
    {
        return false;
    }
    for (face, (dx, dy, dz)) in HORIZONTAL_FACES.iter().zip(HORIZONTAL_OFFSETS) {
        let neighbour = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        if underwater_magma_visible_from_outside(level, neighbour, direction_opposite(*face)) {
            return false;
        }
    }
    true
}

/// Vanilla `UnderwaterMagmaFeature.isWaterOrAir` (`UnderwaterMagmaFeature.java:73-75`).
pub(super) fn is_water_or_air(state_id: i32) -> bool {
    is_empty_block(state_id) || block_is(state_id, "minecraft:water")
}

/// Vanilla `UnderwaterMagmaFeature.isVisibleFromOutside`
/// (`UnderwaterMagmaFeature.java:77-81`).
pub(super) fn underwater_magma_visible_from_outside<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    covered: Direction,
) -> bool {
    let state = level.get_block(pos);
    let Some(facts) = vanilla_block_state_support_facts(state) else {
        return true;
    };
    if facts.dynamic_shape() {
        return true;
    }
    !facts.occlusion_face_full(support_direction(covered))
}

pub(super) fn support_direction(face: Direction) -> VanillaBlockSupportDirection {
    match face {
        Direction::Down => VanillaBlockSupportDirection::Down,
        Direction::Up => VanillaBlockSupportDirection::Up,
        Direction::North => VanillaBlockSupportDirection::North,
        Direction::South => VanillaBlockSupportDirection::South,
        Direction::West => VanillaBlockSupportDirection::West,
        Direction::East => VanillaBlockSupportDirection::East,
    }
}

/// Vanilla `LakeFeature.place` (`LakeFeature.java:24-146`).
pub(super) fn place_lake<L: WorldGenLevel>(
    fluid_provider: &BlockStateProvider,
    barrier_provider: &BlockStateProvider,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if origin.y <= level.get_min_y() + 4 {
        return Some(false);
    }
    let origin = BlockPos {
        x: origin.x - 8,
        y: origin.y - 4,
        z: origin.z - 8,
    };
    let mut grid = vec![false; LAKE_GRID_LEN];
    let spots = random.next_int(4) + 4;
    for _ in 0..spots {
        let xr = random.next_double() * 6.0 + 3.0;
        let yr = random.next_double() * 4.0 + 2.0;
        let zr = random.next_double() * 6.0 + 3.0;
        let xp = random.next_double() * (16.0 - xr - 2.0) + 1.0 + xr / 2.0;
        let yp = random.next_double() * (8.0 - yr - 4.0) + 2.0 + yr / 2.0;
        let zp = random.next_double() * (16.0 - zr - 2.0) + 1.0 + zr / 2.0;
        for xx in 1..15 {
            for zz in 1..15 {
                for yy in 1..7 {
                    let xd = (f64::from(xx) - xp) / (xr / 2.0);
                    let yd = (f64::from(yy) - yp) / (yr / 2.0);
                    let zd = (f64::from(zz) - zp) / (zr / 2.0);
                    if xd * xd + yd * yd + zd * zd < 1.0 {
                        grid[lake_grid_index(xx, yy, zz)] = true;
                    }
                }
            }
        }
    }
    let fluid = fluid_provider.get_state(level, random, origin)?;
    for xx in 0..16 {
        for zz in 0..16 {
            for yy in 0..8 {
                if !lake_is_shell(&grid, xx, yy, zz) {
                    continue;
                }
                let pos = BlockPos {
                    x: origin.x + xx,
                    y: origin.y + yy,
                    z: origin.z + zz,
                };
                let block_state = level.get_block(pos);
                if yy >= 4 && block_state_is_liquid(block_state) {
                    return Some(false);
                }
                if yy < 4 && !block_state_is_solid(block_state) && block_state != fluid {
                    return Some(false);
                }
            }
        }
    }
    let cave_air = default_block_state("minecraft:cave_air");
    for xx in 0..16 {
        for zz in 0..16 {
            for yy in 0..8 {
                if !grid[lake_grid_index(xx, yy, zz)] {
                    continue;
                }
                let place_pos = BlockPos {
                    x: origin.x + xx,
                    y: origin.y + yy,
                    z: origin.z + zz,
                };
                if !lake_can_replace_block(level, level.get_block(place_pos)) {
                    continue;
                }
                let place_air = yy >= 4;
                let state = if place_air { cave_air } else { fluid };
                let _ = level.set_block(place_pos, state, UPDATE_CLIENTS);
                if place_air {
                    level.schedule_block_tick(place_pos, 0);
                    mark_above_for_post_processing(level, place_pos);
                }
            }
        }
    }
    let barrier = barrier_provider.get_state(level, random, origin)?;
    if !is_empty_block(barrier) {
        for xx in 0..16 {
            for zz in 0..16 {
                for yy in 0..8 {
                    if !lake_is_shell(&grid, xx, yy, zz) {
                        continue;
                    }
                    if yy >= 4 && random.next_int(2) == 0 {
                        continue;
                    }
                    let barrier_pos = BlockPos {
                        x: origin.x + xx,
                        y: origin.y + yy,
                        z: origin.z + zz,
                    };
                    let block_state = level.get_block(barrier_pos);
                    if block_state_is_solid(block_state)
                        && !level
                            .block_state_in_tag(block_state, LAVA_POOL_STONE_CANNOT_REPLACE_TAG)
                    {
                        let _ = level.set_block(barrier_pos, barrier, UPDATE_CLIENTS);
                        mark_above_for_post_processing(level, barrier_pos);
                    }
                }
            }
        }
    }
    if block_fluid_is_water(fluid) {
        let ice = default_block_state("minecraft:ice");
        for xx in 0..16 {
            for zz in 0..16 {
                let offset = BlockPos {
                    x: origin.x + xx,
                    y: origin.y + 4,
                    z: origin.z + zz,
                };
                if level.biome_should_freeze(offset)
                    && lake_can_replace_block(level, level.get_block(offset))
                {
                    let _ = level.set_block(offset, ice, UPDATE_CLIENTS);
                }
            }
        }
    }
    Some(true)
}

/// Vanilla `LakeFeature` occupancy index `(xx * 16 + zz) * 8 + yy`
/// (`LakeFeature.java:53`).
pub(super) fn lake_grid_index(xx: i32, yy: i32, zz: i32) -> usize {
    ((xx * 16 + zz) * 8 + yy) as usize
}

pub(super) fn lake_cell(grid: &[bool], xx: i32, yy: i32, zz: i32) -> bool {
    grid[lake_grid_index(xx, yy, zz)]
}

/// Shell of the lake occupancy grid (`LakeFeature.java:65-73`).
pub(super) fn lake_is_shell(grid: &[bool], xx: i32, yy: i32, zz: i32) -> bool {
    !lake_cell(grid, xx, yy, zz)
        && ((xx < 15 && lake_cell(grid, xx + 1, yy, zz))
            || (xx > 0 && lake_cell(grid, xx - 1, yy, zz))
            || (zz < 15 && lake_cell(grid, xx, yy, zz + 1))
            || (zz > 0 && lake_cell(grid, xx, yy, zz - 1))
            || (yy < 7 && lake_cell(grid, xx, yy + 1, zz))
            || (yy > 0 && lake_cell(grid, xx, yy - 1, zz)))
}

/// Vanilla `LakeFeature.canReplaceBlock` (`LakeFeature.java:148-150`).
pub(super) fn lake_can_replace_block<L: WorldGenLevel>(level: &L, state_id: i32) -> bool {
    !level.block_state_in_tag(state_id, FEATURES_CANNOT_REPLACE_TAG)
}

/// Vanilla `BlockBehaviour.BlockStateBase.liquid` (`BlockBehaviour.java:591-593`).
pub(super) fn block_state_is_liquid(state_id: i32) -> bool {
    vanilla_block_state_support_facts(state_id).is_some_and(|facts| facts.liquid())
}

/// Vanilla `OreFeature.place` (`OreFeature.java:24-53`).
///
/// Axis spread uses Java `Math.sin`/`Math.cos` (`OreFeature.java:31-34`);
/// ellipsoid radius along the blob uses `Mth.sin` (`OreFeature.java:83`).
pub(super) fn place_ore<L: WorldGenLevel>(
    config: &OreConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let (targets, size, discard) = (
        &config.targets,
        config.size,
        config.discard_chance_on_air_exposure,
    );
    let dir = random.next_float() * std::f32::consts::PI;
    let spread_xy = size as f32 / 8.0;
    let max_radius = mth_ceil_f32((size as f32 / 16.0 * 2.0 + 1.0) / 2.0);
    let sin_dir = f64::from(dir).sin();
    let cos_dir = f64::from(dir).cos();
    let spread_xy_d = f64::from(spread_xy);
    let x0 = f64::from(origin.x) + sin_dir * spread_xy_d;
    let x1 = f64::from(origin.x) - sin_dir * spread_xy_d;
    let z0 = f64::from(origin.z) + cos_dir * spread_xy_d;
    let z1 = f64::from(origin.z) - cos_dir * spread_xy_d;
    let y0 = f64::from(origin.y + random.next_int(3) - 2);
    let y1 = f64::from(origin.y + random.next_int(3) - 2);
    let x_start = origin.x - mth_ceil_f32(spread_xy) - max_radius;
    let y_start = origin.y - 2 - max_radius;
    let z_start = origin.z - mth_ceil_f32(spread_xy) - max_radius;
    let size_xz = 2 * (mth_ceil_f32(spread_xy) + max_radius);
    let size_y = 2 * (2 + max_radius);
    for xprobe in x_start..=x_start + size_xz {
        for zprobe in z_start..=z_start + size_xz {
            if y_start <= level.get_ocean_floor_wg_height(xprobe, zprobe) {
                return do_place_ore(
                    level, random, targets, discard, size, x0, x1, z0, z1, y0, y1, x_start,
                    y_start, z_start, size_xz, size_y,
                );
            }
        }
    }
    Some(false)
}

/// Vanilla `OreFeature.doPlace` (`OreFeature.java:55-166`).
///
/// Writes go through `LevelChunkSection.setBlockState(..., false)`
/// (`OreFeature.java:147`, `LevelChunkSection.java:64-70`) rather than
/// the normal WorldGenRegion notification path.
#[allow(clippy::too_many_arguments)]
pub(super) fn do_place_ore<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    targets: &[OreTarget],
    discard: f32,
    size: i32,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y0: f64,
    y1: f64,
    x_start: i32,
    y_start: i32,
    z_start: i32,
    size_xz: i32,
    size_y: i32,
) -> Option<bool> {
    let mut placed = 0;
    let mut tested = crate::fast_hash::FastHashSet::default();
    let mut data = vec![0.0f64; (size as usize).saturating_mul(4)];
    for i in 0..size {
        let step = i as f32 / size as f32;
        let ss = random.next_double() * f64::from(size) / 16.0;
        let pulse = mth_sin(f64::from(std::f32::consts::PI * step)) + 1.0f32;
        let r = (f64::from(pulse) * ss + 1.0) / 2.0;
        let base = (i as usize) * 4;
        data[base] = mth_lerp_f64(f64::from(step), x0, x1);
        data[base + 1] = mth_lerp_f64(f64::from(step), y0, y1);
        data[base + 2] = mth_lerp_f64(f64::from(step), z0, z1);
        data[base + 3] = r;
    }
    if size >= 2 {
        for i1 in 0..size - 1 {
            if data[(i1 as usize) * 4 + 3] <= 0.0 {
                continue;
            }
            for i2 in i1 + 1..size {
                if data[(i2 as usize) * 4 + 3] <= 0.0 {
                    continue;
                }
                let dx = data[(i1 as usize) * 4] - data[(i2 as usize) * 4];
                let dy = data[(i1 as usize) * 4 + 1] - data[(i2 as usize) * 4 + 1];
                let dz = data[(i1 as usize) * 4 + 2] - data[(i2 as usize) * 4 + 2];
                let dr = data[(i1 as usize) * 4 + 3] - data[(i2 as usize) * 4 + 3];
                if dr * dr > dx * dx + dy * dy + dz * dz {
                    if dr > 0.0 {
                        data[(i2 as usize) * 4 + 3] = -1.0;
                    } else {
                        data[(i1 as usize) * 4 + 3] = -1.0;
                    }
                }
            }
        }
    }
    for i in 0..size {
        let r = data[(i as usize) * 4 + 3];
        if r < 0.0 {
            continue;
        }
        let xx = data[(i as usize) * 4];
        let yy = data[(i as usize) * 4 + 1];
        let zz = data[(i as usize) * 4 + 2];
        let x_min = mth_floor_f64(xx - r).max(x_start);
        let y_min = mth_floor_f64(yy - r).max(y_start);
        let z_min = mth_floor_f64(zz - r).max(z_start);
        let x_max = mth_floor_f64(xx + r).max(x_min);
        let y_max = mth_floor_f64(yy + r).max(y_min);
        let z_max = mth_floor_f64(zz + r).max(z_min);
        for x in x_min..=x_max {
            let xd = (f64::from(x) + 0.5 - xx) / r;
            if xd * xd >= 1.0 {
                continue;
            }
            for y in y_min..=y_max {
                let yd = (f64::from(y) + 0.5 - yy) / r;
                if xd * xd + yd * yd >= 1.0 {
                    continue;
                }
                for z in z_min..=z_max {
                    let zd = (f64::from(z) + 0.5 - zz) / r;
                    let pos = BlockPos { x, y, z };
                    if xd * xd + yd * yd + zd * zd >= 1.0 || is_outside_build_height(level, pos) {
                        continue;
                    }
                    let bit = i64::from(x - x_start)
                        + i64::from(y - y_start) * i64::from(size_xz)
                        + i64::from(z - z_start) * i64::from(size_xz) * i64::from(size_y);
                    if !tested.insert(bit) {
                        continue;
                    }
                    if !level.ensure_can_write(pos) {
                        continue;
                    }
                    let current = level.get_block(pos);
                    for target in targets {
                        if can_place_ore(current, level, random, discard, &target.target, pos) {
                            if level.set_section_block(pos, target.state) {
                                placed += 1;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    Some(placed > 0)
}

/// Vanilla `ScatteredOreFeature.place` (`ScatteredOreFeature.java:18-38`).
pub(super) fn place_scattered_ore<L: WorldGenLevel>(
    config: &OreConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let (size, discard) = (config.size, config.discard_chance_on_air_exposure);
    let tries = random.next_int(size + 1);
    for i in 0..tries {
        let max_dist = i.min(SCATTERED_ORE_MAX_DIST_FROM_ORIGIN);
        let pos = BlockPos {
            x: origin.x + scattered_ore_axis_offset(random, max_dist),
            y: origin.y + scattered_ore_axis_offset(random, max_dist),
            z: origin.z + scattered_ore_axis_offset(random, max_dist),
        };
        let current = level.get_block(pos);
        for target in &config.targets {
            if can_place_ore(current, level, random, discard, &target.target, pos) {
                let _ = level.set_block(pos, target.state, UPDATE_CLIENTS);
                break;
            }
        }
    }
    Some(true)
}

/// Vanilla `ScatteredOreFeature.getRandomPlacementInOneAxisRelativeToOrigin`
/// (`ScatteredOreFeature.java:50-52`).
pub(super) fn scattered_ore_axis_offset(random: &mut dyn RandomSource, max_dist: i32) -> i32 {
    java_math_round((random.next_float() - random.next_float()) * max_dist as f32)
}

/// Vanilla `Math.round(float)` as used by `ScatteredOreFeature`
/// (`ScatteredOreFeature.java:51`): `(int) Math.floor(a + 0.5f)`.
pub(super) fn java_math_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

/// Vanilla `OreFeature.canPlaceOre` (`OreFeature.java:168-181`).
pub(super) fn can_place_ore<L: WorldGenLevel>(
    ore_pos_state: i32,
    level: &L,
    random: &mut dyn RandomSource,
    discard_chance_on_air_exposure: f32,
    target: &RuleTest,
    pos: BlockPos,
) -> bool {
    rule_test_matches(level, target, ore_pos_state, random)
        && (should_skip_air_check(random, discard_chance_on_air_exposure)
            || !is_adjacent_to_air(level, pos))
}

/// Vanilla `OreFeature.shouldSkipAirCheck` (`OreFeature.java:183-188`).
pub(super) fn should_skip_air_check(
    random: &mut dyn RandomSource,
    discard_chance_on_air_exposure: f32,
) -> bool {
    if discard_chance_on_air_exposure <= 0.0 {
        true
    } else if discard_chance_on_air_exposure >= 1.0 {
        false
    } else {
        random.next_float() >= discard_chance_on_air_exposure
    }
}

/// Vanilla `Feature.isAdjacentToAir` (`Feature.java:202-204`).
pub(super) fn is_adjacent_to_air<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    DIRECTION_OFFSETS.iter().any(|(dx, dy, dz)| {
        is_empty_block(level.get_block(BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        }))
    })
}

/// Vanilla `CoralFeature.place` (`CoralFeature.java:26-32`).
pub(super) fn place_coral_tree<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let Some(state) = random_coral_block_state(&*level, random)? else {
        return Some(false);
    };
    place_coral_tree_feature(level, random, origin, state)
}

/// Vanilla `CoralFeature.place` (`CoralFeature.java:26-32`).
pub(super) fn place_coral_mushroom<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let Some(state) = random_coral_block_state(&*level, random)? else {
        return Some(false);
    };
    place_coral_mushroom_feature(level, random, origin, state)
}

/// Vanilla `CoralFeature.place` (`CoralFeature.java:26-32`).
pub(super) fn place_coral_claw<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let Some(state) = random_coral_block_state(&*level, random)? else {
        return Some(false);
    };
    place_coral_claw_feature(level, random, origin, state)
}

pub(super) fn random_coral_block_state<L: WorldGenLevel>(
    level: &L,
    random: &mut dyn RandomSource,
) -> Option<Option<i32>> {
    let Some(name) = random_live_block_tag_element(level, CORAL_BLOCKS_TAG, random) else {
        return Some(None);
    };
    Some(Some(default_state_named(&name)?))
}

/// Vanilla `HolderGetter.getRandomElementOf` (`HolderGetter.java:21-23`) through
/// `HolderSet.getRandomElement` (`HolderSet.java:148-150`) and
/// `Util.getRandomSafe` (`Util.java:754-755`): an absent or empty tag draws nothing.
pub(super) fn random_live_block_tag_element<L: WorldGenLevel>(
    level: &L,
    tag: &str,
    random: &mut dyn RandomSource,
) -> Option<String> {
    let members = level.registry_tag_members("minecraft:block", tag);
    if members.is_empty() {
        return None;
    }
    let index = random.next_int(members.len() as i32) as usize;
    Some(members[index].to_string())
}

pub(super) fn default_state_named(block_name: &str) -> Option<i32> {
    let type_id = vanilla_block_type_for_resource_id(block_name).map(|entry| entry.id)?;
    vanilla_block_default_state_id(type_id)
}

/// Vanilla `CoralTreeFeature.placeFeature` (`CoralTreeFeature.java:18-51`).
pub(super) fn place_coral_tree_feature<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    state: i32,
) -> Option<bool> {
    let mut pos = origin;
    let trunk_height = random.next_int(3) + 1;
    for _ in 0..trunk_height {
        if !place_coral_block(level, random, pos, state)? {
            return Some(true);
        }
        pos.y += 1;
    }
    let trunk_top = pos;
    let n_branches = random.next_int(3) + 2;
    let directions = shuffled_horizontal(random);
    for &(dx, dy, dz) in directions.iter().take(n_branches as usize) {
        pos = BlockPos {
            x: trunk_top.x + dx,
            y: trunk_top.y + dy,
            z: trunk_top.z + dz,
        };
        let branch_height = random.next_int(5) + 2;
        let mut segment_length = 0;
        for j in 0..branch_height {
            if !place_coral_block(level, random, pos, state)? {
                break;
            }
            segment_length += 1;
            pos.y += 1;
            if j == 0 || (segment_length >= 2 && random.next_float() < 0.25) {
                pos.x += dx;
                pos.y += dy;
                pos.z += dz;
                segment_length = 0;
            }
        }
    }
    Some(true)
}

/// Vanilla `CoralMushroomFeature.placeFeature` (`CoralMushroomFeature.java:17-41`).
pub(super) fn place_coral_mushroom_feature<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    state: i32,
) -> Option<bool> {
    let height = random.next_int(3) + 3;
    let width = random.next_int(3) + 3;
    let length = random.next_int(3) + 3;
    let sink = random.next_int(3) + 1;
    for x in 0..=width {
        for y in 0..=height {
            for z in 0..=length {
                let on_x_edge = x == 0 || x == width;
                let on_y_edge = y == 0 || y == height;
                let on_z_edge = z == 0 || z == length;
                if (on_x_edge && on_y_edge)
                    || (on_z_edge && on_y_edge)
                    || (on_x_edge && on_z_edge)
                    || !(on_x_edge || on_y_edge || on_z_edge)
                    || random.next_float() < 0.1
                {
                    continue;
                }
                let pos = BlockPos {
                    x: origin.x + x,
                    y: origin.y + y - sink,
                    z: origin.z + z,
                };
                let _ = place_coral_block(level, random, pos, state)?;
            }
        }
    }
    Some(true)
}

/// Vanilla `CoralClawFeature.placeFeature` (`CoralClawFeature.java:20-67`).
pub(super) fn place_coral_claw_feature<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    state: i32,
) -> Option<bool> {
    if !place_coral_block(level, random, origin, state)? {
        return Some(false);
    }
    let claw = shuffled_horizontal_source(random);
    let n_branches = random.next_int(2) + 2;
    let mut possible = [
        claw,
        horizontal_clock_wise(claw),
        horizontal_counter_clock_wise(claw),
    ];
    util_shuffle(&mut possible, random);
    for &(bdx, bdy, bdz) in possible.iter().take(n_branches as usize) {
        let mut pos = BlockPos {
            x: origin.x + bdx,
            y: origin.y + bdy,
            z: origin.z + bdz,
        };
        let sideway_length = random.next_int(2) + 1;
        let (segment, inway_length) = if (bdx, bdy, bdz) == claw {
            (claw, random.next_int(3) + 2)
        } else {
            pos.y += 1;
            let up = (0, 1, 0);
            let choices = [(bdx, bdy, bdz), up];
            (choices[random.next_int(2) as usize], random.next_int(3) + 3)
        };
        let mut i = 0;
        while i < sideway_length && place_coral_block(level, random, pos, state)? {
            pos.x += segment.0;
            pos.y += segment.1;
            pos.z += segment.2;
            i += 1;
        }
        pos.x -= segment.0;
        pos.y -= segment.1;
        pos.z -= segment.2;
        pos.y += 1;
        for _ in 0..inway_length {
            pos.x += claw.0;
            pos.y += claw.1;
            pos.z += claw.2;
            if !place_coral_block(level, random, pos, state)? {
                break;
            }
            if random.next_float() < 0.25 {
                pos.y += 1;
            }
        }
    }
    Some(true)
}

pub(super) fn shuffled_horizontal_source(random: &mut dyn RandomSource) -> (i32, i32, i32) {
    HORIZONTAL_OFFSETS[random.next_int(4) as usize]
}

pub(super) fn shuffled_horizontal(random: &mut dyn RandomSource) -> [(i32, i32, i32); 4] {
    let mut directions = HORIZONTAL_OFFSETS;
    util_shuffle(&mut directions, random);
    directions
}

/// Vanilla `Direction.getClockWise` (`Direction.java:195-201`) for HORIZONTAL.
pub(super) fn horizontal_clock_wise((dx, dy, dz): (i32, i32, i32)) -> (i32, i32, i32) {
    (-dz, dy, dx)
}

/// Vanilla `Direction.getCounterClockWise` (`Direction.java:245-251`) for HORIZONTAL.
pub(super) fn horizontal_counter_clock_wise((dx, dy, dz): (i32, i32, i32)) -> (i32, i32, i32) {
    (dz, dy, -dx)
}

/// Vanilla `Util.shuffle` (`Util.java:1026-1032`).
pub(super) fn util_shuffle<T>(list: &mut [T], random: &mut dyn RandomSource) {
    let mut i = list.len();
    while i > 1 {
        let swap_to = random.next_int(i as i32) as usize;
        i -= 1;
        list.swap(i, swap_to);
    }
}

/// Vanilla `CoralFeature.placeCoralBlock` (`CoralFeature.java:36-70`).
pub(super) fn place_coral_block<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    state: i32,
) -> Option<bool> {
    let above = BlockPos {
        x: pos.x,
        y: pos.y + 1,
        z: pos.z,
    };
    let target = level.get_block(pos);
    if !(block_is(target, "minecraft:water") || level.block_state_in_tag(target, CORALS_TAG))
        || !block_is(level.get_block(above), "minecraft:water")
    {
        return Some(false);
    }
    let _ = level.set_block(pos, state, FEATURE_SET_BLOCK);
    if random.next_float() < 0.25 {
        if let Some(coral) = random_live_block_tag_element(level, CORALS_TAG, random) {
            let _ = level.set_block(above, default_state_named(&coral)?, UPDATE_CLIENTS);
        }
    } else if random.next_float() < 0.05 {
        let _ = level.set_block(
            above,
            sea_pickle_state(random.next_int(4) + 1),
            UPDATE_CLIENTS,
        );
    }
    for &(dx, dy, dz) in &HORIZONTAL_OFFSETS {
        if random.next_float() >= 0.2 {
            continue;
        }
        let relative = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        if !block_is(level.get_block(relative), "minecraft:water") {
            continue;
        }
        if let Some(coral) = random_live_block_tag_element(level, WALL_CORALS_TAG, random) {
            let _ = level.set_block(
                relative,
                wall_coral_state(&coral, horizontal_facing_name(dx, dz))?,
                UPDATE_CLIENTS,
            );
        }
    }
    Some(true)
}

pub(super) fn sea_pickle_state(pickles: i32) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("pickles".to_string(), pickles.to_string());
    properties.insert("waterlogged".to_string(), "true".to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:sea_pickle", &properties)
        .expect("vanilla block state")
}

pub(super) fn wall_coral_state(block_name: &str, facing: &str) -> Option<i32> {
    let mut properties = BTreeMap::new();
    properties.insert("facing".to_string(), facing.to_string());
    properties.insert("waterlogged".to_string(), "true".to_string());
    vanilla_block_state_id_by_name_and_properties(block_name, &properties)
}

pub(super) fn horizontal_facing_name(dx: i32, dz: i32) -> &'static str {
    match (dx, dz) {
        (0, -1) => "north",
        (1, 0) => "east",
        (0, 1) => "south",
        (-1, 0) => "west",
        _ => unreachable!("HORIZONTAL facing"),
    }
}

/// Vanilla `SpringFeature.place` (`SpringFeature.java:15-81`).
pub(super) fn place_spring_feature<L: WorldGenLevel>(
    config: &SpringConfig,
    level: &mut L,
    origin: BlockPos,
) -> Option<bool> {
    let fluid_block = config.fluid_block?;
    let valid_blocks = &config.valid_blocks;
    let above = BlockPos {
        x: origin.x,
        y: origin.y + 1,
        z: origin.z,
    };
    let below = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    if !block_in_holder_set(level, level.get_block(above), valid_blocks) {
        return Some(false);
    }
    if config.requires_block_below
        && !block_in_holder_set(level, level.get_block(below), valid_blocks)
    {
        return Some(false);
    }
    let current = level.get_block(origin);
    if !is_empty_block(current) && !block_in_holder_set(level, current, valid_blocks) {
        return Some(false);
    }
    let mut rocks = 0;
    let mut holes = 0;
    for (dx, dy, dz) in SPRING_NEIGHBOR_OFFSETS {
        let neighbour = BlockPos {
            x: origin.x + dx,
            y: origin.y + dy,
            z: origin.z + dz,
        };
        let state = level.get_block(neighbour);
        if block_in_holder_set(level, state, valid_blocks) {
            rocks += 1;
        }
        if is_empty_block(state) {
            holes += 1;
        }
    }
    if rocks == config.rock_count && holes == config.hole_count {
        let _ = level.set_block(origin, fluid_block, UPDATE_CLIENTS);
        level.schedule_fluid_tick(origin, 0);
        return Some(true);
    }
    Some(false)
}

pub(super) fn block_in_holder_set<L: WorldGenLevel>(
    level: &L,
    state: i32,
    blocks: &HolderSet,
) -> bool {
    match blocks {
        HolderSet::Tag(tag) => level.block_state_in_tag(state, tag),
        HolderSet::Direct(members) => members.iter().any(|member| block_is(state, member)),
    }
}

/// Vanilla `BlockBlobFeature.place` (`BlockBlobFeature.java:15-45`).
pub(super) fn place_block_blob<L: WorldGenLevel>(
    config: &BlockBlobConfig,
    level: &mut L,
    mut origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let BlockBlobConfig {
        state,
        ref can_place_on,
    } = *config;
    let min_place_y = level.get_min_y() + 3;
    while origin.y > min_place_y
        && !block_predicates::block_predicate_test(
            can_place_on,
            level,
            BlockPos {
                x: origin.x,
                y: origin.y - 1,
                z: origin.z,
            },
        )?
    {
        origin.y -= 1;
    }
    if origin.y <= min_place_y {
        return Some(false);
    }
    for _ in 0..3 {
        let xr = random.next_int(2);
        let yr = random.next_int(2);
        let zr = random.next_int(2);
        let tr = (xr + yr + zr) as f32 * 0.333 + 0.5;
        let limit = tr * tr;
        let min = BlockPos {
            x: origin.x - xr,
            y: origin.y - yr,
            z: origin.z - zr,
        };
        let max = BlockPos {
            x: origin.x + xr,
            y: origin.y + yr,
            z: origin.z + zr,
        };
        for pos in between_closed(min, max) {
            if dist_to_low_corner_sqr(pos, origin) <= f64::from(limit) {
                let _ = level.set_block(pos, state, FEATURE_SET_BLOCK);
            }
        }
        origin.x += -1 + random.next_int(2);
        origin.y -= random.next_int(2);
        origin.z += -1 + random.next_int(2);
    }
    Some(true)
}

pub(super) fn offset_pos(origin: BlockPos, (dx, dy, dz): (i32, i32, i32)) -> BlockPos {
    BlockPos {
        x: origin.x + dx,
        y: origin.y + dy,
        z: origin.z + dz,
    }
}

/// Vanilla `BlockPos.betweenClosed` (`BlockPos.java:405-426`).
pub(super) fn between_closed(a: BlockPos, b: BlockPos) -> impl Iterator<Item = BlockPos> {
    let min_x = a.x.min(b.x);
    let min_y = a.y.min(b.y);
    let min_z = a.z.min(b.z);
    let width = a.x.max(b.x) - min_x + 1;
    let height = a.y.max(b.y) - min_y + 1;
    let depth = a.z.max(b.z) - min_z + 1;
    let end = width * height * depth;
    (0..end).map(move |index| {
        let x = index % width;
        let slice = index / width;
        let y = slice % height;
        let z = slice / height;
        BlockPos {
            x: min_x + x,
            y: min_y + y,
            z: min_z + z,
        }
    })
}

/// Vanilla `Vec3i.distToLowCornerSqr` (`Vec3i.java:216-220`).
pub(super) fn dist_to_low_corner_sqr(a: BlockPos, b: BlockPos) -> f64 {
    let dx = f64::from(a.x - b.x);
    let dy = f64::from(a.y - b.y);
    let dz = f64::from(a.z - b.z);
    dx * dx + dy * dy + dz * dz
}

/// Vanilla `SpikeFeature.place` (`SpikeFeature.java:17-99`).
pub(super) fn place_spike<L: WorldGenLevel>(
    config: &SpikeConfig,
    level: &mut L,
    mut origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let SpikeConfig {
        state,
        ref can_place_on,
        ref can_replace,
    } = *config;
    while is_empty_block(level.get_block(origin)) && origin.y > level.get_min_y() + 2 {
        origin.y -= 1;
    }
    if !block_predicates::block_predicate_test(can_place_on, level, origin)? {
        return Some(false);
    }
    origin.y += random.next_int(4);
    let height = random.next_int(4) + 7;
    let width = height / 4 + random.next_int(2);
    if width > 1 && random.next_int(60) == 0 {
        origin.y += 10 + random.next_int(30);
    }
    for y_off in 0..height {
        let scale = (1.0 - y_off as f32 / height as f32) * width as f32;
        let new_width = mth_ceil_f32(scale);
        for xo in -new_width..=new_width {
            let dx = xo.abs() as f32 - 0.25;
            for zo in -new_width..=new_width {
                let dz = zo.abs() as f32 - 0.25;
                if (xo != 0 || zo != 0) && dx * dx + dz * dz > scale * scale {
                    continue;
                }
                let on_edge =
                    xo == -new_width || xo == new_width || zo == -new_width || zo == new_width;
                if on_edge && random.next_float() > 0.75 {
                    continue;
                }
                spike_try_place(
                    level,
                    BlockPos {
                        x: origin.x + xo,
                        y: origin.y + y_off,
                        z: origin.z + zo,
                    },
                    state,
                    can_replace,
                )?;
                if y_off != 0 && new_width > 1 {
                    spike_try_place(
                        level,
                        BlockPos {
                            x: origin.x + xo,
                            y: origin.y - y_off,
                            z: origin.z + zo,
                        },
                        state,
                        can_replace,
                    )?;
                }
            }
        }
    }
    let pillar_width = width.clamp(1, 2) - 1;
    for xo in -pillar_width..=pillar_width {
        for zo in -pillar_width..=pillar_width {
            let mut cursor = BlockPos {
                x: origin.x + xo,
                y: origin.y - 1,
                z: origin.z + zo,
            };
            let mut run_length = if xo.abs() == 1 && zo.abs() == 1 {
                random.next_int(5)
            } else {
                50
            };
            while cursor.y > SPIKE_PILLAR_MIN_Y {
                let current = level.get_block(cursor);
                if !is_empty_block(current)
                    && !block_predicates::block_predicate_test(can_replace, level, cursor)?
                    && current != state
                {
                    break;
                }
                let _ = level.set_block(cursor, state, FEATURE_SET_BLOCK);
                cursor.y -= 1;
                run_length -= 1;
                if run_length <= 0 {
                    cursor.y -= random.next_int(5) + 1;
                    run_length = random.next_int(5);
                }
            }
        }
    }
    Some(true)
}

pub(super) fn spike_try_place<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    state: i32,
    can_replace: &BlockPredicate,
) -> Option<()> {
    let current = level.get_block(pos);
    if is_empty_block(current) || block_predicates::block_predicate_test(can_replace, level, pos)? {
        let _ = level.set_block(pos, state, FEATURE_SET_BLOCK);
    }
    Some(())
}
