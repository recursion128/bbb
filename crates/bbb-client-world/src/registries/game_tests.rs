pub const TEST_ENVIRONMENT_REGISTRY: &str = "minecraft:test_environment";
pub const TEST_INSTANCE_REGISTRY: &str = "minecraft:test_instance";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryPacketEntry, WorldStore};
    use bbb_registry_codec::registry_codec::game_tests::{
        TestEnvironmentHolderView, TestInstanceKindView, TestRotation,
    };

    #[test]
    fn projects_known_pack_environment_and_instance_with_session_references() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:weather",
                    br#"{"type":"minecraft:weather","weather":"thunder"}"#.to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:combined",
                    br#"{
                        "type":"minecraft:all_of",
                        "definitions":[
                            "example:weather",
                            {"type":"minecraft:function","setup":"example:setup"}
                        ]
                    }"#
                    .to_vec(),
                ),
            ],
        );
        store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:instance",
                br#"{
                    "type":"minecraft:function",
                    "environment":"example:combined",
                    "function":"example:run",
                    "structure":"example:structure",
                    "max_ticks":40,
                    "rotation":"clockwise_90",
                    "max_attempts":3,
                    "required_successes":2,
                    "padding":16
                }"#
                .to_vec(),
            )],
        );

        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        let instance = store
            .frozen_registry_access()
            .typed_test_instance("example:instance")
            .unwrap();
        assert!(
            matches!(instance.kind, TestInstanceKindView::Function(id) if id.as_str() == "example:run")
        );
        assert!(
            matches!(instance.environment, TestEnvironmentHolderView::Named(id) if id.as_str() == "example:combined")
        );
        assert_eq!(instance.setup_ticks, 0);
        assert!(instance.required);
        assert_eq!(instance.rotation, TestRotation::Clockwise90);
        assert_eq!(instance.max_attempts, 3);
        assert_eq!(instance.required_successes, 2);
        assert_eq!(instance.padding, 16);
    }

    #[test]
    fn projects_network_environment_and_rejects_unknown_or_invalid_fields() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data("example:rain", weather_environment_nbt("rain")),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:broken",
                    br#"{"type":"minecraft:weather","weather":"snow"}"#.to_vec(),
                ),
            ],
        );

        store.begin_registry_configuration();
        assert!(store.finish_registry_configuration().is_err());

        let mut network_store = WorldStore::new();
        network_store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:rain",
                weather_environment_nbt("rain"),
            )],
        );
        network_store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:network_instance",
                test_instance_nbt("example:rain", false, true, true),
            )],
        );
        network_store.begin_registry_configuration();
        network_store.finish_registry_configuration().unwrap();
        let network_instance = network_store
            .frozen_registry_access()
            .typed_test_instance("example:network_instance")
            .unwrap();
        assert!(!network_instance.required);
        assert!(network_instance.manual_only);
        assert!(network_instance.sky_access);

        network_store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:broken_instance",
                br#"{
                    "type":"minecraft:block_based",
                    "environment":"example:rain",
                    "structure":"example:test",
                    "max_ticks":0
                }"#
                .to_vec(),
            )],
        );
        network_store.begin_registry_configuration();
        assert!(network_store.finish_registry_configuration().is_err());
    }

    #[test]
    fn known_pack_test_instance_boolean_fields_reject_json_numbers() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "minecraft:default",
                br#"{"type":"minecraft:all_of","definitions":[]}"#.to_vec(),
            )],
        );
        store.record_registry_entries(
            TEST_INSTANCE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:invalid_boolean",
                br#"{
                    "type":"minecraft:block_based",
                    "environment":"minecraft:default",
                    "structure":"minecraft:empty",
                    "max_ticks":1,
                    "required":0
                }"#
                .to_vec(),
            )],
        );

        store.begin_registry_configuration();
        assert!(store.finish_registry_configuration().is_err());
    }

    #[test]
    fn game_rule_codec_dispatch_rejects_unknown_wrong_type_and_out_of_range_values() {
        let cases = [
            br#"{"minecraft:custom_rule":true}"#.as_slice(),
            br#"{"minecraft:keep_inventory":2}"#.as_slice(),
            br#"{"minecraft:random_tick_speed":true}"#.as_slice(),
            br#"{"minecraft:max_snow_accumulation_height":9}"#.as_slice(),
        ];

        for rules in cases {
            let mut payload = br#"{"type":"minecraft:game_rules","rules":"#.to_vec();
            payload.extend_from_slice(rules);
            payload.push(b'}');
            let mut store = WorldStore::new();
            store.record_registry_entries(
                TEST_ENVIRONMENT_REGISTRY,
                0,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:invalid_rules",
                    payload,
                )],
            );

            store.begin_registry_configuration();
            assert!(store.finish_registry_configuration().is_err());
        }

        let mut store = WorldStore::new();
        store.record_registry_entries(
            TEST_ENVIRONMENT_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:bounded_rules",
                br#"{
                    "type":"minecraft:game_rules",
                    "rules":{
                        "minecraft:fire_spread_radius_around_player":-1,
                        "minecraft:max_minecart_speed":1000,
                        "minecraft:max_snow_accumulation_height":8
                    }
                }"#
                .to_vec(),
            )],
        );
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
    }

    fn weather_environment_nbt(weather: &str) -> Vec<u8> {
        let mut out = vec![10, 8];
        write_nbt_string(&mut out, "type");
        write_nbt_string(&mut out, "minecraft:weather");
        out.push(8);
        write_nbt_string(&mut out, "weather");
        write_nbt_string(&mut out, weather);
        out.push(0);
        out
    }

    fn test_instance_nbt(
        environment: &str,
        required: bool,
        manual_only: bool,
        sky_access: bool,
    ) -> Vec<u8> {
        let mut out = vec![10, 8];
        write_nbt_string(&mut out, "type");
        write_nbt_string(&mut out, "minecraft:block_based");
        out.push(8);
        write_nbt_string(&mut out, "environment");
        write_nbt_string(&mut out, environment);
        out.push(8);
        write_nbt_string(&mut out, "structure");
        write_nbt_string(&mut out, "example:network_instance");
        out.push(3);
        write_nbt_string(&mut out, "max_ticks");
        out.extend_from_slice(&20_i32.to_be_bytes());
        for (name, value) in [
            ("required", required),
            ("manual_only", manual_only),
            ("sky_access", sky_access),
        ] {
            out.push(1);
            write_nbt_string(&mut out, name);
            out.push(u8::from(value));
        }
        out.push(0);
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
