//! Iron golem flower events and gifts on live entities.

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::VANILLA_ITEM_POPPY_ID;
use bbb_protocol::packets::{EntityEvent, ItemStackSummary};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::mob::target::TargetRef;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn apply_flower_change(
    body: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    offering: Option<bool>,
    recipient: Option<TargetRef>,
) {
    // IronGolem.java:238-245.
    if let Some(offering) = offering {
        store.broadcast_to_tracking(
            body.id(),
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: body.id(),
                event_id: if offering { 11 } else { 34 },
            }),
        );
    }
    let Some(recipient) = recipient else {
        return;
    };
    let _ = store.with_entity_mut(recipient.entity_id, |target, store| {
        // OfferFlowerGoal.java:65-71, 86-87; CopperGolem.java:87.
        if target.uuid() != recipient.uuid
            || !matches!(target.data(), EntityData::GeneralLiving(data) if data.is_mob())
            || !level.registry_tag_membership(
                "minecraft:entity_type",
                "minecraft:accepts_iron_golem_gift",
                target.data().entity_resource_id(),
            )
            || target
                .living()?
                .item_by_slot(EquipmentSlot::Saddle)
                .is_some_and(|stack| !stack.is_empty())
            || !body
                .bounding_box()
                .inflate(6.0, 2.0, 6.0)
                .intersects(target.bounding_box())
        {
            return None;
        }
        let poppy = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POPPY_ID),
            count: 1,
            component_patch: Default::default(),
        });
        super::item_slots::set_living_item_in_tick(
            target,
            store,
            level,
            EquipmentSlot::Saddle,
            Some(poppy),
        )
        .ok()?;
        if let EntityData::GeneralLiving(data) = &mut target.data {
            // Mob.java:567-569: the gift guarantees a drop without changing persistence.
            data.mob
                .as_mut()?
                .set_guaranteed_drop(EquipmentSlot::Saddle);
        }
        Some(())
    });
}
