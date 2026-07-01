use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventoryScreenBackground {
    LocalInventory,
    Generic9xRows {
        rows: u8,
    },
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    CartographyTable,
    CraftingTable,
    Crafter,
    EnchantmentTable,
    Furnace,
    Grindstone,
    Hopper,
    Mount {
        kind: MountInventoryKind,
        inventory_columns: u8,
    },
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Stonecutter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryScreenLayout {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) background: InventoryScreenBackground,
    pub(crate) slots: Vec<InventorySlotLayout>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventorySlotLayout {
    pub(crate) slot_id: i16,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

pub(crate) fn local_inventory_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(46);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 154,
        y: 28,
    });
    for y in 0..2 {
        for x in 0..2 {
            slots.push(InventorySlotLayout {
                slot_id: (1 + x + y * 2) as i16,
                x: 98 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for index in 0..4 {
        slots.push(InventorySlotLayout {
            slot_id: (5 + index) as i16,
            x: 8,
            y: 8 + index * 18,
        });
    }
    for y in 0..3 {
        for x in 0..9 {
            slots.push(InventorySlotLayout {
                slot_id: (9 + x + y * 9) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..9 {
        slots.push(InventorySlotLayout {
            slot_id: (36 + x) as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }
    slots.push(InventorySlotLayout {
        slot_id: 45,
        x: 77,
        y: 62,
    });
    slots
}

pub(crate) fn inventory_screen_layout(world: &WorldStore) -> Option<InventoryScreenLayout> {
    if world.local_inventory_is_open() {
        return Some(InventoryScreenLayout {
            width: INVENTORY_SCREEN_WIDTH,
            height: INVENTORY_SCREEN_HEIGHT,
            background: InventoryScreenBackground::LocalInventory,
            slots: local_inventory_slot_layouts(),
        });
    }

    let container = world.inventory().open_container.as_ref()?;
    if let Some(mount) = container.mount {
        let kind = world.open_mount_inventory_kind()?;
        let inventory_columns = match kind {
            MountInventoryKind::Horse => clamped_mount_inventory_columns(mount.inventory_columns),
            MountInventoryKind::Nautilus => 0,
        };
        let equipment_slots = world.open_mount_equipment_slot_visibility()?;
        return Some(InventoryScreenLayout {
            width: MOUNT_SCREEN_WIDTH,
            height: MOUNT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Mount {
                kind,
                inventory_columns,
            },
            slots: mount_inventory_slot_layouts(inventory_columns, equipment_slots),
        });
    }
    let menu_type_id = container.menu_type_id?;
    if let Some(rows) = generic_container_rows(menu_type_id) {
        return Some(InventoryScreenLayout {
            width: GENERIC_CONTAINER_WIDTH,
            height: GENERIC_CONTAINER_BASE_HEIGHT + i32::from(rows) * GENERIC_CONTAINER_ROW_HEIGHT,
            background: InventoryScreenBackground::Generic9xRows { rows },
            slots: generic_container_slot_layouts(rows),
        });
    }
    if menu_type_id == GENERIC_3X3_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: GENERIC_3X3_SCREEN_WIDTH,
            height: GENERIC_3X3_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Generic3x3,
            slots: generic_3x3_slot_layouts(),
        });
    }
    if menu_type_id == CRAFTER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CRAFTER_SCREEN_WIDTH,
            height: CRAFTER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Crafter,
            slots: crafter_slot_layouts(),
        });
    }
    if menu_type_id == CRAFTING_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CRAFTING_SCREEN_WIDTH,
            height: CRAFTING_SCREEN_HEIGHT,
            background: InventoryScreenBackground::CraftingTable,
            slots: crafting_table_slot_layouts(),
        });
    }
    if menu_type_id == ENCHANTMENT_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: ENCHANTMENT_SCREEN_WIDTH,
            height: ENCHANTMENT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::EnchantmentTable,
            slots: enchantment_table_slot_layouts(),
        });
    }
    if menu_type_id == ANVIL_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: ANVIL_SCREEN_WIDTH,
            height: ANVIL_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Anvil,
            slots: anvil_slot_layouts(),
        });
    }
    if menu_type_id == BEACON_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: BEACON_SCREEN_WIDTH,
            height: BEACON_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Beacon,
            slots: beacon_slot_layouts(),
        });
    }
    if menu_type_id == BREWING_STAND_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: BREWING_STAND_SCREEN_WIDTH,
            height: BREWING_STAND_SCREEN_HEIGHT,
            background: InventoryScreenBackground::BrewingStand,
            slots: brewing_stand_slot_layouts(),
        });
    }
    if let Some(background) = furnace_screen_background(menu_type_id) {
        return Some(InventoryScreenLayout {
            width: FURNACE_SCREEN_WIDTH,
            height: FURNACE_SCREEN_HEIGHT,
            background,
            slots: furnace_slot_layouts(),
        });
    }
    if menu_type_id == GRINDSTONE_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: GRINDSTONE_SCREEN_WIDTH,
            height: GRINDSTONE_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Grindstone,
            slots: grindstone_slot_layouts(),
        });
    }
    if menu_type_id == HOPPER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: HOPPER_SCREEN_WIDTH,
            height: HOPPER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Hopper,
            slots: hopper_slot_layouts(),
        });
    }
    if menu_type_id == LECTERN_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: LECTERN_SCREEN_WIDTH,
            height: LECTERN_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Lectern,
            slots: Vec::new(),
        });
    }
    if menu_type_id == LOOM_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: LOOM_SCREEN_WIDTH,
            height: LOOM_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Loom,
            slots: loom_slot_layouts(),
        });
    }
    if menu_type_id == MERCHANT_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: MERCHANT_SCREEN_WIDTH,
            height: MERCHANT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Merchant,
            slots: merchant_slot_layouts(),
        });
    }
    if menu_type_id == SHULKER_BOX_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: SHULKER_BOX_SCREEN_WIDTH,
            height: SHULKER_BOX_SCREEN_HEIGHT,
            background: InventoryScreenBackground::ShulkerBox,
            slots: shulker_box_slot_layouts(),
        });
    }
    if menu_type_id == SMITHING_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: SMITHING_SCREEN_WIDTH,
            height: SMITHING_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Smithing,
            slots: smithing_slot_layouts(),
        });
    }
    if menu_type_id == CARTOGRAPHY_TABLE_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: CARTOGRAPHY_TABLE_SCREEN_WIDTH,
            height: CARTOGRAPHY_TABLE_SCREEN_HEIGHT,
            background: InventoryScreenBackground::CartographyTable,
            slots: cartography_table_slot_layouts(),
        });
    }
    if menu_type_id == STONECUTTER_MENU_TYPE_ID {
        return Some(InventoryScreenLayout {
            width: STONECUTTER_SCREEN_WIDTH,
            height: STONECUTTER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Stonecutter,
            slots: stonecutter_slot_layouts(),
        });
    }
    None
}

