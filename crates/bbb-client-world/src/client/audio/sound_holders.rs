use bbb_registry_codec::registry_codec::sound_variants::SoundEventHolder;

use super::*;

pub(super) fn vanilla_entity_sound_source(entity_type_id: i32) -> SoundSource {
    match vanilla_entity_sound_source_kind(entity_type_id) {
        Some(VanillaEntitySoundSourceKind::Static(source)) => source,
        Some(VanillaEntitySoundSourceKind::Dynamic) | None => SoundSource::Neutral,
    }
}

pub(super) fn direct_sound_holder(sound: &str) -> SoundHolderState {
    SoundHolderState {
        kind: "direct".to_string(),
        registry_id: None,
        location: Some(sound.to_string()),
        fixed_range: None,
    }
}

pub(super) fn sound_event_summary_holder(sound: &SoundEventSummary) -> Option<SoundHolderState> {
    if let Some(registry_id) = sound.registry_id {
        return Some(SoundHolderState {
            kind: "reference".to_string(),
            registry_id: Some(registry_id),
            location: None,
            fixed_range: None,
        });
    }
    Some(SoundHolderState {
        kind: "direct".to_string(),
        registry_id: None,
        location: Some(sound.sound_id.clone()?),
        fixed_range: sound.fixed_range_bits.map(f32::from_bits),
    })
}

pub(super) fn registry_sound_holder(sound: &SoundEventHolder) -> SoundHolderState {
    match sound {
        SoundEventHolder::Named(resource_id) => direct_sound_holder(resource_id.as_str()),
        SoundEventHolder::Inline(event) => SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(event.sound_id.as_str().to_string()),
            fixed_range: event.range.map(|range| range.to_f32()),
        },
    }
}

pub(super) fn sound_holder_state(sound: ProtocolSoundEventHolder) -> SoundHolderState {
    match sound {
        ProtocolSoundEventHolder::Reference { registry_id } => SoundHolderState {
            kind: "reference".to_string(),
            registry_id: Some(registry_id),
            location: None,
            fixed_range: None,
        },
        ProtocolSoundEventHolder::Direct {
            location,
            fixed_range,
        } => SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(location),
            fixed_range,
        },
    }
}
