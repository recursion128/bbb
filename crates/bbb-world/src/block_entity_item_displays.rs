//! Canonical world projection for block entities whose renderer submits item
//! stacks instead of a dedicated model.
//!
//! Vanilla resolves these through `BlockEntityRenderer.extractRenderState` +
//! `ItemModelResolver.updateForTopItem`: the block entity owns the persistent
//! stacks, the block state owns the placement inputs, and the renderer owns the
//! per-slot transform. bbb keeps the same split — this module owns the loaded
//! block-entity facts and the registry-aware stack resolution, native owns the
//! renderer transforms, and the existing single item-model baking path owns the
//! submission.

use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_BRUSHABLE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
        VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID, VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    },
    packets::ItemStackSummary as ProtocolItemStackSummary,
    NbtSummaryValue,
};

use crate::{
    block_name_is_brushable, block_name_is_shelf, chunks::BlockEntityRecord, BlockPos, WorldStore,
};

/// One `ShelfRenderer` render state: the block state's facing plus the occupied
/// display slots and the block entity's bottom-alignment flag.
#[derive(Debug, Clone, PartialEq)]
pub struct ShelfItemDisplaySourceState {
    pub pos: BlockPos,
    /// `ShelfBlock.FACING.get2DDataValue()`; the property is always horizontal.
    pub facing_2d: u8,
    /// `ShelfBlockEntity.getAlignItemsToBottom()`.
    pub align_items_to_bottom: bool,
    /// Occupied `ShelfBlockEntity.getItems()` slots in slot order.
    pub items: Vec<(u8, ProtocolItemStackSummary)>,
}

/// One `CampfireRenderer` render state: the block state's facing plus the
/// occupied cooking slots. Vanilla extracts every slot and skips the empty ones
/// at submit time, so only occupied slots are carried here.
#[derive(Debug, Clone, PartialEq)]
pub struct CampfireItemDisplaySourceState {
    pub pos: BlockPos,
    /// `CampfireBlock.FACING.get2DDataValue()`: south=0, west=1, north=2, east=3.
    pub facing_2d: u8,
    /// Occupied `CampfireBlockEntity.getItems()` slots in slot order.
    pub items: Vec<(u8, ProtocolItemStackSummary)>,
}

/// One `BrushableBlockRenderer` render state. Vanilla submits only when the
/// block has been brushed at least once, the hit side is known and the buried
/// stack is renderable, so this projection carries exactly those cases.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushableItemDisplaySourceState {
    pub pos: BlockPos,
    /// `Direction.get3DDataValue()` of `BrushableBlockEntity.getHitDirection()`.
    pub hit_direction_3d: u8,
    /// `BlockStateProperties.DUSTED` (1..=3 while the item is visible).
    pub dust_progress: i32,
    /// The brushable block's own state id; `BrushableBlockRenderer` samples the
    /// light of the neighbour it was brushed from with this state.
    pub block_state_id: i32,
    pub item: ProtocolItemStackSummary,
}

/// One `VaultRenderer` render state: the ejected reward plus the client-ticked
/// display spin.
#[derive(Debug, Clone, PartialEq)]
pub struct VaultItemDisplaySourceState {
    pub pos: BlockPos,
    pub item: ProtocolItemStackSummary,
    /// `VaultClientData.previousSpin()` / `currentSpin()`.
    pub previous_spin: f32,
    pub current_spin: f32,
}

