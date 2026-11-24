//! Vibration values, listener admission and arrival propagation.

use crate::level_event::VanillaGameEvent;
use crate::world::simulation_tick::vibration::WorldVibrationReads;
use bbb_protocol::debug_subscription_types::VANILLA_DEBUG_GAME_EVENT_TYPES;
use bbb_protocol::nbt_summary::uuid_nbt as uuid_value;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::game_event::{block_pos_center, containing_block_pos, GameEventContext};
use super::value_input::{float_or, int_or, numeric_long};
use super::{entry, unique_field};

/// `VibrationSystem.NO_VIBRATION_FREQUENCY` (`VibrationSystem.java:52`).
pub const NO_VIBRATION_FREQUENCY: i32 = 0;

/// `VibrationSystem.Data.NBT_TAG_KEY` (`:136`).
pub const TAG_LISTENER: &str = "listener";
/// `VibrationSystem.Data.CODEC`'s three fields (`:123-135`).
const TAG_EVENT: &str = "event";
const TAG_SELECTOR: &str = "selector";
const TAG_EVENT_DELAY: &str = "event_delay";
/// `VibrationSelector.CODEC`'s two fields (`VibrationSelector.java:9-15`).
const TAG_SELECTOR_EVENT: &str = "event";
const TAG_SELECTOR_TICK: &str = "tick";
/// `VibrationInfo.CODEC`'s five fields (`VibrationInfo.java:19-30`).
const TAG_GAME_EVENT: &str = "game_event";
const TAG_DISTANCE: &str = "distance";
const TAG_POS: &str = "pos";
const TAG_SOURCE: &str = "source";
const TAG_PROJECTILE_OWNER: &str = "projectile_owner";

/// `BlockTags.DAMPENS_VIBRATIONS` (`BlockTags.java:222`).
pub const DAMPENS_VIBRATIONS_TAG: &str = "minecraft:dampens_vibrations";
/// `BlockTags.OCCLUDES_VIBRATION_SIGNALS` (`BlockTags.java:223`).
pub const OCCLUDES_VIBRATION_SIGNALS_TAG: &str = "minecraft:occludes_vibration_signals";

const fn const_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn const_vibration_frequency_by_resource_id(resource_id: &str) -> i32 {
    if const_str_eq(resource_id, "minecraft:resonate_1") {
        return 1;
    }
    if const_str_eq(resource_id, "minecraft:resonate_2") {
        return 2;
    }
    if const_str_eq(resource_id, "minecraft:resonate_3") {
        return 3;
    }
    if const_str_eq(resource_id, "minecraft:resonate_4") {
        return 4;
    }
    if const_str_eq(resource_id, "minecraft:resonate_5") {
        return 5;
    }
    if const_str_eq(resource_id, "minecraft:resonate_6") {
        return 6;
    }
    if const_str_eq(resource_id, "minecraft:resonate_7") {
        return 7;
    }
    if const_str_eq(resource_id, "minecraft:resonate_8") {
        return 8;
    }
    if const_str_eq(resource_id, "minecraft:resonate_9") {
        return 9;
    }
    if const_str_eq(resource_id, "minecraft:resonate_10") {
        return 10;
    }
    if const_str_eq(resource_id, "minecraft:resonate_11") {
        return 11;
    }
    if const_str_eq(resource_id, "minecraft:resonate_12") {
        return 12;
    }
    if const_str_eq(resource_id, "minecraft:resonate_13") {
        return 13;
    }
    if const_str_eq(resource_id, "minecraft:resonate_14") {
        return 14;
    }
    if const_str_eq(resource_id, "minecraft:resonate_15") {
        return 15;
    }
    if const_str_eq(resource_id, "minecraft:step")
        || const_str_eq(resource_id, "minecraft:swim")
        || const_str_eq(resource_id, "minecraft:flap")
    {
        return 1;
    }
    if const_str_eq(resource_id, "minecraft:projectile_land")
        || const_str_eq(resource_id, "minecraft:hit_ground")
        || const_str_eq(resource_id, "minecraft:splash")
    {
        return 2;
    }
    if const_str_eq(resource_id, "minecraft:item_interact_finish")
        || const_str_eq(resource_id, "minecraft:projectile_shoot")
        || const_str_eq(resource_id, "minecraft:instrument_play")
    {
        return 3;
    }
    if const_str_eq(resource_id, "minecraft:entity_action")
        || const_str_eq(resource_id, "minecraft:elytra_glide")
        || const_str_eq(resource_id, "minecraft:unequip")
    {
        return 4;
    }
    if const_str_eq(resource_id, "minecraft:entity_dismount")
        || const_str_eq(resource_id, "minecraft:equip")
    {
        return 5;
    }
    if const_str_eq(resource_id, "minecraft:entity_interact")
        || const_str_eq(resource_id, "minecraft:shear")
        || const_str_eq(resource_id, "minecraft:entity_mount")
    {
        return 6;
    }
    if const_str_eq(resource_id, "minecraft:entity_damage") {
        return 7;
    }
    if const_str_eq(resource_id, "minecraft:drink") || const_str_eq(resource_id, "minecraft:eat") {
        return 8;
    }
    if const_str_eq(resource_id, "minecraft:container_close")
        || const_str_eq(resource_id, "minecraft:block_close")
        || const_str_eq(resource_id, "minecraft:block_deactivate")
        || const_str_eq(resource_id, "minecraft:block_detach")
    {
        return 9;
    }
    if const_str_eq(resource_id, "minecraft:container_open")
        || const_str_eq(resource_id, "minecraft:block_open")
        || const_str_eq(resource_id, "minecraft:block_activate")
        || const_str_eq(resource_id, "minecraft:block_attach")
        || const_str_eq(resource_id, "minecraft:prime_fuse")
        || const_str_eq(resource_id, "minecraft:note_block_play")
    {
        return 10;
    }
    if const_str_eq(resource_id, "minecraft:block_change") {
        return 11;
    }
    if const_str_eq(resource_id, "minecraft:block_destroy")
        || const_str_eq(resource_id, "minecraft:fluid_pickup")
    {
        return 12;
    }
    if const_str_eq(resource_id, "minecraft:block_place")
        || const_str_eq(resource_id, "minecraft:fluid_place")
    {
        return 13;
    }
    if const_str_eq(resource_id, "minecraft:entity_place")
        || const_str_eq(resource_id, "minecraft:lightning_strike")
        || const_str_eq(resource_id, "minecraft:teleport")
    {
        return 14;
    }
    if const_str_eq(resource_id, "minecraft:entity_die")
        || const_str_eq(resource_id, "minecraft:explode")
    {
        return 15;
    }
    NO_VIBRATION_FREQUENCY
}

