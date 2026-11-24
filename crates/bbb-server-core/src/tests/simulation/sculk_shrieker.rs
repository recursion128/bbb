//! `SculkShriekerBlockEntity`'s runtime through a real level tick: the
//! vibration a delivered game event schedules, the travel clock, the
//! `tryShriek` at the arrival, and `tryRespond` on the 90-tick reset.

use super::*;

use crate::block_entity::vibration::{GameEventTag, VibrationData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
use bbb_protocol::packets::SoundEventHolder;

use super::helpers::{at, shrieker_state};
use super::sounds_sent as sounds;

fn level_events_sent(published: &TickResult) -> Vec<bbb_protocol::packets::LevelEvent> {
    published
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelEvent(update),
                ..
            } => Some(*update),
            _ => None,
        })
        .collect()
}

fn vibration_data(world: &mut ServerWorld, pos: BlockPos) -> VibrationData {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|entity| entity.sculk_shrieker())
        .expect("the shrieker's block entity")
        .vibration_data()
        .clone()
}

fn warning_level(world: &mut ServerWorld, pos: BlockPos) -> i32 {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|entity| entity.sculk_shrieker())
        .expect("the shrieker's block entity")
        .warning_level()
}

/// The world test A/B/C share: a survival session beside a shrieker whose
/// listenable tag is the test override, because the one production row
/// (`minecraft:sculk_sensor_tendrils_clicking`) is a `VanillaGameEvent` this
/// build cannot name yet.
fn warned_world(shrieker: BlockPos) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
    );
    // `MobEffectUtil.addEffectToPlayersAround` filters on survival
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        shrieker,
        shrieker_state(false, true),
        SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, shrieker)
        .and_then(|entity| entity.sculk_shrieker_mut())
        .expect("the write created the shrieker's block entity")
        .set_listenable_events_for_test(GameEventTag::Vibrations);
    // The join and the placement leave their own events in the sink; one tick
    // flushes them before a scenario records its own.
    world.tick(receiver.drain());
    (sender, receiver, world)
}

/// One `sculk_sensor_tendrils_clicking`-class event delivered through the
/// landed listener runtime travels as a vibration and shrieks at the arrival.
#[test]
fn a_delivered_vibration_travels_to_the_shrieker_and_shrieks() {
    let shrieker = at(0, 64, 0);
    let source = at(4, 64, 0);
    let (_sender, receiver, mut world) = warned_world(shrieker);
    let player = world.player_entity_id_for_test(ConnectionId(1));

    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        source,
        Some(player),
        None,
    );
    world.tick(receiver.drain());

    let (candidate, _) = vibration_data(&mut world, shrieker)
        .selection_strategy()
        .current()
        .expect("the event scheduled a candidate");
    assert_eq!(candidate.game_event, "minecraft:block_change");
    assert_eq!(candidate.distance, 4.0);

    // `calculateTravelTimeInTicks` is
    // `floor(distance)`: four more ticks, then the arrival.
    let arrival = 'arrival: {
        for _ in 0..6 {
            let tick = world.tick(receiver.drain());
            if world.block_state(Java26BuiltinDimension::Overworld, shrieker)
                == Some(shrieker_state(true, true))
            {
                break 'arrival tick;
            }
        }
        panic!("the vibration did not arrive within six ticks");
    };
    assert_eq!(
        level_events_sent(&arrival)
            .iter()
            .filter(|event| event.event_type == 3007)
            .map(|event| (event.pos, event.data, event.global))
            .collect::<Vec<_>>(),
        vec![(shrieker, 0, false)],
        "`LevelEvent.PARTICLES_SCULK_SHRIEK`"
    );
    assert_eq!(warning_level(&mut world, shrieker), 1, "tryWarn escalated");
    // `SculkShriekerBlockEntity.shriek` (`:124`) raises `GameEvent.SHRIEK` at
    // the shrieker with the player's entity as the context source.
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        game_events.iter().any(|event| {
            event.game_event.resource_id() == "minecraft:shriek"
                && event.pos == shrieker
                && event.source_entity_id == Some(player)
        }),
        "shriek raised GameEvent.SHRIEK: {game_events:?}"
    );
    assert!(
        vibration_data(&mut world, shrieker)
            .current_vibration()
            .is_none(),
        "the arrival cleared the current vibration"
    );

    // `SculkShriekerBlock.tick` runs ninety
    // ticks later: the flag clears and `tryRespond` answers with the level-one
    // reply sound and the darkness.
    for _ in 0..89 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, shrieker),
        Some(shrieker_state(true, true)),
        "the scheduled tick is ninety ticks out, not sooner"
    );
    let response = world.tick(receiver.drain());
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, shrieker),
        Some(shrieker_state(false, true)),
        "the reset cleared SHRIEKING"
    );
    let reply = sounds(&response)
        .into_iter()
        .find(|sound| {
            matches!(
                &sound.sound,
                SoundEventHolder::Reference { registry_id }
                    if *registry_id
                        == vanilla_sound_event_registry_id("minecraft:entity.warden.nearby_close")
                            .expect("the reply sound is in the catalog")
            )
        })
        .expect("the level-one warning names its reply sound");
    assert_eq!(reply.source, SoundSource::Hostile);
    assert_eq!(reply.volume, 5.0);
    assert_eq!(reply.pitch, 1.0);
    // `playWardenReplySound` offsets by `Mth.randomBetweenInclusive(-10, 10)`.
    assert!(
        (reply.position.x - 0.0).abs() <= 10.5
            && (reply.position.y - 64.0).abs() <= 10.5
            && (reply.position.z - 0.0).abs() <= 10.5,
        "{:?}",
        reply.position
    );
    assert!(
        world
            .player_active_effect_for_test(ConnectionId(1), "minecraft:darkness")
            .is_some(),
        "`Warden.applyDarknessAround` reached the session"
    );
}

