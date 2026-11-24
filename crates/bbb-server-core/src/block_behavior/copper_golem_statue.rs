//! `CopperGolemStatueBlock` (`CopperGolemStatueBlock.java`) identity,
//! pose cycle and analog.

use super::{BlockShapeCtx, LevelCtx};
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::fluid::{with_property, FlowingFluid};
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_state_types::{
    vanilla_block_property_values, vanilla_block_state_property,
};
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_HONEYCOMB_ID};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::opposite;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name, BlockStateAt,
};

const FACING: &str = "facing";
const POSE: &str = "copper_golem_pose";
const WATERLOGGED: &str = "waterlogged";
/// `SoundEvents.COPPER_GOLEM_BECOME_STATUE` (`SoundEvents.java:429`).
pub const POSE_SOUND: &str = "minecraft:entity.copper_golem_become_statue";

/// Either `CopperGolemStatueBlock.useItemOn` or the weathering subclass.
#[must_use]
pub fn is_copper_golem_statue(block_state_id: i32) -> bool {
    matches!(
        block_type_of(block_state_id)
            .and_then(vanilla_block_type)
            .and_then(|entry| entry.use_item_on_override),
        Some(
            VanillaBlockUseItemOnOverride::CopperGolemStatueBlock
                | VanillaBlockUseItemOnOverride::WeatheringCopperGolemStatueBlock
        )
    )
}

/// `WeatheringCopperGolemStatueBlock.useItemOn` (`WeatheringCopperGolemStatueBlock.java`).
#[must_use]
pub fn is_weathering(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::WeatheringCopperGolemStatueBlock)
}

/// `ItemTags.AXES` (`CopperGolemStatueBlock.java`).
#[must_use]
pub fn item_is_axe(item_id: Option<i32>) -> bool {
    item_id.is_some_and(|id| axe_item_ids().contains(&id))
}

/// `Items.HONEYCOMB` (`WeatheringCopperGolemStatueBlock.java`).
#[must_use]
pub fn item_is_honeycomb(item_id: Option<i32>) -> bool {
    item_id == Some(VANILLA_ITEM_HONEYCOMB_ID)
}

/// `CopperGolemStatueBlock.getAnalogOutputSignal` (`:146-148`): pose ordinal + 1.
#[must_use]
pub fn analog_output(block_state_id: i32) -> i32 {
    let Some(values) = pose_values(block_state_id) else {
        return 0;
    };
    let Some(pose) = pose(block_state_id) else {
        return 0;
    };
    values
        .iter()
        .position(|value| *value == pose)
        .and_then(|index| i32::try_from(index).ok())
        .map_or(0, |ordinal| ordinal + 1)
}

/// `CopperGolemStatueBlock.Pose.getNextPose` (`:204-206`) via `BY_ID` wrap.
#[must_use]
pub fn next_state(block_state_id: i32) -> Option<i32> {
    let values = pose_values(block_state_id)?;
    if values.is_empty() {
        return None;
    }
    let pose = pose(block_state_id)?;
    let index = values.iter().position(|value| *value == pose)?;
    let next = values[(index + 1) % values.len()];
    with_property(block_state_id, POSE, next)
}

#[must_use]
pub fn pose(block_state_id: i32) -> Option<&'static str> {
    vanilla_block_state_property(block_state_id, POSE)
}

/// `CopperGolemStatueBlock.getStateForPlacement`
/// (`CopperGolemStatueBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);
    if matches!(horizontal_direction, Direction::Down | Direction::Up) {
        return None;
    }

    let state = default_block_state_id(block_type_id);
    if !is_copper_golem_statue(state) || pose(state)? != "standing" {
        return None;
    }
    let state = with_property(
        state,
        FACING,
        direction_serialized_name(opposite(horizontal_direction)),
    )?;
    with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}

/// `CopperGolemStatueBlock.affectNeighborsAfterRemoval` (`:158-160`).
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbour_for_output_signal(pos, block_type_id);
}

/// `CopperGolemStatueBlock.updateShape` (`CopperGolemStatueBlock.java`).
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

fn pose_values(block_state_id: i32) -> Option<&'static [&'static str]> {
    let resource_id = vanilla_block_type(block_type_of(block_state_id)?)?.resource_id;
    vanilla_block_property_values(resource_id, POSE)
}

fn axe_item_ids() -> &'static HashSet<i32> {
    static IDS: OnceLock<HashSet<i32>> = OnceLock::new();
    IDS.get_or_init(|| {
        let PersistentRegistryTagMembers::Present(members) =
            vanilla_persistent_registry_tag_members_26_1("minecraft:item", "minecraft:axes")
        else {
            return HashSet::new();
        };
        members
            .iter()
            .filter_map(|member| vanilla_item_type_for_resource_id(member).map(|entry| entry.id))
            .collect()
    })
}
