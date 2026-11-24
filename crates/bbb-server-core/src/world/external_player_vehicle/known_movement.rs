//! Known movement through an entity's actual player controller.

use crate::connection::ConnectionId;
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityRegistryTagView, EntityStore};
use bbb_protocol::packets::Vec3d;

use crate::world::{PlayerState, ServerWorld};

impl ServerWorld {
    // ServerPlayer.java:2179-2182; Entity.java:3931-3933.
    pub(in crate::world) fn player_known_movement(&self, target: ConnectionId) -> Option<Vec3d> {
        let player = self.levels.player(target)?;
        if player.avatar_removed {
            return Some(player.known_movement);
        }
        let level = self.level_if_present_by_id(&player.dimension)?;
        let tags = level.chunks.registry_tags_shared().map_or(
            EntityRegistryTagView::Builtin26_1,
            EntityRegistryTagView::Frozen,
        );
        Self::player_known_movement_in_bodies(
            player,
            EntityBodyReadView::new(&level.entities, None, None),
            &tags,
        )
    }

    pub(in crate::world) fn player_known_movement_in_bodies(
        player: &PlayerState,
        bodies: EntityBodyReadView<'_>,
        tags: &EntityRegistryTagView,
    ) -> Option<Vec3d> {
        if player.avatar_removed {
            return Some(player.known_movement);
        }
        let entities = bodies.store();
        let player = Self::unique_session_player(entities, player, player.entity_id())?;
        let Some(vehicle_id) = entities.vehicle(player.entity_id()) else {
            return Some(player.known_movement);
        };
        if bodies.external_player_vehicle(player.entity_id()) != Some(vehicle_id) {
            return None;
        }
        let vehicle = bodies.get(vehicle_id)?;
        Self::known_movement_from_vehicle(
            bodies,
            vehicle,
            Some(player),
            &crate::entity::living::valid_equipment::EquipmentRegistry(tags),
        )
    }

    pub(crate) fn stored_known_movement_in_bodies(
        entity: &Entity,
        bodies: EntityBodyReadView<'_>,
    ) -> Option<Vec3d> {
        Self::known_movement_from_vehicle(bodies, entity, None, bodies.store().item_resolver())
    }

    fn known_movement_from_vehicle(
        bodies: EntityBodyReadView<'_>,
        vehicle: &Entity,
        player: Option<&PlayerState>,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Option<Vec3d> {
        let entities = bodies.store();
        let session = |entity_id| {
            let current = entities.player(entity_id)?;
            Self::unique_session_player(entities, player.unwrap_or(current), entity_id)
        };
        let controller = if player.is_some() || vehicle_alive(vehicle) {
            crate::entity::passenger::direct_controlling_player_with_resolver(
                vehicle,
                Some(bodies),
                resolver,
                |entity_id| {
                    let session = session(entity_id)?;
                    if bodies.external_player_vehicle(entity_id) != Some(vehicle.id()) {
                        return None;
                    }
                    Some(Self::nearby_player_facts(session))
                },
            )?
        } else {
            None
        };
        if let Some(controller) = controller {
            let session = session(controller.entity_id)?;
            if session.profile.uuid != controller.uuid
                || bodies.external_player_vehicle(controller.entity_id) != Some(vehicle.id())
            {
                return None;
            }
            if player.is_some_and(|player| controller.entity_id == player.entity_id())
                || vehicle_alive(vehicle)
            {
                return Some(session.known_movement);
            }
        }
        let movement = vehicle.delta_movement();
        if crate::entity::minecart::is_minecart(vehicle.data()) {
            // AbstractMinecart.java:345-346; OldMinecartBehavior.java:400-403.
            Some(
                if movement.x.is_nan() || movement.y.is_nan() || movement.z.is_nan() {
                    Vec3d::default()
                } else {
                    Vec3d {
                        x: movement.x.clamp(-0.4, 0.4),
                        y: movement.y,
                        z: movement.z.clamp(-0.4, 0.4),
                    }
                },
            )
        } else {
            Some(movement)
        }
    }

    fn unique_session_player<'a>(
        entities: &'a EntityStore,
        rider: &PlayerState,
        entity_id: i32,
    ) -> Option<&'a PlayerState> {
        let player = entities.player(entity_id)?;
        if player.avatar_removed
            || player.entity_id() != entity_id
            || player.dimension != rider.dimension
            || entities.get(entity_id).is_some()
            || entities.entity_id_for_uuid(player.profile.uuid).is_some()
            || entities.player_by_uuid(player.profile.uuid)?.entity_id() != entity_id
        {
            return None;
        }
        Some(player)
    }
}

fn vehicle_alive(vehicle: &Entity) -> bool {
    // Entity.java:2146-2148; LivingEntity.java:1718-1720.
    vehicle.is_alive() && vehicle.living().is_none_or(|living| living.health() > 0.0)
}

#[cfg(test)]
mod tests;
