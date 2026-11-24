//! The dynamic-registry generation a projection has to name holders
//! against, the registry identifiers both directions key on, and the
//! `RegistryFileCodec`-shaped sound-event value every sound-bearing component
//! shares.

use crate::registry_codec::{
    sound_variants::{decode_sound_event_holder, SoundEventHolder},
    RegistryElementValueRef,
};
use bbb_facts::block_types::vanilla_block_type_for_resource_id;
use bbb_facts::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_protocol::component::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, StyledTextRun,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::{
    HolderSetSummary, InstrumentSummary, SoundEventSummary, TrimMaterialSummary, TrimPatternSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::nbt_support::{
    latest_field, normalize_resource_id_units, persistent_compact_holder_set_nbt,
    persistent_float_nbt, project_compact_holder_set, ComponentReader,
};

pub(super) const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
pub(super) const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
pub(super) const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";
pub(super) const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";
pub(super) const BLOCK_REGISTRY: &str = "minecraft:block";
pub(super) const ITEM_REGISTRY: &str = "minecraft:item";
pub(super) const MOB_EFFECT_REGISTRY: &str = "minecraft:mob_effect";
pub(super) const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";
pub(super) const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";
pub(super) const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";

/// Resolves a named persistent dynamic-registry holder to the raw id published
/// for the current client connection.
pub trait PersistentItemStackHolderResolver: PersistentRegistryResolver {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32>;

    /// The inverse of [`Self::holder_id`] over the same publication, which the
    /// encode direction needs because a persistent registry codec writes the
    /// element's identifier rather than its id
    ///
    /// Fails closed by default: a resolver that cannot name its publication has
    /// no identifier to write.
    fn holder_resource_id(&self, _registry_id: &str, _holder_id: i32) -> Option<String> {
        None
    }

    /// Session content registry carrying custom modded items, blocks, entities,
    /// sound events, and particles. Defaults to the Vanilla-only registry.
    fn content_registry(&self) -> &ContentRegistry {
        ContentRegistry::vanilla()
    }

    fn cat_sound_variant(
        &self,
        _holder_id: i32,
    ) -> Option<&crate::registry_codec::sound_variants::CatSoundVariantElement> {
        None
    }

    fn cow_sound_variant(
        &self,
        _resource_id: &str,
    ) -> Option<&crate::registry_codec::sound_variants::CowSoundVariantElement> {
        None
    }

    fn chicken_sound_variant(
        &self,
        _resource_id: &str,
    ) -> Option<&crate::registry_codec::sound_variants::ChickenSoundVariantElement> {
        None
    }

    fn pig_sound_variant(
        &self,
        _resource_id: &str,
    ) -> Option<&crate::registry_codec::sound_variants::PigSoundVariantElement> {
        None
    }

    fn wolf_sound_variant(
        &self,
        _resource_id: &str,
    ) -> Option<&crate::registry_codec::sound_variants::WolfSoundVariantElement> {
        None
    }

    fn wolf_sound_variant_count(&self) -> Option<usize> {
        None
    }

    /// `PaintingVariant.width`/`height` of the published element.
    fn painting_variant_size(&self, _resource_id: &str) -> Option<[i32; 2]> {
        None
    }

    fn jukebox_song(
        &self,
        _holder_id: i32,
    ) -> Option<crate::registry_codec::JukeboxSongRegistryValue> {
        None
    }

    fn tag_holder_ids(
        &self,
        registry_id: &str,
        tag_id: &str,
    ) -> std::result::Result<Vec<i32>, String> {
        match self.tag_members(registry_id, tag_id) {
            PersistentRegistryTagMembers::Present(members) => members
                .into_iter()
                .map(|member| {
                    self.holder_id(registry_id, &member)
                        .ok_or_else(|| format!("missing {registry_id} resource {member}"))
                })
                .collect(),
            PersistentRegistryTagMembers::Absent => {
                Err(format!("missing {registry_id} tag #{tag_id}"))
            }
            PersistentRegistryTagMembers::Unavailable => Err(format!(
                "{registry_id} holder set cannot be decoded without registry context"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RegistryAccessRevision {
    pub session: u64,
    pub contents: u64,
    pub tags: u64,
}

pub struct UnavailableHolderResolver;

impl PersistentRegistryResolver for UnavailableHolderResolver {
    fn registry_available(&self, _: &str) -> bool {
        false
    }

    fn resource_membership(
        &self,
        _registry_id: &str,
        _resource_id: &str,
    ) -> PersistentRegistryMembership {
        PersistentRegistryMembership::Unavailable
    }

    fn tag_members(&self, _registry_id: &str, _tag_id: &str) -> PersistentRegistryTagMembers {
        PersistentRegistryTagMembers::Unavailable
    }
}

impl PersistentItemStackHolderResolver for UnavailableHolderResolver {
    fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
        None
    }
}

/// A registry element the publication that issued the summary's holder ids
/// cannot name has no identifier to write, which is the same refusal
/// `minecraft:trim`'s two holders raise.
pub(super) fn registered_holder_name(
    type_id: i32,
    resource_id: &'static str,
    registry_id: &'static str,
    holder_id: i32,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<String> {
    let name = holder_resource_id(registry_id, holder_id).ok_or(
        ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
            registry_id,
            holder_id,
        },
    )?;
    bbb_protocol::normalize_resource_location(name)
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))
}

/// `RegistryCodecs.homogeneousList(Registries.DAMAGE_TYPE)`
/// is a `HolderSetCodec` over
/// `RegistryFixedCodec`, so every damage-type holder set in this module --
/// `minecraft:damage_resistant`'s `types` and `minecraft:blocks_attacks`'
/// `bypassed_by` and `damage_reductions[].type` -- writes the `"#tag"` id or
/// the compact element list [`persistent_compact_holder_set_nbt`] writes, and
/// names each element through the publication that issued its id.
pub(super) fn persistent_damage_type_holder_set_nbt(
    type_id: i32,
    resource_id: &'static str,
    registry_ids: &[i32],
    tag: Option<&str>,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let names = registry_ids.iter().map(|&holder_id| {
        registered_holder_name(
            type_id,
            resource_id,
            DAMAGE_TYPE_REGISTRY,
            holder_id,
            holder_resource_id,
        )
    });
    persistent_compact_holder_set_nbt(type_id, resource_id, tag, names)
}

/// The inverse of [`persistent_damage_type_holder_set_nbt`].
pub(super) fn project_damage_type_holder_set(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
) -> Result<HolderSetSummary> {
    let (registry_ids, tag) =
        project_compact_holder_set(ComponentReader::new(type_id, resource_id), value, |named| {
            resolve_holder_id(
                DAMAGE_TYPE_REGISTRY,
                &normalize_resource_id_units(named.units())?,
                holder_id,
            )
        })?;
    Ok(HolderSetSummary { registry_ids, tag })
}

pub(super) fn item_resource_id_in_registry(
    item_id: i32,
    content_registry: &ContentRegistry,
) -> Option<&str> {
    bbb_facts::item_types::vanilla_item_type(item_id)
        .map(|item| item.resource_id)
        .or_else(|| {
            content_registry
                .item_by_id(item_id)
                .map(|item| item.resource_id.as_str())
        })
}

pub(super) fn item_id_in_registry(
    resource_id: &str,
    content_registry: &ContentRegistry,
) -> Option<i32> {
    bbb_facts::item_types::vanilla_item_type_for_resource_id(resource_id)
        .map(|item| item.id)
        .or_else(|| {
            content_registry
                .item_by_resource_id(resource_id)
                .map(|item| item.id)
        })
}

pub(super) fn block_resource_id_in_registry(
    block_id: i32,
    content_registry: &ContentRegistry,
) -> Option<&str> {
    bbb_facts::block_types::vanilla_block_type(block_id)
        .map(|block| block.resource_id)
        .or_else(|| {
            content_registry
                .block_by_id(block_id)
                .map(|block| block.resource_id.as_str())
        })
}

pub(super) fn block_id_in_registry(
    resource_id: &str,
    content_registry: &ContentRegistry,
) -> Option<i32> {
    vanilla_block_type_for_resource_id(resource_id)
        .map(|block| block.id)
        .or_else(|| {
            content_registry
                .block_by_resource_id(resource_id)
                .map(|block| block.block_id)
        })
}

pub(super) fn entity_resource_id_in_registry(
    entity_id: i32,
    content_registry: &ContentRegistry,
) -> Option<&str> {
    bbb_facts::entity_types::vanilla_entity_type(entity_id)
        .map(|entity| entity.resource_id)
        .or_else(|| {
            content_registry
                .entity_by_id(entity_id)
                .map(|entity| entity.resource_id.as_str())
        })
}

pub(super) fn entity_id_in_registry(
    resource_id: &str,
    content_registry: &ContentRegistry,
) -> Option<i32> {
    bbb_facts::entity_types::vanilla_entity_type_for_resource_id(resource_id)
        .map(|entity| entity.id)
        .or_else(|| {
            content_registry
                .entity_by_resource_id(resource_id)
                .map(|entity| entity.id)
        })
}

pub(super) fn sound_event_resource_id_in_registry(
    sound_id: i32,
    content_registry: &ContentRegistry,
) -> Option<&str> {
    bbb_facts::sound_event_types::vanilla_sound_event_resource_id(sound_id).or_else(|| {
        content_registry
            .sound_event_by_id(sound_id)
            .map(|sound| sound.resource_id.as_str())
    })
}

pub(super) fn sound_event_id_in_registry(
    resource_id: &str,
    content_registry: &ContentRegistry,
) -> Option<i32> {
    bbb_facts::sound_event_types::vanilla_sound_event_registry_id(resource_id).or_else(|| {
        content_registry
            .sound_event_by_resource_id(resource_id)
            .map(|sound| sound.id)
    })
}

pub(super) fn project_block_holder_set(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<HolderSetSummary> {
    let (registry_ids, tag) =
        project_compact_holder_set(ComponentReader::new(type_id, resource_id), value, |raw| {
            block_holder_id(type_id, resource_id, raw, content_registry)
        })?;
    Ok(HolderSetSummary { registry_ids, tag })
}

fn block_holder_id(
    type_id: i32,
    resource_id: &'static str,
    raw: &bbb_protocol::JavaString,
    content_registry: &ContentRegistry,
) -> Result<i32> {
    let block = normalize_resource_id_units(raw.units())?;
    block_id_in_registry(&block, content_registry).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            format!("unknown {BLOCK_REGISTRY} holder {block}"),
        )
    })
}

pub(super) fn resolve_holder_id(
    registry_id: &'static str,
    resource_id: &str,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
) -> Result<i32> {
    holder_id(registry_id, resource_id)
        .filter(|holder_id| *holder_id >= 0)
        .ok_or_else(
            || ItemStackPersistentProjectionError::UnresolvedRegistryHolder {
                registry_id,
                resource_id: resource_id.to_string(),
            },
        )
}

/// `SoundEvent.CODEC` is `RegistryFileCodec.create(Registries.SOUND_EVENT,
/// DIRECT_CODEC)`, which writes a writer-bound holder as
/// the element's identifier and a direct one as `{sound_id, range?}`
///
/// Every component that names a sound event by holder (`minecraft:break_sound`,
/// `minecraft:piercing_weapon`, `minecraft:consumable`) reaches this one
/// encoder, so the save form cannot drift between them.
pub(super) fn persistent_sound_event_nbt(
    type_id: i32,
    resource_id: &'static str,
    sound: &SoundEventSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if let Some(registry_id) = sound.registry_id {
        let named = sound_event_resource_id_in_registry(registry_id, content_registry)
            .ok_or_else(|| invalid(format!("unknown sound event holder id {registry_id}")))?;
        return Ok(NbtSummaryValue::String(named.into()));
    }
    let sound_id = sound
        .sound_id
        .as_deref()
        .ok_or_else(|| invalid("direct sound event carries no id".to_string()))?;
    let sound_id = bbb_protocol::normalize_resource_location(sound_id)
        .map_err(|error| invalid(error.to_string()))?;
    let mut fields = vec![NbtSummaryEntry {
        name: "sound_id".into(),
        value: NbtSummaryValue::String(sound_id.into()),
    }];
    if let Some(bits) = sound.fixed_range_bits {
        fields.push(NbtSummaryEntry {
            name: "range".into(),
            value: persistent_float_nbt(bits),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// The inverse of [`persistent_sound_event_nbt`], which is also what a real
/// save decodes into: a string names a holder, a compound is a direct event.
pub fn project_sound_event<'a>(
    type_id: i32,
    resource_id: &'static str,
    value: impl Into<RegistryElementValueRef<'a>>,
) -> Result<SoundEventSummary> {
    project_sound_event_with_content_registry(
        type_id,
        resource_id,
        value,
        ContentRegistry::vanilla(),
    )
}

pub(super) fn project_sound_event_with_content_registry<'a>(
    type_id: i32,
    resource_id: &'static str,
    value: impl Into<RegistryElementValueRef<'a>>,
    content_registry: &ContentRegistry,
) -> Result<SoundEventSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let holder = decode_sound_event_holder(
        value.into(),
        SOUND_EVENT_REGISTRY,
        "SoundEvent.CODEC",
        resource_id,
    )
    .map_err(|error| invalid(error.to_string()))?;
    Ok(match holder {
        SoundEventHolder::Named(named) => SoundEventSummary {
            registry_id: Some(
                sound_event_id_in_registry(named.as_str(), content_registry).ok_or_else(|| {
                    invalid(format!(
                        "unknown {SOUND_EVENT_REGISTRY} holder {}",
                        named.as_str()
                    ))
                })?,
            ),
            sound_id: None,
            fixed_range_bits: None,
        },
        SoundEventHolder::Inline(sound) => SoundEventSummary {
            registry_id: None,
            sound_id: Some(sound.sound_id.into_string()),
            fixed_range_bits: sound.range.map(|range| range.0),
        },
    })
}

fn project_description(
    c: ComponentReader,
    fields: &[NbtSummaryEntry],
) -> Result<(String, Box<[StyledTextRun]>, Box<NbtSummaryValue>)> {
    let description = c.field(fields, "description")?;
    c.text(description, "description")?;
    let styled = styled_component_runs_from_nbt_value(description);
    Ok((
        styled_runs_summary_text(&styled),
        styled.into_boxed_slice(),
        Box::new(description.clone()),
    ))
}

/// `ExtraCodecs.POSITIVE_FLOAT`.
fn positive_float(c: ComponentReader, fields: &[NbtSummaryEntry], name: &str) -> Result<u32> {
    let value = c.float(c.field(fields, name)?, name)?;
    if !(value > 0.0 && value <= f32::MAX) {
        return Err(c.error(format!("{name} {value} is not positive")));
    }
    Ok(value.to_bits())
}

/// An inline `TrimMaterial`.
pub(super) fn project_inline_trim_material(
    c: ComponentReader,
    value: &NbtSummaryValue,
) -> Result<TrimMaterialSummary> {
    let fields = c.compound(value, "trim material")?;
    let (description, styled, nbt) = project_description(c, fields)?;
    let mut override_armor_assets = std::collections::BTreeMap::new();
    if let Some(overrides) = latest_field(fields, "override_armor_assets") {
        for entry in c.compound(overrides, "asset overrides")? {
            let key = c.resource_units(entry.name.units(), "equipment asset")?;
            let suffix = project_trim_asset_path(c, &entry.value, "asset override")?;
            if override_armor_assets.insert(key, suffix).is_some() {
                return Err(c.error("duplicate equipment asset"));
            }
        }
    }
    Ok(TrimMaterialSummary {
        asset_name: project_trim_asset_path(c, c.field(fields, "asset_name")?, "asset name")?,
        override_armor_assets,
        description,
        description_styled: Some(styled),
        description_nbt: Some(nbt),
    })
}

fn project_trim_asset_path(
    c: ComponentReader,
    value: &NbtSummaryValue,
    what: &str,
) -> Result<String> {
    let path = c
        .string(value, what)?
        .try_to_string()
        .map_err(|_| c.error(format!("{what} is not a resource path")))?;
    if !path
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'/' | b'.'))
    {
        return Err(c.error(format!("{what} is not a resource path")));
    }
    Ok(path)
}

