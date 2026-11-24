//! The equipment and effect reads of a session player's `hurtServer`:
//! enchantment immunity, helmet and armour wear, and the armour, resistance
//! and protection absorption (`LivingEntity.java:1849-1919`).

use crate::connection::ConnectionId;
use crate::enchantment_combat_effects::CombatValue;
use crate::entity::living::clamp_f32;
use crate::world::LevelId;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_datapack::damage::{DamageTypeFacts, PlayerHurtHooks};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::custom_stat;

use super::enchanted_damage::{EnchantedDamageSource, SourceEntity};
use super::player_melee::MeleeTarget;
use super::{decidecimal_damage, DAMAGE_STATISTIC_LIMIT};
use crate::world::{ServerWorld, WorldEvent};

/// `Player.hurtArmor` (`Player.java:738-740`).
const ARMOR_SLOTS: [EquipmentSlot; 4] = [
    EquipmentSlot::Feet,
    EquipmentSlot::Legs,
    EquipmentSlot::Chest,
    EquipmentSlot::Head,
];

/// The source a session's hurt names, as the enchantment context sees it.
#[derive(Clone, Copy)]
pub(in crate::world) struct PlayerHurtSource<'a> {
    pub(in crate::world) damage_type: &'a DamageTypeFacts,
    pub(in crate::world) causing: Option<SourceEntity>,
    pub(in crate::world) direct: Option<SourceEntity>,
}

impl PlayerHurtSource<'_> {
    pub(super) fn enchanted(&self) -> EnchantedDamageSource<'_> {
        EnchantedDamageSource {
            damage_type: &self.damage_type.resource_id,
            // `DamageSource.isDirect` (`DamageSource.java:30-32`).
            is_direct: self.causing == self.direct,
            causing: self.causing,
            direct: self.direct,
        }
    }
}

/// [`PlayerHurtHooks`] over a connected session. `item_blocking` is the
/// caller's `applyItemBlocking`.
pub(super) struct SessionHurtHooks<'w, 's, B> {
    pub(super) world: &'w mut ServerWorld,
    pub(super) events: &'w mut Vec<WorldEvent>,
    pub(super) connection_id: ConnectionId,
    pub(super) source: PlayerHurtSource<'s>,
    pub(super) caller_dimension: Option<LevelId>,
    pub(super) item_blocking: B,
    /// The first read that could not be answered, which refuses the hurt.
    pub(super) failure: Option<String>,
}

impl<B> SessionHurtHooks<'_, '_, B> {
    fn fail(&mut self, detail: String) {
        self.failure.get_or_insert(detail);
    }

    fn tagged(&self, tag: &str) -> bool {
        self.world
            .player_damage_type_tag(self.connection_id, self.source.damage_type, tag)
            .unwrap_or(false)
    }

    /// `doHurtEquipment` (`LivingEntity.java:1855-1867`).
    fn hurt_equipment(&mut self, damage: f32, slots: &[EquipmentSlot]) {
        if damage <= 0.0 {
            return;
        }
        // `Math.max(1.0F, NaN)` is NaN, which `(int)` makes 0.
        let quarter = damage / 4.0;
        let durability = if quarter.is_nan() {
            0
        } else {
            quarter.max(1.0) as i32
        };
        let equipment = self.world.levels[self.connection_id]
            .inventory
            .equipment_slots()
            .collect::<Vec<_>>();
        for &slot in slots {
            let Some(&(_, index)) = equipment.iter().find(|(entry, _)| *entry == slot) else {
                continue;
            };
            let stack = self.world.levels[self.connection_id]
                .inventory
                .slot(index)
                .cloned()
                .unwrap_or_else(RuntimeItemStack::empty);
            match self.world.armor_wears_from(&stack, self.source.damage_type) {
                Ok(true) => {
                    if let Err(detail) = self.world.hurt_and_break_equipment(
                        self.connection_id,
                        slot,
                        index,
                        durability,
                        self.events,
                    ) {
                        self.fail(detail);
                        return;
                    }
                }
                Ok(false) => {}
                Err(detail) => self.fail(format!("armor wear in {slot:?}: {detail}")),
            }
        }
    }
}

