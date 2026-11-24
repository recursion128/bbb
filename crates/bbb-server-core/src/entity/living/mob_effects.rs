use crate::world::simulation_level::LevelCtx;

use crate::connection::ClientboundPlayPacket;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_GLOWING, VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
    VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
};
use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type_for_resource_id, MobEffectTypeCatalogEntry,
    VANILLA_MOB_EFFECT_ABSORPTION_ID, VANILLA_MOB_EFFECT_BAD_OMEN_ID,
    VANILLA_MOB_EFFECT_GLOWING_ID, VANILLA_MOB_EFFECT_HUNGER_ID, VANILLA_MOB_EFFECT_INFESTED_ID,
    VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID, VANILLA_MOB_EFFECT_INSTANT_HEALTH_ID,
    VANILLA_MOB_EFFECT_INVISIBILITY_ID, VANILLA_MOB_EFFECT_OOZING_ID, VANILLA_MOB_EFFECT_POISON_ID,
    VANILLA_MOB_EFFECT_RAID_OMEN_ID, VANILLA_MOB_EFFECT_REGENERATION_ID,
    VANILLA_MOB_EFFECT_SATURATION_ID, VANILLA_MOB_EFFECT_TYPES, VANILLA_MOB_EFFECT_WEAVING_ID,
    VANILLA_MOB_EFFECT_WIND_CHARGED_ID, VANILLA_MOB_EFFECT_WITHER_ID,
};
use bbb_protocol::packets::ParticlePayload;
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID, VANILLA_PARTICLE_TYPE_INFESTED_ID,
    VANILLA_PARTICLE_TYPE_ITEM_COBWEB_ID, VANILLA_PARTICLE_TYPE_ITEM_SLIME_ID,
    VANILLA_PARTICLE_TYPE_RAID_OMEN_ID, VANILLA_PARTICLE_TYPE_SMALL_GUST_ID,
    VANILLA_PARTICLE_TYPE_TRIAL_OMEN_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::loot::MobEffectInstancePredicate;
use bbb_server_types::food::{cause_food_exhaustion, FoodData};

use super::LivingEntityState;
use crate::entity::attributes::AttributeMap;
#[cfg(test)]
use crate::entity::level::builtin_registry_tag_membership;
use crate::entity::projectile_action::mob_effect::{
    PendingMobEffectAction, PendingRemovalMobEffect, RemovedLivingSnapshot,
};
use crate::entity::{Entity, EntityStore, RemovalReason};

pub mod client_update;
pub mod effect_particles;
use effect_particles::particles_write_marks_dirty;
pub(crate) mod saved_codec;

