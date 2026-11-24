//! The explosion mechanism end to end, without a client: a TNT block lit with
//! flint and steel becomes a primed TNT entity, whose fuse runs down into a
//! `ServerExplosion` that broadcasts `ClientboundExplodePacket`.

use super::*;

use crate::entity::hurting_projectile::fireball::LargeFireballData;
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::item::plain_stack;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    ExplosionSourceEntity, ExplosionSourceEntityKind, PendingExplosion,
};
use bbb_protocol::item_types::{VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_TNT_ID};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn give(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    item_id: i32,
) {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                ..ItemStackSummary::empty()
            },
        }),
    );
}

/// The `ClientboundExplodePacket`s one tick sent, in event order.
fn explode_packets(tick: &TickResult) -> Vec<bbb_protocol::packets::Explosion> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(update),
                ..
            } => Some(update.clone()),
            _ => None,
        })
        .collect()
}

fn tnt_add_entity(tick: &TickResult) -> Option<bbb_protocol::packets::AddEntity> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AddEntity(add),
            ..
        } if add.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID => {
            Some(add.clone())
        }
        _ => None,
    })
}

/// Sets up a lit-TNT scenario: a player standing beside a placed TNT block, the
/// block primed with flint and steel. Returns the priming tick.
fn prime_a_tnt(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> (BlockPos, TickResult) {
    join_loaded(sender, receiver, world, 1, "boomer", 1);
    place_player(
        sender,
        receiver,
        world,
        1,
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 6.5,
        },
        None,
    );

    let floor = BlockPos { x: 5, y: 63, z: 6 };
    let tnt_pos = BlockPos { x: 5, y: 64, z: 6 };

    // Place the TNT block, then swap to flint and steel and light it.
    give(sender, receiver, world, 1, VANILLA_ITEM_TNT_ID);
    right_click(sender, receiver, world, 1, floor, 1);
    give(sender, receiver, world, 1, VANILLA_ITEM_FLINT_AND_STEEL_ID);
    let lit = right_click(sender, receiver, world, 1, tnt_pos, 2);
    (tnt_pos, lit)
}

/// `TntBlock.useItemOn` with flint and steel primes the block: it is removed,
/// the fuse sound is broadcast, `PRIME_FUSE` is raised and a primed TNT entity
/// appears.
#[test]
fn flint_and_steel_lights_a_tnt_block_into_a_primed_entity() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let (tnt_pos, lit) = prime_a_tnt(&sender, &receiver, &mut world);

    // `:115`: the block is removed.
    assert_eq!(
        world.block_state(OVERWORLD, tnt_pos),
        Some(block_state_id("minecraft:air")),
        "lighting the tnt removes the block"
    );

    // `:88`: `SoundEvents.TNT_PRIMED` at `SoundSource.BLOCKS`.
    let primed_sounds: Vec<_> = sounds_sent(&lit)
        .into_iter()
        .filter(|sound| {
            sound.sound
                == bbb_protocol::packets::SoundEventHolder::Reference {
                    registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                        "minecraft:entity.tnt.primed",
                    )
                    .unwrap(),
                }
        })
        .collect();
    assert_eq!(primed_sounds.len(), 1, "the fuse sound is broadcast once");

    // `:89`: `GameEvent.PRIME_FUSE` at the block position.
    let (events, _) = world.game_events(OVERWORLD);
    assert!(
        events
            .iter()
            .any(|event| event.game_event.constant_name() == "PRIME_FUSE" && event.pos == tnt_pos),
        "priming raises PRIME_FUSE at the block"
    );

    // A primed TNT entity was added and tracked to the nearby session.
    assert!(
        tnt_add_entity(&lit).is_some(),
        "a primed tnt entity is spawned and tracked"
    );
}

