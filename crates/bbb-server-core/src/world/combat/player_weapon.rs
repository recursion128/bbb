//! The weapon half of `Player.attack`: `ItemStack.getDamageSource`
//! (`ItemStack.java:1126-1131`), `MaceItem` (`MaceItem.java:51-156`) and
//! `itemAttackInteraction`'s `hurtEnemy` / `postHurtEnemy`
//! (`ItemStack.java:535-555`).

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::enchantment_combat_effects::CombatValue;
use crate::entity::{EntityData, ScoreboardTeams};
use crate::inventory::{
    main_hand_attack_damage_type, weapon_item_damage_per_attack, MainHandAttackDamageType,
};
use crate::level_event::PendingLevelEvent;
use crate::world::LevelId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::VANILLA_ITEM_MACE_ID;
use bbb_protocol::packets::{BlockPos, SetEntityMotion, Vec3d};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::stats::StatKey;
use bbb_server_types::Result;
use uuid::Uuid;

use super::enchanted_damage::SourceEntity;
use super::player_damage_reduction::PlayerHurtSource;
use super::player_melee::{MeleeTarget, PLAYER_ATTACK_DAMAGE_TYPE};
use crate::world::{send, ServerWorld, WorldEvent};

use crate::entity::mace::{
    can_smash_attack, smash_damage, MaceSessionFacts, LEVEL_EVENT_SMASH_ATTACK,
    MACE_SMASH_AIR_SOUND, MACE_SMASH_DAMAGE_TYPE, MACE_SMASH_GROUND_HEAVY_SOUND,
    MACE_SMASH_GROUND_SOUND,
};

/// The living body a swing hurt, as `hurtEnemy` sees it: a body the hurt
/// removed from the store is still the entity vanilla hands on.
pub(super) struct MeleeTargetBody {
    position: Vec3d,
    on_ground: bool,
    on_pos: BlockPos,
    bounding_box: Aabb,
    uuid: Uuid,
}

/// `Player.getWeaponItem` (`Player.java:521-522`).
pub(super) fn player_weapon_stack(
    player: &crate::world::PlayerState,
) -> &bbb_server_types::item_stack::RuntimeItemStack {
    if player.using_item.is_auto_spin_attack() {
        if let Some((_, _, stack)) = &player.using_item.auto_spin_attack {
            return stack;
        }
    }
    player.inventory.selected_stack()
}

impl ServerWorld {
    /// Resolves the live inventory slot backing `Player.getWeaponItem`
    /// (`Player.java:521-522`, `:954`). During a riptide spin, prefers the hand
    /// that initiated the spin, then checks the other hand and the rest of the
    /// inventory so wear lands on the originating stack; returns `None` when
    /// the spin stack is detached from inventory.
    pub(super) fn player_attack_weapon_reference(
        &self,
        connection_id: ConnectionId,
    ) -> std::result::Result<Option<super::EquipmentReference>, String> {
        let player = self
            .levels
            .player(connection_id)
            .ok_or("the session left")?;
        if player.using_item.is_auto_spin_attack() {
            if let Some((_, _, spin_stack)) = &player.using_item.auto_spin_attack {
                if spin_stack.is_empty() {
                    return Ok(None);
                }
                let (primary_slot, secondary_slot) = match player.using_item.used_item_hand() {
                    bbb_protocol::packets::InteractionHand::MainHand => (
                        bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
                        bbb_protocol::equipment_slots::EquipmentSlot::OffHand,
                    ),
                    bbb_protocol::packets::InteractionHand::OffHand => (
                        bbb_protocol::equipment_slots::EquipmentSlot::OffHand,
                        bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
                    ),
                };
                for slot in [primary_slot, secondary_slot] {
                    if let Some(index) = player
                        .inventory
                        .equipment_slots()
                        .find(|(candidate, _)| *candidate == slot)
                        .map(|(_, index)| index)
                    {
                        if player.inventory.slot(index) == Some(spin_stack) {
                            return Ok(Some(super::EquipmentReference::Session {
                                connection: connection_id,
                                index,
                            }));
                        }
                    }
                }
                for index in 0..crate::inventory::INVENTORY_SIZE {
                    if player.inventory.slot(index) == Some(spin_stack) {
                        return Ok(Some(super::EquipmentReference::Session {
                            connection: connection_id,
                            index,
                        }));
                    }
                }
                return Ok(None);
            }
        }
        let dimension = player.dimension;
        self.capture_equipment_reference(
            &dimension,
            SourceEntity::Session(connection_id),
            bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
        )
    }

