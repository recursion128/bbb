//! Tamable deaths deliver the real combat message only to an online owner.

use super::tame_sit::an_owned_wolf;
use super::*;

fn named_wolf(world: &mut ServerWorld, owner: Uuid) -> String {
    let wolf = an_owned_wolf(world, owner, 2.0, true);
    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, wolf)
        .unwrap();
    entity.set_custom_name(Some(bbb_protocol::RichTextComponent::plain("Fang")));
    entity.uuid().to_string()
}

fn death_chats(tick: &TickResult) -> Vec<(ConnectionId, String)> {
    system_chats(tick)
        .into_iter()
        .filter(|(_, message)| message.contains("death.attack.") || message.contains("death.fell."))
        .collect()
}

#[test]
fn a_pet_death_reaches_only_its_online_owner_even_in_another_dimension() {
    for cross_dimension in [false, true] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
        join_loaded(&sender, &receiver, &mut world, 2, "attacker", 0xbeef);
        join_loaded(&sender, &receiver, &mut world, 3, "observer", 0xf00d);
        console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamemode spectator observer",
        );
        let wolf = named_wolf(&mut world, Uuid::from_u128(0xfeed));
        if cross_dimension {
            assert!(world.teleport_player(
                ConnectionId(1),
                crate::world::TeleportTransition {
                    new_level: Java26BuiltinDimension::Nether.into(),
                    position: Vec3d {
                        x: 1.5,
                        y: 70.0,
                        z: 1.5
                    },
                    y_rot: 0.0,
                    x_rot: 0.0,
                    relatives_mask: 0,
                },
                &mut Vec::new()
            ));
        }
        let death = console_command(
            &sender,
            &receiver,
            &mut world,
            2,
            &format!("damage {wolf} 100 minecraft:player_attack by attacker"),
        );
        let messages = death_chats(&death);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert_eq!(messages[0].0, ConnectionId(1));
        assert!(
            messages[0].1.contains("death.attack.player"),
            "{messages:?}"
        );
        assert!(messages[0].1.contains("Fang"), "{messages:?}");
        assert!(messages[0].1.contains("attacker"), "{messages:?}");
    }
}

#[test]
fn a_pet_death_is_silent_when_disabled_or_its_owner_is_offline() {
    for offline in [false, true] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
        join_loaded(&sender, &receiver, &mut world, 2, "observer", 0xbeef);
        let wolf = named_wolf(&mut world, Uuid::from_u128(0xfeed));
        if offline {
            enqueue(
                &sender,
                WorldCommand::Disconnect {
                    connection_id: ConnectionId(1),
                },
            )
            .unwrap();
            world.tick(receiver.drain());
        } else {
            console_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "gamerule show_death_messages false",
            );
        }
        let death = console_command(
            &sender,
            &receiver,
            &mut world,
            2,
            &format!("damage {wolf} 100 minecraft:on_fire"),
        );
        assert_eq!(console_command_answer(&death).0, Some(1));
        assert!(death_chats(&death).is_empty());
    }
}

#[test]
fn a_pet_fall_keeps_its_assisting_attacker_and_kill_uses_its_real_source() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    join_loaded(&sender, &receiver, &mut world, 2, "attacker", 0xbeef);
    let wolf = named_wolf(&mut world, Uuid::from_u128(0xfeed));
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {wolf} 5 minecraft:player_attack by attacker"),
    );
    // Wolf.java:393-400 stood it up; its owner seats it again so it falls
    // instead of teleporting back.
    let wolf_id = world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, wolf.parse().unwrap())
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: wolf_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let position = world.publication().players[0].position;
    console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!(
            "teleport {wolf} {} {} {}",
            position.x + 2.0,
            position.y + 14.0,
            position.z
        ),
    );
    let death = (0..100)
        .find_map(|_| {
            let tick = world.tick(receiver.drain());
            (!death_chats(&tick).is_empty()).then_some(tick)
        })
        .expect("the wounded wolf dies from its real fall");
    let messages = death_chats(&death);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0].0, ConnectionId(1));
    assert!(messages[0].1.contains("death.fell.assist"), "{messages:?}");
    assert!(messages[0].1.contains("attacker"), "{messages:?}");
    assert!(messages[0].1.contains("Fang"), "{messages:?}");

    let wolf = named_wolf(&mut world, Uuid::from_u128(0xfeed));
    let death = console_command(&sender, &receiver, &mut world, 4, &format!("kill {wolf}"));
    let messages = death_chats(&death);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0].0, ConnectionId(1));
    assert!(
        messages[0].1.contains("death.attack.genericKill"),
        "{messages:?}"
    );
    assert!(messages[0].1.contains("Fang"), "{messages:?}");
}

#[test]
fn a_discarded_named_creeper_is_named_in_its_victims_death_message() {
    use super::feeding::{interact_entity_packet, stack_of};
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    join_loaded(&sender, &receiver, &mut world, 2, "observer", 0xbeef);
    named_wolf(&mut world, Uuid::from_u128(0xfeed));
    let position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID;
    let creeper = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            kind,
            Vec3d {
                x: position.x + 1.0,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
        )
        .0;
    let uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, creeper)
        .unwrap()
        .uuid();
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!(r#"data merge entity {uuid} {{NoAI:1b,CustomName:{{text:"Boom"}}}}"#),
    );
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:flint_and_steel", 1));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(creeper),
    );
    let messages = (0..35)
        .find_map(|_| {
            let tick = world.tick(receiver.drain());
            let messages = death_chats(&tick);
            (!messages.is_empty()).then_some(messages)
        })
        .expect("the ignited creeper explodes and kills Fang");
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, creeper)
        .is_none());
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0].0, ConnectionId(1));
    assert!(
        messages[0].1.contains("death.attack.explosion.player"),
        "{messages:?}"
    );
    assert!(messages[0].1.contains("Fang"), "{messages:?}");
    assert!(messages[0].1.contains("Boom"), "{messages:?}");
}

/// `DamageSource.getLocalizedDeathMessage` (`DamageSource.java:79-84`): a
/// killer holding a custom-named stack names it through the `.item` variant.
#[test]
fn a_pet_killed_by_a_named_weapon_uses_the_item_message() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    join_loaded(&sender, &receiver, &mut world, 2, "attacker", 0xbeef);
    let wolf = named_wolf(&mut world, Uuid::from_u128(0xfeed));
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity attacker weapon.mainhand with diamond_sword[custom_name=\"Blade\"]",
    );
    let death = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("damage {wolf} 100 minecraft:player_attack by attacker"),
    );
    let messages = death_chats(&death);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].1.contains("death.attack.player.item"),
        "{messages:?}"
    );
    assert!(messages[0].1.contains("Blade"), "{messages:?}");
}
