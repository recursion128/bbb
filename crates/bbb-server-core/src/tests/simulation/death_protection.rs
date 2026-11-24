//! `LivingEntity.checkTotemDeathProtection`
//! for a session and for a stored mob, reached through `/damage` and `/kill`.

use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

use crate::tests::using_item::{
    entity_events, sounds, stack, Fixture, ALICE, BOB, HOTBAR_MENU_SLOT, HOTBAR_SLOT,
    OFFHAND_MENU_SLOT, OFFHAND_SLOT,
};

const TOTEM: &str = "minecraft:totem_of_undying";

fn effect(world: &ServerWorld, resource_id: &str) -> Option<i32> {
    world
        .player_active_effect_for_test(ConnectionId(ALICE), resource_id)
        .map(|effect| effect.amplifier())
}

#[test]
fn an_offhand_totem_cancels_a_lethal_hit_but_not_a_kill() {
    let mut fixture = Fixture::new(GameType::Survival);
    let alice = fixture.alice;
    fixture.command("effect give alice minecraft:speed 60");
    assert_eq!(effect(&fixture.world, "minecraft:speed"), Some(0));
    fixture.hold(OFFHAND_MENU_SLOT, stack(TOTEM, 1));

    let hurt = fixture.command("damage alice 100");
    // Health is set to one; regeneration II's first effect tick (duration 900,
    // interval 25) may already have healed one more.
    let health = fixture.world.player_health_for_test(ConnectionId(ALICE));
    assert!(health == 1.0 || health == 2.0, "{health}");
    assert!(
        fixture.slot(OFFHAND_SLOT).item_id.is_none(),
        "the totem is spent"
    );
    for viewer in [ALICE, BOB] {
        let events = entity_events(&hurt, viewer, alice);
        assert!(
            events.contains(&35) && !events.contains(&3),
            "{viewer}: {events:?}"
        );
        assert!(!sounds(&hurt, viewer).contains(&"minecraft:entity.player.death"));
    }
    // `DeathProtection.TOTEM_OF_UNDYING`.
    assert_eq!(effect(&fixture.world, "minecraft:speed"), None);
    assert_eq!(effect(&fixture.world, "minecraft:regeneration"), Some(1));
    assert_eq!(effect(&fixture.world, "minecraft:absorption"), Some(1));
    assert_eq!(effect(&fixture.world, "minecraft:fire_resistance"), Some(0));
    assert_eq!(fixture.item_used(TOTEM), 1);
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "minecraft:adventure/totem_of_undying"));

    // `generic_kill` is in `bypasses_invulnerability` (`:1362-1364`).
    fixture.hold(HOTBAR_MENU_SLOT, stack(TOTEM, 1));
    let killed = fixture.command("kill alice");
    assert_eq!(
        fixture.world.player_health_for_test(ConnectionId(ALICE)),
        0.0
    );
    // Unspent, it leaves with the rest of the inventory.
    assert_eq!(fixture.item_used(TOTEM), 1);
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 0);
    assert!(entity_events(&killed, BOB, alice).contains(&3));
}

#[test]
fn a_spent_totem_and_its_effects_survive_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-death-protection-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-death-protection");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = || ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config(), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    world.stock_menu_slot_for_test(ConnectionId(ALICE), OFFHAND_MENU_SLOT, stack(TOTEM, 1));
    chat_command(&sender, &receiver, &mut world, ALICE, "damage alice 100");
    let survived = world.player_health_for_test(ConnectionId(ALICE));
    assert!(survived == 1.0 || survived == 2.0, "{survived}");

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config(), &reloaded).unwrap();
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    assert!(world
        .player_inventory_slot_for_test(ConnectionId(ALICE), OFFHAND_SLOT)
        .item_id
        .is_none());
    assert_eq!(effect(&world, "minecraft:regeneration"), Some(1));
    assert_eq!(effect(&world, "minecraft:fire_resistance"), Some(0));
    let health = world.player_health_for_test(ConnectionId(ALICE));
    assert!((1.0..20.0).contains(&health), "{health}");
    storage.close().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_zombie_holding_a_totem_survives_a_lethal_hit_but_not_a_kill() {
    let mut fixture = Fixture::new(GameType::Survival);
    let position = fixture
        .world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(BOB))
        .unwrap()
        .position;
    let (zombie, uuid) = fixture.world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap()),
    );
    let equip = |world: &mut ServerWorld| {
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_item_slot(
                EquipmentSlot::MainHand,
                Some(crate::entity::item::plain_stack(TOTEM, 1).unwrap()),
            );
    };
    equip(&mut fixture.world);
    fixture.idle();

    let hurt = fixture.command(&format!("damage {uuid} 100"));
    let body = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the protected zombie is still filed");
    let living = body.living().unwrap();
    assert_eq!(living.health(), 1.0);
    assert!(!living.has_item_in_slot(EquipmentSlot::MainHand));
    // `#minecraft:undead` is in `ignores_poison_and_regen`, so the totem's
    // regeneration does not take.
    assert!(living.active_effect("minecraft:regeneration").is_none());
    assert_eq!(
        living
            .active_effect("minecraft:absorption")
            .map(|effect| effect.amplifier()),
        Some(1)
    );
    assert!(living.active_effect("minecraft:fire_resistance").is_some());
    assert!(entity_events(&hurt, BOB, zombie).contains(&35), "{hurt:?}");

    equip(&mut fixture.world);
    let killed = fixture.command(&format!("kill {uuid}"));
    let body = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie);
    let living = body.and_then(|body| body.living());
    assert!(
        living.is_none_or(|living| living.is_dead_or_dying()),
        "{killed:?}"
    );
}

#[test]
fn a_teleporting_totem_moves_the_saved_zombie() {
    let mut fixture = Fixture::new(GameType::Survival);
    let position = fixture
        .world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(BOB))
        .unwrap()
        .position;
    let start = Vec3d {
        x: position.x + 2.0,
        ..position
    };
    let (zombie, uuid) = fixture.world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        start,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap()),
    );
    let pig_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    let (vehicle, vehicle_uuid) = fixture.world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        pig_type,
        start,
        EntityData::GeneralLiving(GeneralLivingData::new(pig_type).unwrap()),
    );
    fixture.command(&format!("ride {uuid} mount {vehicle_uuid}"));
    assert_eq!(
        fixture
            .world
            .stored_passengers_for_test(Java26BuiltinDimension::Overworld, vehicle),
        &[zombie]
    );
    fixture.command(&format!(
        "item replace entity {uuid} weapon.mainhand with {TOTEM}[death_protection=\
         {{death_effects:[{{type:\"minecraft:teleport_randomly\"}}]}}]"
    ));
    fixture.idle();

    let hurt = fixture.command(&format!("damage {uuid} 100"));
    let body = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the protected zombie is still filed");
    assert_eq!(body.living().unwrap().health(), 1.0);
    assert_ne!(body.position(), start);
    assert!(fixture
        .world
        .stored_passengers_for_test(Java26BuiltinDimension::Overworld, vehicle)
        .is_empty());
    for viewer in [ALICE, BOB] {
        assert!(
            hurt.events.iter().any(|event| matches!(
                event.packet_for(ConnectionId(viewer)),
                Some(ClientboundPlayPacket::SetPassengers(update))
                    if update.vehicle_id == vehicle && update.passenger_ids.is_empty()
            )),
            "{viewer}: {hurt:?}"
        );
    }
    let events = entity_events(&hurt, BOB, zombie);
    assert!(events.contains(&46) && events.contains(&35), "{events:?}");
    assert!(sounds(&hurt, BOB).contains(&"minecraft:item.chorus_fruit.teleport"));
}
