//! Damage, death and respawn: the health a hit leaves, the sounds and messages
//! it publishes, and the gate a dead session sits behind.

use super::*;

/// An operator on a survival world, which is what `/damage` needs on both
/// sides: the level to run the command (`Commands.LEVEL_GAMEMASTERS`,
/// DamageCommand.java:24) and a target whose abilities do not carry
/// `invulnerable` (GameType.java:72-76).
fn survival_operator_config(operators: &[(&str, u128)]) -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(operators)
    }
}

/// Every `ClientboundDamageEventPacket` a tick sent.
fn damage_events(tick: &TickResult) -> Vec<(ConnectionId, DamageEvent)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DamageEvent(update),
            } => Some((*connection_id, *update)),
            _ => None,
        })
        .collect()
}

/// `DamageCommand.damage` (DamageCommand.java:104-106) over a live session:
/// the health drops by the amount, the trackers are told through
/// `ServerLevel.broadcastDamageEvent` (ServerLevel.java:1186-1188) and the hurt
/// session is told its own health by `ServerPlayer.doTick`
/// (ServerPlayer.java:671-678).
#[test]
fn damage_drops_a_survival_sessions_health_and_tells_it_and_the_trackers() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    let joined = join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(!joined.is_empty());
    // The first tick already announced the full health, so the next one only
    // announces what the command changed.
    let settled = world.tick(receiver.drain());
    assert!(set_healths(&settled).is_empty(), "{settled:?}");

    let hurt = chat_command(&sender, &receiver, &mut world, 1, "damage alice 6");
    // `minecraft:generic` is `when_caused_by_living_non_player`, and the arm
    // that scaling picks (`DamageSource.scalesWithDifficulty`,
    // DamageSource.java:95) asks whether the causing entity is a living
    // non-player. `/damage <target> <amount>` hurts with
    // `damageSources().generic()` (DamageCommand.java:34), which names no
    // entity at all, so the arm is false and the difficulty scaling
    // (`Player.hurtServer`, Player.java:694-706) never runs.
    assert_eq!(set_healths(&hurt), [(ConnectionId(1), 14.0)], "{hurt:?}");
    let events = damage_events(&hurt);
    assert_eq!(events.len(), 1, "{events:?}");
    let (connection_id, event) = events[0];
    assert_eq!(connection_id, ConnectionId(1));
    // `writeOptionalEntityId` writes a zero for a source that names no entity,
    // which the decoder reads back as -1.
    assert_eq!(event.source_cause_id, -1);
    assert_eq!(event.source_direct_id, -1);
    assert_eq!(event.source_position, None);
    // `sendSuccess` reaches the source alone, and the line it carries is
    // `commands.damage.success` (DamageCommand.java:105).
    let chats = system_chats(&hurt);
    assert_eq!(chats.len(), 1, "{chats:?}");
    assert_eq!(chats[0].0, ConnectionId(1));
    assert!(
        chats[0].1.contains("commands.damage.success"),
        "{:?}",
        chats[0].1
    );
}

/// `/damage` resolves native `EntityStore` UUIDs, evaluates feedback through
/// the canonical clone-safe dispatcher, then applies the retained effect to
/// the live level owner. These three cases pin the production path rather than
/// only its command projection or mechanism unit tests.
#[test]
fn damage_command_reaches_every_current_stored_entity_kind() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice_id = entity_id_of(&world, 1);
    let (item_id, item_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(
            crate::entity::item::plain_stack("minecraft:stone", 1)
                .expect("stone is a built-in item"),
            0.0,
        )),
    );
    let item_hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {item_uuid} 2 minecraft:generic by alice"),
    );
    assert!(system_chats(&item_hurt)[0]
        .1
        .contains("commands.damage.success"));
    let item = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, item_id)
        .expect("a non-lethal hit retains the item");
    let crate::entity::EntityData::Item(item) = item.data() else {
        panic!("the target remains an item")
    };
    assert_eq!(item.health(), 3);
    let (game_events, game_event_count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(game_event_count, 1);
    assert_eq!(game_events.len(), 1);
    assert_eq!(game_events[0].game_event, VanillaGameEvent::ENTITY_DAMAGE);
    assert_eq!(game_events[0].source_entity_id, Some(alice_id));

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (orb_id, orb_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(1)),
    );
    let orb_hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {orb_uuid} 2"),
    );
    assert!(system_chats(&orb_hurt)[0]
        .1
        .contains("commands.damage.success"));
    let orb = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, orb_id)
        .expect("a non-lethal hit retains the orb");
    let crate::entity::EntityData::ExperienceOrb(orb) = orb.data() else {
        panic!("the target remains an experience orb")
    };
    assert_eq!(orb.health(), 3);

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (falling_id, falling_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::FallingBlock(crate::entity::FallingBlockData::new(
            block_state_id("minecraft:sand"),
            BlockPos { x: 4, y: 70, z: 0 },
        )),
    );
    let falling_hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {falling_uuid} 2"),
    );
    assert!(system_chats(&falling_hurt)[0]
        .1
        .contains("commands.damage.invulnerable"));
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, falling_id)
        .is_some());
    assert!(falling_hurt.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityMotion(motion),
            ..
        } if motion.id == falling_id
    )));
}

/// `EndCrystal.hurtServer` removes the crystal before its player-attributed
/// explosion enters `ServerLevel.explode` (`EndCrystal.java:97-102`).
#[test]
fn end_crystal_damage_unpairs_before_the_player_attributed_explosion() {
    let (sender, receiver) = mailbox();
    let mut config = survival_operator_config(&[("alice", 1)]);
    config.datapacks = datapack_config().datapacks;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice_id = entity_id_of(&world, 1);
    let crystal_position = Vec3d {
        x: 8.5,
        y: 64.0,
        z: 0.5,
    };
    let (crystal_id, crystal_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        crystal_position,
        crate::entity::EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, true)),
    );
    let paired = world.tick(receiver.drain());
    assert!(paired.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AddEntity(add),
            ..
        } if add.id == crystal_id
    )));

    let hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {crystal_uuid} 1 minecraft:generic by alice"),
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, crystal_id)
        .is_none());

    let damage = damage_events(&hurt);
    assert_eq!(damage.len(), 1, "{hurt:?}");
    assert_eq!(damage[0].1.entity_id, alice_id);
    assert_eq!(
        damage[0].1.source_type_id,
        bbb_server_datapack::damage::builtin_damage_type_registry()
            .iter()
            .find(|facts| facts.resource_id == "minecraft:player_explosion")
            .expect("player explosion is a built-in damage type")
            .registry_id
    );
    assert_eq!(damage[0].1.source_cause_id, alice_id);
    assert_eq!(damage[0].1.source_direct_id, crystal_id);
}

