use super::*;
use crate::entity::BoatData;
use bbb_server_types::item_stack::RuntimeItemStack;
pub(crate) fn write_container_items(
    entries: &mut Vec<NbtSummaryEntry>,
    slots: &[Option<RuntimeItemStack>],
    items: crate::entity::item::ItemResolver<'_>,
) {
    let saved = slots
        .iter()
        .enumerate()
        .filter_map(|(slot, stack)| {
            let NbtSummaryValue::Compound(stack_entries) = stack.as_ref()?.write(items).ok()??
            else {
                return None;
            };
            let mut fields = Vec::with_capacity(stack_entries.len() + 1);
            fields.push(NbtSummaryEntry {
                name: "Slot".into(),
                value: NbtSummaryValue::Byte(slot as i8),
            });
            fields.extend(stack_entries);
            Some(NbtSummaryValue::Compound(fields))
        })
        .collect();
    entries.push(NbtSummaryEntry {
        name: "Items".into(),
        value: NbtSummaryValue::List(saved),
    });
}

// ContainerHelper.java:40-45; TagValueInput.java:137-139, 403-426: a non-list
// `Items` is empty; unreadable or out-of-range stacks are dropped.
fn read_container_items(
    entries: &[NbtSummaryEntry],
    size: usize,
    items: crate::entity::item::ItemResolver<'_>,
) -> Vec<Option<RuntimeItemStack>> {
    let mut slots = vec![None; size];
    let Some(NbtSummaryValue::List(saved_items)) = optional_field(entries, "Items") else {
        return slots;
    };
    for saved in saved_items {
        let NbtSummaryValue::Compound(fields) = saved else {
            continue;
        };
        // ItemStackWithSlot.java:9-13: Codec.BYTE narrows numbers, UNSIGNED_BYTE then maps to 0..255.
        let slot = optional_field(fields, "Slot")
            .and_then(bbb_protocol::nbt_summary::codec_int_value)
            .unwrap_or(0) as i8 as u8 as usize;
        if let Ok(stack) = RuntimeItemStack::try_read(saved, items) {
            if !stack.is_empty() && slot < size {
                slots[slot] = Some(stack);
            }
        }
    }
    slots
}

pub(super) fn write_chest_vehicle_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    items: &[Option<RuntimeItemStack>],
    loot_table: Option<&str>,
    seed: i64,
    item_resolver: crate::entity::item::ItemResolver<'_>,
) {
    // ContainerEntity.java:58-67: pending loot omits every inventory stack.
    if let Some(table) = loot_table {
        entries.push(NbtSummaryEntry {
            name: "LootTable".into(),
            value: NbtSummaryValue::string(table),
        });
        if seed != 0 {
            entries.push(NbtSummaryEntry {
                name: "LootTableSeed".into(),
                value: NbtSummaryValue::Long(seed),
            });
        }
    } else {
        write_container_items(entries, items, item_resolver);
    }
}

// ContainerEntity.java:69-77: pending loot leaves the inventory empty.
fn read_chest_vehicle_state(
    entries: &[NbtSummaryEntry],
    size: usize,
    items: crate::entity::item::ItemResolver<'_>,
) -> (Vec<Option<RuntimeItemStack>>, Option<String>, i64) {
    let loot_table = match optional_field(entries, "LootTable") {
        Some(NbtSummaryValue::String(raw)) => raw
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
        _ => None,
    };
    let seed = optional_long(entries, "LootTableSeed").unwrap_or(0);
    let slots = if loot_table.is_some() {
        vec![None; size]
    } else {
        read_container_items(entries, size, items)
    };
    (slots, loot_table, seed)
}

pub(super) fn read_chest_boat(
    entries: &[NbtSummaryEntry],
    items: crate::entity::item::ItemResolver<'_>,
    entity_type_id: i32,
) -> ChestBoatData {
    let (slots, loot_table, seed) =
        read_chest_vehicle_state(entries, CHEST_BOAT_CONTAINER_SIZE, items);
    let boat = boat::read_save_data(
        entries,
        BoatData::with_catalog(
            bbb_protocol::entity_types::vanilla_entity_type(entity_type_id).unwrap(),
        ),
    );
    let mut data = chest_boat::ChestBoatData::from_parts(boat, slots)
        .expect("slots match the chest boat size");
    if let Some(id) = loot_table {
        data.set_loot_table(id, seed)
            .expect("normalized loot table id");
    }
    data.set_loot_table_seed(seed);
    data
}