pub(crate) fn mob_effects_predicate_matches<'a, E>(
    predicates: &[(String, MobEffectInstancePredicate)],
    mut lookup: impl FnMut(&str) -> Result<Option<&'a ActiveMobEffect>, E>,
) -> Result<bool, E> {
    let mut matched = true;
    for (id, expected) in predicates {
        matched &= lookup(id)?.is_some_and(|actual| {
            expected.amplifier.matches(actual.amplifier())
                && expected.duration.matches(actual.duration())
                && expected
                    .ambient
                    .is_none_or(|value| value == actual.ambient())
                && expected
                    .visible
                    .is_none_or(|value| value == actual.show_particles())
        });
    }
    Ok(matched)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveMobEffect {
    effect_id: i32,
    duration: i32,
    amplifier: i32,
    ambient: bool,
    show_particles: bool,
    show_icon: bool,
    hidden_effect: Option<Box<Self>>,
}

impl ActiveMobEffect {
    /// A built-in effect retained by the server-side effect list.
    pub fn new(resource_id: &'static str, duration: i32, amplifier: i32) -> Option<Self> {
        let effect = vanilla_mob_effect_type_for_resource_id(resource_id)?;
        (duration > 0)
            .then(|| Self::from_catalog(effect, duration, amplifier, false, true, true, None))
    }

    pub fn give(
        resource_id: &'static str,
        duration: i32,
        amplifier: i32,
        particles: bool,
    ) -> Option<Self> {
        Self::from_saved(
            resource_id,
            duration,
            amplifier,
            false,
            particles,
            particles,
        )
    }

    pub fn with_visibility(
        resource_id: &'static str,
        duration: i32,
        amplifier: i32,
        ambient: bool,
        visible: bool,
        show_icon: bool,
    ) -> Option<Self> {
        Self::from_saved(
            resource_id,
            duration,
            amplifier,
            ambient,
            visible,
            show_icon,
        )
    }

    pub(crate) fn from_saved(
        resource_id: &'static str,
        duration: i32,
        amplifier: i32,
        ambient: bool,
        show_particles: bool,
        show_icon: bool,
    ) -> Option<Self> {
        (duration == -1 || duration > 0).then(|| {
            Self::from_saved_with_hidden(
                resource_id,
                duration,
                amplifier,
                ambient,
                show_particles,
                show_icon,
                None,
            )
        })?
    }

    pub fn from_saved_with_hidden(
        resource_id: &'static str,
        duration: i32,
        amplifier: i32,
        ambient: bool,
        show_particles: bool,
        show_icon: bool,
        hidden_effect: Option<Self>,
    ) -> Option<Self> {
        vanilla_mob_effect_type_for_resource_id(resource_id).map(|effect| {
            Self::from_catalog(
                effect,
                duration,
                amplifier,
                ambient,
                show_particles,
                show_icon,
                hidden_effect,
            )
        })
    }

    #[inline]
    pub(crate) fn from_catalog(
        effect: &MobEffectTypeCatalogEntry,
        duration: i32,
        amplifier: i32,
        ambient: bool,
        show_particles: bool,
        show_icon: bool,
        hidden_effect: Option<Self>,
    ) -> Self {
        Self {
            effect_id: effect.id,
            duration,
            amplifier: amplifier.clamp(0, 255),
            ambient,
            show_particles,
            show_icon,
            hidden_effect: hidden_effect.map(Box::new),
        }
    }

    #[must_use]
    pub fn with_scaled_duration(&self, scale: f32) -> Self {
        let mut copy = self.clone();
        if copy.duration != -1 && copy.duration != 0 {
            copy.duration = ((copy.duration as f32 * scale).floor() as i32).max(1);
        }
        copy
    }

    pub const fn resource_id(&self) -> &'static str {
        self.facts().resource_id
    }

    pub const fn duration(&self) -> i32 {
        self.duration
    }

    pub const fn amplifier(&self) -> i32 {
        self.amplifier
    }

    pub const fn ambient(&self) -> bool {
        self.ambient
    }

    pub const fn show_particles(&self) -> bool {
        self.show_particles
    }

    pub const fn show_icon(&self) -> bool {
        self.show_icon
    }

    pub(crate) fn to_summary(&self) -> bbb_protocol::packets::MobEffectInstanceSummary {
        let details = self.summary_details();
        bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: self.effect_id,
            amplifier: details.amplifier,
            duration: details.duration,
            ambient: details.ambient,
            show_particles: details.show_particles,
            show_icon: details.show_icon,
            hidden_effect: details.hidden_effect,
        }
    }

    fn summary_details(&self) -> bbb_protocol::packets::MobEffectDetailsSummary {
        bbb_protocol::packets::MobEffectDetailsSummary {
            amplifier: self.amplifier,
            duration: self.duration,
            ambient: self.ambient,
            show_particles: self.show_particles,
            show_icon: self.show_icon,
            hidden_effect: self
                .hidden_effect
                .as_ref()
                .map(|effect| Box::new(effect.summary_details())),
        }
    }

    pub fn to_saved_nbt(&self) -> NbtSummaryValue {
        self.to_saved_details(true)
    }

    fn to_saved_details(&self, root: bool) -> NbtSummaryValue {
        let mut fields = Vec::new();
        if root {
            fields.push(NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(self.resource_id()),
            });
        }
        if self.amplifier != 0 {
            fields.push(NbtSummaryEntry {
                name: "amplifier".into(),
                value: NbtSummaryValue::Byte(self.amplifier as u8 as i8),
            });
        }
        if self.duration != 0 {
            fields.push(NbtSummaryEntry {
                name: "duration".into(),
                value: NbtSummaryValue::Int(self.duration),
            });
        }
        if self.ambient {
            fields.push(NbtSummaryEntry {
                name: "ambient".into(),
                value: NbtSummaryValue::Byte(1),
            });
        }
        if !self.show_particles {
            fields.push(NbtSummaryEntry {
                name: "show_particles".into(),
                value: NbtSummaryValue::Byte(0),
            });
        }
        fields.push(NbtSummaryEntry {
            name: "show_icon".into(),
            value: NbtSummaryValue::Byte(if self.show_icon { 1 } else { 0 }),
        });
        if let Some(hidden) = &self.hidden_effect {
            fields.push(NbtSummaryEntry {
                name: "hidden_effect".into(),
                value: hidden.to_saved_details(false),
            });
        }
        NbtSummaryValue::Compound(fields)
    }

    const fn facts(&self) -> &'static MobEffectTypeCatalogEntry {
        &VANILLA_MOB_EFFECT_TYPES[self.effect_id as usize]
    }

    fn is_infinite(&self) -> bool {
        self.duration == -1
    }

    fn tick_count_for(&self, entity_tick_count: i32) -> i32 {
        if self.is_infinite() {
            entity_tick_count
        } else {
            self.duration
        }
    }

    fn has_remaining_duration(&self) -> bool {
        self.is_infinite() || self.duration > 0
    }

    fn is_shorter_duration_than(&self, other: &Self) -> bool {
        !self.is_infinite() && (self.duration < other.duration || other.is_infinite())
    }

    fn update(&mut self, incoming: Self) -> bool {
        debug_assert_eq!(self.effect_id, incoming.effect_id);
        let mut changed = false;
        if incoming.amplifier > self.amplifier {
            if incoming.is_shorter_duration_than(self) {
                let hidden_effect = Box::new(Self {
                    effect_id: self.effect_id,
                    duration: self.duration,
                    amplifier: self.amplifier,
                    ambient: self.ambient,
                    show_particles: self.show_particles,
                    show_icon: self.show_icon,
                    hidden_effect: self.hidden_effect.take(),
                });
                self.hidden_effect = Some(hidden_effect);
            }
            self.amplifier = incoming.amplifier;
            self.duration = incoming.duration;
            changed = true;
        } else if self.is_shorter_duration_than(&incoming) {
            if incoming.amplifier == self.amplifier {
                self.duration = incoming.duration;
                changed = true;
            } else if let Some(hidden) = &mut self.hidden_effect {
                hidden.update(incoming.clone());
            } else {
                self.hidden_effect = Some(Box::new(Self {
                    hidden_effect: None,
                    ..incoming.clone()
                }));
            }
        }

        if (!incoming.ambient && self.ambient) || changed {
            self.ambient = incoming.ambient;
            changed = true;
        }
        if incoming.show_particles != self.show_particles {
            self.show_particles = incoming.show_particles;
            changed = true;
        }
        if incoming.show_icon != self.show_icon {
            self.show_icon = incoming.show_icon;
            changed = true;
        }
        changed
    }

    fn map_duration(&self, mapper: impl FnOnce(i32) -> i32) -> i32 {
        if self.is_infinite() || self.duration == 0 {
            self.duration
        } else {
            mapper(self.duration)
        }
    }

    fn tick_down_duration(&mut self) {
        if let Some(hidden) = &mut self.hidden_effect {
            hidden.tick_down_duration();
        }
        self.duration = self.map_duration(|duration| duration.wrapping_sub(1));
    }

    fn downgrade_to_hidden_effect(&mut self) -> bool {
        if self.duration != 0 {
            return false;
        }
        let Some(mut hidden) = self.hidden_effect.take() else {
            return false;
        };
        self.duration = hidden.duration;
        self.amplifier = hidden.amplifier;
        self.ambient = hidden.ambient;
        self.show_particles = hidden.show_particles;
        self.show_icon = hidden.show_icon;
        self.hidden_effect = hidden.hidden_effect.take();
        true
    }

    fn particle(&self) -> ParticlePayload {
        let particle_type_id = match self.resource_id() {
            "minecraft:trial_omen" => VANILLA_PARTICLE_TYPE_TRIAL_OMEN_ID,
            "minecraft:raid_omen" => VANILLA_PARTICLE_TYPE_RAID_OMEN_ID,
            "minecraft:wind_charged" => VANILLA_PARTICLE_TYPE_SMALL_GUST_ID,
            "minecraft:weaving" => VANILLA_PARTICLE_TYPE_ITEM_COBWEB_ID,
            "minecraft:oozing" => VANILLA_PARTICLE_TYPE_ITEM_SLIME_ID,
            "minecraft:infested" => VANILLA_PARTICLE_TYPE_INFESTED_ID,
            _ => VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        };
        let raw_options = if particle_type_id == VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID {
            let alpha = if self.ambient { 38_u32 } else { 255_u32 };
            ((alpha << 24) | self.facts().color).to_be_bytes().to_vec()
        } else {
            Vec::new()
        };
        ParticlePayload {
            particle_type_id,
            raw_options,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobEffectHurt {
    pub damage_type: &'static str,
    pub amount: f32,
}

#[derive(Debug, Default, PartialEq)]
pub struct MobEffectTickResult {
    pub hurts: Vec<MobEffectHurt>,
    pub effects_changed: bool,
    pub removed: Vec<ActiveMobEffect>,
    /// The instances `onEffectUpdated` owes a `ClientboundUpdateMobEffectPacket`
    /// this tick: the hidden-effect downgrade (`:836`) and the six-hundredth
    /// tick refresh (`:839-840`).
    pub refreshed: Vec<ActiveMobEffect>,
}

/// The rest of the entity one effect operation reads through `this`.
///
pub struct MobEffectBody<'a> {
    pub attributes: &'a mut AttributeMap,
    pub health: &'a mut f32,
    pub absorption_amount: &'a mut f32,
    pub food: Option<&'a mut FoodData>,
    pub abilities_invulnerable: bool,
}

impl MobEffectBody<'_> {
    fn max_health(&self) -> f32 {
        self.attributes
            .value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap_or(0.0) as f32
    }

    fn max_absorption(&self) -> f32 {
        self.attributes
            .value(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID)
            .unwrap_or(0.0) as f32
    }

    fn set_health(&mut self, health: f32) {
        let max = self.max_health();
        *self.health = if health < 0.0 { 0.0 } else { health.min(max) };
    }

    fn heal(&mut self, amount: f32) {
        if *self.health > 0.0 {
            self.set_health(*self.health + amount);
        }
    }

    fn set_absorption_amount(&mut self, amount: f32) {
        let max = self.max_absorption();
        *self.absorption_amount = if amount < 0.0 { 0.0 } else { amount.min(max) };
    }

    ///
    /// The third arm, `Attributes.SCALE` (`:1120`), calls `refreshDimensions`,
    /// which needs the entity a body does not carry; a stored entity reaches it
    /// through [`crate::entity::living::refresh_dirty_attributes`] instead.
    pub fn refresh_dirty_attributes(&mut self) {
        for attribute_id in self.attributes.drain_attributes_to_update() {
            if attribute_id == VANILLA_ATTRIBUTE_MAX_HEALTH_ID {
                let max = self.max_health();
                if *self.health > max {
                    self.set_health(max);
                }
            } else if attribute_id == VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID {
                let max = self.max_absorption();
                if *self.absorption_amount > max {
                    self.set_absorption_amount(max);
                }
            }
        }
    }
}

