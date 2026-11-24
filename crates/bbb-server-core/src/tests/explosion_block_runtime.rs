//! ServerExplosion block callbacks and fire through a real ServerWorld.

use super::*;

use crate::block_entity::vibration::VibrationData;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    ExplosionSourceEntity, ExplosionSourceEntityKind, PendingExplosion,
};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_FIRE_ID, VANILLA_BLOCK_SOUL_FIRE_ID, VANILLA_BLOCK_TNT_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_SILVERFISH_ID, VANILLA_ENTITY_TYPE_TNT_ID};
use bbb_protocol::packets::EntityDataValueKind;

mod beehive;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SENSOR: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

#[test]
fn fireball_block_loot_reads_the_actual_direct_source_nbt() {
    use crate::entity::hurting_projectile::fireball::LargeFireballData;
    use crate::entity::hurting_projectile::HurtingProjectileState;
    use crate::tests::entities::{console_command, console_command_answer};
    use bbb_protocol::NbtSummaryValue;

    let table = serde_json::json!({
        "type":"minecraft:block",
        "pools":[
            {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:fireball","nbt":"{Tags:[\"blast\"]}"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"team":"blue"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_scores","entity":"this",
                "scores":{"probe":7}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"team":"red"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:coal"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_scores","entity":"this",
                "scores":{"probe":8}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:iron_ingot"}]}
        ]
    });
    for tag in ["blast", "other"] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config_with_loot_table(
            "minecraft:blocks/white_wool",
            &table,
        ));
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        place_player(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 0.5,
                y: 300.0,
                z: 6.5,
            },
            None,
        );
        world.ensure_entity_transfer_full_chunk_for_test(DIM, (0, 0));
        let target = BlockPos { x: 6, y: 300, z: 6 };
        assert!(world.set_block(
            DIM,
            target,
            block_state_id("minecraft:white_wool"),
            SET_BLOCK_UPDATE_ALL,
        ));
        let (fireball, uuid) = world.spawn_stored_entity_for_test(
            DIM,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
            Vec3d {
                x: 4.5,
                y: 300.5,
                z: 6.5,
            },
            crate::entity::EntityData::LargeFireball(LargeFireballData::new(
                HurtingProjectileState::new(Some(Uuid::from_u128(1)), 0.0),
                crate::entity::item::plain_stack("minecraft:fire_charge", 1).unwrap(),
                2,
            )),
        );
        let body = world.stored_entity_mut_for_test(DIM, fireball).unwrap();
        assert!(body.add_tag(tag));
        for (operation, command, expected) in [
            (1, "team add blue".to_string(), 1),
            (2, "team add red".to_string(), 2),
            (3, format!("team join blue {uuid}"), 1),
            (4, "scoreboard objectives add probe dummy".to_string(), 1),
            (5, format!("scoreboard players set {uuid} probe 7"), 7),
        ] {
            let tick = console_command(&sender, &receiver, &mut world, operation, &command);
            assert_eq!(console_command_answer(&tick).0, Some(expected));
        }
        let body = world.stored_entity_mut_for_test(DIM, fireball).unwrap();
        assert_eq!(body.delta_movement(), Vec3d::default());
        body.set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
        let NbtSummaryValue::Compound(saved) = world
            .stored_entity_command_data_for_test(DIM, fireball)
            .unwrap()
        else {
            unreachable!()
        };
        assert!(saved.iter().any(|field| field.name == "Tags"
            && field.value == NbtSummaryValue::List(vec![NbtSummaryValue::String(tag.into())])));

        let output = world.tick(receiver.drain());
        assert!(world.stored_entity_for_test(DIM, fireball).is_none());
        assert!(output.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(_),
                ..
            }
        )));
        assert_eq!(
            world.block_state(DIM, target),
            Some(block_state_id("minecraft:air"))
        );
        let mut drops = world
            .stored_items_for_test(DIM)
            .into_iter()
            .map(|(_, _, id, count, ..)| (id, count))
            .collect::<Vec<_>>();
        drops.sort();
        let mut expected = vec![("minecraft:apple".to_owned(), 1)];
        if tag == "blast" {
            expected.push(("minecraft:diamond".to_owned(), 1));
        }
        expected.push(("minecraft:emerald".to_owned(), 1));
        expected.push(("minecraft:gold_ingot".to_owned(), 1));
        expected.sort();
        assert_eq!(drops, expected, "{tag}");
    }
}

