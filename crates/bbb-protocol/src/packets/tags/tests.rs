use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{
        decode_configuration_clientbound, decode_play_clientbound, ConfigurationClientbound,
        PlayClientbound,
    },
};

#[test]
fn decodes_update_tags_in_play_state() {
    let payload = update_tags_payload();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_TAGS, &payload).unwrap();

    assert_eq!(packet, PlayClientbound::UpdateTags(expected_update_tags()));
}

#[test]
fn decodes_update_tags_in_configuration_state() {
    let payload = update_tags_payload();

    let packet =
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_UPDATE_TAGS, &payload)
            .unwrap();

    assert_eq!(
        packet,
        ConfigurationClientbound::UpdateTags(expected_update_tags())
    );
}

#[test]
fn decodes_update_tags_with_default_namespaces() {
    let payload = single_update_tag_payload("item", "logs", &[5, 6]);
    let expected = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![5, 6],
            }],
        }],
    };

    let play_packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_TAGS, &payload).unwrap();
    assert_eq!(play_packet, PlayClientbound::UpdateTags(expected.clone()));

    let configuration_packet =
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_UPDATE_TAGS, &payload)
            .unwrap();
    assert_eq!(
        configuration_packet,
        ConfigurationClientbound::UpdateTags(expected)
    );
}

#[test]
fn rejects_update_tags_with_invalid_registry_id() {
    let payload = single_update_tag_payload("minecraft:Item", "logs", &[5, 6]);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_TAGS, &payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn rejects_update_tags_with_invalid_tag_id() {
    let payload = single_update_tag_payload("minecraft:item", "minecraft:Logs", &[5, 6]);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_TAGS, &payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn rejects_update_tags_with_trailing_bytes() {
    let mut payload = update_tags_payload();
    payload.push(0);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_TAGS, &payload).unwrap_err();
    assert!(err
        .to_string()
        .contains("trailing bytes after update tags packet"));
}

fn single_update_tag_payload(registry: &str, tag: &str, entries: &[i32]) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_string(registry);
    payload.write_var_i32(1);
    write_tag(&mut payload, tag, entries);
    payload.into_inner()
}

fn update_tags_payload() -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);

    payload.write_string("minecraft:item");
    payload.write_var_i32(2);
    write_tag(&mut payload, "minecraft:logs", &[5, 6, 7]);
    write_tag(&mut payload, "minecraft:planks", &[42]);

    payload.write_string("minecraft:block");
    payload.write_var_i32(1);
    write_tag(&mut payload, "minecraft:mineable/pickaxe", &[100, 101]);

    payload.into_inner()
}

fn write_tag(payload: &mut Encoder, name: &str, entries: &[i32]) {
    payload.write_string(name);
    payload.write_var_i32(entries.len() as i32);
    for entry in entries {
        payload.write_var_i32(*entry);
    }
}

fn expected_update_tags() -> UpdateTags {
    UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: "minecraft:logs".to_string(),
                        entries: vec![5, 6, 7],
                    },
                    TagNetworkPayload {
                        tag: "minecraft:planks".to_string(),
                        entries: vec![42],
                    },
                ],
            },
            RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:mineable/pickaxe".to_string(),
                    entries: vec![100, 101],
                }],
            },
        ],
    }
}
