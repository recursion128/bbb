//! Reads of one level's block store for commands and loot.

use crate::block_behavior::is_base_rail_block;
use crate::chunk::ChunkStore;
use bbb_protocol::packets::BlockPos;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use std::cell::{Ref, RefCell, RefMut};
use std::sync::Arc;

use super::{HORIZONTAL_LIMIT, SET_BLOCK_UPDATE_MOVE_BY_PISTON, SET_BLOCK_UPDATE_NEIGHBORS};

pub(super) fn is_in_world_bounds(chunks: &ChunkStore, pos: BlockPos) -> bool {
    chunks.is_inside_build_height(pos.y)
        && (-HORIZONTAL_LIMIT..HORIZONTAL_LIMIT).contains(&pos.x)
        && (-HORIZONTAL_LIMIT..HORIZONTAL_LIMIT).contains(&pos.z)
}

pub(crate) fn removal_affects_neighbors(
    old_state: i32,
    new_state: i32,
    flags: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bool {
    let old_type =
        bbb_server_types::block_queries::block_type_of_with_registry(old_state, content_registry);
    let new_type =
        bbb_server_types::block_queries::block_type_of_with_registry(new_state, content_registry);
    let block_changed = old_type != new_type;
    let moved_by_piston = flags & SET_BLOCK_UPDATE_MOVE_BY_PISTON != 0;
    (block_changed || is_base_rail_block(new_state))
        && (flags & SET_BLOCK_UPDATE_NEIGHBORS != 0 || moved_by_piston)
}

pub(crate) fn stored_block_state(chunks: &ChunkStore, pos: BlockPos) -> i32 {
    if !is_in_world_bounds(chunks, pos) {
        return chunks.air_state_id();
    }
    chunks
        .get_block_state(pos.x, pos.y, pos.z)
        .or_else(|| chunks.generated_block_state(pos.x, pos.y, pos.z))
        .unwrap_or_else(|| chunks.air_state_id())
}

/// The two level reads a loot table's `location_check` makes, as
/// [`crate::loot_runtime`] asks for them.
///
/// It is its own type rather than an impl on [`LevelCtx`](super::LevelCtx)
/// because the drop needs the block store while the loot generator and the
/// outbox are lent out of the same slice; borrowing one field for the read
/// leaves the rest free.
#[derive(Debug)]
pub(super) struct LoadedBlocks<'a> {
    pub(super) chunks: &'a ChunkStore,
}

/// The block reads commands make through `CommandSourceStack.getLevel()`.
impl ChunkStore {
    /// `LevelReader.hasChunkAt(pos)` (BlockPosArgument.java:39): a column
    /// nothing has loaded is refused rather than generated.
    pub(crate) fn has_chunk_at(&self, pos: BlockPos) -> bool {
        self.is_ticking_full(pos.x >> 4, pos.z >> 4)
    }

    /// `Level.isInWorldBounds(pos)` (BlockPosArgument.java:41).
    pub(crate) fn is_in_world_bounds(&self, pos: BlockPos) -> bool {
        is_in_world_bounds(self, pos)
    }

    /// `Level.getBlockState(pos)` as a global block-state id.
    pub(crate) fn block_state_at(&self, pos: BlockPos) -> i32 {
        stored_block_state(self, pos)
    }

    /// `Level.getHeight(type, x, z)` (`Level.java:340-352`): the heightmap's
    /// first-available Y, `None` for a column without a final heightmap.
    pub(crate) fn height_at(&self, heightmap_kind_id: i32, x: i32, z: i32) -> Option<i32> {
        self.final_heightmap_first_available(x >> 4, z >> 4, heightmap_kind_id, x & 15, z & 15)
    }

    pub(crate) fn block_entity_loot_data(
        &self,
        read: crate::loot_runtime::LootBlockEntityRead,
        resolver: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
    ) -> bbb_server_types::Result<Option<crate::loot_runtime::LootBlockEntityData>> {
        use crate::loot_runtime::{LootBlockEntityData, LootBlockEntityRead};
        match read {
            LootBlockEntityRead::Nbt(pos) => self
                .block_entity_command_data(pos, resolver)
                .map(|value| value.map(LootBlockEntityData::Nbt)),
            LootBlockEntityRead::Components(pos) => {
                let Some(body) = self.block_entities().get(pos) else {
                    return Ok(None);
                };
                let resolver = resolver.ok_or_else(|| {
                    bbb_server_types::ServerWorldError::InvalidConfiguration(
                        "block components publication is unavailable".into(),
                    )
                })?;
                Ok(Some(LootBlockEntityData::Components(
                    body.collect_components(
                        crate::block_entity::BlockEntityItemProjection::with_resolver(resolver),
                    ),
                )))
            }
        }
    }

    /// `BlockEntity.saveWithFullMetadata` (`BlockDataAccessor.java:70-72`)
    /// through the registry publication the level's commands hold.
    pub(crate) fn block_entity_command_data(
        &self,
        pos: BlockPos,
        resolver: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
    ) -> bbb_server_types::Result<Option<bbb_protocol::NbtSummaryValue>> {
        let Some(block_entity) = self.block_entities().get(pos) else {
            return Ok(None);
        };
        Ok(Some(match resolver {
            Some(resolver) => block_entity.command_data_with_resolver(resolver),
            None => block_entity.command_data(),
        }?))
    }
}

