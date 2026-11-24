//! The command surface that writes the world: `/setblock`, `/fill`,
//! `/teleport`, `/gamemode`, `/tag`, `/experience`, `/setworldspawn`, the admin
//! copy of a broadcast success line, and the world oracle's setup script.

use bbb_facts::sound_sources::SoundSource;
use bbb_protocol::packets::{PlayerExperience, SoundEventHolder};
use bbb_server_types::save::Java26LevelKey;

use super::*;

mod block_entity_data;
mod enchantment_description;
mod entity_writes;
mod execute_forks;
mod fetch_profile;
mod items;
mod selector_options;
mod time;
mod world_oracle;

/// `/teleport` is the trigger the cross-level arm of `ServerPlayer.teleport`
/// (`ServerPlayer.java:1102-1134`) was missing: a target standing in another
/// level is pulled into the source's own (`TeleportCommand.java:70`), and the
/// session it belongs to is the one that changes level.
#[test]
fn a_teleport_command_pulls_a_target_into_the_sources_level() {
    let (sender, receiver) = mailbox();
    for (connection, name, uuid) in [(1_u64, "alice", 1_u128), (2, "bob", 2)] {
        enqueue(
            &sender,
            WorldCommand::Join {
                client_information: Default::default(),
                context: JoinContext::default(),
                connection_id: ConnectionId(connection),
                profile: profile(name, uuid),
            },
        )
        .unwrap();
    }
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    world.tick(receiver.drain());

    // `/teleport` is gamemaster-gated (TeleportCommand.java:40).
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "op", AdminScope::Manage),
            request: AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_id(Uuid::from_u128(1)),
                permission_level: Some(CommandPermission::GameMaster),
                bypasses_player_limit: None,
            }]),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    // Alice goes to the nether first, so the command's source level is one bob
    // is not in.
    let mut moved = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 8.5,
                y: 70.0,
                z: 8.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut moved,
    ));
    assert_eq!(
        world.player_dimension(ConnectionId(2)),
        Java26BuiltinDimension::Overworld
    );

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "teleport bob 100 70 -20".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());

    // The session moved, which is `ServerPlayer.setServerLevel` (`:1118`).
    assert_eq!(
        world.player_dimension(ConnectionId(2)),
        Java26BuiltinDimension::Nether
    );

    // The order the cross-level arm sends in: the `KEEP_ALL_DATA` respawn
    // (`:1104`) before the position update (`:1119`), both to bob.
    let bob_packets = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet,
            } => Some(packet),
            _ => None,
        })
        .collect::<Vec<_>>();
    let respawn = bob_packets
        .iter()
        .position(|packet| matches!(packet, ClientboundPlayPacket::Respawn(_)))
        .expect("the cross-level arm opens with a respawn");
    let position = bob_packets
        .iter()
        .position(|packet| matches!(packet, ClientboundPlayPacket::PlayerPosition(_)))
        .expect("the transition ends in a position update");
    assert!(respawn < position, "{bob_packets:?}");
    let ClientboundPlayPacket::Respawn(respawn) = bob_packets[respawn] else {
        unreachable!("checked above")
    };
    assert_eq!(respawn.data_to_keep, 3);
    assert_eq!(respawn.common_spawn_info.dimension, "minecraft:the_nether");
    let ClientboundPlayPacket::PlayerPosition(update) = bob_packets[position] else {
        unreachable!("checked above")
    };
    // Neither the destination nor the rotation was written with a `~`, so no
    // axis is relative and the point travels as typed -- with the centre
    // correction the horizontal axes already took (WorldCoordinate.java:34-36).
    assert_eq!(update.position.x, 100.5);
    assert_eq!(update.position.z, -19.5);
    assert_eq!(
        update.relatives_mask,
        PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT
    );

    // Alice, who ran it, is answered with the location key and nobody else is.
    let alice_chat = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => Some(chat.content.component().clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(alice_chat.len(), 1, "{alice_chat:?}");
    let NbtSummaryValue::Compound(entries) = &alice_chat[0] else {
        panic!("a component is a compound");
    };
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "translate")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string(
            "commands.teleport.success.location.single"
        ))
    );
}

/// A same-level `/teleport` written with `~` keeps the relative axes on the
/// wire: `performTeleport` hands `teleportTo` the difference from where the
/// victim stands (TeleportCommand.java:245-247) and the mask is what tells the
/// client to add its own position back
/// (`PositionMoveRotation.calculateAbsolute`, PositionMoveRotation.java:40-63).
#[test]
fn a_relative_teleport_command_sends_the_offset_and_lands_on_the_sum() {
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "op", AdminScope::Manage),
            request: AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_id(Uuid::from_u128(1)),
                permission_level: Some(CommandPermission::GameMaster),
                bypasses_player_limit: None,
            }]),
        },
    )
    .unwrap();
    let opped = world.tick(receiver.drain());
    let before = opped.snapshot.players[0].position;

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "teleport @s ~5 ~ ~-2".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());

    let update = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(*update),
            _ => None,
        })
        .expect("the teleport sends a position update");
    assert_eq!(update.position.x, 5.0);
    assert_eq!(update.position.y, 0.0);
    assert_eq!(update.position.z, -2.0);
    assert_eq!(
        update.relatives_mask,
        PLAYER_RELATIVE_X
            | PLAYER_RELATIVE_Y
            | PLAYER_RELATIVE_Z
            | PLAYER_RELATIVE_DELTA_X
            | PLAYER_RELATIVE_DELTA_Y
            | PLAYER_RELATIVE_DELTA_Z
            | PLAYER_RELATIVE_Y_ROT
            | PLAYER_RELATIVE_X_ROT
    );
    // The server-side position is the sum `Entity.teleportSetPosition`
    // (`Entity.java:3079-3089`) computes out of the same two halves.
    let after = tick.snapshot.players[0].position;
    assert_eq!(after.x, before.x + 5.0);
    assert_eq!(after.y, before.y);
    assert_eq!(after.z, before.z - 2.0);
}

/// `/gamemode` is the per-player game mode's trigger:
/// `GameModeCommand.setGameMode` (GameModeCommand.java:65-72) over
/// `ServerPlayer.setGameMode` (ServerPlayer.java:1734-1756), which is the
/// abilities, the tab-list entry and the `CHANGE_GAME_MODE` game event.
#[test]
fn a_gamemode_command_moves_one_session_and_announces_it_three_ways() {
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "op", AdminScope::Manage),
            request: AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_id(Uuid::from_u128(1)),
                permission_level: Some(CommandPermission::GameMaster),
                bypasses_player_limit: None,
            }]),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    // The fixture world's default game type is creative, so survival is the
    // move -- and it is the one the differential replay needs, because
    // `ServerPlayerGameMode.destroyBlock` asks `preventsBlockDrops`
    // (ServerPlayerGameMode.java:289-291) before it lets a broken block drop.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "gamemode survival".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());

    // `ClientboundGameEventPacket(CHANGE_GAME_MODE, mode.getId())`
    // (ServerPlayer.java:1740).
    let game_events = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::GameEvent(update),
            } => Some(*update),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        game_events
            .iter()
            .filter(|update| update.event_id
                == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID)
            .map(|update| update.param)
            .collect::<Vec<_>>(),
        [GameType::Survival.id() as f32]
    );
    // `onUpdateAbilities` runs twice (`ServerPlayerGameMode.java:66` and
    // `ServerPlayer.java:1753`), and survival is the mode that grants neither
    // flight nor instant build (GameType.java:72-77).
    let abilities = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerAbilities(update),
            } => Some(*update),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(abilities
        .iter()
        .all(|update| !update.instabuild && !update.can_fly));
    // `broadcastAll(UPDATE_GAME_MODE)` (ServerPlayerGameMode.java:67-70).
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
            ..
        } if update.actions == vec![PlayerInfoAction::UpdateGameMode]
    )));

    // Running it again reports nothing and sends nothing, which is
    // `changeGameModeForPlayer`'s early return (ServerPlayerGameMode.java:56-58).
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "gamemode survival".into(),
            },
        },
    )
    .unwrap();
    let again = world.tick(receiver.drain());
    assert!(
        !again.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::GameEvent(_) | ClientboundPlayPacket::SystemChat(_),
                ..
            }
        )),
        "{:?}",
        again.events
    );
}

/// `Entity.addTag` / `removeTag` (Entity.java:391-397) over the live session,
/// and the `Tags` field `saveWithoutId` writes for it (`:2017-2019`) -- which is
/// present exactly while the set is non-empty.
#[test]
fn tags_written_by_the_command_reach_the_save_and_leave_it_when_removed() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    assert!(
        saved_tags(&mut world).is_none(),
        "a fresh player writes no Tags"
    );

    chat_command(&sender, &receiver, &mut world, 1, "tag op add first");
    chat_command(&sender, &receiver, &mut world, 1, "tag op add second");
    assert_eq!(
        saved_tags(&mut world),
        Some(vec!["first".to_string(), "second".to_string()])
    );

    // `/tag list` reads the same set back. The flattened text carries the
    // component-valued arguments only -- the tag count is a boxed `int`, which
    // `TranslatableContents` writes as a primitive -- so what is visible here
    // is the entity and the `ComponentUtils.formatList` join
    // (TagCommand.java:115).
    let listed = chat_command(&sender, &receiver, &mut world, 1, "tag op list");
    assert_eq!(
        system_chats(&listed)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["commands.tag.list.single.success opfirst, second"]
    );

    chat_command(&sender, &receiver, &mut world, 1, "tag op remove first");
    assert_eq!(saved_tags(&mut world), Some(vec!["second".to_string()]));
    chat_command(&sender, &receiver, &mut world, 1, "tag op remove second");
    assert!(
        saved_tags(&mut world).is_none(),
        "an emptied set takes the field out of the document"
    );
}

