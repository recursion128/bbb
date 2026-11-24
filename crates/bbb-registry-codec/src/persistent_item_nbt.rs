//! The saved `ItemStack` structure, read the way `ItemStack.CODEC` reads it
//!
//! There is one acceptance set: Java's reader. `count` is
//! `intRange(1, 99).fieldOf("count").orElse(1)`, so a missing, non-numeric or
//! out-of-range count reads as one and any numeric tag narrows to an int; the
//! `id` and the `components` keys are the only structural refusals. Component
//! values are left to their own codecs (the summary projection).

use bbb_facts::{
    data_component_types::{
        vanilla_data_component_type_for_resource_id, DataComponentTypeCatalogEntry,
    },
    item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE},
};
use bbb_protocol::{
    codec::{ProtocolError, Result},
    content_registry::ContentRegistry,
    nbt_summary::codec_int_value,
    normalize_resource_location, NbtSummaryEntry, NbtSummaryValue,
};

/// The smallest `count` `ItemStack.MAP_CODEC` will encode, and the value
/// `orElse` supplies whenever the field could not be read
/// (`ExtraCodecs.intRange(1, 99).fieldOf("count").orElse(1)`,
pub(crate) const PERSISTENT_ITEM_STACK_MIN_COUNT: i32 = 1;

/// The `count` `ItemStack.CODEC` falls back to. Identical to the range's lower
/// bound, but named separately because it is a different fact: the bound is
/// what the codec validates, the default is what `orElse` substitutes.
pub(crate) const PERSISTENT_ITEM_STACK_DEFAULT_COUNT: i32 = 1;

pub(crate) fn decode_item_stack_fields_with_content_registry<'a>(
    value: &'a NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<Option<(i32, i32, Option<&'a NbtSummaryValue>)>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(invalid("persistent item stack root is not a compound"));
    };
    if entries.is_empty() {
        return Ok(None);
    }

    let raw_id = required_string(entries, "id", "persistent item stack root")?;
    let resource_id = normalize_resource_location(raw_id.to_string()).map_err(|_| {
        invalid(format!(
            "persistent item stack field id is not a valid resource location: {raw_id}"
        ))
    })?;
    let item_id = vanilla_item_type_for_resource_id(&resource_id)
        .map(|entry| entry.id)
        .or_else(|| {
            content_registry
                .item_by_resource_id(&resource_id)
                .map(|entry| entry.id)
        })
        .ok_or_else(|| {
            invalid(format!(
                "persistent item stack field id is not a built-in item: {resource_id}"
            ))
        })?;
    if resource_id == "minecraft:air" {
        return Err(invalid(
            "persistent item stack field id must not be minecraft:air",
        ));
    }
    let count = java_reader_count(find_entry(entries, "count"));
    Ok(Some((item_id, count, find_entry(entries, "components"))))
}

/// Measured on the 26.1 jar: a missing count, `0`, `100`, `"nonsense"` or
/// `{}` read as one; `3b`, `3L` and `3.9d` read as three.
fn java_reader_count(field: Option<&NbtSummaryValue>) -> i32 {
    field
        .and_then(codec_int_value)
        .filter(|count| {
            (PERSISTENT_ITEM_STACK_MIN_COUNT..=VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE).contains(count)
        })
        .unwrap_or(PERSISTENT_ITEM_STACK_DEFAULT_COUNT)
}

pub(crate) fn component_entries(value: &NbtSummaryValue) -> Result<&[NbtSummaryEntry]> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(invalid(
            "persistent item stack field components is not a compound",
        ));
    };
    Ok(entries)
}

pub(crate) fn decode_component_entry(
    entry: &NbtSummaryEntry,
) -> Result<(
    bool,
    &'static DataComponentTypeCatalogEntry,
    Option<&NbtSummaryValue>,
)> {
    let (removed, raw_resource_id) = match entry.name.units().strip_prefix(&[b'!' as u16]) {
        Some(units) => (true, units),
        None => (false, entry.name.units()),
    };
    // A data component patch key is a resource location; that grammar owns the bound.
    let raw_resource_id =
        String::from_utf16(raw_resource_id).map_err(|_| invalid("invalid component identifier"))?;
    let resource_id = normalize_resource_location(raw_resource_id).map_err(|_| {
        invalid(format!(
            "persistent item stack field components has invalid component key {}",
            entry.name.to_display_string()
        ))
    })?;
    let component_type =
        vanilla_data_component_type_for_resource_id(&resource_id).ok_or_else(|| {
            invalid(format!(
                "persistent item stack field components has unknown component {resource_id}"
            ))
        })?;
    if !component_type.persistent {
        return Err(invalid(format!(
            "persistent item stack field components component {resource_id} is not persistent"
        )));
    }
    if removed && !matches!(&entry.value, NbtSummaryValue::Compound(_)) {
        return Err(invalid(format!(
            "persistent item stack field components removed component {resource_id} payload is not a compound"
        )));
    }
    Ok((removed, component_type, (!removed).then_some(&entry.value)))
}

