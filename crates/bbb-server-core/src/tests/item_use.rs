//! `ServerPlayerGameMode.useItem` through the item-use table: projectile items
//! a session throws with a right click in the air, and what the throw leaves
//! behind in packets and in the inventory.

use bbb_facts::sound_sources::SoundSource;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_EGG_ID, VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
    VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
};
use bbb_protocol::packets::{AddEntity, SoundEventHolder};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::*;

mod books_and_bundles;
mod ender_eye;
mod entity_placing;
mod firework;
mod fishing;
mod fluid_containers;
mod spawn_egg;
mod thrown_potion;

/// Hotbar slot 0 in `InventoryMenu` numbering, which the fixture session
/// holds selected.
const HELD_MENU_SLOT: i16 = 36;

/// `Projectile.getMovementToShoot`'s blur: three `triangle(0, 0.0172275)` at
/// `uncertainty == 1`, each inside `[-0.0172275, 0.0172275]`, scaled by the
/// throw power.
const BLUR_PER_POWER: f64 = 0.0172275;

fn hold(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    resource_id: &str,
    count: i32,
) {
    let set = play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: HELD_MENU_SLOT,
            item: ItemStackSummary {
                item_id: Some(item_id(resource_id)),
                count,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    assert!(set.events.is_empty(), "holding {resource_id}: {set:?}");
}

fn throw(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    y_rot: f32,
    x_rot: f32,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence,
            y_rot,
            x_rot,
        }),
    )
}

fn adds_for(tick: &TickResult, connection_id: u64) -> Vec<AddEntity> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: id,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if *id == ConnectionId(connection_id) => Some(add.clone()),
            _ => None,
        })
        .collect()
}

fn held_count(tick: &TickResult, connection_id: u64) -> i32 {
    let player = snapshot_player(tick, connection_id);
    player.inventory.slots[usize::from(player.inventory.selected_slot)].count
}

fn menu_republished(tick: &TickResult, connection_id: u64) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: id,
                packet: ClientboundPlayPacket::ContainerSetContent(content),
            } if *id == ConnectionId(connection_id) && content.container_id == 0
        )
    })
}

fn assert_close(actual: Vec3d, expected: Vec3d, tolerance: f64, what: &str) {
    for (axis, (got, want)) in [
        ("x", (actual.x, expected.x)),
        ("y", (actual.y, expected.y)),
        ("z", (actual.z, expected.z)),
    ] {
        assert!(
            (got - want).abs() <= tolerance,
            "{what} {axis}: {got} is not within {tolerance} of {want} ({actual:?} vs {expected:?})"
        );
    }
}