/// `/experience` over a live session: `ServerPlayer.giveExperienceLevels`
/// (ServerPlayer.java:527-532) and `setExperiencePoints` (`:509-517`) drop
/// `lastSentExp`, which is what makes `doTick` (`:710-713`) send
/// `ClientboundSetExperiencePacket`, and the three fields reach the save
/// through `Player.addAdditionalSaveData` (Player.java:652-654).
#[test]
fn experience_commands_move_the_bar_announce_it_and_reach_the_save() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let levelled = chat_command(&sender, &receiver, &mut world, 1, "xp add op 5 levels");
    assert_eq!(
        set_experience_packets(&levelled),
        vec![PlayerExperience {
            progress: 0.0,
            level: 5,
            total: 0,
        }]
    );
    // Level 5 needs `7 + 5 * 2` points (Player.java:1558), so 3 points is a
    // bar at `3 / 17` (ServerPlayer.java:510-512) and the total is untouched.
    let pointed = chat_command(&sender, &receiver, &mut world, 1, "xp set op 3 points");
    assert_eq!(
        set_experience_packets(&pointed),
        vec![PlayerExperience {
            progress: 3.0 / 17.0,
            level: 5,
            total: 0,
        }]
    );
    // Setting the same points again moves nothing (`:513`), so nothing is
    // sent.
    let again = chat_command(&sender, &receiver, &mut world, 1, "xp set op 3 points");
    assert!(set_experience_packets(&again).is_empty());
    // `Type.POINTS.query` (ExperienceCommand.java:183) reads the bar back.
    let queried = chat_command(&sender, &receiver, &mut world, 1, "xp query op points");
    assert!(
        system_chats(&queried)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.starts_with("commands.experience.query.points")),
        "{:?}",
        system_chats(&queried)
    );
    // And an added point count rolls through `giveExperiencePoints`
    // (Player.java:1500-1521): 14 more points on a bar at 3/17 is level 6
    // (needs 19) holding nothing.
    let rolled = chat_command(&sender, &receiver, &mut world, 1, "experience add op 14");
    assert_eq!(
        set_experience_packets(&rolled),
        vec![PlayerExperience {
            progress: 0.0,
            level: 6,
            total: 14,
        }]
    );

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    let patch = request
        .player_patches
        .first()
        .expect("the joined session is patched");
    assert_eq!(patch.created.experience.level(), 6);
    assert_eq!(patch.created.experience.progress(), 0.0);
    assert_eq!(patch.created.experience.total(), 14);
}

/// `/enchant` over a live session: `ItemStack.enchant` (`ItemStack.java:995-997`)
/// writes `minecraft:enchantments` on the selected main-hand stack.
#[test]
fn enchant_writes_the_live_main_hand_enchantments_component() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let given = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op weapon.mainhand with diamond_sword",
    );
    assert!(
        system_chats(&given)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&given)
    );

    let enchanted = chat_command(&sender, &receiver, &mut world, 1, "enchant op sharpness");
    assert!(
        system_chats(&enchanted)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.enchant.success.single")),
        "{:?}",
        system_chats(&enchanted)
    );

    let snapshot = &world.publication().players[0].inventory;
    let selected = snapshot.slots[usize::from(snapshot.selected_slot)].clone();
    assert_eq!(
        selected.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID)
    );
    let sharpness = bbb_server_datapack::enchantment::builtin_enchantment_registry()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:sharpness")
        .expect("sharpness is in the built-in pack")
        .registry_id;
    assert_eq!(
        selected.component_patch.enchantments,
        [bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: sharpness,
            level: 1,
        }]
    );
}

/// `/effect` over a live session: `LivingEntity.addEffect`
/// (`EffectCommands.java:164-171`) files the instance on the session.
#[test]
fn effect_writes_the_live_session_active_effect() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let given = chat_command(&sender, &receiver, &mut world, 1, "effect give op speed");
    assert!(
        system_chats(&given)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.effect.give.success.single")),
        "{:?}",
        system_chats(&given)
    );

    let effect = world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:speed")
        .expect("effect filed on the session");
    // The command tick already ran `tickEffects` (`LivingEntity.java:828-836`).
    assert_eq!(effect.duration(), 599);
    assert_eq!(effect.amplifier(), 0);
    assert!(effect.show_particles());
}

/// `/clear` over a live session: `Inventory.clearOrCountMatchingItems`
/// (`ClearInventoryCommands.java:66`) with `i -> true` empties the fixture
/// stack and words `commands.clear.success.single` (`:86`).
#[test]
fn clear_empties_the_live_inventory() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "op", 1);

    let cleared = chat_command(&sender, &receiver, &mut world, 1, "clear");
    assert!(
        system_chats(&cleared)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.clear.success.single")),
        "{:?}",
        system_chats(&cleared)
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert!(
        slots
            .iter()
            .all(|slot| slot.count == 0 || slot.item_id.is_none()),
        "{slots:?}"
    );
}

/// `enablePack` hands `ReloadCommand.reloadPacks` the list it built
/// (`DataPackCommand.java:218`, `:227`), which `reloadResources` opens as-is
/// (`MinecraftServer.java:1505-1508`). The world carries that list to the
/// runtime rather than letting the reload re-discover.
#[test]
fn datapack_disable_carries_the_new_selection_to_the_runtime() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "datapack disable vanilla",
    );
    // `getChatLink(true)` (`:226`) names the pack that is going away.
    assert_eq!(
        system_chats(&tick),
        vec![(
            ConnectionId(1),
            "commands.datapack.modify.disable chat.square_brackets \
             pack.nameAndSource vanillapack.source.builtin"
                .to_string()
        )]
    );
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::ReloadRequested {
            selected: Some(ids)
        } if ids.is_empty()
    )));
}

/// `/time query gametime` over a live session:
/// `TimeCommand.queryGameTime` (`TimeCommand.java:131-135`) words
/// `commands.time.query.gametime` from `ServerLevel.getGameTime()`.
#[test]
fn time_query_gametime_reports_the_live_game_time() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let queried = chat_command(&sender, &receiver, &mut world, 1, "time query gametime");
    assert!(
        system_chats(&queried)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.time.query.gametime")),
        "{:?}",
        system_chats(&queried)
    );
}

/// `/spawnpoint` over a live session: `setRespawnPosition`
/// (`SetSpawnCommand.java:78`) stores a forced `RespawnConfig`.
#[test]
fn spawnpoint_sets_the_live_respawn_config() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let set = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "spawnpoint op 8 70 12 90 -10",
    );
    assert!(
        system_chats(&set)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.spawnpoint.success.single")),
        "{:?}",
        system_chats(&set)
    );
    let respawn = world
        .player_respawn_for_test(ConnectionId(1))
        .expect("spawnpoint wrote a live RespawnConfig");
    assert_eq!(
        respawn.dimension,
        Java26LevelKey::from(Java26BuiltinDimension::Overworld)
    );
    assert_eq!(respawn.pos.x, 8);
    assert_eq!(respawn.pos.y, 70);
    assert_eq!(respawn.pos.z, 12);
    assert_eq!(respawn.yaw, 90.0);
    assert_eq!(respawn.pitch, -10.0);
    assert!(respawn.forced);
}

/// Every `ClientboundSetExperiencePacket` a tick sent the first session.
fn set_experience_packets(tick: &TickResult) -> Vec<PlayerExperience> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetExperience(packet),
            } => Some(*packet),
            _ => None,
        })
        .collect()
}

/// `/playsound` over a live session: `PlaySoundCommand.playSound` draws one
/// seed off the level's random source (PlaySoundCommand.java:164) and sends a
/// `ClientboundSoundPacket` built on a direct holder (`:161`, `:187-188`).
#[test]
fn playsound_sends_the_packet_the_command_built_with_one_seed_per_run() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let [x, y, z] = ServerWorld::fixture_spawn_block();

    let played = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("playsound minecraft:entity.experience_orb.pickup record op {x} {y} {z} 0.5 1.5"),
    );
    let sounds = played
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Sound(sound),
            } => Some(sound.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    let sound = &sounds[0];
    assert_eq!(
        sound.sound,
        SoundEventHolder::Direct {
            location: "minecraft:entity.experience_orb.pickup".to_string(),
            fixed_range: None,
        }
    );
    assert_eq!(sound.source, SoundSource::Records);
    assert_eq!(
        sound.position,
        // `Vec3Argument.vec3()` centre-corrects a whole `x` and `z`
        // (Vec3Argument.java:30-32).
        Vec3d {
            x: f64::from(x) + 0.5,
            y: f64::from(y),
            z: f64::from(z) + 0.5,
        }
    );
    assert_eq!(sound.volume, 0.5);
    assert_eq!(sound.pitch, 1.5);
    assert!(system_chats(&played)
        .iter()
        .any(|(connection, text)| *connection == ConnectionId(1)
            && text.starts_with("commands.playsound.success.single")));
}