#[test]
fn tnt_block_loot_reads_the_actual_removed_direct_source_nbt() {
    use crate::entity::TntData;
    use bbb_protocol::NbtSummaryValue;

    let table = serde_json::json!({
        "type":"minecraft:block",
        "pools":[
            {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
            {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:tnt","nbt":"{Tags:[\"blast\"],fuse:0s,explosion_power:3.0f,block_state:{Name:\"minecraft:tnt\"}}"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]}
        ]
    });
    for tag in ["blast", "other"] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config_with_loot_table(
            "minecraft:blocks/white_wool",
            &table,
        ));
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        place_player(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 14.5,
                y: 300.0,
                z: 6.5,
            },
            None,
        );
        let setup = entities::console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamerule tnt_explosion_drop_decay false",
        );
        assert!(entities::console_command_answer(&setup).0.is_some());
        world.ensure_entity_transfer_full_chunk_for_test(DIM, (0, 0));
        let target = BlockPos { x: 6, y: 300, z: 6 };
        assert!(world.set_block(
            DIM,
            target,
            vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_WHITE_WOOL_ID)
                .unwrap(),
            SET_BLOCK_UPDATE_ALL,
        ));
        let owner = world
            .entity_reads()
            .players()
            .find(|player| player.name == "observer")
            .unwrap()
            .id;
        let (tnt, uuid) = world.spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_TNT_ID,
            Vec3d {
                x: 5.5,
                y: 300.5,
                z: 6.5,
            },
            crate::entity::EntityData::Tnt(TntData::with_saved_fields(
                1,
                vanilla_block_default_state_id(VANILLA_BLOCK_TNT_ID).unwrap(),
                Some(owner),
                3.0,
            )),
        );
        let body = world.stored_entity_mut_for_test(DIM, tnt).unwrap();
        assert!(body.add_tag(tag));
        body.set_no_gravity(true);
        let NbtSummaryValue::Compound(saved) =
            world.stored_entity_command_data_for_test(DIM, tnt).unwrap()
        else {
            unreachable!()
        };
        for (name, expected) in [
            (
                "Tags",
                NbtSummaryValue::List(vec![NbtSummaryValue::String(tag.into())]),
            ),
            ("fuse", NbtSummaryValue::Short(1)),
            ("explosion_power", NbtSummaryValue::Float(3.0_f32.to_bits())),
            ("NoGravity", NbtSummaryValue::Byte(1)),
            ("owner", crate::entity::save::uuid_int_array(owner)),
        ] {
            assert_eq!(
                saved
                    .iter()
                    .find(|field| field.name == name)
                    .map(|field| &field.value),
                Some(&expected),
                "{name}"
            );
        }
        let Some(NbtSummaryValue::Compound(block_state)) = saved
            .iter()
            .find(|field| field.name == "block_state")
            .map(|field| &field.value)
        else {
            panic!("the TNT saves its displayed block state");
        };
        assert!(block_state.iter().any(|field| field.name == "Name"
            && field.value == NbtSummaryValue::String("minecraft:tnt".into())));

        let output = world.tick(receiver.drain());
        assert!(world.stored_entity_for_test(DIM, tnt).is_none());
        assert!(world.live_entity_id_for_uuid_for_test(DIM, uuid).is_none());
        assert!(output.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Explode(explosion),
            } if explosion.radius == 3.0
        )));
        assert_eq!(
            world.block_state(DIM, target),
            Some(block_state_id("minecraft:air"))
        );
        let mut drops = world
            .stored_items_for_test(DIM)
            .into_iter()
            .map(|(_, _, id, count, ..)| (id, count))
            .collect::<Vec<_>>();
        drops.sort();
        let mut expected = vec![("minecraft:apple".to_owned(), 1)];
        if tag == "blast" {
            expected.push(("minecraft:diamond".to_owned(), 1));
        }
        assert_eq!(drops, expected, "{tag}");
    }
}

fn explode_at(
    world: &mut ServerWorld,
    center: Vec3d,
    radius: f32,
    interaction: ExplosionInteraction,
    fire: bool,
) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    world.explode(
        DIM,
        PendingExplosion {
            source: None,
            center,
            radius,
            fire,
            interaction,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );
    events
}

fn trigger_state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let block_type_id = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing block type {name}"))
        .id;
    let default = vanilla_block_default_state_id(block_type_id).unwrap();
    properties.iter().fold(default, |state, &(key, value)| {
        with_property(state, key, value).unwrap_or_else(|| panic!("{name} lacks {key}={value}"))
    })
}

fn trigger_at(
    world: &mut ServerWorld,
    pos: BlockPos,
    source: Option<&str>,
    interaction: ExplosionInteraction,
) -> Vec<WorldEvent> {
    trigger_at_radius(world, pos, source, interaction, 3.0)
}

