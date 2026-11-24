//! `Player.attack` (`Player.java:951-1004`) on a stored entity or a session.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::enchantment_combat_effects::CombatValue;
use crate::entity::projectile_action::{StoredHurtRequest, StoredHurtResult};
use crate::entity::store::EntityStore;
use crate::entity::{
    Entity, EntityData, EntityRegistryTagView, ScoreboardTeams, StoredDamageCausingEntity,
    StoredEntityDamageContext, StoredEntityDamageRules, StoredEntityDamageSource,
};
use crate::explosion::{ExplosionSourceEntity, ExplosionSourceEntityKind};
use crate::level_event::PendingHurtEntity;
use crate::world::LevelId;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID,
    VANILLA_ATTRIBUTE_SWEEPING_DAMAGE_RATIO_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    EntityAnimation, LevelParticles, ParticlePayload, SetEntityMotion, Vec3d,
};
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_DAMAGE_INDICATOR_ID, VANILLA_PARTICLE_TYPE_SWEEP_ATTACK_ID,
};
use bbb_server_types::food::{cause_food_exhaustion, EXHAUSTION_ATTACK};
use bbb_server_types::stats::custom_stat;
use bbb_server_types::Result;
use uuid::Uuid;

use super::decidecimal_damage;
use super::enchanted_damage::{EnchantedDamageSource, SourceEntity};
use super::player_damage_reduction::PlayerHurtSource;
use crate::world::{
    send, ServerWorld, WorldEvent, PLAYER_ATTACK_KNOCKBACK_SOUND, PLAYER_ATTACK_NODAMAGE_SOUND,
};

const PLAYER_ATTACK_CRIT_SOUND: &str = "minecraft:entity.player.attack.crit";
const PLAYER_ATTACK_STRONG_SOUND: &str = "minecraft:entity.player.attack.strong";
const PLAYER_ATTACK_WEAK_SOUND: &str = "minecraft:entity.player.attack.weak";
mod stab;

const PLAYER_ATTACK_SWEEP_SOUND: &str = "minecraft:entity.player.attack.sweep";
/// `ClientboundAnimatePacket.CRITICAL_HIT` / `MAGIC_CRITICAL_HIT`.
const ANIMATE_CRITICAL_HIT: u8 = 4;
const ANIMATE_MAGIC_CRITICAL_HIT: u8 = 5;
pub(super) const PLAYER_ATTACK_DAMAGE_TYPE: &str = "minecraft:player_attack";

/// The entity a swing lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeleeTarget {
    Stored(i32),
    Session(ConnectionId),
}

/// What `hurtOrSimulate` / `hurtServer` answered for one body.
enum MeleeHurt {
    Hurt,
    NotHurt,
    Refused,
}

impl ServerWorld {
    /// `Player.attack` (`Player.java:951-1004`) past `cannotAttack`.
    pub(super) fn player_melee_attack(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let dimension = self.levels[connection_id].dimension;
        if let MeleeTarget::Stored(id) = target {
            if self.level_by_id(&dimension).entities.get(id).is_none() {
                return Ok(());
            }
        }
        self.player_melee_attack_inner(connection_id, target, registry_tags, events)
            .map(|_| ())
    }