///
/// `None` is the same refusal the stored path makes for an unavailable
/// registry: the whole sweep declines rather than running against a guessed tag.
pub fn tick_effects_with_registry_tags(
    effects: &mut ActiveMobEffects,
    body: &mut MobEffectBody<'_>,
    registry_tags: &crate::registry_tags::RegistryTagSnapshot,
    entity_resource_id: &str,
    entity_tick_count: i32,
) -> Option<MobEffectTickResult> {
    let inverted = if effects.applies_heal_or_harm(entity_tick_count) {
        registry_tags.contains(
            "minecraft:entity_type",
            "minecraft:inverted_healing_and_harm",
            entity_resource_id,
        )
    } else {
        false
    };
    Some(effects.tick(body, inverted, entity_tick_count))
}

///
/// Every operation on it is vanilla's, taken once: a stored entity reaches them
/// through [`LivingEntityState`], and a session-owned player through its own
/// copy of this container.
const HEAL_OR_HARM_EFFECTS_MASK: u64 =
    (1u64 << VANILLA_MOB_EFFECT_INSTANT_HEALTH_ID) | (1u64 << VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID);

const DEATH_SPAWN_EFFECTS_MASK: u64 = (1u64 << VANILLA_MOB_EFFECT_OOZING_ID)
    | (1u64 << VANILLA_MOB_EFFECT_WEAVING_ID)
    | (1u64 << VANILLA_MOB_EFFECT_WIND_CHARGED_ID);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveMobEffects {
    /// Stored in ascending `resource_id` order to preserve `to_saved_list` and `iter` order.
    effects: Vec<ActiveMobEffect>,
    present_mask: u64,
    dirty: bool,
}

impl ActiveMobEffects {
    pub fn iter(&self) -> impl Iterator<Item = &ActiveMobEffect> {
        self.effects.iter()
    }

    pub fn get(&self, resource_id: &str) -> Option<&ActiveMobEffect> {
        if self.present_mask == 0 {
            return None;
        }
        self.effects
            .binary_search_by(|effect| effect.resource_id().cmp(resource_id))
            .ok()
            .map(|index| &self.effects[index])
    }

    #[inline]
    pub(crate) fn has_id(&self, effect_id: i32) -> bool {
        usize::try_from(effect_id)
            .is_ok_and(|id| id < 64 && (self.present_mask & (1u64 << id)) != 0)
    }

    pub fn is_empty(&self) -> bool {
        self.present_mask == 0
    }

