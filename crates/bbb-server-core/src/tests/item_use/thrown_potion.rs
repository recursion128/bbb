//! `SplashPotionItem.use` / `LingeringPotionItem.use`: what a thrown potion
//! does where it lands, to blocks and to the player who threw it.

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
};
use bbb_protocol::packets::{BlockPos, DataComponentPatchSummary, ItemStackSummary, Vec3d};
use bbb_protocol::potion_types::vanilla_potion_for_resource_id;

use super::*;

fn default_state(block: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block).unwrap();
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap()
}

fn potions(world: &ServerWorld) -> usize {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SPLASH_POTION_ID)
        .count()
}

#[test]
fn a_splash_water_potion_dowses_fire_and_campfires_where_it_lands() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let fire = BlockPos { x: 1, y: 64, z: 0 };
    let campfire = BlockPos { x: -1, y: 64, z: 0 };
    let lit_campfire = default_state("minecraft:campfire");
    let overworld = Java26BuiltinDimension::Overworld;
    world.set_block(
        overworld,
        fire,
        default_state("minecraft:fire"),
        SET_BLOCK_UPDATE_ALL,
    );
    world.set_block(overworld, campfire, lit_campfire, SET_BLOCK_UPDATE_ALL);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:splash_potion")),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(
                        vanilla_potion_for_resource_id("minecraft:water")
                            .unwrap()
                            .id,
                    ),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );

    // Straight down: it lands on the floor cell under the player.
    throw(&sender, &receiver, &mut world, 1, 0.0, 90.0, 1);
    assert_eq!(potions(&world), 1);
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 0);
    for _ in 0..40 {
        if potions(&world) == 0 {
            break;
        }
        world.tick(receiver.drain());
    }

    assert_eq!(potions(&world), 0);
    assert_eq!(live_block(&mut world, fire), Some(0));
    let dowsed = live_block(&mut world, campfire).unwrap();
    assert_ne!(dowsed, lit_campfire);
    assert!(crate::block_behavior::campfire::is_campfire(dowsed));
}

fn potion_stack(item: &str, potion: &str) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id(item)),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                potion_id: Some(vanilla_potion_for_resource_id(potion).unwrap().id),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

/// A survival session at (0.5, 64, 0.5) holding `item` of `potion`.
fn holding_potion(
    item: &str,
    potion: &str,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    world.set_inventory_slot_for_test(ConnectionId(1), 0, potion_stack(item, potion));
    (sender, receiver, world)
}

/// Throws straight down at the player's own feet and ticks until the
/// projectile is gone, then `extra` more ticks; every event on the way.
fn throw_down(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    extra: usize,
) -> Vec<WorldEvent> {
    let mut events = throw(sender, receiver, world, 1, 0.0, 90.0, 1).events;
    let in_flight = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
            .any(|entity| {
                matches!(
                    entity.entity_type_id(),
                    VANILLA_ENTITY_TYPE_SPLASH_POTION_ID | VANILLA_ENTITY_TYPE_LINGERING_POTION_ID
                )
            })
    };
    for _ in 0..40 {
        if !in_flight(world) {
            break;
        }
        events.extend(world.tick(receiver.drain()).events);
    }
    assert!(!in_flight(world));
    for _ in 0..extra {
        events.extend(world.tick(receiver.drain()).events);
    }
    events
}

fn effect_updates(events: &[WorldEvent]) -> Vec<(i32, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::UpdateMobEffect(update),
            } => Some((update.effect_id, update.duration_ticks)),
            _ => None,
        })
        .collect()
}

fn effect_id(resource_id: &str) -> i32 {
    bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(resource_id)
        .unwrap()
        .id
}

#[test]
fn a_splash_potion_at_the_feet_gives_the_thrower_its_full_effect() {
    let (sender, receiver, mut world) =
        holding_potion("minecraft:splash_potion", "minecraft:poison");
    let events = throw_down(&sender, &receiver, &mut world, 0);

    // Landing inside the player's box: distance 0, scale 1, all 900 ticks.
    assert_eq!(
        effect_updates(&events),
        vec![(effect_id("minecraft:poison"), 900)]
    );
    assert!(world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:poison")
        .is_some());
}

#[test]
fn a_splash_harming_potion_hurts_the_thrower_six_points() {
    let (sender, receiver, mut world) =
        holding_potion("minecraft:splash_potion", "minecraft:harming");
    throw_down(&sender, &receiver, &mut world, 0);

    assert_eq!(world.player_health_for_test(ConnectionId(1)), 14.0);
}

#[test]
fn a_splash_healing_potion_heals_the_thrower_four_points() {
    let (sender, receiver, mut world) =
        holding_potion("minecraft:splash_potion", "minecraft:healing");
    world.set_player_health_for_test(ConnectionId(1), 10.0);
    throw_down(&sender, &receiver, &mut world, 0);

    assert_eq!(world.player_health_for_test(ConnectionId(1)), 14.0);
}

#[test]
fn a_splash_water_potion_puts_out_a_burning_thrower() {
    let (sender, receiver, mut world) =
        holding_potion("minecraft:splash_potion", "minecraft:water");
    world.set_player_fire_ticks_for_test(ConnectionId(1), 200);
    let events = throw_down(&sender, &receiver, &mut world, 0);

    assert!(world.player_fire_ticks_for_test(ConnectionId(1)) <= 0);
    let extinguish =
        vanilla_sound_event_registry_id("minecraft:entity.generic.extinguish_fire").unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } if sound.sound == SoundEventHolder::Reference { registry_id: extinguish }
    )));
}

#[test]
fn a_lingering_cloud_gives_the_player_standing_in_it_its_effect() {
    let (sender, receiver, mut world) =
        holding_potion("minecraft:lingering_potion", "minecraft:regeneration");
    // Wait time 10, then the next multiple of five.
    let events = throw_down(&sender, &receiver, &mut world, 20);

    // `PotionContents.forEachEffect` scales by the lingering item's 0.25.
    let updates = effect_updates(&events);
    assert_eq!(updates, vec![(effect_id("minecraft:regeneration"), 225)]);
}