#[test]
fn armor_stand_command_apply_uses_live_may_build_and_block_drops() {
    let causing_player = |game_type| crate::commands::CommandDamageSourceEntity::Player {
        user: NameAndId {
            id: Uuid::from_u128(1),
            name: "alice".into(),
        }
        .into(),
        entity_id: 1,
        position: Vec3d::default(),
        instabuild: game_type == GameType::Creative,
        may_build: bbb_server_types::block_queries::game_type_may_build(game_type),
        spectator: game_type == GameType::Spectator,
        display_name: NbtSummaryValue::string("alice"),
    };
    for game_type in [GameType::Adventure, GameType::Spectator] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let (stand_id, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: 4.5,
                y: 70.0,
                z: 0.5,
            },
            crate::entity::EntityData::ArmorStand(
                crate::entity::ArmorStandData::new().expect("armor stand attributes"),
            ),
        );
        let outcome = world
            .hurt_stored_entity_by_command_for_test(
                Java26BuiltinDimension::Overworld,
                stand_id,
                "minecraft:arrow",
                1.0,
                Some(causing_player(game_type)),
            )
            .expect("the stand exists");
        // `ArmorStand.hurtServer` refuses a causing player without `mayBuild`
        // before its creative or incremental breaking arms (`ArmorStand.java:314-326`).
        assert!(!outcome.applied && !outcome.succeeded, "{game_type:?}");
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
            .is_some());
    }

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule block_drops false",
    );
    let (stand_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            stand_id,
            "minecraft:arrow",
            1.0,
            Some(causing_player(GameType::Survival)),
        )
        .expect("the stand exists");
    assert!(outcome.applied && outcome.succeeded);
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, stand_id)
        .is_none());
    // `Block.popResource` creates no item when `BLOCK_DROPS` is false
    // (`Block.java:442-448`).
    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        0
    );
}

/// The startup tag report owns only dynamic registries today. Installing that
/// report must not turn an unowned fixed item tag into an empty tag: persistent
/// ItemStack validation still resolves it from the verified 26.1 fixed facts.
#[test]
fn stored_damage_with_datapacks_keeps_fixed_registry_tag_facts() {
    let mut config = survival_operator_config(&[("alice", 1)]);
    config.datapacks = datapack_config().datapacks;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:stone".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:repairable".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "items".into(),
                        value: NbtSummaryValue::String("#minecraft:repairs_iron_armor".into()),
                    }]),
                }]),
            },
        ]),
        &world.item_stack_resolver_for_test(),
    )
    .expect("the writer-canonical stack is structurally valid");
    let (item_id, item_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(stack, 0.0)),
    );

    let hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {item_uuid} 1"),
    );
    assert!(system_chats(&hurt)[0].1.contains("commands.damage.success"));
    let item = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, item_id)
        .expect("the non-lethal target remains live");
    let crate::entity::EntityData::Item(item) = item.data() else {
        panic!("the target remains an item")
    };
    assert_eq!(item.health(), 4);
}

/// `ServerPlayer.hurtServer` applies the PVP gate only when
/// `DamageSource.getEntity()` is a player (ServerPlayer.java:976-983). A
/// stored source keeps its entity id on the damage packet without being
/// projected into the player-only hurt facts.
#[test]
fn stored_damage_source_does_not_become_a_player_when_pvp_is_disabled() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (source_id, source_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(1)),
    );
    chat_command(&sender, &receiver, &mut world, 1, "gamerule pvp false");

    let hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 1 minecraft:generic by {source_uuid}"),
    );
    assert_eq!(set_healths(&hurt), [(ConnectionId(1), 19.0)]);
    let damage = damage_events(&hurt);
    assert_eq!(damage.len(), 1, "{damage:?}");
    assert_eq!(damage[0].1.source_cause_id, source_id);
    assert_eq!(damage[0].1.source_direct_id, source_id);
}

/// `DamageSource.getLocalizedDeathMessage` uses any causing entity's display
/// name (DamageSource.java:71-86); a non-player source must stay out of PVP and
/// kill-credit facts without disappearing from the lethal message.
#[test]
fn lethal_stored_damage_source_names_the_entity_in_the_death_message() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (_, source_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        Vec3d {
            x: 4.5,
            y: 70.0,
            z: 0.5,
        },
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(1)),
    );

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 40 minecraft:generic by {source_uuid}"),
    );
    let messages = killed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerCombatKill(update),
                ..
            } => Some(update.message.text().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 1, "{killed:?}");
    assert!(messages[0].contains("alice"), "{messages:?}");
    assert!(
        messages[0].contains("entity.minecraft.experience_orb"),
        "{messages:?}"
    );
}

/// Lethal `/damage` reaches the real protocol component projection and the
/// live store mutation as one path: contents are paired before the discarded
/// source is removed, and both concrete item implementations keep their own
/// component order.
#[test]
fn lethal_stored_damage_releases_container_and_bundle_contents_before_removal() {
    let datapacks = datapack_config()
        .datapacks
        .expect("the startup helper installs one generation");
    let template = |id: &str, count: i32| {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String(id.into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(count),
            },
        ])
    };
    let container_slot = |slot: i32, item: NbtSummaryValue| {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "slot".into(),
                value: NbtSummaryValue::Int(slot),
            },
            NbtSummaryEntry {
                name: "item".into(),
                value: item,
            },
        ])
    };
    let cases = [
        (
            "minecraft:shulker_box",
            "minecraft:container",
            NbtSummaryValue::List(vec![
                container_slot(2, template("minecraft:stone", 1)),
                container_slot(0, template("minecraft:dirt", 2)),
            ]),
            [("minecraft:dirt", 2), ("minecraft:stone", 1)],
        ),
        (
            "minecraft:bundle",
            "minecraft:bundle_contents",
            NbtSummaryValue::List(vec![
                template("minecraft:stone", 1),
                template("minecraft:dirt", 2),
            ]),
            [("minecraft:stone", 1), ("minecraft:dirt", 2)],
        ),
    ];

    for (source_item, component, contents, expected) in cases {
        let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String(source_item.into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(1),
                },
                NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: component.into(),
                        value: contents,
                    }]),
                },
            ]),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .expect("the source stack is writer-canonical");
        let mut config = survival_operator_config(&[("alice", 1)]);
        config.datapacks = Some(Arc::clone(&datapacks));
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(config).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let (source_id, source_uuid) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: 4.5,
                y: 70.0,
                z: 0.5,
            },
            crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(stack, 0.0)),
        );
        let paired = world.tick(receiver.drain());
        assert!(paired.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } if add.id == source_id
        )));

        let killed = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("damage {source_uuid} 5"),
        );
        assert!(system_chats(&killed)[0]
            .1
            .contains("commands.damage.success"));
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, source_id)
            .is_none());
        let mut child_adds = Vec::new();
        for (index, event) in killed.events.iter().enumerate() {
            match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                } if add.entity_type_id
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID =>
                {
                    child_adds.push((index, add.id));
                }
                _ => {}
            }
        }
        assert_eq!(child_adds.len(), 2, "{killed:?}");
        let actual = child_adds
            .iter()
            .map(|(_, child_id)| {
                let child = world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *child_id)
                    .expect("the released child remains live");
                let crate::entity::EntityData::Item(item) = child.data() else {
                    panic!("onDestroyed releases item entities")
                };
                let item = item.item();
                (item.item_resource_id().unwrap().to_owned(), item.count())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            expected
                .iter()
                .map(|(id, count)| ((*id).to_owned(), *count))
                .collect::<Vec<_>>()
        );
        let (game_events, count) = world.game_events(Java26BuiltinDimension::Overworld);
        assert_eq!(count, 1);
        assert_eq!(game_events[0].game_event, VanillaGameEvent::ENTITY_DAMAGE);
        assert_eq!(game_events[0].source_entity_id, None);
    }
}