fn required_string<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a bbb_protocol::JavaString> {
    match find_entry(entries, field) {
        Some(NbtSummaryValue::String(value)) => Ok(value),
        Some(_) => Err(invalid(format!("{what} field {field} is not a string"))),
        None => Err(invalid(format!("{what} is missing field {field}"))),
    }
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], field: &str) -> Option<&'a NbtSummaryValue> {
    // A `CompoundTag` keeps the last of a repeated key.
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
}

fn invalid(detail: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidData(detail.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read_item_stack, UnavailableHolderResolver};

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn item(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        let mut root = vec![entry(
            "id",
            NbtSummaryValue::String("stone".to_string().into()),
        )];
        if !entries.iter().any(|entry| entry.name == "count") {
            root.push(entry("count", NbtSummaryValue::Int(1)));
        }
        root.extend(entries);
        compound(root)
    }

    #[test]
    fn persistent_item_nbt_empty_and_non_compound_roots() {
        let raw = compound(Vec::new());
        assert_eq!(
            read_item_stack(&raw, &UnavailableHolderResolver).unwrap(),
            bbb_protocol::packets::ItemStackSummary::empty()
        );

        assert!(read_item_stack(
            &NbtSummaryValue::List(Vec::new()),
            &UnavailableHolderResolver
        )
        .is_err());
    }

    #[test]
    fn persistent_item_nbt_normalizes_id_and_ignores_unknown_root_fields() {
        let raw = compound(vec![
            entry("id", NbtSummaryValue::String("stone".to_string().into())),
            entry("count", NbtSummaryValue::Int(1)),
            entry("unknown", NbtSummaryValue::Long(91)),
            entry(
                "unknown",
                NbtSummaryValue::String("second".to_string().into()),
            ),
        ]);
        let decoded = read_item_stack(&raw, &UnavailableHolderResolver).unwrap();
        assert_eq!(
            decoded.item_id,
            Some(bbb_facts::item_types::VANILLA_ITEM_STONE_ID)
        );
        assert_eq!(decoded.count, 1);
        assert!(decoded.component_patch.is_empty());
    }

    #[test]
    fn persistent_item_nbt_accepts_explicit_count_and_qualified_id() {
        let raw = compound(vec![
            entry(
                "id",
                NbtSummaryValue::String("minecraft:diamond".to_string().into()),
            ),
            entry("count", NbtSummaryValue::Int(99)),
        ]);
        let decoded = read_item_stack(&raw, &UnavailableHolderResolver).unwrap();
        assert_eq!(
            decoded.item_id,
            Some(bbb_facts::item_types::VANILLA_ITEM_DIAMOND_ID)
        );
        assert_eq!(decoded.count, 99);
    }

    #[test]
    fn persistent_item_nbt_removed_component_accepts_any_compound_and_hides_payload() {
        let raw = item(vec![entry(
            "components",
            compound(vec![entry("!damage", compound(Vec::new()))]),
        )]);
        let decoded = read_item_stack(&raw, &UnavailableHolderResolver).unwrap();
        assert_eq!(
            decoded.component_patch.removed_type_ids,
            [bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID]
        );
        assert!(decoded.component_patch.added_type_ids.is_empty());

        let nonempty = item(vec![entry(
            "components",
            compound(vec![entry(
                "!damage",
                compound(vec![entry("retained", NbtSummaryValue::Int(7))]),
            )]),
        )]);
        let decoded = read_item_stack(&nonempty, &UnavailableHolderResolver).unwrap();
        assert_eq!(
            decoded.component_patch.removed_type_ids,
            [bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID]
        );
        assert!(decoded.component_patch.added_type_ids.is_empty());

        let invalid = item(vec![entry(
            "components",
            compound(vec![entry("!damage", NbtSummaryValue::Int(0))]),
        )]);
        assert!(read_item_stack(&invalid, &UnavailableHolderResolver).is_err());
    }

    fn stone_with(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        let mut root = vec![entry(
            "id",
            NbtSummaryValue::String("minecraft:stone".to_string().into()),
        )];
        root.extend(entries);
        compound(root)
    }

    fn reader_count(count: Option<NbtSummaryValue>) -> i32 {
        let raw = stone_with(count.map_or_else(Vec::new, |count| vec![entry("count", count)]));
        read_item_stack(&raw, &UnavailableHolderResolver)
            .unwrap()
            .count
    }

    /// Java's reader turns every unreadable `count` into one, and bbb's reader
    /// now does the same.
    ///
    /// `ExtraCodecs.intRange(1, 99).fieldOf("count").orElse(1)`
    /// wraps the whole field, so a missing field, a
    /// value outside the range and a tag that is not a number at all are all
    /// absorbed. Measured on the 26.1 jar through `ItemStack.CODEC.parse`:
    /// `{id:"minecraft:stone"}`, `count, `count, `count:-5`,
    /// `count:"nonsense"`, `count:[1,2]`, `count:[I;3]` and `count:{}` every
    /// one of them reads back as `count=1`.
    #[test]
    fn java_reader_defaults_every_unreadable_count_to_one() {
        assert_eq!(reader_count(None), 1);
        for count in [
            NbtSummaryValue::Int(0),
            NbtSummaryValue::Int(100),
            NbtSummaryValue::Int(-5),
            NbtSummaryValue::String("nonsense".to_string().into()),
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(1), NbtSummaryValue::Int(2)]),
            NbtSummaryValue::IntArray(vec![3]),
            compound(Vec::new()),
        ] {
            assert_eq!(reader_count(Some(count.clone())), 1, "{count:?}");
        }
    }

    /// Java's reader narrows any numeric `count` tag; bbb's reader now uses
    /// the same widths.
    ///
    /// `Codec.INT` reads through `NbtOps.getNumberValue`, which answers for
    /// every `NumericTag`.
    /// Measured on the 26.1 jar: `count:3b` and `count:3L` read `3`,
    /// `count.9d` reads `3`, `count.9d` reads `99`, and
    /// `count:4294967298L` reads `2` because the narrowing keeps the low 32
    /// bits.
    #[test]
    fn java_reader_narrows_any_numeric_count_tag() {
        assert_eq!(reader_count(Some(NbtSummaryValue::Byte(3))), 3);
        assert_eq!(reader_count(Some(NbtSummaryValue::Short(3))), 3);
        assert_eq!(reader_count(Some(NbtSummaryValue::Long(3))), 3);
        assert_eq!(
            reader_count(Some(NbtSummaryValue::Double(3.9f64.to_bits()))),
            3
        );
        assert_eq!(
            reader_count(Some(NbtSummaryValue::Double(99.9f64.to_bits()))),
            99
        );
        assert_eq!(reader_count(Some(NbtSummaryValue::Long(4_294_967_298))), 2);
        assert_eq!(reader_count(Some(NbtSummaryValue::Int(99))), 99);
    }

    #[test]
    fn custom_content_registry_item_round_trips_persistent_nbt() {
        use bbb_facts::data_component_types::{
            VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
            VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
            VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        };
        use bbb_protocol::content_registry::{
            ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration,
        };
        use bbb_protocol::packets::{
            DataComponentPatchSummary, DebugStickPropertySummary, ItemStackSummary,
            ItemStackTemplateSummary, ToolRuleSummary, ToolSummary,
        };

        let mut builder = ContentRegistryBuilder::new();
        builder.register_item(ModItemRegistration {
            resource_id: "example_mod:magic_shard".to_string(),
            max_stack_size: 16,
            is_block_item: false,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "example_mod:magic_ore".to_string(),
            state_count: 2,
            light_emission: 0,
            destroy_speed: 3.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        let registry = builder.freeze().unwrap();
        let mod_item_id = registry
            .item_by_resource_id("example_mod:magic_shard")
            .unwrap()
            .id;
        let mod_block_id = registry
            .block_by_resource_id("example_mod:magic_ore")
            .unwrap()
            .block_id;

        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
        patch.use_remainder = Some(Box::new(ItemStackTemplateSummary {
            item_id: mod_item_id,
            count: 2,
            component_patch: Default::default(),
        }));
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID);
        patch.repairable_item_registry_ids = vec![mod_item_id];
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID);
        patch.pot_decorations_item_ids = vec![mod_item_id; 4];
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
        patch.tool = Some(ToolSummary {
            rules: vec![ToolRuleSummary {
                block_registry_ids: vec![mod_block_id],
                block_tag: None,
                speed_bits: Some(6.0_f32.to_bits()),
                correct_for_drops: Some(true),
            }],
            default_mining_speed_bits: 1.0_f32.to_bits(),
            damage_per_block: 1,
            can_destroy_blocks_in_creative: true,
        });
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID);
        patch.debug_stick_properties = vec![DebugStickPropertySummary {
            block_registry_id: mod_block_id,
            property: "lit".into(),
        }];
        patch.added_type_ids.sort_unstable();

        let stack = ItemStackSummary {
            item_id: Some(mod_item_id),
            count: 3,
            component_patch: patch.into(),
        };
        let nbt = crate::item_stack_summary_to_persistent_nbt_with_resolver_and_content_registry(
            &stack,
            &UnavailableHolderResolver,
            &registry,
        )
        .unwrap();
        let decoded = crate::read_item_stack_with_content_registry(
            &nbt,
            &UnavailableHolderResolver,
            &registry,
        )
        .unwrap();
        assert_eq!(decoded, stack);
        assert!(crate::read_item_stack(&nbt, &UnavailableHolderResolver).is_err());
    }
}