/// `SculkShriekerBlockEntity.tryRespond` (`:140-148`) on the scheduled tick:
/// a warning at four summons a warden, and both arms apply darkness.
#[test]
fn the_scheduled_response_summons_at_four() {
    let shrieker = at(0, 64, 1);
    let (_sender, receiver, mut world) = warned_world(shrieker);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Adventure,
        PlayerAbilities::for_game_type(GameType::Adventure),
    );
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 30.5,
            y: 64.0,
            z: 1.5,
        },
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        shrieker,
        shrieker_state(true, true),
        SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, shrieker)
        .and_then(|entity| entity.sculk_shrieker_mut())
        .expect("the shrieker's block entity")
        .set_warning_level_for_test(4);

    let mut events = Vec::new();
    world.tick_sculk_shrieker_scheduled(
        &Java26BuiltinDimension::Overworld.into(),
        shrieker,
        shrieker_state(true, true),
        &mut events,
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, shrieker),
        Some(shrieker_state(false, true)),
        "the reset ran"
    );
    assert!(
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .any(|id| world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID)),
        "a fully warned shrieker summons a warden"
    );
    assert!(
        world
            .player_active_effect_for_test(ConnectionId(1), "minecraft:darkness")
            .is_some(),
        "the shrieker applies darkness to an adventure player thirty blocks away"
    );
    let _ = receiver.drain();
}

