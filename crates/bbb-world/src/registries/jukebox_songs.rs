use bbb_protocol::{
    decode_nbt_summary, styled_component_runs_from_nbt_value, styled_runs_summary_text,
    validate_component_nbt, NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

use super::normalize_codec_identifier;

pub const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";

/// The persistent `SoundEvent.CODEC` value embedded in one synchronized song.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JukeboxSoundEvent {
    Reference {
        resource_id: String,
    },
    Direct {
        resource_id: String,
        fixed_range: Option<f32>,
    },
}

impl JukeboxSoundEvent {
    pub fn resource_id(&self) -> &str {
        match self {
            Self::Reference { resource_id } | Self::Direct { resource_id, .. } => resource_id,
        }
    }

    pub fn fixed_range(&self) -> Option<f32> {
        match self {
            Self::Reference { .. } => None,
            Self::Direct { fixed_range, .. } => *fixed_range,
        }
    }
}

/// Client-consumed fields from vanilla 26.1 `JukeboxSong.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JukeboxSongPresentation {
    pub sound_event: JukeboxSoundEvent,
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    pub description_nbt: NbtSummaryValue,
    pub length_in_seconds: f32,
    pub comparator_output: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JukeboxSongProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: &'static str,
}

impl std::fmt::Display for JukeboxSongProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.resource_id, self.field, self.reason
        )
    }
}

impl WorldStore {
    /// Projects a synchronized song through the current session holder order.
    ///
    /// `None` means the server omitted the registry, the holder id is unknown,
    /// or the selected entry is a data-less stub. Static fallback and stub
    /// completion remain consumer-owned until a payload is available; this API
    /// never substitutes a vanilla ordinal for server-supplied contents.
    pub fn synchronized_jukebox_song_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<JukeboxSongPresentation>, JukeboxSongProjectionError> {
        let Some(entry) = self.registry_entry_by_holder_id(JUKEBOX_SONG_REGISTRY, holder_id) else {
            return Ok(None);
        };
        let Some(raw_data) = entry.raw_data() else {
            return Ok(None);
        };
        decode_jukebox_song(raw_data)
            .map(Some)
            .map_err(|failure| JukeboxSongProjectionError {
                resource_id: entry.id.clone(),
                field: failure.field.to_string(),
                reason: failure.reason,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodeFailure {
    field: &'static str,
    reason: &'static str,
}

fn decode_jukebox_song(raw_data: &[u8]) -> Result<JukeboxSongPresentation, DecodeFailure> {
    let root = decode_nbt_summary(raw_data)
        .map_err(|_| failure("<root>", "invalid NBT"))?
        .ok_or_else(|| failure("<root>", "missing root compound"))?;
    let NbtSummaryValue::Compound(entries) = root else {
        return Err(failure("<root>", "expected compound"));
    };
    let sound_event = decode_sound_event(required_field(&entries, "sound_event")?)?;
    let description_nbt = required_field(&entries, "description")?.clone();
    validate_component_nbt(&description_nbt, "jukebox song description")
        .map_err(|_| failure("description", "invalid Component"))?;
    let description_styled = styled_component_runs_from_nbt_value(&description_nbt);
    let description = styled_runs_summary_text(&description_styled);
    let length_in_seconds = required_float(&entries, "length_in_seconds")?;
    if !(length_in_seconds > 0.0 && length_in_seconds <= f32::MAX) {
        return Err(failure(
            "length_in_seconds",
            "float outside (0, Float.MAX_VALUE]",
        ));
    }
    let comparator_output = required_int(&entries, "comparator_output")?;
    if !(0..=15).contains(&comparator_output) {
        return Err(failure("comparator_output", "integer outside 0..=15"));
    }
    Ok(JukeboxSongPresentation {
        sound_event,
        description,
        description_styled,
        description_nbt,
        length_in_seconds,
        comparator_output,
    })
}

fn decode_sound_event(value: &NbtSummaryValue) -> Result<JukeboxSoundEvent, DecodeFailure> {
    match value {
        NbtSummaryValue::String(resource_id) => Ok(JukeboxSoundEvent::Reference {
            resource_id: normalize_codec_identifier(resource_id)
                .ok_or_else(|| failure("sound_event", "invalid reference identifier"))?,
        }),
        NbtSummaryValue::Compound(entries) => {
            let Some(sound_id) = field(entries, "sound_id") else {
                return Err(failure("sound_event.sound_id", "missing required field"));
            };
            let NbtSummaryValue::String(resource_id) = sound_id else {
                return Err(failure("sound_event.sound_id", "expected string"));
            };
            let resource_id = normalize_codec_identifier(resource_id)
                .ok_or_else(|| failure("sound_event.sound_id", "invalid identifier"))?;
            let fixed_range = match field(entries, "range") {
                Some(NbtSummaryValue::Float(bits)) => Some(f32::from_bits(*bits)),
                Some(_) => return Err(failure("sound_event.range", "expected float")),
                None => None,
            };
            Ok(JukeboxSoundEvent::Direct {
                resource_id,
                fixed_range,
            })
        }
        _ => Err(failure(
            "sound_event",
            "expected resource identifier or direct compound",
        )),
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn required_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &'static str,
) -> Result<&'a NbtSummaryValue, DecodeFailure> {
    field(entries, name).ok_or_else(|| failure(name, "missing required field"))
}

fn required_float(entries: &[NbtSummaryEntry], name: &'static str) -> Result<f32, DecodeFailure> {
    match required_field(entries, name)? {
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits)),
        _ => Err(failure(name, "expected float")),
    }
}

fn required_int(entries: &[NbtSummaryEntry], name: &'static str) -> Result<i32, DecodeFailure> {
    match required_field(entries, name)? {
        NbtSummaryValue::Int(value) => Ok(*value),
        _ => Err(failure(name, "expected integer")),
    }
}

const fn failure(field: &'static str, reason: &'static str) -> DecodeFailure {
    DecodeFailure { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_named_and_direct_sound_event_codec_forms() {
        let named = decode_jukebox_song(&song_nbt(
            SoundFixture::Reference("example:records/custom"),
            "Custom song",
            12.5,
            7,
        ))
        .unwrap();
        assert_eq!(
            named.sound_event,
            JukeboxSoundEvent::Reference {
                resource_id: "example:records/custom".to_string(),
            }
        );
        assert_eq!(named.description, "Custom song");
        assert_eq!(named.length_in_seconds, 12.5);
        assert_eq!(named.comparator_output, 7);

        let direct = decode_jukebox_song(&song_nbt(
            SoundFixture::Direct("example:records/direct", Some(24.0)),
            "Direct song",
            1.0,
            0,
        ))
        .unwrap();
        assert_eq!(
            direct.sound_event,
            JukeboxSoundEvent::Direct {
                resource_id: "example:records/direct".to_string(),
                fixed_range: Some(24.0),
            }
        );
    }

    #[test]
    fn synchronized_projection_follows_custom_server_holder_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            JUKEBOX_SONG_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:custom",
                    song_nbt(
                        SoundFixture::Reference("example:music.custom"),
                        "Custom",
                        20.0,
                        3,
                    ),
                ),
                RegistryPacketEntry::with_raw_data(
                    "minecraft:cat",
                    song_nbt(
                        SoundFixture::Reference("minecraft:music_disc.cat"),
                        "Cat",
                        185.0,
                        2,
                    ),
                ),
            ],
        );