#[test]
fn a_primed_tnt_fuse_runs_down_into_a_broadcast_explosion() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let (_, primed) = prime_a_tnt(&sender, &receiver, &mut world);
    tnt_add_entity(&primed).expect("the priming tick tracks TNT");

    let mut explosions = Vec::new();
    for _ in 0..100 {
        let tick = world.tick(receiver.drain());
        explosions.extend(explode_packets(&tick));
        if !explosions.is_empty() {
            break;
        }
    }

    assert_eq!(
        explosions.len(),
        1,
        "the fuse produces one explosion packet"
    );
    let explosion = &explosions[0];
    // `PrimedTnt.explosionPower` default (`PrimedTnt.java:35`).
    assert_eq!(explosion.radius, 4.0);
    assert!(explosion.block_count > 0);
    // `ServerExplosion.isSmall` is false for a radius-4 destroying explosion, so
    // the large particle is chosen (`ServerLevel.java:1221`).
    assert_eq!(
        explosion.explosion_particle.particle_type_id,
        VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID
    );
    assert_eq!(
        explosion.explosion_sound,
        bbb_protocol::packets::SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.generic.explode",
            )
            .unwrap(),
        }
    );
}

fn minecart_tnt_explosion(center: Vec3d, primed: bool) -> PendingExplosion {
    PendingExplosion {
        source: Some(ExplosionSource {
            direct: ExplosionSourceEntity {
                custom_name: None,
                entity_id: 73,
                entity_resource_id: "minecraft:tnt_minecart".to_string(),
                uuid: Uuid::from_u128(73),
                position: center,
                kind: ExplosionSourceEntityKind::Other,
            },
            indirect: None,
            causing: None,
            in_water: false,
            wind_charge: false,
        }),
        center,
        radius: 1.0,
        fire: false,
        interaction: ExplosionInteraction::Tnt,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::TntMinecart { primed },
        presentation: ExplosionPresentation::Default,
    }
}

fn rail_state_id() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_RAIL_ID,
    )
    .expect("rail default state")
}

fn block_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

#[test]
fn primed_tnt_minecart_explosion_preserves_rail_without_resistance_refusal() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let rail_pos = BlockPos { x: 0, y: 300, z: 0 };
    assert!(world.set_block(OVERWORLD, rail_pos, rail_state_id(), SET_BLOCK_UPDATE_ALL,));

    world.explode(
        OVERWORLD,
        minecart_tnt_explosion(block_center(rail_pos), true),
        &mut Vec::new(),
    );

    assert_eq!(
        world.block_state(OVERWORLD, rail_pos),
        Some(rail_state_id())
    );
}

#[test]
fn primed_tnt_minecart_explosion_preserves_the_block_below_rail() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let support_pos = BlockPos { x: 0, y: 299, z: 0 };
    let rail_pos = BlockPos { x: 0, y: 300, z: 0 };
    let stone = block_state_id("minecraft:stone");
    assert!(world.set_block(OVERWORLD, support_pos, stone, SET_BLOCK_UPDATE_ALL));
    assert!(world.set_block(OVERWORLD, rail_pos, rail_state_id(), SET_BLOCK_UPDATE_ALL,));

    world.explode(
        OVERWORLD,
        minecart_tnt_explosion(block_center(support_pos), true),
        &mut Vec::new(),
    );

    assert_eq!(world.block_state(OVERWORLD, support_pos), Some(stone));
    assert_eq!(
        world.block_state(OVERWORLD, rail_pos),
        Some(rail_state_id())
    );
}

#[test]
fn unprimed_tnt_minecart_and_default_explosions_keep_default_rail_sampling() {
    for damage_profile in [
        ExplosionDamageProfile::TntMinecart { primed: false },
        ExplosionDamageProfile::Default,
    ] {
        let mut world = ServerWorld::new(datapack_config()).unwrap();
        let rail_pos = BlockPos { x: 0, y: 300, z: 0 };
        assert!(world.set_block(OVERWORLD, rail_pos, rail_state_id(), SET_BLOCK_UPDATE_ALL,));
        let mut request = minecart_tnt_explosion(block_center(rail_pos), false);
        request.damage_profile = damage_profile;

        world.explode(OVERWORLD, request, &mut Vec::new());

        assert_eq!(
            world.block_state(OVERWORLD, rail_pos),
            Some(block_state_id("minecraft:air")),
        );
    }
}