fn trigger_at_radius(
    world: &mut ServerWorld,
    pos: BlockPos,
    source: Option<&str>,
    interaction: ExplosionInteraction,
    radius: f32,
) -> Vec<WorldEvent> {
    let center = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    let mut events = Vec::new();
    world.explode(
        DIM,
        PendingExplosion {
            source: source.map(|resource_id| ExplosionSource {
                direct: ExplosionSourceEntity {
                    custom_name: None,
                    entity_id: 73,
                    entity_resource_id: resource_id.into(),
                    uuid: Uuid::from_u128(73),
                    position: center,
                    kind: ExplosionSourceEntityKind::Other,
                },
                indirect: None,
                causing: None,
                in_water: false,
                wind_charge: true,
            }),
            center,
            radius,
            fire: false,
            interaction,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );
    events
}

fn trigger_world(name: &str, properties: &[(&str, &str)]) -> (ServerWorld, BlockPos, i32) {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let pos = BlockPos { x: 6, y: 300, z: 6 };
    assert!(world.set_block(
        DIM,
        BlockPos {
            y: pos.y - 1,
            ..pos
        },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let state = trigger_state(name, properties);
    assert!(world.set_block(DIM, pos, state, SET_BLOCK_UPDATE_ALL));
    (world, pos, state)
}

fn heart_explosion_fixture(natural: bool) -> (ServerWorld, BlockPos, i32) {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let (pos, protector_id) = install_heart_explosion_fixture(&mut world, natural);
    (world, pos, protector_id)
}

fn install_heart_explosion_fixture(world: &mut ServerWorld, natural: bool) -> (BlockPos, i32) {
    let pos = BlockPos { x: 6, y: 300, z: 6 };
    let heart = trigger_state(
        "minecraft:creaking_heart",
        &[
            ("axis", "y"),
            ("creaking_heart_state", "dormant"),
            ("natural", if natural { "true" } else { "false" }),
        ],
    );
    assert!(world.set_block(DIM, pos, heart, SET_BLOCK_UPDATE_ALL));
    let uuid = Uuid::from_u128(0x8724);
    world
        .level_block_entity_for_test(DIM, pos)
        .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
        .expect("heart block entity")
        .set_protector_for_test(uuid);
    let protector_id = world
        .restore_stored_entity_for_test(
            DIM,
            uuid,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            Vec3d {
                x: 10.5,
                y: 300.0,
                z: 6.5,
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID)
                    .expect("creaking catalog data"),
            ),
        )
        .expect("stored protector");
    assert!(world
        .stored_entity_mut_for_test(DIM, protector_id)
        .expect("protector")
        .set_creaking_transient(pos));
    (pos, protector_id)
}

fn heart_explosion_source(pos: BlockPos, instabuild: bool, spectator: bool) -> ExplosionSource {
    let center = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    let player = ExplosionSourceEntity {
        custom_name: None,
        entity_id: 74,
        entity_resource_id: "minecraft:player".into(),
        uuid: Uuid::from_u128(0x4274),
        position: center,
        kind: ExplosionSourceEntityKind::Player {
            instabuild,
            may_build: true,
            spectator,
        },
    };
    ExplosionSource {
        direct: ExplosionSourceEntity {
            custom_name: None,
            entity_id: 73,
            entity_resource_id: "minecraft:tnt".into(),
            uuid: Uuid::from_u128(73),
            position: center,
            kind: ExplosionSourceEntityKind::Other,
        },
        indirect: Some(player.clone()),
        causing: Some(player),
        in_water: false,
        wind_charge: false,
    }
}

fn heart_explode(
    world: &mut ServerWorld,
    pos: BlockPos,
    source: Option<ExplosionSource>,
    interaction: ExplosionInteraction,
) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    world.explode(
        DIM,
        PendingExplosion {
            source,
            center: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            radius: 8.0,
            fire: false,
            interaction,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );
    events
}

fn total_experience(world: &ServerWorld) -> i32 {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| match world.stored_entity_for_test(DIM, id)?.data() {
            crate::entity::EntityData::ExperienceOrb(orb) => Some(orb.value() * orb.count()),
            _ => None,
        })
        .sum()
}

#[test]
fn natural_heart_explosion_dies_with_source_then_tears_down_on_tick_46() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 25.5,
            y: 300.0,
            z: 6.5,
        },
    );
    let (pos, protector_id) = install_heart_explosion_fixture(&mut world, true);
    world.tick(receiver.drain());
    assert!(world.stored_entity_for_test(DIM, protector_id).is_some());
    assert_eq!(
        world
            .stored_entity_for_test(DIM, protector_id)
            .and_then(|entity| entity.living())
            .map(|living| living.death_time),
        Some(0)
    );
    let mut first = heart_explode(
        &mut world,
        pos,
        Some(heart_explosion_source(pos, false, false)),
        ExplosionInteraction::Block,
    );
    assert_eq!(
        world.block_state(DIM, pos),
        Some(block_state_id("minecraft:air"))
    );
    let protector = world
        .stored_entity_for_test(DIM, protector_id)
        .expect("dead protector remains");
    assert!(protector.creaking_is_tearing_down());
    assert_eq!(protector.living().expect("living").health(), 0.0);
    assert_eq!(protector.living().expect("living").death_time, 0);
    assert_eq!(
        protector.living().expect("living").last_hurt_by_player,
        Some(Uuid::from_u128(0x4274))
    );
    assert_eq!(protector.living().expect("living").last_damage_type, None);
    assert_eq!(
        protector.living().expect("living").last_damage_memory_time,
        0
    );
    assert!((20..=24).contains(&total_experience(&world)));
    assert!(world
        .game_events(DIM)
        .0
        .iter()
        .any(|row| row.game_event == VanillaGameEvent::ENTITY_DIE
            && row.source_entity_id == Some(protector_id)));
    let died_before = world
        .game_events(DIM)
        .0
        .iter()
        .filter(|row| {
            row.game_event == VanillaGameEvent::ENTITY_DIE
                && row.source_entity_id == Some(protector_id)
        })
        .count();
    let heart_state = trigger_state(
        "minecraft:creaking_heart",
        &[
            ("axis", "y"),
            ("creaking_heart_state", "dormant"),
            ("natural", "false"),
        ],
    );
    assert!(world.set_block(DIM, pos, heart_state, SET_BLOCK_UPDATE_ALL));
    world
        .level_block_entity_for_test(DIM, pos)
        .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
        .expect("second heart")
        .set_protector_for_test(Uuid::from_u128(0x8724));
    let _ = heart_explode(
        &mut world,
        pos,
        Some(heart_explosion_source(pos, false, false)),
        ExplosionInteraction::Block,
    );
    assert_eq!(
        world
            .game_events(DIM)
            .0
            .iter()
            .filter(|row| row.game_event == VanillaGameEvent::ENTITY_DIE
                && row.source_entity_id == Some(protector_id))
            .count(),
        died_before
    );
    for tick in 1..=20 {
        let result = world.tick(receiver.drain());
        if tick == 1 {
            first.extend(result.events);
        }
    }
    assert!(
        world.stored_entity_for_test(DIM, protector_id).is_some(),
        "20 tick death duration must not remove a tearing creaking"
    );
    assert!(first.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(update), .. } if update.entity_id == protector_id && update.event_id == crate::entity::living::ENTITY_EVENT_DEATH)));
    assert!(first.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. } if update.id == protector_id && update.values.iter().any(|value| value.data_id == VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID && value.value == EntityDataValueKind::Boolean(true)))));
    for _ in 21..46 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world
            .stored_entity_for_test(DIM, protector_id)
            .and_then(|entity| entity.living())
            .map(|living| living.death_time),
        Some(45)
    );
    let protector_position = world
        .stored_entity_for_test(DIM, protector_id)
        .expect("tearing protector")
        .position();
    world.set_player_position_for_test(ConnectionId(1), protector_position);
    let last = world.tick(receiver.drain());
    assert!(world.stored_entity_for_test(DIM, protector_id).is_none());
    let bursts: Vec<_> = last
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelParticles(particles),
                ..
            } => Some(particles.count),
            _ => None,
        })
        .collect();
    assert_eq!(bursts, [100, 10]);
}

