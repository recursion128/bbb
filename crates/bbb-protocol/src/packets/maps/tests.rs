use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_map_item_data_with_decorations_and_patch() {
    let mut payload = Encoder::new();
    payload.write_var_i32(42);
    payload.write_i8(2);
    payload.write_bool(true);
    payload.write_bool(true);
    payload.write_var_i32(2);
    write_decoration(&mut payload, 0, 10, -5, 18, None);
    write_decoration(&mut payload, 4, -20, 30, 7, Some("Village"));
    payload.write_u8(2);
    payload.write_u8(2);
    payload.write_u8(3);
    payload.write_u8(4);
    payload.write_var_i32(4);
    payload.write_bytes(&[1, 2, 3, 4]);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_MAP_ITEM_DATA, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MapItemData(MapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: Some(vec![
                MapDecoration {
                    type_id: 0,
                    x: 10,
                    y: -5,
                    rot: 2,
                    name: None,
                },
                MapDecoration {
                    type_id: 4,
                    x: -20,
                    y: 30,
                    rot: 7,
                    name: Some(crate::RichTextComponent::plain("Village")),
                },
            ]),
            color_patch: Some(MapColorPatch {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_MAP_ITEM_DATA, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after map item data packet")
    );
}

#[test]
fn decodes_map_item_data_without_optional_sections() {
    let mut payload = Encoder::new();
    payload.write_var_i32(7);
    payload.write_i8(0);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_u8(0);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_MAP_ITEM_DATA, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MapItemData(MapItemData {
            map_id: 7,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: None,
        })
    );
}

#[test]
fn rejects_map_color_patch_with_wrong_color_count() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_i8(1);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_u8(2);
    payload.write_u8(2);
    payload.write_u8(0);
    payload.write_u8(0);
    payload.write_var_i32(3);
    payload.write_bytes(&[1, 2, 3]);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_MAP_ITEM_DATA, &payload.into_inner())
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("map color patch has 3 colors, expected 4"));
}

#[test]
fn rejects_map_color_patch_outside_map_bounds() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_i8(1);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_u8(2);
    payload.write_u8(1);
    payload.write_u8(127);
    payload.write_u8(0);
    payload.write_var_i32(2);
    payload.write_bytes(&[1, 2]);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_MAP_ITEM_DATA, &payload.into_inner())
        .unwrap_err();
    assert!(err.to_string().contains("exceeds 128x128 map bounds"));
}

#[test]
fn encodes_map_item_data_with_distinct_optional_states() {
    let update = MapItemData {
        map_id: -42,
        scale: -2,
        locked: true,
        decorations: Some(vec![MapDecoration {
            type_id: 4,
            x: -20,
            y: 30,
            rot: 23,
            name: Some(crate::RichTextComponent::plain("Village")),
        }]),
        color_patch: Some(MapColorPatch {
            start_x: 3,
            start_y: 4,
            width: 2,
            height: 2,
            colors: vec![1, 2, 3, 4],
        }),
    };
    let (packet_id, payload) = encode_play_clientbound_map_item_data(&update).unwrap();
    assert_eq!(packet_id, ids::play::CLIENTBOUND_MAP_ITEM_DATA);
    let PlayClientbound::MapItemData(decoded) =
        decode_play_clientbound(packet_id, &payload).unwrap()
    else {
        panic!("map encoder emitted another packet kind")
    };
    let mut expected = update;
    expected.decorations.as_mut().unwrap()[0].rot = 7;
    assert_eq!(decoded, expected);

    let none = MapItemData {
        map_id: 7,
        scale: 0,
        locked: false,
        decorations: None,
        color_patch: None,
    };
    let empty = MapItemData {
        decorations: Some(Vec::new()),
        ..none.clone()
    };
    let (_, none_payload) = encode_play_clientbound_map_item_data(&none).unwrap();
    let (_, empty_payload) = encode_play_clientbound_map_item_data(&empty).unwrap();
    assert_ne!(none_payload, empty_payload);
    assert_eq!(none_payload.last(), Some(&0));
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_MAP_ITEM_DATA, &empty_payload).unwrap(),
        PlayClientbound::MapItemData(empty)
    );
}

#[test]
fn map_item_data_encoder_rejects_noncanonical_collections_and_patches() {
    let base = MapItemData {
        map_id: 1,
        scale: 0,
        locked: false,
        decorations: None,
        color_patch: None,
    };
    let too_many = MapItemData {
        decorations: Some(vec![
            MapDecoration {
                type_id: 1,
                x: 0,
                y: 0,
                rot: 0,
                name: None,
            };
            MAX_MAP_DECORATIONS + 1
        ]),
        ..base.clone()
    };
    assert!(matches!(
        encode_play_clientbound_map_item_data(&too_many),
        Err(ProtocolError::PacketTooLarge(_, MAX_MAP_DECORATIONS))
    ));

    for patch in [
        MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 0,
            height: 1,
            colors: Vec::new(),
        },
        MapColorPatch {
            start_x: 127,
            start_y: 0,
            width: 2,
            height: 1,
            colors: vec![0; 2],
        },
        MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 2,
            height: 2,
            colors: vec![0; 3],
        },
    ] {
        assert!(encode_play_clientbound_map_item_data(&MapItemData {
            color_patch: Some(patch),
            ..base.clone()
        })
        .is_err());
    }
}

fn write_decoration(
    payload: &mut Encoder,
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    name: Option<&str>,
) {
    payload.write_var_i32(type_id);
    payload.write_i8(x);
    payload.write_i8(y);
    payload.write_u8(rot);
    match name {
        Some(name) => {
            payload.write_bool(true);
            payload.write_bytes(&nbt_string_root(name));
        }
        None => payload.write_bool(false),
    }
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut out = vec![8];
    out.extend_from_slice(&(text.len() as u16).to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    out
}
