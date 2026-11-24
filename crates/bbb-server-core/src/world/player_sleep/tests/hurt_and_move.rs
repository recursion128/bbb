use super::*;
use crate::entity::living::mob_effects::ActiveMobEffect;
use bbb_protocol::packets::MovePlayer;

fn movement(position: Vec3d, yaw: f32, pitch: f32) -> PlayServerbound {
    PlayServerbound::MovePlayer(MovePlayer {
        position: Some(position),
        rotation: Some(PlayerRotation {
            y_rot: yaw,
            x_rot: pitch,
        }),
        on_ground: false,
        horizontal_collision: true,
    })
}

fn teleports(tick: &TickResult) -> Vec<bbb_protocol::packets::PlayerPositionUpdate> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ID,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(*update),
            _ => None,
        })
        .collect()
}

fn survival(r: &mut Runtime) {
    let mut abilities = r.world.player_abilities_for_test(ID);
    abilities.invulnerable = false;
    abilities.instabuild = false;
    abilities.flying = false;
    r.world
        .set_player_mode_and_abilities_for_test(ID, GameType::Survival, abilities);
}

fn saved_villager(r: &mut Runtime) -> i32 {
    let ty = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
    let (id, _) = r.world.spawn_stored_entity_for_test(
        DIM,
        ty,
        Vec3d {
            x: 3.5,
            y: 101.6875,
            z: 2.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(ty).unwrap()),
    );
    let NbtSummaryValue::Compound(mut entries) = r
        .world
        .stored_entity_command_data_for_test(DIM, id)
        .unwrap()
    else {
        panic!("villager command record")
    };
    entries.push(NbtSummaryEntry {
        name: "NoAI".into(),
        value: NbtSummaryValue::Byte(1),
    });
    entries.push(NbtSummaryEntry {
        name: "sleeping_pos".into(),
        value: NbtSummaryValue::IntArray(vec![HEAD.x, HEAD.y, HEAD.z]),
    });
    r.world
        .load_stored_entity_record_for_test(DIM, id, &entries);
    r.world.set_bed_occupied(&DIM.into(), HEAD, true);
    assert_eq!(
        r.world.level(DIM).entities.get(id).unwrap().sleeping_pos(),
        Some(HEAD)
    );
    id
}

#[test]
fn sleep_runtime_wake_queries_with_sleeping_yaw_before_looking_back_at_bed() {
    let mut r = runtime("always", 1);
    r.world.levels[ID].set_y_rot(0.0);
    let stone = state("minecraft:stone", &[]);
    for (x, z) in [(3, 1), (2, 1), (1, 1), (1, 2)] {
        r.world
            .set_block(DIM, BlockPos { x, y: 101, z }, stone, UPDATE_CLIENTS);
    }
    r.click(1, FOOT);
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, Some(HEAD));
    r.play(
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: r.world.levels[ID].entity_id(),
            action: PlayerCommandAction::StopSleeping,
            data: 0,
        }),
    );
    assert_eq!(
        r.world.levels[ID].position(),
        Vec3d {
            x: 1.5,
            y: 101.0,
            z: 3.5,
        }
    );
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, None);
    assert!(!occupied(&r.world));
}