#[test]
fn heart_explosion_xp_uses_frozen_indirect_player_mode() {
    for (natural, instabuild, spectator, indirect, expected_xp) in [
        (true, false, false, true, true),
        (true, true, false, true, false),
        (true, false, true, true, false),
        (false, false, false, true, false),
        (true, false, false, false, false),
    ] {
        let (mut world, pos, protector_id) = heart_explosion_fixture(natural);
        let mut source = heart_explosion_source(pos, instabuild, spectator);
        if !indirect {
            source.indirect = None;
        }
        let _ = heart_explode(&mut world, pos, Some(source), ExplosionInteraction::Block);
        assert_eq!(
            world.block_state(DIM, pos),
            Some(block_state_id("minecraft:air"))
        );
        assert!(world
            .stored_entity_for_test(DIM, protector_id)
            .is_some_and(|entity| entity.creaking_is_tearing_down()));
        assert_eq!(
            total_experience(&world) > 0,
            expected_xp,
            "natural={natural} creative={instabuild} spectator={spectator} indirect={indirect}"
        );
    }
}

#[test]
fn heart_without_live_protector_still_awards_indirect_player_experience() {
    let mut world = spawn_pinned_world(datapack_config());
    let (pos, _) = install_heart_explosion_fixture(&mut world, true);
    world
        .level_block_entity_for_test(DIM, pos)
        .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
        .expect("heart")
        .set_protector_for_test(Uuid::from_u128(0x9999));
    let _ = heart_explode(
        &mut world,
        pos,
        Some(heart_explosion_source(pos, false, false)),
        ExplosionInteraction::Block,
    );
    assert_eq!(
        world.block_state(DIM, pos),
        Some(block_state_id("minecraft:air"))
    );
    assert!((20..=24).contains(&total_experience(&world)));
}

#[test]
fn crystal_player_causing_source_keeps_death_credit_without_indirect_xp() {
    let (mut world, pos, protector_id) = heart_explosion_fixture(true);
    let mut source = heart_explosion_source(pos, false, false);
    source.direct.entity_resource_id = "minecraft:end_crystal".into();
    source.indirect = None;
    let _ = heart_explode(&mut world, pos, Some(source), ExplosionInteraction::Block);
    let protector = world
        .stored_entity_for_test(DIM, protector_id)
        .expect("dead protector");
    assert!(protector.creaking_is_tearing_down());
    assert_eq!(
        protector.living().expect("living").last_hurt_by_player,
        Some(Uuid::from_u128(0x4274))
    );
    assert_eq!(total_experience(&world), 0);
    assert_eq!(
        world
            .game_events(DIM)
            .0
            .iter()
            .filter(|row| row.game_event == VanillaGameEvent::ENTITY_DIE
                && row.source_entity_id == Some(protector_id))
            .count(),
        1
    );
}

#[test]
fn heart_trigger_and_keep_do_not_kill_or_award_experience() {
    for interaction in [ExplosionInteraction::Trigger, ExplosionInteraction::None] {
        let (mut world, pos, protector_id) = heart_explosion_fixture(true);
        let _ = heart_explode(
            &mut world,
            pos,
            Some(heart_explosion_source(pos, false, false)),
            interaction,
        );
        assert!(world
            .stored_entity_for_test(DIM, protector_id)
            .is_some_and(|entity| !entity.creaking_is_tearing_down()));
        assert_eq!(total_experience(&world), 0);
        assert!(world
            .level_block_entity_for_test(DIM, pos)
            .and_then(|entity| entity.creaking_heart())
            .is_some_and(|heart| heart.protector_uuid().is_some()));
    }
}

