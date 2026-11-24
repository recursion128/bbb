//! `EnderDragon` (`EnderDragon.java`): its flight, parts, crystal healing,
//! part-scaled damage and the 200-tick death.

mod path;
mod phases;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{mth_cos, mth_sin, Aabb, Vec3Ops};
use uuid::Uuid;

use crate::entity::living::{self, SynchedValue};
use crate::entity::save::optional_int;
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore, RemovalReason};
use crate::level_event::{PendingHurtEntity, PendingPlayerHurt};
use crate::world::simulation_level::LevelCtx;

use path::{DragonNodes, DragonPath};
pub(crate) use phases::DragonPhase;

/// `EnderDragon.DATA_PHASE`, defined right after `Mob.DATA_MOB_FLAGS_ID`.
const PHASE_DATA_ID: u8 = 16;
/// `EnderDragon.subEntities` (`EnderDragon.java:91-99`).
pub(crate) const PART_COUNT: i32 = 8;
const MOB_ATTACK: &str = "minecraft:mob_attack";

/// One of `subEntities`, in the order the client numbers them after the
/// dragon's own id (`EnderDragon.recreateFromPacket`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragonPart {
    Head,
    Neck,
    Body,
    Tail1,
    Tail2,
    Tail3,
    Wing1,
    Wing2,
}

impl DragonPart {
    const ALL: [Self; 8] = [
        Self::Head,
        Self::Neck,
        Self::Body,
        Self::Tail1,
        Self::Tail2,
        Self::Tail3,
        Self::Wing1,
        Self::Wing2,
    ];

    /// The `EnderDragonPart` dimensions (`EnderDragon.java:91-98`).
    const fn size(self) -> (f32, f32) {
        match self {
            Self::Head => (1.0, 1.0),
            Self::Neck => (3.0, 3.0),
            Self::Body => (5.0, 3.0),
            Self::Tail1 | Self::Tail2 | Self::Tail3 => (2.0, 2.0),
            Self::Wing1 | Self::Wing2 => (4.0, 2.0),
        }
    }
}

/// `DragonFlightHistory` (`DragonFlightHistory.java`).
#[derive(Debug, Clone, PartialEq)]
struct FlightHistory {
    samples: [(f64, f32); 64],
    head: i32,
}

impl Default for FlightHistory {
    fn default() -> Self {
        Self {
            samples: [(0.0, 0.0); 64],
            head: -1,
        }
    }
}

impl FlightHistory {
    fn record(&mut self, y: f64, y_rot: f32) {
        if self.head < 0 {
            self.samples = [(y, y_rot); 64];
        }
        self.head = (self.head + 1) & 63;
        self.samples[self.head as usize] = (y, y_rot);
    }

    fn get(&self, delay: i32) -> (f64, f32) {
        self.samples[((self.head - delay) & 63) as usize]
    }
}

/// What a phase keeps between its `begin` calls (`DragonPhaseInstance`s are
/// cached per dragon, so only `begin` resets them).
#[derive(Debug, Clone, Default, PartialEq)]
struct PhaseScratch {
    target_location: Option<Vec3d>,
    path: Option<DragonPath>,
    ticks: i32,
    first_tick: bool,
    fireball_charge: i32,
    strafe_target: Option<Uuid>,
    flame_cloud: Option<i32>,
}

/// The fight this dragon belongs to, as `EnderDragonFight` last told it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FightLink {
    pub(crate) alive_crystals: i32,
    pub(crate) previously_killed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnderDragonState {
    phase: DragonPhase,
    death_time: i32,
    scratch: PhaseScratch,
    holding_clockwise: bool,
    strafe_clockwise: bool,
    flame_count: i32,
    history: FlightHistory,
    y_rot_a: f32,
    nearest_crystal: Option<i32>,
    sitting_damage: f32,
    in_wall: bool,
    flap_time: f32,
    o_flap_time: f32,
    parts: Option<Box<[Entity; 8]>>,
    nodes: Option<DragonNodes>,
    pub(crate) fight: Option<FightLink>,
    hit_part: Option<DragonPart>,
}

