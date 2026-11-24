//! What a level queues for the driver because it cannot reach the entity store
//! itself: the item entities of `Block.popResource`, the contents a retired
//! block entity's `preRemoveSideEffects` handed back, and the mob, falling
//! block and TNT requests a block body raised.

use crate::block_entity::BlockEntityRemovalDrops;
use crate::entity::block_drops::PendingItemSpawn;
use crate::level_event::{VanillaGameEvent, LEVEL_EVENT_SOUND_STOP_JUKEBOX_SONG};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use super::{BlockMobSpawnRequest, FallingBlockFallRequest, GolemSpawnRequest, LevelCtx};

#[derive(Debug, Clone)]
pub(in crate::world) struct PendingContainerDrop {
    /// The block position `Containers.dropContents(level, pos, container)` was
    /// handed, as the three doubles `dropItemStack` floors.
    pub(in crate::world) position: Vec3d,
    /// Slot-aligned, empty slots included.
    pub(in crate::world) slots: Vec<Option<RuntimeItemStack>>,
    /// How many plain item spawns preceded this drop, so the drain can run the
    /// two queues in the order the level filled them.
    pub(in crate::world) items_before: usize,
}

/// A retired block entity's `preRemoveSideEffects` override that reaches the
/// entity store or the session list rather than the item queue.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::world) enum PendingBlockEntityRemovalEffect {
    SculkShrieker {
        position: BlockPos,
        warning_level: i32,
        can_summon: bool,
    },
    CreakingProtector {
        uuid: Uuid,
    },
    FurnaceExperience {
        position: Vec3d,
        recipes_used: Vec<(String, i32)>,
    },
}

#[derive(Default)]
pub(in crate::world) struct PendingEntitySpawns {
    pub(in crate::world) items: Vec<PendingItemSpawn>,
    pub(in crate::world) container_drops: Vec<PendingContainerDrop>,
    pub(in crate::world) block_mobs: Vec<BlockMobSpawnRequest>,
    pub(in crate::world) pattern_mobs: Vec<GolemSpawnRequest>,
    pub(in crate::world) tnt_primes: Vec<BlockPos>,
    pub(in crate::world) falling_blocks: Vec<FallingBlockFallRequest>,
    pub(in crate::world) removal_effects: Vec<PendingBlockEntityRemovalEffect>,
    /// Structure-placed mobs staged in the store, published once the world
    /// has finalized them.
    pub(in crate::world) structure_mobs: Vec<crate::entity::Entity>,
    pub(in crate::world) worldgen_entities:
        Vec<crate::world::chunk_lifecycle::full_preparation::PreparedWorldgenEntityBatch>,
    /// `Block.pushEntitiesUp` requests as (old state, new state, position);
    /// only the tick drains that hold the events apply them.
    pub(in crate::world) entity_push_ups: Vec<(i32, i32, BlockPos)>,
}

impl PendingEntitySpawns {
    #[must_use]
    pub(in crate::world) fn has_pending(&self) -> bool {
        !self.items.is_empty()
            || !self.container_drops.is_empty()
            || !self.block_mobs.is_empty()
            || !self.pattern_mobs.is_empty()
            || !self.tnt_primes.is_empty()
            || !self.falling_blocks.is_empty()
            || !self.removal_effects.is_empty()
            || !self.structure_mobs.is_empty()
            || !self.worldgen_entities.is_empty()
            || !self.entity_push_ups.is_empty()
    }
}

