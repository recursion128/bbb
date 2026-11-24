use super::*;

/// Vanilla `NoOpFeature.place` (`NoOpFeature.java:12-14`) returns true.
pub(super) fn place_no_op<L: WorldGenLevel>(level: &mut L, origin: BlockPos) -> Option<bool> {
    let _ = (level, origin);
    Some(true)
}

/// Vanilla `VoidStartPlatformFeature.place` (`VoidStartPlatformFeature.java:26-49`).
pub(super) fn place_void_start_platform<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
) -> Option<bool> {
    // `ChunkPos.containing` (`ChunkPos.java:48-50`) plus
    // `SectionPos.blockToSectionCoord` (`SectionPos.java:80-82`).
    let chunk_x = origin.x >> 4;
    let chunk_z = origin.z >> 4;
    // `PLATFORM_ORIGIN_CHUNK = ChunkPos.containing(PLATFORM_OFFSET)` is (0, 0)
    // (`VoidStartPlatformFeature.java:12`).
    if checkerboard_distance(chunk_x, chunk_z, 0, 0) > 1 {
        return Some(true);
    }

    let stone = default_block_state("minecraft:stone");
    let cobblestone = default_block_state("minecraft:cobblestone");
    // `PLATFORM_OFFSET.atY(origin.y + PLATFORM_OFFSET.y)` (`BlockPos.java:236-238`).
    let platform_origin = BlockPos {
        x: VOID_PLATFORM_OFFSET.x,
        y: origin.y + VOID_PLATFORM_OFFSET.y,
        z: VOID_PLATFORM_OFFSET.z,
    };
    let min_x = chunk_x << 4;
    let min_z = chunk_z << 4;
    let max_x = min_x + 15;
    let max_z = min_z + 15;
    let mut pos = BlockPos {
        x: 0,
        y: platform_origin.y,
        z: 0,
    };
    for z in min_z..=max_z {
        for x in min_x..=max_x {
            if checkerboard_distance(platform_origin.x, platform_origin.z, x, z)
                <= VOID_PLATFORM_RADIUS
            {
                pos.x = x;
                pos.z = z;
                let state = if pos == platform_origin {
                    cobblestone
                } else {
                    stone
                };
                let _ = level.set_block(pos, state, UPDATE_CLIENTS);
            }
        }
    }
    Some(true)
}

/// Vanilla `EndPlatformFeature.place` (`EndPlatformFeature.java:17-19`) calls
/// `createEndPlatform(level, origin, false)` (`EndPlatformFeature.java:21-39`).
pub(super) fn place_end_platform<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
) -> Option<bool> {
    create_end_platform(level, origin, false);
    Some(true)
}

// EndPlatformFeature.java:21-39.
pub fn create_end_platform<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    drop_resources: bool,
) {
    let obsidian_block = block_type_id("minecraft:obsidian");
    let air_block = block_type_id("minecraft:air");
    let obsidian = catalog_default_state("minecraft:obsidian");
    let air = catalog_default_state("minecraft:air");
    for dz in -2..=2 {
        for dx in -2..=2 {
            for dy in -1..3 {
                let pos = BlockPos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    z: origin.z + dz,
                };
                let (want_block, want_state) = if dy == -1 {
                    (obsidian_block, obsidian)
                } else {
                    (air_block, air)
                };
                if vanilla_block_state_block_type_id(level.get_block(pos)) != Some(want_block) {
                    if drop_resources {
                        level.destroy_block_for_end_platform(pos);
                    }
                    let _ = level.set_block(pos, want_state, UPDATE_NEIGHBORS | UPDATE_CLIENTS);
                }
            }
        }
    }
}

/// Vanilla `EndPodiumFeature.place` (`EndPodiumFeature.java:29-75`): the
/// exit portal frame, lit with `minecraft:end_portal` when `active`.
pub fn place_end_podium<L: WorldGenLevel>(level: &mut L, origin: BlockPos, active: bool) {
    let flags = UPDATE_NEIGHBORS | UPDATE_CLIENTS;
    let bedrock = catalog_default_state("minecraft:bedrock");
    let place = |level: &mut L, pos: BlockPos, name: &'static str, drop: bool| {
        let state = catalog_default_state(name);
        if drop {
            if vanilla_block_state_block_type_id(level.get_block(pos)) == Some(block_type_id(name))
            {
                return;
            }
            level.destroy_block_for_end_platform(pos);
        }
        let _ = level.set_block(pos, state, flags);
    };
    for y in origin.y - 1..=origin.y + 32 {
        for z in origin.z - 4..=origin.z + 4 {
            for x in origin.x - 4..=origin.x + 4 {
                let pos = BlockPos { x, y, z };
                let (dx, dy, dz) = (x - origin.x, y - origin.y, z - origin.z);
                let distance_sqr = f64::from(dx * dx + dy * dy + dz * dz);
                let inside_rim = distance_sqr < 2.5 * 2.5;
                if !inside_rim && distance_sqr >= 3.5 * 3.5 {
                    continue;
                }
                if y < origin.y {
                    if inside_rim {
                        let _ = level.set_block(pos, bedrock, flags);
                    } else {
                        place(level, pos, "minecraft:end_stone", active);
                    }
                } else if y > origin.y {
                    place(level, pos, "minecraft:air", active);
                } else if !inside_rim {
                    let _ = level.set_block(pos, bedrock, flags);
                } else if active {
                    place(level, pos, "minecraft:end_portal", true);
                } else {
                    place(level, pos, "minecraft:air", false);
                }
            }
        }
    }
    for y in 0..4 {
        let _ = level.set_block(
            BlockPos {
                y: origin.y + y,
                ..origin
            },
            bedrock,
            flags,
        );
    }
    let torch = catalog_default_state("minecraft:wall_torch");
    for (facing, dx, dz) in [
        ("north", 0, -1),
        ("south", 0, 1),
        ("west", -1, 0),
        ("east", 1, 0),
    ] {
        let pos = BlockPos {
            x: origin.x + dx,
            y: origin.y + 2,
            z: origin.z + dz,
        };
        let _ = level.set_block(pos, block_set_property(torch, "facing", facing), flags);
    }
}

/// Vanilla `DesertWellFeature.place` (`DesertWellFeature.java:29-107`).
pub(super) fn place_desert_well<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let sand = catalog_default_state("minecraft:sand");
    let sand_slab = catalog_default_state("minecraft:sandstone_slab");
    let sandstone = catalog_default_state("minecraft:sandstone");
    let water = catalog_default_state("minecraft:water");
    let suspicious_sand = catalog_default_state("minecraft:suspicious_sand");
    let mut origin = BlockPos {
        x: origin.x,
        y: origin.y + 1,
        z: origin.z,
    };
    while is_empty_block(level.get_block(origin)) && origin.y > level.get_min_y() + 2 {
        origin.y -= 1;
    }
    if !block_is(level.get_block(origin), "minecraft:sand") {
        return Some(false);
    }
    for ox in -2..=2 {
        for oz in -2..=2 {
            if is_empty_block(level.get_block(BlockPos {
                x: origin.x + ox,
                y: origin.y - 1,
                z: origin.z + oz,
            })) && is_empty_block(level.get_block(BlockPos {
                x: origin.x + ox,
                y: origin.y - 2,
                z: origin.z + oz,
            })) {
                return Some(false);
            }
        }
    }
    for oy in -2..=0 {
        for ox in -2..=2 {
            for oz in -2..=2 {
                let _ = level.set_block(
                    BlockPos {
                        x: origin.x + ox,
                        y: origin.y + oy,
                        z: origin.z + oz,
                    },
                    sandstone,
                    UPDATE_CLIENTS,
                );
            }
        }
    }
    let _ = level.set_block(origin, water, UPDATE_CLIENTS);
    for &(dx, _, dz) in &HORIZONTAL_OFFSETS {
        let _ = level.set_block(
            BlockPos {
                x: origin.x + dx,
                y: origin.y,
                z: origin.z + dz,
            },
            water,
            UPDATE_CLIENTS,
        );
    }
    let sand_center = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    let _ = level.set_block(sand_center, sand, UPDATE_CLIENTS);
    for &(dx, _, dz) in &HORIZONTAL_OFFSETS {
        let _ = level.set_block(
            BlockPos {
                x: sand_center.x + dx,
                y: sand_center.y,
                z: sand_center.z + dz,
            },
            sand,
            UPDATE_CLIENTS,
        );
    }
    for ox in -2..=2 {
        for oz in -2..=2 {
            if ox == -2 || ox == 2 || oz == -2 || oz == 2 {
                let _ = level.set_block(
                    BlockPos {
                        x: origin.x + ox,
                        y: origin.y + 1,
                        z: origin.z + oz,
                    },
                    sandstone,
                    UPDATE_CLIENTS,
                );
            }
        }
    }
    let _ = level.set_block(
        BlockPos {
            x: origin.x + 2,
            y: origin.y + 1,
            z: origin.z,
        },
        sand_slab,
        UPDATE_CLIENTS,
    );
    let _ = level.set_block(
        BlockPos {
            x: origin.x - 2,
            y: origin.y + 1,
            z: origin.z,
        },
        sand_slab,
        UPDATE_CLIENTS,
    );
    let _ = level.set_block(
        BlockPos {
            x: origin.x,
            y: origin.y + 1,
            z: origin.z + 2,
        },
        sand_slab,
        UPDATE_CLIENTS,
    );
    let _ = level.set_block(
        BlockPos {
            x: origin.x,
            y: origin.y + 1,
            z: origin.z - 2,
        },
        sand_slab,
        UPDATE_CLIENTS,
    );
    for ox in -1..=1 {
        for oz in -1..=1 {
            let state = if ox == 0 && oz == 0 {
                sandstone
            } else {
                sand_slab
            };
            let _ = level.set_block(
                BlockPos {
                    x: origin.x + ox,
                    y: origin.y + 4,
                    z: origin.z + oz,
                },
                state,
                UPDATE_CLIENTS,
            );
        }
    }
    for oy in 1..=3 {
        for &(ox, oz) in &[(-1, -1), (-1, 1), (1, -1), (1, 1)] {
            let _ = level.set_block(
                BlockPos {
                    x: origin.x + ox,
                    y: origin.y + oy,
                    z: origin.z + oz,
                },
                sandstone,
                UPDATE_CLIENTS,
            );
        }
    }
    let first = desert_well_random_water_pos(origin, random);
    place_desert_well_suspicious_sand(
        level,
        BlockPos {
            x: first.x,
            y: first.y - 1,
            z: first.z,
        },
        suspicious_sand,
    );
    let second = desert_well_random_water_pos(origin, random);
    place_desert_well_suspicious_sand(
        level,
        BlockPos {
            x: second.x,
            y: second.y - 2,
            z: second.z,
        },
        suspicious_sand,
    );
    Some(true)
}

