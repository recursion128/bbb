use super::*;

impl ServerWorld {
    pub(super) fn interact_with_living_item(
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
        if self
            .level_by_id(&dimension)
            .entities
            .get(target_id)
            .is_some_and(|target| {
                target.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
            })
        {
            return self.interact_with_allay(connection, dimension, interact, events);
        }
        let player = self.levels.player_at(connection);
        let infinite_materials = player.abilities.instabuild;
        let uuid = player.profile.uuid;
        let mut stack = original.summary().clone();
        let admired = crate::entity::piglin::interact(
            &mut self.level_mut_by_id(&dimension).entities,
            target_id,
            original,
        )
        .ok()?;
        let outcome = if let Some(effects) = admired {
            Some(crate::entity::item_slots::LivingItemInteraction::Equipped(
                effects,
            ))
        } else {
            crate::entity::item_slots::interact_living_item(
                &mut self.level_mut_by_id(&dimension).entities,
                target_id,
                &mut stack,
                uuid,
                infinite_materials,
                tags,
            )
            .ok()?
        };
        let Some(outcome) = outcome else {
            return Some(());
        };
        let player = &mut self.levels[connection];
        if !infinite_materials {
            player.inventory.consume_in_hand(hand, 1, false);
        }
        let position = self
            .level_by_id(&dimension)
            .entities
            .get(target_id)?
            .position();
        match outcome {
            crate::entity::item_slots::LivingItemInteraction::Equipped(effects) => {
                let level = self.level_mut_by_id(&dimension);
                if let Some(sound) = effects.sound {
                    level.entities.play_entity_sound(sound);
                }
                if let Some(event) = effects.equip_event {
                    level.event_sink.game_event_with_source_at(
                        event,
                        position,
                        Some(target_id),
                        None,
                    );
                }
            }
            crate::entity::item_slots::LivingItemInteraction::Dyed => {
                let registry_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    "minecraft:item.dye.use",
                )?;
                self.play_player_sound(
                    connection,
                    Some(connection),
                    bbb_protocol::packets::SoundEventHolder::Reference { registry_id },
                    bbb_protocol::packets::SoundSource::Players,
                    position,
                    1.0,
                    1.0,
                    events,
                );
            }
        }
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

#[cfg(test)]
mod tests;
