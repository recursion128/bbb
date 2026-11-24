use super::*;
use crate::entity::{MerchantItemCost, MerchantOffer, MerchantOffers};

fn stationary(village: &mut Village, kind: i32, dx: i32, dz: i32) -> i32 {
    let id = village.mob(kind, village.at(dx, dz));
    village.merge(id, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
    id
}

fn gossip(village: &Village, id: i32, target: uuid::Uuid, kind: &str) -> Option<i32> {
    let NbtSummaryValue::Compound(record) = village
        .world
        .stored_entity_command_data_for_test(OVERWORLD, id)?
    else {
        return None;
    };
    let NbtSummaryValue::List(gossips) = &record.iter().find(|e| e.name == "Gossips")?.value else {
        return None;
    };
    let target = crate::entity::save::uuid_int_array(target);
    gossips.iter().find_map(|gossip| {
        let NbtSummaryValue::Compound(fields) = gossip else {
            return None;
        };
        let field = |name| fields.iter().find(|e| e.name == name).map(|e| &e.value);
        match (field("Target"), field("Type"), field("Value")) {
            (Some(uuid), Some(NbtSummaryValue::String(id)), Some(NbtSummaryValue::Int(value)))
                if *uuid == target && id == kind =>
            {
                Some(*value)
            }
            _ => None,
        }
    })
}

#[test]
fn hurting_a_villager_accumulates_saved_reputation_and_raises_its_trade_price() {
    let mut village = Village::new();
    let trader = stationary(&mut village, VILLAGER, 0, 0);
    village.merge(trader, vec![entry("Xp", NbtSummaryValue::Int(1))]);
    let trader_uuid = village.entity(trader).unwrap().uuid();
    let player = village.world.publication().players[0].uuid;
    for hit in 1..=9 {
        village.command(&format!(
            "damage {trader_uuid} 1 minecraft:player_attack by mayor"
        ));
        assert_eq!(
            gossip(&village, trader, player, "minor_negative"),
            Some((hit * 25).min(200))
        );
        village.tick_until(21, |_| false);
    }

    let NbtSummaryValue::Compound(saved) = village
        .world
        .stored_entity_command_data_for_test(OVERWORLD, trader)
        .unwrap()
    else {
        panic!()
    };
    village
        .world
        .load_stored_entity_record_for_test(OVERWORLD, trader, &saved);
    assert_eq!(
        gossip(&village, trader, player, "minor_negative"),
        Some(200)
    );
    let offer = MerchantOffer::new(
        MerchantItemCost::new("minecraft:wheat".into(), 20, Vec::new()).unwrap(),
        None,
        crate::entity::item::plain_stack("minecraft:emerald", 1).unwrap(),
        16,
        2,
        0.05,
    );
    village
        .world
        .stored_entity_mut_for_test(OVERWORLD, trader)
        .unwrap()
        .initialize_merchant_offers(MerchantOffers::new(vec![offer]));
    village.merge(
        trader,
        vec![entry(
            "VillagerData",
            NbtSummaryValue::Compound(vec![
                entry("level", NbtSummaryValue::Int(1)),
                entry(
                    "profession",
                    NbtSummaryValue::String("minecraft:farmer".into()),
                ),
                entry("type", NbtSummaryValue::String("minecraft:plains".into())),
            ]),
        )],
    );
    let position = village.center(village.at(0, 0));
    village.command(&format!(
        "tp {trader_uuid} {} {} {}",
        position.x, position.y, position.z
    ));
    let opened = play(
        &village.sender,
        &village.receiver,
        &mut village.world,
        1,
        super::super::feeding::interact_entity_packet(trader),
    );
    assert!(opened
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Send {
        packet: ClientboundPlayPacket::MerchantOffers(offers), ..
    } if offers.offers[0].special_price_diff == 10)));
}

#[test]
fn hurt_gossip_names_the_current_living_attacker_and_respects_damage_acceptance() {
    let mut village = Village::new();
    let victim = stationary(&mut village, VILLAGER, 0, 0);
    let wolf = stationary(
        &mut village,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
        4,
        0,
    );
    let player = village.world.publication().players[0].uuid;
    village.merge(
        wolf,
        vec![entry("Owner", crate::entity::save::uuid_int_array(player))],
    );
    let victim_uuid = village.entity(victim).unwrap().uuid();
    let wolf_uuid = village.entity(wolf).unwrap().uuid();
    village.command(&format!(
        "damage {victim_uuid} 1 minecraft:mob_attack by {wolf_uuid}"
    ));
    assert_eq!(
        gossip(&village, victim, wolf_uuid, "minor_negative"),
        Some(25)
    );
    assert_eq!(gossip(&village, victim, player, "minor_negative"), None);
    village.command(&format!(
        "damage {victim_uuid} 1 minecraft:player_attack by mayor"
    ));
    assert_eq!(gossip(&village, victim, player, "minor_negative"), None);
    village.tick_until(21, |_| false);
    village.command(&format!(
        "damage {victim_uuid} 1 minecraft:mob_attack_no_aggro by mayor"
    ));
    assert_eq!(gossip(&village, victim, player, "minor_negative"), None);
    village.tick_until(21, |_| false);
    village.command(&format!(
        "damage {victim_uuid} 0 minecraft:player_attack by mayor"
    ));
    assert_eq!(gossip(&village, victim, player, "minor_negative"), Some(25));
}

#[test]
fn visible_villagers_remember_murder_including_zombie_conversion_and_cap_the_gossip() {
    let mut village = Village::new();
    village.command("time set midnight");
    village.command("difficulty hard");
    let witness = stationary(&mut village, VILLAGER, -3, 0);
    let blocked = stationary(&mut village, VILLAGER, 3, 0);
    let distant = stationary(&mut village, VILLAGER, 0, 20);
    let invisible = stationary(&mut village, VILLAGER, -6, 0);
    village.command(&format!(
        "effect give {} minecraft:invisibility infinite",
        village.entity(invisible).unwrap().uuid()
    ));
    for dz in -1..=1 {
        for dy in 0..=2 {
            let mut pos = village.at(1, dz);
            pos.y += dy;
            village.set_block(pos, "minecraft:stone");
        }
    }
    let zombie = stationary(&mut village, ZOMBIE, -4, 2);
    let zombie_uuid = village.entity(zombie).unwrap().uuid();
    let victim = stationary(&mut village, VILLAGER, 0, 0);
    village.command(&format!(
        "damage {} 40 minecraft:mob_attack by {zombie_uuid}",
        village.entity(victim).unwrap().uuid()
    ));
    assert_eq!(
        village.count_of(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID),
        1
    );
    assert_eq!(
        gossip(&village, witness, zombie_uuid, "major_negative"),
        Some(25)
    );
    for excluded in [blocked, distant, invisible] {
        assert_eq!(
            gossip(&village, excluded, zombie_uuid, "major_negative"),
            None
        );
    }
    let player = village.world.publication().players[0].uuid;
    for kill in 1..=5 {
        let victim = stationary(&mut village, VILLAGER, 0, 0);
        village.command(&format!(
            "damage {} 40 minecraft:player_attack by mayor",
            village.entity(victim).unwrap().uuid()
        ));
        assert_eq!(
            gossip(&village, witness, player, "major_negative"),
            Some((kill * 25).min(100))
        );
    }
}