#[test]
fn trigger_block_button_and_lever_keep_blocks_and_emit_source_free_activation() {
    for (name, properties) in [
        ("minecraft:stone_button", &[("face", "floor")][..]),
        ("minecraft:lever", &[("face", "floor")][..]),
    ] {
        let (mut world, pos, before) = trigger_world(name, properties);
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            },
        );
        let tnt_pos = BlockPos {
            x: pos.x + 2,
            ..pos
        };
        let tnt = trigger_state("minecraft:tnt", &[]);
        assert!(world.set_block(DIM, tnt_pos, tnt, SET_BLOCK_UPDATE_ALL));
        let ids_before = world.stored_entity_ids_for_test(DIM);
        trigger_at(&mut world, pos, None, ExplosionInteraction::Trigger);
        assert_eq!(
            world.block_state(DIM, pos),
            with_property(before, "powered", "true"),
            "{name} pressed without breaking",
        );
        let (events, _) = world.game_events(DIM);
        assert!(
            events.iter().any(|event| {
                event.game_event == VanillaGameEvent::BLOCK_ACTIVATE
                    && event.pos == pos
                    && event.source_entity_id.is_none()
            }),
            "{name} activation source"
        );
        assert_eq!(world.block_state(DIM, tnt_pos), Some(tnt));
        assert_eq!(
            world.stored_entity_ids_for_test(DIM),
            ids_before,
            "TRIGGER cannot spawn drops or primed TNT"
        );
        assert!(world
            .drain_block_sounds_for_test(DIM)
            .iter()
            .any(|sound| sound.pos == pos));
        if name == "minecraft:stone_button" {
            for _ in 0..21 {
                world.tick(receiver.drain());
            }
        } else {
            trigger_at(&mut world, pos, None, ExplosionInteraction::Trigger);
        }
        assert_eq!(world.block_state(DIM, pos), Some(before), "{name} releases");
        let (events, _) = world.game_events(DIM);
        assert!(
            events.iter().any(|event| {
                event.game_event == VanillaGameEvent::BLOCK_DEACTIVATE
                    && event.pos == pos
                    && event.source_entity_id.is_none()
            }),
            "{name} deactivation"
        );
    }
}

#[test]
fn trigger_block_gate_candle_and_bell_keep_blocks_with_their_own_events() {
    for (name, properties, event) in [
        (
            "minecraft:oak_fence_gate",
            &[][..],
            VanillaGameEvent::BLOCK_OPEN,
        ),
        (
            "minecraft:candle",
            &[("lit", "true")][..],
            VanillaGameEvent::BLOCK_CHANGE,
        ),
        (
            "minecraft:candle_cake",
            &[("lit", "true")][..],
            VanillaGameEvent::BLOCK_CHANGE,
        ),
        ("minecraft:bell", &[][..], VanillaGameEvent::BLOCK_CHANGE),
    ] {
        let (mut world, pos, before) = trigger_world(name, properties);
        let (sender, receiver) = mailbox();
        if name == "minecraft:bell" {
            join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
            world.set_player_position_for_test(
                ConnectionId(1),
                Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y),
                    z: f64::from(pos.z) + 0.5,
                },
            );
        }
        let emitted = trigger_at(&mut world, pos, None, ExplosionInteraction::Trigger);
        let after = world.block_state(DIM, pos).unwrap();
        let expected = match name {
            "minecraft:oak_fence_gate" => with_property(before, "open", "true").unwrap(),
            "minecraft:candle" | "minecraft:candle_cake" => {
                with_property(before, "lit", "false").unwrap()
            }
            _ => before,
        };
        assert_eq!(after, expected, "{name} callback state");
        let (events, _) = world.game_events(DIM);
        assert!(
            events.iter().any(|record| {
                record.game_event == event
                    && record.pos == pos
                    && record.source_entity_id.is_none()
                    && record.affected_state
                        == if name == "minecraft:oak_fence_gate" {
                            Some(before)
                        } else {
                            None
                        }
            }),
            "{name} callback event"
        );
        if name != "minecraft:bell" {
            assert!(world
                .drain_block_sounds_for_test(DIM)
                .iter()
                .any(|sound| sound.pos == pos));
        } else {
            assert!(emitted.iter().any(|event| {
                matches!(event, WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound), ..
                } if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
                    registry_id: vanilla_sound_event_registry_id("minecraft:block.bell.use").unwrap(),
                })
            }), "TRIGGER rings the bell for nearby sessions");
        }
    }
}