#[test]
fn wind_explosion_transfers_a_redirectable_projectile_to_its_living_causing_owner() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..creative_fixture_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "target", 0x7070);
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 0.0,
            y: 65.0,
            z: 0.0,
        },
        None,
    );
    let (owner_id, owner) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 20.0,
            y: 65.0,
            z: 20.0,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    let old_owner = Uuid::from_u128(0xdead);
    let (projectile_id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
        Vec3d {
            x: 1.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::LargeFireball(LargeFireballData::new(
            HurtingProjectileState::new(Some(old_owner), 0.1),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        )),
    );
    let owner_source = ExplosionSourceEntity {
        custom_name: None,
        entity_id: owner_id,
        entity_resource_id: "minecraft:armor_stand".to_string(),
        uuid: owner,
        position: Vec3d {
            x: 20.0,
            y: 65.0,
            z: 20.0,
        },
        kind: ExplosionSourceEntityKind::LivingOther,
    };
    let request = |causing: Option<ExplosionSourceEntity>| PendingExplosion {
        source: Some(ExplosionSource {
            direct: ExplosionSourceEntity {
                custom_name: None,
                entity_id: i32::MAX,
                entity_resource_id: "minecraft:wind_charge".to_string(),
                uuid: Uuid::from_u128(0x9090),
                position: Vec3d {
                    x: 4.0,
                    y: 65.0,
                    z: 0.0,
                },
                kind: ExplosionSourceEntityKind::Other,
            },
            indirect: causing.clone(),
            causing,
            in_water: false,
            wind_charge: true,
        }),
        center: Vec3d {
            x: 0.0,
            y: 65.0,
            z: 0.0,
        },
        radius: 3.0,
        fire: false,
        interaction: ExplosionInteraction::Trigger,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::wind_charge(false, 1.0),
        presentation: ExplosionPresentation::wind_charge("minecraft:entity.wind_charge.wind_burst"),
    };
    let mut events = Vec::new();

    world.explode(OVERWORLD, request(Some(owner_source.clone())), &mut events);
    let projectile = world
        .stored_entity_for_test(OVERWORLD, projectile_id)
        .expect("wind explosions do not damage the projectile");
    let crate::entity::EntityData::LargeFireball(data) = projectile.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));

    events.clear();
    world.explode(
        OVERWORLD,
        PendingExplosion {
            source: Some(ExplosionSource {
                direct: ExplosionSourceEntity {
                    custom_name: None,
                    entity_id: i32::MAX,
                    entity_resource_id: "minecraft:fireball".to_string(),
                    uuid: Uuid::from_u128(0x9191),
                    position: Vec3d {
                        x: 4.0,
                        y: 65.0,
                        z: 0.0,
                    },
                    kind: ExplosionSourceEntityKind::Other,
                },
                indirect: Some(owner_source.clone()),
                causing: Some(owner_source),
                in_water: false,
                wind_charge: false,
            }),
            center: Vec3d {
                x: 0.0,
                y: 65.0,
                z: 0.0,
            },
            radius: 4.0,
            fire: false,
            interaction: ExplosionInteraction::None,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );
    let damage = events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::DamageEvent(damage),
        } => Some(*damage),
        _ => None,
    });
    let damage = damage.unwrap_or_else(|| {
        panic!("the stored living owner remains the causing entity: {events:?}")
    });
    assert_eq!(damage.source_cause_id, owner_id);
    assert_eq!(damage.source_direct_id, i32::MAX);
    assert_eq!(damage.source_position, None);
    assert_eq!(
        damage.source_type_id,
        bbb_server_datapack::damage::builtin_damage_type_registry()
            .iter()
            .find(|facts| facts.resource_id == "minecraft:player_explosion")
            .expect("player explosion is a built-in damage type")
            .registry_id
    );

    world.explode(OVERWORLD, request(None), &mut events);
    let projectile = world
        .stored_entity_for_test(OVERWORLD, projectile_id)
        .expect("the projectile remains live after the second push");
    let crate::entity::EntityData::LargeFireball(data) = projectile.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), None);
}

