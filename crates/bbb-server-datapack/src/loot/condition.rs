//! `LootItemCondition`: the 26.1 condition types, decoded once.

use crate::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_facts::activity_types::{vanilla_activity_for_resource_id, ActivityCatalogEntry};
use bbb_facts::environment_attribute_types::{
    vanilla_environment_attribute, VanillaEnvironmentAttribute, VanillaEnvironmentAttributeRange,
    VanillaEnvironmentAttributeType, VanillaEnvironmentScalarValue,
};
use bbb_protocol::block_state_types::vanilla_block_property_values;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::environment_audio::{AmbientSounds, BackgroundMusic};
use bbb_protocol::environment_particles::{AmbientParticle, ParticleOptions};
use bbb_registry_codec::loot_context_types::VanillaLootContextParam as LootContextParam;
use bbb_registry_codec::registry_codec::environment_source::audio::{
    ambient_sounds_from_source, background_music_from_source,
};
use bbb_registry_codec::registry_codec::environment_source::particles::{
    ambient_particles_from_source, particle_from_source,
};

use super::codec::{boolean, fail, float, id, int, list, long, At, Obj, Result, V};
use super::number::{entity_target, level_based_value, IntRange, NumberProvider};
use super::predicate::{
    DamageSourcePredicate, EntityPredicate, ItemPredicate, LocationPredicate,
    StatePropertiesPredicate,
};
use bbb_registry_codec::level_based_value::LevelBasedValue;
use bbb_server_types::environment::{BedRule, MoonPhase, TriState};

