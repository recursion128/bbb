//! Player-document parsing, projection and authoring tests.

mod attribute_numbers;
mod ender_pearls;

use super::document;
use super::{
    Java26CreatedPlayer, Java26PlayerData, Java26PlayerSavePatch, Java26RootVehicle,
    Java26WardenSpawnTracker,
};
use crate::world_storage::SaveHolderResolver;
use crate::PlayerDataStore;
use bbb_protocol::game_types::GameType;
use bbb_protocol::{
    packets::{ItemStackSummary, PlayerAbilities, WaypointIcon},
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};
use uuid::Uuid;

fn saved_stack(value: &NbtSummaryValue) -> ItemStackSummary {
    bbb_registry_codec::read_item_stack(value, &bbb_registry_codec::UnavailableHolderResolver)
        .unwrap()
}

/// These fixtures reference no dynamic holder, so the unavailable publication
/// is enough to author them.
fn created_player_document(patch: &Java26PlayerSavePatch) -> SaveResult<NamedCompound> {
    document::created_player_document(patch, SaveHolderResolver::unavailable().as_dyn())
}

fn e(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
fn f(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}
fn d(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(value.to_bits())
}
fn bool_(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}
fn strings(values: &[&str]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|v| NbtSummaryValue::String((*v).into()))
            .collect(),
    )
}
fn uuid_array(uuid: Uuid) -> NbtSummaryValue {
    let value = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (value >> 96) as i32,
        (value >> 64) as i32,
        (value >> 32) as i32,
        value as i32,
    ])
}
fn vec3(values: [f64; 3]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.into_iter().map(d).collect())
}
fn item(slot: i8, id: &str, count: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        e("Slot", NbtSummaryValue::Byte(slot)),
        e("id", NbtSummaryValue::String(id.into())),
        e("count", NbtSummaryValue::Int(count)),
    ])
}
fn embedded_entity(uuid: Uuid, id: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        e("id", NbtSummaryValue::String(id.into())),
        e("UUID", uuid_array(uuid)),
        e("Pos", vec3([1.0, 64.0, 2.0])),
        e("Motion", vec3([0.0; 3])),
        e("Rotation", NbtSummaryValue::List(vec![f(0.0), f(0.0)])),
        e("fall_distance", d(0.0)),
        e("Fire", NbtSummaryValue::Short(-20)),
        e("Air", NbtSummaryValue::Short(300)),
        e("OnGround", bool_(true)),
        e("Invulnerable", bool_(false)),
        e("PortalCooldown", NbtSummaryValue::Int(0)),
    ])
}

fn embedded_entity_entries(uuid: Uuid, id: &str) -> Vec<NbtSummaryEntry> {
    match embedded_entity(uuid, id) {
        NbtSummaryValue::Compound(entries) => entries,
        _ => unreachable!(),
    }
}

fn nested_vehicle_entries(root: Uuid, passenger: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entries = embedded_entity_entries(root, "minecraft:oak_boat");
    entries.push(e(
        "Passengers",
        NbtSummaryValue::List(vec![embedded_entity(passenger, "minecraft:oak_boat")]),
    ));
    entries
}

fn root_vehicle_value(attach: Uuid, entity: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        e("Attach", uuid_array(attach)),
        e("Entity", NbtSummaryValue::Compound(entity)),
    ])
}
pub(super) fn canonical(uuid: Uuid) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            e("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            e("UUID", uuid_array(uuid)),
            e("Pos", vec3([1.0, 64.0, 2.0])),
            e("Motion", vec3([0.0; 3])),
            e("Rotation", NbtSummaryValue::List(vec![f(0.0), f(0.0)])),
            e("fall_distance", d(0.0)),
            e("Fire", NbtSummaryValue::Short(-20)),
            e("Air", NbtSummaryValue::Short(300)),
            e("OnGround", bool_(true)),
            e("Invulnerable", bool_(false)),
            e("PortalCooldown", NbtSummaryValue::Int(0)),
            e("Health", f(20.0)),
            e("HurtTime", NbtSummaryValue::Short(0)),
            e("HurtByTimestamp", NbtSummaryValue::Int(0)),
            e("DeathTime", NbtSummaryValue::Short(0)),
            e("AbsorptionAmount", f(0.0)),
            e(
                "current_impulse_context_reset_grace_time",
                NbtSummaryValue::Int(0),
            ),
            e("attributes", NbtSummaryValue::List(vec![])),
            e("FallFlying", bool_(false)),
            e("Brain", NbtSummaryValue::Compound(vec![])),
            e(
                "Inventory",
                NbtSummaryValue::List(vec![item(35, "minecraft:stone", 32)]),
            ),
            e("SelectedItemSlot", NbtSummaryValue::Int(1)),
            e("SleepTimer", NbtSummaryValue::Short(0)),
            e("XpP", f(0.5)),
            e("XpLevel", NbtSummaryValue::Int(1)),
            e("XpTotal", NbtSummaryValue::Int(7)),
            e("XpSeed", NbtSummaryValue::Int(4)),
            e("Score", NbtSummaryValue::Int(9)),
            e("foodLevel", NbtSummaryValue::Int(20)),
            e("foodTickTimer", NbtSummaryValue::Int(0)),
            e("foodSaturationLevel", f(5.0)),
            e("foodExhaustionLevel", f(0.0)),
            e(
                "abilities",
                NbtSummaryValue::Compound(vec![
                    e("invulnerable", bool_(true)),
                    e("flying", bool_(false)),
                    e("mayfly", bool_(true)),
                    e("instabuild", bool_(true)),
                    e("mayBuild", bool_(true)),
                    e("flySpeed", f(0.05)),
                    e("walkSpeed", f(0.1)),
                ]),
            ),
            e(
                "EnderItems",
                NbtSummaryValue::List(vec![item(26, "minecraft:dirt", 1)]),
            ),
            e(
                "warden_spawn_tracker",
                NbtSummaryValue::Compound(vec![
                    e("ticks_since_last_warning", NbtSummaryValue::Int(0)),
                    e("warning_level", NbtSummaryValue::Int(0)),
                    e("cooldown_ticks", NbtSummaryValue::Int(0)),
                ]),
            ),
            e("playerGameType", NbtSummaryValue::Int(1)),
            e("seenCredits", bool_(false)),
            e(
                "recipeBook",
                NbtSummaryValue::Compound(vec![
                    e("recipes", strings(&["stone"])),
                    e("toBeDisplayed", strings(&["minecraft:stone"])),
                ]),
            ),
            e("Dimension", NbtSummaryValue::String("overworld".into())),
            e("spawn_extra_particles_on_fall", bool_(true)),
            e("unknown_future", NbtSummaryValue::Long(42)),
        ],
    )
}
fn replace(document: &mut NamedCompound, name: &str, value: NbtSummaryValue) {
    document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap()
        .value = value;
}

fn field_of<'a>(document: &'a NamedCompound, name: &str) -> &'a NbtSummaryValue {
    &document
        .entries()
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the document has no {name}"))
        .value
}

/// `WardenSpawnTracker.CODEC`'s compound (`WardenSpawnTracker.java:18-25`),
/// spelled out here rather than through the writer under test.
fn tracker_value(ticks: i32, level: i32, cooldown: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        e("ticks_since_last_warning", NbtSummaryValue::Int(ticks)),
        e("warning_level", NbtSummaryValue::Int(level)),
        e("cooldown_ticks", NbtSummaryValue::Int(cooldown)),
    ])
}

/// The food state one of these tests writes, on the same rule
/// [`live_damage`] follows: every one of the four fields differs from the
/// canonical document's, so a field the writer forgot shows up.
///
/// `FoodData::created()` is what this used to answer, and it is exactly the
/// four values the canonical document already holds -- which made the food
/// half of the round-trip test pass whether or not the writer wrote
/// anything at all.
fn live_food() -> bbb_server_types::food::FoodData {
    bbb_server_types::food::FoodData::from_saved(17, 23, 2.5, 1.25)
}

