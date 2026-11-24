use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_synched_data::VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID};
use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    line: &str,
) {
    let tick = console_command(sender, receiver, world, 40, line);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{line}: {:?}",
        console_command_answer(&tick)
    );
}

fn state(world: &ServerWorld, id: i32) -> i32 {
    crate::entity::fish::pufferfish::puff_state(
        world.stored_entity_for_test(DIMENSION, id).unwrap(),
    )
}

fn sting(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::GameEvent(event) } if event.event_id == VANILLA_CLIENT_GAME_EVENT_PUFFER_FISH_STING_ID))
}

fn sound(tick: &TickResult, id: &str) -> bool {
    let expected = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(id).unwrap(),
    };
    tick.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. } if sound.sound == expected))
}

#[test]
fn pufferfish_inflates_near_survival_players_and_deflates_after_they_leave() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    let (fish, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        Vec3d {
            x: 10.6,
            y: 65.3,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoGravity:1b,PersistenceRequired:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    let small = world
        .stored_entity_for_test(DIMENSION, fish)
        .unwrap()
        .bounding_box();
    world.tick_rate_mut().set_frozen(false);
    let mut states = Vec::new();
    let mut blew_up = false;
    for _ in 0..70 {
        let tick = world.tick(Vec::new());
        blew_up |= sound(&tick, "minecraft:entity.puffer_fish.blow_up");
        for event in &tick.events {
            if let WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(data),
                ..
            } = event
            {
                if data.id == fish {
                    for value in &data.values {
                        if value.data_id == VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID {
                            states.push(value.value.clone());
                        }
                    }
                }
            }
        }
        if state(&world, fish) == 2 {
            break;
        }
    }
    assert_eq!(state(&world, fish), 2);
    assert!(blew_up);
    assert!(states.contains(&EntityDataValueKind::Int(1)));
    assert!(
        world
            .stored_entity_for_test(DIMENSION, fish)
            .unwrap()
            .bounding_box()
            .max_x
            - world
                .stored_entity_for_test(DIMENSION, fish)
                .unwrap()
                .bounding_box()
                .min_x
            > small.max_x - small.min_x
    );
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 0.5,
            ..PLAYER_POSITION
        },
    );
    let mut blew_out = false;
    let mut mid = false;
    for _ in 0..150 {
        let tick = world.tick(Vec::new());
        blew_out |= sound(&tick, "minecraft:entity.puffer_fish.blow_out");
        mid |= state(&world, fish) == 1;
        if state(&world, fish) == 0 {
            break;
        }
    }
    assert_eq!(state(&world, fish), 0);
    assert!(mid && blew_out);
}

#[test]
fn pufferfish_player_touch_uses_pickup_reach_and_only_poisons_accepted_hits() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 0.5,
            ..PLAYER_POSITION
        },
    );
    let (fish, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        Vec3d {
            x: 9.6,
            y: 65.3,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,NoGravity:1b,PuffState:2}}"),
    );
    world.set_player_position_for_test(ALICE, PLAYER_POSITION);
    let before = health(&world);
    let tick = world.tick(Vec::new());
    world.tick_rate_mut().set_frozen(false);
    assert_eq!(health(&world), before - 3.0);
    assert!(sting(&tick));
    let poison = world
        .player_active_effect_for_test(ALICE, "minecraft:poison")
        .unwrap();
    assert_eq!(poison.amplifier(), 0);
    assert!((119..=120).contains(&poison.duration()));
    command(&sender, &receiver, &mut world, "effect clear alice");
    let tick = world.tick(Vec::new());
    assert!(!sting(&tick));
    assert!(world
        .player_active_effect_for_test(ALICE, "minecraft:poison")
        .is_none());
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{Silent:1b}}"),
    );
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 0.5,
            ..PLAYER_POSITION
        },
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..22 {
        world.tick(Vec::new());
    }
    world.set_player_position_for_test(ALICE, PLAYER_POSITION);
    let tick = world.tick(Vec::new());
    assert!(!sting(&tick));
    assert!(world
        .player_active_effect_for_test(ALICE, "minecraft:poison")
        .is_some());
    assert_eq!(state(&world, fish), 2);
}

#[test]
fn pufferfish_effective_tag_excludes_mobs_from_threats_and_stings() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    command(&sender, &receiver, &mut world, "gamemode spectator alice");
    let (fish, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        Vec3d {
            y: 65.3,
            ..PLAYER_POSITION
        },
    );
    let (cow, cow_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 8.9,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoGravity:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {cow_uuid} {{NoAI:1b,NoGravity:1b}}"),
    );
    for line in [
        "team add no_collision".to_owned(),
        "team modify no_collision collisionRule never".to_owned(),
        format!("team join no_collision {uuid}"),
        format!("team join no_collision {cow_uuid}"),
    ] {
        command(&sender, &receiver, &mut world, &line);
    }
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new(
        vec![(
            "minecraft:entity_type".to_owned(),
            "minecraft:not_scary_for_pufferfish".to_owned(),
            vec!["minecraft:cow".to_owned()],
        )],
    )));
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..12 {
        world.tick(Vec::new());
    }
    assert_eq!(state(&world, fish), 0);
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,PuffState:2}}"),
    );
    let before = world
        .stored_entity_for_test(DIMENSION, cow)
        .unwrap()
        .living()
        .unwrap()
        .health();
    world.tick_rate_mut().set_frozen(false);
    world.tick(Vec::new());
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, cow)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        before
    );
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new(
        Vec::new(),
    )));
    let tick = world.tick(Vec::new());
    assert!(sound(&tick, "minecraft:entity.puffer_fish.sting"));
    let cow = world
        .stored_entity_for_test(DIMENSION, cow)
        .unwrap()
        .living()
        .unwrap();
    assert!(cow.health() < before);
    assert!(cow.active_effect("minecraft:poison").is_some());
}
