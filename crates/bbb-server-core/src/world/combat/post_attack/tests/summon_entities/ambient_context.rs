use super::*;
use crate::world::{TickResult, WorldEvent};
use bbb_protocol::packets::{BlockPos, SoundEventHolder, SoundSource};
use bbb_protocol::NbtSummaryValue;

const POSITION: Vec3d = Vec3d {
    x: 8.5,
    y: 70.0,
    z: 8.5,
};

fn arena() -> ServerWorld {
    let mut world = world();
    world.tick_rate_mut().set_frozen(true);
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    for x in 5..=29 {
        for z in 6..=10 {
            let _ = world.set_block(OVERWORLD, BlockPos { x, y: 69, z }, stone, 2);
            for y in 70..=74 {
                let _ = world.set_block(OVERWORLD, BlockPos { x, y, z }, air, 2);
            }
        }
    }
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            ..POSITION
        });
    world
}

fn command(world: &mut ServerWorld, operation: &mut u64, text: &str) -> TickResult {
    *operation += 1;
    let output = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
        world.last_command_sequence + 1,
        crate::mailbox::WorldProducerClass::Admin,
        crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: *operation,
                request_id: format!("{operation}:{text}"),
                principal: "agent".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 1_755_000_000_000,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: text.into(),
            },
        },
    )]);
    let report = output
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Admin(outcome) => match &outcome.result {
                Ok(crate::admin::AdminResult::Command(report)) => Some(report),
                _ => None,
            },
            _ => None,
        })
        .expect("the native command answers with a command report");
    assert!(report.result.is_some(), "{text}: {report:?}");
    output
}

fn merge(world: &mut ServerWorld, id: i32, nbt: &str) {
    let NbtSummaryValue::Compound(mut saved) = world
        .stored_entity_command_data_for_test(OVERWORLD, id)
        .unwrap()
    else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(changes) =
        bbb_protocol::nbt_summary::parse_snbt_compound(nbt).unwrap()
    else {
        unreachable!()
    };
    saved.retain(|field| !changes.iter().any(|change| change.name == field.name));
    saved.extend(changes);
    world.load_stored_entity_record_for_test(OVERWORLD, id, &saved);
}

fn mob(world: &mut ServerWorld, kind: &str, nbt: &str) -> i32 {
    let type_id = bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(kind)
        .unwrap()
        .id;
    let id = spawn(world, type_id, POSITION);
    merge(world, id, nbt);
    id
}

fn discard(world: &mut ServerWorld, id: i32) {
    assert!(world.remove_stored_entity_for_test(
        OVERWORLD,
        id,
        crate::entity::RemovalReason::Discarded
    ));
}

