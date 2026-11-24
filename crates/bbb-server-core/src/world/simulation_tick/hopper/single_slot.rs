//! The composter's `WorldlyContainer` (ComposterBlock.java:367-374) and the
//! jukebox's `ContainerSingleItem` (JukeboxBlockEntity.java:101-150) as hopper,
//! dropper and crafter targets.

use super::{WorldContainer, WorldContainerAccess};
use crate::block_behavior::{composter, jukebox};
use crate::block_entity::BlockEntity;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID;
use bbb_protocol::item_types::VANILLA_ITEM_BONE_MEAL_ID;
use bbb_protocol::packets::{BlockPos, Direction, ItemStackSummary};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

/// `ComposterBlock.getContainer` (:367-374): output at level 8, input below 7,
/// nothing at 7.
fn composter_size(state: i32) -> usize {
    usize::from(composter::fill_level(state) != composter::MAX_LEVEL)
}

fn bone_meal() -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_BONE_MEAL_ID),
        count: 1,
        ..ItemStackSummary::empty()
    })
}

impl WorldContainerAccess<'_> {
    pub(super) fn single_slot_at(&self, pos: BlockPos) -> Option<WorldContainer> {
        let chunks = &self.world.level_by_id(&self.dimension).chunks;
        let state = chunks.block_state_at(pos);
        if composter::is_composter(state) {
            return Some(WorldContainer::Composter { pos, state });
        }
        chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .map(|_| WorldContainer::Jukebox(pos))
    }

    fn jukebox_item(&self, pos: BlockPos) -> RuntimeItemStack {
        self.world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .map(|jukebox| jukebox.item().clone())
            .unwrap_or_default()
    }

    /// Whether the composter is still the one the container was taken from;
    /// the vanilla container object goes stale once it has composted.
    fn composter_current(&self, pos: BlockPos, state: i32) -> bool {
        self.world
            .level_by_id(&self.dimension)
            .chunks
            .block_state_at(pos)
            == state
    }

    pub(super) fn single_slot_size(container: WorldContainer) -> Option<usize> {
        match container {
            WorldContainer::Composter { state, .. } => Some(composter_size(state)),
            WorldContainer::Jukebox(_) => Some(1),
            _ => None,
        }
    }

    pub(super) fn single_slot_faces(
        container: WorldContainer,
        direction: Option<Direction>,
    ) -> Option<Vec<usize>> {
        match container {
            // `getSlotsForFace`: input through the top, output through the bottom.
            WorldContainer::Composter { state, .. } => {
                let face = if composter::fill_level(state) == composter::READY {
                    Direction::Down
                } else {
                    Direction::Up
                };
                Some(if composter_size(state) == 1 && direction == Some(face) {
                    vec![0]
                } else {
                    Vec::new()
                })
            }
            WorldContainer::Jukebox(_) => Some(vec![0]),
            _ => None,
        }
    }

    pub(super) fn single_slot_item(&self, container: WorldContainer) -> Option<RuntimeItemStack> {
        match container {
            WorldContainer::Composter { state, .. } => {
                Some(if composter::fill_level(state) == composter::READY {
                    bone_meal()
                } else {
                    RuntimeItemStack::empty()
                })
            }
            WorldContainer::Jukebox(pos) => Some(self.jukebox_item(pos)),
            _ => None,
        }
    }

    /// `setItem` followed by the `setChanged` that acts on it.
    pub(super) fn set_single_slot_item(
        &mut self,
        container: WorldContainer,
        item: RuntimeItemStack,
    ) -> Option<()> {
        match container {
            WorldContainer::Composter { pos, state } => {
                if !self.composter_current(pos, state) {
                    return Some(());
                }
                let fill = composter::fill_level(state);
                if fill < composter::MAX_LEVEL && !item.is_empty() {
                    // `InputContainer.setChanged` (:431-439).
                    let item_id = item.item_id().unwrap_or_default();
                    self.world.with_level(self.dimension, |level| {
                        let next = composter::add_item(level, pos, state, item_id, None);
                        level.level_event(
                            composter::FILL_LEVEL_EVENT,
                            pos,
                            i32::from(next != state),
                        );
                    });
                } else if fill == composter::READY && item.is_empty() {
                    // `OutputContainer.setChanged` (:476-479).
                    self.world.with_level(self.dimension, |level| {
                        composter::empty(level, pos, state, None);
                    });
                }
                Some(())
            }
            WorldContainer::Jukebox(pos) => {
                let state = self
                    .world
                    .level_by_id(&self.dimension)
                    .chunks
                    .block_state_at(pos);
                if let Some(next) = jukebox::with_has_record(state, !item.is_empty()) {
                    self.world
                        .set_the_jukebox_item(self.dimension, pos, next, item);
                }
                Some(())
            }
            _ => None,
        }
    }

    pub(super) fn single_slot_can_place(
        &self,
        container: WorldContainer,
        item: &RuntimeItemStack,
        direction: Option<Direction>,
    ) -> Result<Option<bool>> {
        Ok(match container {
            WorldContainer::Composter { pos, state } => Some(
                direction == Some(Direction::Up)
                    && composter::fill_level(state) < composter::MAX_LEVEL
                    && composter::is_compostable(item.item_id())
                    && self.composter_current(pos, state),
            ),
            // `JukeboxBlockEntity.canPlaceItem` (:143-145).
            WorldContainer::Jukebox(pos) => Some(
                item.has_component(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)?
                    && self.jukebox_item(pos).is_empty(),
            ),
            _ => None,
        })
    }

    pub(super) fn single_slot_can_take(
        &mut self,
        container: WorldContainer,
        into: WorldContainer,
    ) -> Result<Option<bool>> {
        Ok(match container {
            WorldContainer::Composter { pos, state } => Some(
                composter::fill_level(state) == composter::READY
                    && self.composter_current(pos, state),
            ),
            // `JukeboxBlockEntity.canTakeItem` (:148-150).
            WorldContainer::Jukebox(_) => {
                for slot in 0..self.size(into) {
                    if self.get_item(into, slot)?.is_empty() {
                        return Ok(Some(true));
                    }
                }
                Some(false)
            }
            _ => None,
        })
    }
}
