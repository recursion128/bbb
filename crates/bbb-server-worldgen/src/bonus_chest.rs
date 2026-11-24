//! Vanilla `BonusChestFeature` (`Feature.BONUS_CHEST`, `Feature.java:132`):
//! the spawn bonus chest and its four torches.

use bbb_protocol::block_state_types::VanillaBlockSupportType;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;

use super::monster_room::{set_block_entity_loot_table, HORIZONTAL};
use super::{
    catalog_default_state, default_block_state, feature_collision_shape_empty, feature_face_sturdy,
    is_empty_block, offset_block_pos, util_shuffle, WorldGenLevel, UPDATE_CLIENTS,
};

/// `BuiltInLootTables.SPAWN_BONUS_CHEST` (`BuiltInLootTables.java:16`).
const SPAWN_BONUS_CHEST: &str = "minecraft:chests/spawn_bonus_chest";

/// `BonusChestFeature.place` (`BonusChestFeature.java:25-55`).
pub fn place_bonus_chest<L: WorldGenLevel>(
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    // `ChunkPos.containing` and its block bounds (`ChunkPos.java:48-50`,
    // `:111-125`).
    let min_x = (origin.x >> 4) << 4;
    let min_z = (origin.z >> 4) << 4;
    // `Util.toShuffledList(IntStream)` (`Util.java:1002-1012`).
    let mut x_poses: Vec<i32> = (min_x..=min_x + 15).collect();
    util_shuffle(&mut x_poses, random);
    let mut z_poses: Vec<i32> = (min_z..=min_z + 15).collect();
    util_shuffle(&mut z_poses, random);
    for &x in &x_poses {
        for &z in &z_poses {
            // `LevelReader.getHeightmapPos` (`LevelReader.java:80-82`) for
            // `MOTION_BLOCKING_NO_LEAVES` (`Heightmap.java:150-154`).
            let chest_pos = BlockPos {
                x,
                y: level.get_motion_blocking_no_leaves_height(x, z),
                z,
            };
            let state = level.get_block(chest_pos);
            if is_empty_block(state) || collision_shape_is_empty(level, chest_pos, state) {
                level.set_block(
                    chest_pos,
                    catalog_default_state("minecraft:chest"),
                    UPDATE_CLIENTS,
                );
                set_block_entity_loot_table(level, random, chest_pos, SPAWN_BONUS_CHEST);
                let torch = default_block_state("minecraft:torch");
                for direction in HORIZONTAL {
                    let torch_pos = offset_block_pos(chest_pos, direction);
                    if torch_can_survive(level, torch_pos) {
                        level.set_block(torch_pos, torch, UPDATE_CLIENTS);
                    }
                }
                return Some(true);
            }
        }
    }
    Some(false)
}

/// `BlockStateBase.getCollisionShape(level, pos).isEmpty()`
/// (`BlockBehaviour.java:675-677`).
fn collision_shape_is_empty<L: WorldGenLevel>(level: &L, pos: BlockPos, state: i32) -> bool {
    feature_collision_shape_empty(level, pos, state)
}

/// `BaseTorchBlock.canSurvive` (`BaseTorchBlock.java:47-49`) through
/// `Block.canSupportCenter(level, below, UP)` (`Block.java:343-348`).
fn torch_can_survive<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let below_pos = offset_block_pos(pos, Direction::Down);
    feature_face_sturdy(
        level,
        below_pos,
        level.get_block(below_pos),
        Direction::Up,
        VanillaBlockSupportType::Center,
    )
}

#[cfg(test)]
mod tests {

    use crate::random::WorldgenRandom;

    use super::super::test_support::SeamLevel;
    use super::super::tests::{origin, retained_from_json};
    use super::super::{default_block_state, place_configured_feature};
    use super::*;

    const CONFIG_JSON: &[u8] = br#"{"type":"minecraft:bonus_chest","config":{}}"#;

    fn chest_pos() -> BlockPos {
        BlockPos {
            x: 10,
            y: 65,
            z: -7,
        }
    }

    /// (10, -7) is the only chunk column whose heightmap cell is air.
    fn stone_level() -> SeamLevel {
        let mut level = SeamLevel::filled(default_block_state("minecraft:stone"));
        level.motion_blocking_no_leaves_heights.insert((10, -7), 65);
        level.set_cell(chest_pos(), default_block_state("minecraft:air"));
        level
    }

    #[test]
    fn bonus_chest_lands_on_the_first_shuffled_open_column_with_loot_and_torches() {
        let mut level = stone_level();
        let placed = place_configured_feature(
            &retained_from_json(CONFIG_JSON),
            &mut level,
            origin(),
            &mut WorldgenRandom::with_seed(1),
        );
        assert!(placed);
        let chest = catalog_default_state("minecraft:chest");
        let torch = default_block_state("minecraft:torch");
        let at = |x, z| BlockPos { x, y: 65, z };
        assert_eq!(
            level.inner.sets.borrow().as_slice(),
            &[
                (chest_pos(), chest, UPDATE_CLIENTS),
                (at(10, -8), torch, UPDATE_CLIENTS),
                (at(11, -7), torch, UPDATE_CLIENTS),
                (at(10, -6), torch, UPDATE_CLIENTS),
                (at(9, -7), torch, UPDATE_CLIENTS),
            ]
        );
        let loot_tables = level.loot_tables.borrow();
        assert_eq!(loot_tables.len(), 1);
        assert_eq!(
            (loot_tables[0].0, loot_tables[0].1),
            (chest_pos(), SPAWN_BONUS_CHEST)
        );
    }

    #[test]
    fn bonus_chest_skips_torches_without_centre_support() {
        let mut level = stone_level();
        level.set_cell(
            BlockPos {
                x: 10,
                y: 64,
                z: -8,
            },
            default_block_state("minecraft:air"),
        );
        let placed = place_configured_feature(
            &retained_from_json(CONFIG_JSON),
            &mut level,
            origin(),
            &mut WorldgenRandom::with_seed(1),
        );
        assert!(placed);
        assert_eq!(level.inner.sets.borrow().len(), 4);
        assert!(level.inner.sets.borrow().iter().all(|(pos, _, _)| *pos
            != BlockPos {
                x: 10,
                y: 65,
                z: -8
            }));
    }

    #[test]
    fn bonus_chest_returns_false_when_every_column_is_blocked() {
        let mut level = SeamLevel::filled(default_block_state("minecraft:stone"));
        let placed = place_configured_feature(
            &retained_from_json(CONFIG_JSON),
            &mut level,
            origin(),
            &mut WorldgenRandom::with_seed(1),
        );
        assert!(!placed);
        assert!(level.inner.sets.borrow().is_empty());
    }
}
