//! `EnchantmentHelper.doPostAttackEffects` (`EnchantmentHelper.java:210-262`)
//! after a player's melee hit.

use super::*;

const PLAYER: ConnectionId = ConnectionId(1);
const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn survival_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, Vec3d) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.admin_lists = operator_config(&[("fighter", 0xf1647e)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "fighter", 0xf1647e);
    let position = world.publication().players[0].position;
    (sender, receiver, world, position)
}

fn spawn_living(world: &mut ServerWorld, type_id: i32, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            type_id,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(type_id).unwrap(),
            ),
        )
        .0
}

fn enchanted(world: &ServerWorld, item_id: i32, enchantments: &[(&str, i32)]) -> ItemStackSummary {
    let mut stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut entries = Vec::new();
    for (enchantment, level) in enchantments {
        super::sheep_shearing::enchant_for_entity_test(world, &mut stack, enchantment, *level);
        entries.append(&mut stack.component_patch.enchantments);
    }
    stack.component_patch.enchantments = entries;
    stack
}

fn attack(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    entity_id: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id }),
    )
}

/// Fire Aspect ignites the victim for `4 * level` seconds and Bane of
/// Arthropods slows an arthropod, both from the attacker's weapon.
#[test]
fn a_weapons_post_attack_enchantments_ignite_and_slow_the_victim() {
    let (sender, receiver, mut world, position) = survival_world();
    let sword = enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID,
        &[
            ("minecraft:fire_aspect", 2),
            ("minecraft:bane_of_arthropods", 5),
        ],
    );
    world.set_inventory_slot_for_test(PLAYER, 0, sword);
    for _ in 0..15 {
        world.tick(Vec::new());
    }
    world.set_player_ground_for_test(PLAYER, true, 0.0);
    let spider = spawn_living(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPIDER_ID,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );

    attack(&sender, &receiver, &mut world, spider);

    let spider = world.stored_entity_for_test(OVERWORLD, spider).unwrap();
    assert!(
        spider.remaining_fire_ticks() >= 159,
        "{}",
        spider.remaining_fire_ticks()
    );
    let slowness = spider
        .living()
        .unwrap()
        .active_effect("minecraft:slowness")
        .expect("bane of arthropods slows the spider");
    assert_eq!(slowness.amplifier(), 3);
    // `randomBetween(1.5, 3.5) * 20`, less the tick since.
    assert!(
        (29..=70).contains(&slowness.duration()),
        "{}",
        slowness.duration()
    );
}

/// Thorns on the victim's armour hurts the attacker with `thorns` damage and
/// costs the armour two durability (`Enchantments.java` thorns entry).
#[test]
fn thorns_armor_on_the_victim_hurts_the_attacker_and_wears_itself() {
    let (sender, receiver, mut world, position) = survival_world();
    let cow = spawn_living(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );
    let chestplate = enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_CHESTPLATE_ID,
        &[("minecraft:thorns", 3)],
    );
    let native = Some(bbb_server_types::item_stack::RuntimeItemStack::from_summary(chestplate));
    world
        .stored_entity_mut_for_test(OVERWORLD, cow)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(bbb_protocol::equipment_slots::EquipmentSlot::Chest, native);
    world.set_player_ground_for_test(PLAYER, true, 0.0);
    let thorns = world.damage_type_registry_id_for_test("minecraft:thorns");
    let player_entity = world.player_entity_id_for_test(PLAYER);

    let mut thorned = false;
    for _ in 0..9 {
        for _ in 0..11 {
            world.tick(Vec::new());
        }
        world.set_player_ground_for_test(PLAYER, true, 0.0);
        let attacked = attack(&sender, &receiver, &mut world, cow);
        thorned = attacked.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::DamageEvent(damage),
                    ..
                } if damage.entity_id == player_entity
                    && damage.source_type_id == thorns
                    && damage.source_cause_id == cow
            )
        });
        if thorned {
            break;
        }
    }
    assert!(thorned, "a 45% roll failed nine times");
    assert!(world.player_health_for_test(PLAYER) < 20.0);
    let worn = world
        .stored_entity_for_test(OVERWORLD, cow)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Chest)
        .cloned()
        .unwrap();
    assert_eq!(worn.summary().component_patch.damage, Some(2));
}

/// Wind Burst on a falling mace swing explodes at the attacker
/// (`ExplodeEffect.java:72-91`), a wind-charge burst the client is told of.
#[test]
fn a_falling_wind_burst_mace_bursts_at_the_attacker() {
    let (sender, receiver, mut world, position) = survival_world();
    let mace = enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_MACE_ID,
        &[("minecraft:wind_burst", 1)],
    );
    world.set_inventory_slot_for_test(PLAYER, 0, mace);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    let cow = spawn_living(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );
    world.set_player_ground_for_test(PLAYER, false, 4.0);

    let smashed = attack(&sender, &receiver, &mut world, cow);

    let bursts: Vec<_> = smashed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::Explode(update),
            } => Some(update.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(bursts.len(), 1, "{bursts:?}");
    assert!((bursts[0].center.y - position.y).abs() < 1e-6, "{bursts:?}");
}