#[test]
fn throwing_a_snowball_files_an_owned_projectile_from_the_eyes_with_the_throw_sound() {
    // `SnowballItem.use` (SnowballItem.java:24-43): the sound at the feet,
    // `spawnProjectileFromRotation(Snowball::new, .., 0.0F, 1.5F, 1.0F)`, the
    // stat, `consume(1, player)` -- and in creative the consume is a no-op
    // (`ItemStack.consume`, ItemStack.java:1082-1086), so `useItem` finds the
    // stack unchanged and never republishes the menu.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    hold(&sender, &receiver, &mut world, 1, "minecraft:snowball", 16);
    let standing = snapshot_player(&world.tick(Vec::new()), 1).position;

    let thrown = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 7);

    let adds = adds_for(&thrown, 1);
    assert_eq!(adds.len(), 1, "{thrown:?}");
    let add = &adds[0];
    assert_eq!(add.entity_type_id, VANILLA_ENTITY_TYPE_SNOWBALL_ID);
    // `Projectile.getAddEntityPacket` (Projectile.java:346-349) carries the
    // owner's entity id as `data`.
    assert_eq!(add.data, entity_id_of(&world, 1));
    // `ThrowableItemProjectile(type, owner, level, stack)`
    // (ThrowableItemProjectile.java:21-25): `(getX(), getEyeY() - 0.1F, getZ())`.
    let eye = bbb_server_types::interaction::player_eye_position(standing).unwrap();
    assert_eq!(
        add.position,
        Vec3d {
            x: standing.x,
            y: eye.y - f64::from(0.1_f32),
            z: standing.z,
        }
    );
    // Facing +z with no pitch: `shootFromRotation` aims `(0, 0, 1)`, blurred
    // and scaled by `1.5F`; the thrower stands still, so nothing is added.
    assert_close(
        add.delta_movement,
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        BLUR_PER_POWER * 1.5 + 1e-9,
        "snowball velocity",
    );

    let sounds = sounds_sent(&thrown);
    let registry_id = vanilla_sound_event_registry_id("minecraft:entity.snowball.throw").unwrap();
    let throw_sound = sounds
        .iter()
        .find(|sound| sound.sound == SoundEventHolder::Reference { registry_id })
        .unwrap_or_else(|| panic!("the throw sound is sent: {sounds:?}"));
    assert_eq!(throw_sound.source, SoundSource::Neutral);
    assert_eq!(throw_sound.position, standing);
    assert_eq!(throw_sound.volume, 0.5);
    // `0.4F / (nextFloat() * 0.4F + 0.8F)`.
    assert!(
        (0.4 / 1.2..=0.4 / 0.8).contains(&throw_sound.pitch),
        "{}",
        throw_sound.pitch
    );

    assert_eq!(held_count(&thrown, 1), 16);
    assert!(!menu_republished(&thrown, 1), "{thrown:?}");

    // `player.awardStat(Stats.ITEM_USED.get(this))` (`:40`).
    let stats = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    assert!(stats.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AwardStats(update),
            ..
        } if update.stats.iter().any(|entry|
            entry.stat_type_id == bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID
                && entry.value_id == item_id("minecraft:snowball")
                && entry.amount == 1)
    )));
}

#[test]
fn throwing_a_wind_charge_uses_the_eye_position_and_starts_its_half_second_cooldown() {
    // `WindChargeItem.use` (`WindChargeItem.java:27-53`) constructs at eye Y,
    // shoots with `(0.0F, 1.5F, 1.0F)`, then plays the throw sound, awards the
    // stat and consumes. Its prototype carries a 0.5-second use cooldown.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    hold(
        &sender,
        &receiver,
        &mut world,
        1,
        "minecraft:wind_charge",
        16,
    );
    let standing = snapshot_player(&world.tick(Vec::new()), 1).position;

    let thrown = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    let adds = adds_for(&thrown, 1);
    assert_eq!(adds.len(), 1, "{thrown:?}");
    let add = &adds[0];
    assert_eq!(add.entity_type_id, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID);
    assert_eq!(add.data, entity_id_of(&world, 1));
    let eye = bbb_server_types::interaction::player_eye_position(standing).unwrap();
    assert_eq!(
        add.position,
        Vec3d {
            x: standing.x,
            y: eye.y,
            z: standing.z,
        }
    );
    assert_close(
        add.delta_movement,
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        BLUR_PER_POWER * 1.5 + 1e-9,
        "wind-charge velocity",
    );

    let registry_id =
        vanilla_sound_event_registry_id("minecraft:entity.wind_charge.throw").unwrap();
    let sounds = sounds_sent(&thrown);
    let sound = sounds
        .iter()
        .find(|sound| sound.sound == SoundEventHolder::Reference { registry_id })
        .unwrap_or_else(|| panic!("the wind-charge throw sound is sent: {sounds:?}"));
    assert_eq!(sound.source, SoundSource::Neutral);
    assert_eq!(sound.position, standing);
    assert_eq!(sound.volume, 0.5);
    assert!((0.4 / 1.2..=0.4 / 0.8).contains(&sound.pitch));

    let cooldowns: Vec<_> = thrown
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Cooldown(cooldown),
            } => Some(cooldown),
            _ => None,
        })
        .collect();
    assert_eq!(cooldowns.len(), 1, "{thrown:?}");
    assert_eq!(cooldowns[0].cooldown_group, "minecraft:wind_charge");
    assert_eq!(cooldowns[0].duration, 10);
    assert_eq!(held_count(&thrown, 1), 16);
    assert!(!menu_republished(&thrown, 1), "{thrown:?}");

    let blocked = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);
    assert!(adds_for(&blocked, 1).is_empty(), "{blocked:?}");
    assert!(sounds_sent(&blocked).is_empty(), "{blocked:?}");
}