/// `/particle` over a live session: `ParticleCommand.sendParticles`
/// (`ParticleCommand.java:154-157`) sends one `ClientboundLevelParticlesPacket`.
#[test]
fn particle_sends_the_packet_the_command_built() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let [x, y, z] = ServerWorld::fixture_spawn_block();

    let played = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("particle heart {x} {y} {z} 0 0 0 0 0"),
    );
    let particles = played
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::LevelParticles(packet),
            } => Some(packet.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(particles.len(), 1, "{particles:?}");
    let packet = &particles[0];
    assert_eq!(
        packet.particle.particle_type_id,
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_HEART_ID
    );
    assert!(packet.particle.raw_options.is_empty());
    assert!(!packet.override_limiter);
    assert_eq!(
        packet.position,
        Vec3d {
            x: f64::from(x) + 0.5,
            y: f64::from(y),
            z: f64::from(z) + 0.5,
        }
    );
    assert!(system_chats(&played)
        .iter()
        .any(|(connection, text)| *connection == ConnectionId(1)
            && text.starts_with("commands.particle.success")));
}

/// `CommandSourceStack.broadcastToAdmins` (CommandSourceStack.java:479-493)
/// over live sessions: a `sendSuccess(.., true)` line is copied as
/// `chat.type.admin` to every other operator online (`:482-488`) and to the
/// console (`:490-492`), each half behind its own game rule.
#[test]
fn a_broadcast_success_line_reaches_the_other_operators_and_the_console() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1), ("peer", 2)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "peer", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "bystander", 3);

    // `TagCommand.java:72` passes `true`: the source hears its own line, the
    // other operator hears the copy, the bystander hears nothing and the
    // console logs it (`LOG_ADMIN_COMMANDS` starts true, GameRules.java:50).
    let added = chat_command(&sender, &receiver, &mut world, 1, "tag op add first");
    assert_eq!(
        admin_copies(&added),
        vec![ConnectionId(2)],
        "{:?}",
        system_chats(&added)
    );
    assert_eq!(console_admin_copies(&added), 1);
    assert!(system_chats(&added)
        .iter()
        .any(|(connection, text)| *connection == ConnectionId(1)
            && text.starts_with("commands.tag.add.success.single")));
    assert!(system_chats(&added)
        .iter()
        .all(|(connection, _)| *connection != ConnectionId(3)));

    // `ListPlayersCommand.java:35` passes `false`: no copy anywhere.
    let listed = chat_command(&sender, &receiver, &mut world, 1, "list");
    assert!(admin_copies(&listed).is_empty());
    assert_eq!(console_admin_copies(&listed), 0);

    // `/gamerule send_command_feedback false` writes the rule before it
    // reports (`GameRuleCommand.java:35-41`), so its own line and the
    // operators' copies are both already silenced (`:465`, `:482`) while the
    // console copy still goes out (`:490`).
    let silenced = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule send_command_feedback false",
    );
    assert!(
        system_chats(&silenced).is_empty(),
        "{:?}",
        system_chats(&silenced)
    );
    assert_eq!(console_admin_copies(&silenced), 1);
    let quiet = chat_command(&sender, &receiver, &mut world, 1, "tag op add second");
    assert!(
        system_chats(&quiet).is_empty(),
        "{:?}",
        system_chats(&quiet)
    );
    assert_eq!(console_admin_copies(&quiet), 1);

    // And the console half has its own rule.
    let unlogged = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule log_admin_commands false",
    );
    assert_eq!(console_admin_copies(&unlogged), 0);
    let dark = chat_command(&sender, &receiver, &mut world, 1, "tag op add third");
    assert!(system_chats(&dark).is_empty());
    assert_eq!(console_admin_copies(&dark), 0);
}

/// `sendSuccess` renders the supplier once and hands that one component to
/// both halves (CommandSourceStack.java:467-476), and `broadcastToAdmins`
/// wraps it once for every recipient (`:480`), so the line the source read and
/// the message inside every copy are one value rather than three renderings.
#[test]
fn one_rendering_feeds_the_sources_line_every_operator_copy_and_the_console() {
    let mut world =
        ServerWorld::new(operator_config(&[("op", 1), ("peer", 2), ("other", 4)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "peer", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "bystander", 3);
    join_loaded(&sender, &receiver, &mut world, 4, "other", 4);

    let added = chat_command(&sender, &receiver, &mut world, 1, "tag op add first");
    // `:484`: the invoking source is the only operator skipped, so a third one
    // is copied to as well.
    assert_eq!(
        admin_copies(&added),
        vec![ConnectionId(2), ConnectionId(4)],
        "{:?}",
        system_chats(&added)
    );

    let own = added
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => Some(chat.content.component().clone()),
            _ => None,
        })
        .expect("the source reads its own line");

    let copies = added
        .events
        .iter()
        .filter_map(|event| {
            let component = match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SystemChat(chat),
                    ..
                } => chat.content.component(),
                WorldEvent::ConsoleMessage(message) => message.component(),
                _ => return None,
            };
            is_admin_copy(component).then(|| component.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(copies.len(), 3, "two operator copies and the console one");
    assert!(copies.iter().all(|copy| *copy == copies[0]));

    let NbtSummaryValue::Compound(entries) = &copies[0] else {
        panic!("a component is a compound");
    };
    let NbtSummaryValue::List(arguments) = &entries
        .iter()
        .find(|entry| entry.name == "with")
        .expect("the admin translation carries arguments")
        .value
    else {
        panic!("admin arguments are a list");
    };
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[1], own);
}

/// The sessions a tick sent a `chat.type.admin` system chat to, in event order.
fn admin_copies(tick: &TickResult) -> Vec<ConnectionId> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SystemChat(chat),
            } if is_admin_copy(chat.content.component()) => Some(*connection_id),
            _ => None,
        })
        .collect()
}

/// How many `chat.type.admin` lines a tick handed the console.
fn console_admin_copies(tick: &TickResult) -> usize {
    tick.events
        .iter()
        .filter(|event| match event {
            WorldEvent::ConsoleMessage(message) => is_admin_copy(message.component()),
            _ => false,
        })
        .count()
}

/// `broadcastToAdmins`'s component (`CommandSourceStack.java:480`): the
/// `chat.type.admin` translatable, gray and italic.
fn is_admin_copy(component: &NbtSummaryValue) -> bool {
    let NbtSummaryValue::Compound(entries) = component else {
        return false;
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    if field("translate") != Some(&NbtSummaryValue::string("chat.type.admin")) {
        return false;
    }
    assert_eq!(field("color"), Some(&NbtSummaryValue::string("gray")));
    assert_eq!(field("italic"), Some(&NbtSummaryValue::Byte(1)));
    true
}

/// The `Tags` field of the one player patch a save request carries.
fn saved_tags(world: &mut ServerWorld) -> Option<Vec<String>> {
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    let patch = request
        .player_patches
        .first()
        .expect("the joined session is patched");
    (!patch.tags.is_empty()).then(|| {
        patch
            .tags
            .iter()
            .map(|tag| tag.try_to_string().unwrap())
            .collect()
    })
}

/// [`management_command`] with a request id of its own.
///
/// `AdminState.apply` replays a mutation whose request id it has already
/// recorded rather than running it again, and the two runs of one `/setblock`
/// these tests make are deliberately two requests rather than a retry of one.
fn block_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation_id: u64,
    command: &str,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(
                operation_id,
                &format!("{operation_id}:{command}"),
                AdminScope::Command,
            ),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

/// The world `/setblock` and `/fill` are exercised against: one restored save
/// whose only column is forceloaded, which is what makes every position these
/// tests name one `getLoadedBlockPos` accepts.
fn block_command_world() -> ServerWorld {
    let candidate = restore_candidate(10, None, Vec::new());
    ServerWorld::restore(creative_fixture_config(), &candidate).unwrap()
}

/// [`block_command_world`] plus a `TicketType.FORCED` ticket over nether chunk
/// (0, 0), so `getLoadedBlockPos` after `/execute in minecraft:the_nether`
/// accepts the scaled console spawn.
fn execute_in_command_world() -> ServerWorld {
    let mut candidate = restore_candidate(10, None, Vec::new());
    candidate.chunk_tickets.insert(
        Java26LevelKey::from(Java26BuiltinDimension::Nether),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
    ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            ..creative_fixture_config()
        },
        &candidate,
    )
    .unwrap()
}

