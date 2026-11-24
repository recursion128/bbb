//! Configured carvers and the inline holders of biome generation settings.

use std::sync::Arc;

use bbb_registry_codec::RegistryJsonValue as Json;

use super::biome::BiomeFeatures;
use super::block::HolderSet;
use super::feature::positive_int;
use super::json::{self, fail, At, Obj, Result};
use super::value::{self, FloatProvider, HeightProvider, VerticalAnchor};
use super::Refs;

pub const CONFIGURED_CARVER_REGISTRY: &str = "minecraft:worldgen/configured_carver";

/// `ConfiguredWorldCarver.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfiguredCarver {
    pub kind: CarverKind,
    pub probability: f32,
    pub y: HeightProvider,
    pub y_scale: FloatProvider,
    pub lava_level: VerticalAnchor,
    pub debug_settings: Option<CarverDebugSettings>,
    pub replaceable: HolderSet,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CarverKind {
    Cave {
        nether: bool,
        horizontal_radius_multiplier: FloatProvider,
        vertical_radius_multiplier: FloatProvider,
        floor_level: FloatProvider,
    },
    Canyon {
        vertical_rotation: FloatProvider,
        shape: CanyonShape,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanyonShape {
    pub distance_factor: FloatProvider,
    pub thickness: FloatProvider,
    pub width_smoothness: i32,
    pub horizontal_radius_factor: FloatProvider,
    pub vertical_radius_default_factor: f32,
    pub vertical_radius_center_factor: f32,
}

/// `CarverDebugSettings`; absent states keep `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarverDebugSettings {
    pub debug_mode: bool,
    pub air_state: Option<i32>,
    pub water_state: Option<i32>,
    pub lava_state: Option<i32>,
    pub barrier_state: Option<i32>,
}

impl ConfiguredCarver {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let type_id = obj.type_id("type")?;
        obj.req("config", |config| Self::decode_config(&type_id, config))
    }

    fn decode_config(type_id: &str, config: &Json) -> Result<Self> {
        let c = Obj::of(config)?;
        let any_float = |name| c.req(name, value::decode_float_provider);
        let kind = match type_id {
            "minecraft:cave" | "minecraft:nether_cave" => CarverKind::Cave {
                nether: type_id == "minecraft:nether_cave",
                horizontal_radius_multiplier: any_float("horizontal_radius_multiplier")?,
                vertical_radius_multiplier: any_float("vertical_radius_multiplier")?,
                floor_level: c.req("floor_level", |v| {
                    value::decode_float_provider_in(v, -1.0, 1.0)
                })?,
            },
            "minecraft:canyon" => CarverKind::Canyon {
                vertical_rotation: any_float("vertical_rotation")?,
                shape: c.req("shape", |shape| {
                    let s = Obj::of(shape)?;
                    Ok(CanyonShape {
                        distance_factor: s.req("distance_factor", value::decode_float_provider)?,
                        thickness: s.req("thickness", value::decode_float_provider)?,
                        width_smoothness: s.req("width_smoothness", positive_int)?,
                        horizontal_radius_factor: s
                            .req("horizontal_radius_factor", value::decode_float_provider)?,
                        vertical_radius_default_factor: s
                            .req("vertical_radius_default_factor", json::float)?,
                        vertical_radius_center_factor: s
                            .req("vertical_radius_center_factor", json::float)?,
                    })
                })?,
            },
            other => return fail(format!("unknown carver type `{other}`")).at("type"),
        };
        Ok(Self {
            kind,
            probability: c.req("probability", |v| json::float_in(v, 0.0, 1.0))?,
            y: c.req("y", HeightProvider::decode)?,
            y_scale: c.req("yScale", value::decode_float_provider)?,
            lava_level: c.req("lava_level", VerticalAnchor::decode)?,
            debug_settings: c.opt("debug_settings", |debug| {
                let d = Obj::of(debug)?;
                let state = |name| d.opt(name, value::block_state);
                Ok(CarverDebugSettings {
                    debug_mode: d.or("debug_mode", false, json::boolean)?,
                    air_state: state("air_state")?,
                    water_state: state("water_state")?,
                    lava_state: state("lava_state")?,
                    barrier_state: state("barrier_state")?,
                })
            })?,
            replaceable: c.req("replaceable", HolderSet::blocks)?,
        })
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        self.replaceable.refs("minecraft:block", refs);
    }
}

/// One carver entry in a biome's `carvers` holder set.
#[derive(Debug, Clone, PartialEq)]
pub enum BiomeCarverHolder {
    Named(String),
    Tag(String),
    Inline(Arc<ConfiguredCarver>),
}

/// Decoded `carvers` holder set of one biome.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BiomeCarvers {
    pub holders: Vec<BiomeCarverHolder>,
}

