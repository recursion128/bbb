use super::*;
use crate::explosion::{clip_misses, get_seen_percent};
use bbb_server_types::geometry::Aabb;

const CENTER: Vec3d = Vec3d {
    x: 0.5,
    y: 64.061_25,
    z: 0.5,
};

fn state(name: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap()
}

fn open_stone_floor() -> ChunkStore {
    let mut chunks = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    for cx in -1..=0 {
        for cz in -1..=0 {
            let column = chunks.generated_column();
            chunks.insert_column(cx, cz, column);
        }
    }
    for x in -8..=8 {
        for z in -8..=8 {
            chunks
                .set_block_state(x, 63, z, state("minecraft:stone"))
                .unwrap();
        }
    }
    chunks
}

#[test]
fn get_seen_percent_is_one_in_open_air_and_zero_behind_a_wall() {
    let mut chunks = open_stone_floor();
    let level = ExplosionBlocks { chunks: &chunks };
    // `EntityType.ARMOR_STAND` dimensions 0.5 x 1.975, at (3.5, 64, 0.5).
    let stand = Aabb::of_entity(
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
        0.5,
        1.975,
    );
    assert_eq!(get_seen_percent(&level, CENTER, stand), 1.0);
    // A solid stone wall on the line between the centre and the stand.
    for y in 64..=66 {
        chunks
            .set_block_state(2, y, 0, state("minecraft:stone"))
            .unwrap();
    }
    assert_eq!(
        get_seen_percent(&ExplosionBlocks { chunks: &chunks }, CENTER, stand),
        0.0
    );
}

#[test]
fn armor_stand_hit_at_a_known_offset_matches_vanilla() {
    // `ExplodeOracle` printed dist=0.3750781494089765, damage=29.432577,
    // knockback=(0.5424310517079939, 0.3103157641646152, 0.0) for an armor
    // stand at (3.5, 64, 0.5) in open air, exposure 1.0, radius 4.
    let chunks = open_stone_floor();
    let level = ExplosionBlocks { chunks: &chunks };
    let stand_pos = Vec3d {
        x: 3.5,
        y: 64.0,
        z: 0.5,
    };
    let eye = Vec3d {
        x: 3.5,
        y: 64.0 + 1.7775,
        z: 0.5,
    };
    let stand_box = Aabb::of_entity(stand_pos, 0.5, 1.975);
    let hit = entity_hit_with_profile(
        &level,
        CENTER,
        4.0,
        stand_pos,
        eye,
        stand_box,
        0.0,
        &ExplosionDamageProfile::Default,
    )
    .expect("the stand is in range");
    assert!(
        (hit.dist - 0.375_078_149_408_976_5).abs() < 1e-12,
        "{}",
        hit.dist
    );
    assert_eq!(hit.exposure, 1.0);
    assert_eq!(hit.damage, 29.432_577);
    assert!((hit.knockback.x - 0.542_431_051_707_993_9).abs() < 1e-12);
    assert!((hit.knockback.y - 0.310_315_764_164_615_2).abs() < 1e-12);
    assert_eq!(hit.knockback.z, 0.0);
}

#[test]
fn an_entity_beyond_double_radius_is_not_hit() {
    let chunks = open_stone_floor();
    let level = ExplosionBlocks { chunks: &chunks };
    let far = Vec3d {
        x: 20.0,
        y: 64.0,
        z: 0.5,
    };
    let far_box = Aabb::of_entity(far, 0.5, 1.975);
    assert!(entity_hit_with_profile(
        &level,
        CENTER,
        4.0,
        far,
        far,
        far_box,
        0.0,
        &ExplosionDamageProfile::Default
    )
    .is_none());
}

#[test]
fn knockback_resistance_scales_the_push_but_not_the_damage() {
    let chunks = open_stone_floor();
    let level = ExplosionBlocks { chunks: &chunks };
    let pos = Vec3d {
        x: 3.5,
        y: 64.0,
        z: 0.5,
    };
    let eye = Vec3d {
        x: 3.5,
        y: 64.0 + 1.7775,
        z: 0.5,
    };
    let bb = Aabb::of_entity(pos, 0.5, 1.975);
    let full = entity_hit_with_profile(
        &level,
        CENTER,
        4.0,
        pos,
        eye,
        bb,
        0.0,
        &ExplosionDamageProfile::Default,
    )
    .unwrap();
    let half = entity_hit_with_profile(
        &level,
        CENTER,
        4.0,
        pos,
        eye,
        bb,
        0.5,
        &ExplosionDamageProfile::Default,
    )
    .unwrap();
    assert_eq!(full.damage, half.damage);
    assert!((half.knockback.x - full.knockback.x * 0.5).abs() < 1e-12);
    assert!((half.knockback.y - full.knockback.y * 0.5).abs() < 1e-12);
}