pub(crate) fn inventory_screen_selected_hotbar_slot_id(world: &WorldStore) -> Option<i16> {
    let selected_hotbar_slot = i16::from(world.local_player().selected_hotbar_slot.min(8));
    if world.local_inventory_is_open() {
        return Some(36 + selected_hotbar_slot);
    }

    let container = world.inventory().open_container.as_ref()?;
    if let Some(mount) = container.mount {
        let kind = world.open_mount_inventory_kind()?;
        let inventory_columns = match kind {
            MountInventoryKind::Horse => clamped_mount_inventory_columns(mount.inventory_columns),
            MountInventoryKind::Nautilus => 0,
        };
        let mount_inventory_slots =
            i16::try_from(i32::from(inventory_columns) * MOUNT_INVENTORY_ROWS).ok()?;
        return Some(
            MOUNT_EQUIPMENT_SLOT_COUNT + mount_inventory_slots + 27 + selected_hotbar_slot,
        );
    }

    inventory_screen_hotbar_start(container.menu_type_id?).map(|start| start + selected_hotbar_slot)
}

fn inventory_screen_hotbar_start(menu_type_id: i32) -> Option<i16> {
    if let Some(rows) = generic_container_rows(menu_type_id) {
        return Some(i16::from(rows) * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW + 27);
    }

    match menu_type_id {
        GENERIC_3X3_MENU_TYPE_ID => Some(GENERIC_3X3_SLOT_COUNT + 27),
        CRAFTER_MENU_TYPE_ID => Some(CRAFTER_GRID_SLOT_COUNT + 27),
        CRAFTING_MENU_TYPE_ID => Some(CRAFTING_SLOT_COUNT + 27),
        ENCHANTMENT_MENU_TYPE_ID => Some(ENCHANTMENT_SLOT_COUNT + 27),
        ANVIL_MENU_TYPE_ID => Some(ANVIL_SLOT_COUNT + 27),
        BEACON_MENU_TYPE_ID => Some(BEACON_SLOT_COUNT + 27),
        BREWING_STAND_MENU_TYPE_ID => Some(BREWING_STAND_SLOT_COUNT + 27),
        BLAST_FURNACE_MENU_TYPE_ID | FURNACE_MENU_TYPE_ID | SMOKER_MENU_TYPE_ID => {
            Some(FURNACE_SLOT_COUNT + 27)
        }
        GRINDSTONE_MENU_TYPE_ID => Some(GRINDSTONE_SLOT_COUNT + 27),
        HOPPER_MENU_TYPE_ID => Some(HOPPER_SLOT_COUNT + 27),
        LOOM_MENU_TYPE_ID => Some(LOOM_SLOT_COUNT + 27),
        MERCHANT_MENU_TYPE_ID => Some(MERCHANT_SLOT_COUNT + 27),
        SHULKER_BOX_MENU_TYPE_ID => Some(SHULKER_BOX_SLOT_COUNT + 27),
        SMITHING_MENU_TYPE_ID => Some(SMITHING_SLOT_COUNT + 27),
        CARTOGRAPHY_TABLE_MENU_TYPE_ID => Some(CARTOGRAPHY_TABLE_SLOT_COUNT + 27),
        STONECUTTER_MENU_TYPE_ID => Some(STONECUTTER_SLOT_COUNT + 27),
        LECTERN_MENU_TYPE_ID => None,
        _ => None,
    }
}

