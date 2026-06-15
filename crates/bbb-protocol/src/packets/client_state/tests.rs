use super::super::{decode_play_clientbound, BlockPos, PlayClientbound};
use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
};
use uuid::Uuid;

#[test]
fn decodes_boss_event_operations() {
    let id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);

    let payload = boss_event_payload(id, 0, |payload| {
        payload.write_bytes(&nbt_string_root("Raid"));
        payload.write_f32(0.75);
        payload.write_var_i32(5);
        payload.write_var_i32(3);
        payload.write_u8(BOSS_EVENT_FLAG_DARKEN_SCREEN | BOSS_EVENT_FLAG_CREATE_WORLD_FOG);
    });
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BossEvent(BossEvent {
            id,
            operation: BossEventOperation::Add {
                name: "Raid".to_string(),
                progress: 0.75,
                color: BossBarColor::Purple,
                overlay: BossBarOverlay::Notched12,
                flags: BossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        })
    );

    let payload = boss_event_payload(id, 1, |_| {});
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BossEvent(BossEvent {
            id,
            operation: BossEventOperation::Remove,
        })
    );

    let payload = boss_event_payload(id, 2, |payload| {
        payload.write_f32(0.25);
    });
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BossEvent(BossEvent {
            id,
            operation: BossEventOperation::UpdateProgress { progress: 0.25 },
        })
    );

    let payload = boss_event_payload(id, 3, |payload| {
        payload.write_bytes(&nbt_string_root("Dragon"));
    });
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BossEvent(BossEvent {
            id,
            operation: BossEventOperation::UpdateName {
                name: "Dragon".to_string(),
            },
        })
    );

    let payload = boss_event_payload(id, 4, |payload| {
        payload.write_var_i32(6);
        payload.write_var_i32(4);
    });
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BossEvent(BossEvent {
            id,
            operation: BossEventOperation::UpdateStyle {
                color: BossBarColor::White,
                overlay: BossBarOverlay::Notched20,
            },
        })
    );

    let payload = boss_event_payload(id, 5, |payload| {
        payload.write_u8(BOSS_EVENT_FLAG_PLAY_MUSIC);
    });
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BossEvent(BossEvent {
            id,
            operation: BossEventOperation::UpdateProperties {
                flags: BossEventFlags {
                    darken_screen: false,
                    play_music: true,
                    create_world_fog: false,
                },
            },
        })
    );
}

#[test]
fn decodes_change_difficulty_with_wrapped_ids() {
    let payload = change_difficulty_payload(2, true);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ChangeDifficulty(ChangeDifficulty {
            difficulty: Difficulty::Normal,
            locked: true,
        })
    );

    let payload = change_difficulty_payload(5, false);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ChangeDifficulty(ChangeDifficulty {
            difficulty: Difficulty::Easy,
            locked: false,
        })
    );

    let payload = change_difficulty_payload(-1, false);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ChangeDifficulty(ChangeDifficulty {
            difficulty: Difficulty::Hard,
            locked: false,
        })
    );
}

#[test]
fn decodes_cooldown_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_string("minecraft:ender_pearl");
    payload.write_var_i32(40);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_COOLDOWN, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Cooldown(Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 40,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:ender_pearl");
    assert_eq!(decoder.read_var_i32().unwrap(), 40);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_cooldown_packet_with_default_namespace() {
    let mut payload = Encoder::new();
    payload.write_string("ender_pearl");
    payload.write_var_i32(40);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_COOLDOWN, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Cooldown(Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 40,
        })
    );
}