/// `Player.hurtServer` (Player.java:684-686): creative abilities refuse the
/// source, so nothing at all happens and the command reports
/// `commands.damage.invulnerable` (DamageCommand.java:19).
#[test]
fn damage_on_a_creative_session_changes_no_health_at_all() {
    let (sender, receiver) = mailbox();
    // The fixture default is creative, which is exactly the target this pins.
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());

    let refused = chat_command(&sender, &receiver, &mut world, 1, "damage alice 6");
    assert!(set_healths(&refused).is_empty(), "{refused:?}");
    assert!(damage_events(&refused).is_empty(), "{refused:?}");
    // The refusal still reaches the source.
    assert_eq!(system_chats(&refused).len(), 1, "{refused:?}");
}

/// `LivingEntity.hurtServer` (LivingEntity.java:1196-1210): a second hit inside
/// the twenty-tick window charges only what it exceeds the first by, and a hit
/// that does not exceed it charges nothing and answers false.
#[test]
fn a_second_hit_inside_the_window_charges_only_the_difference() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());

    let first = chat_command(&sender, &receiver, &mut world, 1, "damage alice 6");
    assert_eq!(set_healths(&first), [(ConnectionId(1), 14.0)]);
    // `invulnerableTime` is twenty after the hit and falls by one a tick
    // (ServerPlayer.java:576-578), so the next tick is still above the ten the
    // window is measured against.
    let second = chat_command(&sender, &receiver, &mut world, 1, "damage alice 10");
    assert_eq!(
        set_healths(&second),
        [(ConnectionId(1), 10.0)],
        "only the four the second hit exceeded the first by"
    );
    // `:1197-1199`: a hit that does not exceed `lastHurt` is refused outright,
    // and `lastHurt` is now ten.
    let third = chat_command(&sender, &receiver, &mut world, 1, "damage alice 10");
    assert!(set_healths(&third).is_empty(), "{third:?}");
    assert_eq!(system_chats(&third).len(), 1, "the refusal is reported");
    // `:1215-1220` only runs for a hit that took full damage, so neither of the
    // two window hits broadcast a damage event.
    assert!(damage_events(&second).is_empty(), "{second:?}");
    assert!(damage_events(&third).is_empty(), "{third:?}");
}

/// A hit that empties the health runs `ServerPlayer.die`
/// (ServerPlayer.java:869-929): the dying session gets
/// `ClientboundPlayerCombatKillPacket` and everyone gets the death message.
/// `Player.playSound` (Player.java:397-399): the hurt player is the one
/// listener its own hurt sound is not sent to.
///
/// `LivingEntity.hurtServer` plays it at :1255 through `makeSound`
/// (LivingEntity.java:1406-1410), which for a player reaches
/// `Level.playSound` with the player itself as the excluded entity, and
/// `PlayerList.broadcast` (PlayerList.java:597-617) skips that one by identity
/// at :608 before it measures any distance. The client that owns the hurt
/// player plays the sound itself off `handleDamageEvent`, so a server that
/// sent it as well would have the hit sound twice there — while a second
/// session standing on the same block is told about it once.
#[test]
fn the_hurt_session_is_the_one_listener_left_out_of_its_own_hurt_sound() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());

    let hurt = chat_command(&sender, &receiver, &mut world, 1, "damage bob 6");
    let heard = hurt
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(update),
            } => Some((*connection_id, update.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(heard.len(), 1, "{heard:?}");
    assert_eq!(
        heard[0].0,
        ConnectionId(1),
        "the hurt session must not be sent its own hurt sound"
    );
    // The hit happened, which is what makes the single packet an exclusion
    // rather than a server that played nothing at all.
    assert_eq!(set_healths(&hurt), [(ConnectionId(2), 14.0)], "{hurt:?}");
}

#[test]
fn a_lethal_hit_kills_the_session_and_tells_everyone_who_died() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:generic by alice",
    );
    assert_eq!(set_healths(&killed), [(ConnectionId(2), 0.0)], "{killed:?}");
    let kills = killed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerCombatKill(update),
            } => Some((*connection_id, update.message.text().to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(kills.len(), 1, "{kills:?}");
    assert_eq!(kills[0].0, ConnectionId(2), "the packet is the dead one's");
    // `DamageSource.getLocalizedDeathMessage` (DamageSource.java:71-86): a
    // source that names an entity takes the `.player` suffix off and passes the
    // attacker as the second argument (`:84`). The key is
    // `death.attack.<msgId>`, and `generic`'s `msgId` is `generic`.
    assert!(
        kills[0].1.contains("death.attack.generic"),
        "{:?}",
        kills[0].1
    );
    assert!(
        kills[0].1.contains("bob") && kills[0].1.contains("alice"),
        "{:?}",
        kills[0].1
    );
    // `:892-895`: with `show_death_messages` on and no team registered the
    // message reaches everyone.
    let chats = system_chats(&killed);
    assert!(
        chats
            .iter()
            .filter(|(_, line)| line.contains("death.attack.generic"))
            .count()
            == 2,
        "{chats:?}"
    );
    // `:919`: the death animation event goes to the trackers and to the dead
    // session itself.
    assert_eq!(
        killed
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::EntityEvent(EntityEvent { event_id: 3, .. }),
                    ..
                }
            ))
            .count(),
        2,
        "{killed:?}"
    );
}

