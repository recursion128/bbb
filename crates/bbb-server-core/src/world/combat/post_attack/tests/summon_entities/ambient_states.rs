use std::ops::RangeInclusive;

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_WARPED_FUNGUS_ID,
    VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_for_resource_id, VANILLA_ENTITY_TYPE_BAT_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
};
use bbb_protocol::item_types::{VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_WARPED_FUNGUS_ID};
use bbb_protocol::packets::{
    AttackEntity, BlockPos, GameType, InteractEntity, InteractionHand, MovePlayer, PlayServerbound,
    PlayerRotation, SoundEvent, SoundEventHolder, SoundSource,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Vec3Ops;

use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::entity::RemovalReason;

fn block_state_id(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn entity_type_id(kind: &str) -> i32 {
    vanilla_entity_type_for_resource_id(&format!("minecraft:{kind}"))
        .unwrap()
        .id
}

fn ref_sound(name: &str) -> SoundEventHolder {
    SoundEventHolder::Reference {
        registry_id: vanilla_sound_event_registry_id(name).unwrap(),
    }
}

fn age_pitch(baby: bool) -> RangeInclusive<f32> {
    if baby {
        1.3..=1.7
    } else {
        0.8..=1.2
    }
}

fn fill_blocks(
    world: &mut ServerWorld,
    xs: RangeInclusive<i32>,
    ys: RangeInclusive<i32>,
    zs: RangeInclusive<i32>,
    state: i32,
) {
    for x in xs {
        for z in zs.clone() {
            for y in ys.clone() {
                let _ = world.set_block(OVERWORLD, BlockPos { x, y, z }, state, 2);
            }
        }
    }
}

fn build_arena(
    world: &mut ServerWorld,
    xs: RangeInclusive<i32>,
    zs: RangeInclusive<i32>,
    floor_y: i32,
    air_top: Option<i32>,
) {
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    fill_blocks(world, xs.clone(), floor_y..=floor_y, zs.clone(), stone);
    if let Some(top) = air_top {
        let air = block_state_id("minecraft:air");
        fill_blocks(world, xs, (floor_y + 1)..=top, zs, air);
    }
}

fn reload_nbt(
    world: &mut ServerWorld,
    id: i32,
    drop_keys: &[&str],
    add: &[(&str, NbtSummaryValue)],
) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(OVERWORLD, id)
        .unwrap()
    else {
        unreachable!()
    };
    entries.retain(|entry| !drop_keys.iter().any(|name| entry.name == *name));
    for (name, value) in add {
        entries.push(NbtSummaryEntry {
            name: (*name).into(),
            value: value.clone(),
        });
    }
    world.load_stored_entity_record_for_test(OVERWORLD, id, &entries);
    entries
}

fn set_nbt_silent(world: &mut ServerWorld, id: i32, silent: bool) {
    if silent {
        reload_nbt(
            world,
            id,
            &["Silent"],
            &[("Silent", NbtSummaryValue::Byte(1))],
        );
    } else {
        reload_nbt(world, id, &["Silent"], &[]);
    }
}

fn send_play(world: &mut ServerWorld, packet: PlayServerbound) -> Vec<crate::world::WorldEvent> {
    world
        .tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            crate::world::WorldCommand::Play {
                connection_id: PLAYER,
                packet,
            },
        )])
        .events
}