/// Vanilla `Util.getRandom` over the well's five water cells
/// (`Util.java:750-752`, `DesertWellFeature.java:104-105`).
pub(super) fn desert_well_random_water_pos(
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> BlockPos {
    let (dx, dz) = DESERT_WELL_WATER_OFFSETS[random.next_int(5) as usize];
    BlockPos {
        x: origin.x + dx,
        y: origin.y,
        z: origin.z + dz,
    }
}

/// Vanilla `DesertWellFeature.placeSusSand` (`DesertWellFeature.java:109-112`).
pub(super) fn place_desert_well_suspicious_sand<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    suspicious_sand: i32,
) {
    let _ = level.set_block(pos, suspicious_sand, FEATURE_SET_BLOCK);
    level.set_brushable_block_loot_table(pos, DESERT_WELL_ARCHAEOLOGY_LOOT, block_pos_as_long(pos));
}

/// Vanilla `BlockPos.asLong` (`BlockPos.java:107-116`).
pub(super) fn block_pos_as_long(pos: BlockPos) -> i64 {
    let x = i64::from(pos.x) & ((1_i64 << BLOCK_POS_PACKED_HORIZONTAL_LENGTH) - 1);
    let y = i64::from(pos.y) & ((1_i64 << BLOCK_POS_PACKED_Y_LENGTH) - 1);
    let z = i64::from(pos.z) & ((1_i64 << BLOCK_POS_PACKED_HORIZONTAL_LENGTH) - 1);
    (x << BLOCK_POS_X_OFFSET) | y | (z << BLOCK_POS_Z_OFFSET)
}

pub(super) fn block_type_id(block_name: &'static str) -> i32 {
    vanilla_block_type_for_resource_id(block_name)
        .expect("vanilla block")
        .id
}

/// Vanilla `VoidStartPlatformFeature.checkerboardDistance`
/// (`VoidStartPlatformFeature.java:20-22`).
pub(super) fn checkerboard_distance(xa: i32, za: i32, xb: i32, zb: i32) -> i32 {
    (xa - xb).abs().max((za - zb).abs())
}

pub(super) fn default_block_state(block_name: &'static str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(block_name, &BTreeMap::new())
        .expect("vanilla block")
}

pub(super) fn catalog_default_state(block_name: &'static str) -> i32 {
    vanilla_block_default_state_id(block_type_id(block_name)).expect("vanilla block")
}

/// Vanilla `FillLayerFeature.place` (`FillLayerFeature.java:15-33`).
pub(super) fn place_fill_layer<L: WorldGenLevel>(
    height: i32,
    state: i32,
    level: &mut L,
    origin: BlockPos,
) -> Option<bool> {
    let y = level.get_min_y() + height;
    for dx in 0..16 {
        for dz in 0..16 {
            let pos = BlockPos {
                x: origin.x + dx,
                y,
                z: origin.z + dz,
            };
            if bbb_server_types::fluid::block_state_is_air(level.get_block(pos)) {
                let _ = level.set_block(pos, state, UPDATE_CLIENTS);
            }
        }
    }
    Some(true)
}

/// Vanilla `GlowstoneFeature.place` (`GlowstoneFeature.java:19-55`).
pub(super) fn place_glowstone_blob<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !bbb_server_types::fluid::block_state_is_air(level.get_block(origin)) {
        return Some(false);
    }
    let above = BlockPos {
        x: origin.x,
        y: origin.y + 1,
        z: origin.z,
    };
    let above_state = level.get_block(above);
    if !block_is(above_state, "minecraft:netherrack")
        && !block_is(above_state, "minecraft:basalt")
        && !block_is(above_state, "minecraft:blackstone")
    {
        return Some(false);
    }
    let glowstone = default_block_state("minecraft:glowstone");
    let _ = level.set_block(origin, glowstone, UPDATE_CLIENTS);
    for _ in 0..1500 {
        let place_pos = BlockPos {
            x: origin.x + random.next_int(8) - random.next_int(8),
            y: origin.y - random.next_int(12),
            z: origin.z + random.next_int(8) - random.next_int(8),
        };
        if bbb_server_types::fluid::block_state_is_air(level.get_block(place_pos)) {
            let mut neighbours = 0;
            for (dx, dy, dz) in DIRECTION_OFFSETS {
                let neighbour = BlockPos {
                    x: place_pos.x + dx,
                    y: place_pos.y + dy,
                    z: place_pos.z + dz,
                };
                if block_is(level.get_block(neighbour), "minecraft:glowstone") {
                    neighbours += 1;
                }
                if neighbours > 1 {
                    break;
                }
            }
            if neighbours == 1 {
                let _ = level.set_block(place_pos, glowstone, UPDATE_CLIENTS);
            }
        }
    }
    Some(true)
}

pub(super) fn block_is(state_id: i32, block_name: &str) -> bool {
    vanilla_block_state_block_type_id(state_id)
        .and_then(vanilla_block_type)
        .is_some_and(|block| block.resource_id == block_name)
}

/// Vanilla `EndIslandFeature.place` (`EndIslandFeature.java:18-36`).
pub(super) fn place_end_island<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let end_stone = default_block_state("minecraft:end_stone");
    let mut size = random.next_int(3) as f32 + 4.0;
    let mut y = 0i32;
    while size > 0.5 {
        let x_min = mth_floor_f32(-size);
        let x_max = mth_ceil_f32(size);
        let radius = (size + 1.0) * (size + 1.0);
        for x in x_min..=x_max {
            for z in x_min..=x_max {
                if (x * x + z * z) as f32 <= radius {
                    let pos = BlockPos {
                        x: origin.x + x,
                        y: origin.y + y,
                        z: origin.z + z,
                    };
                    // `Feature.setBlock` (`Feature.java:169-171`) uses flags 3.
                    let _ = level.set_block(pos, end_stone, UPDATE_NEIGHBORS | UPDATE_CLIENTS);
                }
            }
        }
        y -= 1;
        size -= random.next_int(2) as f32 + 0.5;
    }
    Some(true)
}

/// Vanilla `Mth.floor(float)` (`Mth.java:61-63`).
pub(super) fn mth_floor_f32(value: f32) -> i32 {
    value.floor() as i32
}

/// Vanilla `Mth.ceil(float)` (`Mth.java:81-83`).
pub(super) fn mth_ceil_f32(value: f32) -> i32 {
    value.ceil() as i32
}

/// Vanilla `Mth.clamp(float)` (`Mth.java:101-103`).
pub(super) fn mth_clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else {
        value.min(max)
    }
}

