//! `LootItemFunction`: the 26.1 function types, decoded once. Component
//! parameters are read by the same component codecs item stacks use, so a
//! function carries the component value it will write.

use crate::frozen_lookup::FrozenDatapackRegistryLookup;
use std::sync::{Arc, OnceLock};

use bbb_protocol::block_entity_types::vanilla_block_entity_type_for_resource_id;
use bbb_protocol::block_state_types::vanilla_block_property_values;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::data_component_types::{
    vanilla_data_component_type, vanilla_data_component_type_for_resource_id,
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID as BANNER_PATTERNS,
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID as BUNDLE_CONTENTS,
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID as CHARGED_PROJECTILES,
    VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID as CONTAINER,
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID as CUSTOM_NAME,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID as FIREWORKS,
    VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID as FIREWORK_EXPLOSION,
    VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID as ITEM_NAME,
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID as WRITABLE_BOOK,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID as WRITTEN_BOOK,
};
use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::loot_context_types::VanillaLootContextParam as LootContextParam;

use super::codec::{
    boolean, custom_data_tag, entries, fail, float, id, int, list, long, source, string,
    text_parameter_with_content_registry, to_nbt, At, Obj, Owned, Result, V,
};
use super::condition::{conditions, LootCondition};
use super::nbt_copy::{self, LootNbtCopy};
use super::number::{
    entity_target, standalone_list, HolderSet, IntRange, ListOperation, NumberProvider,
};
use super::predicate::ItemPredicate;
use super::{item_id_with_content_registry, lookup_content_registry, LootEntry};

/// One function with the `conditions` every `LootItemConditionalFunction`
/// carries (empty for `sequence`, which has none).
#[derive(Debug, Clone, PartialEq)]
pub struct LootFunction {
    pub conditions: Vec<LootCondition>,
    pub kind: LootFunctionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LootFunctionKind {
    ApplyBonus {
        enchantment: String,
        formula: BonusFormula,
    },
    CopyComponents {
        source: LootContextParam,
        include: Option<Vec<&'static str>>,
        exclude: Option<Vec<&'static str>>,
    },
    CopyCustomData(LootNbtCopy),
    CopyName(LootContextParam),
    CopyState {
        block: String,
        /// The listed properties the block declares.
        properties: Vec<String>,
    },
    Discard,
    EnchantRandomly {
        options: Option<HolderSet>,
        only_compatible: bool,
        include_additional_cost_component: bool,
    },
    EnchantWithLevels {
        levels: NumberProvider,
        options: Option<HolderSet>,
        include_additional_cost_component: bool,
    },
    EnchantedCountIncrease {
        enchantment: String,
        count: NumberProvider,
        limit: i32,
    },
    ExplorationMap(LootExplorationMap),
    ExplosionDecay,
    FillPlayerHead(LootContextParam),
    Filtered {
        item_filter: ItemPredicate,
        on_pass: Option<Box<LootFunction>>,
        on_fail: Option<Box<LootFunction>>,
    },
    FurnaceSmelt {
        use_input_count: bool,
    },
    LimitCount(IntRange),
    ModifyContents {
        component: i32,
        modifier: Box<LootFunction>,
    },
    Reference(String),
    Sequence(Vec<LootFunction>),
    SetAttributes {
        modifiers: Vec<AttributeModifierTemplate>,
        replace: bool,
    },
    SetBannerPattern {
        patterns: Vec<NbtSummaryValue>,
        append: bool,
    },
    /// The `written_book_content` the cover writes, and the fields kept from
    /// the held value.
    SetBookCover {
        value: NbtSummaryValue,
        retain: Vec<&'static str>,
    },
    SetComponents(ComponentPatchParameter),
    SetContents {
        component: i32,
        entries: Vec<LootEntry>,
    },
    SetCount {
        count: NumberProvider,
        add: bool,
    },
    SetCustomData(Vec<NbtSummaryEntry>),
    SetCustomModelData {
        floats: Option<(ListOperation, Vec<NumberProvider>)>,
        flags: Option<(ListOperation, Vec<NbtSummaryValue>)>,
        strings: Option<(ListOperation, Vec<NbtSummaryValue>)>,
        colors: Option<(ListOperation, Vec<NumberProvider>)>,
    },
    SetDamage {
        damage: NumberProvider,
        add: bool,
    },
    SetEnchantments {
        enchantments: Vec<(String, NumberProvider)>,
        add: bool,
    },
    /// The `firework_explosion` fields the function spells.
    SetFireworkExplosion(Vec<NbtSummaryEntry>),
    SetFireworks {
        flight_duration: Option<i8>,
        explosions: Option<(ListOperation, Vec<NbtSummaryValue>)>,
    },
    SetInstrument(HolderSet),
    SetItem(i32),
    SetLootTable {
        name: String,
        seed: i64,
    },
    SetLore {
        lore: Vec<NbtSummaryValue>,
        mode: ListOperation,
        entity: Option<LootContextParam>,
    },
    SetName {
        name: Option<NbtSummaryValue>,
        entity: Option<LootContextParam>,
        /// `custom_name` or `item_name`.
        component: i32,
    },
    SetOminousBottleAmplifier(NumberProvider),
    SetPotion(String),
    SetRandomDyes(NumberProvider),
    /// `None` draws from the whole potion registry.
    SetRandomPotion(Option<HolderSet>),
    SetStewEffect(Vec<(String, NumberProvider)>),
    /// A `writable_book_content` or `written_book_content` value whose pages
    /// the operation splices into the held ones.
    SetBookPages {
        component: i32,
        value: NbtSummaryValue,
        mode: ListOperation,
    },
    ToggleTooltips(Vec<(&'static str, bool)>),
}

/// `ApplyBonusCount`'s three formula types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BonusFormula {
    OreDrops,
    UniformBonusCount { bonus_multiplier: i32 },
    BinomialWithBonusCount { extra_rounds: i32, probability: f32 },
}

/// `ExplorationMapFunction`'s fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootExplorationMap {
    pub destination: String,
    pub decoration: String,
    pub zoom: i8,
    pub search_radius: i32,
    pub skip_known_structures: bool,
}

/// `SetAttributesFunction.Modifier`.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeModifierTemplate {
    pub id: String,
    pub attribute: String,
    pub operation: String,
    pub amount: NumberProvider,
    pub slots: Vec<String>,
}