/// Every `ClientboundEntityEventPacket` a tick sent, as (recipient, entity, id).
fn entity_events(tick: &TickResult) -> Vec<(ConnectionId, i32, i8)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::EntityEvent(update),
            } => Some((*connection_id, update.entity_id, update.event_id)),
            _ => None,
        })
        .collect()
}

/// `LivingEntity.tickDeath` (LivingEntity.java:549-555): twenty ticks after the
/// killing blow the body is announced with `EntityEvent.POOF`
/// (EntityEvent.java:55) and removed, which unpairs it from every tracker.
#[test]
fn the_body_poofs_and_is_unpaired_twenty_ticks_after_the_death() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());
    let bob_entity = snapshot_player(&world.tick(receiver.drain()), 2).entity_id;
    let alice_entity = entity_id_of(&world, 1);

    let killed = chat_command(&sender, &receiver, &mut world, 1, "damage bob 40");
    // The command runs at the head of the tick and `tickDeath` runs later in
    // the same one, so the killing tick is already `deathTime` one.
    assert!(
        !entity_events(&killed)
            .iter()
            .any(|(_, _, event_id)| *event_id == 60),
        "{killed:?}"
    );

    // Nineteen more ticks take it to twenty, which is the tick `:551` fires on.
    for tick in 0..18 {
        let quiet = world.tick(receiver.drain());
        assert!(
            !entity_events(&quiet)
                .iter()
                .any(|(_, _, event_id)| *event_id == 60),
            "tick {tick} poofed early: {quiet:?}"
        );
    }
    let poofed = world.tick(receiver.drain());
    let poofs = entity_events(&poofed)
        .into_iter()
        .filter(|(_, _, event_id)| *event_id == 60)
        .collect::<Vec<_>>();
    // `broadcastEntityEvent` reaches every tracker and the entity itself.
    assert_eq!(
        poofs,
        [
            (ConnectionId(1), bob_entity, 60),
            (ConnectionId(2), bob_entity, 60)
        ],
        "{poofed:?}"
    );
    // The dead session also clears its view of the surviving avatar.
    let mut removals = poofed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::RemoveEntities(update),
            } => Some((*connection_id, update.entity_ids.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    removals.sort_unstable_by_key(|(connection_id, _)| connection_id.0);
    assert_eq!(
        removals,
        [
            (ConnectionId(1), vec![bob_entity]),
            (ConnectionId(2), vec![alice_entity])
        ],
        "{poofed:?}"
    );

    let swung = chat_command(&sender, &receiver, &mut world, 1, "swing bob");
    assert!(
        !swung.events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(update),
                ..
            } if update.id == bob_entity)),
        "{swung:?}"
    );

    // The body stays gone rather than being announced again every tick.
    let settled = world.tick(receiver.drain());
    assert!(
        !entity_events(&settled)
            .iter()
            .any(|(_, _, event_id)| *event_id == 60),
        "{settled:?}"
    );
}

/// `handleClientCommand`'s death branch (ServerGamePacketListenerImpl.java:1872-1883)
/// through `PlayerList.respawn` (PlayerList.java:385-451).
///
/// A dead session is gated out of every handler that asks `hasClientLoaded`
/// (`:2198-2200`) because `markClientUnloadedAfterDeath` (`:2212-2214`) raised
/// `waitingForRespawn`; the respawn is the only thing that lowers it again
/// (`:2216-2219`), and it opens a fresh sixty-tick window (`:2218`) the client
/// closes by saying it has loaded.
#[test]
fn a_dead_session_stays_gated_until_the_respawn_rebuilds_it() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());

    let killed = chat_command(&sender, &receiver, &mut world, 1, "damage bob 40");
    assert_eq!(set_healths(&killed), [(ConnectionId(2), 0.0)], "{killed:?}");
    assert!(!snapshot_player(&killed, 2).client_loaded);

    // `ServerPlayer.isInvulnerableTo` (ServerPlayer.java:1266) refuses every
    // source for a session that has not loaded, so the dead one cannot be hurt
    // again while it waits.
    let refused = chat_command(&sender, &receiver, &mut world, 1, "damage bob 1");
    assert!(set_healths(&refused).is_empty(), "{refused:?}");
    assert!(
        system_chats(&refused)[0]
            .1
            .contains("commands.damage.invulnerable"),
        "{refused:?}"
    );

    let respawned = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    // `:412` and `:413`: the respawn packet keeps nothing and is followed by
    // the teleport to the respawn position.
    let to_bob = |tick: &TickResult| {
        tick.events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet,
                } => Some(packet.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let sent = to_bob(&respawned);
    let respawn_index = sent
        .iter()
        .position(|packet| matches!(packet, ClientboundPlayPacket::Respawn(_)))
        .expect("the respawn packet is sent");
    let ClientboundPlayPacket::Respawn(respawn) = &sent[respawn_index] else {
        unreachable!("just matched");
    };
    assert_eq!(respawn.data_to_keep, 0);
    let teleport_index = sent
        .iter()
        .position(|packet| matches!(packet, ClientboundPlayPacket::PlayerPosition(_)))
        .expect("the respawn teleports");
    assert!(teleport_index > respawn_index, "{sent:?}");
    // `restoreFrom` (ServerPlayer.java:1604) puts the new player at full
    // health, which `doTick` reports because the fresh entity has sent none.
    assert_eq!(set_healths(&respawned), [(ConnectionId(2), 20.0)]);
    // `:1878` reopens the client-load window, so the session is still gated.
    assert!(!snapshot_player(&respawned, 2).client_loaded);
    // The avatar is paired again with the other session.
    assert!(
        respawned.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::AddEntity(_),
            }
        )),
        "{respawned:?}"
    );

    let teleport_id = sent
        .iter()
        .find_map(|packet| match packet {
            ClientboundPlayPacket::PlayerPosition(update) => Some(update.id),
            _ => None,
        })
        .expect("the respawn teleports");
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    let loaded = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::PlayerLoaded,
    );
    assert!(snapshot_player(&loaded, 2).client_loaded);

    // And the respawned session takes damage like any other.
    let hurt = chat_command(&sender, &receiver, &mut world, 1, "damage bob 3");
    assert_eq!(set_healths(&hurt), [(ConnectionId(2), 17.0)], "{hurt:?}");
}