fn live_experience() -> bbb_server_types::experience::PlayerExperience {
    bbb_server_types::experience::PlayerExperience::from_saved(0.25, 3, 30)
        .with_enchantment_seed(11)
        .with_score(13)
}

/// The damage state one of these tests writes: every field different from
/// the canonical document's, so a field the writer forgot shows up.
fn live_damage() -> bbb_server_types::player_damage::PlayerDamageState {
    bbb_server_types::player_damage::PlayerDamageState {
        health: 12.5,
        absorption_amount: 3.0,
        invulnerable_time: 17,
        last_hurt: 4.0,
        hurt_time: 9,
        death_time: 2,
    }
}

/// `LivingEntity.readAdditionalSaveData` reads `Health` through `setHealth`
/// (LivingEntity.java:791), whose `Mth.clamp(health, 0.0F, getMaxHealth())`
/// (`:1146-1148`) bounds it by the player's `minecraft:max_health`, and
/// reads `AbsorptionAmount` through `internalSetAbsorptionAmount` (`:778`,
/// `:3337-3339`), which does not clamp. A save another server wrote can
/// carry a value above either cap, and only one of them survives.
#[test]
fn a_restored_health_is_clamped_to_the_max_health_and_an_absorption_is_not() {
    let uuid = Uuid::from_u128(7);
    let mut raw = canonical(uuid);
    replace(&mut raw, "Health", f(100.0));
    replace(&mut raw, "AbsorptionAmount", f(100.0));
    let player = Java26PlayerData::parse(raw, uuid).unwrap();
    let cap = bbb_server_types::player_damage::player_max_health();
    assert_eq!(player.damage(cap).health, cap);
    assert_eq!(player.damage(cap).absorption_amount, 100.0);
    // The cap is the caller's `getMaxHealth()` -- the attribute after the
    // `attributes` field has been applied (`:780`) -- rather than the registry
    // base, so a document that raised it keeps more of the health.
    assert_eq!(player.damage(24.0).health, 24.0);
}

#[test]
fn save_patch_rewrites_live_fields_and_keeps_the_rest() {
    let uuid = Uuid::from_u128(7);
    let raw = canonical(uuid);
    let player = Java26PlayerData::parse(raw.clone(), uuid).unwrap();
    let mut patch = created_patch(uuid);
    patch.position = [1.5, 64.0, -2.25];
    patch.motion = [0.3, 0.4, -0.2];
    patch.rotation = [90.0, -45.0];
    patch.on_ground = false;
    patch.created.damage = live_damage();
    patch.created.food = live_food();
    patch.created.experience = live_experience();
    let patched = player
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();

    for (name, value) in [
        ("Pos", vec3([1.5, 64.0, -2.25])),
        ("Motion", vec3(patch.motion)),
        ("Rotation", NbtSummaryValue::List(vec![f(90.0), f(-45.0)])),
        ("OnGround", bool_(false)),
        ("Health", f(12.5)),
        ("AbsorptionAmount", f(3.0)),
        ("HurtTime", NbtSummaryValue::Short(9)),
        ("DeathTime", NbtSummaryValue::Short(2)),
        ("foodLevel", NbtSummaryValue::Int(17)),
        ("foodTickTimer", NbtSummaryValue::Int(23)),
        ("foodSaturationLevel", f(2.5)),
        ("foodExhaustionLevel", f(1.25)),
        ("XpP", f(0.25)),
        ("XpLevel", NbtSummaryValue::Int(3)),
        ("XpTotal", NbtSummaryValue::Int(30)),
        ("XpSeed", NbtSummaryValue::Int(11)),
        ("Score", NbtSummaryValue::Int(13)),
        // Fields the live player does not own keep the document's values.
        ("unknown_future", NbtSummaryValue::Long(42)),
    ] {
        assert_eq!(field_of(&patched, name), &value, "{name}");
    }
    assert_eq!(player.raw(), &raw);
    assert_eq!(
        field_of(&created_player_document(&patch).unwrap(), "Motion"),
        &vec3(patch.motion)
    );
}

/// `ValueInput.getXOr` and codec `orElse` defaults: a document missing every
/// field but its version, or carrying the wrong tag types, still loads.
#[test]
fn missing_and_mistyped_fields_load_with_vanilla_defaults() {
    let uuid = Uuid::from_u128(7);
    let minimal = NamedCompound::new(
        "",
        vec![e(
            "DataVersion",
            NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
        )],
    );
    let player = Java26PlayerData::parse(minimal.clone(), uuid).unwrap();
    assert_eq!(player.uuid(), uuid);
    assert_eq!(player.position(), [0.0; 3]);
    assert_eq!(player.dimension(), "minecraft:overworld");
    assert_eq!(player.air_supply(), 300);
    assert_eq!(player.game_mode(), None);
    assert_eq!(player.selected_item_slot(), 0);
    assert_eq!(player.damage(20.0).health, 20.0);
    assert_eq!(
        player.food_data(),
        bbb_server_types::food::FoodData::from_saved(20, 0, 5.0, 0.0)
    );
    let abilities = player.abilities();
    assert!(abilities.may_build() && !abilities.may_fly());
    assert_eq!((abilities.fly_speed(), abilities.walk_speed()), (0.05, 0.1));
    assert_eq!(
        player.warden_spawn_tracker(),
        &Java26WardenSpawnTracker::FRESH
    );
    assert!(player.respawn().is_none() && player.root_vehicle().is_none());
    assert_eq!(player.locator_bar_icon().style, "minecraft:default");

    // A document missing fields still takes a full save patch.
    let patched = player
        .raw_with_player_save_patch(
            &created_patch(uuid),
            SaveHolderResolver::unavailable().as_dyn(),
        )
        .unwrap();
    assert_eq!(
        Java26PlayerData::parse(patched, uuid).unwrap().position(),
        created_patch(uuid).position
    );

    let mut raw = canonical(uuid);
    for (name, value) in [
        ("Pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
        ("OnGround", NbtSummaryValue::Short(1)),
        ("foodLevel", NbtSummaryValue::String("full".into())),
        ("SelectedItemSlot", NbtSummaryValue::Int(12)),
        ("playerGameType", NbtSummaryValue::Int(9)),
        (
            "recipeBook",
            NbtSummaryValue::Compound(vec![
                e("recipes", strings(&["stone", "Not Valid"])),
                e("toBeDisplayed", strings(&["stone", "dirt"])),
            ]),
        ),
        (
            "warden_spawn_tracker",
            NbtSummaryValue::Compound(vec![e("warning_level", NbtSummaryValue::Int(-3))]),
        ),
    ] {
        replace(&mut raw, name, value);
    }
    raw.entries_mut().extend([
        e(
            "Tags",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("a".into()),
                NbtSummaryValue::Int(1),
                NbtSummaryValue::String("a".into()),
            ]),
        ),
        e(
            "respawn",
            NbtSummaryValue::Compound(vec![
                e("dimension", NbtSummaryValue::String("overworld".into())),
                e("pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
                e("yaw", f(200.0)),
                e("pitch", f(0.0)),
            ]),
        ),
        e(
            "locator_bar_icon",
            NbtSummaryValue::Compound(vec![e("color", NbtSummaryValue::Int(5))]),
        ),
        e("Silent", bool_(true)),
        e("Silent", bool_(false)),
    ]);
    let player = Java26PlayerData::parse(raw.clone(), uuid).unwrap();
    assert_eq!(player.position(), [0.0; 3]);
    assert!(player.on_ground());
    assert_eq!(player.food_data().food_level(), 20);
    assert_eq!(player.selected_item_slot(), 0);
    assert_eq!(player.game_mode(), Some(GameType::Survival));
    assert_eq!(player.recipe_book().recipes(), &["minecraft:stone"]);
    assert_eq!(player.recipe_book().to_be_displayed(), &["minecraft:stone"]);
    assert_eq!(
        player.warden_spawn_tracker(),
        &Java26WardenSpawnTracker::FRESH
    );
    assert_eq!(player.tags(), &["a"]);
    assert!(player.respawn().is_none());
    assert_eq!(player.locator_bar_icon().style, "minecraft:default");
    assert!(player.silent());
    assert_eq!(player.raw(), &raw);
}

