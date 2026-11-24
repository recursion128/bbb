use super::*;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, PendingExplosion,
};
use bbb_protocol::packets::{ClientCommandAction, Explosion};

fn move_to(r: &mut Runtime, dimension: Java26BuiltinDimension) {
    r.world
        .install_full_chunks_by_key(&dimension.into(), &[(0, 0)])
        .unwrap();
    let mut events = Vec::new();
    assert!(r.world.teleport_player(
        ID,
        TeleportTransition {
            new_level: dimension.into(),
            position: Vec3d {
                x: 1.5,
                y: 101.0,
                z: 2.5
            },
            y_rot: 35.0,
            x_rot: 12.0,
            relatives_mask: 0,
        },
        &mut events
    ));
    let id = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    r.play(1, PlayServerbound::AcceptTeleportation { id });
    r.play(1, PlayServerbound::PlayerLoaded);
    for x in 0..=5 {
        for z in 0..=5 {
            for y in 100..=104 {
                r.world.set_block(
                    dimension,
                    BlockPos { x, y, z },
                    state(
                        if y == 100 {
                            "minecraft:stone"
                        } else {
                            "minecraft:air"
                        },
                        &[],
                    ),
                    UPDATE_CLIENTS,
                );
            }
        }
    }
}

fn block(r: &Runtime, dimension: Java26BuiltinDimension, pos: BlockPos) -> i32 {
    stored_block_state(&r.world.level_by_id(&dimension.into()).chunks, pos)
}

fn explosion(tick: &TickResult) -> &Explosion {
    tick.events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(update),
                ..
            } => Some(update),
            _ => None,
        })
        .expect("actual block use broadcasts an explosion")
}

fn die_and_respawn(r: &mut Runtime) -> TickResult {
    r.world.set_player_dead_for_test(ID);
    r.play(
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    )
}

fn set_anchor(r: &mut Runtime, dimension: Java26BuiltinDimension, charges: &str) {
    assert!(r.world.set_block(
        dimension,
        FOOT,
        state("minecraft:respawn_anchor", &[("charges", charges)]),
        UPDATE_CLIENTS
    ));
}

fn charges(r: &Runtime, dimension: Java26BuiltinDimension) -> i32 {
    crate::block_behavior::respawn_anchor::charges(block(r, dimension, FOOT)).unwrap()
}

#[test]
fn sleep_runtime_nether_and_end_bed_clicks_remove_both_halves_and_use_bad_respawn_point_damage() {
    for dimension in [Java26BuiltinDimension::Nether, Java26BuiltinDimension::End] {
        let mut r = runtime("always", 1);
        move_to(&mut r, dimension);
        let mut reference = runtime("always", 1);
        move_to(&mut reference, dimension);
        for (pos, part) in [(FOOT, "foot"), (HEAD, "head")] {
            r.world.set_block(
                dimension,
                pos,
                state(
                    "minecraft:red_bed",
                    &[("facing", "east"), ("occupied", "false"), ("part", part)],
                ),
                UPDATE_CLIENTS,
            );
        }
        let mut abilities = r.world.player_abilities_for_test(ID);
        abilities.invulnerable = false;
        abilities.instabuild = false;
        abilities.flying = false;
        r.world
            .set_player_mode_and_abilities_for_test(ID, GameType::Survival, abilities);
        r.world.tick_rate_mut().set_frozen(true);
        r.world.random.reseed(0x261);
        let tick = r.click(1, FOOT);
        let center = Vec3d {
            x: 3.5,
            y: 101.5,
            z: 2.5,
        };
        assert_eq!(explosion(&tick).center, center);
        assert_eq!(explosion(&tick).radius, 5.0);
        reference.world.tick_rate_mut().set_frozen(true);
        reference.world.random.reseed(0x261);
        let mut reference_events = Vec::new();
        reference.world.explode(
            dimension,
            PendingExplosion {
                source: None,
                center,
                radius: 5.0,
                fire: true,
                interaction: ExplosionInteraction::Block,
                damage_source: None,
                damage_profile: ExplosionDamageProfile::Default,
                presentation: ExplosionPresentation::Default,
            },
            &mut reference_events,
        );
        let reference_count = reference_events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Explode(update),
                    ..
                } => Some(update.block_count),
                _ => None,
            })
            .unwrap();
        assert_eq!(explosion(&tick).block_count, reference_count);
        assert!(!crate::block_behavior::bed::is_bed(block(
            &r, dimension, FOOT
        )));
        assert!(!crate::block_behavior::bed::is_bed(block(
            &r, dimension, HEAD
        )));
        let bad = r
            .world
            .registries
            .registry_element_index("minecraft:damage_type", "minecraft:bad_respawn_point")
            .unwrap();
        assert!(tick.events.iter().any(|event| matches!(event,WorldEvent::Send {packet:ClientboundPlayPacket::DamageEvent(damage),..} if damage.entity_id==r.world.levels[ID].entity_id()&&damage.source_type_id==bad&&damage.source_position==Some(center)&&damage.source_cause_id== -1&&damage.source_direct_id== -1)), "damage events: {:?}, health: {:?}, transition: {:?}", tick.events.iter().filter(|event| matches!(event,WorldEvent::Send {packet:ClientboundPlayPacket::DamageEvent(_),..})).collect::<Vec<_>>(),r.world.levels[ID].damage,r.world.player_transition_state_for_test(ID));
        assert_eq!(r.world.levels[ID].cold.sleeping_pos, None);
        assert_eq!(r.world.levels[ID].cold.respawn, None);
    }
}