impl WorldStore {
    /// Projects every loaded campfire block entity that carries at least one
    /// cooking item into its `CampfireRenderer` inputs.
    pub fn campfire_item_display_source_states(&self) -> Vec<CampfireItemDisplaySourceState> {
        let mut states = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID {
                continue;
            }
            let Some(campfire) = record.campfire.as_ref() else {
                continue;
            };
            let Some(block) = self.probe_block(pos) else {
                continue;
            };
            if !matches!(
                block.block_name.as_deref(),
                Some("minecraft:campfire" | "minecraft:soul_campfire")
            ) {
                continue;
            }
            let Some(facing_2d) = block
                .block_properties
                .get("facing")
                .and_then(|facing| horizontal_facing_2d_data_value(facing))
            else {
                continue;
            };
            let items = self.resolved_container_slots(&campfire.items);
            if items.is_empty() {
                continue;
            }
            states.push(CampfireItemDisplaySourceState {
                pos,
                facing_2d,
                items,
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    /// Projects every loaded shelf block entity that carries at least one
    /// display item into its `ShelfRenderer` inputs.
    pub fn shelf_item_display_source_states(&self) -> Vec<ShelfItemDisplaySourceState> {
        let mut states = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID {
                continue;
            }
            let Some(shelf) = record.shelf.as_ref() else {
                continue;
            };
            let Some(block) = self.probe_block(pos) else {
                continue;
            };
            if !block.block_name.as_deref().is_some_and(block_name_is_shelf) {
                continue;
            }
            let Some(facing_2d) = block
                .block_properties
                .get("facing")
                .and_then(|facing| horizontal_facing_2d_data_value(facing))
            else {
                continue;
            };
            let items = self.resolved_container_slots(&shelf.items);
            if items.is_empty() {
                continue;
            }
            states.push(ShelfItemDisplaySourceState {
                pos,
                facing_2d,
                align_items_to_bottom: shelf.align_items_to_bottom,
                items,
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    /// Projects every loaded brushable block whose buried item is visible into
    /// its `BrushableBlockRenderer` inputs.
    pub fn brushable_item_display_source_states(&self) -> Vec<BrushableItemDisplaySourceState> {
        let mut states = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_BRUSHABLE_BLOCK_ID {
                continue;
            }
            let Some(brushable) = record.brushable.as_ref() else {
                continue;
            };
            let Some(hit_direction_3d) = brushable.hit_direction_3d else {
                continue;
            };
            let Some(block) = self.probe_block(pos) else {
                continue;
            };
            if !block
                .block_name
                .as_deref()
                .is_some_and(block_name_is_brushable)
            {
                continue;
            }
            let dust_progress = block
                .block_properties
                .get("dusted")
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(0);
            if dust_progress <= 0 {
                continue;
            }
            let Some(item) = self.resolved_block_entity_item(brushable.item.as_ref()) else {
                continue;
            };
            states.push(BrushableItemDisplaySourceState {
                pos,
                hit_direction_3d,
                dust_progress,
                block_state_id: block.block_state_id,
                item,
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    /// Projects every loaded vault holding a display item into its
    /// `VaultRenderer` inputs. Vanilla gates on
    /// `VaultBlockEntity.Client.shouldDisplayActiveEffects`, which is exactly
    /// `VaultSharedData.hasDisplayItem()`.
    pub fn vault_item_display_source_states(&self) -> Vec<VaultItemDisplaySourceState> {
        let mut states = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID {
                continue;
            }
            let Some(shared) = record.vault_shared_data.as_ref() else {
                continue;
            };
            let Some(item) = self.resolved_block_entity_item(shared.display_item.as_ref()) else {
                continue;
            };
            let spin = self
                .vault_block_states()
                .iter()
                .find(|state| state.pos == pos);
            states.push(VaultItemDisplaySourceState {
                pos,
                item,
                previous_spin: spin.map_or(0.0, |state| state.previous_spin),
                current_spin: spin.map_or(0.0, |state| state.current_spin),
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    /// Resolves the occupied slots of one retained container payload.
    fn resolved_container_slots(
        &self,
        slots: &[Option<NbtSummaryValue>],
    ) -> Vec<(u8, ProtocolItemStackSummary)> {
        slots
            .iter()
            .enumerate()
            .filter_map(|(slot, item)| {
                let stack = self.resolved_block_entity_item(item.as_ref())?;
                Some((slot as u8, stack))
            })
            .collect()
    }

    /// Resolves a retained block-entity stack through the single persistent
    /// `ItemStack.OPTIONAL_CODEC` decoder. Stacks the session registries cannot
    /// resolve stay unrendered, matching Vanilla's empty-stack fallback.
    fn resolved_block_entity_item(
        &self,
        item: Option<&NbtSummaryValue>,
    ) -> Option<ProtocolItemStackSummary> {
        let stack = self.persistent_item_stack_from_nbt(item?).ok()?;
        (stack.item_id.is_some() && stack.count > 0).then_some(stack)
    }

    fn loaded_block_entity_records(&self) -> impl Iterator<Item = (BlockPos, &BlockEntityRecord)> {
        self.chunks.iter().flat_map(|chunk| {
            chunk.block_entities.iter().map(move |record| {
                (
                    BlockPos {
                        x: chunk.pos.x * 16 + i32::from(record.local_x),
                        y: i32::from(record.y),
                        z: chunk.pos.z * 16 + i32::from(record.local_z),
                    },
                    record,
                )
            })
        })
    }
}

/// Vanilla `Direction.get2DDataValue()` for the four horizontal directions.
pub(crate) fn horizontal_facing_2d_data_value(facing: &str) -> Option<u8> {
    match facing {
        "south" => Some(0),
        "west" => Some(1),
        "north" => Some(2),
        "east" => Some(3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID;

    use crate::{
        chunks::{
            decode_campfire_block_entity_data, BlockEntityRecord, ChunkColumn, ChunkSection,
            ChunkState, LightData, PaletteDomain, PaletteKind, PalettedContainerData,
        },
        BlockPos, ChunkPos, WorldDimension, WorldStore,
    };

    #[test]
    fn campfire_display_states_carry_facing_and_resolved_occupied_slots() {
        let pos = BlockPos { x: 3, y: 5, z: 7 };
        let world = campfire_world(pos, "north");
        let states = world.campfire_item_display_source_states();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].pos, pos);
        // `Direction.NORTH.get2DDataValue()`.
        assert_eq!(states[0].facing_2d, 2);
        let items = &states[0].items;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, 2);
        assert_eq!(
            items[0].1.item_id,
            Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:cooked_beef"
                )
                .expect("cooked beef is registered")
                .id
            )
        );
        assert_eq!(items[0].1.count, 2);
    }

    #[test]
    fn campfires_without_a_renderable_stack_are_not_projected() {
        let pos = BlockPos { x: 3, y: 5, z: 7 };
        let world = campfire_world_without_items(pos, "north");
        assert!(world.campfire_item_display_source_states().is_empty());
    }

    fn campfire_world_without_items(pos: BlockPos, facing: &str) -> WorldStore {
        let mut world = campfire_world(pos, facing);
        for chunk in &mut world.chunks {
            for record in &mut chunk.block_entities {
                record.campfire = Some(Default::default());
            }
        }
        world
    }

    fn campfire_world(pos: BlockPos, facing: &str) -> WorldStore {
        let block_state_id = crate::vanilla_block_state_id(
            "minecraft:campfire",
            &BTreeMap::from([
                ("facing".to_string(), facing.to_string()),
                ("lit".to_string(), "true".to_string()),
                ("signal_fire".to_string(), "false".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        )
        .expect("campfire block state is registered");
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
        );
        record.raw_nbt = campfire_nbt();
        record.campfire = decode_campfire_block_entity_data(&record.raw_nbt)
            .expect("valid campfire fixture")
            .map(Into::into);

        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: PalettedContainerData {
                    domain: PaletteDomain::BlockStates,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![block_state_id],
                    packed_data: Vec::new(),
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: vec![record],
            light: LightData::default(),
        });
        store
    }

    /// `ContainerHelper.saveAllItems` shape: one cooked-beef stack in slot 2.
    fn campfire_nbt() -> Vec<u8> {
        let mut out = vec![10];
        push_tag_name(&mut out, 9, "Items");
        out.push(10);
        out.extend_from_slice(&1_i32.to_be_bytes());
        push_tag_name(&mut out, 1, "Slot");
        out.push(2);
        push_tag_name(&mut out, 8, "id");
        push_string_payload(&mut out, "minecraft:cooked_beef");
        push_tag_name(&mut out, 3, "count");
        out.extend_from_slice(&2_i32.to_be_bytes());
        out.push(0);
        out.push(0);
        out
    }

    fn push_tag_name(out: &mut Vec<u8>, tag: u8, name: &str) {
        out.push(tag);
        push_string_payload(out, name);
    }

    fn push_string_payload(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
