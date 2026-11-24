use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_SITTING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_CAMEL_DASH_DATA_ID, VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::{no_block_collision_with_moving_pistons, Aabb};

use crate::entity::living::SynchedValue;
use crate::entity::save::optional_long;
use crate::entity::{Entity, EntityData, EntityRegistryTagView, EntityStore, PendingEntitySound};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CamelState {
    last_pose_change_tick: i64,
    dashing: bool,
    dash_cooldown: u8,
}

impl CamelState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
            || entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        {
            Some(Self::from_saved(0))
        } else {
            None
        }
    }

    const fn from_saved(last_pose_change_tick: i64) -> Self {
        Self {
            last_pose_change_tick,
            dashing: false,
            dash_cooldown: 0,
        }
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.dashing = previous.dashing;
        self.dash_cooldown = previous.dash_cooldown;
    }

    fn is_sitting(self) -> bool {
        self.last_pose_change_tick < 0
    }

    // Camel.java:572-575; Camel.java:622-624.
    fn pose_time(self, game_time: i64) -> i64 {
        game_time.wrapping_sub(self.last_pose_change_tick.wrapping_abs())
    }

    fn transition_duration(self) -> i64 {
        if self.is_sitting() {
            40
        } else {
            52
        }
    }

    fn in_transition(self, game_time: i64) -> bool {
        self.pose_time(game_time) < self.transition_duration()
    }

    fn full_stand_tick(game_time: i64) -> i64 {
        game_time.saturating_sub(53).max(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CamelPoseChange {
    Sit,
    Stand,
}

fn state(entity: &Entity) -> Option<CamelState> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.camel_state()
}

// CamelAi.java:114-137.
pub(crate) fn idle_pose_change(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<CamelPoseChange> {
    let state = state(entity)?;
    if entity.is_in_water()
        || state.pose_time(level.game_time()) < 400
        || super::leash::is_leashed(entity)
        || !entity.on_ground()
        || super::passenger::direct_controlling_living_with_registry_tags(
            entity,
            Some(store),
            &level.capture_registry_tags(),
            |id| level.player_by_entity_id(id),
        )?
        .is_some()
        || !can_change_pose(entity, state, level)?
    {
        return None;
    }
    Some(if state.is_sitting() {
        CamelPoseChange::Stand
    } else {
        CamelPoseChange::Sit
    })
}

// Camel.java:410-412; LivingEntity.java:3680-3682.
fn can_change_pose(entity: &Entity, state: CamelState, level: &LevelCtx<'_, '_>) -> Option<bool> {
    let target_pose = if state.is_sitting() {
        VANILLA_ENTITY_POSE_STANDING_ID
    } else {
        VANILLA_ENTITY_POSE_SITTING_ID
    };
    let [width, height, _] = super::general_living::dimensions::geometry(
        entity.data(),
        entity.entity_type_id(),
        target_pose,
    )?;
    let bounds = Aabb::of_entity(entity.position(), width, height);
    no_block_collision_with_moving_pistons(
        bounds,
        |pos| {
            level.block_state_checked(pos)?;
            level.is_moving_piston_at(pos)
        },
        |pos| {
            level.block_state_checked(pos)?;
            level.block_collision_boxes_for_entity(entity, pos)
        },
    )
}

// Camel.java:583-599; CamelHusk.java:115-122.
pub(crate) fn apply_pose_change(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    change: CamelPoseChange,
) -> Option<()> {
    let state = state(entity)?;
    let sitting = change == CamelPoseChange::Sit;
    if state.is_sitting() == sitting {
        return Some(());
    }
    let resource_id = match (entity.entity_type_id(), change) {
        (VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, CamelPoseChange::Sit) => {
            "minecraft:entity.camel_husk.sit"
        }
        (VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, CamelPoseChange::Stand) => {
            "minecraft:entity.camel_husk.stand"
        }
        (_, CamelPoseChange::Sit) => "minecraft:entity.camel.sit",
        (_, CamelPoseChange::Stand) => "minecraft:entity.camel.stand",
    };
    play_action_sound(entity, store, resource_id)?;
    entity.set_pose_id_with_level(
        if sitting {
            VANILLA_ENTITY_POSE_SITTING_ID
        } else {
            VANILLA_ENTITY_POSE_STANDING_ID
        },
        level,
        Some(store),
    )?;
    level.game_event_at(
        crate::level_event::VanillaGameEvent::ENTITY_ACTION,
        entity.position(),
        Some(entity.id()),
    );
    let game_time = level.game_time();
    set_pose_tick(
        entity,
        if sitting {
            game_time.wrapping_neg()
        } else {
            game_time
        },
    );
    Some(())
}

// Camel.java:398-403; Camel.java:694-700.
pub(crate) fn stand_up_if_ready(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let Some(state) = state(entity) else {
        return Some(());
    };
    if state.is_sitting()
        && !state.in_transition(level.game_time())
        && can_change_pose(entity, state, level)?
    {
        apply_pose_change(entity, store, level, CamelPoseChange::Stand)?;
    }
    Some(())
}

// Camel.java:199-201.
pub(crate) fn tick_pose(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    if state(entity).is_some_and(CamelState::is_sitting) && entity.is_in_water() {
        stand_up_instantly(entity, store, level)?;
    }
    Some(())
}

// Camel.java:260-261.
pub(crate) fn refuses_to_move(entity: &Entity, game_time: i64) -> bool {
    state(entity).is_some_and(|state| state.is_sitting() || state.in_transition(game_time))
}

// Camel.java:243-261.
pub(crate) fn apply_travel_gate(entity: &mut Entity, game_time: i64, input: &mut Vec3d) {
    if entity.on_ground() && refuses_to_move(entity, game_time) {
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: entity.delta_movement().y,
            z: 0.0,
        });
        input.x = 0.0;
        input.z = 0.0;
    }
}

