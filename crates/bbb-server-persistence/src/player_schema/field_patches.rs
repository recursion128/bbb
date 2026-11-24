//! Writing the live player fields into a playerdata document while keeping
//! every field the live player does not own.

use super::{
    authored_recipe_id_list, Java26PlayerSavePatch, Java26WardenSpawnTracker,
    DEFAULT_LOCATOR_STYLE, RECIPE_BOOK_SETTING_NAMES,
};
use crate::schema_support::invalid_schema;
use bbb_protocol::{
    equipment_slots::VANILLA_EQUIPMENT_SLOTS,
    game_types::GameType,
    nbt_summary::entry,
    packets::{ItemStackSummary, PlayerAbilities, Vec3d},
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::{
    item_stack_summary_is_empty, item_stack_summary_to_persistent_nbt_with_resolver,
    PersistentItemStackHolderResolver,
};
use bbb_server_types::{block_queries::game_type_may_build, save::SaveResult};

/// Replaces `name` in place, or appends it when the document lacks it.
pub fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    match entries.iter_mut().find(|entry| entry.name == name) {
        Some(field) => field.value = value,
        None => entries.push(entry(name, value)),
    }
}

/// Writes `name` when `value` is present and removes it otherwise.
fn set_optional(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: Option<NbtSummaryValue>) {
    match value {
        Some(value) => set_field(entries, name, value),
        None => entries.retain(|entry| entry.name != name),
    }
}

fn bool_value(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}

fn float_value(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn double_list(values: &[f64]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|value| NbtSummaryValue::Double(value.to_bits()))
            .collect(),
    )
}

