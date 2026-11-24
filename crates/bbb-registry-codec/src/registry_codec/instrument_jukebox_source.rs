//! Instrument publication reuses the persistent direct owner; JukeboxSong owns
//! its direct registry record.

use crate::component_source_codec::decode_component_source;
use crate::registry_codec::recursive_values::{
    required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
};
use bbb_protocol::{
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::{synchronized_registry_element_codec, RegistryCodecError, RegistryElementData};

const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";
const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JukeboxSongRegistryValue {
    pub length_in_seconds_bits: u32,
    pub comparator_output: i32,
}

/// The fields a server reads from an already validated song publication.
pub fn jukebox_song_registry_value(value: &NbtSummaryValue) -> Option<JukeboxSongRegistryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
    };
    let NbtSummaryValue::Float(length) = field("length_in_seconds")? else {
        return None;
    };
    let NbtSummaryValue::Int(comparator_output) = field("comparator_output")? else {
        return None;
    };
    Some(JukeboxSongRegistryValue {
        length_in_seconds_bits: *length,
        comparator_output: *comparator_output,
    })
}

pub(crate) fn decode_instrument_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(INSTRUMENT_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(INSTRUMENT_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    crate::persistent_component_nbt::source_codec::decode_instrument_direct_source(
        data.root(),
        "<root>",
        &mut context,
    )
}

#[cfg(test)]
pub(crate) fn decode_instrument_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_instrument_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decode_jukebox_song_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(JUKEBOX_SONG_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(JUKEBOX_SONG_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    let root = data.root();
    context.charge_record(root, "<root>", "direct record")?;

    let sound_field = "<root>.sound_event";
    let sound_input = required_field(root, "sound_event", "<root>", &context)?;
    let sound = context.with_nested(sound_field, |context| {
        crate::persistent_component_nbt::source_codec::decode_sound_event_holder_source(
            sound_input,
            sound_field,
            context,
        )
    })?;
    let (sound, mut references) = sound.into_parts();

    let mut fields = vec![entry("sound_event", sound)];
    let description_field = "<root>.description";
    let description = required_field(root, "description", "<root>", &context)?;
    let description = context.with_nested(description_field, |context| {
        decode_component_source(description, description_field, context)
    })?;
    let (description, description_refs) = description.into_parts();
    fields.push(entry("description", description));
    references.extend(description_refs);
    let length_field = "<root>.length_in_seconds";
    context.charge_node(length_field)?;
    let length = required_field(root, "length_in_seconds", "<root>", &context)?
        .codec_f32()
        .ok_or_else(|| context.codec_error(length_field, "expected Codec.FLOAT input"))?;
    if !length.is_finite() || length <= 0.0 {
        return Err(context.codec_error(length_field, "float must be positive and finite"));
    }
    fields.push(entry(
        "length_in_seconds",
        NbtSummaryValue::Float(canonical_float_tag_bits(length)),
    ));
    let comparator_field = "<root>.comparator_output";
    context.charge_node(comparator_field)?;
    let comparator = required_field(root, "comparator_output", "<root>", &context)?
        .codec_i32()
        .ok_or_else(|| context.codec_error(comparator_field, "expected Codec.INT input"))?;
    if !(0..=15).contains(&comparator) {
        return Err(context.codec_error(comparator_field, "comparator output is outside 0..=15"));
    }
    fields.push(entry("comparator_output", NbtSummaryValue::Int(comparator)));
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

#[cfg(test)]
pub(crate) fn decode_jukebox_song_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_jukebox_song_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[cfg(test)]
mod tests {
    use crate::{decode_client_known_pack_registry_json, decode_direct_registry_json};

    use super::*;
    use crate::registry_codec::RegistryReferenceKind;

    const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";

    #[test]
    fn audio_records_write_direct_field_order_and_retain_references() {
        let instrument = decode_direct_registry_json(
            br#"{
                "sound_event":"minecraft:item.goat_horn.sound.0",
                "use_duration":7.0,
                "range":128.0,
                "description":{"text":"Horn","click_event":{"action":"show_dialog","dialog":"example:next"}},
                "ignored":true
            }"#,
        )
        .unwrap();
        let decoded = decode_instrument_element(&instrument).unwrap();
        let NbtSummaryValue::Compound(fields) = decoded.network_nbt_unchecked() else {
            panic!("instrument is not a compound")
        };
        assert_eq!(
            fields.iter().map(|field| &field.name).collect::<Vec<_>>(),
            ["sound_event", "use_duration", "range", "description"]
        );
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].target_registry, SOUND_EVENT_REGISTRY);
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[1].target_registry, "minecraft:dialog");

        let jukebox = decode_client_known_pack_registry_json(
            br#"{
                "sound_event":{"sound_id":"example:inline","range":"bad"},
                "description":"Song",
                "length_in_seconds":12.5,
                "comparator_output":15
            }"#,
        )
        .unwrap();
        let decoded = decode_jukebox_song_element(&jukebox).unwrap();
        assert!(decoded.references().next().is_none());
        let NbtSummaryValue::Compound(fields) = decoded.network_nbt_unchecked() else {
            panic!("jukebox song is not a compound")
        };
        assert_eq!(
            fields.iter().map(|field| &field.name).collect::<Vec<_>>(),
            [
                "sound_event",
                "description",
                "length_in_seconds",
                "comparator_output"
            ]
        );
        assert!(matches!(
            &fields[0].value,
            NbtSummaryValue::Compound(sound) if sound.len() == 1
        ));
    }
}