impl Default for EnderDragonState {
    /// The constructor's `EnderDragonPhaseManager` starts hovering
    /// (`EnderDragonPhaseManager.java:15-18`).
    fn default() -> Self {
        Self {
            phase: DragonPhase::Hover,
            death_time: 0,
            scratch: PhaseScratch::default(),
            holding_clockwise: false,
            strafe_clockwise: false,
            flame_count: 0,
            history: FlightHistory::default(),
            y_rot_a: 0.0,
            nearest_crystal: None,
            sitting_damage: 0.0,
            in_wall: false,
            flap_time: 0.0,
            o_flap_time: 0.0,
            parts: None,
            nodes: None,
            fight: None,
            hit_part: None,
        }
    }
}

impl EnderDragonState {
    fn part(&self, part: DragonPart) -> &Entity {
        &self.parts.as_ref().expect("admitted dragon parts")[part as usize]
    }

    fn part_mut(&mut self, part: DragonPart) -> &mut Entity {
        &mut self.parts.as_mut().expect("admitted dragon parts")[part as usize]
    }

    /// `EnderDragon.nearestCrystal`, the crystal healing it.
    pub(crate) const fn is_sitting(&self) -> bool {
        self.phase.is_sitting()
    }

    pub(crate) const fn nearest_crystal(&self) -> Option<i32> {
        self.nearest_crystal
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        let (phase, death_time) = (self.phase, self.death_time);
        *self = previous.clone();
        self.phase = phase;
        self.death_time = death_time;
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &EnderDragonState) {
    for (name, value) in [
        ("DragonPhase", state.phase.id()),
        ("DragonDeathTime", state.death_time),
    ] {
        entries.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Int(value),
        });
    }
}

pub(crate) fn read_save_data(state: &mut EnderDragonState, entries: &[NbtSummaryEntry]) {
    if let Some(id) = optional_int(entries, "DragonPhase") {
        state.phase = DragonPhase::from_id(id);
    }
    state.death_time = optional_int(entries, "DragonDeathTime").unwrap_or(0);
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "DragonPhase" | "DragonDeathTime")
}

pub(crate) fn synched_value(state: &EnderDragonState) -> SynchedValue {
    let id = state.phase.id();
    SynchedValue {
        value: bbb_protocol::packets::EntityDataValue {
            data_id: PHASE_DATA_ID,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: bbb_protocol::packets::EntityDataValueKind::Int(id),
        },
        is_default: id == DragonPhase::Hover.id(),
    }
}

fn state_mut(entity: &mut Entity) -> Option<&mut EnderDragonState> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data.ender_dragon_mut(),
        _ => None,
    }
}

pub(crate) fn state(entity: &Entity) -> Option<&EnderDragonState> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.ender_dragon(),
        _ => None,
    }
}

/// Runs `body` with the dragon state lifted out of the entity, so both can be
/// borrowed mutably.
fn with_state<R>(
    entity: &mut Entity,
    body: impl FnOnce(&mut EnderDragonState, &mut Entity) -> R,
) -> Option<R> {
    let mut state = std::mem::take(state_mut(entity)?);
    let result = body(&mut state, entity);
    *state_mut(entity).expect("the dragon state stays put") = state;
    Some(result)
}

/// `EnderDragonFight.createNewDragon` (`EnderDragonFight.java:464-478`): a
/// dragon in its holding pattern at the fight origin.
pub(crate) fn begin_holding_pattern(entity: &mut Entity) {
    with_state(entity, |state, entity| {
        phases::set_phase(state, entity, None, DragonPhase::HoldingPattern);
    });
}

pub(crate) fn set_fight(entity: &mut Entity, fight: Option<FightLink>) {
    if let Some(state) = state_mut(entity) {
        state.fight = fight;
    }
}

/// Which part an attack named, for the hurt that follows it.
pub(crate) fn set_hit_part(entity: &mut Entity, part: Option<DragonPart>) {
    if let Some(state) = state_mut(entity) {
        state.hit_part = part;
    }
}