#[test]
fn a_survival_throw_consumes_one_and_republishes_the_menu() {
    // `useItem` (ServerPlayerGameMode.java:324-346): the count moved, so the
    // stack is written back and `inventoryMenu.sendAllDataToRemote()` runs;
    // the last one leaves `ItemStack.EMPTY` in the hand.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // A snowball rather than a pearl: it starts no cooldown, so the two throws
    // are back to back and only the count separates them.
    hold(&sender, &receiver, &mut world, 1, "minecraft:snowball", 2);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival");

    let first = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    assert_eq!(adds_for(&first, 1).len(), 1, "{first:?}");
    assert_eq!(
        adds_for(&first, 1)[0].entity_type_id,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID
    );
    assert_eq!(held_count(&first, 1), 1);
    assert!(menu_republished(&first, 1), "{first:?}");

    let second = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);
    assert_eq!(adds_for(&second, 1).len(), 1, "{second:?}");
    assert_eq!(held_count(&second, 1), 0);
    assert!(menu_republished(&second, 1), "{second:?}");

    // An empty hand is refused before `useItem` (handleUseItem, :1407).
    let third = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 3);
    assert!(adds_for(&third, 1).is_empty(), "{third:?}");
}

#[test]
fn a_spectator_throws_nothing() {
    // `useItem` (ServerPlayerGameMode.java:305-307) answers `PASS` for a
    // spectator before the item runs; the rotation snap still happened.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    hold(&sender, &receiver, &mut world, 1, "minecraft:egg", 3);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator");

    let thrown = throw(&sender, &receiver, &mut world, 1, 45.0, 10.0, 1);
    assert!(adds_for(&thrown, 1).is_empty(), "{thrown:?}");
    assert!(sounds_sent(&thrown).is_empty(), "{thrown:?}");
    assert_eq!(held_count(&thrown, 1), 3);
    assert_eq!(
        snapshot_player(&thrown, 1).rotation,
        PlayerRotation {
            y_rot: 45.0,
            x_rot: 10.0
        }
    );
}

