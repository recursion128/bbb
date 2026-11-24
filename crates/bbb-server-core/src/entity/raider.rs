use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID, VANILLA_RAIDER_CELEBRATING_DATA_ID,
    VANILLA_SPELLCASTER_ILLAGER_SPELL_CASTING_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
    VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

pub(crate) mod hold_ground;

use crate::entity::ai::mob::target::TargetRef;
use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_block_pos, optional_bool, optional_int};
use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityData, EntityStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RaiderRaid {
    None,
    Unresolved(i32),
    Current(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaiderRaidJoin {
    pub raid_id: i32,
    pub wave: i32,
    pub patrol_leader: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RaiderRaidFacts {
    pub(crate) center: BlockPos,
    pub(crate) active: bool,
    pub(crate) is_over: bool,
    pub(crate) is_loss: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingRaiderRecruit {
    pub(crate) recruit: TargetRef,
    pub(crate) raid_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RaiderWaveFacts {
    pub(crate) active: bool,
    pub(crate) is_over: bool,
    pub(crate) leader: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingRaiderLeader {
    pub(crate) raider: TargetRef,
    pub(crate) raid_id: i32,
    pub(crate) wave: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RaiderRetainedState {
    patrol_target: Option<BlockPos>,
    patrol_leader: bool,
    patrolling: bool,
    wave: i32,
    can_join_raid: bool,
    raid: RaiderRaid,
    celebrating: bool,
    johnny: Option<bool>,
    ravager: Option<crate::entity::ravager::RavagerTicks>,
    /// `SpellcasterIllager.DATA_SPELL_CASTING_ID` (`SpellcasterIllager.java:23`),
    /// the evoker's and illusioner's current spell id.
    spell: Option<i8>,
    /// `SpellcasterIllager.spellCastingTickCount` (`SpellcasterIllager.java:24`),
    /// persisted as `SpellTicks` for evokers and illusioners.
    spell_ticks: Option<i32>,
    /// `Pillager.IS_CHARGING_CROSSBOW` (`Pillager.java:59`).
    charging_crossbow: Option<bool>,
}

impl RaiderRetainedState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if is_raider(entity_type_id) {
            let spell = spell_default(entity_type_id);
            Some(Self {
                patrol_target: None,
                patrol_leader: false,
                patrolling: false,
                wave: 0,
                can_join_raid: false,
                raid: RaiderRaid::None,
                celebrating: false,
                johnny: if entity_type_id == VANILLA_ENTITY_TYPE_VINDICATOR_ID {
                    Some(false)
                } else {
                    None
                },
                ravager: if entity_type_id == VANILLA_ENTITY_TYPE_RAVAGER_ID {
                    Some(crate::entity::ravager::RavagerTicks::DEFAULT)
                } else {
                    None
                },
                spell,
                spell_ticks: if spell.is_some() { Some(0) } else { None },
                charging_crossbow: charging_default(entity_type_id),
            })
        } else {
            None
        }
    }

    const fn from_saved(
        patrol_target: Option<BlockPos>,
        patrol_leader: bool,
        patrolling: bool,
        wave: i32,
        can_join_raid: bool,
        raid: RaiderRaid,
    ) -> Self {
        Self {
            patrol_target,
            patrol_leader,
            patrolling,
            wave,
            can_join_raid,
            raid,
            celebrating: false,
            johnny: None,
            ravager: None,
            spell: None,
            spell_ticks: None,
            charging_crossbow: None,
        }
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.celebrating = previous.celebrating;
        self.spell = previous.spell;
        self.charging_crossbow = previous.charging_crossbow;
    }

    pub(crate) const fn johnny(self) -> Option<bool> {
        self.johnny
    }

    pub(crate) fn mark_johnny(&mut self) {
        if let Some(johnny) = &mut self.johnny {
            *johnny = true;
        }
    }

    pub(crate) const fn spell_ticks(self) -> Option<i32> {
        self.spell_ticks
    }

    pub(crate) fn set_spell_ticks(&mut self, ticks: i32) {
        if let Some(spell_ticks) = &mut self.spell_ticks {
            *spell_ticks = ticks;
        }
    }

    pub(crate) fn owns_leaf_save_field(self, name: &str) -> bool {
        (self.johnny.is_some() && name == "Johnny")
            || (self.ravager.is_some() && crate::entity::ravager::owns_save_field(name))
            || (self.spell_ticks.is_some() && name == "SpellTicks")
    }

    pub(crate) const fn ravager(self) -> Option<crate::entity::ravager::RavagerTicks> {
        self.ravager
    }

    pub(crate) fn ravager_mut(&mut self) -> Option<&mut crate::entity::ravager::RavagerTicks> {
        self.ravager.as_mut()
    }

    pub(crate) const fn raid(self) -> RaiderRaid {
        self.raid
    }

    // Raider.java:168-170.
    pub(crate) const fn wave(self) -> i32 {
        self.wave
    }

    pub(crate) const fn patrolling(self) -> bool {
        self.patrolling
    }

    pub(crate) const fn patrol_leader(self) -> bool {
        self.patrol_leader
    }

    #[cfg(test)]
    pub(crate) const fn can_join_raid(self) -> bool {
        self.can_join_raid
    }
}

const fn spell_default(entity_type_id: i32) -> Option<i8> {
    if matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_EVOKER_ID | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
    ) {
        Some(0)
    } else {
        None
    }
}

const fn charging_default(entity_type_id: i32) -> Option<bool> {
    if entity_type_id == VANILLA_ENTITY_TYPE_PILLAGER_ID {
        Some(false)
    } else {
        None
    }
}

// Raider.java:70-73, SpellcasterIllager.java:33-36, Pillager.java:93-96.
pub(crate) fn synched_values(state: RaiderRetainedState) -> Vec<SynchedValue> {
    let mut rows = vec![SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_RAIDER_CELEBRATING_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(state.celebrating),
        },
        is_default: !state.celebrating,
    }];
    if let Some(spell) = state.spell {
        rows.push(SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_SPELLCASTER_ILLAGER_SPELL_CASTING_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(spell),
            },
            is_default: spell == 0,
        });
    }
    if let Some(charging) = state.charging_crossbow {
        rows.push(SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(charging),
            },
            is_default: !charging,
        });
    }
    rows
}

