//! Item-model `minecraft:component` predicates, decoded once at pack load.
//!
//! Only the shapes the matcher understands decode; anything else is `None`,
//! which the owning condition treats as an unsupported property.

use std::collections::BTreeMap;

use bbb_protocol::attribute_modifier_types::{
    attribute_modifier_display_id, attribute_modifier_operation_id,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
};
use bbb_protocol::data_component_types::{
    self as component_id, vanilla_data_component_type_for_resource_id,
};
use bbb_protocol::equipment_slots::vanilla_equipment_slot_group_for_serialized_name;
use bbb_protocol::packets::{
    FireworkExplosionShapeSummary, FireworkExplosionSummary, LodestoneTargetSummary,
    PotionContentsSummary, SoundEventSummary, WrittenBookContentSummary,
};
use bbb_protocol::{JavaString, NbtSummaryValue};

use crate::resource_json::{
    ResourceJsonObject, ResourceJsonValue as Json, MAX_RESOURCE_JSON_DEPTH,
};
use crate::TagCatalog;

/// `MinMaxBounds.Ints`; an exact value is `min == max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntBounds {
    pub min: i32,
    pub max: i32,
}

impl IntBounds {
    pub fn matches(self, value: i32) -> bool {
        self.min <= self.max && (self.min..=self.max).contains(&value)
    }

    fn parse(value: &Json) -> Option<Self> {
        if let Some(exact) = json_i32(value) {
            return Some(Self {
                min: exact,
                max: exact,
            });
        }
        let object = value.as_object()?;
        Some(Self {
            min: object.get("min").map(json_i32).unwrap_or(Some(i32::MIN))?,
            max: object.get("max").map(json_i32).unwrap_or(Some(i32::MAX))?,
        })
    }
}

pub fn int_bounds_match(bounds: Option<IntBounds>, value: i32) -> bool {
    bounds.is_none_or(|bounds| bounds.matches(value))
}

/// `MinMaxBounds.Doubles`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoubleBounds {
    pub min: f64,
    pub max: f64,
}

impl DoubleBounds {
    pub fn matches(self, value: f64) -> bool {
        self.min <= self.max && value >= self.min && value <= self.max
    }

    fn parse(value: &Json) -> Option<Self> {
        if let Some(exact) = value.as_f64() {
            return Some(Self {
                min: exact,
                max: exact,
            });
        }
        let object = value.as_object()?;
        Some(Self {
            min: object
                .get("min")
                .map(Json::as_f64)
                .unwrap_or(Some(f64::NEG_INFINITY))?,
            max: object
                .get("max")
                .map(Json::as_f64)
                .unwrap_or(Some(f64::INFINITY))?,
        })
    }
}

/// A registry `HolderSet` written as one id / `#tag` or a list of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemModelHolderSet(pub Vec<ItemModelHolder>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemModelHolder {
    Key(String),
    Tag(String),
}

impl ItemModelHolderSet {
    fn parse(value: &Json) -> Option<Self> {
        let entry = |value: &Json| {
            let value = value.as_str()?;
            match value.strip_prefix('#') {
                Some(tag) if !tag.is_empty() => Some(ItemModelHolder::Tag(tag.to_string())),
                Some(_) => None,
                None if !value.is_empty() => Some(ItemModelHolder::Key(value.to_string())),
                None => None,
            }
        };
        match value {
            Json::Array(values) => values.iter().map(entry).collect::<Option<_>>().map(Self),
            value => entry(value).map(|entry| Self(vec![entry])),
        }
    }

    pub fn matches(&self, key: &str, tags: Option<&TagCatalog>) -> bool {
        self.0.iter().any(|entry| match entry {
            ItemModelHolder::Key(expected) => expected == key,
            ItemModelHolder::Tag(tag) => tags.is_some_and(|tags| tags.contains(tag, key)),
        })
    }
}

