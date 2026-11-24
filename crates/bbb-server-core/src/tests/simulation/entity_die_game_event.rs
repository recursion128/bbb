//! `LivingEntity.die`'s `this.gameEvent(GameEvent.ENTITY_DIE)`
//! raised by a real death and delivered by the
//! tick-end flush: the three production entries a stored mob dies through
//! (combat, environment damage and a command), the recorder with no listener
//! to deliver to, and the one-event-per-death guarantee.

use super::*;

use super::block_state_id as plain_block_state;
use super::helpers::{at, stage_one_blow_from_death};
use crate::entity::{EntityData, GeneralLivingData};
use crate::level_event::{RecordedGameEvent, VanillaGameEvent};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

/// The mob's own position, `Vec3.pos` for the event `Entity.gameEvent` raises
/// (`Entity.java:1331-1337`).
const MOB_POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

/// The containing block retained in `RecordedGameEvent::pos` beside the raw
/// event origin.
const MOB_BLOCK: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

fn spawn_zombie(world: &mut ServerWorld) -> i32 {
    helpers::spawn_living_at(world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, MOB_POSITION).0
}

fn is_dead_or_dying(world: &ServerWorld, mob: i32) -> bool {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, mob)
        .and_then(crate::entity::Entity::living)
        .is_some_and(crate::entity::LivingEntityState::is_dead_or_dying)
}

/// The `ENTITY_DIE` rows the level recorded, oldest first.
fn entity_die_rows(world: &ServerWorld) -> Vec<RecordedGameEvent> {
    world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .into_iter()
        .filter(|row| row.game_event == VanillaGameEvent::ENTITY_DIE)
        .collect()
}

/// Every `EntityEvent.DEATH` broadcast a tick sent, as (recipient, entity):
/// `LivingEntity.die`'s `broadcastEntityEvent(this, (byte)3)`
/// through `ServerLevel.broadcastEntityEvent`
fn death_broadcasts(tick: &TickResult) -> Vec<(ConnectionId, i32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::EntityEvent(update),
            } if update.event_id == crate::entity::living::ENTITY_EVENT_DEATH => {
                Some((*connection_id, update.entity_id))
            }
            _ => None,
        })
        .collect()
}

/// The one row a death at `mob`'s own position records: `GameEvent.Context.of`
/// puts the dying entity in the source slot and leaves the affected state null
fn entity_die_row(mob: i32) -> RecordedGameEvent {
    RecordedGameEvent {
        game_event: VanillaGameEvent::ENTITY_DIE,
        pos: MOB_BLOCK,
        source_entity_id: Some(mob),
        affected_state: None,
    }
}

/// A world with a loaded session beside the mob: the fixture every real death
/// here starts from.
fn loaded_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    helpers::joined_world(datapack_config())
}

/// `LivingEntity.die`: the player kill raises
/// one `ENTITY_DIE` inside the emission gate and broadcasts the death
/// animation outside it, which reaches the mob's tracker.
#[test]
fn a_player_kill_raises_one_entity_die_and_then_the_death_broadcast() {
    let (sender, receiver, mut world) = loaded_world();
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let zombie = spawn_zombie(&mut world);
    // The pairing `seen_by` needs before the broadcast can reach the session.
    world.tick(receiver.drain());
    stage_one_blow_from_death(&mut world, zombie);

    let killed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: zombie }),
    );
    assert!(is_dead_or_dying(&world, zombie), "the blow was lethal");
    assert_eq!(entity_die_rows(&world), vec![entity_die_row(zombie)]);
    assert_eq!(death_broadcasts(&killed), vec![(ConnectionId(1), zombie)]);
}

/// `LivingEntity.baseTick`'s fire arm over
/// `hurt_from_environment`: an environment death raises the same event, with
/// the same source, and broadcasts the death animation with it.
#[test]
fn an_environment_kill_raises_entity_die_with_the_mob_as_its_source() {
    let (sender, receiver, mut world) = loaded_world();
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let zombie = spawn_zombie(&mut world);
    world.tick(receiver.drain());
    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the mob is stored");
    crate::entity::living::set_health(entity, 0.1);
    // `Entity.igniteForSeconds`; the next base tick is
    // one where `remainingFireTicks % 20 == 0`.
    entity.ignite_for_seconds(5.0);

    let died = world.tick(receiver.drain());
    assert!(is_dead_or_dying(&world, zombie), "the fire was lethal");
    assert_eq!(entity_die_rows(&world), vec![entity_die_row(zombie)]);
    assert_eq!(death_broadcasts(&died), vec![(ConnectionId(1), zombie)]);
}