/// `WardenSpawnTracker.getNearbyPlayers`
/// over the world's own player list: the warning copies to every player within
/// sixteen blocks of the shrieker and to nobody outside it.
#[test]
fn the_warning_copies_only_to_the_players_inside_sixteen_blocks() {
    let shrieker = at(0, 64, 0);
    let source = at(3, 64, 0);
    let (sender, receiver, mut world) = warned_world(shrieker);
    let alice = Uuid::from_u128(1);
    let bob = Uuid::from_u128(2);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let far = Vec3d {
        x: 20.5,
        y: 64.0,
        z: 0.5,
    };
    stand_at(&sender, &receiver, &mut world, 2, far);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let deliver = |world: &mut ServerWorld, receiver: &WorldCommandReceiver| {
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            source,
            Some(player),
            None,
        );
        for _ in 0..6 {
            world.tick(receiver.drain());
            if world.block_state(Java26BuiltinDimension::Overworld, shrieker)
                == Some(shrieker_state(true, true))
            {
                break;
            }
        }
    };

    deliver(&mut world, &receiver);
    assert_eq!(
        world
            .warden_spawn_tracker_for_test(alice)
            .map(|tracker| tracker.warning_level()),
        Some(1),
        "the trigger player was warned"
    );
    assert_eq!(
        world
            .warden_spawn_tracker_for_test(bob)
            .map(|tracker| tracker.warning_level()),
        Some(0),
        "twenty blocks out is outside the copy"
    );

    // The 200-tick cooldown (`:108`) has to expire before the next warning,
    // and the 90-tick scheduled tick clears SHRIEKING on the way.
    let near = Vec3d {
        x: 5.5,
        y: 64.0,
        z: 0.5,
    };
    stand_at(&sender, &receiver, &mut world, 2, near);
    for _ in 0..201 {
        world.tick(receiver.drain());
    }
    deliver(&mut world, &receiver);
    assert_eq!(
        world
            .warden_spawn_tracker_for_test(alice)
            .map(|tracker| tracker.warning_level()),
        Some(2),
        "the second warning escalated"
    );
    assert_eq!(
        world
            .warden_spawn_tracker_for_test(bob)
            .map(|tracker| tracker.warning_level()),
        Some(2),
        "a player who walked inside sixteen blocks takes the copy"
    );
}

/// `VibrationSystem.Listener.handleGameEvent`'s first bail
/// and the selector's same-tick rule over the
/// landed runtime: two events in one tick keep the nearer candidate.
#[test]
fn duplicate_events_in_one_tick_keep_the_nearest() {
    let shrieker = at(0, 64, 0);
    let (_sender, receiver, mut world) = warned_world(shrieker);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    for source in [at(6, 64, 0), at(3, 64, 0)] {
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            source,
            Some(player),
            None,
        );
    }
    world.tick(receiver.drain());
    let (candidate, _) = vibration_data(&mut world, shrieker)
        .selection_strategy()
        .current()
        .expect("both events scheduled");
    assert_eq!(candidate.distance, 3.0, "the nearer candidate wins");
}

/// `EuclideanGameEventListenerRegistry.getPostableListenerPosition`
/// (`:115-124`): a listener only hears events inside its own radius, so an
/// event nine blocks away never reaches the shrieker.
#[test]
fn an_event_outside_the_listener_radius_never_reaches_the_shrieker() {
    let shrieker = at(0, 64, 0);
    let (_sender, receiver, mut world) = warned_world(shrieker);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        at(9, 64, 0),
        Some(player),
        None,
    );
    world.tick(receiver.drain());
    assert!(
        vibration_data(&mut world, shrieker)
            .selection_strategy()
            .current()
            .is_none(),
        "the walk never visited the listener"
    );
}

/// `WardenSpawnTracker.tryWarn` through the
/// world's own player list: the first arrival warns, and the 200-tick cooldown
/// suppresses the second until it expires.
#[test]
fn the_world_tracker_escalates_and_respects_its_cooldown() {
    let shrieker = at(0, 64, 0);
    let source = at(3, 64, 0);
    let (_sender, receiver, mut world) = warned_world(shrieker);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let deliver = |world: &mut ServerWorld, receiver: &WorldCommandReceiver| {
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            source,
            Some(player),
            None,
        );
        for _ in 0..6 {
            world.tick(receiver.drain());
            if world.block_state(Java26BuiltinDimension::Overworld, shrieker)
                == Some(shrieker_state(true, true))
            {
                break;
            }
        }
    };
    deliver(&mut world, &receiver);
    assert_eq!(warning_level(&mut world, shrieker), 1);
    // Clear SHRIEKING so the next vibration is admissible, and warn again
    // inside the cooldown: `tryWarn` answers empty, so the shriek runs at
    // warning zero.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        shrieker,
        shrieker_state(false, true),
        SET_BLOCK_UPDATE_ALL,
    ));
    deliver(&mut world, &receiver);
    assert_eq!(
        warning_level(&mut world, shrieker),
        0,
        "the cooldown left the warning level at zero"
    );
}