fn hear(
    world: &mut ServerWorld,
    id: i32,
    family: &str,
    allowed: &[(&str, f32)],
    source: SoundSource,
) {
    world.tick_rate_mut().set_frozen(true);
    world.tick(Vec::new());
    let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
    assert!(body.is_alive_with_health());
    let position = body.position();
    let baby = matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let pitch = if baby { 1.3..=1.7 } else { 0.8..=1.2 };
    assert!(
        world.with_level_and_entities(OVERWORLD, |level, store| store
            .play_spawn_ambient(id, Some(level)))
    );
    let output = world.tick(Vec::new());
    let sounds: Vec<_> = output
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: crate::connection::ClientboundPlayPacket::Sound(sound),
            } if *connection_id == PLAYER => match &sound.sound {
                SoundEventHolder::Reference { registry_id } => {
                    let name = bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(
                        *registry_id,
                    )?;
                    name.strip_prefix("minecraft:entity.")?
                        .starts_with(family)
                        .then_some((name, sound))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    if allowed.is_empty() {
        assert!(
            sounds.is_empty(),
            "unexpected {family} initial sound: {sounds:?}"
        );
    } else {
        assert!(
            sounds.iter().any(|(name, sound)| {
                allowed
                    .iter()
                    .any(|(expected, volume)| name == expected && sound.volume == *volume)
                    && sound.source == source
                    && sound.position == position
                    && pitch.contains(&sound.pitch)
            }),
            "missing {family} initial sound {allowed:?}: {sounds:?}"
        );
        assert!(sounds.iter().all(|(name, sound)| allowed
            .iter()
            .any(|(expected, volume)| name == expected && sound.volume == *volume)));
    }
}

#[test]
fn fox_initial_ambient_reads_brightness_sleep_and_actual_nearby_players() {
    let mut world = arena();
    let mut operation = 10000;
    let ambient = [("minecraft:entity.fox.ambient", 1.0)];
    let night = [
        ("minecraft:entity.fox.ambient", 1.0),
        ("minecraft:entity.fox.screech", 2.0),
    ];
    for baby in [false, true] {
        let id = mob(
            &mut world,
            "minecraft:fox",
            &format!(
                "{{NoAI:1b,NoGravity:1b,Age:{},Sleeping:0b}}",
                if baby { -24000 } else { 0 }
            ),
        );
        assert!(
            matches!(world.stored_entity_for_test(OVERWORLD, id).unwrap().data(), EntityData::GeneralLiving(data) if data.is_baby() == baby)
        );
        command(&mut world, &mut operation, "gamemode spectator alice");
        command(&mut world, &mut operation, "time set day");
        assert_eq!(world.is_bright_outside(&OVERWORLD.into()), Some(true));
        hear(&mut world, id, "fox.", &ambient, SoundSource::Neutral);
        command(&mut world, &mut operation, "time set midnight");
        assert_eq!(world.is_bright_outside(&OVERWORLD.into()), Some(false));
        hear(&mut world, id, "fox.", &night, SoundSource::Neutral);
        command(&mut world, &mut operation, "gamemode survival alice");
        hear(&mut world, id, "fox.", &ambient, SoundSource::Neutral);
        merge(&mut world, id, "{Sleeping:1b}");
        assert!(
            matches!(world.stored_entity_for_test(OVERWORLD, id).unwrap().data(), EntityData::GeneralLiving(data) if data.fox_retained().unwrap().is_sleeping())
        );
        hear(
            &mut world,
            id,
            "fox.",
            &[("minecraft:entity.fox.sleep", 1.0)],
            SoundSource::Neutral,
        );
        merge(&mut world, id, "{Silent:1b}");
        assert!(world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .is_silent());
        hear(&mut world, id, "fox.", &[], SoundSource::Neutral);
        discard(&mut world, id);
    }
}

#[test]
fn piglin_initial_ambient_reads_conversion_hands_repellent_and_nonidle_owners() {
    let mut world = arena();
    let mut operation = 10000;
    command(
        &mut world,
        &mut operation,
        "item replace entity alice armor.head with minecraft:golden_helmet",
    );
    for (flags, sound) in [
        (
            "NoAI:0b,IsImmuneToZombification:0b",
            "minecraft:entity.piglin.retreat",
        ),
        (
            "NoAI:1b,IsImmuneToZombification:0b",
            "minecraft:entity.piglin.ambient",
        ),
        (
            "NoAI:0b,IsImmuneToZombification:1b",
            "minecraft:entity.piglin.ambient",
        ),
        (
            "NoAI:1b,IsImmuneToZombification:1b,IsBaby:1b",
            "minecraft:entity.piglin.ambient",
        ),
    ] {
        let id = mob(
            &mut world,
            "minecraft:piglin",
            &format!("{{{flags},NoGravity:1b,CannotHunt:1b}}"),
        );
        assert!(
            matches!(world.stored_entity_for_test(OVERWORLD, id).unwrap().data(), EntityData::GeneralLiving(data) if data.is_baby() == flags.contains("IsBaby:1b"))
        );
        assert_eq!(
            world.with_level_and_entities(OVERWORLD, |level, _| level.piglins_zombify(POSITION)),
            Some(true)
        );
        assert!(!world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .has_target());
        hear(
            &mut world,
            id,
            "piglin.",
            &[(sound, 1.0)],
            SoundSource::Hostile,
        );
        discard(&mut world, id);
    }
    for hand in ["mainhand", "offhand"] {
        command(
            &mut world,
            &mut operation,
            "item replace entity alice weapon.mainhand with minecraft:air",
        );
        command(
            &mut world,
            &mut operation,
            "item replace entity alice weapon.offhand with minecraft:air",
        );
        command(
            &mut world,
            &mut operation,
            &format!("item replace entity alice weapon.{hand} with minecraft:gold_ingot"),
        );
        world
            .levels
            .player_mut(PLAYER)
            .unwrap()
            .set_position(Vec3d {
                x: 23.5,
                ..POSITION
            });
        let id = mob(&mut world, "minecraft:piglin", "{NoAI:0b,NoGravity:1b,IsImmuneToZombification:1b,CannotHunt:1b,attributes:[{id:\"minecraft:follow_range\",base:32d}]}");
        hear(
            &mut world,
            id,
            "piglin.",
            &[("minecraft:entity.piglin.jealous", 1.0)],
            SoundSource::Hostile,
        );
        command(&mut world, &mut operation, "gamemode spectator alice");
        hear(
            &mut world,
            id,
            "piglin.",
            &[("minecraft:entity.piglin.ambient", 1.0)],
            SoundSource::Hostile,
        );
        command(&mut world, &mut operation, "gamemode survival alice");
        merge(&mut world, id, "{Silent:1b}");
        assert!(world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .is_silent());
        hear(&mut world, id, "piglin.", &[], SoundSource::Hostile);
        discard(&mut world, id);
    }
    command(
        &mut world,
        &mut operation,
        "item replace entity alice weapon.mainhand with minecraft:air",
    );
    command(
        &mut world,
        &mut operation,
        "item replace entity alice weapon.offhand with minecraft:air",
    );
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(Vec3d {
            x: 12.5,
            ..POSITION
        });
    let id = mob(&mut world, "minecraft:piglin", "{NoAI:0b,NoGravity:1b,IsImmuneToZombification:1b,CannotHunt:1b,attributes:[{id:\"minecraft:movement_speed\",base:0d}]}");
    command(
        &mut world,
        &mut operation,
        "setblock 16 74 8 minecraft:soul_campfire[lit=true]",
    );
    let repellent = BlockPos { x: 16, y: 74, z: 8 };
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..80 {
        if world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .piglin()
            .unwrap()
            .nearest_repellent()
            == Some(repellent)
        {
            break;
        }
        world.tick(Vec::new());
    }
    assert_eq!(
        world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .piglin()
            .unwrap()
            .nearest_repellent(),
        Some(repellent)
    );
    assert!(!world
        .level(OVERWORLD)
        .entities
        .mob_ai(id)
        .unwrap()
        .has_target());
    hear(
        &mut world,
        id,
        "piglin.",
        &[("minecraft:entity.piglin.retreat", 1.0)],
        SoundSource::Hostile,
    );
    discard(&mut world, id);
    command(&mut world, &mut operation, "setblock 16 74 8 minecraft:air");
    let id = mob(&mut world, "minecraft:piglin", "{NoAI:1b,NoGravity:1b,IsImmuneToZombification:1b,Brain:{memories:{\"minecraft:admiring_item\":{value:1b,ttl:119L}}}}");
    assert!(crate::entity::piglin::is_admiring(
        world.stored_entity_for_test(OVERWORLD, id).unwrap()
    ));
    hear(&mut world, id, "piglin.", &[], SoundSource::Hostile);
    discard(&mut world, id);
    for kind in ["minecraft:piglin", "minecraft:piglin_brute"] {
        let id = mob(
            &mut world,
            kind,
            "{NoAI:1b,NoGravity:1b,IsImmuneToZombification:1b,CannotHunt:1b}",
        );
        let family = if kind.ends_with("brute") {
            "piglin_brute."
        } else {
            "piglin."
        };
        hear(
            &mut world,
            id,
            family,
            &[(
                if kind.ends_with("brute") {
                    "minecraft:entity.piglin_brute.ambient"
                } else {
                    "minecraft:entity.piglin.ambient"
                },
                1.0,
            )],
            SoundSource::Hostile,
        );
        command(
            &mut world,
            &mut operation,
            "item replace entity alice armor.head with minecraft:air",
        );
        world
            .levels
            .player_mut(PLAYER)
            .unwrap()
            .set_position(Vec3d { x: 9.5, ..POSITION });
        merge(
            &mut world,
            id,
            "{NoAI:0b,attributes:[{id:\"minecraft:movement_speed\",base:0d}]}",
        );
        let health = world.levels[PLAYER].damage.health;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..100 {
            if world.levels[PLAYER].damage.health < health {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(
            world.levels[PLAYER].damage.health > 0.0 && world.levels[PLAYER].damage.health < health
        );
        let target = world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .target_ref()
            .unwrap();
        assert_eq!(target.uuid, world.levels[PLAYER].profile.uuid);
        assert_eq!(target.entity_id, world.player_entity_id_for_test(PLAYER));
        hear(&mut world, id, family, &[], SoundSource::Hostile);
        assert!(world
            .level(OVERWORLD)
            .entities
            .mob_ai(id)
            .unwrap()
            .has_target());
        discard(&mut world, id);
    }
}

#[test]
fn warden_initial_ambient_uses_resolved_saved_anger_and_actual_lifecycle_poses() {
    let mut world = arena();
    let mut operation = 10000;
    for (anger, sound) in [
        (0, "minecraft:entity.warden.ambient"),
        (50, "minecraft:entity.warden.agitated"),
        (100, "minecraft:entity.warden.angry"),
    ] {
        let id = mob(
            &mut world,
            "minecraft:warden",
            &format!(
                "{{NoAI:1b,NoGravity:1b,anger:{{suspects:[{{uuid:[I;0,0,0,1],anger:{anger}}}]}}}}"
            ),
        );
        assert_eq!(world.levels[PLAYER].profile.uuid, uuid::Uuid::from_u128(1));
        hear(
            &mut world,
            id,
            "warden.",
            &[(sound, 4.0)],
            SoundSource::Hostile,
        );
        if anger == 100 {
            command(&mut world, &mut operation, "gamemode spectator alice");
            hear(
                &mut world,
                id,
                "warden.",
                &[("minecraft:entity.warden.ambient", 4.0)],
                SoundSource::Hostile,
            );
            command(&mut world, &mut operation, "gamemode survival alice");
        }
        merge(&mut world, id, "{Silent:1b}");
        assert!(world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .is_silent());
        hear(&mut world, id, "warden.", &[], SoundSource::Hostile);
        discard(&mut world, id);
    }
    let id = world
        .spawn_finalized_entity(
            OVERWORLD.into(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID,
            POSITION,
            crate::entity::SpawnReason::Triggered,
            None,
        )
        .unwrap();
    assert_eq!(
        world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_EMERGING_ID
    );
    hear(&mut world, id, "warden.", &[], SoundSource::Hostile);
    discard(&mut world, id);
    for roaring in [true, false] {
        let flags = if roaring {
            "{NoAI:0b,NoGravity:1b,anger:{suspects:[{uuid:[I;0,0,0,1],anger:100}]},attributes:[{id:\"minecraft:movement_speed\",base:0d}],Brain:{memories:{\"minecraft:dig_cooldown\":{value:{},ttl:1200L}}}}"
        } else {
            "{NoAI:0b,NoGravity:0b,Brain:{memories:{}}}"
        };
        let id = mob(&mut world, "minecraft:warden", flags);
        let pose = if roaring {
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_ROARING_ID
        } else {
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DIGGING_ID
        };
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..100 {
            if world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .pose_id()
                == pose
            {
                break;
            }
            world.tick(Vec::new());
        }
        assert_eq!(
            world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .pose_id(),
            pose
        );
        if !roaring {
            assert!(world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .on_ground());
        }
        hear(&mut world, id, "warden.", &[], SoundSource::Hostile);
        discard(&mut world, id);
    }
}
