use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_mount_screen_open_packet() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    payload.write_var_i32(5);
    payload.write_i32(345);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_MOUNT_SCREEN_OPEN,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MountScreenOpen(MountScreenOpen {
            container_id: 12,
            inventory_columns: 5,
            entity_id: 345,
        })
    );
}

#[test]
fn decodes_open_book_packet() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_OPEN_BOOK, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::OpenBook(OpenBook {
            hand: InteractionHand::OffHand,
        })
    );
}

#[test]
fn decodes_open_sign_editor_packet() {
    let mut payload = Encoder::new();
    payload.write_i64(chunks::encode_block_pos(BlockPos {
        x: -5,
        y: 70,
        z: 12,
    }));
    payload.write_bool(false);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_OPEN_SIGN_EDITOR,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::OpenSignEditor(OpenSignEditor {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        })
    );
}

#[test]
fn decodes_pong_response_packet() {
    let mut payload = Encoder::new();
    payload.write_i64(123456789);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PONG_RESPONSE, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PongResponse(PongResponse { time: 123456789 })
    );
}

#[test]
fn decodes_low_disk_space_warning_packet() {
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_LOW_DISK_SPACE_WARNING, &[]).unwrap();
    assert_eq!(packet, PlayClientbound::LowDiskSpaceWarning);
}

#[test]
fn rejects_trailing_low_disk_space_warning_payload() {
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_LOW_DISK_SPACE_WARNING, &[1]);
    assert!(packet.is_err());
}

#[test]
fn rejects_invalid_open_book_hand_ordinal() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_OPEN_BOOK, &payload.into_inner())
        .unwrap_err();
    assert!(err.to_string().contains("invalid interaction hand"));
}
