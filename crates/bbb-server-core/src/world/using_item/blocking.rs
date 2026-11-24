//! `LivingEntity.getItemBlockingWith` and `applyItemBlocking`
//! (`LivingEntity.java:3568-3582`, `:1283-1320`) for a session, with the
//! `BlocksAttacks` hooks they reach.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::item_components::sound_holder;
use crate::entity::living::clamp_f32;
use crate::entity::EntityData;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID;
use bbb_protocol::packets::{
    BlocksAttacksSummary, HolderSetSummary, InteractionHand, SoundSource, Vec3d,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_datapack::damage::DamageTypeFacts;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::stats::StatKey;

use super::components;
use crate::world::combat::{BlockedByItem, BlockingWeaponSource, HurtDirectEntity, LivingAttacker};
use crate::world::pov_clip::view_vector;
use crate::world::{send, ServerWorld, WorldEvent};

/// A session's `getItemBlockingWith` against one damage source, resolved up
/// to the amount it will be asked to block.
#[derive(Debug)]
pub(in crate::world) struct ItemBlocking {
    hand: InteractionHand,
    item_id: Option<i32>,
    facts: BlocksAttacksSummary,
    /// Per damage reduction, whether `DamageReduction.resolve`
    /// (`BlocksAttacks.java:159-167`) passes its angle and type tests.
    applies: Vec<bool>,
}

impl ItemBlocking {
    /// `BlocksAttacks.resolveBlockedDamage` (`BlocksAttacks.java:127-135`).
    pub(in crate::world) fn resolve(&self, damage: f32) -> f32 {
        let blocked = self
            .facts
            .damage_reductions
            .iter()
            .zip(&self.applies)
            .filter(|(_, applies)| **applies)
            .fold(0.0_f32, |blocked, (reduction, _)| {
                blocked
                    + clamp_f32(
                        f32::from_bits(reduction.base_bits)
                            + f32::from_bits(reduction.factor_bits) * damage,
                        0.0,
                        damage,
                    )
            });
        clamp_f32(blocked, 0.0, damage)
    }

    pub(in crate::world) fn facts(&self) -> &BlocksAttacksSummary {
        &self.facts
    }
}

impl ServerWorld {
    /// `getItemBlockingWith` (`LivingEntity.java:3568-3582`): the item in use
    /// with `BLOCKS_ATTACKS` once its block delay has run.
    fn player_item_blocking_with(
        &self,
        connection_id: ConnectionId,
    ) -> Result<Option<(InteractionHand, &BlocksAttacksSummary)>, String> {
        let Some(player) = self.levels.player(connection_id) else {
            return Ok(None);
        };
        let state = &player.using_item;
        if !state.is_using_item() {
            return Ok(None);
        }
        let summary = state.use_item.summary();
        let Some(facts) = components::blocks_attacks(summary)? else {
            return Ok(None);
        };
        let elapsed = self.use_duration(summary)? - state.use_item_remaining;
        let block_delay_ticks =
            (f32::from_bits(facts.block_delay_seconds_bits) * 20.0).round() as i32;
        Ok((elapsed >= block_delay_ticks).then(|| (state.used_item_hand(), facts)))
    }

    /// `applyItemBlocking` (`LivingEntity.java:1283-1320`) up to
    /// `resolveBlockedDamage`: `None` where it answers zero without resolving
    /// one, which is also the answer when the level cannot say whether a
    /// damage type set holds the source.
    pub(in crate::world) fn player_item_blocking(
        &self,
        connection_id: ConnectionId,
        damage_type: &DamageTypeFacts,
        source_position: Option<Vec3d>,
        direct: &HurtDirectEntity,
    ) -> Result<Option<ItemBlocking>, String> {
        let Some((hand, facts)) = self.player_item_blocking_with(connection_id)? else {
            return Ok(None);
        };
        if let Some(bypassed_by) = &facts.bypassed_by {
            let Some(bypassed) = self.damage_type_in(connection_id, bypassed_by, damage_type)
            else {
                return Ok(None);
            };
            if bypassed {
                return Ok(None);
            }
        }
        if direct.arrow_pierce_level.is_some_and(|level| level > 0) {
            return Ok(None);
        }
        let player = self.levels.player_at(connection_id);
        let angle = match source_position {
            Some(source) => {
                // `calculateViewVector(0.0F, getYHeadRot())`; a session's head
                // turns with its body (`Player.java:455`).
                let view = view_vector(0.0, player.rotation().y_rot);
                let to = Vec3d {
                    x: source.x - player.position().x,
                    y: 0.0,
                    z: source.z - player.position().z,
                }
                .normalize();
                (to.x * view.x + to.y * view.y + to.z * view.z).acos()
            }
            None => f64::from(std::f32::consts::PI),
        };
        let mut applies = Vec::with_capacity(facts.damage_reductions.len());
        for reduction in &facts.damage_reductions {
            let limit = ((std::f64::consts::PI / 180.0) as f32)
                * f32::from_bits(reduction.horizontal_blocking_angle_bits);
            let applied = if angle > f64::from(limit) {
                false
            } else {
                match &reduction.damage_types {
                    Some(types) => match self.damage_type_in(connection_id, types, damage_type) {
                        Some(applied) => applied,
                        None => return Ok(None),
                    },
                    None => true,
                }
            };
            applies.push(applied);
        }
        Ok(Some(ItemBlocking {
            hand,
            item_id: player.using_item.use_item.item_id(),
            facts: facts.clone(),
            applies,
        }))
    }

    /// The rest of `applyItemBlocking` once it resolved `damage_blocked`:
    /// `hurtBlockingItem` (`BlocksAttacks.java:105-116`) and, for a blocked
    /// melee hit, `blockUsingItem` (`LivingEntity.java:1310-1313`). Answers
    /// whether the item in use came out of it unbroken, which is what
    /// `hurtServer`'s later `itemInUse.get(BLOCKS_ATTACKS)` (`:1216`) reads.
    pub(in crate::world) fn apply_item_blocking(
        &mut self,
        connection_id: ConnectionId,
        blocking: &ItemBlocking,
        damage_blocked: f32,
        damage_type: &DamageTypeFacts,
        direct: &HurtDirectEntity,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        if let Some(item_id) = blocking.item_id {
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        let item_damage = &blocking.facts.item_damage;
        let item_damage = if damage_blocked < f32::from_bits(item_damage.threshold_bits) {
            0
        } else {
            (f32::from_bits(item_damage.base_bits)
                + f32::from_bits(item_damage.factor_bits) * damage_blocked)
                .floor() as i32
        };
        let mut unbroken = true;
        if item_damage > 0 {
            unbroken =
                self.hurt_and_break_in_hand(connection_id, blocking.hand, item_damage, events);
        }
        let is_projectile =
            self.player_damage_type_tag(connection_id, damage_type, "minecraft:is_projectile");
        if let (true, Some(false), Some(attacker)) =
            (damage_blocked > 0.0, is_projectile, &direct.living)
        {
            self.block_using_item(connection_id, attacker, events);
        }
        unbroken
    }

    /// `Player.blockUsingItem` (`Player.java:712-720`) over
    /// `LivingEntity.blockUsingItem` (`LivingEntity.java:1353-1355`).
    fn block_using_item(
        &mut self,
        connection_id: ConnectionId,
        attacker: &LivingAttacker,
        events: &mut Vec<WorldEvent>,
    ) {
        match attacker.blocked_by_item {
            BlockedByItem::Knockback => {
                let position = self.levels[connection_id].position();
                self.player_knockback(
                    connection_id,
                    0.5,
                    position.x - attacker.position.x,
                    position.z - attacker.position.z,
                );
            }
            BlockedByItem::Ravager => self.ravager_blocked_by_item(connection_id, attacker),
            BlockedByItem::HoglinThrow => self.hoglin_blocked_by_item(connection_id, attacker),
        }
        let (hand, facts) = match self.player_item_blocking_with(connection_id) {
            Ok(Some(blocking)) => blocking,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!("item use: {error}");
                return;
            }
        };
        let dimension = self.levels[connection_id].dimension;
        let seconds_to_disable = match self.attacker_disable_blocking_seconds(&dimension, attacker)
        {
            Ok(seconds) => seconds,
            Err(error) => {
                tracing::warn!("item use: {error}");
                return;
            }
        };
        if seconds_to_disable > 0.0 {
            let facts = facts.clone();
            self.disable_blocking(connection_id, hand, &facts, seconds_to_disable, events);
        }
    }

    fn hoglin_blocked_by_item(&mut self, connection_id: ConnectionId, attacker: &LivingAttacker) {
        let BlockingWeaponSource::Stored(entity_id) = attacker.weapon_source else {
            return;
        };
        let dimension = self.levels[connection_id].dimension;
        let Some(body) = self.level_by_id(&dimension).entities.get(entity_id) else {
            return;
        };
        if !matches!(body.data(), EntityData::GeneralLiving(data) if !data.is_baby()) {
            return;
        }
        let Some(living) = body.living() else {
            return;
        };
        let body_position = body.position();
        let knockback = living.attribute_value(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID);
        self.player_hoglin_throw(connection_id, body_position, knockback);
    }

    /// `Ravager.blockedByItem` (`Ravager.java:206-219`) on a session, whose
    /// `defender.hurtMarked` publishes the push.
    fn ravager_blocked_by_item(&mut self, connection_id: ConnectionId, attacker: &LivingAttacker) {
        let BlockingWeaponSource::Stored(ravager_id) = attacker.weapon_source else {
            return;
        };
        let dimension = self.levels[connection_id].dimension;
        let defender = self.levels[connection_id].position();
        let random = self.random.clone();
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        let Some(mut ravager) = entities.detach(ravager_id) else {
            return;
        };
        let push =
            crate::entity::ravager::blocked_by_item(&mut ravager, entities, &random, defender);
        entities.attach(ravager);
        let Some(push) = push else {
            return;
        };
        let player = &mut self.levels[connection_id];
        player.set_delta_movement(player.delta_movement().plus(push));
        player.hurt_marked = true;
    }

    /// `BlocksAttacks.disable` (`BlocksAttacks.java:83-103`).
    fn disable_blocking(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        facts: &BlocksAttacksSummary,
        base_seconds: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        // `disableBlockingForTicks` (`BlocksAttacks.java:118-121`).
        let seconds = base_seconds * f32::from_bits(facts.disable_cooldown_scale_bits);
        let cooldown_ticks = if seconds > 0.0 {
            (seconds * 20.0).round() as i32
        } else {
            0
        };
        if cooldown_ticks <= 0 {
            return;
        }
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the blocking session is connected");
        let group = crate::item_cooldowns::cooldown_group(
            super::runtime_item_in_hand(player, hand).summary(),
        )
        .map(str::to_owned);
        if let Ok(group) = group {
            let packet = player
                .cold
                .cooldowns
                .add_group_cooldown(group, cooldown_ticks);
            events.push(send(connection_id, ClientboundPlayPacket::Cooldown(packet)));
        }
        self.stop_using_item(connection_id);
        if let Some(sound) = &facts.disable_sound {
            let sound = sound_holder(sound).expect("validated blocking disable sound");
            self.play_blocking_sound(connection_id, sound, 0.8, events);
        }
    }

    /// `BlocksAttacks.onBlocked` (`BlocksAttacks.java:72-81`).
    pub(in crate::world) fn on_blocked(
        &mut self,
        connection_id: ConnectionId,
        facts: &BlocksAttacksSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(sound) = &facts.block_sound {
            let sound = sound_holder(sound).expect("validated blocking sound");
            self.play_blocking_sound(connection_id, sound, 1.0, events);
        }
    }

    /// `level.playSound(null, user.getX(), user.getY(), user.getZ(), sound,
    /// user.getSoundSource(), volume, 0.8F + level.getRandom().nextFloat() * 0.4F)`.
    fn play_blocking_sound(
        &mut self,
        connection_id: ConnectionId,
        sound: bbb_protocol::packets::SoundEventHolder,
        volume: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        let position = player.position();
        let pitch = 0.8 + self.random.next_float() * 0.4;
        self.play_player_sound(
            connection_id,
            None,
            sound,
            SoundSource::Players,
            position,
            volume,
            pitch,
            events,
        );
    }

    /// `HolderSet<DamageType>.contains(source.typeHolder())`; `None` where the
    /// level carries no tag snapshot to answer a tag with.
    fn damage_type_in(
        &self,
        connection_id: ConnectionId,
        set: &HolderSetSummary,
        damage_type: &DamageTypeFacts,
    ) -> Option<bool> {
        match &set.tag {
            Some(tag) => {
                self.player_damage_type_tag(connection_id, damage_type, tag.trim_start_matches('#'))
            }
            None => Some(set.registry_ids.contains(&damage_type.registry_id)),
        }
    }
}