pub(super) fn read_minecart_container(
    entries: &[NbtSummaryEntry],
    kind: MinecartContainerKind,
    items: crate::entity::item::ItemResolver<'_>,
) -> MinecartContainerData {
    let (slots, loot_table, seed) = read_chest_vehicle_state(entries, kind.container_size(), items);
    let minecart = minecart::read_save_data_with_default_offset_and_registry(
        entries,
        kind.default_display_offset(),
        items.content_registry(),
    );
    // `MinecartHopper.java:136-139`.
    let enabled =
        kind != MinecartContainerKind::Hopper || optional_bool(entries, "Enabled").unwrap_or(true);
    let mut data = MinecartContainerData::from_parts(kind, minecart, slots, enabled)
        .expect("slots match the minecart container size");
    if let Some(id) = loot_table {
        data.set_loot_table(id, seed)
            .expect("normalized loot table id");
    }
    data.set_loot_table_seed(seed);
    data
}

/// FallingBlockEntity.java:281-293.
pub(super) fn write_falling_block(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &FallingBlockData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(), EntitySaveError> {
    entries.extend([
        NbtSummaryEntry {
            name: "BlockState".into(),
            value: block_state_value_with_registry(
                data.block_state_id(),
                "BlockState",
                content_registry,
            )?,
        },
        NbtSummaryEntry {
            name: "Time".into(),
            value: NbtSummaryValue::Int(data.time()),
        },
        NbtSummaryEntry {
            name: "DropItem".into(),
            value: byte_bool(data.drop_item()),
        },
        NbtSummaryEntry {
            name: "HurtEntities".into(),
            value: byte_bool(data.hurt_entities()),
        },
        NbtSummaryEntry {
            name: "FallHurtAmount".into(),
            value: NbtSummaryValue::Float(data.fall_damage_per_distance().to_bits()),
        },
        NbtSummaryEntry {
            name: "FallHurtMax".into(),
            value: NbtSummaryValue::Int(data.fall_damage_max()),
        },
        NbtSummaryEntry {
            name: "CancelDrop".into(),
            value: byte_bool(data.cancel_drop()),
        },
    ]);
    if let Some(block_data) = data.block_data() {
        entries.push(NbtSummaryEntry {
            name: "TileEntityData".into(),
            value: NbtSummaryValue::Compound(block_data.to_vec()),
        });
    }
    Ok(())
}

/// `ItemEntity.addAdditionalSaveData` (`ItemEntity.java:314-323`).
///
/// `Health`, `Age` and `PickupDelay` are `int` fields narrowed to Java shorts
/// on the way out, which is why `INFINITE_PICKUP_DELAY` and
/// `INFINITE_LIFETIME` are the two short extremes; the narrowing is
/// transcribed rather than range-checked, so a value outside a short wraps
/// exactly as vanilla's cast does.
///
/// `Thrower` and `Owner` are written only when set (`EntityReference.store`,
/// `net/minecraft/world/entity/EntityReference.java:96-100`, and
/// `ValueOutput.storeNullable`), and `Item` only for a nonempty stack
/// (`:320-322`). `bobOffs` is not written at all: vanilla re-draws it in the
/// constructor every time the entity is loaded.
pub(super) fn write_item_entity(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &ItemEntityData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    entries.extend([
        NbtSummaryEntry {
            name: "Health".into(),
            value: NbtSummaryValue::Short(data.health() as i16),
        },
        NbtSummaryEntry {
            name: "Age".into(),
            value: NbtSummaryValue::Short(data.age() as i16),
        },
        NbtSummaryEntry {
            name: "PickupDelay".into(),
            value: NbtSummaryValue::Short(data.pickup_delay() as i16),
        },
    ]);
    if let Some(thrower) = data.thrower() {
        entries.push(NbtSummaryEntry {
            name: "Thrower".into(),
            value: uuid_int_array(thrower),
        });
    }
    if let Some(target) = data.target() {
        entries.push(NbtSummaryEntry {
            name: "Owner".into(),
            value: uuid_int_array(target),
        });
    }
    if !data.item().is_empty() {
        entries.push(NbtSummaryEntry {
            name: "Item".into(),
            value: crate::entity::item::write_saved_stack(data.item(), items),
        });
    }
}

/// `ExperienceOrb.addAdditionalSaveData` (`ExperienceOrb.java:259-264`).
///
/// `Health`, `Age` and `Value` are `int` fields narrowed to Java shorts on the
/// way out, and `Count` is the one that stays an `int` -- which is why the
/// entity-region schema this crate reads through requires exactly that
/// shape, and why `Count` is the only one of the four that can hold more than
/// a short.
pub(crate) fn write_block_pos(pos: bbb_protocol::packets::BlockPos) -> NbtSummaryValue {
    NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z])
}

