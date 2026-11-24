//! The `StringRepresentable` enum components: `minecraft:dye` and the animal
//! variant, collar and colour fields.
//!
//! Every one is a `StringRepresentable.fromEnum` codec on the persistent side
//! and an `ByIdMap` stream codec on the network side, so the save form is the
//! enum constant's serialized name and the wire form is an ordinal the
//! generated catalogs already issue. They are tabled rather than spelled out
//! one by one because the only per-component fact is which catalog answers.

use bbb_facts::builtin_enum_component_values::{
    builtin_enum_component_value_for_name, builtin_enum_component_value_name,
    VANILLA_BUILTIN_ENUM_COMPONENTS,
};
use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
use bbb_facts::tropical_fish_patterns::{
    vanilla_tropical_fish_pattern, vanilla_tropical_fish_pattern_for_name,
};

use super::builtin_enum_components::BUILTIN_ENUM_COMPONENT_TYPE_IDS;
use super::*;

/// One component's save form plus a second, different value so the round trip
/// cannot pass by accident on a constant.
struct Probe {
    resource_id: &'static str,
    first: &'static str,
    second: &'static str,
}

fn probes() -> Vec<Probe> {
    let mut probes = vec![Probe {
        resource_id: "minecraft:dye",
        first: "white",
        second: "light_blue",
    }];
    for entry in VANILLA_BUILTIN_ENUM_COMPONENTS {
        // `minecraft:base_color` is already typed-encoded and reached its own
        // test; `minecraft:dye` has a field of its own.
        if !BUILTIN_ENUM_COMPONENT_TYPE_IDS.contains(&entry.type_id) {
            continue;
        }
        let names = entry.value_type.values;
        let resource_id =
            bbb_facts::data_component_types::vanilla_data_component_type(entry.type_id)
                .expect("built-in enum components are catalogued")
                .resource_id;
        probes.push(Probe {
            resource_id,
            first: names
                .first()
                .expect("every enum has a value")
                .serialized_name,
            second: names
                .last()
                .expect("every enum has a value")
                .serialized_name,
        });
    }
    // `minecraft:tropical_fish/pattern` is a packed-id enum, not a built-in
    // enum component descriptor.
    probes.push(Probe {
        resource_id: "minecraft:tropical_fish/pattern",
        first: "kob",
        second: "glitter",
    });
    probes
}

/// A component's save form names it, and the value crosses as the enum's
/// serialized name with no publication in either direction.
#[test]
fn every_enum_component_round_trips_its_serialized_name() {
    for probe in probes() {
        for name in [probe.first, probe.second] {
            let saved = saved_stack_with_components(
                "minecraft:stone",
                vec![entry(
                    probe.resource_id,
                    NbtSummaryValue::String(name.to_string().into()),
                )],
            );
            let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver)
                .unwrap_or_else(|error| panic!("{} {name}: {error}", probe.resource_id));
            assert_eq!(
                item_stack_summary_to_persistent_nbt(&summary).unwrap(),
                saved.clone(),
                "{} {name}",
                probe.resource_id
            );
            // The pair is a fixpoint, and the projected summary is wire-shaped.
            bbb_protocol::packets::validate_data_component_patch_summary(&summary.component_patch)
                .unwrap_or_else(|error| panic!("{} {name}: {error}", probe.resource_id));
        }
    }
}

/// The id the wire carries is the catalog's own, so the two names above really
/// reach different ids.
#[test]
fn the_two_probe_names_reach_different_ids() {
    for probe in probes() {
        let ids = [probe.first, probe.second].map(|name| {
            if probe.resource_id == "minecraft:tropical_fish/pattern" {
                vanilla_tropical_fish_pattern_for_name(name)
                    .expect("known pattern")
                    .packed_id
            } else {
                builtin_enum_component_value_for_name(resource_id_type_id(probe.resource_id), name)
                    .unwrap_or_else(|| panic!("{} {name}", probe.resource_id))
                    .id
            }
        });
        assert_ne!(ids[0], ids[1], "{}", probe.resource_id);
    }
}

fn resource_id_type_id(resource_id: &str) -> i32 {
    if resource_id == "minecraft:dye" {
        return VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
    }
    bbb_facts::data_component_types::vanilla_data_component_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is not a catalogued component"))
        .type_id
}

/// The wire direction's own copy of the name comes back out unchanged, which is
/// what ties the enum catalogs to the wire decoders.
#[test]
fn the_catalog_name_is_the_one_the_wire_carries() {
    for probe in probes() {
        for name in [probe.first, probe.second] {
            if probe.resource_id == "minecraft:tropical_fish/pattern" {
                let packed = vanilla_tropical_fish_pattern_for_name(name)
                    .unwrap()
                    .packed_id;
                assert_eq!(vanilla_tropical_fish_pattern(packed).serialized_name, name);
                continue;
            }
            let type_id = resource_id_type_id(probe.resource_id);
            let id = builtin_enum_component_value_for_name(type_id, name)
                .unwrap_or_else(|| panic!("{} {name}", probe.resource_id))
                .id;
            assert_eq!(builtin_enum_component_value_name(type_id, id), Some(name));
        }
    }
}