/// Parts retain their own base Entity state, without independent store admission.
pub(crate) fn initialize_parts(entity: &mut Entity, uuids: [Uuid; 8]) {
    let parent_id = entity.id();
    let Some(state) = state_mut(entity) else {
        return;
    };
    let mut parts = Vec::with_capacity(DragonPart::ALL.len());
    for (index, (part, uuid)) in DragonPart::ALL.into_iter().zip(uuids).enumerate() {
        let mut body = Entity::new(
            parent_id + index as i32 + 1,
            uuid,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            Vec3d::default(),
            EntityData::EnderDragonPart,
        );
        let (width, height) = part.size();
        body.set_dimensions(width, height);
        parts.push(body);
    }
    state.parts = Some(
        parts
            .into_boxed_slice()
            .try_into()
            .expect("a dragon has eight parts"),
    );
}

fn part_with_parent(store: &EntityStore, part_id: i32) -> Option<(i32, DragonPart, &Entity)> {
    (1..=PART_COUNT).find_map(|offset| {
        let dragon = store.get(part_id.checked_sub(offset)?)?;
        let state = state(dragon)?;
        let part = DragonPart::ALL[(offset - 1) as usize];
        let body = state.parts.as_ref()?.get(part as usize)?;
        Some((dragon.id(), part, body))
    })
}

/// `ServerLevel.getEntityOrPart`: the dragon and part a part id names, with
/// the part's box.
pub(crate) fn part_by_id(store: &EntityStore, part_id: i32) -> Option<(i32, DragonPart, Aabb)> {
    let (dragon, part, body) = part_with_parent(store, part_id)?;
    Some((dragon, part, body.bounding_box()))
}

pub(crate) fn part_entity(store: &EntityStore, part_id: i32) -> Option<&Entity> {
    part_with_parent(store, part_id).map(|(_, _, body)| body)
}

pub(crate) fn part_position(store: &EntityStore, part_id: i32) -> Option<Vec3d> {
    part_entity(store, part_id).map(Entity::position)
}

pub(crate) fn projectile_parts(entity: &Entity) -> impl Iterator<Item = (i32, Aabb, Vec3d)> + '_ {
    state(entity)
        .and_then(|state| state.parts.as_deref())
        .into_iter()
        .flatten()
        .map(|part| (part.id(), part.bounding_box(), part.position()))
}

fn part_box(state: &EnderDragonState, part: DragonPart) -> Aabb {
    state.part(part).bounding_box()
}

/// `EnderDragon.hurt(level, part, source, damage)` (`EnderDragon.java:444-476`)
/// up to `reallyHurt`: `None` refuses the hurt.
pub(crate) fn hurt_damage(
    entity: &mut Entity,
    damage: f32,
    reaches_dragon: bool,
    direct_is_arrow_or_wind_charge: bool,
) -> Option<f32> {
    let state = state_mut(entity)?;
    let part = state.hit_part.take().unwrap_or(DragonPart::Body);
    if state.phase == DragonPhase::Dying {
        return None;
    }
    let mut damage = state.phase.on_hurt(damage, direct_is_arrow_or_wind_charge);
    if part != DragonPart::Head {
        damage = damage / 4.0 + damage.min(1.0);
    }
    (damage >= 0.01 && reaches_dragon).then_some(damage)
}

/// The rest of `EnderDragon.hurt` after `reallyHurt` (`EnderDragon.java:461-473`).
pub(crate) fn after_hurt(entity: &mut Entity, store: &mut EntityStore, health_before: f32) {
    let Some(living) = entity.living() else {
        return;
    };
    let (health, max_health) = (living.health(), living.max_health());
    with_state(entity, |state, entity| {
        if health <= 0.0 && !state.phase.is_sitting() {
            living::set_health(entity, 1.0);
            phases::set_phase(state, entity, Some(store), DragonPhase::Dying);
        }
        if state.phase.is_sitting() {
            state.sitting_damage += health_before - health;
            if state.sitting_damage > 0.25 * max_health {
                state.sitting_damage = 0.0;
                phases::set_phase(state, entity, Some(store), DragonPhase::Takeoff);
            }
        }
    });
}

/// The phase half of `EnderDragon.onCrystalDestroyed` (`EnderDragon.java:814`):
/// a holding dragon turns on the player blamed for the crystal.
pub(crate) fn crystal_destroyed(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    player: Option<Uuid>,
) {
    with_state(entity, |state, entity| {
        if let Some(player) = player.and_then(|uuid| level.player(uuid)) {
            if state.phase == DragonPhase::HoldingPattern && phases::can_attack(&player) {
                phases::strafe_player(state, entity, store, level, &player);
            }
        }
    });
}

