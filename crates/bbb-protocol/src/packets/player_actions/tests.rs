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
    let mut bytes = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_COMBAT_END, &bytes).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerCombatEnd(PlayerCombatEnd { duration: 37 })
    );
    bytes.push(0);
    let error =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_COMBAT_END, &bytes).unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after player combat end packet"));
}

#[test]
fn decodes_player_combat_kill_message() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_bytes(&nbt_string_root("You died"));
    let mut bytes = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL, &bytes).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerCombatKill(PlayerCombatKill {
            player_id: 123,
            message: crate::RichTextComponent::plain("You died"),
        })
    );
    bytes.push(0);
    let error =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL, &bytes).unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after player combat kill packet"));
}

#[test]
fn player_combat_kill_preserves_exact_trusted_component() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_bytes(&rich_death_component_root());

    let PlayClientbound::PlayerCombatKill(packet) = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL,
        &payload.into_inner(),
    )
    .unwrap() else {
        panic!("expected player combat kill packet");
    };

    assert_eq!(packet.message.text(), "Fell from a high place");
    assert!(matches!(
        packet.message.styled()[0].style.click_event,
        Some(crate::ComponentClickEvent::OpenUrl { ref url })
            if url == "https://example.invalid/death"
    ));
    assert!(matches!(
        packet.message.component(),
        crate::NbtSummaryValue::Compound(entries)
            if entries.iter().any(|entry| entry.name == "click_event")
    ));
}

#[test]
fn decodes_player_look_at_position() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_f64(10.5);
    payload.write_f64(64.0);
    payload.write_f64(-2.25);
    payload.write_bool(false);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_LOOK_AT, &payload).unwrap();
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

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_LOOK_AT, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after player look at packet")
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

fn rich_death_component_root() -> Vec<u8> {
    let mut payload = vec![10];
    write_named_nbt_string(&mut payload, "text", "Fell from a high place");
    write_named_nbt_string(&mut payload, "color", "red");
    write_named_nbt_compound(&mut payload, "click_event", |payload| {
        write_named_nbt_string(payload, "action", "open_url");
        write_named_nbt_string(payload, "url", "https://example.invalid/death");
    });
    payload.push(0);
    payload
}

fn write_named_nbt_compound(
    payload: &mut Vec<u8>,
    name: &str,
    write_body: impl FnOnce(&mut Vec<u8>),
) {
    payload.push(10);
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    write_body(payload);
    payload.push(0);
}

fn write_named_nbt_string(payload: &mut Vec<u8>, name: &str, value: &str) {
    payload.push(8);
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
    payload.extend_from_slice(value.as_bytes());
}
