//!
//!
//! An online player ticks outside the entity loop, so its own push step
//! ([`push_entities_from_session_player`]) runs ahead of the stored bodies';
//! both write the impulses straight onto the sessions they push.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_PARROT_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{Aabb, Vec3Ops};
use uuid::Uuid;

use crate::entity::store::{section_of_block, section_walk_order_key, EntityStore};
use crate::entity::Entity;

///
/// Every other entity runs the write its own override never blocks.
pub fn push(entity: &mut Entity, xa: f64, ya: f64, za: f64) {
    // `Creaking.java:193`, `AbstractArrow.java:412-416`, `AbstractWindCharge.java:96`.
    let refuses = match entity.data() {
        crate::entity::EntityData::Arrow(data) => data.in_ground(),
        crate::entity::EntityData::SpectralArrow(data) => data.in_ground(),
        crate::entity::EntityData::Trident(data) => data.in_ground(),
        crate::entity::EntityData::WindCharge(_)
        | crate::entity::EntityData::BreezeWindCharge(_) => true,
        _ => !entity.creaking_can_move(),
    };
    if refuses {
        return;
    }
    if !xa.is_finite() || !ya.is_finite() || !za.is_finite() {
        return;
    }
    entity.set_delta_movement(entity.delta_movement().plus(Vec3d {
        x: xa,
        y: ya,
        z: za,
    }));
    entity.set_needs_sync(true);
}

///
/// `None` when the level cannot answer `onClimbable`, which refuses the pair
/// instead of pushing against a guess. A stored entity is never a spectator,
/// so that term of `LivingEntity.isPushable` is the `false` no stored entity
/// carries.
fn is_pushable(entity: &mut Entity, level: &LevelCtx<'_, '_>) -> Option<bool> {
    match pushable_short_of_climbing(entity) {
        Some(decided) => Some(decided),
        None => Some(!crate::entity::living::on_climbable(entity, level)?),
    }
}

/// `isPushable` up to a living body's `onClimbable` term: `None` when only
/// that term is left to decide.
pub(crate) fn pushable_short_of_climbing(entity: &Entity) -> Option<bool> {
    // `AbstractMinecart.java:168-170`, `AbstractBoat.java:123-125`,
    // `Parrot.java:385-387`, `AbstractHorse.java:254-256` (its `!isVehicle()`
    // is read where the store is), `ArmorStand.java:168-170`, `Bat.java:86-88`.
    if crate::entity::minecart::is_minecart(entity.data())
        || crate::entity::boat::is_boat(entity.data())
        || entity.entity_type_id() == VANILLA_ENTITY_TYPE_PARROT_ID
        || crate::entity::general_living::is_abstract_horse_entity_type(entity.entity_type_id())
    {
        return Some(true);
    }
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_BAT_ID
        || matches!(entity.data(), crate::entity::EntityData::ArmorStand(_))
    {
        return Some(false);
    }
    if !entity.is_alive()
        // Warden.java:518-519.
        || crate::entity::warden::is_digging_or_emerging(entity.entity_type_id(), entity.pose_id())
        || entity
            .living()
            .is_none_or(crate::entity::living::LivingEntityState::is_dead_or_dying)
        || !entity.creaking_can_move()
    {
        return Some(false);
    }
    None
}

/// An online player's body as `LivingEntity.pushEntities` reads it from
/// either side of a pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerPushBody {
    pub entity_id: i32,
    pub uuid: Uuid,
    pub position: Vec3d,
    pub dimensions: [f32; 2],
    pub spectator: bool,
    pub sleeping: bool,
    /// `LivingEntity.isDeadOrDying()`, the `isAlive` term of `isPushable`.
    pub dead_or_dying: bool,
    pub flying: bool,
    pub fall_flying: bool,
}