const VIBRATION_FREQUENCIES: [i32; VANILLA_DEBUG_GAME_EVENT_TYPES.len()] = {
    let mut table = [NO_VIBRATION_FREQUENCY; VANILLA_DEBUG_GAME_EVENT_TYPES.len()];
    let mut index = 0;
    while index < VANILLA_DEBUG_GAME_EVENT_TYPES.len() {
        table[index] = const_vibration_frequency_by_resource_id(
            VANILLA_DEBUG_GAME_EVENT_TYPES[index].resource_id,
        );
        index += 1;
    }
    table
};

/// `VibrationSystem.VIBRATION_FREQUENCY_FOR_EVENT` (`:53-99`) indexed directly
/// by the event's compile-time registry holder id.
#[must_use]
pub const fn vibration_frequency(event: VanillaGameEvent) -> i32 {
    VIBRATION_FREQUENCIES[event.holder_id() as usize]
}

/// `VibrationSystem.VIBRATION_FREQUENCY_FOR_EVENT` (`:53-99`), keyed by the
/// event's resource id.
#[must_use]
pub fn vibration_frequency_for_event(resource_id: &str) -> i32 {
    VanillaGameEvent::from_resource_id(resource_id)
        .map_or(NO_VIBRATION_FREQUENCY, vibration_frequency)
}

#[must_use]
pub fn get_redstone_strength_for_distance(distance: f32, listener_radius: i32) -> i32 {
    let power_scale = 15.0 / f64::from(listener_radius);
    (15 - (power_scale * f64::from(distance)).floor() as i32).max(1)
}

const fn build_tag_mask(members: &[&str]) -> u64 {
    assert!(VANILLA_DEBUG_GAME_EVENT_TYPES.len() <= 64);
    let mut mask = 0_u64;
    let mut index = 0;
    while index < members.len() {
        let holder_id = VanillaGameEvent::by_resource(members[index]).holder_id();
        mask |= 1_u64 << holder_id;
        index += 1;
    }
    mask
}

const VIBRATIONS_MASK: u64 = build_tag_mask(VIBRATIONS);
const WARDEN_CAN_LISTEN_MASK: u64 = build_tag_mask(WARDEN_CAN_LISTEN);
const SHRIEKER_CAN_LISTEN_MASK: u64 = build_tag_mask(SHRIEKER_CAN_LISTEN);
const IGNORE_VIBRATIONS_SNEAKING_MASK: u64 = build_tag_mask(IGNORE_VIBRATIONS_SNEAKING);
const ALLAY_CAN_LISTEN_MASK: u64 = build_tag_mask(ALLAY_CAN_LISTEN);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventTag {
    Vibrations,
    WardenCanListen,
    ShriekerCanListen,
    IgnoreVibrationsSneaking,
    AllayCanListen,
}