/// The command report `command.execute` answered with: the numeric result and
/// one `key(arg, arg)` line per message.
///
/// The arguments are read off the component rather than off its text summary,
/// because the numbers `/setblock` and `/fill` report travel as NBT ints and
/// the summary only carries the display runs.
fn management_report(tick: &TickResult) -> (Option<i32>, Vec<String>) {
    let Ok(AdminResult::Command(report)) = &admin_outcome(tick).result else {
        panic!("a command.execute answers with a command report");
    };
    (
        report.result,
        report
            .messages
            .iter()
            .map(|message| translatable_line(message.component()))
            .collect(),
    )
}

/// One component as `key(arg, arg)`, unwrapping the red root `sendFailure`
/// puts around a refusal (`CommandSourceStack.java:495-499`).
fn translatable_line(value: &NbtSummaryValue) -> String {
    // An unstyled literal collapses to a bare string in the codec, which is
    // what `Component.translationArg` and `translatableEscape`'s stringified
    // arguments both arrive as.
    if let NbtSummaryValue::String(text) = value {
        return text.try_to_string().expect("fixture text is valid Unicode");
    }
    let NbtSummaryValue::Compound(entries) = value else {
        return format!("{value:?}");
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    if let Some(NbtSummaryValue::List(children)) = field("extra") {
        if field("translate").is_none() && children.len() == 1 {
            return translatable_line(&children[0]);
        }
    }
    let Some(NbtSummaryValue::String(key)) = field("translate") else {
        return format!("{value:?}");
    };
    let arguments = match field("with") {
        Some(NbtSummaryValue::List(arguments)) => arguments
            .iter()
            .map(|argument| match argument {
                NbtSummaryValue::Int(value) => value.to_string(),
                // `commands.fill.toobig` takes the area as a `long`, which is
                // the type the multiplication in `fillBlocks` produced.
                NbtSummaryValue::Long(value) => value.to_string(),
                other => translatable_line(other),
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    if arguments.is_empty() {
        return key.try_to_string().expect("fixture key is valid Unicode");
    }
    format!("{key}({})", arguments.join(", "))
}

/// `SetBlockCommand.setBlock` (SetBlockCommand.java:99-135) end to end: the
/// block really moves and the report names the position.
#[test]
fn setblock_writes_the_block_and_names_the_position_it_wrote() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 3, y: 64, z: 5 };

    let tick = block_command(&sender, &receiver, &mut world, 1, "setblock 3 64 5 stone");
    assert_eq!(
        management_report(&tick),
        (
            Some(1),
            vec!["commands.setblock.success(3, 64, 5)".to_string()]
        ),
    );
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:stone"))
    );

    // `place` answers what `Level.setBlock` answered (BlockInput.java:74-76),
    // and a write that changed nothing is `ERROR_FAILED`
    // (SetBlockCommand.java:125-127).
    let again = block_command(&sender, &receiver, &mut world, 2, "setblock 3 64 5 stone");
    let (result, messages) = management_report(&again);
    assert_eq!(result, None, "a refused command returns no value");
    assert_eq!(messages, vec!["commands.setblock.failed".to_string()]);
}

/// `ExecuteCommand.java:255-260` through `CommandSourceStack.withLevel`
/// (`CommandSourceStack.java:307-329`): the console stands at the overworld
/// spawn corner, `/execute in minecraft:the_nether` retargets the inner
/// `/setblock`, and the 8:1 scale of that switch is a no-op on X/Z because
/// that corner is the origin.
#[test]
fn execute_in_the_nether_setblock_writes_the_nether_and_not_the_overworld() {
    let (sender, receiver) = mailbox();
    let mut world = execute_in_command_world();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let overworld_before = world.block_state(Java26BuiltinDimension::Overworld, pos);

    let tick = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run setblock ~ ~ ~ minecraft:stone",
    );
    assert_eq!(
        management_report(&tick),
        (
            Some(1),
            vec!["commands.setblock.success(0, 64, 0)".to_string()]
        ),
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Nether, pos),
        Some(block_state_id("minecraft:stone"))
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, pos),
        overworld_before,
        "the overworld cell the source started on is not the write"
    );
}

/// `ServerChunkCache.hasChunk` exposes only holders at FULL status
/// (`ServerChunkCache.java:262-265`), so `getLoadedBlockPos` rejects a proto
/// dependency column with `argument.pos.unloaded` (`BlockPosArgument.java:37-44`).
#[test]
fn execute_in_the_nether_writes_the_full_ticket_ring_but_not_its_generation_halo() {
    let (sender, receiver) = mailbox();
    let mut world = execute_in_command_world();
    let full_ring = BlockPos {
        x: 2 << 4,
        y: 64,
        z: 0,
    };
    let dependency_halo = BlockPos {
        x: 3 << 4,
        y: 64,
        z: 0,
    };

    let full = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run setblock 32 64 0 minecraft:diamond_block",
    );
    assert_eq!(
        management_report(&full),
        (
            Some(1),
            vec!["commands.setblock.success(32, 64, 0)".to_string()]
        ),
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Nether, full_ring),
        Some(block_state_id("minecraft:diamond_block"))
    );

    let halo_chunk = (dependency_halo.x >> 4, dependency_halo.z >> 4);
    let halo_status = |world: &ServerWorld| {
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .generation_status(halo_chunk.0, halo_chunk.1)
    };
    let halo_before = halo_status(&world);
    assert!(
        halo_before.is_some_and(|status| status < bbb_server_persistence::Java26ChunkStatus::Full),
        "the FULL ring's generation dependency remains materialized as proto data"
    );
    let halo = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "execute in minecraft:the_nether run setblock 48 64 0 minecraft:diamond_block",
    );
    assert_eq!(
        management_report(&halo),
        (None, vec!["argument.pos.unloaded".to_string()])
    );
    assert_eq!(
        halo_status(&world),
        halo_before,
        "the generation-only dependency column is not writable"
    );
}

/// The three literals that are not `replace`: `keep` refuses a cell that is not
/// empty, `destroy` clears it first, and `strict` skips the neighbour pass.
#[test]
fn the_setblock_literals_each_change_what_the_write_does() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 4, y: 64, z: 5 };
    block_command(&sender, &receiver, &mut world, 1, "setblock 4 64 5 stone");

    // `filter` (SetBlockCommand.java:24) is `isEmptyBlock`, checked at `:112`
    // before anything is written.
    let kept = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "setblock 4 64 5 minecraft:dirt keep",
    );
    assert_eq!(
        management_report(&kept),
        (None, vec!["commands.setblock.failed".to_string()]),
    );
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:stone"))
    );

    // `destroy` (`:117-119`) removes what stood there before it places, which
    // is the whole difference from `replace`: the level event and the drop are
    // `Level.destroyBlock`'s (Level.java:276-296).
    let destroyed = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "setblock 4 64 5 minecraft:dirt destroy",
    );
    assert_eq!(management_report(&destroyed).0, Some(1));
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:dirt"))
    );

    // `replace` and the bare form are the same call
    // (SetBlockCommand.java:37, `:75`).
    let replaced = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "setblock 4 64 5 minecraft:stone replace",
    );
    assert_eq!(management_report(&replaced).0, Some(1));
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:stone"))
    );
}

/// `getLoadedBlockPos`'s two gates (BlockPosArgument.java:37-46), each with its
/// own key.
#[test]
fn setblock_refuses_a_position_outside_the_loaded_world() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();

    // Column (100, 100) is not loaded: `hasChunkAt` is false there and vanilla
    // refuses rather than generating it.
    let unloaded = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 1600 64 1600 stone",
    );
    let (result, messages) = management_report(&unloaded);
    assert_eq!(result, None);
    assert_eq!(messages[0], "argument.pos.unloaded");

    // A loaded column with a y above the build height reaches the second gate,
    // which is why the two refusals are not one.
    let outside = block_command(&sender, &receiver, &mut world, 2, "setblock 3 5000 5 stone");
    let (result, messages) = management_report(&outside);
    assert_eq!(result, None);
    assert_eq!(messages[0], "argument.pos.outofworld");
}

/// The `strict` literal is the `2 | 816` of SetBlockCommand.java:125 plus the
/// skipped `updateNeighboursOnBlockSet` of `:129-131`, and a redstone lamp is
/// what tells the two apart.
///
/// `RedstoneLampBlock.neighborChanged` (RedstoneLampBlock.java:44-46) lights an
/// unlit lamp on the spot when it finds a signal, and the only thing that tells
/// the lamp about the block that just arrived beside it is that neighbour pass:
/// the write itself carries no `UPDATE_NEIGHBORS` bit either way.
#[test]
fn a_strict_setblock_leaves_the_neighbours_untold() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let lamp = BlockPos { x: 1, y: 64, z: 1 };
    let power = BlockPos { x: 2, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lamp,
        redstone_lamp_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));

    let strict = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 2 64 1 minecraft:redstone_block strict",
    );
    assert_eq!(management_report(&strict).0, Some(1));
    assert_eq!(
        live_block(&mut world, power),
        Some(block_state_id("minecraft:redstone_block"))
    );
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false)),
        "a strict write runs no neighbour pass, so the lamp never heard about it"
    );

    // The same write without the literal reaches SetBlockCommand.java:130, and
    // the lamp lights
    // inside the command rather than a tick later.
    block_command(&sender, &receiver, &mut world, 2, "setblock 2 64 1 air");
    let plain = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "setblock 2 64 1 minecraft:redstone_block",
    );
    assert_eq!(management_report(&plain).0, Some(1));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );
}

