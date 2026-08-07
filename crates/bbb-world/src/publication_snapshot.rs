//! Immutable publication sharing for semantically owned world payloads.

use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::WorldStore;

/// Serde-transparent structural owner for immutable publication payloads.
///
/// Cloning retains the immutable payload. Any mutation through `DerefMut`
/// first obtains a unique value, so existing field-level mutation boundaries
/// cannot accidentally modify an older published generation.
pub(crate) struct PublicationShared<T>(Arc<T>);

impl<T> PublicationShared<T> {
    #[cfg(test)]
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: Clone> PublicationShared<T> {
    pub(crate) fn into_value(self) -> T {
        Arc::unwrap_or_clone(self.0)
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

    /// Returns the entity-shell construction observation recorded while this
    /// immutable generation was built.
    pub fn entity_publication_snapshot_observation(
        &self,
    ) -> crate::EntityPublicationSnapshotObservation {
        self.entities.publication_snapshot_observation()
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        data_component_types::{
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        },
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_COW_ID,
            VANILLA_ENTITY_TYPE_ITEM_ID,
        },
        mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID,
        packets::{
            AddEntity, AttributeSnapshot, BlockPos as ProtocolBlockPos, BlockUpdate,
            DataComponentPatchSummary, DebugBlockValue, DebugEntityBlockIntersection, DebugEvent,
            DebugSubscriptionEvent, DebugSubscriptionUpdate, DebugSubscriptionValue,
            EntityDataValue, EntityDataValueKind, EntityEvent, EntityMove, EquipmentSlot,
            EquipmentSlotUpdate, ItemStackSummary, ItemStackTemplateSummary, LightUpdate,
            LightUpdateData, MobEffectFlags, NbtSummaryEntry, NbtSummaryValue, RemoveEntities,
            SetEntityData, SetEntityLink, SetEquipment, SetPassengers, SetPlayerInventory,
            UpdateAttributes, UpdateMobEffect, Vec3d,
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

    #[test]
    fn publication_snapshot_shares_recipes_until_the_next_recipe_mutation() {
        let mut world = WorldStore::new();
        world
            .recipes
            .property_sets
            .insert("minecraft:test".to_string(), vec![1, 2]);
        let snapshot = world.publication_snapshot();
        assert!(world.recipes.ptr_eq(&snapshot.recipes));

        world
            .recipes
            .property_sets
            .get_mut("minecraft:test")
            .expect("property set")
            .push(3);
        assert!(!world.recipes.ptr_eq(&snapshot.recipes));
        assert_eq!(snapshot.recipes.property_sets["minecraft:test"], [1, 2]);
        assert_eq!(world.recipes.property_sets["minecraft:test"], [1, 2, 3]);
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
        add_test_entity_with_type(world, id, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
    }

    fn add_test_entity_with_type(world: &mut WorldStore, id: i32, entity_type_id: i32) {
        world.apply_add_entity(AddEntity {
            id,
            uuid: uuid::Uuid::from_u128(id as u128 + 1),
            entity_type_id,
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

    #[test]
    fn entity_tick_work_and_snapshot_rebuild_counts_match_the_generation() {
        let mut world = WorldStore::new();
        add_test_entity(&mut world, 1);
        add_test_entity(&mut world, 2);

        let work = world.advance_entity_client_animations_with_observation(3);
        assert_eq!(work.scanned_entities, 6);
        assert_eq!(work.mutated_entities, 6);

        let generation = world.publication_snapshot();
        let observation = generation.entity_publication_snapshot_observation();
        assert_eq!(observation.rebuilt_entities, generation.entity_count());
        assert_eq!(observation.rebuilt_entities, 2);
    }

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn nested_item_stack(owner: &str, flag: i8, remainder_item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(42),
            count: 2,
            component_patch: DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
                    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
                ],
                custom_data: Some(nested_item_data(owner, flag)),
                item_model: Some("minecraft:diamond_sword".to_string()),
                enchantment_glint_override: Some(true),
                use_remainder: Some(Box::new(ItemStackTemplateSummary {
                    item_id: remainder_item_id,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added: 1,
                        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
                        custom_data: Some(nested_item_data("remainder", flag + 1)),
                        ..DataComponentPatchSummary::default()
                    },
                })),
                ..DataComponentPatchSummary::default()
            },
        }
    }

    fn nested_item_data(owner: &str, flag: i8) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "owner".to_string(),
                value: NbtSummaryValue::String(owner.to_string()),
            },
            NbtSummaryEntry {
                name: "nested".to_string(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "flag".to_string(),
                    value: NbtSummaryValue::Byte(flag),
                }]),
            },
        ])
    }

    fn item_stack_metadata_value(stack: ItemStackSummary) -> EntityDataValue {
        EntityDataValue {
            data_id: crate::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ITEM_ID,
            )
            .expect("generated ItemEntity item accessor"),
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
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
        world
            .queue_light_update(LightUpdate {
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
            .unwrap();
        assert_eq!(world.poll_queued_light_updates(), 1);
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
        world.advance_entity_client_animations(1);
        let generation_b = world.publication_snapshot();
        world.advance_entity_client_animations(1);
        let generation_c = world.publication_snapshot();

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
        let transform_a = generation_a.entities.transform(7).unwrap();
        let transform_b = generation_b.entities.transform(7).unwrap();
        let transform_c = generation_c.entities.transform(7).unwrap();
        assert_eq!(transform_a.old_position.x, 1.0);
        assert_eq!(transform_a.position.x, 1.0);
        assert_eq!(transform_b.old_position.x, 1.0);
        assert!((transform_b.position.x - 4.0 / 3.0).abs() < 1.0e-12);
        assert_eq!(transform_b.interpolation.unwrap().remaining_steps, 2);
        assert_eq!(transform_c.old_position, transform_b.position);
        assert!((transform_c.position.x - 5.0 / 3.0).abs() < 1.0e-12);
        assert_eq!(transform_c.interpolation.unwrap().remaining_steps, 1);
    }

    #[test]
    fn retained_generation_isolated_from_nested_metadata_item_stack_mutation() {
        let mut world = WorldStore::new();
        add_test_entity_with_type(&mut world, 7, VANILLA_ENTITY_TYPE_ITEM_ID);
        let stack_a = nested_item_stack("generation-a", 1, 43);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![item_stack_metadata_value(stack_a.clone())],
        }));
        let generation_a = world.publication_snapshot();

        let stack_b = nested_item_stack("generation-b", 9, 44);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![item_stack_metadata_value(stack_b.clone())],
        }));
        assert!(
            !world
                .entities
                .publication_payload_sharing(&generation_a.entities, 7)
                .unwrap()
                .data_values
        );
        let generation_b = world.publication_snapshot();

        let read_stack = |world: &WorldStore| {
            world
                .probe_entity(7)
                .unwrap()
                .data_values
                .into_iter()
                .find_map(|value| match value.value {
                    EntityDataValueKind::ItemStack(stack) => Some(stack),
                    _ => None,
                })
                .expect("ItemEntity metadata stack")
        };
        assert_eq!(read_stack(&generation_a), stack_a);
        assert_eq!(read_stack(&generation_b), stack_b);
        assert_eq!(generation_a.item_entity_stacks()[0].stack, stack_a);
        assert_eq!(generation_b.item_entity_stacks()[0].stack, stack_b);

        assert_eq!(
            read_stack(&generation_a)
                .component_patch
                .use_remainder
                .as_deref()
                .unwrap()
                .component_patch
                .custom_data,
            Some(nested_item_data("remainder", 2))
        );
        assert_eq!(
            read_stack(&generation_b)
                .component_patch
                .use_remainder
                .as_deref()
                .unwrap()
                .component_patch
                .custom_data,
            Some(nested_item_data("remainder", 10))
        );
    }

    #[test]
    fn retained_generation_isolated_from_all_entity_mutation_families() {
        let mut world = WorldStore::new();
        for id in [7, 8, 10] {
            add_test_entity(&mut world, id);
        }
        add_test_entity_with_type(&mut world, 11, VANILLA_ENTITY_TYPE_COW_ID);
        let generation_a = world.publication_snapshot();

        let unchanged = world
            .entities
            .publication_payload_sharing(&generation_a.entities, 7)
            .unwrap();
        assert!(unchanged.data_values);
        assert!(unchanged.painting_variant);
        assert!(unchanged.equipment);
        assert!(unchanged.attributes);
        assert!(unchanged.passengers);
        assert!(unchanged.effects);
        assert!(unchanged.damage);

        assert!(world.apply_entity_move(EntityMove {
            id: 7,
            has_position: true,
            delta_x: 4_096,
            delta_y: 0,
            delta_z: 0,
            y_rot: Some(45.0),
            x_rot: Some(15.0),
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
        let after_metadata = world
            .entities
            .publication_payload_sharing(&generation_a.entities, 7)
            .unwrap();
        assert!(!after_metadata.data_values);
        assert!(after_metadata.painting_variant);
        assert!(after_metadata.equipment);
        assert!(after_metadata.attributes);
        assert!(after_metadata.passengers);
        assert!(after_metadata.effects);
        assert!(after_metadata.damage);

        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 7,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: stack(5, 1),
            }],
        }));
        assert!(world.apply_update_attributes(UpdateAttributes {
            entity_id: 7,
            attributes: vec![AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 24.0,
                modifiers: Vec::new(),
            }],
        }));
        assert!(world.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
            amplifier: 1,
            duration_ticks: 200,
            flags: MobEffectFlags::default(),
        }));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 7,
            passenger_ids: vec![8],
        }));
        assert!(world.apply_set_entity_link(SetEntityLink {
            source_id: 11,
            dest_id: 7,
        }));
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 7,
            event_id: 2,
        }));
        world.advance_entity_client_animations(1);
        assert_eq!(
            world.apply_remove_entities(RemoveEntities {
                entity_ids: vec![10],
            }),
            1
        );
        add_test_entity(&mut world, 9);
        let generation_b = world.publication_snapshot();

        let entity_a = generation_a.probe_entity(7).unwrap();
        let entity_b = generation_b.probe_entity(7).unwrap();
        assert_eq!(entity_a.position_base.x, 1.0);
        assert_eq!(entity_a.y_rot, 0.0);
        assert_eq!(entity_a.x_rot, 0.0);
        assert_eq!(entity_b.position_base.x, 2.0);
        assert_eq!(entity_b.y_rot, 15.0);
        assert_eq!(entity_b.x_rot, 5.0);
        assert!(entity_a.data_values.is_empty());
        assert_eq!(entity_b.data_values.len(), 1);
        assert!(entity_a.equipment.is_empty());
        assert_eq!(entity_b.equipment.len(), 1);
        assert!(entity_a.attributes.is_empty());
        assert_eq!(entity_b.attributes.len(), 1);
        assert!(entity_a.mob_effects.is_empty());
        assert!(entity_b
            .mob_effects
            .contains_key(&VANILLA_MOB_EFFECT_SPEED_ID));
        assert!(entity_a.passengers.is_empty());
        assert_eq!(entity_b.passengers, vec![8]);
        assert_eq!(entity_a.last_event_id, None);
        assert_eq!(entity_b.last_event_id, Some(2));
        assert_eq!(entity_a.client_animations.age_ticks, 0);
        assert_eq!(entity_b.client_animations.age_ticks, 1);
        assert_eq!(generation_a.probe_entity(11).unwrap().leash_holder_id, None);
        assert_eq!(
            generation_b.probe_entity(11).unwrap().leash_holder_id,
            Some(7)
        );
        assert!(generation_a.probe_entity(10).is_some());
        assert!(generation_b.probe_entity(10).is_none());
        assert!(generation_a.probe_entity(9).is_none());
        assert!(generation_b.probe_entity(9).is_some());
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
    fn retained_generation_isolated_from_debug_subscription_lifecycle_mutations() {
        let mut world = WorldStore::new();
        world.set_requested_debug_subscriptions(&[6, 14]).unwrap();
        let first = ProtocolBlockPos { x: 1, y: 64, z: 1 };
        let second = ProtocolBlockPos { x: 2, y: 64, z: 2 };
        world.apply_debug_block_value(DebugBlockValue {
            pos: first,
            update: DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(DebugSubscriptionValue::EntityBlockIntersection(
                    DebugEntityBlockIntersection::InBlock,
                )),
            },
        });
        let generation_a = world.publication_snapshot();

        world.apply_debug_block_value(DebugBlockValue {
            pos: first,
            update: DebugSubscriptionUpdate {
                subscription_id: 6,
                value: None,
            },
        });
        world.apply_debug_block_value(DebugBlockValue {
            pos: second,
            update: DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(DebugSubscriptionValue::EntityBlockIntersection(
                    DebugEntityBlockIntersection::InAir,
                )),
            },
        });
        world.apply_debug_event(DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 14,
                value: DebugSubscriptionValue::NeighborUpdate(second),
            },
        });
        let generation_b = world.publication_snapshot();

        let first = crate::BlockPos { x: 1, y: 64, z: 1 };
        let second = crate::BlockPos { x: 2, y: 64, z: 2 };
        let values_a = generation_a.debug_subscription_values(6).unwrap();
        assert!(values_a.block_values.contains_key(&first));
        assert!(!values_a.block_values.contains_key(&second));
        assert!(generation_a
            .debug_subscription_values(14)
            .unwrap()
            .events
            .is_empty());
        let values_b = generation_b.debug_subscription_values(6).unwrap();
        assert!(!values_b.block_values.contains_key(&first));
        assert!(values_b.block_values.contains_key(&second));
        assert_eq!(
            generation_b
                .debug_subscription_values(14)
                .unwrap()
                .events
                .len(),
            1
        );
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
        add_test_entity(&mut world, 7);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![EntityDataValue {
                data_id: crate::VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(1),
            }],
        }));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 7,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: stack(5, 1),
            }],
        }));
        assert!(world.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
            amplifier: 0,
            duration_ticks: 20,
            flags: MobEffectFlags::default(),
        }));

        let json = serde_json::to_value(&world).unwrap();
        assert!(json["registries"].is_object());
        assert!(json["chunks"].is_array());
        assert!(json["chunks"][0]["sections"].is_array());
        assert!(json["default_item_max_stack_sizes"].is_object());
        assert!(json["furnace_fuel_item_ids"].is_array());
        assert!(json["furnace_fuel_rules"].is_array());
        assert!(json["entities"].is_array());
        assert!(json["entities"][0]["data_values"].is_array());
        assert!(json["entities"][0]["equipment"].is_array());
        assert!(json["entities"][0]["mob_effects"].is_object());
        assert!(json.get("value").is_none());
        assert!(json.get("inner").is_none());

        let restored: WorldStore = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), json);
    }
}
