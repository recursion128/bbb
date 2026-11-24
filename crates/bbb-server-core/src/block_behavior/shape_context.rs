//! Shape updates over live blocks, generation regions and saved chunk upgrades.

use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;
use bbb_server_types::block_queries::block_type_of;
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, SavedTick, TickPriority};
use bbb_server_types::fluid::{air_block_state_id, block_state_fluid_state, FluidState};
use bbb_server_worldgen::WorldGenLevel;

use crate::block_getter::{BlockGetter, BlockTags};
use crate::chunk::upgrade_data::ChunkUpgradeLevel;
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::simulation_level::LevelCtx;

pub(crate) enum BlockShapeCtx<'a, 'b, 'w> {
    Live(&'a mut LevelCtx<'b, 'w>),
    Generation {
        level: &'a mut dyn WorldGenLevel,
        random: &'a GameRandom,
    },
    Upgrade(&'a mut ChunkUpgradeLevel<'b>),
}

impl BlockShapeCtx<'_, '_, '_> {
    pub(crate) fn beehive_emergency_release(&mut self, pos: BlockPos, state: i32) {
        if let Self::Live(level) = self {
            level.beehive_emergency_release(pos, state);
        }
    }

    pub(crate) fn get_min_y(&self) -> i32 {
        match self {
            Self::Live(level) => level.get_min_y(),
            Self::Generation { level, .. } => level.get_min_y(),
            Self::Upgrade(level) => level.get_min_y(),
        }
    }

    pub(crate) fn block_state(&self, pos: BlockPos) -> i32 {
        match self {
            Self::Live(level) => level.block_state(pos),
            Self::Generation { level, .. } => level.get_block(pos),
            Self::Upgrade(level) => level.state_at(pos).unwrap_or_else(air_block_state_id),
        }
    }

    pub(crate) fn shulker_box_open(&self, pos: BlockPos) -> bool {
        match self {
            Self::Live(level) => level.shulker_box_open(pos),
            Self::Generation { .. } | Self::Upgrade(_) => false,
        }
    }

    pub(crate) fn block_state_in_tag(&self, state: i32, tag: &str) -> bool {
        match self {
            Self::Live(level) => level.block_state_in_tag(state, tag),
            Self::Upgrade(_) => RegistryTagSnapshot::builtin().block_state_in(tag, state),
            Self::Generation { level, .. } => block_type_of(state)
                .and_then(bbb_protocol::block_types::vanilla_block_type)
                .is_some_and(|block| {
                    level.registry_tag_contains("minecraft:block", tag, block.resource_id)
                }),
        }
    }

    pub(crate) fn registry_tag_membership(&self, registry: &str, tag: &str, id: &str) -> bool {
        match self {
            Self::Live(level) => level.registry_tag_membership(registry, tag, id),
            Self::Generation { level, .. } => level.registry_tag_contains(registry, tag, id),
            Self::Upgrade(_) => RegistryTagSnapshot::builtin().contains(registry, tag, id),
        }
    }

    pub(crate) fn fluid_identity_in_tag(&self, fluid: VanillaFluidIdentity, tag: &str) -> bool {
        self.registry_tag_membership("minecraft:fluid", tag, fluid.resource_id())
    }

    pub(crate) fn fluid_state(&self, pos: BlockPos) -> FluidState {
        block_state_fluid_state(self.block_state(pos))
    }

    pub(crate) fn raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> i32 {
        match self {
            Self::Live(level) => level.raw_brightness(pos, sky_darkening),
            Self::Generation { .. } | Self::Upgrade(_) => 0,
        }
    }

    pub(crate) fn schedule_block_tick(
        &mut self,
        pos: BlockPos,
        block: BlockTickType,
        delay: i32,
        priority: TickPriority,
    ) {
        match self {
            Self::Live(level) => level.schedule_block_tick(pos, block, delay, priority),
            Self::Generation { level, .. } => level.schedule_block_tick(pos, delay),
            Self::Upgrade(level) => {
                level.append_block_tick(SavedTick {
                    tick_type: block,
                    pos,
                    delay,
                    priority,
                });
            }
        }
    }

    pub(crate) fn schedule_fluid_tick(
        &mut self,
        pos: BlockPos,
        fluid: FluidTickType,
        delay: i32,
        priority: TickPriority,
    ) {
        match self {
            Self::Live(level) => level.schedule_fluid_tick(pos, fluid, delay, priority),
            Self::Generation { level, .. } => level.schedule_fluid_tick(pos, delay),
            Self::Upgrade(level) => {
                level.append_fluid_tick(SavedTick {
                    tick_type: fluid,
                    pos,
                    delay,
                    priority,
                });
            }
        }
    }

    pub(crate) fn has_scheduled_tick(&self, pos: BlockPos, block: BlockTickType) -> bool {
        matches!(self, Self::Live(level) if level.has_scheduled_tick(pos, block))
    }

    pub(crate) fn level_random(&mut self) -> &GameRandom {
        match self {
            Self::Live(level) => level.level_random(),
            Self::Generation { random, .. } => random,
            Self::Upgrade(level) => level.level_random(),
        }
    }

    pub(crate) fn get_control_input_signal(
        &self,
        pos: BlockPos,
        direction: Direction,
        only_diodes: bool,
    ) -> i32 {
        if let Self::Live(level) = self {
            return level.get_control_input_signal(pos, direction, only_diodes);
        }
        let mut read = |at| self.block_state(at);
        let mut context = crate::redstone::SignalContext::emitting(&mut read)
            .with_reader(&crate::redstone::SturdySignalSources);
        crate::redstone::get_control_input_signal(pos, direction, only_diodes, &mut context)
    }
}