pub(super) fn write_painting(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &crate::entity::PaintingData,
) {
    entries.push(NbtSummaryEntry {
        name: "block_pos".into(),
        value: write_block_pos(data.pose().attached()),
    });
    entries.push(NbtSummaryEntry {
        name: "facing".into(),
        value: NbtSummaryValue::Byte(crate::entity::hanging::horizontal_id(
            data.pose().direction(),
        )),
    });
    entries.push(NbtSummaryEntry {
        name: "variant".into(),
        value: NbtSummaryValue::string(data.variant().resource_id()),
    });
}

pub(super) fn write_item_frame(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &crate::entity::ItemFrameData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    entries.push(NbtSummaryEntry {
        name: "block_pos".into(),
        value: write_block_pos(data.pose().attached()),
    });
    if let Some(item) = data.item() {
        entries.push(NbtSummaryEntry {
            name: "Item".into(),
            value: crate::entity::item::write_saved_stack(item, items),
        });
    }
    entries.extend([
        NbtSummaryEntry {
            name: "ItemRotation".into(),
            value: NbtSummaryValue::Byte(data.rotation() as i8),
        },
        NbtSummaryEntry {
            name: "ItemDropChance".into(),
            value: NbtSummaryValue::Float(bbb_protocol::nbt_summary::canonical_float_tag_bits(
                data.drop_chance(),
            )),
        },
        NbtSummaryEntry {
            name: "Facing".into(),
            value: NbtSummaryValue::Byte(data.pose().direction().id() as i8),
        },
        NbtSummaryEntry {
            name: "Invisible".into(),
            value: byte_bool(data.invisible()),
        },
        NbtSummaryEntry {
            name: "Fixed".into(),
            value: byte_bool(data.fixed()),
        },
    ]);
}

// BlockAttachedEntity.java:124-131: a missing or distant `block_pos` keeps the loaded block position.
fn read_attached_pos(
    entries: &[NbtSummaryEntry],
    position: [f64; 3],
) -> bbb_protocol::packets::BlockPos {
    let initial = bbb_protocol::packets::BlockPos {
        x: position[0].clamp(-30_000_512.0, 30_000_512.0).floor() as i32,
        y: position[1].clamp(-20_000_000.0, 20_000_000.0).floor() as i32,
        z: position[2].clamp(-30_000_512.0, 30_000_512.0).floor() as i32,
    };
    optional_block_pos(entries, "block_pos")
        .filter(|pos| {
            let dx = f64::from(pos.x) - f64::from(initial.x);
            let dy = f64::from(pos.y) - f64::from(initial.y);
            let dz = f64::from(pos.z) - f64::from(initial.z);
            dx * dx + dy * dy + dz * dz < 256.0
        })
        .unwrap_or(initial)
}