        assert_eq!(
            store
                .synchronized_jukebox_song_presentation(0)
                .unwrap()
                .unwrap()
                .sound_event
                .resource_id(),
            "example:music.custom"
        );
        assert_eq!(
            store
                .synchronized_jukebox_song_presentation(1)
                .unwrap()
                .unwrap()
                .sound_event
                .resource_id(),
            "minecraft:music_disc.cat"
        );
    }

    #[test]
    fn stub_unknown_and_absent_registry_do_not_substitute_vanilla_ordinals() {
        let mut store = WorldStore::new();
        assert_eq!(store.synchronized_jukebox_song_presentation(1), Ok(None));
        store.record_registry_entries(
            JUKEBOX_SONG_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("minecraft:cat")],
        );
        assert_eq!(store.synchronized_jukebox_song_presentation(0), Ok(None));
        assert_eq!(store.synchronized_jukebox_song_presentation(1), Ok(None));
    }

    #[test]
    fn malformed_entries_report_resource_and_exact_codec_field() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            JUKEBOX_SONG_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:broken",
                song_nbt(SoundFixture::Reference("example:sound"), "Broken", 0.0, 16),
            )],
        );

        assert_eq!(
            store.synchronized_jukebox_song_presentation(0).unwrap_err(),
            JukeboxSongProjectionError {
                resource_id: "example:broken".to_string(),
                field: "length_in_seconds".to_string(),
                reason: "float outside (0, Float.MAX_VALUE]",
            }
        );

        let invalid_comparator =
            song_nbt(SoundFixture::Reference("example:sound"), "Broken", 1.0, 16);
        assert_eq!(
            decode_jukebox_song(&invalid_comparator).unwrap_err(),
            failure("comparator_output", "integer outside 0..=15")
        );
    }

    enum SoundFixture<'a> {
        Reference(&'a str),
        Direct(&'a str, Option<f32>),
    }

    fn song_nbt(
        sound: SoundFixture<'_>,
        description: &str,
        length_in_seconds: f32,
        comparator_output: i32,
    ) -> Vec<u8> {
        let mut out = vec![10];
        match sound {
            SoundFixture::Reference(resource_id) => {
                write_string(&mut out, "sound_event", resource_id);
            }
            SoundFixture::Direct(resource_id, fixed_range) => {
                out.push(10);
                write_nbt_string(&mut out, "sound_event");
                write_string(&mut out, "sound_id", resource_id);
                if let Some(fixed_range) = fixed_range {
                    write_float(&mut out, "range", fixed_range);
                }
                out.push(0);
            }
        }
        write_string(&mut out, "description", description);
        write_float(&mut out, "length_in_seconds", length_in_seconds);
        write_int(&mut out, "comparator_output", comparator_output);
        out.push(0);
        out
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }

    fn write_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_nbt_string(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_int(out: &mut Vec<u8>, name: &str, value: i32) {
        out.push(3);
        write_nbt_string(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