fn emit_spawn_sounds(world: &mut ServerWorld, id: i32, with_level: bool) -> Vec<SoundEvent> {
    let triggered = if with_level {
        world.with_level_and_entities(OVERWORLD, |level, entities| {
            entities.play_spawn_ambient(id, Some(level))
        })
    } else {
        world
            .level_mut(OVERWORLD)
            .entities
            .play_spawn_ambient(id, None)
    };
    assert!(triggered);
    world
        .tick(Vec::new())
        .events
        .into_iter()
        .filter_map(|event| match event {
            crate::world::WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } if connection_id == PLAYER => Some(sound),
            _ => None,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn assert_spawn_sound(
    world: &mut ServerWorld,
    id: i32,
    expected: &[SoundEventHolder],
    source: SoundSource,
    volume: f32,
    pitch: RangeInclusive<f32>,
    audible: bool,
    with_level: bool,
    ctx: &str,
) {
    let position = world
        .stored_entity_for_test(OVERWORLD, id)
        .unwrap()
        .position();
    let matching: Vec<_> = emit_spawn_sounds(world, id, with_level)
        .into_iter()
        .filter(|sound| expected.contains(&sound.sound))
        .collect();
    if audible {
        assert!(
            matching.iter().any(|sound| {
                sound.position == position
                    && sound.source == source
                    && sound.volume == volume
                    && pitch.contains(&sound.pitch)
            }),
            "missing ambient ({ctx}): {matching:?}"
        );
    } else {
        assert!(
            matching.is_empty(),
            "unexpected ambient ({ctx}): {matching:?}"
        );
    }
}

#[test]
fn wolf_initial_ambient_uses_published_age_sets_anger_and_absolute_health() {
    let (mut world, root) = animal_sound_world();
    build_arena(&mut world, 6..=14, 6..=10, 69, Some(74));
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: 8.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            ..position
        });
    world.tick_rate_mut().set_frozen(true);

    let prepare = |world: &mut ServerWorld, at, baby, angry, tame, health: f32| {
        let id = spawn(world, VANILLA_ENTITY_TYPE_WOLF_ID, at);
        let anger_end = if angry { world.game_time() + 10000 } else { -1 };
        let mut add = vec![
            ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("NoGravity", NbtSummaryValue::Byte(1)),
            ("Health", NbtSummaryValue::Float(health.to_bits())),
            (
                "attributes",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "id".into(),
                        value: NbtSummaryValue::String("minecraft:max_health".into()),
                    },
                    NbtSummaryEntry {
                        name: "base".into(),
                        value: NbtSummaryValue::Double(40.0_f64.to_bits()),
                    },
                ])]),
            ),
            (
                "sound_variant",
                NbtSummaryValue::String("aaa:custom".into()),
            ),
            ("Sitting", NbtSummaryValue::Byte(0)),
            ("anger_end_time", NbtSummaryValue::Long(anger_end)),
        ];
        if tame {
            add.push(("Owner", NbtSummaryValue::IntArray(vec![0, 0, 0, 1])));
        }
        reload_nbt(
            world,
            id,
            &[
                "Age",
                "NoAI",
                "NoGravity",
                "Silent",
                "Owner",
                "Health",
                "attributes",
                "anger_end_time",
                "AngerTime",
                "angry_at",
                "sound_variant",
                "Sitting",
                "InLove",
            ],
            &add,
        );
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health() && !body.is_silent());
        assert!(matches!(body.data(), EntityData::GeneralLiving(data)
            if data.is_baby() == baby && data.is_tame() == tame && data.is_no_ai()));
        assert_eq!(body.living().unwrap().health(), health);
        assert_eq!(body.living().unwrap().max_health(), 40.0);
        assert_eq!(
            crate::entity::ai::mob::target::anger::read(body)
                .unwrap()
                .is_angry(world.level(OVERWORLD).entities.game_time()),
            angry
        );
        id
    };

    let all_wolf_sounds: Vec<_> = [false, true]
        .into_iter()
        .flat_map(|age| {
            ["ambient", "growl", "pant", "whine"].map(move |s| expected_wolf_sound(age, s))
        })
        .collect();
    let hear = |world: &mut ServerWorld, id, baby, allowed: &[&str], audible| {
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        let pos = body.position();
        assert!(body.is_alive_with_health());
        assert!(world.levels[PLAYER].position().distance_to_sqr(pos) < 64.0);
        let allowed: Vec<_> = allowed
            .iter()
            .map(|s| expected_wolf_sound(baby, s))
            .collect();
        let sounds: Vec<_> = emit_spawn_sounds(world, id, false)
            .into_iter()
            .filter(|sound| all_wolf_sounds.contains(&sound.sound))
            .collect();
        if audible {
            assert!(!sounds.is_empty());
            assert!(
                sounds.iter().all(|sound| allowed.contains(&sound.sound)
                    && sound.position == pos
                    && sound.source == SoundSource::Neutral
                    && sound.volume == 0.4
                    && age_pitch(baby).contains(&sound.pitch)),
                "{sounds:?}"
            );
        } else {
            assert!(sounds.is_empty(), "Silent wolf emitted {sounds:?}");
        }
    };

    for (baby, angry, tame, health) in [
        (false, true, false, 8.0),
        (true, true, false, 8.0),
        (false, false, true, 8.0),
        (true, false, true, 8.0),
        (false, false, true, 20.0),
        (false, false, false, 8.0),
    ] {
        let id = prepare(&mut world, position, baby, angry, tame, health);
        world.tick(Vec::new());
        let allowed: &[&str] = if angry {
            &["growl"]
        } else if tame && health < 20.0 {
            &["ambient", "whine"]
        } else {
            &["ambient", "pant"]
        };
        hear(&mut world, id, baby, allowed, true);
        set_nbt_silent(&mut world, id, true);
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_silent());
        assert!(matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby() == baby));
        world.tick(Vec::new());
        hear(&mut world, id, baby, allowed, false);
        assert!(world.remove_stored_entity_for_test(OVERWORLD, id, RemovalReason::Discarded));
    }

    let parents = [
        prepare(&mut world, position, false, false, true, 40.0),
        prepare(
            &mut world,
            Vec3d { x: 9.5, ..position },
            false,
            false,
            true,
            40.0,
        ),
    ];
    for id in parents {
        reload_nbt(
            &mut world,
            id,
            &["NoAI", "NoGravity", "InLove"],
            &[
                ("NoAI", NbtSummaryValue::Byte(0)),
                ("NoGravity", NbtSummaryValue::Byte(0)),
                ("InLove", NbtSummaryValue::Int(600)),
            ],
        );
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(matches!(body.data(), EntityData::GeneralLiving(data)
            if data.is_tame() && !data.is_no_ai() && !data.is_baby()));
    }
    world.tick_rate_mut().set_frozen(false);
    let mut child = None;
    for _ in 0..300 {
        world.tick(Vec::new());
        child = world
            .level(OVERWORLD)
            .entities
            .ids()
            .into_iter()
            .find(|id| {
                !parents.contains(id)
                && world.stored_entity_for_test(OVERWORLD, *id).is_some_and(|body| {
                    body.entity_type_id() == VANILLA_ENTITY_TYPE_WOLF_ID
                        && matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby())
                })
            });
        if child.is_some() {
            break;
        }
    }
    let child = child.expect("actual wolf breeding produces a pup");
    world.tick_rate_mut().set_frozen(true);
    world.tick(Vec::new());
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(OVERWORLD, child)
        .unwrap()
    else {
        unreachable!()
    };
    assert!(saved.iter().any(|entry| {
        entry.name == "Owner" && entry.value == NbtSummaryValue::IntArray(vec![0, 0, 0, 1])
    }));
    assert!(saved.iter().any(|entry| matches!(&entry.value, NbtSummaryValue::String(name)
        if entry.name == "sound_variant"
            && name.with_utf8(|n| world.registries.wolf_sound_variant(n).is_some()).unwrap_or(false))));
    hear(&mut world, child, true, &["ambient", "pant"], true);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn creaking_initial_ambient_reads_actual_activation_and_respects_silent() {
    let mut world = world();
    build_arena(&mut world, 6..=14, 6..=10, 69, Some(74));
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: 8.5,
    };
    let watcher = Vec3d {
        x: 12.5,
        ..position
    };
    world.tick_rate_mut().set_frozen(true);
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(watcher);
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_CREAKING_ID, position);
    let witness = |world: &ServerWorld| {
        let player = &world.levels[PLAYER];
        assert_eq!(player.game_type, GameType::Survival);
        assert!(player.damage.health > 0.0 && !player.abilities.invulnerable);
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health());
        assert!(body.position().distance_to_sqr(player.position()) < 64.0);
    };
    witness(&world);
    let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
    assert!(!body.creaking_is_active() && !body.is_silent());
    assert!(matches!(body.data(), EntityData::GeneralLiving(data)
        if !data.is_no_ai() && !data.is_baby() && data.creaking_retained().is_some()));
    let expected = [ref_sound("minecraft:entity.creaking.ambient")];
    world.tick(Vec::new());
    assert_spawn_sound(
        &mut world,
        id,
        &expected,
        SoundSource::Hostile,
        1.0,
        0.8..=1.2,
        true,
        false,
        "inactive creaking",
    );

    set_nbt_silent(&mut world, id, true);
    witness(&world);
    let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
    assert!(body.is_silent() && !body.creaking_is_active());
    world.tick(Vec::new());
    assert_spawn_sound(
        &mut world,
        id,
        &expected,
        SoundSource::Hostile,
        1.0,
        0.8..=1.2,
        false,
        false,
        "silent creaking",
    );

    set_nbt_silent(&mut world, id, false);
    world.tick_rate_mut().set_frozen(false);
    send_play(
        &mut world,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(watcher),
            rotation: Some(PlayerRotation {
                y_rot: 90.0,
                x_rot: 0.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    for _ in 0..100 {
        if world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .creaking_is_active()
        {
            break;
        }
        world.tick(Vec::new());
    }
    witness(&world);
    let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
    assert!(body.creaking_is_active() && !body.is_silent());
    assert!(matches!(body.data(), EntityData::GeneralLiving(data) if !data.is_no_ai()));
    world.tick_rate_mut().set_frozen(true);
    world.tick(Vec::new());
    assert_spawn_sound(
        &mut world,
        id,
        &expected,
        SoundSource::Hostile,
        1.0,
        0.8..=1.2,
        false,
        false,
        "active creaking",
    );
    assert!(world
        .stored_entity_for_test(OVERWORLD, id)
        .unwrap()
        .creaking_is_active());
}

#[test]
fn saved_zombie_age_and_persistent_anger_select_initial_sound_and_respect_silent() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: 8.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 11.5,
            ..position
        });
    world.tick_rate_mut().set_frozen(true);
    for (kind, baby, angry) in [
        ("zombie_villager", false, false),
        ("zombie_villager", true, false),
        ("zombified_piglin", false, false),
        ("zombified_piglin", true, false),
        ("zombified_piglin", false, true),
        ("zombified_piglin", true, true),
    ] {
        let id = spawn(&mut world, entity_type_id(kind), position);
        let mut add = vec![
            ("IsBaby", NbtSummaryValue::Byte(i8::from(baby))),
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("NoGravity", NbtSummaryValue::Byte(1)),
        ];
        if kind == "zombified_piglin" {
            let end = if angry {
                world.level(OVERWORLD).entities.game_time() + 600
            } else {
                -1
            };
            add.push(("anger_end_time", NbtSummaryValue::Long(end)));
        }
        reload_nbt(
            &mut world,
            id,
            &[
                "IsBaby",
                "NoAI",
                "NoGravity",
                "Silent",
                "anger_end_time",
                "angry_at",
                "AngerTime",
            ],
            &add,
        );
        let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(entity.is_alive_with_health() && !entity.is_silent());
        assert!(matches!(entity.data(), EntityData::GeneralLiving(data)
            if data.zombie_retained().unwrap().is_baby() == baby && data.is_baby() == baby && data.is_no_ai()));
        if kind == "zombified_piglin" {
            assert_eq!(
                crate::entity::ai::mob::target::anger::read(entity)
                    .unwrap()
                    .is_angry(world.level(OVERWORLD).entities.game_time()),
                angry
            );
        }
        let expected = [ref_sound(&format!(
            "minecraft:entity.{kind}.{}",
            if angry { "angry" } else { "ambient" }
        ))];
        let pitch = if !baby {
            0.8..=1.2
        } else if kind == "zombie_villager" {
            1.8..=2.2
        } else {
            1.3..=1.7
        };
        let ctx = format!("{kind}, baby={baby}, angry={angry}");
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Hostile,
            1.0,
            pitch.clone(),
            true,
            false,
            &ctx,
        );
        set_nbt_silent(&mut world, id, true);
        assert!(world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .is_silent());
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Hostile,
            1.0,
            pitch,
            false,
            false,
            &ctx,
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn animal_ages_select_sound_sets_and_silent_animals_emit_no_spawn_sound() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: -4.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(position);
    world.tick_rate_mut().set_frozen(true);
    for (kind, baby) in [
        ("cow", true),
        ("chicken", true),
        ("pig", true),
        ("cat", true),
        ("happy_ghast", false),
        ("happy_ghast", true),
        ("parrot", false),
        ("parrot", true),
    ] {
        if kind == "parrot" {
            world
                .admin
                .set_difficulty(bbb_protocol::difficulties::Difficulty::Peaceful);
        }
        let id = spawn(&mut world, entity_type_id(kind), position);
        reload_nbt(
            &mut world,
            id,
            &["Age", "NoAI", "NoGravity", "Silent"],
            &[
                ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("NoGravity", NbtSummaryValue::Byte(1)),
            ],
        );
        let actual_baby = baby && kind != "parrot";
        assert!(matches!(
            world.stored_entity_for_test(OVERWORLD, id).unwrap().data(),
            EntityData::GeneralLiving(data) if data.is_baby() == actual_baby
        ));
        world.tick(Vec::new());
        let expected = [expected_animal_ambient(kind, actual_baby)];
        let (volume, pitch) = match kind {
            "happy_ghast" => (if baby { 1.0 } else { 4.0 }, 1.0..=1.0),
            "parrot" => (1.0, 0.8..=1.2),
            _ => (1.0, 1.3..=1.7),
        };
        let sounds = emit_spawn_sounds(&mut world, id, false);
        assert!(
            sounds.iter().any(|sound| {
                sound.sound == expected[0]
                    && sound.position == position
                    && pitch.contains(&sound.pitch)
                    && (!matches!(kind, "happy_ghast" | "parrot")
                        || (sound.source == SoundSource::Neutral && sound.volume == volume))
            }),
            "missing {kind} ambient, baby={baby}"
        );
        set_nbt_silent(&mut world, id, true);
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Neutral,
            volume,
            pitch,
            false,
            false,
            kind,
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reloaded_cat_and_panda_state_selects_spawn_ambient_and_respects_silent() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: -4.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(position);
    let owner_uuid = world.levels[PLAYER].profile.uuid;
    for (kind, state, baby, sounds) in [
        ("cat", "love", false, &["test:cat_adult_purr"][..]),
        (
            "cat",
            "tame",
            false,
            &["test:cat_adult_ambient", "test:cat_adult_purreow"][..],
        ),
        (
            "cat",
            "tame",
            true,
            &["test:cat_baby_ambient", "test:cat_baby_purreow"][..],
        ),
        (
            "panda",
            "aggressive",
            false,
            &["minecraft:entity.panda.aggressive_ambient"][..],
        ),
        (
            "panda",
            "worried",
            false,
            &["minecraft:entity.panda.worried_ambient"][..],
        ),
        (
            "panda",
            "weak",
            true,
            &["minecraft:entity.panda.ambient"][..],
        ),
    ] {
        let id = spawn(&mut world, entity_type_id(kind), position);
        let mut add = vec![
            ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("NoGravity", NbtSummaryValue::Byte(1)),
        ];
        if kind == "cat" {
            add.push(("Owner", crate::entity::save::uuid_int_array(owner_uuid)));
            add.push((
                "InLove",
                NbtSummaryValue::Int(if state == "love" { 600 } else { 0 }),
            ));
        } else {
            add.push(("MainGene", NbtSummaryValue::String(state.into())));
            add.push(("HiddenGene", NbtSummaryValue::String("aggressive".into())));
        }
        reload_nbt(
            &mut world,
            id,
            &[
                "Age",
                "NoAI",
                "NoGravity",
                "Silent",
                "Owner",
                "InLove",
                "MainGene",
                "HiddenGene",
            ],
            &add,
        );
        let expected: Vec<_> = sounds
            .iter()
            .map(|sound| {
                if kind == "cat" {
                    SoundEventHolder::Direct {
                        location: (*sound).into(),
                        fixed_range: Some(23.0),
                    }
                } else {
                    ref_sound(sound)
                }
            })
            .collect();
        let ctx = format!("{kind} {state}");
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Neutral,
            1.0,
            age_pitch(baby),
            true,
            false,
            &ctx,
        );
        set_nbt_silent(&mut world, id, true);
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Neutral,
            1.0,
            age_pitch(baby),
            false,
            false,
            &ctx,
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn age_water_playing_dead_and_bat_states_select_spawn_ambient_and_respect_silent() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: -4.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            y: 74.0,
            ..position
        });

    let check =
        |world: &mut ServerWorld, id: i32, name: &str, source: SoundSource, silent: bool| {
            let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
            let volume = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID {
                0.8
            } else {
                1.0
            };
            assert_spawn_sound(
                world,
                id,
                &[ref_sound(name)],
                source,
                volume,
                age_pitch(baby),
                !silent,
                false,
                name,
            );
        };

    for baby in [false, true] {
        let id = spawn(&mut world, VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, position);
        reload_nbt(
            &mut world,
            id,
            &["Age", "NoAI", "NoGravity", "Silent"],
            &[
                ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("NoGravity", NbtSummaryValue::Byte(1)),
            ],
        );
        assert!(matches!(
            world.stored_entity_for_test(OVERWORLD, id).unwrap().data(),
            EntityData::GeneralLiving(data) if data.is_baby() == baby
        ));
        let name = if baby {
            "minecraft:entity.polar_bear.ambient_baby"
        } else {
            "minecraft:entity.polar_bear.ambient"
        };
        check(&mut world, id, name, SoundSource::Neutral, false);
        set_nbt_silent(&mut world, id, true);
        check(&mut world, id, name, SoundSource::Neutral, true);
    }

    let water = block_state_id("minecraft:water");
    let bubble = block_state_id("minecraft:bubble_column");
    let magma = block_state_id("minecraft:magma_block");
    for (kind, dry, wet, source) in [
        (
            "skeleton_horse",
            "minecraft:entity.skeleton_horse.ambient",
            "minecraft:entity.skeleton_horse.ambient_water",
            SoundSource::Neutral,
        ),
        (
            "axolotl",
            "minecraft:entity.axolotl.idle_air",
            "minecraft:entity.axolotl.idle_water",
            SoundSource::Neutral,
        ),
        (
            "guardian",
            "minecraft:entity.guardian.ambient_land",
            "minecraft:entity.guardian.ambient",
            SoundSource::Hostile,
        ),
        (
            "elder_guardian",
            "minecraft:entity.elder_guardian.ambient_land",
            "minecraft:entity.elder_guardian.ambient",
            SoundSource::Hostile,
        ),
        (
            "drowned",
            "minecraft:entity.drowned.ambient",
            "minecraft:entity.drowned.ambient_water",
            SoundSource::Hostile,
        ),
        (
            "dolphin",
            "minecraft:entity.dolphin.ambient",
            "minecraft:entity.dolphin.ambient_water",
            SoundSource::Neutral,
        ),
    ] {
        build_arena(&mut world, 6..=10, -7..=-3, 68, Some(74));
        let id = spawn(&mut world, entity_type_id(kind), position);
        if kind == "skeleton_horse" {
            reload_nbt(
                &mut world,
                id,
                &["NoAI", "NoGravity"],
                &[
                    ("NoAI", NbtSummaryValue::Byte(1)),
                    ("NoGravity", NbtSummaryValue::Byte(1)),
                ],
            );
        }
        world.tick(Vec::new());
        assert!(
            !world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .is_in_water(),
            "dry {kind}"
        );
        check(&mut world, id, dry, source, false);
        if kind == "skeleton_horse" {
            fill_blocks(&mut world, 6..=10, 69..=70, -7..=-3, water);
            for _ in 0..8 {
                world.tick(Vec::new());
                if world
                    .stored_entity_for_test(OVERWORLD, id)
                    .unwrap()
                    .is_in_water()
                {
                    break;
                }
            }
            let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert!(entity.is_alive_with_health() && entity.is_in_water());
            assert!(!entity.is_eye_in_fluid(crate::entity::FluidTag::Water));
            check(&mut world, id, dry, source, false);
        }
        fill_blocks(&mut world, 6..=10, 69..=74, -7..=-3, water);
        for _ in 0..8 {
            world.tick(Vec::new());
            if world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .is_in_water()
            {
                break;
            }
        }
        assert!(
            world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .is_in_water(),
            "wet {kind}"
        );
        if kind == "skeleton_horse" {
            assert!(world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .is_eye_in_fluid(crate::entity::FluidTag::Water));
        }
        check(&mut world, id, wet, source, false);
        if kind == "skeleton_horse" {
            fill_blocks(&mut world, 6..=10, 68..=68, -7..=-3, magma);
            fill_blocks(&mut world, 6..=10, 69..=74, -7..=-3, bubble);
            world.tick(Vec::new());
            let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert!(
                entity.is_alive_with_health()
                    && entity.is_eye_in_fluid(crate::entity::FluidTag::Water)
            );
            let eye = BlockPos {
                x: entity.position().x.floor() as i32,
                y: (entity.position().y + f64::from(entity.eye_height())).floor() as i32,
                z: entity.position().z.floor() as i32,
            };
            assert_eq!(world.level(OVERWORLD).block_state(eye), bubble);
            check(&mut world, id, wet, source, false);
        }
        world
            .stored_entity_mut_for_test(OVERWORLD, id)
            .unwrap()
            .set_silent(true);
        check(&mut world, id, wet, source, true);
        if kind == "axolotl" {
            reload_nbt(
                &mut world,
                id,
                &["Brain", "NoAI", "Silent"],
                &[
                    (
                        "Brain",
                        bbb_protocol::nbt_summary::parse_snbt_compound(
                            "{memories:{'minecraft:play_dead_ticks':{value:200}}}",
                        )
                        .unwrap(),
                    ),
                    ("NoAI", NbtSummaryValue::Byte(0)),
                    ("Silent", NbtSummaryValue::Byte(0)),
                ],
            );
            assert!(crate::entity::axolotl::has_play_dead_ticks(
                world.stored_entity_for_test(OVERWORLD, id).unwrap()
            ));
            for _ in 0..20 {
                world.tick(Vec::new());
                if crate::entity::axolotl::is_playing_dead(
                    world.stored_entity_for_test(OVERWORLD, id).unwrap(),
                ) {
                    break;
                }
            }
            let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert!(entity.is_alive_with_health() && entity.is_in_water() && !entity.is_silent());
            assert!(crate::entity::axolotl::is_playing_dead(entity));
            world.tick_rate_mut().set_frozen(true);
            let names = [ref_sound(dry), ref_sound(wet)];
            assert_spawn_sound(
                &mut world,
                id,
                &names,
                SoundSource::Neutral,
                1.0,
                0.8..=1.2,
                false,
                false,
                "playing-dead axolotl",
            );
            world
                .stored_entity_mut_for_test(OVERWORLD, id)
                .unwrap()
                .set_silent(true);
            world.tick_rate_mut().set_frozen(false);
        }
    }

    world.tick_rate_mut().set_frozen(true);
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_BAT_ID, position);
    let bat_sound = [ref_sound("minecraft:entity.bat.ambient")];
    for flags in [0_i8, 1_i8] {
        reload_nbt(
            &mut world,
            id,
            &["BatFlags", "NoAI", "NoGravity", "Silent"],
            &[
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("NoGravity", NbtSummaryValue::Byte(1)),
                ("BatFlags", NbtSummaryValue::Byte(flags)),
            ],
        );
        let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        let EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!()
        };
        assert_eq!(data.bat_retained().unwrap().is_resting(), flags == 1);
        assert!(entity.is_alive_with_health() && !entity.is_silent());
        let pos = entity.position();
        let sounds = emit_spawn_sounds(&mut world, id, false);
        let mut heard = false;
        for sound in &sounds {
            if sound.sound == bat_sound[0] {
                heard = true;
                assert_eq!(sound.position, pos);
                assert_eq!(sound.source, SoundSource::Neutral);
                assert_eq!(sound.volume, 0.1);
                assert!((0.76..=1.14).contains(&sound.pitch));
            }
        }
        if flags == 0 {
            assert!(heard, "flying bat emitted no ambient");
        }
        world
            .stored_entity_mut_for_test(OVERWORLD, id)
            .unwrap()
            .set_silent(true);
        assert_spawn_sound(
            &mut world,
            id,
            &bat_sound,
            SoundSource::Neutral,
            0.1,
            0.76..=1.14,
            false,
            false,
            "silent bat",
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn idle_tempted_and_panicking_striders_select_spawn_ambient() {
    let (mut world, root) = animal_sound_world();
    build_arena(&mut world, 0..=15, 0..=15, 69, None);
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            y: 70.0,
            z: 8.5,
        });
    let ambient = [ref_sound("minecraft:entity.strider.ambient")];
    for behavior in ["idle", "tempt", "panic"] {
        world.tick_rate_mut().set_frozen(false);
        world.set_inventory_slot_for_test(PLAYER, 0, ItemStackSummary::empty());
        let start = Vec3d {
            x: 7.5,
            y: 70.0,
            z: 8.5,
        };
        let id = spawn(&mut world, VANILLA_ENTITY_TYPE_STRIDER_ID, start);
        let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        let uuid = entity.uuid();
        let health = entity.living().unwrap().health();
        if behavior == "tempt" {
            world.set_inventory_slot_for_test(
                PLAYER,
                0,
                ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_WARPED_FUNGUS_ID),
                    count: 1,
                    component_patch: Default::default(),
                },
            );
            assert_eq!(
                world.levels[PLAYER]
                    .inventory
                    .selected_stack()
                    .item_resource_id(),
                Some("minecraft:warped_fungus")
            );
        } else if behavior == "panic" {
            let outcome = world
                .hurt_stored_entity_by_command_for_test(
                    OVERWORLD,
                    id,
                    "minecraft:player_attack",
                    1.0,
                    None,
                )
                .unwrap();
            assert!(outcome.applied && outcome.succeeded);
        }
        if behavior != "idle" {
            let mut observed = false;
            for _ in 0..200 {
                world.tick(Vec::new());
                let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
                assert!(entity.is_alive_with_health());
                let runtime = world.level(OVERWORLD).entities.mob_ai(id).unwrap();
                let living = entity.living().unwrap();
                let moved = (entity.position().x - start.x).hypot(entity.position().z - start.z);
                let (active, speed) = if behavior == "tempt" {
                    (runtime.tempt_goal_running(), 1.4)
                } else {
                    (runtime.is_panicking(), 1.65)
                };
                if active
                    && moved > 0.25
                    && (f64::from(living.speed())
                        - speed * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
                    .abs()
                        < 1.0e-5
                {
                    observed = true;
                    break;
                }
            }
            assert!(observed, "no actual {behavior} movement established");
        }
        let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert_eq!(entity.uuid(), uuid);
        assert!(entity.is_alive_with_health() && !entity.is_silent());
        assert_eq!(
            entity.living().unwrap().health(),
            health - if behavior == "panic" { 1.0 } else { 0.0 }
        );
        world.tick_rate_mut().set_frozen(true);
        assert_spawn_sound(
            &mut world,
            id,
            &ambient,
            SoundSource::Neutral,
            1.0,
            0.8..=1.2,
            behavior == "idle",
            false,
            behavior,
        );
        if behavior == "idle" {
            world
                .stored_entity_mut_for_test(OVERWORLD, id)
                .unwrap()
                .set_silent(true);
            assert_spawn_sound(
                &mut world,
                id,
                &ambient,
                SoundSource::Neutral,
                1.0,
                0.8..=1.2,
                false,
                false,
                "silent strider",
            );
        }
        assert!(world.remove_stored_entity_for_test(OVERWORLD, id, RemovalReason::Discarded));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retained_equipment_shell_and_sleep_states_and_real_trading_select_spawn_ambient() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: -4.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(position);
    world.tick_rate_mut().set_frozen(true);
    for (kind, pose, held) in [
        ("allay", "idle", false),
        ("allay", "idle", true),
        ("armadillo", "idle", false),
        ("armadillo", "rolling", false),
        ("armadillo", "scared", false),
        ("armadillo", "unrolling", false),
        ("shulker", "closed", false),
        ("shulker", "open", false),
        ("villager", "awake", false),
        ("villager", "sleeping", false),
    ] {
        let id = spawn(&mut world, entity_type_id(kind), position);
        if held {
            let stack = crate::tests::native_stack(ItemStackSummary {
                item_id: Some(VANILLA_ITEM_DIAMOND_ID),
                count: 1,
                component_patch: Default::default(),
            });
            world
                .stored_entity_mut_for_test(OVERWORLD, id)
                .unwrap()
                .living_mut()
                .unwrap()
                .set_item_slot(EquipmentSlot::MainHand, Some(stack));
        }
        let mut add = vec![
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("NoGravity", NbtSummaryValue::Byte(1)),
        ];
        if kind == "armadillo" {
            add.push(("state", NbtSummaryValue::String(pose.into())));
        }
        if kind == "shulker" {
            world.set_block(
                OVERWORLD,
                BlockPos { x: 8, y: 69, z: -5 },
                block_state_id("minecraft:stone"),
                2,
            );
            add.push((
                "Peek",
                NbtSummaryValue::Byte(if pose == "closed" { 0 } else { 100 }),
            ));
        }
        if kind == "villager" && pose == "sleeping" {
            world.set_block(
                OVERWORLD,
                BlockPos { x: 8, y: 70, z: -5 },
                block_state_id("minecraft:red_bed"),
                2,
            );
            add.push(("sleeping_pos", NbtSummaryValue::IntArray(vec![8, 70, -5])));
        }
        reload_nbt(
            &mut world,
            id,
            &[
                "state",
                "Peek",
                "sleeping_pos",
                "NoAI",
                "NoGravity",
                "Silent",
            ],
            &add,
        );
        let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        match kind {
            "allay" => assert_eq!(
                entity
                    .living()
                    .unwrap()
                    .item_by_slot(EquipmentSlot::MainHand)
                    .is_some_and(|s| !s.is_empty()),
                held
            ),
            "armadillo" => assert_eq!(crate::entity::armadillo::is_scared(entity), pose != "idle"),
            "shulker" => assert!(matches!(entity.data(), EntityData::GeneralLiving(data)
                if data.shulker_state().unwrap().is_closed() == (pose == "closed"))),
            _ => assert_eq!(
                entity.sleeping_pos(),
                (pose == "sleeping").then_some(BlockPos { x: 8, y: 70, z: -5 })
            ),
        }
        let name = if matches!(kind, "armadillo" | "shulker" | "villager") {
            format!("minecraft:entity.{kind}.ambient")
        } else if held {
            "minecraft:entity.allay.ambient_with_item".into()
        } else {
            "minecraft:entity.allay.ambient_without_item".into()
        };
        let expected = [ref_sound(&name)];
        let audible = !matches!(
            pose,
            "rolling" | "scared" | "unrolling" | "closed" | "sleeping"
        );
        let volume = if kind == "allay" { 0.4 } else { 1.0 };
        let source = if kind == "shulker" {
            SoundSource::Hostile
        } else {
            SoundSource::Neutral
        };
        let ctx = format!("{kind}, pose={pose}, held={held}");
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            source,
            volume,
            0.8..=1.2,
            audible,
            false,
            &ctx,
        );
        if audible {
            set_nbt_silent(&mut world, id, true);
            assert_spawn_sound(
                &mut world,
                id,
                &expected,
                source,
                volume,
                0.8..=1.2,
                false,
                false,
                &ctx,
            );
        }
    }

    world.tick_rate_mut().set_frozen(false);
    let trade_position = Vec3d {
        x: 10.5,
        y: 70.0,
        z: 8.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            ..trade_position
        });
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = Default::default();
    let villager = spawn(&mut world, VANILLA_ENTITY_TYPE_VILLAGER_ID, trade_position);
    let NbtSummaryValue::Compound(saved) = bbb_protocol::nbt_summary::parse_snbt_compound(
        "{Age:0,NoAI:1b,NoGravity:1b,VillagerData:{type:'minecraft:plains',profession:'minecraft:librarian',level:1}}",
    )
    .unwrap() else {
        unreachable!()
    };
    let add: Vec<_> = saved.iter().map(|e| ("", e.clone())).collect();
    let NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(OVERWORLD, villager)
        .unwrap()
    else {
        unreachable!()
    };
    entries.retain(|entry| {
        ![
            "Age",
            "NoAI",
            "NoGravity",
            "Silent",
            "VillagerData",
            "Offers",
            "sleeping_pos",
        ]
        .contains(&entry.name.to_jdk_string().as_str())
    });
    entries.extend(add.into_iter().map(|(_, e)| e));
    world.load_stored_entity_record_for_test(OVERWORLD, villager, &entries);
    let entity = world.stored_entity_for_test(OVERWORLD, villager).unwrap();
    assert!(matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.villager_profession_and_level() == Some(("minecraft:librarian", 1))
            && !data.is_baby() && data.is_no_ai()));
    assert!(entity.sleeping_pos().is_none() && entity.merchant_offers().is_none());
    let opened = send_play(
        &mut world,
        PlayServerbound::Interact(InteractEntity {
            entity_id: villager,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let screen = opened
        .iter()
        .find_map(|event| match event {
            crate::world::WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::OpenScreen(screen),
            } if *connection_id == PLAYER => Some(screen),
            _ => None,
        })
        .expect("real villager interaction opens merchant menu");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_MERCHANT_ID
    );
    assert!(opened.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::MerchantOffers(offers),
        } if *connection_id == PLAYER && offers.container_id == screen.container_id && !offers.offers.is_empty()
    )));
    assert!(world.levels[PLAYER].inventory.is_merchant_menu());
    let trader = world.stored_entity_for_test(OVERWORLD, villager).unwrap();
    assert_eq!(
        trader.merchant_trading_player(),
        Some(world.levels[PLAYER].entity_id())
    );
    assert!(trader.is_alive_with_health() && !trader.is_silent());
    world.tick_rate_mut().set_frozen(true);
    let trade_sound = [ref_sound("minecraft:entity.villager.trade")];
    for silent in [false, true] {
        if silent {
            world
                .stored_entity_mut_for_test(OVERWORLD, villager)
                .unwrap()
                .set_silent(true);
        }
        assert_spawn_sound(
            &mut world,
            villager,
            &trade_sound,
            SoundSource::Neutral,
            1.0,
            0.8..=1.2,
            !silent,
            false,
            "trading villager",
        );
        assert!(world.levels[PLAYER].inventory.is_merchant_menu());
        assert_eq!(
            world
                .stored_entity_for_test(OVERWORLD, villager)
                .unwrap()
                .merchant_trading_player(),
            Some(world.levels[PLAYER].entity_id())
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reloaded_goat_and_horse_states_select_spawn_ambient_and_respect_silent() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.5,
        y: 70.0,
        z: -4.5,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(position);
    for (kind, baby, screaming) in [
        ("goat", false, false),
        ("goat", false, true),
        ("goat", true, false),
        ("goat", true, true),
        ("horse", false, false),
        ("horse", true, false),
    ] {
        let id = spawn(&mut world, entity_type_id(kind), position);
        let mut add = vec![
            ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("NoGravity", NbtSummaryValue::Byte(1)),
        ];
        if kind == "goat" {
            add.push((
                "IsScreamingGoat",
                NbtSummaryValue::Byte(i8::from(screaming)),
            ));
        }
        reload_nbt(
            &mut world,
            id,
            &["Age", "IsScreamingGoat", "NoAI", "NoGravity", "Silent"],
            &add,
        );
        let EntityData::GeneralLiving(data) =
            world.stored_entity_for_test(OVERWORLD, id).unwrap().data()
        else {
            unreachable!()
        };
        assert_eq!(data.is_baby(), baby);
        if kind == "goat" {
            assert_eq!(data.goat_retained().unwrap().is_screaming(), screaming);
        }
        let name = match (kind, baby, screaming) {
            ("goat", _, true) => "minecraft:entity.goat.screaming.ambient",
            ("goat", _, false) => "minecraft:entity.goat.ambient",
            ("horse", true, _) => "minecraft:entity.baby_horse.ambient",
            ("horse", false, _) => "minecraft:entity.horse.ambient",
            _ => unreachable!(),
        };
        let expected = [ref_sound(name)];
        let volume = if kind == "horse" { 0.8 } else { 1.0 };
        let ctx = format!("{kind}, baby={baby}, screaming={screaming}");
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Neutral,
            volume,
            age_pitch(baby),
            true,
            false,
            &ctx,
        );
        set_nbt_silent(&mut world, id, true);
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Neutral,
            volume,
            age_pitch(baby),
            false,
            false,
            &ctx,
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn zoglin_saved_age_and_actual_attack_target_select_initial_ambient() {
    let mut world = world();
    build_arena(&mut world, 6..=12, 6..=10, 69, None);
    let start = Vec3d {
        x: 8.5,
        y: 70.0,
        z: 8.5,
    };
    let player_id = world.player_entity_id_for_test(PLAYER);
    let player_uuid = world.levels[PLAYER].profile.uuid;
    let hear = |world: &mut ServerWorld, id, baby, name: &str| {
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health() && !body.is_silent());
        assert!(matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby() == baby));
        assert_spawn_sound(
            world,
            id,
            &[ref_sound(name)],
            SoundSource::Hostile,
            1.0,
            age_pitch(baby),
            true,
            false,
            name,
        );
    };
    for baby in [false, true] {
        world.tick_rate_mut().set_frozen(true);
        world
            .levels
            .player_mut(PLAYER)
            .unwrap()
            .set_position(Vec3d { x: 10.5, ..start });
        world.set_inventory_slot_for_test(PLAYER, 0, ItemStackSummary::empty());
        let id = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOGLIN_ID, start);
        reload_nbt(
            &mut world,
            id,
            &["IsBaby", "NoAI", "NoGravity", "Silent"],
            &[
                ("IsBaby", NbtSummaryValue::Byte(i8::from(baby))),
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("NoGravity", NbtSummaryValue::Byte(1)),
            ],
        );
        assert!(!world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .has_target());
        hear(&mut world, id, baby, "minecraft:entity.zoglin.ambient");
        set_nbt_silent(&mut world, id, true);
        assert!(world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .is_silent());
        assert_spawn_sound(
            &mut world,
            id,
            &[ref_sound("minecraft:entity.zoglin.ambient")],
            SoundSource::Hostile,
            1.0,
            age_pitch(baby),
            false,
            false,
            "silent zoglin",
        );
        reload_nbt(
            &mut world,
            id,
            &["NoAI", "Silent"],
            &[("NoAI", NbtSummaryValue::Byte(0))],
        );
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        let uuid = body.uuid();
        let health = body.living().unwrap().health();
        assert!(
            matches!(body.data(), EntityData::GeneralLiving(data) if !data.is_no_ai() && data.is_baby() == baby)
        );
        world.tick_rate_mut().set_frozen(false);
        send_play(
            &mut world,
            PlayServerbound::Attack(AttackEntity { entity_id: id }),
        );
        assert!(
            world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .living()
                .unwrap()
                .health()
                < health
        );
        let targets_player = |world: &ServerWorld| {
            world
                .level(OVERWORLD)
                .entities
                .mob_ai(id)
                .unwrap()
                .target_ref()
                .is_some_and(|target| target.entity_id == player_id && target.uuid == player_uuid)
        };
        for _ in 0..80 {
            if targets_player(&world) {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(
            targets_player(&world),
            "no actual Zoglin target, baby={baby}"
        );
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health());
        assert_eq!(body.uuid(), uuid);
        assert!(world.levels[PLAYER].damage.health > 0.0);
        world.tick_rate_mut().set_frozen(true);
        hear(&mut world, id, baby, "minecraft:entity.zoglin.angry");
        assert!(targets_player(&world));
        assert!(world.remove_stored_entity_for_test(OVERWORLD, id, RemovalReason::Discarded));
    }
}