/// Vanilla `Mth.sqrt(float)` (`Mth.java:57-59`).
pub(super) fn mth_sqrt_f32(value: f32) -> f32 {
    value.sqrt()
}

/// Vanilla `Mth.floor(double)` (`Mth.java:65-67`).
pub(super) fn mth_floor_f64(value: f64) -> i32 {
    value.floor() as i32
}

/// Vanilla `Mth.lerp(double, double, double)` (`Mth.java:540-542`).
pub(super) fn mth_lerp_f64(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

/// `Mth.sin` (`Mth.java:49-51`) and its table (`Mth.java:34-38`).
pub(super) fn mth_sin(radians: f64) -> f32 {
    worldgen_sin_table()[((radians * WORLDGEN_SIN_TABLE_SCALE) as i64 & 0xFFFF) as usize]
}

/// `Mth.cos` (`Mth.java:53-55`): the same table a quarter turn along.
pub(super) fn mth_cos(radians: f64) -> f32 {
    worldgen_sin_table()[((radians * WORLDGEN_SIN_TABLE_SCALE + 16384.0) as i64 & 0xFFFF) as usize]
}

pub(super) const WORLDGEN_SIN_TABLE_SCALE: f64 = 10430.378350470453;

pub(super) fn worldgen_sin_table() -> &'static [f32; 65536] {
    static TABLE: std::sync::OnceLock<Box<[f32; 65536]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Box::new([0.0f32; 65536]);
        for (index, entry) in table.iter_mut().enumerate() {
            *entry = (index as f64 / WORLDGEN_SIN_TABLE_SCALE).sin() as f32;
        }
        table
    })
}

/// Vanilla `EndGatewayFeature.place` (`EndGatewayFeature.java:17-46`).
pub(super) fn place_end_gateway<L: WorldGenLevel>(
    config: &EndGatewayConfig,
    level: &mut L,
    origin: BlockPos,
) -> Option<bool> {
    let EndGatewayConfig { exit, exact } = *config;
    let gateway = default_block_state("minecraft:end_gateway");
    let air = default_block_state("minecraft:air");
    let bedrock = default_block_state("minecraft:bedrock");
    let min_x = origin.x - 1;
    let min_y = origin.y - 2;
    let min_z = origin.z - 1;
    let width = 3;
    let height = 5;
    let depth = 3;
    let count = width * height * depth;
    for index in 0..count {
        // `BlockPos.betweenClosed` (`BlockPos.java:405-427`): x fastest, then y, then z.
        let x = index % width;
        let slice = index / width;
        let y = slice % height;
        let z = slice / height;
        let pos = BlockPos {
            x: min_x + x,
            y: min_y + y,
            z: min_z + z,
        };
        let same_x = pos.x == origin.x;
        let same_y = pos.y == origin.y;
        let same_z = pos.z == origin.z;
        let end = (pos.y - origin.y).abs() == 2;
        let state = if same_x && same_y && same_z {
            gateway
        } else if same_y {
            air
        } else if (end && same_x && same_z) || ((same_x || same_z) && !end) {
            bedrock
        } else {
            air
        };
        let _ = level.set_block(pos, state, UPDATE_NEIGHBORS | UPDATE_CLIENTS);
        if same_x && same_y && same_z {
            if let Some(exit) = exit {
                // Vanilla only writes when `getBlockEntity` is `TheEndGatewayBlockEntity`
                // (`EndGatewayFeature.java:29-33`).
                level.set_end_gateway_exit_position(pos, exit, exact);
            }
        }
    }
    Some(true)
}

/// Vanilla `WeepingVinesFeature.place` (`WeepingVinesFeature.java:23-39`).
pub(super) fn place_weeping_vines<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if !is_empty_block(level.get_block(origin)) {
        return Some(false);
    }
    let above = BlockPos {
        x: origin.x,
        y: origin.y + 1,
        z: origin.z,
    };
    if !is_nether_roof_support(level.get_block(above)) {
        return Some(false);
    }
    place_roof_nether_wart(level, random, origin);
    place_roof_weeping_vines(level, random, origin);
    Some(true)
}

/// Vanilla `WeepingVinesFeature.placeRoofNetherWart` (`WeepingVinesFeature.java:41-67`).
pub(super) fn place_roof_nether_wart<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
) {
    let nether_wart = default_block_state("minecraft:nether_wart_block");
    let _ = level.set_block(origin, nether_wart, UPDATE_CLIENTS);
    for _ in 0..200 {
        let place_pos = BlockPos {
            x: origin.x + random.next_int(6) - random.next_int(6),
            y: origin.y + random.next_int(2) - random.next_int(5),
            z: origin.z + random.next_int(6) - random.next_int(6),
        };
        if is_empty_block(level.get_block(place_pos)) {
            let mut neighbours = 0;
            for (dx, dy, dz) in DIRECTION_OFFSETS {
                let neighbour = BlockPos {
                    x: place_pos.x + dx,
                    y: place_pos.y + dy,
                    z: place_pos.z + dz,
                };
                if is_nether_roof_support(level.get_block(neighbour)) {
                    neighbours += 1;
                }
                if neighbours > 1 {
                    break;
                }
            }
            if neighbours == 1 {
                let _ = level.set_block(place_pos, nether_wart, UPDATE_CLIENTS);
            }
        }
    }
}

/// Vanilla `WeepingVinesFeature.placeRoofWeepingVines` (`WeepingVinesFeature.java:69-92`).
pub(super) fn place_roof_weeping_vines<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
) {
    let plant = default_block_state("minecraft:weeping_vines_plant");
    for _ in 0..100 {
        let mut place_pos = BlockPos {
            x: origin.x + random.next_int(8) - random.next_int(8),
            y: origin.y + random.next_int(2) - random.next_int(7),
            z: origin.z + random.next_int(8) - random.next_int(8),
        };
        if is_empty_block(level.get_block(place_pos)) {
            let above = BlockPos {
                x: place_pos.x,
                y: place_pos.y + 1,
                z: place_pos.z,
            };
            if is_nether_roof_support(level.get_block(above)) {
                let mut vine_height = mth_next_int(random, 1, 8);
                if random.next_int(6) == 0 {
                    vine_height *= 2;
                }
                if random.next_int(5) == 0 {
                    vine_height = 1;
                }
                place_weeping_vines_column(
                    level,
                    random,
                    &mut place_pos,
                    vine_height,
                    plant,
                    17,
                    25,
                );
            }
        }
    }
}

/// Vanilla `WeepingVinesFeature.placeWeepingVinesColumn`
/// (`WeepingVinesFeature.java:94-109`).
pub(super) fn place_weeping_vines_column<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    place_pos: &mut BlockPos,
    total_height: i32,
    plant: i32,
    min_age: i32,
    max_age: i32,
) {
    for height in 0..=total_height {
        if is_empty_block(level.get_block(*place_pos)) {
            let below = BlockPos {
                x: place_pos.x,
                y: place_pos.y - 1,
                z: place_pos.z,
            };
            if height == total_height || !is_empty_block(level.get_block(below)) {
                let age = mth_next_int(random, min_age, max_age);
                let head = weeping_vines_head_state(age);
                let _ = level.set_block(*place_pos, head, UPDATE_CLIENTS);
                break;
            }
            let _ = level.set_block(*place_pos, plant, UPDATE_CLIENTS);
        }
        place_pos.y -= 1;
    }
}

/// Vanilla `LevelReader.isEmptyBlock` (`LevelReader.java:84-86`).
pub(super) fn is_empty_block(state_id: i32) -> bool {
    bbb_server_types::fluid::block_state_is_air(state_id)
}

pub(super) fn is_nether_roof_support(state_id: i32) -> bool {
    block_is(state_id, "minecraft:netherrack") || block_is(state_id, "minecraft:nether_wart_block")
}

pub(super) fn weeping_vines_head_state(age: i32) -> i32 {
    growing_plant_head_state("minecraft:weeping_vines", age)
}

pub(super) fn growing_plant_head_state(block_name: &'static str, age: i32) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("age".to_string(), age.to_string());
    vanilla_block_state_id_by_name_and_properties(block_name, &properties)
        .expect("vanilla block state")
}

/// Vanilla `LevelHeightAccessor.isOutsideBuildHeight` (`LevelHeightAccessor.java:39-40`).
pub(super) fn is_outside_build_height<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    pos.y < level.get_min_y() || pos.y > level.get_max_y()
}

