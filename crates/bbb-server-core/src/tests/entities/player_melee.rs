//! `Player.attack` (`Player.java:951-1004`) on stored mobs over the attack packet.

use super::*;

const PLAYER: ConnectionId = ConnectionId(1);

fn spawn_cow(world: &mut ServerWorld, position: Vec3d) -> i32 {
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            cow_type,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(cow_type).unwrap(),
            ),
        )
        .0
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

fn attack(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    entity_id: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id }),
    )
}

fn sounds(result: &TickResult) -> Vec<&'static str> {
    result
        .events
        .iter()
        .filter_map(|event| match event.packet_for(ConnectionId(1)) {
            Some(ClientboundPlayPacket::Sound(sound)) => match sound.sound {
                SoundEventHolder::Reference { registry_id } => {
                    vanilla_sound_event_resource_id(registry_id)
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn animations(result: &TickResult) -> Vec<(i32, u8)> {
    result
        .events
        .iter()
        .filter_map(|event| match event.packet_for(ConnectionId(1)) {
            Some(ClientboundPlayPacket::EntityAnimation(animation)) => {
                Some((animation.id, animation.action))
            }
            _ => None,
        })
        .collect()
}

fn particles(result: &TickResult) -> Vec<(i32, i32)> {
    result
        .events
        .iter()
        .filter_map(|event| match event.packet_for(ConnectionId(1)) {
            Some(ClientboundPlayPacket::LevelParticles(particles)) => {
                Some((particles.particle.particle_type_id, particles.count))
            }
            _ => None,
        })
        .collect()
}

fn survival_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, Vec3d) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.admin_lists = operator_config(&[("fighter", 0xf1647e)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "fighter", 0xf1647e);
    let position = world.publication().players[0].position;
    (sender, receiver, world, position)
}

/// A full-strength grounded sword swing: Sharpness adds its `damage` effect
/// as the magic boost, the sweep hits the neighbour for `1 + 0 * base`
/// enchanted, and the dealt damage is counted and shown.
#[test]
fn sharpness_sword_sweep_hurts_neighbours_and_records_dealt_damage() {
    let (sender, receiver, mut world, position) = survival_world();
    let mut sword = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    super::sheep_shearing::enchant_for_entity_test(&world, &mut sword, "minecraft:sharpness", 1);
    world.set_inventory_slot_for_test(PLAYER, 0, sword);
    // The equipment change resets the charge; a sword recharges in 12.5 ticks.
    for _ in 0..15 {
        world.tick(Vec::new());
    }
    world.set_player_ground_for_test(PLAYER, true, 0.0);
    let target = spawn_cow(
        &mut world,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );
    let neighbour = spawn_cow(
        &mut world,
        Vec3d {
            x: position.x + 0.8,
            z: position.z + 1.5,
            ..position
        },
    );

    let attacked = attack(&sender, &receiver, &mut world, target);

    // 7 from the sword, 1 from Sharpness I.
    assert_eq!(health(&world, target), 2.0);
    // `1 + SWEEPING_DAMAGE_RATIO(0) * 7`, plus Sharpness I.
    assert_eq!(health(&world, neighbour), 8.0);
    // `LivingEntity.java:1219` tells the attacker which entity hurt the cow,
    // and the knockback pushes it away from the attacker.
    let player_entity = world.player_entity_id_for_test(PLAYER);
    assert!(attacked.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::DamageEvent(damage),
        } if damage.entity_id == target
            && damage.source_cause_id == player_entity
            && damage.source_direct_id == player_entity
            && damage.source_position.is_none()
    )));
    let pushed = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, target)
        .unwrap()
        .delta_movement();
    assert!(pushed.z > 0.0, "{pushed:?}");
    let sounds = sounds(&attacked);
    assert!(
        sounds.contains(&"minecraft:entity.player.attack.sweep"),
        "{sounds:?}"
    );
    assert!(
        !sounds.contains(&"minecraft:entity.player.attack.strong"),
        "{sounds:?}"
    );
    assert!(animations(&attacked).contains(&(target, 5)));
    let particles = particles(&attacked);
    assert!(particles.contains(&(
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SWEEP_ATTACK_ID,
        0
    )));
    assert!(particles.contains(&(
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DAMAGE_INDICATOR_ID,
        4
    )));
    assert_eq!(
        world.player_stat_for_test(
            PLAYER,
            bbb_server_types::stats::custom_stat("minecraft:damage_dealt")
        ),
        80
    );
    // `ItemStack.hurtEnemy` / `postHurtEnemy` (`ItemStack.java:535-555`): the
    // sword's `weapon` component counts the use and costs one durability.
    assert_eq!(
        world.player_stat_for_test(
            PLAYER,
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID,
            )
        ),
        1
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(PLAYER, 0)
            .component_patch
            .damage,
        Some(1)
    );
}