pub(crate) fn finalize_spawn_pose(entity: &mut Entity, game_time: i64) -> bool {
    if state(entity).is_none() {
        return false;
    }
    set_pose_tick(entity, CamelState::full_stand_tick(game_time));
    true
}

// `Camel.standUpInstantly` (`Camel.java:607-619`).
pub(crate) fn stand_up_instantly(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    entity.set_pose_id_with_level(VANILLA_ENTITY_POSE_STANDING_ID, level, Some(store))?;
    level.game_event_at(
        crate::level_event::VanillaGameEvent::ENTITY_ACTION,
        entity.position(),
        Some(entity.id()),
    );
    set_pose_tick(entity, CamelState::full_stand_tick(level.game_time()));
    Some(())
}

pub(crate) fn passenger_attachment_height_and_scale(
    entity: &Entity,
    front: bool,
    game_time: i64,
) -> Option<(f32, f32)> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let pose = data.camel_state()?;
    let age_scale = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
        1.0_f32
    } else if data.ageable_retained()?.is_baby() {
        0.6_f32
    } else {
        1.0_f32
    };
    let living_scale = entity.living()?.attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
    let scale = living_scale * age_scale;
    if entity.removal_reason().is_some() {
        return Some((0.01, scale));
    }
    let [_, standing_height] =
        bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity.entity_type_id())?;
    let dimensions_height = match entity.pose_id() {
        VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID => standing_height * scale,
        VANILLA_ENTITY_POSE_SITTING_ID => (standing_height - 1.43) * scale,
        VANILLA_ENTITY_POSE_SLEEPING_ID => entity.bb_height(),
        _ => return None,
    };

    let mut height = dimensions_height - 0.375 * scale;
    let sitting_height_difference = scale * 1.43;
    let vertical_drop = sitting_height_difference - scale * 0.2;
    let bottom_point = sitting_height_difference - vertical_drop;
    let sitting = pose.is_sitting();
    let pose_time = pose.pose_time(game_time);
    let duration = pose.transition_duration();
    let in_transition = pose.in_transition(game_time);
    if in_transition {
        let (half_point, flex_point_offset) = if sitting {
            (28_i64, if front { 0.5 } else { 0.1 })
        } else if front {
            (24_i64, 0.6)
        } else {
            (32_i64, 0.35)
        };
        let pose_time = pose_time.clamp(0, duration) as f32;
        let first_part = pose_time < half_point as f32;
        let part = if first_part {
            pose_time / half_point as f32
        } else {
            (pose_time - half_point as f32) / (duration - half_point) as f32
        };
        let flex_point = sitting_height_difference - flex_point_offset * vertical_drop;
        let (start, end) = if sitting {
            if first_part {
                (sitting_height_difference, flex_point)
            } else {
                (flex_point, bottom_point)
            }
        } else if first_part {
            (
                bottom_point - sitting_height_difference,
                bottom_point - flex_point,
            )
        } else {
            (bottom_point - flex_point, 0.0)
        };
        height += start + part * (end - start);
    } else if sitting {
        height += bottom_point;
    }
    Some((height, scale))
}

fn state_mut(entity: &mut Entity) -> Option<&mut CamelState> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    data.horse_state_mut()?.camel.as_mut()
}