    /// `Player.createAttackSource` (`Player.java:1011-1013`): the stack's
    /// `damage_type`, then `MaceItem.getItemDamageSource` (`:153-156`), then
    /// `player_attack`.
    pub(super) fn player_attack_damage_type(&mut self, connection_id: ConnectionId) -> String {
        let stack = player_weapon_stack(self.levels.player_at(connection_id))
            .summary()
            .clone();
        self.player_attack_damage_type_for_summary(connection_id, &stack)
    }

    pub(super) fn player_attack_damage_type_for_slot(
        &mut self,
        connection_id: ConnectionId,
        slot: bbb_protocol::equipment_slots::EquipmentSlot,
    ) -> String {
        let player = self.levels.player_at(connection_id);
        let stack = if slot == bbb_protocol::equipment_slots::EquipmentSlot::MainHand {
            player.inventory.selected_stack().summary().clone()
        } else {
            player
                .inventory
                .equipment_slots()
                .find(|(candidate, _)| *candidate == slot)
                .and_then(|(_, index)| player.inventory.slot(index))
                .map(|stack| stack.summary().clone())
                .unwrap_or_else(bbb_protocol::packets::ItemStackSummary::empty)
        };
        self.player_attack_damage_type_for_summary(connection_id, &stack)
    }

    fn player_attack_damage_type_for_summary(
        &self,
        connection_id: ConnectionId,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) -> String {
        let is_mace = stack.item_id == Some(VANILLA_ITEM_MACE_ID);
        let detail = match main_hand_attack_damage_type(stack) {
            Ok(MainHandAttackDamageType::ResourceId(resource_id)) => {
                return resource_id.to_string();
            }
            Ok(MainHandAttackDamageType::RegistryId(registry_id)) => {
                if let Some(entry) = self
                    .cold
                    .damage_types
                    .iter()
                    .find(|entry| entry.registry_id == registry_id)
                {
                    return entry.resource_id.clone();
                }
                format!("damage type holder {registry_id} is not registered")
            }
            Ok(MainHandAttackDamageType::PlayerAttack) => {
                return if is_mace && self.player_can_smash_attack(connection_id) {
                    MACE_SMASH_DAMAGE_TYPE
                } else {
                    PLAYER_ATTACK_DAMAGE_TYPE
                }
                .to_string();
            }
            Err(error) => error.to_string(),
        };
        tracing::warn!("the attacking stack's damage type: {detail}");
        PLAYER_ATTACK_DAMAGE_TYPE.to_string()
    }

    fn player_holds_mace(&self, connection_id: ConnectionId) -> bool {
        player_weapon_stack(&self.levels[connection_id]).item_id() == Some(VANILLA_ITEM_MACE_ID)
    }

