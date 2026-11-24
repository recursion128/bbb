use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryCodecError, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

use super::*;

const OWNER: &str = "minecraft:advancement";
const CODEC: &str = "EntitySubPredicate.CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode_data(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode(data.root(), "$", &mut context(data.source))
}

#[test]
fn criterion_entity_subpredicate_player_and_lightning_use_real_entity_recursion() {
    let player_json = br#"{
      "input":{"jump":false},
      "looking_at":{"team":"prey"},
      "advancements":{},"recipes":{},"stats":[],
      "gamemode":["survival"],"food":{"level":10},"level":{"min":2},
      "type":"player"
    }"#;
    let player_expected = compound(vec![
        entry(
            "level",
            compound(vec![entry("min", NbtSummaryValue::Int(2))]),
        ),
        entry(
            "food",
            compound(vec![entry("level", NbtSummaryValue::Int(10))]),
        ),
        entry("gamemode", NbtSummaryValue::List(vec![string("survival")])),
        entry("looking_at", compound(vec![entry("team", string("prey"))])),
        entry(
            "input",
            compound(vec![entry("jump", NbtSummaryValue::Byte(0))]),
        ),
        entry("type", string("minecraft:player")),
    ]);

    let lightning_json = br#"{
      "entity_struck":{"team":"hit"},
      "blocks_set_on_fire":{"min":1},
      "type":"lightning"
    }"#;
    let lightning_expected = compound(vec![
        entry(
            "blocks_set_on_fire",
            compound(vec![entry("min", NbtSummaryValue::Int(1))]),
        ),
        entry(
            "entity_struck",
            compound(vec![entry("team", string("hit"))]),
        ),
        entry("type", string("minecraft:lightning")),
    ]);

    for (json, expected) in [
        (player_json.as_slice(), player_expected),
        (lightning_json.as_slice(), lightning_expected),
    ] {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
            network(&expected),
        ] {
            let decoded = decode_data(&data).unwrap();
            assert_eq!(decoded.network_nbt_unchecked(), &expected);
        }
    }
}
