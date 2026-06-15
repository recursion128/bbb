use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_position_waypoint_packet_wire_order() {
    let waypoint_id = Uuid::from_u128(0x123456789abcdef0011223344556677);
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bool(true);
    payload.write_uuid(waypoint_id);
    payload.write_string("minecraft:default");
    payload.write_bool(true);
    payload.write_u8(0x11);
    payload.write_u8(0x22);
    payload.write_u8(0x33);
    payload.write_var_i32(1);
    payload.write_var_i32(10);
    payload.write_var_i32(64);
    payload.write_var_i32(-5);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &payload).unwrap(),
        PlayClientbound::Waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x112233),
                },
                data: WaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            },
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_uuid().unwrap(), waypoint_id);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:default");
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_u8().unwrap(), 0x11);
    assert_eq!(decoder.read_u8().unwrap(), 0x22);
    assert_eq!(decoder.read_u8().unwrap(), 0x33);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 10);
    assert_eq!(decoder.read_var_i32().unwrap(), 64);
    assert_eq!(decoder.read_var_i32().unwrap(), -5);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_chunk_waypoint_with_named_identifier() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_bool(false);
    payload.write_string("spawn");
    payload.write_string("minecraft:default");
    payload.write_bool(false);
    payload.write_var_i32(2);
    payload.write_var_i32(-3);
    payload.write_var_i32(4);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &payload).unwrap(),
        PlayClientbound::Waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Update,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Name("spawn".to_string()),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: None,
                },
                data: WaypointData::Chunk(ChunkPos { x: -3, z: 4 }),
            },
        })
    );
}

#[test]
fn decodes_waypoint_icon_style_with_default_namespace() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bool(false);
    payload.write_string("DisplayName");
    payload.write_string("default");
    payload.write_bool(false);
    payload.write_var_i32(0);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &payload.into_inner()).unwrap(),
        PlayClientbound::Waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Name("DisplayName".to_string()),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: None,
                },
                data: WaypointData::Empty,
            },
        })
    );
}

#[test]
fn rejects_invalid_waypoint_icon_style_resource_location() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bool(false);
    payload.write_string("bad-style");
    payload.write_string("minecraft:Default");
    payload.write_bool(false);
    payload.write_var_i32(0);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &payload.into_inner())
        .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_empty_and_azimuth_waypoints() {
    let waypoint_id = Uuid::from_u128(0xabcdef001234567889abcdef00123456);
    let mut empty = Encoder::new();
    empty.write_var_i32(1);
    empty.write_bool(true);
    empty.write_uuid(waypoint_id);
    empty.write_string("minecraft:default");
    empty.write_bool(false);
    empty.write_var_i32(0);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &empty.into_inner()).unwrap(),
        PlayClientbound::Waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Untrack,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: None,
                },
                data: WaypointData::Empty,
            },
        })
    );

    let mut azimuth = Encoder::new();
    azimuth.write_var_i32(5);
    azimuth.write_bool(false);
    azimuth.write_string("north");
    azimuth.write_string("bbb:marker");
    azimuth.write_bool(false);
    azimuth.write_var_i32(3);
    azimuth.write_f32(1.5);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &azimuth.into_inner()).unwrap(),
        PlayClientbound::Waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Update,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Name("north".to_string()),
                icon: WaypointIcon {
                    style: "bbb:marker".to_string(),
                    color_rgb: None,
                },
                data: WaypointData::Azimuth(1.5),
            },
        })
    );
}

#[test]
fn rejects_unknown_waypoint_type() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bool(false);
    payload.write_string("bad");
    payload.write_string("minecraft:default");
    payload.write_bool(false);
    payload.write_var_i32(4);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_WAYPOINT, &payload.into_inner())
        .unwrap_err();
    assert!(err.to_string().contains("invalid waypoint type ordinal 4"));
}
