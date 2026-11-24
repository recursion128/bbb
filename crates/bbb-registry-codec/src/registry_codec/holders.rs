//! Crate-private registry holder codec primitives.
//!
//! Vanilla creates a registration lookup for every registry before it decodes
//! any element. A named holder therefore records a placeholder dependency; it
//! does not recursively decode or project the target. Final generation freeze
//! validates that every named element was bound

use bbb_protocol::NbtSummaryValue;

use super::{
    RegistryCodecError, RegistryCodecReference, RegistryElementValueRef, RegistryReferenceKind,
    RegistryResourceIdentifier,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegistryHolder<T> {
    Named(RegistryResourceIdentifier),
    Inline(T),
}

impl<T> RegistryHolder<T> {
    pub(crate) fn named_reference(
        &self,
        target_registry: &'static str,
    ) -> Option<RegistryCodecReference<'_>> {
        match self {
            Self::Named(resource_id) => Some(RegistryCodecReference {
                target_registry,
                kind: RegistryReferenceKind::Element,
                resource_id,
            }),
            Self::Inline(_) => None,
        }
    }
}

/// Decodes `RegistryFixedCodec`: only a named element is legal. Membership is
/// deliberately deferred until the generation-final validation pass.
pub(crate) fn decode_fixed_holder(
    owner_registry: &str,
    codec: &str,
    field: &str,
    value: RegistryElementValueRef<'_>,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    let raw = value.java_string_ref().ok_or_else(|| {
        RegistryCodecError::native_semantic(
            owner_registry,
            codec,
            field,
            "expected a named registry holder",
        )
    })?;
    RegistryResourceIdentifier::parse_java_string(raw).ok_or_else(|| {
        RegistryCodecError::native_semantic(
            owner_registry,
            codec,
            field,
            format!("invalid resource identifier `{raw}`"),
        )
    })
}

pub(crate) fn encode_fixed_holder(value: &RegistryResourceIdentifier) -> NbtSummaryValue {
    NbtSummaryValue::String(value.as_str().into())
}

/// Decodes `RegistryFileCodec`: a valid identifier is a named placeholder;
/// every other input is offered to the inline element codec.
pub(crate) fn decode_file_holder<T>(
    field: &str,
    value: RegistryElementValueRef<'_>,
    decode_inline: impl FnOnce(RegistryElementValueRef<'_>, &str) -> Result<T, RegistryCodecError>,
) -> Result<RegistryHolder<T>, RegistryCodecError> {
    if let Some(identifier) = value
        .java_string_ref()
        .and_then(RegistryResourceIdentifier::parse_java_string)
    {
        return Ok(RegistryHolder::Named(identifier));
    }
    decode_inline(value, field).map(RegistryHolder::Inline)
}

pub(crate) fn encode_file_holder<T>(
    value: &RegistryHolder<T>,
    encode_inline: impl FnOnce(&T) -> Result<NbtSummaryValue, RegistryCodecError>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    match value {
        RegistryHolder::Named(identifier) => Ok(encode_fixed_holder(identifier)),
        RegistryHolder::Inline(value) => encode_inline(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{decode_direct_registry_json, decode_network_registry_element};

    const OWNER: &str = "minecraft:test_environment";
    const CODEC: &str = "TestEnvironmentDefinition.DIRECT_CODEC";

    fn inline_string(
        value: RegistryElementValueRef<'_>,
        path: &str,
    ) -> Result<String, RegistryCodecError> {
        value.utf8_string().ok_or_else(|| {
            RegistryCodecError::native_semantic(OWNER, CODEC, path, "expected inline string")
        })
    }

    #[test]
    fn fixed_holders_retain_placeholders_without_target_lookup() {
        let value = decode_direct_registry_json(br#""example:self""#).unwrap();
        let holder = decode_fixed_holder(OWNER, CODEC, "clock", value.root()).unwrap();
        assert_eq!(holder.as_str(), "example:self");
        assert_eq!(
            encode_fixed_holder(&holder),
            NbtSummaryValue::String("example:self".to_string().into())
        );
    }

    #[test]
    fn file_holders_choose_named_or_inline_without_dereferencing() {
        let named = decode_direct_registry_json(br#""example:mutual""#).unwrap();
        assert!(matches!(
            decode_file_holder("value", named.root(), inline_string).unwrap(),
            RegistryHolder::Named(id) if id.as_str() == "example:mutual"
        ));

        let inline = decode_direct_registry_json(br#"{"value":"inline"}"#).unwrap();
        let decoded = decode_file_holder("value", inline.root(), |value, _| {
            value
                .field("value")
                .and_then(RegistryElementValueRef::utf8_string)
                .ok_or_else(|| {
                    RegistryCodecError::native_semantic(
                        OWNER,
                        CODEC,
                        "value.value",
                        "missing string",
                    )
                })
        })
        .unwrap();
        assert_eq!(decoded, RegistryHolder::Inline("inline".to_string()));
    }

    #[test]
    fn nbt_primitive_arrays_are_visible_to_list_codecs() {
        for (raw, expected) in [
            (vec![7, 0, 0, 0, 2, 1, -1_i8 as u8], vec![1, -1]),
            (
                vec![11, 0, 0, 0, 2, 0, 0, 0, 1, 255, 255, 255, 255],
                vec![1, -1],
            ),
            (
                vec![
                    12, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 255, 255, 255, 255, 255, 255, 255, 255,
                ],
                vec![1, -1],
            ),
        ] {
            let data = decode_network_registry_element(&raw).unwrap();
            let root = data.root();
            assert_eq!(root.list_len(), Some(2));
            assert_eq!(
                (0..2)
                    .map(|index| root.list_element(index).unwrap().codec_i32().unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}