pub fn holder_set_matches(
    set: Option<&ItemModelHolderSet>,
    key: &str,
    tags: Option<&TagCatalog>,
) -> bool {
    set.is_none_or(|set| set.matches(key, tags))
}

/// `CollectionPredicate`: `contains`, `count` and `size` terms.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionPredicate<T> {
    pub contains: Vec<T>,
    pub counts: Vec<(T, IntBounds)>,
    pub size: Option<IntBounds>,
}

impl<T> CollectionPredicate<T> {
    fn parse(value: &Json, element: impl Fn(&Json) -> Option<T>) -> Option<Self> {
        let object = only_keys(value, &["contains", "count", "size"])?;
        let contains = match object.get("contains") {
            None => Vec::new(),
            Some(values) => values
                .as_array()?
                .iter()
                .map(&element)
                .collect::<Option<_>>()?,
        };
        let counts = match object.get("count") {
            None => Vec::new(),
            Some(entries) => entries
                .as_array()?
                .iter()
                .map(|entry| {
                    let entry = only_keys(entry, &["test", "count"])?;
                    Some((
                        element(entry.get("test")?)?,
                        IntBounds::parse(entry.get("count")?)?,
                    ))
                })
                .collect::<Option<_>>()?,
        };
        let size = object
            .get("size")
            .map(IntBounds::parse)
            .transpose_option()?;
        Some(Self {
            contains,
            counts,
            size,
        })
    }

    pub fn matches<A>(&self, values: &[A], size: usize, test: impl Fn(&T, &A) -> bool) -> bool {
        self.contains
            .iter()
            .all(|expected| values.iter().any(|actual| test(expected, actual)))
            && self.counts.iter().all(|(expected, bounds)| {
                i32::try_from(
                    values
                        .iter()
                        .filter(|actual| test(expected, actual))
                        .count(),
                )
                .is_ok_and(|count| bounds.matches(count))
            })
            && self
                .size
                .is_none_or(|bounds| i32::try_from(size).is_ok_and(|size| bounds.matches(size)))
    }
}

/// `ItemPredicate`: item ids, count and data component matchers.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemPredicate {
    pub items: Option<ItemModelHolderSet>,
    pub count: Option<IntBounds>,
    pub components: Vec<ItemExactComponent>,
    pub predicates: Vec<ItemComponentPredicate>,
}

impl ItemPredicate {
    fn parse(value: &Json) -> Option<Self> {
        let object = only_keys(value, &["items", "count", "components"])?;
        let mut predicate = Self {
            items: object
                .get("items")
                .map(ItemModelHolderSet::parse)
                .transpose_option()?,
            count: object
                .get("count")
                .map(IntBounds::parse)
                .transpose_option()?,
            components: Vec::new(),
            predicates: Vec::new(),
        };
        if let Some(matchers) = object.get("components") {
            let matchers = only_keys(matchers, &["components", "predicates"])?;
            if let Some(components) = matchers.get("components") {
                predicate.components = components
                    .as_object()?
                    .iter()
                    .map(|(component, value)| ItemExactComponent::parse(component.as_str()?, value))
                    .collect::<Option<_>>()?;
            }
            if let Some(predicates) = matchers.get("predicates") {
                predicate.predicates = predicates
                    .as_object()?
                    .iter()
                    .map(|(predicate, value)| {
                        ItemComponentPredicate::parse_partial(predicate.as_str()?, value)
                    })
                    .collect::<Option<_>>()?;
            }
        }
        Some(predicate)
    }
}