/// A second `/kill` on a dying mob and the removal `tickDeath` ends with
/// raise nothing more: the hurt refuses a target
/// that is already dead or dying, so the transition is reached once, and the
/// death broadcast goes out once with it.
#[test]
fn a_second_kill_and_the_removal_tick_do_not_raise_entity_die_again() {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = datapack_config().datapacks;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let zombie = spawn_zombie(&mut world);
    world.tick(receiver.drain());

    let kill = "kill @e[type=minecraft:zombie]";
    let killed = chat_command(&sender, &receiver, &mut world, 1, kill);
    assert!(is_dead_or_dying(&world, zombie));
    assert_eq!(death_broadcasts(&killed), vec![(ConnectionId(1), zombie)]);
    let again = chat_command(&sender, &receiver, &mut world, 1, kill);
    assert!(death_broadcasts(&again).is_empty());
    for _ in 0..25 {
        let quiet = world.tick(receiver.drain());
        assert!(death_broadcasts(&quiet).is_empty());
    }
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
            .is_none(),
        "the body was removed"
    );
    assert_eq!(entity_die_rows(&world), vec![entity_die_row(zombie)]);
}

// ---- the kill overrides ----------------------------------------------------
//
// `Entity.kill` is not the only body that owes
// `ENTITY_DIE`: every override calls through `super`, and three of them run it
// after their own body already made the removal.

use crate::entity::{AreaEffectCloudData, BoatData, PaintingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;

use crate::tests::entities::{console_command, console_command_answer};

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// A session standing at `position`, watching the column the fixtures sit in.
fn world_beside(position: Vec3d) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = loaded_world();
    stand_at(&sender, &receiver, &mut world, 1, position);
    (sender, receiver, world)
}

/// The removals one tick published for `victim`, as `(recipient, ids)`.
fn published_removal(tick: &TickResult, victim: i32) -> Vec<(ConnectionId, Vec<i32>)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::RemoveEntities(remove),
            } if remove.entity_ids.contains(&victim) => {
                Some((*connection_id, remove.entity_ids.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Whether one tick announced `entity` to `connection`.
fn announced(tick: &TickResult, entity: i32, connection: ConnectionId) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if *connection_id == connection && add.id == entity
        )
    })
}

/// `BlockAttachedEntity.hurtServer` over
/// `ItemFrame.kill`: a painting a player breaks
/// leaves the store with `KILLED`, its tracker is told, and `ENTITY_DIE` names
/// the painting itself as its own `GameEvent.Context.sourceEntity`.
/// The one row is the whole `ENTITY_DIE` record: the
/// hit raises no `ENTITY_DIE` of its own.
#[test]
fn an_attacked_painting_raises_entity_die_with_the_painting_as_its_source() {
    let (sender, receiver, mut world) = world_beside(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 2.5,
    });
    // The wall the painting hangs on (`PaintingItem` places it at the air
    // block in front of the clicked face).
    assert!(world.set_block(
        OVERWORLD,
        at(0, 64, -1),
        plain_block_state("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let painting = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PAINTING_ID,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: 0.5,
            },
            EntityData::Painting(PaintingData::new(
                at(0, 64, 0),
                Direction::South,
                crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
            )),
        )
        .0;
    // `HangingEntity.recalculateBoundingBox`,
    // which the raw store spawn does not run.
    crate::entity::hanging::recalculate_bounding_box(
        world
            .stored_entity_mut_for_test(OVERWORLD, painting)
            .expect("the painting is filed in the level"),
    );
    let announced_tick = world.tick(receiver.drain());
    assert!(
        announced(&announced_tick, painting, ConnectionId(1)),
        "the session has to see the painting before its removal means anything"
    );

    let hurt = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: painting,
        }),
    );
    assert!(
        world.stored_entity_for_test(OVERWORLD, painting).is_none(),
        "`kill` removes the painting at once, unlike `LivingEntity.die`"
    );
    assert_eq!(
        published_removal(&hurt, painting),
        vec![(ConnectionId(1), vec![painting])]
    );
    assert_eq!(
        entity_die_rows(&world),
        vec![RecordedGameEvent {
            game_event: VanillaGameEvent::ENTITY_DIE,
            pos: at(0, 64, 0),
            source_entity_id: Some(painting),
            affected_state: None,
        }]
    );
}

/// `VehicleEntity.hurtServer` over
/// `VehicleEntity.destroy`: the destroying hit raises
/// `ENTITY_DAMAGE` and then `kill`'s `ENTITY_DIE`, and the
/// vehicle item lands after both.
#[test]
fn a_destroyed_boat_raises_entity_die_before_its_vehicle_item() {
    let (_sender, receiver, mut world) = world_beside(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 3.5,
    });
    let boat = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 2.5,
            },
            EntityData::Boat(BoatData::new()),
        )
        .0;
    let announced_tick = world.tick(receiver.drain());
    assert!(announced(&announced_tick, boat, ConnectionId(1)));

    // `VehicleEntity.hurtServer`'s `setDamage(getDamage() + damage * 10.0F)`
    // (`:47`) against `:50`'s `getDamage() > 40.0F`.
    let outcome = world
        .hurt_stored_entity_by_command_for_test(OVERWORLD, boat, "minecraft:generic", 10.0, None)
        .expect("the boat takes command damage");
    assert!(outcome.succeeded);
    assert!(world.stored_entity_for_test(OVERWORLD, boat).is_none());
    let (rows, _) = world.game_events(OVERWORLD);
    assert_eq!(
        rows.iter()
            .filter(|row| row.game_event == VanillaGameEvent::ENTITY_DIE)
            .count(),
        1
    );
    let dropped = world.stored_items_for_test(OVERWORLD);
    assert_eq!(
        dropped
            .iter()
            .map(|(_, _, resource_id, count, ..)| (resource_id.as_str(), *count))
            .collect::<Vec<_>>(),
        vec![("minecraft:oak_boat", 1)],
        "`destroy` drops the vehicle item after the kill"
    );
}

