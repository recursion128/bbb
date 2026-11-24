//! Typed projection of environment music and ambient sounds.

use crate::registry_codec::{RegistryCodecError, RegistryElementValueRef};
use crate::{PersistentNbtDecodeLimits, RegistryElementSource};
use bbb_protocol::nbt_summary::{
    checked_int_value, codec_double_value, compound_field as field_value, integer_bool_value,
};
use bbb_protocol::{
    environment_audio::{
        AmbientAddition, AmbientMood, AmbientSounds, BackgroundMusic, Music, SoundEventHolderValue,
    },
    NbtSummaryEntry, NbtSummaryValue,
};

pub fn background_music_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
) -> Result<BackgroundMusic, RegistryCodecError> {
    let value = super::canonical_attribute_source(
        input,
        source,
        limits,
        "minecraft:audio/background_music",
        "EnvironmentAudio",
    )?;
    Ok(background_music_from_canonical(&value)
        .expect("decoded background music has a typed projection"))
}

pub fn ambient_sounds_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
) -> Result<AmbientSounds, RegistryCodecError> {
    let value = super::canonical_attribute_source(
        input,
        source,
        limits,
        "minecraft:audio/ambient_sounds",
        "EnvironmentAudio",
    )?;
    Ok(ambient_sounds_from_canonical(&value)
        .expect("decoded ambient sounds have a typed projection"))
}

pub fn background_music_from_canonical(value: &NbtSummaryValue) -> Option<BackgroundMusic> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    Some(BackgroundMusic {
        default: optional_compound(fields, "default", parse_music)?,
        creative: optional_compound(fields, "creative", parse_music)?,
        underwater: optional_compound(fields, "underwater", parse_music)?,
    })
}

fn parse_music(fields: &[NbtSummaryEntry]) -> Option<Music> {
    let min_delay = checked_int_value(field_value(fields, "min_delay")?)?;
    let max_delay = checked_int_value(field_value(fields, "max_delay")?)?;
    if min_delay < 0 || max_delay < 0 {
        return None;
    }
    Some(Music {
        sound: parse_sound_holder(field_value(fields, "sound")?)?,
        min_delay,
        max_delay,
        replace_current_music: field_value(fields, "replace_current_music")
            .map(integer_bool_value)
            .unwrap_or(Some(false))?,
    })
}

pub fn ambient_sounds_from_canonical(value: &NbtSummaryValue) -> Option<AmbientSounds> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let additions = match field_value(fields, "additions") {
        None => Vec::new(),
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .map(|value| {
                let NbtSummaryValue::Compound(fields) = value else {
                    return None;
                };
                parse_ambient_addition(fields)
            })
            .collect::<Option<Vec<_>>>()?,
        Some(NbtSummaryValue::Compound(fields)) => vec![parse_ambient_addition(fields)?],
        Some(_) => return None,
    };
    Some(AmbientSounds {
        loop_sound: match field_value(fields, "loop") {
            Some(value) => Some(parse_sound_holder(value)?),
            None => None,
        },
        mood: optional_compound(fields, "mood", parse_ambient_mood)?,
        additions,
    })
}

fn parse_ambient_mood(fields: &[NbtSummaryEntry]) -> Option<AmbientMood> {
    let offset = codec_double_value(field_value(fields, "offset")?)?;
    Some(AmbientMood {
        sound: parse_sound_holder(field_value(fields, "sound")?)?,
        tick_delay: checked_int_value(field_value(fields, "tick_delay")?)?,
        block_search_extent: checked_int_value(field_value(fields, "block_search_extent")?)?,
        offset_bits: offset.to_bits(),
    })
}

fn parse_ambient_addition(fields: &[NbtSummaryEntry]) -> Option<AmbientAddition> {
    let tick_chance = codec_double_value(field_value(fields, "tick_chance")?)?;
    Some(AmbientAddition {
        sound: parse_sound_holder(field_value(fields, "sound")?)?,
        tick_chance_bits: tick_chance.to_bits(),
    })
}

fn parse_sound_holder(value: &NbtSummaryValue) -> Option<SoundEventHolderValue> {
    match value {
        NbtSummaryValue::String(resource_id) => Some(SoundEventHolderValue::Reference {
            resource_id: resource_id.try_to_string().ok()?,
        }),
        NbtSummaryValue::Compound(fields) => {
            let NbtSummaryValue::String(sound_id) = field_value(fields, "sound_id")? else {
                return None;
            };
            let range_bits = match field_value(fields, "range") {
                Some(NbtSummaryValue::Float(bits)) => Some(*bits),
                None => None,
                Some(_) => return None,
            };
            Some(SoundEventHolderValue::Direct {
                sound_id: sound_id.try_to_string().ok()?,
                range_bits,
            })
        }
        _ => None,
    }
}

fn optional_compound<T>(
    fields: &[NbtSummaryEntry],
    name: &str,
    decode: impl FnOnce(&[NbtSummaryEntry]) -> Option<T>,
) -> Option<Option<T>> {
    match field_value(fields, name) {
        None => Some(None),
        Some(NbtSummaryValue::Compound(fields)) => decode(fields).map(Some),
        Some(_) => None,
    }
}
