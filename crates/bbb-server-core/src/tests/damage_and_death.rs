//! Mob hurt/death presentation and the death side effects of mobs and players.

use super::*;

use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID};

const ALICE: u64 = 1;

fn world_with_alice() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        datapacks: datapack_config().datapacks,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    world.tick(receiver.drain());
    (sender, receiver, world)
}

/// Spawns a mob `dx` blocks east of Alice, well inside every sound's range.
fn spawn_mob(world: &mut ServerWorld, type_id: i32, dx: f64) -> (i32, uuid::Uuid) {
    let at = world.publication().players[0].position;
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        type_id,
        Vec3d { x: at.x + dx, ..at },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(type_id).unwrap(),
        ),
    )
}

fn sounds(tick: &TickResult) -> Vec<&'static str> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(ALICE),
                packet: ClientboundPlayPacket::Sound(sound),
            } => match sound.sound {
                SoundEventHolder::Reference { registry_id } => {
                    vanilla_sound_event_resource_id(registry_id)
                }
                SoundEventHolder::Direct { .. } => None,
            },
            _ => None,
        })
        .collect()
}

/// `LivingEntity.hurtServer` (LivingEntity.java:1245-1256): a mob plays its
/// own hurt sound, its death sound when the blow kills it, and nothing while
/// silent.
#[test]
fn mobs_play_their_own_hurt_and_death_sounds() {
    let (sender, receiver, mut world) = world_with_alice();
    let (_, hurt) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 2.0);
    let (dying_id, dying) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 4.0);
    let (silent_id, silent) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 6.0);
    crate::entity::living::set_health(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, dying_id)
            .unwrap(),
        0.5,
    );
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, silent_id)
        .unwrap()
        .set_silent(true);

    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("damage {hurt} 1 minecraft:generic"),
    );
    assert!(sounds(&tick).contains(&"minecraft:entity.cow.hurt"));
    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("damage {dying} 2 minecraft:generic"),
    );
    let played = sounds(&tick);
    assert!(played.contains(&"minecraft:entity.cow.death"), "{played:?}");
    assert!(!played.contains(&"minecraft:entity.cow.hurt"), "{played:?}");
    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("damage {silent} 1 minecraft:generic"),
    );
    assert!(!sounds(&tick).contains(&"minecraft:entity.cow.hurt"));
}

/// `IronGolem.hurtServer` (IronGolem.java:206-213): dropping into a new
/// crackiness band plays the crack after the hurt sound.
#[test]
fn an_iron_golem_cracks_when_its_crackiness_changes() {
    let (sender, receiver, mut world) = world_with_alice();
    let (_, golem) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, 3.0);

    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("damage {golem} 5 minecraft:generic"),
    );
    assert_eq!(sounds(&tick), vec!["minecraft:entity.iron_golem.hurt"]);
    for _ in 0..20 {
        world.tick(Vec::new());
    }
    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("damage {golem} 30 minecraft:generic"),
    );
    assert_eq!(
        sounds(&tick),
        vec![
            "minecraft:entity.iron_golem.hurt",
            "minecraft:entity.iron_golem.damage"
        ]
    );
}

/// `ServerPlayer.restoreFrom` (ServerPlayer.java:1591): a death respawn keeps
/// only base values, so Strength's attack damage modifier is gone.
#[test]
fn a_death_respawn_drops_effect_attribute_modifiers() {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
    let (sender, receiver, mut world) = world_with_alice();
    let attack_damage = |world: &ServerWorld| {
        world
            .player_attributes_for_test(ConnectionId(ALICE))
            .value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
            .unwrap()
    };
    let base = attack_damage(&world);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        "effect give alice minecraft:strength 600 1",
    );
    world.tick(Vec::new());
    assert!(attack_damage(&world) > base);

    chat_command(&sender, &receiver, &mut world, ALICE, "kill alice");
    play(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );

    assert_eq!(attack_damage(&world), base);
}

/// Swaps flags in a stored entity's record.
fn set_record_bytes(world: &mut ServerWorld, id: i32, flags: &[(&str, i8)]) {
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
    else {
        panic!("an entity record is a compound");
    };
    entries.retain(|entry| !flags.iter().any(|(name, _)| entry.name == *name));
    for (name, value) in flags {
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: (*name).into(),
            value: bbb_protocol::NbtSummaryValue::Byte(*value),
        });
    }
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, &entries);
}