/// `ArmorStand.hurtServer`'s bypasses-invulnerability arm
/// over `ArmorStand.kill`: the
/// `minecraft:generic_kill` a `/damage` sends removes the stand at once and
/// raises `ENTITY_DIE` from the stand.
#[test]
fn a_generic_kill_armor_stand_raises_entity_die() {
    let (sender, receiver, mut world) = world_beside(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 2.5,
    });

    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:armor_stand 1.5 64 0.5",
    );
    assert_eq!(
        console_command_answer(&summoned),
        (Some(1), vec!["commands.summon.success".to_owned()])
    );
    let [stand] = world.stored_entity_ids_for_test(OVERWORLD)[..] else {
        panic!("one summoned stand");
    };
    let announced_tick = world.tick(receiver.drain());
    assert!(
        announced(&summoned, stand, ConnectionId(1))
            || announced(&announced_tick, stand, ConnectionId(1))
    );

    // `minecraft:generic_kill` is the `bypasses_invulnerability` tag
    // (`DamageTypeTags.BYPASSES_INVULNERABILITY`), which `/kill` on a stand
    // reaches without a hurt arm.
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            OVERWORLD,
            stand,
            "minecraft:generic_kill",
            1.0,
            None,
        )
        .expect("the stand takes the kill damage");
    assert!(!outcome.succeeded, "the arm answers false after the kill");
    assert!(world.stored_entity_for_test(OVERWORLD, stand).is_none());
    assert_eq!(
        entity_die_rows(&world),
        vec![RecordedGameEvent {
            game_event: VanillaGameEvent::ENTITY_DIE,
            pos: at(1, 64, 0),
            source_entity_id: Some(stand),
            affected_state: None,
        }]
    );
}

/// A cloud's instant harm reaches `LivingEntity.hurtServer` and `die`.
/// The
/// production frame must return the dying mob to its store before the
/// catalyst receives `ENTITY_DIE` through the real game-event dispatcher.
#[test]
fn a_lethal_cloud_reaches_the_entity_die_listener_in_the_world() {
    let (_sender, receiver, mut world) = world_beside(Vec3d {
        x: 4.5,
        y: 64.0,
        z: 0.5,
    });
    let catalyst = at(2, 64, 0);
    let catalyst_state =
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:sculk_catalyst",
            &[("bloom".to_string(), "false".to_string())]
                .into_iter()
                .collect(),
        )
        .expect("built-in catalyst state");
    assert!(world.set_block(OVERWORLD, catalyst, catalyst_state, SET_BLOCK_UPDATE_ALL));
    world.tick(receiver.drain());

    let cow = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            VANILLA_ENTITY_TYPE_COW_ID,
            MOB_POSITION,
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
        )
        .0;
    stage_one_blow_from_death(&mut world, cow);
    let potion_contents = bbb_protocol::packets::PotionContentsSummary {
        custom_effects: vec![bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID,
            amplifier: 1,
            duration: 1,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }],
        ..Default::default()
    };
    world.spawn_stored_entity_for_test(
        OVERWORLD,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        MOB_POSITION,
        EntityData::AreaEffectCloud(
            AreaEffectCloudData::new()
                .with_wait_time(0)
                .with_duration(100)
                .with_potion_contents(Some(potion_contents)),
        ),
    );

    for _ in 0..10 {
        world.tick(receiver.drain());
        if is_dead_or_dying(&world, cow) {
            break;
        }
    }
    assert!(is_dead_or_dying(&world, cow), "the cloud killed the cow");
    let events: Vec<_> = world
        .game_events(OVERWORLD)
        .0
        .into_iter()
        .filter(|event| event.source_entity_id == Some(cow))
        .map(|event| event.game_event)
        .collect();
    assert_eq!(
        events,
        vec![
            VanillaGameEvent::ENTITY_DAMAGE,
            VanillaGameEvent::ENTITY_DIE
        ]
    );
    // `SculkCatalystBlockEntity.CatalystListener.handleGameEvent` blooms on
    // every unconsumed death, whatever the reward.
    assert_eq!(
        world.block_state(OVERWORLD, catalyst),
        Some(
            bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                "minecraft:sculk_catalyst",
                &[("bloom".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
            )
            .expect("built-in catalyst state")
        ),
        "the catalyst received the cloud victim's death"
    );
}
