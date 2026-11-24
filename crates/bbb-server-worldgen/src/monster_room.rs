//! Vanilla `MonsterRoomFeature` (`Feature.MONSTER_ROOM`, `Feature.java:99`):
//! the cobblestone dungeon with up to two loot chests and a spawner.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;

use super::multiface::legacy_solid;
use super::{
    block_is, block_set_property, catalog_default_state, default_block_state, direction_opposite,
    is_empty_block, offset_block_pos, WorldGenLevel, FEATURES_CANNOT_REPLACE_TAG, UPDATE_CLIENTS,
};

/// `MonsterRoomFeature.MOBS` (`MonsterRoomFeature.java:24`).
const MOBS: [&str; 4] = [
    "minecraft:skeleton",
    "minecraft:zombie",
    "minecraft:zombie",
    "minecraft:spider",
];
/// `BuiltInLootTables.SIMPLE_DUNGEON` (`BuiltInLootTables.java:18`).
const SIMPLE_DUNGEON: &str = "minecraft:chests/simple_dungeon";
/// `Direction.Plane.HORIZONTAL` (`Direction.java:577`).
pub const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// Vanilla `MonsterRoomFeature.place` (`MonsterRoomFeature.java:32-126`).
pub fn place_monster_room<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let cave_air = default_block_state("minecraft:cave_air");
    let xr = random.next_int(2) + 2;
    let min_x = -xr - 1;
    let max_x = xr + 1;
    let zr = random.next_int(2) + 2;
    let min_z = -zr - 1;
    let max_z = zr + 1;
    let mut hole_count = 0;
    for dx in min_x..=max_x {
        for dy in -1..=4 {
            for dz in min_z..=max_z {
                let hole_pos = offset(origin, dx, dy, dz);
                let solid = legacy_solid(level.get_block(hole_pos))?;
                if dy == -1 && !solid {
                    return Some(false);
                }
                if dy == 4 && !solid {
                    return Some(false);
                }
                if (dx == min_x || dx == max_x || dz == min_z || dz == max_z)
                    && dy == 0
                    && is_empty_block(level.get_block(hole_pos))
                    && is_empty_block(level.get_block(offset_block_pos(hole_pos, Direction::Up)))
                {
                    hole_count += 1;
                }
            }
        }
    }
    if !(1..=5).contains(&hole_count) {
        return Some(false);
    }
    for dx in min_x..=max_x {
        for dy in (-1..=3).rev() {
            for dz in min_z..=max_z {
                let wall_block = offset(origin, dx, dy, dz);
                let wall_state = level.get_block(wall_block);
                if dx == min_x || dy == -1 || dz == min_z || dx == max_x || dy == 4 || dz == max_z {
                    if wall_block.y >= level.get_min_y()
                        && !legacy_solid(
                            level.get_block(offset_block_pos(wall_block, Direction::Down)),
                        )?
                    {
                        let _ = level.set_block(wall_block, cave_air, UPDATE_CLIENTS);
                    } else if legacy_solid(wall_state)? && !block_is(wall_state, "minecraft:chest")
                    {
                        if dy == -1 && random.next_int(4) != 0 {
                            safe_set_block(
                                level,
                                wall_block,
                                default_block_state("minecraft:mossy_cobblestone"),
                            );
                        } else {
                            safe_set_block(
                                level,
                                wall_block,
                                default_block_state("minecraft:cobblestone"),
                            );
                        }
                    }
                } else if !block_is(wall_state, "minecraft:chest")
                    && !block_is(wall_state, "minecraft:spawner")
                {
                    safe_set_block(level, wall_block, cave_air);
                }
            }
        }
    }
    for _ in 0..2 {
        for _ in 0..3 {
            let xc = origin.x + random.next_int(xr * 2 + 1) - xr;
            let yc = origin.y;
            let zc = origin.z + random.next_int(zr * 2 + 1) - zr;
            let chest_pos = BlockPos {
                x: xc,
                y: yc,
                z: zc,
            };
            if is_empty_block(level.get_block(chest_pos)) {
                let mut wall_count = 0;
                for direction in HORIZONTAL {
                    if legacy_solid(level.get_block(offset_block_pos(chest_pos, direction)))? {
                        wall_count += 1;
                    }
                }
                if wall_count == 1 {
                    let chest =
                        reorient(level, chest_pos, catalog_default_state("minecraft:chest"))?;
                    safe_set_block(level, chest_pos, chest);
                    set_block_entity_loot_table(level, random, chest_pos, SIMPLE_DUNGEON);
                    break;
                }
            }
        }
    }
    safe_set_block(level, origin, catalog_default_state("minecraft:spawner"));
    // `getBlockEntity(origin) instanceof SpawnerBlockEntity`: a spawner block
    // always carries one, whether written now or preserved from before.
    if block_is(level.get_block(origin), "minecraft:spawner") {
        let entity_type = MOBS[random.next_int(MOBS.len() as i32) as usize];
        level.set_spawner_entity_id(origin, entity_type);
    }
    Some(true)
}