#[test]
fn refused_explosion_hurt_still_pushes_and_redirect_uses_the_captured_registry_generation() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    world.track_stored_chunk_for_test(OVERWORLD, (0, 0));
    let owner = ExplosionSourceEntity {
        custom_name: None,
        entity_id: 7,
        entity_resource_id: "minecraft:zombie".to_string(),
        uuid: Uuid::from_u128(7),
        position: Vec3d {
            x: 10.0,
            y: 65.0,
            z: 0.0,
        },
        kind: ExplosionSourceEntityKind::Mob,
    };
    let old_owner = Uuid::from_u128(8);
    let (projectile_id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
        Vec3d {
            x: 1.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::LargeFireball(LargeFireballData::new(
            HurtingProjectileState::new(Some(old_owner), 0.1),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        )),
    );
    let snapshot = crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:entity_type".to_string(),
        "minecraft:redirectable_projectile".to_string(),
        Vec::new(),
    )]);
    world.install_level_registry_tags(Arc::new(snapshot));

    world.explode(
        OVERWORLD,
        PendingExplosion {
            source: Some(ExplosionSource {
                direct: ExplosionSourceEntity {
                    custom_name: None,
                    entity_id: 9,
                    entity_resource_id: "minecraft:wind_charge".to_string(),
                    uuid: Uuid::from_u128(9),
                    position: Vec3d {
                        x: 4.0,
                        y: 65.0,
                        z: 0.0,
                    },
                    kind: ExplosionSourceEntityKind::Other,
                },
                indirect: Some(owner.clone()),
                causing: Some(owner),
                in_water: false,
                wind_charge: true,
            }),
            center: Vec3d {
                x: 0.0,
                y: 65.0,
                z: 0.0,
            },
            radius: 3.0,
            fire: false,
            interaction: ExplosionInteraction::Trigger,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::wind_charge(true, 1.0),
            presentation: ExplosionPresentation::wind_charge(
                "minecraft:entity.wind_charge.wind_burst",
            ),
        },
        &mut Vec::new(),
    );

    let projectile = world
        .stored_entity_for_test(OVERWORLD, projectile_id)
        .expect("wind explosions do not damage the projectile");
    let crate::entity::EntityData::LargeFireball(data) = projectile.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(old_owner));
    assert!(projectile.delta_movement().x > 0.0);
}

#[test]
fn connected_direct_source_is_excluded_from_its_own_explosion() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "source", 0x7272);
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 0.0,
            y: 65.0,
            z: 0.0,
        },
        None,
    );
    let player_id = snapshot_player(&world.tick(receiver.drain()), 1).entity_id;
    let player_source = ExplosionSourceEntity {
        custom_name: None,
        entity_id: player_id,
        entity_resource_id: "minecraft:player".to_string(),
        uuid: Uuid::from_u128(0x7272),
        position: Vec3d {
            x: 0.0,
            y: 65.0,
            z: 0.0,
        },
        kind: ExplosionSourceEntityKind::Player {
            instabuild: false,
            may_build: true,
            spectator: false,
        },
    };
    let mut events = Vec::new();
    world.explode(
        OVERWORLD,
        PendingExplosion {
            source: Some(ExplosionSource {
                direct: player_source.clone(),
                indirect: Some(player_source.clone()),
                causing: Some(player_source),
                in_water: false,
                wind_charge: false,
            }),
            center: Vec3d {
                x: 0.0,
                y: 65.0,
                z: 0.0,
            },
            radius: 3.0,
            fire: false,
            interaction: ExplosionInteraction::None,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );

    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::DamageEvent(_),
        }
    )));
    let packet = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Explode(packet),
            } => Some(packet),
            _ => None,
        })
        .expect("the direct source still receives the explosion broadcast");
    assert_eq!(packet.player_knockback, None);
}

