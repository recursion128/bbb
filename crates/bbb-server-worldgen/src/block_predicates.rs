//! Vanilla `BlockPredicate.test(WorldGenLevel, BlockPos)` over the
//! `BlockPredicateType` families.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::registry_data_types::VANILLA_FLUID_IDENTITIES;

use bbb_server_datapack::worldgen::{BlockPredicate, HolderSet};
use bbb_server_types::fluid::{block_state_fluid_state, FluidState};

use super::multiface::is_face_sturdy_at;
use super::{
    block_in_holder_set, can_be_replaced, feature_block_can_survive, offset_pos, WorldGenLevel,
};

/// Vanilla `BlockPredicate.test(level, origin)`.
pub fn block_predicate_test<L: WorldGenLevel>(
    predicate: &BlockPredicate,
    level: &L,
    origin: BlockPos,
) -> Option<bool> {
    match predicate {
        BlockPredicate::True => Some(true),
        BlockPredicate::MatchingBlocks { offset, blocks } => Some(block_in_holder_set(
            level,
            level.get_block(offset_pos(origin, *offset)),
            blocks,
        )),
        BlockPredicate::MatchingBlockTag { offset, tag } => {
            Some(level.block_state_in_tag(level.get_block(offset_pos(origin, *offset)), tag))
        }
        // `MatchingFluidsPredicate.test` (`MatchingFluidsPredicate.java:26-28`)
        // through `TypedInstance.is(HolderSet)` (`TypedInstance.java:18-20`).
        BlockPredicate::MatchingFluids { offset, fluids } => {
            let state = level.get_block(offset_pos(origin, *offset));
            fluid_in_holder_set(level, block_state_fluid_state(state), fluids)
        }
        // `WouldSurvivePredicate.test` (`WouldSurvivePredicate.java:25-27`).
        BlockPredicate::WouldSurvive { offset, state } => Some(feature_block_can_survive(
            level,
            offset_pos(origin, *offset),
            *state,
        )),
        // `ReplaceablePredicate.test` (`ReplaceablePredicate.java:16-18`).
        BlockPredicate::Replaceable { offset } => Some(can_be_replaced(
            level,
            level.get_block(offset_pos(origin, *offset)),
        )),
        // `SolidPredicate.test` (`SolidPredicate.java:17-19`) is
        // `BlockStateBase.isSolid`, the `legacySolid` cache bit
        // (`BlockBehaviour.java:546-548`).
        BlockPredicate::Solid { offset } => {
            let state = level.get_block(offset_pos(origin, *offset));
            Some(vanilla_block_state_support_facts(state)?.legacy_solid())
        }
        // `HasSturdyFacePredicate.test` (`HasSturdyFacePredicate.java:26-29`).
        BlockPredicate::HasSturdyFace { offset, direction } => {
            let test_position = offset_pos(origin, *offset);
            Some(is_face_sturdy_at(
                level,
                test_position,
                level.get_block(test_position),
                *direction,
            ))
        }
        // `InsideWorldBoundsPredicate.test` (`InsideWorldBoundsPredicate.java:19-21`)
        // through `LevelHeightAccessor.isInsideBuildHeight`
        // (`LevelHeightAccessor.java:31-33`).
        BlockPredicate::InsideWorldBounds { offset } => {
            let y = offset_pos(origin, *offset).y;
            Some(y >= level.get_min_y() && y <= level.get_max_y())
        }
        BlockPredicate::AllOf(predicates) => {
            for predicate in predicates {
                if !block_predicate_test(predicate, level, origin)? {
                    return Some(false);
                }
            }
            Some(true)
        }
        BlockPredicate::AnyOf(predicates) => {
            for predicate in predicates {
                if block_predicate_test(predicate, level, origin)? {
                    return Some(true);
                }
            }
            Some(false)
        }
        BlockPredicate::Not(predicate) => Some(!block_predicate_test(predicate, level, origin)?),
        BlockPredicate::Unobstructed => Some(level.is_unobstructed(origin)),
    }
}

/// `HolderSet<Fluid>.contains` against the level's published registry tags.
pub fn fluid_in_holder_set<L: WorldGenLevel>(
    level: &L,
    fluid: FluidState,
    fluids: &HolderSet,
) -> Option<bool> {
    let identity = fluid.identity();
    let resource_id = VANILLA_FLUID_IDENTITIES
        .iter()
        .find(|(candidate, _)| *candidate == identity)
        .map(|(_, resource_id)| *resource_id)?;
    Some(match fluids {
        HolderSet::Tag(tag) => level.registry_tag_contains("minecraft:fluid", tag, resource_id),
        HolderSet::Direct(members) => members.iter().any(|member| member == resource_id),
    })
}