impl GameEventTag {
    #[must_use]
    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::Vibrations => "minecraft:vibrations",
            Self::WardenCanListen => "minecraft:warden_can_listen",
            Self::ShriekerCanListen => "minecraft:shrieker_can_listen",
            Self::IgnoreVibrationsSneaking => "minecraft:ignore_vibrations_sneaking",
            Self::AllayCanListen => "minecraft:allay_can_listen",
        }
    }

    const fn mask(self) -> u64 {
        match self {
            Self::Vibrations => VIBRATIONS_MASK,
            Self::WardenCanListen => WARDEN_CAN_LISTEN_MASK,
            Self::ShriekerCanListen => SHRIEKER_CAN_LISTEN_MASK,
            Self::IgnoreVibrationsSneaking => IGNORE_VIBRATIONS_SNEAKING_MASK,
            Self::AllayCanListen => ALLAY_CAN_LISTEN_MASK,
        }
    }

    #[must_use]
    pub const fn contains_event(self, event: VanillaGameEvent) -> bool {
        (self.mask() & (1_u64 << event.holder_id())) != 0
    }

    #[cfg(test)]
    #[must_use]
    pub fn contains(self, event_resource_id: &str) -> bool {
        VanillaGameEvent::from_resource_id(event_resource_id)
            .is_some_and(|event| self.contains_event(event))
    }
}

/// `data/minecraft/tags/game_event/vibrations.json`.
const VIBRATIONS: &[&str] = &[
    "minecraft:block_attach",
    "minecraft:block_change",
    "minecraft:block_close",
    "minecraft:block_destroy",
    "minecraft:block_detach",
    "minecraft:block_open",
    "minecraft:block_place",
    "minecraft:block_activate",
    "minecraft:block_deactivate",
    "minecraft:container_close",
    "minecraft:container_open",
    "minecraft:drink",
    "minecraft:eat",
    "minecraft:elytra_glide",
    "minecraft:entity_damage",
    "minecraft:entity_die",
    "minecraft:entity_dismount",
    "minecraft:entity_interact",
    "minecraft:entity_mount",
    "minecraft:entity_place",
    "minecraft:entity_action",
    "minecraft:equip",
    "minecraft:explode",
    "minecraft:fluid_pickup",
    "minecraft:fluid_place",
    "minecraft:hit_ground",
    "minecraft:instrument_play",
    "minecraft:item_interact_finish",
    "minecraft:lightning_strike",
    "minecraft:note_block_play",
    "minecraft:prime_fuse",
    "minecraft:projectile_land",
    "minecraft:projectile_shoot",
    "minecraft:shear",
    "minecraft:splash",
    "minecraft:step",
    "minecraft:swim",
    "minecraft:teleport",
    "minecraft:unequip",
    "minecraft:resonate_1",
    "minecraft:resonate_2",
    "minecraft:resonate_3",
    "minecraft:resonate_4",
    "minecraft:resonate_5",
    "minecraft:resonate_6",
    "minecraft:resonate_7",
    "minecraft:resonate_8",
    "minecraft:resonate_9",
    "minecraft:resonate_10",
    "minecraft:resonate_11",
    "minecraft:resonate_12",
    "minecraft:resonate_13",
    "minecraft:resonate_14",
    "minecraft:resonate_15",
    "minecraft:flap",
];

