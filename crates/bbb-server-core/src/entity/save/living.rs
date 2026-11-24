use super::*;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::nbt_summary::compound_last_field;

#[cfg(test)]
mod impulse_tests;
#[cfg(test)]
mod sleep_tests;

pub fn read_player_living_codec_fields(
    attributes: Option<&NbtSummaryValue>,
    effects: Option<&NbtSummaryValue>,
) -> (
    Vec<PackedAttribute>,
    crate::entity::living::mob_effects::ActiveMobEffects,
) {
    let attributes = attributes
        .and_then(PackedAttribute::read_saved_list_codec)
        .unwrap_or_default();
    let effects = effects
        .into_iter()
        .flat_map(crate::entity::living::mob_effects::saved_codec::decode_saved_effect_list);
    (
        attributes,
        crate::entity::living::mob_effects::ActiveMobEffects::from_decoded_saved_effects(effects),
    )
}

fn impulse_impact_position(value: &NbtSummaryValue) -> Option<Vec3d> {
    fn first_three(mut numbers: impl Iterator<Item = f64>) -> Option<Vec3d> {
        Some(Vec3d {
            x: numbers.next()?,
            y: numbers.next()?,
            z: numbers.next()?,
        })
    }
    match value {
        NbtSummaryValue::List(values) => first_three(values.iter().filter_map(command_number_f64)),
        NbtSummaryValue::ByteArray(values) => {
            first_three(values.iter().map(|value| f64::from(*value)))
        }
        NbtSummaryValue::IntArray(values) => {
            first_three(values.iter().map(|value| f64::from(*value)))
        }
        NbtSummaryValue::LongArray(values) => first_three(values.iter().map(|value| *value as f64)),
        _ => None,
    }
}

pub(super) fn write_living_entity(
    entries: &mut Vec<NbtSummaryEntry>,
    entity: &Entity,
    living: &crate::entity::LivingEntityState,
    write_empty_brain: bool,
    store: Option<&EntityStore>,
    items: crate::entity::item::ItemResolver<'_>,
) {
    entries.extend([
        NbtSummaryEntry {
            name: "Health".into(),
            value: NbtSummaryValue::Float(living.health().to_bits()),
        },
        NbtSummaryEntry {
            name: "HurtTime".into(),
            value: NbtSummaryValue::Short(living.hurt_time as i16),
        },
        NbtSummaryEntry {
            name: "HurtByTimestamp".into(),
            value: NbtSummaryValue::Int(living.last_hurt_by_mob_timestamp),
        },
        NbtSummaryEntry {
            name: "DeathTime".into(),
            value: NbtSummaryValue::Short(living.death_time as i16),
        },
        NbtSummaryEntry {
            name: "AbsorptionAmount".into(),
            value: NbtSummaryValue::Float(living.absorption_amount().to_bits()),
        },
        NbtSummaryEntry {
            name: "current_impulse_context_reset_grace_time".into(),
            value: NbtSummaryValue::Int(living.fall_impulse.reset_grace_time),
        },
        NbtSummaryEntry {
            name: "attributes".into(),
            value: NbtSummaryValue::List(
                living
                    .packed_attributes()
                    .iter()
                    .map(PackedAttribute::to_saved_nbt)
                    .collect(),
            ),
        },
        NbtSummaryEntry {
            name: "FallFlying".into(),
            value: byte_bool(entity.is_fall_flying()),
        },
    ]);
    if let Some(position) = living.fall_impulse.impact_position {
        entries.push(NbtSummaryEntry {
            name: "current_explosion_impact_pos".into(),
            value: double_list(position),
        });
    }
    if let Some(position) = entity.sleeping_pos() {
        entries.push(NbtSummaryEntry {
            name: "sleeping_pos".into(),
            value: write_block_pos(position),
        });
    }
    if write_empty_brain {
        entries.push(NbtSummaryEntry {
            name: "Brain".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "memories".into(),
                value: NbtSummaryValue::Compound(Vec::new()),
            }]),
        });
    }
    if let Some(player) = living.last_hurt_by_player {
        entries.push(NbtSummaryEntry {
            name: "last_hurt_by_player".into(),
            value: uuid_int_array(player),
        });
        entries.push(NbtSummaryEntry {
            name: "last_hurt_by_player_memory_time".into(),
            value: NbtSummaryValue::Int(living.last_hurt_by_player_memory_time),
        });
    }
    if let Some(uuid) = last_hurt_by_mob_uuid(living.last_hurt_by_mob(), store) {
        entries.push(NbtSummaryEntry {
            name: "last_hurt_by_mob".into(),
            value: uuid_int_array(uuid),
        });
        entries.push(NbtSummaryEntry {
            name: "ticks_since_last_hurt_by_mob".into(),
            value: NbtSummaryValue::Int(entity.tick_count() - living.last_hurt_by_mob_timestamp),
        });
    }
    let active_effects = living
        .active_effects()
        .map(write_active_mob_effect)
        .collect::<Vec<_>>();
    if !active_effects.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "active_effects".into(),
            value: NbtSummaryValue::List(active_effects),
        });
    }
    let equipment: Vec<NbtSummaryEntry> = living
        .equipment()
        .iter()
        .filter(|(_, stack)| !stack.is_empty())
        .map(|(slot, stack)| NbtSummaryEntry {
            name: slot_serialized_name(slot).into(),
            value: crate::entity::item::write_saved_stack(stack, items),
        })
        .collect();
    if !equipment.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "equipment".into(),
            value: NbtSummaryValue::Compound(equipment),
        });
    }
    let icon = living.waypoint_icon();
    if icon.style != "minecraft:default" || icon.color_rgb.is_some() {
        let mut fields = vec![NbtSummaryEntry {
            name: "style".into(),
            value: NbtSummaryValue::string(icon.style.clone()),
        }];
        if let Some(color) = icon.color_rgb {
            fields.push(NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::Int(color as i32),
            });
        }
        entries.push(NbtSummaryEntry {
            name: "locator_bar_icon".into(),
            value: NbtSummaryValue::Compound(fields),
        });
    }
}

