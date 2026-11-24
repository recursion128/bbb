//! The recursive `ItemStackTemplate` projections the four nested-item-list
//! components carry: `minecraft:container`, `minecraft:bundle_contents`,
//! `minecraft:charged_projectiles` and `minecraft:use_remainder`.
//!
//! `ItemStackTemplate.CODEC` is `Codec.withAlternative(MAP_CODEC.codec(),
//! Item.CODEC, ...)`, so the save form is
//! either a bare item identifier or `{id, count?, components?}` --
//! `count` defaults to `1` and `optionalFieldOf` drops it while encoding
//! which is the one shape difference from the
//! enclosing `ItemStack.CODEC` that always writes `count`
//!
//! Both directions hand each nested `components` patch back to the same entry
//! points the enclosing stack uses, so a nested component the projection
//! cannot carry is the same named refusal it is at the top level.
//! `MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH` (`bbb_protocol::packets`) bounds the
//! nesting of the summary this produces, and every patch level re-runs
//! `validate_data_component_patch_summary`, so a save nested deeper than the
//! wire can carry is refused rather than silently flattened.

use bbb_facts::item_types::VANILLA_ITEM_AIR_ID;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::{
    packets::{
        data_components::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH, DataComponentPatchSummary,
        ItemStackTemplateSummary, SparseDataComponentPatchSummary,
    },
    NbtSummaryEntry, NbtSummaryValue,
};

use super::{
    error::{invalid_persistent_component, Result},
    holder::{item_id_in_registry, item_resource_id_in_registry, ITEM_REGISTRY},
    item_stack::{data_component_patch_to_persistent_nbt, project_persistent_patch_into_at_depth},
    nbt_support::{
        latest_field, normalize_resource_id_units, ComponentReader, MAX_PERSISTENT_LIST_ITEMS,
        MAX_PREALLOC_CAPACITY,
    },
};

/// `ExtraCodecs.intRange(1, 99)` on `ItemStackTemplate.count`
/// (`ItemStackTemplate.java:24`), whose constructor rejects a count of zero and
/// `minecraft:air` beside it (`ItemStackTemplate.java:52-60`).
const PERSISTENT_TEMPLATE_COUNTS: std::ops::RangeInclusive<i32> = 1..=99;

/// The `Item.CODEC` / `count` / `components` triple `ItemStackTemplate.MAP_CODEC`
/// declares, with the defaults its two `optionalFieldOf`s read.
pub struct ItemStackTemplateFields<'a> {
    pub item_id: i32,
    pub resource_id: String,
    pub count: i32,
    pub components: Option<&'a NbtSummaryValue>,
}

/// Reads the template header and borrows its optional component source.
pub fn read_item_stack_template_fields(
    value: &NbtSummaryValue,
) -> std::result::Result<ItemStackTemplateFields<'_>, String> {
    read_item_stack_template_fields_with_content_registry(value, ContentRegistry::vanilla())
}

/// Registry-aware form of [`read_item_stack_template_fields`].
pub fn read_item_stack_template_fields_with_content_registry<'a>(
    value: &'a NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> std::result::Result<ItemStackTemplateFields<'a>, String> {
    let (id, count, components) = match value {
        NbtSummaryValue::String(id) => (id, 1, None),
        NbtSummaryValue::Compound(fields) => {
            let id = latest_field(fields, "id")
                .and_then(super::nbt_support::nbt_string)
                .ok_or_else(|| "item stack template is missing a string id".to_string())?;
            let count = latest_field(fields, "count")
                .map(|count| {
                    bbb_protocol::nbt_summary::codec_int_value(count)
                        .ok_or_else(|| "item stack template count is not numeric".to_string())
                })
                .transpose()?
                .unwrap_or(1);
            (id, count, latest_field(fields, "components"))
        }
        _ => return Err("item stack template is neither an item id nor a compound".to_string()),
    };
    if !PERSISTENT_TEMPLATE_COUNTS.contains(&count) {
        return Err(format!(
            "item stack template count {count} is outside 1..=99"
        ));
    }
    let resource_id = normalize_resource_id_units(id.units()).map_err(|error| error.to_string())?;
    let item_id = item_id_in_registry(&resource_id, content_registry)
        .ok_or_else(|| format!("unknown {ITEM_REGISTRY} holder {resource_id}"))?;
    if item_id == VANILLA_ITEM_AIR_ID {
        return Err("minecraft:air is not an item stack template".to_string());
    }
    Ok(ItemStackTemplateFields {
        item_id,
        resource_id,
        count,
        components,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn project_item_stack_template(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<ItemStackTemplateSummary> {
    if item_depth >= MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!(
                "item stack template nesting depth exceeds {MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH}"
            ),
        ));
    }
    let fields = read_item_stack_template_fields_with_content_registry(value, content_registry)
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error))?;
    let component_patch = if let Some(components) = fields.components {
        let mut patch = Box::<DataComponentPatchSummary>::default();
        project_persistent_patch_into_at_depth(
            components,
            holder_id,
            content_registry,
            &mut patch,
            predicate_depth,
            item_depth + 1,
        )
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))?;
        patch.into()
    } else {
        SparseDataComponentPatchSummary::empty()
    };
    Ok(ItemStackTemplateSummary {
        item_id: fields.item_id,
        count: fields.count,
        component_patch,
    })
}

