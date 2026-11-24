//! `ItemStackTemplate` (`ItemStackTemplate.java:19-96`) and the
//! `PatchedDataComponentMap` writes it builds stacks through, over the native
//! component table. Recipe results and every other template consumer share it.

use std::fmt;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_TYPES,
};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::data_components::fields::{
    clear_component_value, component_values_equal, copy_component_value,
};
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemStackSummary, ItemStackTemplateSummary,
};
use bbb_registry_codec::{
    decode_direct_registry_json, item_stack_template_from_source_with_content_registry,
    registry_codec::RegistryReferenceLookup, PersistentItemStackHolderResolver,
    RegistryElementSource, RegistryElementValueRef,
};
use serde_json::{Map, Value};

mod fraction;

use fraction::Fraction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStackTemplateError {
    pub message: String,
}

impl fmt::Display for ItemStackTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ItemStackTemplateError {}

type Result<T> = std::result::Result<T, ItemStackTemplateError>;

fn error(message: impl Into<String>) -> ItemStackTemplateError {
    ItemStackTemplateError {
        message: message.into(),
    }
}

fn native<T>(result: bbb_protocol::codec::Result<T>) -> Result<T> {
    result.map_err(|err| error(err.to_string()))
}

/// The prototype an item registers, decoded once through the same
/// `DataComponentPatch.CODEC` reader a declared entry takes, so the collapse
/// `PatchedDataComponentMap` makes is a comparison between typed values rather
/// than between codec shapes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrototypeDefaults {
    item_id: i32,
    values: DataComponentPatchSummary,
}

