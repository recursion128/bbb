//! Panda retaliation and food pacification through real world ticks.

use super::*;
use crate::entity::EntityData;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
use bbb_protocol::packets::{InteractEntity, InteractionHand, ItemStackSummary};
use bbb_protocol::NbtSummaryValue;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    line: &str,
) {
    let operation = world.publication().tick;
    let tick = console_command(sender, receiver, world, operation, line);
    let outcome = admin_outcome(&tick);
    assert!(!outcome.replayed, "{line}");
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{line}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{line}: {report:?}");
}

fn prepare(
    gene: &str,
    no_ai: bool,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for line in [
        "gamerule advance_time false",
        "gamerule natural_health_regeneration false",
        "weather clear",
        "team add still_panda",
        "team modify still_panda collisionRule never",
        "attribute alice minecraft:knockback_resistance base set 1",
    ] {
        command(&sender, &receiver, &mut world, line);
    }
    command(&sender, &receiver, &mut world, &format!(
        "summon minecraft:panda 10.0 65 8.5 {{MainGene:\"{gene}\",HiddenGene:\"normal\",Age:0,PersistenceRequired:1b,NoAI:1b}}"
    ));
    let id = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_PANDA_ID)
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIMENSION, id).unwrap().uuid();
    for line in [
        format!("attribute {uuid} minecraft:movement_speed base set 0"),
        format!("attribute {uuid} minecraft:attack_damage base set 1"),
        format!("attribute {uuid} minecraft:knockback_resistance base set 1"),
        format!("team join still_panda {uuid}"),
    ] {
        command(&sender, &receiver, &mut world, &line);
    }
    if !no_ai {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:0b}}"),
        );
    }
    assert!(
        matches!(world.stored_entity_for_test(DIMENSION, id).unwrap().data(),
        EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
    );
    world.tick_rate_mut().set_frozen(false);
    world.tick(receiver.drain());
    world.tick_rate_mut().set_frozen(true);
    let before = world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:player_attack by alice"),
    );
    assert!(
        world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < before
    );
    assert_eq!(health(&world), 20.0);
    world.tick_rate_mut().set_frozen(false);
    (sender, receiver, world, id)
}

fn assert_in_reach(world: &ServerWorld, id: i32) {
    let panda = world.stored_entity_for_test(DIMENSION, id).unwrap();
    let player = world.publication().players[0].position;
    let at = panda.position();
    assert!(health(world) > 0.0 && panda.living().unwrap().health() > 0.0);
    assert!(
        (at.x - player.x).hypot(at.z - player.z) < 1.7 && (at.y - player.y).abs() < 0.2,
        "{at:?} vs {player:?}"
    );
}

#[test]
fn a_normal_panda_retaliates_once_but_a_no_ai_panda_never_bites() {
    for no_ai in [true, false] {
        let (_, receiver, mut world, panda) = prepare("normal", no_ai);
        let mut last = health(&world);
        let mut hits = 0;
        for _ in 0..240 {
            world.tick(receiver.drain());
            assert_in_reach(&world, panda);
            let now = health(&world);
            if now < last {
                hits += 1;
            }
            last = now;
        }
        assert_eq!(hits, usize::from(!no_ai), "NoAI={no_ai}, health={last}");
    }
}

#[test]
fn an_aggressive_panda_keeps_biting_until_actual_bamboo_feeding_calms_it() {
    let (sender, receiver, mut world, panda) = prepare("aggressive", false);
    let mut last = health(&world);
    let mut hits = 0;
    for _ in 0..200 {
        world.tick(receiver.drain());
        assert_in_reach(&world, panda);
        let now = health(&world);
        if now < last {
            hits += 1;
        }
        last = now;
        if hits >= 2 {
            break;
        }
    }
    assert!(hits >= 2, "aggressive retaliation health={last}");
    world.tick_rate_mut().set_frozen(true);
    world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:bamboo")
                    .unwrap()
                    .id,
            ),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: panda,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, 0);
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, panda)
    else {
        panic!("actual Panda record");
    };
    assert!(fields.iter().any(|field| field.name == "InLove"
        && matches!(field.value, NbtSummaryValue::Int(value) if value > 0)));
    assert!(
        matches!(world.stored_entity_for_test(DIMENSION, panda).unwrap().data(),
        EntityData::GeneralLiving(data) if !data.is_no_ai()
            && data.panda_retained().is_some_and(|state| !state.is_sitting()))
    );
    let after_food = health(&world);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..200 {
        world.tick(receiver.drain());
        assert_in_reach(&world, panda);
        assert_eq!(
            health(&world),
            after_food,
            "bamboo ends aggression without sitting"
        );
    }
}
