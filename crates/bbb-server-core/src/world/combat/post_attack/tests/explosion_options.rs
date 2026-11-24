use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::WorldEvent;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::Vec3Ops;

fn stationary(world: &mut ServerWorld, entity: i32) {
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(OVERWORLD, entity)
        .unwrap()
    else {
        unreachable!()
    };
    entries.retain(|entry| entry.name != "NoAI" && entry.name != "NoGravity");
    entries.extend([
        bbb_protocol::NbtSummaryEntry {
            name: "NoAI".into(),
            value: bbb_protocol::NbtSummaryValue::Byte(1),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "NoGravity".into(),
            value: bbb_protocol::NbtSummaryValue::Byte(1),
        },
    ]);
    world.load_stored_entity_record_for_test(OVERWORLD, entity, &entries);
}

#[test]
fn offset_explosions_with_default_knockback_reach_player_and_stored_victims() {
    for player_victim in [true, false] {
        let (mut world, root) = world_with_post_attack_effects(
            "attacker",
            "victim",
            serde_json::json!([{
                "type": "minecraft:explode",
                "offset": [1.25, 1.5, -0.75],
                "radius": 3.5,
                "block_interaction": "trigger",
                "immune_blocks": "#minecraft:blocks_wind_charge_explosions",
                "small_particle": {"type": "minecraft:gust_emitter_small"},
                "large_particle": {"type": "minecraft:gust_emitter_large"},
                "sound": "minecraft:entity.wind_charge.wind_burst"
            }]),
        );
        let position = world.levels[PLAYER].position();
        let zombie = armed_zombie(
            &mut world,
            Vec3d {
                x: position.x - 3.0,
                ..position
            },
        );
        let origin = if player_victim {
            position
        } else {
            Vec3d {
                y: position.y + 8.0,
                ..position
            }
        };
        let center = origin.plus(Vec3d {
            x: 1.25,
            y: 1.5,
            z: -0.75,
        });
        let nearby = spawn(
            &mut world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: center.x + 1.0,
                ..center
            },
        );
        let victim = if player_victim {
            None
        } else {
            Some(spawn(
                &mut world,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                origin,
            ))
        };
        for entity in [Some(zombie), Some(nearby), victim].into_iter().flatten() {
            stationary(&mut world, entity);
        }
        let attacker = PendingHurtEntity::Stored { entity_id: zombie };
        if let Some(victim_entity_id) = victim {
            world
                .level_mut(OVERWORLD)
                .event_sink
                .post_attack(PendingPostAttack {
                    victim_entity_id,
                    damage_type: "minecraft:mob_attack",
                    causing: Some(attacker.clone()),
                    direct: Some(attacker),
                });
        } else {
            let player = world.levels[PLAYER].profile.uuid;
            world
                .level_mut(OVERWORLD)
                .event_sink
                .hurt_player(PendingPlayerHurt {
                    player,
                    damage_type: "minecraft:mob_attack",
                    amount: 2.0,
                    causing: Some(attacker.clone()),
                    direct: Some(attacker),
                    post_attack: true,
                    impulse: None,
                });
        }
        let result = world.tick(Vec::new());
        let bursts: Vec<_> = result
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: PLAYER,
                    packet: ClientboundPlayPacket::Explode(update),
                } => Some(update),
                _ => None,
            })
            .collect();
        assert_eq!(bursts.len(), 1, "{bursts:?}");
        assert_eq!(bursts[0].center, center);
        let body = world.stored_entity_for_test(OVERWORLD, nearby).unwrap();
        assert_eq!(body.living().unwrap().health(), 10.0);
        let impulse = body.delta_movement();
        assert!(impulse.x.is_finite() && impulse.y.is_finite() && impulse.z.is_finite());
        assert!(impulse.x > 0.0 && impulse.x <= 1.0, "{impulse:?}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn an_attributed_players_explosion_block_loot_reads_the_live_selected_item() {
    use bbb_protocol::packets::{AttackEntity, PlayServerbound};
    use bbb_protocol::NbtSummaryValue;

    let table = serde_json::json!({
        "type":"minecraft:block",
        "pools":[
            {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:player",
                    "nbt":"{SelectedItem:{id:'minecraft:iron_sword',count:1}}"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:player",
                    "nbt":"{SelectedItem:{id:'minecraft:iron_sword',count:2}}"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
        ]
    })
    .to_string();
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{
            "enchanted":"attacker", "affected":"attacker",
            "effect":{
                "type":"minecraft:explode", "attribute_to_user":true,
                "offset":[3,1.5,0], "radius":1, "block_interaction":"block",
                "small_particle":{"type":"minecraft:explosion"},
                "large_particle":{"type":"minecraft:explosion_emitter"},
                "sound":"minecraft:entity.generic.explode"
            }
        }]}),
        &[("data/minecraft/loot_table/blocks/white_wool.json", &table)],
    );
    world.ensure_entity_transfer_full_chunk_for_test(OVERWORLD, (0, 0));
    let origin = Vec3d {
        x: 8.5,
        y: 300.0,
        z: 8.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(origin);
    let weapon = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = weapon;
    let player_uuid = world.levels[PLAYER].profile.uuid;
    let NbtSummaryValue::Compound(player_data) = world.player_loot_nbt(player_uuid).unwrap() else {
        panic!("the actual player has canonical NBT");
    };
    let Some(NbtSummaryValue::Compound(selected)) = player_data
        .iter()
        .find(|entry| entry.name == "SelectedItem")
        .map(|entry| &entry.value)
    else {
        panic!("the actual main hand is saved as SelectedItem");
    };
    for (name, value) in [
        ("id", NbtSummaryValue::String("minecraft:iron_sword".into())),
        ("count", NbtSummaryValue::Int(1)),
    ] {
        assert!(selected
            .iter()
            .any(|entry| entry.name == name && entry.value == value));
    }
    let block = BlockPos {
        x: 11,
        y: 301,
        z: 8,
    };
    let wool = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:white_wool",
        &std::collections::BTreeMap::new(),
    )
    .unwrap();
    assert!(world.set_block(OVERWORLD, block, wool, crate::fluid::SET_BLOCK_UPDATE_ALL));
    assert_eq!(world.block_state(OVERWORLD, block), Some(wool));
    assert!(world.stored_items_for_test(OVERWORLD).is_empty());
    let victim = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d { x: 7.0, ..origin },
    );
    stationary(&mut world, victim);
    let initial_health = world
        .stored_entity_for_test(OVERWORLD, victim)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let result = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
        world.last_command_sequence + 1,
        crate::mailbox::WorldProducerClass::Network,
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::Attack(AttackEntity { entity_id: victim }),
        },
    )]);
    let health = world
        .stored_entity_for_test(OVERWORLD, victim)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(health > 0.0 && health < initial_health);
    let bursts = result
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: PLAYER,
                packet: ClientboundPlayPacket::Explode(explosion),
            } => Some(explosion),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(bursts.len(), 1);
    assert_eq!(
        bursts[0].center,
        origin.plus(Vec3d {
            x: 3.0,
            y: 1.5,
            z: 0.0,
        })
    );
    assert!(bbb_server_types::fluid::block_state_is_air(
        world.block_state(OVERWORLD, block).unwrap()
    ));
    let mut drops = world
        .stored_items_for_test(OVERWORLD)
        .into_iter()
        .map(|(_, _, id, count, ..)| (id, count))
        .collect::<Vec<_>>();
    drops.sort();
    assert_eq!(
        drops,
        vec![
            ("minecraft:apple".to_owned(), 1),
            ("minecraft:diamond".to_owned(), 1),
        ]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_wind_launched_players_fall_reads_the_explosion_sources_item_after_disconnect() {
    use crate::entity::hurting_projectile::wind_charge::WindChargeData;
    use crate::entity::hurting_projectile::HurtingProjectileState;
    use crate::entity::EntityData;
    use crate::world::WorldCommand;
    use bbb_protocol::packets::{AttackEntity, GameProfile, MovePlayer, PlayServerbound};
    use bbb_protocol::NbtSummaryValue;
    use uuid::Uuid;

    const VICTIM: ConnectionId = ConnectionId(2);
    const START: Vec3d = Vec3d {
        x: 8.5,
        y: 100.0,
        z: 8.5,
    };
    let advancement = |count| {
        serde_json::json!({
        "criteria":{"flight":{"trigger":"minecraft:fall_after_explosion","conditions":{
            "player":{"type":"minecraft:player"},
            "start_position":{"position":{"y":{"min":99,"max":101}}},
            "distance":{"y":{"min":7}},
            "cause":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:player",
                    "nbt":format!("{{SelectedItem:{{id:'minecraft:iron_sword',count:{count}}}}}")}}]
        }}},
        "rewards":{"experience":0}
    }).to_string()
    };
    let positive = advancement(1);
    let negative = advancement(2);
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{
            "enchanted":"attacker", "affected":"attacker",
            "effect":{
                "type":"minecraft:explode", "attribute_to_user":true,
                "offset":[5,0.5,0], "radius":2, "block_interaction":"trigger",
                "small_particle":{"type":"minecraft:explosion"},
                "large_particle":{"type":"minecraft:explosion_emitter"},
                "sound":"minecraft:entity.generic.explode"
            }
        }]}),
        &[
            ("data/test/advancement/departed_source.json", &positive),
            (
                "data/test/advancement/wrong_departed_item_count.json",
                &negative,
            ),
        ],
    );
    let run = |world: &mut ServerWorld, command: WorldCommand| {
        let sequence = world.last_command_sequence + 1;
        world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
            sequence,
            crate::mailbox::WorldProducerClass::Network,
            command,
        )])
    };
    let joined = run(
        &mut world,
        WorldCommand::Join {
            connection_id: VICTIM,
            profile: GameProfile {
                uuid: Uuid::from_u128(2),
                name: "victim".into(),
                properties: Vec::new(),
            },
            client_information: Default::default(),
            context: Default::default(),
        },
    );
    let teleport = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: VICTIM,
                packet: ClientboundPlayPacket::PlayerPosition(position),
            } => Some(position.id),
            _ => None,
        })
        .expect("the victim joins through the spawn teleport");
    for packet in [
        PlayServerbound::AcceptTeleportation { id: teleport },
        PlayServerbound::PlayerLoaded,
    ] {
        run(
            &mut world,
            WorldCommand::Play {
                connection_id: VICTIM,
                packet,
            },
        );
    }
    let source_uuid = world.levels[PLAYER].profile.uuid;
    let source_id = world.levels[PLAYER].entity_id();
    let dimension = world.levels[PLAYER].dimension;
    let victim_uuid = world.levels[VICTIM].profile.uuid;
    assert_ne!(source_uuid, victim_uuid);
    assert_ne!(source_id, world.levels[VICTIM].entity_id());
    assert_eq!(world.levels[VICTIM].dimension, dimension);
    for connection in [PLAYER, VICTIM] {
        assert_eq!(
            world.levels[connection].game_type,
            bbb_protocol::game_types::GameType::Survival
        );
    }
    world.ensure_entity_transfer_full_chunk_for_test(OVERWORLD, (0, 0));
    let state = |name| {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
                .unwrap()
                .id,
        )
        .unwrap()
    };
    let air = state("minecraft:air");
    let stone = state("minecraft:stone");
    for x in 0..=10 {
        for z in 7..=9 {
            for y in 100..=112 {
                world.set_raw_block_state_for_test(OVERWORLD, BlockPos { x, y, z }, air);
            }
        }
    }
    for x in 7..=9 {
        for z in 7..=9 {
            world.set_raw_block_state_for_test(OVERWORLD, BlockPos { x, y: 99, z }, stone);
        }
    }
    for x in 0..=3 {
        for z in 7..=9 {
            world.set_raw_block_state_for_test(OVERWORLD, BlockPos { x, y: 108, z }, stone);
        }
    }
    let source_position = Vec3d {
        x: 2.5,
        y: 109.0,
        ..START
    };
    world.set_player_position_for_test(PLAYER, source_position);
    world.set_player_position_for_test(VICTIM, START);
    let weapon = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = weapon;
    let assert_selected_item = |world: &ServerWorld| {
        let NbtSummaryValue::Compound(saved) = world.player_loot_nbt(source_uuid).unwrap() else {
            panic!("the source has canonical player data");
        };
        let Some(NbtSummaryValue::Compound(selected)) = saved
            .iter()
            .find(|field| field.name == "SelectedItem")
            .map(|field| &field.value)
        else {
            panic!("the source keeps its actual selected sword");
        };
        for (name, value) in [
            ("id", NbtSummaryValue::String("minecraft:iron_sword".into())),
            ("count", NbtSummaryValue::Int(1)),
        ] {
            assert!(selected
                .iter()
                .any(|field| field.name == name && field.value == value));
        }
    };
    assert_selected_item(&world);
    let initial_xp = world.levels[VICTIM].experience.total();
    let assert_context = |world: &ServerWorld, id: i32, uuid: Uuid, resource: &str| {
        let player = &world.levels[VICTIM];
        assert_eq!(player.profile.uuid, victim_uuid);
        assert!(player.is_alive());
        assert!(!player.abilities.flying);
        assert_eq!(player.cold.fall_impulse.impact_position, Some(START));
        assert!(player.block_contact.starting_to_fall_position.is_none());
        assert_eq!(player.block_contact.fall_distance, 0.0);
        let (cause, _) = player.cold.impulse_explosion_cause.as_ref().unwrap();
        assert_eq!(cause.entity_id, id);
        assert_eq!(cause.uuid, uuid);
        assert_eq!(cause.entity_resource_id, resource);
    };
    let assert_pending = |world: &ServerWorld| {
        let saved = world.player_advancements_to_data(VICTIM);
        for id in ["test:departed_source", "test:wrong_departed_item_count"] {
            assert!(!world.player_advancement_is_done(VICTIM, id), "{id}");
            assert!(
                saved.progress.get(id).is_none_or(|progress| progress
                    .criteria()
                    .iter()
                    .all(|(_, criterion)| !criterion.is_done())),
                "{id}"
            );
        }
    };
    let (charge, charge_uuid) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        Vec3d {
            y: START.y + 0.5,
            ..START
        },
        EntityData::WindCharge(WindChargeData::new(
            HurtingProjectileState::new(Some(victim_uuid), 0.0),
            0,
        )),
    );
    world
        .stored_entity_mut_for_test(OVERWORLD, charge)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        });
    let launched = world.tick(Vec::new());
    assert!(world.stored_entity_for_test(OVERWORLD, charge).is_none());
    assert!(world
        .level(OVERWORLD)
        .entities
        .entity_id_for_uuid(charge_uuid)
        .is_none());
    assert!(launched.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: VICTIM,
            packet: ClientboundPlayPacket::Explode(_)
        }
    )));
    assert_context(&world, charge, charge_uuid, "minecraft:wind_charge");
    let move_victim = |world: &mut ServerWorld, y| {
        let position = Vec3d { y, ..START };
        let result = run(
            world,
            WorldCommand::Play {
                connection_id: VICTIM,
                packet: PlayServerbound::MovePlayer(MovePlayer {
                    position: Some(position),
                    rotation: None,
                    on_ground: false,
                    horizontal_collision: false,
                }),
            },
        );
        assert!(!result.events.iter().any(|event| matches!(
            event,
            WorldEvent::Close { .. }
                | WorldEvent::Send {
                    connection_id: VICTIM,
                    packet: ClientboundPlayPacket::PlayerPosition(_)
                }
        )));
        assert_eq!(world.levels[VICTIM].position(), position);
        result
    };
    for y in 1..=9 {
        move_victim(&mut world, START.y + f64::from(y));
    }
    assert_context(&world, charge, charge_uuid, "minecraft:wind_charge");
    assert_pending(&world);
    let cow = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 1.0,
            ..source_position
        },
    );
    stationary(&mut world, cow);
    let initial_health = world
        .stored_entity_for_test(OVERWORLD, cow)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let attacked = run(
        &mut world,
        WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::Attack(AttackEntity { entity_id: cow }),
        },
    );
    let health = world
        .stored_entity_for_test(OVERWORLD, cow)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(health > 0.0 && health < initial_health);
    let center = source_position.plus(Vec3d {
        x: 5.0,
        y: 0.5,
        z: 0.0,
    });
    assert!(attacked.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: VICTIM, packet: ClientboundPlayPacket::Explode(explosion) }
            if explosion.center == center && explosion.radius == 2.0
    )));
    assert_context(&world, source_id, source_uuid, "minecraft:player");
    assert_selected_item(&world);
    assert_pending(&world);
    run(
        &mut world,
        WorldCommand::Disconnect {
            connection_id: PLAYER,
        },
    );
    assert!(world.levels.player(PLAYER).is_none());
    assert!(!world.sessions.contains_key(&PLAYER));
    assert!(world.player_reference(source_uuid).is_none());
    assert!(world.entity_reference(&dimension, source_uuid).is_none());
    assert!(world
        .level(OVERWORLD)
        .entities
        .player_by_uuid(source_uuid)
        .is_none());
    assert!(world.level(OVERWORLD).entities.player(source_id).is_none());
    assert!(world.sessions.contains_key(&VICTIM));
    assert_context(&world, source_id, source_uuid, "minecraft:player");
    assert_pending(&world);
    let descended = move_victim(&mut world, START.y + 8.5);
    let saved = world.player_advancements_to_data(VICTIM);
    for (id, expected) in [
        ("test:departed_source", true),
        ("test:wrong_departed_item_count", false),
    ] {
        assert_eq!(
            world.player_advancement_is_done(VICTIM, id),
            expected,
            "{id}"
        );
        assert_eq!(descended.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: VICTIM, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
                if update.progress.iter().any(|progress| progress.id == id
                    && progress.criteria.iter().any(|criterion| criterion.name == "flight"
                        && criterion.obtained_epoch_millis.is_some()))
        )), expected, "{id}");
        assert_eq!(
            saved.progress.get(id).is_some_and(|progress| progress
                .criteria()
                .iter()
                .any(|(name, criterion)| name == "flight" && criterion.is_done())),
            expected,
            "{id}"
        );
    }
    assert_eq!(world.levels[VICTIM].experience.total(), initial_xp);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_explosions_keep_the_affected_source_damage_type_and_complete_presentation() {
    use bbb_protocol::packets::{ParticleOptions, SoundEventHolder};
    for attributed in [false, true] {
        let config = crate::tests::datapack_config();
        let damage_type = std::str::from_utf8(
            &config
                .datapacks
                .as_ref()
                .unwrap()
                .registries
                .element("minecraft:damage_type", "minecraft:magic")
                .unwrap()
                .source_json,
        )
        .unwrap();
        let (mut world, root) = world_with_effect_resources(
            serde_json::json!({"minecraft:post_attack":[{
                "enchanted":"attacker", "affected":"victim",
                "effect": {
                    "type":"minecraft:explode", "attribute_to_user":attributed,
                    "damage_type":"test:impact", "offset":[1.25,1.0,-0.75],
                    "radius":1.5, "block_interaction":"none", "immune_blocks":[],
                    "small_particle":{"type":"minecraft:dust", "color":16711680, "scale":0.75},
                    "large_particle":{"type":"minecraft:explosion_emitter"},
                    "block_particles":[{
                        "particle":{"type":"minecraft:block", "block_state":{"Name":"minecraft:gold_block"}},
                        "scaling":2.5, "speed":0.25, "weight":3
                    }],
                    "sound":{"sound_id":"test:blast", "range":7.25}
                }
            }]}),
            &[("data/test/damage_type/impact.json", damage_type)],
        );
        let origin = world.levels[PLAYER].position();
        let center = origin.plus(Vec3d {
            x: 1.25,
            y: 1.0,
            z: -0.75,
        });
        world
            .levels
            .player_mut(PLAYER)
            .unwrap()
            .set_position(Vec3d {
                x: center.x + 2.0,
                ..center
            });
        let victim = spawn(
            &mut world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            origin,
        );
        stationary(&mut world, victim);
        let zombie = armed_zombie(
            &mut world,
            Vec3d {
                x: origin.x - 6.0,
                ..origin
            },
        );
        let attacker = PendingHurtEntity::Stored { entity_id: zombie };
        world
            .level_mut(OVERWORLD)
            .event_sink
            .post_attack(PendingPostAttack {
                victim_entity_id: victim,
                damage_type: "minecraft:mob_attack",
                causing: Some(attacker.clone()),
                direct: Some(attacker),
            });
        let player_id = world.levels[PLAYER].entity_id();
        let type_id = world
            .cold
            .damage_types
            .iter()
            .find(|kind| kind.resource_id == "test:impact")
            .unwrap()
            .registry_id;
        let result = world.tick(Vec::new());
        let damage = result
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: PLAYER,
                    packet: ClientboundPlayPacket::DamageEvent(damage),
                } if damage.entity_id == player_id && damage.source_type_id == type_id => {
                    Some(damage)
                }
                _ => None,
            })
            .expect("custom damage reaches the nearby player");
        assert!(world.levels[PLAYER].damage.health < 20.0);
        assert_eq!(damage.source_cause_id, if attributed { victim } else { -1 });
        assert_eq!(
            damage.source_direct_id,
            if attributed { victim } else { -1 }
        );
        assert_eq!(damage.source_position, (!attributed).then_some(center));
        if attributed {
            assert_eq!(
                world
                    .stored_entity_for_test(OVERWORLD, victim)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health(),
                10.0
            );
        }
        let packet = result
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: PLAYER,
                    packet: ClientboundPlayPacket::Explode(packet),
                } => Some(packet),
                _ => None,
            })
            .expect("custom explosion reaches its player");
        assert_eq!(packet.center, center);
        assert_eq!(packet.radius, 1.5);
        assert_eq!(packet.block_count, 0);
        assert_eq!(
            packet.explosion_sound,
            SoundEventHolder::Direct {
                location: "test:blast".into(),
                fixed_range: Some(7.25),
            }
        );
        assert_eq!(
            packet.explosion_particle,
            ParticleOptions::Dust {
                color: 0xff0000,
                scale: 0.75
            }
            .payload(bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DUST_ID)
            .unwrap()
        );
        let gold =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:gold_block")
                .unwrap();
        assert_eq!(
            packet.block_particles,
            vec![bbb_protocol::packets::WeightedExplosionParticle {
                particle: ParticleOptions::Block {
                    state_id: bbb_protocol::block_state_types::vanilla_block_default_state_id(
                        gold.id
                    )
                    .unwrap()
                }
                .payload(bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID)
                .unwrap(),
                scaling: 2.5,
                speed: 0.25,
                weight: 3,
            }]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn reloaded_immune_block_tags_change_the_actual_enchantment_explosion() {
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{
            "enchanted":"attacker", "affected":"victim",
            "effect":{
                "type":"minecraft:explode", "offset":[0,-1,0], "radius":1,
                "block_interaction":"tnt", "immune_blocks":"#test:shield",
                "small_particle":{"type":"minecraft:explosion"},
                "large_particle":{"type":"minecraft:explosion_emitter"},
                "sound":"minecraft:entity.generic.explode"
            }
        }]}),
        &[(
            "data/test/tags/block/shield.json",
            r#"{"values":["minecraft:gold_block"]}"#,
        )],
    );
    assert!(world.set_game_rule_for_test(
        "minecraft:tnt_explodes",
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    ));
    let origin = world.levels[PLAYER].position();
    let block = BlockPos {
        x: origin.x.floor() as i32,
        y: origin.y.floor() as i32,
        z: origin.z.floor() as i32,
    };
    let gold =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:gold_block")
            .unwrap();
    let gold_state =
        bbb_protocol::block_state_types::vanilla_block_default_state_id(gold.id).unwrap();
    world
        .level_mut(OVERWORLD)
        .chunks
        .set_block_state(block.x, block.y, block.z, gold_state)
        .unwrap();
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: origin.x + 20.0,
            ..origin
        });
    let victim = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            y: origin.y + 1.0,
            ..origin
        },
    );
    stationary(&mut world, victim);
    let zombie = armed_zombie(
        &mut world,
        Vec3d {
            x: origin.x - 6.0,
            ..origin
        },
    );
    for reloaded in [false, true] {
        if reloaded {
            std::fs::write(
                root.join("order/data/test/tags/block/shield.json"),
                r#"{"replace":true,"values":[]}"#,
            )
            .unwrap();
            let startup = bbb_server_datapack::startup::DatapackStartup::load(
                &bbb_server_datapack::startup::DatapackStartupOptions {
                    datapacks_dir: Some(root.clone()),
                    saved_data_packs: None,
                    safe_mode: false,
                    limits: Default::default(),
                },
            )
            .unwrap();
            world.install_datapacks(
                crate::world::ReloadedDatapacks::Built(std::sync::Arc::new(startup)),
                &mut Vec::new(),
            );
        }
        let attacker = PendingHurtEntity::Stored { entity_id: zombie };
        world
            .level_mut(OVERWORLD)
            .event_sink
            .post_attack(PendingPostAttack {
                victim_entity_id: victim,
                damage_type: "minecraft:mob_attack",
                causing: Some(attacker.clone()),
                direct: Some(attacker),
            });
        let result = world.tick(Vec::new());
        assert!(result.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: PLAYER,
                packet: ClientboundPlayPacket::Explode(_)
            }
        )));
        let actual = crate::world::stored_block_state(&world.level(OVERWORLD).chunks, block);
        if reloaded {
            assert!(bbb_server_types::fluid::block_state_is_air(actual));
        } else {
            assert_eq!(actual, gold_state);
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_attributed_projectiles_undamaging_explosion_redirects_to_its_living_owner() {
    use crate::entity::{ArrowData, EntityData, HurtingProjectileState, LargeFireballData};
    let (mut world, root) = world_with_post_attack_effects(
        "attacker",
        "damaging_entity",
        serde_json::json!([{
            "type":"minecraft:explode", "attribute_to_user":true, "radius":3,
            "block_interaction":"none", "small_particle":{"type":"minecraft:explosion"},
            "large_particle":{"type":"minecraft:explosion_emitter"},
            "sound":"minecraft:entity.generic.explode"
        }]),
    );
    let origin = Vec3d {
        x: 8.5,
        y: 70.0,
        z: -4.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: origin.x - 5.0,
            ..origin
        });
    let owner = world.levels[PLAYER].profile.uuid;
    let weapon = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_BOW_ID,
        "minecraft:fire_aspect",
        1,
    ));
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = weapon.clone();
    let stack = |item_id| {
        crate::tests::native_stack(ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        })
    };
    let mut arrow = ArrowData::new(stack(bbb_protocol::item_types::VANILLA_ITEM_ARROW_ID));
    arrow.projectile.set_owner(Some(owner));
    arrow.set_weapon(Some(weapon));
    let projectile = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
            origin,
            EntityData::Arrow(arrow),
        )
        .0;
    let fireball = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
            Vec3d {
                x: origin.x + 1.5,
                ..origin
            },
            EntityData::LargeFireball(LargeFireballData::new(
                HurtingProjectileState::new(Some(uuid::Uuid::from_u128(911)), 0.0),
                stack(bbb_protocol::item_types::VANILLA_ITEM_FIRE_CHARGE_ID),
                1,
            )),
        )
        .0;
    let victim = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: origin.x + 4.0,
            ..origin
        },
    );
    stationary(&mut world, victim);
    world
        .level_mut(OVERWORLD)
        .event_sink
        .post_attack(PendingPostAttack {
            victim_entity_id: victim,
            damage_type: "minecraft:arrow",
            causing: Some(PendingHurtEntity::Player { uuid: owner }),
            direct: Some(PendingHurtEntity::Stored {
                entity_id: projectile,
            }),
        });
    let result = world.tick(Vec::new());
    assert!(result.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: PLAYER,
            packet: ClientboundPlayPacket::Explode(_)
        }
    )));
    let EntityData::LargeFireball(data) = world
        .stored_entity_for_test(OVERWORLD, fireball)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    std::fs::remove_dir_all(root).unwrap();
}