#[test]
fn end_crystal_nested_explosion_finishes_before_the_outer_broadcast() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        datapacks: datapack_config().datapacks,
        ..creative_fixture_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "witness", 0x7171);
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new(
        std::iter::empty::<(String, String, Vec<String>)>(),
    )));
    let (crystal_id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        Vec3d {
            x: 0.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, true)),
    );
    world.track_stored_chunk_for_test(OVERWORLD, (0, 0));
    let causing = ExplosionSourceEntity {
        custom_name: None,
        entity_id: 70,
        entity_resource_id: "minecraft:zombie".to_string(),
        uuid: Uuid::from_u128(70),
        position: Vec3d {
            x: 8.0,
            y: 65.0,
            z: 0.0,
        },
        kind: ExplosionSourceEntityKind::Mob,
    };
    let mut events = Vec::new();
    world.explode(
        OVERWORLD,
        PendingExplosion {
            source: Some(ExplosionSource {
                direct: ExplosionSourceEntity {
                    custom_name: None,
                    entity_id: 71,
                    entity_resource_id: "minecraft:tnt".to_string(),
                    uuid: Uuid::from_u128(71),
                    position: Vec3d {
                        x: 2.0,
                        y: 65.0,
                        z: 0.0,
                    },
                    kind: ExplosionSourceEntityKind::Other,
                },
                indirect: Some(causing.clone()),
                causing: Some(causing),
                in_water: false,
                wind_charge: false,
            }),
            center: Vec3d {
                x: 0.0,
                y: 65.0,
                z: 0.0,
            },
            radius: 2.0,
            fire: false,
            interaction: ExplosionInteraction::None,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );

    assert!(world
        .stored_entity_for_test(OVERWORLD, crystal_id)
        .is_none());
    let nested_damage = events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::DamageEvent(packet),
        } if packet.source_direct_id == crystal_id => Some(*packet),
        _ => None,
    });
    let nested_damage = nested_damage.expect("nested damage keeps the removed Mob cause");
    assert_eq!(nested_damage.source_cause_id, 70);
    assert_eq!(
        nested_damage.source_type_id,
        bbb_server_datapack::damage::builtin_damage_type_registry()
            .iter()
            .find(|facts| facts.resource_id == "minecraft:player_explosion")
            .unwrap()
            .registry_id
    );
}