#[test]
fn canonical_writer_shape_projects_and_preserves_raw() {
    let uuid = Uuid::from_u128(7);
    let raw = canonical(uuid);
    let player = Java26PlayerData::parse(raw.clone(), uuid).unwrap();
    assert_eq!(player.uuid(), uuid);
    assert_eq!(player.dimension(), "minecraft:overworld");
    let Some(NbtSummaryValue::List(inventory)) = player.inventory_input() else {
        panic!("inventory list");
    };
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&inventory[0], "Slot"),
        Some(&NbtSummaryValue::Byte(35))
    );
    assert_eq!(saved_stack(&inventory[0]).count, 32);
    let Some(NbtSummaryValue::List(ender_items)) = player.ender_items_input() else {
        panic!("ender list");
    };
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&ender_items[0], "Slot"),
        Some(&NbtSummaryValue::Byte(26))
    );
    assert_eq!(player.game_mode(), Some(GameType::Creative));
    assert_eq!(player.recipe_book().recipes(), &["minecraft:stone"]);
    assert_eq!(player.into_raw(), raw);
}

#[test]
fn root_vehicle_parse_exposes_root_and_indirect_attach() {
    let player_uuid = Uuid::from_u128(7);
    let root_uuid = Uuid::from_u128(8);
    let passenger_uuid = Uuid::from_u128(9);
    let tree = nested_vehicle_entries(root_uuid, passenger_uuid);
    let mut raw = canonical(player_uuid);
    raw.entries_mut().push(e(
        "RootVehicle",
        root_vehicle_value(passenger_uuid, tree.clone()),
    ));
    let parsed = Java26PlayerData::parse(raw, player_uuid).unwrap();
    let root_vehicle = parsed.root_vehicle().unwrap();
    assert_eq!(root_vehicle.attach().unwrap(), passenger_uuid);
    assert_eq!(root_vehicle.entity().uuid(), root_uuid);
    assert_eq!(
        root_vehicle.entity().passengers().next().unwrap().uuid(),
        passenger_uuid
    );
    assert_eq!(root_vehicle.entity_entries(), tree);

    let root_attach = Java26RootVehicle::new(
        root_uuid,
        crate::Java26EntityRecord::parse(&tree.clone()).unwrap(),
    );
    assert_eq!(root_attach.attach().unwrap(), root_uuid);
}

#[test]
fn cold_living_lists_preserve_noncanonical_values_and_duplicate_root_fields_are_checked() {
    let uuid = Uuid::from_u128(7);
    let modifier = NbtSummaryValue::Compound(vec![
        e("id", NbtSummaryValue::String("example:bonus".into())),
        e("amount", d(1.0)),
        e("operation", NbtSummaryValue::String("add_value".into())),
    ]);
    let mut raw = canonical(uuid);
    replace(
        &mut raw,
        "attributes",
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Compound(vec![
                e("id", NbtSummaryValue::String("minecraft:max_health".into())),
                e("base", d(100.0)),
                e("modifiers", NbtSummaryValue::List(vec![modifier.clone()])),
            ]),
            NbtSummaryValue::Compound(vec![
                e(
                    "id",
                    NbtSummaryValue::String("minecraft:flying_speed".into()),
                ),
                e("base", d(4.0)),
                e("modifiers", NbtSummaryValue::List(vec![modifier.clone()])),
            ]),
        ]),
    );
    raw.entries_mut().push(e(
        "active_effects",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            e("id", NbtSummaryValue::String("minecraft:speed".into())),
            e("amplifier", NbtSummaryValue::Byte(-1)),
            e("duration", NbtSummaryValue::Int(-1)),
            e("ambient", bool_(false)),
            e(
                "hidden_effect",
                NbtSummaryValue::Compound(vec![e("show_particles", bool_(true))]),
            ),
        ])]),
    ));
    Java26PlayerData::parse(raw, uuid).unwrap();

    let mut raw = canonical(uuid);
    replace(
        &mut raw,
        "attributes",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            e("id", NbtSummaryValue::String("minecraft:max_health".into())),
            e("base", d(20.0)),
            e(
                "modifiers",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    e("id", NbtSummaryValue::String("example:bonus".into())),
                    e("amount", d(1.0)),
                    e("operation", NbtSummaryValue::String("multiply".into())),
                ])]),
            ),
        ])]),
    );
    assert_cold_living_list_round_trips(raw, uuid, "attributes");

    let mut raw = canonical(uuid);
    raw.entries_mut().push(e(
        "active_effects",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![e(
            "id",
            NbtSummaryValue::String("minecraft:not_an_effect".into()),
        )])]),
    ));
    assert_cold_living_list_round_trips(raw, uuid, "active_effects");

    let mut raw = canonical(uuid);
    replace(
        &mut raw,
        "attributes",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            e("id", NbtSummaryValue::String("minecraft:max_health".into())),
            e("base", d(20.0)),
            e(
                "modifiers",
                NbtSummaryValue::List(vec![modifier.clone(), modifier]),
            ),
        ])]),
    );
    assert_cold_living_list_round_trips(raw, uuid, "attributes");

    let mut raw = canonical(uuid);
    replace(
        &mut raw,
        "attributes",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            e(
                "id",
                NbtSummaryValue::String("minecraft:not_an_attribute".into()),
            ),
            e("base", d(1.0)),
        ])]),
    );
    assert_cold_living_list_round_trips(raw, uuid, "attributes");
}

fn assert_cold_living_list_round_trips(raw: NamedCompound, uuid: Uuid, field: &str) {
    let expected = field_of(&raw, field).clone();
    let parsed = Java26PlayerData::parse(raw, uuid).unwrap();
    let mut patch = created_patch(uuid);
    let NbtSummaryValue::List(values) = &expected else {
        panic!("list fixture required");
    };
    match field {
        "attributes" => {
            assert_eq!(parsed.attributes_input(), Some(&expected));
            assert_eq!(parsed.attributes(), values);
            patch.created.attributes = values.clone();
        }
        "active_effects" => {
            assert_eq!(parsed.active_effects_input(), Some(&expected));
            assert_eq!(parsed.active_effects(), values);
            patch.created.active_effects = values.clone();
        }
        _ => panic!("living field fixture required"),
    }
    let created = created_player_document(&patch).unwrap();
    assert_eq!(field_of(&created, field), &expected);
    let saved = parsed
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert_eq!(field_of(&saved, field), &expected);
}