fn last_hurt_by_mob_uuid(
    reference: Option<crate::entity::LastHurtByMob>,
    store: Option<&EntityStore>,
) -> Option<Uuid> {
    let reference = reference?;
    reference.uuid().or_else(|| {
        reference
            .stored_entity_id()
            .and_then(|entity_id| store.and_then(|store| store.get(entity_id)))
            .map(Entity::uuid)
    })
}

pub(super) fn write_armor_stand(
    entries: &mut Vec<NbtSummaryEntry>,
    entity: &Entity,
    data: &ArmorStandData,
    store: Option<&EntityStore>,
    items: crate::entity::item::ItemResolver<'_>,
) {
    write_living_entity(entries, entity, data.living(), true, store, items);
    entries.extend([
        NbtSummaryEntry {
            name: "Invisible".into(),
            value: byte_bool(entity.is_invisible()),
        },
        NbtSummaryEntry {
            name: "Small".into(),
            value: byte_bool(data.is_small()),
        },
        NbtSummaryEntry {
            name: "ShowArms".into(),
            value: byte_bool(data.show_arms()),
        },
        NbtSummaryEntry {
            name: "DisabledSlots".into(),
            value: NbtSummaryValue::Int(data.disabled_slots()),
        },
        NbtSummaryEntry {
            name: "NoBasePlate".into(),
            value: byte_bool(!data.show_base_plate()),
        },
    ]);
    if data.is_marker() {
        entries.push(NbtSummaryEntry {
            name: "Marker".into(),
            value: byte_bool(true),
        });
    }
    let pose: Vec<NbtSummaryEntry> = data
        .pose()
        .members()
        .into_iter()
        .filter(|(_, rotations, default)| rotations != default)
        .map(|(name, rotations, _)| NbtSummaryEntry {
            name: name.into(),
            value: rotations_record(rotations),
        })
        .collect();
    entries.push(NbtSummaryEntry {
        name: "Pose".into(),
        value: NbtSummaryValue::Compound(pose),
    });
}

pub(super) fn rotations_record(rotations: Rotations) -> NbtSummaryValue {
    NbtSummaryValue::List(
        rotations
            .components()
            .into_iter()
            .map(|value| NbtSummaryValue::Float(value.to_bits()))
            .collect(),
    )
}