/// `Entity.saveWithoutId` (`Entity.java:1974-2046`) and the `LivingEntity`,
/// `Player` and `ServerPlayer` `addAdditionalSaveData` fields the live player
/// owns. Optional fields are written only when Vanilla writes them and
/// removed otherwise.
pub(super) fn write_save_patch(
    entries: &mut Vec<NbtSummaryEntry>,
    patch: &Java26PlayerSavePatch,
    warden_spawn_tracker: &Java26WardenSpawnTracker,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<()> {
    set_optional(
        entries,
        "ender_pearls",
        (!patch.ender_pearls.is_empty()).then(|| super::ender_pearls::value(&patch.ender_pearls)),
    );
    for (name, shoulder) in ["ShoulderEntityLeft", "ShoulderEntityRight"]
        .into_iter()
        .zip(&patch.shoulder_entities)
    {
        set_optional(
            entries,
            name,
            (!shoulder.is_empty()).then(|| NbtSummaryValue::Compound(shoulder.clone())),
        );
    }
    let created = &patch.created;
    let damage = &created.damage;
    let rotation = patch
        .rotation
        .iter()
        .map(|value| float_value(*value))
        .collect();
    for (name, value) in [
        ("Pos", double_list(&patch.position)),
        ("Motion", double_list(&patch.motion)),
        ("Rotation", NbtSummaryValue::List(rotation)),
        (
            "fall_distance",
            NbtSummaryValue::Double(created.fall_distance.to_bits()),
        ),
        // Entity.java:1985-1986: Java's narrowing casts wrap rather than clamp.
        (
            "Fire",
            NbtSummaryValue::Short(created.remaining_fire_ticks as i16),
        ),
        ("Air", NbtSummaryValue::Short(created.air_supply as i16)),
        ("OnGround", bool_value(patch.on_ground)),
        ("Invulnerable", bool_value(created.entity_invulnerable)),
        (
            "PortalCooldown",
            NbtSummaryValue::Int(created.portal_cooldown),
        ),
        ("Health", float_value(damage.health)),
        ("HurtTime", NbtSummaryValue::Short(damage.hurt_time as i16)),
        (
            "DeathTime",
            NbtSummaryValue::Short(damage.death_time as i16),
        ),
        ("AbsorptionAmount", float_value(damage.absorption_amount)),
        (
            "current_impulse_context_reset_grace_time",
            NbtSummaryValue::Int(created.impulse_reset_grace_time),
        ),
        (
            "attributes",
            NbtSummaryValue::List(created.attributes.clone()),
        ),
        ("FallFlying", bool_value(created.fall_flying)),
        (
            "SelectedItemSlot",
            NbtSummaryValue::Int(i32::from(created.selected_item_slot)),
        ),
        ("SleepTimer", NbtSummaryValue::Short(created.sleep_timer)),
        (
            "HurtByTimestamp",
            NbtSummaryValue::Int(created.last_hurt_by.hurt_by_timestamp),
        ),
        ("XpP", float_value(created.experience.progress())),
        ("XpLevel", NbtSummaryValue::Int(created.experience.level())),
        ("XpTotal", NbtSummaryValue::Int(created.experience.total())),
        (
            "XpSeed",
            NbtSummaryValue::Int(created.experience.enchantment_seed()),
        ),
        ("Score", NbtSummaryValue::Int(created.experience.score())),
        ("foodLevel", NbtSummaryValue::Int(created.food.food_level())),
        (
            "foodTickTimer",
            NbtSummaryValue::Int(created.food.tick_timer()),
        ),
        (
            "foodSaturationLevel",
            float_value(created.food.saturation_level()),
        ),
        (
            "foodExhaustionLevel",
            float_value(created.food.exhaustion_level()),
        ),
        (
            "abilities",
            abilities_value(&created.abilities, created.game_mode),
        ),
        (
            "warden_spawn_tracker",
            warden_spawn_tracker_value(warden_spawn_tracker),
        ),
        (
            "playerGameType",
            NbtSummaryValue::Int(created.game_mode.id()),
        ),
        ("recipeBook", recipe_book_value(created)),
        (
            "Dimension",
            NbtSummaryValue::string(created.dimension.clone()),
        ),
        (
            "spawn_extra_particles_on_fall",
            bool_value(created.spawn_extra_particles_on_fall),
        ),
        ("seenCredits", bool_value(created.seen_credits)),
    ] {
        set_field(entries, name, value);
    }
    // Entity.java:1991-2023: the conditional entity fields.
    set_optional(entries, "CustomName", patch.custom_name.clone());
    set_optional(entries, "Silent", created.silent.then(|| bool_value(true)));
    set_optional(
        entries,
        "Glowing",
        created.glowing.then(|| bool_value(true)),
    );
    set_optional(
        entries,
        "TicksFrozen",
        (created.ticks_frozen > 0).then_some(NbtSummaryValue::Int(created.ticks_frozen)),
    );
    set_optional(
        entries,
        "HasVisualFire",
        created.has_visual_fire.then(|| bool_value(true)),
    );
    set_optional(
        entries,
        "Tags",
        (!patch.tags.is_empty()).then(|| {
            NbtSummaryValue::List(
                patch
                    .tags
                    .iter()
                    .cloned()
                    .map(NbtSummaryValue::string)
                    .collect(),
            )
        }),
    );
    set_optional(
        entries,
        "data",
        (!patch.custom_data.is_empty())
            .then(|| NbtSummaryValue::Compound(patch.custom_data.clone())),
    );
    // LivingEntity.java:730-755: impact position, effects and icon only when set.
    set_optional(
        entries,
        "current_explosion_impact_pos",
        created
            .impulse_impact_position
            .map(|Vec3d { x, y, z }| double_list(&[x, y, z])),
    );
    set_optional(
        entries,
        "active_effects",
        (!created.active_effects.is_empty())
            .then(|| NbtSummaryValue::List(created.active_effects.clone())),
    );
    let icon = &created.locator_bar_icon;
    set_optional(
        entries,
        "locator_bar_icon",
        (icon.style != DEFAULT_LOCATOR_STYLE || icon.color_rgb.is_some()).then(|| {
            let mut fields = vec![entry("style", NbtSummaryValue::string(icon.style.clone()))];
            if let Some(color) = icon.color_rgb {
                fields.push(entry("color", NbtSummaryValue::Int(color as i32)));
            }
            NbtSummaryValue::Compound(fields)
        }),
    );
    // LivingEntity.java:738-747.
    for (reference, (name, counter)) in [created.last_hurt_by.player, created.last_hurt_by.mob]
        .into_iter()
        .zip([
            ("last_hurt_by_player", "last_hurt_by_player_memory_time"),
            ("last_hurt_by_mob", "ticks_since_last_hurt_by_mob"),
        ])
    {
        set_optional(
            entries,
            name,
            reference.map(|(uuid, _)| crate::schema_support::uuid_nbt(uuid)),
        );
        set_optional(
            entries,
            counter,
            reference.map(|(_, ticks)| NbtSummaryValue::Int(ticks)),
        );
    }
    set_optional(
        entries,
        "sleeping_pos",
        created
            .sleeping_pos
            .map(|position| NbtSummaryValue::IntArray(position.to_vec())),
    );
    // ServerPlayer.java:415,428-436.
    set_optional(
        entries,
        "respawn",
        patch.respawn.as_ref().map(|respawn| {
            let pos = respawn.global_pos().pos;
            let mut fields = vec![
                entry(
                    "dimension",
                    NbtSummaryValue::string(respawn.global_pos().dimension.as_str()),
                ),
                entry("pos", NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z])),
                entry("yaw", float_value(respawn.yaw())),
                entry("pitch", float_value(respawn.pitch())),
            ];
            if respawn.forced() {
                fields.push(entry("forced", bool_value(true)));
            }
            NbtSummaryValue::Compound(fields)
        }),
    );
    // Player.java:660.
    set_optional(
        entries,
        "LastDeathLocation",
        patch.last_death_location.as_ref().map(|pos| {
            NbtSummaryValue::Compound(vec![
                entry("dimension", NbtSummaryValue::string(pos.dimension.as_str())),
                entry(
                    "pos",
                    NbtSummaryValue::IntArray(vec![pos.pos.x, pos.pos.y, pos.pos.z]),
                ),
            ])
        }),
    );
    // ServerPlayer.java:411,417,2041.
    set_optional(
        entries,
        "previousPlayerGameType",
        patch
            .previous_game_mode
            .map(|mode| NbtSummaryValue::Int(mode.id())),
    );
    set_optional(
        entries,
        "entered_nether_pos",
        patch.entered_nether_pos.map(|pos| double_list(&pos)),
    );
    set_optional(
        entries,
        "raid_omen_position",
        patch
            .raid_omen_position
            .map(|pos| NbtSummaryValue::IntArray(pos.to_vec())),
    );
    set_optional(
        entries,
        "RootVehicle",
        patch.root_vehicle.as_ref().map(|vehicle| {
            let mut fields = Vec::new();
            if let Some(attach) = vehicle.attach_input() {
                fields.push(entry("Attach", attach.clone()));
            }
            fields.push(entry(
                "Entity",
                NbtSummaryValue::Compound(vehicle.entity_entries().to_vec()),
            ));
            NbtSummaryValue::Compound(fields)
        }),
    );
    created.inventory.write(entries, resolver)?;
    created.ender_items.write(entries, resolver)
}