#[test]
fn trigger_block_opens_wooden_door_and_trapdoor_but_not_iron_or_powered_states() {
    let (mut door_world, pos, lower) = trigger_world("minecraft:oak_door", &[]);
    let upper_pos = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    let upper = with_property(lower, "half", "upper").unwrap();
    assert!(door_world.set_block(DIM, upper_pos, upper, SET_BLOCK_UPDATE_ALL));
    trigger_at(&mut door_world, pos, None, ExplosionInteraction::Trigger);
    assert_eq!(
        door_world.block_state(DIM, pos),
        with_property(lower, "open", "true"),
    );
    assert_eq!(
        door_world.block_state(DIM, upper_pos),
        with_property(upper, "open", "true"),
    );
    let (events, _) = door_world.game_events(DIM);
    assert!(events.iter().any(|event| {
        event.pos == pos
            && event.game_event == VanillaGameEvent::BLOCK_OPEN
            && event.source_entity_id.is_none()
    }));
    assert!(door_world
        .drain_block_sounds_for_test(DIM)
        .iter()
        .any(|sound| sound.pos == pos));

    let (mut upper_world, pos, lower) = trigger_world("minecraft:oak_door", &[]);
    let upper_pos = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    let upper = with_property(lower, "half", "upper").unwrap();
    assert!(upper_world.set_block(DIM, upper_pos, upper, SET_BLOCK_UPDATE_ALL));
    trigger_at_radius(
        &mut upper_world,
        upper_pos,
        None,
        ExplosionInteraction::Trigger,
        0.2,
    );
    assert_eq!(
        upper_world.block_state(DIM, pos),
        Some(lower),
        "upper half cannot toggle"
    );
    assert_eq!(upper_world.block_state(DIM, upper_pos), Some(upper));

    let (mut iron_world, iron_pos, iron_lower) = trigger_world("minecraft:iron_door", &[]);
    let iron_upper_pos = BlockPos {
        y: iron_pos.y + 1,
        ..iron_pos
    };
    let iron_upper = with_property(iron_lower, "half", "upper").unwrap();
    assert!(iron_world.set_block(DIM, iron_upper_pos, iron_upper, SET_BLOCK_UPDATE_ALL));
    trigger_at(
        &mut iron_world,
        iron_pos,
        None,
        ExplosionInteraction::Trigger,
    );
    assert_eq!(iron_world.block_state(DIM, iron_pos), Some(iron_lower));
    assert_eq!(
        iron_world.block_state(DIM, iron_upper_pos),
        Some(iron_upper)
    );

    for name in ["minecraft:oak_trapdoor", "minecraft:iron_trapdoor"] {
        let (mut world, pos, before) = trigger_world(name, &[]);
        trigger_at(&mut world, pos, None, ExplosionInteraction::Trigger);
        assert_eq!(
            world.block_state(DIM, pos),
            if name == "minecraft:oak_trapdoor" {
                with_property(before, "open", "true")
            } else {
                Some(before)
            },
            "{name} wind-charge gate",
        );
        let (events, _) = world.game_events(DIM);
        assert_eq!(
            events.iter().any(|event| {
                event.pos == pos
                    && event.game_event == VanillaGameEvent::BLOCK_OPEN
                    && event.source_entity_id.is_none()
            }),
            name == "minecraft:oak_trapdoor"
        );
    }
    for name in ["minecraft:oak_fence_gate", "minecraft:stone_button"] {
        let properties = if name == "minecraft:stone_button" {
            &[("face", "floor"), ("powered", "true")][..]
        } else {
            &[("powered", "true")][..]
        };
        let (mut world, pos, before) = trigger_world(name, properties);
        trigger_at(&mut world, pos, None, ExplosionInteraction::Trigger);
        assert_eq!(
            world.block_state(DIM, pos),
            Some(before),
            "{name} already powered"
        );
    }
    let (mut powered_trapdoor, pos, before) =
        trigger_world("minecraft:oak_trapdoor", &[("powered", "true")]);
    trigger_at(
        &mut powered_trapdoor,
        pos,
        None,
        ExplosionInteraction::Trigger,
    );
    assert_eq!(powered_trapdoor.block_state(DIM, pos), Some(before));
}

#[test]
fn breeze_trigger_respects_mob_griefing_while_wind_charge_and_source_free_trigger_do_not() {
    for source in [
        Some("minecraft:breeze_wind_charge"),
        Some("minecraft:wind_charge"),
        None,
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(operator_config(&[("operator", 1)]));
        join_loaded(&sender, &receiver, &mut world, 1, "operator", 1);
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamerule mob_griefing false",
        );
        assert_eq!(
            world.game_rule_for_test("minecraft:mob_griefing"),
            Some(GameRuleValue::Boolean(false)),
        );
        let pos = BlockPos { x: 6, y: 300, z: 6 };
        let before = trigger_state("minecraft:stone_button", &[("face", "floor")]);
        assert!(world.set_block(
            DIM,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
        assert!(world.set_block(DIM, pos, before, SET_BLOCK_UPDATE_ALL));
        world.random.reseed(17);
        trigger_at(&mut world, pos, source, ExplosionInteraction::Trigger);
        let expected = if source == Some("minecraft:breeze_wind_charge") {
            before
        } else {
            with_property(before, "powered", "true").unwrap()
        };
        assert_eq!(world.block_state(DIM, pos), Some(expected), "{source:?}");
    }
}

#[test]
fn keep_and_destroy_never_run_trigger_callbacks() {
    for interaction in [ExplosionInteraction::None, ExplosionInteraction::Block] {
        let (mut world, pos, before) = trigger_world("minecraft:oak_fence_gate", &[]);
        trigger_at(&mut world, pos, None, interaction);
        let (events, _) = world.game_events(DIM);
        assert!(!events.iter().any(|record| {
            record.pos == pos && record.game_event == VanillaGameEvent::BLOCK_OPEN
        }));
        if interaction == ExplosionInteraction::None {
            assert_eq!(world.block_state(DIM, pos), Some(before));
        } else {
            assert_eq!(
                world.block_state(DIM, pos),
                Some(block_state_id("minecraft:air"))
            );
        }
    }
}

fn silverfish_ids(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(DIM, id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SILVERFISH_ID)
        })
        .collect()
}