/// An inline `TrimPattern`.
pub(super) fn project_inline_trim_pattern(
    c: ComponentReader,
    value: &NbtSummaryValue,
) -> Result<TrimPatternSummary> {
    let fields = c.compound(value, "trim pattern")?;
    let (description, styled, nbt) = project_description(c, fields)?;
    Ok(TrimPatternSummary {
        asset_id: c.resource(c.field(fields, "asset_id")?, "asset id")?,
        description,
        description_styled: Some(styled),
        description_nbt: Some(nbt),
        // `Codec.BOOL.fieldOf("decal").orElse(false)` absorbs any failure.
        decal: latest_field(fields, "decal")
            .and_then(|value| crate::registry_codec::RegistryElementValueRef::Nbt(value).as_bool())
            .unwrap_or(false),
    })
}

/// An inline `Instrument`.
pub(super) fn project_inline_instrument(
    c: ComponentReader,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<InstrumentSummary> {
    let fields = c.compound(value, "instrument")?;
    let (description, styled, nbt) = project_description(c, fields)?;
    Ok(InstrumentSummary {
        sound_event: project_sound_event_with_content_registry(
            c.type_id,
            c.resource_id,
            c.field(fields, "sound_event")?,
            content_registry,
        )?,
        use_duration_bits: positive_float(c, fields, "use_duration")?,
        range_bits: positive_float(c, fields, "range")?,
        description,
        description_styled: Some(styled),
        description_nbt: Some(nbt),
    })
}

fn inline_description_nbt(
    type_id: i32,
    resource_id: &'static str,
    nbt: Option<&NbtSummaryValue>,
) -> Result<NbtSummaryValue> {
    let nbt = nbt.ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "inline holder description lacks its Component NBT".to_string(),
        )
    })?;
    ComponentReader::new(type_id, resource_id).text(nbt, "description")?;
    Ok(nbt.clone())
}