impl LoadedBlocks<'_> {
    pub(super) fn loaded_block_state(&self, pos: BlockPos) -> Option<i32> {
        // `ServerLevel.isLoaded(pos)`: a column nothing brought in is the
        // absent answer, not a generated one, because vanilla's predicate is
        // whether a chunk is there and not what the generator would make.
        if !is_in_world_bounds(self.chunks, pos) || !self.chunks.is_loaded(pos.x >> 4, pos.z >> 4) {
            return None;
        }
        Some(stored_block_state(self.chunks, pos))
    }

    pub(super) fn loaded_structure_matches(
        &self,
        pos: BlockPos,
        expected: &bbb_server_datapack::loot::HolderSet,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Option<bool> {
        self.loaded_block_state(pos)?;
        if !resolver.registry_available("minecraft:worldgen/structure") {
            return None;
        }
        let target = self.chunks.structure_component(pos.x >> 4, pos.z >> 4)?;
        let mut answer = self.piece_matches(target, pos, expected, resolver, &[]);
        if !answer.matches && !answer.missing.is_empty() {
            // Start columns no one has read answer from their stored record.
            let stored = std::mem::take(&mut answer.missing)
                .into_iter()
                .filter_map(|(x, z)| Some(((x, z), self.chunks.stored_structure_component(x, z)?)))
                .collect::<Vec<_>>();
            if !stored.is_empty() {
                answer = self.piece_matches(target, pos, expected, resolver, &stored);
            }
        }
        if answer.matches {
            Some(true)
        } else if answer.unknown_tag || !answer.missing.is_empty() {
            None
        } else {
            Some(false)
        }
    }

    fn piece_matches(
        &self,
        target: &NoiseStructureChunkData,
        pos: BlockPos,
        expected: &bbb_server_datapack::loot::HolderSet,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        stored: &[((i32, i32), Arc<NoiseStructureChunkData>)],
    ) -> PieceMatch {
        let mut unknown_tag = false;
        let mut missing = Vec::new();
        let matches = target.referenced_piece_contains(
            pos,
            |id| {
                expected
                    .contains(id, |tag| {
                        resolver
                            .tag_contains("minecraft:worldgen/structure", tag, id)
                            .ok_or(())
                    })
                    .unwrap_or_else(|()| {
                        unknown_tag = true;
                        false
                    })
            },
            |x, z| {
                let source = self
                    .chunks
                    .structure_component_or_unloaded(x, z)
                    .or_else(|| {
                        stored
                            .iter()
                            .find(|(chunk, _)| *chunk == (x, z))
                            .map(|(_, data)| &**data)
                    });
                if source.is_none() {
                    // A cold saved source needs hydration before its metadata can answer.
                    self.chunks.cold_column(x, z);
                    missing.push((x, z));
                }
                source
            },
        );
        PieceMatch {
            matches,
            unknown_tag,
            missing,
        }
    }

    /// `LevelReader.canSeeSky`: full sky light, on a loaded position.
    pub(super) fn loaded_can_see_sky(&self, pos: BlockPos) -> Option<bool> {
        self.loaded_block_state(pos)?;
        Some(self.chunks.sky_light_level(pos.x, pos.y, pos.z) >= 15)
    }
}

struct PieceMatch {
    matches: bool,
    unknown_tag: bool,
    missing: Vec<(i32, i32)>,
}

#[derive(Clone, Copy)]
pub(in crate::world) enum LootChunks<'s, 'a, 'w> {
    Shared(&'s ChunkStore),
    Chunks(&'s RefCell<&'a mut ChunkStore>),
    Level(&'s RefCell<super::simulation_level::LevelCtx<'a, 'w>>),
}

pub(in crate::world) enum LootChunkRead<'a> {
    Shared(&'a ChunkStore),
    Borrowed(Ref<'a, ChunkStore>),
}

impl std::ops::Deref for LootChunkRead<'_> {
    type Target = ChunkStore;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Shared(chunks) => chunks,
            Self::Borrowed(chunks) => chunks,
        }
    }
}

impl LootChunks<'_, '_, '_> {
    pub(in crate::world) fn borrow(&self) -> LootChunkRead<'_> {
        match self {
            Self::Shared(chunks) => LootChunkRead::Shared(chunks),
            Self::Chunks(chunks) => {
                LootChunkRead::Borrowed(Ref::map(chunks.borrow(), |chunks| &**chunks))
            }
            Self::Level(level) => {
                LootChunkRead::Borrowed(Ref::map(level.borrow(), |level| &*level.chunks))
            }
        }
    }

    pub(in crate::world) fn borrow_mut(&self) -> Option<RefMut<'_, ChunkStore>> {
        Some(match self {
            Self::Shared(_) => return None,
            Self::Chunks(chunks) => RefMut::map(chunks.borrow_mut(), |chunks| &mut **chunks),
            Self::Level(level) => RefMut::map(level.borrow_mut(), |level| level.chunks),
        })
    }
}