#[test]
fn rejects_invalid_cooldown_group() {
    let mut payload = Encoder::new();
    payload.write_string("minecraft:EnderPearl");
    payload.write_var_i32(40);
    let payload = payload.into_inner();

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_COOLDOWN, &payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_damage_event_without_source_position_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_var_i32(7);
    payload.write_var_i32(0);
    payload.write_var_i32(35);
    payload.write_bool(false);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_DAMAGE_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::DamageEvent(DamageEvent {
            entity_id: 123,
            source_type_id: 7,
            source_cause_id: -1,
            source_direct_id: 34,
            source_position: None,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 35);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn decodes_damage_event_with_source_position_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(456);
    payload.write_var_i32(12);
    payload.write_var_i32(79);
    payload.write_var_i32(0);
    payload.write_bool(true);
    payload.write_f64(1.25);
    payload.write_f64(-2.5);
    payload.write_f64(64.0);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_DAMAGE_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::DamageEvent(DamageEvent {
            entity_id: 456,
            source_type_id: 12,
            source_cause_id: 78,
            source_direct_id: -1,
            source_position: Some(Vec3d {
                x: 1.25,
                y: -2.5,
                z: 64.0,
            }),
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 456);
    assert_eq!(decoder.read_var_i32().unwrap(), 12);
    assert_eq!(decoder.read_var_i32().unwrap(), 79);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_f64().unwrap(), 1.25);
    assert_eq!(decoder.read_f64().unwrap(), -2.5);
    assert_eq!(decoder.read_f64().unwrap(), 64.0);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_update_mob_effect_packet_wire_order_and_flags() {
    let flags = MOB_EFFECT_FLAG_AMBIENT | MOB_EFFECT_FLAG_SHOW_ICON | MOB_EFFECT_FLAG_BLEND;
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_var_i32(5);
    payload.write_var_i32(2);
    payload.write_var_i32(600);
    payload.write_u8(flags);
    let payload = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::UpdateMobEffect(UpdateMobEffect {
            entity_id: 123,
            effect_id: 5,
            amplifier: 2,
            duration_ticks: 600,
            flags: MobEffectFlags {
                raw: flags,
                ambient: true,
                visible: false,
                show_icon: true,
                blend: true,
            },
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_var_i32().unwrap(), 600);
    assert_eq!(decoder.read_u8().unwrap(), flags);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_remove_mob_effect_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_var_i32(5);
    let payload = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::RemoveMobEffect(RemoveMobEffect {
            entity_id: 123,
            effect_id: 5,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_player_health() {
    let mut payload = Encoder::new();
    payload.write_f32(0.0);
    payload.write_var_i32(17);
    payload.write_f32(1.5);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_HEALTH, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetHealth(PlayerHealth {
            health: 0.0,
            food: 17,
            saturation: 1.5,
        })
    );
}

#[test]
fn decodes_player_experience() {
    let mut payload = Encoder::new();
    payload.write_f32(0.625);
    payload.write_var_i32(12);
    payload.write_var_i32(345);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_EXPERIENCE, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetExperience(PlayerExperience {
            progress: 0.625,
            level: 12,
            total: 345,
        })
    );
}

#[test]
fn decodes_held_slot() {
    let mut payload = Encoder::new();
    payload.write_var_i32(6);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_HELD_SLOT, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 6 })
    );
}

#[test]
fn decodes_game_event_and_set_time() {
    let mut payload = Encoder::new();
    payload.write_u8(7);
    payload.write_f32(0.75);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_EVENT, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::GameEvent(GameEvent {
            event_id: 7,
            param: 0.75,
        })
    );

    let mut payload = Encoder::new();
    payload.write_i64(12345);
    payload.write_var_i32(2);
    payload.write_var_i32(0);
    payload.write_var_i64(6000);
    payload.write_f32(0.25);
    payload.write_f32(1.0);
    payload.write_var_i32(1);
    payload.write_var_i64(18000);
    payload.write_f32(0.5);
    payload.write_f32(0.0);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_TIME, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetTime(PlayTime {
            game_time: 12345,
            clock_updates: vec![
                ClockUpdate {
                    clock_id: 0,
                    total_ticks: 6000,
                    partial_tick: 0.25,
                    rate: 1.0,
                },
                ClockUpdate {
                    clock_id: 1,
                    total_ticks: 18000,
                    partial_tick: 0.5,
                    rate: 0.0,
                },
            ],
        })
    );
}

#[test]
fn decodes_title_camera_and_ticking_packets() {
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT,
        &nbt_string_root("Action"),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetActionBarText(SetActionBarText {
            content: "Action".to_string(),
        })
    );

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_TITLE_TEXT,
        &nbt_string_root("Title"),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetTitleText(SetTitleText {
            content: "Title".to_string(),
        })
    );

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT,
        &nbt_string_root("Subtitle"),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetSubtitleText(SetSubtitleText {
            content: "Subtitle".to_string(),
        })
    );

    let mut payload = Encoder::new();
    payload.write_bool(true);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_CLEAR_TITLES, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ClearTitles(ClearTitles { reset_times: true })
    );

    let mut payload = Encoder::new();
    payload.write_i32(10);
    payload.write_i32(70);
    payload.write_i32(-5);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_TITLES_ANIMATION,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetTitlesAnimation(SetTitlesAnimation {
            fade_in: 10,
            stay: 70,
            fade_out: -5,
        })
    );

    let mut payload = Encoder::new();
    payload.write_f32(0.25);
    payload.write_bool(true);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_TICKING_STATE, &payload.into_inner())
            .unwrap();
    let PlayClientbound::TickingState(ticking_state) = packet else {
        panic!("wrong packet");
    };
    assert_eq!(
        ticking_state,
        TickingState {
            tick_rate: 0.25,
            frozen: true,
        }
    );
    assert_eq!(ticking_state.clamped_tick_rate(), 1.0);
    assert_eq!(
        TickingState {
            tick_rate: 2.5,
            frozen: false,
        }
        .clamped_tick_rate(),
        2.5
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(40);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_TICKING_STEP, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::TickingStep(TickingStep { tick_steps: 40 })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(12345);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_CAMERA, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetCamera(SetCamera { camera_id: 12345 })
    );
}

#[test]
fn decodes_player_abilities_spawn_distance_and_system_chat() {
    let mut payload = Encoder::new();
    payload.write_u8(0b0000_1101);
    payload.write_f32(0.05);
    payload.write_f32(0.1);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_PLAYER_ABILITIES,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::PlayerAbilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        })
    );

    let mut payload = Encoder::new();
    payload.write_string("overworld");
    payload.write_i64(encode_block_pos(-5, 70, 12));
    payload.write_f32(90.0);
    payload.write_f32(-10.0);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetDefaultSpawnPosition(SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        })
    );

    let mut invalid = Encoder::new();
    invalid.write_string("minecraft:Overworld");
    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
        &invalid.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetSimulationDistance(SetSimulationDistance { distance: 12 })
    );

    let mut payload = nbt_string_root("Server restarting");
    payload.push(1);
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SYSTEM_CHAT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SystemChat(SystemChat {
            content: "Server restarting".to_string(),
            overlay: true,
        })
    );
}

fn boss_event_payload(id: Uuid, operation: i32, write_body: impl FnOnce(&mut Encoder)) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_uuid(id);
    payload.write_var_i32(operation);
    write_body(&mut payload);
    payload.into_inner()
}

fn change_difficulty_payload(id: i32, locked: bool) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_var_i32(id);
    payload.write_bool(locked);
    payload.into_inner()
}

fn encode_block_pos(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut payload = vec![8];
    payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
    payload.extend_from_slice(text.as_bytes());
    payload
}
