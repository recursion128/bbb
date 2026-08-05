use bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::packets::{
    EntityDataRegistryIdentity, EntityDataRegistryReference, EntityDataRegistryRevision,
    EntityDataRegistrySource, EntityDataValue, EntityDataValueKind,
    EquipmentSlot as ProtocolEquipmentSlot, EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    InteractionHand, ItemStackSummary as ProtocolItemStackSummary,
    SetEntityData as ProtocolSetEntityData, SetEquipment as ProtocolSetEquipment,
    UpdateAttributes as ProtocolUpdateAttributes,
};

use crate::{
    FrozenRegistryAccess, PaintingVariantClientProjection, RegistryEntrySource, WorldStore,
    PAINTING_VARIANT_REGISTRY,
};

use super::dimensions::vanilla_living_entity_type;
use super::schema::{
    vanilla_entity_metadata_accessor_for_entity_type_and_data_id,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_painting_variant_accessor_for_entity_type,
    vanilla_entity_metadata_type_is_living, vanilla_entity_type_has_attribute,
};

const ATTRIBUTE_REGISTRY: &str = "minecraft:attribute";
const VILLAGER_TYPE_REGISTRY: &str = "minecraft:villager_type";
const VILLAGER_PROFESSION_REGISTRY: &str = "minecraft:villager_profession";

