use crate::connection::ClientboundPlayPacket;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityEvent, InteractionHand, SoundSource};
use bbb_server_types::item_stack::static_item_component;

use crate::entity::consume_effect;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::Entity;

/// `level.broadcastEntityEvent(this, (byte)35)` (`LivingEntity.java:1388`).
pub const ENTITY_EVENT_DEATH_PROTECTION: i8 = 35;

pub fn protecting_hand<P, E>(
    bypasses_invulnerability: bool,
    mut protection_in: impl FnMut(InteractionHand) -> Result<Option<P>, E>,
) -> Result<Option<(InteractionHand, P)>, E> {
    if bypasses_invulnerability {
        return Ok(None);
    }
    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        if let Some(protection) = protection_in(hand)? {
            return Ok(Some((hand, protection)));
        }
    }
    Ok(None)
}

/// The stored-mob body of `checkTotemDeathProtection`; true when the death is
/// cancelled. A tag or stack this server cannot read records a failure and
/// leaves the death to go through.
pub(crate) fn check_totem_death_protection(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    damage_type: &str,
) -> bool {
    let bypasses_invulnerability = level.registry_tag_membership(
        "minecraft:damage_type",
        "minecraft:bypasses_invulnerability",
        damage_type,
    );
    let found = protecting_hand(bypasses_invulnerability, |hand| {
        let Some(stack) = entity
            .living()
            .and_then(|living| living.item_by_slot(hand_slot(hand)))
            .filter(|stack| !stack.is_empty())
        else {
            return Ok(None);
        };
        static_item_component(
            stack.summary(),
            VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        )
        .map(|value| {
            value.map(|value| {
                let effects = value.death_protection_effects.clone();
                (stack.with_count(stack.count() - 1), effects)
            })
        })
    });
    let (hand, (rest, effects)) = match found {
        Ok(Some(found)) => found,
        Ok(None) | Err(_) => return false,
    };

    if entity
        .living_mut()
        .and_then(|living| living.update_item_slot(hand_slot(hand), rest))
        .is_none()
    {
        return false;
    }
    crate::entity::living::set_health(entity, 1.0);
    for effect in effects {
        let _ = consume_effect::apply_to_entity(entity, store, level, &effect);
    }
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: ENTITY_EVENT_DEATH_PROTECTION,
        }),
    );
    true
}

pub(crate) fn teleport_randomly(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    diameter: f32,
) -> Option<()> {
    let diameter = f64::from(diameter);
    let min_y = f64::from(level.min_y());
    let max_y = f64::from(level.min_y() + level.logical_height()? - 1);
    for _ in 0..16 {
        let random = store.random();
        let position = entity.position();
        let x = position.x + (random.next_double() - 0.5) * diameter;
        let y = position.y + (random.next_double() - 0.5) * diameter;
        let y = if y < min_y { min_y } else { y.min(max_y) };
        let z = position.z + (random.next_double() - 0.5) * diameter;
        let destination = bbb_protocol::packets::Vec3d { x, y, z };
        if crate::entity::living::random_teleport::attempt(entity, store, level, destination, true)?
        {
            level.game_event_at(
                crate::level_event::VanillaGameEvent::TELEPORT,
                position,
                Some(entity.id()),
            );
            let (resource_id, source) = if entity.entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID
            {
                ("minecraft:entity.fox.teleport", SoundSource::Neutral)
            } else {
                ("minecraft:item.chorus_fruit.teleport", SoundSource::Players)
            };
            store.play_entity_sound(PendingEntitySound {
                position: entity.position(),
                resource_id: resource_id.to_owned(),
                source,
                volume: 1.0,
                pitch: 1.0,
            });
            entity.reset_fall_distance();
            entity.living_mut()?.fall_impulse.reset();
            break;
        }
    }
    Some(())
}

const fn hand_slot(hand: InteractionHand) -> EquipmentSlot {
    match hand {
        InteractionHand::MainHand => EquipmentSlot::MainHand,
        InteractionHand::OffHand => EquipmentSlot::OffHand,
    }
}