#[test]
fn every_table_item_throws_its_own_projectile_with_its_own_sound_and_aim() {
    // One row per `Item.use`, and the egg row answers all three egg items
    // through the catalog's `use_override`. `ExperienceBottleItem` aims
    // `-20.0F` above the look and throws at `0.7F`
    // (ExperienceBottleItem.java:35).
    struct Case {
        item: &'static str,
        entity_type_id: i32,
        sound: &'static str,
        source: SoundSource,
        y_offset: f32,
        pow: f64,
    }
    let cases = [
        Case {
            item: "minecraft:snowball",
            entity_type_id: VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            sound: "minecraft:entity.snowball.throw",
            source: SoundSource::Neutral,
            y_offset: 0.0,
            pow: 1.5,
        },
        Case {
            item: "minecraft:egg",
            entity_type_id: VANILLA_ENTITY_TYPE_EGG_ID,
            sound: "minecraft:entity.egg.throw",
            source: SoundSource::Players,
            y_offset: 0.0,
            pow: 1.5,
        },
        Case {
            item: "minecraft:blue_egg",
            entity_type_id: VANILLA_ENTITY_TYPE_EGG_ID,
            sound: "minecraft:entity.egg.throw",
            source: SoundSource::Players,
            y_offset: 0.0,
            pow: 1.5,
        },
        Case {
            item: "minecraft:brown_egg",
            entity_type_id: VANILLA_ENTITY_TYPE_EGG_ID,
            sound: "minecraft:entity.egg.throw",
            source: SoundSource::Players,
            y_offset: 0.0,
            pow: 1.5,
        },
        Case {
            item: "minecraft:ender_pearl",
            entity_type_id: VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
            sound: "minecraft:entity.ender_pearl.throw",
            source: SoundSource::Neutral,
            y_offset: 0.0,
            pow: 1.5,
        },
        Case {
            item: "minecraft:experience_bottle",
            entity_type_id: VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
            sound: "minecraft:entity.experience_bottle.throw",
            source: SoundSource::Neutral,
            y_offset: -20.0,
            pow: 0.7,
        },
    ];
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for (index, case) in cases.iter().enumerate() {
        hold(&sender, &receiver, &mut world, 1, case.item, 1);
        // Looking 30 degrees up while facing -x (yRot 90).
        let (y_rot, x_rot) = (90.0_f32, -30.0_f32);
        let thrown = throw(
            &sender,
            &receiver,
            &mut world,
            1,
            y_rot,
            x_rot,
            i32::try_from(index).unwrap() + 1,
        );
        let adds = adds_for(&thrown, 1);
        assert_eq!(adds.len(), 1, "{}: {thrown:?}", case.item);
        assert_eq!(adds[0].entity_type_id, case.entity_type_id, "{}", case.item);
        // `shootFromRotation` (Projectile.java:152-159): the unit direction
        // from the angles, `yOffset` folded into the pitch, then `pow`.
        let to_radians = std::f32::consts::PI / 180.0;
        let xd = -(y_rot * to_radians).sin() * (x_rot * to_radians).cos();
        let yd = -((x_rot + case.y_offset) * to_radians).sin();
        let zd = (y_rot * to_radians).cos() * (x_rot * to_radians).cos();
        let length = f64::from(xd * xd + yd * yd + zd * zd).sqrt();
        let expected = Vec3d {
            x: f64::from(xd) / length * case.pow,
            y: f64::from(yd) / length * case.pow,
            z: f64::from(zd) / length * case.pow,
        };
        assert_close(
            adds[0].delta_movement,
            expected,
            BLUR_PER_POWER * case.pow + 1e-6,
            case.item,
        );
        let registry_id = vanilla_sound_event_registry_id(case.sound).unwrap();
        let sounds = sounds_sent(&thrown);
        let sound = sounds
            .iter()
            .find(|sound| sound.sound == SoundEventHolder::Reference { registry_id })
            .unwrap_or_else(|| panic!("{}: {sounds:?}", case.item));
        assert_eq!(sound.source, case.source, "{}", case.item);
    }
}

#[test]
fn the_throw_carries_the_throwers_last_reported_movement() {
    // `shootFromRotation` (Projectile.java:157-158) adds
    // `source.getKnownMovement()`, which for a `ServerPlayer` is the last
    // accepted movement packet's displacement (ServerPlayer.java:2179-2182),
    // with the vertical part dropped while the source is on the ground; a
    // client tick that reports no movement zeroes it again
    // (`handleClientTickEnd`, ServerGamePacketListenerImpl.java:2170-2177).
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    hold(&sender, &receiver, &mut world, 1, "minecraft:snowball", 16);
    let standing = snapshot_player(&world.tick(Vec::new()), 1).position;

    // Airborne: the whole displacement rides along.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: standing.x + 0.3,
            y: standing.y + 0.2,
            z: standing.z,
        },
    );
    let moved_airborne = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: standing.x + 0.6,
                y: standing.y + 0.4,
                z: standing.z,
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    assert!(moved_airborne.events.is_empty(), "{moved_airborne:?}");
    let thrown = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    let adds = adds_for(&thrown, 1);
    assert_eq!(adds.len(), 1, "{thrown:?}");
    assert_close(
        adds[0].delta_movement,
        Vec3d {
            x: 0.3,
            y: 0.2,
            z: 1.5,
        },
        BLUR_PER_POWER * 1.5 + 1e-9,
        "airborne throw",
    );

    // Back on the floor: the vertical displacement is dropped. The ground
    // has to be real, as the tick's own travel rewrites `onGround`
    // (`LivingEntity.java:3061-3065`).
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: standing.x + 0.9,
            y: standing.y,
            z: standing.z,
        },
    );
    let thrown = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);
    let adds = adds_for(&thrown, 1);
    assert_eq!(adds.len(), 1, "{thrown:?}");
    assert_close(
        adds[0].delta_movement,
        Vec3d {
            x: 0.3,
            y: 0.0,
            z: 1.5,
        },
        BLUR_PER_POWER * 1.5 + 1e-9,
        "grounded throw",
    );

    // A client tick without a movement packet zeroes the known movement.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientTickEnd,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientTickEnd,
    );
    let thrown = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 3);
    let adds = adds_for(&thrown, 1);
    assert_eq!(adds.len(), 1, "{thrown:?}");
    assert_close(
        adds[0].delta_movement,
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        BLUR_PER_POWER * 1.5 + 1e-9,
        "still throw",
    );
}

