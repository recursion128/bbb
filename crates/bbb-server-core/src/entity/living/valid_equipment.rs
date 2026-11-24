use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
use bbb_protocol::entity_types::vanilla_entity_type;
use bbb_protocol::equipment_slots::{vanilla_equipment_slot_by_id, EquipmentSlot};
use bbb_protocol::packets::data_components::EquippableSummary;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::{static_item_component, RuntimeItemStack};

use crate::entity::{Entity, EntityData, EntityRegistryTagView};

// Mob.java:818-827; LivingEntity.java:3821-3825.
pub(crate) fn has_valid_item_with_registry_tags(
    entity: &Entity,
    registry_tags: &EntityRegistryTagView,
    slot: EquipmentSlot,
) -> Option<bool> {
    has_valid_item_with_resolver(entity, slot, &EquipmentRegistry(registry_tags))
}

pub(crate) fn has_valid_item_with_resolver(
    entity: &Entity,
    slot: EquipmentSlot,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Option<bool> {
    let living = entity.living()?;
    let Some(stack) = living.item_by_slot(slot) else {
        return Some(false);
    };
    if !can_use_slot(entity, slot)? {
        return Some(false);
    }
    let Some(equippable) = equippable(stack)? else {
        return Some(false);
    };
    if vanilla_equipment_slot_by_id(equippable.slot_id)?.slot != slot {
        return Some(false);
    }
    can_be_equipped_by_with_resolver(entity.entity_type_id(), equippable, resolver)
}

/// `LivingEntity.isEquippableInSlot` (`LivingEntity.java:3821-3825`) for a
/// stack that is not worn yet.
pub(crate) fn is_equippable_in_slot(
    entity: &Entity,
    stack: &RuntimeItemStack,
    slot: EquipmentSlot,
    registry_tags: &EntityRegistryTagView,
) -> Option<bool> {
    let Some(equippable) = equippable(stack)? else {
        return Some(slot == EquipmentSlot::MainHand && can_use_slot(entity, slot)?);
    };
    if vanilla_equipment_slot_by_id(equippable.slot_id)?.slot != slot
        || !can_use_slot(entity, slot)?
    {
        return Some(false);
    }
    can_be_equipped_by(entity.entity_type_id(), equippable, registry_tags)
}

/// The stack's `minecraft:equippable` component; `None` is one this build
/// cannot read.
pub(crate) fn equippable(stack: &RuntimeItemStack) -> Option<Option<&EquippableSummary>> {
    let patch =
        static_item_component(stack.summary(), VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID).ok()?;
    Some(patch.and_then(|patch| patch.equippable.as_ref()))
}

/// `Equippable.canBeEquippedBy` (`Equippable.java:174-176`).
pub(crate) fn can_be_equipped_by(
    entity_type_id: i32,
    equippable: &EquippableSummary,
    registry_tags: &EntityRegistryTagView,
) -> Option<bool> {
    can_be_equipped_by_with_resolver(
        entity_type_id,
        equippable,
        &EquipmentRegistry(registry_tags),
    )
}

fn can_be_equipped_by_with_resolver(
    entity_type_id: i32,
    equippable: &EquippableSummary,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Option<bool> {
    let Some(allowed) = &equippable.allowed_entities else {
        return Some(true);
    };
    if let Some(tag) = &allowed.tag {
        let resource = vanilla_entity_type(entity_type_id)
            .map(|entry| entry.resource_id)
            .or_else(|| {
                resolver
                    .content_registry()
                    .entity_by_id(entity_type_id)
                    .map(|spec| spec.resource_id.as_str())
            })?;
        resolver.tag_contains("minecraft:entity_type", tag, resource)
    } else {
        Some(allowed.registry_ids.contains(&entity_type_id))
    }
}

pub(crate) struct EquipmentRegistry<'a>(pub(crate) &'a EntityRegistryTagView);

impl bbb_protocol::PersistentRegistryResolver for EquipmentRegistry<'_> {
    fn tag_contains(&self, registry: &str, tag: &str, resource: &str) -> Option<bool> {
        Some(self.0.contains(registry, tag, resource))
    }

    fn registry_available(&self, registry_id: &str) -> bool {
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_available_26_1(
            registry_id,
        )
    }

    fn resource_membership(
        &self,
        registry: &str,
        resource: &str,
    ) -> bbb_protocol::PersistentRegistryMembership {
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
            registry, resource,
        )
    }

    fn tag_members(&self, registry: &str, tag: &str) -> bbb_protocol::PersistentRegistryTagMembers {
        use crate::entity::level::EntityRegistryTagView;
        use bbb_protocol::PersistentRegistryTagMembers;
        match self.0 {
            EntityRegistryTagView::Builtin26_1 => bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(registry, tag),
            EntityRegistryTagView::Frozen(snapshot) => {
                if snapshot.tag_exists(registry, tag) {
                    PersistentRegistryTagMembers::Present(
                        snapshot.members(registry, tag).iter().map(|member| member.to_string()).collect(),
                    )
                } else {
                    PersistentRegistryTagMembers::Absent
                }
            }
        }
    }
}

impl PersistentItemStackHolderResolver for EquipmentRegistry<'_> {
    fn holder_id(&self, registry: &str, resource: &str) -> Option<i32> {
        (registry == "minecraft:sound_event")
            .then(|| bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource))
            .flatten()
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        match self.0 {
            EntityRegistryTagView::Builtin26_1 => {
                bbb_protocol::content_registry::ContentRegistry::vanilla()
            }
            EntityRegistryTagView::Frozen(snapshot) => snapshot.content_registry(),
        }
    }
}

pub(crate) fn can_use_slot(entity: &Entity, slot: EquipmentSlot) -> Option<bool> {
    if let EntityData::ArmorStand(data) = entity.data() {
        return Some(data.can_use_slot(slot));
    }
    entity.living()?;
    let EntityData::GeneralLiving(data) = entity.data() else {
        return Some(true);
    };
    // Horse.java:227-229; Llama.java:304-306; SkeletonHorse.java:178-180;
    // ZombieHorse.java:170-172: these overrides accept every equipment slot.
    if matches!(
        data.entity_resource_id(),
        "minecraft:horse"
            | "minecraft:llama"
            | "minecraft:trader_llama"
            | "minecraft:skeleton_horse"
            | "minecraft:zombie_horse"
    ) {
        return Some(true);
    }
    let adult_alive = entity.is_alive() && entity.living()?.health() > 0.0 && !data.is_baby();
    if slot == EquipmentSlot::Saddle {
        if let Some(horse) = data.abstract_horse_retained() {
            // Camel.java:645-647: both Camel and CamelHusk are always tamed.
            return Some(
                adult_alive
                    && (matches!(
                        data.entity_resource_id(),
                        "minecraft:camel" | "minecraft:camel_husk"
                    ) || horse.is_tame()),
            );
        }
        if matches!(
            data.entity_resource_id(),
            "minecraft:pig" | "minecraft:strider"
        ) {
            return Some(adult_alive);
        }
    }
    if matches!(slot, EquipmentSlot::Saddle | EquipmentSlot::Body)
        && matches!(
            data.entity_resource_id(),
            "minecraft:nautilus" | "minecraft:zombie_nautilus"
        )
    {
        return Some(adult_alive && data.is_tame());
    }
    if slot == EquipmentSlot::Body && data.entity_resource_id() == "minecraft:happy_ghast" {
        return Some(adult_alive);
    }
    Some(true)
}

#[cfg(test)]
mod tests;