pub(super) fn slot_serialized_name(
    slot: bbb_protocol::equipment_slots::EquipmentSlot,
) -> &'static str {
    bbb_protocol::equipment_slots::vanilla_equipment_slot_by_ordinal(slot.ordinal())
        .map_or("", |entry| entry.serialized_name)
}

fn read_pose(members: &[NbtSummaryEntry]) -> ArmorStandPose {
    let mut pose = ArmorStandPose::DEFAULT;
    for (name, slot) in [
        ("Head", &mut pose.head),
        ("Body", &mut pose.body),
        ("LeftArm", &mut pose.left_arm),
        ("RightArm", &mut pose.right_arm),
        ("LeftLeg", &mut pose.left_leg),
        ("RightLeg", &mut pose.right_leg),
    ] {
        let Some(NbtSummaryValue::List(values)) = optional_field(members, name) else {
            continue;
        };
        let [x, y, z] = values.as_slice() else {
            continue;
        };
        if let (Some(x), Some(y), Some(z)) = (numeric_float(x), numeric_float(y), numeric_float(z))
        {
            *slot = Rotations::new(x, y, z);
        }
    }
    pose
}

pub(super) fn load_living_entity(
    entity: &mut Entity,
    entries: &[NbtSummaryEntry],
    level: Option<&LevelCtx<'_, '_>>,
    items: crate::entity::item::ItemResolver<'_>,
) {
    let tick_count = entity.tick_count();
    let living = entity.living_mut().unwrap();
    living
        .internal_set_absorption_amount(optional_float(entries, "AbsorptionAmount").unwrap_or(0.0));
    // `:779-781`.
    let attributes = compound_last_field(entries, "attributes")
        .and_then(PackedAttribute::read_saved_list_codec)
        .unwrap_or_default();
    living.apply_packed_attributes(&attributes);
    // `:791`.
    let health = optional_float(entries, "Health").unwrap_or_else(|| living.max_health());
    crate::entity::living::set_health(entity, health);
    let living = entity.living_mut().unwrap();
    // `:792-794`; `:814-815` then file the reference and restamp
    // `lastHurtByMobTimestamp` from `ticks_since_last_hurt_by_mob` below.
    living.hurt_time = i32::from(optional_short(entries, "HurtTime").unwrap_or(0));
    living.death_time = i32::from(optional_short(entries, "DeathTime").unwrap_or(0));
    // `:803`.
    entity.set_fall_flying(optional_bool(entries, "FallFlying").unwrap_or(false));
    let sleeping_pos = optional_field(entries, "sleeping_pos").and_then(read_block_pos_codec_input);
    entity.set_sleeping_pos(sleeping_pos);
    if let Some(position) = sleeping_pos {
        let _ = entity.set_pose_id_with_context(
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID,
            level,
            None,
        );
        if !entity.is_first_tick() {
            entity.set_pos(Vec3d {
                x: f64::from(position.x) + 0.5,
                y: f64::from(position.y) + 0.6875,
                z: f64::from(position.z) + 0.5,
            });
        }
    }
    let living = entity.living_mut().unwrap();
    // `:812-813`.
    living.last_hurt_by_player = optional_uuid(entries, "last_hurt_by_player");
    living.last_hurt_by_player_memory_time =
        optional_int(entries, "last_hurt_by_player_memory_time").unwrap_or(0);
    // `:814-815`: `EntityReference.read` keeps the UUID, which the level's
    // own lookup resolves on every read, and the saved age is measured
    // against the tick count the reloaded entity starts from.
    living.set_last_hurt_by_mob(
        optional_uuid(entries, "last_hurt_by_mob").map(crate::entity::LastHurtByMob::Uuid),
        optional_int(entries, "ticks_since_last_hurt_by_mob").unwrap_or(0) + tick_count,
    );
    // `:816`.
    let equipment = living.equipment_mut();
    equipment.clear();
    // `EntityEquipment.CODEC`: entries with an unknown slot or bad stack are dropped.
    if let Some(NbtSummaryValue::Compound(slots)) = optional_field(entries, "equipment") {
        for (slot, stack) in slots.iter().filter_map(|entry| {
            let slot = bbb_protocol::equipment_slots::vanilla_equipment_slot_for_serialized_name(
                &entry.name.try_to_string().ok()?,
            )?
            .slot;
            Some((slot, item::read_saved_stack(&entry.value, items)?))
        }) {
            equipment.set(slot, Some(stack));
        }
    }
    // `:818-819`.
    living.fall_impulse = crate::entity::living::FallImpulseContext {
        reset_grace_time: optional_int(entries, "current_impulse_context_reset_grace_time")
            .unwrap_or(0),
        impact_position: optional_field(entries, "current_explosion_impact_pos")
            .and_then(impulse_impact_position),
    };
    let active_effects = compound_last_field(entries, "active_effects")
        .into_iter()
        .flat_map(crate::entity::living::mob_effects::saved_codec::decode_saved_effect_list);
    living.replace_active_effects(active_effects);
    // `LivingEntity.readAdditionalSaveData` / `Waypoint.Icon.CODEC` (`Waypoint.java:36-42`):
    // mask ARGB ints to 24-bit RGB, accept 3-float RGB lists, and normalize `style`.
    let icon = match optional_field(entries, "locator_bar_icon") {
        Some(NbtSummaryValue::Compound(fields)) => fields.as_slice(),
        _ => &[],
    };
    let target = living.waypoint_icon_mut();
    target.color_rgb = parse_waypoint_icon_color(optional_field(icon, "color"));
    match optional_field(icon, "style") {
        Some(NbtSummaryValue::String(style)) => {
            const MAX_WAYPOINT_STYLE_LEN: usize = 256;
            target.style = style
                .try_to_string()
                .ok()
                .filter(|raw| raw.len() <= MAX_WAYPOINT_STYLE_LEN)
                .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
                .filter(|normalized| normalized.len() <= MAX_WAYPOINT_STYLE_LEN)
                .unwrap_or_else(|| "minecraft:default".into());
        }
        _ => "minecraft:default".clone_into(&mut target.style),
    }
}

