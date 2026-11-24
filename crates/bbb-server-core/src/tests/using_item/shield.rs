//! A raised shield (`Items.SHIELD`'s `BlocksAttacks`, `Items.java:1842-1853`)
//! against `/damage`, as `LivingEntity.hurtServer` and `applyItemBlocking`
//! (`LivingEntity.java:1155-1320`) run it for a session.

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_BROKEN_ID;
use bbb_server_types::stats::custom_stat;

use super::*;
use crate::entity::{EntityData, GeneralLivingData};

/// Alice two blocks in front of Bob's line of sight: she faces `+z` and Bob
/// stands at `+z`.
fn face_to_face(fixture: &mut Fixture) -> Vec3d {
    let alice = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    fixture.world.set_player_position_for_test(
        ConnectionId(BOB),
        Vec3d {
            z: alice.z + 2.0,
            ..alice
        },
    );
    alice
}

/// A shield raised in the off hand, idled until `ticks_used` ticks of use
/// have run.
fn raise_shield(fixture: &mut Fixture, shield: ItemStackSummary, ticks_used: i32) {
    fixture.hold(OFFHAND_MENU_SLOT, shield);
    let raised = fixture.use_item(InteractionHand::OffHand);
    assert_eq!(living_flags(&raised, BOB, fixture.alice), [3]);
    for _ in 1..ticks_used {
        fixture.idle();
    }
    assert_eq!(fixture.use_state().2, 72000 - ticks_used);
}

fn damaged(mut item: ItemStackSummary, damage: i32) -> ItemStackSummary {
    let patch = &mut item.component_patch;
    patch.damage = Some(damage);
    patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    item
}

fn health(fixture: &Fixture) -> f32 {
    fixture.world.player_health_for_test(ConnectionId(ALICE))
}

fn stat(fixture: &Fixture, name: &str) -> i32 {
    fixture
        .world
        .player_stat_for_test(ConnectionId(ALICE), custom_stat(name))
}

fn damage_events_to(tick: &TickResult, viewer: u64) -> usize {
    packets_to(tick, viewer)
        .filter(|packet| matches!(packet, ClientboundPlayPacket::DamageEvent(_)))
        .count()
}

fn motions(tick: &TickResult, viewer: u64, entity_id: i32) -> Vec<Vec3d> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SetEntityMotion(motion) if motion.id == entity_id => {
                Some(motion.delta_movement)
            }
            _ => None,
        })
        .collect()
}

fn hurt_animations(tick: &TickResult, viewer: u64) -> Vec<f32> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::HurtAnimation(animation) => Some(animation.yaw),
            _ => None,
        })
        .collect()
}

fn assert_close(actual: Vec3d, expected: Vec3d) {
    let close = |a: f64, b: f64| (a - b).abs() < 1.0e-9;
    assert!(
        close(actual.x, expected.x) && close(actual.y, expected.y) && close(actual.z, expected.z),
        "{actual:?} != {expected:?}"
    );
}

/// `getItemBlockingWith` (`LivingEntity.java:3568-3582`): the shield's
/// quarter second is five ticks of use, and a hit inside them lands in full,
/// knocking Alice back from Bob through `markHurt` and `indicateDamage`.
#[test]
fn a_shield_blocks_nothing_before_its_block_delay() {
    let mut fixture = Fixture::new(GameType::Survival);
    face_to_face(&mut fixture);
    raise_shield(&mut fixture, stack("minecraft:shield", 1), 3);

    let hit = fixture.command("damage alice 4 minecraft:mob_attack by bob");
    assert_eq!(health(&fixture), 16.0);
    assert_eq!(damage_events_to(&hit, ALICE), 1);
    assert_eq!(damage_events_to(&hit, BOB), 1);
    assert!(sounds(&hit, BOB)
        .iter()
        .all(|sound| *sound != "minecraft:item.shield.block"));
    // `knockback(0.4, 0, 2)` off the ground: half the old velocity, less the
    // push toward Bob, plus the 0.4 lift. `markHurt` tells Alice herself.
    let pushed = Vec3d {
        x: 0.0,
        y: 0.4,
        z: -0.4,
    };
    assert_close(motions(&hit, ALICE, fixture.alice)[0], pushed);
    assert_close(motions(&hit, BOB, fixture.alice)[0], pushed);
    // `ServerPlayer.indicateDamage` (`ServerPlayer.java:2115-2118`): Bob
    // straight ahead is `atan2(2, 0)`, ninety degrees.
    assert_eq!(hurt_animations(&hit, ALICE), [90.0]);
    assert!(hurt_animations(&hit, BOB).is_empty());
    assert_eq!(stat(&fixture, "minecraft:damage_blocked_by_shield"), 0);
}

