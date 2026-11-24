//! Typed `condition` / `range_dispatch` / `select` properties
//! (`ConditionalItemModelProperties`, `RangeSelectItemModelProperties`,
//! `SelectItemModelProperties`). A property this client cannot evaluate keeps
//! its type id as `Unsupported` so the tree can collapse that branch at bake.

use bbb_protocol::data_component_types::{
    vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
};
use bbb_protocol::{JavaString, NbtSummaryEntry, NbtSummaryValue};

use super::predicates::{component_type_id, ItemComponentPredicate, ItemModelScalar};
use crate::resource_json::ResourceJsonValue as Json;
use crate::resources::ResourceLocation;

#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelCondition {
    Broken,
    Damaged,
    BundleHasSelectedItem,
    Carried,
    Selected,
    UsingItem,
    ViewEntity,
    ExtendedView,
    FishingRodCast,
    CustomModelData {
        index: usize,
    },
    HasComponent {
        component: i32,
        ignore_default: bool,
    },
    KeybindDown {
        keybind: String,
    },
    Component(ItemComponentPredicate),
    Unsupported(String),
}

impl ItemModelCondition {
    pub(super) fn parse(property: &str, object: &Json) -> Self {
        let parsed = match property {
            "minecraft:broken" => Some(Self::Broken),
            "minecraft:damaged" => Some(Self::Damaged),
            "minecraft:bundle/has_selected_item" => Some(Self::BundleHasSelectedItem),
            "minecraft:carried" => Some(Self::Carried),
            "minecraft:selected" => Some(Self::Selected),
            "minecraft:using_item" => Some(Self::UsingItem),
            "minecraft:view_entity" => Some(Self::ViewEntity),
            "minecraft:extended_view" => Some(Self::ExtendedView),
            "minecraft:fishing_rod/cast" => Some(Self::FishingRodCast),
            "minecraft:custom_model_data" => Some(Self::CustomModelData {
                index: index_field(object),
            }),
            "minecraft:has_component" => object
                .get("component")
                .and_then(Json::as_str)
                .and_then(normalized_component_type_id)
                .map(|component| Self::HasComponent {
                    component,
                    ignore_default: bool_field(object, "ignore_default", false),
                }),
            "minecraft:keybind_down" => {
                object
                    .get("keybind")
                    .and_then(Json::as_str)
                    .map(|keybind| Self::KeybindDown {
                        keybind: keybind.to_string(),
                    })
            }
            "minecraft:component" => object
                .get("predicate")
                .and_then(Json::as_str)
                .and_then(|predicate| {
                    ItemComponentPredicate::parse_condition(predicate, object.get("value"))
                })
                .map(Self::Component),
            _ => None,
        };
        parsed.unwrap_or_else(|| Self::Unsupported(property.to_string()))
    }