/// `data/minecraft/tags/game_event/warden_can_listen.json`, which is
/// `#minecraft:vibrations` plus `minecraft:shriek` and
/// `#minecraft:shrieker_can_listen`.
const WARDEN_CAN_LISTEN: &[&str] = &[
    "minecraft:block_attach",
    "minecraft:block_change",
    "minecraft:block_close",
    "minecraft:block_destroy",
    "minecraft:block_detach",
    "minecraft:block_open",
    "minecraft:block_place",
    "minecraft:block_activate",
    "minecraft:block_deactivate",
    "minecraft:container_close",
    "minecraft:container_open",
    "minecraft:drink",
    "minecraft:eat",
    "minecraft:elytra_glide",
    "minecraft:entity_damage",
    "minecraft:entity_die",
    "minecraft:entity_dismount",
    "minecraft:entity_interact",
    "minecraft:entity_mount",
    "minecraft:entity_place",
    "minecraft:entity_action",
    "minecraft:equip",
    "minecraft:explode",
    "minecraft:fluid_pickup",
    "minecraft:fluid_place",
    "minecraft:hit_ground",
    "minecraft:instrument_play",
    "minecraft:item_interact_finish",
    "minecraft:lightning_strike",
    "minecraft:note_block_play",
    "minecraft:prime_fuse",
    "minecraft:projectile_land",
    "minecraft:projectile_shoot",
    "minecraft:shear",
    "minecraft:splash",
    "minecraft:step",
    "minecraft:swim",
    "minecraft:teleport",
    "minecraft:unequip",
    "minecraft:resonate_1",
    "minecraft:resonate_2",
    "minecraft:resonate_3",
    "minecraft:resonate_4",
    "minecraft:resonate_5",
    "minecraft:resonate_6",
    "minecraft:resonate_7",
    "minecraft:resonate_8",
    "minecraft:resonate_9",
    "minecraft:resonate_10",
    "minecraft:resonate_11",
    "minecraft:resonate_12",
    "minecraft:resonate_13",
    "minecraft:resonate_14",
    "minecraft:resonate_15",
    "minecraft:shriek",
    "minecraft:sculk_sensor_tendrils_clicking",
];

/// `data/minecraft/tags/game_event/shrieker_can_listen.json`.
const SHRIEKER_CAN_LISTEN: &[&str] = &["minecraft:sculk_sensor_tendrils_clicking"];

/// `data/minecraft/tags/game_event/ignore_vibrations_sneaking.json`.
const IGNORE_VIBRATIONS_SNEAKING: &[&str] = &[
    "minecraft:hit_ground",
    "minecraft:projectile_shoot",
    "minecraft:step",
    "minecraft:swim",
    "minecraft:item_interact_start",
    "minecraft:item_interact_finish",
];

/// `data/minecraft/tags/game_event/allay_can_listen.json`.
const ALLAY_CAN_LISTEN: &[&str] = &["minecraft:note_block_play"];