/// Past the delay a hit from the front is blocked whole: the shield sound
/// instead of the damage event, `hurtBlockingItem`'s statistic and wear,
/// `blockedByItem`'s push, and no `markHurt` for a hit that did no damage.
#[test]
fn a_raised_shield_blocks_a_hit_from_the_front() {
    let mut fixture = Fixture::new(GameType::Survival);
    face_to_face(&mut fixture);
    raise_shield(&mut fixture, stack("minecraft:shield", 1), 5);

    let hit = fixture.command("damage alice 4 minecraft:mob_attack by bob");
    assert_eq!(health(&fixture), 20.0);
    assert_eq!(damage_events_to(&hit, ALICE), 0);
    assert_eq!(damage_events_to(&hit, BOB), 0);
    for viewer in [ALICE, BOB] {
        assert!(
            sounds(&hit, viewer).contains(&"minecraft:item.shield.block"),
            "{viewer}"
        );
    }
    // `ItemDamageFunction(3, 1, 1)` over four blocked: five points of wear.
    assert_eq!(fixture.slot(OFFHAND_SLOT).component_patch.damage, Some(5));
    assert_eq!(fixture.item_used("minecraft:shield"), 1);
    assert_eq!(stat(&fixture, "minecraft:damage_blocked_by_shield"), 40);
    // `blockedByItem`'s `knockback(0.5, 0, -2)` first, then the hit's own
    // `knockback(0.4, 0, 2)`; only the trackers hear of it, off `needsSync`.
    assert!(motions(&hit, ALICE, fixture.alice).is_empty());
    assert_close(
        motions(&hit, BOB, fixture.alice)[0],
        Vec3d {
            x: 0.0,
            y: 0.4,
            z: 0.25 - 0.4,
        },
    );
    assert!(hurt_animations(&hit, ALICE).is_empty());
    assert!(fixture.use_state().0);
}

#[test]
fn adult_hoglins_and_zoglins_throw_a_shield_defender_unless_resisted() {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID;

    for kind in [VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID] {
        for (baby, resistance) in [(false, 0.2), (true, 0.2), (false, 0.8)] {
            let mut fixture = Fixture::new(GameType::Survival);
            let alice = face_to_face(&mut fixture);
            fixture.command("gamemode spectator bob");
            fixture.command(&format!(
                "attribute alice minecraft:knockback_resistance base set {resistance}"
            ));
            let (source, uuid) = fixture.world.spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                kind,
                Vec3d {
                    z: alice.z + 2.0,
                    ..alice
                },
                EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
            );
            let body = fixture
                .world
                .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, source)
                .unwrap();
            if kind == VANILLA_ENTITY_TYPE_HOGLIN_ID {
                assert!(crate::entity::general_living::set_spawner_age(
                    body,
                    if baby { -24_000 } else { 0 }
                ));
            } else {
                assert!(crate::entity::general_living::set_zoglin_baby(body, baby));
            }
            body.living_mut()
                .unwrap()
                .attributes_mut()
                .get_instance(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID)
                .unwrap()
                .set_base_value(0.6);
            raise_shield(&mut fixture, stack("minecraft:shield", 1), 5);
            // Off the ground, so `knockback` keeps the throw's rise: the
            // tick's own travel finds nothing under a body a block up.
            fixture.command("execute as alice at @s run tp @s ~ ~1 ~");
            let hit = fixture.command(&format!("damage alice 4 minecraft:mob_attack by {uuid}"));
            assert_eq!(health(&fixture), 20.0);
            assert!(sounds(&hit, ALICE).contains(&"minecraft:item.shield.block"));
            let motion = motions(&hit, BOB, fixture.alice)[0];
            let ordinary_hit = 0.4 * (1.0 - resistance);
            if !baby && resistance < 0.6 {
                let horizontal_throw = motion.x.hypot(motion.z + ordinary_hit);
                assert!(
                    (0.04 - 1e-6..0.14 + 1e-6).contains(&horizontal_throw),
                    "{kind}: {motion:?}"
                );
                assert!((0.0..=0.2).contains(&motion.y));
                for viewer in [ALICE, BOB] {
                    assert_close(motions(&hit, viewer, fixture.alice)[0], motion);
                }
            } else {
                assert_close(
                    motion,
                    Vec3d {
                        x: 0.0,
                        y: 0.0,
                        z: -ordinary_hit,
                    },
                );
                assert!(motions(&hit, ALICE, fixture.alice).is_empty());
                assert_close(motions(&hit, BOB, fixture.alice)[0], motion);
            }
        }
    }
}