/// `LivingEntity.tick` after `baseTick` for a dragon: `tickDeath` ran there
/// in vanilla, so a dying dragon only runs it here.
pub(crate) fn tick(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let dying = entity
        .living()
        .is_some_and(|living| living.is_dead_or_dying());
    with_state(entity, |state, entity| {
        // EnderDragon.java:122-126,146,172.
        let flap = |time: f32| mth_cos(f64::from(time * std::f32::consts::TAU));
        if flap(state.o_flap_time) <= -0.3 && flap(state.flap_time) >= -0.3 {
            level.game_event_at(
                crate::level_event::VanillaGameEvent::FLAP,
                entity.position(),
                Some(entity.id()),
            );
        }
        state.o_flap_time = state.flap_time;
        if dying {
            tick_death(state, entity, store, level);
        } else {
            ai_step(state, entity, store, level);
        }
    });
}

/// `EnderDragon.tickDeath` (`EnderDragon.java:505-553`).
fn tick_death(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    state.death_time += 1;
    let experience = match state.fight {
        Some(fight) if !fight.previously_killed => 12000,
        _ => 500,
    };
    let random = store.random().clone();
    if state.death_time > 150 && state.death_time % 5 == 0 && level.mob_drops() {
        crate::entity::experience_orb::award(
            store,
            &random,
            entity.position(),
            (experience as f32 * 0.08) as i32,
        );
    }
    if state.death_time == 1 && !entity.is_silent() {
        level.global_level_event(1028, entity.block_position(), 0);
    }
    let rise = Vec3d {
        x: 0.0,
        y: f64::from(0.1_f32),
        z: 0.0,
    };
    entity.set_pos(entity.position().plus(rise));
    for part in state
        .parts
        .as_mut()
        .expect("admitted dragon parts")
        .iter_mut()
    {
        part.set_pos(part.position().plus(rise));
    }
    if state.death_time >= 200 {
        if level.mob_drops() {
            crate::entity::experience_orb::award(
                store,
                &random,
                entity.position(),
                (experience as f32 * 0.2) as i32,
            );
        }
        store.queue_command(EntityCommand::DragonKilled(entity.uuid()));
        entity.set_removed(RemovalReason::Killed);
        // EnderDragon.java:551.
        level.game_event_at(
            crate::level_event::VanillaGameEvent::ENTITY_DIE,
            entity.position(),
            Some(entity.id()),
        );
    }
}

/// `EnderDragon.aiStep` on the server (`EnderDragon.java:145-335`).
fn ai_step(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    check_crystals(state, entity, store);
    // EnderDragon.java:178-193.
    let movement = entity.delta_movement();
    let flap_speed = 0.2_f32 / (movement.horizontal_distance_sqr().sqrt() as f32 * 10.0 + 1.0)
        * 2.0_f64.powf(movement.y) as f32;
    state.flap_time += if state.phase.is_sitting() {
        0.1
    } else if state.in_wall {
        flap_speed * 0.5
    } else {
        flap_speed
    };
    let no_ai = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai());
    if no_ai {
        state.flap_time = 0.5;
        return;
    }
    state.history.record(entity.position().y, entity.y_rot());
    let phase = state.phase;
    phases::server_tick(state, entity, store, level);
    if state.phase != phase {
        phases::server_tick(state, entity, store, level);
    }
    if let Some(target) = state.scratch.target_location {
        fly_towards(state, entity, target);
    }
    let y_rot = entity.y_rot();
    if let Some(living) = entity.living_mut() {
        living.y_body_rot = y_rot;
    }
    tick_parts(state, entity);
    if entity.living().is_some_and(|living| living.hurt_time == 0) {
        let body = part_box(state, DragonPart::Body).center();
        for wing in [DragonPart::Wing1, DragonPart::Wing2] {
            let area = part_box(state, wing).inflate(4.0, 2.0, 4.0).moved(Vec3d {
                x: 0.0,
                y: -2.0,
                z: 0.0,
            });
            knock_back(state, entity, store, level, area, body);
        }
        for part in [DragonPart::Head, DragonPart::Neck] {
            let area = part_box(state, part).inflate(1.0, 1.0, 1.0);
            hurt_inside(entity, store, level, area, 10.0);
        }
    }
    state.in_wall = [DragonPart::Head, DragonPart::Neck, DragonPart::Body]
        .into_iter()
        .fold(false, |hit, part| {
            check_walls(store, level, part_box(state, part)) | hit
        });
}

