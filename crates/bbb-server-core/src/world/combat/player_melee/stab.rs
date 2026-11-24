//! The spear on a session: the jab, `PiercingWeapon.attack`
//! (`PiercingWeapon.java:75-94`), and the held charge,
//! `KineticWeapon.damageEntities` (`KineticWeapon.java:101-146`), both
//! striking through `Player.stabAttack` (`Player.java:1191-1238`).

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{EntityEvent, InteractionHand, ItemStackSummary, SoundSource, Vec3d};
use bbb_server_types::food::cause_food_exhaustion;
use bbb_server_types::interaction::AttackRange;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use super::{MeleeHurt, MeleeTarget};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::enchantment_combat_effects::CombatValue;
use crate::entity::ai::mob::stroll::view_vector;
use crate::entity::projectile_hit::{ProjectileCandidate, ProjectileTarget};
use crate::entity::spear::{self, Reach};
use crate::entity::{EntityData, EntityRegistryTagView};
use crate::inventory::attack_range;
use crate::world::combat::enchanted_damage::SourceEntity;
use crate::world::combat::EquipmentReference;
use crate::world::external_player_ride_events::DismountLanding;
use crate::world::{ServerWorld, WorldEvent, PLAYER_ATTACK_NODAMAGE_SOUND};

/// `handlePlayerAction`'s `cannotAttackWithItem(itemInHand, 5)`
/// (`ServerGamePacketListenerImpl.java:1270`).
const STAB_CHARGE_TOLERANCE: i32 = 5;

/// `Player.stabAttack`'s `causeFoodExhaustion(0.1F)` (`Player.java:1236`).
const STAB_EXHAUSTION: f32 = 0.1;

/// `ServerLevel.broadcastEntityEvent(livingEntity, (byte)2)`, which the
/// client answers with `onKineticHit` (`KineticWeapon.java:140`).
const ENTITY_EVENT_KINETIC_HIT: i8 = 2;

/// `Item.getUseDuration`'s kinetic answer (`Item.java:309-316`).
const KINETIC_USE_DURATION: i32 = 72000;

/// One `stabAttack` call's arguments past its target.
#[derive(Debug, Clone, Copy)]
struct Stab {
    slot: EquipmentSlot,
    base_damage: f32,
    deals_damage: bool,
    deals_knockback: bool,
    dismounts: bool,
}

/// `Vec3.dot`.
fn dot(a: Vec3d, b: Vec3d) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

impl ServerWorld {
    /// `handlePlayerAction`'s `STAB` arm (`ServerGamePacketListenerImpl.java:1265-1279`)
    /// past its spectator gate.
    pub(in crate::world) fn player_stab(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = &self.levels[connection_id];
        let Ok(main_hand) = player.inventory.selected_item() else {
            return;
        };
        if player
            .cannot_attack_with_item(&main_hand, STAB_CHARGE_TOLERANCE)
            .unwrap_or(true)
        {
            return;
        }
        let Some(weapon) = spear::piercing_weapon(&main_hand) else {
            return;
        };
        // `PiercingWeapon.attack` (`PiercingWeapon.java:75-94`).
        let stab = Stab {
            slot: EquipmentSlot::MainHand,
            base_damage: player.attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32,
            deals_damage: true,
            deals_knockback: weapon.deals_knockback,
            dismounts: weapon.dismounts,
        };
        let targets = self.player_spear_targets(connection_id, &main_hand);
        let mut hit = false;
        self.with_weapon_reference(
            connection_id,
            EquipmentSlot::MainHand,
            |world, reference| {
                for (target, dragon_part) in targets {
                    if let (MeleeTarget::Stored(dragon_id), Some(part)) = (target, dragon_part) {
                        let dimension = world.levels[connection_id].dimension;
                        if let Some(dragon) = world
                            .level_mut_by_id(&dimension)
                            .entities
                            .get_mut(dragon_id)
                        {
                            crate::entity::ender_dragon::set_hit_part(dragon, Some(part));
                        }
                    }
                    hit |= world
                        .player_stab_attack(connection_id, target, stab, reference.clone(), events)
                        .unwrap_or(false);
                }
            },
        );
        self.levels[connection_id].on_attack();
        let dimension = self.levels[connection_id].dimension;
        if let Err(detail) = self.post_piercing_attack(
            dimension,
            crate::world::combat::EnchantmentEffectEntity::Session(connection_id),
            events,
        ) {
            tracing::warn!("spear: {detail}");
        }
        if let Some(sound) = weapon.hit_sound.filter(|_| hit) {
            self.play_player_sound(
                connection_id,
                None,
                sound,
                SoundSource::Players,
                self.levels[connection_id].position(),
                1.0,
                1.0,
                events,
            );
        }
        if let Some(sound) = weapon.sound {
            self.play_player_sound(
                connection_id,
                Some(connection_id),
                sound,
                SoundSource::Players,
                self.levels[connection_id].position(),
                1.0,
                1.0,
                events,
            );
        }
        self.player_swing(connection_id, InteractionHand::MainHand, false, events);
    }