/// `DamageReduction.resolve` (`BlocksAttacks.java:159-167`): a source behind
/// Alice is outside the shield's ninety degrees, and `hurtBlockingItem`
/// still counts the use.
#[test]
fn a_shield_does_not_block_a_hit_from_behind() {
    let mut fixture = Fixture::new(GameType::Survival);
    let alice = face_to_face(&mut fixture);
    raise_shield(&mut fixture, stack("minecraft:shield", 1), 5);

    let hit = fixture.command(&format!(
        "damage alice 4 minecraft:mob_attack at {} {} {}",
        alice.x,
        alice.y,
        alice.z - 2.0
    ));
    assert_eq!(health(&fixture), 16.0);
    assert_eq!(damage_events_to(&hit, BOB), 1);
    assert_eq!(fixture.slot(OFFHAND_SLOT).component_patch.damage, None);
    assert_eq!(fixture.item_used("minecraft:shield"), 1);
    assert_eq!(stat(&fixture, "minecraft:damage_blocked_by_shield"), 0);
}

/// `bypassedBy` is `#minecraft:bypasses_shield`, which holds
/// `#minecraft:bypasses_armor` and so `minecraft:generic`: no block, no wear
/// and no use counted.
#[test]
fn a_shield_is_bypassed_by_its_damage_type_tag() {
    let mut fixture = Fixture::new(GameType::Survival);
    face_to_face(&mut fixture);
    raise_shield(&mut fixture, stack("minecraft:shield", 1), 5);

    fixture.command("damage alice 4 minecraft:generic by bob");
    assert_eq!(health(&fixture), 16.0);
    assert_eq!(fixture.item_used("minecraft:shield"), 0);
}