/// `DataComponentPatch.CODEC` over `components`, resolved against holder ids
/// on first use.
#[derive(Debug, Clone)]
pub struct ComponentPatchParameter {
    pub nbt: NbtSummaryValue,
    patch: Arc<OnceLock<std::result::Result<DataComponentPatchSummary, String>>>,
}

impl PartialEq for ComponentPatchParameter {
    fn eq(&self, other: &Self) -> bool {
        self.nbt == other.nbt
    }
}

impl ComponentPatchParameter {
    pub fn patch(
        &self,
        decode: impl FnOnce(&NbtSummaryValue) -> std::result::Result<DataComponentPatchSummary, String>,
    ) -> std::result::Result<&DataComponentPatchSummary, &str> {
        self.patch
            .get_or_init(|| decode(&self.nbt))
            .as_ref()
            .map_err(String::as_str)
    }
}

impl LootFunction {
    /// `LootItemFunctions.ROOT_CODEC`: an object dispatched on `function`, or a
    /// bare array as an inline `sequence`.
    pub fn decode(value: V<'_>, lookup: Option<&FrozenDatapackRegistryLookup<'_>>) -> Result<Self> {
        if value.list_len().is_some() {
            return Ok(Self {
                conditions: Vec::new(),
                kind: LootFunctionKind::Sequence(list(value, |value| Self::decode(value, lookup))?),
            });
        }
        let obj = Obj::of(value)?;
        let kind = obj.req("function", id)?;
        let kind = decode_kind(&kind, &obj, value, lookup).at_kind(&kind)?;
        let conditions = if matches!(kind, LootFunctionKind::Sequence(_)) {
            Vec::new()
        } else {
            conditions(&obj, lookup)?
        };
        Ok(Self { conditions, kind })
    }
}