/// `ServerGamePacketListenerImpl.hasClientLoaded` (:2198-2200) is a timeout,
/// not a latch: the listener's constructor opens the window at sixty (`:280`)
/// and `tickClientLoadTimeout` (`:2202-2206`) counts it down from
/// `ServerPlayer.tick` (ServerPlayer.java:573), so a client that never sends
/// `ServerboundPlayerLoadedPacket` is treated as loaded once it runs out.
#[test]
fn a_session_that_never_reports_loading_is_gated_for_sixty_ticks_and_no_more() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
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
    let joined = world.tick(receiver.drain());
    assert!(!snapshot_player(&joined, 1).client_loaded);
    // The join tick already took one step off the sixty, so fifty-eight more
    // leave it at one and the next one closes it.
    for tick in 0..58 {
        let gated = world.tick(receiver.drain());
        assert!(
            !snapshot_player(&gated, 1).client_loaded,
            "tick {tick} loaded early"
        );
    }
    let loaded = world.tick(receiver.drain());
    assert!(snapshot_player(&loaded, 1).client_loaded);
}

/// `LivingEntity.addAdditionalSaveData` (LivingEntity.java:723-728) writes the
/// health, and `readAdditionalSaveData` (LivingEntity.java:777-820) reads it
/// back through `setHealth` (`:791`), so a hurt session rejoins at the health
/// it left with.
#[test]
fn the_health_a_hit_left_survives_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-damage-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-damage");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();

    let (sender, receiver) = mailbox();
    let mut world =
        ServerWorld::restore(survival_operator_config(&[("alice", 1)]), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let hurt = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 7 minecraft:cactus",
    );
    assert_eq!(set_healths(&hurt), [(ConnectionId(1), 13.0)]);
    let earned_food = live_food(&mut world);
    assert_eq!(earned_food.exhaustion_level(), 0.1);

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world =
        ServerWorld::restore(survival_operator_config(&[("alice", 1)]), &reloaded).unwrap();
    let (_, saved) = reloaded
        .load_player(profile("alice", 1).uuid)
        .unwrap()
        .expect("the hurt session's save wrote its playerdata");
    let cap = bbb_server_datapack::damage::player_max_health();
    assert_eq!(saved.damage(cap).health, 13.0);
    assert_eq!(saved.food_data(), earned_food);
    // `readAdditionalSaveData` (LivingEntity.java:791-793) reads neither
    // `lastHurt` nor `invulnerableTime`, which `addAdditionalSaveData` never
    // wrote, so both restart at zero.
    assert_eq!(saved.damage(cap).last_hurt, 0.0);
    assert_eq!(saved.damage(cap).invulnerable_time, 0);

    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        earned_food.exhaustion_level(),
    );
    // The restored health is what the live session holds: thirteen more damage
    // empties it exactly, which twenty would have survived.
    let killed = chat_command(&sender, &receiver, &mut world, 1, "damage alice 13");
    assert_eq!(set_healths(&killed), [(ConnectionId(1), 0.0)], "{killed:?}");
    storage.close().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

/// The death message one tick sent to a session, if any.
fn combat_kill_message(tick: &TickResult, connection_id: ConnectionId) -> Option<String> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: recipient,
            packet: ClientboundPlayPacket::PlayerCombatKill(update),
        } if *recipient == connection_id => Some(update.message.text().to_string()),
        _ => None,
    })
}

/// `getKillCredit` (`LivingEntity.java:1946-1952`) with no player reference
/// remembered resolves the mob behind `lastHurtByMob`, and
/// `ServerPlayer.die` (`:913-914`) counts that mob's own entity type.
#[test]
fn a_mob_kill_counts_the_mob_type_on_the_victim() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (_, zombie_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 2.0,
            y: 64.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
                .expect("a zombie is a living entity"),
        ),
    );

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 40 minecraft:generic by {zombie_uuid}"),
    );
    assert_eq!(set_healths(&killed), [(ConnectionId(1), 0.0)], "{killed:?}");
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID
            )
        ),
        1
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        0
    );
}

/// A player kill counts on both sides, and a dead session takes no second
/// death: `Player.hurtServer` returns before `die` once the state is dying.
#[test]
fn a_player_kill_credits_both_sides_once() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::{custom_stat, StatKey};

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:generic by alice",
    );
    assert_eq!(set_healths(&killed), [(ConnectionId(2), 0.0)], "{killed:?}");
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(2),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        1
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), custom_stat("minecraft:player_kills")),
        1
    );

    let again = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:generic by alice",
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(2),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        1
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), custom_stat("minecraft:player_kills")),
        1
    );
    assert_eq!(
        combat_kill_message(&again, ConnectionId(2)),
        None,
        "{again:?}"
    );
}

/// `getKillCredit` resolves the remembered player reference through
/// `EntityReference.getEntity(level, Player.class)`, whose lookup is the
/// server's own player list (`ServerLevel.java:1397-1399`), so an attacker who
/// left the server credits nobody -- and does not fall through to a remembered
/// mob.
#[test]
fn an_attacker_who_left_credits_nobody() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::{custom_stat, StatKey};

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 1 minecraft:generic by bob",
    );
    chat_command(&sender, &receiver, &mut world, 1, "kick bob gone");

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic",
    );
    assert_eq!(set_healths(&killed), [(ConnectionId(1), 0.0)], "{killed:?}");
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        0
    );
    let message = combat_kill_message(&killed, ConnectionId(1)).expect("the dead session is told");
    assert!(message.contains("death.attack.generic"), "{message}");
    assert!(
        !message.contains("death.attack.generic.player"),
        "{message}"
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(1), custom_stat("minecraft:player_kills")),
        0
    );
}

/// `lastHurtByPlayerMemoryTime` (`LivingEntity.java:460-464`): the remembered
/// player is forgotten once the hundred-tick window closes.
#[test]
fn an_expired_attribution_credits_nobody() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 1 minecraft:generic by bob",
    );
    for _ in 0..101 {
        world.tick(receiver.drain());
    }

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic",
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        0
    );
    let message = combat_kill_message(&killed, ConnectionId(1)).expect("the dead session is told");
    assert!(
        !message.contains("death.attack.generic.player"),
        "{message}"
    );
}

/// The remembered player resolves through `getPlayerInAnyDimension`
/// (`EntityReference.java:78-82`), which on a `ServerLevel` is the server's
/// whole player list (`ServerLevel.java:1397-1399`), so an attacker who walked
/// into another dimension takes the credit there exactly as it would at home.
#[test]
fn an_attacker_in_another_dimension_still_takes_the_credit() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::{custom_stat, StatKey};

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 1 minecraft:generic by bob",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run tp bob 0 64 0",
    );
    assert_eq!(
        world
            .player_transition_state_for_test(ConnectionId(2))
            .0
            .builtin(),
        Some(Java26BuiltinDimension::Nether)
    );

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic",
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        1
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(2), custom_stat("minecraft:player_kills")),
        1
    );
    let message = combat_kill_message(&killed, ConnectionId(1)).expect("the dead session is told");
    assert!(message.contains("death.attack.generic.player"), "{message}");
}

