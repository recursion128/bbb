use bbb_protocol::{
    jukebox_song_types::vanilla_26_1_jukebox_song_by_holder_id,
    styled_component_runs_from_nbt_value, styled_runs_summary_text, validate_component_nbt,
    NbtSummaryEntry, NbtSummaryValue, RichTextComponent, StyledTextRun,
};
use serde::{Deserialize, Serialize};

use crate::{FrozenRegistryAccess, WorldStore};

use super::{decode_persistent_sound_event, PersistentSoundEvent};

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

/// The exact subset consumed by Vanilla `LevelEventHandler.playJukeboxSong`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JukeboxSongLevelEventPresentation {
    pub sound_event: JukeboxSoundEvent,
    pub description: RichTextComponent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JukeboxSongProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: String,
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
    /// Resolves level-event holder data without allowing a synchronized
    /// session registry to fall back to the fixed Vanilla bootstrap ordinal.
    pub(crate) fn jukebox_song_level_event_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<JukeboxSongLevelEventPresentation>, JukeboxSongProjectionError> {
        if self
            .frozen_registry_access()
            .has_server_contents(JUKEBOX_SONG_REGISTRY)
        {
            let Some(presentation) = self.synchronized_jukebox_song_presentation(holder_id)? else {
                return Ok(None);
            };
            let description = RichTextComponent::try_from_component_nbt(
                presentation.description_nbt,
            )
            .map_err(|reason| JukeboxSongProjectionError {
                resource_id: self
                    .registry_resource_id_by_holder_id(JUKEBOX_SONG_REGISTRY, holder_id)
                    .unwrap_or("<unknown>")
                    .to_string(),
                field: "description".to_string(),
                reason,
            })?;
            return Ok(Some(JukeboxSongLevelEventPresentation {
                sound_event: presentation.sound_event,
                description,
            }));
        }

        let Some(song) = vanilla_26_1_jukebox_song_by_holder_id(holder_id) else {
            return Ok(None);
        };
        let description = RichTextComponent::try_from_component_nbt(translatable_component(
            song.description_translation_key(),
        ))
        .expect("generated jukebox description translation is a valid Component");
        Ok(Some(JukeboxSongLevelEventPresentation {
            sound_event: JukeboxSoundEvent::Reference {
                resource_id: song.sound_event_id.to_string(),
            },
            description,
        }))
    }

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
        if let Some(presentation) = self.registries.typed.jukebox_songs.get(&entry.id) {
            return Ok(Some(presentation.clone()));
        }
        let Some(data) = entry.element_data() else {
            return Ok(None);
        };
        decode_jukebox_song_with_access(&data.value, Some(self.frozen_registry_access()))
            .map(Some)
            .map_err(|failure| JukeboxSongProjectionError {
                resource_id: entry.id.clone(),
                field: failure.field.to_string(),
                reason: failure.reason,
            })
    }
}

fn translatable_component(translation_key: String) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "translate".to_string(),
        value: NbtSummaryValue::String(translation_key),
    }])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DecodeFailure {
    pub field: &'static str,
    pub reason: String,
}

pub(super) fn decode_jukebox_song_with_access(
    value: &NbtSummaryValue,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<JukeboxSongPresentation, DecodeFailure> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(failure("<root>", "expected compound"));
    };
    let sound_event =
        decode_persistent_sound_event(required_field(entries, "sound_event")?, registry_access)
            .map(JukeboxSoundEvent::from)
            .map_err(|error| failure(sound_event_field(error.field), error.reason))?;
    let description_nbt = required_field(entries, "description")?.clone();
    validate_component_nbt(&description_nbt, "jukebox song description")
        .map_err(|_| failure("description", "invalid Component"))?;
    let description_styled = styled_component_runs_from_nbt_value(&description_nbt);
    let description = styled_runs_summary_text(&description_styled);
    let length_in_seconds = required_float(entries, "length_in_seconds")?;
    if !(length_in_seconds > 0.0 && length_in_seconds <= f32::MAX) {
        return Err(failure(
            "length_in_seconds",
            "float outside (0, Float.MAX_VALUE]",
        ));
    }
    let comparator_output = required_int(entries, "comparator_output")?;
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