#[test]
fn cold_living_fields_preserve_outer_shapes_and_live_patch_repairs_them() {
    let uuid = Uuid::from_u128(0xc01d);
    let shapes = [
        None,
        Some(NbtSummaryValue::Int(7)),
        Some(NbtSummaryValue::ByteArray(vec![1, 2])),
        Some(NbtSummaryValue::IntArray(vec![1, 2])),
        Some(NbtSummaryValue::LongArray(vec![1, 2])),
        Some(NbtSummaryValue::List(vec![NbtSummaryValue::Int(7)])),
        Some(NbtSummaryValue::List(vec![NbtSummaryValue::Compound(
            vec![],
        )])),
    ];
    let mut patch = created_patch(uuid);
    patch.created.attributes = vec![NbtSummaryValue::Compound(vec![
        e("id", NbtSummaryValue::String("minecraft:max_health".into())),
        e("base", d(24.0)),
    ])];
    patch.created.active_effects = vec![NbtSummaryValue::Compound(vec![e(
        "id",
        NbtSummaryValue::String("minecraft:speed".into()),
    )])];
    for shape in shapes {
        let mut raw = canonical(uuid);
        raw.entries_mut().retain(|entry| entry.name != "attributes");
        if let Some(value) = &shape {
            for name in ["attributes", "active_effects"] {
                raw.entries_mut().push(e(name, value.clone()));
            }
        }
        if matches!(&shape, Some(NbtSummaryValue::List(_))) {
            for field in ["attributes", "active_effects"] {
                assert_cold_living_list_round_trips(raw.clone(), uuid, field);
            }
        }
        let parsed = Java26PlayerData::parse(raw, uuid).unwrap();
        assert_eq!(parsed.attributes_input(), shape.as_ref());
        assert_eq!(parsed.active_effects_input(), shape.as_ref());
        let list = match &shape {
            Some(NbtSummaryValue::List(values)) => values.as_slice(),
            _ => &[],
        };
        assert_eq!(parsed.attributes(), list);
        assert_eq!(parsed.active_effects(), list);
        let rewritten = parsed
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        let repaired = Java26PlayerData::parse(rewritten, uuid).unwrap();
        assert_eq!(repaired.attributes(), patch.created.attributes);
        assert_eq!(repaired.active_effects(), patch.created.active_effects);
        let empty = parsed
            .raw_with_player_save_patch(
                &created_patch(uuid),
                SaveHolderResolver::unavailable().as_dyn(),
            )
            .unwrap();
        let empty = Java26PlayerData::parse(empty, uuid).unwrap();
        assert_eq!(
            empty.attributes_input(),
            Some(&NbtSummaryValue::List(vec![]))
        );
        assert_eq!(empty.active_effects_input(), None);
    }
}

#[test]
#[ignore = "requires BBB_TEST_JAVA_26_1_WORLD from an authorized official-server run"]
fn verified_official_java_26_1_playerdata_reads_without_loss() {
    let root = std::env::var_os("BBB_TEST_JAVA_26_1_WORLD")
        .expect("BBB_TEST_JAVA_26_1_WORLD must name an official 26.1 world");
    let paths = WorldSavePaths::new(root).unwrap();
    // The capture's replay observes a second client, so the world holds
    // more than one playerdata file. This test is about the probe's own
    // session, so it names that player rather than taking whichever file
    // the directory happens to list first.
    let uuid = bbb_protocol::codec::offline_player_uuid("capture-probe");
    let loaded = PlayerDataStore::new(paths, WorldSaveLimits::default())
        .unwrap()
        .load(uuid)
        .unwrap()
        .unwrap();

    let player = Java26PlayerData::parse(loaded.document.clone(), uuid).unwrap();
    assert_eq!(player.raw(), &loaded.document);
    assert_eq!(player.uuid(), uuid);
    assert_eq!(player.selected_item_slot(), 1);
    // What the capture replay leaves in the held slot:
    // `INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE` in
    // `crates/bbb-client-net/src/types/probe.rs`. This crate cannot reach the
    // client's replay constants, so `scripts/tests/test_capture_replay_expectations.py`
    // asserts the two agree rather than letting this expectation rot the
    // way it silently did once already.
    const CAPTURED_HELD_STONE: i32 = 48;
    let Some(NbtSummaryValue::List(inventory)) = player.inventory_input() else {
        panic!("inventory list");
    };
    assert!(inventory.iter().any(|value| {
        let stack = saved_stack(value);
        stack.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
            && stack.count == CAPTURED_HELD_STONE
    }));
    assert!(player
        .ender_items_input()
        .is_none_or(|value| value == &NbtSummaryValue::List(Vec::new())));
    assert_eq!(player.game_mode(), Some(GameType::Creative));
    // The replay also stocks redstone dust, whose inventory-change criteria
    // add eight recipes beside the original nineteen stone recipes.
    const CAPTURED_RECIPE_COUNT: usize = 27;
    assert_eq!(player.recipe_book().recipes().len(), CAPTURED_RECIPE_COUNT);
    assert_eq!(
        player.recipe_book().to_be_displayed(),
        player.recipe_book().recipes()
    );
}

/// A player the save has never held is authored from scratch, and the tag
/// set the session carries has to reach that document too.
#[test]
fn a_created_document_writes_the_tag_set_the_session_carries() {
    let uuid = Uuid::from_u128(7);
    let mut patch = created_patch(uuid);
    assert!(!created_player_document(&patch)
        .unwrap()
        .entries()
        .iter()
        .any(|entry| entry.name == "Tags"));

    patch.tags = vec![
        "marked".into(),
        bbb_protocol::JavaString::from(vec![0xd800]),
    ];
    let document = created_player_document(&patch).unwrap();
    assert_eq!(
        document
            .entries()
            .iter()
            .find(|entry| entry.name == "Tags")
            .map(|entry| entry.value.clone()),
        Some(NbtSummaryValue::List(vec![
            NbtSummaryValue::string("marked"),
            NbtSummaryValue::string_from_utf16_slice(&[0xd800])
        ]))
    );
    // The authored document goes back through the reader, which is what the
    // save path does before it publishes one.
    let parsed = Java26PlayerData::parse(document, uuid).unwrap();
    assert_eq!(parsed.tags(), patch.tags);
}

pub(super) fn created_patch(uuid: Uuid) -> Java26PlayerSavePatch {
    Java26PlayerSavePatch {
        stats: bbb_server_types::stats::ServerStatsCounter::new(),
        uuid,
        position: [1.5, 64.0, 2.5],
        motion: [0.0; 3],
        rotation: [90.0, -30.0],
        on_ground: true,
        custom_name: None,
        custom_data: Vec::new(),
        tags: Vec::new(),
        root_vehicle: None,
        ender_pearls: Vec::new(),
        shoulder_entities: Default::default(),
        respawn: None,
        last_death_location: None,
        previous_game_mode: None,
        entered_nether_pos: None,
        raid_omen_position: None,
        warden_spawn_tracker: None,
        created: Java26CreatedPlayer {
            sleep_timer: 0,
            sleeping_pos: None,
            last_hurt_by: Default::default(),
            spawn_extra_particles_on_fall: false,
            seen_credits: false,
            impulse_reset_grace_time: 0,
            impulse_impact_position: None,
            has_visual_fire: false,
            silent: false,
            glowing: false,
            dimension: "minecraft:overworld".to_string(),
            game_mode: GameType::Creative,
            entity_invulnerable: false,
            fall_distance: 0.0,
            remaining_fire_ticks: 0,
            air_supply: 300,
            ticks_frozen: 0,
            portal_cooldown: 0,
            attributes: Vec::new(),
            active_effects: Vec::new(),
            locator_bar_icon: WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: None,
            },
            damage: bbb_server_types::player_damage::PlayerDamageState::created(),
            food: bbb_server_types::food::FoodData::created(),
            experience: bbb_server_types::experience::PlayerExperience::created(),
            abilities: PlayerAbilities {
                invulnerable: true,
                flying: true,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            },
            inventory: vec![bbb_protocol::packets::ItemStackSummary::empty(); 43].into(),
            ender_items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 27].into(),
            fall_flying: false,
            selected_item_slot: 4,
            recipe_book_settings: [false, false, true, true, false, false, false, false],
            recipe_book_recipes: Vec::new(),
            recipe_book_to_be_displayed: Vec::new(),
        },
    }
}