#[test]
fn sleep_runtime_overworld_anchor_dry_explosion_matches_standard_block_fire_and_wet_origin_exhausts_rays(
) {
    let seed = 0x26_1001;
    let mut dry = runtime("always", 1);
    let mut reference = runtime("always", 1);
    let mut wet = runtime("always", 1);
    for r in [&mut dry, &mut reference, &mut wet] {
        set_anchor(r, DIM, "1");
        r.world.tick_rate_mut().set_frozen(true);
        r.world.random.reseed(seed);
    }
    wet.world.set_block(
        DIM,
        BlockPos {
            y: FOOT.y + 1,
            ..FOOT
        },
        state("minecraft:water", &[("level", "0")]),
        UPDATE_CLIENTS,
    );
    let dry_tick = dry.click(1, FOOT);
    reference
        .world
        .set_block(DIM, FOOT, state("minecraft:air", &[]), UPDATE_CLIENTS);
    let mut reference_events = Vec::new();
    reference.world.explode(
        DIM,
        PendingExplosion {
            source: None,
            center: Vec3d {
                x: 2.5,
                y: 101.5,
                z: 2.5,
            },
            radius: 5.0,
            fire: true,
            interaction: ExplosionInteraction::Block,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut reference_events,
    );
    reference.tick();
    for x in 0..=8 {
        for z in 0..=8 {
            for y in 95..=109 {
                let pos = BlockPos { x, y, z };
                assert_eq!(
                    block(&dry, DIM, pos),
                    block(&reference, DIM, pos),
                    "{pos:?}"
                );
            }
        }
    }
    assert!(explosion(&dry_tick).block_count > 0);
    let wet_tick = wet.click(1, FOOT);
    // RespawnAnchorBlock.java:159-175; Blocks.java:284-296.
    assert_eq!(explosion(&wet_tick).block_count, 0);
    assert_eq!(block(&wet, DIM, FOOT), state("minecraft:air", &[]));
    assert_eq!(
        block(&wet, DIM, BlockPos { y: 100, ..FOOT }),
        state("minecraft:stone", &[])
    );
}

#[test]
fn sleep_runtime_actual_death_respawns_at_the_bed_set_by_empty_hand_use() {
    let mut r = runtime("always", 1);
    r.click(1, FOOT);
    let respawn = r.world.levels[ID].cold.respawn.clone().unwrap();
    r.world.tick_rate_mut().set_frozen(true);
    let tick = die_and_respawn(&mut r);
    let player = &r.world.levels.player_at(ID);
    assert_eq!(
        player.dimension,
        r.world.require_level_id(&respawn.dimension)
    );
    assert_eq!(player.cold.respawn, Some(respawn));
    assert_eq!(player.cold.sleeping_pos, None);
    assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!(player.rotation().x_rot, 0.0);
    assert!((player.position().x - 3.5).abs() <= 3.0);
    assert!((player.position().z - 2.5).abs() <= 3.0);
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Respawn(_),
            ..
        }
    )));
    assert!(!tick.events.iter().any(|event| matches!(event,WorldEvent::Send {packet:ClientboundPlayPacket::GameEvent(update),..} if update.event_id==bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID)));
}

#[test]
fn sleep_runtime_actual_anchor_respawn_consumes_one_charge_and_privately_uses_destination_random() {
    let mut r = runtime("always", 1);
    move_to(&mut r, Java26BuiltinDimension::Nether);
    set_anchor(&mut r, Java26BuiltinDimension::Nether, "2");
    r.click(1, FOOT);
    r.world.tick_rate_mut().set_frozen(true);
    let seed = 0x261_dee1;
    r.world.random.reseed(seed);
    let tick = die_and_respawn(&mut r);
    assert_eq!(charges(&r, Java26BuiltinDimension::Nether), 1);
    assert_eq!(
        r.world.levels[ID].dimension,
        Java26BuiltinDimension::Nether.into()
    );
    let registry_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:block.respawn_anchor.deplete",
    )
    .unwrap();
    let sounds = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } if sound.sound
                == bbb_protocol::packets::SoundEventHolder::Reference { registry_id } =>
            {
                Some((connection_id, sound))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sounds.len(), 1);
    assert_eq!(*sounds[0].0, ID);
    assert_eq!(
        sounds[0].1.position,
        Vec3d {
            x: 2.0,
            y: 101.0,
            z: 2.0
        }
    );
}

