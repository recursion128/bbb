//! The mob effects a session carries.
//!
//! A player is not a stored entity here, so it keeps its own
//! `LivingEntity.activeEffects` (`LivingEntity.java:207-208`) and drives the
//! shared runtime over it. What differs from the stored path is the audience:
//! `ServerPlayer` overrides all three callbacks to send the session its own
//! effect (`ServerPlayer.java:1636-1663`), which is the only place
//! `ClientboundUpdateMobEffectPacket`'s `blend` is true.
//!
//! The reads that answer off the attribute map an effect modifies live here
//! too, next to what moves them.

use crate::connection::ClientboundPlayPacket;
use crate::entity::living::mob_effects::client_update::{
    remove_mob_effect_packet, update_mob_effect_packet,
};
use crate::entity::living::mob_effects::{
    tick_effects_with_registry_tags, ActiveMobEffect, ActiveMobEffects, MobEffectBody,
};
use crate::entity::{AttributeMap, AttributeSupplier, PackedAttribute};
use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::LevelId;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
    VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID,
    VANILLA_ATTRIBUTE_MAX_HEALTH_ID, VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
    VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID,
};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_GLOWING, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, InteractionHand, Vec3d};
use bbb_protocol::RichTextComponent;
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use super::player_packets::send;
use super::raids::{MAX_RAID_OMEN_LEVEL, RAID_OMEN_EFFECT};
use super::{PlayerState, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;

/// `EntityType.PLAYER`'s resource id, which every registry-tag test on a
/// session's effects resolves against.
const PLAYER_RESOURCE_ID: &str = "minecraft:player";

/// `MobEffects.LEVITATION`, the one effect whose arrival and departure move a
/// session's levitation window (`ServerPlayer.java:1639-1641`, `:1660-1661`).
const LEVITATION_RESOURCE_ID: &str = "minecraft:levitation";

/// `MobEffects.BAD_OMEN`, whose body is the only thing that records a raid
/// omen position (`BadOmenMobEffect.java:28`).
const BAD_OMEN_RESOURCE_ID: &str = "minecraft:bad_omen";

/// The raid omen `BadOmenMobEffect` hands the session (`BadOmenMobEffect.java:27`).
const RAID_OMEN_DURATION: i32 = 600;

/// `MobEffects.DARKNESS`'s resource id.
const DARKNESS_RESOURCE_ID: &str = "minecraft:darkness";
/// `Warden.applyDarknessAround`'s `MobEffectInstance(DARKNESS, 260, 0, false,
/// false)` (`Warden.java:402`).
const DARKNESS_DURATION: i32 = 260;
/// The `displayEffectLimit` `applyDarknessAround` passes (`:403`).
const DARKNESS_DISPLAY_LIMIT: i32 = 200;

/// The body the shared effect runtime reads a session through.
///
/// Health and absorption live on the `PlayerDamageState`, and the `FoodData` is
/// what makes the saturation and hunger `Player` casts succeed
/// (`SaturationMobEffect.java:13-19`, `HungerMobEffect.java:13-19`).
/// `abilities.invulnerable` is the `causeFoodExhaustion` gate
/// (`Player.java:1561-1567`). It borrows the fields rather than the session
/// because the effect map is borrowed beside it.
fn effects_and_body(player: &mut PlayerState) -> (&mut ActiveMobEffects, MobEffectBody<'_>) {
    let abilities_invulnerable = player.abilities.invulnerable;
    (
        &mut player.effects,
        MobEffectBody {
            attributes: &mut player.attributes,
            health: &mut player.damage.health,
            absorption_amount: &mut player.damage.absorption_amount,
            food: Some(&mut player.food),
            abilities_invulnerable,
        },
    )
}

pub(in crate::world) fn new_mob_effect(
    effect_id: i32,
    duration: i32,
    amplifier: i32,
) -> Option<ActiveMobEffect> {
    let effect = bbb_protocol::mob_effect_types::vanilla_mob_effect_type(effect_id)?;
    (duration > 0).then(|| {
        ActiveMobEffect::from_catalog(effect, duration, amplifier, false, true, true, None)
    })
}

impl PlayerState {
    pub(crate) fn active_effect(&self, resource_id: &str) -> Option<&ActiveMobEffect> {
        self.effects.get(resource_id)
    }

    #[inline]
    pub(crate) fn has_effect_id(&self, effect_id: i32) -> bool {
        self.effects.has_id(effect_id)
    }

    #[inline]
    pub(crate) fn active_effect_id(&self, effect_id: i32) -> Option<&ActiveMobEffect> {
        if !self.effects.has_id(effect_id) {
            return None;
        }
        let resource_id =
            bbb_protocol::mob_effect_types::vanilla_mob_effect_type(effect_id)?.resource_id;
        self.effects.get(resource_id)
    }

    // ServerPlayer.java:1597-1599; AbsorptionMobEffect.java:22-24.
    pub(in crate::world) fn restart_absorption_effect(&mut self) {
        if let Some(effect) = self
            .active_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_ABSORPTION_ID)
            .cloned()
        {
            let (effects, mut body) = effects_and_body(self);
            effects.add(&mut body, effect);
        }
    }

    /// `ServerPlayer.updateInvisibilityStatus` (`ServerPlayer.java:1903-1910`)
    /// over `LivingEntity.updateInvisibilityStatus` (`LivingEntity.java:870-878`).
    fn update_invisibility_status(&mut self) {
        let spectator = self.game_type == GameType::Spectator;
        let (invisible, _) = crate::entity::living::mob_effects::compute_living_visibility_flags(
            self.effects
                .has_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INVISIBILITY_ID),
            false,
            spectator,
            true,
            false,
        );
        self.set_shared_flag(
            VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
            invisible.unwrap_or(false),
        );
        if spectator || self.effects.is_empty() {
            self.effect_particles.remove();
        } else {
            self.effect_particles.update(&self.effects);
        }
    }

    /// `LivingEntity.updateGlowingStatus` (`LivingEntity.java:891-896`) over
    /// `LivingEntity.isCurrentlyGlowing` (`:3876-3878`), which off the client
    /// is `hasEffect(GLOWING)` or `Entity.isCurrentlyGlowing`'s own
    /// `hasGlowingTag` (`Entity.java:2668-2670`).
    ///
    /// That tag has one writer, `setGlowingTag` off the `Glowing` save field
    /// (`Entity.java:2085`, `:2663-2665`).
    fn update_glowing_status(&mut self) {
        let (_, glowing) = crate::entity::living::mob_effects::compute_living_visibility_flags(
            false,
            self.effects
                .has_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_GLOWING_ID),
            false,
            false,
            self.cold.glowing_tag,
        );
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_GLOWING, glowing);
    }
}

