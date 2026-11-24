//! Guard rails for the persistent `ItemStack.CODEC` projection: official 26.1
//! stack shapes must read into an `ItemStackSummary` without loss, and the
//! summary must survive write -> read unchanged.

use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::{parse_snbt_compound, NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver;

use crate::world::registry_runtime::item_stack_resolver_for_test;

/// Compounds compare as maps: `CompoundTag` keeps no order.
fn canonical(value: &NbtSummaryValue) -> NbtSummaryValue {
    match value {
        NbtSummaryValue::Compound(entries) => {
            let mut entries: Vec<NbtSummaryEntry> = entries
                .iter()
                .map(|entry| NbtSummaryEntry {
                    name: entry.name.clone(),
                    value: canonical(&entry.value),
                })
                .collect();
            entries.sort_by_key(|entry| entry.name.to_jdk_string());
            NbtSummaryValue::Compound(entries)
        }
        NbtSummaryValue::List(values) => {
            NbtSummaryValue::List(values.iter().map(canonical).collect())
        }
        other => other.clone(),
    }
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name.to_jdk_string() == name)
        .map(|entry| &entry.value)
}

fn component_keys(stack: &NbtSummaryValue) -> Vec<String> {
    match field(stack, "components") {
        Some(NbtSummaryValue::Compound(entries)) => entries
            .iter()
            .map(|entry| entry.name.to_jdk_string())
            .collect(),
        _ => Vec::new(),
    }
}

struct RoundTrip {
    summary: ItemStackSummary,
    written: NbtSummaryValue,
}

/// read -> summary -> write -> read -> summary, failing on any lost or changed
/// state between the two summaries.
fn round_trip(input: &NbtSummaryValue) -> Result<RoundTrip, String> {
    let resolver = item_stack_resolver_for_test();
    let summary = bbb_registry_codec::read_item_stack(input, resolver)
        .map_err(|error| format!("first projection: {error:?}"))?;
    let written = item_stack_summary_to_persistent_nbt_with_resolver(&summary, resolver)
        .map_err(|error| format!("write: {error:?}"))?;
    let second = bbb_registry_codec::read_item_stack(&written, resolver)
        .map_err(|error| format!("second projection: {error:?}"))?;
    if second != summary {
        return Err(format!(
            "summary changed across write/read:\n first: {summary:?}\nsecond: {second:?}"
        ));
    }
    Ok(RoundTrip { summary, written })
}

/// Every `components` key of the input is present in the summary, as an
/// addition or (for `!` keys) a removal.
fn check_components_present(
    input: &NbtSummaryValue,
    summary: &ItemStackSummary,
) -> Result<(), String> {
    for key in component_keys(input) {
        let (removed, resource_id) = match key.strip_prefix('!') {
            Some(resource_id) => (true, resource_id),
            None => (false, key.as_str()),
        };
        let type_id = vanilla_data_component_type_for_resource_id(resource_id)
            .ok_or_else(|| format!("unknown component {resource_id}"))?
            .type_id;
        let patch = &summary.component_patch;
        let present = if removed {
            patch.removed_type_ids.contains(&type_id)
        } else {
            patch.added_type_ids.contains(&type_id)
        };
        if !present {
            return Err(format!("component {key} lost on read: {patch:?}"));
        }
    }
    Ok(())
}