#[test]
fn wind_charge_profile_pushes_without_explosion_damage() {
    let chunks = open_stone_floor();
    let level = ExplosionBlocks { chunks: &chunks };
    let pos = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 0.5,
    };
    let eye = Vec3d {
        x: 1.5,
        y: 65.62,
        z: 0.5,
    };
    let bb = Aabb::of_entity(pos, 0.6, 1.8);
    let default = entity_hit_with_profile(
        &level,
        CENTER,
        1.2,
        pos,
        eye,
        bb,
        0.0,
        &ExplosionDamageProfile::Default,
    )
    .unwrap();
    let wind = entity_hit_with_profile(
        &level,
        CENTER,
        1.2,
        pos,
        eye,
        bb,
        0.0,
        &ExplosionDamageProfile::wind_charge(false, 1.22),
    )
    .unwrap();
    assert!(default.damage > 0.0);
    assert_eq!(wind.damage, 0.0);
    let multiplier = f64::from(1.22_f32);
    assert!((wind.knockback.x - default.knockback.x * multiplier).abs() < 1e-12);
    assert!((wind.knockback.y - default.knockback.y * multiplier).abs() < 1e-12);
}

#[test]
fn wind_charged_effect_burst_only_knocks_back() {
    let chunks = open_stone_floor();
    let level = ExplosionBlocks { chunks: &chunks };
    let pos = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 0.5,
    };
    let eye = Vec3d {
        x: 1.5,
        y: 65.62,
        z: 0.5,
    };
    let bb = Aabb::of_entity(pos, 0.6, 1.8);
    let default = entity_hit_with_profile(
        &level,
        CENTER,
        1.2,
        pos,
        eye,
        bb,
        0.0,
        &ExplosionDamageProfile::Default,
    )
    .unwrap();
    let effect = entity_hit_with_profile(
        &level,
        CENTER,
        1.2,
        pos,
        eye,
        bb,
        0.0,
        &wind_charged_burst_profile(),
    )
    .unwrap();
    assert_eq!(effect.damage, 0.0);
    assert_eq!(effect.knockback, default.knockback);
}

#[test]
fn explosion_clip_keeps_entry_occlusion() {
    let mut chunks = open_stone_floor();
    chunks
        .set_block_state(0, 64, 0, state("minecraft:stone"))
        .unwrap();
    let level = ExplosionBlocks { chunks: &chunks };
    let point = |x| Vec3d { x, y: 64.5, z: 0.5 };
    assert!(!clip_misses(&level, point(-1.0), point(2.0)));
    assert!(clip_misses(&level, point(0.9999), point(2.0)));
    assert!(clip_misses(&level, point(0.5), point(0.5)));
}

#[test]
fn named_blocks_break_while_obsidian_survives_the_actual_sample() {
    let mut chunks = open_stone_floor();
    for x in -12..=12 {
        for z in -12..=12 {
            for y in 61..=63 {
                chunks
                    .set_block_state(x, y, z, state("minecraft:stone"))
                    .unwrap();
            }
        }
    }
    let removed = [
        (BlockPos { x: 2, y: 64, z: 0 }, "minecraft:dirt"),
        (BlockPos { x: 2, y: 65, z: 0 }, "minecraft:dirt"),
        (BlockPos { x: -2, y: 64, z: 1 }, "minecraft:oak_planks"),
        (BlockPos { x: -2, y: 65, z: 1 }, "minecraft:oak_planks"),
        (BlockPos { x: -2, y: 66, z: 1 }, "minecraft:oak_planks"),
        (BlockPos { x: 0, y: 64, z: 3 }, "minecraft:cobblestone"),
        (BlockPos { x: 3, y: 64, z: -3 }, "minecraft:sand"),
        (
            BlockPos {
                x: -1,
                y: 64,
                z: -2,
            },
            "minecraft:tnt",
        ),
    ];
    for (pos, name) in removed {
        chunks
            .set_block_state(pos.x, pos.y, pos.z, state(name))
            .unwrap();
    }
    let obsidian = BlockPos { x: 1, y: 66, z: 1 };
    chunks
        .set_block_state(
            obsidian.x,
            obsidian.y,
            obsidian.z,
            state("minecraft:obsidian"),
        )
        .unwrap();
    let sampled = calculate_exploded_positions(
        &chunks,
        &GameRandom::with_seed(987_654_321),
        None,
        CENTER,
        4.0,
        &ExplosionDamageProfile::Default,
    );
    for (pos, _) in removed {
        assert!(sampled.positions.contains(&pos));
    }
    assert!(!sampled.positions.contains(&obsidian));
}