/// `Painting.readAdditionalSaveData` (`Painting.java:143-148`).
pub(super) fn read_painting(
    entries: &[NbtSummaryEntry],
    position: [f64; 3],
    command_existing: Option<&Entity>,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> crate::entity::PaintingData {
    use crate::entity::hanging::PaintingVariant;
    let facing = optional_field(entries, "facing")
        .and_then(bbb_protocol::nbt_summary::codec_int_value)
        .map_or(Direction::South, |id| {
            crate::entity::hanging::from_horizontal_id(id as i8)
        });
    let variant = match optional_field(entries, "variant") {
        Some(NbtSummaryValue::String(variant)) => variant
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
            .and_then(|variant| PaintingVariant::resolve(items, &variant)),
        _ => None,
    }
    .or_else(|| match command_existing.map(Entity::data) {
        Some(EntityData::Painting(data)) => Some(data.variant().clone()),
        _ => None,
    })
    .unwrap_or_else(|| PaintingVariant::default_variant(items));
    crate::entity::PaintingData::new(read_attached_pos(entries, position), facing, variant)
}

/// `ItemFrame.readAdditionalSaveData` (`ItemFrame.java:341-354`).
pub(super) fn read_item_frame(
    entries: &[NbtSummaryEntry],
    position: [f64; 3],
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> crate::entity::ItemFrameData {
    // Direction.java:45: Codec.BYTE uses the boxed Number, without NumericTag's floor.
    let facing = optional_field(entries, "Facing")
        .and_then(bbb_protocol::nbt_summary::codec_int_value)
        .map_or(Direction::Down, |id| {
            crate::entity::hanging::from_3d_id(id as i8)
        });
    let item = optional_field(entries, "Item")
        .and_then(|value| crate::entity::item::read_saved_stack(value, items))
        .and_then(normalize_item_frame_stack);
    crate::entity::ItemFrameData::new(read_attached_pos(entries, position), facing)
        .with_saved_fields(
            item,
            i32::from(optional_byte(entries, "ItemRotation").unwrap_or(0)),
            optional_float(entries, "ItemDropChance").unwrap_or(1.0),
            optional_bool(entries, "Fixed").unwrap_or(false),
            optional_bool(entries, "Invisible").unwrap_or(false),
        )
}

fn normalize_item_frame_stack(
    stack: bbb_server_types::item_stack::RuntimeItemStack,
) -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
    (!stack.is_empty()).then(|| stack.with_count(1))
}

pub(super) fn write_experience_orb(entries: &mut Vec<NbtSummaryEntry>, data: &ExperienceOrbData) {
    entries.extend([
        NbtSummaryEntry {
            name: "Health".into(),
            value: NbtSummaryValue::Short(data.health() as i16),
        },
        NbtSummaryEntry {
            name: "Age".into(),
            value: NbtSummaryValue::Short(data.age() as i16),
        },
        NbtSummaryEntry {
            name: "Value".into(),
            value: NbtSummaryValue::Short(data.value() as i16),
        },
        NbtSummaryEntry {
            name: "Count".into(),
            value: NbtSummaryValue::Int(data.count()),
        },
    ]);
}

/// `PrimedTnt.addAdditionalSaveData` (`PrimedTnt.java:139-147`).
pub(super) fn write_tnt(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &TntData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(), EntitySaveError> {
    entries.push(NbtSummaryEntry {
        name: "fuse".into(),
        value: NbtSummaryValue::Short(data.fuse() as i16),
    });
    entries.push(NbtSummaryEntry {
        name: "block_state".into(),
        value: block_state_value_with_registry(
            data.block_state_id(),
            "block_state",
            content_registry,
        )?,
    });
    // `:142-144`: the default 4.0 is omitted.
    if data.explosion_power() != crate::entity::tnt::DEFAULT_EXPLOSION_POWER {
        entries.push(NbtSummaryEntry {
            name: "explosion_power".into(),
            value: NbtSummaryValue::Float(data.explosion_power().to_bits()),
        });
    }
    if let Some(owner) = data.owner() {
        entries.push(NbtSummaryEntry {
            name: "owner".into(),
            value: uuid_int_array(owner),
        });
    }
    Ok(())
}

/// `PrimedTnt.readAdditionalSaveData` (`PrimedTnt.java:150-155`).
pub(super) fn read_tnt(
    entries: &[NbtSummaryEntry],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> TntData {
    // `:151`: `getShortOr("fuse", 80)`.
    let fuse =
        optional_short(entries, "fuse").unwrap_or(crate::entity::tnt::DEFAULT_FUSE_TIME as i16);
    // `:152`: `read("block_state", BlockState.CODEC).orElse(DEFAULT_BLOCK_STATE)`.
    let block_state_id = optional_field(entries, "block_state")
        .and_then(|value| read_block_state_from_value_with_registry(value, content_registry))
        .unwrap_or_else(crate::entity::tnt::default_block_state_id);
    // `:153`: `Mth.clamp(getFloatOr("explosion_power", 4.0), 0.0, 128.0)`.
    let explosion_power = optional_float(entries, "explosion_power")
        .unwrap_or(crate::entity::tnt::DEFAULT_EXPLOSION_POWER)
        .clamp(0.0, crate::entity::tnt::MAX_EXPLOSION_POWER);
    let owner = optional_uuid(entries, "owner");
    TntData::with_saved_fields(i32::from(fuse), block_state_id, owner, explosion_power)
}

/// `BlockState.CODEC` (`Blocks.java` state) as the `{Name, Properties}`
/// compound `bbb_server_persistence` reads back, under `key`.
pub(crate) fn block_state_value_with_registry(
    block_state_id: i32,
    key: &'static str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, EntitySaveError> {
    if let Some(identity) = vanilla_block_state_identity(block_state_id) {
        let mut state_entries = vec![NbtSummaryEntry {
            name: "Name".into(),
            value: NbtSummaryValue::string(identity.name.clone()),
        }];
        if !identity.properties.is_empty() {
            state_entries.push(NbtSummaryEntry {
                name: "Properties".into(),
                value: NbtSummaryValue::Compound(
                    identity
                        .properties
                        .iter()
                        .map(|(name, value)| NbtSummaryEntry {
                            name: name.clone().into(),
                            value: NbtSummaryValue::string(value.clone()),
                        })
                        .collect(),
                ),
            });
        }
        return Ok(NbtSummaryValue::Compound(state_entries));
    }
    if let Some((custom, offset)) = content_registry.block_by_state_id(block_state_id) {
        let mut state_entries = vec![NbtSummaryEntry {
            name: "Name".into(),
            value: NbtSummaryValue::string(custom.resource_id().to_owned()),
        }];
        if custom.state_count > 1 || offset > 0 {
            state_entries.push(NbtSummaryEntry {
                name: "Properties".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY.into(),
                    value: NbtSummaryValue::string(offset.to_string()),
                }]),
            });
        }
        return Ok(NbtSummaryValue::Compound(state_entries));
    }
    Err(field_error(
        key,
        "block state is outside the built-in 26.1 registry",
    ))
}