    ///
    /// Vanilla lists `activeEffects.values()` of a `HashMap` (`:207`); this
    /// walks the same instances in resource-id order, which the official
    /// server does not itself reproduce between runs.
    pub fn to_saved_list(&self) -> Vec<NbtSummaryValue> {
        self.effects
            .iter()
            .map(ActiveMobEffect::to_saved_nbt)
            .collect()
    }

    pub(crate) fn from_saved_list(values: &[NbtSummaryValue]) -> Self {
        Self::from_decoded_saved_effects(values.iter().filter_map(saved_codec::decode_saved_effect))
    }

    fn insert_sorted(&mut self, effect: ActiveMobEffect) {
        let bit = 1u64 << (effect.effect_id as u32);
        if self.present_mask & bit != 0 {
            if let Some(existing) = self
                .effects
                .iter_mut()
                .find(|existing| existing.effect_id == effect.effect_id)
            {
                *existing = effect;
                return;
            }
        }
        let index = self
            .effects
            .binary_search_by(|existing| existing.resource_id().cmp(effect.resource_id()))
            .unwrap_err();
        self.effects.insert(index, effect);
        self.present_mask |= bit;
    }

    pub(crate) fn from_decoded_saved_effects(
        decoded: impl IntoIterator<Item = ActiveMobEffect>,
    ) -> Self {
        let mut this = Self::default();
        for effect in decoded {
            this.insert_sorted(effect);
        }
        this.dirty = this.present_mask != 0;
        this
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[cfg(test)]
    pub(crate) fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn add(
        &mut self,
        body: &mut MobEffectBody<'_>,
        effect: ActiveMobEffect,
    ) -> Option<ActiveMobEffect> {
        let facts = effect.facts();
        let amplifier = effect.amplifier;
        let bit = 1u64 << (facts.id as u32);
        let published = if self.present_mask & bit == 0 {
            body.attributes
                .replace_mob_effect_modifiers(facts, amplifier);
            let index = self
                .effects
                .binary_search_by(|existing| existing.resource_id().cmp(facts.resource_id))
                .unwrap_err();
            self.effects.insert(index, effect.clone());
            self.present_mask |= bit;
            Some(effect)
        } else {
            let current = self
                .effects
                .iter_mut()
                .find(|existing| existing.effect_id == facts.id)
                .expect("present_mask bit matches active effect");
            let changed = current.update(effect);
            if changed {
                body.attributes.remove_mob_effect_modifiers(facts);
                body.attributes
                    .replace_mob_effect_modifiers(facts, current.amplifier);
            }
            changed.then(|| current.clone())
        };
        // `LivingEntity.addEffect` calls `newEffect.onEffectStarted(this)` even when
        // `effect.update(newEffect)` returns false (LivingEntity.java:1010).
        if facts.id == VANILLA_MOB_EFFECT_ABSORPTION_ID {
            body.set_absorption_amount(body.absorption_amount.max(4.0 * (amplifier + 1) as f32));
        }
        self.dirty |= published.is_some();
        published
    }

    pub fn remove_no_update(&mut self, resource_id: &str) -> Option<ActiveMobEffect> {
        if self.present_mask == 0 {
            return None;
        }
        let index = self
            .effects
            .binary_search_by(|effect| effect.resource_id().cmp(resource_id))
            .ok()?;
        let removed = self.effects.remove(index);
        self.present_mask &= !(1u64 << (removed.effect_id as u32));
        if self.present_mask == 0 {
            self.effects = Vec::new();
        }
        Some(removed)
    }

    pub fn on_removed(&mut self, body: &mut MobEffectBody<'_>, effects: &[ActiveMobEffect]) {
        self.dirty = true;
        for effect in effects {
            body.attributes.remove_mob_effect_modifiers(effect.facts());
        }
    }

    pub fn take_all(&mut self) -> Vec<ActiveMobEffect> {
        self.present_mask = 0;
        std::mem::take(&mut self.effects)
    }

    pub fn replace_all(
        &mut self,
        body: &mut MobEffectBody<'_>,
        effects: impl IntoIterator<Item = ActiveMobEffect>,
    ) {
        for effect in &self.effects {
            body.attributes.remove_mob_effect_modifiers(effect.facts());
        }
        self.effects.clear();
        self.present_mask = 0;
        for effect in effects {
            body.attributes
                .replace_mob_effect_modifiers(effect.facts(), effect.amplifier);
            self.insert_sorted(effect);
        }
        if self.present_mask == 0 {
            self.effects = Vec::new();
        }
        self.dirty = true;
    }

    pub fn applies_heal_or_harm(&self, entity_tick_count: i32) -> bool {
        if self.present_mask & HEAL_OR_HARM_EFFECTS_MASK == 0 {
            return false;
        }
        self.effects.iter().any(|effect| {
            effect.has_remaining_duration()
                && (effect.effect_id == VANILLA_MOB_EFFECT_INSTANT_HEALTH_ID
                    || effect.effect_id == VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID)
                && should_apply_id(
                    effect.effect_id,
                    effect.tick_count_for(entity_tick_count),
                    effect.amplifier,
                )
        })
    }

    pub fn tick(
        &mut self,
        body: &mut MobEffectBody<'_>,
        inverted: bool,
        entity_tick_count: i32,
    ) -> MobEffectTickResult {
        if self.present_mask == 0 {
            return MobEffectTickResult::default();
        }
        let mut result = MobEffectTickResult::default();
        let mut index = 0;
        while index < self.effects.len() {
            let should_remove = {
                let effect = &mut self.effects[index];
                let effect_id = effect.effect_id;
                if !effect.has_remaining_duration()
                    || (should_apply_id(
                        effect_id,
                        effect.tick_count_for(entity_tick_count),
                        effect.amplifier,
                    ) && matches!(
                        apply_effect_tick_id(
                            body,
                            effect_id,
                            effect.amplifier,
                            inverted,
                            &mut result.hurts,
                        ),
                        EffectTickApplication::Remove
                    ))
                {
                    body.attributes.remove_mob_effect_modifiers(effect.facts());
                    true
                } else {
                    effect.tick_down_duration();
                    if effect.downgrade_to_hidden_effect() {
                        body.attributes
                            .replace_mob_effect_modifiers(effect.facts(), effect.amplifier);
                        result.effects_changed = true;
                        result.refreshed.push(effect.clone());
                    }
                    if effect.has_remaining_duration() {
                        // `:839-840` is an `else if` on `tickServer`'s answer, not on
                        // the downgrade above, so an instance that took over a hidden
                        // duration already divisible by six hundred is announced twice.
                        if effect.duration % 600 == 0 {
                            result.refreshed.push(effect.clone());
                            self.dirty = true;
                        }
                        false
                    } else {
                        body.attributes.remove_mob_effect_modifiers(effect.facts());
                        true
                    }
                }
            };
            if should_remove {
                let removed = self.effects.remove(index);
                self.present_mask &= !(1u64 << (removed.effect_id as u32));
                result.removed.push(removed);
            } else {
                index += 1;
            }
        }
        result.effects_changed |= !result.removed.is_empty();
        // Effectless entities should retain no heap allocation.
        if self.present_mask == 0 {
            self.effects = Vec::new();
        }
        self.dirty |= result.effects_changed;
        result
    }
}

fn resolve_inverted_heal_and_harm(
    effects: &ActiveMobEffects,
    level: &LevelCtx<'_, '_>,
    entity_resource_id: &str,
    entity_tick_count: i32,
) -> bool {
    effects.applies_heal_or_harm(entity_tick_count)
        && level.registry_tag_membership(
            "minecraft:entity_type",
            "minecraft:inverted_healing_and_harm",
            entity_resource_id,
        )
}

impl LivingEntityState {
    #[inline]
    fn effect_parts(&mut self) -> (&mut ActiveMobEffects, MobEffectBody<'_>) {
        (
            &mut self.effects,
            MobEffectBody {
                attributes: &mut self.attributes,
                health: &mut self.health,
                absorption_amount: &mut self.absorption_amount,
                food: None,
                abilities_invulnerable: false,
            },
        )
    }