#[test]
fn sleep_runtime_move_near_ignores_rotation_far_corrects_without_rate_fall_or_stats() {
    let mut r = runtime("always", 1);
    survival(&mut r);
    r.click(1, FOOT);
    let player = &r.world.levels.player_at(ID);
    let start = player.position();
    let yaw = player.rotation().y_rot;
    let packets = player.received_move_packets;
    let walked = player
        .cold
        .stats
        .value(custom_stat("minecraft:walk_one_cm"));
    let fall = player.block_contact.fall_distance;
    let near = r.play(
        1,
        movement(
            Vec3d {
                x: start.x + 1.0,
                ..start
            },
            450.0,
            370.0,
        ),
    );
    let player = &r.world.levels.player_at(ID);
    assert_eq!(player.position(), start);
    assert_eq!(player.rotation().y_rot, yaw);
    assert_eq!(player.received_move_packets, packets);
    assert_eq!(player.block_contact.fall_distance, fall);
    assert_eq!(
        player
            .cold
            .stats
            .value(custom_stat("minecraft:walk_one_cm")),
        walked
    );
    assert!(teleports(&near).is_empty());
    let far = r.play(
        1,
        movement(
            Vec3d {
                x: start.x + 2.0,
                ..start
            },
            450.0,
            370.0,
        ),
    );
    let updates = teleports(&far);
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].position, start);
    assert_eq!((updates[0].y_rot, updates[0].x_rot), (90.0, 10.0));
    assert_eq!(updates[0].relatives_mask, 0);
    let player = &r.world.levels.player_at(ID);
    assert_eq!(player.position(), start);
    assert_eq!(player.cold.sleeping_pos, Some(HEAD));
    assert_eq!(player.received_move_packets, packets);
    assert_eq!(player.block_contact.fall_distance, fall);
    assert_eq!(
        player
            .cold
            .stats
            .value(custom_stat("minecraft:walk_one_cm")),
        walked
    );
    assert!(!player.movement.client_is_floating);
}

#[test]
fn sleep_runtime_awaiting_teleport_gate_precedes_sleep_correction() {
    let mut r = runtime("always", 1);
    r.click(1, FOOT);
    let start = r.world.levels[ID].position();
    let first = r.play(
        1,
        movement(
            Vec3d {
                x: start.x + 2.0,
                ..start
            },
            450.0,
            0.0,
        ),
    );
    let teleport = teleports(&first)[0];
    let packets = r.world.levels[ID].received_move_packets;
    let second = r.play(
        1,
        movement(
            Vec3d {
                x: start.x + 20.0,
                ..start
            },
            -450.0,
            0.0,
        ),
    );
    assert!(teleports(&second).is_empty());
    let player = &r.world.levels.player_at(ID);
    assert_eq!(player.position(), start);
    assert_eq!(player.rotation().y_rot, -90.0);
    assert_eq!(player.received_move_packets, packets);
    r.play(1, PlayServerbound::AcceptTeleportation { id: teleport.id });
    let third = r.play(
        1,
        movement(
            Vec3d {
                x: start.x + 20.0,
                ..start
            },
            180.0,
            0.0,
        ),
    );
    assert_eq!(teleports(&third).len(), 1);
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, Some(HEAD));
}

#[test]
fn sleep_runtime_player_hurt_pre_admission_guards_preserve_sleep_and_occupied() {
    for guard in 0..3 {
        let mut r = runtime("always", 1);
        if guard != 0 {
            survival(&mut r);
        }
        r.click(1, FOOT);
        if guard == 1 {
            r.world.levels[ID].entity_invulnerable = true;
        }
        if guard == 2 {
            r.world.add_player_mob_effect(
                ID,
                ActiveMobEffect::new("minecraft:fire_resistance", 200, 0).unwrap(),
                &mut Vec::new(),
            );
        }
        let damage_type = if guard == 2 {
            "minecraft:in_fire"
        } else {
            "minecraft:generic"
        };
        let (applied, events) = r.world.hurt_connected_player_by_command_for_test(
            ID,
            damage_type,
            3.0,
            None,
            None,
            None,
        );
        assert!(!applied);
        assert_eq!(r.world.levels[ID].cold.sleeping_pos, Some(HEAD));
        assert!(occupied(&r.world));
        assert!(!events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::EntityAnimation(a), .. } if a.action == 2)));
    }
}