impl ServerWorld {
    /// `LivingEntity.updateDataBeforeSync`'s dirty-effects read
    /// (`LivingEntity.java:823-826`) down to `updateDirtyEffects`
    /// (`:862-868`) for one session: both callbacks the one `effectsDirty`
    /// gate covers, run here ahead of the shared-flags byte the level tracker
    /// publishes (`ServerEntity.java:91`).
    pub(in crate::world) fn refresh_player_dirty_effects(&mut self, connection_id: ConnectionId) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        if !player.effects.take_dirty() {
            return;
        }
        player.update_invisibility_status();
        player.update_glowing_status();
    }

    /// Every session of one level through
    /// [`Self::refresh_player_dirty_effects`]: `ServerEntity` runs
    /// `updateDataBeforeSync` in both of its entry points, the per-tick
    /// `sendChanges` (`ServerEntity.java:90-91`) and the pairing row a viewer
    /// starts from (`:269-270`).
    pub(in crate::world) fn refresh_level_player_dirty_effects(&mut self, dimension: &LevelId) {
        // The pairing paths run for a session whose level the world does not
        // run, which is also when there is nothing to publish.
        let Some(level) = self.level_if_present_by_id(dimension) else {
            return;
        };
        let order = level.player_order.clone();
        for connection_id in order {
            self.refresh_player_dirty_effects(connection_id);
        }
    }

    /// `Warden.applyDarknessAround` (`Warden.java:401-404`) over
    /// `MobEffectUtil.addEffectToPlayersAround` (`MobEffectUtil.java:51-71`).
    pub(super) fn apply_darkness_around(
        &mut self,
        dimension: &LevelId,
        position: Vec3d,
        radius: f64,
        source: Option<Uuid>,
        events: &mut Vec<WorldEvent>,
    ) {
        let effect_source = source.and_then(|uuid| {
            self.levels
                .get(dimension)?
                .entities
                .entity_id_for_uuid(uuid)
                .map(|id| (*dimension, SourceEntity::Stored(id)))
        });
        let board = self.scoreboard.board();
        let source_team = source.and_then(|uuid| board.players_team(&uuid.to_string()));
        let chosen: Vec<_> = self
            .levels
            .players()
            .filter(|player| {
                player.dimension == *dimension
                    && !player.avatar_removed
                    && matches!(player.game_type, GameType::Survival | GameType::Adventure)
                    && position.distance_to_sqr(player.position()) < radius * radius
                    && !source_team.is_some_and(|team| {
                        board
                            .players_team(&player.profile.name)
                            .is_some_and(|other| team.name == other.name)
                    })
                    && player
                        .active_effect(DARKNESS_RESOURCE_ID)
                        .is_none_or(|effect| {
                            effect.amplifier() < 0
                                || effect.duration() != -1
                                    && effect.duration() < DARKNESS_DISPLAY_LIMIT
                        })
            })
            .map(|player| player.connection_id)
            .collect();
        for connection_id in chosen {
            let effect = ActiveMobEffect::with_visibility(
                DARKNESS_RESOURCE_ID,
                DARKNESS_DURATION,
                0,
                false,
                false,
                false,
            )
            .expect("vanilla darkness effect");
            self.add_player_mob_effect_from(connection_id, effect, effect_source, events);
        }
    }

    // Axolotl.java:469-490.
    pub(super) fn apply_axolotl_support(
        &mut self,
        dimension: &LevelId,
        axolotl: Uuid,
        player: Uuid,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(level) = self.level_if_present_by_id(dimension) else {
            return;
        };
        let Some(axolotl_id) = level.entities.entity_id_for_uuid(axolotl) else {
            return;
        };
        let Some(axolotl) = level.entities.get(axolotl_id) else {
            return;
        };
        if axolotl.entity_type_id() != VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            return;
        }
        let area = axolotl.bounding_box().inflate(20.0, 20.0, 20.0);
        let Some(connection) = self.connection_of(player) else {
            return;
        };
        let Some(player) = self.levels.player(connection) else {
            return;
        };
        if player.dimension != *dimension || player.avatar_removed {
            return;
        }
        let Ok(bounds) = player.bounding_box() else {
            return;
        };
        if !area.intersects(bounds) {
            return;
        }
        let previous_duration = player
            .active_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_REGENERATION_ID)
            .map_or(0, ActiveMobEffect::duration);
        if (0..2400).contains(&previous_duration) {
            let effect = new_mob_effect(
                bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_REGENERATION_ID,
                (previous_duration + 100).min(2400),
                0,
            )
            .expect("vanilla regeneration effect");
            self.add_player_mob_effect_from(
                connection,
                effect,
                Some((*dimension, SourceEntity::Stored(axolotl_id))),
                events,
            );
        }
        self.remove_player_mob_effect(connection, "minecraft:mining_fatigue", events);
    }

    /// `LivingEntity.addEffect` (`LivingEntity.java:993-1012`) for a session,
    /// with `ServerPlayer.onEffectAdded`'s own send (`ServerPlayer.java:1636-1645`).
    pub(crate) fn add_player_mob_effect(
        &mut self,
        connection_id: ConnectionId,
        effect: ActiveMobEffect,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        self.add_player_mob_effect_from(connection_id, effect, None, events)
    }

    pub(in crate::world) fn add_player_mob_effect_from(
        &mut self,
        connection_id: ConnectionId,
        effect: ActiveMobEffect,
        source: Option<(LevelId, SourceEntity)>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let registry_tags = self.player_registry_tags(connection_id).map_or(
            crate::entity::EntityRegistryTagView::Builtin26_1,
            crate::entity::EntityRegistryTagView::Frozen,
        );
        if !crate::entity::player_mob_effect_admitted_with_registry_tag_view(
            &registry_tags,
            effect.resource_id(),
        ) {
            return Some(false);
        }
        let resource_id = effect.resource_id();
        let (entity_id, published, newly_added) = {
            let player = self.levels.player_mut(connection_id)?;
            let entity_id = player.entity_id();
            // `:998-1008`: only an effect the map did not already hold reaches
            // `onEffectAdded`, and so only that one opens the window below.
            let newly_added = player.effects.get(resource_id).is_none();
            let (effects, mut body) = effects_and_body(player);
            let Some(published) = effects.add(&mut body, effect) else {
                return Some(false);
            };
            if newly_added && resource_id == LEVITATION_RESOURCE_ID {
                // `:1639-1641`.
                player.cold.levitation_start_tick = player.tick_count;
                player.cold.levitation_start_position = Some(player.position());
            }
            (entity_id, published, newly_added)
        };
        // `:1638`. This is the one send in this server that carries `blend`.
        events.push(send(
            connection_id,
            ClientboundPlayPacket::UpdateMobEffect(update_mob_effect_packet(
                entity_id, &published, true,
            )),
        ));
        if let Some(sound) = newly_added
            .then(|| crate::entity::living::mob_effects::sound_on_added(resource_id))
            .flatten()
        {
            self.play_effect_added_sound(connection_id, sound, events);
        }
        // `:1644`, which `onEffectUpdated` reaches too (`:1651`).
        self.trigger_effects_changed(connection_id, source, events);
        Some(true)
    }

    /// `MobEffect.onEffectAdded` (`MobEffect.java:98-100`), heard by every
    /// player in range. The seed is hashed rather than drawn, so the sound
    /// leaves the world's gameplay stream alone.
    fn play_effect_added_sound(
        &self,
        connection_id: ConnectionId,
        resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(registry_id) =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
        else {
            return;
        };
        let player = &self.levels[connection_id];
        let sound = bbb_protocol::packets::SoundEvent {
            sound: bbb_protocol::packets::SoundEventHolder::Reference { registry_id },
            source: bbb_protocol::packets::SoundSource::Players,
            position: player.position(),
            volume: 1.0,
            pitch: 1.0,
            seed: i64::from(player.tick_count) ^ i64::from(player.entity_id()) << 32,
        };
        self.broadcast_level_sound_by_key(
            &self.player_level_id(connection_id),
            &sound,
            None,
            events,
        );
    }

    /// `ServerPlayer.onEffectsRemoved`'s levitation arm
    /// (`ServerPlayer.java:1660-1661`).
    fn close_levitation_window(
        &mut self,
        connection_id: ConnectionId,
        removed: &[ActiveMobEffect],
    ) {
        if !removed
            .iter()
            .any(|effect| effect.resource_id() == LEVITATION_RESOURCE_ID)
        {
            return;
        }
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.cold.levitation_start_position = None;
        }
    }

    /// `LivingEntity.removeEffect` (`LivingEntity.java:1044-1052`) over
    /// `ServerPlayer.onEffectsRemoved` (`ServerPlayer.java:1655-1663`).
    pub(crate) fn remove_player_mob_effect(
        &mut self,
        connection_id: ConnectionId,
        resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) -> Option<ActiveMobEffect> {
        let (entity_id, removed) = {
            let player = self.levels.player_mut(connection_id)?;
            let entity_id = player.entity_id();
            let removed = player.effects.remove_no_update(resource_id)?;
            let (effects, mut body) = effects_and_body(player);
            effects.on_removed(&mut body, std::slice::from_ref(&removed));
            (entity_id, removed)
        };
        self.refresh_one_player_dirty_attributes(connection_id, events);
        events.push(send(
            connection_id,
            ClientboundPlayPacket::RemoveMobEffect(remove_mob_effect_packet(entity_id, &removed)),
        ));
        self.close_levitation_window(connection_id, std::slice::from_ref(&removed));
        // `:1665`, which the whole removed list shares.
        self.trigger_effects_changed(connection_id, None, events);
        Some(removed)
    }

    /// `LivingEntity.removeAllEffects` (`LivingEntity.java:953-966`), whose
    /// removed instances each owe the session one packet (`:1657-1661`).
    pub(crate) fn remove_all_player_mob_effects(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Vec<ActiveMobEffect> {
        let (entity_id, removed) = {
            let Some(player) = self.levels.player_mut(connection_id) else {
                return Vec::new();
            };
            let entity_id = player.entity_id();
            let removed = player.effects.take_all();
            if removed.is_empty() {
                return removed;
            }
            let (effects, mut body) = effects_and_body(player);
            effects.on_removed(&mut body, &removed);
            (entity_id, removed)
        };
        self.refresh_one_player_dirty_attributes(connection_id, events);
        for effect in &removed {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::RemoveMobEffect(remove_mob_effect_packet(entity_id, effect)),
            ));
        }
        self.close_levitation_window(connection_id, &removed);
        self.trigger_effects_changed(connection_id, None, events);
        removed
    }
}