/// Vanilla `TwistingVinesFeature.place` (`TwistingVinesFeature.java:21-57`).
pub(super) fn place_twisting_vines<L: WorldGenLevel>(
    spread_width: i32,
    spread_height: i32,
    max_height: i32,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if is_invalid_twisting_placement(level, origin) {
        return Some(false);
    }
    let plant = default_block_state("minecraft:twisting_vines_plant");
    let attempts = spread_width.checked_mul(spread_width)?;
    for _ in 0..attempts {
        let mut place_pos = BlockPos {
            x: origin.x + mth_next_int(random, -spread_width, spread_width),
            y: origin.y + mth_next_int(random, -spread_height, spread_height),
            z: origin.z + mth_next_int(random, -spread_width, spread_width),
        };
        if find_first_air_block_above_ground(level, &mut place_pos)
            && !is_invalid_twisting_placement(level, place_pos)
        {
            let mut vine_height = mth_next_int(random, 1, max_height);
            if random.next_int(6) == 0 {
                vine_height *= 2;
            }
            if random.next_int(5) == 0 {
                vine_height = 1;
            }
            place_twisting_vines_column(level, random, &mut place_pos, vine_height, plant);
        }
    }
    Some(true)
}

/// Vanilla `TwistingVinesFeature.findFirstAirBlockAboveGround`
/// (`TwistingVinesFeature.java:59-69`).
pub(super) fn find_first_air_block_above_ground<L: WorldGenLevel>(
    level: &L,
    place_pos: &mut BlockPos,
) -> bool {
    loop {
        place_pos.y -= 1;
        if is_outside_build_height(level, *place_pos) {
            return false;
        }
        if !is_empty_block(level.get_block(*place_pos)) {
            break;
        }
    }
    place_pos.y += 1;
    true
}

/// Vanilla `TwistingVinesFeature.placeWeepingVinesColumn`
/// (`TwistingVinesFeature.java:71-86`).
pub(super) fn place_twisting_vines_column<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    place_pos: &mut BlockPos,
    total_height: i32,
    plant: i32,
) {
    // `TwistingVinesFeature.java:52` passes AGE bounds 17 and 25.
    const MIN_AGE: i32 = 17;
    const MAX_AGE: i32 = 25;
    for height in 1..=total_height {
        if is_empty_block(level.get_block(*place_pos)) {
            let above = BlockPos {
                x: place_pos.x,
                y: place_pos.y + 1,
                z: place_pos.z,
            };
            if height == total_height || !is_empty_block(level.get_block(above)) {
                let age = mth_next_int(random, MIN_AGE, MAX_AGE);
                let head = growing_plant_head_state("minecraft:twisting_vines", age);
                let _ = level.set_block(*place_pos, head, UPDATE_CLIENTS);
                break;
            }
            let _ = level.set_block(*place_pos, plant, UPDATE_CLIENTS);
        }
        place_pos.y += 1;
    }
}

/// Vanilla `TwistingVinesFeature.isInvalidPlacementLocation`
/// (`TwistingVinesFeature.java:88-95`).
pub(super) fn is_invalid_twisting_placement<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    if !is_empty_block(level.get_block(pos)) {
        return true;
    }
    let below = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    let below_state = level.get_block(below);
    !block_is(below_state, "minecraft:netherrack")
        && !block_is(below_state, "minecraft:warped_nylium")
        && !block_is(below_state, "minecraft:warped_wart_block")
}

/// Vanilla `BasaltPillarFeature.place` (`BasaltPillarFeature.java:19-77`).
pub(super) fn place_basalt_pillar<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let above = BlockPos {
        x: origin.x,
        y: origin.y + 1,
        z: origin.z,
    };
    if !is_empty_block(level.get_block(origin)) || is_empty_block(level.get_block(above)) {
        return Some(false);
    }
    let basalt = vanilla_block_default_state_id(block_type_id("minecraft:basalt"))?;
    let mut pos = origin;
    let mut hangoff = [true; 4];
    while is_empty_block(level.get_block(pos)) {
        if is_outside_build_height(level, pos) {
            return Some(true);
        }
        let _ = level.set_block(pos, basalt, UPDATE_CLIENTS);
        for (index, (dx, dy, dz)) in CARDINAL_OFFSETS.iter().enumerate() {
            if hangoff[index] {
                let hang = BlockPos {
                    x: pos.x + dx,
                    y: pos.y + dy,
                    z: pos.z + dz,
                };
                hangoff[index] = place_basalt_hangoff(level, random, hang, basalt);
            }
        }
        pos.y -= 1;
    }
    pos.y += 1;
    for (dx, dy, dz) in CARDINAL_OFFSETS {
        let hang = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        place_basalt_base_hangoff(level, random, hang, basalt);
    }
    pos.y -= 1;
    for dx in -3i32..4 {
        for dz in -3i32..4 {
            let probability = dx.abs() * dz.abs();
            if random.next_int(10) < 10 - probability {
                let mut base = BlockPos {
                    x: pos.x + dx,
                    y: pos.y,
                    z: pos.z + dz,
                };
                let mut max_drop = 3;
                loop {
                    let below = BlockPos {
                        x: base.x,
                        y: base.y - 1,
                        z: base.z,
                    };
                    if !is_empty_block(level.get_block(below)) {
                        break;
                    }
                    base.y -= 1;
                    max_drop -= 1;
                    if max_drop <= 0 {
                        break;
                    }
                }
                let below = BlockPos {
                    x: base.x,
                    y: base.y - 1,
                    z: base.z,
                };
                if !is_empty_block(level.get_block(below)) {
                    let _ = level.set_block(base, basalt, UPDATE_CLIENTS);
                }
            }
        }
    }
    Some(true)
}

/// Vanilla `BasaltPillarFeature.placeHangOff` (`BasaltPillarFeature.java:85-92`).
pub(super) fn place_basalt_hangoff<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    hang_off_pos: BlockPos,
    basalt: i32,
) -> bool {
    if random.next_int(10) != 0 {
        let _ = level.set_block(hang_off_pos, basalt, UPDATE_CLIENTS);
        true
    } else {
        false
    }
}

/// Vanilla `BasaltPillarFeature.placeBaseHangOff` (`BasaltPillarFeature.java:79-83`).
pub(super) fn place_basalt_base_hangoff<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    basalt: i32,
) {
    if random.next_boolean() {
        let _ = level.set_block(pos, basalt, UPDATE_CLIENTS);
    }
}

/// Vanilla `BasaltColumnsFeature.place` (`BasaltColumnsFeature.java:39-65`).
///
/// Vanilla reads lava sea level from `context.chunkGenerator().getSeaLevel()`
/// (`BasaltColumnsFeature.java:40`). This owner uses the existing
/// `WorldGenLevel.get_sea_level` seam.
pub(super) fn place_basalt_columns<L: WorldGenLevel>(
    reach_provider: &IntProvider,
    height_provider: &IntProvider,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let lava_sea_level = level.get_sea_level();
    if !basalt_columns_can_place_at(level, lava_sea_level, origin) {
        return Some(false);
    }
    let column_height = height_provider.sample(random);
    let clustered = random.next_float() < 0.9;
    let scan_reach = column_height.min(if clustered {
        BASALT_COLUMNS_CLUSTERED_REACH
    } else {
        BASALT_COLUMNS_UNCLUSTERED_REACH
    });
    let count = if clustered {
        BASALT_COLUMNS_CLUSTERED_SIZE
    } else {
        BASALT_COLUMNS_UNCLUSTERED_SIZE
    };
    let mut placed = false;
    let width = scan_reach * 2 + 1;
    // `randomBetweenClosed` draws lazily before each reach sample (`BlockPos.java:274-294`).
    for _ in 0..count {
        let pos = BlockPos {
            x: origin.x - scan_reach + random.next_int(width),
            y: origin.y + random.next_int(1),
            z: origin.z - scan_reach + random.next_int(width),
        };
        let blocks_to_place_y = column_height - dist_manhattan(pos, origin);
        if blocks_to_place_y >= 0 {
            let column_reach = reach_provider.sample(random);
            placed |=
                place_basalt_column(level, lava_sea_level, pos, blocks_to_place_y, column_reach)?;
        }
    }
    Some(placed)
}

