use std::{collections::BTreeMap, sync::OnceLock};

use serde_json::Value;

use crate::item_types::VANILLA_ITEM_TYPES;

#[path = "item_prototype_components/generated_26_1.rs"]
mod generated;

const EXPECTED_ITEM_COUNT: usize = 1_506;

#[derive(Clone, Copy)]
pub(super) enum StaticProtoNode {
    Bool(bool),
    UInt(u32),
    Float(u16),
    Str(u16),
    Array { offset: u16, len: u16 },
    Object { offset: u16, len: u16 },
}

static VANILLA_ITEM_PROTOTYPE_COMPONENT_CATALOG: VanillaItemPrototypeComponentCatalog =
    VanillaItemPrototypeComponentCatalog::new();

/// Immutable Vanilla item prototype component values, indexed by numeric item id.
#[derive(Debug)]
pub struct VanillaItemPrototypeComponentCatalog {
    item_resource_ids: OnceLock<Box<[String]>>,
    per_item_component_codec_values: [OnceLock<BTreeMap<i32, Value>>; EXPECTED_ITEM_COUNT],
    component_codec_values_by_item_id: OnceLock<Box<[BTreeMap<i32, Value>]>>,
}

impl VanillaItemPrototypeComponentCatalog {
    const fn new() -> Self {
        Self {
            item_resource_ids: OnceLock::new(),
            per_item_component_codec_values: [const { OnceLock::new() }; EXPECTED_ITEM_COUNT],
            component_codec_values_by_item_id: OnceLock::new(),
        }
    }

    pub fn item_resource_id(&self, item_type_id: i32) -> Option<&'static str> {
        let index = usize::try_from(item_type_id).ok()?;
        VANILLA_ITEM_TYPES.get(index).map(|entry| entry.resource_id)
    }

    pub fn item_resource_ids(&self) -> &[String] {
        self.item_resource_ids.get_or_init(|| {
            debug_assert_eq!(VANILLA_ITEM_TYPES.len(), EXPECTED_ITEM_COUNT);
            VANILLA_ITEM_TYPES
                .iter()
                .map(|entry| entry.resource_id.to_owned())
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
    }

    pub fn component_codec_values_by_item_id(&self) -> &[BTreeMap<i32, Value>] {
        self.component_codec_values_by_item_id
            .get_or_init(materialize_component_codec_values)
    }

    pub fn component_codec_values(&self, item_type_id: i32) -> Option<&BTreeMap<i32, Value>> {
        let index = usize::try_from(item_type_id).ok()?;
        if index >= EXPECTED_ITEM_COUNT {
            return None;
        }
        if let Some(all) = self.component_codec_values_by_item_id.get() {
            return all.get(index);
        }
        Some(
            self.per_item_component_codec_values[index]
                .get_or_init(|| materialize_item_component_codec_values(index)),
        )
    }

    /// Zero-allocation static membership check on the generated prototype table.
    pub fn has_component(&self, item_type_id: i32, component_type_id: i32) -> bool {
        static_component_node(item_type_id, component_type_id).is_some()
    }

    /// Zero-allocation integer component lookup on the generated prototype table.
    pub fn int_component(&self, item_type_id: i32, component_type_id: i32) -> Option<i64> {
        match static_component_node(item_type_id, component_type_id)? {
            StaticProtoNode::UInt(value) => Some(i64::from(value)),
            _ => None,
        }
    }

    /// Zero-allocation string component lookup on the generated prototype table.
    pub fn str_component(&self, item_type_id: i32, component_type_id: i32) -> Option<&'static str> {
        match static_component_node(item_type_id, component_type_id)? {
            StaticProtoNode::Str(str_index) => Some(generated::str_at(str_index)),
            _ => None,
        }
    }

    /// Zero-allocation boolean component lookup on the generated prototype table.
    pub fn bool_component(&self, item_type_id: i32, component_type_id: i32) -> Option<bool> {
        match static_component_node(item_type_id, component_type_id)? {
            StaticProtoNode::Bool(flag) => Some(flag),
            _ => None,
        }
    }
}

fn eval_proto_node(node_index: usize) -> Value {
    match generated::node_at(node_index) {
        StaticProtoNode::Bool(flag) => Value::Bool(flag),
        StaticProtoNode::UInt(number) => Value::Number(serde_json::Number::from(number)),
        StaticProtoNode::Float(float_index) => {
            let value = f64::from_bits(generated::float_bits_at(float_index));
            Value::Number(
                serde_json::Number::from_f64(value)
                    .expect("generated item prototype float is finite"),
            )
        }
        StaticProtoNode::Str(str_index) => Value::String(generated::str_at(str_index).to_owned()),
        StaticProtoNode::Array { offset, len } => {
            let start = usize::from(offset);
            let end = start + usize::from(len);
            Value::Array(
                (start..end)
                    .map(|pos| eval_proto_node(usize::from(generated::array_item_at(pos))))
                    .collect(),
            )
        }
        StaticProtoNode::Object { offset, len } => {
            let start = usize::from(offset);
            let end = start + usize::from(len);
            let mut map = serde_json::Map::with_capacity(usize::from(len));
            for pos in start..end {
                let (key, child) = generated::object_field_at(pos);
                map.insert(
                    generated::str_at(key).to_owned(),
                    eval_proto_node(usize::from(child)),
                );
            }
            Value::Object(map)
        }
    }
}

