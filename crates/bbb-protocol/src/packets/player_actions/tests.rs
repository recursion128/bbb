use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_player_combat_enter_empty_packet() {
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER, &[]).unwrap();
    assert_eq!(packet, PlayClientbound::PlayerCombatEnter);
}

#[test]
fn rejects_player_combat_enter_with_trailing_bytes() {
    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER, &[1]).unwrap_err();
    assert!(err
        .to_string()
        .contains("trailing bytes after player combat enter packet"));
}

#[test]
fn decodes_player_combat_end_duration() {
    let mut payload = Encoder::new();
    payload.write_var_i32(37);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLAYER_COMBAT_END,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerCombatEnd(PlayerCombatEnd { duration: 37 })
    );
}

#[test]
fn decodes_player_combat_kill_message() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_bytes(&nbt_string_root("You died"));

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerCombatKill(PlayerCombatKill {
            player_id: 123,
            message: "You died".to_string(),
        })
    );
}

#[test]
fn decodes_player_look_at_position() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_f64(10.5);
    payload.write_f64(64.0);
    payload.write_f64(-2.25);
    payload.write_bool(false);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_LOOK_AT, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerLookAt(PlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: Vec3d {
                x: 10.5,
                y: 64.0,
                z: -2.25,
            },
            target: None,
        })
    );
}

#[test]
fn decodes_player_look_at_entity_target() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_f64(1.0);
    payload.write_f64(2.0);
    payload.write_f64(3.0);
    payload.write_bool(true);
    payload.write_var_i32(456);
    payload.write_var_i32(1);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_LOOK_AT, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerLookAt(PlayerLookAt {
            from_anchor: EntityAnchor::Feet,
            position: Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            target: Some(PlayerLookAtTarget {
                entity_id: 456,
                to_anchor: EntityAnchor::Eyes,
            }),
        })
    );
}

#[test]
fn rejects_invalid_player_look_at_anchor() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_bool(false);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_LOOK_AT, &payload.into_inner())
        .unwrap_err();
    assert!(err.to_string().contains("invalid entity anchor ordinal 2"));
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut out = vec![8];
    out.extend_from_slice(&(text.len() as u16).to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    out
}
