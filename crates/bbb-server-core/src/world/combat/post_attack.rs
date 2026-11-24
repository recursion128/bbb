//! `EnchantmentHelper.doPostAttackEffects` (`EnchantmentHelper.java:210-262`)
//! and the `EnchantmentEntityEffect`s its `post_attack` entries apply.

use crate::connection::ConnectionId;
use crate::enchantment_entity_effects::{
    sample_float_provider, EnchantmentEntityEffect, EnchantmentTarget, HolderSet,
};
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::EntityRegistryTagView;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionDamageSource, ExplosionInteraction, ExplosionSource,
    ExplosionSourceEntity, ExplosionSourceEntityKind, PendingExplosion,
};
use crate::item_durability::{damage_and_max, hurt_and_break, MineBlockOutcome};
use crate::level_event::PendingHurtEntity;
use crate::world::LevelId;
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{BlockPos, SoundEvent, SoundSource, Vec3d};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::enchanted_damage::SourceEntity;
use super::player_damage_reduction::PlayerHurtSource;
use super::player_melee::MeleeTarget;
use crate::world::{ServerWorld, WorldEvent};

#[derive(Debug, Clone)]
pub(in crate::world) struct EnchantedItem {
    pub(in crate::world) owner: Option<SourceEntity>,
    pub(in crate::world) source: EnchantedItemSource,
}

#[derive(Debug, Clone)]
pub(in crate::world) enum EnchantedItemSource {
    Equipment {
        slot: EquipmentSlot,
        reference: Option<super::EquipmentReference>,
    },
    Detached(RuntimeItemStack),
    ProjectileWeapon {
        projectile: i32,
        kill_on_attackerless_break: bool,
    },
}

impl EnchantedItem {
    pub(in crate::world) fn equipment(
        owner: SourceEntity,
        slot: EquipmentSlot,
        reference: Option<super::EquipmentReference>,
    ) -> Self {
        Self {
            owner: Some(owner),
            source: EnchantedItemSource::Equipment { slot, reference },
        }
    }

    pub(in crate::world) fn projectile_weapon(
        owner: Option<SourceEntity>,
        projectile: i32,
    ) -> Self {
        Self::projectile_weapon_on_break(owner, projectile, false)
    }

    pub(in crate::world) fn projectile_weapon_on_break(
        owner: Option<SourceEntity>,
        projectile: i32,
        kill_on_attackerless_break: bool,
    ) -> Self {
        Self {
            owner,
            source: EnchantedItemSource::ProjectileWeapon {
                projectile,
                kill_on_attackerless_break,
            },
        }
    }

    pub(in crate::world) fn detached(owner: Option<SourceEntity>, stack: RuntimeItemStack) -> Self {
        Self {
            owner,
            source: EnchantedItemSource::Detached(stack),
        }
    }
}

/// `Vec3.addLocalCoordinates` (`Vec3.java:339-341`) over
/// `applyLocalCoordinatesToRotation` (`:323-337`).
fn add_local_coordinates(look: Vec3d, local: [f64; 3]) -> Vec3d {
    use bbb_server_types::geometry::{mth_cos, mth_sin};
    let to_degrees = 180.0_f32 / std::f32::consts::PI;
    let yaw = (-look.x).atan2(look.z) as f32 * to_degrees;
    let length = (look.x * look.x + look.y * look.y + look.z * look.z).sqrt();
    let pitch = (-look.y / length).asin() as f32 * to_degrees;
    let to_radians = std::f32::consts::PI / 180.0;
    let angle = |degrees: f32| f64::from(degrees * to_radians);
    let (y_cos, y_sin) = (mth_cos(angle(yaw + 90.0)), mth_sin(angle(yaw + 90.0)));
    let (x_cos, x_sin) = (mth_cos(angle(-pitch)), mth_sin(angle(-pitch)));
    let (x_cos_up, x_sin_up) = (mth_cos(angle(-pitch + 90.0)), mth_sin(angle(-pitch + 90.0)));
    let forwards = [
        f64::from(y_cos * x_cos),
        f64::from(x_sin),
        f64::from(y_sin * x_cos),
    ];
    let up = [
        f64::from(y_cos * x_cos_up),
        f64::from(x_sin_up),
        f64::from(y_sin * x_cos_up),
    ];
    // `forwards.cross(up).scale(-1.0)`.
    let left = [
        -(forwards[1] * up[2] - forwards[2] * up[1]),
        -(forwards[2] * up[0] - forwards[0] * up[2]),
        -(forwards[0] * up[1] - forwards[1] * up[0]),
    ];
    let axis = |i: usize| forwards[i] * local[2] + up[i] * local[1] + left[i] * local[0];
    Vec3d {
        x: axis(0),
        y: axis(1),
        z: axis(2),
    }
}

/// `Mth.randomBetween` (`Mth.java:747-749`).
fn random_between(next_float: f32, min: f32, max: f32) -> f32 {
    next_float * (max - min) + min
}

/// `Math.round(float)`.
fn java_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

impl ServerWorld {
    pub(super) fn do_post_attack_effects_checked(
        &mut self,
        dimension: &LevelId,
        victim: MeleeTarget,
        source: PlayerHurtSource<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let weapon_reference = source
            .causing
            .map(|owner| match owner {
                SourceEntity::Session(id) => self.player_attack_weapon_reference(id),
                SourceEntity::Stored(_) => {
                    self.capture_equipment_reference(dimension, owner, EquipmentSlot::MainHand)
                }
            })
            .transpose()?
            .flatten();
        self.do_post_attack_effects_with_item_checked(
            dimension,
            victim,
            source,
            EquipmentSlot::MainHand,
            weapon_reference,
            events,
        )
    }

    /// `doPostAttackEffects`: the victim's worn enchantments for `VICTIM`
    /// entries, then the causing living entity's weapon for `ATTACKER` ones
    /// (`EnchantmentHelper.java:210-262`).
    pub(super) fn do_post_attack_effects_with_item_checked(
        &mut self,
        dimension: &LevelId,
        victim: MeleeTarget,
        source: PlayerHurtSource<'_>,
        weapon_slot: EquipmentSlot,
        weapon_reference: Option<super::EquipmentReference>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        if !self.post_attack_victim_exists(dimension, victim) {
            return Ok(());
        }
        let Some(attacker) = source.causing else {
            return self.victim_post_attack(dimension, victim, source, events);
        };
        let detached_spin = match (attacker, weapon_reference.as_ref()) {
            (SourceEntity::Session(id), None) => self.levels.player(id).and_then(|player| {
                player
                    .using_item
                    .is_auto_spin_attack()
                    .then(|| {
                        player
                            .using_item
                            .auto_spin_attack
                            .as_ref()
                            .map(|(_, _, stack)| stack.clone())
                    })
                    .flatten()
            }),
            _ => None,
        };
        let weapon = if let Some(super::EquipmentReference::Stored {
            dimension: source_dimension,
            reference,
        }) = weapon_reference.as_ref()
        {
            let store = &self.level_by_id(source_dimension).entities;
            let native = store.referenced_equipment_item(*reference);
            if let Some(native) = native.filter(|native| !native.is_empty()) {
                crate::entity::enchanted_damage::stack_enchantments(
                    native,
                    store.enchantment_registry(),
                )
                .into_iter()
                .filter(|(id, _)| self.enchantment_matches_slot(id, weapon_slot))
                .collect()
            } else {
                Vec::new()
            }
        } else if let Some(reference) = weapon_reference.clone() {
            let stack = self.referenced_equipment_stack(reference)?;
            if stack.is_empty() {
                Vec::new()
            } else {
                crate::entity::block_drops::loot_tool_for_runtime_stack(
                    &stack,
                    &self.datapack_tables.enchantment_registry,
                )
                .enchantments
                .into_iter()
                .filter(|(id, _)| self.enchantment_matches_slot(id, weapon_slot))
                .collect()
            }
        } else if matches!(attacker, SourceEntity::Stored(_)) {
            Vec::new()
        } else {
            self.weapon_enchantments(dimension, attacker)?
                .unwrap_or_default()
        };
        self.victim_post_attack(dimension, victim, source, events)?;
        let mut item = match detached_spin {
            Some(stack) => EnchantedItem::detached(Some(attacker), stack),
            None => EnchantedItem::equipment(attacker, weapon_slot, weapon_reference.clone()),
        };
        for (id, level) in weapon {
            self.post_attack_entries(
                dimension,
                &id,
                level,
                EnchantmentTarget::Attacker,
                &mut item,
                victim,
                source,
                events,
            )?;
        }
        if let SourceEntity::Session(id) = attacker {
            self.sync_spin_attack_stack(id, &item);
        }
        Ok(())
    }