/// `Player.blockUsingItem` (`Player.java:712-720`): an axe's `Weapon`
/// and a warden disable the shield for five seconds.
#[test]
fn axes_and_wardens_disable_a_raised_shield() {
    for warden in [false, true] {
        let mut fixture = Fixture::new(GameType::Survival);
        let alice = face_to_face(&mut fixture);
        let (source, damage_type) = if warden {
            fixture.command("gamemode spectator bob");
            let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
            let mut data = GeneralLivingData::new(kind).unwrap();
            data.set_no_ai(true);
            let (_, uuid) = fixture.world.spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                kind,
                Vec3d {
                    z: alice.z + 2.0,
                    ..alice
                },
                EntityData::GeneralLiving(data),
            );
            (uuid.to_string(), "minecraft:mob_attack")
        } else {
            fixture.world.stock_menu_slot_for_test(
                ConnectionId(BOB),
                HOTBAR_MENU_SLOT,
                stack("minecraft:iron_axe", 1),
            );
            ("bob".to_owned(), "minecraft:player_attack")
        };
        raise_shield(&mut fixture, stack("minecraft:shield", 1), 5);

        let hit = fixture.command(&format!("damage alice 4 {damage_type} by {source}"));
        assert_eq!(health(&fixture), 20.0);
        let cooldowns: Vec<_> = packets_to(&hit, ALICE)
            .filter_map(|packet| match packet {
                ClientboundPlayPacket::Cooldown(cooldown) => Some(cooldown.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(cooldowns.len(), 1, "{cooldowns:?}");
        assert_eq!(cooldowns[0].duration, 100);
        assert!(fixture
            .world
            .player_is_on_cooldown_for_test(ConnectionId(ALICE), &stack("minecraft:shield", 1)));
        assert!(!fixture.use_state().0);
        let heard = sounds(&hit, BOB);
        assert!(heard.contains(&"minecraft:item.shield.break"), "{heard:?}");
        assert!(heard.contains(&"minecraft:item.shield.block"), "{heard:?}");
        // `stopUsingItem` clears the use bit and leaves the hand bit.
        assert_eq!(living_flags(&hit, BOB, fixture.alice), [2]);
    }
}
/// A block that wears the shield out breaks it through
/// `onEquippedItemBroken` (`LivingEntity.java:3781-3784`), and `hurtServer`'s
/// item in use then has no `BLOCKS_ATTACKS` left: the damage event goes out
/// in place of the block sound.
#[test]
fn a_block_that_breaks_the_shield_broadcasts_the_damage_event() {
    let mut fixture = Fixture::new(GameType::Survival);
    face_to_face(&mut fixture);
    raise_shield(&mut fixture, damaged(stack("minecraft:shield", 1), 333), 5);

    let hit = fixture.command("damage alice 4 minecraft:mob_attack by bob");
    assert_eq!(health(&fixture), 20.0);
    assert!(is_empty_stack(&fixture.slot(OFFHAND_SLOT)));
    // `EntityEvent.OFFHAND_ITEM_BROKEN`.
    assert_eq!(entity_events(&hit, ALICE, fixture.alice), [48]);
    assert_eq!(entity_events(&hit, BOB, fixture.alice), [48]);
    assert_eq!(damage_events_to(&hit, BOB), 1);
    assert!(!sounds(&hit, BOB).contains(&"minecraft:item.shield.block"));
    assert_eq!(
        fixture.world.player_stat_for_test(
            ConnectionId(ALICE),
            StatKey::new(
                VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
                item_id("minecraft:shield")
            ),
        ),
        1
    );
    // The use goes on over the emptied hand until the client lets go.
    assert!(fixture.use_state().0);
    fixture.idle();
    assert!(fixture.use_state().0);
    let released = fixture.action(PlayerActionKind::ReleaseUseItem);
    assert!(!fixture.use_state().0);
    assert_eq!(living_flags(&released, BOB, fixture.alice), [2]);
}

fn is_empty_stack(item: &ItemStackSummary) -> bool {
    item.item_id.is_none() || item.count <= 0
}

/// A ravager's bite on a raised shield (`Ravager.blockedByItem`,
/// `Ravager.java:206-219`): the shield takes every bite, and on the stun
/// branch the ravager stands still for `STUN_DURATION` (`:57`) before it roars
/// (`:170-175`, `:156-159`).
#[test]
fn a_shield_stuns_a_ravager_whose_bite_it_blocks() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture.command("gamemode spectator bob");
    let alice = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    raise_shield(&mut fixture, stack("minecraft:shield", 1), 5);
    let (ravager, _) = fixture.world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
        Vec3d {
            z: alice.z + 2.0,
            ..alice
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_RAVAGER_ID).unwrap()),
    );
    let ravager_position = |fixture: &Fixture| {
        fixture
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, ravager)
            .unwrap()
            .position()
    };
    let mut stunned = false;
    for _ in 0..400 {
        let tick = fixture.idle();
        if entity_events(&tick, ALICE, ravager).contains(&39) {
            assert!(sounds(&tick, ALICE).contains(&"minecraft:entity.ravager.stunned"));
            stunned = true;
            break;
        }
    }
    assert!(
        stunned,
        "a blocked bite stuns the ravager on half its rolls"
    );
    assert_eq!(health(&fixture), 20.0, "the shield took every bite");
    let stunned_at = ravager_position(&fixture);
    let mut roar_sound = None;
    let mut roar_event = None;
    for index in 1..=60 {
        let tick = fixture.idle();
        if sounds(&tick, ALICE).contains(&"minecraft:entity.ravager.roar") {
            roar_sound = Some(index);
        }
        if entity_events(&tick, ALICE, ravager).contains(&69) {
            roar_event = Some(index);
        }
        assert!(
            !entity_events(&tick, ALICE, ravager).contains(&4),
            "a stunned or roaring ravager does not bite"
        );
    }
    assert_eq!(
        (roar_sound, roar_event),
        (Some(40), Some(50)),
        "the stun runs out into a roar that lands ten ticks later"
    );
    let moved = ravager_position(&fixture);
    assert!(
        (moved.x - stunned_at.x).abs() < 0.5 && (moved.z - stunned_at.z).abs() < 0.5,
        "the ravager stands still while stunned: {stunned_at:?} -> {moved:?}"
    );
}
