use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    sound_event_types::vanilla_sound_event_registry_id, NbtSummaryEntry, NbtSummaryValue,
};

use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_named_instrument_holder_from_session_registry() {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(INSTRUMENT_REGISTRY);
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:ponder_goat_horn"),
    ];
    registries
        .contents
        .insert(INSTRUMENT_REGISTRY.to_string(), contents);

    assert_eq!(
        decode(&string("ponder_goat_horn"), Some(&registries)).unwrap(),
        Some(DialogDataComponentValueState::Instrument {
            instrument: DialogInstrumentHolderState::Named {
                registry_id: 1,
                resource_id: "minecraft:ponder_goat_horn".to_string(),
            },
        })
    );
}

#[test]
fn decodes_direct_instrument_with_named_sound_and_component() {
    let description = compound([
        ("text", string("Ponder")),
        ("italic", NbtSummaryValue::Byte(1)),
    ]);
    let value = compound([
        ("sound_event", string("block.note_block.harp")),
        ("use_duration", NbtSummaryValue::Double(7.0f64.to_bits())),
        ("range", NbtSummaryValue::Int(256)),
        ("description", description.clone()),
        ("ignored", NbtSummaryValue::Int(1)),
    ]);
    let decoded = decode(&value, None).unwrap();
    let patch =
        super::super::persistent_patch(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID, &value, None);
    assert_eq!(patch.instrument.registry_id, None);
    assert_eq!(
        patch.instrument.direct.as_deref().unwrap().description,
        "Ponder"
    );
    assert_eq!(
        patch
            .instrument
            .direct
            .as_deref()
            .unwrap()
            .description_styled
            .as_ref()
            .unwrap()[0]
            .style
            .italic,
        Some(true)
    );
    assert_eq!(
        patch
            .instrument
            .direct
            .as_deref()
            .and_then(|instrument| instrument.description_nbt.as_deref()),
        Some(&description)
    );
    let Some(DialogDataComponentValueState::Instrument {
        instrument: DialogInstrumentHolderState::Direct { instrument },
    }) = decoded
    else {
        panic!("expected direct instrument")
    };
    assert_eq!(
        instrument.sound_event,
        DialogSoundEventHolderState::Named {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.note_block.harp")
                .unwrap(),
            resource_id: "minecraft:block.note_block.harp".to_string(),
        }
    );
    assert_eq!(instrument.use_duration_bits, 7.0f32.to_bits());
    assert_eq!(instrument.range_bits, 256.0f32.to_bits());
    assert_eq!(instrument.description.text, "Ponder");
    assert_eq!(instrument.description.component, description);
    assert_eq!(instrument.description.runs[0].style.italic, Some(true));
}

#[test]
fn decodes_direct_instrument_with_direct_lenient_sound() {
    let decoded = decode(
        &compound([
            (
                "sound_event",
                compound([
                    ("sound_id", string("example:horn")),
                    ("range", string("leniently ignored")),
                ]),
            ),
            (
                "use_duration",
                NbtSummaryValue::Float(f32::from_bits(1).to_bits()),
            ),
            ("range", NbtSummaryValue::Byte(1)),
            ("description", string("Horn")),
        ]),
        Some(&RegistrySet::default()),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Instrument {
        instrument: DialogInstrumentHolderState::Direct { instrument },
    }) = decoded
    else {
        panic!("expected direct instrument")
    };
    assert_eq!(
        instrument.sound_event,
        DialogSoundEventHolderState::Direct {
            sound_id: "example:horn".to_string(),
            fixed_range_bits: None,
        }
    );
    assert_eq!(instrument.use_duration_bits, f32::from_bits(1).to_bits());
    assert_eq!(instrument.range_bits, 1.0f32.to_bits());
    assert_eq!(instrument.description.text, "Horn");
}

#[test]
fn rejects_malformed_direct_instrument_records_and_positive_floats() {
    let valid = || {
        vec![
            ("sound_event", string("block.note_block.harp")),
            ("use_duration", NbtSummaryValue::Int(1)),
            ("range", NbtSummaryValue::Int(1)),
            ("description", string("Horn")),
        ]
    };
    for field in ["sound_event", "use_duration", "range", "description"] {
        assert!(decode(
            &NbtSummaryValue::Compound(
                valid()
                    .into_iter()
                    .filter(|(name, _)| *name != field)
                    .map(|(name, value)| entry(name, value))
                    .collect(),
            ),
            None,
        )
        .is_err());
    }

    for invalid in [
        0.0f32.to_bits(),
        (-0.0f32).to_bits(),
        (-1.0f32).to_bits(),
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
    ] {
        for field in ["use_duration", "range"] {
            let value = NbtSummaryValue::Compound(
                valid()
                    .into_iter()
                    .map(|(name, value)| {
                        entry(
                            name,
                            if name == field {
                                NbtSummaryValue::Float(invalid)
                            } else {
                                value
                            },
                        )
                    })
                    .collect(),
            );
            assert!(decode(&value, None).is_err());
        }
    }

    for value in [
        NbtSummaryValue::List(Vec::new()),
        compound([
            ("sound_event", string("example:missing")),
            ("use_duration", NbtSummaryValue::Int(1)),
            ("range", NbtSummaryValue::Int(1)),
            ("description", string("Horn")),
        ]),
        compound([
            ("sound_event", string("block.note_block.harp")),
            ("use_duration", NbtSummaryValue::Int(1)),
            ("range", NbtSummaryValue::Int(1)),
            ("description", NbtSummaryValue::Int(1)),
        ]),
    ] {
        assert!(decode(&value, None).is_err());
    }
}

fn decode(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        "minecraft:instrument",
        value,
        registries,
    )
}

fn compound<const N: usize>(entries: [(&str, NbtSummaryValue); N]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| entry(name, value))
            .collect(),
    )
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}
