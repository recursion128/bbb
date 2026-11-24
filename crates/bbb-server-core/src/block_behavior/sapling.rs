//! Sapling stages, mangrove propagule aging and `TreeGrower.growTree`.

use super::LevelCtx;
use crate::fluid::{above, with_property};
use bbb_protocol::block_types::{vanilla_block_type, VanillaRandomTickOverride};
use bbb_protocol::packets::BlockPos;
use bbb_server_worldgen::{place_configured_feature, WorldGenLevel};

use super::{
    air_block_state_id, block_type_of, bool_property, int_property, offset, random_tick_override,
    SET_BLOCK_UPDATE_CLIENTS, SET_BLOCK_UPDATE_NONE,
};

const STAGE: &str = "stage";
const AGE: &str = "age";
const HANGING: &str = "hanging";
const MAX_MANGROVE_AGE: i32 = 4;

#[must_use]
pub fn is_sapling(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(random_tick_override)
        == Some(VanillaRandomTickOverride::SaplingBlock)
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if level.max_block_brightness(above(pos)) >= 9 && level.level_random().next_int(7) == 0 {
        advance_tree(level, pos, block_state_id);
    }
}

pub(super) fn mangrove_random_tick(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    match bool_property(block_state_id, HANGING) {
        Some(false) if level.level_random().next_int(7) == 0 => {
            advance_tree(level, pos, block_state_id);
        }
        Some(true) => {
            let Some(age) = int_property(block_state_id, AGE) else {
                return;
            };
            if age < MAX_MANGROVE_AGE {
                if let Some(next) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
                    level.set_block(
                        pos,
                        next,
                        SET_BLOCK_UPDATE_CLIENTS,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                }
            }
        }
        _ => {}
    }
}

