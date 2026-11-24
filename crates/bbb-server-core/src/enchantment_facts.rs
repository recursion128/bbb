use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::OnceLock;

use bbb_protocol::equipment_slots::EquipmentSlot;

use crate::equipment_drop_effects::EquipmentDropEffects;
use crate::item_durability::{
    equipment_slot_group_matches, EnchantmentValueEffects, ValueEffectGeneration,
};
use bbb_registry_codec::level_based_value::LevelBasedValue;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot;

pub const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

/// One `effects` member by component id; the codec reads keys as resource
/// locations, so `item_damage` names `minecraft:item_damage`.
pub(crate) fn effect_component<'a>(
    document: &'a bbb_registry_codec::RegistryJsonValue,
    component: &str,
) -> Option<&'a bbb_registry_codec::RegistryJsonValue> {
    document
        .get("effects")?
        .as_object()?
        .iter()
        .find_map(|(name, value)| {
            bbb_protocol::normalize_resource_location(name.try_to_string().unwrap_or_default())
                .is_ok_and(|id| id == component)
                .then_some(value)
        })
}

/// The `minecraft:enchantment` registry of the built-in datapack alone, in the
/// order `RegistryDataLoader` hands its ids out.
///
/// This is what a world the runtime wired no datapack generation into reads. It
/// goes to the same embedded built-in pack `DatapackStartup::load` starts from
/// (`VanillaDatapackSnapshot::embedded_shared`) rather than to a table written here,
/// and lists it the way `RegistryDataLoader` lists a registry directory: by
/// path, which is the order the ids follow.
#[must_use]
pub fn builtin_enchantment_registry() -> &'static [String] {
    static BUILTIN: OnceLock<Vec<String>> = OnceLock::new();
    BUILTIN.get_or_init(|| {
        let snapshot = VanillaDatapackSnapshot::embedded_shared()
            .expect("the embedded built-in datapack decodes");
        let mut paths: Vec<&str> = snapshot
            .paths()
            .filter(|path| path.starts_with("enchantment/") && path.ends_with(".json"))
            .collect();
        paths.sort_unstable();
        paths
            .into_iter()
            .map(|path| {
                let name = path
                    .trim_start_matches("enchantment/")
                    .trim_end_matches(".json");
                format!("minecraft:{name}")
            })
            .collect()
    })
}

/// The parsed `enchantment/<name>.json` of the built-in datapack, in the same
/// order [`builtin_enchantment_registry`] lists their ids.
///
/// It is the fallback the same way that function is: a world whose runtime
/// wired no datapack generation still has to answer what an enchantment does,
/// and the embedded pack is what `RegistryDataLoader` would have read.
#[must_use]
pub fn builtin_enchantment_documents() -> &'static [bbb_registry_codec::RegistryJsonValue] {
    static BUILTIN: OnceLock<Vec<bbb_registry_codec::RegistryJsonValue>> = OnceLock::new();
    BUILTIN.get_or_init(|| {
        let snapshot = VanillaDatapackSnapshot::embedded_shared()
            .expect("the embedded built-in datapack decodes");
        let mut paths: Vec<&str> = snapshot
            .paths()
            .filter(|path| path.starts_with("enchantment/") && path.ends_with(".json"))
            .collect();
        paths.sort_unstable();
        paths
            .into_iter()
            .map(|path| {
                let bytes = snapshot
                    .read_path(path)
                    .expect("a path the snapshot listed is one it holds");
                bbb_registry_codec::parse_registry_json(bytes)
                    .expect("the embedded built-in datapack holds valid JSON")
            })
            .collect()
    })
}
/// `EnchantmentEffectComponents.PREVENT_EQUIPMENT_DROP`
/// (`EnchantmentEffectComponents.java:115`).
pub const PREVENT_EQUIPMENT_DROP_EFFECT: &str = "minecraft:prevent_equipment_drop";

/// `EnchantmentEffectComponents.PREVENT_ARMOR_CHANGE`, which
/// `Mob.compareArmor` reads (`Mob.java:581-582`).
pub const PREVENT_ARMOR_CHANGE_EFFECT: &str = "minecraft:prevent_armor_change";

pub const EQUIPMENT_DROPS_EFFECT: &str = "minecraft:equipment_drops";