impl std::ops::Deref for PendingEntitySpawns {
    type Target = Vec<PendingItemSpawn>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl std::ops::DerefMut for PendingEntitySpawns {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl LevelCtx<'_, '_> {
    /// `BlockEntity.preRemoveSideEffects`'s plan turned into spawn requests:
    /// `Containers.dropContents(level, pos, container)` for the container
    /// family, the lectern book and the jukebox record at their own offsets.
    ///
    pub(in crate::world) fn queue_block_entity_removal_drops(
        &mut self,
        pos: BlockPos,
        drops: BlockEntityRemovalDrops,
    ) {
        match drops {
            BlockEntityRemovalDrops::None => {}
            BlockEntityRemovalDrops::ContainerSlots(slots) => {
                self.item_spawns.container_drops.push(PendingContainerDrop {
                    position: Vec3d {
                        x: f64::from(pos.x),
                        y: f64::from(pos.y),
                        z: f64::from(pos.z),
                    },
                    slots: physical_original_container_drop_stacks(&slots),
                    items_before: self.item_spawns.items.len(),
                });
            }
            BlockEntityRemovalDrops::LecternBook { stack, offset } => {
                let position = Vec3d {
                    x: f64::from(pos.x) + offset[0],
                    y: f64::from(pos.y) + offset[1],
                    z: f64::from(pos.z) + offset[2],
                };
                if let Some(item) = physical_drop_stack(&stack) {
                    self.item_spawns.items.push(PendingItemSpawn {
                        position,
                        item,
                        default_pickup_delay: true,
                    });
                }
            }
            BlockEntityRemovalDrops::SculkShriekerResponse {
                warning_level,
                can_summon,
            } => {
                self.item_spawns.removal_effects.push(
                    PendingBlockEntityRemovalEffect::SculkShrieker {
                        position: pos,
                        warning_level,
                        can_summon,
                    },
                );
            }
            BlockEntityRemovalDrops::Furnace {
                slots,
                recipes_used,
            } => {
                self.item_spawns.container_drops.push(PendingContainerDrop {
                    position: Vec3d {
                        x: f64::from(pos.x),
                        y: f64::from(pos.y),
                        z: f64::from(pos.z),
                    },
                    slots: physical_original_container_drop_stacks(&slots),
                    items_before: self.item_spawns.items.len(),
                });
                self.item_spawns.removal_effects.push(
                    PendingBlockEntityRemovalEffect::FurnaceExperience {
                        position: Vec3d {
                            x: f64::from(pos.x) + 0.5,
                            y: f64::from(pos.y) + 0.5,
                            z: f64::from(pos.z) + 0.5,
                        },
                        recipes_used,
                    },
                );
            }
            BlockEntityRemovalDrops::CreakingHeartProtector { uuid } => {
                self.item_spawns
                    .removal_effects
                    .push(PendingBlockEntityRemovalEffect::CreakingProtector { uuid });
            }
            BlockEntityRemovalDrops::Jukebox {
                state,
                record,
                stops_song,
            } => {
                if stops_song {
                    self.jukebox_stop_events(pos, state);
                }
                if let Some(stack) = record {
                    let dx = (self.random.next_float() - 0.5) * 0.7;
                    let dz = (self.random.next_float() - 0.5) * 0.7;
                    let position = Vec3d {
                        x: f64::from(pos.x) + 0.5 + f64::from(dx),
                        y: f64::from(pos.y) + 1.01,
                        z: f64::from(pos.z) + 0.5 + f64::from(dz),
                    };
                    if let Some(item) = physical_drop_stack(&stack) {
                        self.item_spawns.items.push(PendingItemSpawn {
                            position,
                            item,
                            default_pickup_delay: true,
                        });
                    }
                }
                self.jukebox_stop_events(pos, state);
            }
        }
    }
}

impl LevelCtx<'_, '_> {
    fn jukebox_stop_events(&mut self, pos: BlockPos, state: i32) {
        self.block_game_event(VanillaGameEvent::JUKEBOX_STOP_PLAY, pos, Some(state));
        self.level_event(LEVEL_EVENT_SOUND_STOP_JUKEBOX_SONG, pos, 0);
    }
}

pub(in crate::world) fn physical_container_drop_stacks(
    slots: &[RuntimeItemStack],
) -> Vec<Option<RuntimeItemStack>> {
    slots.iter().map(physical_drop_stack).collect()
}

fn physical_original_container_drop_stacks(
    slots: &[RuntimeItemStack],
) -> Vec<Option<RuntimeItemStack>> {
    slots.iter().cloned().map(Some).collect()
}

fn physical_drop_stack(stack: &RuntimeItemStack) -> Option<RuntimeItemStack> {
    (!stack.is_empty()).then(|| stack.clone())
}