impl<B> PlayerHurtHooks for SessionHurtHooks<'_, '_, B>
where
    B: FnMut(&mut ServerWorld, &mut Vec<WorldEvent>, f32) -> Option<f32>,
{
    fn hurt_admitted(&mut self) {
        self.world
            .remove_player_shoulders(self.connection_id, self.events);
    }

    fn fire_resistant(&mut self) -> bool {
        self.world.levels[self.connection_id]
            .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_FIRE_RESISTANCE_ID)
    }

    fn immune_to_damage(&mut self) -> bool {
        match self.world.player_immune_to_damage_from_level(
            self.connection_id,
            self.source,
            self.caller_dimension.as_ref(),
        ) {
            Ok(immune) => immune,
            Err(detail) => {
                self.fail(format!("damage immunity: {detail}"));
                false
            }
        }
    }

    fn item_blocking(&mut self, damage: f32) -> Option<f32> {
        (self.item_blocking)(self.world, self.events, damage)
    }

    fn hurt_helmet(&mut self, damage: f32) -> bool {
        if !self.tagged("minecraft:damages_helmet") {
            return false;
        }
        let player = &self.world.levels.player_at(self.connection_id);
        let head_empty = player
            .inventory
            .equipment_slots()
            .into_iter()
            .find(|(slot, _)| *slot == EquipmentSlot::Head)
            .and_then(|(_, index)| player.inventory.slot(index))
            .is_none_or(|stack| stack.is_empty());
        if head_empty {
            return false;
        }
        // `Player.hurtHelmet` (`Player.java:743-745`).
        self.hurt_equipment(damage, &[EquipmentSlot::Head]);
        true
    }

    fn absorb(&mut self, damage: f32) -> f32 {
        let mut damage = damage;
        // `getDamageAfterArmorAbsorb` (`LivingEntity.java:1869-1876`).
        if !self.tagged("minecraft:bypasses_armor") {
            self.hurt_equipment(damage, &ARMOR_SLOTS);
            let player = &self.world.levels.player_at(self.connection_id);
            let armor = player.attribute_value(VANILLA_ATTRIBUTE_ARMOR_ID).floor() as i32 as f32;
            let toughness = player.attribute_value(VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID) as f32;
            // `CombatRules.getDamageAfterAbsorb` (`CombatRules.java:16-32`).
            let toughness = 2.0 + toughness / 4.0;
            let real_armor = clamp_f32(armor - damage / toughness, armor * 0.2, 20.0);
            let mut fraction = real_armor / 25.0;
            match self
                .world
                .weapon_armor_effectiveness(self.connection_id, self.source, fraction)
            {
                Ok(Some(modified)) => fraction = clamp_f32(modified, 0.0, 1.0),
                Ok(None) => {}
                Err(detail) => self.fail(format!("armor effectiveness: {detail}")),
            }
            damage *= 1.0 - fraction;
        }
        // `getDamageAfterMagicAbsorb` (`:1878-1919`).
        if self.tagged("minecraft:bypasses_effects") {
            return damage;
        }
        let resistance = self.world.levels[self.connection_id]
            .active_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_RESISTANCE_ID)
            .map(|effect| effect.amplifier());
        if let Some(amplifier) = resistance {
            if !self.tagged("minecraft:bypasses_resistance") {
                let absorb = 25_i32.wrapping_sub(amplifier.wrapping_add(1).wrapping_mul(5));
                let old_damage = damage;
                damage = (damage * absorb as f32 / 25.0).max(0.0);
                let resisted = old_damage - damage;
                if resisted > 0.0 && resisted < DAMAGE_STATISTIC_LIMIT {
                    self.world.levels[self.connection_id].award_stat(
                        &mut self.world.scoreboard,
                        custom_stat("minecraft:damage_resisted"),
                        decidecimal_damage(resisted),
                    );
                }
            }
        }
        if damage <= 0.0 {
            return 0.0;
        }
        if self.tagged("minecraft:bypasses_enchantments") {
            return damage;
        }
        let protection = match self
            .world
            .player_damage_protection(self.connection_id, self.source)
        {
            Ok(protection) => protection,
            Err(detail) => {
                self.fail(format!("damage protection: {detail}"));
                0.0
            }
        };
        if protection > 0.0 {
            // `CombatRules.getDamageAfterMagicAbsorb` (`CombatRules.java:34-37`).
            damage *= 1.0 - clamp_f32(protection, 0.0, 20.0) / 25.0;
        }
        damage
    }
}

impl ServerWorld {
    /// Whether `doHurtEquipment` wears `stack` for `damage_type`: an
    /// `EQUIPPABLE` with `damageOnHurt` that `canBeHurtBy` the source
    /// (`ItemStack.java:1112-1115`). `isDamageableItem` is `hurtAndBreak`'s.
    fn armor_wears_from(
        &self,
        stack: &RuntimeItemStack,
        damage_type: &DamageTypeFacts,
    ) -> Result<bool, String> {
        if stack.is_empty() {
            return Ok(false);
        }
        if !crate::entity::equippable_damage_on_hurt(stack)? {
            return Ok(false);
        }
        Ok(!crate::entity::item_resists_damage(
            stack,
            &damage_type.resource_id,
            &self.item_stack_resolver(),
        ))
    }