#[test]
fn an_item_without_a_table_row_still_only_acknowledges_and_turns() {
    // A `FireworkRocketItem` overrides `use` but has no row yet, so it falls
    // through to the generic `Item.use` read, which for it is `PASS`: the
    // acknowledgement and the rotation and nothing else.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    hold(
        &sender,
        &receiver,
        &mut world,
        1,
        "minecraft:firework_rocket",
        3,
    );

    let used = throw(&sender, &receiver, &mut world, 1, 10.0, 20.0, 4);
    assert!(
        matches!(
            used.events.as_slice(),
            [WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::BlockChangedAck(ack),
            }] if ack.sequence == 4
        ),
        "{used:?}"
    );
    assert_eq!(held_count(&used, 1), 3);
}

#[test]
fn an_ender_pearl_throw_starts_a_one_second_cooldown_that_blocks_the_next_throw() {
    // `ItemStack.use` applies the pearl's `use_cooldown` (1.0s = 20 ticks)
    // through `applyAfterUseComponentSideEffects` (ItemStack.java:397-411);
    // `useItem` (ServerPlayerGameMode.java:309-311) then refuses a second
    // throw while the group runs, and `ItemCooldowns.tick` ends it with a
    // zero-duration packet on the twentieth tick.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    hold(
        &sender,
        &receiver,
        &mut world,
        1,
        "minecraft:ender_pearl",
        16,
    );

    let first = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    assert_eq!(adds_for(&first, 1).len(), 1, "{first:?}");
    // `ServerItemCooldowns.onCooldownStarted` (ServerItemCooldowns.java:15-18).
    let started: Vec<_> = first
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Cooldown(cooldown),
            } => Some(cooldown.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(started.len(), 1, "{first:?}");
    assert_eq!(started[0].cooldown_group, "minecraft:ender_pearl");
    assert_eq!(started[0].duration, 20);

    // A second throw inside the cooldown is refused: no projectile, no sound.
    let blocked = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);
    assert!(adds_for(&blocked, 1).is_empty(), "{blocked:?}");
    assert!(sounds_sent(&blocked).is_empty(), "{blocked:?}");

    // The first throw already spent this tick's counter, so nineteen more idle
    // ticks reach the twentieth, which ends the cooldown.
    let mut ended = None;
    for _ in 0..19 {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Cooldown(cooldown),
            } = event
            {
                ended = Some(cooldown.clone());
            }
        }
    }
    let ended = ended.expect("the cooldown ends");
    assert_eq!(ended.cooldown_group, "minecraft:ender_pearl");
    assert_eq!(ended.duration, 0);

    // The cooldown is over, so the next throw lands.
    let after = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 3);
    assert_eq!(adds_for(&after, 1).len(), 1, "{after:?}");
}
