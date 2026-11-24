use bbb_protocol::nbt_summary::{codec_float_value, compound_field as field};
use bbb_protocol::{NbtSummaryValue, StyledTextRun};

use crate::WorldStore;

use super::{canonical_description, project_validated_sound_event, RegistrySoundEvent};

pub const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";

/// Client-consumed fields from vanilla 26.1 `Instrument.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentPresentation {
    pub sound_event: RegistrySoundEvent,
    pub use_duration: f32,
    pub range: f32,
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    pub description_nbt: NbtSummaryValue,
}

impl WorldStore {
    /// Projects one instrument through the current synchronized holder order.
    ///
    /// This method never interprets a server holder through the vanilla bootstrap
    /// ordinal. Data-less stubs and absent registries remain explicit `None` so the
    /// consumer can apply only its documented source-aware fallback.
    pub fn synchronized_instrument_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&InstrumentPresentation> {
        let entry = self.registry_entry_by_holder_id(INSTRUMENT_REGISTRY, holder_id)?;
        self.registries.typed.instruments.get(&entry.id)
    }
}

pub(super) fn decode_validated_instrument(value: NbtSummaryValue) -> InstrumentPresentation {
    let NbtSummaryValue::Compound(entries) = value else {
        unreachable!("canonical instrument compound");
    };
    let sound_event = project_validated_sound_event(
        field(&entries, "sound_event").expect("canonical instrument sound"),
    );
    let use_duration = field(&entries, "use_duration")
        .and_then(codec_float_value)
        .expect("canonical instrument duration");
    let range = field(&entries, "range")
        .and_then(codec_float_value)
        .expect("canonical instrument range");
    let (description, description_styled, description_nbt) = canonical_description(entries);
    InstrumentPresentation {
        sound_event,
        use_duration,
        range,
        description,
        description_styled,
        description_nbt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn projects_network_and_known_pack_instruments_in_session_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            INSTRUMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:network_horn",
                    instrument_nbt("example:network", 7.0, 128.0, "Network horn"),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:known_horn",
                    br#"{
                        "sound_event":{"sound_id":"example:known","range":24.0},
                        "use_duration":3.5,
                        "range":64.0,
                        "description":{"text":"Known horn","italic":true}
                    }"#
                    .to_vec(),
                ),
            ],
        );

        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        let network = store.synchronized_instrument_presentation(0).unwrap();
        assert_eq!(network.sound_event.resource_id(), "example:network");
        assert_eq!(network.use_duration, 7.0);
        assert_eq!(network.range, 128.0);
        assert_eq!(network.description, "Network horn");

        let known = store.synchronized_instrument_presentation(1).unwrap();
        assert_eq!(known.sound_event.resource_id(), "example:known");
        assert_eq!(known.sound_event.fixed_range(), Some(24.0));
        assert_eq!(known.description_styled[0].style.italic, Some(true));
    }

    #[test]
    fn rejects_invalid_instrument_fields_without_bootstrap_fallback() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            INSTRUMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:broken",
                    instrument_nbt("example:sound", 0.0, 16.0, "Broken"),
                ),
                RegistryPacketEntry::stub("minecraft:sing_goat_horn"),
            ],
        );

        assert_eq!(store.synchronized_instrument_presentation(1), None);
        assert_eq!(store.synchronized_instrument_presentation(2), None);
        store.begin_registry_configuration();
        assert!(store.finish_registry_configuration().is_err());
    }

    fn instrument_nbt(
        sound_event: &str,
        use_duration: f32,
        range: f32,
        description: &str,
    ) -> Vec<u8> {
        let mut out = vec![10, 10];
        write_nbt_string(&mut out, "sound_event");
        out.push(8);
        write_nbt_string(&mut out, "sound_id");
        write_nbt_string(&mut out, sound_event);
        out.push(0);
        out.push(5);
        write_nbt_string(&mut out, "use_duration");
        out.extend_from_slice(&use_duration.to_bits().to_be_bytes());
        out.push(5);
        write_nbt_string(&mut out, "range");
        out.extend_from_slice(&range.to_bits().to_be_bytes());
        out.push(8);
        write_nbt_string(&mut out, "description");
        write_nbt_string(&mut out, description);
        out.push(0);
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