    pub fn active_effects(&self) -> impl Iterator<Item = &ActiveMobEffect> {
        self.effects.iter()
    }

    pub fn active_effect(&self, resource_id: &str) -> Option<&ActiveMobEffect> {
        self.effects.get(resource_id)
    }

    pub fn add_active_effect(&mut self, effect: ActiveMobEffect) -> bool {
        self.add_active_effect_published(effect).is_some()
    }

    pub(crate) fn add_active_effect_published(
        &mut self,
        effect: ActiveMobEffect,
    ) -> Option<ActiveMobEffect> {
        let (effects, mut body) = self.effect_parts();
        effects.add(&mut body, effect)
    }

    fn remove_active_effect_no_update(&mut self, resource_id: &str) -> Option<ActiveMobEffect> {
        self.effects.remove_no_update(resource_id)
    }

    fn on_effects_removed(&mut self, effects: &[ActiveMobEffect]) {
        let (active, mut body) = self.effect_parts();
        active.on_removed(&mut body, effects);
    }

    pub(crate) fn replace_active_effects(
        &mut self,
        effects: impl IntoIterator<Item = ActiveMobEffect>,
    ) {
        let (active, mut body) = self.effect_parts();
        active.replace_all(&mut body, effects);
    }

    pub(super) fn tick_active_effects(
        &mut self,
        level: &LevelCtx<'_, '_>,
        entity_resource_id: &str,
        entity_tick_count: i32,
    ) -> MobEffectTickResult {
        let inverted = resolve_inverted_heal_and_harm(
            &self.effects,
            level,
            entity_resource_id,
            entity_tick_count,
        );
        let (effects, mut body) = self.effect_parts();
        effects.tick(&mut body, inverted, entity_tick_count)
    }

    pub(super) fn visible_effect_particles(&self) -> Vec<ParticlePayload> {
        self.effects
            .iter()
            .filter(|effect| effect.show_particles)
            .map(ActiveMobEffect::particle)
            .collect()
    }

    pub(super) fn all_visible_effects_ambient(&self) -> bool {
        !self.effects.is_empty()
            && self
                .effects
                .iter()
                .all(|effect| !effect.show_particles || effect.ambient)
    }

