//! `BlockBehaviour.onProjectileHit`: what a projectile Alice looses does to
//! the block it lands in.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
};
use bbb_server_types::stats::custom_stat;

use super::ranged::{archer, enchanted, entities_of, OVERWORLD};
use super::*;

const TARGET_Z: i32 = 6;

fn block(fixture: &mut Fixture, x: i32, y: i32) -> i32 {
    fixture
        .world
        .block_state(OVERWORLD, BlockPos { x, y, z: TARGET_Z })
        .unwrap()
}

fn property(fixture: &mut Fixture, x: i32, y: i32, name: &str) -> Option<&'static str> {
    bbb_protocol::block_state_types::vanilla_block_state_property(block(fixture, x, y), name)
}

fn is(fixture: &mut Fixture, x: i32, y: i32, resource_id: &str) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block(fixture, x, y))
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .is_some_and(|entry| entry.resource_id == resource_id)
}

/// Sends the projectile Alice just loosed, without gravity, straight at the
/// block face at `(x, y, TARGET_Z)` from two blocks out.
fn aim(fixture: &mut Fixture, id: i32, x: f64, y: f64, speed: f64) {
    let projectile = fixture
        .world
        .stored_entity_mut_for_test(OVERWORLD, id)
        .unwrap();
    projectile.set_pos(Vec3d {
        x,
        y,
        z: f64::from(TARGET_Z) - 2.0,
    });
    projectile.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: speed,
    });
    projectile.set_no_gravity(true);
}

fn newest(fixture: &Fixture, entity_type_id: i32, before: &[i32]) -> i32 {
    entities_of(fixture, entity_type_id)
        .iter()
        .map(|entity| entity.id())
        .find(|id| !before.contains(id))
        .expect("Alice loosed a projectile")
}

/// A full bow draw, its arrow then aimed at `(x, y)`.
fn shoot_arrow(fixture: &mut Fixture, bow: ItemStackSummary, x: f64, y: f64) -> Vec<TickResult> {
    let before: Vec<i32> = entities_of(fixture, VANILLA_ENTITY_TYPE_ARROW_ID)
        .iter()
        .map(|entity| entity.id())
        .collect();
    fixture.hold(HOTBAR_MENU_SLOT, bow);
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 5));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..20 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let arrow = newest(fixture, VANILLA_ENTITY_TYPE_ARROW_ID, &before);
    aim(fixture, arrow, x, y, 1.0);
    (0..4).map(|_| fixture.idle()).collect()
}

fn throw_trident(fixture: &mut Fixture, trident: ItemStackSummary, y: f64) -> Vec<TickResult> {
    fixture.hold(HOTBAR_MENU_SLOT, trident);
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..10 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let trident = newest(fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID, &[]);
    aim(fixture, trident, 0.5, y, 2.0);
    (0..4).map(|_| fixture.idle()).collect()
}

fn bow() -> ItemStackSummary {
    stack("minecraft:bow", 1)
}

fn stat(fixture: &Fixture, id: &str) -> i32 {
    fixture
        .world
        .player_stat_for_test(ConnectionId(ALICE), custom_stat(id))
}

#[test]
fn an_arrow_rings_a_bell_it_hits_from_the_side() {
    let mut fixture = archer();
    fixture.command("setblock 0 64 6 stone");
    fixture.command("setblock 0 65 6 bell[attachment=floor,facing=north]");

    let ticks = shoot_arrow(&mut fixture, bow(), 0.5, 65.5);

    let bell = BlockPos {
        x: 0,
        y: 65,
        z: TARGET_Z,
    };
    assert!(ticks
        .iter()
        .any(|tick| packets_to(tick, ALICE).any(|packet| matches!(
            packet,
            ClientboundPlayPacket::BlockEvent(event) if event.pos == bell
        ))));
    assert!(ticks
        .iter()
        .any(|tick| sounds(tick, ALICE).contains(&"minecraft:block.bell.use")));
    assert_eq!(stat(&fixture, "minecraft:bell_ring"), 1);
}

/// Dead centre from more than 30 blocks away is the strongest signal and
/// "Bullseye".
#[test]
fn an_arrow_in_a_target_powers_its_neighbours_and_counts_the_hit() {
    let mut fixture = archer();
    fixture.command("setblock 0 65 6 target");
    fixture.command("setblock 1 65 6 redstone_lamp");
    fixture.hold(HOTBAR_MENU_SLOT, bow());
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 5));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..20 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let arrow = newest(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID, &[]);
    aim(&mut fixture, arrow, 0.5, 65.5, 1.0);
    fixture.world.set_player_position_for_test(
        ConnectionId(ALICE),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: -30.5,
        },
    );
    for _ in 0..4 {
        fixture.idle();
    }

    assert_eq!(property(&mut fixture, 0, 65, "power"), Some("15"));
    assert_eq!(property(&mut fixture, 1, 65, "lit"), Some("true"));
    assert_eq!(stat(&fixture, "minecraft:target_hit"), 1);
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "minecraft:adventure/bullseye"));
}