/// `EnderDragon.checkCrystals` (`EnderDragon.java:351-375`).
fn check_crystals(state: &mut EnderDragonState, entity: &mut Entity, store: &EntityStore) {
    if let Some(id) = state.nearest_crystal {
        if store.get(id).is_none_or(|crystal| !crystal.is_alive()) {
            state.nearest_crystal = None;
        } else if entity.tick_count() % 10 == 0 {
            if let Some(living) = entity.living() {
                let (health, max) = (living.health(), living.max_health());
                if health < max {
                    living::set_health(entity, health + 1.0);
                }
            }
        }
    }
    if store.random().next_int(10) == 0 {
        let position = entity.position();
        state.nearest_crystal = store
            .ids_in_box_sections(entity.bounding_box().inflate(32.0, 32.0, 32.0))
            .into_iter()
            .filter_map(|id| store.get(id))
            .filter(|crystal| {
                crystal.entity_type_id() == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
                    && crystal.is_alive()
                    && crystal
                        .bounding_box()
                        .intersects(entity.bounding_box().inflate(32.0, 32.0, 32.0))
            })
            .map(|crystal| (crystal.position().distance_to_sqr(position), crystal.id()))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id);
    }
}

/// The flight half of `aiStep` (`EnderDragon.java:204-243`): turn toward the
/// phase's target and move without collision.
fn fly_towards(state: &mut EnderDragonState, entity: &mut Entity, target: Vec3d) {
    let position = entity.position();
    let (xdd, zdd) = (target.x - position.x, target.z - position.z);
    let mut ydd = target.y - position.y;
    let dist_to_target = xdd * xdd + ydd * ydd + zdd * zdd;
    let max = f64::from(state.phase.fly_speed());
    let horizontal = (xdd * xdd + zdd * zdd).sqrt();
    if horizontal > 0.0 {
        ydd = (ydd / horizontal).clamp(-max, max);
    }
    let mut delta = entity.delta_movement();
    delta.y += ydd * 0.01;
    let mut y_rot = living::wrap_degrees(entity.y_rot());
    let aim = Vec3d {
        x: xdd,
        y: target.y - position.y,
        z: zdd,
    }
    .normalize();
    let radians = |degrees: f32| f64::from(degrees * (std::f32::consts::PI / 180.0));
    let dir = Vec3d {
        x: f64::from(mth_sin(radians(y_rot))),
        y: delta.y,
        z: f64::from(-mth_cos(radians(y_rot))),
    }
    .normalize();
    let alignment = ((dot(dir, aim) as f32 + 0.5) / 1.5).max(0.0);
    if xdd.abs() > 1.0e-5 || zdd.abs() > 1.0e-5 {
        let wanted = living::wrap_degrees(
            180.0 - (xdd.atan2(zdd) as f32) * (180.0 / std::f32::consts::PI) - y_rot,
        )
        .clamp(-50.0, 50.0);
        state.y_rot_a *= 0.8;
        state.y_rot_a += wanted * state.phase.turn_speed(delta);
        y_rot += state.y_rot_a * 0.1;
    }
    let span = (2.0 / (dist_to_target + 1.0)) as f32;
    let speed = f64::from(0.06 * (alignment * span + (1.0 - span)));
    delta.x += speed * f64::from(mth_sin(radians(y_rot)));
    delta.z -= speed * f64::from(mth_cos(radians(y_rot)));
    let step = if state.in_wall {
        delta.scale(f64::from(0.8_f32))
    } else {
        delta
    };
    entity.set_pos(position.plus(step));
    let actual = delta.normalize();
    let slide = 0.8 + 0.15 * (dot(actual, dir) + 1.0) / 2.0;
    entity.set_delta_movement(delta.multiply(slide, f64::from(0.91_f32), slide));
    entity.set_rot(y_rot, entity.x_rot());
}

