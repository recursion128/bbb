//! Source profiles use the persistent profile reader and writer.

use crate::registry_codec::{
    native_outcome::validate_raw_input,
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};

pub(crate) fn decode_resolvable_profile_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    validate_raw_input(input, field, context)?;
    crate::persistent_item_stack_projection::canonical_profile_source_nbt(input)
        .map(CanonicalRegistryCodecValue::new)
        .map_err(|error| context.codec_error(field, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_client_known_pack_registry_json, decode_direct_registry_json,
        recursive_values::RegistryCodecDecodeLimits, RegistryElementSource,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    const OWNER: &str = "minecraft:dialog";
    const CODEC: &str = "ResolvableProfile.CODEC";

    fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(OWNER, CODEC, source)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(value.to_string().into())
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    #[test]
    fn three_sources_share_full_writer_order_and_identifier_normalization() {
        let expected = compound(vec![
            entry("id", NbtSummaryValue::IntArray(vec![0, 0, 0, 1])),
            entry("name", string("Alex")),
            entry("texture", string("minecraft:skin")),
            entry("cape", string("example:cape")),
            entry("elytra", string("minecraft:wings")),
            entry("model", string("wide")),
        ]);
        let json = br#"{"id":[0,0,0,1],"name":"Alex","texture":"skin","cape":"example:cape","elytra":"wings","model":"wide"}"#;
        let server = decode_direct_registry_json(json).unwrap();
        let client = decode_client_known_pack_registry_json(json).unwrap();
        let nbt = compound(vec![
            entry("id", NbtSummaryValue::IntArray(vec![0, 0, 0, 1])),
            entry("name", string("Alex")),
            entry("texture", string("skin")),
            entry("cape", string("example:cape")),
            entry("elytra", string("wings")),
            entry("model", string("wide")),
        ]);
        for (input, source) in [
            (server.root(), server.source),
            (client.root(), client.source),
            (
                RegistryElementValueRef::Nbt(&nbt),
                RegistryElementSource::NetworkNbt,
            ),
        ] {
            let decoded =
                decode_resolvable_profile_source(input, "$", &mut context(source)).unwrap();
            assert_eq!(decoded.network_nbt_unchecked(), &expected);
            assert_eq!(decoded.references().count(), 0);
            let mut limited = RegistryCodecDecodeContext::with_limits(
                OWNER,
                CODEC,
                source,
                RegistryCodecDecodeLimits {
                    max_nodes: 1,
                    ..RegistryCodecDecodeLimits::DEFAULT
                },
            )
            .unwrap();
            assert!(decode_resolvable_profile_source(input, "$", &mut limited).is_err());
        }
    }
}