fn offset(origin: BlockPos, dx: i32, dy: i32, dz: i32) -> BlockPos {
    BlockPos {
        x: origin.x + dx,
        y: origin.y + dy,
        z: origin.z + dz,
    }
}

/// `Feature.safeSetBlock` (`Feature.java:177-181`) with
/// `Feature.isReplaceable(BlockTags.FEATURES_CANNOT_REPLACE)` (`:173-175`).
fn safe_set_block<L: WorldGenLevel>(level: &mut L, pos: BlockPos, state: i32) {
    let existing = level.get_block(pos);
    if !level.block_state_in_tag(existing, FEATURES_CANNOT_REPLACE_TAG) {
        let _ = level.set_block(pos, state, UPDATE_CLIENTS);
    }
}

/// `RandomizableContainer.setBlockEntityLootTable` (`RandomizableContainer.java:42-48`):
/// the `nextLong` seed is drawn only when a container block entity is there,
/// and the write itself goes through the level's block-entity seam.
pub fn set_block_entity_loot_table<L: WorldGenLevel>(
    level: &mut L,
    random: &mut dyn RandomSource,
    pos: BlockPos,
    loot_table: &'static str,
) {
    if !block_is(level.get_block(pos), "minecraft:chest") {
        return;
    }
    let seed = random.next_long();
    level.set_block_entity_loot_table(pos, loot_table, seed);
}

/// `StructurePiece.reorient` (`StructurePiece.java:417-459`).
fn reorient<L: WorldGenLevel>(level: &L, block_pos: BlockPos, block_state: i32) -> Option<i32> {
    let mut solid_neighbor: Option<Direction> = None;
    for direction in HORIZONTAL {
        let state = level.get_block(offset_block_pos(block_pos, direction));
        if block_is(state, "minecraft:chest") {
            return Some(block_state);
        }
        if solid_render(state)? {
            if solid_neighbor.is_some() {
                solid_neighbor = None;
                break;
            }
            solid_neighbor = Some(direction);
        }
    }
    if let Some(solid_neighbor) = solid_neighbor {
        return Some(block_set_property(
            block_state,
            "facing",
            direction_name(direction_opposite(solid_neighbor)),
        ));
    }
    let mut lock_dir = Direction::North;
    let mut relative_pos = offset_block_pos(block_pos, lock_dir);
    if solid_render(level.get_block(relative_pos))? {
        lock_dir = direction_opposite(lock_dir);
        relative_pos = offset_block_pos(block_pos, lock_dir);
    }
    if solid_render(level.get_block(relative_pos))? {
        lock_dir = clockwise(lock_dir);
        relative_pos = offset_block_pos(block_pos, lock_dir);
    }
    if solid_render(level.get_block(relative_pos))? {
        lock_dir = direction_opposite(lock_dir);
    }
    Some(block_set_property(
        block_state,
        "facing",
        direction_name(lock_dir),
    ))
}

/// `BlockStateBase.isSolidRender` (`BlockBehaviour.java:655-657`).
fn solid_render(state: i32) -> Option<bool> {
    vanilla_block_state_support_facts(state).map(|facts| facts.solid_render())
}