#[test]
fn hoglin_initial_ambient_uses_current_conversion_and_actual_ai_state() {
    let mut world = world();
    build_arena(&mut world, 0..=20, 0..=16, 69, None);
    let start = Vec3d {
        x: 8.5,
        y: 70.0,
        z: 8.5,
    };
    let player_id = world.player_entity_id_for_test(PLAYER);
    let player_uuid = world.levels[PLAYER].profile.uuid;
    let prepare = |world: &mut ServerWorld, baby, no_ai, immune| {
        world.tick_rate_mut().set_frozen(true);
        world
            .levels
            .player_mut(PLAYER)
            .unwrap()
            .set_position(Vec3d { x: 10.5, ..start });
        world.set_inventory_slot_for_test(PLAYER, 0, ItemStackSummary::empty());
        let id = spawn(world, VANILLA_ENTITY_TYPE_HOGLIN_ID, start);
        reload_nbt(
            world,
            id,
            &[
                "Age",
                "NoAI",
                "NoGravity",
                "Silent",
                "IsImmuneToZombification",
            ],
            &[
                ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
                ("NoAI", NbtSummaryValue::Byte(i8::from(no_ai))),
                ("NoGravity", NbtSummaryValue::Byte(1)),
                (
                    "IsImmuneToZombification",
                    NbtSummaryValue::Byte(i8::from(immune)),
                ),
            ],
        );
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(matches!(body.data(), EntityData::GeneralLiving(data)
            if data.is_baby() == baby && data.is_no_ai() == no_ai && data.zombification().unwrap().immune == immune));
        assert!(!world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .has_target());
        id
    };
    let hear = |world: &mut ServerWorld, id, baby, suffix: &str| {
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health() && !body.is_silent());
        assert!(matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby() == baby));
        let pos = body.position();
        assert_eq!(
            world.with_level_and_entities(OVERWORLD, |level, _| level.piglins_zombify(pos)),
            Some(true)
        );
        let expected = [ref_sound(&format!("minecraft:entity.hoglin.{suffix}"))];
        assert_spawn_sound(
            world,
            id,
            &expected,
            SoundSource::Hostile,
            1.0,
            age_pitch(baby),
            true,
            true,
            suffix,
        );
    };
    for (baby, no_ai, immune, suffix) in [
        (false, false, false, "retreat"),
        (false, true, false, "ambient"),
        (true, true, false, "ambient"),
        (false, false, true, "ambient"),
    ] {
        let id = prepare(&mut world, baby, no_ai, immune);
        hear(&mut world, id, baby, suffix);
        set_nbt_silent(&mut world, id, true);
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_silent());
        let pos = body.position();
        assert_eq!(
            world.with_level_and_entities(OVERWORLD, |level, _| level.piglins_zombify(pos)),
            Some(true)
        );
        let expected = [ref_sound(&format!("minecraft:entity.hoglin.{suffix}"))];
        assert_spawn_sound(
            &mut world,
            id,
            &expected,
            SoundSource::Hostile,
            1.0,
            age_pitch(baby),
            false,
            true,
            suffix,
        );
        assert!(world.remove_stored_entity_for_test(OVERWORLD, id, RemovalReason::Discarded));
    }
    for baby in [false, true] {
        let id = prepare(&mut world, baby, false, true);
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        let uuid = body.uuid();
        let health = body.living().unwrap().health();
        world.tick_rate_mut().set_frozen(false);
        send_play(
            &mut world,
            PlayServerbound::Attack(AttackEntity { entity_id: id }),
        );
        assert!(
            world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .living()
                .unwrap()
                .health()
                < health
        );
        let selected = |world: &ServerWorld| {
            let store = &world.level(OVERWORLD).entities;
            let body = store.get(id).unwrap();
            let target = store.mob_ai(id).unwrap().target_ref();
            if baby {
                crate::entity::hoglin::retreat_target(body)
                    .is_some_and(|t| t.entity_id == player_id && t.uuid == player_uuid)
                    && crate::entity::hoglin::is_fleeing(body, target)
            } else {
                target.is_some_and(|t| t.entity_id == player_id && t.uuid == player_uuid)
            }
        };
        for _ in 0..80 {
            if selected(&world) {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(selected(&world), "actual hoglin combat state, baby={baby}");
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health());
        assert_eq!(body.uuid(), uuid);
        assert!(world.levels[PLAYER].damage.health > 0.0);
        world.tick_rate_mut().set_frozen(true);
        world.tick(Vec::new());
        hear(&mut world, id, baby, if baby { "retreat" } else { "angry" });
        assert!(selected(&world));
        assert!(world.remove_stored_entity_for_test(OVERWORLD, id, RemovalReason::Discarded));
    }
    let id = prepare(&mut world, false, false, true);
    let repellent = BlockPos { x: 9, y: 70, z: 8 };
    let fungus = vanilla_block_default_state_id(VANILLA_BLOCK_WARPED_FUNGUS_ID).unwrap();
    assert!(world.set_block(OVERWORLD, repellent, fungus, 2));
    world.tick_rate_mut().set_frozen(false);
    let repelled = |world: &ServerWorld| {
        let store = &world.level(OVERWORLD).entities;
        crate::entity::hoglin::nearest_repellent(store.get(id).unwrap()) == Some(repellent)
            && !store.mob_ai(id).unwrap().has_target()
    };
    for _ in 0..80 {
        if repelled(&world) {
            break;
        }
        world.tick(Vec::new());
    }
    assert!(repelled(&world), "actual tagged repellent sensor");
    assert!(world
        .stored_entity_for_test(OVERWORLD, id)
        .unwrap()
        .is_alive_with_health());
    world.tick_rate_mut().set_frozen(true);
    world.tick(Vec::new());
    hear(&mut world, id, false, "retreat");
    assert!(repelled(&world));
}