#[test]
fn exploded_tnt_block_keeps_its_chain_fuse_and_owner_through_entity_save_reload() {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let tnt_pos = BlockPos { x: 6, y: 100, z: 6 };
    assert!(world.set_block(
        DIM,
        tnt_pos,
        vanilla_block_default_state_id(VANILLA_BLOCK_TNT_ID).unwrap(),
        SET_BLOCK_UPDATE_ALL,
    ));
    let owner_uuid = Uuid::from_u128(0x1234);
    let center = Vec3d {
        x: 5.5,
        y: 100.5,
        z: 6.5,
    };
    world.explode(
        DIM,
        PendingExplosion {
            source: Some(ExplosionSource {
                direct: ExplosionSourceEntity {
                    custom_name: None,
                    entity_id: 73,
                    entity_resource_id: "minecraft:tnt".into(),
                    uuid: Uuid::from_u128(73),
                    position: center,
                    kind: ExplosionSourceEntityKind::Other,
                },
                indirect: Some(ExplosionSourceEntity {
                    custom_name: None,
                    entity_id: 74,
                    entity_resource_id: "minecraft:player".into(),
                    uuid: owner_uuid,
                    position: center,
                    kind: ExplosionSourceEntityKind::Player {
                        instabuild: false,
                        may_build: true,
                        spectator: false,
                    },
                }),
                causing: None,
                in_water: false,
                wind_charge: false,
            }),
            center,
            radius: 4.0,
            fire: false,
            interaction: ExplosionInteraction::Tnt,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut Vec::new(),
    );
    assert_eq!(
        world.block_state(DIM, tnt_pos),
        Some(block_state_id("minecraft:air"))
    );
    let tnt_ids: Vec<_> = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(DIM, id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_TNT_ID)
        })
        .collect();
    assert_eq!(tnt_ids.len(), 1);
    let chained = world.stored_entity_for_test(DIM, tnt_ids[0]).unwrap();
    let crate::entity::EntityData::Tnt(data) = chained.data() else {
        panic!("the chain callback spawned primed TNT");
    };
    let fuse = data.fuse();
    let chained_uuid = chained.uuid();
    assert!((10..=29).contains(&fuse));
    assert_eq!(data.owner(), Some(owner_uuid));

    let request = world
        .save_request(
            1_786_000_000_999,
            200,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let mut candidate = restore_candidate(200, None, Vec::new());
    super::entities::restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let restored_id = restored
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|&id| {
            restored
                .stored_entity_for_test(DIM, id)
                .is_some_and(|entity| entity.uuid() == chained_uuid)
        })
        .expect("the primed TNT survived the entity-region reload");
    let restored_entity = restored.stored_entity_for_test(DIM, restored_id).unwrap();
    let crate::entity::EntityData::Tnt(restored_data) = restored_entity.data() else {
        panic!("the restored entity is primed TNT");
    };
    assert_eq!(restored_data.fuse(), fuse);
    assert_eq!(restored_data.owner(), Some(owner_uuid));

    let (_sender, receiver) = mailbox();
    let mut exploded = false;
    for _ in 0..(fuse + 20) {
        restored.tick(receiver.drain());
        if restored
            .positioned_game_events_for_test(DIM)
            .iter()
            .any(|event| {
                event.recorded.game_event == VanillaGameEvent::EXPLODE
                    && event.recorded.source_entity_id == Some(restored_id)
            })
        {
            exploded = true;
            break;
        }
    }
    assert!(
        exploded,
        "the restored chain fuse continues to its explosion"
    );
    assert!(restored.stored_entity_for_test(DIM, restored_id).is_none());
}