/// `ExtraCodecs.RGB_COLOR_CODEC`: packed int masked to 24-bit RGB or 3-element `[r, g, b]` float list.
fn parse_waypoint_icon_color(value: Option<&NbtSummaryValue>) -> Option<u32> {
    match value? {
        NbtSummaryValue::Int(color) => Some((*color as u32) & 0x00ff_ffff),
        NbtSummaryValue::Byte(color) => Some((i32::from(*color) as u32) & 0x00ff_ffff),
        NbtSummaryValue::Short(color) => Some((i32::from(*color) as u32) & 0x00ff_ffff),
        NbtSummaryValue::Long(color) => Some((*color as u32) & 0x00ff_ffff),
        NbtSummaryValue::List(values) if values.len() == 3 => {
            let channel = |entry: &NbtSummaryValue| -> Option<u32> {
                let raw = match entry {
                    NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
                    NbtSummaryValue::Double(bits) => f64::from_bits(*bits),
                    NbtSummaryValue::Byte(v) => f64::from(*v),
                    NbtSummaryValue::Short(v) => f64::from(*v),
                    NbtSummaryValue::Int(v) => f64::from(*v),
                    NbtSummaryValue::Long(v) => *v as f64,
                    _ => return None,
                };
                if !raw.is_finite() {
                    return None;
                }
                Some((raw.clamp(0.0, 1.0) * 255.0).floor() as u32)
            };
            let r = channel(&values[0])?;
            let g = channel(&values[1])?;
            let b = channel(&values[2])?;
            Some((r << 16) | (g << 8) | b)
        }
        _ => None,
    }
}