fn materialize_item_component_codec_values(item_index: usize) -> BTreeMap<i32, Value> {
    let start = generated::item_offset_at(item_index);
    let end = generated::item_offset_at(item_index + 1);
    let mut map = BTreeMap::new();
    for pos in start..end {
        let (type_id, node_index) = generated::item_component_at(pos);
        map.insert(i32::from(type_id), eval_proto_node(usize::from(node_index)));
    }
    map
}

fn static_component_node(item_type_id: i32, component_type_id: i32) -> Option<StaticProtoNode> {
    let item_index = usize::try_from(item_type_id).ok()?;
    let target_type = u8::try_from(component_type_id).ok()?;
    if item_index >= EXPECTED_ITEM_COUNT {
        return None;
    }
    let mut lo = generated::item_offset_at(item_index);
    let mut hi = generated::item_offset_at(item_index + 1);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (entry_type, node_index) = generated::item_component_at(mid);
        if entry_type < target_type {
            lo = mid + 1;
        } else if entry_type > target_type {
            hi = mid;
        } else {
            return Some(generated::node_at(usize::from(node_index)));
        }
    }
    None
}

fn materialize_component_codec_values() -> Box<[BTreeMap<i32, Value>]> {
    let mut evaluated: Vec<Value> = Vec::with_capacity(generated::NODE_COUNT);
    for index in 0..generated::NODE_COUNT {
        let value = match generated::node_at(index) {
            StaticProtoNode::Bool(flag) => Value::Bool(flag),
            StaticProtoNode::UInt(number) => Value::Number(serde_json::Number::from(number)),
            StaticProtoNode::Float(float_index) => {
                let value = f64::from_bits(generated::float_bits_at(float_index));
                Value::Number(
                    serde_json::Number::from_f64(value)
                        .expect("generated item prototype float is finite"),
                )
            }
            StaticProtoNode::Str(str_index) => {
                Value::String(generated::str_at(str_index).to_owned())
            }
            StaticProtoNode::Array { offset, len } => {
                let start = usize::from(offset);
                let end = start + usize::from(len);
                Value::Array(
                    (start..end)
                        .map(|pos| evaluated[usize::from(generated::array_item_at(pos))].clone())
                        .collect(),
                )
            }
            StaticProtoNode::Object { offset, len } => {
                let start = usize::from(offset);
                let end = start + usize::from(len);
                let mut map = serde_json::Map::with_capacity(usize::from(len));
                for pos in start..end {
                    let (key, child) = generated::object_field_at(pos);
                    map.insert(
                        generated::str_at(key).to_owned(),
                        evaluated[usize::from(child)].clone(),
                    );
                }
                Value::Object(map)
            }
        };
        evaluated.push(value);
    }

    let mut by_item = Vec::with_capacity(EXPECTED_ITEM_COUNT);
    for item_index in 0..EXPECTED_ITEM_COUNT {
        let start = generated::item_offset_at(item_index);
        let end = generated::item_offset_at(item_index + 1);
        let mut map = BTreeMap::new();
        for pos in start..end {
            let (type_id, node_index) = generated::item_component_at(pos);
            map.insert(
                i32::from(type_id),
                evaluated[usize::from(node_index)].clone(),
            );
        }
        by_item.push(map);
    }
    by_item.into_boxed_slice()
}

/// Retained error type for downstream signature compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPrototypeComponentCatalogError(String);

impl std::fmt::Display for ItemPrototypeComponentCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid item prototype component catalog: {}", self.0)
    }
}

impl std::error::Error for ItemPrototypeComponentCatalogError {}