/// A falling mace swing is a `mace_smash` hit with the fall bonus
/// (`MaceItem.java:89-113`), then `hurtEnemy` lifts the attacker, plays the
/// ground smash, knocks neighbours up and away, and `postHurtEnemy` clears the
/// fall before the weapon wears.
#[test]
fn a_falling_mace_smash_hits_harder_and_knocks_neighbours_away() {
    let (sender, receiver, mut world, position) = survival_world();
    let mace_id = bbb_protocol::item_types::VANILLA_ITEM_MACE_ID;
    world.set_inventory_slot_for_test(
        PLAYER,
        0,
        ItemStackSummary {
            item_id: Some(mace_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    // `-3.4` attack speed recharges in 34 ticks.
    for _ in 0..30 {
        world.tick(Vec::new());
    }
    let target = spawn_cow(
        &mut world,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );
    let neighbour = spawn_cow(
        &mut world,
        Vec3d {
            x: position.x + 2.0,
            z: position.z + 1.5,
            ..position
        },
    );
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    world.set_player_ground_for_test(PLAYER, false, 4.0);

    let smashed = attack(&sender, &receiver, &mut world, target);

    let mace_smash = world.damage_type_registry_id_for_test("minecraft:mace_smash");
    assert!(smashed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::DamageEvent(damage),
        } if damage.entity_id == target && damage.source_type_id == mace_smash
    )));
    let sounds = sounds(&smashed);
    assert!(
        sounds.contains(&"minecraft:item.mace.smash_ground"),
        "{sounds:?}"
    );
    assert!(smashed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::LevelEvent(level_event),
        } if level_event.event_type == 2013 && level_event.data == 750
    )));
    let player_entity = world.player_entity_id_for_test(PLAYER);
    assert!(smashed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::SetEntityMotion(motion),
        } if motion.id == player_entity && motion.delta_movement.y == f64::from(0.01_f32)
    )));
    let pushed = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, neighbour)
        .unwrap()
        .delta_movement();
    // The 0.7 lift has had one tick of travel since.
    assert!(pushed.x > 0.0 && pushed.y > 0.4, "{pushed:?}");
    assert_eq!(world.player_fall_distance_for_test(PLAYER), 0.0);
    assert_eq!(
        world.player_stat_for_test(
            PLAYER,
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                mace_id,
            )
        ),
        1
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(PLAYER, 0)
            .component_patch
            .damage,
        Some(1)
    );
}

/// A falling fist swing crits for half again; a sprinting one adds the
/// knockback half, plays the strong sound and stops the sprint.
#[test]
fn critical_and_sprinting_swings_take_their_own_arms() {
    let (sender, receiver, mut world, position) = survival_world();
    // A fist recharges in 5 ticks.
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    world.set_player_ground_for_test(PLAYER, false, 1.0);
    let crit_target = spawn_cow(
        &mut world,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );

    let critical = attack(&sender, &receiver, &mut world, crit_target);

    assert_eq!(health(&world, crit_target), 8.5);
    assert_eq!(
        sounds(&critical),
        vec![
            "minecraft:entity.player.attack.crit",
            "minecraft:entity.cow.hurt"
        ]
    );
    assert!(animations(&critical).contains(&(crit_target, 4)));

    for _ in 0..10 {
        world.tick(Vec::new());
    }
    let player_entity = world.player_entity_id_for_test(PLAYER);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: player_entity,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    world.set_player_ground_for_test(PLAYER, true, 0.0);
    let sprint_target = spawn_cow(
        &mut world,
        Vec3d {
            x: position.x - 3.0,
            z: position.z + 1.5,
            ..position
        },
    );
    let before = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, sprint_target)
        .unwrap()
        .delta_movement();

    let sprinted = attack(&sender, &receiver, &mut world, sprint_target);

    assert_eq!(health(&world, sprint_target), 9.0);
    assert_eq!(
        sounds(&sprinted),
        vec![
            "minecraft:entity.player.attack.knockback",
            "minecraft:entity.player.attack.strong",
            "minecraft:entity.cow.hurt",
        ]
    );
    let after = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, sprint_target)
        .unwrap()
        .delta_movement();
    // The extra 0.5 knockback along the facing (+z at yaw 0) dwarfs the hurt's
    // own 0.4 away from the player, which is mostly along +x here.
    assert!(after.z - before.z > 0.3, "{before:?} -> {after:?}");
    assert!(!world.player_sprinting_for_test(PLAYER));
}

