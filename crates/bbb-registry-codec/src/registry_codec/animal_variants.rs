//! Exact persistent and network codecs for synchronized animal variants.

use std::cmp::Ordering;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::{
    synchronized_registry_element_codec, RegistryCodecError, RegistryCodecReference,
    RegistryElementData, RegistryElementSource, RegistryElementValueRef, RegistryReferenceKind,
    RegistryResourceIdentifier,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAssetResourceTexture {
    pub asset_id: RegistryResourceIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelAndTexture<M> {
    pub model: M,
    pub asset: ClientAssetResourceTexture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PigModel {
    Normal,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CowModel {
    Normal,
    Cold,
    Warm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChickenModel {
    Normal,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZombieNautilusModel {
    Normal,
    Warm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolderReference {
    Element(RegistryResourceIdentifier),
    Tag(RegistryResourceIdentifier),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolderSetReference {
    pub references: Vec<HolderReference>,
}

/// A `Codec.DOUBLE` value represented by its exact IEEE-754 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecDouble(pub u64);

impl CodecDouble {
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoonBrightnessRange {
    pub min: Option<CodecDouble>,
    pub max: Option<CodecDouble>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnCondition {
    Biome { biomes: HolderSetReference },
    Structure { structures: HolderSetReference },
    MoonBrightness { range: MoonBrightnessRange },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnSelector {
    pub condition: Option<SpawnCondition>,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnPrioritySelectors {
    pub selectors: Vec<SpawnSelector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfAssetInfo {
    pub wild: ClientAssetResourceTexture,
    pub tame: ClientAssetResourceTexture,
    pub angry: ClientAssetResourceTexture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfVariantElement {
    pub assets: WolfAssetInfo,
    pub baby_assets: WolfAssetInfo,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PigVariantElement {
    pub model_and_texture: ModelAndTexture<PigModel>,
    pub baby_asset: ClientAssetResourceTexture,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrogVariantElement {
    pub asset: ClientAssetResourceTexture,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatVariantElement {
    pub adult_asset: ClientAssetResourceTexture,
    pub baby_asset: ClientAssetResourceTexture,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CowVariantElement {
    pub model_and_texture: ModelAndTexture<CowModel>,
    pub baby_asset: ClientAssetResourceTexture,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChickenVariantElement {
    pub model_and_texture: ModelAndTexture<ChickenModel>,
    pub baby_asset: ClientAssetResourceTexture,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZombieNautilusVariantElement {
    pub model_and_texture: ModelAndTexture<ZombieNautilusModel>,
    pub spawn_conditions: SpawnPrioritySelectors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimalVariantElement {
    Wolf(WolfVariantElement),
    Pig(PigVariantElement),
    Frog(FrogVariantElement),
    Cat(CatVariantElement),
    Cow(CowVariantElement),
    Chicken(ChickenVariantElement),
    ZombieNautilus(ZombieNautilusVariantElement),
}

impl AnimalVariantElement {
    #[inline]
    pub fn spawn_conditions(&self) -> &SpawnPrioritySelectors {
        match self {
            Self::Wolf(value) => &value.spawn_conditions,
            Self::Pig(value) => &value.spawn_conditions,
            Self::Frog(value) => &value.spawn_conditions,
            Self::Cat(value) => &value.spawn_conditions,
            Self::Cow(value) => &value.spawn_conditions,
            Self::Chicken(value) => &value.spawn_conditions,
            Self::ZombieNautilus(value) => &value.spawn_conditions,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimalVariantReference<'a> {
    pub target_registry: &'static str,
    pub reference: &'a HolderReference,
}

/// Decodes the codec selected by the element's origin. Server datapack values
/// use the persistent `DIRECT_CODEC`; known-pack JSON and network NBT use the
/// corresponding `NETWORK_CODEC`.
pub fn decode_animal_variant_element(
    registry: &str,
    data: &RegistryElementData,
) -> Result<AnimalVariantElement, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(registry, data.source)?;
    if !is_animal_variant_registry(registry) {
        return Err(RegistryCodecError::coverage(
            registry,
            codec,
            "<registry>",
            "unsupported animal variant registry",
        ));
    }
    let root = data.root();
    if !root.is_compound() {
        return Err(codec_error(
            registry,
            codec,
            "<root>",
            "expected a compound",
        ));
    }
    let spawn_conditions = match data.source {
        RegistryElementSource::ServerDatapackJson => decode_spawn_conditions(
            registry,
            codec,
            required(root, registry, codec, "spawn_conditions")?,
        )?,
        RegistryElementSource::ClientKnownPackJson | RegistryElementSource::NetworkNbt => {
            SpawnPrioritySelectors {
                selectors: Vec::new(),
            }
        }
    };

    Ok(match registry {
        "minecraft:wolf_variant" => AnimalVariantElement::Wolf(WolfVariantElement {
            assets: decode_wolf_assets(
                registry,
                codec,
                required(root, registry, codec, "assets")?,
                "assets",
            )?,
            baby_assets: decode_wolf_assets(
                registry,
                codec,
                required(root, registry, codec, "baby_assets")?,
                "baby_assets",
            )?,
            spawn_conditions,
        }),
        "minecraft:pig_variant" => AnimalVariantElement::Pig(PigVariantElement {
            model_and_texture: decode_model_and_texture(
                registry,
                codec,
                root,
                PigModel::Normal,
                decode_pig_model,
            )?,
            baby_asset: decode_asset_field(registry, codec, root, "baby_asset_id")?,
            spawn_conditions,
        }),
        "minecraft:frog_variant" => AnimalVariantElement::Frog(FrogVariantElement {
            asset: decode_asset_field(registry, codec, root, "asset_id")?,
            spawn_conditions,
        }),
        "minecraft:cat_variant" => AnimalVariantElement::Cat(CatVariantElement {
            adult_asset: decode_asset_field(registry, codec, root, "asset_id")?,
            baby_asset: decode_asset_field(registry, codec, root, "baby_asset_id")?,
            spawn_conditions,
        }),
        "minecraft:cow_variant" => AnimalVariantElement::Cow(CowVariantElement {
            model_and_texture: decode_model_and_texture(
                registry,
                codec,
                root,
                CowModel::Normal,
                decode_cow_model,
            )?,
            baby_asset: decode_asset_field(registry, codec, root, "baby_asset_id")?,
            spawn_conditions,
        }),
        "minecraft:chicken_variant" => AnimalVariantElement::Chicken(ChickenVariantElement {
            model_and_texture: decode_model_and_texture(
                registry,
                codec,
                root,
                ChickenModel::Normal,
                decode_chicken_model,
            )?,
            baby_asset: decode_asset_field(registry, codec, root, "baby_asset_id")?,
            spawn_conditions,
        }),
        "minecraft:zombie_nautilus_variant" => {
            AnimalVariantElement::ZombieNautilus(ZombieNautilusVariantElement {
                model_and_texture: decode_model_and_texture(
                    registry,
                    codec,
                    root,
                    ZombieNautilusModel::Normal,
                    decode_zombie_nautilus_model,
                )?,
                spawn_conditions,
            })
        }
        _ => unreachable!("the animal variant registry was checked before decoding"),
    })
}

fn is_animal_variant_registry(registry: &str) -> bool {
    matches!(
        registry,
        "minecraft:wolf_variant"
            | "minecraft:pig_variant"
            | "minecraft:frog_variant"
            | "minecraft:cat_variant"
            | "minecraft:cow_variant"
            | "minecraft:chicken_variant"
            | "minecraft:zombie_nautilus_variant"
    )
}

/// Emits the canonical NBT shape produced by the animal variant's
/// `NETWORK_CODEC`. In particular, persistent `spawn_conditions` never enter
/// this projection.
pub fn encode_animal_variant_network(
    element: &AnimalVariantElement,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let registry = animal_variant_registry(element);
    let _codec = synchronized_registry_element_codec(registry, RegistryElementSource::NetworkNbt)?;
    Ok(match element {
        AnimalVariantElement::Wolf(value) => compound_value(vec![
            ("assets", encode_wolf_assets(&value.assets)),
            ("baby_assets", encode_wolf_assets(&value.baby_assets)),
        ]),
        AnimalVariantElement::Pig(value) => encode_model_and_baby(
            &value.model_and_texture,
            &value.baby_asset,
            PigModel::Normal,
            pig_model_name,
        ),
        AnimalVariantElement::Frog(value) => {
            compound_value(vec![("asset_id", encode_identifier(&value.asset.asset_id))])
        }
        AnimalVariantElement::Cat(value) => compound_value(vec![
            ("asset_id", encode_identifier(&value.adult_asset.asset_id)),
            (
                "baby_asset_id",
                encode_identifier(&value.baby_asset.asset_id),
            ),
        ]),
        AnimalVariantElement::Cow(value) => encode_model_and_baby(
            &value.model_and_texture,
            &value.baby_asset,
            CowModel::Normal,
            cow_model_name,
        ),
        AnimalVariantElement::Chicken(value) => encode_model_and_baby(
            &value.model_and_texture,
            &value.baby_asset,
            ChickenModel::Normal,
            chicken_model_name,
        ),
        AnimalVariantElement::ZombieNautilus(value) => encode_model_and_texture(
            &value.model_and_texture,
            ZombieNautilusModel::Normal,
            zombie_nautilus_model_name,
        ),
    })
}

/// Returns only the registry-backed references retained from persistent spawn
/// selectors. Asset identifiers are client assets, not registry holders.
pub fn animal_variant_references(
    element: &AnimalVariantElement,
) -> Vec<AnimalVariantReference<'_>> {
    let selectors = element.spawn_conditions();
    let mut references = Vec::new();
    for selector in &selectors.selectors {
        let (target_registry, holder_set) = match &selector.condition {
            Some(SpawnCondition::Biome { biomes }) => ("minecraft:worldgen/biome", biomes),
            Some(SpawnCondition::Structure { structures }) => {
                ("minecraft:worldgen/structure", structures)
            }
            Some(SpawnCondition::MoonBrightness { .. }) | None => continue,
        };
        references.extend(
            holder_set
                .references
                .iter()
                .map(|reference| AnimalVariantReference {
                    target_registry,
                    reference,
                }),
        );
    }
    references
}

pub(crate) fn animal_variant_registry_codec_references(
    element: &AnimalVariantElement,
) -> Vec<RegistryCodecReference<'_>> {
    animal_variant_references(element)
        .into_iter()
        .map(|reference| {
            let (kind, resource_id) = match reference.reference {
                HolderReference::Element(resource_id) => {
                    (RegistryReferenceKind::Element, resource_id)
                }
                HolderReference::Tag(resource_id) => (RegistryReferenceKind::Tag, resource_id),
            };
            RegistryCodecReference {
                target_registry: reference.target_registry,
                kind,
                resource_id,
            }
        })
        .collect()
}

fn animal_variant_registry(element: &AnimalVariantElement) -> &'static str {
    match element {
        AnimalVariantElement::Wolf(_) => "minecraft:wolf_variant",
        AnimalVariantElement::Pig(_) => "minecraft:pig_variant",
        AnimalVariantElement::Frog(_) => "minecraft:frog_variant",
        AnimalVariantElement::Cat(_) => "minecraft:cat_variant",
        AnimalVariantElement::Cow(_) => "minecraft:cow_variant",
        AnimalVariantElement::Chicken(_) => "minecraft:chicken_variant",
        AnimalVariantElement::ZombieNautilus(_) => "minecraft:zombie_nautilus_variant",
    }
}

fn decode_wolf_assets(
    registry: &str,
    codec: &str,
    value: RegistryElementValueRef<'_>,
    field: &str,
) -> Result<WolfAssetInfo, RegistryCodecError> {
    if !value.is_compound() {
        return Err(codec_error(registry, codec, field, "expected a compound"));
    }
    Ok(WolfAssetInfo {
        wild: decode_asset_field(registry, codec, value, &format!("{field}.wild"))?,
        tame: decode_asset_field(registry, codec, value, &format!("{field}.tame"))?,
        angry: decode_asset_field(registry, codec, value, &format!("{field}.angry"))?,
    })
}

fn decode_asset_field(
    registry: &str,
    codec: &str,
    value: RegistryElementValueRef<'_>,
    field: &str,
) -> Result<ClientAssetResourceTexture, RegistryCodecError> {
    let value = required(value, registry, codec, field)?;
    Ok(ClientAssetResourceTexture {
        asset_id: decode_identifier(registry, codec, field, value)?,
    })
}

fn decode_model_and_texture<M: Copy>(
    registry: &str,
    codec: &str,
    value: RegistryElementValueRef<'_>,
    default: M,
    decode_model: fn(&str) -> Option<M>,
) -> Result<ModelAndTexture<M>, RegistryCodecError> {
    let model = match value.field("model") {
        Some(model) => {
            let model = model
                .java_string_ref()
                .ok_or_else(|| codec_error(registry, codec, "model", "expected a string"))?;
            model
                .with_utf8(decode_model)
                .flatten()
                .ok_or_else(|| codec_error(registry, codec, "model", "unsupported model value"))?
        }
        None => default,
    };
    Ok(ModelAndTexture {
        model,
        asset: decode_asset_field(registry, codec, value, "asset_id")?,
    })
}

fn decode_spawn_conditions(
    registry: &str,
    codec: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<SpawnPrioritySelectors, RegistryCodecError> {
    let Some(values) = value.list_elements() else {
        return Err(codec_error(
            registry,
            codec,
            "spawn_conditions",
            "expected a list",
        ));
    };
    let selectors = values
        .enumerate()
        .map(|(index, value)| decode_spawn_selector(registry, codec, index, value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpawnPrioritySelectors { selectors })
}

fn decode_spawn_selector(
    registry: &str,
    codec: &str,
    index: usize,
    value: RegistryElementValueRef<'_>,
) -> Result<SpawnSelector, RegistryCodecError> {
    let path = format!("spawn_conditions[{index}]");
    if !value.is_compound() {
        return Err(codec_error(registry, codec, &path, "expected a compound"));
    }
    let priority_path = format!("{path}.priority");
    let priority =
        codec_int(required(value, registry, codec, &priority_path)?).ok_or_else(|| {
            codec_error(
                registry,
                codec,
                &priority_path,
                "expected a numeric Codec.INT",
            )
        })?;
    let condition = value
        .field("condition")
        .map(|value| decode_spawn_condition(registry, codec, &format!("{path}.condition"), value))
        .transpose()?;
    Ok(SpawnSelector {
        condition,
        priority,
    })
}

fn decode_spawn_condition(
    registry: &str,
    codec: &str,
    path: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<SpawnCondition, RegistryCodecError> {
    if !value.is_compound() {
        return Err(codec_error(registry, codec, path, "expected a compound"));
    }
    let type_path = format!("{path}.type");
    let condition_type = string(required(value, registry, codec, &type_path)?)
        .ok_or_else(|| codec_error(registry, codec, &type_path, "expected a string"))?;
    let condition_type =
        RegistryResourceIdentifier::parse_java_string(condition_type).ok_or_else(|| {
            codec_error(
                registry,
                codec,
                &type_path,
                "invalid condition type identifier",
            )
        })?;
    match condition_type.as_str() {
        "minecraft:biome" => Ok(SpawnCondition::Biome {
            biomes: decode_holder_set(
                registry,
                codec,
                &format!("{path}.biomes"),
                required(value, registry, codec, &format!("{path}.biomes"))?,
            )?,
        }),
        "minecraft:structure" => Ok(SpawnCondition::Structure {
            structures: decode_holder_set(
                registry,
                codec,
                &format!("{path}.structures"),
                required(value, registry, codec, &format!("{path}.structures"))?,
            )?,
        }),
        "minecraft:moon_brightness" => Ok(SpawnCondition::MoonBrightness {
            range: decode_moon_range(
                registry,
                codec,
                &format!("{path}.range"),
                required(value, registry, codec, &format!("{path}.range"))?,
            )?,
        }),
        _ => Err(codec_error(
            registry,
            codec,
            &type_path,
            "unsupported spawn condition dispatch",
        )),
    }
}

fn decode_holder_set(
    registry: &str,
    codec: &str,
    path: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<HolderSetReference, RegistryCodecError> {
    let references = if let Some(value) = value.java_string_ref() {
        if let Some(units) = value.units().strip_prefix(&[b'#' as u16]) {
            let identifier = String::from_utf16(units)
                .ok()
                .and_then(RegistryResourceIdentifier::parse_owned)
                .ok_or_else(|| codec_error(registry, codec, path, "invalid resource identifier"))?;
            vec![HolderReference::Tag(identifier)]
        } else {
            vec![HolderReference::Element(decode_identifier_string(
                registry, codec, path, value,
            )?)]
        }
    } else if let Some(values) = value.list_elements() {
        values
            .enumerate()
            .map(|(index, value)| {
                let entry_path = format!("{path}[{index}]");
                let value = string(value).ok_or_else(|| {
                    codec_error(
                        registry,
                        codec,
                        &entry_path,
                        "expected an element identifier",
                    )
                })?;
                if value.units().starts_with(&[b'#' as u16]) {
                    return Err(codec_error(
                        registry,
                        codec,
                        &entry_path,
                        "tag identifiers are not valid inside a holder element list",
                    ));
                }
                decode_identifier_string(registry, codec, &entry_path, value)
                    .map(HolderReference::Element)
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return Err(codec_error(
            registry,
            codec,
            path,
            "expected a tag, element identifier, or element list",
        ));
    };
    Ok(HolderSetReference { references })
}

fn decode_moon_range(
    registry: &str,
    codec: &str,
    path: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<MoonBrightnessRange, RegistryCodecError> {
    let range = if let Some(value) = codec_double(value) {
        let value = CodecDouble(value.to_bits());
        MoonBrightnessRange {
            min: Some(value),
            max: Some(value),
        }
    } else if value.is_compound() {
        let min = value
            .field("min")
            .map(|value| decode_codec_double(registry, codec, &format!("{path}.min"), value))
            .transpose()?;
        let max = value
            .field("max")
            .map(|value| decode_codec_double(registry, codec, &format!("{path}.max"), value))
            .transpose()?;
        MoonBrightnessRange { min, max }
    } else {
        return Err(codec_error(
            registry,
            codec,
            path,
            "expected a number or {min,max} compound",
        ));
    };
    if let (Some(min), Some(max)) = (range.min, range.max) {
        if java_double_cmp(min.get(), max.get()) == Ordering::Greater {
            return Err(codec_error(
                registry,
                codec,
                path,
                "swapped moon brightness range bounds",
            ));
        }
    }
    Ok(range)
}

fn decode_codec_double(
    registry: &str,
    codec: &str,
    path: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<CodecDouble, RegistryCodecError> {
    codec_double(value)
        .map(|value| CodecDouble(value.to_bits()))
        .ok_or_else(|| codec_error(registry, codec, path, "expected a numeric Codec.DOUBLE"))
}

fn java_double_cmp(left: f64, right: f64) -> Ordering {
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) if left == right => (left.to_bits() as i64).cmp(&(right.to_bits() as i64)),
        (false, false) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    }
}

fn decode_identifier(
    registry: &str,
    codec: &str,
    path: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    let value = string(value)
        .ok_or_else(|| codec_error(registry, codec, path, "expected an identifier string"))?;
    decode_identifier_string(registry, codec, path, value)
}

fn decode_identifier_string(
    registry: &str,
    codec: &str,
    path: &str,
    value: &bbb_protocol::JavaString,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    RegistryResourceIdentifier::parse_java_string(value)
        .ok_or_else(|| codec_error(registry, codec, path, "invalid resource identifier"))
}

fn decode_pig_model(value: &str) -> Option<PigModel> {
    match value {
        "normal" => Some(PigModel::Normal),
        "cold" => Some(PigModel::Cold),
        _ => None,
    }
}

fn decode_cow_model(value: &str) -> Option<CowModel> {
    match value {
        "normal" => Some(CowModel::Normal),
        "cold" => Some(CowModel::Cold),
        "warm" => Some(CowModel::Warm),
        _ => None,
    }
}

fn decode_chicken_model(value: &str) -> Option<ChickenModel> {
    match value {
        "normal" => Some(ChickenModel::Normal),
        "cold" => Some(ChickenModel::Cold),
        _ => None,
    }
}

fn decode_zombie_nautilus_model(value: &str) -> Option<ZombieNautilusModel> {
    match value {
        "normal" => Some(ZombieNautilusModel::Normal),
        "warm" => Some(ZombieNautilusModel::Warm),
        _ => None,
    }
}

fn pig_model_name(model: PigModel) -> &'static str {
    match model {
        PigModel::Normal => "normal",
        PigModel::Cold => "cold",
    }
}

fn cow_model_name(model: CowModel) -> &'static str {
    match model {
        CowModel::Normal => "normal",
        CowModel::Cold => "cold",
        CowModel::Warm => "warm",
    }
}

fn chicken_model_name(model: ChickenModel) -> &'static str {
    match model {
        ChickenModel::Normal => "normal",
        ChickenModel::Cold => "cold",
    }
}

fn zombie_nautilus_model_name(model: ZombieNautilusModel) -> &'static str {
    match model {
        ZombieNautilusModel::Normal => "normal",
        ZombieNautilusModel::Warm => "warm",
    }
}

fn encode_model_and_baby<M: Copy + PartialEq>(
    model_and_texture: &ModelAndTexture<M>,
    baby_asset: &ClientAssetResourceTexture,
    default: M,
    model_name: fn(M) -> &'static str,
) -> NbtSummaryValue {
    let mut fields = encode_model_and_texture_fields(model_and_texture, default, model_name);
    fields.push(("baby_asset_id", encode_identifier(&baby_asset.asset_id)));
    compound_value(fields)
}

fn encode_model_and_texture<M: Copy + PartialEq>(
    value: &ModelAndTexture<M>,
    default: M,
    model_name: fn(M) -> &'static str,
) -> NbtSummaryValue {
    compound_value(encode_model_and_texture_fields(value, default, model_name))
}

fn encode_model_and_texture_fields<M: Copy + PartialEq>(
    value: &ModelAndTexture<M>,
    default: M,
    model_name: fn(M) -> &'static str,
) -> Vec<(&'static str, NbtSummaryValue)> {
    let mut fields = Vec::new();
    if value.model != default {
        fields.push((
            "model",
            NbtSummaryValue::String(model_name(value.model).into()),
        ));
    }
    fields.push(("asset_id", encode_identifier(&value.asset.asset_id)));
    fields
}

fn encode_wolf_assets(value: &WolfAssetInfo) -> NbtSummaryValue {
    compound_value(vec![
        ("wild", encode_identifier(&value.wild.asset_id)),
        ("tame", encode_identifier(&value.tame.asset_id)),
        ("angry", encode_identifier(&value.angry.asset_id)),
    ])
}

fn encode_identifier(value: &RegistryResourceIdentifier) -> NbtSummaryValue {
    NbtSummaryValue::String(value.as_str().into())
}

fn compound_value(fields: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

fn required<'a>(
    value: RegistryElementValueRef<'a>,
    registry: &str,
    codec: &str,
    field: &str,
) -> Result<RegistryElementValueRef<'a>, RegistryCodecError> {
    let leaf = field.rsplit('.').next().unwrap_or(field);
    value
        .field(leaf)
        .ok_or_else(|| codec_error(registry, codec, field, "missing required field"))
}

fn string(value: RegistryElementValueRef<'_>) -> Option<&bbb_protocol::JavaString> {
    value.java_string_ref()
}

fn codec_int(value: RegistryElementValueRef<'_>) -> Option<i32> {
    value.codec_i32()
}

fn codec_double(value: RegistryElementValueRef<'_>) -> Option<f64> {
    value.codec_f64()
}

fn codec_error(registry: &str, codec: &str, field: &str, reason: &str) -> RegistryCodecError {
    RegistryCodecError::native_semantic(registry, codec, field, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_client_known_pack_registry_json, decode_direct_registry_json,
    };

    fn decode_direct(registry: &str, json: &str) -> AnimalVariantElement {
        let data = decode_direct_registry_json(json.as_bytes()).unwrap();
        decode_animal_variant_element(registry, &data).unwrap()
    }

    fn decode_known(registry: &str, json: &str) -> AnimalVariantElement {
        let data = decode_client_known_pack_registry_json(json.as_bytes()).unwrap();
        decode_animal_variant_element(registry, &data).unwrap()
    }

    fn encoded_direct(registry: &str, json: &str) -> NbtSummaryValue {
        encode_animal_variant_network(&decode_direct(registry, json)).unwrap()
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(value.to_string().into())
    }

    fn compound(fields: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
        compound_value(fields)
    }

    fn encoded_last_field<'a>(
        entries: &'a [NbtSummaryEntry],
        field: &str,
    ) -> Option<&'a NbtSummaryValue> {
        entries
            .iter()
            .rev()
            .find(|entry| entry.name == field)
            .map(|entry| &entry.value)
    }

    #[test]
    fn all_seven_direct_codecs_have_exact_network_projections() {
        let cases = [
            (
                "minecraft:wolf_variant",
                r#"{
                    "assets":{"wild":"wolf/wild","tame":"example:wolf/tame","angry":":wolf/angry"},
                    "baby_assets":{"wild":"wolf/baby_wild","tame":"wolf/baby_tame","angry":"wolf/baby_angry"},
                    "spawn_conditions":[{"priority":0}]
                }"#,
                compound(vec![
                    (
                        "assets",
                        compound(vec![
                            ("wild", string("minecraft:wolf/wild")),
                            ("tame", string("example:wolf/tame")),
                            ("angry", string("minecraft:wolf/angry")),
                        ]),
                    ),
                    (
                        "baby_assets",
                        compound(vec![
                            ("wild", string("minecraft:wolf/baby_wild")),
                            ("tame", string("minecraft:wolf/baby_tame")),
                            ("angry", string("minecraft:wolf/baby_angry")),
                        ]),
                    ),
                ]),
            ),
            (
                "minecraft:pig_variant",
                r#"{"asset_id":"pig/adult","baby_asset_id":"pig/baby","spawn_conditions":[]}"#,
                compound(vec![
                    ("asset_id", string("minecraft:pig/adult")),
                    ("baby_asset_id", string("minecraft:pig/baby")),
                ]),
            ),
            (
                "minecraft:frog_variant",
                r#"{"asset_id":"frog/adult","spawn_conditions":[]}"#,
                compound(vec![("asset_id", string("minecraft:frog/adult"))]),
            ),
            (
                "minecraft:cat_variant",
                r#"{"asset_id":"cat/adult","baby_asset_id":"cat/baby","spawn_conditions":[]}"#,
                compound(vec![
                    ("asset_id", string("minecraft:cat/adult")),
                    ("baby_asset_id", string("minecraft:cat/baby")),
                ]),
            ),
            (
                "minecraft:cow_variant",
                r#"{"asset_id":"cow/adult","baby_asset_id":"cow/baby","spawn_conditions":[]}"#,
                compound(vec![
                    ("asset_id", string("minecraft:cow/adult")),
                    ("baby_asset_id", string("minecraft:cow/baby")),
                ]),
            ),
            (
                "minecraft:chicken_variant",
                r#"{"asset_id":"chicken/adult","baby_asset_id":"chicken/baby","spawn_conditions":[]}"#,
                compound(vec![
                    ("asset_id", string("minecraft:chicken/adult")),
                    ("baby_asset_id", string("minecraft:chicken/baby")),
                ]),
            ),
            (
                "minecraft:zombie_nautilus_variant",
                r#"{"asset_id":"nautilus/adult","spawn_conditions":[]}"#,
                compound(vec![("asset_id", string("minecraft:nautilus/adult"))]),
            ),
        ];

        for (registry, json, expected) in cases {
            assert_eq!(encoded_direct(registry, json), expected, "{registry}");
        }
    }

    #[test]
    fn both_network_sources_skip_spawn_conditions_without_decoding_them() {
        let cases = [
            (
                "minecraft:wolf_variant",
                r#"{"assets":{"wild":"a","tame":"b","angry":"c"},"baby_assets":{"wild":"d","tame":"e","angry":"f"}}"#,
            ),
            (
                "minecraft:pig_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            ("minecraft:frog_variant", r#"{"asset_id":"a"}"#),
            (
                "minecraft:cat_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            (
                "minecraft:cow_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            (
                "minecraft:chicken_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            ("minecraft:zombie_nautilus_variant", r#"{"asset_id":"a"}"#),
        ];
        for (registry, json) in cases {
            let with_unknown_spawn = json
                .strip_suffix('}')
                .map(|prefix| format!(r#"{prefix},"spawn_conditions":"not decoded"}}"#))
                .unwrap();
            let element = decode_known(registry, &with_unknown_spawn);
            assert!(animal_variant_references(&element).is_empty());
            assert!(animal_variant_registry_codec_references(&element).is_empty());
            let encoded = encode_animal_variant_network(&element).unwrap();
            let NbtSummaryValue::Compound(fields) = &encoded else {
                panic!("network value was not compound")
            };
            assert!(fields.iter().all(|field| field.name != "spawn_conditions"));

            let network = RegistryElementData::from_network_nbt(encoded);
            let element = decode_animal_variant_element(registry, &network).unwrap();
            assert!(animal_variant_references(&element).is_empty());
            assert!(animal_variant_registry_codec_references(&element).is_empty());
        }
    }

    #[test]
    fn model_defaults_are_omitted_and_non_defaults_are_retained() {
        let default = encoded_direct(
            "minecraft:pig_variant",
            r#"{"model":"normal","asset_id":"pig","baby_asset_id":"baby","spawn_conditions":[]}"#,
        );
        let NbtSummaryValue::Compound(default_fields) = default else {
            panic!()
        };
        assert!(default_fields.iter().all(|field| field.name != "model"));

        for (registry, json, expected_model, expected_asset) in [
            (
                "minecraft:pig_variant",
                r#"{"model":"cold","asset_id":"pig","baby_asset_id":"baby","spawn_conditions":[]}"#,
                "cold",
                "minecraft:pig",
            ),
            (
                "minecraft:cow_variant",
                r#"{"model":"warm","asset_id":"cow","baby_asset_id":"baby","spawn_conditions":[]}"#,
                "warm",
                "minecraft:cow",
            ),
            (
                "minecraft:chicken_variant",
                r#"{"model":"cold","asset_id":"chicken","baby_asset_id":"baby","spawn_conditions":[]}"#,
                "cold",
                "minecraft:chicken",
            ),
            (
                "minecraft:zombie_nautilus_variant",
                r#"{"model":"warm","asset_id":"nautilus","spawn_conditions":[]}"#,
                "warm",
                "minecraft:nautilus",
            ),
        ] {
            let NbtSummaryValue::Compound(fields) = encoded_direct(registry, json) else {
                panic!()
            };
            assert_eq!(
                encoded_last_field(&fields, "model"),
                Some(&string(expected_model))
            );
            assert_eq!(
                encoded_last_field(&fields, "asset_id"),
                Some(&string(expected_asset))
            );
        }
    }

    #[test]
    fn biome_and_structure_element_and_tag_references_are_retained() {
        let element = decode_direct(
            "minecraft:cat_variant",
            r##"{
                "asset_id":"cat","baby_asset_id":"kitten",
                "spawn_conditions":[
                    {"priority":3,"condition":{"type":"biome","biomes":["forest","example:grove"]}},
                    {"priority":2,"condition":{"type":"structure","structures":"#village"}},
                    {"priority":1,"condition":{"type":"biome","biomes":"desert"}}
                ]
            }"##,
        );
        let references = animal_variant_references(&element);
        assert_eq!(
            references,
            vec![
                AnimalVariantReference {
                    target_registry: "minecraft:worldgen/biome",
                    reference: &HolderReference::Element(RegistryResourceIdentifier(
                        "minecraft:forest".into()
                    )),
                },
                AnimalVariantReference {
                    target_registry: "minecraft:worldgen/biome",
                    reference: &HolderReference::Element(RegistryResourceIdentifier(
                        "example:grove".into()
                    )),
                },
                AnimalVariantReference {
                    target_registry: "minecraft:worldgen/structure",
                    reference: &HolderReference::Tag(RegistryResourceIdentifier(
                        "minecraft:village".into()
                    )),
                },
                AnimalVariantReference {
                    target_registry: "minecraft:worldgen/biome",
                    reference: &HolderReference::Element(RegistryResourceIdentifier(
                        "minecraft:desert".into()
                    )),
                },
            ]
        );
    }
}
