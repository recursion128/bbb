//! Wither boss bar and nether star through real stored-entity deaths.

use super::*;

fn boss_operations(events: &[WorldEvent]) -> Vec<bbb_protocol::packets::BossEventOperation> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::BossEvent(boss),
            } => Some(boss.operation.clone()),
            _ => None,
        })
        .collect()
}

/// `WitherBoss`: the session
/// tracking a wither sees its purple, screen-darkening bar follow its health,
/// the bar leaves with the removed body, and the death drops a nether star.
#[test]
fn a_wither_shows_its_boss_bar_to_trackers_and_drops_a_nether_star() {
    use bbb_protocol::packets::{BossBarColor, BossEventOperation};
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
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let wither = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_WITHER_ID);

    let tick = world.tick(receiver.drain());
    let added = boss_operations(&tick.events);
    let Some(BossEventOperation::Add {
        name,
        progress,
        color,
        flags,
        ..
    }) = added.first()
    else {
        panic!("the tracking session receives the wither bar: {added:?}");
    };
    assert_eq!(*progress, 1.0);
    assert_eq!(*color, BossBarColor::Purple);
    assert!(flags.darken_screen && !flags.play_music);
    assert!(
        matches!(name.component(), bbb_protocol::NbtSummaryValue::Compound(fields)
        if fields.iter().any(|field| field.name == "translate"
            && field.value == bbb_protocol::NbtSummaryValue::String("entity.minecraft.wither".into())))
    );

    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, wither)
        .expect("the wither is stored");
    let max = entity.living().unwrap().max_health();
    crate::entity::living::set_health(entity, max / 2.0);
    let tick = world.tick(receiver.drain());
    assert!(
        boss_operations(&tick.events)
            .iter()
            .any(|operation| matches!(operation,
                BossEventOperation::UpdateProgress { progress } if (*progress - 0.5).abs() < 0.01)),
        "{:?}",
        boss_operations(&tick.events)
    );

    stage_one_blow_from_death(&mut world, wither);
    kill_stored(&mut world, wither);
    let stars: Vec<_> = items(&world)
        .into_iter()
        .filter(|item| item.2 == "minecraft:nether_star")
        .collect();
    assert_eq!(stars.len(), 1, "{stars:?}");
    assert_eq!(stars[0].1, MOB_POSITION);
    assert_eq!(stars[0].4, 10, "spawnAtLocation's default pickup delay");

    let mut removed = false;
    for _ in 0..30 {
        let tick = world.tick(receiver.drain());
        removed |= boss_operations(&tick.events).contains(&BossEventOperation::Remove);
    }
    assert!(removed, "the bar leaves with the removed wither");
}

/// `WitherBoss` renames its bar to `getDisplayName()` on `setCustomName`
/// and a NoAI wither skips the
/// `customServerAiStep` that moves its progress.
#[test]
fn a_wither_bar_takes_the_team_display_name_and_freezes_without_ai() {
    use bbb_protocol::packets::BossEventOperation;
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
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let wither = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_WITHER_ID);
    world.tick(receiver.drain());
    let mut operations = Vec::new();
    for command in [
        "team add red",
        "team modify red color red",
        "team join red @e[type=wither]",
        "data merge entity @e[type=wither,limit=1] {CustomName:\"Boss\",NoAI:1b}",
    ] {
        let tick = chat_command(&sender, &receiver, &mut world, 1, command);
        operations.extend(boss_operations(&tick.events));
    }
    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, wither)
        .expect("the wither is stored");
    let max = entity.living().unwrap().max_health();
    crate::entity::living::set_health(entity, max / 2.0);
    let tick = world.tick(receiver.drain());
    operations.extend(boss_operations(&tick.events));
    assert!(
        operations.iter().any(|operation| matches!(operation,
            BossEventOperation::UpdateName { name }
                if format!("{:?}", name.component()).contains("\"Boss\"")
                    && format!("{:?}", name.component()).contains("\"red\""))),
        "{operations:?}"
    );
    assert!(
        !operations
            .iter()
            .any(|operation| matches!(operation, BossEventOperation::UpdateProgress { .. })),
        "{operations:?}"
    );
}