#[test]
fn sleep_runtime_blocked_anchor_preserves_charge_and_missing_event_precedes_default_respawn() {
    let mut r = runtime("always", 1);
    move_to(&mut r, Java26BuiltinDimension::Nether);
    set_anchor(&mut r, Java26BuiltinDimension::Nether, "2");
    r.click(1, FOOT);
    for x in 0..=4 {
        for z in 0..=4 {
            for y in 100..=104 {
                let pos = BlockPos { x, y, z };
                if pos != FOOT {
                    r.world.set_block(
                        Java26BuiltinDimension::Nether,
                        pos,
                        state("minecraft:stone", &[]),
                        UPDATE_CLIENTS,
                    );
                }
            }
        }
    }
    r.world.tick_rate_mut().set_frozen(true);
    let tick = die_and_respawn(&mut r);
    assert_eq!(charges(&r, Java26BuiltinDimension::Nether), 2);
    assert_eq!(r.world.levels[ID].dimension, DIM.into());
    let missing = |tick: &TickResult| {
        tick.events.iter().any(|event| matches!(event,WorldEvent::Send {packet:ClientboundPlayPacket::GameEvent(update),..} if update.event_id==bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID))
    };
    assert!(missing(&tick));
    // `PlayerList.java:395-397`: the unusable anchor is forgotten, so no
    // deplete sound follows (`:425-426`) and the next death is a plain one.
    assert_eq!(r.world.levels[ID].cold.respawn, None);
    let deplete = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:block.respawn_anchor.deplete",
    )
    .unwrap();
    assert!(!tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.sound == (bbb_protocol::packets::SoundEventHolder::Reference { registry_id: deplete })
    )));
    assert!(!missing(&die_and_respawn(&mut r)));
}

#[test]
fn sleep_runtime_anchor_resolver_forced_zero_and_non_consuming_queries_preserve_charge() {
    let mut r = runtime("always", 1);
    move_to(&mut r, Java26BuiltinDimension::Nether);
    set_anchor(&mut r, Java26BuiltinDimension::Nether, "2");
    r.click(1, FOOT);
    let mut config = r.world.levels[ID].cold.respawn.clone().unwrap();
    let (spawn, missing) = r.world.resolve_sleep_respawn(Some(&config), false).unwrap();
    assert!(!missing);
    assert_eq!(spawn.dimension, r.world.require_level_id(&config.dimension));
    assert_eq!(charges(&r, Java26BuiltinDimension::Nether), 2);
    config.forced = true;
    set_anchor(&mut r, Java26BuiltinDimension::Nether, "0");
    let (spawn, missing) = r.world.resolve_sleep_respawn(Some(&config), true).unwrap();
    assert!(!missing);
    assert_eq!(spawn.dimension, r.world.require_level_id(&config.dimension));
    assert_eq!(charges(&r, Java26BuiltinDimension::Nether), 0);
}

/// `isPossibleToRespawnInThis` overrides (`BasePressurePlateBlock.java:49`,
/// `SignBlock.java:80`, `AbstractBannerBlock.java:25`) keep a forced point
/// on solid-shaped plates, signs and banners usable.
#[test]
fn forced_respawn_lands_on_plates_signs_and_banners() {
    let mut r = runtime("always", 1);
    let pos = BlockPos { x: 4, y: 101, z: 4 };
    let config = PlayerRespawnConfig {
        dimension: DIM.into(),
        pos,
        yaw: 0.0,
        pitch: 0.0,
        forced: true,
    };
    for block in [
        "minecraft:stone_pressure_plate",
        "minecraft:light_weighted_pressure_plate",
        "minecraft:oak_sign",
        "minecraft:oak_wall_sign",
        "minecraft:oak_hanging_sign",
        "minecraft:oak_wall_hanging_sign",
        "minecraft:white_banner",
        "minecraft:white_wall_banner",
    ] {
        let block = bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(block)
                .unwrap()
                .id,
        )
        .unwrap();
        r.world.set_block(DIM, pos, block, UPDATE_CLIENTS);
        let (spawn, missing) = r.world.resolve_sleep_respawn(Some(&config), false).unwrap();
        assert!(!missing, "{block}");
        assert_eq!(spawn.position.y, 101.1, "{block}");
    }
    r.world
        .set_block(DIM, pos, state("minecraft:stone", &[]), UPDATE_CLIENTS);
    let (_, missing) = r.world.resolve_sleep_respawn(Some(&config), false).unwrap();
    assert!(missing);
}