fn sound_event_field(field: &'static str) -> &'static str {
    match field {
        "holder" => "sound_event",
        "sound_id" => "sound_event.sound_id",
        "range" => "sound_event.range",
        _ => "sound_event",
    }
}

impl From<PersistentSoundEvent> for JukeboxSoundEvent {
    fn from(value: PersistentSoundEvent) -> Self {
        match value {
            PersistentSoundEvent::Named { resource_id, .. } => Self::Reference { resource_id },
            PersistentSoundEvent::Direct {
                resource_id,
                fixed_range,
            } => Self::Direct {
                resource_id,
                fixed_range,
            },
        }
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
    numeric_f32(required_field(entries, name)?).ok_or_else(|| failure(name, "expected number"))
}

fn required_int(entries: &[NbtSummaryEntry], name: &'static str) -> Result<i32, DecodeFailure> {
    numeric_i32(required_field(entries, name)?).ok_or_else(|| failure(name, "expected integer"))
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn failure(field: &'static str, reason: impl Into<String>) -> DecodeFailure {
    DecodeFailure {
        field,
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_named_and_direct_sound_event_codec_forms() {
        let named = decode_jukebox_song_nbt(&song_nbt(
            SoundFixture::Reference("minecraft:music_disc.cat"),
            "Custom song",
            12.5,
            7,
        ))
        .unwrap();
        assert_eq!(
            named.sound_event,
            JukeboxSoundEvent::Reference {
                resource_id: "minecraft:music_disc.cat".to_string(),
            }
        );
        assert_eq!(named.description, "Custom song");
        assert_eq!(named.length_in_seconds, 12.5);
        assert_eq!(named.comparator_output, 7);

        let direct = decode_jukebox_song_nbt(&song_nbt(
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
                        SoundFixture::Direct("example:music.custom", None),
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
        let level_event = store
            .jukebox_song_level_event_presentation(0)
            .unwrap()
            .unwrap();
        assert_eq!(
            level_event.sound_event.resource_id(),
            "example:music.custom"
        );
        assert_eq!(
            level_event.description.component(),
            &NbtSummaryValue::String("Custom".to_string())
        );
    }

    #[test]
    fn level_event_projection_uses_generated_fallback_only_without_server_contents() {
        let mut store = WorldStore::new();
        let fallback = store
            .jukebox_song_level_event_presentation(1)
            .unwrap()
            .unwrap();
        assert_eq!(
            fallback.sound_event.resource_id(),
            "minecraft:music_disc.cat"
        );
        assert_eq!(
            fallback.description.component(),
            &translatable_component("jukebox_song.minecraft.cat".to_string())
        );
        assert_eq!(store.jukebox_song_level_event_presentation(21), Ok(None));

        store.record_registry_entries(
            JUKEBOX_SONG_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("minecraft:cat")],
        );
        assert_eq!(store.jukebox_song_level_event_presentation(0), Ok(None));
        assert_eq!(store.jukebox_song_level_event_presentation(1), Ok(None));
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
                song_nbt(
                    SoundFixture::Direct("example:sound", None),
                    "Broken",
                    0.0,
                    16,
                ),
            )],
        );

        assert_eq!(
            store.synchronized_jukebox_song_presentation(0).unwrap_err(),
            JukeboxSongProjectionError {
                resource_id: "example:broken".to_string(),
                field: "length_in_seconds".to_string(),
                reason: "float outside (0, Float.MAX_VALUE]".to_string(),
            }
        );

        let invalid_comparator = song_nbt(
            SoundFixture::Direct("example:sound", None),
            "Broken",
            1.0,
            16,
        );
        assert_eq!(
            decode_jukebox_song_nbt(&invalid_comparator).unwrap_err(),
            failure("comparator_output", "integer outside 0..=15")
        );
    }

    enum SoundFixture<'a> {
        Reference(&'a str),
        Direct(&'a str, Option<f32>),
    }

    fn decode_jukebox_song_nbt(raw_data: &[u8]) -> Result<JukeboxSongPresentation, DecodeFailure> {
        let data = super::super::element_value::decode_network_element(raw_data).unwrap();
        decode_jukebox_song_with_access(&data.value, None)
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