fn fixture_json(relative: &str) -> serde_json::Value {
    let path = format!("{}/../../{relative}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn report(failures: &[String], total: usize) {
    assert!(
        failures.is_empty(),
        "{} of {total} stacks failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every distinct stack the official server's `LootTable.fill` wrote through
/// `ItemStack.CODEC` (`scripts/generate-chest-loot-fill-fixture.py`).
#[test]
fn official_chest_loot_stacks_round_trip_without_loss() {
    let fixture = fixture_json("scripts/data/chest-loot-fills-26.1.json");
    let mut stacks = BTreeSet::new();
    for seeds in fixture["tables"].as_object().expect("tables").values() {
        for slots in seeds.as_object().expect("seeds").values() {
            for snbt in slots.as_object().expect("slots").values() {
                stacks.insert(snbt.as_str().expect("an SNBT stack").to_owned());
            }
        }
    }
    assert!(
        stacks.len() > 500,
        "the fixture lists {} distinct stacks",
        stacks.len()
    );

    let mut failures = Vec::new();
    let mut with_components = 0;
    for snbt in &stacks {
        let input = parse_snbt_compound(snbt).unwrap_or_else(|error| panic!("{snbt}: {error:?}"));
        if !component_keys(&input).is_empty() {
            with_components += 1;
        }
        let outcome = round_trip(&input).and_then(|trip| {
            check_components_present(&input, &trip.summary)?;
            // The fixture is codec output, so the canonical writer must
            // reproduce it exactly.
            if canonical(&trip.written) != canonical(&input) {
                return Err(format!("written form differs: {:?}", trip.written));
            }
            Ok(())
        });
        if let Err(error) = outcome {
            failures.push(format!("{snbt}: {error}"));
        }
    }
    assert!(
        with_components >= 100,
        "only {with_components} stacks carry components"
    );
    report(&failures, stacks.len());
}

struct PrototypeFixture {
    item_ids: Vec<String>,
    /// Persistent component resource ids each item's prototype declares.
    declared: BTreeMap<i64, Vec<String>>,
}

fn prototype_fixture() -> PrototypeFixture {
    let fixture = fixture_json("crates/bbb-facts/data/item_prototype_components_26_1.json");
    let item_ids = fixture["item_resource_ids"]
        .as_array()
        .expect("item_resource_ids")
        .iter()
        .map(|id| id.as_str().expect("an item id").to_owned())
        .collect();
    let mut declared: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for component in fixture["component_values"]
        .as_array()
        .expect("component_values")
    {
        let resource_id = component["resource_id"].as_str().expect("resource_id");
        for value in component["values"].as_array().expect("values") {
            for range in value["item_id_ranges"].as_array().expect("ranges") {
                let low = range[0].as_i64().expect("range start");
                let high = range[1].as_i64().expect("range end");
                for item in low..=high {
                    declared
                        .entry(item)
                        .or_default()
                        .push(resource_id.to_owned());
                }
            }
        }
    }
    PrototypeFixture { item_ids, declared }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::String(value.into()),
    }
}

fn plain_stack(item: &str, components: Option<NbtSummaryValue>) -> NbtSummaryValue {
    let mut entries = vec![
        string_entry("id", item),
        NbtSummaryEntry {
            name: "count".into(),
            value: NbtSummaryValue::Int(1),
        },
    ];
    if let Some(components) = components {
        entries.push(NbtSummaryEntry {
            name: "components".into(),
            value: components,
        });
    }
    NbtSummaryValue::Compound(entries)
}

/// The prototype fixture holds `DataComponentMap` values, not stack patches,
/// so each item id becomes the default stack `{id, count:1}`.
#[test]
fn every_item_default_stack_round_trips() {
    let fixture = prototype_fixture();
    let mut failures = Vec::new();
    let mut total = 0;
    for item in fixture.item_ids.iter().filter(|id| *id != "minecraft:air") {
        total += 1;
        let input = plain_stack(item, None);
        let outcome = round_trip(&input).and_then(|trip| {
            if !trip.summary.component_patch.added_type_ids.is_empty()
                || !trip.summary.component_patch.removed_type_ids.is_empty()
            {
                return Err(format!("a default stack grew a patch: {:?}", trip.summary));
            }
            if canonical(&trip.written) != canonical(&input) {
                return Err(format!("written form differs: {:?}", trip.written));
            }
            Ok(())
        });
        if let Err(error) = outcome {
            failures.push(format!("{item}: {error}"));
        }
    }
    report(&failures, total);
}

/// `DataComponentPatch.CODEC` removal keys (`DataComponentPatch.java:290-313`)
/// for every component each item's prototype declares; a removal value is
/// `Codec.EMPTY` (`:315-317`), written as `{}`.
#[test]
fn every_item_prototype_removal_patch_round_trips() {
    let fixture = prototype_fixture();
    let mut failures = Vec::new();
    let mut total = 0;
    for (index, item) in fixture.item_ids.iter().enumerate() {
        let Some(declared) = fixture.declared.get(&(index as i64)) else {
            continue;
        };
        if item == "minecraft:air" {
            continue;
        }
        total += 1;
        let removals = declared
            .iter()
            .map(|resource_id| NbtSummaryEntry {
                name: format!("!{resource_id}").as_str().into(),
                value: NbtSummaryValue::Compound(Vec::new()),
            })
            .collect();
        let input = plain_stack(item, Some(NbtSummaryValue::Compound(removals)));
        let outcome = round_trip(&input).and_then(|trip| {
            check_components_present(&input, &trip.summary)?;
            if canonical(&trip.written) != canonical(&input) {
                return Err(format!("written form differs: {:?}", trip.written));
            }
            Ok(())
        });
        if let Err(error) = outcome {
            failures.push(format!("{item}: {error}"));
        }
    }
    assert!(
        total > 1000,
        "only {total} items declare prototype components"
    );
    report(&failures, total);
}

/// One hand-written official stack: round-trips, keeps every component it
/// names, and (when `exact`) writes back the codec's own canonical form.
fn check_hand_case(snbt: &str, exact: bool) {
    let input = parse_snbt_compound(snbt).unwrap_or_else(|error| panic!("{snbt}: {error:?}"));
    let trip = round_trip(&input).unwrap_or_else(|error| panic!("{snbt}: {error}"));
    check_components_present(&input, &trip.summary)
        .unwrap_or_else(|error| panic!("{snbt}: {error}"));
    if exact {
        assert_eq!(
            canonical(&trip.written),
            canonical(&input),
            "{snbt}: written form differs"
        );
    }
}

macro_rules! hand_case {
    ($(#[$meta:meta])* $name:ident, exact: $exact:expr, $snbt:expr) => {
        $(#[$meta])*
        #[test]
        fn $name() {
            check_hand_case($snbt, $exact);
        }
    };
}

// `!` removal keys (`DataComponentPatch.java:290-317`) beside an addition.
hand_case!(
    removed_prototype_components,
    exact: true,
    r##"{id:"minecraft:diamond_sword",count:1,components:{"!minecraft:tool":{},"!minecraft:enchantable":{},"minecraft:damage":7}}"##
);

// `InstrumentComponent.CODEC` is `Instrument.CODEC`, a `RegistryFileCodec`
// (`InstrumentComponent.java:18`, `Instrument.java:37`): a reference id ...
hand_case!(
    instrument_reference_holder,
    exact: true,
    r##"{id:"minecraft:goat_horn",count:1,components:{"minecraft:instrument":"minecraft:admire_goat_horn"}}"##
);

// ... or an inline `Instrument.DIRECT_CODEC` (`Instrument.java:17-25`), whose
// `sound_event` is itself a `RegistryFileCodec` (`SoundEvent.java:16-22`).
hand_case!(
    instrument_inline_holder,
    exact: true,
    r##"{id:"minecraft:goat_horn",count:1,components:{"minecraft:instrument":{sound_event:"minecraft:item.goat_horn.sound.3",use_duration:5.0f,range:128.0f,description:{translate:"instrument.custom.horn"}}}}"##
);

hand_case!(
    instrument_inline_holder_with_inline_sound,
    exact: true,
    r##"{id:"minecraft:goat_horn",count:1,components:{"minecraft:instrument":{sound_event:{sound_id:"minecraft:custom.horn",range:32.0f},use_duration:7.0f,range:256.0f,description:"Custom horn"}}}"##
);

// `JukeboxSong.CODEC` is a `RegistryFixedCodec` (`JukeboxSong.java:40`): 26.1
// accepts only a reference here, never an inline song.
hand_case!(
    jukebox_playable_reference,
    exact: true,
    r##"{id:"minecraft:stick",count:1,components:{"minecraft:jukebox_playable":"minecraft:pigstep"}}"##
);

// `ArmorTrim.CODEC` (`ArmorTrim.java:22-25`): material and pattern are both
// `RegistryFileCodec`s (`TrimMaterial.java:25`, `TrimPattern.java:33`).
hand_case!(
    trim_reference_holders,
    exact: true,
    r##"{id:"minecraft:diamond_chestplate",count:1,components:{"minecraft:trim":{material:"minecraft:gold",pattern:"minecraft:coast"}}}"##
);

// Inline `TrimMaterial.DIRECT_CODEC` (`TrimMaterial.java:15-21`,
// `MaterialAssetGroup.java:20-27`) and `TrimPattern.DIRECT_CODEC`
// (`TrimPattern.java:16-23`).
hand_case!(
    trim_inline_holders,
    exact: true,
    r##"{id:"minecraft:iron_leggings",count:1,components:{"minecraft:trim":{material:{asset_name:"custom_metal",description:{text:"Custom metal",color:"#123456"}},pattern:{asset_id:"minecraft:custom_pattern",description:{translate:"trim_pattern.custom"},decal:1b}}}}"##
);

// `BundleContents.CODEC` is a list of `ItemStackTemplate.CODEC`
// (`BundleContents.java:28`, `ItemStackTemplate.java:21-29`): nested stacks
// carry their own patch, and the encoder omits a default `count` of 1.
hand_case!(
    bundle_contents_nested_stacks,
    exact: true,
    r##"{id:"minecraft:bundle",count:1,components:{"minecraft:bundle_contents":[{id:"minecraft:diamond",count:3},{id:"minecraft:iron_sword",components:{"minecraft:damage":5,"minecraft:custom_name":"Nested"}},{id:"minecraft:enchanted_book",components:{"minecraft:stored_enchantments":{"minecraft:sharpness":3}}}]}}"##
);

// `ItemStackTemplate.CODEC` also reads the bare `Item.CODEC` id and an absent
// `count` (`ItemStackTemplate.java:24`, `:29`).
hand_case!(
    bundle_contents_template_alternative_forms,
    exact: false,
    r##"{id:"minecraft:bundle",count:1,components:{"minecraft:bundle_contents":["minecraft:stick",{id:"minecraft:apple"}]}}"##
);

// `ItemContainerContents.CODEC` (`ItemContainerContents.java:28-30`,
// `:176-182`): sparse `{slot, item}` entries, with a bundle nested inside.
hand_case!(
    container_nested_stacks,
    exact: true,
    r##"{id:"minecraft:shulker_box",count:1,components:{"minecraft:container":[{slot:0,item:{id:"minecraft:diamond",count:64}},{slot:13,item:{id:"minecraft:bundle",components:{"minecraft:bundle_contents":[{id:"minecraft:emerald",count:2}]}}},{slot:26,item:{id:"minecraft:potion",components:{"minecraft:potion_contents":{potion:"minecraft:swiftness"}}}}]}}"##
);

// `custom_name` is `ComponentSerialization.CODEC` and `lore` its
// size-limited list (`DataComponents.java:123`, `:138`, `ItemLore.java:24`).
hand_case!(
    custom_name_and_lore_text_components,
    exact: true,
    r##"{id:"minecraft:diamond",count:1,components:{"minecraft:custom_name":{text:"Shiny",color:"gold",bold:1b,extra:[{translate:"item.minecraft.diamond",italic:0b}]},"minecraft:lore":["Plain line",{text:"Red line",color:"red",italic:0b},{translate:"chat.type.text",with:["a",{text:"b",underlined:1b}]}]}}"##
);

hand_case!(
    custom_name_plain_string,
    exact: true,
    r##"{id:"minecraft:name_tag",count:1,components:{"minecraft:custom_name":"Plain"}}"##
);