fn session_is_pushable(
    session: &PlayerPushBody,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<bool> {
    if session.dead_or_dying || session.spectator {
        return Some(false);
    }
    if session.flying {
        return Some(true);
    }
    let pos = block_position(session.position);
    let climbable = crate::entity::living::on_climbable_at(level, pos, session.fall_flying)?;
    if climbable {
        if let Some(player) = store.player_mut(session.entity_id) {
            player.set_last_climbable_pos(pos);
        }
    }
    Some(!climbable)
}

fn block_position(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn session_box(session: &PlayerPushBody) -> Aabb {
    Aabb::of_entity(
        session.position,
        session.dimensions[0],
        session.dimensions[1],
    )
}

///
/// Both float literals widen to double before they meet the double operand.
pub(crate) fn pair_impulse(this_position: Vec3d, entity_position: Vec3d) -> Option<(f64, f64)> {
    let mut xa = entity_position.x - this_position.x;
    let mut za = entity_position.z - this_position.z;
    // `Mth.absMax(xa, za)` (`:1787`), the larger magnitude of the two.
    let mut dd = xa.abs().max(za.abs());
    if dd < f64::from(0.01_f32) {
        return None;
    }
    dd = dd.sqrt();
    xa /= dd;
    za /= dd;
    let pow = (1.0 / dd).min(1.0);
    xa *= pow;
    za *= pow;
    xa *= f64::from(0.05_f32);
    za *= f64::from(0.05_f32);
    Some((xa, za))
}

/// One entity `getPushableEntities` answered: a stored body or an online
/// player.
#[derive(Debug, Clone, Copy)]
enum Pushable {
    Stored(i32),
    Session(PlayerPushBody),
}

fn pushable_entities(
    except: Option<i32>,
    body_box: Aabb,
    own_team: Option<u32>,
    with_sessions: bool,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<Vec<Pushable>> {
    if level.scoreboard_teams().pushes_nothing(own_team) {
        return Some(Vec::new());
    }
    let mut found = Vec::new();
    for candidate in store.ids_in_box_sections(body_box) {
        if Some(candidate) == except {
            continue;
        }
        let Some(other) = store.get(candidate) else {
            continue;
        };
        if other.bounding_box().intersects(body_box) {
            found.push((
                section_walk_order_key(other.filed_section()),
                candidate,
                Pushable::Stored(candidate),
            ));
        }
    }
    if with_sessions {
        for session in store
            .players()
            .filter(|player| Some(player.body().id()) != except)
            .filter_map(crate::world::PlayerState::push_body)
        {
            if session_box(&session).intersects(body_box) {
                let section = section_of_block(block_position(session.position));
                found.push((
                    section_walk_order_key(section),
                    session.entity_id,
                    Pushable::Session(session),
                ));
            }
        }
    }
    if found.is_empty() {
        return Some(Vec::new());
    }
    if found.len() > 1
        && found
            .iter()
            .any(|(_, _, candidate)| matches!(candidate, Pushable::Session(_)))
    {
        found.sort_by_key(|(section, id, _)| (*section, *id));
    }
    let mut pushable = Vec::new();
    let teams = level.scoreboard_teams();
    for (_, _, candidate) in found {
        let their_team = match candidate {
            Pushable::Stored(id) => {
                if !is_pushable(store.get_mut(id)?, level)? {
                    continue;
                }
                let other = store.get(id)?;
                if crate::entity::general_living::is_abstract_horse_entity_type(
                    other.entity_type_id(),
                ) && !store.passengers(id).is_empty()
                {
                    continue;
                }
                teams.entity_team(other.uuid())
            }
            Pushable::Session(session) => {
                if !session_is_pushable(&session, store, level)? {
                    continue;
                }
                store.player(session.entity_id)?.team(teams)
            }
        };
        if teams.collision_admits(own_team, their_team) {
            pushable.push(candidate);
        }
    }
    Some(pushable)
}

///
/// The body is detached from the store while its tick runs, so the two arms of
/// the push write different places: the body's own arm writes the detached
/// entity and the other's arm writes the store.
fn push_pair(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    other_id: i32,
) -> Option<()> {
    let body_id = body.id();
    // `!this.isPassengerOfSameVehicle(entity)` (`:1783`).
    if store.root_vehicle(body_id) == store.root_vehicle(other_id) {
        return Some(());
    }
    let other = store.get(other_id)?;
    // `!entity.noPhysics && !this.noPhysics` (`:1784`).
    if other.sleeping_pos().is_some() || other.no_physics() || body.no_physics() {
        return Some(());
    }
    let Some((xa, za)) = pair_impulse(other.position(), body.position()) else {
        return Some(());
    };
    // `if (!this.isVehicle() && this.isPushable()) this.push(-xa, 0.0, -za)`
    // (`:1801-1803`), then the same for `entity` (`:1805-1807`).
    if store.passengers(other_id).is_empty() && is_pushable(store.get_mut(other_id)?, level)? {
        push(store.get_mut(other_id)?, -xa, 0.0, -za);
    }
    if store.passengers(body_id).is_empty() && is_pushable(body, level)? {
        push(body, xa, 0.0, za);
    }
    Some(())
}

fn session_root_vehicle(store: &EntityStore, entity_id: i32) -> i32 {
    store
        .external_player_root_vehicle(entity_id)
        .unwrap_or(entity_id)
}

fn apply_session_push_if_pushable(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    session: &PlayerPushBody,
    dx: f64,
    dz: f64,
) -> Option<()> {
    if store.passengers(session.entity_id).is_empty() && session_is_pushable(session, store, level)?
    {
        if let Some(player) = store.player_mut(session.entity_id) {
            player.push_impulse(Vec3d {
                x: dx,
                y: 0.0,
                z: dz,
            });
        }
    }
    Some(())
}

/// [`push_pair`] with an online player as `this`.
fn push_session_pair(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    session: PlayerPushBody,
) -> Option<()> {
    if session_root_vehicle(store, session.entity_id) == store.root_vehicle(body.id()) {
        return Some(());
    }
    if session.sleeping || session.spectator || body.no_physics() {
        return Some(());
    }
    let Some((xa, za)) = pair_impulse(session.position, body.position()) else {
        return Some(());
    };
    apply_session_push_if_pushable(store, level, &session, -xa, -za)?;
    if store.passengers(body.id()).is_empty() && is_pushable(body, level)? {
        push(body, xa, 0.0, za);
    }
    Some(())
}

///
/// `None` when the level or the store cannot answer a query the step needs, so
/// the owning tick fails closed rather than pushing against a guess.
pub(crate) fn push_entities(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let own_team = level.scoreboard_teams().entity_team(entity.uuid());
    let pushable = pushable_entities(
        Some(entity.id()),
        entity.bounding_box(),
        own_team,
        true,
        store,
        level,
    )?;
    if !pushable.is_empty() {
        cram(entity, store, level, &pushable)?;
    }
    for candidate in pushable {
        // `doPush(entity)` (`:3177`).
        let other_id = match candidate {
            Pushable::Stored(id) => id,
            Pushable::Session(session) => session.entity_id,
        };
        // Parrot.java:390-394.
        if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID
            && matches!(candidate, Pushable::Session(_))
        {
            continue;
        }
        crate::entity::warden::anger::on_push(entity, store, level, other_id);
        iron_golem_push(entity, store, level, other_id);
        if matches!(candidate, Pushable::Stored(_)) {
            crate::entity::slime::push_iron_golem(entity, store, level, other_id);
        }
        match candidate {
            Pushable::Stored(other_id) => vehicle::push_by_body(entity, store, level, other_id)?,
            Pushable::Session(session) => push_session_pair(entity, store, level, session)?,
        }
    }
    Some(())
}

// IronGolem.java:105-111.
fn iron_golem_push(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    other_id: i32,
) {
    if entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID {
        return;
    }
    let Some((other_type, uuid)) = store
        .get(other_id)
        .map(|other| (other.entity_type_id(), other.uuid()))
    else {
        return;
    };
    if crate::entity::ai::mob::target::selector::is_enemy(other_type)
        && other_type != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID
        && store.random().next_int(20) == 0
    {
        crate::entity::ai::mob::target::sense::set_body_target(
            entity,
            store,
            level,
            Some(crate::entity::ai::mob::target::TargetRef {
                entity_id: other_id,
                uuid,
            }),
        );
    }
}

fn should_cram_pusher(store: &mut EntityStore, pushable: &[Pushable], max_cramming: i32) -> bool {
    box_reaches_cramming(pushable.len(), max_cramming)
        && store.random().next_int(4) == 0
        && crowd_reaches_cramming(store, pushable, max_cramming)
}

///
/// The two size tests are not the same one twice: the first is every pushable
/// the box holds, the second drops those riding something (`:3164-3168`). The
/// draw sits between them and is skipped outright when the first fails.
fn cram(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pushable: &[Pushable],
) -> Option<()> {
    if !should_cram_pusher(store, pushable, level.max_entity_cramming()) {
        return Some(());
    }
    // `this.hurtServer(serverLevel, this.damageSources().cramming(), 6.0F)`
    // (`:3171`): the pusher hurts itself, with a source that names neither a
    // direct nor a causing entity.
    crate::entity::general_living::hurt_from_environment_outcome(
        entity,
        store,
        level,
        "minecraft:cramming",
        6.0,
    )?;
    Some(())
}

fn box_reaches_cramming(pushable: usize, max_cramming: i32) -> bool {
    max_cramming > 0 && pushable as i32 > max_cramming - 1
}

/// The second size test (`:3162-3170`), which counts only the pushables for
/// which `Entity.isPassenger()` is false.
fn crowd_reaches_cramming(store: &EntityStore, pushable: &[Pushable], max_cramming: i32) -> bool {
    let riders = pushable
        .iter()
        .filter(|candidate| match candidate {
            Pushable::Stored(id) => store.vehicle(*id).is_some(),
            Pushable::Session(session) => {
                store.external_player_vehicle(session.entity_id).is_some()
            }
        })
        .count();
    pushable.len() - riders > max_cramming as usize - 1
}

///
/// Same box scan, same `EntitySelector.pushableBy` predicate, same cramming
/// arm and the same push order as [`push_entities`], with the other online
/// players among the pushables. The pusher's own half of each pair
/// (`Entity.push(Entity)`'s `:1805-1807`) lands on its session like every
/// other push does.
pub(crate) fn push_entities_from_session_player(
    pusher: PlayerPushBody,
    own_team: Option<u32>,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let body_box = session_box(&pusher);
    let pushable = pushable_entities(
        Some(pusher.entity_id),
        body_box,
        own_team,
        true,
        store,
        level,
    )?;
    if pushable.is_empty() {
        return Some(());
    }
    // `if (this.level() instanceof ServerLevel serverLevel)` (`:3159-3174`):
    // the pusher is a session of the level this seam stands for.
    if should_cram_pusher(store, &pushable, level.max_entity_cramming()) {
        // `this.hurtServer(serverLevel, this.damageSources().cramming(), 6.0F)`
        // (`:3171`) through the seam a level reaches its own sessions by.
        level.hurt_player(crate::level_event::PendingPlayerHurt {
            player: pusher.uuid,
            damage_type: "minecraft:cramming",
            amount: 6.0,
            causing: None,
            direct: None,
            post_attack: false,
            impulse: None,
        });
    }
    let pusher_root = session_root_vehicle(store, pusher.entity_id);
    for candidate in pushable {
        let pushed = match candidate {
            Pushable::Stored(other_id)
                if store
                    .get(other_id)
                    .is_some_and(|other| crate::entity::minecart::is_minecart(other.data())) =>
            {
                vehicle::minecart_pushed_by_session(store, other_id, pusher);
                None
            }
            Pushable::Stored(other_id) => {
                push_stored_away(store, level, other_id, pusher, pusher_root)?
            }
            Pushable::Session(other) => {
                push_session_away(store, level, other, pusher, pusher_root)?
            }
        };
        let Some((xa, za)) = pushed else {
            continue;
        };
        // `if (!entity.isVehicle() && entity.isPushable()) entity.push(xa, 0.0, za)`
        // (`:1805-1807`).
        apply_session_push_if_pushable(store, level, &pusher, xa, za)?;
    }
    Some(())
}

/// The stored body's half of a pair a session pushes; answers the pair's
/// impulse when the pair pushes at all.
fn push_stored_away(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    other_id: i32,
    pusher: PlayerPushBody,
    pusher_root: i32,
) -> Option<Option<(f64, f64)>> {
    // `!this.isPassengerOfSameVehicle(entity)` (`:1783`).
    if pusher_root == store.root_vehicle(other_id) {
        return Some(None);
    }
    let other = store.get(other_id)?;
    // `LivingEntity.push(Entity)` skips a sleeper (`LivingEntity.java:2295-2299`);
    // `!entity.noPhysics && !this.noPhysics` (`:1784`).
    if other.sleeping_pos().is_some() || other.no_physics() || pusher.spectator {
        return Some(None);
    }
    if crate::entity::boat::is_boat(other.data())
        && !vehicle::boat_takes_push(other, session_box(&pusher), false)
    {
        return Some(None);
    }
    let Some((xa, za)) = pair_impulse(other.position(), pusher.position) else {
        return Some(None);
    };
    // `if (!this.isVehicle() && this.isPushable()) this.push(-xa, 0.0, -za)`
    // (`:1801-1803`).
    if store.passengers(other_id).is_empty() && is_pushable(store.get_mut(other_id)?, level)? {
        push(store.get_mut(other_id)?, -xa, 0.0, -za);
    }
    Some(Some((xa, za)))
}

/// [`push_stored_away`] with another online player as `this`.
fn push_session_away(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    other: PlayerPushBody,
    pusher: PlayerPushBody,
    pusher_root: i32,
) -> Option<Option<(f64, f64)>> {
    let other_root = session_root_vehicle(store, other.entity_id);
    if pusher_root == other_root || other.sleeping || other.spectator || pusher.spectator {
        return Some(None);
    }
    let Some((xa, za)) = pair_impulse(other.position, pusher.position) else {
        return Some(None);
    };
    apply_session_push_if_pushable(store, level, &other, -xa, -za)?;
    Some(Some((xa, za)))
}

mod vehicle;
pub(crate) use vehicle::{
    armor_stand_push_minecarts, boat_push_and_pickup, minecart_push_and_pickup,
};

#[cfg(test)]
mod tests;