    /// Each worn enchantment whose slots include the one it is worn in, in
    /// `EquipmentSlot.VALUES` order (`EnchantmentHelper.java:134-156`).
    pub(in crate::world) fn player_equipment_enchantments(
        &self,
        connection_id: ConnectionId,
    ) -> Result<Vec<(String, i32)>, String> {
        let player = self
            .levels
            .player(connection_id)
            .ok_or("original equipment body unavailable")?;
        let tables = &*self.datapack_tables;
        let mut enchantments = Vec::new();
        for (slot, index) in player.inventory.equipment_slots() {
            let Some(stack) = player.inventory.slot(index) else {
                continue;
            };
            let patch = &stack.summary().component_patch;
            if stack.is_empty()
                || patch.enchantments.is_empty()
                || patch.removed_type_ids.contains(
                    &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                )
            {
                continue;
            }
            for entry in &patch.enchantments {
                if !(0..=255).contains(&entry.level) {
                    continue;
                }
                let Ok(holder_id) = usize::try_from(entry.holder_id) else {
                    continue;
                };
                if !tables.holder_matches_slot(holder_id, slot) {
                    continue;
                }
                let Some(id) = tables.enchantment_registry.get(holder_id) else {
                    continue;
                };
                enchantments.push((id.clone(), entry.level));
            }
        }
        Ok(enchantments)
    }

    /// `EnchantmentHelper.isImmuneToDamage` (`EnchantmentHelper.java:158-164`)
    /// for a session.
    pub(in crate::world) fn player_immune_to_damage(
        &mut self,
        connection_id: ConnectionId,
        source: PlayerHurtSource<'_>,
    ) -> Result<bool, String> {
        self.player_immune_to_damage_from_level(connection_id, source, None)
    }

    pub(in crate::world) fn player_immune_to_damage_from_level(
        &mut self,
        connection_id: ConnectionId,
        source: PlayerHurtSource<'_>,
        caller: Option<&LevelId>,
    ) -> Result<bool, String> {
        let enchantments = self.player_equipment_enchantments(connection_id)?;
        let dimension = self.levels[connection_id].dimension;
        self.enchanted_damage_immunity(
            caller.unwrap_or(&dimension),
            &enchantments,
            MeleeTarget::Session(connection_id),
            &source.enchanted(),
        )
    }

    /// `EnchantmentHelper.getDamageProtection` (`EnchantmentHelper.java:166-172`).
    fn player_damage_protection(
        &mut self,
        connection_id: ConnectionId,
        source: PlayerHurtSource<'_>,
    ) -> Result<f32, String> {
        let enchantments = self.player_equipment_enchantments(connection_id)?;
        let dimension = self.levels[connection_id].dimension;
        self.enchanted_damage_value(
            &dimension,
            &enchantments,
            MeleeTarget::Session(connection_id),
            &source.enchanted(),
            CombatValue::DamageProtection,
            0.0,
        )
    }

    /// `EnchantmentHelper.modifyArmorEffectiveness` (`EnchantmentHelper.java:192-200`)
    /// over `source.getWeaponItem()` (`DamageSource.java:67-69`); `None` without one.
    fn weapon_armor_effectiveness(
        &mut self,
        victim: ConnectionId,
        source: PlayerHurtSource<'_>,
        fraction: f32,
    ) -> Result<Option<f32>, String> {
        let dimension = self.levels[victim].dimension;
        let enchantments: Vec<(String, i32)> = match source.direct {
            None => return Ok(None),
            Some(SourceEntity::Session(attacker)) => {
                let attacker = self
                    .levels
                    .player(attacker)
                    .ok_or("original direct damage body unavailable")?;
                let weapon = super::player_weapon::player_weapon_stack(attacker);
                if weapon.is_empty() {
                    return Ok(None);
                }
                let patch = &weapon.summary().component_patch;
                if patch.enchantments.is_empty()
                    || patch
                        .removed_type_ids
                        .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
                {
                    return Ok(None);
                }
                let registry = &self.datapack_tables.enchantment_registry;
                let mut resolved = Vec::with_capacity(patch.enchantments.len());
                for entry in &patch.enchantments {
                    let Some(id) = usize::try_from(entry.holder_id)
                        .ok()
                        .and_then(|index| registry.get(index))
                    else {
                        return Err(
                            "the weapon's enchantments do not resolve in this generation".into(),
                        );
                    };
                    if (0..=255).contains(&entry.level) {
                        resolved.push((id.clone(), entry.level));
                    }
                }
                resolved
            }
            Some(SourceEntity::Stored(entity_id)) => {
                let Some(weapon) = self
                    .level_by_id(&dimension)
                    .entities
                    .get(entity_id)
                    .and_then(crate::entity::enchanted_damage::weapon_item)
                else {
                    return Ok(None);
                };
                crate::entity::enchanted_damage::stack_enchantments(
                    weapon,
                    self.level_by_id(&dimension).entities.enchantment_registry(),
                )
            }
        };
        if enchantments.is_empty() {
            return Ok(None);
        }
        self.enchanted_damage_value(
            &dimension,
            &enchantments,
            MeleeTarget::Session(victim),
            &source.enchanted(),
            CombatValue::ArmorEffectiveness,
            fraction,
        )
        .map(Some)
    }
}