/// `LivingEntity.pushEntities`'s `MAX_ENTITY_CRAMMING` arm
/// (`LivingEntity.java:3158-3173`) on the real level tick: a cow whose own box
/// holds a crowd above the rule's threshold charges itself the six the rule
/// names.
///
/// The crowd is parked minecarts stacked on the cow: they are pushable
/// (`AbstractMinecart.java:168-170`), have no cramming arm of their own, and a
/// cart at the cow's own `x` and `z` takes no impulse from it
/// (`AbstractMinecart.java:503`), so the pile stays in the cow's box.
fn crammed_world(
    rule: Option<i32>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());
    if let Some(value) = rule {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("gamerule max_entity_cramming {value}"),
        );
        assert_eq!(
            world.game_rule_for_test("minecraft:max_entity_cramming"),
            Some(bbb_protocol::game_rules::GameRuleValue::Integer(value)),
            "`/gamerule` writes the rule table the entity seam reads"
        );
    }
    let dimension = Java26BuiltinDimension::Overworld;
    let spot = world.publication().players[0].position;
    let cow = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (pusher, _) = world.spawn_stored_entity_for_test(
        dimension,
        cow,
        spot,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow).expect("a cow is a living entity"),
        ),
    );
    for _ in 0..24 {
        world.spawn_stored_entity_for_test(
            dimension,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            spot,
            crate::entity::EntityData::Minecart(crate::entity::minecart::MinecartData::new()),
        );
    }
    (sender, receiver, world, pusher)
}

fn cow_health(world: &ServerWorld, entity_id: i32) -> f32 {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the cow is stored")
        .living()
        .expect("a cow is living")
        .health()
}

/// How many ticks to give the charge, whose draw is `nextInt(4) == 0`
/// (`LivingEntity.java:3161`).
const CRAMMING_TICKS: usize = 40;

#[test]
fn a_crowd_in_the_pushers_box_charges_it_six_on_the_real_tick() {
    let (_sender, receiver, mut world, pusher) = crammed_world(None);
    let before = cow_health(&world, pusher);
    assert!(
        before > 6.0,
        "the fixture cow has more than one charge: {before}"
    );
    let mut charged = 0;
    let mut ticks = 0;
    while ticks < CRAMMING_TICKS && charged == 0 {
        world.tick(receiver.drain());
        ticks += 1;
        let after = cow_health(&world, pusher);
        if after != before {
            assert_eq!(
                before - after,
                6.0,
                "`hurtServer(serverLevel, damageSources().cramming(), 6.0F)` at tick {ticks}"
            );
            charged = ticks;
        }
    }
    assert_eq!(charged, ticks, "the crowd must charge the cow");
}

#[test]
fn a_rule_above_the_crowd_size_leaves_the_pusher_alone() {
    // The live rule, not the registered 24: 24 bodies are below 40.
    let (_sender, receiver, mut world, pusher) = crammed_world(Some(40));
    let before = cow_health(&world, pusher);
    for _ in 0..CRAMMING_TICKS {
        world.tick(receiver.drain());
        assert_eq!(cow_health(&world, pusher), before);
    }
    assert_eq!(
        world.game_rule_for_test("minecraft:max_entity_cramming"),
        Some(bbb_protocol::game_rules::GameRuleValue::Integer(40))
    );
}

#[test]
fn damage_exhaustion_requires_unabsorbed_damage_and_spares_invulnerable_abilities() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice minecraft:absorption 60 0",
    );
    assert_eq!(world.player_absorption_for_test(ConnectionId(1)), 4.0);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 3 minecraft:cactus",
    );
    assert_eq!(world.player_absorption_for_test(ConnectionId(1)), 1.0);
    assert_eq!(live_food(&mut world).exhaustion_level(), 0.0);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 5 minecraft:cactus",
    );
    assert_eq!(world.player_absorption_for_test(ConnectionId(1)), 0.0);
    assert_eq!(live_food(&mut world).exhaustion_level(), 0.1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 5 minecraft:cactus",
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        0.1,
        "rejected cooldown hit is free"
    );

    let mut abilities = world.player_abilities_for_test(ConnectionId(1));
    abilities.invulnerable = true;
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Survival, abilities);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 8 minecraft:cactus",
    );
    assert_eq!(live_food(&mut world).exhaustion_level(), 0.1);
    let bypass = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 6 minecraft:generic_kill",
    );
    assert!(
        !set_healths(&bypass).is_empty(),
        "bypass damage still lands"
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        0.1,
        "zero-exhaustion bypass damage remains free"
    );
}

#[test]
fn damage_exhaustion_reaches_the_food_cascade_and_health_packet() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule natural_health_regeneration false",
    );
    world.set_player_food_for_test(ConnectionId(1), 20, 0.5);
    let mut last = None;
    for hit in 1..=41 {
        last = Some(chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("damage alice {} minecraft:cactus", hit as f32 * 0.01),
        ));
    }
    let food = live_food(&mut world);
    assert_eq!(food.saturation_level(), 0.0);
    assert_eq!(food.food_level(), 20);
    assert!((food.exhaustion_level() - 0.1).abs() < 0.00001);
    assert!(last.unwrap().events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetHealth(health) }
        if health.food == 20 && health.saturation == 0.0
    )));
}

fn food_attack_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "tick freeze");
    (sender, receiver, world)
}

fn food_attack_target(world: &mut ServerWorld, type_id: i32) -> i32 {
    let position = world.publication().players[0].position;
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            type_id,
            Vec3d {
                z: position.z + 1.5,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(type_id).unwrap(),
            ),
        )
        .0
}

#[test]
fn stored_attack_exhaustion_charges_success_only_and_uses_abilities_gate() {
    let (sender, receiver, mut world) = food_attack_world();
    let cow = food_attack_target(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, cow)
        .unwrap()
        .set_invulnerable(true);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: cow }),
    );
    assert_eq!(live_food(&mut world).exhaustion_level(), 0.0);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, cow)
        .unwrap()
        .set_invulnerable(false);
    let health_before = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
        .unwrap()
        .living()
        .unwrap()
        .health();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: cow }),
    );
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < health_before
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        bbb_server_types::food::EXHAUSTION_ATTACK
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: i32::MAX,
        }),
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        bbb_server_types::food::EXHAUSTION_ATTACK
    );

    let other = food_attack_target(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    let health_before = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, other)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut abilities = world.player_abilities_for_test(ConnectionId(1));
    abilities.invulnerable = true;
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Survival, abilities);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: other }),
    );
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, other)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < health_before
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        bbb_server_types::food::EXHAUSTION_ATTACK
    );
}