/// Vanilla `BasaltColumnsFeature.placeColumn` (`BasaltColumnsFeature.java:67-100`).
pub(super) fn place_basalt_column<L: WorldGenLevel>(
    level: &mut L,
    lava_sea_level: i32,
    origin: BlockPos,
    column_height: i32,
    reach: i32,
) -> Option<bool> {
    let basalt = vanilla_block_default_state_id(block_type_id("minecraft:basalt"))?;
    let mut placed_any = false;
    for pos in between_closed(
        BlockPos {
            x: origin.x - reach,
            y: origin.y,
            z: origin.z - reach,
        },
        BlockPos {
            x: origin.x + reach,
            y: origin.y,
            z: origin.z + reach,
        },
    ) {
        let step_limit = dist_manhattan(pos, origin);
        let column_pos = if is_air_or_lava_ocean(level, lava_sea_level, pos) {
            find_basalt_column_surface(level, lava_sea_level, pos, step_limit)
        } else {
            find_basalt_column_air(level, pos, step_limit)
        };
        let Some(column_pos) = column_pos else {
            continue;
        };
        let mut blocks_y = column_height - step_limit / 2;
        let mut cursor = column_pos;
        while blocks_y >= 0 {
            if is_air_or_lava_ocean(level, lava_sea_level, cursor) {
                let _ = level.set_block(cursor, basalt, FEATURE_SET_BLOCK);
                cursor.y += 1;
                placed_any = true;
            } else if block_is(level.get_block(cursor), "minecraft:basalt") {
                cursor.y += 1;
            } else {
                break;
            }
            blocks_y -= 1;
        }
    }
    Some(placed_any)
}

/// Vanilla `BasaltColumnsFeature.findSurface` (`BasaltColumnsFeature.java:102-113`).
pub(super) fn find_basalt_column_surface<L: WorldGenLevel>(
    level: &L,
    lava_sea_level: i32,
    mut cursor: BlockPos,
    mut limit: i32,
) -> Option<BlockPos> {
    while cursor.y > level.get_min_y() + 1 && limit > 0 {
        limit -= 1;
        if basalt_columns_can_place_at(level, lava_sea_level, cursor) {
            return Some(cursor);
        }
        cursor.y -= 1;
    }
    None
}

/// Vanilla `BasaltColumnsFeature.canPlaceAt` (`BasaltColumnsFeature.java:115-123`).
pub(super) fn basalt_columns_can_place_at<L: WorldGenLevel>(
    level: &L,
    lava_sea_level: i32,
    pos: BlockPos,
) -> bool {
    if !is_air_or_lava_ocean(level, lava_sea_level, pos) {
        return false;
    }
    let below = BlockPos {
        x: pos.x,
        y: pos.y - 1,
        z: pos.z,
    };
    let below_state = level.get_block(below);
    !is_empty_block(below_state) && !basalt_columns_cannot_place_on(below_state)
}

/// Vanilla `BasaltColumnsFeature.findAir` (`BasaltColumnsFeature.java:125-141`).
pub(super) fn find_basalt_column_air<L: WorldGenLevel>(
    level: &L,
    mut cursor: BlockPos,
    mut limit: i32,
) -> Option<BlockPos> {
    while cursor.y <= level.get_max_y() && limit > 0 {
        limit -= 1;
        let state = level.get_block(cursor);
        if basalt_columns_cannot_place_on(state) {
            return None;
        }
        if is_empty_block(state) {
            return Some(cursor);
        }
        cursor.y += 1;
    }
    None
}

/// Vanilla `BasaltColumnsFeature.isAirOrLavaOcean` (`BasaltColumnsFeature.java:143-146`).
pub(super) fn is_air_or_lava_ocean<L: WorldGenLevel>(
    level: &L,
    lava_sea_level: i32,
    pos: BlockPos,
) -> bool {
    let state = level.get_block(pos);
    is_empty_block(state) || (block_is(state, "minecraft:lava") && pos.y <= lava_sea_level)
}

pub(super) fn basalt_columns_cannot_place_on(state: i32) -> bool {
    BASALT_COLUMNS_CANNOT_PLACE_ON
        .iter()
        .any(|block_name| block_is(state, block_name))
}

/// Vanilla `BlueIceFeature.place` (`BlueIceFeature.java:18-68`).
pub(super) fn place_blue_ice<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    if origin.y > level.get_sea_level() - 1 {
        return Some(false);
    }
    let below = BlockPos {
        x: origin.x,
        y: origin.y - 1,
        z: origin.z,
    };
    if !block_is(level.get_block(origin), "minecraft:water")
        && !block_is(level.get_block(below), "minecraft:water")
    {
        return Some(false);
    }
    let mut found_packed_ice = false;
    for (dx, dy, dz) in DIRECTION_OFFSETS {
        if (dx, dy, dz) == (0, -1, 0) {
            continue;
        }
        let neighbour = BlockPos {
            x: origin.x + dx,
            y: origin.y + dy,
            z: origin.z + dz,
        };
        if block_is(level.get_block(neighbour), "minecraft:packed_ice") {
            found_packed_ice = true;
            break;
        }
    }
    if !found_packed_ice {
        return Some(false);
    }
    let blue_ice = default_block_state("minecraft:blue_ice");
    let _ = level.set_block(origin, blue_ice, UPDATE_CLIENTS);
    for _ in 0..200 {
        let y_off = random.next_int(5) - random.next_int(6);
        let mut xz_diff = 3;
        if y_off < 2 {
            xz_diff += y_off / 2;
        }
        if xz_diff >= 1 {
            let place_pos = BlockPos {
                x: origin.x + random.next_int(xz_diff) - random.next_int(xz_diff),
                y: origin.y + y_off,
                z: origin.z + random.next_int(xz_diff) - random.next_int(xz_diff),
            };
            let place_state = level.get_block(place_pos);
            if is_empty_block(place_state)
                || block_is(place_state, "minecraft:water")
                || block_is(place_state, "minecraft:packed_ice")
                || block_is(place_state, "minecraft:ice")
            {
                for (dx, dy, dz) in DIRECTION_OFFSETS {
                    let relative = BlockPos {
                        x: place_pos.x + dx,
                        y: place_pos.y + dy,
                        z: place_pos.z + dz,
                    };
                    if block_is(level.get_block(relative), "minecraft:blue_ice") {
                        let _ = level.set_block(place_pos, blue_ice, UPDATE_CLIENTS);
                        break;
                    }
                }
            }
        }
    }
    Some(true)
}

/// Vanilla `IcebergFeature.place` (`IcebergFeature.java:20-78`).
///
/// Vanilla rewrites origin Y from `context.chunkGenerator().getSeaLevel()`
/// (`IcebergFeature.java:23`). This owner uses the existing
/// `WorldGenLevel.get_sea_level` seam.
pub(super) fn place_iceberg<L: WorldGenLevel>(
    main_block: i32,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let origin = BlockPos {
        x: origin.x,
        y: level.get_sea_level(),
        z: origin.z,
    };
    let snow_on_top = random.next_double() > 0.7;
    let shape_angle = random.next_double() * 2.0 * std::f64::consts::PI;
    let shape_ellipse_a = 11 - random.next_int(5);
    let shape_ellipse_c = 3 + random.next_int(3);
    let is_ellipse = random.next_double() > 0.7;
    let mut over_water_height = if is_ellipse {
        random.next_int(6) + 6
    } else {
        random.next_int(15) + 3
    };
    if !is_ellipse && random.next_double() > 0.9 {
        over_water_height += random.next_int(19) + 7;
    }
    let under_water_height = (over_water_height + random.next_int(11)).min(18);
    let width = (over_water_height + random.next_int(7) - random.next_int(5)).min(11);
    let a = if is_ellipse { shape_ellipse_a } else { 11 };

    for xo in -a..a {
        for zo in -a..a {
            for y_off in 0..over_water_height {
                let radius = if is_ellipse {
                    iceberg_height_dependent_radius_ellipse(y_off, over_water_height, width)
                } else {
                    iceberg_height_dependent_radius_round(random, y_off, over_water_height, width)
                };
                if is_ellipse || xo < radius {
                    generate_iceberg_block(
                        level,
                        random,
                        origin,
                        over_water_height,
                        xo,
                        y_off,
                        zo,
                        radius,
                        a,
                        is_ellipse,
                        shape_ellipse_c,
                        shape_angle,
                        snow_on_top,
                        main_block,
                    )?;
                }
            }
        }
    }

    iceberg_smooth(
        level,
        origin,
        width,
        over_water_height,
        is_ellipse,
        shape_ellipse_a,
    )?;

    for xo in -a..a {
        for zo in -a..a {
            let mut y_off = -1;
            while y_off > -under_water_height {
                let new_a = if is_ellipse {
                    mth_ceil_f32(
                        a as f32
                            * (1.0
                                - (y_off as f64).powi(2) as f32
                                    / (under_water_height as f32 * 8.0)),
                    )
                } else {
                    a
                };
                let radius = iceberg_height_dependent_radius_steep(
                    random,
                    -y_off,
                    under_water_height,
                    width,
                );
                if xo < radius {
                    generate_iceberg_block(
                        level,
                        random,
                        origin,
                        under_water_height,
                        xo,
                        y_off,
                        zo,
                        radius,
                        new_a,
                        is_ellipse,
                        shape_ellipse_c,
                        shape_angle,
                        snow_on_top,
                        main_block,
                    )?;
                }
                y_off -= 1;
            }
        }
    }

    let do_cut_out = if is_ellipse {
        random.next_double() > 0.1
    } else {
        random.next_double() > 0.7
    };
    if do_cut_out {
        generate_iceberg_cut_out(
            random,
            level,
            width,
            over_water_height,
            origin,
            is_ellipse,
            shape_ellipse_a,
            shape_angle,
            shape_ellipse_c,
        )?;
    }
    Some(true)
}