/// `SpellcasterIllager.setIsCastingSpell` (`SpellcasterIllager.java:63-66`).
pub(crate) fn set_spell_casting(entity: &mut Entity, spell: i8) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(current) = data
        .raider_retained_mut()
        .and_then(|state| state.spell.as_mut())
    else {
        return;
    };
    if *current != spell {
        *current = spell;
        entity.mark_synched_dirty(VANILLA_SPELLCASTER_ILLAGER_SPELL_CASTING_DATA_ID);
    }
}

/// `Pillager.setChargingCrossbow` (`Pillager.java:108-110`); false for a
/// mob that is no pillager.
pub(crate) fn set_charging_crossbow(entity: &mut Entity, charging: bool) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(current) = data
        .raider_retained_mut()
        .and_then(|state| state.charging_crossbow.as_mut())
    else {
        return false;
    };
    if *current != charging {
        *current = charging;
        entity.mark_synched_dirty(VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID);
    }
    true
}

pub(crate) fn apply_celebration(
    entity: &mut Entity,
    store: &mut EntityStore,
    change: Option<bool>,
    sound: bool,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.raider_retained_mut() else {
        return;
    };
    if let Some(celebrating) = change {
        if state.celebrating != celebrating {
            state.celebrating = celebrating;
            entity.mark_synched_dirty(VANILLA_RAIDER_CELEBRATING_DATA_ID);
        }
    }
    if !sound || entity.is_silent() {
        return;
    }
    let resource_id = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_EVOKER_ID => "minecraft:entity.evoker.celebrate",
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID => "minecraft:entity.illusioner.ambient",
        VANILLA_ENTITY_TYPE_PILLAGER_ID => "minecraft:entity.pillager.celebrate",
        VANILLA_ENTITY_TYPE_RAVAGER_ID => "minecraft:entity.ravager.celebrate",
        VANILLA_ENTITY_TYPE_VINDICATOR_ID => "minecraft:entity.vindicator.celebrate",
        VANILLA_ENTITY_TYPE_WITCH_ID => "minecraft:entity.witch.celebrate",
        _ => return,
    };
    let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let pitch = crate::entity::living::voice_pitch(store, baby);
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch,
    });
}

pub(crate) const fn is_raider(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_EVOKER_ID
            | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
            | VANILLA_ENTITY_TYPE_PILLAGER_ID
            | VANILLA_ENTITY_TYPE_RAVAGER_ID
            | VANILLA_ENTITY_TYPE_VINDICATOR_ID
            | VANILLA_ENTITY_TYPE_WITCH_ID
    )
}

/// `Raids.canJoinRaid` (`Raids.java`).
pub(crate) fn can_join_raid(entity: &crate::entity::Entity) -> bool {
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.raider_retained()
        .is_some_and(|state| state.can_join_raid)
        && entity.is_alive_with_health()
        && entity
            .living()
            .is_some_and(|living| living.no_action_time <= 2400)
}

