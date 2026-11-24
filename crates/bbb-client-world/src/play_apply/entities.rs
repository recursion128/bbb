use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, DamageEvent as ProtocolDamageEvent,
    EntityAnimation as ProtocolEntityAnimation, EntityEvent as ProtocolEntityEvent,
    TakeItemEntity as ProtocolTakeItemEntity, TeleportEntity as ProtocolTeleportEntity,
};

use crate::entities::VanillaEntityEventKind;
use crate::{LevelEventSoundRandomState, VillagerParticleKind, WorldStore};

use super::{EntityTrackingEmitterParticleKind, PlayApplyEffects};

const CRITICAL_HIT_ANIMATION_ACTION: u8 = 4;
const MAGIC_CRITICAL_HIT_ANIMATION_ACTION: u8 = 5;
pub(super) const TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS: u32 = 3;
pub(super) const TOTEM_TRACKING_EMITTER_LIFETIME_TICKS: u32 = 30;
pub(super) const HONEY_BLOCK_SLIDE_PARTICLE_COUNT: u32 = 5;
pub(super) const HONEY_BLOCK_JUMP_PARTICLE_COUNT: u32 = 10;

fn tracking_emitter_for_entity_animation(action: u8) -> Option<EntityTrackingEmitterParticleKind> {
    match action {
        // Vanilla `ClientPacketListener.handleAnimate`: action 4 calls
        // `createTrackingEmitter(entity, ParticleTypes.CRIT)`.
        CRITICAL_HIT_ANIMATION_ACTION => Some(EntityTrackingEmitterParticleKind::Crit),
        // Vanilla action 5 calls
        // `createTrackingEmitter(entity, ParticleTypes.ENCHANTED_HIT)`.
        MAGIC_CRITICAL_HIT_ANIMATION_ACTION => {
            Some(EntityTrackingEmitterParticleKind::EnchantedHit)
        }
        _ => None,
    }
}

impl WorldStore {
    pub(super) fn apply_play_add_entity(
        &mut self,
        entity: ProtocolAddEntity,
        effects: &mut dyn PlayApplyEffects,
    ) {
        if let Some(applied) = self.apply_add_entity_packet(entity) {
            let position = self
                .probe_entity_transform(applied.entity_id)
                .map(|transform| {
                    [
                        transform.position.x,
                        transform.position.y,
                        transform.position.z,
                    ]
                });
            if let Some(sound) =
                self.post_add_entity_sound_for_entity(applied.entity_id, applied.entity_type_id)
            {
                effects.entity_sound(&sound, position);
            }
            if let Some(state) = self.llama_spit_recreation_particle_state(applied.entity_id) {
                effects.llama_spit_recreation_particles(self, state);
            }
        }
    }

    pub(super) fn apply_play_entity_animation(
        &mut self,
        update: ProtocolEntityAnimation,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let tracking_emitter = tracking_emitter_for_entity_animation(update.action);
        let applied = self.apply_entity_animation(update);
        if let (true, Some(kind)) = (applied, tracking_emitter) {
            effects.tracking_emitter_particles(
                self,
                update.id,
                kind,
                TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
            );
        }
    }

    pub(super) fn apply_play_damage_event(
        &mut self,
        update: ProtocolDamageEvent,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let entity_id = update.entity_id;
        let local_player_effects = if self.local_player_id() == Some(entity_id) {
            self.damage_type_effects_by_holder_id(update.source_type_id)
        } else {
            None
        };
        self.apply_damage_event(update);
        // Vanilla `LocalPlayer.playSound` submits its own hurt sound, while remote
        // `LivingEntity.playSound` is rejected by `ClientLevel` and arrives through the
        // server-owned Sound/SoundEntity packet path.
        if self.local_player_id() == Some(entity_id) {
            if let Some(state) =
                self.local_player_damage_hurt_sound(local_player_effects, || random.next_float())
            {
                effects.positioned_sound(&state);
            }
        }
    }