trait AtKind<T> {
    fn at_kind(self, kind: &str) -> Result<T>;
}

impl<T> AtKind<T> for Result<T> {
    fn at_kind(self, kind: &str) -> Result<T> {
        self.map_err(|mut error| {
            error.message = format!("{kind}: {}", error.message);
            error
        })
    }
}

pub(super) fn functions(
    obj: &Obj<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Vec<LootFunction>> {
    obj.or("functions", Vec::new(), |value| {
        list(value, |value| LootFunction::decode(value, lookup))
    })
}

fn decode_kind(
    kind: &str,
    obj: &Obj<'_>,
    value: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootFunctionKind> {
    use LootFunctionKind as K;
    let content_registry = lookup_content_registry(lookup);
    let provider = |name| obj.req(name, NumberProvider::decode);
    let add = || obj.or_else("add", false, boolean);
    Ok(match kind {
        "minecraft:apply_bonus" => K::ApplyBonus {
            enchantment: obj.req("enchantment", id)?,
            formula: bonus_formula(obj)?,
        },
        "minecraft:copy_components" => {
            let types = |name| {
                obj.opt(name, |value| {
                    list(value, |value| {
                        let component = id(value)?;
                        vanilla_data_component_type_for_resource_id(&component)
                            .map(|entry| entry.resource_id)
                            .map_or_else(|| fail(format!("unknown component {component}")), Ok)
                    })
                })
            };
            K::CopyComponents {
                source: obj.req("source", |value| match string(value)?.as_str() {
                    "block_entity" => Ok(LootContextParam::BlockEntity),
                    "tool" => Ok(LootContextParam::Tool),
                    _ => entity_target(value),
                })?,
                include: types("include")?,
                exclude: types("exclude")?,
            }
        }
        "minecraft:copy_custom_data" => K::CopyCustomData(nbt_copy::decode(obj)?),
        "minecraft:copy_name" => K::CopyName(obj.req("source", |value| {
            if value
                .java_string_ref()
                .is_some_and(|value| value == "block_entity")
            {
                Ok(LootContextParam::BlockEntity)
            } else {
                entity_target(value)
            }
        })?),
        "minecraft:copy_state" => {
            let block = obj.req("block", |value| {
                let block = id(value)?;
                if vanilla_block_type_for_resource_id(&block).is_none()
                    && content_registry.block_by_resource_id(&block).is_none()
                {
                    return fail(format!("unknown block {block}"));
                }
                Ok(block)
            })?;
            let mod_spec = vanilla_block_type_for_resource_id(&block)
                .is_none()
                .then(|| content_registry.block_by_resource_id(&block))
                .flatten();
            let properties = obj
                .req("properties", |value| list(value, |value| string(value)))?
                .into_iter()
                .filter(|name| {
                    if let Some(spec) = mod_spec {
                        spec.state_count > 1
                            && name
                                == bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY
                    } else {
                        vanilla_block_property_values(&block, name).is_some()
                    }
                })
                .collect();
            K::CopyState { block, properties }
        }
        "minecraft:discard" => K::Discard,
        "minecraft:enchant_randomly" => K::EnchantRandomly {
            options: obj.opt("options", HolderSet::decode)?,
            only_compatible: obj.or("only_compatible", true, boolean)?,
            include_additional_cost_component: obj.or(
                "include_additional_cost_component",
                false,
                boolean,
            )?,
        },
        "minecraft:enchant_with_levels" => K::EnchantWithLevels {
            levels: provider("levels")?,
            options: obj.opt("options", HolderSet::decode)?,
            include_additional_cost_component: obj.or(
                "include_additional_cost_component",
                false,
                boolean,
            )?,
        },
        "minecraft:enchanted_count_increase" => K::EnchantedCountIncrease {
            enchantment: obj.req("enchantment", id)?,
            count: provider("count")?,
            limit: obj.or("limit", 0, int)?,
        },
        "minecraft:exploration_map" => K::ExplorationMap(exploration_map(obj)?),
        "minecraft:explosion_decay" => K::ExplosionDecay,
        "minecraft:fill_player_head" => K::FillPlayerHead(obj.req("entity", entity_target)?),
        "minecraft:filtered" => {
            let branch = |name| {
                obj.opt(name, |value| {
                    LootFunction::decode(value, lookup).map(Box::new)
                })
            };
            K::Filtered {
                item_filter: obj.req("item_filter", |value| {
                    ItemPredicate::decode_with_content_registry(value, content_registry)
                })?,
                on_pass: branch("on_pass")?,
                on_fail: branch("on_fail")?,
            }
        }
        "minecraft:furnace_smelt" => K::FurnaceSmelt {
            use_input_count: obj.or("use_input_count", true, boolean)?,
        },
        "minecraft:limit_count" => K::LimitCount(obj.req("limit", IntRange::decode)?),
        "minecraft:modify_contents" => K::ModifyContents {
            component: obj.req("component", container_component)?,
            modifier: Box::new(obj.req("modifier", |value| LootFunction::decode(value, lookup))?),
        },
        "minecraft:reference" => K::Reference(obj.req("name", id)?),
        "minecraft:sequence" => K::Sequence(obj.req("functions", |value| {
            list(value, |value| LootFunction::decode(value, lookup))
        })?),
        "minecraft:set_attributes" => K::SetAttributes {
            modifiers: obj.req("modifiers", |value| list(value, attribute_modifier))?,
            replace: obj.or("replace", true, boolean)?,
        },
        "minecraft:set_banner_pattern" => K::SetBannerPattern {
            patterns: match obj.req("patterns", |value| {
                component_parameter_with_content_registry(BANNER_PATTERNS, value, content_registry)
            })? {
                NbtSummaryValue::List(patterns) => patterns,
                _ => return fail("banner patterns are not a list").at("patterns"),
            },
            append: obj.req("append", boolean)?,
        },
        "minecraft:set_book_cover" => book_cover(obj, value)?,
        "minecraft:set_components" => K::SetComponents(ComponentPatchParameter {
            nbt: obj.req("components", |value| {
                if entries(value).is_some_and(|entries| {
                    entries
                        .iter()
                        .any(|(key, _)| key.units().iter().all(|unit| *unit == u16::from(b'!')))
                }) {
                    return fail("empty component id");
                }
                to_nbt(value)
            })?,
            patch: Arc::default(),
        }),
        "minecraft:set_contents" => K::SetContents {
            component: obj.req("component", container_component)?,
            entries: obj.req("entries", |value| {
                list(value, |value| LootEntry::decode(value, lookup))
            })?,
        },
        "minecraft:set_count" => K::SetCount {
            count: provider("count")?,
            add: add(),
        },
        "minecraft:set_custom_data" => K::SetCustomData(obj.req("tag", custom_data_tag)?),
        "minecraft:set_custom_model_data" => {
            const MAX: usize = i32::MAX as usize;
            K::SetCustomModelData {
                floats: obj.opt("floats", |value| {
                    standalone_list(value, MAX, NumberProvider::decode)
                })?,
                flags: obj.opt("flags", |value| {
                    standalone_list(value, MAX, |value| {
                        boolean(value).map(|flag| NbtSummaryValue::Byte(i8::from(flag)))
                    })
                })?,
                strings: obj.opt("strings", |value| {
                    standalone_list(value, MAX, |value| {
                        value
                            .java_string()
                            .map(NbtSummaryValue::String)
                            .map_or_else(|| fail("expected a string"), Ok)
                    })
                })?,
                colors: obj.opt("colors", |value| {
                    standalone_list(value, MAX, color_provider)
                })?,
            }
        }
        "minecraft:set_damage" => K::SetDamage {
            damage: provider("damage")?,
            add: add(),
        },
        "minecraft:set_enchantments" => K::SetEnchantments {
            enchantments: obj.or("enchantments", Vec::new(), |value| {
                Obj::of(value)?
                    .entries()
                    .into_iter()
                    .map(|(key, value)| {
                        let enchantment = bbb_protocol::normalize_resource_location(
                            key.try_to_string()
                                .or_else(|_| fail("invalid identifier"))?,
                        )
                        .or_else(|_| fail(format!("invalid enchantment `{key}`")))?;
                        Ok((enchantment, NumberProvider::decode(value).at(&key)?))
                    })
                    .collect()
            })?,
            add: add(),
        },
        "minecraft:set_firework_explosion" => firework_explosion(obj, value)?,
        "minecraft:set_fireworks" => fireworks(obj, value)?,
        "minecraft:set_instrument" => K::SetInstrument(obj.req("options", |value| {
            holder_set_parameter("minecraft:instrument", value)
        })?),
        "minecraft:set_item" => K::SetItem(obj.req("item", |value| {
            item_id_with_content_registry(value, content_registry)
        })?),
        "minecraft:set_loot_table" => {
            // The run method never reads `type`, but the codec requires one.
            obj.req("type", |value| {
                let kind = id(value)?;
                vanilla_block_entity_type_for_resource_id(&kind).map_or_else(
                    || fail(format!("unknown block entity type {kind}")),
                    |_| Ok(()),
                )
            })?;
            K::SetLootTable {
                name: obj.req("name", id)?,
                seed: obj.or("seed", 0, long)?,
            }
        }
        "minecraft:set_lore" => {
            const MAX_LINES: usize = 256;
            K::SetLore {
                lore: obj.req("lore", |value| {
                    let lines = list(value, |value| {
                        text_parameter_with_content_registry(value, content_registry)
                    })?;
                    if lines.len() > MAX_LINES {
                        return fail(format!("too many lore lines (max {MAX_LINES})"));
                    }
                    Ok(lines)
                })?,
                mode: ListOperation::decode(obj, MAX_LINES)?,
                entity: obj.opt("entity", entity_target)?,
            }
        }
        "minecraft:set_name" => K::SetName {
            name: obj.opt("name", |value| {
                text_parameter_with_content_registry(value, content_registry)
            })?,
            entity: obj.opt("entity", entity_target)?,
            component: obj.or("target", CUSTOM_NAME, |value| {
                match string(value)?.as_str() {
                    "custom_name" => Ok(CUSTOM_NAME),
                    "item_name" => Ok(ITEM_NAME),
                    other => fail(format!("unknown name target {other}")),
                }
            })?,
        },
        "minecraft:set_ominous_bottle_amplifier" => {
            K::SetOminousBottleAmplifier(provider("amplifier")?)
        }
        "minecraft:set_potion" => K::SetPotion(obj.req("id", id)?),
        "minecraft:set_random_dyes" => K::SetRandomDyes(provider("number_of_dyes")?),
        "minecraft:set_random_potion" => K::SetRandomPotion(obj.opt("options", |value| {
            holder_set_parameter("minecraft:potion", value)
        })?),
        "minecraft:set_stew_effect" => {
            K::SetStewEffect(obj.or("effects", Vec::new(), |value| {
                let mut effects: Vec<(String, NumberProvider)> = Vec::new();
                for effect in list(value, |value| {
                    let obj = Obj::of(value)?;
                    let effect = obj.req("type", id)?;
                    if bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
                        &effect,
                    )
                    .is_none()
                    {
                        return fail(format!("unknown mob effect {effect}")).at("type");
                    }
                    Ok((effect, obj.req("duration", NumberProvider::decode)?))
                })? {
                    if effects.iter().any(|(existing, _)| *existing == effect.0) {
                        return fail("Encountered duplicate mob effects");
                    }
                    effects.push(effect);
                }
                Ok(effects)
            })?)
        }
        "minecraft:set_writable_book_pages" => book_pages(obj, value, WRITABLE_BOOK, 100)?,
        "minecraft:set_written_book_pages" => {
            book_pages(obj, value, WRITTEN_BOOK, i32::MAX as usize)?
        }
        "minecraft:toggle_tooltips" => K::ToggleTooltips(obj.req("toggles", |value| {
            let entries = entries(value).map_or_else(|| fail("expected an object"), Ok)?;
            entries
                .into_iter()
                .map(|(key, shown)| {
                    let component = bbb_protocol::normalize_resource_location(
                        key.try_to_string()
                            .or_else(|_| fail("invalid identifier"))?,
                    )
                    .ok()
                    .and_then(|id| vanilla_data_component_type_for_resource_id(&id))
                    .map_or_else(|| fail(format!("unknown component {key}")), Ok)?;
                    Ok((component.resource_id, boolean(shown).at(&key)?))
                })
                .collect()
        })?),
        other => return fail(format!("unknown function type {other}")),
    })
}

fn bonus_formula(obj: &Obj<'_>) -> Result<BonusFormula> {
    let parameter = |name| obj.req("parameters", |value| Obj::of(value)?.req(name, int));
    Ok(match obj.req("formula", id)?.as_str() {
        "minecraft:ore_drops" => BonusFormula::OreDrops,
        "minecraft:uniform_bonus_count" => BonusFormula::UniformBonusCount {
            bonus_multiplier: parameter("bonusMultiplier")?,
        },
        "minecraft:binomial_with_bonus_count" => BonusFormula::BinomialWithBonusCount {
            extra_rounds: parameter("extra")?,
            probability: obj.req("parameters", |value| {
                Obj::of(value)?.req("probability", float)
            })?,
        },
        other => return fail(format!("unknown formula {other}")).at("formula"),
    })
}

fn exploration_map(obj: &Obj<'_>) -> Result<LootExplorationMap> {
    let decoration = obj.or("decoration", "minecraft:mansion".to_string(), id)?;
    if !bbb_protocol::static_registry_elements::vanilla_static_registry_elements(
        "minecraft:map_decoration_type",
    )
    .is_some_and(|values| values.contains(&decoration.as_str()))
    {
        return fail(format!("unknown map decoration {decoration}")).at("decoration");
    }
    Ok(LootExplorationMap {
        destination: obj.or("destination", "minecraft:on_treasure_maps".to_string(), id)?,
        decoration,
        zoom: obj.or("zoom", 2, |value| int(value).map(|zoom| zoom as i8))?,
        search_radius: obj.or("search_radius", 50, int)?,
        skip_known_structures: obj.or("skip_existing_chunks", true, boolean)?,
    })
}

/// `ContainerComponentManipulators.CODEC`.
pub(super) fn container_component(value: V<'_>) -> Result<i32> {
    match id(value)?.as_str() {
        "minecraft:container" => Ok(CONTAINER),
        "minecraft:bundle_contents" => Ok(BUNDLE_CONTENTS),
        "minecraft:charged_projectiles" => Ok(CHARGED_PROJECTILES),
        other => fail(format!("unknown container component {other}")),
    }
}

fn attribute_modifier(value: V<'_>) -> Result<AttributeModifierTemplate> {
    let obj = Obj::of(value)?;
    let attribute = obj.req("attribute", id)?;
    if bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id(&attribute).is_none() {
        return fail(format!("unknown attribute {attribute}")).at("attribute");
    }
    let operation = obj.req("operation", |value| {
        let operation = string(value)?;
        bbb_protocol::attribute_modifier_types::attribute_modifier_operation_id(&operation)
            .map_or_else(
                || fail(format!("unknown operation {operation}")),
                |_| Ok(operation.to_string()),
            )
    })?;
    let slot = |value| {
        let name = string(value)?;
        bbb_protocol::equipment_slots::vanilla_equipment_slot_group_for_serialized_name(&name)
            .map(|slot| slot.serialized_name.to_string())
            .map_or_else(|| fail(format!("unknown slot {name}")), Ok)
    };
    let slots = obj.req("slot", |value| {
        let slots = if value.java_string_ref().is_some() {
            vec![slot(value)?]
        } else {
            list(value, slot)?
        };
        if slots.is_empty() {
            return fail("List must have contents");
        }
        Ok(slots)
    })?;
    Ok(AttributeModifierTemplate {
        id: obj.req("id", id)?,
        attribute,
        operation,
        amount: obj.req("amount", NumberProvider::decode)?,
        slots,
    })
}

/// `SetCustomModelDataFunction.COLOR_PROVIDER_CODEC`: a provider, or an RGB
/// float triple as a constant.
fn color_provider(value: V<'_>) -> Result<NumberProvider> {
    if let Ok(provider) = NumberProvider::decode(value) {
        return Ok(provider);
    }
    let channels = list(value, float)?;
    let [red, green, blue] = channels[..] else {
        return fail("expected an RGB triple");
    };
    let channel = |value: f32| ((value * 255.0).floor() as i32) & 255;
    let rgb = -16_777_216i32 | channel(red) << 16 | channel(green) << 8 | channel(blue);
    Ok(NumberProvider::Constant(rgb as f32))
}

fn component_parameter_with_content_registry(
    type_id: i32,
    value: V<'_>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue> {
    bbb_registry_codec::decode_loot_component_parameter_source_with_content_registry(
        type_id,
        value,
        source(value),
        Default::default(),
        content_registry,
    )
    .or_else(|error| fail(error.to_string()))
}

/// `RegistryCodecs.homogeneousList` over a registry the codec checks ids in.
fn holder_set_parameter(registry: &'static str, value: V<'_>) -> Result<HolderSet> {
    let set = bbb_registry_codec::decode_loot_holder_set_parameter_source(
        registry,
        value,
        source(value),
        Default::default(),
    )
    .or_else(|error| fail(error.to_string()))?;
    let id = |value: &NbtSummaryValue| match value {
        NbtSummaryValue::String(id) => id
            .try_to_string()
            .or_else(|_| fail("holder set entry is not an identifier")),
        _ => fail("holder set entry is not an id"),
    };
    Ok(match &set {
        NbtSummaryValue::String(raw) => {
            let raw = raw
                .try_to_string()
                .or_else(|_| fail("holder set entry is not an identifier"))?;
            match raw.strip_prefix('#') {
                Some(tag) => HolderSet::Tag(tag.to_owned()),
                None => HolderSet::Ids(vec![raw]),
            }
        }
        NbtSummaryValue::List(values) => {
            HolderSet::Ids(values.iter().map(id).collect::<Result<_>>()?)
        }
        _ => return fail("holder set is neither an id nor a list"),
    })
}

fn book_component(type_id: i32, record: &Owned) -> Result<NbtSummaryValue> {
    let record = record.view();
    bbb_registry_codec::decode_loot_book_parameter_source(type_id, record, source(record))
        .or_else(|error| fail(error.to_string()))
}

/// `SetBookCoverFunction`: absent fields keep the held value's.
fn book_cover(obj: &Obj<'_>, like: V<'_>) -> Result<LootFunctionKind> {
    let mut fields = Vec::new();
    for field in ["title", "author"] {
        fields.push((
            field,
            obj.get(field)
                .map_or_else(|| Owned::string(like, ""), Owned::of),
        ));
    }
    fields.push(("resolved", Owned::boolean(like, true)));
    if let Some(generation) = obj.get("generation") {
        fields.push(("generation", Owned::of(generation)));
    }
    let value = book_component(WRITTEN_BOOK, &Owned::record(like, fields))?;
    let retain = ["title", "author", "generation", "pages", "resolved"]
        .into_iter()
        .filter(|field| !obj.has(field) || matches!(*field, "pages" | "resolved"))
        .collect();
    Ok(LootFunctionKind::SetBookCover { value, retain })
}

fn book_pages(
    obj: &Obj<'_>,
    like: V<'_>,
    component: i32,
    max_pages: usize,
) -> Result<LootFunctionKind> {
    let pages = obj.req("pages", |value| {
        value
            .list_len()
            .map_or_else(|| fail("expected a list"), |_| Ok(value))
    })?;
    let mut fields = Vec::new();
    if component == WRITTEN_BOOK {
        fields.push(("title", Owned::string(like, "")));
        fields.push(("author", Owned::string(like, "")));
    }
    fields.push(("pages", Owned::of(pages)));
    Ok(LootFunctionKind::SetBookPages {
        component,
        value: book_component(component, &Owned::record(like, fields))?,
        mode: ListOperation::decode(obj, max_pages)?,
    })
}

/// Checks a record against the component codec it stands for.
fn validate_component(type_id: i32, like: V<'_>, record: Owned) -> Result<()> {
    let component = vanilla_data_component_type(type_id).expect("generated component");
    let patch = to_nbt(Owned::record(like, vec![(component.resource_id, record)]).view())?;
    bbb_registry_codec::read_component_patch(&patch, &bbb_registry_codec::UnavailableHolderResolver)
        .map(|_| ())
        .or_else(|error| fail(error.to_string()))
}

fn fireworks(obj: &Obj<'_>, like: V<'_>) -> Result<LootFunctionKind> {
    const MAX_EXPLOSIONS: usize = 256;
    let explosions = obj.get("explosions");
    let mut record = Vec::new();
    if let Some(flight) = obj.get("flight_duration") {
        record.push(("flight_duration", Owned::of(flight)));
    }
    if let Some(explosions) = explosions {
        record.push((
            "explosions",
            Owned::of(Obj::of(explosions)?.req("values", Ok)?),
        ));
    }
    validate_component(FIREWORKS, like, Owned::record(like, record))?;
    Ok(LootFunctionKind::SetFireworks {
        // `ExtraCodecs.UNSIGNED_BYTE` narrows to a byte before the component.
        flight_duration: obj.opt("flight_duration", |value| {
            int(value).map(|value| value as i8)
        })?,
        explosions: explosions
            .map(|value| standalone_list(value, MAX_EXPLOSIONS, explosion_value))
            .transpose()
            .at("explosions")?,
    })
}

fn firework_explosion(obj: &Obj<'_>, like: V<'_>) -> Result<LootFunctionKind> {
    let mappings = [
        ("shape", "shape"),
        ("colors", "colors"),
        ("fade_colors", "fade_colors"),
        ("trail", "has_trail"),
        ("twinkle", "has_twinkle"),
    ];
    let mut record = Vec::new();
    for (parameter, component) in mappings {
        if let Some(value) = obj.get(parameter) {
            record.push((component, Owned::of(value)));
        }
    }
    if !obj.has("shape") {
        record.push((
            "shape",
            Owned::string(like, FireworkExplosionShapeSummary::SmallBall.as_str()),
        ));
    }
    let record = Owned::record(like, record);
    let NbtSummaryValue::Compound(mut fields) = explosion_value(record.view())? else {
        unreachable!("explosion values are records")
    };
    validate_component(FIREWORK_EXPLOSION, like, record)?;
    fields.retain(|entry| {
        mappings
            .iter()
            .any(|(parameter, component)| entry.name == *component && obj.has(parameter))
    });
    Ok(LootFunctionKind::SetFireworkExplosion(fields))
}

/// `FireworkExplosion.CODEC` as the component NBT it writes.
fn explosion_value(value: V<'_>) -> Result<NbtSummaryValue> {
    let obj = Obj::of(value)?;
    let mut fields = vec![NbtSummaryEntry {
        name: "shape".into(),
        value: NbtSummaryValue::string(obj.req("shape", string)?),
    }];
    for name in ["colors", "fade_colors"] {
        if let Some(colors) = obj.opt(name, |value| list(value, int))? {
            fields.push(NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::List(
                    colors.into_iter().map(NbtSummaryValue::Int).collect(),
                ),
            });
        }
    }
    for name in ["has_trail", "has_twinkle"] {
        if let Some(flag) = obj.opt(name, boolean)? {
            fields.push(NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::Byte(i8::from(flag)),
            });
        }
    }
    Ok(NbtSummaryValue::Compound(fields))
}