impl ServerWorld {
    /// `ServerPlayer.sendActivePlayerEffects` (`ServerPlayer.java:1128`): every
    /// active effect re-sent to a session whose client has just been handed a
    /// fresh level.
    pub(super) fn send_active_player_effects(
        &self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        for effect in player.effects.iter() {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::UpdateMobEffect(update_mob_effect_packet(
                    player.entity_id(),
                    effect,
                    false,
                )),
            ));
        }
    }

    /// `LivingEntity.tickEffects` (`LivingEntity.java:479`, `:828-846`) for every
    /// session, run where `baseTick` runs it.
    pub(super) fn tick_player_mob_effects(&mut self, events: &mut Vec<WorldEvent>) {
        let connection_ids = self
            .levels
            .player_entries()
            .filter(|(_, player)| player.body_tick_enabled && !player.effects.is_empty())
            .map(|(connection_id, _)| *connection_id)
            .collect::<Vec<_>>();
        for connection_id in connection_ids {
            self.tick_one_player_mob_effects(connection_id, events);
        }
    }

    fn tick_one_player_mob_effects(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let registry_tags = self.player_registry_tags(connection_id).unwrap_or_default();
        // The two bodies the shared runtime leaves open, in the walk's own
        // resource-id order.
        self.tick_player_bad_omen(connection_id, events);
        self.tick_player_raid_omen(connection_id, events);
        let (entity_id, result) = {
            let Some(player) = self.levels.player_mut(connection_id) else {
                return;
            };
            let entity_id = player.entity_id();
            let tick_count = player.tick_count;
            let (effects, mut body) = effects_and_body(player);
            let Some(result) = tick_effects_with_registry_tags(
                effects,
                &mut body,
                &registry_tags,
                PLAYER_RESOURCE_ID,
                tick_count,
            ) else {
                return;
            };
            (entity_id, result)
        };
        self.refresh_one_player_dirty_attributes(connection_id, events);
        // `:839-840` then `:837-838`: the refresh precedes the removal in the
        // walk, and a removed instance is never also refreshed.
        for effect in &result.refreshed {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::UpdateMobEffect(update_mob_effect_packet(
                    entity_id, effect, false,
                )),
            ));
            self.trigger_effects_changed(connection_id, None, events);
        }
        for effect in &result.removed {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::RemoveMobEffect(remove_mob_effect_packet(entity_id, effect)),
            ));
            self.close_levitation_window(connection_id, std::slice::from_ref(effect));
            self.trigger_effects_changed(connection_id, None, events);
        }
        for hurt in result.hurts {
            self.hurt_connected_player(
                connection_id,
                hurt.damage_type,
                hurt.amount,
                None,
                None,
                None,
                events,
            );
        }
    }

    /// `BadOmenMobEffect.applyEffectTick` (`BadOmenMobEffect.java:20-34`) for a
    /// session, whose `shouldApplyEffectTickThisTick` is every tick (`:15-17`).
    fn tick_player_bad_omen(&mut self, connection_id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let Some(bad_omen) = player.active_effect(BAD_OMEN_RESOURCE_ID) else {
            return;
        };
        let amplification = bad_omen.amplifier();
        // `:21-23`.
        if player.game_type == GameType::Spectator
            || self.admin.settings().difficulty == Difficulty::Peaceful
        {
            return;
        }
        let dimension = player.dimension;
        let position = block_position(player);
        // `:24` over `ServerLevel.isVillage` (`ServerLevel.java:1534-1535`).
        let Some(village) = self.is_close_to_village_id(&dimension, position_array(position), 1)
        else {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!(
                    "bad omen in {}: no village answer for the session at {} {} {}",
                    self.level_key(dimension).resource_id(),
                    position.x,
                    position.y,
                    position.z
                ),
            )));
            return;
        };
        if !village {
            return;
        }
        // `:25-26` over `ServerLevel.getRaidAt` (`ServerLevel.java:1554-1556`).
        let raid_omen_level = self
            .levels
            .get(&dimension)
            .and_then(|level| level.raids.raid_omen_level_at(position));
        if raid_omen_level.is_some_and(|level| level >= MAX_RAID_OMEN_LEVEL) {
            return;
        }
        // `:27-28`.
        let effect = ActiveMobEffect::new(RAID_OMEN_EFFECT, RAID_OMEN_DURATION, amplification)
            .expect("raid omen is a vanilla id with a positive duration");
        self.add_player_mob_effect(connection_id, effect, events);
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.cold.raid_omen_position = Some(position);
        }
        // `:29` returns false, which removes bad omen (`LivingEntity.java:836-838`).
        self.remove_player_mob_effect(connection_id, BAD_OMEN_RESOURCE_ID, events);
    }

    /// `RaidOmenMobEffect.applyEffectTick` (`RaidOmenMobEffect.java:20-31`) for
    /// a session: the last tick of a raid omen starts or extends the raid at
    /// the position bad omen recorded, and the instance goes with it rather
    /// than ticking down.
    fn tick_player_raid_omen(&mut self, connection_id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let Some(raid_omen) = player.active_effect(RAID_OMEN_EFFECT) else {
            return;
        };
        // `MobEffectInstance.tickServer` (`MobEffectInstance.java:224-229`) over
        // `RaidOmenMobEffect.shouldApplyEffectTickThisTick` (`:15-17`).
        let tick_count = if raid_omen.duration() == -1 {
            player.tick_count
        } else {
            raid_omen.duration()
        };
        // `:21-23`: a spectator and a session with no recorded position both
        // leave the instance to run out on its own.
        if tick_count != 1
            || player.game_type == GameType::Spectator
            || player.cold.raid_omen_position.is_none()
        {
            return;
        }
        let position = player
            .cold
            .raid_omen_position
            .expect("the position is recorded");
        self.create_or_extend_raid(connection_id, position, events);
        // `:25`.
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.cold.raid_omen_position = None;
        }
        // `:26` returns false, which is the removal at `LivingEntity.java:836-838`
        // rather than the one running out of duration would reach.
        self.remove_player_mob_effect(connection_id, RAID_OMEN_EFFECT, events);
    }

    /// The tag snapshot of the level a session is in, or `None` for a level
    /// carrying none -- which is the `Unavailable` the stored path refuses on.
    pub(in crate::world) fn player_registry_tags(
        &self,
        connection_id: ConnectionId,
    ) -> Option<std::sync::Arc<crate::registry_tags::RegistryTagSnapshot>> {
        let dimension = &self.levels.player(connection_id)?.dimension;
        self.level_by_id(dimension).chunks.registry_tags_shared()
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn player_active_effect_for_test(
        &self,
        connection_id: ConnectionId,
        resource_id: &str,
    ) -> Option<ActiveMobEffect> {
        self.levels
            .player(connection_id)?
            .active_effect(resource_id)
            .cloned()
    }

    pub(crate) fn player_attribute_value_for_test(
        &self,
        connection_id: ConnectionId,
        attribute_id: i32,
    ) -> f64 {
        self.levels[connection_id].attribute_value(attribute_id)
    }

    pub(crate) fn player_max_health_for_test(&self, connection_id: ConnectionId) -> f32 {
        self.levels[connection_id].max_health()
    }

    pub(crate) fn player_health_for_test(&self, connection_id: ConnectionId) -> f32 {
        self.levels[connection_id].damage.health
    }

    pub(crate) fn set_player_health_for_test(&mut self, connection_id: ConnectionId, health: f32) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test session is connected");
        let max_health = player.max_health();
        player.damage.set_health(health, max_health);
    }

    pub(crate) fn player_absorption_for_test(&self, connection_id: ConnectionId) -> f32 {
        self.levels[connection_id].damage.absorption_amount
    }

    pub(crate) fn set_player_food_for_test(
        &mut self,
        connection_id: ConnectionId,
        food: i32,
        saturation: f32,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test session is connected");
        player.food.set_food_level(food);
        player.food.set_saturation(saturation);
    }

    pub(crate) fn hurt_connected_player_for_test(
        &mut self,
        connection_id: ConnectionId,
        amount: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.hurt_connected_player(
            connection_id,
            "minecraft:magic",
            amount,
            None,
            None,
            None,
            events,
        );
    }
}