#[derive(Debug, Clone, PartialEq)]
pub enum EnvironmentAttributeExpectedValue {
    Scalar(VanillaEnvironmentScalarValue),
    Integer(i32),
    Color(u32),
    Activity(&'static ActivityCatalogEntry),
    MoonPhase(MoonPhase),
    TriState(TriState),
    BedRule(BedRule),
    BackgroundMusic(BackgroundMusic),
    AmbientSounds(AmbientSounds),
    Particle(ParticleOptions),
    AmbientParticles(Vec<AmbientParticle>),
}

/// `LootItemCondition.DIRECT_CODEC`: an object dispatched on `condition`, or a
/// bare array as an inline `all_of`.
#[derive(Debug, Clone, PartialEq)]
pub enum LootCondition {
    AllOf(Vec<LootCondition>),
    AnyOf(Vec<LootCondition>),
    BlockStateProperty {
        block: String,
        properties: Option<StatePropertiesPredicate>,
    },
    DamageSourceProperties(Option<Box<DamageSourcePredicate>>),
    EnchantmentActiveCheck(bool),
    EntityProperties {
        entity: LootContextParam,
        predicate: Option<Box<EntityPredicate>>,
    },
    EntityScores {
        entity: LootContextParam,
        scores: Vec<(bbb_protocol::JavaString, IntRange)>,
    },
    EnvironmentAttributeCheck {
        attribute: &'static VanillaEnvironmentAttribute,
        value: EnvironmentAttributeExpectedValue,
    },
    Inverted(Box<LootCondition>),
    KilledByPlayer,
    LocationCheck {
        predicate: Option<LocationPredicate>,
        offset: [i32; 3],
    },
    MatchTool(Option<ItemPredicate>),
    RandomChance(NumberProvider),
    RandomChanceWithEnchantedBonus {
        unenchanted_chance: f32,
        enchanted_chance: LevelBasedValue,
        enchantment: String,
    },
    Reference(String),
    SurvivesExplosion,
    TableBonus {
        enchantment: String,
        chances: Vec<f32>,
    },
    TimeCheck {
        clock: String,
        period: Option<i64>,
        value: IntRange,
    },
    ValueCheck {
        value: NumberProvider,
        range: IntRange,
    },
    WeatherCheck {
        raining: Option<bool>,
        thundering: Option<bool>,
    },
}

impl LootCondition {
    pub fn decode(value: V<'_>, lookup: Option<&FrozenDatapackRegistryLookup<'_>>) -> Result<Self> {
        if value.list_len().is_some() {
            return list(value, |value| Self::decode(value, lookup)).map(Self::AllOf);
        }
        let obj = Obj::of(value)?;
        let terms = || {
            obj.req("terms", |value| {
                list(value, |value| Self::decode(value, lookup))
            })
        };
        let content_registry = crate::loot::lookup_content_registry(lookup);
        Ok(match obj.req("condition", id)?.as_str() {
            "minecraft:all_of" => Self::AllOf(terms()?),
            "minecraft:any_of" => Self::AnyOf(terms()?),
            "minecraft:block_state_property" => {
                let block = obj.req("block", |value| {
                    let block = id(value)?;
                    if vanilla_block_type_for_resource_id(&block).is_none()
                        && content_registry.block_by_resource_id(&block).is_none()
                    {
                        return fail(format!("unknown block {block}"));
                    }
                    Ok(block)
                })?;
                let properties = obj.opt("properties", StatePropertiesPredicate::decode)?;
                // `LootItemBlockStatePropertyCondition.validate`.
                let mod_spec = vanilla_block_type_for_resource_id(&block)
                    .is_none()
                    .then(|| content_registry.block_by_resource_id(&block))
                    .flatten();
                if let Some((name, _)) = properties.iter().flat_map(|p| &p.0).find(|(name, _)| {
                    !name
                        .with_utf8(|name| {
                            if let Some(spec) = mod_spec {
                                spec.state_count > 1
                                    && name
                                        == bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY
                            } else {
                                vanilla_block_property_values(&block, name).is_some()
                            }
                        })
                        .unwrap_or(false)
                }) {
                    return fail(format!("Block {block} has no property {name}"));
                }
                Self::BlockStateProperty { block, properties }
            }
            "minecraft:damage_source_properties" => {
                Self::DamageSourceProperties(obj.opt("predicate", |value| {
                    DamageSourcePredicate::decode_with_content_registry(value, content_registry)
                        .map(Box::new)
                })?)
            }
            "minecraft:enchantment_active_check" => {
                Self::EnchantmentActiveCheck(obj.req("active", boolean)?)
            }
            "minecraft:entity_properties" => Self::EntityProperties {
                entity: obj.req("entity", entity_target)?,
                predicate: obj.opt("predicate", |value| {
                    EntityPredicate::decode_with_content_registry(value, content_registry)
                        .map(Box::new)
                })?,
            },
            "minecraft:entity_scores" => Self::EntityScores {
                entity: obj.req("entity", entity_target)?,
                scores: obj.req("scores", |value| {
                    Obj::of(value)?
                        .entries()
                        .into_iter()
                        .map(|(name, range)| Ok((name.clone(), IntRange::decode(range).at(&name)?)))
                        .collect()
                })?,
            },
            "minecraft:environment_attribute_check" => {
                let attribute = obj.req("attribute", |value| {
                    let id = id(value)?;
                    vanilla_environment_attribute(&id)
                        .map_or_else(|| fail(format!("unknown environment attribute {id}")), Ok)
                })?;
                let value = obj.req("value", |value| {
                    decode_environment_attribute_value(attribute, value, lookup)
                })?;
                Self::EnvironmentAttributeCheck { attribute, value }
            }
            "minecraft:inverted" => Self::Inverted(Box::new(
                obj.req("term", |value| Self::decode(value, lookup))?,
            )),
            "minecraft:killed_by_player" => Self::KilledByPlayer,
            "minecraft:location_check" => {
                let offset = |name| obj.or(name, 0, int);
                Self::LocationCheck {
                    predicate: obj.opt("predicate", |value| {
                        LocationPredicate::decode_with_content_registry(value, content_registry)
                    })?,
                    offset: [offset("offsetX")?, offset("offsetY")?, offset("offsetZ")?],
                }
            }
            "minecraft:match_tool" => Self::MatchTool(obj.opt("predicate", |value| {
                ItemPredicate::decode_with_content_registry(value, content_registry)
            })?),
            "minecraft:random_chance" => {
                Self::RandomChance(obj.req("chance", NumberProvider::decode)?)
            }
            "minecraft:random_chance_with_enchanted_bonus" => {
                Self::RandomChanceWithEnchantedBonus {
                    unenchanted_chance: obj.req("unenchanted_chance", |value| {
                        let chance = float(value)?;
                        if !(0.0..=1.0).contains(&chance) {
                            return fail(format!("Value {chance} outside of range [0.0:1.0]"));
                        }
                        Ok(chance)
                    })?,
                    enchanted_chance: obj.req("enchanted_chance", level_based_value)?,
                    enchantment: obj.req("enchantment", id)?,
                }
            }
            "minecraft:reference" => Self::Reference(obj.req("name", id)?),
            "minecraft:survives_explosion" => Self::SurvivesExplosion,
            "minecraft:table_bonus" => Self::TableBonus {
                enchantment: obj.req("enchantment", id)?,
                chances: obj.req("chances", |value| {
                    let chances = list(value, float)?;
                    if chances.is_empty() {
                        return fail("List must have contents");
                    }
                    Ok(chances)
                })?,
            },
            "minecraft:time_check" => Self::TimeCheck {
                clock: obj.req("clock", id)?,
                period: obj.opt("period", long)?,
                value: obj.req("value", IntRange::decode)?,
            },
            "minecraft:value_check" => Self::ValueCheck {
                value: obj.req("value", NumberProvider::decode)?,
                range: obj.req("range", IntRange::decode)?,
            },
            "minecraft:weather_check" => Self::WeatherCheck {
                raining: obj.opt("raining", boolean)?,
                thundering: obj.opt("thundering", boolean)?,
            },
            other => return fail(format!("unknown condition type {other}")).at("condition"),
        })
    }
}

/// `{"condition": ..}` lists under `conditions`, absent as empty.
pub(super) fn conditions(
    obj: &Obj<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Vec<LootCondition>> {
    obj.or("conditions", Vec::new(), |value| {
        list(value, |value| LootCondition::decode(value, lookup))
    })
}

pub(super) fn decode_environment_attribute_value(
    attribute: &'static VanillaEnvironmentAttribute,
    value: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<EnvironmentAttributeExpectedValue> {
    let scalar = match attribute.value_type {
        VanillaEnvironmentAttributeType::Boolean => {
            VanillaEnvironmentScalarValue::Boolean(boolean(value)?)
        }
        VanillaEnvironmentAttributeType::Float | VanillaEnvironmentAttributeType::AngleDegrees => {
            let value = float(value)?;
            if !attribute.value_range.accepts_float(value) {
                return fail(format!(
                    "value outside environment attribute range: {}",
                    attribute.resource_id
                ));
            }
            VanillaEnvironmentScalarValue::FloatBits(value.to_bits())
        }
        VanillaEnvironmentAttributeType::Integer => {
            let parsed = int(value)?;
            let in_range = match attribute.value_range {
                VanillaEnvironmentAttributeRange::Any => true,
                VanillaEnvironmentAttributeRange::NonNegativeFloat => parsed >= 0,
                VanillaEnvironmentAttributeRange::SkyLight => (0..=15).contains(&parsed),
                VanillaEnvironmentAttributeRange::UnitFloat => (0..=1).contains(&parsed),
            };
            if !in_range {
                return fail(format!(
                    "value outside environment attribute range: {}",
                    attribute.resource_id
                ));
            }
            return Ok(EnvironmentAttributeExpectedValue::Integer(parsed));
        }
        VanillaEnvironmentAttributeType::RgbColor | VanillaEnvironmentAttributeType::ArgbColor => {
            return bbb_registry_codec::registry_codec::environment_source::color::color_from_source(
                value,
                super::codec::source(value),
                Default::default(),
                attribute,
            )
            .map(EnvironmentAttributeExpectedValue::Color)
            .or_else(|error| fail(error.to_string()));
        }
        VanillaEnvironmentAttributeType::Activity => {
            let activity = id(value)?;
            return vanilla_activity_for_resource_id(&activity)
                .map(EnvironmentAttributeExpectedValue::Activity)
                .map_or_else(|| fail(format!("unknown activity {activity}")), Ok);
        }
        VanillaEnvironmentAttributeType::BedRule => {
            return crate::environment_attributes::discrete::rule_from_source(
                value,
                super::codec::source(value),
            )
            .map(EnvironmentAttributeExpectedValue::BedRule)
            .or_else(|error| fail(error.to_string()));
        }
        VanillaEnvironmentAttributeType::BackgroundMusic => {
            return background_music_from_source(
                value,
                super::codec::source(value),
                Default::default(),
            )
            .map(EnvironmentAttributeExpectedValue::BackgroundMusic)
            .or_else(|error| fail(error.to_string()));
        }
        VanillaEnvironmentAttributeType::AmbientSounds => {
            return ambient_sounds_from_source(
                value,
                super::codec::source(value),
                Default::default(),
            )
            .map(EnvironmentAttributeExpectedValue::AmbientSounds)
            .or_else(|error| fail(error.to_string()));
        }
        VanillaEnvironmentAttributeType::Particle => {
            return particle_from_source(
                value,
                super::codec::source(value),
                Default::default(),
                lookup,
            )
            .map(EnvironmentAttributeExpectedValue::Particle)
            .or_else(|error| fail(error.to_string()));
        }
        VanillaEnvironmentAttributeType::AmbientParticles => {
            return ambient_particles_from_source(
                value,
                super::codec::source(value),
                Default::default(),
                lookup,
            )
            .map(EnvironmentAttributeExpectedValue::AmbientParticles)
            .or_else(|error| fail(error.to_string()));
        }
        VanillaEnvironmentAttributeType::TriState => {
            return value
                .as_bool()
                .map(TriState::from_bool)
                .or_else(|| {
                    value
                        .utf8_string()
                        .as_deref()
                        .and_then(TriState::from_serialized_name)
                })
                .map(EnvironmentAttributeExpectedValue::TriState)
                .map_or_else(|| fail("invalid tri state"), Ok);
        }
        VanillaEnvironmentAttributeType::MoonPhase => {
            let phase = super::codec::string(value)?;
            return MoonPhase::from_serialized_name(&phase)
                .map(EnvironmentAttributeExpectedValue::MoonPhase)
                .map_or_else(|| fail(format!("unknown moon phase {phase}")), Ok);
        }
    };
    Ok(EnvironmentAttributeExpectedValue::Scalar(scalar))
}