/// Vanilla `IcebergFeature.generateCutOut` (`IcebergFeature.java:80-119`).
#[allow(clippy::too_many_arguments)]
pub(super) fn generate_iceberg_cut_out<L: WorldGenLevel>(
    random: &mut dyn RandomSource,
    level: &mut L,
    width: i32,
    height: i32,
    global_origin: BlockPos,
    is_ellipse: bool,
    shape_ellipse_a: i32,
    shape_angle: f64,
    shape_ellipse_c: i32,
) -> Option<()> {
    let random_sign_x = if random.next_boolean() { -1 } else { 1 };
    let random_sign_z = if random.next_boolean() { -1 } else { 1 };
    let mut x_off = random.next_int((width / 2 - 2).max(1));
    if random.next_boolean() {
        x_off = width / 2 + 1 - random.next_int((width - width / 2 - 1).max(1));
    }
    let mut z_off = random.next_int((width / 2 - 2).max(1));
    if random.next_boolean() {
        z_off = width / 2 + 1 - random.next_int((width - width / 2 - 1).max(1));
    }
    if is_ellipse {
        x_off = random.next_int((shape_ellipse_a - 5).max(1));
        z_off = x_off;
    }
    let local_origin = BlockPos {
        x: random_sign_x * x_off,
        y: 0,
        z: random_sign_z * z_off,
    };
    let angle = if is_ellipse {
        shape_angle + std::f64::consts::FRAC_PI_2
    } else {
        random.next_double() * 2.0 * std::f64::consts::PI
    };
    for y_off in 0..height - 3 {
        let radius = iceberg_height_dependent_radius_round(random, y_off, height, width);
        iceberg_carve(
            radius,
            y_off,
            global_origin,
            level,
            false,
            angle,
            local_origin,
            shape_ellipse_a,
            shape_ellipse_c,
        )?;
    }
    let mut y_off = -1;
    let under_cut = -height + random.next_int(5);
    while y_off > under_cut {
        let radius = iceberg_height_dependent_radius_steep(random, -y_off, height, width);
        iceberg_carve(
            radius,
            y_off,
            global_origin,
            level,
            true,
            angle,
            local_origin,
            shape_ellipse_a,
            shape_ellipse_c,
        )?;
        y_off -= 1;
    }
    Some(())
}

/// Vanilla `IcebergFeature.carve` (`IcebergFeature.java:121-152`).
#[allow(clippy::too_many_arguments)]
pub(super) fn iceberg_carve<L: WorldGenLevel>(
    radius: i32,
    y_off: i32,
    global_origin: BlockPos,
    level: &mut L,
    under_water: bool,
    angle: f64,
    local_origin: BlockPos,
    shape_ellipse_a: i32,
    shape_ellipse_c: i32,
) -> Option<()> {
    let a = radius + 1 + shape_ellipse_a / 3;
    let c = (radius - 3).min(3) + shape_ellipse_c / 2 - 1;
    let water = vanilla_block_default_state_id(block_type_id("minecraft:water"))?;
    let air = vanilla_block_default_state_id(block_type_id("minecraft:air"))?;
    for xo in -a..a {
        for zo in -a..a {
            let signed_dist = iceberg_signed_distance_ellipse(xo, zo, local_origin, a, c, angle);
            if signed_dist < 0.0 {
                let pos = BlockPos {
                    x: global_origin.x + xo,
                    y: global_origin.y + y_off,
                    z: global_origin.z + zo,
                };
                let state = level.get_block(pos);
                if is_iceberg_state(state) || block_is(state, "minecraft:snow_block") {
                    if under_water {
                        let _ = level.set_block(pos, water, FEATURE_SET_BLOCK);
                    } else {
                        let _ = level.set_block(pos, air, FEATURE_SET_BLOCK);
                        iceberg_remove_floating_snow_layer(level, pos, air);
                    }
                }
            }
        }
    }
    Some(())
}

/// Vanilla `IcebergFeature.removeFloatingSnowLayer` (`IcebergFeature.java:154-158`).
pub(super) fn iceberg_remove_floating_snow_layer<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    air: i32,
) {
    let above = BlockPos {
        x: pos.x,
        y: pos.y + 1,
        z: pos.z,
    };
    if block_is(level.get_block(above), "minecraft:snow") {
        let _ = level.set_block(above, air, FEATURE_SET_BLOCK);
    }
}

/// Vanilla `IcebergFeature.generateIcebergBlock` (`IcebergFeature.java:160-188`).
#[allow(clippy::too_many_arguments)]
pub(super) fn generate_iceberg_block<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    height: i32,
    xo: i32,
    y_off: i32,
    zo: i32,
    radius: i32,
    a: i32,
    is_ellipse: bool,
    shape_ellipse_c: i32,
    shape_angle: f64,
    snow_on_top: bool,
    main_block: i32,
) -> Option<()> {
    let signed_dist = if is_ellipse {
        iceberg_signed_distance_ellipse(
            xo,
            zo,
            BlockPos { x: 0, y: 0, z: 0 },
            a,
            iceberg_ellipse_c(y_off, height, shape_ellipse_c),
            shape_angle,
        )
    } else {
        iceberg_signed_distance_circle(xo, zo, BlockPos { x: 0, y: 0, z: 0 }, radius, random)
    };
    if signed_dist < 0.0 {
        let pos = BlockPos {
            x: origin.x + xo,
            y: origin.y + y_off,
            z: origin.z + zo,
        };
        let compare_val = if is_ellipse {
            -0.5
        } else {
            f64::from(-6 - random.next_int(3))
        };
        if signed_dist > compare_val && random.next_double() > 0.9 {
            return Some(());
        }
        set_iceberg_block(
            pos,
            level,
            random,
            height - y_off,
            height,
            is_ellipse,
            snow_on_top,
            main_block,
        )?;
    }
    Some(())
}

/// Vanilla `IcebergFeature.setIcebergBlock` (`IcebergFeature.java:190-210`).
#[allow(clippy::too_many_arguments)]
pub(super) fn set_iceberg_block<L: WorldGenLevel>(
    pos: BlockPos,
    level: &mut L,
    random: &mut dyn RandomSource,
    h_diff: i32,
    height: i32,
    is_ellipse: bool,
    snow_on_top: bool,
    main_block: i32,
) -> Option<()> {
    let state = level.get_block(pos);
    if is_empty_block(state)
        || block_is(state, "minecraft:snow_block")
        || block_is(state, "minecraft:ice")
        || block_is(state, "minecraft:water")
    {
        let randomness = !is_ellipse || random.next_double() > 0.05;
        let divisor = if is_ellipse { 3 } else { 2 };
        let snow_block = vanilla_block_default_state_id(block_type_id("minecraft:snow_block"))?;
        if snow_on_top
            && !block_is(state, "minecraft:water")
            && f64::from(h_diff)
                <= f64::from(random.next_int((height / divisor).max(1))) + f64::from(height) * 0.6
            && randomness
        {
            let _ = level.set_block(pos, snow_block, FEATURE_SET_BLOCK);
        } else {
            let _ = level.set_block(pos, main_block, FEATURE_SET_BLOCK);
        }
    }
    Some(())
}

/// Vanilla `IcebergFeature.getEllipseC` (`IcebergFeature.java:212-219`).
pub(super) fn iceberg_ellipse_c(y_off: i32, height: i32, shape_ellipse_c: i32) -> i32 {
    let mut c = shape_ellipse_c;
    if y_off > 0 && height - y_off <= 3 {
        c -= 4 - (height - y_off);
    }
    c
}

/// Vanilla `IcebergFeature.signedDistanceCircle` (`IcebergFeature.java:221-224`).
pub(super) fn iceberg_signed_distance_circle(
    xo: i32,
    zo: i32,
    origin: BlockPos,
    radius: i32,
    random: &mut dyn RandomSource,
) -> f64 {
    let off = 10.0 * mth_clamp_f32(random.next_float(), 0.2, 0.8) / radius as f32;
    f64::from(off) + f64::from(xo - origin.x).powi(2) + f64::from(zo - origin.z).powi(2)
        - f64::from(radius).powi(2)
}

