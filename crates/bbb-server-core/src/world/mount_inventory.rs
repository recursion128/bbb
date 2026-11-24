//! `ServerPlayer.openHorseInventory` (`ServerPlayer.java:1357-1367`) and the
//! riding commands of `handlePlayerCommand` (`ServerGamePacketListenerImpl.java:1706-1723`).

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::MountScreenOpen;
use bbb_server_types::interaction::is_within_entity_interaction_range;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{send, ServerWorld, WorldEvent};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::living::valid_equipment::can_use_slot;
use crate::entity::{Entity, EntityData, EntityRegistryTagView};
use crate::inventory::mount_menu::MountMenuState;

/// `AbstractMountInventoryMenu.stillValid`'s range buffer
/// (`AbstractMountInventoryMenu.java:27-31`).
const MOUNT_INTERACTION_BUFFER: f64 = 4.0;

/// The number of slots a mount menu exposes, without cloning any stacks.
fn mount_slot_count(mount: &Entity) -> Option<usize> {
    let EntityData::GeneralLiving(data) = mount.data() else {
        return None;
    };
    let cargo = if is_nautilus(mount) || is_camel(mount) {
        0
    } else {
        data.horse_phenotype()?.inventory_size()
    };
    mount.living()?;
    if cargo > 0 {
        data.internal_inventory()?;
    }
    i16::try_from(cargo / 3).ok()?;
    Some(2 + cargo)
}

/// The menu's slots over one mount: its saddle, body and cargo.
fn mount_slots(mount: &Entity) -> Option<(i16, Vec<RuntimeItemStack>)> {
    let EntityData::GeneralLiving(data) = mount.data() else {
        return None;
    };
    // `getInventoryColumns` (`AbstractChestedHorse.java:186-188`,
    // `Llama.java:299-301`), three rows each.
    // AbstractHorse.java:1043-1045: camels inherit zero cargo columns.
    let cargo = if is_nautilus(mount) || is_camel(mount) {
        0
    } else {
        data.horse_phenotype()?.inventory_size()
    };
    let living = mount.living()?;
    let mut slots = Vec::with_capacity(2 + cargo);
    for slot in [EquipmentSlot::Saddle, EquipmentSlot::Body] {
        slots.push(
            living
                .item_by_slot(slot)
                .cloned()
                .unwrap_or_else(RuntimeItemStack::empty),
        );
    }
    if cargo > 0 {
        let inventory = data.internal_inventory()?;
        for index in 0..cargo {
            slots.push(
                inventory
                    .item(index)
                    .cloned()
                    .unwrap_or_else(RuntimeItemStack::empty),
            );
        }
    }
    Some((i16::try_from(cargo / 3).ok()?, slots))
}

fn is_camel(mount: &Entity) -> bool {
    matches!(
        mount.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    )
}