fn generic_container_rows(menu_type_id: i32) -> Option<u8> {
    (GENERIC_CONTAINER_FIRST_MENU_TYPE_ID..=GENERIC_CONTAINER_LAST_MENU_TYPE_ID)
        .contains(&menu_type_id)
        .then(|| (menu_type_id - GENERIC_CONTAINER_FIRST_MENU_TYPE_ID + 1) as u8)
}

fn furnace_screen_background(menu_type_id: i32) -> Option<InventoryScreenBackground> {
    match menu_type_id {
        BLAST_FURNACE_MENU_TYPE_ID => Some(InventoryScreenBackground::BlastFurnace),
        FURNACE_MENU_TYPE_ID => Some(InventoryScreenBackground::Furnace),
        SMOKER_MENU_TYPE_ID => Some(InventoryScreenBackground::Smoker),
        _ => None,
    }
}

fn generic_container_slot_layouts(rows: u8) -> Vec<InventorySlotLayout> {
    let rows = rows.clamp(1, 6);
    let row_count = i32::from(rows);
    let container_slot_count = i16::from(rows) * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW;
    let inventory_top = 18 + row_count * GENERIC_CONTAINER_ROW_HEIGHT + 13;
    let mut slots = Vec::with_capacity(container_slot_count as usize + 36);

    for y in 0..row_count {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: container_slot_count + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: inventory_top + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: container_slot_count + 27 + x as i16,
            x: 8 + x * 18,
            y: inventory_top + 58,
        });
    }

    slots
}

fn generic_3x3_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(GENERIC_3X3_SLOT_COUNT as usize + 36);
    for y in 0..GENERIC_3X3_SLOT_COLUMNS {
        for x in 0..GENERIC_3X3_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * GENERIC_3X3_SLOT_COLUMNS) as i16,
                x: 62 + x * 18,
                y: 17 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: GENERIC_3X3_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: GENERIC_3X3_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn crafter_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(CRAFTER_TOTAL_SLOT_COUNT as usize);
    for y in 0..CRAFTER_GRID_SLOT_COLUMNS {
        for x in 0..CRAFTER_GRID_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * CRAFTER_GRID_SLOT_COLUMNS) as i16,
                x: 26 + x * 18,
                y: 17 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: CRAFTER_GRID_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: CRAFTER_GRID_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }
    slots.push(InventorySlotLayout {
        slot_id: CRAFTER_RESULT_SLOT,
        x: 134,
        y: 35,
    });

    slots
}

fn crafting_table_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(CRAFTING_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 124,
        y: 35,
    });
    for y in 0..CRAFTING_GRID_SLOT_COLUMNS {
        for x in 0..CRAFTING_GRID_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (1 + x + y * CRAFTING_GRID_SLOT_COLUMNS) as i16,
                x: 30 + x * 18,
                y: 17 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: CRAFTING_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: CRAFTING_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn anvil_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(ANVIL_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 27,
        y: 47,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 76,
        y: 47,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 134,
        y: 47,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: ANVIL_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: ANVIL_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn enchantment_table_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(ENCHANTMENT_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 15,
        y: 47,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 35,
        y: 47,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: ENCHANTMENT_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: ENCHANTMENT_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn beacon_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(BEACON_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 136,
        y: 110,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: BEACON_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 36 + x * 18,
                y: 137 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: BEACON_SLOT_COUNT + 27 + x as i16,
            x: 36 + x * 18,
            y: 195,
        });
    }

    slots
}