impl WorldStore {
    pub fn apply_set_entity_data(&mut self, mut packet: ProtocolSetEntityData) -> bool {
        self.counters.entity_data_updates_received += 1;
        self.counters.entity_data_values_received += packet.values.len();
        let id = packet.id;
        let Some(entity_type_id) = self.entities.entity_type_id(id) else {
            self.counters.entity_data_updates_ignored += 1;
            return false;
        };
        for value in &packet.values {
            let Some(accessor) = vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
                entity_type_id,
                value.data_id,
            ) else {
                self.record_apply_error(
                    "entity_data_schema",
                    format!(
                        "unknown entity data field {} for entity {id} type {entity_type_id}",
                        value.data_id
                    ),
                );
                self.counters.entity_data_updates_ignored += 1;
                return false;
            };
            if accessor.serializer_id != value.serializer_id {
                self.record_apply_error(
                    "entity_data_schema",
                    format!(
                        "invalid entity data item type for entity {id} type {entity_type_id} field {} (id {}): expected serializer {} ({}), received {}",
                        accessor.java_name,
                        value.data_id,
                        accessor.serializer_id,
                        accessor.serializer_name,
                        value.serializer_id
                    ),
                );
                self.counters.entity_data_updates_ignored += 1;
                return false;
            }
            let component_validation = match &value.value {
                EntityDataValueKind::Component(component)
                | EntityDataValueKind::OptionalComponent(Some(component)) => {
                    crate::client::validate_rich_component_with_registry(
                        component.component(),
                        self.frozen_registry_access(),
                    )
                }
                EntityDataValueKind::PaintingVariant(variant) => {
                    let Some(direct) = &variant.direct else {
                        continue;
                    };
                    [
                        ("title", direct.title.as_ref()),
                        ("author", direct.author.as_ref()),
                    ]
                    .into_iter()
                    .filter_map(|(field, component)| component.map(|component| (field, component)))
                    .try_for_each(|(field, component)| {
                        crate::client::validate_rich_component_with_registry(
                            component.component(),
                            self.frozen_registry_access(),
                        )
                        .map_err(|error| format!("painting variant {field}: {error}"))
                    })
                }
                _ => Ok(()),
            };
            if let Err(error) = component_validation {
                self.record_apply_error(
                    "entity_data_component",
                    format!(
                        "invalid entity data component for entity {id} type {entity_type_id} field {} (id {}): {error}",
                        accessor.java_name, value.data_id
                    ),
                );
                self.counters.entity_data_updates_ignored += 1;
                return false;
            }
        }
        if let Err(error) =
            bind_entity_data_registry_references(&mut packet.values, self.frozen_registry_access())
        {
            self.record_apply_error(
                "entity_data_registry",
                format!("invalid registry holder for entity {id} type {entity_type_id}: {error}"),
            );
            self.counters.entity_data_updates_ignored += 1;
            return false;
        }
        let is_local_player = self.local_player_id == Some(id);
        let local_living_entity_accessors = is_local_player
            .then_some(entity_type_id)
            .and_then(vanilla_entity_metadata_living_entity_accessors_for_entity_type);
        let updated_values = packet.values.clone();
        let type_id = entity_type_id;
        let painting_variant_projection =
            self.painting_variant_projection_from_metadata(type_id, &updated_values);
        let mut local_living_entity_flags = None;
        let Some(()) = self.entities.with_metadata_mut(id, |metadata| {
            for value in packet.values {
                if local_living_entity_accessors
                    .is_some_and(|accessors| value.data_id == accessors.flags_data_id)
                {
                    if let EntityDataValueKind::Byte(flags) = &value.value {
                        local_living_entity_flags =
                            local_living_entity_accessors.map(|accessors| (*flags, accessors));
                    }
                }
                if let Some(existing) = metadata
                    .data_values
                    .iter_mut()
                    .find(|existing| existing.data_id == value.data_id)
                {
                    *existing = value;
                } else {
                    metadata.data_values.push(value);
                }
            }
            metadata.data_values.sort_by_key(|value| value.data_id);
            if let Some(projection) = painting_variant_projection {
                *metadata.painting_variant_projection = projection;
            }
        }) else {
            self.counters.entity_data_updates_ignored += 1;
            return false;
        };
        let _ = self
            .entities
            .sync_client_animation_targets_from_metadata(id);
        let _ = self
            .entities
            .sync_client_animation_events_from_metadata_update(id, &updated_values);
        let _ = self
            .entities
            .sync_fishing_hook_from_metadata_update(id, &updated_values);
        let _ = self.entities.refresh_client_position_from_entity_data(id);
        let minecart_command_block_updated = self
            .entities
            .sync_minecart_command_block_from_metadata_update(id, &updated_values);
        if minecart_command_block_updated {
            self.refresh_minecart_command_block_editor_from_authoritative_data(id);
        }
        if let Some((flags, accessors)) = local_living_entity_flags {
            self.sync_local_using_item_from_living_entity_flags(flags, accessors);
        }
        self.counters.entity_data_updates_applied += 1;
        true
    }

    fn painting_variant_projection_from_metadata(
        &mut self,
        entity_type_id: i32,
        values: &[bbb_protocol::packets::EntityDataValue],
    ) -> Option<Option<PaintingVariantClientProjection>> {
        let data_id =
            vanilla_entity_metadata_painting_variant_accessor_for_entity_type(entity_type_id)?;
        let variant = values.iter().find_map(|value| {
            (value.data_id == data_id)
                .then_some(&value.value)
                .and_then(|value| match value {
                    EntityDataValueKind::PaintingVariant(variant) => Some(variant),
                    _ => None,
                })
        })?;
        if let Some(direct) = variant.direct.as_ref() {
            return Some(Some(PaintingVariantClientProjection {
                width: direct.width,
                height: direct.height,
                asset_id: direct.asset_id.clone(),
            }));
        }
        let Some(holder_id) = variant
            .registry
            .as_ref()
            .map(EntityDataRegistryReference::holder_id)
        else {
            return Some(None);
        };
        Some(match self.painting_variant_client_projection(holder_id) {
            Ok(projection) => projection,
            Err(err) => {
                self.record_apply_error("painting_variant", err);
                None
            }
        })
    }

    fn sync_local_using_item_from_living_entity_flags(
        &mut self,
        flags: i8,
        accessors: super::schema::VanillaLivingEntityMetadataAccessors,
    ) {
        let server_using_item = flags & accessors.is_using_mask != 0;
        if !server_using_item {
            if self.local_player.interaction.using_item {
                self.take_local_using_item();
            }
            return;
        }

        if self.local_player.interaction.using_item {
            return;
        }

        let hand = if flags & accessors.off_hand_mask != 0 {
            InteractionHand::OffHand
        } else {
            InteractionHand::MainHand
        };
        if self.local_item_in_hand_is_non_empty(hand) {
            self.set_local_using_item_with_hand(true, hand);
        }
    }

    pub fn local_player_is_sleeping(&self) -> bool {
        self.local_player_id
            .and_then(|id| self.entities.pose(id))
            .is_some_and(|pose| pose == VANILLA_ENTITY_POSE_SLEEPING_ID)
    }

    /// Vanilla `Entity.isSleeping` for any tracked entity: the synced pose
    /// metadata equals `Pose.SLEEPING`. `ScreenEffectRenderer.renderScreenEffect`
    /// consumes this for the camera entity to suppress first-person overlays.
    pub fn entity_is_sleeping(&self, id: i32) -> bool {
        self.entities
            .pose(id)
            .is_some_and(|pose| pose == VANILLA_ENTITY_POSE_SLEEPING_ID)
    }

    /// Vanilla `Entity.isOnFire` for the local player, consumed by the
    /// first-person fire screen effect.
    pub fn local_player_is_on_fire(&self) -> bool {
        self.local_player_id
            .and_then(|id| self.entities.on_fire(id))
            .unwrap_or(false)
    }

    pub fn apply_set_equipment(&mut self, packet: ProtocolSetEquipment) -> bool {
        self.counters.entity_equipment_updates_received += 1;
        self.counters.entity_equipment_slots_received += packet.slots.len();
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            self.counters.entity_equipment_updates_ignored += 1;
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
            self.counters.entity_equipment_updates_ignored += 1;
            return false;
        }
        let Some(()) = self
            .entities
            .with_equipment_mut(packet.entity_id, |equipment| {
                for update in packet.slots {
                    if let Some(existing) = equipment
                        .equipment
                        .iter_mut()
                        .find(|existing| existing.slot == update.slot)
                    {
                        *existing = update;
                    } else {
                        equipment.equipment.push(update);
                    }
                }
                equipment
                    .equipment
                    .sort_by_key(|update| update.slot.ordinal());
            })
        else {
            self.counters.entity_equipment_updates_ignored += 1;
            return false;
        };
        self.counters.entity_equipment_updates_applied += 1;
        self.refresh_entity_active_swing_duration(packet.entity_id);
        true
    }

    /// Applies vanilla `LivingEntity.handleEntityEvent(55)` / `swapHandItems` to synced equipment.
    pub(crate) fn apply_living_swap_hand_items_event(&mut self, entity_id: i32) -> bool {
        let Some(entity_type_id) = self.entities.entity_type_id(entity_id) else {
            return false;
        };
        if !vanilla_entity_metadata_type_is_living(entity_type_id) {
            return false;
        }
        let main_hand = self
            .equipment_item(entity_id, ProtocolEquipmentSlot::MainHand)
            .unwrap_or_else(ProtocolItemStackSummary::empty);
        let off_hand = self
            .equipment_item(entity_id, ProtocolEquipmentSlot::OffHand)
            .unwrap_or_else(ProtocolItemStackSummary::empty);
        let Some(()) = self.entities.with_equipment_mut(entity_id, |equipment| {
            for (slot, item) in [
                (ProtocolEquipmentSlot::MainHand, off_hand),
                (ProtocolEquipmentSlot::OffHand, main_hand),
            ] {
                if let Some(existing) = equipment
                    .equipment
                    .iter_mut()
                    .find(|existing| existing.slot == slot)
                {
                    existing.item = item;
                } else {
                    equipment
                        .equipment
                        .push(ProtocolEquipmentSlotUpdate { slot, item });
                }
            }
            equipment
                .equipment
                .sort_by_key(|update| update.slot.ordinal());
        }) else {
            return false;
        };
        self.refresh_entity_active_swing_duration(entity_id);
        true
    }

    pub fn apply_update_attributes(&mut self, packet: ProtocolUpdateAttributes) -> bool {
        self.counters.entity_attribute_updates_received += 1;
        self.counters.entity_attributes_received += packet.attributes.len();
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            self.counters.entity_attribute_updates_ignored += 1;
            return false;
        };
        if !vanilla_entity_metadata_type_is_living(entity_type_id) {
            self.counters.entity_attribute_updates_ignored += 1;
            return false;
        }
        let mut ignored_snapshot = false;
        let mut resolved_attributes = Vec::with_capacity(packet.attributes.len());
        for mut attribute in packet.attributes {
            let canonical_attribute = self
                .frozen_registry_access()
                .entry_by_holder_id(ATTRIBUTE_REGISTRY, attribute.attribute_id)
                .and_then(|entry| vanilla_attribute_type_for_resource_id(entry.resource_id));
            let Some(canonical_attribute) = canonical_attribute else {
                ignored_snapshot = true;
                continue;
            };
            if !vanilla_entity_type_has_attribute(entity_type_id, canonical_attribute.id) {
                ignored_snapshot = true;
                continue;
            }
            attribute.attribute_id = canonical_attribute.id;
            resolved_attributes.push(attribute);
        }
        let Some(()) = self
            .entities
            .with_attributes_mut(packet.entity_id, |attributes| {
                for attribute in resolved_attributes {
                    if let Some(existing) = attributes
                        .attributes
                        .iter_mut()
                        .find(|existing| existing.attribute_id == attribute.attribute_id)
                    {
                        *existing = attribute;
                    } else {
                        attributes.attributes.push(attribute);
                    }
                }
                attributes
                    .attributes
                    .sort_by_key(|attribute| attribute.attribute_id);
            })
        else {
            self.counters.entity_attribute_updates_ignored += 1;
            return false;
        };
        if ignored_snapshot {
            self.counters.entity_attribute_updates_ignored += 1;
        }
        self.counters.entity_attribute_updates_applied += 1;
        true
    }
}