/// `LivingEntity.<init>` (`LivingEntity.java:274`) followed by the packed list
/// `readAdditionalSaveData` applies over it (`:779-781`), which is the map a
/// session starts play with.
pub(super) fn player_attribute_map(packed: &[PackedAttribute]) -> AttributeMap {
    let mut attributes = AttributeMap::new(
        AttributeSupplier::for_entity_type("minecraft:player")
            .expect("DefaultAttributes has a player row"),
    );
    attributes.apply(packed);
    attributes
}

/// [`player_attribute_map`] then `Player.java:640`: the movement speed base is
/// the abilities' walking speed whatever the packed list said.
pub(super) fn restored_player_attribute_map(
    packed: &[PackedAttribute],
    walking_speed: f32,
) -> AttributeMap {
    let mut attributes = player_attribute_map(packed);
    attributes
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .expect("the player attribute supplier declares movement speed")
        .set_base_value(f64::from(walking_speed));
    attributes
}

/// `LivingEntity.getMaxHealth` (LivingEntity.java:1954-1956) off a map the
/// session does not hold yet, which is what `:791` clamps the restored health
/// against. Not `bbb_server_datapack::damage::player_max_health`, which is
/// the registry base this exists to stop being the answer.
pub(super) fn attribute_max_health(attributes: &AttributeMap) -> f32 {
    attributes
        .value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .expect("the player attribute supplier declares max health") as f32
}