    pub(super) fn apply_play_entity_event(
        &mut self,
        update: ProtocolEntityEvent,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let entity_type_id = self.entities.entity_type_id(update.entity_id);
        let event_kind = entity_type_id.and_then(|entity_type_id| {
            crate::entities::events::entity_event_kind_with_registry(
                entity_type_id,
                update.event_id,
                self.content_registry(),
            )
        });
        let firework_explosion_particles =
            if event_kind == Some(VanillaEntityEventKind::FireworkExplosion) {
                self.firework_rocket_explosion_particle_state(update.entity_id)
            } else {
                None
            };
        let kinetic_hit_sound = if event_kind == Some(VanillaEntityEventKind::LivingKineticHit) {
            self.kinetic_hit_sound_for_entity(update.entity_id)
        } else {
            None
        };
        let arrow_effect_particles = if event_kind == Some(VanillaEntityEventKind::ArrowEffectClear)
        {
            self.arrow_effect_particle_state(update.entity_id)
        } else {
            None
        };
        let entity_taming_particles = match event_kind {
            Some(VanillaEntityEventKind::Taming { succeeded }) => {
                self.entity_taming_particle_state(update.entity_id, succeeded)
            }
            _ => None,
        };
        let ocelot_trusting_particles = match event_kind {
            Some(VanillaEntityEventKind::OcelotTrusting { succeeded }) => {
                self.ocelot_trusting_particle_state(update.entity_id, succeeded)
            }
            _ => None,
        };
        let villager_particles = match event_kind {
            Some(VanillaEntityEventKind::VillagerHeart) => {
                self.villager_particle_state(update.entity_id, VillagerParticleKind::Heart)
            }
            Some(VanillaEntityEventKind::VillagerAngry) => {
                self.villager_particle_state(update.entity_id, VillagerParticleKind::Angry)
            }
            Some(VanillaEntityEventKind::VillagerHappy) => {
                self.villager_particle_state(update.entity_id, VillagerParticleKind::Happy)
            }
            Some(VanillaEntityEventKind::VillagerSplash) => {
                self.villager_particle_state(update.entity_id, VillagerParticleKind::Splash)
            }
            _ => None,
        };
        let dolphin_happy_particles = if event_kind == Some(VanillaEntityEventKind::DolphinHappy) {
            self.dolphin_happy_particle_state(update.entity_id)
        } else {
            None
        };
        let fox_eat_particles = if event_kind == Some(VanillaEntityEventKind::FoxEat) {
            self.fox_eat_particle_state(update.entity_id)
        } else {
            None
        };
        if event_kind == Some(VanillaEntityEventKind::PlayerFinishUsingItem)
            && self.local_player_id() == Some(update.entity_id)
        {
            if let Some(hand) = self.local_player().interaction.using_item_hand {
                if let Some(sound) = self.local_consume_sound(hand, random) {
                    effects.positioned_sound(&sound);
                }
                if let Some(state) = self.local_consume_particles(hand, 16) {
                    effects.living_entity_item_particles(self, state);
                }
            }
        }
        let animal_love_particles = if event_kind == Some(VanillaEntityEventKind::AnimalLove) {
            self.animal_love_particle_state(update.entity_id)
        } else {
            None
        };
        let allay_duplication_particles =
            if event_kind == Some(VanillaEntityEventKind::AllayDuplication) {
                self.allay_duplication_particle_state(update.entity_id)
            } else {
                None
            };
        let snowball_hit_particles = if event_kind == Some(VanillaEntityEventKind::SnowballHit) {
            self.snowball_hit_particle_state(update.entity_id)
        } else {
            None
        };
        let thrown_egg_hit_particles = if event_kind == Some(VanillaEntityEventKind::ThrownEggHit) {
            self.thrown_egg_hit_particle_state(update.entity_id)
        } else {
            None
        };
        let ravager_roar_particles = if event_kind == Some(VanillaEntityEventKind::RavagerRoar) {
            self.ravager_roar_particle_state(update.entity_id)
        } else {
            None
        };
        let witch_magic_particles = if event_kind == Some(VanillaEntityEventKind::WitchMagic) {
            self.witch_magic_particle_state(update.entity_id)
        } else {
            None
        };
        let living_entity_portal_particles =
            if event_kind == Some(VanillaEntityEventKind::LivingPortal) {
                self.living_entity_portal_particle_state(update.entity_id)
            } else {
                None
            };
        let living_entity_poof_particles = if event_kind == Some(VanillaEntityEventKind::LivingPoof)
        {
            self.living_entity_poof_particle_state(update.entity_id)
        } else {
            None
        };
        let living_entity_item_break = match event_kind {
            Some(VanillaEntityEventKind::LivingItemBreak(slot)) => {
                self.living_entity_item_break_state(update.entity_id, slot)
            }
            _ => None,
        };
        let mob_spawn_particles = if event_kind == Some(VanillaEntityEventKind::MobSpawn) {
            self.mob_spawn_particle_state(update.entity_id)
        } else {
            None
        };
        let living_entity_drown_particles =
            if event_kind == Some(VanillaEntityEventKind::LivingDrown) {
                self.living_entity_drown_particle_state(update.entity_id)
            } else {
                None
            };
        let honey_block_particles = match event_kind {
            Some(VanillaEntityEventKind::HoneySlide) => self.honey_block_particle_state(
                update.entity_id,
                HONEY_BLOCK_SLIDE_PARTICLE_COUNT,
                false,
            ),
            Some(VanillaEntityEventKind::HoneyJump) => self.honey_block_particle_state(
                update.entity_id,
                HONEY_BLOCK_JUMP_PARTICLE_COUNT,
                true,
            ),
            _ => None,
        };
        let rabbit_sprint_particles = if event_kind == Some(VanillaEntityEventKind::RabbitJump) {
            self.rabbit_sprint_particle_state(update.entity_id, || random.next_double())
        } else {
            None
        };
        if let Some(state) = rabbit_sprint_particles {
            effects.rabbit_sprint_particles(self, state);
        }
        // `LivingEntity.handleEntityEvent(3)` submits a sound before its death transition.
        // Only `LocalPlayer.playSound` plays that call locally; remote entities are gated
        // by `ClientLevel` and use the server-owned Sound/SoundEntity packet path.
        if event_kind == Some(VanillaEntityEventKind::LivingDeath)
            && self.local_player_id() == Some(update.entity_id)
        {
            if let Some(state) = self.local_player_death_sound(|| random.next_float()) {
                effects.positioned_sound(&state);
            }
        }
        let applied = self.apply_entity_event(update);
        if applied && event_kind == Some(VanillaEntityEventKind::FishingHookPull) {
            let _ = self.apply_fishing_hook_pull(update.entity_id);
        }
        if let Some(state) = firework_explosion_particles {
            if state.has_explosions {
                effects.firework_explosion_particles(self, &state);
            } else {
                effects.firework_empty_explosion_particles(
                    self,
                    [state.position.x, state.position.y, state.position.z],
                );
            }
        }
        if applied {
            if let Some(state) = ravager_roar_particles {
                effects.ravager_roar_particles(self, state);
                let _ = self.apply_ravager_roar_knockback(update.entity_id);
            }
            if let Some(state) = witch_magic_particles {
                effects.witch_magic_particles(self, state);
            }
            if let Some(state) = living_entity_poof_particles {
                effects.living_entity_poof_particles(self, state);
            }
            if let Some(state) = living_entity_item_break {
                if let Some(sound) =
                    self.living_entity_item_break_sound(&state, || random.next_float())
                {
                    effects.positioned_sound(&sound);
                }
                effects.living_entity_item_particles(self, state);
            }
            if let Some(state) = mob_spawn_particles {
                effects.living_entity_poof_particles(self, state);
            }
            if let Some(state) = living_entity_drown_particles {
                effects.living_entity_drown_particles(self, state);
            }
            if let Some(state) = living_entity_portal_particles {
                effects.living_entity_portal_particles(self, state);
            }
            if let Some(state) = arrow_effect_particles {
                effects.arrow_effect_particles(self, state);
            }
            if let Some(state) = kinetic_hit_sound {
                effects.entity_sound(&state, None);
            }
            if let Some(state) = entity_taming_particles {
                effects.entity_taming_particles(self, state);
            }
            if let Some(state) = ocelot_trusting_particles {
                effects.entity_taming_particles(self, state);
            }
            if let Some(state) = villager_particles {
                effects.villager_particles(self, state);
            }
            if let Some(state) = dolphin_happy_particles {
                effects.dolphin_happy_particles(self, state);
            }
            if let Some(state) = fox_eat_particles {
                effects.fox_eat_particles(self, state);
            }
            if let Some(state) = animal_love_particles {
                effects.animal_love_particles(self, state);
            }
            if let Some(state) = allay_duplication_particles {
                effects.allay_duplication_particles(self, state);
            }
            if let Some(state) = snowball_hit_particles {
                effects.snowball_hit_particles(self, state);
            }
            if let Some(state) = thrown_egg_hit_particles {
                effects.thrown_egg_hit_particles(self, state);
            }
            if let Some(state) = honey_block_particles {
                effects.honey_block_particles(self, state);
            }
        }
        if applied && event_kind == Some(VanillaEntityEventKind::TotemUse) {
            effects.tracking_emitter_particles(
                self,
                update.entity_id,
                EntityTrackingEmitterParticleKind::TotemOfUndying,
                TOTEM_TRACKING_EMITTER_LIFETIME_TICKS,
            );
            if let Some(state) = self.totem_use_sound_for_entity(update.entity_id) {
                effects.positioned_sound(&state);
            }
            // Vanilla case 35 also starts the item-activation
            // animation when the event entity is the local player.
            if self.local_player_id() == Some(update.entity_id) {
                self.display_local_player_item_activation();
            }
        }
        if applied {
            let state = match event_kind {
                Some(VanillaEntityEventKind::GuardianAttackSound) => {
                    self.guardian_attack_sound_for_entity(update.entity_id)
                }
                Some(VanillaEntityEventKind::SnifferDiggingSound) => {
                    self.sniffer_digging_sound_for_entity(update.entity_id)
                }
                _ => None,
            };
            if let Some(state) = state {
                let position = self
                    .probe_entity_transform(update.entity_id)
                    .map(|entity| [entity.position.x, entity.position.y, entity.position.z]);
                effects.entity_sound(&state, position);
            }
        }
        if applied && event_kind == Some(VanillaEntityEventKind::EvokerFangsAttack) {
            if let Some(state) =
                self.evoker_fangs_attack_sound_for_entity(update.entity_id, || random.next_float())
            {
                effects.positioned_sound(&state);
            }
        }
        if applied && event_kind == Some(VanillaEntityEventKind::ZombieVillagerCure) {
            if let Some(state) =
                self.zombie_villager_cure_sound_for_entity(update.entity_id, || random.next_float())
            {
                effects.positioned_sound(&state);
            }
        }
        if applied && event_kind == Some(VanillaEntityEventKind::ArmorStandHit) {
            if let Some(state) = self.armor_stand_hit_sound_for_entity(update.entity_id) {
                effects.positioned_sound(&state);
            }
        }
        if applied && event_kind == Some(VanillaEntityEventKind::ArmadilloPeek) {
            if let Some(state) = self.armadillo_peek_sound_for_entity(update.entity_id) {
                effects.positioned_sound(&state);
            }
        }
    }

    pub(super) fn apply_play_take_item_entity(
        &mut self,
        update: ProtocolTakeItemEntity,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let pickup_sound =
            self.take_item_entity_pickup_sound_with_random(update.item_id, || random.next_float());
        let pickup_particles =
            self.take_item_entity_pickup_particle_state(update.item_id, update.player_id);
        if let Some(state) = pickup_sound.map(|state| self.record_positioned_sound(state)) {
            effects.positioned_sound(&state);
        }
        if let Some(state) = pickup_particles.as_ref() {
            effects.take_item_entity_pickup_particles(self, state);
        }
        self.apply_take_item_entity(update);
    }

    pub(super) fn apply_play_teleport_entity(
        &mut self,
        update: ProtocolTeleportEntity,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let (_, recovery, vehicle_report) = self.apply_teleport_entity_with_recovery(update);
        if let Some(recovery) = recovery {
            effects.removed_player_vehicle_teleport(recovery);
        }
        if let Some(report) = vehicle_report {
            effects.vehicle_move_report(report);
        }
    }
}