#[test]
fn stored_attack_exhaustion_counts_creaking_absorb_but_not_its_refusal() {
    let (sender, receiver, mut world) = food_attack_world();
    let creaking = food_attack_target(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID,
    );
    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, creaking)
        .unwrap();
    entity.set_creaking_transient(BlockPos { x: 0, y: 64, z: 0 });
    let health_before = entity.living().unwrap().health();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: creaking,
        }),
    );
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, creaking)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        health_before
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        bbb_server_types::food::EXHAUSTION_ATTACK
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: creaking,
        }),
    );
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        bbb_server_types::food::EXHAUSTION_ATTACK,
        "hurt animation window refuses the next hit"
    );
}

/// Health is `20 - damage` in `f32`, so reading the damage back rounds.
fn assert_close(taken: f32, expected: f32) {
    assert!(
        (taken - expected).abs() < 1e-5,
        "took {taken}, expected {expected}"
    );
}

/// `CombatRules.getDamageAfterAbsorb` (`CombatRules.java:16-32`) without a
/// weapon.
fn after_armor(damage: f32, armor: f32, toughness: f32) -> f32 {
    let toughness = 2.0 + toughness / 4.0;
    let real_armor = (armor - damage / toughness).max(armor * 0.2).min(20.0);
    damage * (1.0 - real_armor / 25.0)
}

/// `Player.actuallyHurt` (`Player.java:748-751`) through armour, resistance
/// and protection, with `Player.hurtArmor` and `hurtHelmet` wearing the worn
/// pieces (`LivingEntity.java:1186-1188`, `:1855-1876`).
#[test]
fn worn_armor_resistance_and_protection_reduce_a_sessions_damage() {
    const FEET: usize = 36;
    const CHEST: usize = 38;
    const HEAD: usize = 39;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice = ConnectionId(1);
    use bbb_server_types::stats::custom_stat;
    let run = |world: &mut ServerWorld, command: &str| {
        chat_command(&sender, &receiver, world, 1, command);
        world.tick(receiver.drain());
    };
    // Each hit lands on full health outside the last one's window.
    let hit = |world: &mut ServerWorld, command: &str| {
        world.set_player_health_for_test(alice, 20.0);
        for _ in 0..11 {
            world.tick(Vec::new());
        }
        run(world, command);
        20.0 - world.player_health_for_test(alice)
    };
    let worn_damage = |world: &ServerWorld, slot| {
        world
            .player_inventory_slot_for_test(alice, slot)
            .component_patch
            .damage
    };

    run(
        &mut world,
        "item replace entity alice armor.chest with iron_chestplate",
    );
    let taken = hit(&mut world, "damage alice 10 minecraft:cactus");
    assert_close(taken, after_armor(10.0, 6.0, 0.0));
    assert_eq!(worn_damage(&world, CHEST), Some(2), "(int) max(1, 10 / 4)");

    // Resistance II takes 10/25 of what the armour let through.
    run(&mut world, "effect give alice minecraft:resistance 60 1");
    let taken = hit(&mut world, "damage alice 10 minecraft:cactus");
    let armored = after_armor(10.0, 6.0, 0.0);
    let resisted = (armored * 15.0 / 25.0).max(0.0);
    assert_close(taken, resisted);
    assert_eq!(
        world.player_stat_for_test(alice, custom_stat("minecraft:damage_resisted")),
        ((armored - resisted) * 10.0).round() as i32
    );
    run(&mut world, "effect clear alice");

    // Protection IV is four points of `damage_protection`.
    run(
        &mut world,
        "item replace entity alice armor.chest with iron_chestplate[enchantments={protection:4}]",
    );
    let taken = hit(&mut world, "damage alice 10 minecraft:cactus");
    assert_close(taken, after_armor(10.0, 6.0, 0.0) * (1.0 - 4.0 / 25.0));

    // `bypasses_armor` skips both the absorption and the wear.
    run(
        &mut world,
        "item replace entity alice armor.chest with iron_chestplate",
    );
    let taken = hit(&mut world, "damage alice 10 minecraft:starve");
    assert_close(taken, 10.0);
    assert_eq!(worn_damage(&world, CHEST), None);

    // A falling anvil wears the helmet, takes a quarter off, then the
    // armour pass wears every piece by the reduced amount.
    run(
        &mut world,
        "item replace entity alice armor.head with iron_helmet",
    );
    let taken = hit(&mut world, "damage alice 8 minecraft:falling_anvil");
    assert_close(taken, after_armor(6.0, 8.0, 0.0));
    assert_eq!(
        worn_damage(&world, HEAD),
        Some(3),
        "2 for the helmet, 1 for the armour"
    );
    assert_eq!(worn_damage(&world, CHEST), Some(1));

    // Frost walker's `damage_immunity` refuses `burn_from_stepping` sources.
    run(
        &mut world,
        "item replace entity alice armor.feet with diamond_boots[enchantments={frost_walker:2}]",
    );
    let taken = hit(&mut world, "damage alice 5 minecraft:hot_floor");
    assert_close(taken, 0.0);
    assert_eq!(worn_damage(&world, FEET), None);
}

/// `InfestedMobEffect.onMobHurt` (`InfestedMobEffect.java:28-36`) on a
/// session: a one-in-ten roll per successful hurt spawns silverfish.
#[test]
fn an_infested_session_releases_silverfish_when_hurt() {
    fn answer_keep_alives(sender: &WorldCommandSender, tick: TickResult) {
        for event in tick.events {
            if let WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                sender
                    .try_send(WorldCommand::Play {
                        connection_id,
                        packet: PlayServerbound::KeepAlive { id },
                    })
                    .unwrap();
            }
        }
    }
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice minecraft:infested 600",
    );
    world.tick(receiver.drain());
    let silverfish = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                    .is_some_and(|entity| {
                        entity.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SILVERFISH_ID
                    })
            })
            .count()
    };
    let mut hits = 0;
    while silverfish(&world) == 0 {
        assert!(hits < 200, "a 10% roll failed 200 times");
        hits += 1;
        world.set_player_health_for_test(ConnectionId(1), 20.0);
        for _ in 0..11 {
            answer_keep_alives(&sender, world.tick(receiver.drain()));
        }
        let hurt = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "damage alice 1 minecraft:generic",
        );
        answer_keep_alives(&sender, hurt);
        answer_keep_alives(&sender, world.tick(receiver.drain()));
    }
    assert!((1..=2).contains(&silverfish(&world)));
}