/// The block-state argument writes the properties it names, and
/// `BlockInput.place`'s neighbour-shape read (BlockInput.java:67) is what
/// decides the rest of them.
#[test]
fn setblock_places_the_properties_the_argument_wrote_out() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let log = BlockPos { x: 6, y: 64, z: 6 };
    let leaf = BlockPos { x: 7, y: 64, z: 5 };
    let ground = BlockPos { x: 7, y: 64, z: 6 };

    block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 6 64 6 minecraft:oak_log[axis=x]",
    );
    assert_eq!(
        live_block(&mut world, log).map(block_state_property_pair),
        Some(("minecraft:oak_log".to_string(), Some("x".to_string()))),
    );

    // `grass_block` is a block whose `updateShape` really answers a different
    // state: `SnowyBlock.updateShape` (SnowyBlock.java:32-45) reads the block
    // above and rewrites `SNOWY` from it. With snow standing over the cell, a
    // write that names no property lands snowy whatever was typed.
    block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "setblock 7 65 6 minecraft:snow_block",
    );
    block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "setblock 7 64 6 minecraft:grass_block",
    );
    assert_eq!(
        snowy(&mut world, ground),
        Some("true".to_string()),
        "the neighbour-shape read is what set it, not the argument"
    );

    // Naming it is what `overwriteWithDefinedProperties`
    // (BlockInput.java:103-113) exists for: the shape update runs first and is
    // then overwritten at every property the argument spelled out, so the
    // typed `snowy=false` is what lands -- and the write counts as a change
    // rather than reporting `ERROR_FAILED` for having written what was already
    // there.
    let named = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "setblock 7 64 6 minecraft:grass_block[snowy=false]",
    );
    assert_eq!(management_report(&named).0, Some(1));
    assert_eq!(snowy(&mut world, ground), Some("false".to_string()));

    // The leaf beside the log is the other half of the same rule read from the
    // scheduled-tick side: `LeavesBlock.updateShape` (LeavesBlock.java:89-109)
    // answers the state it was given and only arms a tick, and
    // `LeavesBlock.tick` (`:79-81`) is what writes the distance the log
    // implies. `persistent` is the argument's and survives either way.
    block_command(
        &sender,
        &receiver,
        &mut world,
        5,
        "setblock 7 64 5 minecraft:oak_leaves[persistent=true]",
    );
    let state = live_block(&mut world, leaf).expect("the leaf landed");
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .expect("a generated state");
    assert_eq!(identity.name, "minecraft:oak_leaves");
    assert_eq!(
        identity.properties.get("persistent").map(String::as_str),
        Some("true")
    );
}

/// The `snowy` property of a live cell, for the assertions that read what a
/// neighbour-shape update left.
fn snowy(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    let state = live_block(world, pos)?;
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("snowy")
        .cloned()
}

/// The block name and the one property this crate names, for an assertion that
/// says which state landed.
fn block_state_property_pair(state: i32) -> (String, Option<String>) {
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .expect("a generated state");
    (
        identity.name.clone(),
        identity.properties.get("axis").cloned(),
    )
}

/// The states in a box, read as block names so an assertion says which blocks
/// landed rather than which ids did.
fn box_block_names(
    world: &mut ServerWorld,
    min: BlockPos,
    max: BlockPos,
) -> Vec<(BlockPos, String)> {
    let mut names = Vec::new();
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let pos = BlockPos { x, y, z };
                let state = live_block(world, pos).expect("a loaded cell");
                names.push((
                    pos,
                    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                        .expect("a generated state")
                        .name
                        .clone(),
                ));
            }
        }
    }
    names
}

/// How many cells in a box hold `name`.
fn box_count(world: &mut ServerWorld, min: BlockPos, max: BlockPos, name: &str) -> usize {
    box_block_names(world, min, max)
        .into_iter()
        .filter(|(_, block)| block == name)
        .count()
}

/// `FillCommand.fillBlocks` (FillCommand.java:143-206) in its plain mode: every
/// cell of the box, and a count of the ones the write changed.
#[test]
fn fill_writes_the_whole_box_and_counts_what_changed() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let min = BlockPos { x: 0, y: 64, z: 0 };
    let max = BlockPos { x: 2, y: 64, z: 2 };

    let tick = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fill 0 64 0 2 64 2 stone",
    );
    assert_eq!(
        management_report(&tick),
        (Some(9), vec!["commands.fill.success(9)".to_string()]),
    );
    assert_eq!(box_count(&mut world, min, max, "minecraft:stone"), 9);

    // `:181`: a write that changed nothing is not counted, and `:199-201`
    // refuses a fill that changed nothing at all.
    let again = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "fill 0 64 0 2 64 2 stone",
    );
    assert_eq!(
        management_report(&again),
        (None, vec!["commands.fill.failed".to_string()]),
    );

    // One cell put back is one cell counted, which is the difference between
    // "how many cells the box has" and "how many the write moved".
    block_command(&sender, &receiver, &mut world, 3, "setblock 1 64 1 air");
    let partial = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "fill 0 64 0 2 64 2 stone",
    );
    assert_eq!(management_report(&partial).0, Some(1));
}

/// `CloneCommands.java:286` copies the source box's scheduled ticks.
#[test]
fn clone_copies_scheduled_block_ticks() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let source = BlockPos { x: 0, y: 64, z: 0 };
    let dest = BlockPos { x: 8, y: 64, z: 0 };
    block_command(&sender, &receiver, &mut world, 1, "setblock 0 64 0 stone");
    world.test_schedule_block_tick(
        Java26BuiltinDimension::Overworld,
        source,
        block_state_id("minecraft:stone"),
        100,
    );
    block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "clone 0 64 0 0 64 0 8 64 0",
    );
    let positions = world.test_block_tick_positions(Java26BuiltinDimension::Overworld);
    assert!(
        positions.contains(&dest),
        "copied tick positions: {positions:?}"
    );
}

/// `FillCommand.Mode` (FillCommand.java:222-246): `outline` leaves the inside
/// alone and `hollow` clears it, and both use the same interior test.
#[test]
fn the_two_shell_fill_modes_read_the_same_interior_test_two_ways() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let min = BlockPos { x: 4, y: 64, z: 4 };
    let max = BlockPos { x: 6, y: 66, z: 6 };

    block_command(&sender, &receiver, &mut world, 1, "fill 4 64 4 6 66 6 dirt");
    // The one interior cell of a 3x3x3 is the only one `OUTLINE`'s filter
    // answers null for (`:226-231`), and a null block is not written and not
    // counted (`:177-180`).
    let outline = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "fill 4 64 4 6 66 6 minecraft:stone outline",
    );
    assert_eq!(management_report(&outline).0, Some(26));
    assert_eq!(
        live_block(&mut world, BlockPos { x: 5, y: 65, z: 5 }),
        Some(block_state_id("minecraft:dirt")),
        "outline leaves the inside as it found it"
    );
    assert_eq!(box_count(&mut world, min, max, "minecraft:stone"), 26);

    // `HOLLOW` writes `HOLLOW_CORE` there instead (`:34`, `:237-242`), so the
    // inside becomes air and is counted with the rest.
    block_command(&sender, &receiver, &mut world, 3, "fill 4 64 4 6 66 6 dirt");
    let hollow = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "fill 4 64 4 6 66 6 minecraft:stone hollow",
    );
    assert_eq!(management_report(&hollow).0, Some(27));
    assert_eq!(
        live_block(&mut world, BlockPos { x: 5, y: 65, z: 5 }),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(box_count(&mut world, min, max, "minecraft:stone"), 26);
}

/// The two predicates that stand in front of the loop: `keep`'s `isEmptyBlock`
/// (FillCommand.java:85) and the `<filter>` argument `replace` takes (`:73`).
#[test]
fn the_fill_predicates_decide_which_cells_the_loop_admits() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let min = BlockPos { x: 8, y: 64, z: 8 };
    let max = BlockPos {
        x: 10,
        y: 64,
        z: 10,
    };
    block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fill 8 64 8 10 64 10 dirt",
    );
    block_command(&sender, &receiver, &mut world, 2, "setblock 9 64 9 air");

    // `keep` touches only the empty cell.
    let kept = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "fill 8 64 8 10 64 10 minecraft:stone keep",
    );
    assert_eq!(management_report(&kept).0, Some(1));
    assert_eq!(box_count(&mut world, min, max, "minecraft:dirt"), 8);
    assert_eq!(box_count(&mut world, min, max, "minecraft:stone"), 1);

    // A `<filter>` naming a block touches only the cells holding it.
    let filtered = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "fill 8 64 8 10 64 10 minecraft:cobblestone replace minecraft:dirt",
    );
    assert_eq!(management_report(&filtered).0, Some(8));
    assert_eq!(box_count(&mut world, min, max, "minecraft:cobblestone"), 8);

    // A `#tag` filter resolves through the block-tag registry, which is the
    // half a `minecraft:block_state` argument refuses outright.
    let tagged = block_command(
        &sender,
        &receiver,
        &mut world,
        5,
        "fill 8 64 8 10 64 10 minecraft:dirt replace #minecraft:stone_bricks",
    );
    assert_eq!(
        management_report(&tagged),
        (None, vec!["commands.fill.failed".to_string()]),
        "no cell holds a stone brick, so the tag admits none of them"
    );
    let stone_tag = block_command(
        &sender,
        &receiver,
        &mut world,
        6,
        "fill 8 64 8 10 64 10 minecraft:dirt replace #minecraft:base_stone_overworld",
    );
    assert_eq!(
        management_report(&stone_tag).0,
        Some(1),
        "the one stone cell is in `#base_stone_overworld` and the cobblestone are not"
    );
}

