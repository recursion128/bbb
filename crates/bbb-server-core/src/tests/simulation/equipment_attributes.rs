//! `LivingEntity.collectEquipmentChanges`' attribute half
//! on the real tick, with the readers that
//! take the live value.

use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
    VANILLA_ATTRIBUTE_ATTACK_SPEED_ID, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
    VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
    VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::{PlayerCommand, PlayerCommandAction};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

#[test]
fn equipped_stacks_move_their_wearers_attributes_and_the_readers_follow() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let value =
        |world: &ServerWorld, id| world.player_attribute_value_for_test(ConnectionId(1), id);
    let run = |world: &mut ServerWorld, command: &str| {
        chat_command(&sender, &receiver, world, 1, command);
        world.tick(receiver.drain());
    };

    // Diamond sword: `+6` damage and `-2.4F` speed on the main hand.
    run(
        &mut world,
        "item replace entity alice weapon.mainhand with diamond_sword",
    );
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID), 7.0);
    assert_eq!(
        value(&world, VANILLA_ATTRIBUTE_ATTACK_SPEED_ID),
        4.0 + f64::from(-2.4_f32)
    );
    for (components, damage) in [
        ("max_damage=4,damage=4", 1.0),
        ("max_damage=4,damage=4,unbreakable={}", 7.0),
    ] {
        run(
            &mut world,
            &format!("item replace entity alice weapon.mainhand with diamond_sword[{components}]"),
        );
        assert_eq!(value(&world, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID), damage);
    }
    run(
        &mut world,
        "item replace entity alice armor.chest with iron_chestplate",
    );
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_ARMOR_ID), 6.0);

    // The sword's modifiers leave with it; efficiency V adds `5^2 + 1`, and
    // everything leaves with the stacks.
    run(
        &mut world,
        "item replace entity alice weapon.mainhand with diamond_pickaxe[enchantments={efficiency:5}]",
    );
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID), 5.0);
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID), 26.0);
    run(&mut world, "clear alice");
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_ARMOR_ID), 0.0);
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID), 1.0);
    assert_eq!(value(&world, VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID), 0.0);

    // `ServerPlayer.updatePlayerAttributes` and `LivingEntity.setSprinting`.
    run(&mut world, "gamemode creative alice");
    assert_eq!(
        value(&world, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID),
        5.0
    );
    assert_eq!(
        value(&world, VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID),
        5.0
    );
    let walking = value(&world, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    let entity_id = entity_id_of(&world, 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    assert_eq!(
        value(&world, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        walking * (1.0 + f64::from(0.3_f32))
    );

    // A stored mob takes its weapon the same way: zombie `3.0` plus the sword.
    let (zombie, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 4.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap()),
    );
    run(
        &mut world,
        "item replace entity @e[type=zombie,limit=1] weapon.mainhand with diamond_sword",
    );
    let attack = world
        .stored_entity_for_test(DIMENSION, zombie)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID);
    assert_eq!(attack, 9.0);

    // Only permanent modifiers are saved (`AttributeInstance.pack`); a rejoin
    // reapplies the worn chestplate's once through the same change detection.
    run(
        &mut world,
        "item replace entity alice armor.chest with iron_chestplate",
    );
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    join_loaded(&sender, &receiver, &mut world, 2, "alice", 1);
    world.tick(Vec::new());
    assert_eq!(
        world.player_attribute_value_for_test(ConnectionId(2), VANILLA_ATTRIBUTE_ARMOR_ID),
        6.0
    );
}