fn is_nautilus(mount: &Entity) -> bool {
    matches!(
        mount.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

impl ServerWorld {
    pub(in crate::world) fn registry_tags_for(
        &self,
        connection_id: ConnectionId,
    ) -> EntityRegistryTagView {
        let dimension = self.levels.player_at(connection_id).dimension;
        self.level_by_id(&dimension)
            .chunks
            .registry_tags_shared()
            .map(EntityRegistryTagView::Frozen)
            .unwrap_or(EntityRegistryTagView::Builtin26_1)
    }

    /// `AbstractHorse.openCustomInventoryScreen` (`AbstractHorse.java:407-411`)
    /// over `ServerPlayer.openHorseInventory`.
    pub(in crate::world) fn open_mount_inventory(
        &mut self,
        connection_id: ConnectionId,
        mount_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let player = self.levels.player_at(connection_id);
        let player_entity_id = player.entity_id();
        let dimension = player.dimension;
        let tags = self.registry_tags_for(connection_id);
        let level = self.level_by_id(&dimension);
        let Some(mount) = level.entities.get(mount_id) else {
            return;
        };
        let camel = is_camel(mount);
        let tamed = match mount.data() {
            // Camel.java:645-647.
            EntityData::GeneralLiving(_) if camel => true,
            EntityData::GeneralLiving(data) if is_nautilus(mount) => data.is_tame(),
            EntityData::GeneralLiving(data) => data
                .abstract_horse_retained()
                .is_some_and(|horse| horse.is_tame()),
            _ => false,
        };
        let passengers = level.entities.passengers(mount_id);
        // Camel.java:650-654: inventory access does not depend on the current passengers.
        if !tamed || (!camel && !(passengers.is_empty() || passengers.contains(&player_entity_id)))
        {
            return;
        }
        let Some((columns, slots)) = mount_slots(mount) else {
            return;
        };
        let state = MountMenuState::new(
            mount_id,
            mount.entity_type_id(),
            can_use_slot(mount, EquipmentSlot::Saddle) == Some(true),
            can_use_slot(mount, EquipmentSlot::Body) == Some(true),
            tags,
        );
        let Some(container_id) = self.begin_container_open(connection_id, events) else {
            return;
        };
        let prepared = self.levels[connection_id].inventory.prepare_mount_menu(
            &resolver,
            None,
            container_id,
            columns,
            slots,
            state,
        );
        match prepared {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!("mount menu: {error}");
                return;
            }
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::MountScreenOpen(MountScreenOpen {
                container_id,
                inventory_columns: i32::from(columns),
                entity_id: mount_id,
            }),
        ));
        self.initialize_player_menu(connection_id, events);
    }

    pub(in crate::world) fn refresh_mount_menu(&mut self, connection_id: ConnectionId) {
        let player = self.levels.player_at(connection_id);
        let Some(mount_id) = player.inventory.mount_entity() else {
            return;
        };
        let Some((_, slots)) = self
            .level_by_id(&player.dimension)
            .entities
            .get(mount_id)
            .and_then(mount_slots)
        else {
            return;
        };
        self.levels[connection_id]
            .inventory
            .refresh_container_items(slots);
    }

    /// Writes the menu's slots back onto the mount's equipment and cargo.
    pub(in crate::world) fn flush_mount_menu(&mut self, connection_id: ConnectionId) {
        let player = self.levels.player_at(connection_id);
        let Some(mount_id) = player.inventory.mount_entity() else {
            return;
        };
        let items = player.inventory.container_items();
        let dimension = player.dimension;
        let level = self.level_mut_by_id(&dimension);
        let Some(mount) = level.entities.get_mut(mount_id) else {
            return;
        };
        let position = mount.position();
        let effects = crate::entity::item_slots::write_mount_menu_slots(mount, items);
        for effect in effects {
            if let Some(sound) = effect.sound {
                level.entities.play_entity_sound(sound);
            }
            if let Some(event) = effect.equip_event {
                level
                    .event_sink
                    .game_event_with_source_at(event, position, Some(mount_id), None);
            }
        }
    }

    /// `AbstractMountInventoryMenu.stillValid` (`AbstractMountInventoryMenu.java:27-31`).
    pub(in crate::world) fn mount_menu_still_valid(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        let Some(mount_id) = player.inventory.mount_entity() else {
            return false;
        };
        let Some(mount) = self.level_by_id(&player.dimension).entities.get(mount_id) else {
            return false;
        };
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        let size = player.inventory.container_item_count();
        mount.is_alive()
            && mount.living().is_some_and(|living| living.health() > 0.0)
            && mount_slot_count(mount) == Some(size)
            && is_within_entity_interaction_range(
                eye,
                &mount.bounding_box(),
                player.entity_interaction_range(),
                MOUNT_INTERACTION_BUFFER,
            )
    }

    /// `OPEN_INVENTORY` (`ServerGamePacketListenerImpl.java:1719-1723`).
    pub(in crate::world) fn open_vehicle_inventory(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        let player_entity_id = player.entity_id();
        let Some(vehicle_id) = self
            .level_by_id(&player.dimension)
            .entities
            .vehicle(player_entity_id)
        else {
            return;
        };
        if self
            .level_by_id(&player.dimension)
            .entities
            .get(vehicle_id)
            .is_some_and(|entity| crate::entity::item_slots::container_size(entity).is_some())
        {
            self.open_vehicle_container(connection_id, vehicle_id, events);
        } else {
            self.open_mount_inventory(connection_id, vehicle_id, events);
        }
    }

    /// `START_RIDING_JUMP` (`ServerGamePacketListenerImpl.java:1706-1713`): the
    /// controlled mount is saddled and carries the player in its first seat
    /// (`AbstractHorse.java:961-963`; `AbstractNautilus.java:170-171`).
    pub(in crate::world) fn start_vehicle_riding_jump(
        &mut self,
        connection_id: ConnectionId,
        data: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if data <= 0 {
            return;
        }
        let player = self.levels.player_at(connection_id);
        let player_entity_id = player.entity_id();
        let dimension = player.dimension;
        let tags = self.registry_tags_for(connection_id);
        let game_time = self.game_time();
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        let Some(vehicle_id) = entities.vehicle(player_entity_id) else {
            return;
        };
        if entities.passengers(vehicle_id).first() != Some(&player_entity_id) {
            return;
        }
        let Some(mut vehicle) = entities.detach(vehicle_id) else {
            return;
        };
        let dashed = crate::entity::nautilus::start_jump(&mut vehicle, entities, &tags)
            || crate::entity::camel::start_jump(&mut vehicle, entities, &tags, game_time);
        if !dashed {
            crate::entity::general_living::handle_abstract_horse_start_jump(
                &mut vehicle,
                entities,
                &tags,
            );
        }
        let position = vehicle.position();
        entities.attach(vehicle);
        if dashed {
            self.level_mut_by_id(&dimension)
                .event_sink
                .game_event_with_source_at(
                    crate::level_event::VanillaGameEvent::ENTITY_ACTION,
                    position,
                    Some(vehicle_id),
                    None,
                );
        }
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
    }
}
