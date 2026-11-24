//! `HorseInventoryMenu` over a working copy of the mount's saddle, body and
//! cargo slots, which the world refreshes before every menu operation.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;

use bbb_protocol::equipment_slots::{vanilla_equipment_slot_by_id, EquipmentSlot};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{ContainerAnchor, MenuLayout, PlayerInventoryMenu};
use crate::entity::living::valid_equipment::{can_be_equipped_by, equippable};
use crate::entity::EntityRegistryTagView;

const SLOT_SADDLE: i16 = 0;
const SLOT_BODY_ARMOR: i16 = 1;
const SLOT_INVENTORY_START: i16 = 2;

/// The mount behind the menu: what `ArmorSlot.mayPlace` (`ArmorSlot.java:45-47`)
/// reads through `LivingEntity.isEquippableInSlot` (`LivingEntity.java:3821-3826`).
#[derive(Debug, Clone, PartialEq)]
pub struct MountMenuState {
    entity_id: i32,
    entity_type_id: i32,
    can_use_saddle: bool,
    can_use_body: bool,
    tags: EntityRegistryTagView,
}

impl MountMenuState {
    pub(crate) fn new(
        entity_id: i32,
        entity_type_id: i32,
        can_use_saddle: bool,
        can_use_body: bool,
        tags: EntityRegistryTagView,
    ) -> Self {
        Self {
            entity_id,
            entity_type_id,
            can_use_saddle,
            can_use_body,
            tags,
        }
    }
}

impl PlayerInventoryMenu {
    pub fn is_mount_menu(&self) -> bool {
        matches!(self.menu.layout, MenuLayout::Mount { .. })
    }

    /// The entity id of the mount behind the open mount menu.
    pub fn mount_entity(&self) -> Option<i32> {
        self.menu.mount.as_ref().map(|mount| mount.entity_id)
    }

    /// Opens `HorseInventoryMenu` over `[saddle, body, cargo..]`; false when
    /// the slots do not match the column count.
    pub(crate) fn prepare_mount_menu(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        container_id: i32,
        columns: i16,
        container: Vec<RuntimeItemStack>,
        mount: MountMenuState,
    ) -> Result<bool> {
        let layout = MenuLayout::Mount { columns };
        if !self.install_container_menu(
            resolver,
            maps,
            container_id,
            layout,
            ContainerAnchor {
                positions: Vec::new(),
            },
            container,
        )? {
            return Ok(false);
        }
        self.menu.mount = Some(mount);
        self.menu_items()?;
        self.carried().summary();
        Ok(true)
    }

    pub(super) fn mount_may_place(&self, slot: EquipmentSlot, item: &RuntimeItemStack) -> bool {
        let Some(mount) = self.menu.mount.as_ref() else {
            return false;
        };
        let can_use = if slot == EquipmentSlot::Saddle {
            mount.can_use_saddle
        } else {
            mount.can_use_body
        };
        let Some(Some(equippable)) = equippable(item) else {
            return false;
        };
        can_use
            && vanilla_equipment_slot_by_id(equippable.slot_id)
                .is_some_and(|declared| declared.slot == slot)
            && can_be_equipped_by(mount.entity_type_id, equippable, &mount.tags) == Some(true)
    }

    /// `AbstractMountInventoryMenu.quickMoveStack`
    /// (`AbstractMountInventoryMenu.java:43-91`).
    pub(super) fn mount_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        columns: i16,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let original = self.slot_item(slot).into_owned();
        if original.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let cargo = columns * 3;
        let player_start = SLOT_INVENTORY_START + cargo;
        let end = self.menu.layout.slot_count();
        let mut stack = original.clone();
        let fits = |menu: &Self, target: i16, stack: &RuntimeItemStack| -> Result<bool> {
            Ok(menu.slot_may_place(target, stack)? && menu.slot_item(target).is_empty())
        };
        if slot < player_start {
            if !self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                player_start,
                end,
                true,
            )? {
                return Ok(RuntimeItemStack::empty());
            }
        } else if fits(self, SLOT_BODY_ARMOR, &stack)? {
            if !self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                SLOT_BODY_ARMOR,
                SLOT_SADDLE + 2,
                false,
            )? {
                return Ok(RuntimeItemStack::empty());
            }
        } else if fits(self, SLOT_SADDLE, &stack)? {
            if !self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                SLOT_SADDLE,
                SLOT_BODY_ARMOR,
                false,
            )? {
                return Ok(RuntimeItemStack::empty());
            }
        } else if cargo == 0
            || !self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                SLOT_INVENTORY_START,
                player_start,
                false,
            )?
        {
            // `:62-77`: a move between the player rows answers `EMPTY` even
            // when it moved something; the menu holds a copy, so the source is
            // written back before that answer.
            let rows_end = player_start + 27;
            if (rows_end..end).contains(&slot) {
                self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    player_start,
                    rows_end,
                    false,
                )?;
            } else {
                self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    rows_end,
                    end,
                    false,
                )?;
            }
            if stack.count() != original.count() {
                self.finish_slot_move(resolver, maps.as_deref_mut(), slot, stack)?;
            }
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_slot_move(resolver, maps, slot, stack)?;
        Ok(original)
    }
}
