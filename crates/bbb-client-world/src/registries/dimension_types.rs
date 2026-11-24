use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::registry_codec::{DecodedDimensionType, DirectDimensionTypeFacts};

use crate::FrozenRegistryAccess;

use super::{decode_client_environment_attributes, ClientEnvironmentAttributes, TIMELINE_REGISTRY};

pub const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DimensionTypeClientProjection {
    pub(crate) facts: DirectDimensionTypeFacts,
    pub(crate) timeline_resource_ids: Vec<String>,
    pub(crate) environment_attributes: ClientEnvironmentAttributes,
}

pub(super) fn decode_dimension_type_client_projection(
    resource_id: &str,
    value: DecodedDimensionType,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<DimensionTypeClientProjection, String> {
    let timeline_resource_ids = match &value.timelines {
        Some(value) => decode_timeline_holder_set(value, registry_access)
            .map_err(|error| format!("{resource_id} timelines: {error}"))?,
        None => Vec::new(),
    };
    let environment_attributes = match &value.attributes {
        Some(NbtSummaryValue::Compound(entries)) => {
            decode_client_environment_attributes(entries, registry_access)
                .map_err(|error| format!("{resource_id} attributes.{error}"))?
        }
        Some(_) => unreachable!("dimension owner emits an attribute map"),
        None => ClientEnvironmentAttributes::default(),
    };
    Ok(DimensionTypeClientProjection {
        facts: value.facts,
        timeline_resource_ids,
        environment_attributes,
    })
}

fn decode_timeline_holder_set(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<Vec<String>, String> {
    let holder_set = registry_access
        .persistent_holder_set_from_nbt(TIMELINE_REGISTRY, value)
        .map_err(|error| error.to_string())?;
    Ok(holder_set
        .entries
        .into_iter()
        .map(|entry| entry.resource_id.to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryPacketEntry, WorldIntProvider};
    use bbb_registry_codec::decode_exact_synchronized_registry_element;
    use bbb_registry_codec::registry_codec::{DimensionCardinalLight, DimensionSkybox};

    #[test]
    fn known_pack_value_uses_the_dimension_projection() {
        let known = super::super::element_value::decode_known_pack_element(
            br##"{
                "has_fixed_time": true,
                "has_skylight": true,
                "has_ceiling": false,
                "has_ender_dragon_fight": true,
                "coordinate_scale": 2.5,
                "min_y": -64,
                "height": 384,
                "logical_height": 384,
                "infiniburn": "#example:forever",
                "ambient_light": 0.25,
                "monster_spawn_light_level": {"type":"weighted_list","distribution":[
                    {"data":{"type":"clamped","source":{"type":"clamped_normal",
                        "mean":-0.0,"deviation":1.0,"min_inclusive":0,"max_inclusive":12},
                        "min_inclusive":1,"max_inclusive":8},"weight":0},
                    {"data":{"type":"trapezoid","min":2,"max":10,"plateau":4},"weight":1}
                ]},
                "monster_spawn_block_light_limit": 4,
                "skybox": "end",
                "cardinal_light": "nether",
                "default_clock": "minecraft:the_end"
            }"##,
        )
        .unwrap();
        let store = crate::WorldStore::new();
        let value =
            decode_exact_synchronized_registry_element(DIMENSION_TYPE_REGISTRY, &known).unwrap();
        let projection = decode_dimension_type_client_projection(
            "example:known",
            value.dimension_type().unwrap().clone(),
            store.frozen_registry_access(),
        )
        .unwrap();

        let facts = projection.facts;
        let scalars = facts.scalars;
        assert_eq!((scalars.min_y, scalars.height), (-64, 384));
        assert!(scalars.has_fixed_time);
        assert!(scalars.has_skylight);
        assert!(!scalars.has_ceiling);
        assert!(scalars.has_ender_dragon_fight);
        assert_eq!(scalars.coordinate_scale, 2.5);
        assert_eq!(scalars.logical_height, 384);
        assert_eq!(facts.infiniburn_tag, "example:forever");
        let provider = facts.monster_spawn_light_level;
        let report = serde_json::json!({"WeightedList":[
            {"provider":{"Clamped":{"source":{"ClampedNormal":{
                "mean_bits":0,"deviation_bits":1065353216,
                "min_inclusive":0,"max_inclusive":12}},
                "min_inclusive":1,"max_inclusive":8}},"weight":0},
            {"provider":{"Trapezoid":{
                "min_inclusive":2,"max_inclusive":10,"plateau":4}},"weight":1}
        ]});
        assert_eq!(serde_json::to_value(&provider).unwrap(), report);
        assert_eq!(
            serde_json::from_value::<WorldIntProvider>(report).unwrap(),
            provider
        );
        assert_eq!(
            (provider.min_inclusive(), provider.max_inclusive()),
            (1, 10)
        );
        assert_eq!(scalars.monster_spawn_block_light_limit, 4);
        assert_eq!(scalars.ambient_light, 0.25);
        assert_eq!(scalars.skybox, DimensionSkybox::End);
        assert_eq!(scalars.cardinal_light, DimensionCardinalLight::Nether);
        assert_eq!(facts.default_clock.as_deref(), Some("minecraft:the_end"));
    }

    #[test]
    fn strict_projection_requires_network_scalars_and_hashed_tag_shape() {
        let valid = super::super::element_value::decode_known_pack_element(
            br##"{
                "has_skylight":true,
                "has_ceiling":false,
                "has_ender_dragon_fight":false,
                "coordinate_scale":1.0,
                "min_y":-64,
                "height":384,
                "logical_height":384,
                "infiniburn":"#example:burns_forever",
                "ambient_light":0.0,
                "monster_spawn_light_level":0,
                "monster_spawn_block_light_limit":0
            }"##,
        )
        .unwrap();
        let store = crate::WorldStore::new();
        let value =
            decode_exact_synchronized_registry_element(DIMENSION_TYPE_REGISTRY, &valid).unwrap();
        let projection = decode_dimension_type_client_projection(
            "example:valid",
            value.dimension_type().unwrap().clone(),
            store.frozen_registry_access(),
        )
        .unwrap();
        let facts = projection.facts;
        let scalars = facts.scalars;
        assert!(!scalars.has_fixed_time);
        assert_eq!(facts.infiniburn_tag, "example:burns_forever");
        assert_eq!(
            facts.monster_spawn_light_level,
            WorldIntProvider::Constant(0)
        );
        assert_eq!(scalars.monster_spawn_block_light_limit, 0);

        for (json, expected) in [
            (
                br##"{
                    "has_skylight":true,
                    "has_ceiling":false,
                    "has_ender_dragon_fight":false,
                    "coordinate_scale":1.0,
                    "min_y":-64,
                    "height":384,
                    "logical_height":384,
                    "infiniburn":"example:not_hashed",
                    "ambient_light":0.0
                }"##
                .as_slice(),
                "infiniburn",
            ),
            (
                br##"{
                    "has_skylight":true,
                    "has_ceiling":false,
                    "has_ender_dragon_fight":false,
                    "coordinate_scale":0.0,
                    "min_y":-64,
                    "height":384,
                    "logical_height":384,
                    "infiniburn":"#example:valid",
                    "ambient_light":0.0
                }"##
                .as_slice(),
                "coordinate_scale",
            ),
        ] {
            let value = super::super::element_value::decode_known_pack_element(json).unwrap();
            let error = decode_exact_synchronized_registry_element(DIMENSION_TYPE_REGISTRY, &value)
                .unwrap_err();
            assert!(error.field.contains(expected), "{error}");
        }
    }

    #[test]
    fn timeline_holder_sets_follow_session_tag_order_and_diagnose_unknowns() {
        use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};

        let mut store = crate::WorldStore::new();
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("example:custom"),
                RegistryPacketEntry::stub("minecraft:day"),
            ],
        );
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: TIMELINE_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:layers".to_string(),
                    entries: vec![1, 0],
                }],
            }],
        });
        assert_eq!(
            decode_timeline_holder_set(
                &NbtSummaryValue::String("#example:layers".into()),
                store.frozen_registry_access()
            )
            .unwrap(),
            ["minecraft:day", "example:custom"]
        );
        assert_eq!(
            decode_timeline_holder_set(
                &NbtSummaryValue::String("example:missing".into()),
                store.frozen_registry_access()
            )
            .unwrap_err(),
            "missing minecraft:timeline resource example:missing"
        );
    }
}