#[test]
fn tnt_without_a_player_spawns_infested_silverfish_only_with_block_drops() {
    let infested = BlockPos { x: 6, y: 100, z: 6 };
    let center = Vec3d {
        x: 6.5,
        y: 100.5,
        z: 6.5,
    };
    for block_drops in [true, false] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(operator_config(&[("observer", 1)]));
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 8.5,
                y: 100.0,
                z: 6.5,
            },
        );
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("gamerule block_drops {block_drops}"),
        );
        assert_eq!(
            world.game_rule_for_test("minecraft:block_drops"),
            Some(GameRuleValue::Boolean(block_drops)),
        );
        assert!(world.set_block(
            DIM,
            infested,
            block_state_id("minecraft:infested_stone"),
            SET_BLOCK_UPDATE_ALL,
        ));
        let mut events = Vec::new();
        world.explode(
            DIM,
            PendingExplosion {
                source: Some(ExplosionSource {
                    direct: ExplosionSourceEntity {
                        custom_name: None,
                        entity_id: 73,
                        entity_resource_id: "minecraft:tnt".into(),
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
                radius: 4.0,
                fire: false,
                interaction: ExplosionInteraction::Tnt,
                damage_source: None,
                damage_profile: ExplosionDamageProfile::Default,
                presentation: ExplosionPresentation::Default,
            },
            &mut events,
        );
        events.extend(world.tick(receiver.drain()).events);
        assert_eq!(
            world.block_state(DIM, infested),
            Some(block_state_id("minecraft:air"))
        );
        let silverfish = silverfish_ids(&world);
        assert_eq!(silverfish.len(), usize::from(block_drops));
        let animations: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::EntityEvent(update),
                    ..
                } if update.event_id == 20 => Some(update.entity_id),
                _ => None,
            })
            .collect();
        assert_eq!(
            animations, silverfish,
            "spawnAnim follows the successful add"
        );
        for id in silverfish {
            let added = events.iter().position(|event| {
                matches!(
                    event,
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::AddEntity(packet),
                        ..
                    } if packet.id == id
                )
            });
            let animated = events.iter().position(|event| {
                matches!(
                    event,
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::EntityEvent(update),
                        ..
                    } if update.entity_id == id && update.event_id == 20
                )
            });
            assert!(added.zip(animated).is_some_and(|(a, b)| a < b));
        }
    }
}

#[test]
fn explode_vibration_reads_wool_before_the_same_explosion_removes_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    let sensor = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:sculk_sensor",
        &[
            ("sculk_sensor_phase".to_owned(), "inactive".to_owned()),
            ("power".to_owned(), "0".to_owned()),
            ("waterlogged".to_owned(), "false".to_owned()),
        ]
        .into_iter()
        .collect(),
    )
    .expect("sensor state");
    assert!(world.set_block(DIM, SENSOR, sensor, SET_BLOCK_UPDATE_ALL));
    let wool = block_state_id("minecraft:white_wool");
    for y in 63..=65 {
        for z in -1..=1 {
            assert!(world.set_block(DIM, BlockPos { x: 4, y, z }, wool, SET_BLOCK_UPDATE_ALL));
        }
    }
    world.tick(receiver.drain());
    *world
        .level_block_entity_for_test(DIM, SENSOR)
        .and_then(|entity| entity.sculk_sensor_mut())
        .expect("sensor block entity")
        .vibration_data_mut_for_test() = VibrationData::default();

    let center = Vec3d {
        x: 8.1,
        y: 64.5,
        z: 0.5,
    };
    explode_at(&mut world, center, 6.0, ExplosionInteraction::Block, false);
    let vibration = world
        .level_block_entity_for_test(DIM, SENSOR)
        .and_then(|entity| entity.sculk_sensor())
        .expect("sensor block entity")
        .vibration_data()
        .clone();
    let scheduled = vibration
        .selection_strategy()
        .current()
        .map(|(info, _)| info)
        .or(vibration.current_vibration().copied());
    assert!(
        scheduled.is_none_or(|info| info.game_event != "minecraft:explode"),
        "the old wool wall occludes EXPLODE"
    );
    assert_eq!(
        world.block_state(DIM, BlockPos { x: 4, y: 64, z: 0 }),
        Some(block_state_id("minecraft:air"))
    );
}

fn fire_world(seed: i64) -> ServerWorld {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    for x in -2..=2 {
        for z in -2..=2 {
            let support = if (x + z) & 1 == 0 {
                "minecraft:stone"
            } else {
                "minecraft:soul_soil"
            };
            assert!(world.set_block(
                DIM,
                BlockPos { x, y: 299, z },
                block_state_id(support),
                SET_BLOCK_UPDATE_ALL,
            ));
        }
    }
    world.random.reseed(seed);
    world
}

#[test]
fn keep_explosion_fire_uses_ordinary_and_soul_support() {
    let center = Vec3d {
        x: 0.5,
        y: 300.5,
        z: 0.5,
    };
    let mut without_fire = fire_world(0);
    let mut with_fire = fire_world(0);
    explode_at(
        &mut without_fire,
        center,
        4.0,
        ExplosionInteraction::None,
        false,
    );
    explode_at(
        &mut with_fire,
        center,
        4.0,
        ExplosionInteraction::None,
        true,
    );
    let mut ordinary = 0;
    let mut soul = 0;
    for x in -2..=2 {
        for z in -2..=2 {
            let pos = BlockPos { x, y: 300, z };
            let block_type = with_fire
                .block_state(DIM, pos)
                .and_then(vanilla_block_state_block_type_id);
            match block_type {
                Some(VANILLA_BLOCK_FIRE_ID) => ordinary += 1,
                Some(VANILLA_BLOCK_SOUL_FIRE_ID) => soul += 1,
                _ => {}
            }
            assert_eq!(
                without_fire.block_state(DIM, BlockPos { x, y: 299, z }),
                with_fire.block_state(DIM, BlockPos { x, y: 299, z }),
                "KEEP does not destroy supports",
            );
        }
    }
    assert!(ordinary > 0, "ordinary fire on stone");
    assert!(soul > 0, "soul fire on soul soil");
}
