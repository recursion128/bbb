//! Exact persistent and network codecs for synchronized animal sound variants.

#[cfg(test)]
use bbb_protocol::NbtSummaryEntry;
use bbb_protocol::{
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::{
    synchronized_registry_element_codec, RegistryCodecError, RegistryCodecReference,
    RegistryElementSource, RegistryElementValueRef, RegistryReferenceKind,
    RegistryResourceIdentifier,
};

const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";
const WOLF_REGISTRY: &str = "minecraft:wolf_sound_variant";
const PIG_REGISTRY: &str = "minecraft:pig_sound_variant";
const CAT_REGISTRY: &str = "minecraft:cat_sound_variant";
const COW_REGISTRY: &str = "minecraft:cow_sound_variant";
const CHICKEN_REGISTRY: &str = "minecraft:chicken_sound_variant";

/// The exact IEEE-754 bits produced by `Codec.FLOAT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecFloat(pub u32);

impl CodecFloat {
    pub fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }

    pub fn to_f32(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundEventElement {
    pub sound_id: RegistryResourceIdentifier,
    pub range: Option<CodecFloat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundEventHolder {
    Named(RegistryResourceIdentifier),
    Inline(SoundEventElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfSoundSet {
    pub ambient_sound: SoundEventHolder,
    pub death_sound: SoundEventHolder,
    pub growl_sound: SoundEventHolder,
    pub hurt_sound: SoundEventHolder,
    pub pant_sound: SoundEventHolder,
    pub whine_sound: SoundEventHolder,
    pub step_sound: SoundEventHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfSoundVariantElement {
    pub adult_sounds: Box<WolfSoundSet>,
    pub baby_sounds: Box<WolfSoundSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PigSoundSet {
    pub ambient_sound: SoundEventHolder,
    pub hurt_sound: SoundEventHolder,
    pub death_sound: SoundEventHolder,
    pub step_sound: SoundEventHolder,
    pub eat_sound: SoundEventHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PigSoundVariantElement {
    pub adult_sounds: Box<PigSoundSet>,
    pub baby_sounds: Box<PigSoundSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatSoundSet {
    pub ambient_sound: SoundEventHolder,
    pub stray_ambient_sound: SoundEventHolder,
    pub hiss_sound: SoundEventHolder,
    pub hurt_sound: SoundEventHolder,
    pub death_sound: SoundEventHolder,
    pub eat_sound: SoundEventHolder,
    pub beg_for_food_sound: SoundEventHolder,
    pub purr_sound: SoundEventHolder,
    pub purreow_sound: SoundEventHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatSoundVariantElement {
    pub adult_sounds: Box<CatSoundSet>,
    pub baby_sounds: Box<CatSoundSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CowSoundVariantElement {
    pub ambient_sound: SoundEventHolder,
    pub hurt_sound: SoundEventHolder,
    pub death_sound: SoundEventHolder,
    pub step_sound: SoundEventHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChickenSoundSet {
    pub ambient_sound: SoundEventHolder,
    pub hurt_sound: SoundEventHolder,
    pub death_sound: SoundEventHolder,
    pub step_sound: SoundEventHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChickenSoundVariantElement {
    pub adult_sounds: Box<ChickenSoundSet>,
    pub baby_sounds: Box<ChickenSoundSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundVariantElement {
    Wolf(WolfSoundVariantElement),
    Pig(PigSoundVariantElement),
    Cat(CatSoundVariantElement),
    Cow(CowSoundVariantElement),
    Chicken(ChickenSoundVariantElement),
}

/// Decodes one animal sound variant with the generated codec for its origin.
///
/// Each sound holder accepts either a named identifier or an inline
/// `SoundEvent`. Named lookup deliberately remains a later frozen-registry
/// responsibility.
pub fn decode_sound_variant_element(
    registry: &str,
    source: RegistryElementSource,
    root: RegistryElementValueRef<'_>,
) -> Result<SoundVariantElement, RegistryCodecError> {
    // Route lookup precedes all element inspection, making unsupported
    // registries fail closed at `<registry>`.
    let codec = synchronized_registry_element_codec(registry, source)?;
    match registry {
        WOLF_REGISTRY => decode_wolf(root, codec).map(SoundVariantElement::Wolf),
        PIG_REGISTRY => decode_pig(root, codec).map(SoundVariantElement::Pig),
        CAT_REGISTRY => decode_cat(root, codec).map(SoundVariantElement::Cat),
        COW_REGISTRY => decode_cow(root, codec).map(SoundVariantElement::Cow),
        CHICKEN_REGISTRY => decode_chicken(root, codec).map(SoundVariantElement::Chicken),
        _ => Err(RegistryCodecError::coverage(
            registry,
            codec,
            "<registry>",
            "registry is not supported by the sound variant codec",
        )),
    }
}

/// Encodes the canonical synchronized network-NBT representation.
pub fn encode_sound_variant_network(
    element: &SoundVariantElement,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let (registry, value) = match element {
        SoundVariantElement::Wolf(value) => (WOLF_REGISTRY, encode_wolf(value)),
        SoundVariantElement::Pig(value) => (PIG_REGISTRY, encode_pig(value)),
        SoundVariantElement::Cat(value) => (CAT_REGISTRY, encode_cat(value)),
        SoundVariantElement::Cow(value) => (COW_REGISTRY, encode_cow(value)),
        SoundVariantElement::Chicken(value) => (CHICKEN_REGISTRY, encode_chicken(value)),
    };
    // Cow/chicken intentionally use their generated `DIRECT_CODEC` network route.
    synchronized_registry_element_codec(registry, RegistryElementSource::NetworkNbt)?;
    Ok(value)
}

pub(crate) fn sound_variant_registry_codec_references(
    element: &SoundVariantElement,
) -> Vec<RegistryCodecReference<'_>> {
    let mut references = Vec::new();
    match element {
        SoundVariantElement::Wolf(value) => {
            wolf_references(&value.adult_sounds, &mut references);
            wolf_references(&value.baby_sounds, &mut references);
        }
        SoundVariantElement::Pig(value) => {
            pig_references(&value.adult_sounds, &mut references);
            pig_references(&value.baby_sounds, &mut references);
        }
        SoundVariantElement::Cat(value) => {
            cat_references(&value.adult_sounds, &mut references);
            cat_references(&value.baby_sounds, &mut references);
        }
        SoundVariantElement::Cow(value) => cow_references(value, &mut references),
        SoundVariantElement::Chicken(value) => {
            chicken_references(&value.adult_sounds, &mut references);
            chicken_references(&value.baby_sounds, &mut references);
        }
    }
    references
        .into_iter()
        .map(|resource_id| RegistryCodecReference {
            target_registry: SOUND_EVENT_REGISTRY,
            kind: RegistryReferenceKind::Element,
            resource_id,
        })
        .collect()
}

fn decode_wolf(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<WolfSoundVariantElement, RegistryCodecError> {
    let root = compound(value, WOLF_REGISTRY, codec, "<root>")?;
    Ok(WolfSoundVariantElement {
        adult_sounds: Box::new(decode_wolf_set(
            required(root, "adult_sounds", WOLF_REGISTRY, codec, "adult_sounds")?,
            codec,
            "adult_sounds",
        )?),
        baby_sounds: Box::new(decode_wolf_set(
            required(root, "baby_sounds", WOLF_REGISTRY, codec, "baby_sounds")?,
            codec,
            "baby_sounds",
        )?),
    })
}

fn decode_wolf_set(
    value: RegistryElementValueRef<'_>,
    codec: &str,
    path: &str,
) -> Result<WolfSoundSet, RegistryCodecError> {
    let fields = compound(value, WOLF_REGISTRY, codec, path)?;
    Ok(WolfSoundSet {
        ambient_sound: holder_field(fields, "ambient_sound", WOLF_REGISTRY, codec, path)?,
        death_sound: holder_field(fields, "death_sound", WOLF_REGISTRY, codec, path)?,
        growl_sound: holder_field(fields, "growl_sound", WOLF_REGISTRY, codec, path)?,
        hurt_sound: holder_field(fields, "hurt_sound", WOLF_REGISTRY, codec, path)?,
        pant_sound: holder_field(fields, "pant_sound", WOLF_REGISTRY, codec, path)?,
        whine_sound: holder_field(fields, "whine_sound", WOLF_REGISTRY, codec, path)?,
        step_sound: holder_field(fields, "step_sound", WOLF_REGISTRY, codec, path)?,
    })
}

fn decode_pig(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<PigSoundVariantElement, RegistryCodecError> {
    let root = compound(value, PIG_REGISTRY, codec, "<root>")?;
    Ok(PigSoundVariantElement {
        adult_sounds: Box::new(decode_pig_set(
            required(root, "adult_sounds", PIG_REGISTRY, codec, "adult_sounds")?,
            codec,
            "adult_sounds",
        )?),
        baby_sounds: Box::new(decode_pig_set(
            required(root, "baby_sounds", PIG_REGISTRY, codec, "baby_sounds")?,
            codec,
            "baby_sounds",
        )?),
    })
}

fn decode_pig_set(
    value: RegistryElementValueRef<'_>,
    codec: &str,
    path: &str,
) -> Result<PigSoundSet, RegistryCodecError> {
    let fields = compound(value, PIG_REGISTRY, codec, path)?;
    Ok(PigSoundSet {
        ambient_sound: holder_field(fields, "ambient_sound", PIG_REGISTRY, codec, path)?,
        hurt_sound: holder_field(fields, "hurt_sound", PIG_REGISTRY, codec, path)?,
        death_sound: holder_field(fields, "death_sound", PIG_REGISTRY, codec, path)?,
        step_sound: holder_field(fields, "step_sound", PIG_REGISTRY, codec, path)?,
        eat_sound: holder_field(fields, "eat_sound", PIG_REGISTRY, codec, path)?,
    })
}

fn decode_cat(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<CatSoundVariantElement, RegistryCodecError> {
    let root = compound(value, CAT_REGISTRY, codec, "<root>")?;
    Ok(CatSoundVariantElement {
        adult_sounds: Box::new(decode_cat_set(
            required(root, "adult_sounds", CAT_REGISTRY, codec, "adult_sounds")?,
            codec,
            "adult_sounds",
        )?),
        baby_sounds: Box::new(decode_cat_set(
            required(root, "baby_sounds", CAT_REGISTRY, codec, "baby_sounds")?,
            codec,
            "baby_sounds",
        )?),
    })
}

fn decode_cat_set(
    value: RegistryElementValueRef<'_>,
    codec: &str,
    path: &str,
) -> Result<CatSoundSet, RegistryCodecError> {
    let fields = compound(value, CAT_REGISTRY, codec, path)?;
    Ok(CatSoundSet {
        ambient_sound: holder_field(fields, "ambient_sound", CAT_REGISTRY, codec, path)?,
        stray_ambient_sound: holder_field(
            fields,
            "stray_ambient_sound",
            CAT_REGISTRY,
            codec,
            path,
        )?,
        hiss_sound: holder_field(fields, "hiss_sound", CAT_REGISTRY, codec, path)?,
        hurt_sound: holder_field(fields, "hurt_sound", CAT_REGISTRY, codec, path)?,
        death_sound: holder_field(fields, "death_sound", CAT_REGISTRY, codec, path)?,
        eat_sound: holder_field(fields, "eat_sound", CAT_REGISTRY, codec, path)?,
        beg_for_food_sound: holder_field(fields, "beg_for_food_sound", CAT_REGISTRY, codec, path)?,
        purr_sound: holder_field(fields, "purr_sound", CAT_REGISTRY, codec, path)?,
        purreow_sound: holder_field(fields, "purreow_sound", CAT_REGISTRY, codec, path)?,
    })
}

fn decode_cow(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<CowSoundVariantElement, RegistryCodecError> {
    let fields = compound(value, COW_REGISTRY, codec, "<root>")?;
    Ok(CowSoundVariantElement {
        ambient_sound: holder_field(fields, "ambient_sound", COW_REGISTRY, codec, "<root>")?,
        hurt_sound: holder_field(fields, "hurt_sound", COW_REGISTRY, codec, "<root>")?,
        death_sound: holder_field(fields, "death_sound", COW_REGISTRY, codec, "<root>")?,
        step_sound: holder_field(fields, "step_sound", COW_REGISTRY, codec, "<root>")?,
    })
}

fn decode_chicken(
    value: RegistryElementValueRef<'_>,
    codec: &str,
) -> Result<ChickenSoundVariantElement, RegistryCodecError> {
    let root = compound(value, CHICKEN_REGISTRY, codec, "<root>")?;
    Ok(ChickenSoundVariantElement {
        adult_sounds: Box::new(decode_chicken_set(
            required(
                root,
                "adult_sounds",
                CHICKEN_REGISTRY,
                codec,
                "adult_sounds",
            )?,
            codec,
            "adult_sounds",
        )?),
        baby_sounds: Box::new(decode_chicken_set(
            required(root, "baby_sounds", CHICKEN_REGISTRY, codec, "baby_sounds")?,
            codec,
            "baby_sounds",
        )?),
    })
}

fn decode_chicken_set(
    value: RegistryElementValueRef<'_>,
    codec: &str,
    path: &str,
) -> Result<ChickenSoundSet, RegistryCodecError> {
    let fields = compound(value, CHICKEN_REGISTRY, codec, path)?;
    Ok(ChickenSoundSet {
        ambient_sound: holder_field(fields, "ambient_sound", CHICKEN_REGISTRY, codec, path)?,
        hurt_sound: holder_field(fields, "hurt_sound", CHICKEN_REGISTRY, codec, path)?,
        death_sound: holder_field(fields, "death_sound", CHICKEN_REGISTRY, codec, path)?,
        step_sound: holder_field(fields, "step_sound", CHICKEN_REGISTRY, codec, path)?,
    })
}

fn holder_field(
    fields: RegistryElementValueRef<'_>,
    name: &str,
    registry: &str,
    codec: &str,
    parent_path: &str,
) -> Result<SoundEventHolder, RegistryCodecError> {
    let path = nested_path(parent_path, name);
    decode_sound_event_holder(
        required(fields, name, registry, codec, &path)?,
        registry,
        codec,
        &path,
    )
}

pub fn decode_sound_event_holder(
    value: RegistryElementValueRef<'_>,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<SoundEventHolder, RegistryCodecError> {
    match value.java_string_ref() {
        Some(raw) => RegistryResourceIdentifier::parse_java_string(raw)
            .map(SoundEventHolder::Named)
            .ok_or_else(|| codec_error(registry, codec, path, "invalid identifier")),
        None if value.is_compound() => {
            let sound_id_path = nested_path(path, "sound_id");
            let sound_id = required(value, "sound_id", registry, codec, &sound_id_path)?
                .java_string_ref()
                .and_then(RegistryResourceIdentifier::parse_java_string)
                .ok_or_else(|| {
                    codec_error(
                        registry,
                        codec,
                        &sound_id_path,
                        "expected a valid string identifier",
                    )
                })?;
            // `lenientOptionalFieldOf` drops both an absent field and one that
            // `Codec.FLOAT` cannot decode.
            let range = value.field("range").and_then(codec_float);
            Ok(SoundEventHolder::Inline(SoundEventElement {
                sound_id,
                range,
            }))
        }
        _ => Err(codec_error(
            registry,
            codec,
            path,
            "expected a named string or inline sound-event compound",
        )),
    }
}

// Every NumericTag is a Number and createFloat emits FloatTag
fn codec_float(value: RegistryElementValueRef<'_>) -> Option<CodecFloat> {
    value.codec_f32().map(CodecFloat::from_f32)
}

fn encode_wolf(value: &WolfSoundVariantElement) -> NbtSummaryValue {
    compound_value(vec![
        ("adult_sounds", encode_wolf_set(&value.adult_sounds)),
        ("baby_sounds", encode_wolf_set(&value.baby_sounds)),
    ])
}

fn encode_wolf_set(value: &WolfSoundSet) -> NbtSummaryValue {
    compound_value(vec![
        (
            "ambient_sound",
            encode_sound_event_holder(&value.ambient_sound),
        ),
        ("death_sound", encode_sound_event_holder(&value.death_sound)),
        ("growl_sound", encode_sound_event_holder(&value.growl_sound)),
        ("hurt_sound", encode_sound_event_holder(&value.hurt_sound)),
        ("pant_sound", encode_sound_event_holder(&value.pant_sound)),
        ("whine_sound", encode_sound_event_holder(&value.whine_sound)),
        ("step_sound", encode_sound_event_holder(&value.step_sound)),
    ])
}

fn encode_pig(value: &PigSoundVariantElement) -> NbtSummaryValue {
    compound_value(vec![
        ("adult_sounds", encode_pig_set(&value.adult_sounds)),
        ("baby_sounds", encode_pig_set(&value.baby_sounds)),
    ])
}

fn encode_pig_set(value: &PigSoundSet) -> NbtSummaryValue {
    compound_value(vec![
        (
            "ambient_sound",
            encode_sound_event_holder(&value.ambient_sound),
        ),
        ("hurt_sound", encode_sound_event_holder(&value.hurt_sound)),
        ("death_sound", encode_sound_event_holder(&value.death_sound)),
        ("step_sound", encode_sound_event_holder(&value.step_sound)),
        ("eat_sound", encode_sound_event_holder(&value.eat_sound)),
    ])
}

fn encode_cat(value: &CatSoundVariantElement) -> NbtSummaryValue {
    compound_value(vec![
        ("adult_sounds", encode_cat_set(&value.adult_sounds)),
        ("baby_sounds", encode_cat_set(&value.baby_sounds)),
    ])
}

fn encode_cat_set(value: &CatSoundSet) -> NbtSummaryValue {
    compound_value(vec![
        (
            "ambient_sound",
            encode_sound_event_holder(&value.ambient_sound),
        ),
        (
            "stray_ambient_sound",
            encode_sound_event_holder(&value.stray_ambient_sound),
        ),
        ("hiss_sound", encode_sound_event_holder(&value.hiss_sound)),
        ("hurt_sound", encode_sound_event_holder(&value.hurt_sound)),
        ("death_sound", encode_sound_event_holder(&value.death_sound)),
        ("eat_sound", encode_sound_event_holder(&value.eat_sound)),
        (
            "beg_for_food_sound",
            encode_sound_event_holder(&value.beg_for_food_sound),
        ),
        ("purr_sound", encode_sound_event_holder(&value.purr_sound)),
        (
            "purreow_sound",
            encode_sound_event_holder(&value.purreow_sound),
        ),
    ])
}

fn encode_cow(value: &CowSoundVariantElement) -> NbtSummaryValue {
    compound_value(vec![
        (
            "ambient_sound",
            encode_sound_event_holder(&value.ambient_sound),
        ),
        ("hurt_sound", encode_sound_event_holder(&value.hurt_sound)),
        ("death_sound", encode_sound_event_holder(&value.death_sound)),
        ("step_sound", encode_sound_event_holder(&value.step_sound)),
    ])
}

fn encode_chicken(value: &ChickenSoundVariantElement) -> NbtSummaryValue {
    compound_value(vec![
        ("adult_sounds", encode_chicken_set(&value.adult_sounds)),
        ("baby_sounds", encode_chicken_set(&value.baby_sounds)),
    ])
}

fn encode_chicken_set(value: &ChickenSoundSet) -> NbtSummaryValue {
    compound_value(vec![
        (
            "ambient_sound",
            encode_sound_event_holder(&value.ambient_sound),
        ),
        ("hurt_sound", encode_sound_event_holder(&value.hurt_sound)),
        ("death_sound", encode_sound_event_holder(&value.death_sound)),
        ("step_sound", encode_sound_event_holder(&value.step_sound)),
    ])
}

pub(crate) fn encode_sound_event_holder(value: &SoundEventHolder) -> NbtSummaryValue {
    match value {
        SoundEventHolder::Named(identifier) => NbtSummaryValue::String(identifier.as_str().into()),
        SoundEventHolder::Inline(event) => {
            let mut fields = vec![(
                "sound_id",
                NbtSummaryValue::String(event.sound_id.as_str().into()),
            )];
            if let Some(range) = event.range {
                // `NbtOps.createFloat` delegates to `FloatTag.valueOf`, whose
                // zero cache canonicalizes either signed zero to +0. NaN
                // payloads/signs remain untouched.
                fields.push((
                    "range",
                    NbtSummaryValue::Float(canonical_float_tag_bits(range.to_f32())),
                ));
            }
            compound_value(fields)
        }
    }
}

fn wolf_references<'a>(value: &'a WolfSoundSet, output: &mut Vec<&'a RegistryResourceIdentifier>) {
    extend_named(
        output,
        [
            &value.ambient_sound,
            &value.death_sound,
            &value.growl_sound,
            &value.hurt_sound,
            &value.pant_sound,
            &value.whine_sound,
            &value.step_sound,
        ],
    );
}

fn pig_references<'a>(value: &'a PigSoundSet, output: &mut Vec<&'a RegistryResourceIdentifier>) {
    extend_named(
        output,
        [
            &value.ambient_sound,
            &value.hurt_sound,
            &value.death_sound,
            &value.step_sound,
            &value.eat_sound,
        ],
    );
}

fn cat_references<'a>(value: &'a CatSoundSet, output: &mut Vec<&'a RegistryResourceIdentifier>) {
    extend_named(
        output,
        [
            &value.ambient_sound,
            &value.stray_ambient_sound,
            &value.hiss_sound,
            &value.hurt_sound,
            &value.death_sound,
            &value.eat_sound,
            &value.beg_for_food_sound,
            &value.purr_sound,
            &value.purreow_sound,
        ],
    );
}

fn cow_references<'a>(
    value: &'a CowSoundVariantElement,
    output: &mut Vec<&'a RegistryResourceIdentifier>,
) {
    extend_named(
        output,
        [
            &value.ambient_sound,
            &value.hurt_sound,
            &value.death_sound,
            &value.step_sound,
        ],
    );
}

fn chicken_references<'a>(
    value: &'a ChickenSoundSet,
    output: &mut Vec<&'a RegistryResourceIdentifier>,
) {
    extend_named(
        output,
        [
            &value.ambient_sound,
            &value.hurt_sound,
            &value.death_sound,
            &value.step_sound,
        ],
    );
}

fn extend_named<'a, const N: usize>(
    output: &mut Vec<&'a RegistryResourceIdentifier>,
    holders: [&'a SoundEventHolder; N],
) {
    output.extend(holders.into_iter().filter_map(|holder| match holder {
        SoundEventHolder::Named(identifier) => Some(identifier),
        SoundEventHolder::Inline(_) => None,
    }));
}

fn compound<'a>(
    value: RegistryElementValueRef<'a>,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<RegistryElementValueRef<'a>, RegistryCodecError> {
    if value.is_compound() {
        Ok(value)
    } else {
        Err(codec_error(registry, codec, path, "expected a compound"))
    }
}

fn required<'a>(
    value: RegistryElementValueRef<'a>,
    name: &str,
    registry: &str,
    codec: &str,
    path: &str,
) -> Result<RegistryElementValueRef<'a>, RegistryCodecError> {
    value
        .field(name)
        .ok_or_else(|| codec_error(registry, codec, path, "required field is missing"))
}

#[cfg(test)]
fn last_field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .rev()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn nested_path(parent: &str, field: &str) -> String {
    if parent == "<root>" {
        field.to_string()
    } else {
        format!("{parent}.{field}")
    }
}

fn compound_value(fields: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| entry(name, value))
            .collect(),
    )
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

    const WOLF_FIELDS: &[&str] = &[
        "ambient_sound",
        "death_sound",
        "growl_sound",
        "hurt_sound",
        "pant_sound",
        "whine_sound",
        "step_sound",
    ];
    const PIG_FIELDS: &[&str] = &[
        "ambient_sound",
        "hurt_sound",
        "death_sound",
        "step_sound",
        "eat_sound",
    ];
    const CAT_FIELDS: &[&str] = &[
        "ambient_sound",
        "stray_ambient_sound",
        "hiss_sound",
        "hurt_sound",
        "death_sound",
        "eat_sound",
        "beg_for_food_sound",
        "purr_sound",
        "purreow_sound",
    ];
    const FOUR_FIELDS: &[&str] = &["ambient_sound", "hurt_sound", "death_sound", "step_sound"];

    fn named(name: impl Into<String>) -> NbtSummaryValue {
        NbtSummaryValue::String(name.into().into())
    }

    fn inline(sound_id: &str, range: Option<NbtSummaryValue>) -> NbtSummaryValue {
        let mut fields = vec![
            entry("unknown", NbtSummaryValue::Int(99)),
            entry("sound_id", named(sound_id)),
        ];
        if let Some(range) = range {
            fields.push(entry("range", range));
        }
        NbtSummaryValue::Compound(fields)
    }

    fn set(names: &[&str]) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    entry(
                        *name,
                        if index == 0 {
                            inline("custom:inline", Some(NbtSummaryValue::Int(12)))
                        } else {
                            named(format!("custom:{name}"))
                        },
                    )
                })
                .collect(),
        )
    }

    fn age_variant(names: &[&str]) -> NbtSummaryValue {
        compound_value(vec![
            ("adult_sounds", set(names)),
            ("baby_sounds", set(names)),
        ])
    }

    fn fields(value: &NbtSummaryValue) -> &[NbtSummaryEntry] {
        match value {
            NbtSummaryValue::Compound(fields) => fields,
            other => panic!("expected compound, got {other:?}"),
        }
    }

    fn names(value: &NbtSummaryValue) -> Vec<&bbb_protocol::JavaString> {
        fields(value).iter().map(|field| &field.name).collect()
    }

    #[test]
    fn all_five_schemas_decode_mixed_holders_and_encode_exact_shapes() {
        for (registry, value, expected_fields, has_age_sets) in [
            (WOLF_REGISTRY, age_variant(WOLF_FIELDS), WOLF_FIELDS, true),
            (PIG_REGISTRY, age_variant(PIG_FIELDS), PIG_FIELDS, true),
            (CAT_REGISTRY, age_variant(CAT_FIELDS), CAT_FIELDS, true),
            (COW_REGISTRY, set(FOUR_FIELDS), FOUR_FIELDS, false),
            (
                CHICKEN_REGISTRY,
                age_variant(FOUR_FIELDS),
                FOUR_FIELDS,
                true,
            ),
        ] {
            let decoded = decode_sound_variant_element(
                registry,
                RegistryElementSource::NetworkNbt,
                RegistryElementValueRef::Nbt(&value),
            )
            .unwrap();
            let encoded = encode_sound_variant_network(&decoded).unwrap();
            let set_value = if has_age_sets {
                assert_eq!(names(&encoded), ["adult_sounds", "baby_sounds"]);
                last_field(fields(&encoded), "adult_sounds").unwrap()
            } else {
                &encoded
            };
            assert_eq!(names(set_value), expected_fields);

            let inline_value = last_field(fields(set_value), expected_fields[0]).unwrap();
            assert_eq!(names(inline_value), ["sound_id", "range"]);
            assert_eq!(
                last_field(fields(inline_value), "range"),
                Some(&NbtSummaryValue::Float(12.0_f32.to_bits()))
            );

            let named_count = expected_fields.len() * if has_age_sets { 2 } else { 1 }
                - if has_age_sets { 2 } else { 1 };
            assert_eq!(
                sound_variant_registry_codec_references(&decoded).len(),
                named_count
            );
        }
    }

    #[test]
    fn network_encode_network_decode_is_typed_round_trip() {
        for (registry, value) in [
            (WOLF_REGISTRY, age_variant(WOLF_FIELDS)),
            (PIG_REGISTRY, age_variant(PIG_FIELDS)),
            (CAT_REGISTRY, age_variant(CAT_FIELDS)),
            (COW_REGISTRY, set(FOUR_FIELDS)),
            (CHICKEN_REGISTRY, age_variant(FOUR_FIELDS)),
        ] {
            let initial = decode_sound_variant_element(
                registry,
                RegistryElementSource::NetworkNbt,
                RegistryElementValueRef::Nbt(&value),
            )
            .unwrap();
            let network = encode_sound_variant_network(&initial).unwrap();
            let decoded = decode_sound_variant_element(
                registry,
                RegistryElementSource::NetworkNbt,
                RegistryElementValueRef::Nbt(&network),
            )
            .unwrap();
            assert_eq!(decoded, initial);
        }
    }
}