/// Parsed equipment-drop effects in this generation's enchantment registry order.
pub fn load_equipment_drop_effects(
    datapacks: Option<&DatapackStartup>,
) -> Arc<Vec<Option<EquipmentDropEffects>>> {
    let lookup = datapacks.map(FrozenDatapackRegistryLookup::from_startup);
    let parse = |id: &str, document: &bbb_registry_codec::RegistryJsonValue| {
        match EquipmentDropEffects::parse(document, lookup.as_ref()) {
            Ok(effects) => effects,
            Err(detail) => Some(EquipmentDropEffects::unmodelled(format!(
                "enchantment {id} declares {EQUIPMENT_DROPS_EFFECT} as {detail}"
            ))),
        }
    };
    Arc::new(
        datapacks
            .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
            .map(|registry| {
                registry
                    .elements
                    .iter()
                    .map(|element| {
                        parse(
                            &element.id,
                            element.json().expect("enchantment legacy JSON is required"),
                        )
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                builtin_enchantment_registry()
                    .iter()
                    .zip(builtin_enchantment_documents())
                    .map(|(id, document)| parse(id, document))
                    .collect()
            }),
    )
}

/// One `EnchantmentEffectComponents` value-effect list for every element of the
/// `minecraft:enchantment` registry, in the same order
/// [`load_enchantment_registry`] lists their ids.
pub fn load_enchantment_value_effects(
    datapacks: Option<&Arc<DatapackStartup>>,
    component: &str,
) -> Arc<Vec<Option<EnchantmentValueEffects>>> {
    let generation = ValueEffectGeneration::new(datapacks);
    let datapacks = datapacks.map(Arc::as_ref);
    let parse = |id: &str, document: &bbb_registry_codec::RegistryJsonValue| {
        match EnchantmentValueEffects::parse_in(&generation, id, document, component) {
            Ok(effects) => effects,
            Err(detail) => Some(EnchantmentValueEffects::unmodelled(format!(
                "enchantment {id} declares {component} as {detail}"
            ))),
        }
    };
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .map(|element| {
                    parse(
                        &element.id,
                        element.json().expect("enchantment legacy JSON is required"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            builtin_enchantment_registry()
                .iter()
                .zip(builtin_enchantment_documents())
                .map(|(id, document)| parse(id, document))
                .collect()
        });
    Arc::new(loaded)
}

/// The `minecraft:enchantment` registry in `RegistryDataLoader` order
/// (`RegistryDataLoader.java:83-129`).
pub fn load_enchantment_registry(datapacks: Option<&DatapackStartup>) -> Arc<Vec<String>> {
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .map(|element| element.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| builtin_enchantment_registry().to_vec());
    Arc::new(loaded)
}

/// The `Enchantment.EnchantmentDefinition` fields the anvil and grindstone
/// read (`Enchantment.java:671-680`), with the holder sets resolved against the
/// generation's tags.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnchantmentDefinition {
    pub max_level: i32,
    pub anvil_cost: i32,
    min_cost_base: i32,
    min_cost_per_level: i32,
    /// Item ids of `supported_items`, which `Enchantment.canEnchant` tests.
    pub supported_items: BTreeSet<i32>,
    /// Holder ids of `exclusive_set`.
    pub exclusive_set: BTreeSet<usize>,
    /// Membership in `#minecraft:curse` (`EnchantmentTags.CURSE`).
    pub curse: bool,
}

impl EnchantmentDefinition {
    /// `Enchantment.getMinCost` (`Enchantment.java:154-156`).
    #[must_use]
    pub fn min_cost(&self, level: i32) -> i32 {
        self.min_cost_base
            .wrapping_add(self.min_cost_per_level.wrapping_mul(level - 1))
    }
}

/// `Enchantment.areCompatible` (`Enchantment.java:167-169`) over holder ids.
#[must_use]
pub fn enchantments_compatible(definitions: &[EnchantmentDefinition], a: usize, b: usize) -> bool {
    a != b
        && !definitions
            .get(a)
            .is_some_and(|definition| definition.exclusive_set.contains(&b))
        && !definitions
            .get(b)
            .is_some_and(|definition| definition.exclusive_set.contains(&a))
}

/// Every enchantment's [`EnchantmentDefinition`], by holder id.
pub fn load_enchantment_definitions(
    datapacks: Option<&DatapackStartup>,
    tags: &crate::registry_tags::RegistryTagSnapshot,
) -> Arc<Vec<EnchantmentDefinition>> {
    let ids = load_enchantment_registry(datapacks);
    let members =
        |registry: &str, value: Option<&bbb_registry_codec::RegistryJsonValue>| -> Vec<String> {
            let mut out = Vec::new();
            let mut push = |raw: &str| match raw.strip_prefix('#') {
                Some(tag) => {
                    if let Ok(tag) = bbb_protocol::normalize_resource_location(tag.to_string()) {
                        out.extend(tags.members(registry, &tag).iter().map(|id| id.to_string()));
                    }
                }
                None => out.extend(bbb_protocol::normalize_resource_location(raw.to_string())),
            };
            match value {
                Some(bbb_registry_codec::RegistryJsonValue::String(raw)) => {
                    raw.with_utf8(&mut push);
                }
                Some(bbb_registry_codec::RegistryJsonValue::Array(values)) => {
                    values
                        .iter()
                        .filter_map(|value| value.utf8_string())
                        .for_each(|value| push(&value));
                }
                _ => {}
            }
            out
        };
    let int = |document: &bbb_registry_codec::RegistryJsonValue, path: &[&str]| {
        path.iter()
            .try_fold(document, |value, key| value.get(key))
            .and_then(bbb_registry_codec::RegistryJsonValue::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(0)
    };
    let parse =
        |id: &str, document: &bbb_registry_codec::RegistryJsonValue| EnchantmentDefinition {
            max_level: int(document, &["max_level"]),
            anvil_cost: int(document, &["anvil_cost"]),
            min_cost_base: int(document, &["min_cost", "base"]),
            min_cost_per_level: int(document, &["min_cost", "per_level_above_first"]),
            supported_items: members("minecraft:item", document.get("supported_items"))
                .iter()
                .filter_map(|item| {
                    bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
                })
                .map(|item| item.id)
                .collect(),
            exclusive_set: members(ENCHANTMENT_REGISTRY, document.get("exclusive_set"))
                .iter()
                .filter_map(|other| ids.iter().position(|id| id == other))
                .collect(),
            curse: tags.contains(ENCHANTMENT_REGISTRY, "minecraft:curse", id),
        };
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .map(|element| {
                    parse(
                        &element.id,
                        element.json().expect("enchantment legacy JSON is required"),
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| {
            builtin_enchantment_registry()
                .iter()
                .zip(builtin_enchantment_documents())
                .map(|(id, document)| parse(id, document))
                .collect()
        });
    Arc::new(loaded)
}

/// One enchantment's declared equipment slots, for
/// `EnchantmentHelper.getEnchantmentLevel` (`EnchantmentHelper.java:284-296`),
/// whose `Enchantment.getSlotItems` (`Enchantment.java:107-119`) walks exactly
/// the slots the enchantment's `slots` list names
/// (`Enchantment.java:679-690`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantmentSlots {
    pub resource_id: String,
    pub slots: Vec<EquipmentSlot>,
}

/// The declared slots of every element of the `minecraft:enchantment`
/// registry, in [`load_enchantment_registry`] order.
///
/// The one loot read that needs it is a level: `EnchantmentHelper.getEnchantmentLevel`
/// is a maximum over the stacks an enchantment declares, not over every slot
/// the entity has, so which slots those are belongs to the same generation the
/// enchantment itself came from.
pub fn load_enchantment_slots(datapacks: Option<&DatapackStartup>) -> Arc<Vec<EnchantmentSlots>> {
    let parse = |id: &str, document: &bbb_registry_codec::RegistryJsonValue| EnchantmentSlots {
        resource_id: id.to_string(),
        slots: declared_slots(document),
    };
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .map(|element| {
                    parse(
                        &element.id,
                        element.json().expect("enchantment legacy JSON is required"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            builtin_enchantment_registry()
                .iter()
                .zip(builtin_enchantment_documents())
                .map(|(id, document)| parse(id, document))
                .collect()
        });
    Arc::new(loaded)
}

/// `EquipmentSlotGroup` (`EquipmentSlotGroup.java:14-25`, `:64-70`) expanded to
/// the `EquipmentSlot`s one `slots` entry names, in `EquipmentSlot.VALUES`
/// order (`EquipmentSlot.java:23`).
fn declared_slots(document: &bbb_registry_codec::RegistryJsonValue) -> Vec<EquipmentSlot> {
    let Some(groups) = document
        .get("slots")
        .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
    else {
        return Vec::new();
    };
    let groups: Vec<String> = groups
        .iter()
        .filter_map(bbb_registry_codec::RegistryJsonValue::utf8_string)
        .collect();
    bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS
        .iter()
        .map(|entry| entry.slot)
        .filter(|slot| {
            groups
                .iter()
                .any(|group| equipment_slot_group_matches(group, *slot))
        })
        .collect()
}

/// `EnchantmentAttributeEffect` (`EnchantmentAttributeEffect.java:16-26`): one
/// entry of an enchantment's `minecraft:attributes` component.
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentAttributeEffect {
    pub id: String,
    pub attribute_id: i32,
    pub amount: LevelBasedValue,
    /// `AttributeModifier.Operation`'s serialized name.
    pub operation: String,
}

impl EnchantmentAttributeEffect {
    /// `idForSlot` (`EnchantmentAttributeEffect.java:31-33`).
    #[must_use]
    pub fn modifier_id(&self, slot: EquipmentSlot) -> String {
        let suffix = bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS
            .iter()
            .find(|entry| entry.slot == slot)
            .map_or("", |entry| entry.serialized_name);
        format!("{}/{suffix}", self.id)
    }
}

/// One enchantment's `minecraft:attributes` effects and the `slots` groups
/// `Enchantment.matchingSlot` tests (`Enchantment.java:126-128`).
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentAttributeEffects {
    slots: Vec<String>,
    pub effects: Vec<EnchantmentAttributeEffect>,
}

impl EnchantmentAttributeEffects {
    #[must_use]
    pub fn matching_slot(&self, slot: EquipmentSlot) -> bool {
        self.slots
            .iter()
            .any(|group| equipment_slot_group_matches(group, slot))
    }

    fn parse(document: &bbb_registry_codec::RegistryJsonValue) -> Result<Option<Self>, String> {
        let Some(list) = effect_component(document, "minecraft:attributes") else {
            return Ok(None);
        };
        let effects = list
            .as_array()
            .ok_or_else(|| "attributes is not a list".to_string())?
            .iter()
            .map(parse_attribute_effect)
            .collect::<Result<Vec<_>, String>>()?;
        let slots = document
            .get("slots")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
            .map(|groups| {
                groups
                    .iter()
                    .filter_map(bbb_registry_codec::RegistryJsonValue::utf8_string)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Some(Self { slots, effects }))
    }
}

/// `EnchantmentAttributeEffect.MAP_CODEC` (`EnchantmentAttributeEffect.java:16-26`).
pub(crate) fn parse_attribute_effect(
    effect: &bbb_registry_codec::RegistryJsonValue,
) -> Result<EnchantmentAttributeEffect, String> {
    let text = |name: &str| {
        effect
            .get(name)
            .and_then(bbb_registry_codec::RegistryJsonValue::utf8_string)
            .ok_or_else(|| format!("an attribute effect has no {name}"))
    };
    let normalized = |name: &str| {
        text(name).and_then(|value| {
            bbb_protocol::normalize_resource_location(value).map_err(|error| error.to_string())
        })
    };
    let attribute = normalized("attribute")?;
    Ok(EnchantmentAttributeEffect {
        id: normalized("id")?,
        attribute_id: bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id(
            &attribute,
        )
        .ok_or_else(|| format!("unknown attribute {attribute}"))?
        .id,
        amount: LevelBasedValue::parse(
            effect
                .get("amount")
                .ok_or_else(|| "an attribute effect has no amount".to_string())?,
        )?,
        operation: text("operation")?,
    })
}

/// Every element of the `minecraft:enchantment` registry that declares
/// `minecraft:attributes`, by id. `Err` is a declaration this server cannot read.
pub type EnchantmentAttributeTable = BTreeMap<String, Result<EnchantmentAttributeEffects, String>>;

pub fn load_enchantment_attribute_effects(
    datapacks: Option<&DatapackStartup>,
) -> Arc<EnchantmentAttributeTable> {
    let parse = |id: &str, document: &bbb_registry_codec::RegistryJsonValue| {
        EnchantmentAttributeEffects::parse(document)
            .map_err(|detail| format!("enchantment {id} declares minecraft:attributes as {detail}"))
            .transpose()
            .map(|effects| (id.to_string(), effects))
    };
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .filter_map(|element| {
                    parse(
                        &element.id,
                        element.json().expect("enchantment legacy JSON is required"),
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| {
            builtin_enchantment_registry()
                .iter()
                .zip(builtin_enchantment_documents())
                .filter_map(|(id, document)| parse(id, document))
                .collect()
        });
    Arc::new(loaded)
}

/// `EnchantmentHelper.has` checks active item enchantments for the effect
/// (`EnchantmentHelper.java:126-132`, `EnchantmentHelper.java:448-455`).
pub fn load_enchantments_with_effect(
    datapacks: Option<&DatapackStartup>,
    effect_id: &str,
) -> Arc<BTreeSet<String>> {
    let Ok(effect_id) = bbb_protocol::normalize_resource_location(effect_id.to_string()) else {
        return Arc::new(BTreeSet::new());
    };
    fn collect<'a>(
        elements: impl Iterator<Item = (&'a str, &'a bbb_registry_codec::RegistryJsonValue)>,
        effect_id: &str,
    ) -> BTreeSet<String> {
        elements
            .filter(|&(_id, document)| {
                document
                    .get("effects")
                    .and_then(bbb_registry_codec::RegistryJsonValue::as_object)
                    .is_some_and(|effects| {
                        effects.iter().any(|(key, _)| {
                            bbb_protocol::normalize_resource_location(
                                key.try_to_string().unwrap_or_default(),
                            )
                            .is_ok_and(|key| key == effect_id)
                        })
                    })
            })
            .map(|(id, _document)| id.to_string())
            .collect()
    }
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            collect(
                registry.elements.iter().map(|element| {
                    (
                        element.id.as_str(),
                        element.json().expect("enchantment legacy JSON is required"),
                    )
                }),
                &effect_id,
            )
        })
        .unwrap_or_else(|| {
            collect(
                builtin_enchantment_registry()
                    .iter()
                    .map(String::as_str)
                    .zip(builtin_enchantment_documents()),
                &effect_id,
            )
        });
    Arc::new(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    #[test]
    fn active_enchantment_effect_set_replaces_builtin_and_normalizes_shorthand() {
        let builtin = load_enchantments_with_effect(None, PREVENT_EQUIPMENT_DROP_EFFECT);
        let removed = builtin
            .iter()
            .next()
            .expect("the built-in pack declares the effect")
            .clone();
        let mut startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .expect("the embedded pack loads");
        let registry = startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == ENCHANTMENT_REGISTRY)
            .expect("the active generation owns enchantments");
        let replacement = registry
            .elements
            .iter()
            .find(|element| {
                element.id != removed
                    && element
                        .json()
                        .expect("enchantment legacy JSON is required")
                        .get("effects")
                        .is_some_and(bbb_registry_codec::RegistryJsonValue::is_object)
            })
            .expect("there is another enchantment")
            .id
            .clone();
        for element in &mut registry.elements {
            let mut json: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
            let Some(effects) = json
                .get_mut("effects")
                .and_then(serde_json::Value::as_object_mut)
            else {
                continue;
            };
            effects.remove(PREVENT_EQUIPMENT_DROP_EFFECT);
            effects.remove("prevent_equipment_drop");
            if element.id == replacement {
                effects.insert("prevent_equipment_drop".to_string(), serde_json::json!({}));
            }
            element.source_json = serde_json::to_vec(&json).unwrap().into();
            element.source = Some(
                bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap(),
            );
        }
        let active = load_enchantments_with_effect(Some(&startup), PREVENT_EQUIPMENT_DROP_EFFECT);
        assert!(!active.contains(&removed));
        assert!(active.contains(&replacement));
    }
}
