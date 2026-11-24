use bbb_protocol::nbt_summary::compound_field as field;
use bbb_protocol::{
    jukebox_song_types::{
        vanilla_26_1_jukebox_song_by_holder_id, vanilla_26_1_jukebox_song_by_song_id,
        VanillaJukeboxSong,
    },
    NbtSummaryEntry, NbtSummaryValue, RichTextComponent, StyledTextRun,
};
use bbb_registry_codec::registry_codec::jukebox_song_registry_value;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

use super::{
    canonical_description, project_validated_sound_event, RegistryProjectionError,
    RegistrySoundEvent,
};

pub const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";

/// Client-consumed fields from vanilla 26.1 `JukeboxSong.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JukeboxSongPresentation {
    pub sound_event: RegistrySoundEvent,
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    pub description_nbt: NbtSummaryValue,
    pub length_in_seconds: f32,
    pub comparator_output: i32,
}

/// The exact subset consumed by Vanilla `LevelEventHandler.playJukeboxSong`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JukeboxSongLevelEventPresentation {
    pub sound_event: RegistrySoundEvent,
    pub description: RichTextComponent,
}

impl WorldStore {
    /// Resolves level-event holder data without allowing a synchronized
    /// session registry to fall back to the fixed Vanilla bootstrap ordinal.
    pub(crate) fn jukebox_song_level_event_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<JukeboxSongLevelEventPresentation>, RegistryProjectionError> {
        if self
            .frozen_registry_access()
            .has_server_contents(JUKEBOX_SONG_REGISTRY)
        {
            if let Some(presentation) = self.synchronized_jukebox_song_presentation(holder_id) {
                let description =
                    RichTextComponent::try_from_component_nbt(presentation.description_nbt.clone())
                        .map_err(|reason| RegistryProjectionError {
                            resource_id: self
                                .registry_resource_id_by_holder_id(JUKEBOX_SONG_REGISTRY, holder_id)
                                .unwrap_or("<unknown>")
                                .to_string(),
                            field: "description".to_string(),
                            reason,
                        })?;
                return Ok(Some(JukeboxSongLevelEventPresentation {
                    sound_event: presentation.sound_event.clone(),
                    description,
                }));
            }
            let Some(entry) = self.registry_entry_by_holder_id(JUKEBOX_SONG_REGISTRY, holder_id)
            else {
                return Ok(None);
            };
            if entry.has_data {
                return Ok(None);
            }
            return Ok(vanilla_26_1_jukebox_song_by_song_id(&entry.id)
                .map(vanilla_level_event_presentation));
        }

        let Some(song) = vanilla_26_1_jukebox_song_by_holder_id(holder_id) else {
            return Ok(None);
        };
        Ok(Some(vanilla_level_event_presentation(song)))
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
    ) -> Option<&JukeboxSongPresentation> {
        let entry = self.registry_entry_by_holder_id(JUKEBOX_SONG_REGISTRY, holder_id)?;
        self.registries.typed.jukebox_songs.get(&entry.id)
    }
}

fn vanilla_level_event_presentation(
    song: &VanillaJukeboxSong,
) -> JukeboxSongLevelEventPresentation {
    let description = RichTextComponent::try_from_component_nbt(translatable_component(
        song.description_translation_key(),
    ))
    .expect("generated jukebox description translation is a valid Component");
    JukeboxSongLevelEventPresentation {
        sound_event: RegistrySoundEvent::Reference {
            resource_id: song.sound_event_id.to_string(),
        },
        description,
    }
}

fn translatable_component(translation_key: String) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "translate".into(),
        value: NbtSummaryValue::string(translation_key),
    }])
}

pub(super) fn decode_validated_jukebox_song(value: NbtSummaryValue) -> JukeboxSongPresentation {
    let song = jukebox_song_registry_value(&value).expect("canonical jukebox fields");
    let NbtSummaryValue::Compound(entries) = value else {
        unreachable!("canonical jukebox compound");
    };
    let sound_event = project_validated_sound_event(
        field(&entries, "sound_event").expect("canonical jukebox sound"),
    );
    let (description, description_styled, description_nbt) = canonical_description(entries);
    JukeboxSongPresentation {
        sound_event,
        description,
        description_styled,
        description_nbt,
        length_in_seconds: f32::from_bits(song.length_in_seconds_bits),
        comparator_output: song.comparator_output,
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
            RegistrySoundEvent::Reference {
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
            RegistrySoundEvent::Direct {
                resource_id: "example:records/direct".to_string(),
                fixed_range: Some(24.0),
            }
        );
    }

    #[test]
    fn synchronized_projection_follows_custom_server_holder_order() {
        let mut store = WorldStore::new();
        store.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(
            &mut store,
            Some(JUKEBOX_SONG_REGISTRY),
        );
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
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store
                .synchronized_jukebox_song_presentation(0)
                .unwrap()
                .sound_event
                .resource_id(),
            "example:music.custom"
        );
        assert_eq!(
            store
                .synchronized_jukebox_song_presentation(1)
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
            &NbtSummaryValue::string("Custom")
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
        let stub = store
            .jukebox_song_level_event_presentation(0)
            .unwrap()
            .unwrap();
        assert_eq!(stub.sound_event.resource_id(), "minecraft:music_disc.cat");
        assert_eq!(store.jukebox_song_level_event_presentation(1), Ok(None));
    }

    #[test]
    fn stub_unknown_and_absent_registry_do_not_substitute_vanilla_ordinals() {
        let mut store = WorldStore::new();
        assert_eq!(store.synchronized_jukebox_song_presentation(1), None);
        store.record_registry_entries(
            JUKEBOX_SONG_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("minecraft:cat")],
        );
        assert_eq!(store.synchronized_jukebox_song_presentation(0), None);
        assert_eq!(store.synchronized_jukebox_song_presentation(1), None);
    }

    #[test]
    fn malformed_entries_are_rejected_before_publication() {
        let mut store = WorldStore::new();
        store.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(
            &mut store,
            Some(JUKEBOX_SONG_REGISTRY),
        );
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

        assert!(store.finish_registry_configuration().is_err());

        let invalid_comparator = song_nbt(
            SoundFixture::Direct("example:sound", None),
            "Broken",
            1.0,
            16,
        );
        assert!(decode_jukebox_song_nbt(&invalid_comparator).is_err());
    }

    enum SoundFixture<'a> {
        Reference(&'a str),
        Direct(&'a str, Option<f32>),
    }

    fn decode_jukebox_song_nbt(raw_data: &[u8]) -> Result<JukeboxSongPresentation, String> {
        let data = super::super::element_value::decode_network_element(raw_data).unwrap();
        let registries = crate::RegistrySet::vanilla_26_1();
        let value = super::super::revision::exact_network_value(
            JUKEBOX_SONG_REGISTRY,
            &data,
            registries.frozen_access(),
        )?;
        Ok(decode_validated_jukebox_song(value))
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