    pub fn property_type(&self) -> &str {
        match self {
            Self::Broken => "minecraft:broken",
            Self::Damaged => "minecraft:damaged",
            Self::BundleHasSelectedItem => "minecraft:bundle/has_selected_item",
            Self::Carried => "minecraft:carried",
            Self::Selected => "minecraft:selected",
            Self::UsingItem => "minecraft:using_item",
            Self::ViewEntity => "minecraft:view_entity",
            Self::ExtendedView => "minecraft:extended_view",
            Self::FishingRodCast => "minecraft:fishing_rod/cast",
            Self::CustomModelData { .. } => "minecraft:custom_model_data",
            Self::HasComponent { .. } => "minecraft:has_component",
            Self::KeybindDown { .. } => "minecraft:keybind_down",
            Self::Component(_) => "minecraft:component",
            Self::Unsupported(property) => property,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemModelCompassTarget {
    None,
    Lodestone,
    Recovery,
    Spawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemModelTimeSource {
    Random,
    Daytime,
    MoonPhase,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelRangeProperty {
    Damage {
        normalize: bool,
    },
    CustomModelData {
        index: usize,
    },
    Count {
        normalize: bool,
    },
    BundleFullness,
    Cooldown,
    UseDuration {
        remaining: bool,
    },
    UseCycle {
        period: f32,
    },
    CrossbowPull,
    Compass {
        target: ItemModelCompassTarget,
        wobble: bool,
    },
    Time {
        source: ItemModelTimeSource,
        wobble: bool,
    },
    Unsupported(String),
}

impl ItemModelRangeProperty {
    pub(super) fn parse(property: &str, object: &Json) -> Self {
        let wobble = bool_field(object, "wobble", true);
        let parsed = match property {
            "minecraft:damage" => Some(Self::Damage {
                normalize: bool_field(object, "normalize", true),
            }),
            "minecraft:custom_model_data" => Some(Self::CustomModelData {
                index: index_field(object),
            }),
            "minecraft:count" => Some(Self::Count {
                normalize: bool_field(object, "normalize", true),
            }),
            "minecraft:bundle/fullness" => Some(Self::BundleFullness),
            "minecraft:cooldown" => Some(Self::Cooldown),
            "minecraft:use_duration" => Some(Self::UseDuration {
                remaining: bool_field(object, "remaining", false),
            }),
            "minecraft:use_cycle" => Some(Self::UseCycle {
                period: object
                    .get("period")
                    .and_then(Json::as_f64)
                    .map(|period| period as f32)
                    .filter(|period| *period > 0.0)
                    .unwrap_or(1.0),
            }),
            "minecraft:crossbow/pull" => Some(Self::CrossbowPull),
            "minecraft:compass" => object
                .get("target")
                .and_then(Json::as_str)
                .and_then(|target| match target {
                    "none" => Some(ItemModelCompassTarget::None),
                    "lodestone" => Some(ItemModelCompassTarget::Lodestone),
                    "recovery" => Some(ItemModelCompassTarget::Recovery),
                    "spawn" => Some(ItemModelCompassTarget::Spawn),
                    _ => None,
                })
                .map(|target| Self::Compass { target, wobble }),
            "minecraft:time" => object
                .get("source")
                .and_then(Json::as_str)
                .and_then(|source| match source {
                    "random" => Some(ItemModelTimeSource::Random),
                    "daytime" => Some(ItemModelTimeSource::Daytime),
                    "moon_phase" => Some(ItemModelTimeSource::MoonPhase),
                    _ => None,
                })
                .map(|source| Self::Time { source, wobble }),
            _ => None,
        };
        parsed.unwrap_or_else(|| Self::Unsupported(property.to_string()))
    }

    pub fn property_type(&self) -> &str {
        match self {
            Self::Damage { .. } => "minecraft:damage",
            Self::CustomModelData { .. } => "minecraft:custom_model_data",
            Self::Count { .. } => "minecraft:count",
            Self::BundleFullness => "minecraft:bundle/fullness",
            Self::Cooldown => "minecraft:cooldown",
            Self::UseDuration { .. } => "minecraft:use_duration",
            Self::UseCycle { .. } => "minecraft:use_cycle",
            Self::CrossbowPull => "minecraft:crossbow/pull",
            Self::Compass { .. } => "minecraft:compass",
            Self::Time { .. } => "minecraft:time",
            Self::Unsupported(property) => property,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelSelectProperty {
    DisplayContext,
    MainHand,
    ContextDimension,
    ContextEntityType,
    LocalTime {
        pattern: String,
        locale: String,
        time_zone: Option<String>,
    },
    ChargeType,
    TrimMaterial,
    BlockState {
        property: JavaString,
    },
    CustomModelData {
        index: usize,
    },
    /// A persistent component compared through its own codec.
    Component {
        type_id: i32,
    },
    Unsupported(String),
}

impl ItemModelSelectProperty {
    pub(super) fn parse(property: &str, object: &Json) -> Self {
        let string = |field: &str| object.get(field).and_then(Json::as_str).map(str::to_string);
        let parsed = match property {
            "minecraft:display_context" => Some(Self::DisplayContext),
            "minecraft:main_hand" => Some(Self::MainHand),
            "minecraft:context_dimension" => Some(Self::ContextDimension),
            "minecraft:context_entity_type" => Some(Self::ContextEntityType),
            "minecraft:local_time" => Some(Self::LocalTime {
                pattern: string("pattern").unwrap_or_default(),
                locale: string("locale").unwrap_or_default(),
                time_zone: string("time_zone"),
            }),
            "minecraft:charge_type" => Some(Self::ChargeType),
            "minecraft:trim_material" => Some(Self::TrimMaterial),
            "minecraft:block_state" => object
                .get("block_state_property")
                .and_then(Json::as_string)
                .map(|property| Self::BlockState {
                    property: property.utf16_units().into(),
                }),
            "minecraft:custom_model_data" => Some(Self::CustomModelData {
                index: index_field(object),
            }),
            "minecraft:component" => object
                .get("component")
                .and_then(Json::as_str)
                .and_then(vanilla_data_component_type_for_resource_id)
                .filter(|component| component.persistent)
                .map(|component| Self::Component {
                    type_id: component.type_id,
                }),
            _ => None,
        };
        parsed.unwrap_or_else(|| Self::Unsupported(property.to_string()))
    }

    pub fn property_type(&self) -> &str {
        match self {
            Self::DisplayContext => "minecraft:display_context",
            Self::MainHand => "minecraft:main_hand",
            Self::ContextDimension => "minecraft:context_dimension",
            Self::ContextEntityType => "minecraft:context_entity_type",
            Self::LocalTime { .. } => "minecraft:local_time",
            Self::ChargeType => "minecraft:charge_type",
            Self::TrimMaterial => "minecraft:trim_material",
            Self::BlockState { .. } => "minecraft:block_state",
            Self::CustomModelData { .. } => "minecraft:custom_model_data",
            Self::Component { .. } => "minecraft:component",
            Self::Unsupported(property) => property,
        }
    }

    /// Typed `when` values. `ExtraCodecs.compactListCodec` makes a JSON list
    /// several values unless the property's own value is a list, which only
    /// component values can be; those keep both readings.
    pub(super) fn parse_when(&self, when: &Json) -> ItemModelSelectWhen {
        let compact = match when {
            Json::Array(values) => values.iter().collect(),
            value => vec![value],
        };
        let Self::Component { type_id } = *self else {
            return ItemModelSelectWhen::Values(
                compact
                    .into_iter()
                    .filter_map(ItemModelScalar::parse)
                    .collect(),
            );
        };
        let to_nbt = |value: &Json| {
            if type_id == VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID {
                if let Json::Object(entries) = value {
                    return entries
                        .iter()
                        .map(|(name, value)| {
                            Some(NbtSummaryEntry {
                                name: JavaString::from(name.utf16_units()),
                                value: NbtSummaryValue::string_from_utf16(
                                    value.as_string()?.utf16_units(),
                                ),
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                        .map(NbtSummaryValue::Compound);
                }
            }
            value.to_nbt_summary().ok()
        };
        let block_state_object =
            type_id == VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID && when.as_object().is_some();
        if !block_state_object && when.to_nbt_summary().is_err() {
            return ItemModelSelectWhen::Component(ComponentSelectWhen::default());
        }
        ItemModelSelectWhen::Component(ComponentSelectWhen {
            alternatives: compact.iter().map(|value| to_nbt(value)).collect(),
            whole: to_nbt(when),
            strings: compact
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelSelectWhen {
    Values(Vec<ItemModelScalar>),
    Component(ComponentSelectWhen),
}

/// A component `when` value kept as tags until the component codec reads it
/// against the registries in use.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentSelectWhen {
    /// Each element of a compact list, if every element converts.
    pub alternatives: Option<Vec<NbtSummaryValue>>,
    /// The whole value, for components whose own value is a list.
    pub whole: Option<NbtSummaryValue>,
    /// String elements, for registry keys that need no codec.
    pub strings: Vec<String>,
}

fn normalized_component_type_id(component: &str) -> Option<i32> {
    component_type_id(&ResourceLocation::parse(component).ok()?.id())
}

fn index_field(object: &Json) -> usize {
    object
        .get("index")
        .and_then(Json::as_u64)
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or(0)
}

fn bool_field(object: &Json, field: &str, default: bool) -> bool {
    object.get(field).and_then(Json::as_bool).unwrap_or(default)
}