/// `ItemEntity.readAdditionalSaveData` (`ItemEntity.java:326-336`): an
/// undecodable `Item` is empty, which the caller discards.
pub(super) fn read_item_entity(
    entries: &[NbtSummaryEntry],
    bob_offs: f32,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> ItemEntityData {
    let stack = optional_field(entries, "Item")
        .and_then(|value| item::read_saved_stack(value, items))
        .unwrap_or_default();
    ItemEntityData::new(stack, bob_offs).with_saved_fields(
        i32::from(optional_short(entries, "Health").unwrap_or(item::DEFAULT_HEALTH as i16)),
        i32::from(optional_short(entries, "Age").unwrap_or(0)),
        i32::from(optional_short(entries, "PickupDelay").unwrap_or(0)),
        optional_uuid(entries, "Thrower"),
        optional_uuid(entries, "Owner"),
    )
}

/// `Entity.load` (`Entity.java:2048-2104`) followed by the subtype's
/// `readAdditionalSaveData`, into a store slot.
/// `Entity.load`'s motion clamp (`Entity.java:2053-2055`).
pub(super) fn clamp_loaded_motion(component: f64) -> f64 {
    if !component.is_finite() || component.abs() > 10.0 {
        0.0
    } else {
        component
    }
}

/// `FallingBlockEntity.readAdditionalSaveData`
/// (`FallingBlockEntity.java:296-306`).
pub(super) fn read_falling_block(
    entries: &[NbtSummaryEntry],
    registry_tag_membership: &mut impl FnMut(&str, &str, &str) -> bool,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> FallingBlockData {
    let block_state_id = optional_field(entries, "BlockState")
        .and_then(|value| read_block_state_from_value_with_registry(value, content_registry))
        .unwrap_or_else(|| {
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_SAND_ID,
            )
            .expect("sand has a default state")
        });
    // The start position is a synched value, not a saved one
    // (`FallingBlockEntity.java:73`, `:118-124`), so a loaded entity has no
    // record of where it started. Vanilla leaves `DATA_START_POS` at
    // `BlockPos.ZERO` (`:133`) in exactly the same situation.
    let data = FallingBlockData::new(
        block_state_id,
        bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
    );
    let hurt_entities = optional_bool(entries, "HurtEntities").unwrap_or_else(|| {
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
            .and_then(bbb_protocol::block_types::vanilla_block_type)
            .is_some_and(|block| {
                registry_tag_membership("minecraft:block", "minecraft:anvil", block.resource_id)
            })
    });
    data.with_saved_fields(
        optional_int(entries, "Time").unwrap_or(0),
        optional_bool(entries, "DropItem").unwrap_or(true),
        optional_bool(entries, "CancelDrop").unwrap_or(false),
        hurt_entities,
        optional_float(entries, "FallHurtAmount").unwrap_or(0.0),
        optional_int(entries, "FallHurtMax").unwrap_or(40),
    )
    .with_block_data(match optional_field(entries, "TileEntityData") {
        Some(NbtSummaryValue::Compound(entries)) => Some(entries.clone()),
        _ => None,
    })
}

/// `OminousItemSpawner.java:114-117`.
pub(super) fn read_ominous_item_spawner(
    entries: &[NbtSummaryEntry],
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> crate::entity::OminousItemSpawnerData {
    let item = optional_field(entries, "item")
        .and_then(|value| item::read_saved_stack(value, items))
        .unwrap_or_default();
    crate::entity::OminousItemSpawnerData::from_saved(
        item,
        optional_long(entries, "spawn_item_after_ticks").unwrap_or(0),
    )
}

#[cfg(test)]
mod item_frame_tests {
    use super::*;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn base_fields() -> Vec<NbtSummaryEntry> {
        vec![entry(
            "block_pos",
            NbtSummaryValue::IntArray(vec![0, 64, 0]),
        )]
    }

    fn stack(id: &str, count: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(id)),
            entry("count", NbtSummaryValue::Int(count)),
        ])
    }

    #[test]
    fn item_frame_load_normalizes_count_and_preserves_components() {
        let components = NbtSummaryValue::Compound(vec![entry(
            "minecraft:custom_data",
            NbtSummaryValue::Compound(vec![entry("answer", NbtSummaryValue::Int(42))]),
        )]);
        let mut item_fields = match stack("minecraft:stone", 64) {
            NbtSummaryValue::Compound(fields) => fields,
            _ => unreachable!(),
        };
        item_fields.push(entry("components", components.clone()));
        let mut fields = base_fields();
        fields.push(entry("Item", NbtSummaryValue::Compound(item_fields)));
        let frame = read_item_frame(
            &fields,
            [0.5, 64.0, 0.5],
            &bbb_registry_codec::UnavailableHolderResolver,
        );
        let item = frame.item().expect("non-empty saved item");
        assert_eq!(item.count(), 1);
        let NbtSummaryValue::Compound(raw) = item
            .write(&bbb_registry_codec::UnavailableHolderResolver)
            .unwrap()
            .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(
            raw.iter()
                .find(|field| field.name == "components")
                .map(|field| &field.value),
            Some(&components)
        );
    }

    #[test]
    fn item_frame_load_maps_every_empty_stack_to_none() {
        for item in [
            NbtSummaryValue::Compound(Vec::new()),
            stack("minecraft:air", 1),
        ] {
            let mut fields = base_fields();
            fields.push(entry("Item", item));
            assert!(read_item_frame(
                &fields,
                [0.5, 64.0, 0.5],
                &bbb_registry_codec::UnavailableHolderResolver
            )
            .item()
            .is_none());
        }
    }

    #[test]
    fn item_frame_load_defaults_missing_facing_and_rotation() {
        let frame = read_item_frame(
            &base_fields(),
            [0.5, 64.0, 0.5],
            &bbb_registry_codec::UnavailableHolderResolver,
        );
        assert_eq!(frame.pose().direction(), Direction::Down);
        assert_eq!(frame.rotation(), 0);
    }
}