#[test]
fn portal_cooldown_is_authored_rewritten_and_restored_from_live_state() {
    let uuid = Uuid::from_u128(0x26_01);
    let mut patch = created_patch(uuid);
    patch.created.portal_cooldown = 10;
    let created = created_player_document(&patch).unwrap();
    let loaded = Java26PlayerData::parse(created, uuid).unwrap();
    assert_eq!(loaded.portal_cooldown(), 10);

    patch.created.portal_cooldown = 7;
    let rewritten = loaded
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let restored = Java26PlayerData::parse(rewritten, uuid).unwrap();
    assert_eq!(restored.portal_cooldown(), 7);

    patch.created.portal_cooldown = 0;
    let rewritten = restored
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert_eq!(
        Java26PlayerData::parse(rewritten, uuid)
            .unwrap()
            .portal_cooldown(),
        0
    );
}

#[test]
fn environment_state_load_preserves_signed_values_and_writer_shapes() {
    let uuid = Uuid::from_u128(0x26_01);
    let mut raw = canonical(uuid);
    replace(&mut raw, "Fire", NbtSummaryValue::Short(-20));
    replace(&mut raw, "Air", NbtSummaryValue::Short(-19));
    let player = Java26PlayerData::parse(raw.clone(), uuid).unwrap();
    assert_eq!(player.remaining_fire_ticks(), -20);
    assert_eq!(player.air_supply(), -19);
    assert_eq!(player.ticks_frozen(), 0);

    raw.entries_mut()
        .push(e("TicksFrozen", NbtSummaryValue::Int(200)));
    assert_eq!(
        Java26PlayerData::parse(raw.clone(), uuid)
            .unwrap()
            .ticks_frozen(),
        200
    );
}

#[test]
fn environment_state_patch_removes_stale_frozen_ticks_and_preserves_unknown_fields() {
    let uuid = Uuid::from_u128(0x26_01);
    let mut raw = canonical(uuid);
    raw.entries_mut()
        .push(e("TicksFrozen", NbtSummaryValue::Int(140)));
    let baseline = Java26PlayerData::parse(raw, uuid).unwrap();
    for ticks in [0, -1, i32::MIN, 42] {
        let mut patch = created_patch(uuid);
        patch.created.ticks_frozen = ticks;
        let raw = baseline
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        assert_eq!(
            raw.entries()
                .iter()
                .filter(|entry| entry.name == "TicksFrozen")
                .count(),
            usize::from(ticks > 0)
        );
        assert_eq!(field_of(&raw, "unknown_future"), &NbtSummaryValue::Long(42));
        assert_eq!(
            Java26PlayerData::parse(raw, uuid).unwrap().ticks_frozen(),
            ticks.max(0)
        );
    }
}

#[test]
fn locator_bar_icon_style_and_rgb_round_trip_through_create_and_raw_patch() {
    let uuid = Uuid::from_u128(0x26_02);
    for icon in [
        WaypointIcon {
            style: "minecraft:green_arrow".to_string(),
            color_rgb: None,
        },
        WaypointIcon {
            style: "minecraft:default".to_string(),
            color_rgb: Some(0xab_cd_ef),
        },
    ] {
        let mut patch = created_patch(uuid);
        patch.created.locator_bar_icon = icon.clone();
        let created = created_player_document(&patch).unwrap();
        let parsed = Java26PlayerData::parse(created, uuid).unwrap();
        assert_eq!(parsed.locator_bar_icon(), &icon);

        let retained = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
        let rewritten = retained
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        let reparsed = Java26PlayerData::parse(rewritten, uuid).unwrap();
        assert_eq!(reparsed.locator_bar_icon(), &icon);
    }
}

/// `LivingEntity.addAdditionalSaveData` stores `attributes` whatever it holds
/// (`LivingEntity.java:731`) and `active_effects` only for a non-empty map
/// (`:732-734`), which makes an empty live map a removal rather than an empty
/// list.
#[test]
fn the_live_attributes_and_active_effects_replace_what_a_document_retained() {
    let uuid = Uuid::from_u128(0xeff);
    let packed_health = NbtSummaryValue::Compound(vec![
        e("id", NbtSummaryValue::String("minecraft:max_health".into())),
        e("base", d(20.0)),
        e(
            "modifiers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                e(
                    "id",
                    NbtSummaryValue::String("minecraft:effect.health_boost".into()),
                ),
                e("amount", d(4.0)),
                e("operation", NbtSummaryValue::String("add_value".into())),
            ])]),
        ),
    ]);
    let saved_effect = NbtSummaryValue::Compound(vec![
        e(
            "id",
            NbtSummaryValue::String("minecraft:health_boost".into()),
        ),
        e("duration", NbtSummaryValue::Int(400)),
        e("show_icon", bool_(true)),
    ]);

    let mut patch = created_patch(uuid);
    patch.created.attributes = vec![packed_health.clone()];
    patch.created.active_effects = vec![saved_effect.clone()];
    let document = created_player_document(&patch).unwrap();
    let parsed = Java26PlayerData::parse(document, uuid).unwrap();
    assert_eq!(parsed.attributes(), std::slice::from_ref(&packed_health));
    assert_eq!(parsed.active_effects(), [saved_effect]);

    // A retained document's stale pair is replaced, and an empty live effect
    // map removes the field rather than emptying it.
    let mut patch = created_patch(uuid);
    patch.created.attributes = vec![packed_health];
    let rewritten = parsed
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert!(!rewritten
        .entries()
        .iter()
        .any(|entry| entry.name == "active_effects"));
    let parsed = Java26PlayerData::parse(rewritten, uuid).unwrap();
    assert!(parsed.active_effects().is_empty());
    assert_eq!(parsed.attributes().len(), 1);
}

fn live_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

#[test]
fn live_inventory_and_fall_flying_replace_stale_owned_fields() {
    let uuid = Uuid::from_u128(0x51a7e);
    let mut raw = canonical(uuid);
    raw.entries_mut().push(e(
        "equipment",
        NbtSummaryValue::Compound(vec![e(
            "offhand",
            NbtSummaryValue::Compound(vec![
                e("id", NbtSummaryValue::String("minecraft:dirt".into())),
                e("count", NbtSummaryValue::Int(1)),
            ]),
        )]),
    ));
    raw.entries_mut()
        .push(e("bbb_unknown", NbtSummaryValue::Long(77)));
    replace(&mut raw, "FallFlying", bool_(false));
    let parsed = Java26PlayerData::parse(raw, uuid).unwrap();

    let mut patch = created_patch(uuid);
    let mut inventory = vec![ItemStackSummary::empty(); 43];
    inventory[0] = live_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 5);
    inventory[36] = live_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 1);
    patch.created.inventory = inventory.into();
    patch.created.fall_flying = true;
    let rewritten = parsed
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert_eq!(
        rewritten
            .entries()
            .iter()
            .find(|entry| entry.name == "bbb_unknown")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Long(77))
    );
    let restored = Java26PlayerData::parse(rewritten, uuid).unwrap();
    assert!(restored.fall_flying());
    assert_eq!(
        restored.inventory_input(),
        Some(&NbtSummaryValue::List(vec![item(0, "minecraft:stone", 5)]))
    );
    let feet = bbb_protocol::nbt_summary::value_field(restored.equipment_input().unwrap(), "feet")
        .unwrap();
    assert_eq!(saved_stack(feet).count, 1);
}