/// `ERROR_AREA_TOO_LARGE` (FillCommand.java:31-33), whose bound is the
/// `max_block_modifications` game rule (`:152`) rather than the retired
/// `maxCommandChainLength` reading of it.
#[test]
fn fill_refuses_a_box_larger_than_the_game_rule_admits() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();

    block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule max_block_modifications 4",
    );
    let refused = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "fill 0 64 0 2 64 2 stone",
    );
    assert_eq!(
        management_report(&refused),
        (None, vec!["commands.fill.toobig(4, 9)".to_string()]),
    );
    assert_eq!(
        live_block(&mut world, BlockPos { x: 0, y: 64, z: 0 }),
        Some(block_state_id("minecraft:air")),
        "the refusal happens before the loop, so nothing was written"
    );

    // The bound is inclusive: an area equal to it is filled, and the very next
    // one up is not -- the comparison is `>` rather than `>=` and neither side
    // of it is a rounding.
    let allowed = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "fill 0 64 0 1 64 1 stone",
    );
    assert_eq!(management_report(&allowed).0, Some(4));
    let one_over = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "fill 0 70 0 0 74 0 stone",
    );
    assert_eq!(
        management_report(&one_over),
        (None, vec!["commands.fill.toobig(4, 5)".to_string()]),
    );
}

/// `/fill`'s neighbour pass covers the positions whose own write answered true
/// (FillCommand.java:186-188) and no others, which is what separates it from
/// `/setblock`'s pass over its one position whatever happened
/// (SetBlockCommand.java:129-131).
///
/// The witness is a lamp that is unlit while a signal is already beside it: a
/// `strict` write put the power there without telling anyone, so any neighbour
/// pass that reaches the lamp lights it on the spot
/// (`RedstoneLampBlock.neighborChanged`, RedstoneLampBlock.java:44-46). The
/// `/fill` below writes air over air, so its write answers false and the pass
/// must not run -- a pass over every position instead would light the lamp
/// through a cell nothing happened in.
#[test]
fn a_fill_that_wrote_nothing_tells_no_neighbour() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let lamp = BlockPos { x: 5, y: 64, z: 9 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lamp,
        redstone_lamp_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    let strict = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 6 64 9 minecraft:redstone_block strict",
    );
    assert_eq!(management_report(&strict).0, Some(1));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false))
    );

    // The other cell beside the lamp is already air, so this writes nothing at
    // all -- and `count == 0` is the refusal FillCommand.java:199-201 raises
    // after the pass it did not run.
    let empty = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "fill 5 64 10 5 64 10 air",
    );
    assert_eq!(
        management_report(&empty),
        (None, vec!["commands.fill.failed".to_string()]),
    );
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false)),
        "no position changed, so no position was passed to the neighbour update"
    );

    // One that does change reaches the same lamp through the same pass.
    let wrote = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "fill 5 64 10 5 64 10 minecraft:stone",
    );
    assert_eq!(management_report(&wrote).0, Some(1));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );
}

/// `destroy` removes what stood there before the write, and the write itself is
/// what `placeNeeded` (SetBlockCommand.java:116-122) decides.
///
/// Placing air over a solid block is the one case where the two come apart: the
/// destroy is the whole of the change, `placeNeeded` is false because both the
/// argument and the destroyed cell are air, and `BlockInput.place` is never
/// called. A server that skipped the destroy would leave the block standing and
/// still report the success.
#[test]
fn a_destroying_setblock_of_air_is_the_destroy_alone() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 7, y: 62, z: 7 };
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:stone")),
        "the fixture column stands to y=63"
    );

    let destroyed = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 7 62 7 minecraft:air destroy",
    );
    assert_eq!(
        management_report(&destroyed),
        (
            Some(1),
            vec!["commands.setblock.success(7, 62, 7)".to_string()]
        ),
    );
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:air"))
    );

    // Running it again reports the same success, and that is vanilla's answer
    // rather than a shortcut: `ERROR_FAILED` is guarded by `placeNeeded`
    // (SetBlockCommand.java:125), which is false for air over air, so the
    // `place` that would have answered false is never called at all. Only the
    // modes that always place can report the failure.
    let again = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "setblock 7 62 7 minecraft:air destroy",
    );
    assert_eq!(
        management_report(&again),
        (
            Some(1),
            vec!["commands.setblock.success(7, 62, 7)".to_string()]
        ),
    );
    let replace = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "setblock 7 62 7 minecraft:air",
    );
    assert_eq!(
        management_report(&replace),
        (None, vec!["commands.setblock.failed".to_string()]),
    );
}

/// `strict` on a `/fill` is the same two things it is on a `/setblock`: the
/// `UPDATE_SKIP_ALL_SIDEEFFECTS` write and the skipped neighbour pass.
#[test]
fn a_strict_fill_leaves_the_neighbours_untold() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let lamp = BlockPos {
        x: 12,
        y: 64,
        z: 12,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lamp,
        redstone_lamp_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));

    let strict = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fill 13 64 12 13 64 12 minecraft:redstone_block strict",
    );
    assert_eq!(management_report(&strict).0, Some(1));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false))
    );

    block_command(&sender, &receiver, &mut world, 2, "setblock 13 64 12 air");
    let plain = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "fill 13 64 12 13 64 12 minecraft:redstone_block",
    );
    assert_eq!(management_report(&plain).0, Some(1));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );
}

/// Every console line `scripts/server_world_oracle.py` types at the server that
/// authors its shared input save (`setup_script`, `PREAMBLE` and each
/// scenario's `setup`).
///
/// Four kinds of command make the whole script and this dispatcher now runs
/// every one of them, so the four columns the `forceload add` lines force are
/// the command's own tickets rather than tickets [`world_oracle_world`] put in
/// the save.
const WORLD_ORACLE_SETUP: &[&str] = &[
    "tick freeze",
    "gamerule random_tick_speed 1024",
    "gamerule block_drops false",
    "gamerule spawn_mobs false",
    "gamerule advance_weather false",
    "gamerule advance_time false",
    "forceload add 0 0",
    "forceload add 16 0",
    "forceload add 32 0",
    "forceload add 48 0",
    "fill 0 -61 0 15 -61 15 minecraft:dirt",
    "fill 16 -61 0 31 -61 15 minecraft:dirt",
    "fill 32 -61 0 47 -61 15 minecraft:dirt",
    "fill 48 -61 0 63 -61 15 minecraft:dirt",
    "fill 0 -60 3 15 -60 5 minecraft:stone",
    "fill 0 -59 3 15 -59 5 minecraft:stone",
    "fill 1 -59 4 14 -59 4 minecraft:air",
    "setblock 1 -59 4 minecraft:water",
    "fill 4 -50 10 4 -47 10 minecraft:sand",
    "setblock 18 -55 2 minecraft:oak_log",
    "setblock 19 -55 2 minecraft:oak_leaves[distance=1,persistent=true]",
    "setblock 20 -55 2 minecraft:oak_leaves[distance=2,persistent=true]",
    "setblock 21 -55 2 minecraft:oak_leaves[distance=3,persistent=true]",
    "setblock 22 -55 2 minecraft:oak_leaves[distance=4,persistent=true]",
    "setblock 23 -55 2 minecraft:oak_leaves[distance=5,persistent=true]",
    "setblock 24 -55 2 minecraft:oak_leaves[distance=6,persistent=true]",
    "setblock 18 -55 2 minecraft:air",
    "fill 19 -50 9 21 -50 11 minecraft:oak_leaves",
    "setblock 38 -55 8 minecraft:sand",
    "setblock 34 300 8 minecraft:sand",
    "setblock 48 -60 4 minecraft:redstone_lamp",
    "setblock 49 -60 4 minecraft:redstone_block",
    "setblock 49 -60 4 minecraft:air",
    "setblock 48 -60 6 minecraft:redstone_lamp",
    "setblock 49 -60 6 minecraft:redstone_block",
    "setblock 49 -60 6 minecraft:air",
    "setblock 49 -60 6 minecraft:redstone_block",
    "setblock 51 -60 6 minecraft:redstone_lamp",
    "setblock 52 -60 6 minecraft:redstone_block",
    "setblock 52 -60 6 minecraft:air",
    "setblock 54 -59 4 minecraft:stone",
    "setblock 54 -58 4 minecraft:redstone_torch",
    "setblock 54 -60 4 minecraft:redstone_torch",
];

