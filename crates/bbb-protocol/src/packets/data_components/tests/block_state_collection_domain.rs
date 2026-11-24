use super::*;
use crate::packets::data_components::catalog::vanilla_data_component_descriptor;
use crate::{data_component_types::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, JavaString};

fn patch(entries: impl IntoIterator<Item = (JavaString, JavaString)>) -> DataComponentPatchSummary {
    DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID],
        block_state_properties: BlockStateProperties::from_entries(entries),
        ..DataComponentPatchSummary::default()
    }
}

fn roundtrip(expected: &DataComponentPatchSummary) -> DataComponentPatchSummary {
    validate_data_component_patch_summary(expected).unwrap();
    let mut out = Encoder::new();
    encode_data_component_patch_summary(&mut out, expected).unwrap();
    let wire = out.into_inner();
    let mut decoder = Decoder::new(&wire);
    let decoded = decode_data_component_patch_summary(&mut decoder).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(&decoded, expected);
    validate_data_component_patch_summary(&decoded).unwrap();
    decoded
}

#[test]
fn block_state_collection_domain_257_and_4096_properties_survive_complete_patch_wire() {
    for count in [257, 4096] {
        let expected = patch((0..count).map(|index| {
            (
                format!("property_{index}").into(),
                format!("value_{index}").into(),
            )
        }));
        let decoded = roundtrip(&expected);
        assert_eq!(decoded.block_state_properties.len(), count);
        for index in 0..count {
            assert_eq!(
                decoded
                    .block_state_properties
                    .get_str(&format!("property_{index}")),
                Some(&JavaString::from(format!("value_{index}")))
            );
        }
        assert_eq!(roundtrip(&decoded), expected);
    }
}

#[test]
fn block_state_collection_domain_oversized_header_is_rejected_before_property_payload() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID).unwrap();
    let mut out = Encoder::new();
    out.write_var_i32(4097);
    let header_len = out.into_inner().len();
    let mut out = Encoder::new();
    out.write_var_i32(4097);
    out.write_bytes(&[0xff, 0xfe, 0xfd]);
    let wire = out.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut summary = DataComponentPatchSummary::default();
    let error = descriptor.decode(&mut decoder, &mut summary).unwrap_err();
    assert!(
        matches!(error, ProtocolError::PacketTooLarge(4097, 4096)),
        "{error}"
    );
    assert_eq!(decoder.position(), header_len);
    assert_eq!(decoder.remaining(), &[0xff, 0xfe, 0xfd]);
    assert!(summary.block_state_properties.is_empty());

    let expected =
        patch((0..4097).map(|index| (format!("property_{index}").into(), "value".into())));
    assert!(matches!(
        validate_data_component_patch_summary(&expected),
        Err(ProtocolError::PacketTooLarge(4097, 4096))
    ));
    let mut out = Encoder::new();
    let error = descriptor.encode(&expected, &mut out).unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(4097, 4096)));
    assert!(encode_data_component_patch_summary(&mut Encoder::new(), &expected).is_err());
}

#[test]
fn block_state_collection_domain_32767_utf16_key_and_value_remain_wire_legal() {
    let text = format!("{}a", "😀".repeat(16383));
    let java = JavaString::from(text);
    assert_eq!(java.units().len(), 32767);
    let expected = patch([(java.clone(), java.clone())]);
    let decoded = roundtrip(&expected);
    assert_eq!(decoded.block_state_properties.get(&java), Some(&java));
}

#[test]
fn block_state_collection_domain_32768_utf16_key_and_value_fail_both_wire_directions() {
    let oversized = JavaString::from("😀".repeat(16384));
    assert_eq!(oversized.units().len(), 32768);
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID).unwrap();
    for (key, value) in [
        (oversized.clone(), JavaString::from("value")),
        (JavaString::from("property"), oversized.clone()),
    ] {
        let expected = patch([(key.clone(), value.clone())]);
        assert!(matches!(
            validate_data_component_patch_summary(&expected),
            Err(ProtocolError::StringTooLong {
                actual: 32768,
                max: 32767
            })
        ));
        let mut out = Encoder::new();
        let error = descriptor.encode(&expected, &mut out).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual: 32768,
                max: 32767
            }
        ));
        assert!(encode_data_component_patch_summary(&mut Encoder::new(), &expected).is_err());

        let wire =
            single_data_component_payload(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, |out| {
                out.write_var_i32(1);
                out.write_utf16_string(key.units());
                out.write_utf16_string(value.units());
            });
        let error = decode_data_component_patch_summary(&mut Decoder::new(&wire)).unwrap_err();
        assert!(
            matches!(
                error,
                ProtocolError::StringTooLong {
                    actual: 32768,
                    max: 32767
                }
            ),
            "{error}"
        );
    }
}