pub(super) fn load_armor_stand(
    entity: &mut Entity,
    entries: &[NbtSummaryEntry],
    level: Option<&LevelCtx<'_, '_>>,
) {
    armor_stand::set_invisible(entity, optional_bool(entries, "Invisible").unwrap_or(false));
    let previous = [entity.bb_width(), entity.bb_height()];
    armor_stand::set_small(entity, optional_bool(entries, "Small").unwrap_or(false));
    let _ = crate::entity::living::resolve_dimensions_change(entity, previous, level, None);
    let previous = [entity.bb_width(), entity.bb_height()];
    armor_stand::set_show_arms(entity, optional_bool(entries, "ShowArms").unwrap_or(false));
    let _ = crate::entity::living::resolve_dimensions_change(entity, previous, level, None);
    armor_stand::set_disabled_slots(entity, optional_int(entries, "DisabledSlots").unwrap_or(0));
    let previous = [entity.bb_width(), entity.bb_height()];
    armor_stand::set_no_base_plate(
        entity,
        optional_bool(entries, "NoBasePlate").unwrap_or(false),
    );
    let _ = crate::entity::living::resolve_dimensions_change(entity, previous, level, None);
    let previous = [entity.bb_width(), entity.bb_height()];
    let marker = optional_bool(entries, "Marker").unwrap_or(false);
    armor_stand::set_marker(entity, marker);
    let _ = crate::entity::living::resolve_dimensions_change(entity, previous, level, None);
    entity.set_no_physics(marker || entity.no_gravity());
    if let Some(NbtSummaryValue::Compound(pose)) = optional_field(entries, "Pose") {
        armor_stand::set_armor_stand_pose(entity, read_pose(pose));
    }
}

/// The `Entity` and `LivingEntity` save fields the shared readers own.
pub(crate) fn is_living_base_save_field(name: &str) -> bool {
    matches!(
        name,
        "id" | "Pos"
            | "Motion"
            | "Rotation"
            | "fall_distance"
            | "Fire"
            | "Air"
            | "OnGround"
            | "Invulnerable"
            | "PortalCooldown"
            | "UUID"
            | "CustomName"
            | "CustomNameVisible"
            | "Silent"
            | "NoGravity"
            | "Glowing"
            | "TicksFrozen"
            | "HasVisualFire"
            | "Tags"
            | "data"
            | "Passengers"
            | "Health"
            | "HurtTime"
            | "HurtByTimestamp"
            | "DeathTime"
            | "AbsorptionAmount"
            | "current_impulse_context_reset_grace_time"
            | "current_explosion_impact_pos"
            | "attributes"
            | "FallFlying"
            | "sleeping_pos"
            | "last_hurt_by_player"
            | "last_hurt_by_player_memory_time"
            | "last_hurt_by_mob"
            | "ticks_since_last_hurt_by_mob"
            | "equipment"
            | "active_effects"
            | "locator_bar_icon"
    )
}

pub(crate) fn read_mob_retained_state(entries: &[NbtSummaryEntry]) -> MobRetainedState {
    let mut mob = MobRetainedState::default();
    mob.leash = crate::entity::leash::read_leash_data(entries);
    mob.home_radius = optional_int(entries, "home_radius").unwrap_or(-1);
    if mob.home_radius >= 0 {
        mob.home_position = crate::entity::save::optional_block_pos(entries, "home_pos")
            .unwrap_or(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
    }
    mob.set_can_pick_up_loot(optional_bool(entries, "CanPickUpLoot").unwrap_or(false));
    mob.set_persistence_required(optional_bool(entries, "PersistenceRequired").unwrap_or(false));
    mob.set_left_handed(optional_bool(entries, "LeftHanded").unwrap_or(false));
    mob.set_death_loot_table(optional_field(entries, "DeathLootTable").and_then(|value| {
        match value {
            NbtSummaryValue::String(raw) => raw
                .try_to_string()
                .ok()
                .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
            _ => None,
        }
    }));
    mob.set_death_loot_table_seed(
        optional_field(entries, "DeathLootTableSeed")
            .filter(|value| super::command_number_f64(value).is_some())
            .map_or(0, super::command_number_i64),
    );
    mob.set_no_ai(optional_bool(entries, "NoAI").unwrap_or(false));

    // `DropChances.CODEC`: invalid entries are dropped.
    if let Some(NbtSummaryValue::Compound(chances)) = optional_field(entries, "drop_chances") {
        for entry in chances {
            let slot = entry
                .name
                .with_utf8(vanilla_equipment_slot_for_serialized_name)
                .flatten();
            if let (Some(slot), Some(chance)) = (slot, numeric_float(&entry.value)) {
                mob.set_drop_chance(slot.slot, chance);
            }
        }
    }
    mob
}

#[cfg(test)]
mod growth_tests;