fn brewing_stand_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(BREWING_STAND_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 56,
        y: 51,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 79,
        y: 58,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 102,
        y: 51,
    });
    slots.push(InventorySlotLayout {
        slot_id: 3,
        x: 79,
        y: 17,
    });
    slots.push(InventorySlotLayout {
        slot_id: 4,
        x: 17,
        y: 17,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: BREWING_STAND_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: BREWING_STAND_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn hopper_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(HOPPER_SLOT_COUNT as usize + 36);
    for x in 0..HOPPER_SLOT_COUNT {
        slots.push(InventorySlotLayout {
            slot_id: x,
            x: 44 + i32::from(x) * 18,
            y: 20,
        });
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: HOPPER_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 51 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: HOPPER_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 109,
        });
    }

    slots
}

fn clamped_mount_inventory_columns(inventory_columns: i32) -> u8 {
    inventory_columns.clamp(0, MOUNT_MAX_INVENTORY_COLUMNS) as u8
}

fn mount_inventory_slot_layouts(
    inventory_columns: u8,
    equipment_slots: MountEquipmentSlotVisibility,
) -> Vec<InventorySlotLayout> {
    let inventory_columns = i32::from(inventory_columns);
    let mount_inventory_slot_count = inventory_columns * MOUNT_INVENTORY_ROWS;
    let player_inventory_start =
        MOUNT_EQUIPMENT_SLOT_COUNT + i16::try_from(mount_inventory_slot_count).unwrap_or_default();
    let mut slots = Vec::with_capacity(
        MOUNT_EQUIPMENT_SLOT_COUNT as usize + mount_inventory_slot_count as usize + 36,
    );
    if equipment_slots.saddle {
        slots.push(InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        });
    }
    if equipment_slots.body.is_some() {
        slots.push(InventorySlotLayout {
            slot_id: 1,
            x: 8,
            y: 36,
        });
    }
    for y in 0..MOUNT_INVENTORY_ROWS {
        for x in 0..inventory_columns {
            slots.push(InventorySlotLayout {
                slot_id: MOUNT_EQUIPMENT_SLOT_COUNT
                    + i16::try_from(x + y * inventory_columns).unwrap_or_default(),
                x: 80 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: player_inventory_start + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: player_inventory_start + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn furnace_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(FURNACE_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 56,
        y: 17,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 56,
        y: 53,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 116,
        y: 35,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: FURNACE_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: FURNACE_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn grindstone_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(GRINDSTONE_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 49,
        y: 19,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 49,
        y: 40,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 129,
        y: 34,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: GRINDSTONE_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: GRINDSTONE_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn shulker_box_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(SHULKER_BOX_SLOT_COUNT as usize + 36);
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: SHULKER_BOX_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: SHULKER_BOX_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn loom_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(LOOM_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 13,
        y: 26,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 33,
        y: 26,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 23,
        y: 45,
    });
    slots.push(InventorySlotLayout {
        slot_id: 3,
        x: 143,
        y: 57,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: LOOM_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: LOOM_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn merchant_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(MERCHANT_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 136,
        y: 37,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 162,
        y: 37,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 220,
        y: 37,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: MERCHANT_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 108 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: MERCHANT_SLOT_COUNT + 27 + x as i16,
            x: 108 + x * 18,
            y: 142,
        });
    }

    slots
}

fn cartography_table_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(CARTOGRAPHY_TABLE_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 15,
        y: 15,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 15,
        y: 52,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 145,
        y: 39,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: CARTOGRAPHY_TABLE_SLOT_COUNT
                    + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: CARTOGRAPHY_TABLE_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn smithing_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(SMITHING_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 8,
        y: 48,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 26,
        y: 48,
    });
    slots.push(InventorySlotLayout {
        slot_id: 2,
        x: 44,
        y: 48,
    });
    slots.push(InventorySlotLayout {
        slot_id: 3,
        x: 98,
        y: 48,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: SMITHING_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: SMITHING_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

fn stonecutter_slot_layouts() -> Vec<InventorySlotLayout> {
    let mut slots = Vec::with_capacity(STONECUTTER_SLOT_COUNT as usize + 36);
    slots.push(InventorySlotLayout {
        slot_id: 0,
        x: 20,
        y: 33,
    });
    slots.push(InventorySlotLayout {
        slot_id: 1,
        x: 143,
        y: 33,
    });
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: STONECUTTER_SLOT_COUNT + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: STONECUTTER_SLOT_COUNT + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}
