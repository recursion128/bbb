use super::*;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_server_types::stats::StatKey;

/// `Player.killedEntity` awards the killer's
/// `Stats.ENTITY_KILLED` for a mob victim, while an unattributed death does not.
#[test]
fn a_player_killing_a_mob_awards_entity_killed_for_the_mob_type() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());

    let (zombie, zombie_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 2.0,
            y: 64.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
                .expect("a zombie is a living entity"),
        ),
    );
    crate::entity::living::set_health(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
            .expect("the zombie is stored"),
        0.1,
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {zombie_uuid} 40 minecraft:generic by alice"),
    );

    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ),
        ),
        1,
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                VANILLA_ENTITY_TYPE_COW_ID,
            ),
        ),
        0,
    );

    let (cow, cow_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 4.0,
            y: 64.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID)
                .expect("a cow is a living entity"),
        ),
    );
    crate::entity::living::set_health(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, cow)
            .expect("the cow is stored"),
        0.1,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {cow_uuid} 40 minecraft:generic"),
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                VANILLA_ENTITY_TYPE_COW_ID,
            ),
        ),
        0,
    );

    let (_killer, killer_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 6.0,
            y: 64.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
                .expect("a zombie is a living entity"),
        ),
    );
    let (victim, victim_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 8.0,
            y: 64.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID)
                .expect("a cow is a living entity"),
        ),
    );
    crate::entity::living::set_health(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, victim)
            .expect("the cow is stored"),
        0.1,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {victim_uuid} 40 minecraft:generic by {killer_uuid}"),
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                VANILLA_ENTITY_TYPE_COW_ID,
            ),
        ),
        0,
    );

    let (mannequin, mannequin_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        Vec3d {
            x: 10.0,
            y: 64.0,
            z: 0.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
                .expect("a mannequin is a living entity"),
        ),
    );
    crate::entity::living::set_health(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, mannequin)
            .expect("the mannequin is stored"),
        0.1,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {mannequin_uuid} 40 minecraft:generic by alice"),
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            ),
        ),
        1,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {mannequin_uuid} 40 minecraft:generic by alice"),
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            ),
        ),
        1,
    );
}
