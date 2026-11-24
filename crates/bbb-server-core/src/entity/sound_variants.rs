//! Published sound holder conversion and entity playback.

use bbb_protocol::packets::{SoundEvent, SoundEventHolder, SoundSource, Vec3d};
use bbb_registry_codec::registry_codec::sound_variants::SoundEventHolder as PublishedSound;

use super::EntityStore;

pub(crate) fn registered_sound(resource_id: &str) -> Option<SoundEventHolder> {
    Some(SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)?,
    })
}

pub(crate) fn registered_sound_with_registry(
    resource_id: &str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<SoundEventHolder> {
    if let Some(holder) = registered_sound(resource_id) {
        return Some(holder);
    }
    if let Some(spec) = content_registry.sound_event_by_resource_id(resource_id) {
        return Some(SoundEventHolder::Reference {
            registry_id: spec.id,
        });
    }
    bbb_protocol::normalize_resource_location(resource_id.to_owned())
        .ok()
        .map(|location| SoundEventHolder::Direct {
            location,
            fixed_range: None,
        })
}

pub(crate) fn sound_holder(sound: &PublishedSound) -> Option<SoundEventHolder> {
    match sound {
        PublishedSound::Named(id) => registered_sound(id.as_str()),
        PublishedSound::Inline(event) => Some(SoundEventHolder::Direct {
            location: event.sound_id.as_str().to_owned(),
            fixed_range: event.range.map(|range| range.to_f32()),
        }),
    }
}

pub(crate) fn play(
    store: &mut EntityStore,
    sound: SoundEventHolder,
    position: Vec3d,
    source: SoundSource,
    volume: f32,
    pitch: f32,
) {
    let seed = store.random().next_long();
    store.emit_entity_sound(SoundEvent {
        sound,
        position,
        source,
        volume,
        pitch,
        seed,
    });
}