/// `DataComponentPredicate` subset used by item models.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemComponentPredicate {
    /// `DataComponentPredicate.AnyValueType`: the component is present.
    AnyValue(i32),
    Damage {
        damage: Option<IntBounds>,
        durability: Option<IntBounds>,
    },
    Enchantments {
        stored: bool,
        predicates: Vec<EnchantmentPredicate>,
    },
    PotionContents(ItemModelHolderSet),
    CustomData(NbtSummaryValue),
    Container(CollectionPredicate<ItemPredicate>),
    BundleContents(CollectionPredicate<ItemPredicate>),
    FireworkExplosion(FireworkExplosionPredicate),
    Fireworks {
        explosions: Option<CollectionPredicate<FireworkExplosionPredicate>>,
        flight_duration: Option<IntBounds>,
    },
    WritableBookContent {
        pages: Option<CollectionPredicate<JavaString>>,
    },
    WrittenBookContent {
        author: Option<JavaString>,
        title: Option<JavaString>,
        generation: Option<IntBounds>,
        resolved: Option<bool>,
        pages: Option<CollectionPredicate<String>>,
    },
    AttributeModifiers {
        modifiers: Option<CollectionPredicate<AttributeModifierPredicate>>,
    },
    Trim {
        material: Option<ItemModelHolderSet>,
        pattern: Option<ItemModelHolderSet>,
    },
    JukeboxPlayable {
        song: Option<ItemModelHolderSet>,
    },
    VillagerVariant(ItemModelHolderSet),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantmentPredicate {
    pub enchantments: Option<ItemModelHolderSet>,
    pub levels: Option<IntBounds>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireworkExplosionPredicate {
    pub shape: Option<FireworkExplosionShapeSummary>,
    pub has_twinkle: Option<bool>,
    pub has_trail: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeModifierPredicate {
    pub attribute: Option<ItemModelHolderSet>,
    pub id: Option<String>,
    pub amount: Option<DoubleBounds>,
    pub operation: Option<i32>,
    pub slot: Option<i32>,
}

impl ItemComponentPredicate {
    /// The `minecraft:component` condition's `predicate` / `value` pair.
    pub(super) fn parse_condition(predicate: &str, value: Option<&Json>) -> Option<Self> {
        let value = value?;
        match predicate {
            "minecraft:container" | "minecraft:bundle_contents" => {
                let object = value.as_object()?;
                if object.is_empty() {
                    return component_type_id(predicate).map(Self::AnyValue);
                }
                let items = CollectionPredicate::parse(
                    only_keys(value, &["items"])?.get("items")?,
                    ItemPredicate::parse,
                )?;
                Some(if predicate == "minecraft:container" {
                    Self::Container(items)
                } else {
                    Self::BundleContents(items)
                })
            }
            _ if predicate_type_is_complex(predicate) => Self::parse_partial(predicate, value),
            _ => component_type_id(predicate).map(Self::AnyValue),
        }
    }

    /// One entry of `DataComponentMatchers.predicates`.
    fn parse_partial(predicate: &str, value: &Json) -> Option<Self> {
        Some(match predicate {
            "minecraft:custom_data" => Self::CustomData(custom_data_nbt(value)?),
            "minecraft:damage" => {
                let object = only_keys(value, &["damage", "durability"])?;
                Self::Damage {
                    damage: object
                        .get("damage")
                        .map(IntBounds::parse)
                        .transpose_option()?,
                    durability: object
                        .get("durability")
                        .map(IntBounds::parse)
                        .transpose_option()?,
                }
            }
            "minecraft:enchantments" | "minecraft:stored_enchantments" => Self::Enchantments {
                stored: predicate == "minecraft:stored_enchantments",
                predicates: value
                    .as_array()?
                    .iter()
                    .map(|predicate| {
                        let predicate = only_keys(predicate, &["levels", "enchantments"])?;
                        Some(EnchantmentPredicate {
                            enchantments: predicate
                                .get("enchantments")
                                .map(ItemModelHolderSet::parse)
                                .transpose_option()?,
                            levels: predicate
                                .get("levels")
                                .map(IntBounds::parse)
                                .transpose_option()?,
                        })
                    })
                    .collect::<Option<_>>()?,
            },
            "minecraft:trim" => {
                let object = only_keys(value, &["material", "pattern"])?;
                Self::Trim {
                    material: optional_holder_set(object, "material")?,
                    pattern: optional_holder_set(object, "pattern")?,
                }
            }
            "minecraft:jukebox_playable" => Self::JukeboxPlayable {
                song: optional_holder_set(only_keys(value, &["song"])?, "song")?,
            },
            "minecraft:potion_contents" => Self::PotionContents(ItemModelHolderSet::parse(value)?),
            "minecraft:villager/variant" => {
                Self::VillagerVariant(ItemModelHolderSet::parse(value)?)
            }
            "minecraft:writable_book_content" => Self::WritableBookContent {
                pages: only_keys(value, &["pages"])?
                    .get("pages")
                    .map(|pages| {
                        CollectionPredicate::parse(pages, |page| {
                            page.as_string().map(|page| page.utf16_units().into())
                        })
                    })
                    .transpose_option()?,
            },
            "minecraft:written_book_content" => {
                let object = only_keys(
                    value,
                    &["author", "title", "generation", "resolved", "pages"],
                )?;
                let java_string = |field: &str| {
                    object
                        .get(field)
                        .map(|value| value.as_string().map(|value| value.utf16_units().into()))
                        .transpose_option()
                };
                Self::WrittenBookContent {
                    author: java_string("author")?,
                    title: java_string("title")?,
                    generation: object
                        .get("generation")
                        .map(IntBounds::parse)
                        .transpose_option()?,
                    resolved: object
                        .get("resolved")
                        .map(Json::as_bool)
                        .transpose_option()?,
                    pages: object
                        .get("pages")
                        .map(|pages| {
                            CollectionPredicate::parse(pages, |page| {
                                simple_component_text(page).map(str::to_string)
                            })
                        })
                        .transpose_option()?,
                }
            }
            "minecraft:attribute_modifiers" => Self::AttributeModifiers {
                modifiers: only_keys(value, &["modifiers"])?
                    .get("modifiers")
                    .map(|modifiers| {
                        CollectionPredicate::parse(modifiers, AttributeModifierPredicate::parse)
                    })
                    .transpose_option()?,
            },
            "minecraft:firework_explosion" => {
                Self::FireworkExplosion(FireworkExplosionPredicate::parse(value)?)
            }
            "minecraft:fireworks" => {
                let object = only_keys(value, &["explosions", "flight_duration"])?;
                Self::Fireworks {
                    explosions: object
                        .get("explosions")
                        .map(|explosions| {
                            CollectionPredicate::parse(
                                explosions,
                                FireworkExplosionPredicate::parse,
                            )
                        })
                        .transpose_option()?,
                    flight_duration: object
                        .get("flight_duration")
                        .map(IntBounds::parse)
                        .transpose_option()?,
                }
            }
            _ if predicate_type_is_complex(predicate) => return None,
            _ => {
                if !value.as_object()?.is_empty() {
                    return None;
                }
                Self::AnyValue(component_type_id(predicate)?)
            }
        })
    }
}

impl FireworkExplosionPredicate {
    fn parse(value: &Json) -> Option<Self> {
        let object = only_keys(value, &["shape", "has_twinkle", "has_trail"])?;
        Some(Self {
            shape: object
                .get("shape")
                .map(|shape| {
                    shape
                        .as_str()
                        .and_then(FireworkExplosionShapeSummary::from_serialized_name)
                })
                .transpose_option()?,
            has_twinkle: object
                .get("has_twinkle")
                .map(Json::as_bool)
                .transpose_option()?,
            has_trail: object
                .get("has_trail")
                .map(Json::as_bool)
                .transpose_option()?,
        })
    }
}

impl AttributeModifierPredicate {
    fn parse(value: &Json) -> Option<Self> {
        let object = only_keys(value, &["attribute", "id", "amount", "operation", "slot"])?;
        Some(Self {
            attribute: optional_holder_set(object, "attribute")?,
            id: object
                .get("id")
                .map(|id| id.as_str().map(str::to_string))
                .transpose_option()?,
            amount: object
                .get("amount")
                .map(DoubleBounds::parse)
                .transpose_option()?,
            operation: object
                .get("operation")
                .map(|operation| operation.as_str().and_then(attribute_modifier_operation_id))
                .transpose_option()?,
            slot: object
                .get("slot")
                .map(|slot| slot.as_str().and_then(equipment_slot_group_id))
                .transpose_option()?,
        })
    }
}

/// `DataComponentExactPredicate`: one component compared by value.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemExactComponent {
    CustomName(String),
    ItemName(String),
    Lore(Vec<String>),
    /// Components whose value is one number, string or flag.
    Scalar {
        type_id: i32,
        value: ItemModelScalar,
    },
    Unbreakable,
    CustomData(NbtSummaryValue),
    LodestoneTracker {
        target: Option<LodestoneTargetSummary>,
        tracked: bool,
    },
    AttributeModifiers(Vec<ExactAttributeModifier>),
    Enchantments {
        stored: bool,
        levels: Vec<(String, i32)>,
    },
    PotionContents(PotionContentsSummary),
    WritableBookContent {
        pages: Vec<JavaString>,
        page_filters: Vec<Option<JavaString>>,
    },
    WrittenBookContent(WrittenBookContentSummary),
    FireworkExplosion(FireworkExplosionSummary),
    Fireworks {
        flight_duration: i32,
        explosions: Vec<FireworkExplosionSummary>,
    },
    JukeboxPlayable(ExactJukeboxPlayable),
    Trim {
        material: ExactTrimMaterial,
        pattern: ExactTrimPattern,
    },
    VillagerVariant(String),
}

/// A select `when` value or exact scalar component value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemModelScalar {
    /// Pack strings and stack values are both Java strings, so a case written
    /// with an unpaired unit never matches a replacement character.
    String(JavaString),
    I32(i32),
    Bool(bool),
}