/// Vanilla `IcebergFeature.signedDistanceEllipse` (`IcebergFeature.java:226-230`).
pub(super) fn iceberg_signed_distance_ellipse(
    xo: i32,
    zo: i32,
    origin: BlockPos,
    a: i32,
    c: i32,
    angle: f64,
) -> f64 {
    let dx = f64::from(xo - origin.x);
    let dz = f64::from(zo - origin.z);
    ((dx * angle.cos() - dz * angle.sin()) / f64::from(a)).powi(2)
        + ((dx * angle.sin() + dz * angle.cos()) / f64::from(c)).powi(2)
        - 1.0
}

/// Vanilla `IcebergFeature.heightDependentRadiusRound` (`IcebergFeature.java:232-241`).
pub(super) fn iceberg_height_dependent_radius_round(
    random: &mut dyn RandomSource,
    y_off: i32,
    height: i32,
    width: i32,
) -> i32 {
    let k = 3.5 - random.next_float();
    let mut scale = (1.0 - (y_off as f64).powi(2) as f32 / (height as f32 * k)) * width as f32;
    if height > 15 + random.next_int(5) {
        let temp_y_off = if y_off < 3 + random.next_int(6) {
            y_off / 2
        } else {
            y_off
        };
        scale = (1.0 - temp_y_off as f32 / (height as f32 * k * 0.4)) * width as f32;
    }
    mth_ceil_f32(scale / 2.0)
}

/// Vanilla `IcebergFeature.heightDependentRadiusEllipse` (`IcebergFeature.java:243-247`).
pub(super) fn iceberg_height_dependent_radius_ellipse(y_off: i32, height: i32, width: i32) -> i32 {
    let scale = (1.0 - (y_off as f64).powi(2) as f32 / (height as f32 * 1.0)) * width as f32;
    mth_ceil_f32(scale / 2.0)
}

/// Vanilla `IcebergFeature.heightDependentRadiusSteep` (`IcebergFeature.java:249-253`).
pub(super) fn iceberg_height_dependent_radius_steep(
    random: &mut dyn RandomSource,
    y_off: i32,
    height: i32,
    width: i32,
) -> i32 {
    let k = 1.0 + random.next_float() / 2.0;
    let scale = (1.0 - y_off as f32 / (height as f32 * k)) * width as f32;
    mth_ceil_f32(scale / 2.0)
}

/// Vanilla `IcebergFeature.isIcebergState` (`IcebergFeature.java:255-257`).
pub(super) fn is_iceberg_state(state: i32) -> bool {
    block_is(state, "minecraft:packed_ice")
        || block_is(state, "minecraft:snow_block")
        || block_is(state, "minecraft:blue_ice")
}

/// Vanilla `IcebergFeature.smooth` (`IcebergFeature.java:263-295`).
pub(super) fn iceberg_smooth<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    width: i32,
    height: i32,
    is_ellipse: bool,
    shape_ellipse_a: i32,
) -> Option<()> {
    let a = if is_ellipse {
        shape_ellipse_a
    } else {
        width / 2
    };
    let air = vanilla_block_default_state_id(block_type_id("minecraft:air"))?;
    for x in -a..=a {
        for z in -a..=a {
            for y_off in 0..=height {
                let pos = BlockPos {
                    x: origin.x + x,
                    y: origin.y + y_off,
                    z: origin.z + z,
                };
                let state = level.get_block(pos);
                if is_iceberg_state(state) || block_is(state, "minecraft:snow") {
                    let below = BlockPos {
                        x: pos.x,
                        y: pos.y - 1,
                        z: pos.z,
                    };
                    if is_empty_block(level.get_block(below)) {
                        let _ = level.set_block(pos, air, FEATURE_SET_BLOCK);
                        let above = BlockPos {
                            x: pos.x,
                            y: pos.y + 1,
                            z: pos.z,
                        };
                        let _ = level.set_block(above, air, FEATURE_SET_BLOCK);
                    } else if is_iceberg_state(state) {
                        let sides = [
                            BlockPos {
                                x: pos.x - 1,
                                y: pos.y,
                                z: pos.z,
                            },
                            BlockPos {
                                x: pos.x + 1,
                                y: pos.y,
                                z: pos.z,
                            },
                            BlockPos {
                                x: pos.x,
                                y: pos.y,
                                z: pos.z - 1,
                            },
                            BlockPos {
                                x: pos.x,
                                y: pos.y,
                                z: pos.z + 1,
                            },
                        ];
                        let counter = sides
                            .iter()
                            .filter(|side| !is_iceberg_state(level.get_block(**side)))
                            .count();
                        if counter >= 3 {
                            let _ = level.set_block(pos, air, FEATURE_SET_BLOCK);
                        }
                    }
                }
            }
        }
    }
    Some(())
}

/// Vanilla `ReplaceBlobsFeature.place` (`ReplaceBlobsFeature.java:20-48`).
pub(super) fn place_netherrack_replace_blobs<L: WorldGenLevel>(
    config: &ReplaceSphereConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let ReplaceSphereConfig {
        target,
        state: replace,
        ref radius,
    } = *config;
    let target_block = vanilla_block_state_block_type_id(target)?;
    let cursor = BlockPos {
        x: origin.x,
        y: mth_clamp_i32(origin.y, level.get_min_y() + 1, level.get_max_y()),
        z: origin.z,
    };
    let Some(center) = find_replace_blob_target(level, cursor, target_block) else {
        return Some(false);
    };
    let radius_x = radius.sample(random);
    let radius_y = radius.sample(random);
    let radius_z = radius.sample(random);
    let maximum_radius = radius_x.max(radius_y.max(radius_z));
    let mut replaced_any = false;
    for pos in within_manhattan(center, radius_x, radius_y, radius_z) {
        if dist_manhattan(pos, center) > maximum_radius {
            break;
        }
        if vanilla_block_state_block_type_id(level.get_block(pos)) == Some(target_block) {
            // `Feature.setBlock` (`Feature.java:169-171`) uses flags 3.
            let _ = level.set_block(pos, replace, UPDATE_NEIGHBORS | UPDATE_CLIENTS);
            replaced_any = true;
        }
    }
    Some(replaced_any)
}

/// Vanilla `ReplaceBlobsFeature.findTarget` (`ReplaceBlobsFeature.java:51-62`).
pub(super) fn find_replace_blob_target<L: WorldGenLevel>(
    level: &L,
    mut cursor: BlockPos,
    target_block: i32,
) -> Option<BlockPos> {
    while cursor.y > level.get_min_y() + 1 {
        if vanilla_block_state_block_type_id(level.get_block(cursor)) == Some(target_block) {
            return Some(cursor);
        }
        cursor.y -= 1;
    }
    None
}

/// Vanilla `DeltaFeature.place` (`DeltaFeature.java:27-61`).
pub(super) fn place_delta_feature<L: WorldGenLevel>(
    config: &DeltaConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let DeltaConfig {
        contents,
        rim,
        ref size,
        ref rim_size,
    } = *config;
    let contents_block = vanilla_block_state_block_type_id(contents)?;
    let spawn_rim = random.next_double() < DELTA_RIM_SPAWN_CHANCE;
    let rim_x = if spawn_rim {
        rim_size.sample(random)
    } else {
        0
    };
    let rim_z = if spawn_rim {
        rim_size.sample(random)
    } else {
        0
    };
    let has_rim = spawn_rim && rim_x != 0 && rim_z != 0;
    let radius_x = size.sample(random);
    let radius_z = size.sample(random);
    let radius_limit = radius_x.max(radius_z);
    let mut any_placed = false;
    for pos in within_manhattan(origin, radius_x, 0, radius_z) {
        if dist_manhattan(pos, origin) > radius_limit {
            break;
        }
        if is_delta_clear(level, pos, contents_block) {
            if has_rim {
                any_placed = true;
                let _ = level.set_block(pos, rim, UPDATE_NEIGHBORS | UPDATE_CLIENTS);
            }
            let offset = BlockPos {
                x: pos.x + rim_x,
                y: pos.y,
                z: pos.z + rim_z,
            };
            if is_delta_clear(level, offset, contents_block) {
                any_placed = true;
                let _ = level.set_block(offset, contents, UPDATE_NEIGHBORS | UPDATE_CLIENTS);
            }
        }
    }
    Some(any_placed)
}

/// Vanilla `DeltaFeature.isClear` (`DeltaFeature.java:63-81`).
pub(super) fn is_delta_clear<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    contents_block: i32,
) -> bool {
    let state = level.get_block(pos);
    if vanilla_block_state_block_type_id(state) == Some(contents_block) {
        return false;
    }
    if DELTA_CANNOT_REPLACE
        .iter()
        .any(|block_name| block_is(state, block_name))
    {
        return false;
    }
    for (dx, dy, dz) in DIRECTION_OFFSETS {
        let neighbour = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        let neighbour_is_air = is_empty_block(level.get_block(neighbour));
        let is_up = (dx, dy, dz) == (0, 1, 0);
        if neighbour_is_air && !is_up || !neighbour_is_air && is_up {
            return false;
        }
    }
    true
}