fn validate_trim_asset_path(
    type_id: i32,
    resource_id: &'static str,
    path: &str,
    what: &str,
) -> Result<()> {
    if !path
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'/' | b'.'))
    {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("{what} is not a resource path"),
        ));
    }
    Ok(())
}

/// `RegistryFileCodec.encode` writes a direct holder through its element codec
pub(super) fn persistent_inline_trim_material_nbt(
    type_id: i32,
    resource_id: &'static str,
    material: &TrimMaterialSummary,
) -> Result<NbtSummaryValue> {
    validate_trim_asset_path(type_id, resource_id, &material.asset_name, "asset name")?;
    let mut fields = vec![NbtSummaryEntry {
        name: "asset_name".into(),
        value: NbtSummaryValue::string(&material.asset_name),
    }];
    if !material.override_armor_assets.is_empty() {
        let mut overrides = Vec::with_capacity(material.override_armor_assets.len());
        for (asset, suffix) in &material.override_armor_assets {
            let asset =
                bbb_protocol::normalize_resource_location(asset.as_str()).map_err(|error| {
                    invalid_persistent_component(type_id, resource_id, error.to_string())
                })?;
            validate_trim_asset_path(type_id, resource_id, suffix, "asset override")?;
            overrides.push(NbtSummaryEntry {
                name: asset.into(),
                value: NbtSummaryValue::string(suffix),
            });
        }
        fields.push(NbtSummaryEntry {
            name: "override_armor_assets".into(),
            value: NbtSummaryValue::Compound(overrides),
        });
    }
    fields.push(NbtSummaryEntry {
        name: "description".into(),
        value: inline_description_nbt(type_id, resource_id, material.description_nbt.as_deref())?,
    });
    Ok(NbtSummaryValue::Compound(fields))
}