/// A survival swing at another session runs `ServerPlayer.hurtServer`: the
/// hit player alone is sent its knockback motion, both sides count the
/// damage, and a shared team without friendly fire refuses the next swing.
#[test]
fn a_session_hits_another_session_until_a_shared_team_forbids_it() {
    let (sender, receiver, mut world, _) = survival_world();
    join_loaded(&sender, &receiver, &mut world, 2, "target", 0x7a26e7);
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    world.set_player_ground_for_test(PLAYER, false, 0.0);
    let target = entity_id_of(&world, 2);
    let health_before = world.player_health_for_test(ConnectionId(2));

    let hit = attack(&sender, &receiver, &mut world, target);

    assert_eq!(
        world.player_health_for_test(ConnectionId(2)),
        health_before - 1.0
    );
    let motions: Vec<u64> = hit
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(id),
                packet: ClientboundPlayPacket::SetEntityMotion(motion),
            } if motion.id == target => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(motions, vec![2]);
    assert_eq!(
        sounds(&hit),
        vec![
            "minecraft:entity.player.hurt",
            "minecraft:entity.player.attack.strong"
        ]
    );
    assert_eq!(
        world.player_stat_for_test(
            PLAYER,
            bbb_server_types::stats::custom_stat("minecraft:damage_dealt")
        ),
        10
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(2),
            bbb_server_types::stats::custom_stat("minecraft:damage_taken")
        ),
        10
    );

    chat_command(&sender, &receiver, &mut world, 1, "team add blue");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "team modify blue friendlyFire false",
    );
    chat_command(&sender, &receiver, &mut world, 1, "team join blue fighter");
    chat_command(&sender, &receiver, &mut world, 1, "team join blue target");
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    let health_before = world.player_health_for_test(ConnectionId(2));
    let refused = attack(&sender, &receiver, &mut world, target);

    assert_eq!(world.player_health_for_test(ConnectionId(2)), health_before);
    assert_eq!(
        sounds(&refused),
        vec!["minecraft:entity.player.attack.nodamage"]
    );
}

/// `handleAnimate` (`ServerGamePacketListenerImpl.java:1682-1686`): an empty
/// swing is shown to trackers only and `ServerPlayer.swing` restarts the
/// charge, so the next blow is a weak one.
#[test]
fn an_empty_swing_is_shown_to_trackers_and_restarts_the_charge() {
    let (sender, receiver, mut world, position) = survival_world();
    join_loaded(&sender, &receiver, &mut world, 2, "watcher", 0x3a7c4e);
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    // `ChunkMap.move` pairs the two sessions on an accepted move.
    for connection_id in [1, 2] {
        play(
            &sender,
            &receiver,
            &mut world,
            connection_id,
            PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        );
    }
    world.set_player_ground_for_test(PLAYER, true, 0.0);
    let swinger = world.player_entity_id_for_test(PLAYER);

    let swung = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Swing {
            hand: InteractionHand::OffHand,
        },
    );

    let animations: Vec<(ConnectionId, i32, u8)> = swung
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::EntityAnimation(animation),
            } => Some((*connection_id, animation.id, animation.action)),
            _ => None,
        })
        .collect();
    assert_eq!(animations, vec![(ConnectionId(2), swinger, 3)]);
    let cow = spawn_cow(
        &mut world,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );
    attack(&sender, &receiver, &mut world, cow);
    // A full-strength fist would leave 9.0.
    assert!(health(&world, cow) > 9.5, "{}", health(&world, cow));
}