impl PrototypeDefaults {
    /// Every component `item_id` registers a default for.
    pub fn of_item(
        item_id: i32,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self> {
        let content_registry = lookup.content_registry();
        if let Some(item) = vanilla_item_type(item_id) {
            let mut component_values = Map::new();
            for component in VANILLA_DATA_COMPONENT_TYPES.iter() {
                if let Some(value) = prototype_component_value(item_id, component.type_id) {
                    component_values.insert(component.resource_id.to_string(), value.clone());
                }
            }
            let mut root = Map::new();
            root.insert(
                "id".to_string(),
                Value::String(item.resource_id.to_string()),
            );
            root.insert("components".to_string(), Value::Object(component_values));
            let bytes =
                serde_json::to_vec(&Value::Object(root)).map_err(|err| error(err.to_string()))?;
            let decoded =
                decode_direct_registry_json(&bytes).map_err(|err| error(err.to_string()))?;
            let summary = item_stack_template_from_source_with_content_registry(
                decoded.root(),
                RegistryElementSource::ServerDatapackJson,
                lookup,
                content_registry,
            )
            .map_err(|err| {
                error(format!(
                    "the default this item registers does not decode: {err}"
                ))
            })?;
            return Ok(Self {
                item_id,
                values: summary.component_patch.into_inner(),
            });
        }
        if let Some(mod_item) = content_registry.item_by_id(item_id) {
            let mut values = DataComponentPatchSummary::default();
            values
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
            values.max_stack_size = Some(i32::from(mod_item.max_stack_size));
            return Ok(Self { item_id, values });
        }
        Err(error(format!("unknown item type {item_id}")))
    }

    pub fn item_id(&self) -> i32 {
        self.item_id
    }

    /// `prototype.get(type) != null` (`PatchedDataComponentMap.java:90`, `:115`).
    pub fn has(&self, type_id: i32) -> bool {
        prototype_has_component(self.item_id, type_id)
            || self.values.added_type_ids.contains(&type_id)
    }

    fn max_stack_size(&self, patch: &DataComponentPatchSummary) -> i32 {
        match patch_declares(patch, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID) {
            Some(false) => 1,
            Some(true) => patch.max_stack_size.unwrap_or(1),
            None => self
                .values
                .max_stack_size
                .or_else(|| {
                    prototype_component_value(
                        self.item_id,
                        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                    )
                    .and_then(codec_int)
                })
                .unwrap_or(1),
        }
    }

    /// `Objects.equals(value, defaultValue)` for a present value
    /// (`PatchedDataComponentMap.java:48`, `:73`, `:110`).
    fn equals_default(&self, patch: &DataComponentPatchSummary, type_id: i32) -> Result<bool> {
        if !self.values.added_type_ids.contains(&type_id) {
            return Ok(false);
        }
        native(component_values_equal(type_id, &self.values, patch))
    }
}

/// `patch.put(type, Optional.of(value))`: an existing key keeps its place.
fn put_value(
    patch: &mut DataComponentPatchSummary,
    type_id: i32,
    source: &DataComponentPatchSummary,
) -> Result<()> {
    native(copy_component_value(type_id, source, patch))?;
    patch.removed_type_ids.retain(|id| *id != type_id);
    if !patch.added_type_ids.contains(&type_id) {
        patch.added_type_ids.push(type_id);
    }
    Ok(())
}

/// `patch.put(type, Optional.empty())`.
fn put_removal(patch: &mut DataComponentPatchSummary, type_id: i32) -> Result<()> {
    native(clear_component_value(type_id, patch))?;
    patch.added_type_ids.retain(|id| *id != type_id);
    if !patch.removed_type_ids.contains(&type_id) {
        patch.removed_type_ids.push(type_id);
    }
    Ok(())
}

/// `patch.remove(type)`: the entry goes, and the typed fields it owned with it.
fn remove_entry(patch: &mut DataComponentPatchSummary, type_id: i32) -> Result<()> {
    native(clear_component_value(type_id, patch))?;
    patch.added_type_ids.retain(|id| *id != type_id);
    patch.removed_type_ids.retain(|id| *id != type_id);
    Ok(())
}

/// `PatchedDataComponentMap.applyPatch` (`PatchedDataComponentMap.java:99-120`):
/// a value equal to the prototype default drops the entry, any other value
/// replaces it, and a removal survives only where the prototype has something
/// to remove. Only the keyed fields of `overlay` are read.
pub fn apply_patch(
    base: &mut DataComponentPatchSummary,
    overlay: &DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
) -> Result<()> {
    for &type_id in &overlay.added_type_ids {
        if defaults.equals_default(overlay, type_id)? {
            remove_entry(base, type_id)?;
        } else {
            put_value(base, type_id, overlay)?;
        }
    }
    for &type_id in &overlay.removed_type_ids {
        if defaults.has(type_id) {
            put_removal(base, type_id)?;
        } else {
            remove_entry(base, type_id)?;
        }
    }
    Ok(())
}

/// `PatchedDataComponentMap.fromPatch` (`PatchedDataComponentMap.java:34-42`).
/// Replaying a patch `isPatchSanitized` (`:44-58`) accepts leaves it unchanged,
/// so the replay alone covers both branches.
pub fn patch_from(
    patch: &DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
) -> Result<DataComponentPatchSummary> {
    let mut sanitized = DataComponentPatchSummary::default();
    apply_patch(&mut sanitized, patch, defaults)?;
    Ok(sanitized)
}

/// `PatchedDataComponentMap.set` (`PatchedDataComponentMap.java:69-80`) with
/// the value `type_id` keys in `value`.
pub fn set_component(
    patch: &mut DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
    type_id: i32,
    value: &DataComponentPatchSummary,
) -> Result<()> {
    native(component_values_equal(type_id, value, value))?;
    if defaults.equals_default(value, type_id)? {
        remove_entry(patch, type_id)
    } else {
        put_value(patch, type_id, value)
    }
}

/// `PatchedDataComponentMap.remove` (`PatchedDataComponentMap.java:86-97`),
/// which `set` with a null value (`:73-77`) resolves to as well.
pub fn remove_component(
    patch: &mut DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
    type_id: i32,
) -> Result<()> {
    if defaults.has(type_id) {
        put_removal(patch, type_id)
    } else {
        remove_entry(patch, type_id)
    }
}

/// `ItemStackTemplate`: a non-empty item holder, a non-zero count and a patch
/// (`ItemStackTemplate.java:52-60`), with the prototype its stacks compare
/// against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStackTemplate {
    pub item_id: i32,
    pub count: i32,
    pub components: DataComponentPatchSummary,
    prototype_defaults: PrototypeDefaults,
}