pub(super) fn persistent_inline_trim_pattern_nbt(
    type_id: i32,
    resource_id: &'static str,
    pattern: &TrimPatternSummary,
) -> Result<NbtSummaryValue> {
    let asset_id = bbb_protocol::normalize_resource_location(pattern.asset_id.as_str())
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))?;
    let mut fields = vec![
        NbtSummaryEntry {
            name: "asset_id".into(),
            value: NbtSummaryValue::string(&asset_id),
        },
        NbtSummaryEntry {
            name: "description".into(),
            value: inline_description_nbt(
                type_id,
                resource_id,
                pattern.description_nbt.as_deref(),
            )?,
        },
    ];
    if pattern.decal {
        fields.push(NbtSummaryEntry {
            name: "decal".into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

pub(super) fn persistent_inline_instrument_nbt(
    type_id: i32,
    resource_id: &'static str,
    instrument: &InstrumentSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    for (name, bits) in [
        ("use_duration", instrument.use_duration_bits),
        ("range", instrument.range_bits),
    ] {
        let value = f32::from_bits(bits);
        if !(value > 0.0 && value <= f32::MAX) {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!("{name} {value} is not positive"),
            ));
        }
    }
    Ok(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "sound_event".into(),
            value: persistent_sound_event_nbt(
                type_id,
                resource_id,
                &instrument.sound_event,
                content_registry,
            )?,
        },
        NbtSummaryEntry {
            name: "use_duration".into(),
            value: persistent_float_nbt(instrument.use_duration_bits),
        },
        NbtSummaryEntry {
            name: "range".into(),
            value: persistent_float_nbt(instrument.range_bits),
        },
        NbtSummaryEntry {
            name: "description".into(),
            value: inline_description_nbt(
                type_id,
                resource_id,
                instrument.description_nbt.as_deref(),
            )?,
        },
    ]))
}