/// `Creeper.killedEntity` (Creeper.java:171-180): a charged creeper's blast
/// drops one head for the mobs it kills.
#[test]
fn a_charged_creeper_blast_drops_the_heads_of_what_it_kills() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    let (_sender, receiver, mut world) = world_with_alice();
    let (creeper, _) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_CREEPER_ID, 3.0);
    set_record_bytes(
        &mut world,
        creeper,
        &[("NoAI", 1), ("Silent", 1), ("powered", 1), ("ignited", 1)],
    );
    for dx in [4.0, 2.0] {
        spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, dx);
    }
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    let heads = world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter(|item| item.2 == "minecraft:zombie_head")
        .count();
    assert_eq!(heads, 1);
}

fn disconnect_key(tick: &TickResult, connection: u64) -> Option<String> {
    closed_reasons(tick)
        .into_iter()
        .find(|(id, _)| *id == ConnectionId(connection))
        .and_then(|(_, reason)| reason.reason_component())
        .map(|component| component.text().to_string())
}

/// ServerGamePacketListenerImpl.java:1613-1616: a `§` in chat or a command
/// ends the session.
#[test]
fn illegal_chat_characters_disconnect_the_sender() {
    let (sender, receiver, mut world) = world_with_alice();
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        PlayServerbound::ChatCommand {
            command: "say \u{a7}cred".to_string(),
        },
    );
    assert_eq!(
        disconnect_key(&tick, ALICE).as_deref(),
        Some("multiplayer.disconnect.illegal_characters")
    );
}

/// ServerGamePacketListenerImpl.java:1660-1667: twenty per message against a
/// threshold of two hundred, decaying one a tick, kicks a non-operator.
#[test]
fn chat_spam_disconnects_a_non_operator_but_not_an_operator() {
    let (sender, receiver, mut world) = world_with_alice();
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let chat = || PlayServerbound::Chat(ChatMessage::unsigned("hi", 0, 0));
    let mut kicked = Vec::new();
    for _ in 0..12 {
        for connection in [ALICE, 2] {
            let tick = play(&sender, &receiver, &mut world, connection, chat());
            if let Some(key) = disconnect_key(&tick, connection) {
                kicked.push((connection, key));
            }
        }
    }
    assert_eq!(kicked, vec![(2, "disconnect.spam".to_string())]);
}

/// LivingEntity.java:697-720 with OozingMobEffect.java:35: a dead session's
/// avatar leaves its slimes when it is removed.
#[test]
fn a_dying_player_with_oozing_leaves_slimes() {
    let (sender, receiver, mut world) = world_with_alice();
    let slimes = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                    .is_some_and(|entity| {
                        entity.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SLIME_ID
                    })
            })
            .count()
    };
    chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        "effect give alice minecraft:oozing 600",
    );
    chat_command(&sender, &receiver, &mut world, ALICE, "kill alice");
    for _ in 0..25 {
        world.tick(receiver.drain());
    }
    assert_eq!(slimes(&world), 2);
}

/// ServerPlayer.java:911-916: a wither's kill credit leaves a wither rose.
#[test]
fn a_player_killed_by_a_wither_leaves_a_wither_rose() {
    let (sender, receiver, mut world) = world_with_alice();
    let (wither, wither_uuid) = spawn_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID,
        6.0,
    );
    set_record_bytes(&mut world, wither, &[("NoAI", 1), ("Silent", 1)]);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("damage alice 1000 minecraft:wither_skull by {wither_uuid}"),
    );
    let at = world.publication().players[0].position;
    let pos = BlockPos {
        x: at.x.floor() as i32,
        y: at.y.floor() as i32,
        z: at.z.floor() as i32,
    };
    let rose_block = world
        .block_state(Java26BuiltinDimension::Overworld, pos)
        .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_WITHER_ROSE_ID);
    let rose_item = world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .iter()
        .any(|item| item.2 == "minecraft:wither_rose");
    assert!(rose_block || rose_item);
}

/// `Creeper.spawnLingeringCloud` (Creeper.java:241-258): a creeper carrying
/// an effect leaves a cloud of it behind.
#[test]
fn an_exploding_creeper_with_an_effect_leaves_a_lingering_cloud() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    };
    let (sender, receiver, mut world) = world_with_alice();
    let (creeper, creeper_uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_CREEPER_ID, 3.0);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        &format!("effect give {creeper_uuid} minecraft:speed 600"),
    );
    set_record_bytes(&mut world, creeper, &[("NoAI", 1), ("ignited", 1)]);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    let clouds = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| {
                    entity.entity_type_id() == VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID
                })
        })
        .count();
    assert_eq!(clouds, 1);
}