    fn take_removal_effects(&mut self, reason: RemovalReason) -> Vec<PendingRemovalMobEffect> {
        if !reason.should_destroy() {
            return Vec::new();
        }
        let effects = if reason == RemovalReason::Killed
            && self.effects.present_mask & DEATH_SPAWN_EFFECTS_MASK != 0
        {
            self.effects
                .iter()
                .filter_map(|effect| match effect.effect_id {
                    VANILLA_MOB_EFFECT_OOZING_ID => Some(PendingRemovalMobEffect::Oozing),
                    VANILLA_MOB_EFFECT_WEAVING_ID => Some(PendingRemovalMobEffect::Weaving),
                    VANILLA_MOB_EFFECT_WIND_CHARGED_ID => {
                        Some(PendingRemovalMobEffect::WindCharged)
                    }
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        let removed = self.effects.take_all();
        self.on_effects_removed(&removed);
        effects
    }
}

pub(crate) struct PreparedRemovalMobEffects {
    pub source: RemovedLivingSnapshot,
    pub effects: Vec<PendingRemovalMobEffect>,
}

pub(crate) fn prepare_mob_removed(
    entity: &mut Entity,
    reason: RemovalReason,
) -> Option<PreparedRemovalMobEffects> {
    let effects = entity
        .living_mut()
        .map(|living| living.take_removal_effects(reason))
        .unwrap_or_default();
    if effects.is_empty() {
        return None;
    }
    let source = RemovedLivingSnapshot {
        entity_id: entity.id(),
        entity_resource_id: entity.data().entity_resource_id(),
        uuid: entity.uuid(),
        is_mob: matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_mob()),
        position: entity.position(),
        block_position: entity.block_position(),
        bounding_box: entity.bounding_box(),
        was_touching_water: entity.was_touching_water(),
    };
    Some(PreparedRemovalMobEffects { source, effects })
}

pub(crate) fn on_mob_hurt(entity: &Entity, store: &mut EntityStore, succeeded: bool) {
    if !succeeded
        || entity
            .living()
            .is_none_or(|living| !living.effects.has_id(VANILLA_MOB_EFFECT_INFESTED_ID))
    {
        return;
    }
    let velocities = infested_child_velocities(store.random(), entity.x_rot(), entity.y_rot());
    if velocities.is_empty() {
        return;
    }
    store.queue_command(crate::entity::EntityCommand::MobEffect(
        PendingMobEffectAction::Infested {
            position: bbb_protocol::packets::Vec3d {
                x: entity.position().x,
                y: entity.position().y + f64::from(entity.bb_height() / 2.0),
                z: entity.position().z,
            },
            velocities,
        },
    ));
}

pub fn infested_child_velocities(
    random: &crate::random::GameRandom,
    x_rot: f32,
    y_rot: f32,
) -> Vec<bbb_protocol::packets::Vec3d> {
    if random.next_float() > 0.1 {
        return Vec::new();
    }
    let count = random.next_int(2) + 1;
    let real_x_rot = x_rot * (std::f32::consts::PI / 180.0);
    let real_y_rot = -y_rot * (std::f32::consts::PI / 180.0);
    let x_cos = real_x_rot.cos();
    let view_x = real_y_rot.sin() * x_cos * 0.3;
    let view_y = -real_x_rot.sin() * 0.3 * 1.5;
    let view_z = real_y_rot.cos() * x_cos * 0.3;
    (0..count)
        .map(|_| {
            let angle = random.next_float() * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
            let sin = angle.sin();
            let cos = angle.cos();
            bbb_protocol::packets::Vec3d {
                x: f64::from(view_x * cos + view_z * sin),
                y: f64::from(view_y),
                z: f64::from(-view_x * sin + view_z * cos),
            }
        })
        .collect()
}

pub(crate) fn compute_living_visibility_flags(
    has_invisibility: bool,
    has_glowing: bool,
    spectator: bool,
    allow_invisibility: bool,
    glowing_tag: bool,
) -> (Option<bool>, bool) {
    let invisible = if spectator {
        Some(true)
    } else if allow_invisibility {
        Some(has_invisibility)
    } else {
        None
    };
    (invisible, glowing_tag || has_glowing)
}

pub(super) fn synchronize_metadata(entity: &mut Entity) {
    let armor_stand = matches!(entity.data(), crate::entity::EntityData::ArmorStand(_));
    let glowing_tag = entity.has_glowing_tag();
    let Some(living) = entity.living_mut() else {
        return;
    };
    if !living.effects.take_dirty() {
        return;
    }
    let (invisible, glowing) = compute_living_visibility_flags(
        living.effects.has_id(VANILLA_MOB_EFFECT_INVISIBILITY_ID),
        living.effects.has_id(VANILLA_MOB_EFFECT_GLOWING_ID),
        false,
        !armor_stand,
        glowing_tag,
    );
    let (particles_changed, ambience_changed) = if armor_stand {
        (false, false)
    } else {
        let (particles, ambience) = updated_effect_rows(living);
        let particles_changed = particles_write_marks_dirty(&living.effect_particles, &particles);
        living.effect_particles = particles;
        let ambience_changed = ambience != living.effect_ambience;
        living.effect_ambience = ambience;
        (particles_changed, ambience_changed)
    };
    if let Some(invisible) = invisible {
        entity.set_invisible(invisible);
    }
    entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_GLOWING, glowing);
    if particles_changed {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID);
    }
    if ambience_changed {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID);
    }
}

fn updated_effect_rows(living: &LivingEntityState) -> (Vec<ParticlePayload>, bool) {
    if living.effects.is_empty() {
        return (Vec::new(), living.effect_ambience);
    }
    (
        living.visible_effect_particles(),
        living.all_visible_effects_ambient(),
    )
}

pub(crate) fn synced_effect_rows(
    living: &LivingEntityState,
    armor_stand: bool,
) -> (Vec<ParticlePayload>, bool) {
    if armor_stand || !living.effects.is_dirty() {
        return (living.effect_particles.clone(), living.effect_ambience);
    }
    updated_effect_rows(living)
}

///
/// Vanilla reads the flag off the effect, so `MobEffectTypeCatalogEntry` is
/// where it belongs; that catalog is generated and neither it nor its generator
/// is writable from this crate, so the three ids are enumerated once here rather
/// than at each reader.
pub fn is_instantaneous(effect_resource_id: &str) -> bool {
    matches!(
        effect_resource_id,
        "minecraft:instant_health" | "minecraft:instant_damage" | "minecraft:saturation"
    )
}

fn should_apply_id(effect_id: i32, tick_count: i32, amplifier: i32) -> bool {
    let interval = match effect_id {
        VANILLA_MOB_EFFECT_REGENERATION_ID => Some(50_i32 >> (amplifier & 31)),
        VANILLA_MOB_EFFECT_POISON_ID => Some(25_i32 >> (amplifier & 31)),
        VANILLA_MOB_EFFECT_WITHER_ID => Some(40_i32 >> (amplifier & 31)),
        VANILLA_MOB_EFFECT_INSTANT_HEALTH_ID
        | VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID
        | VANILLA_MOB_EFFECT_SATURATION_ID
        | VANILLA_MOB_EFFECT_ABSORPTION_ID
        | VANILLA_MOB_EFFECT_HUNGER_ID
        | VANILLA_MOB_EFFECT_BAD_OMEN_ID => return true,
        VANILLA_MOB_EFFECT_RAID_OMEN_ID => return tick_count == 1,
        _ => return false,
    };
    interval.is_some_and(|interval| interval <= 0 || tick_count % interval == 0)
}