#[test]
fn created_document_writes_live_hotbar_equipment_and_fall_flying() {
    let uuid = Uuid::from_u128(0xc7ea7ed);
    let mut patch = created_patch(uuid);
    let mut inventory = vec![ItemStackSummary::empty(); 43];
    inventory[4] = live_stack(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID, 12);
    inventory[40] = live_stack(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID, 1);
    patch.created.inventory = inventory.into();
    patch.created.fall_flying = true;
    let restored = Java26PlayerData::parse(created_player_document(&patch).unwrap(), uuid).unwrap();
    assert!(restored.fall_flying());
    assert_eq!(
        restored.inventory_input(),
        Some(&NbtSummaryValue::List(vec![item(4, "minecraft:stone", 12)]))
    );
    let offhand =
        bbb_protocol::nbt_summary::value_field(restored.equipment_input().unwrap(), "offhand")
            .unwrap();
    assert_eq!(saved_stack(offhand).count, 1);
}

/// The authored document has to be one this crate's own reader takes, since
/// that reader is what every later startup runs it through.
#[test]
fn a_created_player_document_parses_back_into_the_state_it_was_authored_from() {
    let uuid = Uuid::from_u128(0x5eed);
    let patch = created_patch(uuid);
    let document = created_player_document(&patch).unwrap();
    assert_eq!(document.root_name(), "");
    let player = Java26PlayerData::parse(document, uuid).unwrap();

    assert_eq!(player.uuid(), uuid);
    assert_eq!(player.position(), patch.position);
    assert_eq!(player.rotation(), patch.rotation);
    assert!(player.on_ground());
    assert_eq!(player.dimension(), "minecraft:overworld");
    assert_eq!(player.game_mode(), Some(GameType::Creative));
    assert!(!player.invulnerable());
    assert_eq!(player.selected_item_slot(), 4);
    assert_eq!(
        player.recipe_book().settings(),
        patch.created.recipe_book_settings
    );
    assert!(player.recipe_book().recipes().is_empty());
    assert!(player.recipe_book().to_be_displayed().is_empty());
    let abilities = player.abilities();
    assert!(abilities.invulnerable());
    assert!(abilities.flying());
    assert!(abilities.may_fly());
    assert!(abilities.instant_build());
    // Derived from the game type, not carried: creative may build.
    assert!(abilities.may_build());
    assert_eq!(abilities.fly_speed(), 0.05);
    assert_eq!(abilities.walk_speed(), 0.1);
}

#[test]
fn created_document_writes_live_recipe_book_id_lists() {
    let uuid = Uuid::from_u128(0x5eed);
    let mut patch = created_patch(uuid);
    patch.created.recipe_book_recipes = vec!["minecraft:stone".to_string()];
    patch.created.recipe_book_to_be_displayed = vec!["minecraft:stone".to_string()];
    let player = Java26PlayerData::parse(created_player_document(&patch).unwrap(), uuid).unwrap();
    assert_eq!(player.recipe_book().recipes(), &["minecraft:stone"]);
    assert_eq!(player.recipe_book().to_be_displayed(), &["minecraft:stone"]);
    assert_eq!(
        player.recipe_book().settings(),
        patch.created.recipe_book_settings
    );
}

#[test]
fn live_recipe_book_lists_replace_stale_owned_fields() {
    let uuid = Uuid::from_u128(0x51a7e);
    let parsed = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
    assert_eq!(parsed.recipe_book().recipes(), &["minecraft:stone"]);
    let mut patch = created_patch(uuid);
    patch.created.recipe_book_recipes = vec!["minecraft:oak_planks".to_string()];
    patch.created.recipe_book_to_be_displayed = vec!["minecraft:oak_planks".to_string()];
    let rewritten = parsed
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let restored = Java26PlayerData::parse(rewritten, uuid).unwrap();
    assert_eq!(restored.recipe_book().recipes(), &["minecraft:oak_planks"]);
    assert_eq!(
        restored.recipe_book().to_be_displayed(),
        &["minecraft:oak_planks"]
    );
    assert_eq!(
        restored.recipe_book().settings(),
        patch.created.recipe_book_settings
    );
}

#[test]
fn a_created_player_document_keeps_entity_and_ability_invulnerability_independent() {
    let uuid = Uuid::from_u128(0x5eed);
    let mut patch = created_patch(uuid);
    patch.created.entity_invulnerable = true;
    patch.created.abilities.invulnerable = false;
    let player = Java26PlayerData::parse(created_player_document(&patch).unwrap(), uuid).unwrap();

    assert!(player.invulnerable());
    assert!(!player.abilities().invulnerable());
}

#[test]
fn root_vehicle_created_document_writes_some_and_omits_none() {
    let player_uuid = Uuid::from_u128(0x5eed);
    let root_uuid = Uuid::from_u128(0x600d);
    let passenger_uuid = Uuid::from_u128(0x700d);
    let mut patch = created_patch(player_uuid);
    patch.root_vehicle = Some(Java26RootVehicle::new(
        passenger_uuid,
        crate::Java26EntityRecord::parse(&nested_vehicle_entries(root_uuid, passenger_uuid))
            .unwrap(),
    ));
    let document = created_player_document(&patch).unwrap();
    let parsed = Java26PlayerData::parse(document, player_uuid).unwrap();
    assert_eq!(parsed.root_vehicle().unwrap().entity().uuid(), root_uuid);

    patch.root_vehicle = None;
    let document = created_player_document(&patch).unwrap();
    assert!(document
        .entries()
        .iter()
        .all(|entry| entry.name != "RootVehicle"));
}

#[test]
fn a_created_player_document_carries_live_fall_state() {
    let uuid = Uuid::from_u128(0x5eed);
    let mut patch = created_patch(uuid);
    patch.created.fall_distance = 4.25;
    patch.created.fall_flying = true;
    let document = created_player_document(&patch).unwrap();
    let player = Java26PlayerData::parse(document, uuid).unwrap();
    assert_eq!(player.fall_distance(), 4.25);
    assert!(player.fall_flying());
}

