//! Typed projection of configured environment particles.

use crate::registry_codec::{RegistryCodecError, RegistryElementValueRef, RegistryReferenceLookup};
use crate::{PersistentItemStackHolderResolver, PersistentNbtDecodeLimits, RegistryElementSource};
use bbb_facts::particle_types::{
    vanilla_particle_type_for_resource_id, VanillaParticleCodecFamily,
};
use bbb_protocol::nbt_summary::{
    checked_int_value, codec_double_value, codec_float_value, compound_field as field_value,
};
use bbb_protocol::{
    content_registry::ContentRegistry,
    environment_particles::{AmbientParticle, ParticleOptionValue, ParticleOptions},
    packets::ItemStackSummary,
    NbtSummaryEntry, NbtSummaryValue,
};

pub fn particle_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
    lookup: Option<&(impl RegistryReferenceLookup + PersistentItemStackHolderResolver)>,
) -> Result<ParticleOptions, RegistryCodecError> {
    let content_registry = lookup.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        PersistentItemStackHolderResolver::content_registry,
    );
    let value =
        particle_nbt_from_source_with_content_registry(input, source, limits, content_registry)?;
    particle_from_canonical_with_content_registry(&value, lookup, content_registry)
}

pub fn particle_nbt_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    particle_nbt_from_source_with_content_registry(
        input,
        source,
        limits,
        ContentRegistry::vanilla(),
    )
}

pub fn particle_nbt_from_source_with_content_registry(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    super::canonical_attribute_source_with_content_registry(
        input,
        source,
        limits,
        "minecraft:visual/default_dripstone_particle",
        "EnvironmentParticles",
        content_registry,
    )
}

pub fn ambient_particles_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
    lookup: Option<&(impl RegistryReferenceLookup + PersistentItemStackHolderResolver)>,
) -> Result<Vec<AmbientParticle>, RegistryCodecError> {
    let content_registry = lookup.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        PersistentItemStackHolderResolver::content_registry,
    );
    let value = super::canonical_attribute_source_with_content_registry(
        input,
        source,
        limits,
        "minecraft:visual/ambient_particles",
        "EnvironmentParticles",
        content_registry,
    )?;
    ambient_particles_from_canonical_with_content_registry(&value, lookup, content_registry)
}

pub fn particle_from_canonical(
    value: &NbtSummaryValue,
    lookup: Option<&(impl RegistryReferenceLookup + PersistentItemStackHolderResolver)>,
) -> Result<ParticleOptions, RegistryCodecError> {
    let content_registry = lookup.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        PersistentItemStackHolderResolver::content_registry,
    );
    particle_from_canonical_with_content_registry(value, lookup, content_registry)
}

pub fn particle_from_canonical_with_content_registry(
    value: &NbtSummaryValue,
    lookup: Option<&(impl RegistryReferenceLookup + PersistentItemStackHolderResolver)>,
    content_registry: &ContentRegistry,
) -> Result<ParticleOptions, RegistryCodecError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(projection_error("expected particle compound"));
    };
    let resource_id = field_value(fields, "type")
        .and_then(|value| RegistryElementValueRef::Nbt(value).java_string_ref())
        .and_then(|value| value.try_to_string().ok())
        .ok_or_else(|| projection_error("invalid particle type"))?;
    let family = vanilla_particle_type_for_resource_id(&resource_id)
        .map(|entry| entry.codec_family)
        .or_else(|| {
            content_registry
                .particle_by_resource_id(&resource_id)
                .map(|_| VanillaParticleCodecFamily::Simple)
        })
        .ok_or_else(|| projection_error("invalid particle type"))?;
    let stack = if family == VanillaParticleCodecFamily::Item {
        let lookup = lookup.ok_or_else(|| {
            projection_error("item particle requires a registry publication").aborting_speculation()
        })?;
        let item = field_value(fields, "item")
            .ok_or_else(|| projection_error("item particle is missing its item"))?;
        let template = crate::item_stack_template_from_source_with_content_registry(
            RegistryElementValueRef::Nbt(item),
            RegistryElementSource::NetworkNbt,
            lookup,
            content_registry,
        )?;
        Some(ItemStackSummary {
            item_id: Some(template.item_id),
            count: template.count,
            component_patch: template.component_patch,
        })
    } else {
        None
    };
    let option = parse_particle_option(family, fields, stack, content_registry)
        .ok_or_else(|| projection_error("invalid canonical particle options"))?;
    Ok(ParticleOptions {
        resource_id,
        fields: fields.clone(),
        option,
    })
}

pub fn ambient_particles_from_canonical(
    value: &NbtSummaryValue,
    lookup: Option<&(impl RegistryReferenceLookup + PersistentItemStackHolderResolver)>,
) -> Result<Vec<AmbientParticle>, RegistryCodecError> {
    let content_registry = lookup.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        PersistentItemStackHolderResolver::content_registry,
    );
    ambient_particles_from_canonical_with_content_registry(value, lookup, content_registry)
}