fn apply_effect_tick_id(
    body: &mut MobEffectBody<'_>,
    effect_id: i32,
    amplifier: i32,
    inverted: bool,
    hurts: &mut Vec<MobEffectHurt>,
) -> EffectTickApplication {
    match effect_id {
        VANILLA_MOB_EFFECT_REGENERATION_ID => {
            if *body.health < body.max_health() {
                body.heal(1.0);
            }
        }
        VANILLA_MOB_EFFECT_POISON_ID if *body.health > 1.0 => hurts.push(MobEffectHurt {
            damage_type: "minecraft:magic",
            amount: 1.0,
        }),
        VANILLA_MOB_EFFECT_WITHER_ID => hurts.push(MobEffectHurt {
            damage_type: "minecraft:wither",
            amount: 1.0,
        }),
        VANILLA_MOB_EFFECT_INSTANT_HEALTH_ID | VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID => {
            let is_harm = effect_id == VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID;
            if is_harm == inverted {
                body.heal(4_i32.wrapping_shl(amplifier as u32).max(0) as f32);
            } else {
                hurts.push(MobEffectHurt {
                    damage_type: "minecraft:magic",
                    amount: 6_i32.wrapping_shl(amplifier as u32).max(0) as f32,
                });
            }
        }
        VANILLA_MOB_EFFECT_SATURATION_ID => {
            if let Some(food) = body.food.as_deref_mut() {
                food.eat(amplifier + 1, 1.0);
            }
        }
        VANILLA_MOB_EFFECT_HUNGER_ID => {
            if let Some(food) = body.food.as_deref_mut() {
                cause_food_exhaustion(
                    food,
                    body.abilities_invulnerable,
                    0.005 * (amplifier + 1) as f32,
                );
            }
        }
        VANILLA_MOB_EFFECT_ABSORPTION_ID if *body.absorption_amount <= 0.0 => {
            return EffectTickApplication::Remove;
        }
        _ => {}
    }
    EffectTickApplication::Keep
}

#[cfg(test)]
fn should_apply(resource_id: &str, tick_count: i32, amplifier: i32) -> bool {
    vanilla_mob_effect_type_for_resource_id(resource_id)
        .is_some_and(|entry| should_apply_id(entry.id, tick_count, amplifier))
}