#[test]
fn player_attacked_end_crystal_keeps_player_cause_and_entity_source_position() {
    let table = serde_json::json!({
        "type":"minecraft:block",
        "pools":[
            {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:end_crystal","nbt":"{Tags:[\"blast\"],ShowBottom:0b}"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:end_crystal","predicates":{"minecraft:custom_data":"{seal:7}"}}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:end_crystal","components":{"minecraft:custom_name":{"text":"Blast source"}}}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]}
        ]
    });
    for (tag, seal, name) in [
        ("blast", 7, "Blast source"),
        ("other", 7, "Other source"),
        ("blast", 8, "Blast source"),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config_with_loot_table("minecraft:blocks/white_wool", &table);
        config.game_type = GameType::Survival;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "attacker", 0x7373);
        // Keep the attacker alive while preserving a real damage event.
        for command in [
            "gamerule block_explosion_drop_decay false",
            "effect give attacker minecraft:resistance 60 3 true",
        ] {
            let tick = entities::console_command(&sender, &receiver, &mut world, 1, command);
            assert!(
                entities::console_command_answer(&tick).0.is_some(),
                "{command}"
            );
        }
        place_player(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 4.5,
                y: 300.0,
                z: 8.5,
            },
            None,
        );
        let player_id = snapshot_player(&world.tick(receiver.drain()), 1).entity_id;
        world.ensure_entity_transfer_full_chunk_for_test(OVERWORLD, (0, 0));
        let target = BlockPos { x: 9, y: 300, z: 8 };
        assert!(world.set_block(
            OVERWORLD,
            target,
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_WHITE_WOOL_ID,
            )
            .unwrap(),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        let crystal_position = Vec3d {
            x: 7.0,
            y: 300.0,
            z: 8.5,
        };
        let (crystal_id, uuid) = world.spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            crystal_position,
            crate::entity::EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, false)),
        );
        assert!(world
            .stored_entity_mut_for_test(OVERWORLD, crystal_id)
            .unwrap()
            .add_tag(tag));
        let command = format!(
            "data merge entity {uuid} {{data:{{seal:{seal}}},CustomName:{{text:\"{name}\"}}}}"
        );
        let changed = entities::console_command(&sender, &receiver, &mut world, 2, &command);
        let (result, feedback) = entities::console_command_answer(&changed);
        assert_eq!(result, Some(1));
        assert!(feedback
            .iter()
            .any(|key| key == "commands.data.entity.modified"));
        let custom_data = vec![NbtSummaryEntry {
            name: "seal".into(),
            value: NbtSummaryValue::Int(seal),
        }];
        let custom_name = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String(name.into()),
        }]);
        let crystal = world.stored_entity_for_test(OVERWORLD, crystal_id).unwrap();
        assert_eq!(crystal.custom_data(), custom_data.as_slice());
        assert_eq!(crystal.custom_name().unwrap().component(), &custom_name);
        world.tick(receiver.drain());
        let NbtSummaryValue::Compound(saved) = world
            .stored_entity_command_data_for_test(OVERWORLD, crystal_id)
            .unwrap()
        else {
            panic!("the actual crystal has canonical save data");
        };
        for (name, expected) in [
            (
                "Tags",
                NbtSummaryValue::List(vec![NbtSummaryValue::String(tag.into())]),
            ),
            ("ShowBottom", NbtSummaryValue::Byte(0)),
            ("data", NbtSummaryValue::Compound(custom_data)),
            ("CustomName", custom_name),
        ] {
            assert_eq!(
                saved
                    .iter()
                    .find(|field| field.name == name)
                    .map(|field| &field.value),
                Some(&expected)
            );
        }

        let attacked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Attack(AttackEntity {
                entity_id: crystal_id,
            }),
        );
        let damage = attacked.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::DamageEvent(packet),
                ..
            } if packet.source_direct_id == crystal_id => Some(packet),
            _ => None,
        });
        let damage = damage.unwrap_or_else(|| {
            panic!("the crystal explosion retains its player source: {attacked:?}")
        });
        assert_eq!(damage.source_cause_id, player_id);
        assert_eq!(damage.source_direct_id, crystal_id);
        assert_eq!(damage.source_position, None);
        let explosion = attacked.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(packet),
                ..
            } if packet.center == crystal_position => Some(packet),
            _ => None,
        });
        assert!(explosion.is_some());
        assert_eq!(explosion.unwrap().radius, 6.0);
        assert!(world.player_health_for_test(ConnectionId(1)) > 0.0);
        assert!(world
            .stored_entity_for_test(OVERWORLD, crystal_id)
            .is_none());
        assert!(world
            .live_entity_id_for_uuid_for_test(OVERWORLD, uuid)
            .is_none());
        assert_eq!(
            world.block_state(OVERWORLD, target),
            Some(block_state_id("minecraft:air"))
        );
        let mut drops = world
            .stored_items_for_test(OVERWORLD)
            .into_iter()
            .map(|(_, _, id, count, ..)| (id, count))
            .collect::<Vec<_>>();
        drops.sort();
        let mut expected = vec![("minecraft:apple".to_owned(), 1)];
        if tag == "blast" {
            expected.push(("minecraft:diamond".to_owned(), 1));
        }
        if seal == 7 {
            expected.push(("minecraft:emerald".to_owned(), 1));
        }
        if name == "Blast source" {
            expected.push(("minecraft:gold_ingot".to_owned(), 1));
        }
        assert_eq!(drops, expected, "tag={tag}, seal={seal}, name={name}");
    }
}