impl ItemModelScalar {
    pub(super) fn parse(value: &Json) -> Option<Self> {
        match value {
            Json::String(value) => Some(Self::String(value.utf16_units().into())),
            Json::Bool(value) => Some(Self::Bool(*value)),
            Json::Number(_) => value
                .as_i64()
                .and_then(|value| i32::try_from(value).ok())
                .or_else(|| value.as_u64().and_then(|value| i32::try_from(value).ok()))
                .map(Self::I32),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactAttributeModifier {
    pub attribute: String,
    pub modifier_id: String,
    pub amount_bits: u64,
    pub operation_id: i32,
    pub slot_id: i32,
    pub display_id: i32,
    pub display_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactJukeboxPlayable {
    RegistryKey(String),
    DirectSong {
        sound_event: SoundEventSummary,
        description: String,
        length_in_seconds_bits: u32,
        comparator_output: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactTrimMaterial {
    RegistryKey(String),
    Direct {
        asset_name: String,
        override_armor_assets: BTreeMap<String, String>,
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactTrimPattern {
    RegistryKey(String),
    Direct {
        asset_id: String,
        description: String,
        decal: bool,
    },
}

impl ItemExactComponent {
    fn parse(component: &str, value: &Json) -> Option<Self> {
        Some(match component {
            "minecraft:custom_name" => Self::CustomName(simple_component_text(value)?.to_string()),
            "minecraft:item_name" => Self::ItemName(simple_component_text(value)?.to_string()),
            "minecraft:lore" => Self::Lore(
                value
                    .as_array()?
                    .iter()
                    .map(|line| simple_component_text(line).map(str::to_string))
                    .collect::<Option<_>>()?,
            ),
            "minecraft:unbreakable" => {
                value.as_object().filter(|value| value.is_empty())?;
                Self::Unbreakable
            }
            "minecraft:custom_data" => Self::CustomData(custom_data_nbt(value)?),
            "minecraft:lodestone_tracker" => {
                let object = only_keys(value, &["target", "tracked"])?;
                if let Some(target) = object.get("target").filter(|target| !matches!(target, Json::Null)) {
                    only_keys(target, &["dimension", "pos"])?;
                }
                let (target, tracked) =
                    bbb_registry_codec::persistent_item_stack_projection::read_lodestone_tracker(
                        bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
                            &value.to_registry_json(),
                        ),
                    )
                    .ok()?;
                Self::LodestoneTracker { target, tracked }
            }
            "minecraft:attribute_modifiers" => Self::AttributeModifiers(
                value
                    .as_array()?
                    .iter()
                    .map(exact_attribute_modifier)
                    .collect::<Option<_>>()?,
            ),
            "minecraft:enchantments" | "minecraft:stored_enchantments" => Self::Enchantments {
                stored: component == "minecraft:stored_enchantments",
                levels: value
                    .as_object()?
                    .iter()
                    .map(|(key, level)| {
                        let key = key.as_str()?;
                        let level = json_i32(level)?;
                        (!key.is_empty() && !key.starts_with('#') && (1..=255).contains(&level))
                            .then(|| (key.to_string(), level))
                    })
                    .collect::<Option<_>>()?,
            },
            "minecraft:potion_contents" => Self::PotionContents(
                bbb_registry_codec::persistent_item_stack_projection::potion_contents::read(
                    bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
                        &value.to_registry_json(),
                    ),
                    MAX_RESOURCE_JSON_DEPTH,
                )
                .ok()?,
            ),
            "minecraft:writable_book_content" => {
                let (pages, page_filters) =
                    bbb_registry_codec::persistent_item_stack_projection::book_components::read_writable(
                        bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
                            &value.to_registry_json(),
                        ),
                    )
                    .ok()?;
                Self::WritableBookContent {
                    pages,
                    page_filters,
                }
            }
            "minecraft:written_book_content" => {
                let (content, _, _) =
                    bbb_registry_codec::persistent_item_stack_projection::book_components::read_written(
                        bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
                            &value.to_registry_json(),
                        ),
                    )
                    .ok()?;
                Self::WrittenBookContent(content)
            }
            "minecraft:firework_explosion" => Self::FireworkExplosion(
                bbb_registry_codec::persistent_item_stack_projection::firework_components::read_explosion(
                    bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
                        &value.to_registry_json(),
                    ),
                )
                .ok()?,
            ),
            "minecraft:fireworks" => {
                let (flight_duration, explosions) =
                    bbb_registry_codec::persistent_item_stack_projection::firework_components::read_fireworks(
                        bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
                            &value.to_registry_json(),
                        ),
                    )
                    .ok()?;
                Self::Fireworks {
                    flight_duration,
                    explosions,
                }
            }
            "minecraft:jukebox_playable" => Self::JukeboxPlayable(exact_jukebox_playable(value)?),
            "minecraft:trim" => {
                let object = only_keys(value, &["material", "pattern"])?;
                Self::Trim {
                    material: exact_trim_material(object.get("material")?)?,
                    pattern: exact_trim_pattern(object.get("pattern")?)?,
                }
            }
            "minecraft:villager/variant" => {
                Self::VillagerVariant(direct_registry_key(value)?.to_string())
            }
            _ => {
                let component = vanilla_data_component_type_for_resource_id(component)
                    .filter(|component| component.persistent)?;
                if !scalar_exact_component(component.type_id) {
                    return None;
                }
                Self::Scalar {
                    type_id: component.type_id,
                    value: ItemModelScalar::parse(value)?,
                }
            }
        })
    }
}

fn scalar_exact_component(type_id: i32) -> bool {
    matches!(
        type_id,
        component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID
            | component_id::VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID
    )
}

fn exact_attribute_modifier(value: &Json) -> Option<ExactAttributeModifier> {
    let object = only_keys(
        value,
        &["type", "id", "amount", "operation", "slot", "display"],
    )?;
    let (display_id, display_text) = match object.get("display") {
        None => (VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, None),
        Some(display) => {
            let display = only_keys(display, &["type", "value"])?;
            match attribute_modifier_display_id(display.get("type")?.as_str()?)? {
                id @ (VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID
                | VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID)
                    if !display.contains_key("value") =>
                {
                    (id, None)
                }
                id @ VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID => {
                    (id, Some(component_summary_text(display.get("value")?)?))
                }
                _ => return None,
            }
        }
    };
    Some(ExactAttributeModifier {
        attribute: direct_registry_key(object.get("type")?)?.to_string(),
        modifier_id: object.get("id")?.as_str()?.to_string(),
        amount_bits: object.get("amount")?.as_f64()?.to_bits(),
        operation_id: attribute_modifier_operation_id(object.get("operation")?.as_str()?)?,
        slot_id: object
            .get("slot")
            .map(|slot| slot.as_str().and_then(equipment_slot_group_id))
            .unwrap_or(Some(0))?,
        display_id,
        display_text,
    })
}

fn exact_jukebox_playable(value: &Json) -> Option<ExactJukeboxPlayable> {
    if value.is_string() {
        return Some(ExactJukeboxPlayable::RegistryKey(
            direct_registry_key(value)?.to_string(),
        ));
    }
    let object = only_keys(
        value,
        &[
            "sound_event",
            "description",
            "length_in_seconds",
            "comparator_output",
        ],
    )?;
    let length = object.get("length_in_seconds")?.as_f64()? as f32;
    let comparator_output = json_i32(object.get("comparator_output")?)?;
    if !length.is_finite() || length <= 0.0 || !(0..=15).contains(&comparator_output) {
        return None;
    }
    let sound_event = bbb_registry_codec::persistent_item_stack_projection::read_sound_event(
        component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        "minecraft:jukebox_playable",
        bbb_registry_codec::registry_codec::RegistryElementValueRef::Json(
            &object.get("sound_event")?.to_registry_json(),
        ),
    )
    .ok()?;
    Some(ExactJukeboxPlayable::DirectSong {
        sound_event,
        description: component_summary_text(object.get("description")?)?,
        length_in_seconds_bits: length.to_bits(),
        comparator_output,
    })
}

fn exact_trim_material(value: &Json) -> Option<ExactTrimMaterial> {
    if value.is_string() {
        return Some(ExactTrimMaterial::RegistryKey(
            direct_registry_key(value)?.to_string(),
        ));
    }
    let object = only_keys(
        value,
        &["asset_name", "override_armor_assets", "description"],
    )?;
    let suffix = |value: &Json| {
        value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let mut override_armor_assets = BTreeMap::new();
    if let Some(overrides) = object.get("override_armor_assets") {
        for (key, value) in overrides.as_object()?.iter() {
            let key = key
                .as_str()
                .filter(|key| !key.is_empty() && !key.starts_with('#'))?;
            override_armor_assets.insert(key.to_string(), suffix(value)?);
        }
    }
    Some(ExactTrimMaterial::Direct {
        asset_name: suffix(object.get("asset_name")?)?,
        override_armor_assets,
        description: component_summary_text(object.get("description")?)?,
    })
}

fn exact_trim_pattern(value: &Json) -> Option<ExactTrimPattern> {
    if value.is_string() {
        return Some(ExactTrimPattern::RegistryKey(
            direct_registry_key(value)?.to_string(),
        ));
    }
    let object = only_keys(value, &["asset_id", "description", "decal"])?;
    Some(ExactTrimPattern::Direct {
        asset_id: direct_registry_key(object.get("asset_id")?)?.to_string(),
        description: component_summary_text(object.get("description")?)?,
        decal: object
            .get("decal")
            .map(Json::as_bool)
            .unwrap_or(Some(false))?,
    })
}

/// Flattened text of a JSON text component, `None` once any text is not a
/// valid `String`.
fn component_summary_text(value: &Json) -> Option<String> {
    fn append(value: &Json, out: &mut String) -> Option<()> {
        match value {
            Json::String(text) => out.push_str(text.as_str()?),
            Json::Array(items) => {
                for item in items {
                    append(item, out)?;
                }
            }
            Json::Object(entries) => {
                if let Some(text) = entries.get("text") {
                    append(text, out)?;
                } else {
                    if let Some(primary) = ["fallback", "translate", "keybind", "selector", "nbt"]
                        .into_iter()
                        .find_map(|field| entries.get(field))
                    {
                        append(primary, out)?;
                    }
                    if let Some(with) = entries.get("with") {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        append(with, out)?;
                    }
                }
                if let Some(extra) = entries.get("extra") {
                    append(extra, out)?;
                }
            }
            _ => {}
        }
        Some(())
    }
    let mut out = String::new();
    append(value, &mut out)?;
    (!out.is_empty()).then_some(out)
}

fn simple_component_text(value: &Json) -> Option<&str> {
    match value {
        Json::String(value) => value.as_str(),
        Json::Object(value) if value.len() == 1 => value.get("text").and_then(Json::as_str),
        _ => None,
    }
}

fn custom_data_nbt(value: &Json) -> Option<NbtSummaryValue> {
    match value {
        Json::String(value) => bbb_protocol::parse_snbt_compound_utf16(&value.utf16_units()).ok(),
        _ => value
            .to_nbt_summary()
            .ok()
            .filter(|value| matches!(value, NbtSummaryValue::Compound(_))),
    }
}

fn optional_holder_set(
    object: &ResourceJsonObject,
    field: &str,
) -> Option<Option<ItemModelHolderSet>> {
    object
        .get(field)
        .map(ItemModelHolderSet::parse)
        .transpose_option()
}

fn direct_registry_key(value: &Json) -> Option<&str> {
    value
        .as_str()
        .filter(|value| !value.is_empty() && !value.starts_with('#'))
}

fn equipment_slot_group_id(value: &str) -> Option<i32> {
    Some(vanilla_equipment_slot_group_for_serialized_name(value)?.id)
}

/// Predicate types with their own codec (`DataComponentPredicates`); every
/// other id names a component for `AnyValueType`.
fn predicate_type_is_complex(predicate: &str) -> bool {
    matches!(
        predicate,
        "minecraft:damage"
            | "minecraft:enchantments"
            | "minecraft:stored_enchantments"
            | "minecraft:potion_contents"
            | "minecraft:custom_data"
            | "minecraft:container"
            | "minecraft:bundle_contents"
            | "minecraft:firework_explosion"
            | "minecraft:fireworks"
            | "minecraft:writable_book_content"
            | "minecraft:written_book_content"
            | "minecraft:attribute_modifiers"
            | "minecraft:trim"
            | "minecraft:jukebox_playable"
            | "minecraft:villager/variant"
    )
}

pub(super) fn component_type_id(component: &str) -> Option<i32> {
    vanilla_data_component_type_for_resource_id(component).map(|component| component.type_id)
}

fn only_keys<'a>(value: &'a Json, keys: &[&str]) -> Option<&'a ResourceJsonObject> {
    let object = value.as_object()?;
    object
        .keys()
        .all(|key| key.as_str().is_some_and(|key| keys.contains(&key)))
        .then_some(object)
}

fn json_i32(value: &Json) -> Option<i32> {
    i32::try_from(value.as_i64()?).ok()
}

/// `Option<Option<T>>` from a parsed optional field: the outer `None` rejects
/// the whole predicate.
trait TransposeOption<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}

impl<T> TransposeOption<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(None) => None,
            Some(Some(value)) => Some(Some(value)),
        }
    }
}