    /// `KineticWeapon.damageEntities` (`KineticWeapon.java:101-146`) on a
    /// session's use tick.
    pub(in crate::world) fn player_kinetic_use_tick(
        &mut self,
        connection_id: ConnectionId,
        stack: &RuntimeItemStack,
        remaining: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(weapon) = spear::kinetic_weapon(stack.summary()) else {
            return;
        };
        let ticks_used = KINETIC_USE_DURATION - remaining;
        if ticks_used < weapon.delay_ticks {
            return;
        }
        let ticks_used = ticks_used - weapon.delay_ticks;
        let attacker_known = self
            .player_known_movement(connection_id)
            .unwrap_or(self.levels[connection_id].known_movement);
        let player = &self.levels[connection_id];
        let rotation = player.rotation();
        let look = view_vector(rotation.x_rot, rotation.y_rot);
        let attacker_speed = dot(look, spear::motion(attacker_known));
        let base_damage = player
            .attributes
            .base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
            .unwrap_or(0.0);
        let slot = match player.using_item.used_item_hand() {
            InteractionHand::MainHand => EquipmentSlot::MainHand,
            InteractionHand::OffHand => EquipmentSlot::OffHand,
        };
        let dimension = player.dimension;
        let game_time = self.game_time();
        let targets = self.player_spear_targets(connection_id, stack.summary());
        let mut affected = false;
        self.with_weapon_reference(connection_id, slot, |world, reference| {
            for (target, dragon_part) in targets {
                let (entity_id, living, target_speed) = match target {
                    MeleeTarget::Stored(id) => {
                        let entities = &world.level_by_id(&dimension).entities;
                        let Some(entity) = entities.get(id) else {
                            continue;
                        };
                        let bodies =
                            crate::entity::store::EntityBodyReadView::new(entities, None, None);
                        let known = ServerWorld::stored_known_movement_in_bodies(entity, bodies)
                            .unwrap_or_else(|| entity.delta_movement());
                        (
                            id,
                            entity.living().is_some(),
                            dot(look, spear::motion(known)),
                        )
                    }
                    MeleeTarget::Session(id) => {
                        let known = world
                            .player_known_movement(id)
                            .unwrap_or(world.levels[id].known_movement);
                        let other = &world.levels[id];
                        (other.entity_id(), true, dot(look, spear::motion(known)))
                    }
                };
                // `wasRecentlyStabbed` and `rememberStabbedEntity`
                // (`LivingEntity.java:2807-2819`).
                let Some(recent) = world.levels[connection_id]
                    .using_item
                    .recent_kinetic_enemies
                    .as_mut()
                else {
                    return;
                };
                if recent.get(&entity_id).is_some_and(|(stamp, _)| {
                    game_time - stamp < i64::from(weapon.contact_cooldown_ticks)
                }) {
                    continue;
                }
                recent.insert(entity_id, (game_time, living));
                let Some(strike) =
                    weapon.strike(ticks_used, attacker_speed, target_speed, 1.0, base_damage)
                else {
                    continue;
                };
                if let (MeleeTarget::Stored(dragon_id), Some(part)) = (target, dragon_part) {
                    if let Some(dragon) = world
                        .level_mut_by_id(&dimension)
                        .entities
                        .get_mut(dragon_id)
                    {
                        crate::entity::ender_dragon::set_hit_part(dragon, Some(part));
                    }
                }
                let stab = Stab {
                    slot,
                    base_damage: strike.damage,
                    deals_damage: strike.deals_damage,
                    deals_knockback: strike.deals_knockback,
                    dismounts: strike.dismounts,
                };
                affected |= world
                    .player_stab_attack(connection_id, target, stab, reference.clone(), events)
                    .unwrap_or(false);
            }
        });
        if !affected {
            return;
        }
        let entity_id = self.levels[connection_id].entity_id();
        self.send_to_player_tracking_and_self(
            connection_id,
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: ENTITY_EVENT_KINETIC_HIT,
            }),
            events,
        );
        // `stabbedEntities(e -> e instanceof LivingEntity)` (`LivingEntity.java:2821-2823`).
        let stabbed = self.levels[connection_id]
            .using_item
            .recent_kinetic_enemies
            .as_ref()
            .map_or(0, |recent| {
                recent.values().filter(|(_, living)| *living).count()
            });
        self.trigger_spear_mobs(connection_id, stabbed as i32, events);
    }

    /// `body` with the stack in the session's `slot` captured as the weapon
    /// its strikes wear down.
    fn with_weapon_reference(
        &mut self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
        body: impl FnOnce(&mut Self, Option<EquipmentReference>),
    ) {
        let dimension = self.levels[connection_id].dimension;
        let reference = self
            .capture_equipment_reference(&dimension, SourceEntity::Session(connection_id), slot)
            .ok()
            .flatten();
        body(self, reference);
    }

    /// `ProjectileUtil.getHitEntitiesAlong(player, player.getAttackRangeWith(stack),
    /// e -> PiercingWeapon.canHitEntity(player, e), COLLIDER)`.
    fn player_spear_targets(
        &mut self,
        connection_id: ConnectionId,
        stack: &ItemStackSummary,
    ) -> Vec<(MeleeTarget, Option<crate::entity::ender_dragon::DragonPart>)> {
        let attacker_known = self
            .player_known_movement(connection_id)
            .unwrap_or(self.levels[connection_id].known_movement);
        let player = &self.levels[connection_id];
        let range = attack_range(stack)
            .ok()
            .flatten()
            .unwrap_or_else(|| AttackRange::default_for_player(player.entity_interaction_range()));
        let creative = player.game_type == GameType::Creative;
        let Ok(eye) = player.eye_position() else {
            return Vec::new();
        };
        let rotation = player.rotation();
        let reach = Reach {
            eye,
            look: view_vector(rotation.x_rot, rotation.y_rot),
            known_movement: attacker_known,
            min: range.effective_min_range(creative),
            max: range.effective_max_range(creative),
            margin: range.hitbox_margin,
        };
        let attacker = player.profile.uuid;
        let dimension = player.dimension;
        let entities = &self.level_by_id(&dimension).entities;
        let root_vehicle = entities
            .external_player_vehicle(player.entity_id())
            .map(|vehicle| entities.root_vehicle(vehicle));
        // `canHarmPlayer` (`ServerPlayer.java:987-989`, `Player.java:727-735`) and
        // `isPassengerOfSameVehicle` for the other sessions.
        let pvp = self.game_rule_bool("minecraft:pvp", true);
        let spared: Vec<Uuid> = self
            .levels
            .player_entries()
            .filter(|(_, other)| other.dimension == dimension)
            .filter(|(_, other)| {
                !pvp || !self
                    .scoreboard
                    .board()
                    .team_allows_player_harm(&player.profile.name, &other.profile.name)
                    || root_vehicle.is_some()
                        && entities
                            .external_player_vehicle(other.entity_id())
                            .map(|vehicle| entities.root_vehicle(vehicle))
                            == root_vehicle
            })
            .map(|(_, other)| other.profile.uuid)
            .collect();
        let hits = self.with_level(dimension, |level| {
            let store = &*level.structure_entities.as_ref()?.store;
            Some(spear::hits_along(
                store,
                level,
                reach,
                |candidate, level| {
                    spear::can_hit(candidate, level)
                        && match candidate {
                            ProjectileCandidate::Stored(entity) => root_vehicle
                                .is_none_or(|root| store.root_vehicle(entity.id()) != root),
                            ProjectileCandidate::Player(other) => {
                                other.uuid != attacker && !spared.contains(&other.uuid)
                            }
                        }
                },
            ))
        });
        let entities = &self.level_by_id(&dimension).entities;
        hits.unwrap_or_default()
            .into_iter()
            .filter_map(|target| match target {
                ProjectileTarget::Stored { entity_id } => {
                    if let Some((dragon_id, part, _)) =
                        crate::entity::ender_dragon::part_by_id(entities, entity_id)
                    {
                        Some((MeleeTarget::Stored(dragon_id), Some(part)))
                    } else {
                        Some((MeleeTarget::Stored(entity_id), None))
                    }
                }
                ProjectileTarget::Player { entity_id, .. } => self
                    .connection_by_entity_id(entity_id)
                    .map(|conn| (MeleeTarget::Session(conn), None)),
            })
            .collect()
    }

    /// `Player.cannotAttack` (`Player.java:1015-1017`) for a stab: an
    /// interaction records the attack and refuses it, and a hanging entity
    /// may skip it.
    fn player_stab_refused(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        registry_tags: &EntityRegistryTagView,
    ) -> bool {
        let MeleeTarget::Stored(entity_id) = target else {
            return false;
        };
        let player = &self.levels[connection_id];
        let (uuid, dimension) = (player.profile.uuid, player.dimension);
        let game_time = self.game_time();
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        match entities.get(entity_id).map(|entity| entity.data()) {
            Some(EntityData::Interaction(_)) => {
                let Some(mut entity) = entities.detach(entity_id) else {
                    return true;
                };
                let skipped = crate::entity::interaction::skip_attack_interaction(
                    &mut entity,
                    uuid,
                    game_time,
                );
                entities.attach(entity);
                skipped
            }
            Some(
                EntityData::Painting(_)
                | EntityData::ItemFrame(_)
                | EntityData::GlowItemFrame(_)
                | EntityData::LeashKnot(_),
            ) => self
                .player_attack_hanging_skip(connection_id, entity_id, registry_tags)
                .unwrap_or(true),
            Some(_) => false,
            None => true,
        }
    }

    /// `Player.stabAttack` (`Player.java:1191-1238`).
    fn player_stab_attack(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        stab: Stab,
        weapon_reference: Option<EquipmentReference>,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<bool> {
        let dimension = self.levels[connection_id].dimension;
        let registry_tags = self
            .level_by_id(&dimension)
            .chunks
            .registry_tags_shared()
            .map(EntityRegistryTagView::Frozen)
            .unwrap_or(EntityRegistryTagView::Builtin26_1);
        if self.player_stab_refused(connection_id, target, &registry_tags) {
            return Ok(false);
        }
        let damage_type = self.player_attack_damage_type_for_slot(connection_id, stab.slot);
        let mut base_damage = stab.base_damage;
        let mut magic_boost = self.player_weapon_combat_value_for_slot(
            connection_id,
            &dimension,
            target,
            &damage_type,
            stab.slot,
            CombatValue::Damage,
            base_damage,
        ) - base_damage;
        let player = &self.levels[connection_id];
        let held = player.using_item.is_using_item()
            && match player.using_item.used_item_hand() {
                InteractionHand::MainHand => stab.slot == EquipmentSlot::MainHand,
                InteractionHand::OffHand => stab.slot == EquipmentSlot::OffHand,
            };
        if !held {
            let scale = player.attack_strength_scale(0.5)?;
            magic_boost *= scale;
            base_damage *= 0.2 + scale * scale * 0.8;
        }
        if stab.deals_knockback
            && self.player_stab_deflects(connection_id, target, &registry_tags, events)?
        {
            return Ok(true);
        }
        let total_damage = if stab.deals_damage {
            base_damage + magic_boost
        } else {
            0.0
        };
        let (hurt_mob, old_health) = match target {
            MeleeTarget::Stored(entity_id) => {
                let Some(entity) = self.level_by_id(&dimension).entities.get(entity_id) else {
                    return Ok(false);
                };
                (
                    entity.living().map(|_| (entity.id(), entity.uuid())),
                    entity.living().map(|living| living.health()),
                )
            }
            MeleeTarget::Session(target_id) => {
                let other = &self.levels[target_id];
                (
                    Some((other.entity_id(), other.profile.uuid)),
                    Some(other.damage.health),
                )
            }
        };
        let old_movement = match target {
            MeleeTarget::Session(target_id) => Some(self.levels[target_id].delta_movement()),
            MeleeTarget::Stored(_) => None,
        };
        let target_body = hurt_mob.and_then(|_| self.melee_target_body(&dimension, target));
        let was_hurt = stab.deals_damage
            && match self.player_melee_hurt(
                connection_id,
                target,
                &damage_type,
                total_damage,
                &registry_tags,
                events,
            )? {
                MeleeHurt::Hurt => true,
                MeleeHurt::NotHurt => false,
                MeleeHurt::Refused => return Ok(false),
            };
        if stab.deals_knockback {
            let attribute_knockback = self.levels[connection_id]
                .attribute_value(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID)
                as f32;
            let knockback = 0.4
                + self.player_weapon_combat_value_for_slot(
                    connection_id,
                    &dimension,
                    target,
                    &damage_type,
                    stab.slot,
                    CombatValue::Knockback,
                    attribute_knockback,
                ) / 2.0;
            self.player_cause_extra_knockback(
                connection_id,
                &dimension,
                target,
                knockback,
                old_movement,
                events,
            );
        }
        let dismounted = stab.dismounts && self.stab_dismount(&dimension, target, events);
        if !was_hurt && !stab.deals_knockback && !dismounted {
            return Ok(false);
        }
        let target_entity_id = match target {
            MeleeTarget::Stored(entity_id) => entity_id,
            MeleeTarget::Session(target_id) => self.levels[target_id].entity_id(),
        };
        self.player_attack_visual_effects(
            connection_id,
            target_entity_id,
            false,
            true,
            stab.deals_damage,
            magic_boost,
            events,
        )?;
        self.levels[connection_id].set_last_hurt_mob(hurt_mob);
        if let Some(body) = &target_body {
            self.player_item_attack_interaction(
                connection_id,
                (target, body),
                &damage_type,
                was_hurt,
                stab.slot,
                weapon_reference,
                events,
            )?;
        }
        if let Some(old_health) = old_health {
            self.player_damage_stats_and_hearts(
                connection_id,
                &dimension,
                target,
                old_health,
                events,
            );
        }
        let player = &mut self.levels[connection_id];
        cause_food_exhaustion(
            &mut player.food,
            player.abilities.invulnerable,
            STAB_EXHAUSTION,
        );
        Ok(true)
    }

    /// `Player.deflectProjectile` (`Player.java:1019-1028`).
    fn player_stab_deflects(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<bool> {
        let MeleeTarget::Stored(entity_id) = target else {
            return Ok(false);
        };
        let player = &self.levels[connection_id];
        let (uuid, rotation, dimension) =
            (player.profile.uuid, player.rotation(), player.dimension);
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        let deflected = entities.get_mut(entity_id).is_some_and(|entity| {
            crate::entity::hurting_projectile::can_deflect_by_player_attack_with_registry_tag_view(
                entity,
                registry_tags,
            ) && crate::entity::hurting_projectile::deflect_by_player_attack(
                entity,
                uuid,
                rotation.y_rot,
                rotation.x_rot,
            )
        });
        if deflected {
            self.play_server_side_sound(connection_id, PLAYER_ATTACK_NODAMAGE_SOUND, events)?;
        }
        Ok(deflected)
    }

    /// `if (dismounts && target.isPassenger()) target.stopRiding()`.
    fn stab_dismount(
        &mut self,
        dimension: &crate::world::LevelId,
        target: MeleeTarget,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        match target {
            MeleeTarget::Stored(entity_id) => {
                let entities = &mut self.level_mut_by_id(dimension).entities;
                entities.vehicle(entity_id).is_some() && entities.stop_stored_riding(entity_id)
            }
            MeleeTarget::Session(target_id) => {
                let riding = self
                    .level_by_id(dimension)
                    .entities
                    .external_player_vehicle(self.levels[target_id].entity_id())
                    .is_some();
                riding
                    && self
                        .stop_external_player_ride(
                            dimension,
                            target_id,
                            DismountLanding::Search,
                            events,
                        )
                        .unwrap_or(false)
            }
        }
    }
}