#[test]
fn large_legal_empty_world_explosions_keep_distinct_positions() {
    let chunks = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    let center = Vec3d {
        x: 0.5,
        y: 64.5,
        z: 0.5,
    };
    let mut previous_count = 0;
    for radius in [96.0, 128.0] {
        let sampled = calculate_exploded_positions(
            &chunks,
            &GameRandom::with_seed(0),
            None,
            center,
            radius,
            &ExplosionDamageProfile::Default,
        );
        assert!(sampled.positions.len() > previous_count);
        previous_count = sampled.positions.len();
        let distinct: FxHashSet<_> = sampled
            .positions
            .iter()
            .map(|pos| (pos.x, pos.y, pos.z))
            .collect();
        assert_eq!(distinct.len(), sampled.positions.len());
    }
}

#[test]
fn used_portal_tnt_leaves_portal_blocks_intact() {
    let mut chunks = open_stone_floor();
    let portal = BlockPos { x: 0, y: 64, z: 0 };
    chunks
        .set_block_state(
            portal.x,
            portal.y,
            portal.z,
            state("minecraft:nether_portal"),
        )
        .unwrap();
    let sampled = calculate_exploded_positions(
        &chunks,
        &GameRandom::with_seed(0),
        None,
        Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5,
        },
        4.0,
        &ExplosionDamageProfile::PrimedTntUsedPortal,
    );
    assert!(!sampled.positions.contains(&portal));
    assert!(sampled.positions.contains(&BlockPos { x: 1, y: 64, z: 0 }));
}

#[test]
fn wind_charge_breaks_nonimmune_stone_at_the_origin() {
    let mut chunks = open_stone_floor();
    let stone = BlockPos { x: 0, y: 64, z: 0 };
    chunks
        .set_block_state(stone.x, stone.y, stone.z, state("minecraft:stone"))
        .unwrap();
    let center = Vec3d {
        x: 0.5,
        y: 64.5,
        z: 0.5,
    };
    let sampled = calculate_exploded_positions(
        &chunks,
        &GameRandom::with_seed(0),
        None,
        center,
        1.2,
        &ExplosionDamageProfile::wind_charge(false, 1.22),
    );
    assert!(sampled.positions.contains(&stone));
}

fn runtime_world() -> (ServerWorld, crate::mailbox::WorldCommandReceiver) {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig {
        game_type: bbb_protocol::game_types::GameType::Survival,
        ..super::super::ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "witness", 1);
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.set_position(Vec3d {
        x: 1.5,
        y: 300.0,
        z: 0.5,
    });
    player.set_delta_movement(Vec3d::default());
    (world, receiver)
}

#[test]
fn custom_explosion_damage_type_and_explicit_position_reach_stored_and_connected_hurt() {
    let (mut world, receiver) = runtime_world();
    let dimension = world.levels[ConnectionId(1)].dimension;
    let (zombie, _) = world.spawn_stored_entity_for_test(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 1.5,
            y: 300.0,
            z: 1.5,
        },
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            )
            .unwrap(),
        ),
    );
    world.tick(receiver.drain());
    let center = Vec3d {
        x: 0.5,
        y: 300.0,
        z: 0.5,
    };
    let explicit_position = Vec3d {
        x: -2.0,
        y: 301.0,
        z: 0.5,
    };
    let mut events = Vec::new();
    world
        .explode(
            dimension,
            PendingExplosion {
                source: None,
                center,
                radius: 1.5,
                fire: false,
                interaction: ExplosionInteraction::None,
                damage_source: Some(ExplosionDamageSource {
                    damage_type: "minecraft:magic".to_owned(),
                    explicit_position: Some(explicit_position),
                }),
                damage_profile: ExplosionDamageProfile::Simple {
                    explodes_blocks: false,
                    damages_entities: true,
                    knockback_multiplier: 0.0,
                    immune_blocks: None,
                },
                presentation: ExplosionPresentation::Default,
            },
            &mut events,
        )
        .unwrap();
    let magic = world
        .cold
        .damage_types
        .iter()
        .find(|damage| damage.resource_id == "minecraft:magic")
        .unwrap()
        .registry_id;
    for entity_id in [world.levels[ConnectionId(1)].entity_id(), zombie] {
        let damage = events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::DamageEvent(damage),
                    ..
                } if damage.entity_id == entity_id => Some(damage),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing damage for {entity_id}: {events:?}"));
        assert_eq!(damage.source_type_id, magic);
        assert_eq!(damage.source_position, Some(explicit_position));
        assert_eq!(damage.source_cause_id, -1);
        assert_eq!(damage.source_direct_id, -1);
    }
    assert!(world.levels[ConnectionId(1)].damage.health < 20.0);
    assert!(
        world
            .level_by_id(&dimension)
            .entities
            .get(zombie)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < 20.0
    );
}

