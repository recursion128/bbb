use super::*;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use serde_json::{json, Value};

fn state(block: &str, property: &str, value: &str) -> &'static VanillaBlockStateIdentity {
    let block = vanilla_block_type_for_resource_id(block).unwrap();
    let default = vanilla_block_default_state_id(block.id).unwrap();
    let id = crate::fluid::with_property(default, property, value).unwrap();
    vanilla_block_state_identity(id).unwrap()
}

fn matches(predicate: Value, state: &VanillaBlockStateIdentity) -> bool {
    let predicate =
        bbb_server_datapack::loot::parse_state_properties_predicate(&predicate).unwrap();
    matches_state_properties(&predicate, state).unwrap()
}

#[test]
fn state_properties_ranges_use_typed_comparison_and_invalid_bounds_never_match() {
    let wheat = state("minecraft:wheat", "age", "3");
    for range in [
        json!({}),
        json!({"min":null, "max":null}),
        json!({"min":"+2", "max":"03"}),
        json!({"max":"7"}),
    ] {
        assert!(matches(json!({"age":range}), wheat));
    }
    for range in [
        json!({"min":"4"}),
        json!({"max":"2"}),
        json!({"min":"4", "max":"2"}),
        json!({"max":"8"}),
        json!({"min":"-1"}),
    ] {
        assert!(!matches(json!({"age":range}), wheat));
    }
    let lit = state("minecraft:redstone_lamp", "lit", "true");
    let dark = state("minecraft:redstone_lamp", "lit", "false");
    assert!(matches(json!({"lit":{"min":"false"}}), lit));
    assert!(!matches(json!({"lit":{"min":"true"}}), dark));
    assert!(!matches(json!({"lit":{"max":"TRUE"}}), lit));
}

#[test]
fn state_properties_enum_range_uses_java_ordinal_instead_of_property_list_order() {
    let up = state("minecraft:pointed_dripstone", "vertical_direction", "up");
    let down = state("minecraft:pointed_dripstone", "vertical_direction", "down");
    assert!(matches(json!({"vertical_direction":{"min":"down"}}), up));
    assert!(!matches(json!({"vertical_direction":{"min":"up"}}), down));
    assert!(matches(json!({"vertical_direction":{"max":"up"}}), down));
    assert!(!matches(
        json!({"vertical_direction":{"max":"north"}}),
        down
    ));
}

#[test]
fn state_properties_missing_names_fail() {
    let wheat = state("minecraft:wheat", "age", "3");
    assert!(matches(json!({}), wheat));
    assert!(!matches(json!({"missing":{}}), wheat));
}

#[test]
fn state_properties_loot_and_location_consumers_share_the_typed_matcher() {
    use crate::loot_runtime::{
        location_predicate_matches, matches_conditions, param_sets, LootOrigin, LootParams,
        LootTool,
    };
    use crate::random::GameRandom;
    use bbb_protocol::packets::BlockPos;
    use bbb_server_datapack::loot::{parse_loot_condition, LootRegistries};

    let wheat = state("minecraft:wheat", "age", "3");
    let level = |_: BlockPos| Some(wheat.id);
    let tool = LootTool::empty();
    let registries = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        }),
        block_state_id: Some(wheat.id),
        tool: Some(&tool),
        ..LootParams::new(param_sets::BLOCK)
    };
    for (range, expected) in [
        (json!({"min":"+2","max":"3"}), true),
        (json!({"min":"4"}), false),
    ] {
        let properties = json!({"age":range});
        let condition = parse_loot_condition(&json!({"condition":"minecraft:block_state_property", "block":"minecraft:wheat", "properties":properties}), None).unwrap();
        let random = GameRandom::with_seed(24);
        assert_eq!(
            matches_conditions(&registries, "state", &[condition], &params, &random),
            Ok(expected)
        );
        assert_eq!(
            location_predicate_matches(
                &bbb_server_datapack::loot::parse_location_predicate(
                    &json!({"block":{"blocks":"minecraft:wheat", "state":properties}})
                )
                .unwrap(),
                0.0,
                64.0,
                0.0,
                None,
                Some(&level),
                None,
            ),
            Ok(expected)
        );
    }
}