#[test]
fn turtle_initial_ambient_requires_actual_dry_adult_ground_contact() {
    let mut world = world();
    build_arena(&mut world, 0..=16, 0..=16, 69, None);
    let water = vanilla_block_default_state_id(VANILLA_BLOCK_WATER_ID).unwrap();
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            y: 70.0,
            z: 8.5,
        });
    let expected = [ref_sound("minecraft:entity.turtle.ambient_land")];
    let hear = |world: &mut ServerWorld, id, audible| {
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.is_alive_with_health());
        assert!(
            world.levels[PLAYER]
                .position()
                .distance_to_sqr(body.position())
                < 64.0
        );
        assert_spawn_sound(
            world,
            id,
            &expected,
            SoundSource::Neutral,
            1.0,
            0.8..=1.2,
            audible,
            false,
            "turtle",
        );
    };
    for baby in [true, false] {
        world.tick_rate_mut().set_frozen(true);
        let id = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            Vec3d {
                x: 8.5,
                y: 70.25,
                z: 8.5,
            },
        );
        reload_nbt(
            &mut world,
            id,
            &["Age", "NoAI"],
            &[
                ("Age", NbtSummaryValue::Int(if baby { -24000 } else { 0 })),
                ("NoAI", NbtSummaryValue::Byte(0)),
            ],
        );
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..80 {
            world.tick(Vec::new());
            if world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .on_ground()
            {
                break;
            }
        }
        world.tick_rate_mut().set_frozen(true);
        world.tick(Vec::new());
        reload_nbt(
            &mut world,
            id,
            &["NoAI"],
            &[("NoAI", NbtSummaryValue::Byte(1))],
        );
        let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        assert!(body.on_ground() && !body.is_in_water() && !body.is_silent());
        assert!(
            matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby() == baby && data.is_no_ai())
        );
        hear(&mut world, id, !baby);
        if !baby {
            set_nbt_silent(&mut world, id, true);
            let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert!(body.on_ground() && !body.is_in_water() && body.is_silent());
            hear(&mut world, id, false);
            set_nbt_silent(&mut world, id, false);
            fill_blocks(&mut world, 7..=10, 70..=70, 7..=10, water);
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..80 {
                world.tick(Vec::new());
                let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
                if body.on_ground() && body.is_in_water() {
                    break;
                }
            }
            world.tick_rate_mut().set_frozen(true);
            world.tick(Vec::new());
            let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert!(body.on_ground() && body.is_in_water() && !body.is_silent());
            hear(&mut world, id, false);
        }
        assert!(world.remove_stored_entity_for_test(OVERWORLD, id, RemovalReason::Discarded));
    }
}