#[must_use]
pub fn catalog_game_event_resource_id(resource_id: &str) -> Option<&'static str> {
    VANILLA_DEBUG_GAME_EVENT_TYPES
        .iter()
        .find(|entry| entry.resource_id == resource_id)
        .map(|entry| entry.resource_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VibrationEntity {
    pub uuid: Uuid,
    /// `Entity instanceof ServerPlayer`.
    pub is_player: bool,
    pub is_projectile: bool,
    /// `Entity.getControllingPassenger()` when that passenger is a player.
    pub controlling_passenger_player: Option<Uuid>,
    /// `Projectile.getOwner()`.
    pub projectile_owner: Option<Uuid>,
    /// `ItemEntity.getOwner()`.
    pub item_owner: Option<Uuid>,
}

impl VibrationEntity {
    /// `SculkShriekerBlockEntity.tryGetPlayer` (`SculkShriekerBlockEntity.java:88-98`):
    /// only online `ServerPlayer`s (directly, as controlling passenger, or as a
    /// projectile/item owner) resolve to a shrieker target.
    #[must_use]
    pub fn shriek_player_with(
        self,
        mut is_online_player: impl FnMut(Uuid) -> bool,
    ) -> Option<Uuid> {
        if self.is_player {
            return Some(self.uuid);
        }
        self.controlling_passenger_player
            .or_else(|| {
                self.is_projectile
                    .then_some(self.projectile_owner)
                    .flatten()
                    .filter(|&uuid| is_online_player(uuid))
            })
            .or_else(|| self.item_owner.filter(|&uuid| is_online_player(uuid)))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VibrationSourceFacts {
    /// `Entity.isSpectator()`.
    pub spectator: bool,
    /// `Entity.isSteppingCarefully()`.
    pub stepping_carefully: bool,
    /// `Entity.dampensVibrations()`.
    pub dampens_vibrations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VibrationInfo {
    /// `game_event`, by `minecraft:game_event` resource id.
    pub game_event: &'static str,
    pub distance: f32,
    pub pos: Vec3d,
    /// `source`.
    pub source: Option<Uuid>,
    /// `projectile_owner`.
    pub projectile_owner: Option<Uuid>,
}

impl VibrationInfo {
    pub(crate) fn entity(
        &self,
        mut resolve: impl FnMut(Uuid) -> Option<VibrationEntity>,
    ) -> Option<VibrationEntity> {
        self.source.and_then(&mut resolve)
    }

    pub(crate) fn projectile_owner(
        &self,
        source_entity: Option<VibrationEntity>,
        mut resolve: impl FnMut(Uuid) -> Option<VibrationEntity>,
    ) -> Option<VibrationEntity> {
        source_entity
            .filter(|entity| entity.is_projectile)
            .and_then(|entity| entity.projectile_owner)
            .or(self.projectile_owner)
            .and_then(&mut resolve)
    }

    fn save(&self) -> NbtSummaryValue {
        let mut entries = vec![
            entry(TAG_GAME_EVENT, NbtSummaryValue::string(self.game_event)),
            entry(
                TAG_DISTANCE,
                NbtSummaryValue::Float(self.distance.to_bits()),
            ),
            entry(
                TAG_POS,
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Double(self.pos.x.to_bits()),
                    NbtSummaryValue::Double(self.pos.y.to_bits()),
                    NbtSummaryValue::Double(self.pos.z.to_bits()),
                ]),
            ),
        ];
        if let Some(source) = self.source {
            entries.push(entry(TAG_SOURCE, uuid_value(source)));
        }
        if let Some(projectile_owner) = self.projectile_owner {
            entries.push(entry(TAG_PROJECTILE_OWNER, uuid_value(projectile_owner)));
        }
        NbtSummaryValue::Compound(entries)
    }

    fn load(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        let game_event = match unique_field(entries, TAG_GAME_EVENT) {
            Some(NbtSummaryValue::String(resource_id)) => resource_id
                .with_utf8(catalog_game_event_resource_id)
                .flatten()?,
            _ => return None,
        };
        let distance = float_or(entries, TAG_DISTANCE, 0.0);
        if !(0.0..=f32::MAX).contains(&distance) {
            return None;
        }
        let Some(NbtSummaryValue::List(coordinates)) = unique_field(entries, TAG_POS) else {
            return None;
        };
        if coordinates.len() != 3 {
            return None;
        }
        let mut components = [0.0_f64; 3];
        for (index, coordinate) in coordinates.iter().enumerate() {
            match coordinate {
                NbtSummaryValue::Double(bits) => components[index] = f64::from_bits(*bits),
                _ => return None,
            }
        }
        Some(Self {
            game_event,
            distance,
            pos: Vec3d {
                x: components[0],
                y: components[1],
                z: components[2],
            },
            source: unique_field(entries, TAG_SOURCE).and_then(super::codec_uuid),
            projectile_owner: unique_field(entries, TAG_PROJECTILE_OWNER)
                .and_then(super::codec_uuid),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VibrationSelector {
    current_vibration_data: Option<(VibrationInfo, i64)>,
}

impl VibrationSelector {
    pub fn add_candidate(&mut self, new_vibration: VibrationInfo, tick_time: i64) {
        if self.should_replace_vibration(&new_vibration, tick_time) {
            self.current_vibration_data = Some((new_vibration, tick_time));
        }
    }

    fn should_replace_vibration(&self, new_vibration: &VibrationInfo, tick_time: i64) -> bool {
        let Some((previous, previous_tick)) = &self.current_vibration_data else {
            return true;
        };
        if tick_time != *previous_tick {
            return false;
        }
        match new_vibration.distance.partial_cmp(&previous.distance) {
            Some(std::cmp::Ordering::Less) => true,
            Some(std::cmp::Ordering::Greater) => false,
            _ => {
                vibration_frequency_for_event(new_vibration.game_event)
                    > vibration_frequency_for_event(previous.game_event)
            }
        }
    }

    #[must_use]
    pub fn chosen_candidate(&self, time: i64) -> Option<VibrationInfo> {
        let (vibration, tick) = self.current_vibration_data.as_ref()?;
        (*tick < time).then_some(*vibration)
    }

    pub fn start_over(&mut self) {
        self.current_vibration_data = None;
    }

    #[must_use]
    pub fn current(&self) -> Option<(VibrationInfo, i64)> {
        self.current_vibration_data
    }

    fn save(&self) -> NbtSummaryValue {
        let mut entries = Vec::new();
        if let Some((vibration, _)) = &self.current_vibration_data {
            entries.push(entry(TAG_SELECTOR_EVENT, vibration.save()));
        }
        entries.push(entry(
            TAG_SELECTOR_TICK,
            NbtSummaryValue::Long(
                self.current_vibration_data
                    .as_ref()
                    .map_or(-1, |(_, tick)| *tick),
            ),
        ));
        NbtSummaryValue::Compound(entries)
    }

    fn load(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        let tick = unique_field(entries, TAG_SELECTOR_TICK).and_then(numeric_long)?;
        let vibration = unique_field(entries, TAG_SELECTOR_EVENT).and_then(VibrationInfo::load);
        Some(Self {
            current_vibration_data: vibration.map(|vibration| (vibration, tick)),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VibrationData {
    current_vibration: Option<VibrationInfo>,
    travel_time_in_ticks: i32,
    selection_strategy: VibrationSelector,
    reload_vibration_particle: bool,
}

impl VibrationData {
    #[must_use]
    pub const fn selection_strategy(&self) -> &VibrationSelector {
        &self.selection_strategy
    }

    pub fn selection_strategy_mut(&mut self) -> &mut VibrationSelector {
        &mut self.selection_strategy
    }

    #[must_use]
    pub const fn current_vibration(&self) -> Option<&VibrationInfo> {
        self.current_vibration.as_ref()
    }

    pub fn set_current_vibration(&mut self, current_vibration: Option<VibrationInfo>) {
        self.current_vibration = current_vibration;
    }

    #[must_use]
    pub const fn travel_time_in_ticks(&self) -> i32 {
        self.travel_time_in_ticks
    }

    pub fn set_travel_time_in_ticks(&mut self, travel_time_in_ticks: i32) {
        self.travel_time_in_ticks = travel_time_in_ticks;
    }

    pub fn decrement_travel_time(&mut self) {
        self.travel_time_in_ticks = (self.travel_time_in_ticks - 1).max(0);
    }

    #[must_use]
    pub const fn should_reload_vibration_particle(&self) -> bool {
        self.reload_vibration_particle
    }

    pub fn set_reload_vibration_particle(&mut self, reload_vibration_particle: bool) {
        self.reload_vibration_particle = reload_vibration_particle;
    }

    #[must_use]
    pub fn save(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        if let Some(vibration) = &self.current_vibration {
            entries.push(entry(TAG_EVENT, vibration.save()));
        }
        entries.push(entry(TAG_SELECTOR, self.selection_strategy.save()));
        entries.push(entry(
            TAG_EVENT_DELAY,
            NbtSummaryValue::Int(self.travel_time_in_ticks),
        ));
        entries
    }

    #[must_use]
    pub fn load(entries: &[NbtSummaryEntry]) -> Option<Self> {
        let selection_strategy = VibrationSelector::load(unique_field(entries, TAG_SELECTOR)?)?;
        let travel_time_in_ticks = int_or(entries, TAG_EVENT_DELAY, 0);
        if travel_time_in_ticks < 0 {
            return None;
        }
        Some(Self {
            current_vibration: unique_field(entries, TAG_EVENT).and_then(VibrationInfo::load),
            travel_time_in_ticks,
            selection_strategy,
            // `Ticker.tryReloadVibrationParticle` is its only writer.
            reload_vibration_particle: true,
        })
    }
}

pub(crate) trait VibrationSystem {
    fn vibration_data(&self) -> &VibrationData;
    fn vibration_data_mut(&mut self) -> &mut VibrationData;

    fn vibration_listener_position(&self) -> Option<Vec3d>;

    fn vibration_listenable_events(&self) -> GameEventTag {
        GameEventTag::Vibrations
    }

    fn can_trigger_avoid_vibration(&self) -> bool {
        false
    }

    fn can_receive_vibration(
        &self,
        level: &mut WorldVibrationReads<'_>,
        pos: BlockPos,
        event: VanillaGameEvent,
        context: GameEventContext,
    ) -> bool;

    fn is_valid_vibration(
        &self,
        level: &mut WorldVibrationReads<'_>,
        event: VanillaGameEvent,
        context: GameEventContext,
    ) -> bool {
        if !level.game_event_in_tag(event, self.vibration_listenable_events()) {
            return false;
        }
        if let Some(facts) = context
            .source_entity_id
            .and_then(|entity_id| level.source_entity_facts(entity_id))
        {
            if facts.spectator {
                return false;
            }
            if facts.stepping_carefully
                && level.game_event_in_tag(event, GameEventTag::IgnoreVibrationsSneaking)
            {
                // VibrationSystem.java:415-420.
                if self.can_trigger_avoid_vibration() {
                    if let Some(entity_id) = context.source_entity_id {
                        level.trigger_avoid_vibration(entity_id);
                    }
                }
                return false;
            }
            if facts.dampens_vibrations {
                return false;
            }
        }
        context
            .affected_state
            .is_none_or(|state| !level.block_state_in_tag(state, DAMPENS_VIBRATIONS_TAG))
    }
}

pub(crate) fn listener_handle_game_event(
    system: &mut dyn VibrationSystem,
    level: &mut WorldVibrationReads<'_>,
    event: VanillaGameEvent,
    context: GameEventContext,
    source_position: Vec3d,
) -> bool {
    if system.vibration_data().current_vibration().is_some() {
        return false;
    }
    if !system.is_valid_vibration(level, event, context) {
        return false;
    }
    let Some(destination) = system.vibration_listener_position() else {
        return false;
    };
    if !system.can_receive_vibration(level, containing_block_pos(source_position), event, context) {
        return false;
    }
    if is_occluded(level, source_position, destination) {
        return false;
    }
    let source_entity = context
        .source_entity_id
        .and_then(|id| level.entity_identity(id));
    schedule_vibration(
        system.vibration_data_mut(),
        level.game_time(),
        event.resource_id(),
        source_entity,
        source_position,
        destination,
    );
    true
}

pub(crate) fn schedule_vibration(
    data: &mut VibrationData,
    game_time: i64,
    event_resource_id: &'static str,
    source_entity: Option<VibrationEntity>,
    origin: Vec3d,
    destination: Vec3d,
) {
    let vibration = VibrationInfo {
        game_event: event_resource_id,
        distance: distance(origin, destination) as f32,
        pos: origin,
        source: source_entity.map(|entity| entity.uuid),
        projectile_owner: source_entity.and_then(|entity| entity.projectile_owner),
    };
    data.selection_strategy_mut()
        .add_candidate(vibration, game_time);
}

pub(crate) struct VibrationArrival {
    pub info: VibrationInfo,
    pub distance: f32,
}

pub(crate) struct VibrationParticle {
    pub origin: Vec3d,
    pub travel_time_in_ticks: i32,
}

pub(crate) struct VibrationTick {
    pub arrival: Option<VibrationArrival>,
    pub particles: [Option<VibrationParticle>; 2],
    pub changed: bool,
}

impl VibrationData {
    #[must_use]
    pub(crate) fn has_pending_tick(&self, game_time: i64) -> bool {
        self.current_vibration().is_some()
            || self
                .selection_strategy()
                .chosen_candidate(game_time)
                .is_some()
    }

    pub(crate) fn tick(
        &mut self,
        game_time: i64,
        listener_position: Option<Vec3d>,
        arrival_ready: bool,
    ) -> VibrationTick {
        let mut tick = VibrationTick {
            arrival: None,
            particles: [None, None],
            changed: false,
        };
        if self.current_vibration().is_none() {
            if let Some(chosen) = self.selection_strategy().chosen_candidate(game_time) {
                let travel_time_in_ticks = chosen.distance.floor() as i32;
                self.set_current_vibration(Some(chosen));
                self.set_travel_time_in_ticks(travel_time_in_ticks);
                tick.particles[0] = Some(VibrationParticle {
                    origin: chosen.pos,
                    travel_time_in_ticks,
                });
                tick.changed = true;
                self.selection_strategy_mut().start_over();
            }
        }
        let Some(vibration) = self.current_vibration().copied() else {
            return tick;
        };
        let had_travel_time = self.travel_time_in_ticks() > 0;
        if self.should_reload_vibration_particle() {
            if let Some(destination) = listener_position {
                let initial_travel_time = vibration.distance.floor() as i32;
                if initial_travel_time > 0 {
                    let alpha = 1.0
                        - f64::from(self.travel_time_in_ticks()) / f64::from(initial_travel_time);
                    tick.particles[1] = Some(VibrationParticle {
                        origin: Vec3d {
                            x: lerp(alpha, vibration.pos.x, destination.x),
                            y: lerp(alpha, vibration.pos.y, destination.y),
                            z: lerp(alpha, vibration.pos.z, destination.z),
                        },
                        travel_time_in_ticks: self.travel_time_in_ticks(),
                    });
                    self.set_reload_vibration_particle(false);
                }
            }
        }
        self.decrement_travel_time();
        if self.travel_time_in_ticks() <= 0 {
            if arrival_ready {
                let origin = containing_block_pos(vibration.pos);
                let destination = listener_position.map_or(origin, containing_block_pos);
                tick.arrival = Some(VibrationArrival {
                    info: vibration,
                    distance: distance_between_in_blocks(origin, destination),
                });
                self.set_current_vibration(None);
                tick.changed = true;
            }
        } else {
            tick.changed |= had_travel_time;
        }
        tick
    }
}

#[must_use]
pub fn distance_between_in_blocks(origin: BlockPos, destination: BlockPos) -> f32 {
    let dx = f64::from(origin.x - destination.x);
    let dy = f64::from(origin.y - destination.y);
    let dz = f64::from(origin.z - destination.z);
    (dx * dx + dy * dy + dz * dz).sqrt() as f32
}

#[must_use]
pub(crate) fn is_occluded(
    level: &WorldVibrationReads<'_>,
    origin: Vec3d,
    destination: Vec3d,
) -> bool {
    let from = Vec3d {
        x: origin.x.floor() + 0.5,
        y: origin.y.floor() + 0.5,
        z: origin.z.floor() + 0.5,
    };
    let to = Vec3d {
        x: destination.x.floor() + 0.5,
        y: destination.y.floor() + 0.5,
        z: destination.z.floor() + 0.5,
    };
    for (step_x, step_y, step_z) in DIRECTIONS {
        let nudged = Vec3d {
            x: from.x + f64::from(step_x) * 1.0e-5,
            y: from.y + f64::from(step_y) * 1.0e-5,
            z: from.z + f64::from(step_z) * 1.0e-5,
        };
        if !block_in_line(level, nudged, to) {
            return false;
        }
    }
    true
}

/// `Direction.values()` (`Direction.java:8-16`) in declaration order.
const DIRECTIONS: [(i32, i32, i32); 6] = [
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];

fn block_in_line(level: &WorldVibrationReads<'_>, from: Vec3d, to: Vec3d) -> bool {
    let mut hit = false;
    traverse_blocks(from, to, |pos| {
        hit = level
            .block_state(pos)
            .is_some_and(|state| level.block_state_in_tag(state, OCCLUDES_VIBRATION_SIGNALS_TAG));
        hit
    });
    hit
}

fn traverse_blocks(from: Vec3d, to: Vec3d, mut visit: impl FnMut(BlockPos) -> bool) {
    if from == to {
        return;
    }
    let to_x = lerp(-1.0e-7, to.x, from.x);
    let to_y = lerp(-1.0e-7, to.y, from.y);
    let to_z = lerp(-1.0e-7, to.z, from.z);
    let from_x = lerp(-1.0e-7, from.x, to.x);
    let from_y = lerp(-1.0e-7, from.y, to.y);
    let from_z = lerp(-1.0e-7, from.z, to.z);
    let mut current = BlockPos {
        x: from_x.floor() as i32,
        y: from_y.floor() as i32,
        z: from_z.floor() as i32,
    };
    if visit(current) {
        return;
    }
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let dz = to_z - from_z;
    let sign_x = sign(dx);
    let sign_y = sign(dy);
    let sign_z = sign(dz);
    let t_delta_x = if sign_x == 0 {
        f64::MAX
    } else {
        f64::from(sign_x) / dx
    };
    let t_delta_y = if sign_y == 0 {
        f64::MAX
    } else {
        f64::from(sign_y) / dy
    };
    let t_delta_z = if sign_z == 0 {
        f64::MAX
    } else {
        f64::from(sign_z) / dz
    };
    let mut t_x = t_delta_x
        * if sign_x > 0 {
            1.0 - frac(from_x)
        } else {
            frac(from_x)
        };
    let mut t_y = t_delta_y
        * if sign_y > 0 {
            1.0 - frac(from_y)
        } else {
            frac(from_y)
        };
    let mut t_z = t_delta_z
        * if sign_z > 0 {
            1.0 - frac(from_z)
        } else {
            frac(from_z)
        };
    while t_x <= 1.0 || t_y <= 1.0 || t_z <= 1.0 {
        if t_x < t_y {
            if t_x < t_z {
                current.x += sign_x;
                t_x += t_delta_x;
            } else {
                current.z += sign_z;
                t_z += t_delta_z;
            }
        } else if t_y < t_z {
            current.y += sign_y;
            t_y += t_delta_y;
        } else {
            current.z += sign_z;
            t_z += t_delta_z;
        }
        if visit(current) {
            return;
        }
    }
}

fn lerp(alpha: f64, p0: f64, p1: f64) -> f64 {
    p0 + alpha * (p1 - p0)
}

fn frac(value: f64) -> f64 {
    value - value.floor()
}

fn sign(value: f64) -> i32 {
    if value == 0.0 {
        0
    } else if value > 0.0 {
        1
    } else {
        -1
    }
}

/// `Vec3.distanceTo`, which `VibrationInfo.distance` stores as a float.
fn distance(left: Vec3d, right: Vec3d) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[must_use]
pub fn block_position_source(pos: BlockPos) -> Vec3d {
    block_pos_center(pos)
}

/// The built-in `minecraft:block` tag lookup, for a caller without a registry
/// generation: the two block tags the vibration path reads.
#[must_use]
pub fn builtin_block_state_in_tag(block_state_id: i32, tag_id: &str) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id).is_some_and(
        |block_type_id| {
            bbb_server_types::fluid::block_type_ids_in_tag(tag_id).contains(&block_type_id)
        },
    )
}

#[cfg(test)]
mod tests;