    /// The victim's worn enchantments for `VICTIM` entries
    /// (`EnchantmentHelper.java:240-245`).
    fn victim_post_attack(
        &mut self,
        dimension: &LevelId,
        victim: MeleeTarget,
        source: PlayerHurtSource<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let owner = match victim {
            MeleeTarget::Session(id) => SourceEntity::Session(id),
            MeleeTarget::Stored(id) => SourceEntity::Stored(id),
        };
        for entry in VANILLA_EQUIPMENT_SLOTS {
            let slot = entry.slot;
            let reference = self.capture_equipment_reference(dimension, owner, slot)?;
            let mut item = EnchantedItem::equipment(owner, slot, reference);
            let enchantments = match owner {
                SourceEntity::Session(id) => self.session_slot_enchantments(id, slot)?,
                SourceEntity::Stored(id) => self.stored_slot_enchantments(dimension, id, slot)?,
            };
            for (id, level) in enchantments {
                if self.enchantment_matches_slot(&id, slot) {
                    self.post_attack_entries(
                        dimension,
                        &id,
                        level,
                        EnchantmentTarget::Victim,
                        &mut item,
                        victim,
                        source,
                        events,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// `doPostAttackEffectsWithItemSource` for an arrow or trident carrying
    /// `weapon` (`AbstractArrow.java:475`, `ThrownTrident.java:134-138`): the
    /// victim's worn enchantments, then the weapon's `ATTACKER` entries for a
    /// living attacker, or for any attacker when `attackerless` (the trident's
    /// `onBreak` arm, `EnchantmentHelper.java:255-259`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn projectile_weapon_post_attack(
        &mut self,
        dimension: &LevelId,
        victim: MeleeTarget,
        source: PlayerHurtSource<'_>,
        projectile_entity_id: i32,
        weapon: Option<&RuntimeItemStack>,
        attackerless: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        if !self.post_attack_victim_exists(dimension, victim) {
            return Ok(());
        }
        self.victim_post_attack(dimension, victim, source, events)?;
        let (Some(weapon), Some(attacker)) =
            (weapon.filter(|stack| !stack.is_empty()), source.causing)
        else {
            return Ok(());
        };
        let living_attacker = match attacker {
            SourceEntity::Session(_) => true,
            SourceEntity::Stored(id) => self
                .level_by_id(dimension)
                .entities
                .get(id)
                .is_some_and(|entity| entity.living().is_some()),
        };
        if !living_attacker && !attackerless {
            return Ok(());
        }
        let enchantments: Vec<(String, i32)> = crate::entity::enchanted_damage::stack_enchantments(
            weapon,
            &self.datapack_tables.enchantment_slots,
        )
        .into_iter()
        .filter(|(id, _)| {
            !living_attacker || self.enchantment_matches_slot(id, EquipmentSlot::MainHand)
        })
        .collect();
        let mut item = EnchantedItem::projectile_weapon_on_break(
            living_attacker.then_some(attacker),
            projectile_entity_id,
            attackerless && !living_attacker,
        );
        for (id, level) in enchantments {
            self.post_attack_entries(
                dimension,
                &id,
                level,
                EnchantmentTarget::Attacker,
                &mut item,
                victim,
                source,
                events,
            )?;
        }
        Ok(())
    }

    /// A post-attack an entity tick owed after its hurt (`Mob.java:1369`,
    /// `EvokerFangs.java:115-116`), run once the driver has applied that hurt.
    pub(in crate::world) fn queued_post_attack_checked(
        &mut self,
        dimension: &LevelId,
        victim: SourceEntity,
        damage_type: &str,
        causing: Option<PendingHurtEntity>,
        direct: Option<PendingHurtEntity>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let victim = match victim {
            SourceEntity::Session(id) => {
                let Some(connection) = self.online(id) else {
                    return Ok(());
                };
                MeleeTarget::Session(connection)
            }
            SourceEntity::Stored(id) => MeleeTarget::Stored(id),
        };
        let registry = std::sync::Arc::clone(&self.cold.damage_types);
        let Some(damage_type) = registry
            .iter()
            .find(|entry| entry.resource_id == damage_type)
        else {
            return Err(format!(
                "post_attack damage type {damage_type} is not registered"
            ));
        };
        let entity = |entity: PendingHurtEntity| match entity {
            PendingHurtEntity::Player { uuid } => {
                self.connection_of(uuid).map(SourceEntity::Session)
            }
            PendingHurtEntity::Stored { entity_id } => Some(SourceEntity::Stored(entity_id)),
            PendingHurtEntity::FallingBlock { .. } => None,
        };
        let source = PlayerHurtSource {
            damage_type,
            causing: causing.and_then(entity),
            direct: direct.and_then(entity),
        };
        self.do_post_attack_effects_checked(dimension, victim, source, events)
    }

    /// `LivingEntity.heal` on a projectile's living owner.
    fn heal_projectile_owner(
        &mut self,
        dimension: &LevelId,
        owner: Option<&PendingHurtEntity>,
        amount: f32,
    ) {
        match owner {
            Some(PendingHurtEntity::Player { uuid }) => {
                if let Some(player) = self
                    .connection_of(*uuid)
                    .and_then(|id| self.levels.player_mut(id))
                {
                    let max_health = player.max_health();
                    player.damage.heal(amount, max_health);
                }
            }
            Some(PendingHurtEntity::Stored { entity_id }) => {
                if let Some(entity) = self.level_mut_by_id(dimension).entities.get_mut(*entity_id) {
                    let health = entity.living().map_or(0.0, |living| living.health());
                    if health > 0.0 {
                        crate::entity::living::set_health(entity, health + amount);
                    }
                }
            }
            Some(PendingHurtEntity::FallingBlock { .. }) | None => {}
        }
    }

    /// The post-attack of the projectile class that dealt `hurt`, each of
    /// which gates it differently: `LargeFireball.java:47-48`,
    /// `SmallFireball.java:41-44`, `ShulkerBullet.java:284-287`, `LlamaSpit.java:69-71`,
    /// `WitherSkull.java:63-68`, `AbstractWindCharge.java:87-88`.
    pub(in crate::world) fn projectile_post_attack(
        &mut self,
        dimension: &LevelId,
        hurt: &crate::entity::projectile_action::PendingProjectileHurt,
        hurt_accepted: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        if !matches!(hurt.direct, Some(PendingHurtEntity::Stored { .. })) {
            return Ok(());
        }
        let victim = match hurt.target {
            PendingHurtEntity::Player { uuid } => match self.connection_of(uuid) {
                Some(id) => SourceEntity::Session(id),
                None => return Ok(()),
            },
            PendingHurtEntity::Stored { entity_id } => SourceEntity::Stored(entity_id),
            PendingHurtEntity::FallingBlock { .. } => return Ok(()),
        };
        let entities = &self.level_by_id(dimension).entities;
        // `WitherSkull.java:66-71`: a kill heals the living owner instead.
        let skull_kill = hurt.direct_resource_id == "minecraft:wither_skull"
            && hurt_accepted
            && hurt.damage_type == "minecraft:wither_skull"
            && match victim {
                SourceEntity::Session(id) => self
                    .levels
                    .player(id)
                    .is_none_or(|player| player.damage.is_dead_or_dying()),
                SourceEntity::Stored(id) => {
                    !entities
                        .get(id)
                        .is_some_and(crate::entity::Entity::is_alive_with_health)
                        && crate::entity::ender_dragon::part_by_id(entities, id).is_none()
                }
            };
        if skull_kill {
            self.heal_projectile_owner(dimension, hurt.causing.as_ref(), 5.0);
            return Ok(());
        }
        let runs = match hurt.direct_resource_id {
            "minecraft:fireball" => true,
            "minecraft:small_fireball" | "minecraft:shulker_bullet" | "minecraft:llama_spit" => {
                hurt_accepted
            }
            // Only a living owner's skull deals `wither_skull` damage.
            "minecraft:wither_skull" => {
                hurt_accepted && hurt.damage_type == "minecraft:wither_skull"
            }
            "minecraft:wind_charge" | "minecraft:breeze_wind_charge" => {
                hurt_accepted
                    && match victim {
                        SourceEntity::Session(_) => true,
                        SourceEntity::Stored(id) => entities
                            .get(id)
                            .is_some_and(|entity| entity.living().is_some()),
                    }
            }
            _ => false,
        };
        if runs {
            self.queued_post_attack_checked(
                dimension,
                victim,
                hurt.damage_type,
                hurt.causing.clone(),
                hurt.direct.clone(),
                events,
            )?;
        }
        Ok(())
    }

    /// `Enchantment.doPostAttack` (`Enchantment.java:316-349`) for one
    /// `(enchantment, level)` in `for_target`'s role.
    #[allow(clippy::too_many_arguments)]
    fn post_attack_entries(
        &mut self,
        dimension: &LevelId,
        enchantment_id: &str,
        level: i32,
        for_target: EnchantmentTarget,
        item: &mut EnchantedItem,
        victim: MeleeTarget,
        source: PlayerHurtSource<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_entity_effects);
        let entries = table
            .get(enchantment_id)
            .ok_or_else(|| format!("enchantment {enchantment_id} is not loaded"))?
            .post_attack
            .as_ref()
            .map_err(Clone::clone)?;
        for entry in entries.iter().filter(|entry| entry.enchanted == for_target) {
            if !self.post_attack_victim_exists(dimension, victim) {
                return Ok(());
            }
            let matched = match &entry.requirements {
                None => Ok(true),
                Some(condition) => self.enchanted_damage_condition(
                    dimension,
                    enchantment_id,
                    level,
                    victim,
                    &source.enchanted(),
                    condition,
                ),
            };
            match matched {
                Ok(true) => {}
                Ok(false) => continue,
                Err(detail) => {
                    return Err(format!(
                        "{enchantment_id} post_attack requirements: {detail}"
                    ))
                }
            }
            let target = match entry.affected {
                EnchantmentTarget::Attacker => source.causing,
                EnchantmentTarget::DamagingEntity => source.direct,
                EnchantmentTarget::Victim => Some(match victim {
                    MeleeTarget::Session(id) => SourceEntity::Session(id),
                    MeleeTarget::Stored(id) => SourceEntity::Stored(id),
                }),
            };
            let Some(target) = target else {
                continue;
            };
            let Some(target_position) = self.entity_position(dimension, target) else {
                continue;
            };
            // An effect that removes the target still lets later effects read it.
            let captured = match target {
                SourceEntity::Stored(id) => self.level_by_id(dimension).entities.get(id).cloned(),
                SourceEntity::Session(_) => None,
            };
            self.apply_entity_effect(
                dimension,
                &entry.effect,
                level,
                item,
                target,
                target_position,
                captured.as_ref(),
                events,
            )
            .map_err(|detail| format!("{enchantment_id} post_attack: {detail}"))?;
        }
        Ok(())
    }

    /// `EnchantmentEntityEffect.apply` on `target` at `position`; `captured`
    /// is a stored target's body as it was before the effects ran.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn apply_entity_effect(
        &mut self,
        dimension: &LevelId,
        effect: &EnchantmentEntityEffect,
        level: i32,
        item: &mut EnchantedItem,
        target: SourceEntity,
        position: Vec3d,
        captured: Option<&crate::entity::Entity>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        match effect {
            EnchantmentEntityEffect::AllOf(effects) => {
                for effect in effects {
                    self.apply_entity_effect(
                        dimension, effect, level, item, target, position, captured, events,
                    )?;
                }
                Ok(())
            }
            EnchantmentEntityEffect::RunFunction(function) => {
                use crate::commands::types::CommandDamageEntityRef;
                use crate::commands::{CommandOrigin, CommandPermission, CommandStoredEntity};
                let level_id = *dimension;
                let dimension = self.level_key(level_id);
                let (executing, rotation, eye_height) = match target {
                    SourceEntity::Session(id) => {
                        let player = self.levels.player(id).ok_or("the session left")?;
                        let command = CommandDamageEntityRef::Player(
                            self.command_player_by_id(player.profile.uuid)
                                .ok_or("the session left")?,
                        );
                        (
                            command.as_command_source(CommandPermission::GameMaster),
                            [player.rotation().y_rot, player.rotation().x_rot],
                            player.eye_height().ok(),
                        )
                    }
                    SourceEntity::Stored(id) => {
                        let Some(entity) =
                            self.level_by_id(&level_id).entities.get(id).or(captured)
                        else {
                            return Ok(());
                        };
                        let command = CommandDamageEntityRef::Stored(CommandStoredEntity {
                            entity,
                            dimension,
                            registry_tags: None,
                        });
                        (
                            command.as_command_source_with_name(
                                CommandPermission::GameMaster,
                                self.stored_display_name_of(entity),
                                self.plugins().content_registry(),
                            ),
                            [entity.y_rot(), entity.x_rot()],
                            Some(entity.eye_height()),
                        )
                    }
                };
                let origin = CommandOrigin {
                    position: [position.x, position.y, position.z],
                    rotation,
                    level: dimension.clone(),
                    anchor: bbb_protocol::entity_anchors::EntityAnchor::Feet,
                    eye_height,
                };
                self.execute_enchantment_function(function, executing, origin, events);
                Ok(())
            }
            EnchantmentEntityEffect::Ignite { duration } => {
                // `Entity.igniteForSeconds` (`Entity.java:625-635`), scaled by
                // `BURNING_TIME` for living targets (`LivingEntity.java:3926`).
                let ticks = (duration.calculate(level) * 20.0).floor() as i32;
                match target {
                    SourceEntity::Session(id) => {
                        let player = self.levels.player_mut(id).ok_or("the session left")?;
                        player.ignite_environment(ticks);
                    }
                    SourceEntity::Stored(id) => {
                        if let Some(entity) = self.level_mut_by_id(dimension).entities.get_mut(id) {
                            crate::entity::living::ignite_living_for_ticks(entity, ticks);
                        }
                    }
                }
                Ok(())
            }
            EnchantmentEntityEffect::ApplyMobEffect {
                to_apply,
                min_duration,
                max_duration,
                min_amplifier,
                max_amplifier,
            } => {
                if let SourceEntity::Stored(id) = target {
                    let living = self
                        .level_by_id(dimension)
                        .entities
                        .get(id)
                        .is_some_and(|entity| entity.living().is_some());
                    if !living {
                        return Ok(());
                    }
                }
                let candidates = self.holder_set(dimension, "minecraft:mob_effect", to_apply)?;
                // `ApplyMobEffect.apply` (`ApplyMobEffect.java:39-54`): pick,
                // duration, then amplifier, all off the living entity's random.
                let (min_d, max_d) = (min_duration.calculate(level), max_duration.calculate(level));
                let (min_a, max_a) = (
                    min_amplifier.calculate(level),
                    max_amplifier.calculate(level),
                );
                let draws = |random: &crate::random::GameRandom| {
                    if candidates.is_empty() {
                        return None;
                    }
                    let selected = &candidates[random.next_int(candidates.len() as i32) as usize];
                    let ticks =
                        java_round(random_between(random.next_float(), min_d, max_d) * 20.0);
                    let amplifier =
                        java_round(random_between(random.next_float(), min_a, max_a)).max(0);
                    Some((selected.clone(), ticks, amplifier))
                };
                let drawn = draws(&self.random.clone());
                let Some((effect_id, ticks, amplifier)) = drawn else {
                    return Ok(());
                };
                let static_id = vanilla_mob_effect_type_for_resource_id(&effect_id)
                    .map(|entry| entry.resource_id)
                    .ok_or_else(|| format!("unknown mob effect {effect_id}"))?;
                let Some(effect) = ActiveMobEffect::new(static_id, ticks, amplifier) else {
                    return Ok(());
                };
                let added = match target {
                    SourceEntity::Session(id) => self.add_player_mob_effect(id, effect, events),
                    SourceEntity::Stored(id) => self.add_stored_mob_effect(dimension, id, effect),
                };
                added
                    .map(|_| ())
                    .ok_or_else(|| "the effect target is missing".to_string())
            }
            EnchantmentEntityEffect::ApplyImpulse {
                direction,
                coordinate_scale,
                magnitude,
            } => {
                // `ApplyEntityImpulse.apply` (`ApplyEntityImpulse.java:24-33`).
                let rotation = match target {
                    SourceEntity::Session(id) => {
                        let rotation = self.levels.player(id).ok_or("the session left")?.rotation();
                        (rotation.x_rot, rotation.y_rot)
                    }
                    SourceEntity::Stored(id) => {
                        let Some(entity) = self.level_by_id(dimension).entities.get(id) else {
                            return Ok(());
                        };
                        (entity.x_rot(), entity.y_rot())
                    }
                };
                let look = crate::entity::ai::mob::stroll::view_vector(rotation.0, rotation.1);
                let local = add_local_coordinates(look, *direction);
                let scale = f64::from(magnitude.calculate(level));
                let push = Vec3d {
                    x: local.x * coordinate_scale[0] * scale,
                    y: local.y * coordinate_scale[1] * scale,
                    z: local.z * coordinate_scale[2] * scale,
                };
                match target {
                    SourceEntity::Session(id) => {
                        let player = self.levels.player_mut(id).ok_or("the session left")?;
                        let delta = player.delta_movement();
                        player.set_delta_movement(Vec3d {
                            x: delta.x + push.x,
                            y: delta.y + push.y,
                            z: delta.z + push.z,
                        });
                        player.hurt_marked = true;
                        player.cold.fall_impulse.apply_grace(10);
                    }
                    SourceEntity::Stored(id) => {
                        if let Some(entity) = self.level_mut_by_id(dimension).entities.get_mut(id) {
                            let delta = entity.delta_movement();
                            entity.set_delta_movement(Vec3d {
                                x: delta.x + push.x,
                                y: delta.y + push.y,
                                z: delta.z + push.z,
                            });
                            entity.set_needs_sync(true);
                            if let Some(living) = entity.living_mut() {
                                living.fall_impulse.apply_grace(10);
                            }
                        }
                    }
                }
                Ok(())
            }
            EnchantmentEntityEffect::ApplyExhaustion(amount) => {
                // `ApplyExhaustion.apply` (`ApplyExhaustion.java:18-22`).
                if let SourceEntity::Session(id) = target {
                    let player = self.levels.player_mut(id).ok_or("the session left")?;
                    bbb_server_types::food::cause_food_exhaustion(
                        &mut player.food,
                        player.abilities.invulnerable,
                        amount.calculate(level),
                    );
                }
                Ok(())
            }
            EnchantmentEntityEffect::ChangeItemDamage(amount) => {
                // `ChangeItemDamage.apply` (`ChangeItemDamage.java:20-27`).
                let change = amount.calculate(level) as i32;
                self.change_enchanted_item_damage(dimension, item, change, events)
            }
            EnchantmentEntityEffect::DamageEntity {
                damage_type,
                min_damage,
                max_damage,
            } => {
                // `DamageEntity.apply` (`DamageEntity.java:26-29`).
                let (min, max) = (min_damage.calculate(level), max_damage.calculate(level));
                let damage = random_between(self.random.next_float(), min, max);
                let owner = item
                    .owner
                    .map(|owner| self.pending_hurt_entity(owner))
                    .transpose()?
                    .and_then(|owner| self.pending_hurt_source_entity(dimension, owner));
                match target {
                    SourceEntity::Session(id) => {
                        self.hurt_connected_player_checked(
                            id,
                            damage_type,
                            damage,
                            owner.clone(),
                            owner,
                            None,
                            events,
                        )
                        .ok_or("the original post-attack session hurt could not be published")?;
                    }
                    SourceEntity::Stored(id) => {
                        let registry_tags = self
                            .level_by_id(dimension)
                            .chunks
                            .registry_tags_shared()
                            .map(EntityRegistryTagView::Frozen)
                            .unwrap_or(EntityRegistryTagView::Builtin26_1);
                        if self
                            .level_by_id(dimension)
                            .entities
                            .get(id)
                            .is_none_or(|body| body.removal_reason().is_some())
                        {
                            return Ok(());
                        }
                        self.hurt_stored_entity_from_level(
                            *dimension,
                            id,
                            damage_type,
                            damage,
                            owner.clone(),
                            owner,
                            None,
                            registry_tags,
                            false,
                            None,
                            events,
                        )
                        .ok_or("the original post-attack damage could not be published")?;
                    }
                }
                Ok(())
            }
            EnchantmentEntityEffect::PlaySound {
                sounds,
                volume,
                pitch,
            } => {
                // `PlaySoundEffect.apply` (`PlaySoundEffect.java:29-44`).
                let (silent, source) = match target {
                    SourceEntity::Session(id) => {
                        let player = self.levels.player(id).ok_or("the session left")?;
                        (player.silent, SoundSource::Players)
                    }
                    SourceEntity::Stored(id) => {
                        let Some(entity) =
                            self.level_by_id(dimension).entities.get(id).or(captured)
                        else {
                            return Ok(());
                        };
                        (
                            entity.is_silent(),
                            crate::entity::living::living_sound_source(entity),
                        )
                    }
                };
                if silent || sounds.is_empty() {
                    return Ok(());
                }
                let index = (level - 1).clamp(0, sounds.len() as i32 - 1) as usize;
                let sample = |random: &crate::random::GameRandom| {
                    let volume = sample_float_provider(volume, || random.next_float());
                    let pitch = sample_float_provider(pitch, || random.next_float());
                    (volume, pitch)
                };
                let (volume, pitch) = sample(&self.random.clone());
                let sound = SoundEvent {
                    sound: sounds[index].clone(),
                    source,
                    position,
                    volume,
                    pitch,
                    seed: self.random.next_long(),
                };
                self.broadcast_level_sound_by_key(dimension, &sound, None, events);
                Ok(())
            }
            EnchantmentEntityEffect::SummonEntity {
                entity_types,
                join_team,
            } => {
                // `SummonEntityEffect.apply` (`SummonEntityEffect.java:32-51`).
                let block = BlockPos {
                    x: position.x.floor() as i32,
                    y: position.y.floor() as i32,
                    z: position.z.floor() as i32,
                };
                if !(-30_000_000..30_000_000).contains(&block.x)
                    || !(-30_000_000..30_000_000).contains(&block.z)
                    || !(-20_000_000..20_000_000).contains(&block.y)
                {
                    return Ok(());
                }
                let candidates =
                    self.holder_set(dimension, "minecraft:entity_type", entity_types)?;
                if candidates.is_empty() {
                    return Ok(());
                }
                let level_state = self.level_mut_by_id(dimension);
                let pick = level_state.random.next_int(candidates.len() as i32) as usize;
                let entity_type = &candidates[pick];
                let Some(entity_type_id) =
                    bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(entity_type)
                        .map(|facts| facts.id)
                        .or_else(|| {
                            self.plugins()
                                .content_registry()
                                .entity_by_resource_id(entity_type)
                                .map(|spec| spec.id)
                        })
                else {
                    return Ok(());
                };
                let team = if *join_team {
                    use crate::commands::types::CommandDamageEntityRef as EntityRef;
                    let store = &self.level_by_id(dimension).entities;
                    let player = match target {
                        SourceEntity::Session(id) => self.levels.player(id),
                        SourceEntity::Stored(id) => store.player(id),
                    };
                    let affected = if let Some(player) = player {
                        Some(EntityRef::Player(
                            player.command_row(self.level_key(player.dimension)),
                        ))
                    } else if let SourceEntity::Stored(id) = target {
                        store.get(id).or(captured).map(|entity| {
                            EntityRef::Stored(crate::commands::CommandStoredEntity {
                                entity,
                                dimension: self.level_key(*dimension),
                                registry_tags: None,
                            })
                        })
                    } else {
                        None
                    };
                    let reads = self.entity_reads();
                    affected
                        .and_then(|entity| {
                            self.scoreboard.board().entity_team_name(entity, |uuid| {
                                reads.living_command_entity_by_uuid(uuid)
                            })
                        })
                        .cloned()
                } else {
                    None
                };
                // `EntityType.create` (`EntityType.java:1340-1348`).
                let yaw = crate::world::wrap_degrees(self.random.next_float() * 360.0);
                let Some(entity_id) = self.spawn_finalized_entity(
                    *dimension,
                    entity_type_id,
                    Vec3d {
                        x: f64::from(block.x) + 0.5,
                        y: f64::from(block.y),
                        z: f64::from(block.z) + 0.5,
                    },
                    crate::entity::SpawnReason::Triggered,
                    Some(yaw),
                ) else {
                    return Ok(());
                };
                let difficulty = self.admin.settings().difficulty;
                self.with_level_and_entities(*dimension, |level, entities| {
                    entities.set_difficulty(difficulty);
                    entities.play_spawn_ambient(entity_id, Some(level));
                });
                let cause = item
                    .owner
                    .and_then(|owner| match owner {
                        SourceEntity::Session(id) => self.levels.player(id),
                        SourceEntity::Stored(id) => self.level_by_id(dimension).entities.player(id),
                    })
                    .map(|player| player.profile.uuid);
                let entity = self
                    .level_mut_by_id(dimension)
                    .entities
                    .get_mut(entity_id)
                    .expect("a summoned entity was published");
                entity.set_lightning_cause(cause);
                entity.set_pos(position);
                let uuid = entity.uuid();
                if let Some(team) = team {
                    self.scoreboard_mut()
                        .add_player_to_team(&uuid.to_string(), &team);
                }
                Ok(())
            }
            EnchantmentEntityEffect::Explode(explosion) => {
                // ExplodeEffect.java:61-89: attribution uses the affected entity.
                let source = if explosion.attribute_to_user {
                    self.enchantment_explosion_source(
                        dimension,
                        target,
                        captured,
                        explosion.damage_type.is_some(),
                    )
                } else {
                    None
                };
                let center = position.plus(explosion.offset);
                self.explode(
                    *dimension,
                    PendingExplosion {
                        source,
                        center,
                        radius: explosion.radius.calculate(level).max(0.0),
                        fire: explosion.create_fire,
                        interaction: explosion.interaction,
                        damage_source: explosion.damage_type.as_ref().map(|damage_type| {
                            ExplosionDamageSource {
                                damage_type: damage_type.clone(),
                                explicit_position: (!explosion.attribute_to_user).then_some(center),
                            }
                        }),
                        damage_profile: ExplosionDamageProfile::Simple {
                            explodes_blocks: explosion.interaction != ExplosionInteraction::None,
                            damages_entities: explosion.damage_type.is_some(),
                            knockback_multiplier: explosion.knockback_multiplier.calculate(level),
                            immune_blocks: explosion.immune_blocks.clone(),
                        },
                        presentation: explosion.presentation.clone(),
                    },
                    events,
                );
                Ok(())
            }
            EnchantmentEntityEffect::SpawnParticles(particles) => {
                // `SpawnParticlesEffect.apply` (`SpawnParticlesEffect.java:57-73`).
                match target {
                    SourceEntity::Session(id) => {
                        let movement = self
                            .player_known_movement(id)
                            .ok_or("the session's known movement is unavailable")?;
                        let bounds = self
                            .levels
                            .player(id)
                            .ok_or("the session left")?
                            .bounding_box()
                            .map_err(|error| error.to_string())?;
                        let dimensions = [
                            (bounds.max_x - bounds.min_x) as f32,
                            (bounds.max_y - bounds.min_y) as f32,
                        ];
                        let packet = particles.packet(position, dimensions, movement, &self.random);
                        self.publish_player_particles(id, packet, events);
                    }
                    SourceEntity::Stored(id) => {
                        let level_state = self.level_mut_by_id(dimension);
                        let store = &mut level_state.entities;
                        let Some(entity) = store.get(id).or(captured) else {
                            return Ok(());
                        };
                        let bounds = entity.bounding_box();
                        let dimensions = [
                            (bounds.max_x - bounds.min_x) as f32,
                            (bounds.max_y - bounds.min_y) as f32,
                        ];
                        let movement = crate::entity::location_effects::known_movement(
                            entity,
                            crate::entity::store::EntityBodyReadView::new(
                                store,
                                Some(entity),
                                None,
                            ),
                        )
                        .ok_or("the known movement of a player-controlled entity is not tracked")?;
                        let packet =
                            particles.packet(position, dimensions, movement, store.random());
                        store.send_particles(packet);
                    }
                }
                Ok(())
            }
            EnchantmentEntityEffect::ReplaceBlock { .. }
            | EnchantmentEntityEffect::SetBlockProperties { .. } => {
                self.apply_single_block_effect(dimension, effect, target, position)
            }
            EnchantmentEntityEffect::ReplaceDisk { .. } => {
                self.replace_disk(dimension, effect, level, target, position)
            }
            EnchantmentEntityEffect::Unmodelled(effect_type) => Err(format!(
                "the effect {effect_type} has no owner in this server"
            )),
        }
    }

    fn change_enchanted_item_damage(
        &mut self,
        dimension: &LevelId,
        item: &mut EnchantedItem,
        change: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        match &mut item.source {
            EnchantedItemSource::Equipment { slot, reference } => {
                let Some(owner) = item.owner else {
                    return Ok(());
                };
                self.change_equipped_enchanted_item_damage(
                    dimension,
                    owner,
                    *slot,
                    reference.clone(),
                    change,
                    events,
                )
            }
            EnchantedItemSource::Detached(stack) => self
                .change_enchanted_stack_damage(dimension, item.owner, stack, change, events)
                .map(|_| ()),
            EnchantedItemSource::ProjectileWeapon {
                projectile,
                kill_on_attackerless_break,
            } => {
                let Some((mut weapon, is_trident)) = self
                    .level_by_id(dimension)
                    .entities
                    .get(*projectile)
                    .and_then(|entity| {
                        let weapon = crate::entity::enchanted_damage::weapon_item(entity)?.clone();
                        let is_trident =
                            matches!(entity.data(), crate::entity::EntityData::Trident(_));
                        Some((weapon, is_trident))
                    })
                else {
                    return Ok(());
                };
                let Some(broken_item_id) = self.change_enchanted_stack_damage(
                    dimension,
                    item.owner,
                    &mut weapon,
                    change,
                    events,
                )?
                else {
                    return Ok(());
                };
                let broke = broken_item_id.is_some();
                if is_trident {
                    if broke && *kill_on_attackerless_break && item.owner.is_none() {
                        // `ThrownTrident.java:136`, `Entity.java:399-402`.
                        if let Some(removed) = self
                            .level_mut_by_id(dimension)
                            .entities
                            .remove(*projectile, crate::entity::RemovalReason::Killed)
                        {
                            self.level_mut_by_id(dimension)
                                .event_sink
                                .game_event_with_source(
                                    crate::level_event::VanillaGameEvent::ENTITY_DIE,
                                    crate::world::player_packets::block_position(
                                        removed.position(),
                                    ),
                                    Some(removed.id()),
                                    None,
                                );
                        }
                    } else {
                        if let Some(entity) = self
                            .level_mut_by_id(dimension)
                            .entities
                            .get_mut(*projectile)
                        {
                            crate::entity::abstract_arrow::set_item(
                                entity,
                                if broke {
                                    RuntimeItemStack::empty()
                                } else {
                                    weapon
                                },
                            );
                        }
                        if let (Some(broken_item_id), Some(SourceEntity::Session(id))) =
                            (broken_item_id, item.owner)
                        {
                            if let Some(id) = self.online(id) {
                                self.on_player_equipped_item_broken_checked(
                                    id,
                                    EquipmentSlot::MainHand,
                                    broken_item_id,
                                    events,
                                )?;
                            }
                        }
                    }
                } else {
                    if let Some(entity) = self
                        .level_mut_by_id(dimension)
                        .entities
                        .get_mut(*projectile)
                    {
                        crate::entity::abstract_arrow::set_weapon(
                            entity,
                            (!broke).then_some(weapon),
                        );
                    }
                    if let (Some(broken_item_id), Some(SourceEntity::Session(id))) =
                        (broken_item_id, item.owner)
                    {
                        if let Some(id) = self.online(id) {
                            self.on_player_equipped_item_broken_checked(
                                id,
                                EquipmentSlot::MainHand,
                                broken_item_id,
                                events,
                            )?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    pub(super) fn change_enchanted_stack_damage(
        &mut self,
        dimension: &LevelId,
        owner: Option<SourceEntity>,
        native: &mut RuntimeItemStack,
        change: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<Option<Option<i32>>, String> {
        if native.is_empty() {
            return Ok(None);
        }
        let mut stack = native.summary().clone();
        let Some((before, max_damage)) = damage_and_max(&stack) else {
            return Ok(None);
        };
        let player = owner.and_then(|owner| match owner {
            SourceEntity::Session(id) => self.levels.player(id),
            SourceEntity::Stored(id) => self.level_by_id(dimension).entities.player(id),
        });
        let infinite_materials = player.is_some_and(|player| player.abilities.instabuild);
        let connection = player.and_then(|player| self.connection_of(player.profile.uuid));
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_item_damage);
        let enchantments = Self::resolve_value_effects(&stack, &table);
        let mut outcome = MineBlockOutcome::default();
        hurt_and_break(
            &mut stack,
            change,
            infinite_materials,
            &enchantments,
            &self.random,
            &self.value_effect_level(dimension),
            &mut outcome,
        );
        drop(enchantments);
        if let (Some(connection), Some(raw_new_damage)) = (connection, outcome.raw_new_damage) {
            self.trigger_item_durability_changed(connection, native, raw_new_damage, events);
        }
        let broken_item_id = outcome.broken_item_id;
        let broke = broken_item_id.is_some();
        let damage = if broke {
            max_damage
        } else {
            damage_and_max(&stack).map_or(before, |(damage, _)| damage)
        };
        if !broke && damage == before {
            return Ok(None);
        }
        let mut edited = native
            .clone()
            .with_component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                Some(&NbtSummaryValue::Int(damage)),
                &self.item_stack_resolver(),
            )
            .map_err(|error| error.to_string())?;
        if broke {
            edited.shrink(1);
        }
        *native = edited;
        Ok(Some(broken_item_id))
    }

    fn change_equipped_enchanted_item_damage(
        &mut self,
        dimension: &LevelId,
        owner: SourceEntity,
        slot: EquipmentSlot,
        reference: Option<super::EquipmentReference>,
        change: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let reference = match reference {
            Some(reference) => reference,
            None => {
                let Some(reference) = self.capture_equipment_reference(dimension, owner, slot)?
                else {
                    return Ok(());
                };
                reference
            }
        };
        match owner {
            SourceEntity::Session(id) => self.hurt_and_break_session_reference_checked(
                self.online(id).ok_or("the item owner left")?,
                slot,
                reference,
                change,
                events,
            ),
            SourceEntity::Stored(_) => {
                let super::EquipmentReference::Stored {
                    dimension: source_dimension,
                    reference,
                } = reference
                else {
                    return Err("the stored effect has a non-stored item reference".into());
                };
                let Some(mut native) = self
                    .level_by_id(&source_dimension)
                    .entities
                    .referenced_equipment_item(reference)
                    .cloned()
                else {
                    return Ok(());
                };
                let Some(broken_item_id) = self.change_enchanted_stack_damage(
                    &source_dimension,
                    Some(owner),
                    &mut native,
                    change,
                    events,
                )?
                else {
                    return Ok(());
                };
                self.level_mut_by_id(&source_dimension)
                    .entities
                    .update_equipment_item_reference(reference, native)
                    .ok_or("the original enchanted stack could not be updated")?;
                if broken_item_id.is_some() {
                    self.with_stored_hurt_level_checked(
                        &source_dimension,
                        None,
                        |store, level, _| {
                            store.on_equipment_item_reference_broken(reference, level, slot)
                        },
                    )
                    .ok_or("the equipment break level publication refused")??;
                }
                Ok(())
            }
        }
    }

    /// One stored entity's equipment stack and its wire projection, or `None`
    /// for an empty slot or a non-living entity.
    fn stored_slot_stack(
        &self,
        dimension: &LevelId,
        entity_id: i32,
        slot: EquipmentSlot,
    ) -> Result<Option<(RuntimeItemStack, ItemStackSummary)>, String> {
        let Some(native) = self
            .level_by_id(dimension)
            .entities
            .get(entity_id)
            .and_then(|entity| entity.living())
            .and_then(|living| living.item_by_slot(slot))
            .filter(|stack| !stack.is_empty())
            .cloned()
        else {
            return Ok(None);
        };
        let summary = native.summary().clone();
        Ok(Some((native, summary)))
    }

    fn sync_spin_attack_stack(&mut self, connection_id: ConnectionId, item: &EnchantedItem) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if !player.using_item.is_auto_spin_attack() {
            return;
        }
        let updated = match &item.source {
            EnchantedItemSource::Detached(stack) => Some(stack.clone()),
            EnchantedItemSource::Equipment {
                reference: Some(reference),
                ..
            } => self.referenced_equipment_stack(reference.clone()).ok(),
            _ => None,
        };
        if let (Some(updated), Some(player)) = (updated, self.levels.player_mut(connection_id)) {
            if let Some((_, _, spin_stack)) = player.using_item.auto_spin_attack.as_mut() {
                *spin_stack = updated;
            }
        }
    }

    /// `attacker.getWeaponItem()` for a living causing entity, iterated with
    /// `MAINHAND` slot matching (`EnchantmentHelper.java:210-215`, `:248-254`);
    /// `None` for a non-living one.
    fn weapon_enchantments(
        &self,
        dimension: &LevelId,
        attacker: SourceEntity,
    ) -> Result<Option<Vec<(String, i32)>>, String> {
        let enchantments = match attacker {
            SourceEntity::Session(id) => {
                let Some(player) = self.levels.player(id) else {
                    return Ok(None);
                };
                let stack = super::player_weapon::player_weapon_stack(player);
                if stack.is_empty() || stack.summary().component_patch.enchantments.is_empty() {
                    Vec::new()
                } else {
                    crate::entity::block_drops::loot_tool_for_runtime_stack(
                        stack,
                        &self.datapack_tables.enchantment_registry,
                    )
                    .enchantments
                    .into_iter()
                    .collect()
                }
            }
            SourceEntity::Stored(id) => {
                let living = self
                    .level_by_id(dimension)
                    .entities
                    .get(id)
                    .is_some_and(|entity| entity.living().is_some());
                if !living {
                    return Ok(None);
                }
                self.stored_slot_enchantments(dimension, id, EquipmentSlot::MainHand)?
            }
        };
        Ok(Some(
            enchantments
                .into_iter()
                .filter(|(id, _)| self.enchantment_matches_slot(id, EquipmentSlot::MainHand))
                .collect(),
        ))
    }

    /// `EnchantmentHelper.doPostPiercingAttackEffects`
    /// (`EnchantmentHelper.java:218-225`) over `Enchantment.doPostPiercingAttack`
    /// (`Enchantment.java:351-357`): each main-hand enchantment's
    /// `post_piercing_attack` effects, required in the user's entity context
    /// and applied to the user.
    pub(in crate::world) fn post_piercing_attack(
        &mut self,
        dimension: LevelId,
        user: SourceEntity,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let Some(enchantments) = self.weapon_enchantments(&dimension, user)? else {
            return Ok(());
        };
        if enchantments.is_empty() {
            return Ok(());
        }
        let mut item = match user {
            SourceEntity::Session(id) => {
                let reference = self.player_attack_weapon_reference(id)?;
                match reference {
                    Some(reference) => {
                        EnchantedItem::equipment(user, EquipmentSlot::MainHand, Some(reference))
                    }
                    None => match self.levels.player(id).and_then(|player| {
                        player
                            .using_item
                            .is_auto_spin_attack()
                            .then(|| {
                                player
                                    .using_item
                                    .auto_spin_attack
                                    .as_ref()
                                    .map(|(_, _, stack)| stack.clone())
                            })
                            .flatten()
                    }) {
                        Some(stack) => EnchantedItem::detached(Some(user), stack),
                        None => EnchantedItem::equipment(user, EquipmentSlot::MainHand, None),
                    },
                }
            }
            SourceEntity::Stored(_) => {
                EnchantedItem::equipment(user, EquipmentSlot::MainHand, None)
            }
        };
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_entity_effects);
        for (enchantment_id, level) in enchantments {
            let entries = table
                .get(&enchantment_id)
                .ok_or_else(|| format!("enchantment {enchantment_id} is not loaded"))?
                .post_piercing_attack
                .as_ref()
                .map_err(Clone::clone)?;
            for entry in entries {
                let position = match user {
                    SourceEntity::Session(id) => {
                        self.levels.player(id).ok_or("the session left")?.position()
                    }
                    SourceEntity::Stored(id) => self
                        .level_by_id(&dimension)
                        .entities
                        .get(id)
                        .ok_or("the attacker left the level")?
                        .position(),
                };
                if let Some(condition) = &entry.requirements {
                    let matched = match user {
                        SourceEntity::Session(id) => self.session_requirements_match(
                            id,
                            &enchantment_id,
                            level,
                            None,
                            condition,
                        )?,
                        SourceEntity::Stored(_) => self.enchantment_entity_requirements_match(
                            &dimension, user, position, None, level, condition,
                        )?,
                    };
                    if !matched {
                        continue;
                    }
                }
                self.apply_entity_effect(
                    &dimension,
                    &entry.effect,
                    level,
                    &mut item,
                    user,
                    position,
                    None,
                    events,
                )
                .map_err(|detail| format!("{enchantment_id} post_piercing_attack: {detail}"))?;
            }
        }
        if let SourceEntity::Session(id) = user {
            self.sync_spin_attack_stack(id, &item);
        }
        Ok(())
    }

    fn enchantment_matches_slot(&self, enchantment_id: &str, slot: EquipmentSlot) -> bool {
        self.datapack_tables
            .enchantment_matches_slot(enchantment_id, slot)
    }

    fn session_slot_enchantments(
        &self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
    ) -> Result<Vec<(String, i32)>, String> {
        let player = self
            .levels
            .player(connection_id)
            .ok_or("original equipment owner unavailable")?;
        let Some(stack) = player
            .inventory
            .equipment_slots()
            .find(|(candidate, _)| *candidate == slot)
            .and_then(|(_, index)| player.inventory.slot(index))
        else {
            return Ok(Vec::new());
        };
        let patch = &stack.summary().component_patch;
        if stack.is_empty()
            || patch.enchantments.is_empty()
            || patch.removed_type_ids.contains(
                &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            )
        {
            return Ok(Vec::new());
        }
        Ok(patch
            .enchantments
            .iter()
            .filter_map(|entry| {
                if !(0..=255).contains(&entry.level) {
                    return None;
                }
                let index = usize::try_from(entry.holder_id).ok()?;
                let id = self.datapack_tables.enchantment_registry.get(index)?;
                Some((id.clone(), entry.level))
            })
            .collect())
    }

    fn stored_slot_enchantments(
        &self,
        dimension: &LevelId,
        entity_id: i32,
        slot: EquipmentSlot,
    ) -> Result<Vec<(String, i32)>, String> {
        let Some((_, stack)) = self.stored_slot_stack(dimension, entity_id, slot)? else {
            return Ok(Vec::new());
        };
        stack
            .component_patch
            .enchantments
            .iter()
            .map(|entry| {
                usize::try_from(entry.holder_id)
                    .ok()
                    .and_then(|index| self.datapack_tables.enchantment_registry.get(index))
                    .map(|id| (id.clone(), entry.level))
                    .ok_or_else(|| {
                        format!(
                            "enchantment holder {} is not in the registry",
                            entry.holder_id
                        )
                    })
            })
            .collect()
    }

    /// `HolderSet` members in order; a tag reads the level's snapshot.
    fn holder_set(
        &self,
        dimension: &LevelId,
        registry: &str,
        set: &HolderSet,
    ) -> Result<Vec<String>, String> {
        match set {
            HolderSet::Direct(ids) => Ok(ids.clone()),
            HolderSet::Tag(tag) => Ok(self
                .level_by_id(dimension)
                .chunks
                .registry_tags()
                .map(|tags| {
                    tags.members(registry, tag)
                        .iter()
                        .map(|id| id.to_string())
                        .collect()
                })
                .unwrap_or_default()),
        }
    }

    fn pending_hurt_entity(&self, entity: SourceEntity) -> Result<PendingHurtEntity, String> {
        match entity {
            SourceEntity::Session(id) => self
                .levels
                .player(id)
                .map(|player| PendingHurtEntity::Player {
                    uuid: player.profile.uuid,
                })
                .ok_or_else(|| "the owner session left".to_string()),
            SourceEntity::Stored(entity_id) => Ok(PendingHurtEntity::Stored { entity_id }),
        }
    }

    fn post_attack_victim_exists(&self, dimension: &LevelId, victim: MeleeTarget) -> bool {
        match victim {
            MeleeTarget::Session(id) => self.levels.player(id).is_some(),
            MeleeTarget::Stored(id) => {
                let entities = &self.level_by_id(dimension).entities;
                entities.get(id).is_some()
                    || crate::entity::ender_dragon::part_by_id(entities, id).is_some()
            }
        }
    }

    fn enchantment_explosion_entity(
        &self,
        dimension: &LevelId,
        target: SourceEntity,
        captured: Option<&crate::entity::Entity>,
    ) -> Option<ExplosionSourceEntity> {
        let store = &self.level_by_id(dimension).entities;
        let player = match target {
            SourceEntity::Session(id) => self.levels.player(id),
            SourceEntity::Stored(id) => store.player(id),
        };
        if let Some(player) = player {
            return crate::entity::damage::damage_causing_explosion_source_entity(
                crate::entity::damage::StoredDamageCausingEntity::Player {
                    uuid: player.profile.uuid,
                    entity_id: player.entity_id(),
                    instabuild: player.abilities.instabuild,
                },
                store,
                player.may_build(),
                Some(player.game_type == bbb_protocol::game_types::GameType::Spectator),
                Some(player.position()),
            );
        }
        let SourceEntity::Stored(id) = target else {
            return None;
        };
        store
            .get(id)
            .or(captured)
            .map(crate::entity::damage::explosion_source_entity)
    }

    fn enchantment_explosion_source(
        &self,
        dimension: &LevelId,
        target: SourceEntity,
        captured: Option<&crate::entity::Entity>,
        custom_damage_type: bool,
    ) -> Option<ExplosionSource> {
        let direct = self.enchantment_explosion_entity(dimension, target, captured)?;
        let store = &self.level_by_id(dimension).entities;
        let body = match target {
            SourceEntity::Session(id) => self.levels.player(id).map(|player| player.body()),
            SourceEntity::Stored(id) => store.get(id).or(captured),
        }?;
        // Explosion.java:17-23.
        let indirect = if direct.is_living_non_player()
            || matches!(direct.kind, ExplosionSourceEntityKind::Player { .. })
        {
            Some(direct.clone())
        } else {
            let owner = match body.data() {
                crate::entity::EntityData::Tnt(data) => data.owner(),
                data => data.projectile_owner(),
            };
            owner.and_then(|owner| {
                let id = store
                    .player_by_uuid(owner)
                    .map(|player| player.entity_id())
                    .or_else(|| store.entity_id_for_uuid(owner))?;
                self.enchantment_explosion_entity(dimension, SourceEntity::Stored(id), None)
                    .filter(|owner| {
                        owner.is_living_non_player()
                            || matches!(owner.kind, ExplosionSourceEntityKind::Player { .. })
                    })
            })
        };
        Some(ExplosionSource {
            in_water: body.was_touching_water(),
            wind_charge: matches!(
                direct.entity_resource_id.as_str(),
                "minecraft:wind_charge" | "minecraft:breeze_wind_charge"
            ),
            causing: if custom_damage_type {
                Some(direct.clone())
            } else {
                indirect.clone()
            },
            direct,
            indirect,
        })
    }

    fn entity_position(&self, dimension: &LevelId, entity: SourceEntity) -> Option<Vec3d> {
        match entity {
            SourceEntity::Session(id) => self.levels.player(id).map(|player| player.position()),
            SourceEntity::Stored(id) => self
                .level_by_id(dimension)
                .entities
                .get(id)
                .map(|entity| entity.position())
                .or_else(|| {
                    crate::entity::ender_dragon::part_position(
                        &self.level_by_id(dimension).entities,
                        id,
                    )
                }),
        }
    }
}

#[cfg(test)]
mod tests;