    /// `MaceItem.canSmashAttack` (`MaceItem.java:149-151`).
    fn player_can_smash_attack(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        can_smash_attack(
            player.block_contact.fall_distance,
            player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING),
        )
    }

    /// `Item.getAttackDamageBonus` (`Player.java:971`), which only
    /// `MaceItem.getAttackDamageBonus` (`MaceItem.java:89-113`) overrides.
    pub(super) fn player_attack_damage_bonus(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        target: MeleeTarget,
        damage_type: &str,
    ) -> f32 {
        if !self.player_holds_mace(connection_id) || !self.player_can_smash_attack(connection_id) {
            return 0.0;
        }
        let fall_distance = self.levels[connection_id].block_contact.fall_distance;
        let damage = smash_damage(fall_distance);
        let per_block = self.player_weapon_combat_value(
            connection_id,
            dimension,
            target,
            damage_type,
            CombatValue::SmashDamagePerFallenBlock,
            0.0,
        );
        (damage + f64::from(per_block) * fall_distance) as f32
    }

    pub(super) fn melee_target_body(
        &self,
        dimension: &LevelId,
        target: MeleeTarget,
    ) -> Option<MeleeTargetBody> {
        match target {
            MeleeTarget::Stored(entity_id) => {
                let entity = self.level_by_id(dimension).entities.get(entity_id)?;
                Some(MeleeTargetBody {
                    position: entity.position(),
                    on_ground: entity.on_ground(),
                    on_pos: entity.get_on_pos(),
                    bounding_box: entity.bounding_box(),
                    uuid: entity.uuid(),
                })
            }
            MeleeTarget::Session(target_id) => {
                let target = self.levels.player_at(target_id);
                Some(MeleeTargetBody {
                    position: target.position(),
                    on_ground: target.on_ground(),
                    on_pos: crate::entity::body_on_pos(
                        target.position(),
                        target.block_contact.supporting,
                        1.0e-5,
                        |_| unreachable!("a 1e-5 offset never reads the supporting state"),
                    ),
                    bounding_box: target.bounding_box().ok()?,
                    uuid: target.profile.uuid,
                })
            }
        }
    }

    /// `Player.itemAttackInteraction` (`Player.java:1087-1117`) on a living
    /// target: `hurtEnemy`, the post-attack enchantment effects, then the
    /// weapon's wear.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn player_item_attack_interaction(
        &mut self,
        connection_id: ConnectionId,
        (target, body): (MeleeTarget, &MeleeTargetBody),
        damage_type: &str,
        apply_to_target: bool,
        weapon_slot: bbb_protocol::equipment_slots::EquipmentSlot,
        weapon_reference: Option<super::EquipmentReference>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let spinning = player.using_item.is_auto_spin_attack();
        let captured = match weapon_reference.clone() {
            Some(reference) => self
                .referenced_equipment_stack(reference)
                .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?,
            None if spinning => match &player.using_item.auto_spin_attack {
                Some((_, _, stack)) => stack.clone(),
                None => return Ok(()),
            },
            None => return Ok(()),
        };
        let captured_summary = captured.summary();
        let item_id = captured_summary.item_id;
        let mace = item_id == Some(VANILLA_ITEM_MACE_ID);
        if mace {
            self.mace_hurt_enemy(connection_id, target, body, events)?;
        }
        let item_hurt_enemy = weapon_item_damage_per_attack(captured_summary)?.is_some();
        if item_hurt_enemy {
            if let Some(item_id) = item_id {
                self.levels[connection_id].award_stat_once(
                    &mut self.scoreboard,
                    StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
                );
            }
        }
        let damage_types = std::sync::Arc::clone(&self.cold.damage_types);
        if let Some(facts) = damage_types
            .iter()
            .find(|facts| apply_to_target && facts.resource_id == damage_type)
        {
            let attacker = Some(SourceEntity::Session(connection_id));
            let source = PlayerHurtSource {
                damage_type: facts,
                causing: attacker,
                direct: attacker,
            };
            self.do_post_attack_effects_with_item_checked(
                &dimension,
                target,
                source,
                weapon_slot,
                weapon_reference.clone(),
                events,
            )
            .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?;
        }
        let original = match weapon_reference.clone() {
            Some(reference) => self
                .referenced_equipment_stack(reference)
                .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?,
            None if spinning => match &self.levels[connection_id].using_item.auto_spin_attack {
                Some((_, _, stack)) => stack.clone(),
                None => return Ok(()),
            },
            None => return Ok(()),
        };
        if original.is_empty() || !item_hurt_enemy {
            return Ok(());
        }
        let stack = original.summary();
        // `MaceItem.postHurtEnemy` (`MaceItem.java:82-86`).
        if stack.item_id == Some(VANILLA_ITEM_MACE_ID)
            && self.player_can_smash_attack(connection_id)
        {
            self.reset_player_fall_distance(connection_id, events);
        }
        let weapon = match weapon_item_damage_per_attack(stack) {
            Ok(weapon) => weapon,
            Err(error) => {
                tracing::warn!("the attacking stack's weapon component: {error}");
                return Err(bbb_server_types::ServerWorldError::InvalidConfiguration(
                    error.to_string(),
                ));
            }
        };
        let Some(item_damage_per_attack) = weapon else {
            return Ok(());
        };
        if let Some(reference) = weapon_reference {
            self.hurt_and_break_session_reference_checked(
                connection_id,
                weapon_slot,
                reference.clone(),
                item_damage_per_attack,
                events,
            )
            .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?;
            let updated = self
                .referenced_equipment_stack(reference.clone())
                .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?;
            if updated.is_empty() {
                let super::EquipmentReference::Session { connection, index } = reference else {
                    return Err(bbb_server_types::ServerWorldError::InvalidConfiguration(
                        "the attacking item lost its session".into(),
                    ));
                };
                self.levels[connection].inventory.set_inventory_slot(
                    index,
                    bbb_server_types::item_stack::RuntimeItemStack::empty(),
                );
            }
            if spinning {
                if let Some((_, _, spin_stack)) = self.levels[connection_id]
                    .using_item
                    .auto_spin_attack
                    .as_mut()
                {
                    *spin_stack = updated;
                }
            }
        } else if spinning {
            let mut updated = original;
            if let Some(broken_item_id) = self
                .change_enchanted_stack_damage(
                    &dimension,
                    Some(SourceEntity::Session(connection_id)),
                    &mut updated,
                    item_damage_per_attack,
                    events,
                )
                .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?
            {
                if let Some((_, _, spin_stack)) = self.levels[connection_id]
                    .using_item
                    .auto_spin_attack
                    .as_mut()
                {
                    *spin_stack = updated;
                }
                if let Some(broken_item_id) = broken_item_id {
                    self.on_player_equipped_item_broken_checked(
                        connection_id,
                        weapon_slot,
                        broken_item_id,
                        events,
                    )
                    .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?;
                }
            }
        }

        Ok(())
    }

    /// `MaceItem.hurtEnemy` (`MaceItem.java:51-73`).
    fn mace_hurt_enemy(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        body: &MeleeTargetBody,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        if !self.player_can_smash_attack(connection_id) {
            return Ok(());
        }
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the attacking session remains present");
        {
            let mut delta = player.delta_movement();
            delta.y = f64::from(0.01_f32);
            player.set_delta_movement(delta);
        }
        // `calculateImpactPosition` (`:75-79`).
        let impact = match player.cold.fall_impulse.impact_position {
            Some(impact) if impact.y <= player.position().y => impact,
            _ => player.position(),
        };
        player
            .cold
            .fall_impulse
            .set_ignore_fall_damage(true, impact);
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetEntityMotion(SetEntityMotion {
                id: player.entity_id(),
                delta_movement: player.delta_movement(),
            }),
        ));
        let sound = if body.on_ground {
            player.cold.spawn_extra_particles_on_fall = true;
            if player.block_contact.fall_distance > 5.0 {
                MACE_SMASH_GROUND_HEAVY_SOUND
            } else {
                MACE_SMASH_GROUND_SOUND
            }
        } else {
            MACE_SMASH_AIR_SOUND
        };
        self.play_entity_sound(connection_id, None, sound, 1.0, 1.0, events)?;
        self.mace_knockback(connection_id, target, body, events);
        Ok(())
    }

    /// `MaceItem.knockback` (`MaceItem.java:115-147`). Stored bodies are
    /// visited in section order, then sessions.
    fn mace_knockback(
        &mut self,
        connection_id: ConnectionId,
        target: MeleeTarget,
        body: &MeleeTargetBody,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let player_uuid = player.profile.uuid;
        let heavy = player.block_contact.fall_distance > 5.0;
        self.broadcast_level_event_by_key(
            &dimension,
            None,
            PendingLevelEvent {
                event_type: LEVEL_EVENT_SMASH_ATTACK,
                pos: body.on_pos,
                data: 750,
            },
            events,
        );
        let player = self.levels.player_at(connection_id);
        let teams = &self.scoreboard.board().teams;
        let player_team = teams.team_of(&player.profile.name);
        let area = body.bounding_box.inflate(3.5, 3.5, 3.5);
        let power = |position: Vec3d, knockback_resistance: f64| {
            let direction = Vec3d {
                x: position.x - body.position.x,
                y: position.y - body.position.y,
                z: position.z - body.position.z,
            };
            let length =
                (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z)
                    .sqrt();
            let power = (3.5 - length)
                * f64::from(0.7_f32)
                * if heavy { 2.0 } else { 1.0 }
                * (1.0 - knockback_resistance);
            let (x, z) = if length < f64::from(1.0e-5_f32) {
                (0.0, 0.0)
            } else {
                (direction.x / length * power, direction.z / length * power)
            };
            (power, x, z)
        };
        let entities = &self.level_by_id(&dimension).entities;
        let stored: Vec<i32> = entities
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
                    // `isOwnedBy(entity)` names the hit body, not the attacker.
                    && !matches!(entity.tamable_owner(), Some((true, Some(owner))) if owner == body.uuid)
                    && !matches!(entity.data(), EntityData::ArmorStand(data) if data.is_marker())
                    && distance_to_sqr(body.position, entity.position()) <= 3.5 * 3.5
            })
            .collect();
        let sessions: Vec<ConnectionId> = self
            .levels
            .player_entries()
            .filter(|&(&id, other)| {
                id != connection_id
                    && MeleeTarget::Session(id) != target
                    && other.dimension == dimension
                    && !other.avatar_removed
                    && other.game_type != GameType::Spectator
                    && other.bounding_box().is_ok_and(|aabb| aabb.intersects(area))
                    && !ScoreboardTeams::allied(player_team, teams.team_of(&other.profile.name))
                    && distance_to_sqr(body.position, other.position()) <= 3.5 * 3.5
                    && !(other.game_type == GameType::Creative && other.abilities.flying)
            })
            .map(|(&id, _)| id)
            .collect();
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        for id in stored {
            let Some(entity) = entities.get_mut(id) else {
                continue;
            };
            let resistance = entity.living().map_or(0.0, |living| {
                living.attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
            });
            let (power, x, z) = power(entity.position(), resistance);
            if power > 0.0 {
                crate::entity::push(entity, x, f64::from(0.7_f32), z);
            }
        }
        for id in sessions {
            let other = self
                .levels
                .player_mut(id)
                .expect("the session was just listed");
            let (power, x, z) = power(
                other.position(),
                other.attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID),
            );
            if power > 0.0 && x.is_finite() && z.is_finite() {
                {
                    let mut delta = other.delta_movement();
                    delta.x += x;
                    other.set_delta_movement(delta);
                }
                {
                    let mut delta = other.delta_movement();
                    delta.y += f64::from(0.7_f32);
                    other.set_delta_movement(delta);
                }
                {
                    let mut delta = other.delta_movement();
                    delta.z += z;
                    other.set_delta_movement(delta);
                }
                other.needs_sync = true;
                events.push(send(
                    id,
                    ClientboundPlayPacket::SetEntityMotion(SetEntityMotion {
                        id: other.entity_id(),
                        delta_movement: other.delta_movement(),
                    }),
                ));
            }
        }
    }
}

impl crate::world::PlayerState {
    pub(crate) fn mace_facts(&self) -> MaceSessionFacts {
        MaceSessionFacts {
            on_ground: self.on_ground(),
            on_pos: crate::entity::body_on_pos(
                self.position(),
                self.block_contact.supporting,
                1.0e-5,
                |_| unreachable!("a 1e-5 offset never reads the supporting state"),
            ),
            creative_flying: self.game_type == GameType::Creative && self.abilities.flying,
            knockback_resistance: self.attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID),
        }
    }
}

fn distance_to_sqr(first: Vec3d, second: Vec3d) -> f64 {
    let (dx, dy, dz) = (first.x - second.x, first.y - second.y, first.z - second.z);
    dx * dx + dy * dy + dz * dz
}
