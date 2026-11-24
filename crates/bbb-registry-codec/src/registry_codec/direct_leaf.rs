//! Exact synchronized codecs for registry elements without holder dependencies.

use bbb_protocol::{
    nbt_summary::{canonical_float_tag_bits, entry},
    normalize_resource_location, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use super::{
    synchronized_registry_element_codec, RegistryCodecError, RegistryElementData,
    RegistryElementValueRef,
};

const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";
const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";
const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";

const BANNER_PATTERN_CODEC: &str = "BannerPattern.DIRECT_CODEC";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageScaling {
    Never,
    WhenCausedByLivingNonPlayer,
    Always,
}

impl DamageScaling {
    fn decode(value: RegistryElementValueRef<'_>) -> Option<Self> {
        match value.java_string_ref()? {
            value if value == "never" => Some(Self::Never),
            value if value == "when_caused_by_living_non_player" => {
                Some(Self::WhenCausedByLivingNonPlayer)
            }
            value if value == "always" => Some(Self::Always),
            _ => None,
        }
    }

    fn serialized_name(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::WhenCausedByLivingNonPlayer => "when_caused_by_living_non_player",
            Self::Always => "always",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageEffects {
    Hurt,
    Thorns,
    Drowning,
    Burning,
    Poking,
    Freezing,
}

impl DamageEffects {
    #[inline]
    pub fn from_codec_name(value: &str) -> Option<Self> {
        match value {
            "hurt" => Some(Self::Hurt),
            "thorns" => Some(Self::Thorns),
            "drowning" => Some(Self::Drowning),
            "burning" => Some(Self::Burning),
            "poking" => Some(Self::Poking),
            "freezing" => Some(Self::Freezing),
            _ => None,
        }
    }

    fn decode(value: RegistryElementValueRef<'_>) -> Option<Self> {
        match value.java_string_ref()? {
            value if value == "hurt" => Some(Self::Hurt),
            value if value == "thorns" => Some(Self::Thorns),
            value if value == "drowning" => Some(Self::Drowning),
            value if value == "burning" => Some(Self::Burning),
            value if value == "poking" => Some(Self::Poking),
            value if value == "freezing" => Some(Self::Freezing),
            _ => None,
        }
    }

    fn serialized_name(self) -> &'static str {
        match self {
            Self::Hurt => "hurt",
            Self::Thorns => "thorns",
            Self::Drowning => "drowning",
            Self::Burning => "burning",
            Self::Poking => "poking",
            Self::Freezing => "freezing",
        }
    }

    /// The `SoundEvent` each constant carries.
    #[inline]
    pub fn sound_resource_id(self) -> &'static str {
        match self {
            Self::Hurt | Self::Thorns => "minecraft:entity.player.hurt",
            Self::Drowning => "minecraft:entity.player.hurt_drown",
            Self::Burning => "minecraft:entity.player.hurt_on_fire",
            Self::Poking => "minecraft:entity.player.hurt_sweet_berry_bush",
            Self::Freezing => "minecraft:entity.player.hurt_freeze",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathMessageType {
    Default,
    FallVariants,
    IntentionalGameDesign,
}

impl DeathMessageType {
    fn decode(value: RegistryElementValueRef<'_>) -> Option<Self> {
        match value.java_string_ref()? {
            value if value == "default" => Some(Self::Default),
            value if value == "fall_variants" => Some(Self::FallVariants),
            value if value == "intentional_game_design" => Some(Self::IntentionalGameDesign),
            _ => None,
        }
    }

    fn serialized_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::FallVariants => "fall_variants",
            Self::IntentionalGameDesign => "intentional_game_design",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageTypeElement {
    pub message_id: bbb_protocol::JavaString,
    pub scaling: DamageScaling,
    pub exhaustion: f32,
    pub effects: DamageEffects,
    pub death_message_type: DeathMessageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerPatternElement {
    pub asset_id: String,
    pub translation_key: bbb_protocol::JavaString,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirectLeafRegistryElement {
    DamageType(DamageTypeElement),
    BannerPattern(BannerPatternElement),
    WorldClock,
}

/// Decodes one synchronized registry element through the generated codec route.
///
/// `DamageType.DIRECT_CODEC` requires its first three fields and supplies the
/// last two defaults. `BannerPattern.DIRECT_CODEC`
/// requires both of its string-backed fields.
/// `WorldClock.DIRECT_CODEC` is a unit map codec.
pub fn decode_direct_leaf_registry_element(
    registry: &str,
    data: &RegistryElementData,
) -> Result<DirectLeafRegistryElement, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(registry, data.source)?;
    match registry {
        DAMAGE_TYPE_REGISTRY => {
            decode_damage_type(data.root(), codec).map(DirectLeafRegistryElement::DamageType)
        }
        BANNER_PATTERN_REGISTRY => {
            decode_banner_pattern(data.root(), codec).map(DirectLeafRegistryElement::BannerPattern)
        }
        WORLD_CLOCK_REGISTRY => {
            decode_world_clock(data.root(), codec)?;
            Ok(DirectLeafRegistryElement::WorldClock)
        }
        _ => Err(RegistryCodecError::coverage(
            registry,
            codec,
            "<registry>",
            "registry is not supported by the direct leaf codec",
        )),
    }
}

/// Encodes the canonical synchronized network-NBT representation.
pub fn encode_direct_leaf_registry_element_network(
    element: &DirectLeafRegistryElement,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    match element {
        DirectLeafRegistryElement::DamageType(element) => Ok(encode_damage_type(element)),
        DirectLeafRegistryElement::BannerPattern(element) => encode_banner_pattern(element),
        DirectLeafRegistryElement::WorldClock => Ok(NbtSummaryValue::Compound(Vec::new())),
    }
}

pub fn decode_damage_type(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<DamageTypeElement, RegistryCodecError> {
    require_compound(value, DAMAGE_TYPE_REGISTRY, codec)?;
    let message_id = string_field(value, "message_id", DAMAGE_TYPE_REGISTRY, codec)?.to_owned();
    let scaling_value = required_field(value, "scaling", DAMAGE_TYPE_REGISTRY, codec)?;
    let scaling = DamageScaling::decode(scaling_value).ok_or_else(|| {
        codec_error(
            DAMAGE_TYPE_REGISTRY,
            codec,
            "scaling",
            "expected one of never, when_caused_by_living_non_player, always",
        )
    })?;
    let exhaustion_value = required_field(value, "exhaustion", DAMAGE_TYPE_REGISTRY, codec)?;
    // NbtOps exposes every NumericTag as a Number and createFloat writes a
    // FloatTag.
    let exhaustion = codec_float(exhaustion_value).ok_or_else(|| {
        codec_error(
            DAMAGE_TYPE_REGISTRY,
            codec,
            "exhaustion",
            "expected a numeric NBT tag",
        )
    })?;
    let effects = match value.field("effects") {
        Some(value) => DamageEffects::decode(value).ok_or_else(|| {
            codec_error(
                DAMAGE_TYPE_REGISTRY,
                codec,
                "effects",
                "expected one of hurt, thorns, drowning, burning, poking, freezing",
            )
        })?,
        None => DamageEffects::Hurt,
    };
    let death_message_type = match value.field("death_message_type") {
        Some(value) => DeathMessageType::decode(value).ok_or_else(|| {
            codec_error(
                DAMAGE_TYPE_REGISTRY,
                codec,
                "death_message_type",
                "expected one of default, fall_variants, intentional_game_design",
            )
        })?,
        None => DeathMessageType::Default,
    };
    Ok(DamageTypeElement {
        message_id,
        scaling,
        exhaustion,
        effects,
        death_message_type,
    })
}

fn encode_damage_type(element: &DamageTypeElement) -> NbtSummaryValue {
    let mut entries = vec![
        entry(
            "message_id",
            NbtSummaryValue::String(element.message_id.clone()),
        ),
        entry(
            "scaling",
            NbtSummaryValue::String(element.scaling.serialized_name().into()),
        ),
        entry(
            "exhaustion",
            NbtSummaryValue::Float(canonical_float_tag_bits(element.exhaustion)),
        ),
    ];
    if element.effects != DamageEffects::Hurt {
        entries.push(entry(
            "effects",
            NbtSummaryValue::String(element.effects.serialized_name().into()),
        ));
    }
    if element.death_message_type != DeathMessageType::Default {
        entries.push(entry(
            "death_message_type",
            NbtSummaryValue::String(element.death_message_type.serialized_name().into()),
        ));
    }
    NbtSummaryValue::Compound(entries)
}

fn decode_banner_pattern(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<BannerPatternElement, RegistryCodecError> {
    require_compound(value, BANNER_PATTERN_REGISTRY, codec)?;
    let raw_asset_id = string_field(value, "asset_id", BANNER_PATTERN_REGISTRY, codec)?;
    let asset_id = raw_asset_id
        .with_utf8(|raw| normalize_resource_location(raw).ok())
        .flatten()
        .ok_or_else(|| {
            codec_error(
                BANNER_PATTERN_REGISTRY,
                codec,
                "asset_id",
                format!("invalid identifier `{raw_asset_id}`"),
            )
        })?;
    let translation_key = required_field(value, "translation_key", BANNER_PATTERN_REGISTRY, codec)?
        .java_string()
        .ok_or_else(|| {
            codec_error(
                BANNER_PATTERN_REGISTRY,
                codec,
                "translation_key",
                "expected a string",
            )
        })?;
    Ok(BannerPatternElement {
        asset_id,
        translation_key,
    })
}

fn encode_banner_pattern(
    element: &BannerPatternElement,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let asset_id = normalize_resource_location(element.asset_id.as_str()).map_err(|_| {
        codec_error(
            BANNER_PATTERN_REGISTRY,
            BANNER_PATTERN_CODEC,
            "asset_id",
            format!("invalid identifier `{}`", element.asset_id),
        )
    })?;
    Ok(NbtSummaryValue::Compound(vec![
        entry("asset_id", NbtSummaryValue::String(asset_id.into())),
        entry("translation_key", element.translation_key.to_nbt()),
    ]))
}

fn decode_world_clock(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<(), RegistryCodecError> {
    require_compound(value, WORLD_CLOCK_REGISTRY, codec)
}

fn require_compound(
    value: RegistryElementValueRef<'_>,
    registry: &str,
    codec: &str,
) -> Result<(), RegistryCodecError> {
    if value.is_compound() {
        Ok(())
    } else {
        Err(codec_error(
            registry,
            codec,
            "<root>",
            "expected a compound",
        ))
    }
}

fn required_field<'a>(
    value: RegistryElementValueRef<'a>,
    name: &str,
    registry: &str,
    codec: &str,
) -> Result<RegistryElementValueRef<'a>, RegistryCodecError> {
    value
        .field(name)
        .ok_or_else(|| codec_error(registry, codec, name, "required field is missing"))
}

fn string_field<'a>(
    value: RegistryElementValueRef<'a>,
    name: &str,
    registry: &str,
    codec: &str,
) -> Result<&'a bbb_protocol::JavaString, RegistryCodecError> {
    required_field(value, name, registry, codec)?
        .java_string_ref()
        .ok_or_else(|| codec_error(registry, codec, name, "expected a string"))
}

fn codec_float(value: RegistryElementValueRef<'_>) -> Option<f32> {
    value.codec_f32()
}

fn codec_error(
    registry: &str,
    codec: &str,
    field: &str,
    reason: impl Into<String>,
) -> RegistryCodecError {
    RegistryCodecError::native_semantic(registry, codec, field, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::RegistryElementSource;
    use bbb_protocol::NbtSummaryEntry;

    fn data(source: RegistryElementSource, entries: Vec<NbtSummaryEntry>) -> RegistryElementData {
        assert_eq!(source, RegistryElementSource::NetworkNbt);
        RegistryElementData::from_network_nbt(NbtSummaryValue::Compound(entries))
    }

    fn damage_data(exhaustion: NbtSummaryValue) -> RegistryElementData {
        data(
            RegistryElementSource::NetworkNbt,
            vec![
                entry(
                    "message_id",
                    NbtSummaryValue::String("test".to_string().into()),
                ),
                entry(
                    "scaling",
                    NbtSummaryValue::String("never".to_string().into()),
                ),
                entry("exhaustion", exhaustion),
            ],
        )
    }

    fn decode_damage(data: &RegistryElementData) -> DamageTypeElement {
        match decode_direct_leaf_registry_element(DAMAGE_TYPE_REGISTRY, data).unwrap() {
            DirectLeafRegistryElement::DamageType(element) => element,
            other => panic!("expected damage type, got {other:?}"),
        }
    }

    fn encoded_field<'a>(
        entries: &'a [NbtSummaryEntry],
        name: &str,
    ) -> Option<&'a NbtSummaryValue> {
        entries
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }

    #[test]
    fn damage_type_defaults_are_omitted_and_non_defaults_are_serialized() {
        let raw = bbb_protocol::JavaString::from(vec![0xd800, 0xd801, 0xfffd]);
        let raw_damage = decode_damage(&data(
            RegistryElementSource::NetworkNbt,
            vec![
                entry("message_id", NbtSummaryValue::String(raw.clone())),
                entry("scaling", NbtSummaryValue::string("never")),
                entry("exhaustion", NbtSummaryValue::Int(1)),
            ],
        ));
        assert_eq!(raw_damage.message_id, raw);
        let NbtSummaryValue::Compound(encoded) = encode_damage_type(&raw_damage) else {
            panic!("expected compound");
        };
        assert_eq!(encoded_field(&encoded, "message_id"), Some(&raw.to_nbt()));

        let defaults = decode_damage(&damage_data(NbtSummaryValue::Int(1)));
        assert_eq!(defaults.effects, DamageEffects::Hurt);
        assert_eq!(defaults.death_message_type, DeathMessageType::Default);
        let encoded = encode_direct_leaf_registry_element_network(
            &DirectLeafRegistryElement::DamageType(defaults),
        )
        .unwrap();
        let NbtSummaryValue::Compound(entries) = encoded else {
            panic!("expected compound")
        };
        assert_eq!(encoded_field(&entries, "effects"), None);
        assert_eq!(encoded_field(&entries, "death_message_type"), None);

        let non_default = decode_damage(&data(
            RegistryElementSource::NetworkNbt,
            vec![
                entry(
                    "message_id",
                    NbtSummaryValue::String("test".to_string().into()),
                ),
                entry(
                    "scaling",
                    NbtSummaryValue::String("always".to_string().into()),
                ),
                entry("exhaustion", NbtSummaryValue::Float(0)),
                entry(
                    "effects",
                    NbtSummaryValue::String("freezing".to_string().into()),
                ),
                entry(
                    "death_message_type",
                    NbtSummaryValue::String("intentional_game_design".to_string().into()),
                ),
            ],
        ));
        assert_eq!(non_default.scaling, DamageScaling::Always);
        assert_eq!(non_default.effects, DamageEffects::Freezing);
        assert_eq!(
            non_default.death_message_type,
            DeathMessageType::IntentionalGameDesign
        );
        let encoded = encode_direct_leaf_registry_element_network(
            &DirectLeafRegistryElement::DamageType(non_default),
        )
        .unwrap();
        let NbtSummaryValue::Compound(entries) = encoded else {
            panic!("expected compound")
        };
        assert_eq!(
            encoded_field(&entries, "effects"),
            Some(&NbtSummaryValue::String("freezing".to_string().into()))
        );
        assert_eq!(
            encoded_field(&entries, "death_message_type"),
            Some(&NbtSummaryValue::String(
                "intentional_game_design".to_string().into()
            ))
        );
    }

    #[test]
    fn banner_pattern_canonicalizes_identifiers_and_accepts_arbitrary_translation_keys() {
        for (raw, expected) in [
            ("base", "minecraft:base"),
            ("example:patterns/stripe", "example:patterns/stripe"),
        ] {
            let decoded = decode_direct_leaf_registry_element(
                BANNER_PATTERN_REGISTRY,
                &data(
                    RegistryElementSource::NetworkNbt,
                    vec![
                        entry("asset_id", NbtSummaryValue::String(raw.to_string().into())),
                        entry(
                            "translation_key",
                            NbtSummaryValue::String("Any key: ☃".to_string().into()),
                        ),
                    ],
                ),
            )
            .unwrap();
            let DirectLeafRegistryElement::BannerPattern(element) = decoded else {
                panic!("expected banner pattern")
            };
            assert_eq!(element.asset_id, expected);
            assert_eq!(element.translation_key, "Any key: ☃");
            let encoded = encode_direct_leaf_registry_element_network(
                &DirectLeafRegistryElement::BannerPattern(element),
            )
            .unwrap();
            let NbtSummaryValue::Compound(entries) = encoded else {
                panic!("expected compound")
            };
            assert_eq!(
                encoded_field(&entries, "asset_id"),
                Some(&NbtSummaryValue::String(expected.to_string().into()))
            );
        }
    }

    #[test]
    fn world_clock_accepts_any_compound_and_encodes_empty_compound() {
        let decoded = decode_direct_leaf_registry_element(
            WORLD_CLOCK_REGISTRY,
            &data(
                RegistryElementSource::NetworkNbt,
                vec![entry("ignored", NbtSummaryValue::List(vec![]))],
            ),
        )
        .unwrap();
        assert_eq!(decoded, DirectLeafRegistryElement::WorldClock);
        assert_eq!(
            encode_direct_leaf_registry_element_network(&decoded).unwrap(),
            NbtSummaryValue::Compound(Vec::new())
        );

        let error = decode_direct_leaf_registry_element(
            WORLD_CLOCK_REGISTRY,
            &RegistryElementData::from_network_nbt(NbtSummaryValue::String(
                "not a map".to_string().into(),
            )),
        )
        .unwrap_err();
        assert_eq!(error.field, "<root>");
    }
}
