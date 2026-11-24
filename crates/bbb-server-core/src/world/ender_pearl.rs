//! Ender pearl chunk loading, impact damage, owner lookup and level transitions.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ENDERMITE_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_protocol::packets::{
    LevelParticles, ParticlePayload, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
    PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
    PLAYER_RELATIVE_ROTATE_DELTA, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::entity::{EntityData, RemovalReason, SpawnReason, StoredEntityTransition};
use crate::level_event::PendingHurtEntity;
use crate::world::{LevelId, ServerWorld, TeleportTransition, WorldEvent};

// Relative.java:22-23.
const PRESERVE_MOTION_AND_ROTATION: i32 = PLAYER_RELATIVE_DELTA_X
    | PLAYER_RELATIVE_DELTA_Y
    | PLAYER_RELATIVE_DELTA_Z
    | PLAYER_RELATIVE_ROTATE_DELTA
    | PLAYER_RELATIVE_X_ROT
    | PLAYER_RELATIVE_Y_ROT;

impl ServerWorld {
    // ThrownEnderpearl.java:175-186; TicketType.java:24.
    pub(super) fn refresh_ender_pearl_tickets(&mut self, dimension: &LevelId) {
        if self
            .level_by_id(dimension)
            .entities
            .ender_pearl_ids()
            .is_empty()
        {
            return;
        }
        let mut tickets = std::mem::take(
            &mut self
                .level_mut_by_id(dimension)
                .chunk_lifecycle
                .ender_pearl_tickets,
        );
        let entities = &self.level_by_id(dimension).entities;
        for &pearl_id in entities.ender_pearl_ids() {
            let Some(pearl) = entities.get(pearl_id) else {
                continue;
            };
            let EntityData::EnderPearl(data) = pearl.data() else {
                continue;
            };
            if pearl.is_alive()
                && data
                    .projectile()
                    .owner()
                    .is_some_and(|owner| self.connection_of(owner).is_some())
            {
                tickets.insert(super::player_packets::player_chunk(pearl.position()), 40);
            }
        }
        self.level_mut_by_id(dimension)
            .chunk_lifecycle
            .ender_pearl_tickets = tickets;
    }

    // ThrownEnderpearl.java:163-173; GameRules.java:35.
    pub(super) fn discard_ender_pearl_for_dead_owner(&mut self, dimension: &LevelId, id: i32) {
        let Some(pearl) = self.level_by_id(dimension).entities.get(id) else {
            return;
        };
        let EntityData::EnderPearl(data) = pearl.data() else {
            return;
        };
        let Some(connection) = data
            .projectile()
            .owner()
            .and_then(|owner| self.connection_of(owner))
        else {
            return;
        };
        let owner = &self.levels[connection];
        if owner.damage.is_dead_or_dying()
            && !owner.cold.won_game
            && self.game_rule_bool("minecraft:ender_pearls_vanish_on_death", true)
        {
            self.level_mut_by_id(dimension)
                .entities
                .remove(id, RemovalReason::Discarded);
        }
    }

    // ThrownEnderpearl.java:78-149.
    pub(super) fn apply_ender_pearl_hit(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        target: Option<PendingHurtEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(pearl) = self.level_by_id(dimension).entities.get(entity_id) else {
            return;
        };
        let EntityData::EnderPearl(data) = pearl.data() else {
            return;
        };
        let owner = data.projectile().owner();
        let destination = pearl.old_position();
        let impact = pearl.position();
        let portal_cooldown = pearl.portal_cooldown() > 0;
        if let Some(target) = target {
            let causing = owner.and_then(|owner| self.ender_pearl_owner_source(dimension, owner));
            let direct =
                self.pending_hurt_source_entity(dimension, PendingHurtEntity::Stored { entity_id });
            match target {
                PendingHurtEntity::Player { uuid } => {
                    if let Some(connection) = self.connection_of(uuid) {
                        self.hurt_connected_player(
                            connection,
                            "minecraft:thrown",
                            0.0,
                            causing,
                            direct,
                            None,
                            events,
                        );
                    }
                }
                PendingHurtEntity::Stored { entity_id } => {
                    let _ = self.hurt_stored_entity_by_command(
                        *dimension,
                        entity_id,
                        "minecraft:thrown",
                        0.0,
                        causing,
                        direct,
                        None,
                        events,
                    );
                }
                PendingHurtEntity::FallingBlock { .. } => {}
            }
        }
        self.ender_pearl_particles(dimension, impact);
        if self
            .level_by_id(dimension)
            .entities
            .get(entity_id)
            .is_some_and(|pearl| pearl.is_alive())
        {
            if let Some(owner) = owner {
                self.teleport_ender_pearl_owner(
                    dimension,
                    owner,
                    destination,
                    portal_cooldown,
                    events,
                );
            }
        }
        self.level_mut_by_id(dimension)
            .entities
            .remove(entity_id, RemovalReason::Discarded);
    }

