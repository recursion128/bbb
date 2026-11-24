use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    loot_context_types::vanilla_loot_entity_target,
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry};

use super::*;

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode(data: &RegistryElementData) -> LootContextCodecValue {
    decode_scoreboard_name_provider_source(data.root(), "$", &mut context(data.source)).unwrap()
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

#[test]
fn fixed_and_context_families_are_exact_in_all_three_sources() {
    let fixed_input = compound(vec![
        entry("type", string("fixed")),
        entry("name", string("Player One")),
    ]);
    let fixed_expected = compound(vec![
        entry("name", string("Player One")),
        entry("type", string("minecraft:fixed")),
    ]);
    for data in [
        decode_direct_registry_json(br#"{"type":"fixed","name":"Player One"}"#).unwrap(),
        decode_client_known_pack_registry_json(br#"{"type":"fixed","name":"Player One"}"#).unwrap(),
        network(&fixed_input),
    ] {
        let decoded = decode(&data);
        assert_eq!(decoded.canonical.network_nbt_unchecked(), &fixed_expected);
        assert!(decoded.referenced_context_params.is_empty());
    }

    let context_input = compound(vec![
        entry("type", string("context")),
        entry("target", string("direct_attacker")),
    ]);
    for data in [
        decode_direct_registry_json(br#"{"type":"context","target":"direct_attacker"}"#).unwrap(),
        decode_client_known_pack_registry_json(br#"{"type":"context","target":"direct_attacker"}"#)
            .unwrap(),
        network(&context_input),
    ] {
        let decoded = decode(&data);
        assert_eq!(
            decoded.canonical.network_nbt_unchecked(),
            &string("direct_attacker")
        );
        assert_eq!(
            decoded.referenced_context_params,
            vec![
                vanilla_loot_entity_target("direct_attacker")
                    .unwrap()
                    .context_param
            ]
        );
    }
}

#[test]
fn every_inline_target_writes_the_target_and_owns_its_context_fact() {
    for target in [
        "this",
        "attacker",
        "direct_attacker",
        "attacking_player",
        "target_entity",
        "interacting_entity",
    ] {
        let json = format!("\"{target}\"");
        for data in [
            decode_direct_registry_json(json.as_bytes()).unwrap(),
            decode_client_known_pack_registry_json(json.as_bytes()).unwrap(),
            network(&string(target)),
        ] {
            let decoded = decode(&data);
            assert_eq!(decoded.canonical.network_nbt_unchecked(), &string(target));
            assert_eq!(
                decoded.referenced_context_params,
                vec![vanilla_loot_entity_target(target).unwrap().context_param]
            );
        }
    }
}
