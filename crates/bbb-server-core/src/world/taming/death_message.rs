//! Owner-only death messages for stored tamable animals.

use bbb_protocol::packets::SystemChat;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::damage::{CombatEntry, DamageTypeFacts};

use crate::connection::ClientboundPlayPacket;
use crate::entity::{EntityData, StoredDamageCausingEntity};
use crate::world::display_names::{stored_display_name, stored_display_name_parts};
use crate::world::{send, LevelId, ServerWorld, WorldEvent, RULE_SHOW_DEATH_MESSAGES};

pub(in crate::world) struct PetDamageContext {
    pub(in crate::world) entity_id: i32,
    pub(in crate::world) tick_count: i32,
    pub(in crate::world) enters_combat: bool,
    pub(in crate::world) entry: CombatEntry,
    pub(in crate::world) damage_type: DamageTypeFacts,
}

pub(in crate::world) struct PetDamageSnapshots<'a> {
    pub(in crate::world) causing_entity: Option<StoredDamageCausingEntity>,
    pub(in crate::world) direct_entity_id: Option<i32>,
    pub(in crate::world) causing_entity_snapshot:
        Option<&'a crate::explosion::ExplosionSourceEntity>,
    pub(in crate::world) direct_entity_snapshot:
        Option<&'a crate::explosion::ExplosionSourceEntity>,
}

impl ServerWorld {
    pub(in crate::world) fn record_pet_damage(
        &mut self,
        dimension: &LevelId,
        damage: PetDamageContext,
        snapshots: PetDamageSnapshots<'_>,
    ) {
        let PetDamageContext {
            entity_id,
            tick_count,
            enters_combat,
            mut entry,
            damage_type,
        } = damage;
        let PetDamageSnapshots {
            causing_entity,
            direct_entity_id,
            causing_entity_snapshot,
            direct_entity_snapshot,
        } = snapshots;
        let causing = causing_entity
            .and_then(|source| self.pet_damage_entity_name(dimension, source.entity_id()))
            .or_else(|| {
                causing_entity_snapshot.map(|source| self.pet_damage_snapshot_name(source))
            });
        let direct = direct_entity_id
            .and_then(|id| self.pet_damage_entity_name(dimension, id))
            .or_else(|| direct_entity_snapshot.map(|source| self.pet_damage_snapshot_name(source)));
        entry.attacker = causing.clone();
        entry.attacker_weapon = causing_entity
            .and_then(|source| self.named_main_hand_display_of(dimension, source.entity_id()));
        if let Some(entity) = self.level_mut_by_id(dimension).entities.get_mut(entity_id) {
            entity.record_pet_death_damage(
                entry,
                tick_count,
                enters_combat,
                damage_type,
                causing,
                direct,
            );
        }
    }

    /// `TamableAnimal.die` (TamableAnimal.java:223-231).
    pub(in crate::world) fn send_pet_death_message(
        &self,
        dimension: &LevelId,
        entity_id: i32,
        damage_type: &DamageTypeFacts,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.game_rule_bool(RULE_SHOW_DEATH_MESSAGES, true) {
            return;
        }
        let Some(entity) = self.level_by_id(dimension).entities.get(entity_id) else {
            return;
        };
        let EntityData::GeneralLiving(data) = entity.data() else {
            return;
        };
        let Some(tamable) = data.tamable_retained() else {
            return;
        };
        let Some(owner) = tamable.owner().and_then(|uuid| self.player_reference(uuid)) else {
            return;
        };
        let victim = stored_display_name(
            entity,
            self.scoreboard
                .board()
                .players_team(&entity.uuid().to_string()),
        );
        let kill_credit = entity
            .living()
            .and_then(|living| match living.last_hurt_by_player {
                Some(uuid) => self
                    .player_reference(uuid)
                    .map(|id| self.display_name_of(&self.levels[id].profile)),
                None => living.last_hurt_by_mob().and_then(|reference| {
                    let (dimension, id) = self.entity_reference(dimension, reference.uuid()?)?;
                    self.pet_damage_entity_name(&dimension, id)
                }),
            });
        let message = tamable.death_message(
            entity.tick_count(),
            damage_type,
            victim,
            kill_credit,
            self.plugins().content_registry(),
        );
        events.push(send(
            owner,
            ClientboundPlayPacket::SystemChat(SystemChat::from_component(message, false)),
        ));
    }

    fn pet_damage_snapshot_name(
        &self,
        source: &crate::explosion::ExplosionSourceEntity,
    ) -> NbtSummaryValue {
        stored_display_name_parts(
            &source.entity_resource_id,
            source.uuid,
            source.custom_name.as_ref(),
            self.scoreboard
                .board()
                .players_team(&source.uuid.to_string()),
        )
    }

    fn pet_damage_entity_name(
        &self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> Option<NbtSummaryValue> {
        if let Some(id) = self.connection_by_entity_id(entity_id) {
            return Some(self.display_name_of(&self.levels[id].profile));
        }
        let entity = self.level_by_id(dimension).entities.get(entity_id)?;
        Some(stored_display_name(
            entity,
            self.scoreboard
                .board()
                .players_team(&entity.uuid().to_string()),
        ))
    }
}