/// A creative session beside a shrieker, so `SetCreativeModeSlot` with a
/// negative slot is `player.drop(itemStack, true)`
/// -- the one production path
/// that writes an item entity's `thrower`.
fn dropper_world(shrieker: BlockPos) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Creative,
        PlayerAbilities::for_game_type(GameType::Creative),
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        shrieker,
        shrieker_state(false, true),
        SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, shrieker)
        .and_then(|entity| entity.sculk_shrieker_mut())
        .expect("the write created the shrieker's block entity")
        .set_listenable_events_for_test(GameEventTag::Vibrations);
    world.tick(receiver.drain());
    (sender, receiver, world)
}

/// One stack out of the session's hand, as the entity id of the item entity it
/// became.
fn drop_stack(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    resource_id: &str,
) -> i32 {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(bbb_protocol::packets::SetCreativeModeSlot {
            // `:2016`: any negative slot number drops instead of writing.
            slot_num: -1,
            item: ItemStackSummary {
                item_id: Some(item_id(resource_id)),
                count: 1,
                component_patch: Default::default(),
            },
        }),
    );
    world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|(_, _, stack, ..)| stack == resource_id)
        .unwrap_or_else(|| panic!("the drop created a {resource_id} item entity"))
        .0
}

/// `isValidVibration`'s `sourceEntity.dampensVibrations()`
/// over `ItemEntity.dampensVibrations`
/// two stacks the same session dropped from its own
/// hand differ only in `ItemTags.DAMPENS_VIBRATIONS`, and only the wool is
/// refused.
///
/// The stone half is also the proof of `ItemEntity.getOwner()`: the
/// shrieker's `canReceiveVibration`
/// answers false unless `tryGetPlayer` resolves the source, and for an item
/// entity that is the `thrower` alone.
#[test]
fn a_dropped_wool_stack_dampens_the_vibration_the_same_hand_otherwise_delivers() {
    let shrieker = at(0, 64, 0);
    let source = at(4, 64, 0);
    let (sender, receiver, mut world) = dropper_world(shrieker);
    let wool = drop_stack(&sender, &receiver, &mut world, "minecraft:white_wool");
    let stone = drop_stack(&sender, &receiver, &mut world, "minecraft:stone");

    for item in [wool, stone] {
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            source,
            Some(item),
            None,
        );
    }
    world.tick(receiver.drain());

    // An equal-distance, equal-frequency event never replaces a held candidate
    // (`VibrationSelector.shouldReplaceVibration`), so the stone source proves
    // the earlier wool event was refused.
    let (candidate, _) = vibration_data(&mut world, shrieker)
        .selection_strategy()
        .current()
        .expect("the stone drop scheduled a candidate");
    assert_eq!(
        candidate.source,
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, stone)
            .map(|entity| entity.uuid()),
        "the wool stack dampens its own vibration and the stone stack does not"
    );

    // The arrival shrieks, which only a resolved `thrower` can reach: the
    // warning escalation is `tryShriek(level, player)` with a real session.
    for _ in 0..6 {
        world.tick(receiver.drain());
        if world.block_state(Java26BuiltinDimension::Overworld, shrieker)
            == Some(shrieker_state(true, true))
        {
            break;
        }
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, shrieker),
        Some(shrieker_state(true, true)),
        "the item entity's thrower is the player the shrieker shrieked for"
    );
    assert_eq!(warning_level(&mut world, shrieker), 1);
}

/// The other half of the same read: an item entity nobody threw resolves no
/// owner, so `tryGetPlayer` answers null
/// and `canReceiveVibration` refuses the event outright.
#[test]
fn an_item_entity_with_no_thrower_never_reaches_the_shrieker() {
    let shrieker = at(0, 64, 0);
    let source = at(4, 64, 0);
    let (_sender, receiver, mut world) = dropper_world(shrieker);
    let stack =
        crate::entity::item::plain_stack("minecraft:stone", 1).expect("a plain stack decodes");
    let (unthrown, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(stack, 0.0)),
    );
    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        source,
        Some(unthrown),
        None,
    );
    world.tick(receiver.drain());

    assert!(vibration_data(&mut world, shrieker)
        .selection_strategy()
        .current()
        .is_none());
}