// Raider.java:86-112.
pub(crate) fn ai_step(
    entity: &mut crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(state) = data.raider_retained() else {
        return;
    };
    if entity.is_alive_with_health() && state.can_join_raid {
        match state.raid() {
            RaiderRaid::None if level.game_time() % 20 == 0 && can_join_raid(entity) => {
                if let Some(raid_id) = level.raid_id_at(entity.block_position()) {
                    store.queue_command(crate::entity::EntityCommand::RaiderRecruit(
                        PendingRaiderRecruit {
                            recruit: TargetRef {
                                entity_id: entity.id(),
                                uuid: entity.uuid(),
                            },
                            raid_id,
                        },
                    ));
                }
            }
            RaiderRaid::Current(_) => {
                let resets_idle = store.mob_ai(entity.id()).is_some_and(|runtime| {
                    let view = runtime.target_view(entity, store, level);
                    runtime
                        .target_ref()
                        .and_then(|target| view.facts_of(target))
                        .is_some_and(|target| {
                            matches!(
                                target.entity_type_id(),
                                VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
                            ) && crate::entity::ai::mob::target::conditions::is_valid_target(
                                &view, &target,
                            )
                        })
                });
                if resets_idle {
                    entity.living_mut().unwrap().no_action_time = 0;
                }
            }
            RaiderRaid::None | RaiderRaid::Unresolved(_) => {}
        }
    }
    if let Some(living) = entity.living_mut() {
        living.no_action_time = living.no_action_time.wrapping_add(2);
    }
}

///
/// `None` is an entity that is not a raider — the `instanceof` the predicate
/// opens with — and is what tells the loot engine the term is `NotApplicable`
/// rather than unread.
pub(crate) fn is_captain(
    entity: &crate::entity::Entity,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<bool> {
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let state = data.raider_retained()?;
    if !state.patrol_leader() {
        return Some(false);
    }
    let head = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::Head));
    Some(head.is_some_and(|stack| {
        crate::entity::general_living::spawn_finalize_equipment::is_ominous_banner(stack, items)
    }))
}

///
/// This crate holds the entity, not the level's `Raids`, so the second
/// disjunct arrives as [`LevelCtx::raid_at`]'s answer for the
/// raider's own block position: `Some(false)` is the level's map answering
/// that no active raid covers the block, which is the `null` vanilla's
/// `getRaidAt` returned. The disjunction is read left to right, so a raider
/// whose saved id the level resolved into a current raid is `true` whatever
/// the map says.
///
/// Two states leave the term open and answer `None`, which the loot engine
/// counts as a refusal: a level that owns no raid map (the `instanceof
/// ServerLevel` gate's other side, which vanilla answers `false` for — the
/// crate cannot tell that apart from an unread map), and a saved raid id the
/// level has not resolved, whose `getCurrentRaid()` is not yet known to be
/// null.
pub(crate) fn has_raid(
    entity: &crate::entity::Entity,
    raid_at_position: Option<bool>,
) -> Option<bool> {
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    match data.raider_retained()?.raid() {
        RaiderRaid::Current(_) => Some(true),
        RaiderRaid::None => raid_at_position,
        RaiderRaid::Unresolved(_) => None,
    }
}

#[cfg(test)]
impl crate::entity::GeneralLivingData {
    pub(crate) fn set_raider_raid(&mut self, raid: RaiderRaid) {
        self.raider_retained_mut()
            .expect("a raider retains raid state")
            .raid = raid;
    }
}

pub fn prepare_patrol_leader(
    entity: &mut crate::entity::Entity,
    entity_random: &GameRandom,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_PILLAGER_ID {
        return false;
    }
    let pos = entity.block_position();
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.raider_retained_mut() else {
        return false;
    };
    state.patrol_leader = true;
    state.patrolling = true;
    state.patrol_target = Some(BlockPos {
        x: pos.x.wrapping_add(entity_random.next_int(1000) - 500),
        y: pos.y,
        z: pos.z.wrapping_add(entity_random.next_int(1000) - 500),
    });
    true
}

pub fn patrol_spawn_state(
    entity: &crate::entity::Entity,
) -> Option<(Option<BlockPos>, bool, bool)> {
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let state = data.raider_retained()?;
    Some((state.patrol_target, state.patrol_leader, state.patrolling))
}

pub(crate) fn apply_patrol_step(
    entity: &mut crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
    result: crate::entity::ai::mob::patrol::PatrolStepResult,
) {
    if let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.raider_retained_mut() {
            state.patrol_target = result.target;
            state.patrolling = result.patrolling;
        }
    }
    if let Some((companions, target)) = result.companion_target {
        for id in companions {
            let Some(companion) = store.get_mut(id) else {
                continue;
            };
            if let crate::entity::EntityData::GeneralLiving(data) = &mut companion.data {
                if let Some(state) = data.raider_retained_mut() {
                    state.patrol_target = Some(target);
                    state.patrolling = true;
                }
            }
        }
    }
}