    fn player_melee_attack_inner(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> Result<Option<()>> {
        let weapon_reference = self
            .player_attack_weapon_reference(connection_id)
            .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?;
        let player = self.levels.player_at(connection_id);
        let attack_strength_scale = player.attack_strength_scale(0.5)?;
        // `Player.attack` (`Player.java:953`).
        let attribute_damage = match &player.using_item.auto_spin_attack {
            Some((_, damage, _)) => *damage,
            None => player.attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32,
        };
        let sprinting = player.shared_flags & VANILLA_ENTITY_SHARED_FLAG_SPRINTING != 0;
        let player_uuid = player.profile.uuid;
        let dimension = player.dimension;
        let y_rot = player.rotation().y_rot;
        let x_rot = player.rotation().x_rot;
        // Player.java:982-994: the original target survives logically even if hurt removes its store entry.
        let (hurt_mob, old_health, can_deflect) = match target {
            MeleeTarget::Stored(entity_id) => {
                let Some(target) = self.level_by_id(&dimension).entities.get(entity_id) else {
                    return Ok(Some(()));
                };
                let can_deflect = crate::entity::hurting_projectile::can_deflect_by_player_attack_with_registry_tag_view(
                    target,
                    registry_tags,
                );
                (
                    target.living().map(|_| (target.id(), target.uuid())),
                    target.living().map(|living| living.health()),
                    can_deflect,
                )
            }
            MeleeTarget::Session(target_id) => {
                let target = self.levels.player_at(target_id);
                (
                    Some((target.entity_id(), target.profile.uuid)),
                    Some(target.damage.health),
                    false,
                )
            }
        };
        let damage_type = self.player_attack_damage_type(connection_id);
        // `:957` reads the charge before `onAttack` (`:959`) resets it.
        let magic_boost = attack_strength_scale
            * (self.player_weapon_combat_value(
                connection_id,
                &dimension,
                target,
                &damage_type,
                CombatValue::Damage,
                attribute_damage,
            ) - attribute_damage);
        let mut base_damage =
            attribute_damage * (0.2 + attack_strength_scale * attack_strength_scale * 0.8);
        self.levels[connection_id].on_attack();
        if let (true, MeleeTarget::Stored(entity_id)) = (can_deflect, target) {
            let deflected = self
                .level_mut_by_id(&dimension)
                .entities
                .get_mut(entity_id)
                .is_some_and(|entity| {
                    crate::entity::hurting_projectile::deflect_by_player_attack(
                        entity,
                        player_uuid,
                        y_rot,
                        x_rot,
                    )
                });
            debug_assert!(deflected, "the prepared projectile remains deflectable");
            if deflected {
                self.play_server_side_sound(connection_id, PLAYER_ATTACK_NODAMAGE_SOUND, events)?;
            }
            return Ok(Some(()));
        }
        if base_damage > 0.0 || magic_boost > 0.0 {
            let full_strength_attack = attack_strength_scale > 0.9;
            let knockback_attack = sprinting && full_strength_attack;
            if knockback_attack {
                self.play_server_side_sound(connection_id, PLAYER_ATTACK_KNOCKBACK_SOUND, events)?;
            }
            base_damage +=
                self.player_attack_damage_bonus(connection_id, &dimension, target, &damage_type);
            let critical_attack = full_strength_attack
                && self.player_can_critical_attack(connection_id, hurt_mob.is_some());
            if critical_attack {
                base_damage *= 1.5;
            }
            let total_damage = base_damage + magic_boost;
            let sweep_attack = self.player_is_sweep_attack(
                connection_id,
                full_strength_attack,
                critical_attack,
                knockback_attack,
            );
            let old_movement = match target {
                MeleeTarget::Session(target_id) => Some(self.levels[target_id].delta_movement()),
                MeleeTarget::Stored(_) => None,
            };
            let target_body = hurt_mob.and_then(|_| self.melee_target_body(&dimension, target));
            match self.player_melee_hurt(
                connection_id,
                target,
                &damage_type,
                total_damage,
                registry_tags,
                events,
            )? {
                MeleeHurt::Refused => return Ok(None),
                MeleeHurt::NotHurt => {
                    self.play_server_side_sound(
                        connection_id,
                        PLAYER_ATTACK_NODAMAGE_SOUND,
                        events,
                    )?;
                }
                MeleeHurt::Hurt => {
                    let attribute_knockback = self.levels[connection_id]
                        .attribute_value(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID)
                        as f32;
                    let knockback = self.player_weapon_combat_value(
                        connection_id,
                        &dimension,
                        target,
                        &damage_type,
                        CombatValue::Knockback,
                        attribute_knockback,
                    ) / 2.0
                        + if knockback_attack { 0.5 } else { 0.0 };
                    self.player_cause_extra_knockback(
                        connection_id,
                        &dimension,
                        target,
                        knockback,
                        old_movement,
                        events,
                    );
                    if sweep_attack
                        && self
                            .player_sweep_attack(
                                connection_id,
                                target,
                                &damage_type,
                                (base_damage, attack_strength_scale),
                                registry_tags,
                                events,
                            )?
                            .is_none()
                    {
                        return Ok(None);
                    }
                    let target_entity_id = match target {
                        MeleeTarget::Stored(entity_id) => entity_id,
                        MeleeTarget::Session(target_id) => self.levels[target_id].entity_id(),
                    };
                    self.player_attack_visual_effects(
                        connection_id,
                        target_entity_id,
                        critical_attack,
                        sweep_attack,
                        full_strength_attack,
                        magic_boost,
                        events,
                    )?;
                    let Some(player) = self.levels.player_mut(connection_id) else {
                        return Ok(Some(()));
                    };
                    player.set_last_hurt_mob(hurt_mob);
                    if let Some(body) = &target_body {
                        self.player_item_attack_interaction(
                            connection_id,
                            (target, body),
                            &damage_type,
                            true,
                            EquipmentSlot::MainHand,
                            weapon_reference.clone(),
                            events,
                        )?;
                    }
                    if let Some(old_health) = old_health {
                        if self.levels.player(connection_id).is_some() {
                            self.player_damage_stats_and_hearts(
                                connection_id,
                                &dimension,
                                target,
                                old_health,
                                events,
                            );
                        }
                    }
                    let Some(player) = self.levels.player_mut(connection_id) else {
                        return Ok(Some(()));
                    };
                    cause_food_exhaustion(
                        &mut player.food,
                        player.abilities.invulnerable,
                        EXHAUSTION_ATTACK,
                    );
                }
            }
        }
        // `Player.attack` (`Player.java:1002`) -> `LivingEntity.postPiercingAttack`
        // (`LivingEntity.java:1674-1678`).
        let Some(post_dimension) = self
            .levels
            .player(connection_id)
            .map(|player| player.dimension)
        else {
            return Ok(Some(()));
        };
        if let Err(detail) =
            self.post_piercing_attack(post_dimension, SourceEntity::Session(connection_id), events)
        {
            tracing::warn!("player attack post_piercing_attack: {detail}");
        }
        Ok(Some(()))
    }

    fn player_melee_hurt(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        damage_type: &str,
        amount: f32,
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> Result<MeleeHurt> {
        match target {
            MeleeTarget::Stored(entity_id) => self.player_melee_hurt_stored(
                connection_id,
                entity_id,
                damage_type,
                amount,
                registry_tags,
                events,
            ),
            // `ServerPlayer.hurtServer` (`ServerPlayer.java:971-984`).
            MeleeTarget::Session(target_id) => {
                let player = self.levels.player_at(connection_id);
                let source = self.pending_hurt_source_entity(
                    &player.dimension,
                    PendingHurtEntity::Player {
                        uuid: player.profile.uuid,
                    },
                );
                Ok(
                    match self.hurt_connected_player_checked(
                        target_id,
                        damage_type,
                        amount,
                        source.clone(),
                        source,
                        None,
                        events,
                    ) {
                        Some(true) => MeleeHurt::Hurt,
                        Some(false) => MeleeHurt::NotHurt,
                        None => MeleeHurt::Refused,
                    },
                )
            }
        }
    }

    /// `entity.hurtOrSimulate(playerAttack, amount)` on a stored body, with
    /// `Creaking.hurtServer`'s protector arms (`Creaking.java:152-180`).
    fn player_melee_hurt_stored(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        damage_type_id: &str,
        amount: f32,
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> Result<MeleeHurt> {
        let player = self.levels.player_at(connection_id);
        let player_entity_id = player.entity_id();
        let player_uuid = player.profile.uuid;
        let instabuild = player.abilities.instabuild;
        let may_build = player.may_build();
        let spectator = player.game_type == GameType::Spectator;
        let player_position = player.position();
        let dimension = player.dimension;
        let Some(damage_type) = self
            .cold
            .damage_types
            .iter()
            .find(|entry| entry.resource_id == damage_type_id)
            .cloned()
        else {
            return Ok(MeleeHurt::NotHurt);
        };
        let request = StoredHurtRequest {
            direct_entity_snapshot: None,
            target_entity_id: entity_id,
            damage_type_resource_id: damage_type_id,
            amount,
            direct_resource_id: Some("minecraft:player"),
            registry_tags,
            causing_entity_snapshot: Some(ExplosionSourceEntity {
                custom_name: None,
                entity_id: player_entity_id,
                entity_resource_id: "minecraft:player".to_string(),
                uuid: player_uuid,
                position: player_position,
                kind: ExplosionSourceEntityKind::Player {
                    instabuild,
                    may_build,
                    spectator,
                },
            }),
            source_position: Some(player_position),
            explicit_source_position: None,
        };
        // A melee blow's direct entity *is* the causing one
        // (`DamageSource.isDirect`), so both `ATTACKING_ENTITY` and
        // `DIRECT_ATTACKING_ENTITY` name the session this player owns.
        let attacker_views = self.session_attacker_views(Some(player_uuid), Some(player_uuid));
        let source = StoredEntityDamageSource {
            damage_type: &damage_type,
            direct_entity_id: Some(player_entity_id),
            causing_entity: Some(StoredDamageCausingEntity::Player {
                uuid: player_uuid,
                entity_id: player_entity_id,
                instabuild,
            }),
            attacker_views: attacker_views.views(),
        };
        let rules = StoredEntityDamageRules {
            mob_griefing: self.game_rule_bool("minecraft:mob_griefing", true),
            entity_drops: self.game_rule_bool("minecraft:entity_drops", true),
            tnt_explodes: self.game_rule_bool("minecraft:tnt_explodes", true),
            fall_damage_immune: false,
        };
        let context = StoredEntityDamageContext {
            block_drops: self.game_rule_bool("minecraft:block_drops", true),
            attacker_may_build: may_build,
            attacker_spectator: Some(spectator),
        };
        let result = self.with_stored_hurt_level_checked(&dimension, None, |entities, level, _| {
            crate::entity::projectile_action::hurt_stored_entity(
                request, None, source, rules, context, entities, level,
            )
        });
        let _ = self.run_entity_commands(&dimension, events);
        let Some(result) = result else {
            return Ok(MeleeHurt::Refused);
        };
        Ok(match result {
            StoredHurtResult::Applied { outcome, .. } if outcome.succeeded => MeleeHurt::Hurt,
            StoredHurtResult::Applied { .. } | StoredHurtResult::DropNoop => MeleeHurt::NotHurt,
        })
    }

    /// `ServerPlayer.getEnchantedDamage` (`ServerPlayer.java:2195-2197`) and the
    /// enchantment half of `LivingEntity.getKnockback` (`LivingEntity.java:1511-1516`):
    /// `EnchantmentHelper.modifyDamage` / `modifyKnockback`
    /// (`EnchantmentHelper.java:174-180`, `:202-208`) over the main-hand stack,
    /// each entry filtered by `Enchantment.damageContext` (`Enchantment.java:463-473`)
    /// for `victim` under the attack's source.
    ///
    /// A refused requirement leaves `amount` unmodified and is recorded.
    pub(super) fn player_weapon_combat_value(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        victim: MeleeTarget,
        damage_type: &str,
        value: CombatValue,
        amount: f32,
    ) -> f32 {
        let enchantments = {
            let stack =
                super::player_weapon::player_weapon_stack(self.levels.player_at(connection_id));
            if stack.is_empty() || stack.summary().component_patch.enchantments.is_empty() {
                return amount;
            }
            crate::entity::block_drops::loot_tool_for_runtime_stack(
                stack,
                &self.datapack_tables.enchantment_registry,
            )
            .enchantments
            .into_iter()
            .collect::<Vec<_>>()
        };
        self.player_stack_combat_value(
            connection_id,
            dimension,
            victim,
            damage_type,
            &enchantments,
            value,
            amount,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn player_weapon_combat_value_for_slot(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        victim: MeleeTarget,
        damage_type: &str,
        slot: EquipmentSlot,
        value: CombatValue,
        amount: f32,
    ) -> f32 {
        let enchantments = {
            let player = self.levels.player_at(connection_id);
            let Some(stack) = player
                .inventory
                .equipment_slots()
                .find(|(candidate, _)| *candidate == slot)
                .and_then(|(_, index)| player.inventory.slot(index))
            else {
                return amount;
            };
            if stack.is_empty() || stack.summary().component_patch.enchantments.is_empty() {
                return amount;
            }
            crate::entity::block_drops::loot_tool_for_runtime_stack(
                stack,
                &self.datapack_tables.enchantment_registry,
            )
            .enchantments
            .into_iter()
            .collect::<Vec<_>>()
        };
        self.player_stack_combat_value(
            connection_id,
            dimension,
            victim,
            damage_type,
            &enchantments,
            value,
            amount,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn player_stack_combat_value(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        victim: MeleeTarget,
        damage_type: &str,
        enchantments: &[(String, i32)],
        value: CombatValue,
        amount: f32,
    ) -> f32 {
        if enchantments.is_empty() {
            return amount;
        }
        let attacking = Some(SourceEntity::Session(connection_id));
        let source = EnchantedDamageSource {
            damage_type,
            is_direct: true,
            causing: attacking,
            direct: attacking,
        };
        match self.enchanted_damage_value(dimension, enchantments, victim, &source, value, amount) {
            Ok(amount) => amount,
            Err(detail) => {
                tracing::warn!("player attack enchantment {value:?}: {detail}");
                amount
            }
        }
    }

    /// `Player.canCriticalAttack` (`Player.java:1030-1039`).
    fn player_can_critical_attack(
        &mut self,
        connection_id: ConnectionId,
        target_living: bool,
    ) -> bool {
        let on_climbable = self.player_on_climbable(connection_id);
        let player = self.levels.player_at(connection_id);
        let passenger = self
            .level_by_id(&player.dimension)
            .entities
            .external_player_vehicle(player.entity_id())
            .is_some();
        player.block_contact.fall_distance > 0.0
            && !player.on_ground()
            && !on_climbable
            && !player
                .fluid_interaction
                .is_in_fluid(crate::entity::FluidTag::Water)
            && !player
                .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_BLINDNESS_ID)
            && !passenger
            && target_living
            && player.shared_flags & VANILLA_ENTITY_SHARED_FLAG_SPRINTING == 0
    }

    /// `Player.isSweepAttack` (`Player.java:1041-1051`).
    fn player_is_sweep_attack(
        &self,
        connection_id: ConnectionId,
        full_strength_attack: bool,
        critical_attack: bool,
        knockback_attack: bool,
    ) -> bool {
        let player = self.levels.player_at(connection_id);
        if !full_strength_attack || critical_attack || knockback_attack || !player.on_ground() {
            return false;
        }
        let movement = player.known_movement;
        let max_speed = f64::from(player.speed) * 2.5;
        if movement.x * movement.x + movement.z * movement.z >= max_speed * max_speed {
            return false;
        }
        let Ok(stack) = player.inventory.selected_item() else {
            return false;
        };
        let Some(item) = stack
            .item_id
            .and_then(bbb_protocol::item_types::vanilla_item_type)
        else {
            return false;
        };
        matches!(
            self.level_by_id(&player.dimension)
                .chunks
                .registry_tags()
                .map(|tags| tags.contains("minecraft:item", "minecraft:swords", item.resource_id)),
            Some(true)
        )
    }

    /// `Player.causeExtraKnockback` (`Player.java:1120-1141`).
    fn player_cause_extra_knockback(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        target: MeleeTarget,
        amount: f32,
        old_movement: Option<Vec3d>,
        events: &mut Vec<WorldEvent>,
    ) {
        if amount > 0.0 {
            let (sin, cos) = y_rot_sin_cos(self.levels[connection_id].rotation().y_rot);
            match target {
                MeleeTarget::Stored(entity_id) => {
                    let entities = &mut self.level_mut_by_id(dimension).entities;
                    if let Some(mut entity) = entities.detach(entity_id) {
                        if entity.living().is_some() {
                            knockback_stored(&mut entity, entities, f64::from(amount), sin, -cos);
                        } else {
                            crate::entity::push(
                                &mut entity,
                                -sin * f64::from(amount),
                                0.1,
                                cos * f64::from(amount),
                            );
                        }
                        entities.attach(entity);
                    }
                }
                MeleeTarget::Session(target_id) => {
                    self.player_knockback(target_id, f64::from(amount), sin, -cos);
                }
            }
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the attacking session remains present");
            player.set_delta_movement(Vec3d {
                x: player.delta_movement().x * 0.6,
                y: player.delta_movement().y,
                z: player.delta_movement().z * 0.6,
            });
            player.set_sprinting(false);
        }
        // `:1136-1140`: the hit player alone is sent its motion, and its
        // trackers never do.
        if let (MeleeTarget::Session(target_id), Some(old_movement)) = (target, old_movement) {
            let target = self
                .levels
                .player_mut(target_id)
                .expect("the hit session remains present");
            if std::mem::take(&mut target.hurt_marked) {
                let packet = ClientboundPlayPacket::SetEntityMotion(SetEntityMotion {
                    id: target.entity_id(),
                    delta_movement: target.delta_movement(),
                });
                target.set_delta_movement(old_movement);
                events.push(send(target_id, packet));
            }
        }
    }

    /// `Player.doSweepAttack` (`Player.java:1148-1171`). Stored bodies are
    /// visited in section order, then sessions; the section slot of a session
    /// among stored bodies is not tracked.
    fn player_sweep_attack(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        damage_type: &str,
        (base_damage, attack_strength_scale): (f32, f32),
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> Result<Option<()>> {
        self.play_server_side_sound(connection_id, PLAYER_ATTACK_SWEEP_SOUND, events)?;
        let player = self.levels.player_at(connection_id);
        let sweep_damage = 1.0
            + player.attribute_value(VANILLA_ATTRIBUTE_SWEEPING_DAMAGE_RATIO_ID) as f32
                * base_damage;
        let player_position = player.position();
        let player_uuid = player.profile.uuid;
        let teams = &self.scoreboard.board().teams;
        let player_team = teams.team_of(&player.profile.name);
        let (sin, cos) = y_rot_sin_cos(player.rotation().y_rot);
        let dimension = player.dimension;
        let player_height = player
            .bounding_box()
            .map_or(0.0, |aabb| aabb.max_y - aabb.min_y);
        let entities = &self.level_by_id(&dimension).entities;
        let target_box = match target {
            MeleeTarget::Stored(entity_id) => entities.get(entity_id).map(Entity::bounding_box),
            MeleeTarget::Session(target_id) => self.levels[target_id].bounding_box().ok(),
        };
        let Some(target_box) = target_box else {
            return Ok(Some(()));
        };
        let area = target_box.inflate(1.0, 0.25, 1.0);
        let mut nearby: Vec<MeleeTarget> = entities
            .ids_in_box_sections(area)
            .into_iter()
            .filter(|&id| {
                let Some(entity) = entities.get(id) else {
                    return false;
                };
                entity.living().is_some()
                    && entity.bounding_box().intersects(area)
                    && MeleeTarget::Stored(id) != target
                    && !self.player_allied_to_stored(
                        player_uuid,
                        player_team,
                        teams,
                        entities,
                        entity,
                    )
                    && !matches!(entity.data(), EntityData::ArmorStand(data) if data.is_marker())
                    && distance_to_sqr(player_position, entity.position()) < 9.0
            })
            .map(MeleeTarget::Stored)
            .collect();
        nearby.extend(
            self.levels
                .player_entries()
                .filter(|&(&id, other)| {
                    id != connection_id
                        && MeleeTarget::Session(id) != target
                        && other.dimension == dimension
                        && !other.avatar_removed
                        && other.game_type != GameType::Spectator
                        && other.bounding_box().is_ok_and(|aabb| aabb.intersects(area))
                        && !ScoreboardTeams::allied(player_team, teams.team_of(&other.profile.name))
                        && distance_to_sqr(player_position, other.position()) < 9.0
                })
                .map(|(&id, _)| MeleeTarget::Session(id)),
        );
        for nearby in nearby {
            let enchanted = self.player_weapon_combat_value(
                connection_id,
                &dimension,
                nearby,
                damage_type,
                CombatValue::Damage,
                sweep_damage,
            ) * attack_strength_scale;
            match self.player_melee_hurt(
                connection_id,
                nearby,
                damage_type,
                enchanted,
                registry_tags,
                events,
            )? {
                MeleeHurt::Hurt => {}
                MeleeHurt::Refused => return Ok(None),
                MeleeHurt::NotHurt => continue,
            }
            match nearby {
                MeleeTarget::Stored(id) => {
                    let entities = &mut self.level_mut_by_id(&dimension).entities;
                    if let Some(mut entity) = entities.detach(id) {
                        knockback_stored(&mut entity, entities, 0.4, sin, -cos);
                        entities.attach(entity);
                    }
                }
                MeleeTarget::Session(id) => self.player_knockback(id, 0.4, sin, -cos),
            }
            // `Player.java:1162`.
            let damage_types = std::sync::Arc::clone(&self.cold.damage_types);
            if let Some(facts) = damage_types
                .iter()
                .find(|facts| facts.resource_id == damage_type)
            {
                let attacker = Some(SourceEntity::Session(connection_id));
                let source = PlayerHurtSource {
                    damage_type: facts,
                    causing: attacker,
                    direct: attacker,
                };
                if let Err(detail) =
                    self.do_post_attack_effects_checked(&dimension, nearby, source, events)
                {
                    tracing::warn!("{detail}");
                    return Ok(None);
                }
            }
        }
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position: Vec3d {
                x: player_position.x - sin,
                y: player_position.y + player_height * 0.5,
                z: player_position.z + cos,
            },
            offset: Vec3d {
                x: -sin,
                y: 0.0,
                z: cos,
            },
            max_speed: 0.0,
            count: 0,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_SWEEP_ATTACK_ID,
                raw_options: Vec::new(),
            },
        };
        self.publish_player_particles(connection_id, packet, events);
        Ok(Some(()))
    }

    /// `Entity.isAlliedTo(Entity)` (`Entity.java:2696-2706`) from a player to
    /// a stored body: one same team either way, where a tame animal falls back
    /// to its root owner's (`TamableAnimal.java:190-204`, `:207-220`), the
    /// root owner itself, or `Allay.considersEntityAsAlly` (`Allay.java:159-170`).
    /// Illager and evoker allies are never players.
    pub(super) fn player_allied_to_stored(
        &self,
        player_uuid: Uuid,
        player_team: Option<u32>,
        teams: &ScoreboardTeams,
        entities: &EntityStore,
        entity: &Entity,
    ) -> bool {
        let root_owner = match entity.tamable_owner() {
            Some((true, _)) => self.root_owner(entity),
            _ => None,
        };
        if root_owner
            .as_ref()
            .is_some_and(|owner| owner.uuid == player_uuid)
        {
            return true;
        }
        let entity_team = teams.entity_team(entity.uuid());
        let owner_team = root_owner.and_then(|owner| teams.team_of(&owner.scoreboard_name));
        if ScoreboardTeams::allied(player_team, entity_team.or(owner_team))
            || ScoreboardTeams::allied(player_team, owner_team)
        {
            return true;
        }
        entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
            && entities.liked_player(entity.id()) == Some(player_uuid)
    }

    /// `OwnableEntity.getRootOwner` (`OwnableEntity.java:17-34`), resolving
    /// each owner in any dimension (`EntityReference.getLivingEntity`).
    fn root_owner(&self, entity: &Entity) -> Option<RootOwner> {
        let mut seen = vec![entity.uuid()];
        let mut owner = entity.ownable_owner()??;
        loop {
            if let Some(player) = self
                .levels
                .players()
                .find(|player| player.profile.uuid == owner)
            {
                return Some(RootOwner {
                    uuid: owner,
                    scoreboard_name: player.profile.name.clone(),
                });
            }
            let stored = self.levels.values().find_map(|level| {
                let id = level.entities.entity_id_for_uuid(owner)?;
                level.entities.get(id)
            })?;
            stored.living()?;
            let next = match stored.ownable_owner() {
                None => {
                    return Some(RootOwner {
                        uuid: owner,
                        scoreboard_name: owner.to_string(),
                    })
                }
                Some(next) => next?,
            };
            if seen.contains(&next) {
                return None;
            }
            seen.push(owner);
            owner = next;
        }
    }

    /// `Player.attackVisualEffects` (`Player.java:1053-1073`) with
    /// `ServerPlayer.crit` / `magicCrit` (`ServerPlayer.java:1713-1720`).
    #[allow(clippy::too_many_arguments)]
    fn player_attack_visual_effects(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        critical_attack: bool,
        sweep_or_stab_attack: bool,
        full_strength_attack: bool,
        magic_boost: f32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let player_entity_id = self.levels[connection_id].entity_id();
        if critical_attack {
            self.play_server_side_sound(connection_id, PLAYER_ATTACK_CRIT_SOUND, events)?;
            self.send_to_player_tracking_and_self(
                connection_id,
                player_entity_id,
                ClientboundPlayPacket::EntityAnimation(EntityAnimation {
                    id: entity_id,
                    action: ANIMATE_CRITICAL_HIT,
                }),
                events,
            );
        }
        if !critical_attack && !sweep_or_stab_attack {
            self.play_server_side_sound(
                connection_id,
                if full_strength_attack {
                    PLAYER_ATTACK_STRONG_SOUND
                } else {
                    PLAYER_ATTACK_WEAK_SOUND
                },
                events,
            )?;
        }
        if magic_boost > 0.0 {
            self.send_to_player_tracking_and_self(
                connection_id,
                player_entity_id,
                ClientboundPlayPacket::EntityAnimation(EntityAnimation {
                    id: entity_id,
                    action: ANIMATE_MAGIC_CRITICAL_HIT,
                }),
                events,
            );
        }
        Ok(())
    }

    /// `Player.damageStatsAndHearts` (`Player.java:1075-1085`) for a living
    /// target.
    fn player_damage_stats_and_hearts(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        target: MeleeTarget,
        old_health: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        // A body the hurt removed keeps the health it was removed with,
        // which no hurt that removes a body (`ArmorStand.hurtServer`) lowers.
        let target = match target {
            MeleeTarget::Stored(entity_id) => self
                .level_by_id(dimension)
                .entities
                .get(entity_id)
                .map(|entity| {
                    let aabb = entity.bounding_box();
                    (
                        entity.living().map_or(old_health, |living| living.health()),
                        entity.position(),
                        aabb.max_y - aabb.min_y,
                    )
                }),
            MeleeTarget::Session(target_id) => {
                let target = self.levels.player_at(target_id);
                let height = target
                    .bounding_box()
                    .map_or(0.0, |aabb| aabb.max_y - aabb.min_y);
                Some((target.damage.health, target.position(), height))
            }
        };
        let actual_damage = target.map_or(0.0, |(health, ..)| (old_health - health).max(0.0));
        self.levels[connection_id].award_stat(
            &mut self.scoreboard,
            custom_stat("minecraft:damage_dealt"),
            decidecimal_damage(actual_damage),
        );
        let Some((_, position, height)) = target else {
            return;
        };
        if actual_damage > 2.0 {
            let packet = LevelParticles {
                override_limiter: false,
                always_show: false,
                position: Vec3d {
                    x: position.x,
                    y: position.y + height * 0.5,
                    z: position.z,
                },
                offset: Vec3d {
                    x: 0.1,
                    y: 0.0,
                    z: 0.1,
                },
                max_speed: 0.2,
                count: (f64::from(actual_damage) * 0.5) as i32,
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_DAMAGE_INDICATOR_ID,
                    raw_options: Vec::new(),
                },
            };
            self.publish_player_particles(connection_id, packet, events);
        }
    }
}

struct RootOwner {
    uuid: Uuid,
    /// `getScoreboardName`: a player's profile name, anything else's UUID.
    scoreboard_name: String,
}

/// `LivingEntity.knockback` with `Creaking.knockback`'s frozen-body refusal
/// (`Creaking.java:448-452`).
fn knockback_stored(entity: &mut Entity, entities: &mut EntityStore, power: f64, xd: f64, zd: f64) {
    if entity.creaking_can_move() {
        crate::entity::living::knockback(entity, entities, power, xd, zd);
    }
}

/// `Mth.sin` / `Mth.cos` of `getYRot() * (float) (Math.PI / 180.0)`.
fn y_rot_sin_cos(y_rot: f32) -> (f64, f64) {
    let radians = f64::from(y_rot * (std::f64::consts::PI / 180.0) as f32);
    (
        f64::from(bbb_server_types::geometry::mth_sin(radians)),
        f64::from(bbb_server_types::geometry::mth_cos(radians)),
    )
}

fn distance_to_sqr(first: Vec3d, second: Vec3d) -> f64 {
    let (dx, dy, dz) = (first.x - second.x, first.y - second.y, first.z - second.z);
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn set_player_ground_for_test(
        &mut self,
        connection_id: ConnectionId,
        on_ground: bool,
        fall_distance: f64,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test player is connected");
        player.set_on_ground(on_ground);
        player.block_contact.fall_distance = fall_distance;
        player.known_movement = Vec3d::default();
    }

    pub(crate) fn player_fall_distance_for_test(&self, connection_id: ConnectionId) -> f64 {
        self.levels[connection_id].block_contact.fall_distance
    }

    pub(crate) fn damage_type_registry_id_for_test(&self, resource_id: &str) -> i32 {
        self.cold
            .damage_types
            .iter()
            .find(|entry| entry.resource_id == resource_id)
            .expect("the damage type is registered")
            .registry_id
    }

    pub(crate) fn player_sprinting_for_test(&self, connection_id: ConnectionId) -> bool {
        self.levels[connection_id].shared_flags & VANILLA_ENTITY_SHARED_FLAG_SPRINTING != 0
    }
}
