//! Allay sample exchange and duplication through the existing world owners.

use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityAnimation, EntityEvent, SoundEventHolder, SoundSource};

impl ServerWorld {
    // Allay.java:281-311.
    pub(super) fn interact_with_allay(
        &mut self,
        connection: crate::connection::ConnectionId,
        dimension: LevelId,
        interact: &super::LivingItemInteractArgs<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let super::LivingItemInteractArgs {
            target_id,
            hand,
            original,
            tags,
        } = *interact;
        let target = self.level_by_id(&dimension).entities.get(target_id)?;
        if target.entity_type_id() != VANILLA_ENTITY_TYPE_ALLAY_ID || !target.is_alive_with_health()
        {
            return Some(());
        }
        let sample = target
            .living()?
            .item_by_slot(EquipmentSlot::MainHand)
            .filter(|item| !item.is_empty())
            .cloned();
        let position = target.position();
        let player = self.levels.player_at(connection);
        let uuid = player.profile.uuid;
        let infinite = player.abilities.instabuild;
        let (effects, sound) = if crate::entity::allay::is_dancing(target)
            && !original.is_empty()
            && original.item_resource_id().is_some_and(|item| {
                tags.contains("minecraft:item", "minecraft:duplicates_allays", item)
            })
            && crate::entity::allay::can_duplicate(target)
        {
            let entities = &mut self.level_mut_by_id(&dimension).entities;
            // Allay.java:458-466.
            let mut child =
                crate::entity::save::create_fresh_entity(entities, "minecraft:allay", position)?;
            crate::entity::conversion::set_persistence_required(&mut child);
            crate::entity::allay::reset_duplication_cooldown(&mut child);
            crate::entity::allay::reset_duplication_cooldown(entities.get_mut(target_id)?);
            entities.add_fresh(child);
            entities.broadcast_to_tracking(
                target_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id: target_id,
                    event_id: 18,
                }),
            );
            (
                crate::entity::item_slots::SlotWriteEffects::default(),
                "minecraft:block.amethyst_block.chime",
            )
        } else if sample.is_none() && !original.is_empty() {
            let target = self
                .level_mut_by_id(&dimension)
                .entities
                .get_mut(target_id)?;
            let effects = crate::entity::item_slots::set_living_item(
                target,
                EquipmentSlot::MainHand,
                Some(original.with_count(1)),
            )
            .ok()?;
            crate::entity::allay::remember_player(target, Some(uuid));
            (effects, "minecraft:entity.allay.item_given")
        } else if let Some(mut returned) =
            sample.filter(|_| hand == InteractionHand::MainHand && original.is_empty())
        {
            let entities = &mut self.level_mut_by_id(&dimension).entities;
            let mut body = entities.detach(target_id)?;
            let result = crate::entity::item_slots::set_living_item(
                &mut body,
                EquipmentSlot::MainHand,
                None,
            );
            if result.is_ok() {
                crate::entity::allay::remember_player(&mut body, None);
                if body
                    .living_mut()
                    .expect("allay living state")
                    .swing(InteractionHand::MainHand)
                {
                    entities.broadcast_to_tracking(
                        target_id,
                        ClientboundPlayPacket::EntityAnimation(EntityAnimation {
                            id: target_id,
                            action: ANIMATE_SWING_MAIN_HAND,
                        }),
                    );
                }
                crate::entity::allay::delivery::drop_carried_items(&mut body, entities);
            }
            entities.attach(body);
            let effects = result.ok()?;
            let resolver = self.item_stack_resolver();
            self.levels[connection]
                .inventory
                .add_anywhere(&resolver, &mut returned, infinite)
                .ok()?;
            (effects, "minecraft:entity.allay.item_taken")
        } else {
            return Some(());
        };
        if !infinite && !original.is_empty() {
            self.levels[connection]
                .inventory
                .consume_in_hand(hand, 1, false);
        }
        let level = self.level_mut_by_id(&dimension);
        if let Some(sound) = effects.sound {
            level.entities.play_entity_sound(sound);
        }
        if let Some(event) = effects.equip_event {
            level
                .event_sink
                .game_event_with_source_at(event, position, Some(target_id), None);
        }
        let registry_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(sound)?;
        self.play_player_sound(
            connection,
            Some(connection),
            SoundEventHolder::Reference { registry_id },
            SoundSource::Neutral,
            position,
            2.0,
            1.0,
            events,
        );
        self.broadcast_player_menu(
            connection,
            crate::inventory::MenuBroadcastMode::Changes,
            events,
        );
        self.player_game_event_at(
            connection,
            crate::level_event::VanillaGameEvent::ENTITY_INTERACT,
            position,
            None,
        );
        self.trigger_player_entity_interaction(
            connection,
            "minecraft:player_interacted_with_entity",
            dimension,
            target_id,
            original.summary(),
            events,
        );
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
        Some(())
    }
}