#[test]
fn nautilus_initial_ambient_reads_actual_eye_water_and_saved_age() {
    let mut world = world();
    build_arena(&mut world, 0..=16, 0..=16, 69, None);
    let water = vanilla_block_default_state_id(VANILLA_BLOCK_WATER_ID).unwrap();
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            y: 70.0,
            z: 8.5,
        });
    let position = Vec3d {
        x: 8.5,
        y: 70.875,
        z: 8.5,
    };
    world.tick_rate_mut().set_frozen(true);
    let mut actors = Vec::new();
    for (kind, age, baby) in [
        ("nautilus", 0, false),
        ("nautilus", -24000, true),
        ("zombie_nautilus", -24000, false),
    ] {
        let id = spawn(&mut world, entity_type_id(kind), position);
        reload_nbt(
            &mut world,
            id,
            &["Age", "NoAI", "NoGravity", "Silent"],
            &[
                ("Age", NbtSummaryValue::Int(age)),
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("NoGravity", NbtSummaryValue::Byte(1)),
            ],
        );
        actors.push((id, kind, baby));
    }
    for depth in 0..=2 {
        if depth != 0 {
            let ys = if depth == 1 { 70..=70 } else { 71..=74 };
            fill_blocks(&mut world, 7..=10, ys, 7..=10, water);
        }
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..8 {
            world.tick(Vec::new());
            if actors.iter().all(|&(id, _, _)| {
                let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
                body.is_in_water() == (depth != 0) && body.is_under_water() == (depth == 2)
            }) {
                break;
            }
        }
        world.tick_rate_mut().set_frozen(true);
        world.tick(Vec::new());
        for &(id, kind, baby) in &actors {
            let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert!(body.is_alive_with_health() && !body.is_silent());
            assert_eq!(body.is_in_water(), depth != 0);
            assert_eq!(body.is_under_water(), depth == 2);
            assert_eq!(
                body.is_eye_in_fluid(crate::entity::FluidTag::Water),
                depth == 2
            );
            assert!(
                matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby() == baby && data.is_no_ai())
            );
            assert!(
                world.levels[PLAYER]
                    .position()
                    .distance_to_sqr(body.position())
                    < 64.0
            );
            let sound_kind = if baby { "baby_nautilus" } else { kind };
            let suffix = if depth == 2 {
                "ambient"
            } else {
                "ambient_land"
            };
            let expected = [ref_sound(&format!(
                "minecraft:entity.{sound_kind}.{suffix}"
            ))];
            let ctx = format!("{kind} baby={baby} depth={depth}");
            assert_spawn_sound(
                &mut world,
                id,
                &expected,
                SoundSource::Neutral,
                1.0,
                age_pitch(baby),
                true,
                false,
                &ctx,
            );
        }
    }
}