impl BiomeCarvers {
    /// Decodes a biome's `carvers` field, returning `Ok(None)` when the field is absent.
    pub fn decode_optional(biome: &Json) -> Result<Option<Self>> {
        let obj = Obj::of(biome)?;
        match obj.get("carvers") {
            Some(carvers) => Self::decode_value(carvers).at("carvers").map(Some),
            None => Ok(None),
        }
    }

    /// Decodes a biome's `carvers` holder-set value: a `#tag`, one holder, or a homogeneous list.
    pub fn decode_value(value: &Json) -> Result<Self> {
        let holders = match value {
            Json::String(_) => {
                let raw = json::string(value)?;
                match raw.strip_prefix('#') {
                    Some(tag) => vec![BiomeCarverHolder::Tag(json::id(&Json::String(tag.into()))?)],
                    None => vec![BiomeCarverHolder::Named(json::id(value)?)],
                }
            }
            Json::Object(_) => vec![BiomeCarverHolder::Inline(Arc::new(
                ConfiguredCarver::decode(value)?,
            ))],
            Json::Array(items) => {
                let mut kind = None;
                let mut out = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    let holder = match item {
                        Json::String(_) => {
                            let raw = json::string(item).at_index(index)?;
                            if raw.starts_with('#') {
                                return fail("must list named or inline configured carvers")
                                    .at_index(index);
                            }
                            BiomeCarverHolder::Named(json::id(item).at_index(index)?)
                        }
                        Json::Object(_) => BiomeCarverHolder::Inline(Arc::new(
                            ConfiguredCarver::decode(item).at_index(index)?,
                        )),
                        _ => {
                            return fail("must list named or inline configured carvers")
                                .at_index(index)
                        }
                    };
                    let is_named = matches!(holder, BiomeCarverHolder::Named(_));
                    if kind.is_some_and(|expected| expected != is_named) {
                        return fail("direct holder list must be homogeneous");
                    }
                    kind = Some(is_named);
                    out.push(holder);
                }
                out
            }
            _ => return fail("must be a configured-carver holder set"),
        };
        Ok(Self { holders })
    }

    pub(crate) fn inline_refs(&self, refs: &mut Refs) {
        for holder in &self.holders {
            if let BiomeCarverHolder::Inline(carver) = holder {
                carver.refs(refs);
            }
        }
    }
}

/// Decodes the inline carvers and placed features of a biome's generation
/// settings and collects their references; named and tag holders are
/// checked by the biome codec itself.
pub fn biome_inline_generation_refs(biome: &Json, features: &BiomeFeatures) -> Result<Refs> {
    let obj = Obj::of(biome)?;
    let mut refs = Refs::default();
    if let Some(carvers) = obj.get("carvers") {
        inline_holders(carvers, &mut refs, |value, refs| {
            ConfiguredCarver::decode(value)?.refs(refs);
            Ok(())
        })
        .at("carvers")?;
    }
    features.refs(&mut refs);
    Ok(refs)
}

fn inline_holders(
    value: &Json,
    refs: &mut Refs,
    mut decode: impl FnMut(&Json, &mut Refs) -> Result<()>,
) -> Result<()> {
    match value {
        Json::Object(_) => decode(value, refs),
        Json::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                if let Json::Object(_) = item {
                    decode(item, refs).at_index(index)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_registry_codec::registry_codec::RegistryReferenceKind;

    fn biome(carver_config: &str) -> Json {
        let text = format!(
            r#"{{"carvers":[{{"type":"minecraft:cave","config":{carver_config}}}],
                "features":[[{{"feature":{{"type":"minecraft:no_op","config":{{}}}},"placement":[]}}]]}}"#
        );
        bbb_registry_codec::parse_registry_json(text.as_bytes()).unwrap()
    }

    #[test]
    fn biome_inline_holders_decode_and_reject_illegal_content() {
        let valid = biome(
            r##"{"floor_level":0.0,"vertical_radius_multiplier":1.0,
                "horizontal_radius_multiplier":1.0,
                "replaceable":"#minecraft:overworld_carver_replaceables",
                "lava_level":{"above_bottom":8},"yScale":1.0,"y":{"absolute":0},
                "probability":0.15}"##,
        );
        assert_eq!(
            biome_inline_generation_refs(&valid, &BiomeFeatures::decode(&valid).unwrap())
                .unwrap()
                .items,
            [(
                "minecraft:block",
                RegistryReferenceKind::Tag,
                "minecraft:overworld_carver_replaceables".to_string()
            )]
        );
        assert!(biome_inline_generation_refs(
            &biome("{}"),
            &BiomeFeatures::decode(&valid).unwrap()
        )
        .is_err());
    }
}