/// Vanilla `ReplaceBlockFeature.place` (`ReplaceBlockFeature.java:15-28`).
pub(super) fn place_replace_single_block<L: WorldGenLevel>(
    targets: &[OreTarget],
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let current = level.get_block(origin);
    for target in targets {
        if rule_test_matches(level, &target.target, current, random) {
            let _ = level.set_block(origin, target.state, UPDATE_CLIENTS);
            break;
        }
    }
    Some(true)
}

/// Vanilla `PointedDripstoneFeature.place` (`PointedDripstoneFeature.java:17-34`).
pub(super) fn place_pointed_dripstone<L: WorldGenLevel>(
    config: &PointedDripstoneConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let chances = *config;
    let Some(tip_dy) = dripstone_tip_direction(level, origin, random) else {
        return Some(false);
    };
    let root = BlockPos {
        x: origin.x,
        y: origin.y - tip_dy,
        z: origin.z,
    };
    create_patch_of_dripstone_blocks(level, random, root, chances);
    let ahead = BlockPos {
        x: origin.x,
        y: origin.y + tip_dy,
        z: origin.z,
    };
    let height = if random.next_float() < chances.chance_of_taller_dripstone
        && is_empty_or_water(level.get_block(ahead))
    {
        2
    } else {
        1
    };
    grow_pointed_dripstone(level, origin, tip_dy, height, false);
    Some(true)
}

/// Vanilla `PointedDripstoneFeature.getTipDirection` (`PointedDripstoneFeature.java:36-46`).
pub(super) fn dripstone_tip_direction<L: WorldGenLevel>(
    level: &L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<i32> {
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
    let can_above = is_dripstone_base(level, level.get_block(above));
    let can_below = is_dripstone_base(level, level.get_block(below));
    if can_above && can_below {
        Some(if random.next_boolean() { -1 } else { 1 })
    } else if can_above {
        Some(-1)
    } else if can_below {
        Some(1)
    } else {
        None
    }
}

/// Vanilla `PointedDripstoneFeature.createPatchOfDripstoneBlocks`
/// (`PointedDripstoneFeature.java:48-67`).
pub(super) fn create_patch_of_dripstone_blocks<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    chances: PointedDripstoneConfig,
) {
    let _ = place_dripstone_block_if_possible(level, pos);
    for (dx, dy, dz) in HORIZONTAL_OFFSETS {
        if random.next_float() > chances.chance_of_directional_spread {
            continue;
        }
        let pos1 = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        let _ = place_dripstone_block_if_possible(level, pos1);
        if random.next_float() > chances.chance_of_spread_radius2 {
            continue;
        }
        let (rdx, rdy, rdz) = direction_get_random(random);
        let pos2 = BlockPos {
            x: pos1.x + rdx,
            y: pos1.y + rdy,
            z: pos1.z + rdz,
        };
        let _ = place_dripstone_block_if_possible(level, pos2);
        if random.next_float() > chances.chance_of_spread_radius3 {
            continue;
        }
        let (rdx, rdy, rdz) = direction_get_random(random);
        let pos3 = BlockPos {
            x: pos2.x + rdx,
            y: pos2.y + rdy,
            z: pos2.z + rdz,
        };
        let _ = place_dripstone_block_if_possible(level, pos3);
    }
}

/// Vanilla `Direction.getRandom` (`Direction.java:307-308`).
pub(super) fn direction_get_random(random: &mut dyn RandomSource) -> (i32, i32, i32) {
    DIRECTION_OFFSETS[random.next_int(6) as usize]
}

/// Vanilla `DripstoneUtils.placeDripstoneBlockIfPossible` (`DripstoneUtils.java:92-100`).
pub(super) fn place_dripstone_block_if_possible<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
) -> bool {
    let state = level.get_block(pos);
    if level.block_state_in_tag(state, DRIPSTONE_REPLACEABLE_TAG) {
        let dripstone = default_block_state("minecraft:dripstone_block");
        let _ = level.set_block(pos, dripstone, UPDATE_CLIENTS);
        true
    } else {
        false
    }
}

/// Vanilla `DripstoneUtils.growPointedDripstone` (`DripstoneUtils.java:76-90`).
pub(super) fn grow_pointed_dripstone<L: WorldGenLevel>(
    level: &mut L,
    start: BlockPos,
    tip_dy: i32,
    height: i32,
    merged_tip: bool,
) {
    let root = BlockPos {
        x: start.x,
        y: start.y - tip_dy,
        z: start.z,
    };
    let root_state = level.get_block(root);
    if !is_dripstone_base(level, root_state) {
        return;
    }
    let mut pos = start;
    for thickness in dripstone_column_thicknesses(height, merged_tip) {
        let waterlogged = is_water_at(level.get_block(pos));
        let state = pointed_dripstone_state(tip_dy, thickness, waterlogged);
        let _ = level.set_block(pos, state, UPDATE_CLIENTS);
        pos.y += tip_dy;
    }
}

/// Vanilla `DripstoneUtils.buildBaseToTipColumn` (`DripstoneUtils.java:58-74`).
pub(super) fn dripstone_column_thicknesses(height: i32, merged_tip: bool) -> Vec<&'static str> {
    let mut thicknesses = Vec::new();
    if height >= 3 {
        thicknesses.push("base");
        thicknesses.extend(std::iter::repeat_n("middle", (height - 3) as usize));
    }
    if height >= 2 {
        thicknesses.push("frustum");
    }
    if height >= 1 {
        thicknesses.push(if merged_tip { "tip_merge" } else { "tip" });
    }
    thicknesses
}

pub(super) fn pointed_dripstone_state(tip_dy: i32, thickness: &str, waterlogged: bool) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert(
        "vertical_direction".to_string(),
        if tip_dy > 0 {
            "up".to_string()
        } else {
            "down".to_string()
        },
    );
    properties.insert("thickness".to_string(), thickness.to_string());
    properties.insert(
        "waterlogged".to_string(),
        if waterlogged {
            "true".to_string()
        } else {
            "false".to_string()
        },
    );
    vanilla_block_state_id_by_name_and_properties("minecraft:pointed_dripstone", &properties)
        .expect("vanilla block state")
}

/// Vanilla `DripstoneUtils.isDripstoneBase` (`DripstoneUtils.java:113-115`).
pub(super) fn is_dripstone_base<L: WorldGenLevel>(level: &L, state_id: i32) -> bool {
    if block_is(state_id, "minecraft:dripstone_block") {
        return true;
    }
    level.block_state_in_tag(state_id, DRIPSTONE_REPLACEABLE_TAG)
}

/// Vanilla `DripstoneUtils.isEmptyOrWater` (`DripstoneUtils.java:117-119`).
pub(super) fn is_empty_or_water(state_id: i32) -> bool {
    is_empty_block(state_id) || block_is(state_id, "minecraft:water")
}

/// Vanilla `DripstoneUtils.isDripstoneBaseOrLava` (`DripstoneUtils.java:109-111`).
pub(super) fn is_dripstone_base_or_lava<L: WorldGenLevel>(level: &L, state_id: i32) -> bool {
    is_dripstone_base(level, state_id) || block_is(state_id, "minecraft:lava")
}

/// Vanilla `DripstoneUtils.isEmptyOrWaterOrLava` (`DripstoneUtils.java:125-127`).
pub(super) fn is_empty_or_water_or_lava(state_id: i32) -> bool {
    is_empty_or_water(state_id) || block_is(state_id, "minecraft:lava")
}

/// Vanilla `LevelReader.isWaterAt` (`LevelReader.java:136-138`) for a still-water cell.
pub(super) fn is_water_at(state_id: i32) -> bool {
    block_is(state_id, "minecraft:water")
}

/// Vanilla `DripstoneUtils.isNeitherEmptyNorWater` (`DripstoneUtils.java:121-123`).
pub(super) fn is_neither_empty_nor_water(state_id: i32) -> bool {
    !is_empty_block(state_id) && !block_is(state_id, "minecraft:water")
}

/// Vanilla `FluidState.is(FluidTags.WATER)` for a block state: still water or
/// a waterlogged cell.
pub(super) fn block_fluid_is_water(state_id: i32) -> bool {
    if is_water_at(state_id) {
        return true;
    }
    vanilla_block_state_identity(state_id)
        .and_then(|identity| {
            identity
                .properties
                .get("waterlogged")
                .map(|value| value == "true")
        })
        .unwrap_or(false)
}
