use crate::entity::living::valid_equipment::EquipmentRegistry;
use crate::entity::store::EntityBodyReadView;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use uuid::Uuid;

use crate::entity::{Entity, EntityData, EntityRegistryTagView, EntityStore, NearbyPlayer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerIdentity {
    pub uuid: Uuid,
    pub entity_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectLivingController {
    Player(PlayerIdentity),
    Stored(i32),
}

pub fn direct_controlling_player_with_registry_tags(
    entity: &Entity,
    store: Option<&EntityStore>,
    registry_tags: &EntityRegistryTagView,
    player_by_entity_id: impl Fn(i32) -> Option<NearbyPlayer>,
) -> Option<Option<PlayerIdentity>> {
    direct_controlling_player_with_resolver(
        entity,
        store.map(|store| EntityBodyReadView::new(store, Some(entity), None)),
        &EquipmentRegistry(registry_tags),
        player_by_entity_id,
    )
}

pub(crate) fn direct_controlling_player_with_resolver(
    entity: &Entity,
    bodies: Option<EntityBodyReadView<'_>>,
    resolver: &dyn PersistentItemStackHolderResolver,
    player_by_entity_id: impl Fn(i32) -> Option<NearbyPlayer>,
) -> Option<Option<PlayerIdentity>> {
    let controller = query(
        entity,
        bodies,
        resolver,
        player_by_entity_id,
        !crate::entity::boat::is_boat(entity.data()),
    );
    controller.map(|controller| {
        controller.and_then(|controller| match controller {
            crate::entity::passenger::DirectLivingController::Player(player) => Some(player),
            crate::entity::passenger::DirectLivingController::Stored(_) => None,
        })
    })
}

pub(crate) fn direct_controlling_living_with_registry_tags(
    entity: &Entity,
    store: Option<&EntityStore>,
    registry_tags: &EntityRegistryTagView,
    player_by_entity_id: impl Fn(i32) -> Option<NearbyPlayer>,
) -> Option<Option<DirectLivingController>> {
    query(
        entity,
        store.map(|store| EntityBodyReadView::new(store, Some(entity), None)),
        &EquipmentRegistry(registry_tags),
        player_by_entity_id,
        false,
    )
}

fn query(
    entity: &Entity,
    bodies: Option<EntityBodyReadView<'_>>,
    resolver: &dyn PersistentItemStackHolderResolver,
    player_by_entity_id: impl Fn(i32) -> Option<NearbyPlayer>,
    player_only: bool,
) -> Option<Option<DirectLivingController>> {
    let boat = crate::entity::boat::is_boat(entity.data());
    let data = match entity.data() {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    };
    let horse = data.is_some_and(|data| data.abstract_horse_retained().is_some());
    let resource = entity.data().entity_resource_id();
    let special = matches!(
        resource,
        "minecraft:pig"
            | "minecraft:strider"
            | "minecraft:nautilus"
            | "minecraft:zombie_nautilus"
            | "minecraft:happy_ghast"
    );
    // Entity.java:3423-3425; Mob.java:216-219: ordinary Mob's controller is a Mob.
    let mut player_allowed = boat || horse || special;
    if !player_allowed && player_only {
        return Some(None);
    }
    if player_allowed && !boat {
        let slot = if resource == "minecraft:happy_ghast" {
            EquipmentSlot::Body
        } else {
            EquipmentSlot::Saddle
        };
        player_allowed = crate::entity::living::valid_equipment::has_valid_item_with_resolver(
            entity, slot, resolver,
        )?;
        if player_allowed && resource == "minecraft:happy_ghast" {
            let state = data?.happy_ghast_state()?;
            player_allowed = state.still_timeout <= 0 && !state.stays_still;
        }
    }
    if !player_allowed {
        if player_only {
            return Some(None);
        }
        return mob_controller_with_resolver(entity, bodies, resolver)
            .map(|id| id.map(DirectLivingController::Stored));
    }
    let bodies = bodies?;
    let store = bodies.store();
    let Some(first) = first_passenger(entity, store)? else {
        return Some(None);
    };
    if let Some(passenger) = bodies.get(first) {
        if store.is_external_player_passenger(first) || !boat && !passenger.is_alive() {
            return None;
        }
        if passenger.entity_type_id() != VANILLA_ENTITY_TYPE_PLAYER_ID {
            if player_only || boat && passenger.living().is_none() {
                return Some(None);
            }
            if boat {
                return Some(Some(DirectLivingController::Stored(first)));
            }
            return mob_controller_with_resolver(entity, Some(bodies), resolver)
                .map(|id| id.map(DirectLivingController::Stored));
        }
    }
    let (player, session) = resolve_player(first, bodies, &player_by_entity_id, !boat)?;
    // Pig.java:89-92; Strider.java:196-199: Player.isHolding checks both hands.
    if matches!(resource, "minecraft:pig" | "minecraft:strider") {
        let item = vanilla_item_type_for_resource_id(if resource == "minecraft:pig" {
            "minecraft:carrot_on_a_stick"
        } else {
            "minecraft:warped_fungus_on_a_stick"
        })?
        .id;
        let holding = if let Some(passenger) = bodies.get(first) {
            let living = passenger.living()?;
            [EquipmentSlot::MainHand, EquipmentSlot::OffHand]
                .into_iter()
                .any(|slot| {
                    living
                        .item_by_slot(slot)
                        .is_some_and(|stack| stack.item_id() == Some(item))
                })
        } else {
            let session = session?;
            session.main_hand_item_id == Some(item) || session.off_hand_item_id == Some(item)
        };
        if !holding {
            return Some(None);
        }
    }
    Some(Some(DirectLivingController::Player(player)))
}

fn first_passenger(entity: &Entity, store: &EntityStore) -> Option<Option<i32>> {
    let Some(&first) = store.passengers(entity.id()).first() else {
        return Some(None);
    };
    if first == entity.id()
        || !store.passenger_relation_matches(first, Some(entity.id()))
        || store.has_indirect_passenger(first, entity.id())
        || store.entity_id_for_uuid(entity.uuid()) != Some(entity.id())
    {
        return None;
    }
    Some(Some(first))
}

pub(crate) fn mob_controller(
    entity: &Entity,
    store: Option<&EntityStore>,
    registry_tags: &EntityRegistryTagView,
    detached: Option<&Entity>,
) -> Option<Option<i32>> {
    mob_controller_with_resolver(
        entity,
        store.map(|store| EntityBodyReadView::new(store, detached, None)),
        &EquipmentRegistry(registry_tags),
    )
}

fn mob_controller_with_resolver(
    entity: &Entity,
    bodies: Option<EntityBodyReadView<'_>>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Option<Option<i32>> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return Some(None);
    };
    if !data.is_mob() || data.is_no_ai() {
        return Some(None);
    }
    let bodies = bodies?;
    let store = bodies.store();
    let Some(first) = first_passenger(entity, store)? else {
        return Some(None);
    };
    let passenger = if let Some(passenger) = bodies.get(first) {
        passenger
    } else {
        return store.is_external_player_passenger(first).then_some(None);
    };
    if store.is_external_player_passenger(first) {
        return None;
    }
    if passenger.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID
        || !matches!(passenger.data(), EntityData::GeneralLiving(data) if data.is_mob())
    {
        return Some(None);
    }
    if resolver.tag_contains(
        "minecraft:entity_type",
        "minecraft:non_controlling_rider",
        passenger.data().entity_resource_id(),
    )? {
        Some(None)
    } else {
        Some(Some(first))
    }
}

fn resolve_player(
    first: i32,
    bodies: EntityBodyReadView<'_>,
    player_by_entity_id: &impl Fn(i32) -> Option<NearbyPlayer>,
    require_alive: bool,
) -> Option<(PlayerIdentity, Option<NearbyPlayer>)> {
    let store = bodies.store();
    if let Some(passenger) = bodies.get(first) {
        if store.is_external_player_passenger(first) || require_alive && !passenger.is_alive() {
            return None;
        }
        let player = entity_player(passenger)?;
        if store.entity_id_for_uuid(player.uuid) != Some(first) {
            return None;
        }
        let session = player_by_entity_id(first);
        if session.is_some_and(|session| session.entity_id != first || session.uuid != player.uuid)
        {
            return None;
        }
        Some((player, session))
    } else if store.is_external_player_passenger(first) {
        let session = player_by_entity_id(first)?;
        if session.entity_id != first
            || store.contains_uuid(session.uuid)
            || store
                .unavailable_registered_uuids()
                .any(|uuid| store.entity_id_for_uuid(uuid) == Some(first))
        {
            return None;
        }
        Some((
            PlayerIdentity {
                uuid: session.uuid,
                entity_id: first,
            },
            Some(session),
        ))
    } else {
        None
    }
}

fn entity_player(entity: &Entity) -> Option<PlayerIdentity> {
    (entity.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID).then_some(PlayerIdentity {
        uuid: entity.uuid(),
        entity_id: entity.id(),
    })
}

#[cfg(test)]
mod tests;