fn dot(a: Vec3d, b: Vec3d) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// The `tickPart` calls of `aiStep` (`EnderDragon.java:266-313`).
fn tick_parts(state: &mut EnderDragonState, entity: &Entity) {
    let position = entity.position();
    let y_rot = entity.y_rot();
    let tilt = (state.history.get(5).0 - state.history.get(10).0) as f32
        * 10.0
        * (std::f32::consts::PI / 180.0);
    let (cc_tilt, ss_tilt) = (mth_cos(f64::from(tilt)), mth_sin(f64::from(tilt)));
    let rot1 = y_rot * (std::f32::consts::PI / 180.0);
    let (ss1, cc1) = (mth_sin(f64::from(rot1)), mth_cos(f64::from(rot1)));
    let at = |x: f32, y: f64, z: f32| Vec3d {
        x: position.x + f64::from(x),
        y: position.y + y,
        z: position.z + f64::from(z),
    };
    state
        .part_mut(DragonPart::Body)
        .set_pos(at(ss1 * 0.5, 0.0, -cc1 * 0.5));
    state
        .part_mut(DragonPart::Wing1)
        .set_pos(at(cc1 * 4.5, 2.0, ss1 * 4.5));
    state
        .part_mut(DragonPart::Wing2)
        .set_pos(at(cc1 * -4.5, 2.0, ss1 * -4.5));
    let head_rot = f64::from(rot1 - state.y_rot_a * 0.01);
    let (ss2, cc2) = (mth_sin(head_rot), mth_cos(head_rot));
    let y_offset = if state.phase.is_sitting() {
        -1.0
    } else {
        (state.history.get(5).0 - state.history.get(0).0) as f32
    };
    state.part_mut(DragonPart::Head).set_pos(at(
        ss2 * 6.5 * cc_tilt,
        f64::from(y_offset + ss_tilt * 6.5),
        -cc2 * 6.5 * cc_tilt,
    ));
    state.part_mut(DragonPart::Neck).set_pos(at(
        ss2 * 5.5 * cc_tilt,
        f64::from(y_offset + ss_tilt * 5.5),
        -cc2 * 5.5 * cc_tilt,
    ));
    let p1 = state.history.get(5);
    for (index, part) in [DragonPart::Tail1, DragonPart::Tail2, DragonPart::Tail3]
        .into_iter()
        .enumerate()
    {
        let p0 = state.history.get(12 + index as i32 * 2);
        let rot = rot1 + living::wrap_degrees(p0.1 - p1.1) * (std::f32::consts::PI / 180.0);
        let (ss, cc) = (mth_sin(f64::from(rot)), mth_cos(f64::from(rot)));
        let dd = (index + 1) as f32 * 2.0;
        state.part_mut(part).set_pos(at(
            -(ss1 * 1.5 + ss * dd) * cc_tilt,
            p0.0 - p1.0 - f64::from((dd + 1.5) * ss_tilt) + 1.5,
            (cc1 * 1.5 + cc * dd) * cc_tilt,
        ));
    }
}

/// `EnderDragon.knockBack` (`EnderDragon.java:377-394`) over
/// `EntitySelector.NO_CREATIVE_OR_SPECTATOR`.
fn knock_back(
    state: &EnderDragonState,
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    area: Aabb,
    body: Vec3d,
) {
    let push = |position: Vec3d| {
        let (xd, zd) = (position.x - body.x, position.z - body.z);
        let dd = (xd * xd + zd * zd).max(0.1);
        Vec3d {
            x: xd / dd * 4.0,
            y: f64::from(0.2_f32),
            z: zd / dd * 4.0,
        }
    };
    let hurts = !state.phase.is_sitting();
    let mut hit_players: Vec<(i32, Uuid, Vec3d)> = Vec::new();
    for player in level.players() {
        if player.spectator || player.instabuild || player.dead_or_dying {
            continue;
        }
        let bounds = Aabb::of_entity(player.position, player.dimensions[0], player.dimensions[1]);
        if bounds.intersects(area) {
            hit_players.push((player.entity_id, player.uuid, player.position));
        }
    }
    for (entity_id, uuid, position) in hit_players {
        if let Some(state) = store.player_mut(entity_id) {
            state.push_impulse(push(position));
        }
        if hurts {
            hurt_player(entity, level, uuid, 5.0);
        }
    }
    for id in living_inside(entity, store, area) {
        if let Some(target) = store.get_mut(id) {
            let delta = target.delta_movement().plus(push(target.position()));
            target.set_delta_movement(delta);
            target.mark_hurt();
        }
        if hurts {
            hurt_mob(entity, store, level, id, 5.0);
        }
    }
}