/// `Abilities.Packed.CODEC` (`Abilities.java:53-64`); `mayBuild` follows the
/// game type (`GameType.java:79`).
fn abilities_value(abilities: &PlayerAbilities, game_mode: GameType) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("invulnerable", bool_value(abilities.invulnerable)),
        entry("flying", bool_value(abilities.flying)),
        entry("mayfly", bool_value(abilities.can_fly)),
        entry("instabuild", bool_value(abilities.instabuild)),
        entry("mayBuild", bool_value(game_type_may_build(game_mode))),
        entry("flySpeed", float_value(abilities.flying_speed)),
        entry("walkSpeed", float_value(abilities.walking_speed)),
    ])
}

/// `ServerRecipeBook.Packed.CODEC` (`ServerRecipeBook.java:150-158`); the
/// settings flags are `optionalFieldOf(name, false)` and written only when set.
fn recipe_book_value(created: &super::Java26CreatedPlayer) -> NbtSummaryValue {
    let mut fields = vec![
        entry(
            "recipes",
            authored_recipe_id_list(&created.recipe_book_recipes),
        ),
        entry(
            "toBeDisplayed",
            authored_recipe_id_list(&created.recipe_book_to_be_displayed),
        ),
    ];
    for (name, enabled) in RECIPE_BOOK_SETTING_NAMES
        .into_iter()
        .zip(created.recipe_book_settings)
    {
        if enabled {
            fields.push(entry(name, bool_value(true)));
        }
    }
    NbtSummaryValue::Compound(fields)
}

/// `WardenSpawnTracker.CODEC` (`WardenSpawnTracker.java:18-25`).
pub fn warden_spawn_tracker_value(tracker: &Java26WardenSpawnTracker) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry(
            "ticks_since_last_warning",
            NbtSummaryValue::Int(tracker.ticks_since_last_warning()),
        ),
        entry(
            "warning_level",
            NbtSummaryValue::Int(tracker.warning_level()),
        ),
        entry(
            "cooldown_ticks",
            NbtSummaryValue::Int(tracker.cooldown_ticks()),
        ),
    ])
}