#[test]
fn a_flaming_arrow_primes_tnt_and_lights_candles_and_campfires() {
    let mut fixture = archer();
    for x in [0, 3, 6] {
        fixture.command(&format!("setblock {x} 64 6 stone"));
    }
    fixture.command("setblock 0 65 6 tnt");
    fixture.command("setblock 3 65 6 candle");
    fixture.command("setblock 6 65 6 campfire[lit=false]");
    let flame = enchanted(&fixture, bow(), &[("minecraft:flame", 1)]);

    shoot_arrow(&mut fixture, flame.clone(), 0.5, 65.5);
    assert!(is(&mut fixture, 0, 65, "minecraft:air"));
    assert_eq!(entities_of(&fixture, VANILLA_ENTITY_TYPE_TNT_ID).len(), 1);

    shoot_arrow(&mut fixture, flame.clone(), 3.5, 65.2);
    assert_eq!(property(&mut fixture, 3, 65, "lit"), Some("true"));

    shoot_arrow(&mut fixture, flame, 6.5, 65.2);
    assert_eq!(property(&mut fixture, 6, 65, "lit"), Some("true"));
}

#[test]
fn arrows_break_pots_and_chorus_flowers_unless_the_rule_forbids_it() {
    let mut fixture = archer();
    fixture.command("setblock 0 65 6 decorated_pot");
    fixture.command("setblock 3 64 6 end_stone");
    fixture.command("setblock 3 65 6 chorus_flower");

    shoot_arrow(&mut fixture, bow(), 0.5, 65.5);
    assert!(is(&mut fixture, 0, 65, "minecraft:air"));
    shoot_arrow(&mut fixture, bow(), 3.5, 65.5);
    assert!(is(&mut fixture, 3, 65, "minecraft:air"));

    fixture.command("gamerule projectiles_can_break_blocks false");
    fixture.command("setblock 6 65 6 decorated_pot");
    shoot_arrow(&mut fixture, bow(), 6.5, 65.5);
    assert!(is(&mut fixture, 6, 65, "minecraft:decorated_pot"));
}

#[test]
fn an_arrow_chimes_amethyst_and_tilts_a_big_dripleaf() {
    let mut fixture = archer();
    fixture.command("setblock 0 65 6 amethyst_block");
    fixture.command("setblock 3 64 6 dirt");
    fixture.command("setblock 3 65 6 big_dripleaf[facing=north]");

    let ticks = shoot_arrow(&mut fixture, bow(), 0.5, 65.5);
    assert!(ticks
        .iter()
        .any(|tick| sounds(tick, ALICE).contains(&"minecraft:block.amethyst_block.chime")));

    shoot_arrow(&mut fixture, bow(), 3.5, 65.8);
    assert_eq!(property(&mut fixture, 3, 65, "tilt"), Some("full"));
}

#[test]
fn a_fast_trident_snaps_a_dripstone_tip() {
    let mut fixture = archer();
    fixture.command("setblock 0 64 6 stone");
    fixture.command("setblock 0 65 6 pointed_dripstone[vertical_direction=up,thickness=tip]");

    throw_trident(&mut fixture, stack("minecraft:trident", 1), 65.3);

    assert!(is(&mut fixture, 0, 65, "minecraft:air"));
}

/// Channeling's `hit_block` effect: in a thunderstorm a trident striking a
/// lightning rod under open sky calls a bolt down on it.
#[test]
fn a_channeling_trident_calls_lightning_onto_a_rod() {
    let mut fixture = archer();
    fixture.command("weather thunder 40000t");
    for _ in 0..200 {
        if fixture.world.level_weather(OVERWORLD).is_thundering() {
            break;
        }
        fixture.idle();
    }
    assert!(fixture.world.level_weather(OVERWORLD).is_thundering());
    fixture.command("setblock 0 64 6 stone");
    fixture.command("setblock 0 65 6 lightning_rod");
    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:channeling", 1)],
    );

    let ticks = throw_trident(&mut fixture, trident, 65.5);

    assert!(ticks.iter().any(|tick| {
        tick.events.iter().any(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::AddEntity(add))
                    if add.entity_type_id
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID
            )
        })
    }));
}

#[test]
fn player_arrows_respect_spawn_protection_for_ignition_and_breaking() {
    for (operator, target_x, allowed) in [(false, 0, false), (true, 0, true), (false, 6, true)] {
        let mut fixture = archer();
        fixture.command("setworldspawn 0 64 6");
        if operator {
            fixture.command("op alice");
        }
        enqueue(
            &fixture.sender,
            WorldCommand::Admin {
                envelope: admin_envelope(1, "projectile-spawn-protection", AdminScope::Manage),
                request: AdminRequest::SetSetting(AdminSetting::SpawnProtectionRadius(2)),
            },
        )
        .unwrap();
        fixture.idle();
        assert_eq!(fixture.world.admin_settings().spawn_protection_radius, 2);
        fixture.command(&format!("setblock {target_x} 64 6 stone"));
        fixture.command(&format!("setblock {target_x} 65 6 campfire[lit=false]"));
        fixture.command(&format!("setblock {} 65 6 decorated_pot", target_x + 1));
        let flame = enchanted(&fixture, bow(), &[("minecraft:flame", 1)]);

        shoot_arrow(&mut fixture, flame, f64::from(target_x) + 0.5, 65.2);
        assert_eq!(
            property(&mut fixture, target_x, 65, "lit"),
            Some(if allowed { "true" } else { "false" }),
            "operator={operator}, target_x={target_x}"
        );
        shoot_arrow(&mut fixture, bow(), f64::from(target_x) + 1.5, 65.5);
        assert!(
            is(
                &mut fixture,
                target_x + 1,
                65,
                if allowed {
                    "minecraft:air"
                } else {
                    "minecraft:decorated_pot"
                }
            ),
            "operator={operator}, target_x={target_x}"
        );
    }
}