pub(crate) fn block_tag_matches(
    state: i32,
    tag: &str,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    block_type_of(state)
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .is_some_and(|block| membership("minecraft:block", tag, block.resource_id))
}

pub(super) fn fluid_tag_matches(
    fluid: VanillaFluidIdentity,
    tag: &str,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    membership("minecraft:fluid", tag, fluid.resource_id())
}

impl BlockTags for BlockShapeCtx<'_, '_, '_> {
    fn contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        self.registry_tag_membership(registry_id, tag_id, element_id)
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        match self {
            Self::Live(level) => level.content_registry(),
            Self::Generation { .. } | Self::Upgrade(_) => {
                bbb_protocol::content_registry::ContentRegistry::vanilla()
            }
        }
    }

    fn block_in_tag(&self, block_state_id: i32, tag_id: &str) -> bool {
        self.block_state_in_tag(block_state_id, tag_id)
    }
}

impl BlockGetter for BlockShapeCtx<'_, '_, '_> {
    fn block_state(&self, pos: BlockPos) -> i32 {
        self.block_state(pos)
    }
    fn raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> Option<i32> {
        Some(self.raw_brightness(pos, sky_darkening))
    }
    fn tags(&self) -> &dyn BlockTags {
        self
    }
}

impl crate::redstone::SignalSourceReader for BlockShapeCtx<'_, '_, '_> {
    fn comparator_output(&self, pos: BlockPos) -> i32 {
        match self {
            Self::Live(level) => level.block_entity_output_signal(pos),
            Self::Generation { .. } | Self::Upgrade(_) => 0,
        }
    }

    fn chest_open_count(&self, pos: BlockPos) -> i32 {
        match self {
            Self::Live(level) => level.chest_open_count(pos),
            Self::Generation { .. } | Self::Upgrade(_) => 0,
        }
    }

    fn jukebox_playing(&self, pos: BlockPos) -> bool {
        match self {
            Self::Live(level) => level.jukebox_playing(pos),
            Self::Generation { .. } | Self::Upgrade(_) => false,
        }
    }

    fn shulker_open(&self, pos: BlockPos) -> bool {
        self.shulker_box_open(pos)
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        BlockTags::content_registry(self)
    }

    fn support_face(
        &self,
        state: i32,
        pos: BlockPos,
        direction: Direction,
        support_type: bbb_protocol::block_state_types::VanillaBlockSupportType,
    ) -> bool {
        bbb_server_types::block_queries::face_sturdy_at_with_registry(
            state,
            pos,
            direction,
            support_type,
            BlockTags::content_registry(self),
        )
    }
}