fn direct_step(world: &mut ServerWorld, pos: BlockPos) {
    world.step_on_sculk_shrieker_for_test(
        &Java26BuiltinDimension::Overworld.into(),
        pos,
        Uuid::from_u128(1),
        &mut Vec::new(),
    );
}

fn recorded_shrieks(world: &ServerWorld) -> Vec<crate::level_event::RecordedGameEvent> {
    world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .into_iter()
        .filter(|event| event.game_event == VanillaGameEvent::SHRIEK)
        .collect()
}

#[test]
fn direct_step_shrieks_immediately_with_player_source_and_resets_at_ninety_ticks() {
    let pos = at(0, 64, 0);
    let (_, receiver, mut world) = warned_world(pos);
    let entity_id = world.player_entity_id_for_test(ConnectionId(1));
    direct_step(&mut world, pos);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, pos),
        Some(shrieker_state(true, true))
    );
    assert_eq!(warning_level(&mut world, pos), 1);
    assert_eq!(recorded_shrieks(&world).len(), 1);
    assert_eq!(
        recorded_shrieks(&world)[0].source_entity_id,
        Some(entity_id)
    );
    assert_eq!(recorded_shrieks(&world)[0].pos, pos);
    assert_eq!(vibration_data(&mut world, pos), VibrationData::default());
    let first = world.tick(receiver.drain());
    assert_eq!(
        level_events_sent(&first)
            .iter()
            .filter(|event| event.event_type == 3007 && event.pos == pos)
            .count(),
        1
    );
    for _ in 1..89 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, pos),
        Some(shrieker_state(true, true))
    );
    world.tick(receiver.drain());
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, pos),
        Some(shrieker_state(false, true))
    );
}

#[test]
fn direct_step_repeat_is_noop_and_warning_cooldown_refuses_a_new_shriek() {
    let pos = at(0, 64, 0);
    let (_, _, mut world) = warned_world(pos);
    direct_step(&mut world, pos);
    let tracker = world.warden_spawn_tracker_for_test(Uuid::from_u128(1));
    direct_step(&mut world, pos);
    assert_eq!(warning_level(&mut world, pos), 1);
    assert_eq!(
        world.warden_spawn_tracker_for_test(Uuid::from_u128(1)),
        tracker
    );
    assert_eq!(recorded_shrieks(&world).len(), 1);
    world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        shrieker_state(false, true),
        SET_BLOCK_UPDATE_ALL,
    );
    direct_step(&mut world, pos);
    assert_eq!(warning_level(&mut world, pos), 0);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, pos),
        Some(shrieker_state(false, true))
    );
    assert_eq!(recorded_shrieks(&world).len(), 1);
}

#[test]
fn direct_step_non_summoning_shrieker_still_shrieks_without_warning() {
    for (can_summon, command) in [
        (false, None),
        (true, Some("difficulty peaceful")),
        (true, Some("gamerule spawn_wardens false")),
    ] {
        let pos = at(0, 64, 0);
        let (sender, receiver, mut world) = warned_world(pos);
        if let Some(command) = command {
            enqueue(
                &sender,
                WorldCommand::Admin {
                    envelope: admin_envelope(1, "shrieker-settings", AdminScope::Command),
                    request: AdminRequest::ExecuteCommand {
                        command: command.to_owned(),
                    },
                },
            )
            .unwrap();
            let applied = world.tick(receiver.drain());
            assert!(admin_outcome(&applied).result.is_ok());
        }
        if !can_summon {
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                pos,
                shrieker_state(false, false),
                SET_BLOCK_UPDATE_ALL,
            ));
        }
        let tracker = world.warden_spawn_tracker_for_test(Uuid::from_u128(1));
        direct_step(&mut world, pos);
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(shrieker_state(true, can_summon))
        );
        assert_eq!(warning_level(&mut world, pos), 0);
        assert_eq!(
            world.warden_spawn_tracker_for_test(Uuid::from_u128(1)),
            tracker
        );
        assert_eq!(recorded_shrieks(&world).len(), 1);
    }
}