type WorldOracleBox<'a> = (&'a str, [i32; 6], &'a [(i32, i32, i32, &'a str)]);

/// What the official 26.1 server had written inside each scenario box when it
/// stopped, read back out of the region files its `prepare_world` produced.
///
/// Only the cells that are not `minecraft:air` are listed; the box is complete,
/// so every position not named here is air on both sides. The property lists
/// are the palette's own, which is `StateDefinition`'s name order.
const WORLD_ORACLE_BOXES: &[WorldOracleBox<'_>] = &[
    (
        "fluid_trench",
        [0, -60, 3, 15, -58, 5],
        &[
            (0, -60, 3, "minecraft:stone"),
            (0, -60, 4, "minecraft:stone"),
            (0, -60, 5, "minecraft:stone"),
            (0, -59, 3, "minecraft:stone"),
            (0, -59, 4, "minecraft:stone"),
            (0, -59, 5, "minecraft:stone"),
            (1, -60, 3, "minecraft:stone"),
            (1, -60, 4, "minecraft:stone"),
            (1, -60, 5, "minecraft:stone"),
            (1, -59, 3, "minecraft:stone"),
            (1, -59, 4, "minecraft:water[level=0]"),
            (1, -59, 5, "minecraft:stone"),
            (2, -60, 3, "minecraft:stone"),
            (2, -60, 4, "minecraft:stone"),
            (2, -60, 5, "minecraft:stone"),
            (2, -59, 3, "minecraft:stone"),
            (2, -59, 5, "minecraft:stone"),
            (3, -60, 3, "minecraft:stone"),
            (3, -60, 4, "minecraft:stone"),
            (3, -60, 5, "minecraft:stone"),
            (3, -59, 3, "minecraft:stone"),
            (3, -59, 5, "minecraft:stone"),
            (4, -60, 3, "minecraft:stone"),
            (4, -60, 4, "minecraft:stone"),
            (4, -60, 5, "minecraft:stone"),
            (4, -59, 3, "minecraft:stone"),
            (4, -59, 5, "minecraft:stone"),
            (5, -60, 3, "minecraft:stone"),
            (5, -60, 4, "minecraft:stone"),
            (5, -60, 5, "minecraft:stone"),
            (5, -59, 3, "minecraft:stone"),
            (5, -59, 5, "minecraft:stone"),
            (6, -60, 3, "minecraft:stone"),
            (6, -60, 4, "minecraft:stone"),
            (6, -60, 5, "minecraft:stone"),
            (6, -59, 3, "minecraft:stone"),
            (6, -59, 5, "minecraft:stone"),
            (7, -60, 3, "minecraft:stone"),
            (7, -60, 4, "minecraft:stone"),
            (7, -60, 5, "minecraft:stone"),
            (7, -59, 3, "minecraft:stone"),
            (7, -59, 5, "minecraft:stone"),
            (8, -60, 3, "minecraft:stone"),
            (8, -60, 4, "minecraft:stone"),
            (8, -60, 5, "minecraft:stone"),
            (8, -59, 3, "minecraft:stone"),
            (8, -59, 5, "minecraft:stone"),
            (9, -60, 3, "minecraft:stone"),
            (9, -60, 4, "minecraft:stone"),
            (9, -60, 5, "minecraft:stone"),
            (9, -59, 3, "minecraft:stone"),
            (9, -59, 5, "minecraft:stone"),
            (10, -60, 3, "minecraft:stone"),
            (10, -60, 4, "minecraft:stone"),
            (10, -60, 5, "minecraft:stone"),
            (10, -59, 3, "minecraft:stone"),
            (10, -59, 5, "minecraft:stone"),
            (11, -60, 3, "minecraft:stone"),
            (11, -60, 4, "minecraft:stone"),
            (11, -60, 5, "minecraft:stone"),
            (11, -59, 3, "minecraft:stone"),
            (11, -59, 5, "minecraft:stone"),
            (12, -60, 3, "minecraft:stone"),
            (12, -60, 4, "minecraft:stone"),
            (12, -60, 5, "minecraft:stone"),
            (12, -59, 3, "minecraft:stone"),
            (12, -59, 5, "minecraft:stone"),
            (13, -60, 3, "minecraft:stone"),
            (13, -60, 4, "minecraft:stone"),
            (13, -60, 5, "minecraft:stone"),
            (13, -59, 3, "minecraft:stone"),
            (13, -59, 5, "minecraft:stone"),
            (14, -60, 3, "minecraft:stone"),
            (14, -60, 4, "minecraft:stone"),
            (14, -60, 5, "minecraft:stone"),
            (14, -59, 3, "minecraft:stone"),
            (14, -59, 5, "minecraft:stone"),
            (15, -60, 3, "minecraft:stone"),
            (15, -60, 4, "minecraft:stone"),
            (15, -60, 5, "minecraft:stone"),
            (15, -59, 3, "minecraft:stone"),
            (15, -59, 4, "minecraft:stone"),
            (15, -59, 5, "minecraft:stone"),
        ],
    ),
    (
        "falling_sand_column",
        [4, -61, 10, 4, -46, 10],
        &[
            (4, -61, 10, "minecraft:dirt"),
            (4, -50, 10, "minecraft:sand"),
            (4, -49, 10, "minecraft:sand"),
            (4, -48, 10, "minecraft:sand"),
            (4, -47, 10, "minecraft:sand"),
        ],
    ),
    (
        "leaf_distance_cascade",
        [18, -55, 2, 24, -55, 2],
        &[
            (
                19,
                -55,
                2,
                "minecraft:oak_leaves[distance=1,persistent=true,waterlogged=false]",
            ),
            (
                20,
                -55,
                2,
                "minecraft:oak_leaves[distance=2,persistent=true,waterlogged=false]",
            ),
            (
                21,
                -55,
                2,
                "minecraft:oak_leaves[distance=3,persistent=true,waterlogged=false]",
            ),
            (
                22,
                -55,
                2,
                "minecraft:oak_leaves[distance=4,persistent=true,waterlogged=false]",
            ),
            (
                23,
                -55,
                2,
                "minecraft:oak_leaves[distance=5,persistent=true,waterlogged=false]",
            ),
            (
                24,
                -55,
                2,
                "minecraft:oak_leaves[distance=6,persistent=true,waterlogged=false]",
            ),
        ],
    ),
    (
        "leaf_decay_patch",
        [19, -51, 9, 21, -49, 11],
        &[
            (
                19,
                -50,
                9,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                19,
                -50,
                10,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                19,
                -50,
                11,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                20,
                -50,
                9,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                20,
                -50,
                10,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                20,
                -50,
                11,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                21,
                -50,
                9,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                21,
                -50,
                10,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
            (
                21,
                -50,
                11,
                "minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]",
            ),
        ],
    ),
    (
        "entity_fall_and_land",
        [34, -61, 8, 38, -55, 8],
        &[
            (34, -61, 8, "minecraft:dirt"),
            (35, -61, 8, "minecraft:dirt"),
            (36, -61, 8, "minecraft:dirt"),
            (37, -61, 8, "minecraft:dirt"),
            (38, -61, 8, "minecraft:dirt"),
            (38, -55, 8, "minecraft:sand"),
        ],
    ),
    (
        "redstone_lamp_release",
        [48, -60, 4, 49, -60, 4],
        &[(48, -60, 4, "minecraft:redstone_lamp[lit=true]")],
    ),
    (
        "redstone_lamp_repowered",
        [48, -60, 6, 52, -60, 6],
        &[
            (48, -60, 6, "minecraft:redstone_lamp[lit=true]"),
            (49, -60, 6, "minecraft:redstone_block"),
            (51, -60, 6, "minecraft:redstone_lamp[lit=true]"),
        ],
    ),
    (
        "redstone_torch_tower",
        [54, -61, 4, 54, -58, 4],
        &[
            (54, -61, 4, "minecraft:dirt"),
            (54, -60, 4, "minecraft:redstone_torch[lit=true]"),
            (54, -59, 4, "minecraft:stone"),
            (54, -58, 4, "minecraft:redstone_torch[lit=true]"),
        ],
    ),
];

/// `ServerLevel.setChunkForced` (ServerLevel.java:1497-1504) through the
/// dispatcher: the ticket `/forceload add` files is the same
/// `TicketType.FORCED` at `ChunkMap.FORCED_TICKET_LEVEL` a save carries, and
/// what `query` reads back is what `remove` takes away.
#[test]
fn a_forceload_command_files_the_ticket_a_save_would_have_carried() {
    let (sender, receiver) = mailbox();
    let mut candidate = restore_candidate_without_tickets(10, None, Vec::new());
    // `TicketType.PORTAL` is the other type a save can carry
    // (TicketType.java:23). It is not a `FORCED` ticket, so `/forceload` neither
    // lists it nor takes it away.
    candidate.chunk_tickets.insert(
        Java26LevelKey::from(Java26BuiltinDimension::Overworld),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 9,
            chunk_z: 9,
            level: 31,
            ticks_left: Some(300),
        }],
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();

    let tick = block_command(&sender, &receiver, &mut world, 1, "forceload add 0 0 20 0");
    assert_eq!(
        management_report(&tick),
        (
            Some(2),
            vec![
                "commands.forceload.added.multiple(2, minecraft:overworld, [0, 0], [1, 0])"
                    .to_string()
            ]
        )
    );
    assert_eq!(
        world.restored_chunk_ticket_for_test(Java26BuiltinDimension::Overworld, (1, 0)),
        Some((31, None))
    );
    // The ticket makes the two columns simulate, which is the whole point of
    // the root: nobody is online and the save declared no forced column. The
    // portal ticket simulates too -- both persisted types are flags 15.
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        [(0, 0), (1, 0), (9, 9)]
    );

    let tick = block_command(&sender, &receiver, &mut world, 2, "forceload query");
    assert_eq!(
        management_report(&tick),
        (
            Some(2),
            vec![
                "commands.forceload.list.multiple(2, minecraft:overworld, [0, 0], [1, 0])"
                    .to_string()
            ]
        )
    );
    // Block 150 150 is column (9, 9), the one the portal ticket sits on.
    let tick = block_command(&sender, &receiver, &mut world, 3, "forceload query 150 150");
    assert_eq!(
        management_report(&tick).1,
        ["commands.forceload.query.failure([9, 9], minecraft:overworld)".to_string()],
        "a portal ticket is not a forced chunk"
    );

    let tick = block_command(&sender, &receiver, &mut world, 4, "forceload remove all");
    assert_eq!(
        management_report(&tick),
        (
            Some(0),
            vec!["commands.forceload.removed.all(minecraft:overworld)".to_string()]
        )
    );
    assert_eq!(
        world.restored_chunk_ticket_for_test(Java26BuiltinDimension::Overworld, (1, 0)),
        None
    );
    let portal = world.restored_chunk_ticket_for_test(Java26BuiltinDimension::Overworld, (9, 9));
    assert!(
        matches!(portal, Some((31, Some(_)))),
        "`removeAll` walks the forced set, not the whole ticket list: {portal:?}"
    );
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        [(9, 9)]
    );
}