#[test]
fn simple_explosion_immune_sets_preserve_normal_resistance_or_replace_it() {
    let obsidian = state("minecraft:obsidian");
    let obsidian_type =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:obsidian")
            .unwrap()
            .id;
    for (immune_blocks, survives) in [
        (None, true),
        (
            Some(std::sync::Arc::new(rustc_hash::FxHashSet::default())),
            false,
        ),
        (
            Some(std::sync::Arc::new([obsidian_type].into_iter().collect())),
            true,
        ),
    ] {
        let (mut world, _) = runtime_world();
        let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld;
        let key: LevelId = dimension.into();
        let pos = BlockPos { x: 0, y: 300, z: 0 };
        world
            .level_mut_by_id(&key)
            .chunks
            .set_block_state(pos.x, pos.y, pos.z, obsidian)
            .unwrap();
        world
            .explode(
                dimension,
                PendingExplosion {
                    source: None,
                    center: Vec3d {
                        x: 0.5,
                        y: 300.5,
                        z: 0.5,
                    },
                    radius: 1.2,
                    fire: false,
                    interaction: ExplosionInteraction::Block,
                    damage_source: None,
                    damage_profile: ExplosionDamageProfile::Simple {
                        explodes_blocks: true,
                        damages_entities: false,
                        knockback_multiplier: 0.0,
                        immune_blocks,
                    },
                    presentation: ExplosionPresentation::Default,
                },
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(
            super::super::stored_block_state(&world.level_by_id(&key).chunks, pos) == obsidian,
            survives
        );
    }
}

#[test]
fn custom_explosion_presentation_and_simple_flying_player_have_no_impulse() {
    use bbb_protocol::packets::{ParticleOptions, WeightedExplosionParticle};
    let small_particle = ParticleOptions::Dust {
        color: 0x12ab34,
        scale: 1.25,
    }
    .payload(bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DUST_ID)
    .unwrap();
    let large_particle = ParticleOptions::Block {
        state_id: state("minecraft:stone"),
    }
    .payload(bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID)
    .unwrap();
    let block_particles = vec![WeightedExplosionParticle {
        particle: small_particle.clone(),
        scaling: 0.75,
        speed: 1.5,
        weight: 3,
    }];
    let sound = SoundEventHolder::Direct {
        location: "example:blast".to_owned(),
        fixed_range: Some(12.5),
    };
    for (game_type, interaction, expected_particle) in [
        (
            bbb_protocol::game_types::GameType::Survival,
            ExplosionInteraction::None,
            &small_particle,
        ),
        (
            bbb_protocol::game_types::GameType::Creative,
            ExplosionInteraction::Block,
            &large_particle,
        ),
    ] {
        let (mut world, _) = runtime_world();
        let player = world.levels.player_mut(ConnectionId(1)).unwrap();
        player.game_type = game_type;
        player.abilities.flying = true;
        let dimension = player.dimension;
        let center = player.position().plus(Vec3d {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        });
        let mut events = Vec::new();
        world
            .explode(
                dimension,
                PendingExplosion {
                    source: None,
                    center,
                    radius: 3.0,
                    fire: false,
                    interaction,
                    damage_source: None,
                    damage_profile: ExplosionDamageProfile::Simple {
                        explodes_blocks: interaction != ExplosionInteraction::None,
                        damages_entities: false,
                        knockback_multiplier: 2.0,
                        immune_blocks: None,
                    },
                    presentation: ExplosionPresentation::Custom {
                        small_particle: small_particle.clone(),
                        large_particle: large_particle.clone(),
                        block_particles: block_particles.clone(),
                        sound: sound.clone(),
                    },
                },
                &mut events,
            )
            .unwrap();
        assert_eq!(
            world.levels[ConnectionId(1)].delta_movement(),
            Vec3d::default()
        );
        let packet = events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Explode(packet),
                    ..
                } => Some(packet),
                _ => None,
            })
            .unwrap();
        assert_eq!(packet.player_knockback, None);
        assert_eq!(&packet.explosion_particle, expected_particle);
        assert_eq!(packet.explosion_sound, sound);
        assert_eq!(packet.block_particles, block_particles);
        if interaction == ExplosionInteraction::None {
            assert_eq!(packet.block_count, 0);
        }
        let (packet_id, bytes) =
            bbb_protocol::packets::encode_play_clientbound_explode(packet.clone()).unwrap();
        let decoded = bbb_protocol::packets::decode_play_clientbound(packet_id, &bytes).unwrap();
        assert!(
            matches!(decoded, bbb_protocol::packets::PlayClientbound::Explosion(value) if value == *packet)
        );
    }
}