/// `Player.getDisplayName` (Player.java:1676-1686) formats the name for its
/// team, and both `CombatTracker.getDeathMessage` and the join and leave
/// announcements name the player by it.
#[test]
fn death_messages_and_presence_announcements_carry_the_team_prefix() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());
    for command in [
        "team add red",
        "team modify red prefix \"[R] \"",
        "team join red bob",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:generic by alice",
    );
    let deaths: Vec<_> = system_chats(&killed)
        .into_iter()
        .filter(|(_, line)| line.contains("death.attack.generic"))
        .collect();
    assert_eq!(deaths.len(), 2, "{deaths:?}");
    assert!(
        deaths.iter().all(|(_, line)| line.contains("[R] ")),
        "{deaths:?}"
    );

    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(2),
        },
    )
    .unwrap();
    let left = world.tick(receiver.drain());
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 3, "bob", 2);
    let heard: Vec<_> = [left, joined, handshake]
        .iter()
        .flat_map(system_chats)
        .filter(|(id, line)| *id == ConnectionId(1) && line.contains("multiplayer.player"))
        .map(|(_, line)| line)
        .collect();
    assert_eq!(heard.len(), 2, "{heard:?}");
    assert!(heard[0].contains("multiplayer.player.left") && heard[0].contains("[R] "));
    assert!(heard[1].contains("multiplayer.player.joined") && heard[1].contains("[R] "));
}

/// `handleAttack` drops a target outside the level's live border
/// (`ServerGamePacketListenerImpl.java:1790`).
#[test]
fn an_attack_on_a_target_outside_the_live_border_does_nothing() {
    let (sender, receiver, mut world) = food_attack_world();
    let cow = food_attack_target(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    let health = |world: &ServerWorld| {
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
            .unwrap()
            .living()
            .unwrap()
            .health()
    };
    let health_before = health(&world);
    // The player stays inside; the cow 1.5 blocks away is out.
    let position = world.publication().players[0].position;
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("worldborder center {} {}", position.x, position.z),
    );
    chat_command(&sender, &receiver, &mut world, 1, "worldborder set 2");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: cow }),
    );
    assert_eq!(health(&world), health_before);
    chat_command(&sender, &receiver, &mut world, 1, "worldborder set 1000");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: cow }),
    );
    assert!(health(&world) < health_before);
}

/// `resolvePlayerResponsibleForDamage` (`LivingEntity.java:1339-1346`): a
/// tame wolf's bite credits its owner, so a later sourceless death names
/// the owner and counts as a player kill on both sides.
#[test]
fn a_tame_wolfs_bite_credits_its_owner() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::{custom_stat, StatKey};

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick(receiver.drain());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:wolf 2 64 0 {NoAI:1b,Owner:[I;0,0,0,2]}",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 1 minecraft:mob_attack by @e[type=minecraft:wolf,limit=1]",
    );
    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic",
    );
    let message = combat_kill_message(&killed, ConnectionId(1)).expect("alice died");
    assert!(message.contains("death.attack.generic.player"), "{message}");
    assert!(message.contains("bob"), "{message}");
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID
            )
        ),
        1
    );
    assert_eq!(
        world.player_stat_for_test(ConnectionId(2), custom_stat("minecraft:player_kills")),
        1
    );
}

/// `AbstractArrow.canHitEntity` (`AbstractArrow.java:593-596`): a player's
/// arrow flies through a player its owner cannot harm, whether the `pvp`
/// rule is off or both share a team without friendly fire.
#[test]
fn a_players_arrow_flies_through_a_player_its_owner_cannot_harm() {
    for setup in [
        &[][..],
        &["gamerule pvp false"][..],
        &[
            "team add allies",
            "team modify allies friendlyFire false",
            "team join allies alice",
            "team join allies bob",
        ][..],
    ] {
        let (sender, receiver) = mailbox();
        let mut world =
            ServerWorld::new(survival_operator_config(&[("alice", 1), ("bob", 2)])).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
        settle_joined_bodies(&mut world);
        for command in setup {
            chat_command(&sender, &receiver, &mut world, 1, command);
        }
        let feet = 200.0;
        for (connection, x) in [(1, 20.5), (2, 0.5)] {
            world.set_player_position_for_test(
                ConnectionId(connection),
                Vec3d { x, y: feet, z: 0.5 },
            );
        }
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "summon arrow 0.5 202.5 0.5 {UUID:[I;0,0,0,90],Owner:[I;0,0,0,1],Motion:[0.0,-2.0,0.0]}",
        );
        let arrow_y = |world: &ServerWorld| {
            let id = world
                .live_entity_id_for_uuid_for_test(
                    Java26BuiltinDimension::Overworld,
                    Uuid::from_u128(90),
                )
                .unwrap();
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .position()
                .y
        };
        for _ in 0..4 {
            world.tick(Vec::new());
        }
        let hurt = world.player_health_for_test(ConnectionId(2)) < 20.0;
        if setup.is_empty() {
            assert!(hurt, "a harmable player is hit");
        } else {
            assert!(!hurt, "{setup:?}");
            assert!(
                arrow_y(&world) < feet + 1.0,
                "{setup:?}: the arrow passed through"
            );
        }
    }
}

#[test]
fn player_combat_enter_and_end_packets_follow_living_attacker_timeout_and_death() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: datapack_config().datapacks,
        ..survival_operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    settle_joined_bodies(&mut world);

    // Environmental damage does not enter combat (`CombatTracker.shouldEnterCombat`).
    let fall = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 2 minecraft:fall",
    );
    assert_eq!(set_healths(&fall), [(ConnectionId(2), 18.0)], "{fall:?}");
    assert!(
        !fall.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerCombatEnter
                    | ClientboundPlayPacket::PlayerCombatEnd(_),
            }
        )),
        "{fall:?}"
    );
    for _ in 0..20 {
        world.tick(receiver.drain());
    }

    // Living attacker damage enters combat (`ServerPlayer.onEnterCombat`).
    let entered = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 2 minecraft:player_attack by alice",
    );
    assert!(
        entered.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerCombatEnter,
            }
        )),
        "{entered:?}"
    );

    // Expiring >300 ticks leaves combat on the 20-tick recheck (`ServerPlayer.onLeaveCombat`).
    let mut left_duration = None;
    for _ in 0..325 {
        let tick = world.tick(receiver.drain());
        for event in tick.events {
            if let WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerCombatEnd(end),
            } = event
            {
                left_duration = Some(end.duration);
            }
        }
        if left_duration.is_some() {
            break;
        }
    }
    assert!(
        left_duration.is_some_and(|duration| duration > 300),
        "expected combat timeout duration > 300, got {left_duration:?}"
    );

    // Lethal living-attacker damage enters combat and immediately ends combat on death.
    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:player_attack by alice",
    );
    let combat_packets = killed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerCombatEnter,
            } => Some("enter"),
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::PlayerCombatEnd(_),
            } => Some("end"),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(combat_packets, ["enter", "end"]);
}
