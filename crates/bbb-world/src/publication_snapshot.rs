//! Immutable publication sharing for semantically owned world payloads.

use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::WorldStore;

/// Serde-transparent structural owner used only inside [`WorldStore`].
///
/// Cloning retains the immutable payload. Any mutation through `DerefMut`
/// first obtains a unique value, so existing field-level mutation boundaries
/// cannot accidentally modify an older published generation.
pub(crate) struct PublicationShared<T>(Arc<T>);

impl<T> PublicationShared<T> {
    #[cfg(test)]
    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> From<T> for PublicationShared<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> Clone for PublicationShared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Default> Default for PublicationShared<T> {
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T> Deref for PublicationShared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> DerefMut for PublicationShared<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<T: fmt::Debug> fmt::Debug for PublicationShared<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<T: PartialEq> PartialEq for PublicationShared<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Eq> Eq for PublicationShared<T> {}

impl<T: Serialize> Serialize for PublicationShared<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for PublicationShared<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::from)
    }
}

impl WorldStore {
    /// Builds the immutable world value carried by one simulation publication.
    ///
    /// Chunk columns and session-static tables retain their structural owners;
    /// the entity ECS and other per-generation client state remain explicit
    /// value snapshots so their costs stay at publication rather than moving to
    /// the next simulation mutation.
    pub fn publication_snapshot(&self) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        packets::{
            AddEntity, BlockPos as ProtocolBlockPos, BlockUpdate, EntityDataValue,
            EntityDataValueKind, EntityMove, ItemStackSummary, LightUpdate, LightUpdateData,
            SetEntityData, SetPlayerInventory, Vec3d,
        },
    };

    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    use super::*;

    fn single_value_palette(domain: PaletteDomain, entry_count: usize) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![0],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn test_chunk(pos: ChunkPos, light: u8) -> ChunkColumn {
        ChunkColumn {
            pos,
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_palette(PaletteDomain::BlockStates, 4_096),
                biomes: single_value_palette(PaletteDomain::Biomes, 64),
            }],
            block_entities: Vec::new(),
            light: LightData {
                sky_y_mask: vec![1],
                block_y_mask: Vec::new(),
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: vec![vec![light; 2_048]],
                block_updates: Vec::new(),
            },
        }
    }

    fn chunk_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(test_chunk(ChunkPos { x: 0, z: 0 }, 1));
        world.insert_decoded_chunk(test_chunk(ChunkPos { x: 1, z: 0 }, 3));
        world
    }

    fn add_test_entity(world: &mut WorldStore, id: i32) {
        world.apply_add_entity(AddEntity {
            id,
            uuid: uuid::Uuid::from_u128(id as u128 + 1),
            entity_type_id: 150,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: 2.0,
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    #[test]
    fn publication_shares_unchanged_columns_and_session_static_owners() {
        let mut world = chunk_world();
        world.set_default_item_max_stack_sizes([(1, 16)].into());
        let generation_a = world.publication_snapshot();

        assert!(world.registries.ptr_eq(&generation_a.registries));
        assert!(world
            .items
            .default_item_max_stack_sizes
            .ptr_eq(&generation_a.items.default_item_max_stack_sizes));
        assert!(world.chunks[0].ptr_eq(&generation_a.chunks[0]));
        assert!(world.chunks[1].ptr_eq(&generation_a.chunks[1]));

        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: 1,
        }));

        assert!(!world.chunks[0].ptr_eq(&generation_a.chunks[0]));
        assert!(world.chunks[1].ptr_eq(&generation_a.chunks[1]));
        assert!(world.registries.ptr_eq(&generation_a.registries));
        assert!(world
            .items
            .default_item_max_stack_sizes
            .ptr_eq(&generation_a.items.default_item_max_stack_sizes));

        world.record_registry("benchmark:reload", 0);
        world.set_default_item_max_stack_sizes([(1, 32)].into());
        assert!(!world.registries.ptr_eq(&generation_a.registries));
        assert!(!world
            .items
            .default_item_max_stack_sizes
            .ptr_eq(&generation_a.items.default_item_max_stack_sizes));
        assert!(world.chunks[1].ptr_eq(&generation_a.chunks[1]));
    }

    #[test]
    fn retained_generation_isolated_from_block_light_load_and_unload_mutations() {
        let mut world = chunk_world();
        let generation_a = world.publication_snapshot();

        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: 1,
        }));
        assert!(world
            .apply_light_update(LightUpdate {
                chunk_x: 0,
                chunk_z: 0,
                light_data: LightUpdateData {
                    sky_y_mask: vec![1],
                    block_y_mask: Vec::new(),
                    empty_sky_y_mask: Vec::new(),
                    empty_block_y_mask: Vec::new(),
                    sky_updates: vec![vec![2; 2_048]],
                    block_updates: Vec::new(),
                },
            })
            .unwrap());
        world.insert_decoded_chunk(test_chunk(ChunkPos { x: 2, z: 0 }, 4));
        assert!(world.forget_chunk(ChunkPos { x: 1, z: 0 }));
        let generation_b = world.publication_snapshot();

        assert_eq!(
            generation_a
                .probe_block(crate::BlockPos { x: 0, y: 0, z: 0 })
                .unwrap()
                .block_state_id,
            0
        );
        assert_eq!(
            generation_b
                .probe_block(crate::BlockPos { x: 0, y: 0, z: 0 })
                .unwrap()
                .block_state_id,
            1
        );
        assert_eq!(
            generation_a
                .probe_chunk(ChunkPos { x: 0, z: 0 })
                .unwrap()
                .light
                .sky_updates[0][0],
            1
        );
        assert_eq!(
            generation_b
                .probe_chunk(ChunkPos { x: 0, z: 0 })
                .unwrap()
                .light
                .sky_updates[0][0],
            2
        );
        assert!(generation_a.probe_chunk(ChunkPos { x: 1, z: 0 }).is_some());
        assert!(generation_b.probe_chunk(ChunkPos { x: 1, z: 0 }).is_none());
        assert!(generation_a.probe_chunk(ChunkPos { x: 2, z: 0 }).is_none());
        assert!(generation_b.probe_chunk(ChunkPos { x: 2, z: 0 }).is_some());
    }

    #[test]
    fn retained_generation_isolated_from_entity_transform_and_metadata_mutations() {
        let mut world = WorldStore::new();
        add_test_entity(&mut world, 7);
        let generation_a = world.publication_snapshot();

        assert!(world.apply_entity_move(EntityMove {
            id: 7,
            has_position: true,
            delta_x: 4_096,
            delta_y: 0,
            delta_z: 0,
            y_rot: Some(45.0),
            x_rot: None,
            on_ground: true,
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![EntityDataValue {
                data_id: crate::VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(0x20),
            }],
        }));
        let generation_b = world.publication_snapshot();

        let entity_a = generation_a.probe_entity(7).unwrap();
        let entity_b = generation_b.probe_entity(7).unwrap();
        assert_eq!(entity_a.position_base.x, 1.0);
        assert_eq!(entity_b.position_base.x, 2.0);
        assert!(entity_a.data_values.is_empty());
        assert_eq!(entity_b.data_values.len(), 1);
        assert_eq!(
            entity_b.data_values[0].value,
            EntityDataValueKind::Byte(0x20)
        );
    }

    #[test]
    fn retained_generation_isolated_from_registry_reload_and_item_profile_mutations() {
        let mut world = WorldStore::new();
        world.record_registry("benchmark:before", 1);
        world.set_default_item_max_stack_sizes([(4, 16)].into());
        let generation_a = world.publication_snapshot();
        let old_session = generation_a.registry_access_revision().session;

        world.reset_registry_session();
        world.record_registry("benchmark:after", 2);
        world.set_default_item_max_stack_sizes([(4, 32)].into());
        let generation_b = world.publication_snapshot();

        assert_eq!(generation_a.registry_access_revision().session, old_session);
        assert_eq!(
            generation_b.registry_access_revision().session,
            old_session + 1
        );
        assert_eq!(
            generation_a.registries().registries[0].name,
            "benchmark:before"
        );
        assert_eq!(
            generation_b.registries().registries[0].name,
            "benchmark:after"
        );
        assert_eq!(generation_a.item_max_stack_size_for_protocol_id(4), 16);
        assert_eq!(generation_b.item_max_stack_size_for_protocol_id(4), 32);
    }

    #[test]
    fn retained_generation_isolated_from_inventory_and_ui_mutations() {
        let mut world = WorldStore::new();
        let generation_a = world.publication_snapshot();

        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: stack(5, 3),
        }));
        world.apply_low_disk_space_warning();
        let generation_b = world.publication_snapshot();

        assert!(generation_a.inventory().player_slots.is_empty());
        assert_eq!(
            generation_b.inventory().player_slots[0].item.item_id,
            Some(5)
        );
        assert_eq!(generation_a.client_ui().low_disk_space_warning_count, 0);
        assert_eq!(generation_b.client_ui().low_disk_space_warning_count, 1);
    }

    #[test]
    fn structural_owners_preserve_world_store_json_shape_and_round_trip() {
        let mut world = chunk_world();
        world.record_registry("benchmark:registry", 9);
        world.set_furnace_fuel_profile([3].into(), vec![crate::FurnaceFuelRule::AddItem(3)]);

        let json = serde_json::to_value(&world).unwrap();
        assert!(json["registries"].is_object());
        assert!(json["chunks"].is_array());
        assert!(json["chunks"][0]["sections"].is_array());
        assert!(json["default_item_max_stack_sizes"].is_object());
        assert!(json["furnace_fuel_item_ids"].is_array());
        assert!(json["furnace_fuel_rules"].is_array());
        assert!(json.get("value").is_none());
        assert!(json.get("inner").is_none());

        let restored: WorldStore = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), json);
    }
}