#[test]
fn sleep_runtime_admitted_player_hurt_wakes_before_cooldown_refusal_and_publishes_clear() {
    for cooldown in [false, true] {
        let mut r = runtime("always", 2);
        survival(&mut r);
        r.click(1, FOOT);
        if cooldown {
            let player = r.world.levels.player_mut(ID).unwrap();
            player.damage.invulnerable_time = 20;
            player.damage.last_hurt = 4.0;
        }
        let health = r.world.levels[ID].damage.health;
        let entity_id = r.world.levels[ID].entity_id();
        let (applied, _) = r.world.hurt_connected_player_by_command_for_test(
            ID,
            "minecraft:generic",
            3.0,
            None,
            None,
            None,
        );
        assert_eq!(applied, !cooldown);
        let player = &r.world.levels.player_at(ID);
        assert_eq!(player.cold.sleeping_pos, None);
        assert_eq!(player.cold.sleep_timer, 0);
        assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
        assert_eq!(player.rotation().x_rot, 0.0);
        assert!(!occupied(&r.world));
        if cooldown {
            assert_eq!(player.damage.health, health);
        }
        let tick = r.tick();
        assert!(tick.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::SetEntityData(data))
                if data.id == entity_id
                    && data.values.iter().any(|value| value.data_id == 14
                        && value.value == EntityDataValueKind::OptionalBlockPos(None))
        )));
    }
}

#[test]
fn sleep_runtime_stored_villager_actual_hurt_guards_and_cooldown_wake_order() {
    for guard in 0..3 {
        let mut r = runtime("always", 1);
        let id = saved_villager(&mut r);
        let entity = r.world.level_mut(DIM).entities.get_mut(id).unwrap();
        if guard == 1 {
            entity.set_invulnerable(true);
        }
        if guard == 2 {
            entity.living_mut().unwrap().add_active_effect(
                ActiveMobEffect::new("minecraft:fire_resistance", 200, 0).unwrap(),
            );
        }
        let living = entity.living_mut().unwrap();
        living.invulnerable_time = 20;
        living.last_hurt = 4.0;
        let health = living.health();
        let damage_type = if guard == 2 {
            "minecraft:in_fire"
        } else {
            "minecraft:generic"
        };
        let result = r
            .world
            .hurt_stored_entity_by_command_for_test(DIM, id, damage_type, 3.0, None)
            .unwrap();
        assert!(!result.applied);
        let entity = r.world.level(DIM).entities.get(id).unwrap();
        assert_eq!(entity.living().unwrap().health(), health);
        if guard == 0 {
            assert_eq!(entity.sleeping_pos(), None);
            assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_STANDING_ID);
            assert_eq!(entity.x_rot(), 0.0);
            assert_eq!([entity.bb_width(), entity.bb_height()], [0.6, 1.95]);
            assert!(!occupied(&r.world));
            let tick = r.tick();
            assert!(tick.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == id && data.values.iter().any(|value| value.data_id == 14
                    && value.value == EntityDataValueKind::OptionalBlockPos(None)))));
        } else {
            assert_eq!(entity.sleeping_pos(), Some(HEAD));
            assert_eq!([entity.bb_width(), entity.bb_height()], [0.2, 0.2]);
            assert!(occupied(&r.world));
        }
    }
}

#[test]
fn sleep_runtime_removed_bed_actual_stored_tick_clears_sleep_and_restores_box() {
    let mut r = runtime("always", 1);
    let id = saved_villager(&mut r);
    let entity = r.world.level_mut(DIM).entities.get_mut(id).unwrap();
    entity.set_rot(35.0, 12.0);
    r.world
        .set_block(DIM, HEAD, state("minecraft:air", &[]), UPDATE_CLIENTS);
    r.tick();
    let entity = r.world.level(DIM).entities.get(id).unwrap();
    assert_eq!(entity.sleeping_pos(), None);
    assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!([entity.bb_width(), entity.bb_height()], [0.6, 1.95]);
    let box_ = entity.bounding_box();
    assert_eq!(
        box_,
        bbb_server_types::geometry::Aabb::of_entity(
            entity.position(),
            entity.bb_width(),
            entity.bb_height()
        )
    );
}