/// `SaplingBlock.advanceTree`, also its `performBonemeal`.
pub(super) fn advance_tree(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if int_property(block_state_id, STAGE) == Some(0) {
        if let Some(next) = with_property(block_state_id, STAGE, "1") {
            level.set_block(
                pos,
                next,
                SET_BLOCK_UPDATE_NONE,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    } else {
        grow_tree(level, pos, block_state_id);
    }
}

/// One `TreeGrower` constant (`TreeGrower.java:29-69`); feature ids are
/// configured feature keys.
struct TreeGrower {
    secondary_chance: f32,
    mega_tree: Option<&'static str>,
    secondary_mega_tree: Option<&'static str>,
    tree: Option<&'static str>,
    secondary_tree: Option<&'static str>,
    flowers: Option<&'static str>,
    secondary_flowers: Option<&'static str>,
}

impl TreeGrower {
    const fn simple(
        mega_tree: Option<&'static str>,
        tree: Option<&'static str>,
        flowers: Option<&'static str>,
    ) -> Self {
        Self {
            secondary_chance: 0.0,
            mega_tree,
            secondary_mega_tree: None,
            tree,
            secondary_tree: None,
            flowers,
            secondary_flowers: None,
        }
    }
}

const OAK: TreeGrower = TreeGrower {
    secondary_chance: 0.1,
    mega_tree: None,
    secondary_mega_tree: None,
    tree: Some("minecraft:oak"),
    secondary_tree: Some("minecraft:fancy_oak"),
    flowers: Some("minecraft:oak_bees_005"),
    secondary_flowers: Some("minecraft:fancy_oak_bees_005"),
};
const SPRUCE: TreeGrower = TreeGrower {
    secondary_chance: 0.5,
    mega_tree: Some("minecraft:mega_spruce"),
    secondary_mega_tree: Some("minecraft:mega_pine"),
    tree: Some("minecraft:spruce"),
    secondary_tree: None,
    flowers: None,
    secondary_flowers: None,
};
const MANGROVE: TreeGrower = TreeGrower {
    secondary_chance: 0.85,
    mega_tree: None,
    secondary_mega_tree: None,
    tree: Some("minecraft:mangrove"),
    secondary_tree: Some("minecraft:tall_mangrove"),
    flowers: None,
    secondary_flowers: None,
};
const AZALEA: TreeGrower = TreeGrower::simple(None, Some("minecraft:azalea_tree"), None);

/// The `TreeGrower` each sapling block carries in `Blocks`, plus
/// `AzaleaBlock.performBonemeal`'s.
const GROWERS: &[(&str, TreeGrower)] = &[
    ("minecraft:oak_sapling", OAK),
    ("minecraft:spruce_sapling", SPRUCE),
    (
        "minecraft:birch_sapling",
        TreeGrower::simple(
            None,
            Some("minecraft:birch"),
            Some("minecraft:birch_bees_005"),
        ),
    ),
    (
        "minecraft:jungle_sapling",
        TreeGrower::simple(
            Some("minecraft:mega_jungle_tree"),
            Some("minecraft:jungle_tree_no_vine"),
            None,
        ),
    ),
    (
        "minecraft:acacia_sapling",
        TreeGrower::simple(None, Some("minecraft:acacia"), None),
    ),
    (
        "minecraft:cherry_sapling",
        TreeGrower::simple(
            None,
            Some("minecraft:cherry"),
            Some("minecraft:cherry_bees_005"),
        ),
    ),
    (
        "minecraft:dark_oak_sapling",
        TreeGrower::simple(Some("minecraft:dark_oak"), None, None),
    ),
    (
        "minecraft:pale_oak_sapling",
        TreeGrower::simple(Some("minecraft:pale_oak_bonemeal"), None, None),
    ),
    ("minecraft:mangrove_propagule", MANGROVE),
    ("minecraft:azalea", AZALEA),
    ("minecraft:flowering_azalea", AZALEA),
];

const FLOWERS_TAG: &str = "minecraft:flowers";
/// `TreeGrower.growTree` writes with flag `260` (`UPDATE_NONE | UPDATE_SKIP_ALL_SIDEEFFECTS`,
/// `TreeGrower.java:137-148,168,176`) and broadcasts `sendBlockUpdated` only if the
/// single-tree origin stays `empty`.
const GROW_TREE_FLAGS: i32 = 260;

fn grower_of(state: i32) -> Option<&'static TreeGrower> {
    let entry = vanilla_block_type(block_type_of(state)?)?;
    GROWERS
        .iter()
        .find_map(|(id, grower)| (*id == entry.resource_id).then_some(grower))
}

/// `TreeGrower.getMinimumHeight` (`TreeGrower.java:199-209`): the plain
/// tree's trunk base height, 0 when the grower has none.
pub(super) fn minimum_height(level: &LevelCtx<'_, '_>, state: i32) -> i32 {
    grower_of(state)
        .and_then(|grower| grower.tree)
        .and_then(|id| level.configured_feature(id))
        .and_then(|feature| match &*feature {
            bbb_server_datapack::worldgen::feature::ConfiguredFeature::Tree(tree) => {
                Some(tree.trunk_placer.base_height)
            }
            _ => None,
        })
        .unwrap_or(0)
}

/// `TreeGrower.growTree` (`TreeGrower.java:127-179`).
pub(super) fn grow_tree(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> bool {
    let Some(block) = block_type_of(state) else {
        return false;
    };
    let Some(grower) = grower_of(state) else {
        return false;
    };
    let random = level.level_random();
    let mega =
        if grower.secondary_mega_tree.is_some() && random.next_float() < grower.secondary_chance {
            grower.secondary_mega_tree
        } else {
            grower.mega_tree
        };
    if let Some(feature) = mega.and_then(|id| level.configured_feature(id)) {
        for dx in [0, -1] {
            for dz in [0, -1] {
                let corners = [(dx, dz), (dx + 1, dz), (dx, dz + 1), (dx + 1, dz + 1)]
                    .map(|(x, z)| offset(pos, x, 0, z));
                if !corners
                    .iter()
                    .all(|&corner| block_type_of(level.block_state(corner)) == Some(block))
                {
                    continue;
                }
                let air = air_block_state_id();
                for corner in corners {
                    set(level, corner, air);
                }
                if place_configured_feature(&feature, level, corners[0], &mut &*random) {
                    return true;
                }
                for corner in corners {
                    set(level, corner, state);
                }
                return false;
            }
        }
    }

    let has_flowers =
        (grower.flowers.is_some() || grower.secondary_flowers.is_some()) && has_flowers(level, pos);
    let secondary = random.next_float() < grower.secondary_chance;
    let key = match (secondary, has_flowers) {
        (true, true) if grower.secondary_flowers.is_some() => grower.secondary_flowers,
        (true, _) if grower.secondary_tree.is_some() => grower.secondary_tree,
        (_, true) if grower.flowers.is_some() => grower.flowers,
        _ => grower.tree,
    };
    let Some(feature) = key.and_then(|id| level.configured_feature(id)) else {
        return false;
    };
    let empty = level.fluid_state(pos).create_legacy_block();
    set(level, pos, empty);
    if place_configured_feature(&feature, level, pos, &mut &*random) {
        if level.block_state(pos) == empty {
            level.send_block_updated_with_flags(pos, state, empty, SET_BLOCK_UPDATE_CLIENTS);
        }
        return true;
    }
    set(level, pos, state);
    false
}

fn set(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    level.set_block(
        pos,
        state,
        GROW_TREE_FLAGS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

/// `TreeGrower.hasFlowers`: the 5x3x5 box from one below to one above.
fn has_flowers(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    (-2..=2).any(|x| {
        (-1..=1).any(|y| {
            (-2..=2).any(|z| {
                level.block_state_in_tag(level.block_state(offset(pos, x, y, z)), FLOWERS_TAG)
            })
        })
    })
}
