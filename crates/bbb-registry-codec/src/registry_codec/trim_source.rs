//! Exact source-aware synchronized owners for armor-trim definitions.
//!
//! The persistent component dispatcher already owns both direct record
//! schemas. Registry publication enters those direct codecs without the
//! surrounding `RegistryFileCodec` holder and retains their recursive
//! Component references for the frozen-generation check

use crate::{
    persistent_component_nbt::source_codec::{
        decode_trim_material_direct_source, decode_trim_pattern_direct_source,
    },
    registry_codec::recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
};

use super::{synchronized_registry_element_codec, RegistryCodecError, RegistryElementData};

const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";

pub(crate) fn decode_trim_material_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(TRIM_MATERIAL_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(TRIM_MATERIAL_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_trim_material_direct_source(data.root(), "<root>", &mut context)
}

pub(crate) fn decode_trim_pattern_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(TRIM_PATTERN_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(TRIM_PATTERN_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_trim_pattern_direct_source(data.root(), "<root>", &mut context)
}

#[cfg(test)]
pub(crate) fn decode_trim_material_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_trim_material_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[cfg(test)]
pub(crate) fn decode_trim_pattern_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_trim_pattern_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        decode_client_known_pack_registry_json, decode_direct_registry_json,
        decode_network_registry_element,
    };
    use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

    use super::*;

    #[test]
    fn direct_records_reuse_component_owner_and_write_canonical_field_order() {
        let material = decode_direct_registry_json(
            br#"{
                "asset_name":"quartz",
                "override_armor_assets":{},
                "description":{"text":"Quartz","click_event":{"action":"show_dialog","dialog":"example:next"}},
                "ignored":true
            }"#,
        )
        .unwrap();
        let material = decode_trim_material_element(&material).unwrap();
        assert_eq!(
            material.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![
                entry(
                    "asset_name",
                    NbtSummaryValue::String("quartz".to_string().into())
                ),
                entry(
                    "description",
                    NbtSummaryValue::Compound(vec![
                        entry("text", NbtSummaryValue::String("Quartz".to_string().into())),
                        entry(
                            "click_event",
                            NbtSummaryValue::Compound(vec![
                                entry(
                                    "action",
                                    NbtSummaryValue::String("show_dialog".to_string().into()),
                                ),
                                entry(
                                    "dialog",
                                    NbtSummaryValue::String("example:next".to_string().into()),
                                ),
                            ]),
                        ),
                    ]),
                ),
            ])
        );
        let references = material.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].target_registry, "minecraft:dialog");
        assert_eq!(references[0].resource_id.as_str(), "example:next");

        let pattern = decode_client_known_pack_registry_json(
            br#"{"asset_id":"spire","description":"Spire","decal":"bad","ignored":null}"#,
        )
        .unwrap();
        assert_eq!(
            decode_trim_pattern_element(&pattern)
                .unwrap()
                .network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![
                entry(
                    "asset_id",
                    NbtSummaryValue::String("minecraft:spire".to_string().into()),
                ),
                entry(
                    "description",
                    NbtSummaryValue::String("Spire".to_string().into()),
                ),
                entry("decal", NbtSummaryValue::Byte(0)),
            ])
        );
    }

    #[test]
    fn canonical_network_values_round_trip_through_the_same_direct_owner() {
        for (decode, registry) in [
            (
                decode_trim_material_element as fn(&RegistryElementData) -> _,
                TRIM_MATERIAL_REGISTRY,
            ),
            (decode_trim_pattern_element, TRIM_PATTERN_REGISTRY),
        ] {
            let json = if registry == TRIM_MATERIAL_REGISTRY {
                br#"{"asset_name":"gold","description":"Gold"}"#.as_slice()
            } else {
                br#"{"asset_id":"coast","description":"Coast"}"#.as_slice()
            };
            let direct = decode_direct_registry_json(json).unwrap();
            let canonical = decode(&direct).unwrap().network_nbt_unchecked().clone();
            let encoded = encode_nbt_summary(&canonical, 8192).unwrap();
            let network = decode_network_registry_element(&encoded).unwrap();
            assert_eq!(
                decode(&network).unwrap().network_nbt_unchecked(),
                &canonical
            );
        }
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }
}