    fn ender_pearl_owner_source(
        &self,
        dimension: &LevelId,
        owner: uuid::Uuid,
    ) -> Option<crate::commands::CommandDamageSourceEntity> {
        if self.connection_of(owner).is_some() {
            return self
                .pending_hurt_source_entity(dimension, PendingHurtEntity::Player { uuid: owner });
        }
        let (dimension, entity_id) = self.entity_reference(dimension, owner)?;
        self.pending_hurt_source_entity(&dimension, PendingHurtEntity::Stored { entity_id })
    }

    fn teleport_ender_pearl_owner(
        &mut self,
        destination_level: &LevelId,
        owner: uuid::Uuid,
        position: Vec3d,
        portal_cooldown: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection) = self.connection_of(owner) {
            let player = &self.levels[connection];
            if player.avatar_removed
                || player.damage.is_dead_or_dying()
                || player.cold.sleeping_pos.is_some()
            {
                return;
            }
            let origin = player.position();
            let rotation = player.rotation();
            if self.random.next_float() < 0.05
                && self
                    .game_rules
                    .spawning_monsters(self.admin.settings().difficulty)
            {
                let _ = self.with_spawn_runtime(destination_level, &[], |runtime, random, _| {
                    let mut endermite = create_natural_spawn_root(
                        runtime.entities,
                        VANILLA_ENTITY_TYPE_ENDERMITE_ID,
                        origin,
                    )?;
                    endermite.set_rot(rotation.y_rot, rotation.x_rot);
                    runtime.finalize_and_add(endermite, SpawnReason::Triggered, random)
                });
            }
            if portal_cooldown {
                self.levels[connection].set_portal_cooldown();
            }
            if self.teleport_player_id(
                connection,
                TeleportTransition {
                    new_level: *destination_level,
                    position,
                    y_rot: 0.0,
                    x_rot: 0.0,
                    relatives_mask: PRESERVE_MOTION_AND_ROTATION,
                },
                events,
            ) {
                self.reset_player_fall_distance(connection, events);
                self.levels[connection].reset_fall_impulse();
                self.hurt_connected_player(
                    connection,
                    "minecraft:ender_pearl",
                    5.0,
                    None,
                    None,
                    None,
                    events,
                );
            }
            self.ender_pearl_sound(destination_level, position, events);
            return;
        }
        let Some((source_level, owner_id)) = self.entity_reference(destination_level, owner) else {
            return;
        };
        let Some(entity) = self.level_by_id(&source_level).entities.get(owner_id) else {
            return;
        };
        // ThrownEnderpearl.java:152-158; WitherBoss.java:553, EnderDragon.java:845,
        // Creaking.java:348-349, FishingHook.java:539-541 and LivingEntity.java:3686-3687.
        if !entity.is_alive_with_health()
            || entity.sleeping_pos().is_some()
            || (source_level != *destination_level
                && (matches!(
                    entity.entity_type_id(),
                    VANILLA_ENTITY_TYPE_WITHER_ID
                        | VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
                        | VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
                ) || entity.is_heart_bound()))
        {
            return;
        }
        let transition = StoredEntityTransition {
            position,
            delta_movement: entity.delta_movement(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            relatives_mask: 0,
        };
        self.level_mut_by_id(&source_level)
            .entities
            .stop_stored_riding(owner_id);
        if let Ok(Some(id)) = self.teleport_entity_subtree(
            source_level,
            *destination_level,
            owner_id,
            transition,
            false,
            events,
        ) {
            if let Some(entity) = self.level_mut_by_id(destination_level).entities.get_mut(id) {
                entity.reset_fall_distance();
                if let Some(living) = entity.living_mut() {
                    living.fall_impulse.reset();
                }
            }
        }
        self.ender_pearl_sound(destination_level, position, events);
    }

    fn ender_pearl_particles(&mut self, dimension: &LevelId, impact: Vec3d) {
        for _ in 0..32 {
            let particles = LevelParticles {
                override_limiter: false,
                always_show: false,
                position: Vec3d {
                    y: impact.y + self.random.next_double() * 2.0,
                    ..impact
                },
                offset: Vec3d {
                    x: self.random.next_gaussian(),
                    y: 0.0,
                    z: self.random.next_gaussian(),
                },
                max_speed: 1.0,
                count: 0,
                particle: ParticlePayload {
                    particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_PORTAL_ID,
                    raw_options: Vec::new(),
                },
            };
            self.level_mut_by_id(dimension)
                .entities
                .send_particles(particles);
        }
    }

    fn ender_pearl_sound(
        &mut self,
        dimension: &LevelId,
        position: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(registry_id) =
            vanilla_sound_event_registry_id("minecraft:entity.player.teleport")
        {
            let sound = SoundEvent {
                sound: SoundEventHolder::Reference { registry_id },
                source: SoundSource::Players,
                position,
                volume: 1.0,
                pitch: 1.0,
                seed: self.random.next_long(),
            };
            self.broadcast_level_sound_by_key(dimension, &sound, None, events);
        }
    }
}

#[cfg(test)]
mod tests;