#[test]
fn command_stored_hurt_applies_hanging_and_source_position_live() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let (painting_id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PAINTING_ID,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        crate::entity::EntityData::Painting(crate::entity::PaintingData::new(
            BlockPos { x: 0, y: 65, z: 0 },
            Direction::South,
            crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
        )),
    );
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            OVERWORLD,
            painting_id,
            "minecraft:generic",
            1.0,
            None,
        )
        .expect("the painting transaction resolves");
    assert!(outcome.succeeded);
    assert!(world
        .stored_entity_for_test(OVERWORLD, painting_id)
        .is_none());

    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (cow_id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        cow_type,
        Vec3d {
            x: 2.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    let outcome = world
        .hurt_stored_entity_by_command_at_for_test(
            OVERWORLD,
            cow_id,
            "minecraft:mob_attack",
            1.0,
            [0.0, 65.0, 0.0],
        )
        .expect("the living transaction resolves");
    assert!(outcome.succeeded);
    let delta = world
        .stored_entity_for_test(OVERWORLD, cow_id)
        .unwrap()
        .delta_movement();
    assert!((delta.x - 0.4).abs() < 1.0e-9, "{delta:?}");
}

#[test]
fn live_stored_mob_alone_enables_conditional_difficulty_scaling() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        difficulty: Difficulty::Hard,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    join_loaded(&sender, &receiver, &mut world, 4, "dave", 4);
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (_, cow_uuid) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        cow_type,
        Vec3d {
            x: 4.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    let (_, stand_uuid) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 5.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let (_, item_uuid) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 6.0,
            y: 65.0,
            z: 0.0,
        },
        crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(
            plain_stack("minecraft:stone", 1).unwrap(),
            0.0,
        )),
    );

    let mob = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 4 minecraft:generic by {cow_uuid}"),
    );
    let living_other = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage bob 4 minecraft:generic by {stand_uuid}"),
    );
    let player = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage carol 4 minecraft:generic by alice",
    );
    let other = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage dave 4 minecraft:generic by {item_uuid}"),
    );

    assert_eq!(set_healths(&mob), [(ConnectionId(1), 14.0)]);
    assert_eq!(set_healths(&living_other), [(ConnectionId(2), 14.0)]);
    assert_eq!(set_healths(&player), [(ConnectionId(3), 16.0)]);
    assert_eq!(set_healths(&other), [(ConnectionId(4), 16.0)]);
}

#[test]
fn removed_living_sources_keep_frozen_difficulty_scaling() {
    for (entity_type_id, data, is_mob) in [
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                )
                .unwrap(),
            ),
            true,
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
            false,
        ),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Survival,
            difficulty: Difficulty::Hard,
            ..operator_config(&[("alice", 1)])
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let position = Vec3d {
            x: 4.0,
            y: 65.0,
            z: 0.0,
        };
        let (entity_id, uuid) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Nether,
            entity_type_id,
            position,
            data,
        );
        assert!(
            world.discard_stored_command_source_for_test(Java26BuiltinDimension::Nether, entity_id)
        );
        let causing = crate::commands::CommandDamageSourceEntity::Stored {
            entity_id,
            entity_resource_id: if is_mob {
                "minecraft:cow"
            } else {
                "minecraft:armor_stand"
            }
            .to_string(),
            uuid: Some(uuid),
            position: Some(position),
            is_mob,
            is_living_non_player: true,
            display_name: NbtSummaryValue::string("source"),
        };
        let (succeeded, _) = world.hurt_connected_player_by_command_for_test(
            ConnectionId(1),
            "minecraft:generic",
            4.0,
            Some(causing),
            None,
            None,
        );
        assert!(succeeded);
        let settled = world.tick(receiver.drain());
        assert_eq!(set_healths(&settled), [(ConnectionId(1), 14.0)]);
    }
}

#[test]
fn cross_dimension_removed_mob_source_does_not_consult_the_target_store() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule mob_griefing false",
    );
    let source_position = Vec3d {
        x: 8.0,
        y: 65.0,
        z: 0.0,
    };
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (source_id, source_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Nether,
        cow_type,
        source_position,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    assert!(world.discard_stored_command_source_for_test(Java26BuiltinDimension::Nether, source_id));
    let (target_id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d::default(),
        crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(
            plain_stack("minecraft:stone", 1).unwrap(),
            0.0,
        )),
    );
    let outcome = world
        .hurt_stored_entity_by_command_with_facts_for_test(
            OVERWORLD,
            target_id,
            "minecraft:generic",
            1.0,
            Some(crate::commands::CommandDamageSourceEntity::Stored {
                entity_id: source_id,
                entity_resource_id: "minecraft:cow".to_string(),
                uuid: Some(source_uuid),
                position: Some(source_position),
                is_mob: true,
                is_living_non_player: true,
                display_name: NbtSummaryValue::string("cow"),
            }),
            None,
            None,
            crate::entity::EntityRegistryTagView::Builtin26_1,
            false,
        )
        .expect("the target item remains resolvable");
    assert!(!outcome.applied && !outcome.succeeded, "{outcome:?}");
    assert!(world.stored_entity_for_test(OVERWORLD, target_id).is_some());
}