/// `MinecraftServer.setRespawnData` (MinecraftServer.java:1699-1706) writes and
/// broadcasts only when the new `RespawnData` differs.
#[test]
fn setworldspawn_broadcasts_only_when_the_stored_respawn_data_changes() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "op", AdminScope::Manage),
            request: AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_id(Uuid::from_u128(1)),
                permission_level: Some(CommandPermission::GameMaster),
                bypasses_player_limit: None,
            }]),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    let spawn_packets = |tick: &TickResult| {
        tick.events
            .iter()
            .flat_map(WorldEvent::deliveries)
            .filter_map(|(connection_id, packet)| match packet {
                ClientboundPlayPacket::SetDefaultSpawnPosition(update) => {
                    Some((connection_id, update.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "setworldspawn 0 64 0".into(),
            },
        },
    )
    .unwrap();
    let unchanged = world.tick(receiver.drain());
    assert!(
        spawn_packets(&unchanged).is_empty(),
        "{:?}",
        unchanged.events
    );

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "setworldspawn 8 64 8 45 -10".into(),
            },
        },
    )
    .unwrap();
    let changed = world.tick(receiver.drain());
    let packets = spawn_packets(&changed);
    assert_eq!(packets.len(), 2, "{:?}", changed.events);
    let connections = packets
        .iter()
        .map(|(id, _)| *id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        connections,
        [ConnectionId(1), ConnectionId(2)].into_iter().collect()
    );
    for (_, update) in &packets {
        assert_eq!(update.dimension, "minecraft:overworld");
        assert_eq!(update.pos, BlockPos { x: 8, y: 64, z: 8 });
        assert_eq!(update.yaw, 45.0);
        assert_eq!(update.pitch, -10.0);
    }

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "setworldspawn 8 64 8 45 -10".into(),
            },
        },
    )
    .unwrap();
    let again = world.tick(receiver.drain());
    assert!(spawn_packets(&again).is_empty(), "{:?}", again.events);
}

/// `PrimaryLevelData.setTagData` (`PrimaryLevelData.java:132`) writes the
/// live `respawnData` on every save.
#[test]
fn setworldspawn_reaches_the_save_request() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setworldspawn 8 64 8 45 -10",
    );

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.level_state.spawn,
        Java26LevelSpawn::new("minecraft:overworld".to_string(), [8, 64, 8], 45.0, -10.0)
    );
}

/// `advancements={..}` (EntitySelectorOptions.java:437-447) over the live
/// `PlayerAdvancements` store: the projection the command reads carries the
/// same two answers `getOrStartProgress` would have.
#[test]
fn a_selector_over_the_advancement_store_names_only_the_player_that_holds_it() {
    // The tree is the loaded generation's, so the config carries one.
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..operator_config(&[("op", 1), ("other", 2)])
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "other", 2);

    // `PlayerAdvancements.award` in place of the criterion trigger that fires
    // it on a live session.
    assert!(world.award_player_advancement_for_test(
        ConnectionId(1),
        "minecraft:story/root",
        "crafting_table",
        0,
    ));
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:story/root"));
    assert!(!world.player_advancement_is_done(ConnectionId(2), "minecraft:story/root"));

    let held = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "tag @a[advancements={minecraft:story/root=true}] add marked",
    );
    assert_eq!(
        system_chats(&held)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        // The success line, and the admin copy `broadcastToAdmins` sends
        // beside it.
        vec![
            "commands.tag.add.success.single markedop",
            "chat.type.admin opcommands.tag.add.success.single markedop",
        ]
    );
    // The other half of the same store: the player who has not started it.
    let missing = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "tag @a[advancements={minecraft:story/root=false}] add unmarked",
    );
    assert_eq!(
        system_chats(&missing)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "commands.tag.add.success.single unmarkedother",
            "chat.type.admin opcommands.tag.add.success.single unmarkedother",
        ]
    );
}

/// `Player.giveExperienceLevels` (Player.java:1546-1550) plays the level-up
/// sound on a fifth level once `tickCount` is past 100, and
/// `giveExperiencePoints` raises `DATA_SCORE_ID` (`:1501`, `:505-508`).
#[test]
fn experience_plays_the_level_up_sound_and_points_raise_the_score() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    for _ in 0..105 {
        world.tick(receiver.drain());
    }
    let levelled = chat_command(&sender, &receiver, &mut world, 1, "xp add op 5 levels");
    let sounds: Vec<_> = levelled
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Sound(sound),
            } => Some(sound),
            _ => None,
        })
        .collect();
    let levelup = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.player.levelup",
    )
    .unwrap();
    assert!(
        sounds.iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: levelup
            }
            && sound.source == SoundSource::Players
            && sound.volume == 5.0 / 30.0 * 0.75
            && sound.pitch == 1.0),
        "{sounds:?}"
    );
    let pointed = chat_command(&sender, &receiver, &mut world, 1, "xp add op 14");
    let score = pointed.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::SetEntityData(data),
        } => data.values.iter().find_map(|value| {
            (value.data_id == bbb_protocol::entity_synched_data::VANILLA_PLAYER_SCORE_DATA_ID)
                .then_some(value.value.clone())
        }),
        _ => None,
    });
    assert_eq!(
        score,
        Some(bbb_protocol::packets::EntityDataValueKind::Int(14))
    );
}

/// `BlockStateParser.readTag` (`BlockStateParser.java:372-377`) looks the tag
/// up in the loaded block registry, so `/execute if block` accepts a tag only
/// a datapack defines and matches its members, and a world without the pack
/// refuses the same spelling as an unknown tag.
#[test]
fn execute_if_block_matches_a_datapack_block_tag() {
    let (sender, receiver) = mailbox();
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(
        datapack_config_with_block_tags(&[("test:quarry", &["minecraft:stone"], false)]),
        &candidate,
    )
    .unwrap();
    block_command(&sender, &receiver, &mut world, 1, "setblock 3 64 8 stone");
    block_command(&sender, &receiver, &mut world, 2, "setblock 4 64 8 dirt");
    for (id, command, expected) in [
        (
            3,
            "execute if block 3 64 8 #test:quarry",
            "commands.execute.conditional.pass",
        ),
        (
            4,
            "execute if block 4 64 8 #test:quarry",
            "commands.execute.conditional.fail",
        ),
    ] {
        let tick = block_command(&sender, &receiver, &mut world, id, command);
        assert_eq!(
            management_report(&tick).1,
            vec![expected.to_string()],
            "{command}"
        );
    }

    let mut vanilla = block_command_world();
    let tick = block_command(
        &sender,
        &receiver,
        &mut vanilla,
        5,
        "execute if block 3 64 8 #test:quarry",
    );
    let (result, messages) = management_report(&tick);
    assert_eq!(result, None);
    assert!(
        messages[0].starts_with("arguments.block.tag.unknown"),
        "{messages:?}"
    );
}