/// Every field a fresh `ServerPlayer` carries into its first save, pinned to
/// the tag type and the value vanilla declares. A default that drifts breaks
/// here rather than in a save an official server refuses.
#[test]
fn a_created_player_document_carries_the_vanilla_new_player_defaults() {
    let uuid = Uuid::from_u128(0x5eed);
    let document = created_player_document(&created_patch(uuid)).unwrap();
    let entries = document.entries();
    let field = |name: &str| {
        &entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("the created document has no {name}"))
            .value
    };

    assert_eq!(
        field("DataVersion"),
        &NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)
    );
    assert_eq!(field("UUID"), &uuid_array(uuid));
    assert_eq!(field("Motion"), &vec3([0.0; 3]));
    assert_eq!(field("fall_distance"), &d(0.0));
    // `Entity.remainingFireTicks` has no initialiser (`Entity.java:248`).
    assert_eq!(field("Fire"), &NbtSummaryValue::Short(0));
    // `Entity.TOTAL_AIR_SUPPLY` (`Entity.java:192`).
    assert_eq!(field("Air"), &NbtSummaryValue::Short(300));
    assert_eq!(field("Invulnerable"), &bool_(false));
    assert_eq!(field("PortalCooldown"), &NbtSummaryValue::Int(0));
    // `Attributes.MAX_HEALTH`'s player base, out of the generated catalog.
    assert_eq!(field("Health"), &f(20.0));
    assert_eq!(field("HurtTime"), &NbtSummaryValue::Short(0));
    assert_eq!(field("HurtByTimestamp"), &NbtSummaryValue::Int(0));
    assert_eq!(field("DeathTime"), &NbtSummaryValue::Short(0));
    assert_eq!(field("AbsorptionAmount"), &f(0.0));
    assert_eq!(
        field("current_impulse_context_reset_grace_time"),
        &NbtSummaryValue::Int(0)
    );
    assert_eq!(field("attributes"), &NbtSummaryValue::List(Vec::new()));
    assert_eq!(field("FallFlying"), &bool_(false));
    // `Brain.Packed.CODEC`'s `memories` map is not optional
    // (`Brain.java:463-468`), so an empty brain is not an empty compound.
    assert_eq!(
        field("Brain"),
        &NbtSummaryValue::Compound(vec![e("memories", NbtSummaryValue::Compound(Vec::new()))])
    );
    assert_eq!(field("Inventory"), &NbtSummaryValue::List(Vec::new()));
    assert_eq!(field("EnderItems"), &NbtSummaryValue::List(Vec::new()));
    assert_eq!(field("SleepTimer"), &NbtSummaryValue::Short(0));
    assert_eq!(field("XpP"), &f(0.0));
    assert_eq!(field("XpLevel"), &NbtSummaryValue::Int(0));
    assert_eq!(field("XpTotal"), &NbtSummaryValue::Int(0));
    assert_eq!(field("XpSeed"), &NbtSummaryValue::Int(0));
    assert_eq!(field("Score"), &NbtSummaryValue::Int(0));
    // `FoodData` (`FoodData.java:12-17`).
    assert_eq!(field("foodLevel"), &NbtSummaryValue::Int(20));
    assert_eq!(field("foodTickTimer"), &NbtSummaryValue::Int(0));
    assert_eq!(field("foodSaturationLevel"), &f(5.0));
    assert_eq!(field("foodExhaustionLevel"), &f(0.0));
    assert_eq!(
        field("warden_spawn_tracker"),
        &NbtSummaryValue::Compound(vec![
            e("ticks_since_last_warning", NbtSummaryValue::Int(0)),
            e("warning_level", NbtSummaryValue::Int(0)),
            e("cooldown_ticks", NbtSummaryValue::Int(0)),
        ])
    );
    assert_eq!(field("seenCredits"), &bool_(false));
    assert_eq!(field("spawn_extra_particles_on_fall"), &bool_(false));
    // `SelectedItemSlot` is an int, not the byte the slot indices are.
    assert_eq!(field("SelectedItemSlot"), &NbtSummaryValue::Int(4));
    // Two required id lists, and only the flags this session turned on:
    // DFU omits an optional field that equals its default.
    assert_eq!(
        field("recipeBook"),
        &NbtSummaryValue::Compound(vec![
            e("recipes", NbtSummaryValue::List(Vec::new())),
            e("toBeDisplayed", NbtSummaryValue::List(Vec::new())),
            e("isFurnaceGuiOpen", bool_(true)),
            e("isFurnaceFilteringCraftable", bool_(true)),
        ])
    );

    // `saveWithoutId` leaves the entity type out, and a fresh player has
    // set none of the conditional fields.
    for absent in [
        "id",
        "CustomName",
        "CustomNameVisible",
        "Silent",
        "NoGravity",
        "Glowing",
        "TicksFrozen",
        "HasVisualFire",
        "Tags",
        "data",
        "Passengers",
        "active_effects",
        "current_explosion_impact_pos",
        "sleeping_pos",
        "equipment",
        "LastDeathLocation",
        "previousPlayerGameType",
        "entered_nether_pos",
        "RootVehicle",
        "respawn",
        "raid_omen_position",
        "ender_pearls",
        "ShoulderEntityLeft",
        "ShoulderEntityRight",
    ] {
        assert!(
            !entries.iter().any(|entry| entry.name == absent),
            "a fresh player writes no {absent}"
        );
    }
}

/// `ServerPlayer.java:387` reads the tracker and `:408` writes it back, so a
/// document's three counters survive a save instead of resetting to the fresh
/// tracker.
#[test]
fn the_warden_spawn_tracker_round_trips_through_parse_and_the_save_patch() {
    let uuid = Uuid::from_u128(0x7a2d);
    let mut raw = canonical(uuid);
    replace(
        &mut raw,
        "warden_spawn_tracker",
        NbtSummaryValue::Compound(vec![
            e("ticks_since_last_warning", NbtSummaryValue::Int(4096)),
            e("warning_level", NbtSummaryValue::Int(3)),
            e("cooldown_ticks", NbtSummaryValue::Int(157)),
            // No codec field: the reader ignores it, and the save path
            // re-encodes the compound from the parsed counters, so it goes.
            e("unknown_future", NbtSummaryValue::Int(9)),
        ]),
    );

    let parsed = Java26PlayerData::parse(raw, uuid).unwrap();
    let tracker = parsed.warden_spawn_tracker();
    assert_eq!(tracker.ticks_since_last_warning(), 4096);
    assert_eq!(tracker.warning_level(), 3);
    assert_eq!(tracker.cooldown_ticks(), 157);

    let rewritten = parsed
        .raw_with_player_save_patch(
            &created_patch(uuid),
            SaveHolderResolver::unavailable().as_dyn(),
        )
        .unwrap();
    assert_eq!(
        field_of(&rewritten, "warden_spawn_tracker"),
        &tracker_value(4096, 3, 157)
    );
    let parsed = Java26PlayerData::parse(rewritten, uuid).unwrap();
    assert_eq!(parsed.warden_spawn_tracker(), tracker);
}

/// The other half of the round trip: the world's live tracker is what a save
/// records, for a document the save already holds and for a player it has
/// never held.
#[test]
fn a_live_warden_spawn_tracker_is_written_by_both_save_paths() {
    let uuid = Uuid::from_u128(0x7a2e);
    let live = Java26WardenSpawnTracker::new(12, 4, 200);

    let patch = Java26PlayerSavePatch {
        warden_spawn_tracker: Some(live.clone()),
        ..created_patch(uuid)
    };
    let parsed = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
    let rewritten = parsed
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert_eq!(
        field_of(&rewritten, "warden_spawn_tracker"),
        &tracker_value(12, 4, 200)
    );
    assert_eq!(
        Java26PlayerData::parse(rewritten, uuid)
            .unwrap()
            .warden_spawn_tracker(),
        &live
    );

    let created = created_player_document(&patch).unwrap();
    assert_eq!(
        field_of(&created, "warden_spawn_tracker"),
        &tracker_value(12, 4, 200)
    );
    assert_eq!(
        Java26PlayerData::parse(created, uuid)
            .unwrap()
            .warden_spawn_tracker(),
        &live
    );
}

/// `WardenSpawnTracker()` (`WardenSpawnTracker.java:41-43`): the tracker a
/// freshly built `ServerPlayer` carries, which is what a fresh player's first
/// document writes.
#[test]
fn a_fresh_player_keeps_the_vanilla_warden_spawn_tracker_default() {
    let uuid = Uuid::from_u128(0x7a2f);
    let fresh = Java26WardenSpawnTracker::FRESH;
    assert_eq!(fresh.ticks_since_last_warning(), 0);
    assert_eq!(fresh.warning_level(), 0);
    assert_eq!(fresh.cooldown_ticks(), 0);

    let created = created_player_document(&created_patch(uuid)).unwrap();
    assert_eq!(
        field_of(&created, "warden_spawn_tracker"),
        &tracker_value(0, 0, 0)
    );
    assert_eq!(
        Java26PlayerData::parse(created, uuid)
            .unwrap()
            .warden_spawn_tracker(),
        &fresh
    );
}

#[test]
fn environment_visual_fire_round_trips_and_false_removes_stale_field() {
    let uuid = Uuid::from_u128(0x26_01);
    let mut patch = created_patch(uuid);
    patch.created.has_visual_fire = true;
    let raw = created_player_document(&patch).unwrap();
    let baseline = Java26PlayerData::parse(raw, uuid).unwrap();
    assert!(baseline.has_visual_fire());
    patch.created.has_visual_fire = false;
    let raw = baseline
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert!(!raw
        .entries()
        .iter()
        .any(|entry| entry.name == "HasVisualFire"));
    assert!(!Java26PlayerData::parse(raw, uuid)
        .unwrap()
        .has_visual_fire());
}