/// `Raider` and `PatrollingMonster.finalizeSpawn` state writes; answers
/// whether the raider now leads a patrol.
pub(crate) fn finalize_spawn_state(
    entity: &mut crate::entity::Entity,
    can_join_raid: bool,
    become_leader: bool,
    patrolling: bool,
) -> Option<bool> {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    let state = data.raider_retained_mut()?;
    state.can_join_raid = can_join_raid;
    state.patrol_leader |= become_leader;
    state.patrolling |= patrolling;
    Some(state.patrol_leader)
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &RaiderRetainedState) {
    if let Some(target) = state.patrol_target {
        entries.push(NbtSummaryEntry {
            name: "patrol_target".into(),
            value: NbtSummaryValue::IntArray(vec![target.x, target.y, target.z]),
        });
    }
    entries.extend([
        boolean_entry("PatrolLeader", state.patrol_leader),
        boolean_entry("Patrolling", state.patrolling),
        NbtSummaryEntry {
            name: "Wave".into(),
            value: NbtSummaryValue::Int(state.wave),
        },
        boolean_entry("CanJoinRaid", state.can_join_raid),
    ]);
    if let RaiderRaid::Current(raid_id) = state.raid {
        entries.push(NbtSummaryEntry {
            name: "RaidId".into(),
            value: NbtSummaryValue::Int(raid_id),
        });
    }
    if state.johnny() == Some(true) {
        entries.push(boolean_entry("Johnny", true));
    }
    if let Some(ticks) = state.ravager {
        crate::entity::ravager::write_save_data(entries, ticks);
    }
    if let Some(ticks) = state.spell_ticks {
        entries.push(NbtSummaryEntry {
            name: "SpellTicks".into(),
            value: NbtSummaryValue::Int(ticks),
        });
    }
}

pub(crate) fn read_save_data(
    entity_type_id: i32,
    entries: &[NbtSummaryEntry],
) -> RaiderRetainedState {
    let mut state = RaiderRetainedState::from_saved(
        optional_block_pos(entries, "patrol_target"),
        optional_bool(entries, "PatrolLeader").unwrap_or(false),
        optional_bool(entries, "Patrolling").unwrap_or(false),
        optional_int(entries, "Wave").unwrap_or(0),
        optional_bool(entries, "CanJoinRaid").unwrap_or(false),
        optional_int(entries, "RaidId").map_or(RaiderRaid::None, RaiderRaid::Unresolved),
    );
    if entity_type_id == VANILLA_ENTITY_TYPE_VINDICATOR_ID {
        state.johnny = Some(optional_bool(entries, "Johnny").unwrap_or(false));
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_RAVAGER_ID {
        state.ravager = Some(crate::entity::ravager::read_save_data(entries));
    }
    state.spell = spell_default(entity_type_id);
    if state.spell.is_some() {
        state.spell_ticks = Some(optional_int(entries, "SpellTicks").unwrap_or(0));
    }
    state.charging_crossbow = charging_default(entity_type_id);
    state
}

pub fn resolve_raider_raid(
    entity: &mut crate::entity::Entity,
    resolve: impl FnOnce(i32) -> bool,
) -> Option<RaiderRaidJoin> {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    let state = data.raider_retained_mut()?;
    let RaiderRaid::Unresolved(raid_id) = state.raid else {
        return None;
    };
    if !resolve(raid_id) {
        state.raid = RaiderRaid::None;
        return None;
    }
    state.raid = RaiderRaid::Current(raid_id);
    Some(RaiderRaidJoin {
        raid_id,
        wave: state.wave,
        patrol_leader: state.patrol_leader,
    })
}

pub fn set_patrol_leader(entity: &mut crate::entity::Entity) -> bool {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.raider_retained_mut() else {
        return false;
    };
    state.patrol_leader = true;
    state.patrolling = true;
    true
}

pub fn join_raid(entity: &mut crate::entity::Entity, raid_id: i32, wave: i32) -> bool {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.raider_retained_mut() else {
        return false;
    };
    state.raid = RaiderRaid::Current(raid_id);
    state.wave = wave;
    state.can_join_raid = true;
    true
}

pub(crate) fn holds_current_raid(entity: &crate::entity::Entity) -> bool {
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.raider_retained()
        .is_some_and(|state| matches!(state.raid, RaiderRaid::Current(_)))
}

pub fn clear_raider_raid(entity: &mut crate::entity::Entity) -> bool {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.raider_retained_mut() else {
        return false;
    };
    state.raid = RaiderRaid::None;
    true
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(
        name,
        "patrol_target" | "PatrolLeader" | "Patrolling" | "Wave" | "CanJoinRaid" | "RaidId"
    )
}

fn boolean_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

#[cfg(test)]
mod tests;