/// `Direction.getClockWise` (`Direction.java:195-203`).
fn clockwise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::East,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        Direction::East => Direction::South,
        Direction::Down | Direction::Up => direction,
    }
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;
    use bbb_protocol::packets::{BlockPos, Direction};

    use crate::test_support::SeamLevel;
    use crate::tests::{default_state, origin, place_feature, retained_from_json, RecordingLevel};
    use crate::{offset_block_pos, WorldGenLevel, UPDATE_CLIENTS};

    fn retained() -> bbb_server_datapack::worldgen::ConfiguredFeature {
        retained_from_json(br#"{"type":"minecraft:monster_room","config":{}}"#)
    }

    /// Solid stone with one two-high air notch on the west edge column of the
    /// room the seed-1 radii select, which is the single hole the validity
    /// scan wants.
    fn notched_level() -> (RecordingLevel, i32, i32) {
        let mut random = WorldgenRandom::with_seed(1);
        let xr = random.next_int(2) + 2;
        let zr = random.next_int(2) + 2;
        let level = RecordingLevel::filled(default_state("minecraft:stone"));
        let origin = origin();
        let air = default_state("minecraft:air");
        for dy in 0..=1 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x - xr - 1, origin.y + dy, origin.z), air);
        }
        (level, xr, zr)
    }

    #[test]
    fn monster_room_carves_walls_chests_and_spawner_through_the_seams() {
        let (inner, xr, zr) = notched_level();
        let mut level = SeamLevel::wrap(inner);
        assert!(place_feature(&retained(), &mut level, origin()));

        let origin = origin();
        let sets = level.inner.sets.borrow().clone();
        let cobblestone = default_state("minecraft:cobblestone");
        let mossy = default_state("minecraft:mossy_cobblestone");
        let cave_air = default_state("minecraft:cave_air");
        let spawner = crate::catalog_default_state("minecraft:spawner");
        // First wall cell: the top corner of the west wall, whose support
        // below is still stone.
        assert_eq!(
            sets[0],
            (
                BlockPos {
                    x: origin.x - xr - 1,
                    y: origin.y + 3,
                    z: origin.z - zr - 1,
                },
                cobblestone,
                UPDATE_CLIENTS
            )
        );
        // Floor ring cells are mossy or plain cobblestone, nothing else.
        for (pos, state, flags) in &sets {
            if pos.y == origin.y - 1 {
                assert!(
                    *state == cobblestone || *state == mossy,
                    "floor write {pos:?}"
                );
                assert_eq!(*flags, UPDATE_CLIENTS);
            }
        }
        // Every interior cell at the origin level is carved to cave air.
        for dx in -xr..=xr {
            for dz in -zr..=zr {
                let pos = BlockPos {
                    x: origin.x + dx,
                    y: origin.y,
                    z: origin.z + dz,
                };
                if pos != origin {
                    assert!(
                        sets.contains(&(pos, cave_air, UPDATE_CLIENTS)),
                        "interior {pos:?} not carved"
                    );
                }
            }
        }
        assert_eq!(
            sets.last().copied(),
            Some((origin, spawner, UPDATE_CLIENTS))
        );
        let spawners = level.spawners.borrow().clone();
        assert_eq!(spawners.len(), 1);
        assert_eq!(spawners[0].0, origin);
        assert!(super::MOBS.contains(&spawners[0].1));
        // Each chest looks away from its single solid neighbour and got the
        // dungeon loot table.
        let loot = level.loot_tables.borrow().clone();
        for (pos, table, _seed) in &loot {
            assert_eq!(*table, super::SIMPLE_DUNGEON);
            let state = level.inner.get_block(*pos);
            assert!(crate::block_is(state, "minecraft:chest"));
            let facing = bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .unwrap()
                .properties
                .get("facing")
                .unwrap()
                .clone();
            let behind = match facing.as_str() {
                "north" => Direction::South,
                "south" => Direction::North,
                "west" => Direction::East,
                _ => Direction::West,
            };
            assert_eq!(
                level.inner.get_block(offset_block_pos(*pos, behind)),
                cobblestone
            );
        }
        assert!(loot.len() <= 2);
    }

    #[test]
    fn monster_room_refuses_open_air() {
        let mut open = RecordingLevel::admitting();
        assert!(!place_feature(&retained(), &mut open, origin()));
        assert!(open.sets.borrow().is_empty());
    }
}