/// `Entity.java:2004-2006` writes `Glowing` only when set; `:2085` reads it.
#[test]
fn the_glowing_tag_round_trips_and_false_removes_the_field() {
    let uuid = Uuid::from_u128(0x26_02);
    let mut patch = created_patch(uuid);
    patch.created.glowing = true;
    let raw = created_player_document(&patch).unwrap();
    let baseline = Java26PlayerData::parse(raw, uuid).unwrap();
    assert!(baseline.glowing());
    patch.created.glowing = false;
    let raw = baseline
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert!(!raw.entries().iter().any(|entry| entry.name == "Glowing"));
}

#[test]
fn impulse_context_new_and_retained_documents_restore_live_values_without_clamping() {
    let uuid = Uuid::from_u128(0x2601);
    let mut original = canonical(uuid);
    original
        .entries_mut()
        .push(e("current_explosion_impact_pos", vec3([99.0, 40.0, -23.0])));
    let baseline = Java26PlayerData::parse(original, uuid).unwrap();
    for grace in [0, -1, i32::MIN, i32::MAX] {
        let impact = bbb_protocol::packets::Vec3d {
            x: -13.25,
            y: 64.125,
            z: 9.5,
        };
        let mut patch = created_patch(uuid);
        patch.created.impulse_reset_grace_time = grace;
        patch.created.impulse_impact_position = Some(impact);
        let retained = baseline
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        for raw in [created_player_document(&patch).unwrap(), retained] {
            assert_eq!(
                field_of(&raw, "current_impulse_context_reset_grace_time"),
                &NbtSummaryValue::Int(grace)
            );
            assert_eq!(
                field_of(&raw, "current_explosion_impact_pos"),
                &vec3([impact.x, impact.y, impact.z])
            );
            let restored = Java26PlayerData::parse(raw, uuid).unwrap();
            assert_eq!(restored.impulse_reset_grace_time(), grace);
            assert_eq!(restored.impulse_impact_position(), Some(impact));
        }
    }
}

#[test]
fn extra_fall_particles_new_and_retained_saves_write_live_boolean_including_clear() {
    let uuid = Uuid::from_u128(0x26_0601);
    let mut patch = created_patch(uuid);
    let mut retained = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
    for flag in [true, false, true] {
        patch.created.spawn_extra_particles_on_fall = flag;
        let updated = retained
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        for raw in [created_player_document(&patch).unwrap(), updated.clone()] {
            assert_eq!(
                field_of(&raw, "spawn_extra_particles_on_fall"),
                &bool_(flag)
            );
            assert_eq!(
                Java26PlayerData::parse(raw, uuid)
                    .unwrap()
                    .spawn_extra_particles_on_fall(),
                flag
            );
        }
        retained = Java26PlayerData::parse(updated, uuid).unwrap();
    }
}

#[test]
fn silent_defaults_false_and_reads_any_numeric_flag() {
    let uuid = Uuid::from_u128(0x26_0701);
    assert!(!Java26PlayerData::parse(canonical(uuid), uuid)
        .unwrap()
        .silent());
    for flag in [false, true] {
        let mut raw = canonical(uuid);
        raw.entries_mut().push(e("Silent", bool_(flag)));
        assert_eq!(Java26PlayerData::parse(raw, uuid).unwrap().silent(), flag);
    }
    for (value, silent) in [
        (NbtSummaryValue::Byte(2), true),
        (NbtSummaryValue::Int(1), true),
        (NbtSummaryValue::String("true".into()), false),
    ] {
        let mut raw = canonical(uuid);
        raw.entries_mut().push(e("Silent", value));
        assert_eq!(Java26PlayerData::parse(raw, uuid).unwrap().silent(), silent);
    }
}

mod native_root_vehicle_attach_tests;
mod raw_root_vehicle;

mod sleep_state;

mod ender_input;

/// `ServerPlayer.java:388,398,400` read and `:411,:417,:2041` write the three
/// nullable fields; a cleared one leaves the document.
#[test]
fn nullable_server_player_fields_round_trip_and_clear() {
    let uuid = Uuid::from_u128(0x5afe);
    let patch = Java26PlayerSavePatch {
        previous_game_mode: Some(GameType::Adventure),
        entered_nether_pos: Some([1.5, 64.0, -2.25]),
        raid_omen_position: Some([3, 70, -4]),
        ..created_patch(uuid)
    };
    let created = created_player_document(&patch).unwrap();
    assert_eq!(
        field_of(&created, "previousPlayerGameType"),
        &NbtSummaryValue::Int(2)
    );
    assert_eq!(
        field_of(&created, "raid_omen_position"),
        &NbtSummaryValue::IntArray(vec![3, 70, -4])
    );
    let parsed = Java26PlayerData::parse(created, uuid).unwrap();
    assert_eq!(parsed.previous_game_mode(), Some(GameType::Adventure));
    assert_eq!(parsed.entered_nether_pos(), Some([1.5, 64.0, -2.25]));
    assert_eq!(parsed.raid_omen_position(), Some([3, 70, -4]));

    let cleared = parsed
        .raw_with_player_save_patch(
            &created_patch(uuid),
            SaveHolderResolver::unavailable().as_dyn(),
        )
        .unwrap();
    let cleared = Java26PlayerData::parse(cleared, uuid).unwrap();
    assert_eq!(cleared.previous_game_mode(), None);
    assert_eq!(cleared.entered_nether_pos(), None);
    assert_eq!(cleared.raid_omen_position(), None);
}

#[test]
fn save_patch_writes_and_clears_the_last_attackers() {
    let uuid = Uuid::from_u128(7);
    let player = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
    let mut patch = created_patch(uuid);
    let remembered = crate::Java26PlayerLastHurtBy {
        hurt_by_timestamp: 40,
        player: Some((Uuid::from_u128(8), 55)),
        mob: Some((Uuid::from_u128(9), 12)),
    };
    patch.created.last_hurt_by = remembered;
    let patched = player
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let reread = Java26PlayerData::parse(patched, uuid).unwrap();
    assert_eq!(reread.last_hurt_by(), remembered);

    patch.created.last_hurt_by = Default::default();
    let cleared = reread
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    assert!(!cleared.entries().iter().any(|entry| matches!(
        entry.name.to_jdk_string().as_str(),
        "last_hurt_by_player"
            | "last_hurt_by_player_memory_time"
            | "last_hurt_by_mob"
            | "ticks_since_last_hurt_by_mob"
    )));
    assert_eq!(
        Java26PlayerData::parse(cleared, uuid)
            .unwrap()
            .last_hurt_by(),
        Default::default()
    );
}

#[test]
fn non_finite_rotation_and_vector3f_waypoint_color_decode_safely() {
    let uuid = Uuid::from_u128(11);
    let raw = NamedCompound::new(
        "",
        vec![
            e("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            e(
                "Rotation",
                NbtSummaryValue::List(vec![f(f32::NAN), f(f32::INFINITY)]),
            ),
            e(
                "locator_bar_icon",
                NbtSummaryValue::Compound(vec![
                    e("style", NbtSummaryValue::string("minecraft:bowtie")),
                    e("color", NbtSummaryValue::List(vec![f(1.0), f(0.5), f(0.0)])),
                ]),
            ),
        ],
    );
    let player = Java26PlayerData::parse(raw, uuid).unwrap();
    assert_eq!(player.rotation(), [0.0, 0.0]);
    assert_eq!(player.locator_bar_icon().style, "minecraft:bowtie");
    assert_eq!(player.locator_bar_icon().color_rgb, Some(0x00ff_7f00));
}
