//! The map carrier a session is projected to, and the framed-map removals and
//! item-frame comparator output a map state change reaches.

use super::*;
use crate::world::LevelId;

pub(super) fn map_frame_direction(direction: Direction) -> ServerMapFrameDirection {
    match direction {
        Direction::Down => ServerMapFrameDirection::Down,
        Direction::Up => ServerMapFrameDirection::Up,
        Direction::North => ServerMapFrameDirection::North,
        Direction::South => ServerMapFrameDirection::South,
        Direction::West => ServerMapFrameDirection::West,
        Direction::East => ServerMapFrameDirection::East,
    }
}

pub(in crate::world) fn map_carrier(
    connection_id: ConnectionId,
    player: &PlayerState,
    dimension: &str,
    resolver: &dyn PersistentRegistryResolver,
) -> bbb_server_types::Result<(ServerMapCarrier, Vec<i32>)> {
    map_carrier_facts(connection_id, player, dimension, resolver)
}

impl ServerMapCarrier {
    pub(in crate::world) fn holding(&self, held_maps: &[i32], map_id: i32) -> Self {
        Self {
            holds_matching_map: held_maps.contains(&map_id),
            ..self.clone()
        }
    }
}

/// The carrier a session projects to for any map, beside the filled-map ids
/// its inventory slots hold.
pub(in crate::world) fn map_carrier_facts(
    connection_id: ConnectionId,
    player: &PlayerState,
    dimension: &str,
    resolver: &dyn PersistentRegistryResolver,
) -> bbb_server_types::Result<(ServerMapCarrier, Vec<i32>)> {
    let held = player
        .inventory
        .inventory_items()
        .iter()
        .filter(|stack| stack.item_id() == Some(VANILLA_ITEM_FILLED_MAP_ID))
        .filter_map(|stack| stack.map_id())
        .collect();
    let wears_map_invisibility_equipment = player
        .inventory
        .equipment_slots()
        .filter(|(slot, _)| !matches!(slot, EquipmentSlot::MainHand | EquipmentSlot::OffHand))
        .filter_map(|(_, index)| player.inventory.slot(index))
        .filter_map(|stack| stack.item_id().and_then(vanilla_item_type))
        .any(|item| {
            registry_tag_contains(
                resolver,
                "minecraft:item",
                "minecraft:map_invisibility_equipment",
                item.resource_id,
            )
        });
    Ok((
        ServerMapCarrier {
            connection_id,
            plain_name: player.profile.name.clone(),
            dimension: dimension.to_string(),
            x: player.position().x,
            z: player.position().z,
            y_rot: f64::from(player.rotation().y_rot),
            removed: player.avatar_removed,
            holds_matching_map: false,
            wears_map_invisibility_equipment,
        },
        held,
    ))
}

impl ServerWorld {
    /// The frame removals still queued, which save must see before it writes
    /// the map data.
    pub(in crate::world) fn apply_all_framed_map_removals(&mut self) {
        for level in self.levels.values_mut() {
            let removals = level.entities.drain_commands_where(|command| {
                matches!(command, crate::entity::EntityCommand::FramedMapRemoval(_))
            });
            for command in removals {
                if let crate::entity::EntityCommand::FramedMapRemoval(removal) = command {
                    self.maps
                        .removed_from_frame(removal.map_id, removal.pos, removal.entity_id);
                }
            }
        }
    }

    pub(in crate::world) fn item_frame_output_changed_by_key(
        &mut self,
        dimension: &LevelId,
        attached: BlockPos,
    ) {
        self.with_level(dimension, |level| {
            level.update_neighbour_for_output_signal(attached, VANILLA_BLOCK_AIR_ID);
        });
    }

    #[cfg(test)]
    pub(crate) fn item_frame_output_changed_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        attached: BlockPos,
    ) {
        self.item_frame_output_changed_by_key(&dimension.into(), attached);
    }
}