#[cfg(test)]
fn apply_effect_tick(
    body: &mut MobEffectBody<'_>,
    resource_id: &str,
    amplifier: i32,
    inverted: bool,
    hurts: &mut Vec<MobEffectHurt>,
) -> EffectTickApplication {
    let Some(entry) = vanilla_mob_effect_type_for_resource_id(resource_id) else {
        return EffectTickApplication::Keep;
    };
    apply_effect_tick_id(body, entry.id, amplifier, inverted, hurts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectTickApplication {
    Keep,
    Remove,
}

pub(crate) fn can_be_affected_in_level(
    level: &LevelCtx<'_, '_>,
    entity_resource_id: &str,
    effect_resource_id: &str,
) -> bool {
    if fixed_effect_rejection(entity_resource_id, effect_resource_id) {
        return false;
    }
    can_be_affected_by(effect_resource_id, |tag_id| {
        level.registry_tag_membership("minecraft:entity_type", tag_id, entity_resource_id)
    })
}

pub fn can_be_affected_with_registry_tags(
    registry_tags: &crate::registry_tags::RegistryTagSnapshot,
    entity_resource_id: &str,
    effect_resource_id: &str,
) -> bool {
    if fixed_effect_rejection(entity_resource_id, effect_resource_id) {
        return false;
    }
    can_be_affected_by(effect_resource_id, |tag_id| {
        registry_tags.contains("minecraft:entity_type", tag_id, entity_resource_id)
    })
}

pub fn can_be_affected_with_registry_tag_view(
    registry_tags: &crate::entity::EntityRegistryTagView,
    entity_resource_id: &str,
    effect_resource_id: &str,
) -> bool {
    if fixed_effect_rejection(entity_resource_id, effect_resource_id) {
        return false;
    }
    can_be_affected_by(effect_resource_id, |tag_id| {
        registry_tags.contains("minecraft:entity_type", tag_id, entity_resource_id)
    })
}

#[cfg(test)]
pub(crate) fn can_be_affected_builtin(entity_resource_id: &str, effect_resource_id: &str) -> bool {
    if fixed_effect_rejection(entity_resource_id, effect_resource_id) {
        return false;
    }
    can_be_affected_by(effect_resource_id, |tag_id| {
        builtin_registry_tag_membership("minecraft:entity_type", tag_id, entity_resource_id)
    })
}

fn fixed_effect_rejection(entity_resource_id: &str, effect_resource_id: &str) -> bool {
    matches!(
        (entity_resource_id, effect_resource_id),
        (
            "minecraft:wither" | "minecraft:wither_skeleton",
            "minecraft:wither"
        ) | ("minecraft:parched", "minecraft:weakness")
            | (
                "minecraft:spider"
                    | "minecraft:cave_spider"
                    | "minecraft:nautilus"
                    | "minecraft:zombie_nautilus",
                "minecraft:poison"
            )
    )
}

// LivingEntity.java:1014-1022.
fn can_be_affected_by(effect_resource_id: &str, mut membership: impl FnMut(&str) -> bool) -> bool {
    match effect_resource_id {
        "minecraft:infested" => !membership("minecraft:immune_to_infested"),
        "minecraft:oozing" => {
            membership("minecraft:immune_to_infested") || !membership("minecraft:immune_to_oozing")
        }
        "minecraft:regeneration" | "minecraft:poison" => {
            membership("minecraft:immune_to_infested")
                || membership("minecraft:immune_to_oozing")
                || !membership("minecraft:ignores_poison_and_regen")
        }
        _ => true,
    }
}

///
/// The audience is the entity's own `ServerPlayer` passengers, not the entity
/// tracker: a rider learns the mount's effects from these packets, and every
/// other viewer only ever sees the particle colours the synched data carries.
///
pub(crate) fn send_effect_update(
    store: &mut EntityStore,
    entity_id: i32,
    effect: &ActiveMobEffect,
) {
    store.send_mob_effect_to_passengers(
        entity_id,
        ClientboundPlayPacket::UpdateMobEffect(client_update::update_mob_effect_packet(
            entity_id, effect, false,
        )),
    );
}

pub(super) fn send_effect_removals(
    store: &mut EntityStore,
    entity_id: i32,
    effects: &[ActiveMobEffect],
) {
    for effect in effects {
        store.send_mob_effect_to_passengers(
            entity_id,
            ClientboundPlayPacket::RemoveMobEffect(client_update::remove_mob_effect_packet(
                entity_id, effect,
            )),
        );
    }
}

pub(crate) fn add_active_effect_in_level(
    target: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    effect: ActiveMobEffect,
) -> Option<bool> {
    if fixed_add_effect_rejection(target.data().entity_resource_id()) {
        return Some(false);
    }
    let accepted = can_be_affected_in_level(
        level,
        target.data().entity_resource_id(),
        effect.resource_id(),
    );
    if !accepted {
        return Some(false);
    }
    let entity_id = target.id();
    let added_sound = target
        .living()
        .filter(|living| living.active_effect(effect.resource_id()).is_none())
        .and(sound_on_added(effect.resource_id()));
    let Some(published) = target
        .living_mut()
        .and_then(|living| living.add_active_effect_published(effect))
    else {
        return Some(false);
    };
    send_effect_update(store, entity_id, &published);
    play_sound_on_added(target, store, added_sound);
    Some(true)
}

///
/// The removed instance is handed back as well as sent: `onEffectsRemoved`
/// owes each `ServerPlayer` passenger a `ClientboundRemoveMobEffectPacket`
/// (`:1091-1095`).
pub fn remove_active_effect(
    entity: &mut Entity,
    store: &mut EntityStore,
    resource_id: &str,
) -> Option<ActiveMobEffect> {
    remove_active_effect_with(entity, store, resource_id, |entity, _store| {
        super::refresh_dirty_attributes(entity);
        Some(())
    })
    .flatten()
}

pub(crate) fn remove_active_effect_in_level(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    resource_id: &str,
) -> Option<Option<ActiveMobEffect>> {
    remove_active_effect_with(entity, store, resource_id, |entity, store| {
        super::refresh_dirty_attributes_with_level(entity, level, store)
    })
}

fn remove_active_effect_with(
    entity: &mut Entity,
    store: &mut EntityStore,
    resource_id: &str,
    refresh: impl FnOnce(&mut Entity, &mut EntityStore) -> Option<()>,
) -> Option<Option<ActiveMobEffect>> {
    if entity
        .living()
        .and_then(|living| living.active_effect(resource_id))
        .is_none()
    {
        return Some(None);
    }
    let removed = entity
        .living_mut()?
        .remove_active_effect_no_update(resource_id)?;
    entity
        .living_mut()?
        .on_effects_removed(std::slice::from_ref(&removed));
    send_effect_removals(store, entity.id(), std::slice::from_ref(&removed));
    refresh(entity, store)?;
    Some(Some(removed))
}

pub fn remove_all_active_effects(
    entity: &mut Entity,
    store: &mut EntityStore,
) -> Vec<ActiveMobEffect> {
    remove_all_active_effects_with(entity, store, |entity, _store| {
        super::refresh_dirty_attributes(entity);
        Some(())
    })
    .unwrap_or_default()
}

pub(crate) fn remove_all_active_effects_in_level(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<Vec<ActiveMobEffect>> {
    remove_all_active_effects_with(entity, store, |entity, store| {
        super::refresh_dirty_attributes_with_level(entity, level, store)
    })
}

fn remove_all_active_effects_with(
    entity: &mut Entity,
    store: &mut EntityStore,
    refresh: impl FnOnce(&mut Entity, &mut EntityStore) -> Option<()>,
) -> Option<Vec<ActiveMobEffect>> {
    if entity
        .living()
        .is_none_or(|living| living.effects.is_empty())
    {
        return Some(Vec::new());
    }
    let living = entity.living_mut()?;
    let removed = living.effects.take_all();
    living.on_effects_removed(&removed);
    send_effect_removals(store, entity.id(), &removed);
    refresh(entity, store)?;
    Some(removed)
}

pub fn add_active_effect_with_registry_tags(
    target: &mut Entity,
    store: &mut EntityStore,
    registry_tags: &crate::registry_tags::RegistryTagSnapshot,
    effect: ActiveMobEffect,
) -> Option<bool> {
    if fixed_add_effect_rejection(target.data().entity_resource_id()) {
        return Some(false);
    }
    let accepted = can_be_affected_with_registry_tags(
        registry_tags,
        target.data().entity_resource_id(),
        effect.resource_id(),
    );
    if !accepted {
        return Some(false);
    }
    let entity_id = target.id();
    let added_sound = target
        .living()
        .filter(|living| living.active_effect(effect.resource_id()).is_none())
        .and(sound_on_added(effect.resource_id()));
    let Some(published) = target
        .living_mut()
        .and_then(|living| living.add_active_effect_published(effect))
    else {
        return Some(false);
    };
    send_effect_update(store, entity_id, &published);
    play_sound_on_added(target, store, added_sound);
    Some(true)
}

/// `MobEffect.onEffectAdded` (MobEffect.java:98-100) for the effects
/// registered `withSoundOnAdded` (MobEffects.java:104-113). It goes through
/// `Level.playSound`, so a silent body still plays it.
pub(crate) fn sound_on_added(resource_id: &str) -> Option<&'static str> {
    Some(match resource_id {
        "minecraft:bad_omen" => "minecraft:event.mob_effect.bad_omen",
        "minecraft:trial_omen" => "minecraft:event.mob_effect.trial_omen",
        "minecraft:raid_omen" => "minecraft:event.mob_effect.raid_omen",
        _ => return None,
    })
}

fn play_sound_on_added(target: &Entity, store: &mut EntityStore, sound: Option<&str>) {
    if let Some(sound) = sound {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position: target.position(),
            resource_id: sound.to_owned(),
            source: super::living_sound_source(target),
            volume: 1.0,
            pitch: 1.0,
        });
    }
}

pub(crate) fn fixed_add_effect_rejection(entity_resource_id: &str) -> bool {
    matches!(
        entity_resource_id,
        "minecraft:wither" | "minecraft:ender_dragon"
    )
}

#[cfg(test)]
mod tests;