/// `EnderDragon.hurt(level, entities)` (`EnderDragon.java:396-404`).
fn hurt_inside(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    area: Aabb,
    amount: f32,
) {
    let mut hit_players: Vec<Uuid> = Vec::new();
    for player in level.players() {
        if player.spectator || player.instabuild || player.dead_or_dying {
            continue;
        }
        let bounds = Aabb::of_entity(player.position, player.dimensions[0], player.dimensions[1]);
        if bounds.intersects(area) {
            hit_players.push(player.uuid);
        }
    }
    for uuid in hit_players {
        hurt_player(entity, level, uuid, amount);
    }
    for id in living_inside(entity, store, area) {
        hurt_mob(entity, store, level, id, amount);
    }
}

fn living_inside(entity: &Entity, store: &EntityStore, area: Aabb) -> Vec<i32> {
    store
        .ids_in_box_sections(area)
        .into_iter()
        .filter(|&id| {
            id != entity.id()
                && store.get(id).is_some_and(|target| {
                    target.living().is_some()
                        && target.is_alive()
                        && target.bounding_box().intersects(area)
                })
        })
        .collect()
}

fn hurt_player(entity: &Entity, level: &mut LevelCtx<'_, '_>, player: Uuid, amount: f32) {
    let attacker = PendingHurtEntity::Stored {
        entity_id: entity.id(),
    };
    level.hurt_player(PendingPlayerHurt {
        player,
        damage_type: MOB_ATTACK,
        amount,
        causing: Some(attacker.clone()),
        direct: Some(attacker),
        post_attack: true,
        impulse: None,
    });
}

fn hurt_mob(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target: i32,
    amount: f32,
) {
    let hurt = crate::entity::ai::mob::brain_runtime::hurt_entity_from_body(
        entity, store, level, target, MOB_ATTACK, amount,
    );
    if hurt == Some(true) {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.post_attack(crate::level_event::PendingPostAttack {
            victim_entity_id: target,
            damage_type: MOB_ATTACK,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
        });
    }
}

/// `EnderDragon.checkWalls` (`EnderDragon.java:410-442`): the blocks a part
/// overlaps break under mob griefing, otherwise they slow the dragon.
fn check_walls(store: &EntityStore, level: &mut LevelCtx<'_, '_>, bounds: Aabb) -> bool {
    let floor = |value: f64| value.floor() as i32;
    let (x0, y0, z0) = (
        floor(bounds.min_x),
        floor(bounds.min_y),
        floor(bounds.min_z),
    );
    let (x1, y1, z1) = (
        floor(bounds.max_x),
        floor(bounds.max_y),
        floor(bounds.max_z),
    );
    let griefing = level.mob_griefing();
    let (mut hit_wall, mut destroyed) = (false, false);
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                let pos = BlockPos { x, y, z };
                let state = level.block_state(pos);
                if block_state_is_air(state)
                    || level.block_state_in_tag(state, "minecraft:dragon_transparent")
                {
                    continue;
                }
                if griefing && !level.block_state_in_tag(state, "minecraft:dragon_immune") {
                    destroyed = level.remove_block(pos) || destroyed;
                } else {
                    hit_wall = true;
                }
            }
        }
    }
    if destroyed {
        let random = store.random();
        let pos = BlockPos {
            x: x0 + random.next_int(x1 - x0 + 1),
            y: y0 + random.next_int(y1 - y0 + 1),
            z: z0 + random.next_int(z1 - z0 + 1),
        };
        level.level_event(2008, pos, 0);
    }
    hit_wall
}