/// Returns the static Vanilla 26.1 item prototype component catalog.
#[inline]
pub fn vanilla_item_prototype_component_catalog(
) -> Result<&'static VanillaItemPrototypeComponentCatalog, ItemPrototypeComponentCatalogError> {
    Ok(&VANILLA_ITEM_PROTOTYPE_COMPONENT_CATALOG)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::{json, Value};

    use super::*;
    use crate::{data_component_types::VANILLA_DATA_COMPONENT_TYPES, MC_VERSION};

    const EXPECTED_SOURCE: &str = "BuiltInRegistries.DATA_COMPONENT_INITIALIZERS with VanillaRegistries.createLookup and each persistent component codec";
    const EXPECTED_COMPONENT_COUNT: usize = 59;
    const EXPECTED_MEMBERSHIP_COUNT: usize = 18_981;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Artifact {
        component_values: Vec<ArtifactComponent>,
        item_resource_ids: Vec<String>,
        source: String,
        vanilla_version: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArtifactComponent {
        resource_id: String,
        type_id: i32,
        values: Vec<ArtifactComponentValue>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArtifactComponentValue {
        item_id_ranges: Vec<[i32; 2]>,
        value: Value,
    }

    fn fixture_json() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/item_prototype_components_26_1.json");
        std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
    }

    #[test]
    fn static_table_matches_json_fixture_across_all_items_and_components() {
        let artifact: Artifact = serde_json::from_str(&fixture_json()).unwrap();
        assert_eq!(artifact.vanilla_version, MC_VERSION);
        assert_eq!(artifact.source, EXPECTED_SOURCE);
        assert_eq!(artifact.item_resource_ids.len(), EXPECTED_ITEM_COUNT);
        assert_eq!(artifact.component_values.len(), EXPECTED_COMPONENT_COUNT);

        let mut expected_by_item = vec![BTreeMap::new(); EXPECTED_ITEM_COUNT];
        let mut membership_count = 0usize;
        for component in &artifact.component_values {
            let entry = &VANILLA_DATA_COMPONENT_TYPES[component.type_id as usize];
            assert_eq!(entry.resource_id, component.resource_id);
            assert!(entry.persistent);
            for group in &component.values {
                for &[start, end] in &group.item_id_ranges {
                    for item_id in start..=end {
                        assert!(expected_by_item[item_id as usize]
                            .insert(component.type_id, group.value.clone())
                            .is_none());
                        membership_count += 1;
                    }
                }
            }
        }
        assert_eq!(membership_count, EXPECTED_MEMBERSHIP_COUNT);

        let catalog = vanilla_item_prototype_component_catalog().unwrap();
        assert_eq!(
            catalog.item_resource_ids(),
            artifact.item_resource_ids.as_slice()
        );
        assert_eq!(
            catalog.component_codec_values_by_item_id(),
            expected_by_item.as_slice()
        );
    }

    #[test]
    fn catalog_has_expected_identities_counts_and_representative_values() {
        let catalog = vanilla_item_prototype_component_catalog().unwrap();

        assert_eq!(catalog.item_resource_ids().len(), EXPECTED_ITEM_COUNT);
        assert_eq!(
            catalog.component_codec_values_by_item_id().len(),
            EXPECTED_ITEM_COUNT
        );
        assert_eq!(catalog.item_resource_ids()[0], "minecraft:air");
        assert_eq!(
            catalog.item_resource_ids()[942],
            "minecraft:netherite_sword"
        );
        assert_eq!(
            catalog
                .component_codec_values_by_item_id()
                .iter()
                .map(BTreeMap::len)
                .sum::<usize>(),
            EXPECTED_MEMBERSHIP_COUNT
        );

        let air = catalog.component_codec_values(0).unwrap();
        assert_eq!(air.get(&1), Some(&json!(64)));
        assert_eq!(catalog.int_component(0, 1), Some(64));
        assert!(catalog.has_component(0, 9));
        assert!(!catalog.has_component(0, 2));
        assert_eq!(
            air.get(&9),
            Some(&json!({ "translate": "block.minecraft.air" }))
        );
        assert_eq!(air.get(&80), Some(&json!("minecraft:entity.item.break")));

        let netherite_sword = catalog.component_codec_values(942).unwrap();
        assert_eq!(netherite_sword.get(&1), Some(&json!(1)));
        assert_eq!(netherite_sword.get(&2), Some(&json!(2031)));
        assert_eq!(catalog.int_component(942, 2), Some(2031));
        assert_eq!(
            netherite_sword.get(&27),
            Some(&json!({ "types": "#minecraft:is_fire" }))
        );

        assert!(catalog.component_codec_values(-1).is_none());
        assert!(catalog
            .component_codec_values(EXPECTED_ITEM_COUNT as i32)
            .is_none());
    }

    #[test]
    fn public_loader_is_stable_and_read_only() {
        let first = vanilla_item_prototype_component_catalog().unwrap();
        let second = vanilla_item_prototype_component_catalog().unwrap();
        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn single_item_lookup_is_lazy_and_matches_full_table() {
        let fresh = VanillaItemPrototypeComponentCatalog::new();
        assert_eq!(fresh.item_resource_id(0), Some("minecraft:air"));
        assert_eq!(
            fresh.item_resource_id(942),
            Some("minecraft:netherite_sword")
        );
        assert_eq!(fresh.item_resource_id(-1), None);
        assert!(fresh.item_resource_ids.get().is_none());

        let sword = fresh.component_codec_values(942).unwrap();
        assert!(fresh.component_codec_values_by_item_id.get().is_none());
        assert_eq!(
            fresh.str_component(0, 80),
            Some("minecraft:entity.item.break")
        );
        assert_eq!(sword, &fresh.component_codec_values_by_item_id()[942]);
    }
}