fn set_pose_tick(entity: &mut Entity, tick: i64) -> bool {
    let Some(current) = state_mut(entity) else {
        return false;
    };
    if current.last_pose_change_tick == tick {
        return false;
    }
    current.last_pose_change_tick = tick;
    entity.mark_synched_dirty(VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID);
    true
}

// Camel.java:316-318; Camel.java:636-639.
fn set_dashing(entity: &mut Entity, dashing: bool) {
    let Some(state) = state_mut(entity) else {
        return;
    };
    if state.dashing == dashing {
        return;
    }
    state.dashing = dashing;
    if state.dash_cooldown == 0 {
        state.dash_cooldown = 55;
    }
    entity.mark_synched_dirty(VANILLA_CAMEL_DASH_DATA_ID);
}

// Camel.java:281-283; Camel.java:321-329; CamelHusk.java:100-102.
pub(crate) fn start_jump(
    entity: &mut Entity,
    store: &mut EntityStore,
    tags: &EntityRegistryTagView,
    game_time: i64,
) -> bool {
    if state(entity).is_none()
        || refuses_to_move(entity, game_time)
        || super::living::valid_equipment::has_valid_item_with_registry_tags(
            entity,
            tags,
            EquipmentSlot::Saddle,
        ) != Some(true)
    {
        return false;
    }
    let sound = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
        "minecraft:entity.camel_husk.dash"
    } else {
        "minecraft:entity.camel.dash"
    };
    if play_action_sound(entity, store, sound).is_none() {
        return false;
    }
    set_dashing(entity, true);
    true
}

// Camel.java:178-189; Camel.java:331-333; CamelHusk.java:105-107.
pub(crate) fn tick_dash(entity: &mut Entity, store: &mut EntityStore) {
    let Some(camel) = state(entity) else {
        return;
    };
    if camel.dashing
        && camel.dash_cooldown < 50
        && (entity.on_ground()
            || entity.is_in_water()
            || entity.is_in_lava()
            || store.vehicle(entity.id()).is_some())
    {
        set_dashing(entity, false);
    }
    let Some(state) = state_mut(entity) else {
        return;
    };
    if state.dash_cooldown == 0 {
        return;
    }
    state.dash_cooldown -= 1;
    if state.dash_cooldown == 0 {
        let resource_id = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
            "minecraft:entity.camel_husk.dash_ready"
        } else {
            "minecraft:entity.camel.dash_ready"
        };
        // Level.java:389-393.
        let pos = entity.block_position();
        store.play_entity_sound(PendingEntitySound {
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            resource_id: resource_id.to_owned(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
}

// AbstractHorse.java:397-399; LivingEntity.java:1406-1409.
fn play_action_sound(entity: &Entity, store: &mut EntityStore, resource_id: &str) -> Option<()> {
    if entity.is_silent() {
        return Some(());
    }
    let bbb_facts::entity_ambient_sounds::VanillaMobSpawnAmbientSound::Fixed { volume, .. } =
        bbb_facts::entity_ambient_sounds::vanilla_mob_spawn_ambient_sound(
            entity.data().entity_resource_id(),
        )?
    else {
        return None;
    };
    let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby())
        && entity.entity_type_id() != VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID;
    let pitch = super::living::voice_pitch(store, baby);
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_owned(),
        source: super::living::living_sound_source(entity),
        volume,
        pitch,
    });
    Some(())
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    name == "LastPoseTick"
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: CamelState) {
    entries.push(long_entry("LastPoseTick", state.last_pose_change_tick));
}

pub(crate) fn read_save_data(
    entity_type_id: i32,
    entries: &[NbtSummaryEntry],
) -> Option<CamelState> {
    CamelState::new(entity_type_id)?;
    Some(CamelState::from_saved(
        optional_long(entries, "LastPoseTick").unwrap_or(0),
    ))
}

pub(crate) fn synched_values(state: CamelState) -> [SynchedValue; 2] {
    [
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_CAMEL_DASH_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(state.dashing),
            },
            is_default: !state.dashing,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(state.last_pose_change_tick),
            },
            is_default: state.last_pose_change_tick == 0,
        },
    ]
}

pub(crate) fn finish_load(entity: &mut Entity, level: Option<&LevelCtx<'_, '_>>) {
    let sitting = match entity.data() {
        EntityData::GeneralLiving(data) => data.camel_state().is_some_and(CamelState::is_sitting),
        _ => false,
    };
    if sitting {
        let _ = entity.set_pose_id_with_context(VANILLA_ENTITY_POSE_SITTING_ID, level, None);
    }
}

fn long_entry(name: &str, value: i64) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Long(value),
    }
}

#[cfg(test)]
#[path = "camel/tests.rs"]
mod tests;
