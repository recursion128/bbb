use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type_for_resource_id, MobEffectTypeCatalogEntry,
};
use bbb_protocol::nbt_summary::{codec_int_value, compound_last_field as field};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::ActiveMobEffect;

fn int_or(fields: &[NbtSummaryEntry], name: &str, default: i32) -> i32 {
    field(fields, name)
        .and_then(codec_int_value)
        .unwrap_or(default)
}

fn bool_or(fields: &[NbtSummaryEntry], name: &str, default: bool) -> bool {
    field(fields, name)
        .and_then(codec_int_value)
        .map_or(default, |value| value as i8 != 0)
}

// MobEffectInstance.java:413-457: optional fields that fail to decode keep their defaults.
fn decode_details(
    effect: &MobEffectTypeCatalogEntry,
    value: &NbtSummaryValue,
) -> Option<ActiveMobEffect> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    // ExtraCodecs.java:143-147: Number.byteValue, then unsigned conversion.
    let amplifier = i32::from(int_or(fields, "amplifier", 0) as u8);
    let show_particles = bool_or(fields, "show_particles", true);
    Some(ActiveMobEffect::from_catalog(
        effect,
        int_or(fields, "duration", 0),
        amplifier,
        bool_or(fields, "ambient", false),
        show_particles,
        bool_or(fields, "show_icon", show_particles),
        field(fields, "hidden_effect").and_then(|value| decode_details(effect, value)),
    ))
}

// MobEffectInstance.java:30-34; Registry.java:33-44; Identifier.java:43-71.
pub(crate) fn decode_saved_effect(value: &NbtSummaryValue) -> Option<ActiveMobEffect> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let NbtSummaryValue::String(id) = field(fields, "id")? else {
        return None;
    };
    let effect = id
        .try_to_string()
        .ok()
        .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
        .and_then(|id| vanilla_mob_effect_type_for_resource_id(&id))?;
    decode_details(effect, value)
}

pub(crate) fn decode_saved_effect_list(
    value: &NbtSummaryValue,
) -> impl Iterator<Item = ActiveMobEffect> + '_ {
    // Effect entries are compounds; primitive arrays cannot contain a valid entry.
    let values = match value {
        NbtSummaryValue::List(values) => values.as_slice(),
        _ => &[],
    };
    values.iter().filter_map(decode_saved_effect)
}