pub fn write_player_inventory(
    entries: &mut Vec<NbtSummaryEntry>,
    slots: &[ItemStackSummary],
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<()> {
    let (inventory, equipment) = encode_player_inventory(slots, resolver)?;
    write_encoded_player_inventory(entries, inventory, equipment);
    Ok(())
}

pub(super) fn write_encoded_player_inventory(
    entries: &mut Vec<NbtSummaryEntry>,
    inventory: Vec<NbtSummaryValue>,
    equipment: Vec<NbtSummaryEntry>,
) {
    let inventory_value = NbtSummaryValue::List(inventory);
    match entries.iter().position(|field| field.name == "Inventory") {
        Some(index) => entries[index].value = inventory_value,
        None => entries.push(entry("Inventory", inventory_value)),
    }
    entries.retain(|field| field.name != "equipment");
    if !equipment.is_empty() {
        entries.push(entry("equipment", NbtSummaryValue::Compound(equipment)));
    }
}

/// `PlayerEnderChestContainer.storeAsSlots`
/// (`PlayerEnderChestContainer.java:40-47`) over the `EnderItems` field.
pub fn write_player_ender_items(
    entries: &mut Vec<NbtSummaryEntry>,
    slots: &[ItemStackSummary],
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<()> {
    let value = NbtSummaryValue::List(encode_player_ender_items(slots, resolver)?);
    match entries.iter().position(|field| field.name == "EnderItems") {
        Some(index) => entries[index].value = value,
        None => entries.push(entry("EnderItems", value)),
    }
    Ok(())
}

pub(super) fn encode_stack(
    stack: &ItemStackSummary,
    resolver: &dyn PersistentItemStackHolderResolver,
    path: impl FnOnce() -> String,
) -> SaveResult<Vec<NbtSummaryEntry>> {
    let encoded =
        item_stack_summary_to_persistent_nbt_with_resolver(stack, resolver).map_err(|error| {
            invalid_schema(
                &path(),
                format!("live stack cannot be persisted losslessly: {error}"),
            )
        })?;
    match encoded {
        NbtSummaryValue::Compound(entries) => Ok(entries),
        _ => {
            unreachable!("persistent ItemStack codec writes a compound")
        }
    }
}

fn encode_player_ender_items(
    slots: &[ItemStackSummary],
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<Vec<NbtSummaryValue>> {
    let mut items = Vec::new();
    for (slot, stack) in slots.iter().enumerate() {
        // `storeAsSlots` skips an empty stack (`:43`).
        if item_stack_summary_is_empty(stack) {
            continue;
        }
        let mut stack_entries =
            encode_stack(stack, resolver, || format!("player.EnderItems[{slot}]"))?;
        stack_entries.insert(0, entry("Slot", NbtSummaryValue::Byte(slot as i8)));
        items.push(NbtSummaryValue::Compound(stack_entries));
    }
    Ok(items)
}

fn encode_player_inventory(
    slots: &[ItemStackSummary],
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<(Vec<NbtSummaryValue>, Vec<NbtSummaryEntry>)> {
    encode_player_inventory_slots(slots, item_stack_summary_is_empty, |stack, path| {
        encode_stack(stack, resolver, || path)
    })
}

pub(super) fn encode_player_inventory_slots<T>(
    slots: &[T],
    is_empty: impl Fn(&T) -> bool,
    mut encode_stack: impl FnMut(&T, String) -> SaveResult<Vec<NbtSummaryEntry>>,
) -> SaveResult<(Vec<NbtSummaryValue>, Vec<NbtSummaryEntry>)> {
    // Inventory.java:388-395; PlayerEquipment.java:15-27: selected mainhand is storage.
    let mut inventory = Vec::new();
    for (slot, stack) in slots.iter().take(36).enumerate() {
        if is_empty(stack) {
            continue;
        }
        let mut stack_entries = encode_stack(stack, format!("player.Inventory[{slot}]"))?;
        stack_entries.insert(0, entry("Slot", NbtSummaryValue::Byte(slot as i8)));
        inventory.push(NbtSummaryValue::Compound(stack_entries));
    }

    let mut equipment = Vec::new();
    for equipment_slot in VANILLA_EQUIPMENT_SLOTS
        .iter()
        .filter(|equipment| equipment.serialized_name != "mainhand")
    {
        let slot = usize::try_from(
            equipment_slot
                .player_inventory_slot
                .expect("non-mainhand player equipment has an inventory slot"),
        )
        .expect("generated player inventory slot is non-negative");
        let Some(stack) = slots.get(slot).filter(|stack| !is_empty(stack)) else {
            continue;
        };
        let entries = encode_stack(
            stack,
            format!("player.equipment.{}", equipment_slot.serialized_name),
        )?;
        equipment.push(entry(
            equipment_slot.serialized_name,
            NbtSummaryValue::Compound(entries),
        ));
    }

    Ok((inventory, equipment))
}