impl ItemStackTemplate {
    /// `ItemStackTemplate.CODEC` over a registry source, whose holders resolve
    /// against `lookup`.
    pub fn from_source(
        value: RegistryElementValueRef<'_>,
        source: RegistryElementSource,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self> {
        let summary = item_stack_template_from_source_with_content_registry(
            value,
            source,
            lookup,
            lookup.content_registry(),
        )
        .map_err(|err| error(err.to_string()))?;
        Ok(Self {
            prototype_defaults: PrototypeDefaults::of_item(summary.item_id, lookup)?,
            item_id: summary.item_id,
            count: summary.count,
            components: summary.component_patch.into_inner(),
        })
    }

    /// The defaults every write onto a stack of this item compares against.
    pub fn prototype_defaults(&self) -> &PrototypeDefaults {
        &self.prototype_defaults
    }

    /// `ItemStackTemplate.create` (`ItemStackTemplate.java:74-76`).
    pub fn create(&self) -> ItemStackSummary {
        self.apply(self.count, &DataComponentPatchSummary::default())
            .expect("template components were validated at construction")
    }

    /// `ItemStackTemplate.apply` (`ItemStackTemplate.java:92-96`): the stack is
    /// built over `additional`, the template's own patch is applied on top, and
    /// `validate` (`:78-86`) has the last word. A count at or below zero stays a
    /// live empty stack carrying its item and patch.
    pub fn apply(
        &self,
        count: i32,
        additional: &DataComponentPatchSummary,
    ) -> Result<ItemStackSummary> {
        self.apply_with_content_registry(count, additional, ContentRegistry::vanilla())
    }

    /// Content-registry-aware form of [`Self::apply`].
    pub fn apply_with_content_registry(
        &self,
        count: i32,
        additional: &DataComponentPatchSummary,
        content_registry: &ContentRegistry,
    ) -> Result<ItemStackSummary> {
        let mut patch = patch_from(additional, &self.prototype_defaults)?;
        apply_patch(&mut patch, &self.components, &self.prototype_defaults)?;
        let stack = ItemStackSummary {
            item_id: Some(self.item_id),
            count,
            component_patch: patch.into(),
        };
        Ok(validate_strict_with_defaults(
            stack,
            Some(&self.prototype_defaults),
            content_registry,
        ))
    }
}

/// `ItemStack.validateStrict` (`ItemStack.java:159-168`) as
/// `ItemStackTemplate.validate` (`ItemStackTemplate.java:78-86`) applies it: an
/// invalid stack is replaced by the empty one. An empty stack reads the empty
/// component map (`ItemStack.java:220-222`) and count zero (`:1055-1057`), so it
/// always passes.
pub fn validate_strict(stack: ItemStackSummary) -> ItemStackSummary {
    validate_strict_with_content_registry(stack, ContentRegistry::vanilla())
}

/// Content-registry-aware form of [`validate_strict`].
pub fn validate_strict_with_content_registry(
    stack: ItemStackSummary,
    content_registry: &ContentRegistry,
) -> ItemStackSummary {
    validate_strict_with_defaults(stack, None, content_registry)
}

fn validate_strict_with_defaults(
    stack: ItemStackSummary,
    defaults: Option<&PrototypeDefaults>,
    content_registry: &ContentRegistry,
) -> ItemStackSummary {
    let Some(item_id) = stack.item_id.filter(|_| stack.count > 0) else {
        return stack;
    };
    let patch = &stack.component_patch;
    let stack_max = defaults
        .filter(|defaults| defaults.item_id == item_id)
        .map(|defaults| defaults.max_stack_size(patch))
        .unwrap_or_else(|| max_stack_size_with_content_registry(item_id, patch, content_registry));
    if components_are_valid(item_id, patch, stack_max, content_registry) && stack.count <= stack_max
    {
        return stack;
    }
    ItemStackSummary::empty()
}

/// `ItemStack.validateComponents` (`ItemStack.java:271-306`). Its container
/// arms read `nonEmptyItems`, `items` and `items` of kinds whose every 26.1
/// prototype default is the empty list, so the patch's own lists are the whole
/// population.
fn components_are_valid(
    item_id: i32,
    patch: &DataComponentPatchSummary,
    stack_max: i32,
    content_registry: &ContentRegistry,
) -> bool {
    if has_component(item_id, patch, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID) && stack_max > 1 {
        return false;
    }
    patch
        .container
        .items()
        .chain(&patch.bundle_contents_items)
        .chain(&patch.charged_projectiles_items)
        .all(|item| contained_item_fits(item, content_registry))
        && bundle_weight(&patch.bundle_contents_items, content_registry).is_some()
}

/// `ItemStack.validateContainedItemSizes` (`ItemStack.java:308-318`).
fn contained_item_fits(
    item: &ItemStackTemplateSummary,
    content_registry: &ContentRegistry,
) -> bool {
    item.count
        <= max_stack_size_with_content_registry(
            item.item_id,
            &item.component_patch,
            content_registry,
        )
}

/// `BundleContents.EMPTY` (`BundleContents.java:27`), which every one of the
/// seventeen bundles registers as its prototype default.
const EMPTY_BUNDLE_CONTENTS: &[ItemStackTemplateSummary] = &[];

/// `BundleContents.BUNDLE_IN_BUNDLE_WEIGHT` (`BundleContents.java:32`).
const BUNDLE_IN_BUNDLE_WEIGHT: Fraction = Fraction::of(1, 16);

/// `BundleContents.weight` (`BundleContents.java:103-105`) over its memoized
/// `computeContentWeight` (`:50-67`): what every carried stack adds up to, and
/// nothing where that arithmetic leaves `int`. A weight above one is not an
/// error -- only insertion bounds a bundle at one.
fn bundle_weight(
    items: &[ItemStackTemplateSummary],
    content_registry: &ContentRegistry,
) -> Option<Fraction> {
    let mut weight = Fraction::ZERO;
    for item in items {
        let carried = contained_item_weight(item, content_registry)?;
        weight = weight.add(carried.multiply_by(Fraction::get(item.count, 1)?)?)?;
    }
    Some(weight)
}

/// `BundleContents.getWeight` (`BundleContents.java:69-77`): a nested bundle
/// costs its own contents plus `BUNDLE_IN_BUNDLE_WEIGHT`, an occupied hive
/// costs `BEEHIVE_WEIGHT` (`:35`), and everything else costs one slot of its
/// own maximum.
fn contained_item_weight(
    item: &ItemStackTemplateSummary,
    content_registry: &ContentRegistry,
) -> Option<Fraction> {
    if let Some(nested) = carried_bundle_contents(item) {
        return bundle_weight(nested, content_registry)?.add(BUNDLE_IN_BUNDLE_WEIGHT);
    }
    if carries_bees(item) {
        return Some(Fraction::ONE);
    }
    Fraction::get(
        1,
        max_stack_size_with_content_registry(item.item_id, &item.component_patch, content_registry),
    )
}

/// `item.get(DataComponents.BUNDLE_CONTENTS)`: a bundle carrying nothing is
/// still a bundle, because its prototype registers the empty contents.
fn carried_bundle_contents(item: &ItemStackTemplateSummary) -> Option<&[ItemStackTemplateSummary]> {
    match patch_declares(
        &item.component_patch,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    ) {
        Some(true) => Some(&item.component_patch.bundle_contents_items),
        Some(false) => None,
        None => {
            prototype_has_component(item.item_id, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID)
                .then_some(EMPTY_BUNDLE_CONTENTS)
        }
    }
}

/// `item.getOrDefault(DataComponents.BEES, Bees.EMPTY).bees()`, whose prototype
/// default on the bee nest and the beehive is the empty list, so only a patch
/// entry ever puts a bee in one.
fn carries_bees(item: &ItemStackTemplateSummary) -> bool {
    patch_declares(&item.component_patch, VANILLA_DATA_COMPONENT_BEES_TYPE_ID) == Some(true)
        && !item.component_patch.bees.is_empty()
}

/// `PatchedDataComponentMap.get` (`PatchedDataComponentMap.java:61-63`): a patch
/// entry decides, an empty one hides the prototype default, and only an absent
/// key falls through to it.
pub fn patch_declares(patch: &DataComponentPatchSummary, type_id: i32) -> Option<bool> {
    if patch.removed_type_ids.contains(&type_id) {
        return Some(false);
    }
    patch.added_type_ids.contains(&type_id).then_some(true)
}

/// `ItemInstance.getMaxStackSize` (`ItemInstance.java:14-16`):
/// `getOrDefault(DataComponents.MAX_STACK_SIZE, 1)`.
pub fn max_stack_size(item_id: i32, patch: &DataComponentPatchSummary) -> i32 {
    max_stack_size_with_content_registry(item_id, patch, ContentRegistry::vanilla())
}

/// Content-registry-aware form of [`max_stack_size`].
pub fn max_stack_size_with_content_registry(
    item_id: i32,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> i32 {
    match patch_declares(patch, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID) {
        Some(false) => 1,
        Some(true) => patch.max_stack_size.unwrap_or(1),
        None => prototype_component_value(item_id, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID)
            .and_then(codec_int)
            .or_else(|| {
                content_registry
                    .item_by_id(item_id)
                    .map(|spec| i32::from(spec.max_stack_size))
            })
            .unwrap_or(1),
    }
}

/// `Codec.INT` over `JsonOps.getNumberValue`, which reads any JSON number and
/// narrows it with `Number.intValue`.
pub fn codec_int(value: &Value) -> Option<i32> {
    let number = value.as_number()?;
    number
        .as_i64()
        .map(|number| number as i32)
        .or_else(|| number.as_f64().map(|number| number as i32))
}

/// `DataComponentMap.has` (`DataComponentMap.java:86-88`) over the same patch
/// and prototype order.
pub fn has_component(item_id: i32, patch: &DataComponentPatchSummary, type_id: i32) -> bool {
    patch_declares(patch, type_id).unwrap_or_else(|| prototype_has_component(item_id, type_id))
}

/// `DataComponentMap.get` on the item's registered prototype
/// (`Item.components`), which decides whether a patch entry has a default to
/// override or to remove.
pub fn prototype_has_component(item_id: i32, type_id: i32) -> bool {
    prototype_component_value(item_id, type_id).is_some()
}

/// The registered prototype value itself, which `PatchedDataComponentMap.get`
/// (`PatchedDataComponentMap.java:61-63`) reads through an absent patch entry.
pub fn prototype_component_value(item_id: i32, type_id: i32) -> Option<&'static Value> {
    bbb_protocol::vanilla_item_prototype_component_catalog()
        .ok()
        .and_then(|catalog| catalog.component_codec_values(item_id))
        .and_then(|components| components.get(&type_id))
}

#[cfg(test)]
mod tests;
