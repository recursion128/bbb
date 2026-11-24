//! Minecart and boat contact: the cart's own `push(Entity)` override and the
//! pickup sweeps both vehicles run at the end of their tick.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::{mth_cos, mth_sin, Aabb, Vec3Ops};

use super::{
    is_pushable, push, push_pair, push_session_pair, pushable_entities, session_box,
    PlayerPushBody, Pushable,
};
use crate::entity::boat::is_boat;
use crate::entity::minecart::is_minecart;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;

/// `other.push(body)` through the other's own override: a minecart runs
/// `AbstractMinecart.push(Entity)`, everything else `Entity.push(Entity)`.
pub(super) fn push_by_body(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    other_id: i32,
) -> Option<()> {
    if store
        .get(other_id)
        .is_some_and(|other| is_minecart(other.data()))
    {
        minecart_pushed_by_body(store, other_id, body);
        return Some(());
    }
    if store.get(other_id).is_some_and(|other| {
        is_boat(other.data()) && !boat_takes_push(other, body.bounding_box(), is_boat(body.data()))
    }) {
        return Some(());
    }
    push_pair(body, store, level, other_id)
}

/// `AbstractBoat.push(Entity)` (`AbstractBoat.java:175-183`): another boat
/// pushes only from below the boat's top, anything else only from its floor.
pub(super) fn boat_takes_push(boat: &Entity, pusher: Aabb, pusher_is_boat: bool) -> bool {
    let own = boat.bounding_box();
    if pusher_is_boat {
        pusher.min_y < own.max_y
    } else {
        pusher.min_y <= own.min_y
    }
}

/// `AbstractMinecart.push(Entity)` (`AbstractMinecart.java:496-525`) up to
/// the split between a cart and any other entity.
fn minecart_impulse(cart: Vec3d, entity: Vec3d) -> Option<(f64, f64)> {
    let mut xa = entity.x - cart.x;
    let mut za = entity.z - cart.z;
    let mut dd = xa * xa + za * za;
    if dd < f64::from(1.0e-4_f32) {
        return None;
    }
    dd = dd.sqrt();
    xa /= dd;
    za /= dd;
    let pow = (1.0 / dd).min(1.0);
    xa *= pow;
    za *= pow;
    xa *= f64::from(0.1_f32);
    za *= f64::from(0.1_f32);
    xa *= 0.5;
    za *= 0.5;
    Some((xa, za))
}

/// `cart.push(body)` with the cart stored and the pusher detached.
fn minecart_pushed_by_body(store: &mut EntityStore, cart_id: i32, body: &mut Entity) {
    let Some(cart) = store.get(cart_id) else {
        return;
    };
    if body.no_physics() || cart.no_physics() || store.passengers(cart_id).contains(&body.id()) {
        return;
    }
    let Some((xa, za)) = minecart_impulse(cart.position(), body.position()) else {
        return;
    };
    if is_minecart(body.data()) {
        push_other_minecart(store, cart_id, body, xa, za);
    } else if let Some(cart) = store.get_mut(cart_id) {
        push(cart, -xa, 0.0, -za);
        push(body, xa / 4.0, 0.0, za / 4.0);
    }
}

/// `cart.push(player)` for an online player walking into a stored cart.
pub(super) fn minecart_pushed_by_session(
    store: &mut EntityStore,
    cart_id: i32,
    session: PlayerPushBody,
) {
    let Some(cart) = store.get(cart_id) else {
        return;
    };
    if session.spectator
        || cart.no_physics()
        || store.passengers(cart_id).contains(&session.entity_id)
    {
        return;
    }
    let Some((xa, za)) = minecart_impulse(cart.position(), session.position) else {
        return;
    };
    if let Some(cart) = store.get_mut(cart_id) {
        push(cart, -xa, 0.0, -za);
    }
    if let Some(player) = store.player_mut(session.entity_id) {
        player.push_impulse(Vec3d {
            x: xa / 4.0,
            y: 0.0,
            z: za / 4.0,
        });
    }
}

/// `AbstractMinecart.pushOtherMinecart` (`AbstractMinecart.java:530-563`)
/// without the experimental-movement arm.
fn push_other_minecart(
    store: &mut EntityStore,
    cart_id: i32,
    other: &mut Entity,
    xa: f64,
    za: f64,
) {
    let Some(cart) = store.get_mut(cart_id) else {
        return;
    };
    let dir = Vec3d {
        x: other.position().x - cart.position().x,
        y: 0.0,
        z: other.position().z - cart.position().z,
    }
    .normalize();
    let y_rot = f64::from(cart.y_rot() * (std::f64::consts::PI / 180.0) as f32);
    let facing = Vec3d {
        x: f64::from(mth_cos(y_rot)),
        y: 0.0,
        z: f64::from(mth_sin(y_rot)),
    }
    .normalize();
    let dot = (dir.x * facing.x + dir.z * facing.z).abs();
    if dot < f64::from(0.8_f32) {
        return;
    }
    let movement = cart.delta_movement();
    let other_movement = other.delta_movement();
    let furnace = |entity: &Entity| matches!(entity.data(), EntityData::FurnaceMinecart(_));
    match (furnace(cart), furnace(other)) {
        (false, true) => {
            cart.set_delta_movement(movement.multiply(0.2, 1.0, 0.2));
            push(cart, other_movement.x - xa, 0.0, other_movement.z - za);
            other.set_delta_movement(other_movement.multiply(0.95, 1.0, 0.95));
        }
        (true, false) => {
            other.set_delta_movement(other_movement.multiply(0.2, 1.0, 0.2));
            push(other, movement.x + xa, 0.0, movement.z + za);
            cart.set_delta_movement(movement.multiply(0.95, 1.0, 0.95));
        }
        _ => {
            let xdd = (other_movement.x + movement.x) / 2.0;
            let zdd = (other_movement.z + movement.z) / 2.0;
            cart.set_delta_movement(movement.multiply(0.2, 1.0, 0.2));
            push(cart, xdd - xa, 0.0, zdd - za);
            other.set_delta_movement(other_movement.multiply(0.2, 1.0, 0.2));
            push(other, xdd + xa, 0.0, zdd + za);
        }
    }
}