pub fn ambient_particles_from_canonical_with_content_registry(
    value: &NbtSummaryValue,
    lookup: Option<&(impl RegistryReferenceLookup + PersistentItemStackHolderResolver)>,
    content_registry: &ContentRegistry,
) -> Result<Vec<AmbientParticle>, RegistryCodecError> {
    let NbtSummaryValue::List(values) = value else {
        return Err(projection_error("expected ambient particle list"));
    };
    values
        .iter()
        .map(|value| {
            let NbtSummaryValue::Compound(fields) = value else {
                return Err(projection_error("expected ambient particle compound"));
            };
            let particle = field_value(fields, "particle")
                .ok_or_else(|| projection_error("missing ambient particle"))?;
            let probability = field_value(fields, "probability")
                .and_then(codec_float_value)
                .ok_or_else(|| projection_error("invalid ambient particle probability"))?;
            Ok(AmbientParticle {
                particle: particle_from_canonical_with_content_registry(
                    particle,
                    lookup,
                    content_registry,
                )?,
                probability_bits: probability.to_bits(),
            })
        })
        .collect()
}

fn projection_error(reason: &str) -> RegistryCodecError {
    RegistryCodecError::native_semantic(
        "minecraft:environment_attribute_check",
        "EnvironmentParticles",
        "value",
        reason,
    )
}

fn parse_particle_option(
    family: VanillaParticleCodecFamily,
    fields: &[NbtSummaryEntry],
    stack: Option<ItemStackSummary>,
    content_registry: &ContentRegistry,
) -> Option<ParticleOptionValue> {
    let integer = |name| checked_int_value(field_value(fields, name)?);
    let float = |name| codec_float_value(field_value(fields, name)?);
    match family {
        VanillaParticleCodecFamily::Simple => Some(ParticleOptionValue::Empty),
        VanillaParticleCodecFamily::Block => Some(ParticleOptionValue::Block {
            block_state_id: parse_particle_block_state(
                field_value(fields, "block_state")?,
                content_registry,
            )?,
        }),
        VanillaParticleCodecFamily::Power => Some(ParticleOptionValue::Power {
            power_bits: field_value(fields, "power")
                .map(codec_float_value)
                .unwrap_or(Some(1.0))?
                .to_bits(),
        }),
        VanillaParticleCodecFamily::Dust => Some(ParticleOptionValue::Dust {
            color: integer("color")? as u32,
            scale_bits: float("scale")?.to_bits(),
        }),
        VanillaParticleCodecFamily::DustColorTransition => {
            Some(ParticleOptionValue::DustColorTransition {
                from_color: integer("from_color")? as u32,
                to_color: integer("to_color")? as u32,
                scale_bits: float("scale")?.to_bits(),
            })
        }
        VanillaParticleCodecFamily::Spell => Some(ParticleOptionValue::Spell {
            color: field_value(fields, "color")
                .map(checked_int_value)
                .unwrap_or(Some(-1))? as u32,
            power_bits: field_value(fields, "power")
                .map(codec_float_value)
                .unwrap_or(Some(1.0))?
                .to_bits(),
        }),
        VanillaParticleCodecFamily::Color => Some(ParticleOptionValue::Color {
            argb: integer("color")? as u32,
        }),
        VanillaParticleCodecFamily::SculkCharge => Some(ParticleOptionValue::SculkCharge {
            roll_bits: float("roll")?.to_bits(),
        }),
        VanillaParticleCodecFamily::Item => Some(ParticleOptionValue::Item { stack: stack? }),
        VanillaParticleCodecFamily::Vibration => {
            let NbtSummaryValue::Compound(destination) = field_value(fields, "destination")? else {
                return None;
            };
            Some(ParticleOptionValue::Vibration {
                block_pos: parse_block_pos(field_value(destination, "pos")?)?,
                arrival_in_ticks: integer("arrival_in_ticks")?,
            })
        }
        VanillaParticleCodecFamily::Trail => {
            let NbtSummaryValue::List(target) = field_value(fields, "target")? else {
                return None;
            };
            let [x, y, z] = target.as_slice() else {
                return None;
            };
            Some(ParticleOptionValue::Trail {
                target_bits: [
                    codec_double_value(x)?.to_bits(),
                    codec_double_value(y)?.to_bits(),
                    codec_double_value(z)?.to_bits(),
                ],
                color: integer("color")? as u32,
                duration: integer("duration")?,
            })
        }
        VanillaParticleCodecFamily::Shriek => Some(ParticleOptionValue::Shriek {
            delay: integer("delay")?,
        }),
    }
}

fn parse_particle_block_state(
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Option<i32> {
    use std::collections::BTreeMap;

    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let NbtSummaryValue::String(name) = field_value(fields, "Name")? else {
        return None;
    };
    let mut properties = BTreeMap::new();
    if let Some(value) = field_value(fields, "Properties") {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        for entry in entries {
            let NbtSummaryValue::String(value) = &entry.value else {
                return None;
            };
            properties.insert(
                entry.name.try_to_string().ok()?,
                value.try_to_string().ok()?,
            );
        }
    }
    name.with_utf8(|name| {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            name,
            &properties,
        )
        .or_else(|| {
            properties
                .is_empty()
                .then(|| content_registry.block_by_resource_id(name))
                .flatten()
                .map(|spec| spec.first_state_id)
        })
    })
    .flatten()
}

fn parse_block_pos(value: &NbtSummaryValue) -> Option<[i32; 3]> {
    match value {
        NbtSummaryValue::IntArray(values) => values.as_slice().try_into().ok(),
        _ => None,
    }
}