fn bind_entity_data_registry_references(
    values: &mut [EntityDataValue],
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    for value in values {
        match &mut value.value {
            EntityDataValueKind::RegistryId { serializer, value } => {
                bind_entity_data_registry_reference(value, serializer.registry(), registry_access)?;
            }
            EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                ..
            } => {
                bind_entity_data_registry_reference(
                    villager_type,
                    VILLAGER_TYPE_REGISTRY,
                    registry_access,
                )?;
                bind_entity_data_registry_reference(
                    profession,
                    VILLAGER_PROFESSION_REGISTRY,
                    registry_access,
                )?;
            }
            EntityDataValueKind::PaintingVariant(variant) => {
                if let Some(reference) = variant.registry.as_mut() {
                    bind_entity_data_registry_reference(
                        reference,
                        PAINTING_VARIANT_REGISTRY,
                        registry_access,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn bind_entity_data_registry_reference(
    reference: &mut EntityDataRegistryReference,
    registry: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    let holder_id = reference.holder_id();
    let entry = registry_access
        .entry_by_holder_id(registry, holder_id)
        .ok_or_else(|| format!("missing {registry} holder id {holder_id}"))?;
    let revision = registry_access.revision();
    reference.bind(EntityDataRegistryIdentity::new(
        registry,
        entry.resource_id,
        match entry.source {
            RegistryEntrySource::Server => EntityDataRegistrySource::Server,
            RegistryEntrySource::VanillaFallback => EntityDataRegistrySource::VanillaFallback,
        },
        EntityDataRegistryRevision {
            session: revision.session,
            contents: revision.contents,
            tags: revision.tags,
        },
    ));
    Ok(())
}