/// `OldMinecartBehavior.pushAndPickupEntities` (`OldMinecartBehavior.java:366-391`).
pub(crate) fn minecart_push_and_pickup(
    cart: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let hitbox = cart
        .bounding_box()
        .inflate(f64::from(0.2_f32), 0.0, f64::from(0.2_f32));
    // `Minecart.java:65-67` is the only rideable cart.
    let rideable = matches!(cart.data(), EntityData::Minecart(_));
    if rideable && cart.delta_movement().horizontal_distance_sqr() >= 0.01 {
        let own_team = level.scoreboard_teams().entity_team(cart.uuid());
        for candidate in pushable_entities(Some(cart.id()), hitbox, own_team, true, store, level)? {
            match candidate {
                Pushable::Stored(id) => {
                    let other = store.get(id)?;
                    let seats = other.entity_type_id() != VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
                        && !is_minecart(other.data())
                        && store.passengers(cart.id()).is_empty()
                        && store.vehicle(id).is_none();
                    if seats {
                        crate::entity::passenger::seat_on_detached_vehicle(cart, store, level, id);
                    } else {
                        push_by_body(cart, store, level, id)?;
                    }
                }
                Pushable::Session(session) => push_session_pair(cart, store, level, session)?,
            }
        }
        return Some(());
    }
    for id in store.ids_in_box_sections(hitbox) {
        if id == cart.id() || store.passengers(cart.id()).contains(&id) {
            continue;
        }
        let Some(other) = store.get_mut(id) else {
            continue;
        };
        if is_minecart(other.data())
            && other.bounding_box().intersects(hitbox)
            && is_pushable(other, level)?
        {
            minecart_pushed_by_body(store, id, cart);
        }
    }
    Some(())
}

/// The pickup sweep closing `AbstractBoat.tick()` (`AbstractBoat.java:274-292`).
pub(crate) fn boat_push_and_pickup(
    boat: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    player_controls: bool,
) -> Option<()> {
    let hitbox = boat
        .bounding_box()
        .inflate(f64::from(0.2_f32), -0.01, f64::from(0.2_f32));
    let own_team = level.scoreboard_teams().entity_team(boat.uuid());
    let max_passengers = if crate::entity::chest_boat::chest_boat(boat.data()).is_some() {
        1
    } else {
        2
    };
    for candidate in pushable_entities(Some(boat.id()), hitbox, own_team, true, store, level)? {
        let id = match candidate {
            Pushable::Stored(id) => id,
            // Players are in `#cannot_be_pushed_onto_boats`.
            Pushable::Session(session) => {
                if boat_takes_push(boat, session_box(&session), false) {
                    push_session_pair(boat, store, level, session)?;
                }
                continue;
            }
        };
        // `!entity.hasPassenger(this)`.
        if store.vehicle(boat.id()) == Some(id) {
            continue;
        }
        let other = store.get(id)?;
        let seats = !player_controls
            && store.passengers(boat.id()).len() < max_passengers
            && store.vehicle(id).is_none()
            && other.bb_width() < boat.bb_width()
            && other.living().is_some()
            && !level.registry_tag_membership(
                "minecraft:entity_type",
                "minecraft:cannot_be_pushed_onto_boats",
                other.data().entity_resource_id(),
            );
        if seats {
            crate::entity::passenger::seat_on_detached_vehicle(boat, store, level, id);
        } else {
            // `this.push(entity)` is `Entity.push(Entity)` from the boat's
            // side, whose two impulses are the pair's from the other's side.
            if boat_takes_push(boat, other.bounding_box(), is_boat(other.data())) {
                push_pair(boat, store, level, id)?;
            }
        }
    }
    Some(())
}

/// `ArmorStand.pushEntities` (`ArmorStand.java:177-183`): a stand only pushes
/// the rideable minecarts whose position is within `0.2` of its own.
pub(crate) fn armor_stand_push_minecarts(stand: &mut Entity, store: &mut EntityStore) {
    let bounds = stand.bounding_box();
    for id in store.ids_in_box_sections(bounds) {
        let Some(cart) = store.get(id) else {
            continue;
        };
        if id != stand.id()
            && matches!(cart.data(), EntityData::Minecart(_))
            && cart.bounding_box().intersects(bounds)
            && cart.position().distance_to_sqr(stand.position()) <= 0.2
        {
            minecart_pushed_by_body(store, id, stand);
        }
    }
}