/// `ItemStackTemplate.CODEC.listOf()` (`ChargedProjectiles`,
/// `BundleContents`), whose `NbtOps` list also reads the specialized
/// array tags.
#[allow(clippy::too_many_arguments)]
pub(super) fn project_item_stack_template_list(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<Vec<ItemStackTemplateSummary>> {
    let reader = ComponentReader::new(type_id, resource_id);
    let items = reader.list(value, "item stack templates")?;
    if items.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(reader.error(format!(
            "item stack template list has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            items.len()
        )));
    }
    let mut out = Vec::with_capacity(items.len().min(MAX_PREALLOC_CAPACITY));
    for item in items.iter() {
        out.push(project_item_stack_template(
            type_id,
            resource_id,
            &item,
            holder_id,
            content_registry,
            predicate_depth,
            item_depth,
        )?);
    }
    Ok(out)
}

/// `ItemStackTemplate.CODEC` written back: `id` always, `count` only when it
/// differs from the `1` its `optionalFieldOf` defaults to, `components` only
/// when the patch is not `DataComponentPatch.EMPTY`.
#[allow(clippy::too_many_arguments)]
pub(super) fn persistent_item_stack_template_nbt(
    type_id: i32,
    resource_id: &'static str,
    template: &ItemStackTemplateSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<NbtSummaryValue> {
    if item_depth >= MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!(
                "item stack template nesting depth exceeds {MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH}"
            ),
        ));
    }
    if !PERSISTENT_TEMPLATE_COUNTS.contains(&template.count) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("item stack count is outside 1..=99: {}", template.count),
        ));
    }
    let item_resource_id = item_resource_id_in_registry(template.item_id, content_registry)
        .ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {ITEM_REGISTRY} holder id {}", template.item_id),
            )
        })?;
    if template.item_id == VANILLA_ITEM_AIR_ID {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            "minecraft:air is not an item stack template".to_string(),
        ));
    }

    let mut fields = vec![NbtSummaryEntry {
        name: "id".into(),
        value: NbtSummaryValue::String(item_resource_id.into()),
    }];
    if template.count != 1 {
        fields.push(NbtSummaryEntry {
            name: "count".into(),
            value: NbtSummaryValue::Int(template.count),
        });
    }
    if let Some(components) = data_component_patch_to_persistent_nbt(
        &template.component_patch,
        holder_resource_id,
        content_registry,
        predicate_depth,
        item_depth + 1,
    )? {
        fields.push(NbtSummaryEntry {
            name: "components".into(),
            value: components,
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// [`persistent_item_stack_template_nbt`] over a `listOf()` field.
#[allow(clippy::too_many_arguments)]
pub(super) fn persistent_item_stack_template_list_nbt(
    type_id: i32,
    resource_id: &'static str,
    templates: &[ItemStackTemplateSummary],
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<NbtSummaryValue> {
    let mut items = Vec::with_capacity(templates.len());
    for template in templates {
        items.push(persistent_item_stack_template_nbt(
            type_id,
            resource_id,
            template,
            holder_resource_id,
            content_registry,
            predicate_depth,
            item_depth,
        )?);
    }
    Ok(NbtSummaryValue::List(items))
}