impl PlayerState {
    /// `LivingEntity.getMaxHealth` (LivingEntity.java:1954-1956): the attribute
    /// with its modifiers, not the registry base, so a `HEALTH_BOOST` raises it.
    pub(super) fn max_health(&self) -> f32 {
        attribute_max_health(&self.attributes)
    }

    /// `LivingEntity.getMaxAbsorption` (LivingEntity.java:1958-1960), zero until
    /// an `ABSORPTION` modifier raises it.
    pub(super) fn max_absorption(&self) -> f32 {
        self.attributes
            .value(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID)
            .expect("the player attribute supplier declares max absorption") as f32
    }

    /// `MobEffectUtil.getDigSpeedAmplification` when `hasDigSpeed`
    /// (`MobEffectUtil.java:25-40`).
    fn dig_speed_amplification(&self) -> Option<i32> {
        [
            bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_HASTE_ID,
            bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        ]
        .into_iter()
        .filter_map(|effect_id| self.active_effect_id(effect_id))
        .map(ActiveMobEffect::amplifier)
        .max()
    }

    pub(super) fn attribute_value(&self, attribute_id: i32) -> f64 {
        self.attributes
            .value(attribute_id)
            .expect("the player attribute supplier declares every player attribute")
    }

    /// `LivingEntity.setSprinting` (`LivingEntity.java:2271-2278`).
    pub(super) fn set_sprinting(&mut self, sprinting: bool) {
        self.set_shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
            sprinting,
        );
        self.attributes.set_sprinting_modifier(sprinting);
    }

    /// The creative half of `updatePlayerAttributes` (`ServerPlayer.java:611-627`).
    pub(super) fn update_creative_interaction_ranges(&mut self) {
        let creative = self.game_type == bbb_protocol::game_types::GameType::Creative;
        for (attribute, id, amount) in [
            (
                VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
                "minecraft:creative_mode_block_range",
                0.5,
            ),
            (
                VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID,
                "minecraft:creative_mode_entity_range",
                2.0,
            ),
        ] {
            let instance = self
                .attributes
                .get_instance(attribute)
                .expect("the player supplier declares both interaction ranges");
            if creative {
                instance.add_or_update_transient_modifier(
                    crate::entity::attributes::AttributeModifier {
                        id: id.to_owned(),
                        amount,
                        operation: crate::entity::attributes::AttributeOperation::AddValue,
                    },
                );
            } else {
                instance.remove_modifier(id);
            }
        }
    }

    /// `Player.blockInteractionRange` (`Player.java:1969-1971`).
    pub(super) fn block_interaction_range(&self) -> f64 {
        self.attribute_value(VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)
    }

    /// `Player.entityInteractionRange` (`Player.java:1973-1975`).
    pub(super) fn entity_interaction_range(&self) -> f64 {
        self.attribute_value(VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID)
    }

    /// What `Player.getDestroySpeed` (`Player.java:586-616`) reads off the session.
    pub(super) fn destroy_speed_facts(&self) -> crate::block_destroy::PlayerDestroySpeedFacts {
        crate::block_destroy::PlayerDestroySpeedFacts {
            mining_efficiency: self.attribute_value(VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID),
            block_break_speed: self.attribute_value(VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID),
            submerged_mining_speed: self
                .attribute_value(VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID),
            dig_speed_amplification: self.dig_speed_amplification(),
            mining_fatigue_amplifier: self
                .active_effect("minecraft:mining_fatigue")
                .map(crate::entity::living::mob_effects::ActiveMobEffect::amplifier),
            eye_in_water: self
                .fluid_interaction
                .is_eye_in_fluid(crate::entity::FluidTag::Water),
            on_ground: self.on_ground(),
        }
    }

    /// `LivingEntity.getCurrentSwingDuration` (LivingEntity.java:1978-1986).
    pub(super) fn current_swing_duration(&self) -> i32 {
        let stack = match self.swinging_arm {
            InteractionHand::MainHand => self.inventory.selected_stack(),
            InteractionHand::OffHand => self.inventory.off_hand_stack(),
        };
        let duration = crate::entity::living::swing_animation_duration(stack);
        if let Some(amplification) = self.dig_speed_amplification() {
            return duration - (1 + amplification);
        }
        match self.active_effect("minecraft:mining_fatigue") {
            Some(effect) => duration + (1 + effect.amplifier()) * 2,
            None => duration,
        }
    }
}

/// `Entity.blockPosition` (`Entity.java:3634-3636`), the floored position
/// `setPos` keeps on the entity (`Entity.java:3729-3733`).
fn block_position(player: &PlayerState) -> BlockPos {
    BlockPos {
        x: player.position().x.floor() as i32,
        y: player.position().y.floor() as i32,
        z: player.position().z.floor() as i32,
    }
}

fn position_array(position: BlockPos) -> [i32; 3] {
    [position.x, position.y, position.z]
}