#[test]
fn command_direct_position_is_effective_but_raw_at_position_wins() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let spawn_target = |world: &mut ServerWorld| {
        world
            .spawn_stored_entity_for_test(
                OVERWORLD,
                cow_type,
                Vec3d {
                    x: 2.0,
                    y: 65.0,
                    z: 0.0,
                },
                crate::entity::EntityData::GeneralLiving(
                    crate::entity::GeneralLivingData::new(cow_type).unwrap(),
                ),
            )
            .0
    };
    let direct_source = |x| crate::commands::CommandDamageSourceEntity::Stored {
        entity_id: 700,
        entity_resource_id: "minecraft:cow".to_string(),
        uuid: Some(Uuid::from_u128(700)),
        position: Some(Vec3d { x, y: 65.0, z: 0.0 }),
        is_mob: true,
        is_living_non_player: true,
        display_name: NbtSummaryValue::string("cow"),
    };

    let direct_target = spawn_target(&mut world);
    assert!(
        world
            .hurt_stored_entity_by_command_with_facts_for_test(
                OVERWORLD,
                direct_target,
                "minecraft:mob_attack",
                1.0,
                None,
                Some(direct_source(0.0)),
                None,
                crate::entity::EntityRegistryTagView::Builtin26_1,
                false,
            )
            .expect("the direct-position hit resolves")
            .succeeded
    );
    let direct_delta = world
        .stored_entity_for_test(OVERWORLD, direct_target)
        .unwrap()
        .delta_movement();
    assert!((direct_delta.x - 0.4).abs() < 1.0e-9, "{direct_delta:?}");

    let raw_target = spawn_target(&mut world);
    assert!(
        world
            .hurt_stored_entity_by_command_with_facts_for_test(
                OVERWORLD,
                raw_target,
                "minecraft:mob_attack",
                1.0,
                None,
                Some(direct_source(4.0)),
                Some([0.0, 65.0, 0.0]),
                crate::entity::EntityRegistryTagView::Builtin26_1,
                false,
            )
            .expect("the raw-position hit resolves")
            .succeeded
    );
    let raw_delta = world
        .stored_entity_for_test(OVERWORLD, raw_target)
        .unwrap()
        .delta_movement();
    assert!((raw_delta.x - 0.4).abs() < 1.0e-9, "{raw_delta:?}");
}

#[test]
fn command_live_hurt_consumes_the_effects_frozen_registry_and_fall_facts() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let spawn_painting = |world: &mut ServerWorld| {
        world
            .spawn_stored_entity_for_test(
                OVERWORLD,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PAINTING_ID,
                Vec3d {
                    x: 0.5,
                    y: 65.0,
                    z: 0.5,
                },
                crate::entity::EntityData::Painting(crate::entity::PaintingData::new(
                    BlockPos { x: 0, y: 65, z: 0 },
                    Direction::South,
                    crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
                )),
            )
            .0
    };

    let fall_immune_id = spawn_painting(&mut world);
    let outcome = world
        .hurt_stored_entity_by_command_with_facts_for_test(
            OVERWORLD,
            fall_immune_id,
            "minecraft:fall",
            1.0,
            None,
            None,
            None,
            crate::entity::EntityRegistryTagView::Builtin26_1,
            true,
        )
        .expect("the frozen fall membership resolves");
    assert!(!outcome.applied && !outcome.succeeded, "{outcome:?}");
    assert!(world
        .stored_entity_for_test(OVERWORLD, fall_immune_id)
        .is_some());
}

mod hanging;
mod redstone;
