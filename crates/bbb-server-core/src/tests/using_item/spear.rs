//! The spear: the jab `PiercingWeapon.attack` answers a `STAB` action with,
//! the lunge enchantment that rides on it, and the held kinetic charge.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::Vec3d;

use super::ranged::{archer, damage, enchanted, entities_of, OVERWORLD};
use super::*;

/// Alice with a spear held long enough for a full attack charge.
fn spearman(spear: impl FnOnce(&Fixture) -> ItemStackSummary) -> Fixture {
    let mut fixture = archer();
    let spear = spear(&fixture);
    fixture.hold(HOTBAR_MENU_SLOT, spear);
    for _ in 0..25 {
        fixture.idle();
    }
    fixture
}

fn zombie_at(fixture: &mut Fixture, z: f64) -> i32 {
    fixture.command(&format!("summon minecraft:zombie 0.5 64 {z}"));
    entities_of(fixture, VANILLA_ENTITY_TYPE_ZOMBIE_ID)[0].id()
}

fn stationary_zombie(fixture: &mut Fixture) -> i32 {
    fixture.command("time set midnight");
    let id = zombie_at(fixture, 3.5);
    let uuid = fixture
        .world
        .stored_entity_for_test(OVERWORLD, id)
        .unwrap()
        .uuid();
    fixture.command(&format!(
        "data merge entity {uuid} {{NoAI:1b,NoGravity:1b}}"
    ));
    id
}

fn health(fixture: &Fixture, entity_id: i32) -> f32 {
    fixture
        .world
        .stored_entity_for_test(OVERWORLD, entity_id)
        .and_then(|entity| entity.living().map(|living| living.health()))
        .unwrap_or(0.0)
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

/// `PiercingWeapon.attack`: a jab strikes the zombie between the spear's
/// minimum and maximum reach, plays the attack sound to the others and the
/// hit sound to everyone, and wears the spear.
#[test]
fn a_spear_jab_strikes_along_its_reach() {
    let mut fixture = spearman(|_| stack("minecraft:iron_spear", 1));
    let zombie = zombie_at(&mut fixture, 3.5);

    let jab = fixture.action(PlayerActionKind::Stab);

    assert!(
        health(&fixture, zombie) < 20.0,
        "{}",
        health(&fixture, zombie)
    );
    let heard = sounds(&jab, BOB);
    assert!(heard.contains(&"minecraft:item.spear.attack"), "{heard:?}");
    assert!(heard.contains(&"minecraft:item.spear.hit"), "{heard:?}");
    assert!(!sounds(&jab, ALICE).contains(&"minecraft:item.spear.attack"));
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 1);
}

/// A zombie nearer than the spear's two-block minimum reach is not struck.
#[test]
fn a_spear_jab_misses_inside_its_minimum_reach() {
    let mut fixture = spearman(|_| stack("minecraft:iron_spear", 1));
    let zombie = zombie_at(&mut fixture, 1.5);

    let jab = fixture.action(PlayerActionKind::Stab);

    assert_eq!(health(&fixture, zombie), 20.0);
    assert!(!sounds(&jab, BOB).contains(&"minecraft:item.spear.hit"));
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 0);
}

/// The stack's own `attack_range` replaces the spear's two-block minimum
/// reach (`LivingEntity.getAttackRangeWith`).
#[test]
fn a_given_attack_range_reaches_inside_the_spears_minimum() {
    let mut fixture = archer();
    fixture.command("give alice iron_spear[attack_range={min_reach:0.0f,max_reach:4.5f}]");
    for _ in 0..25 {
        fixture.idle();
    }
    let zombie = zombie_at(&mut fixture, 1.5);

    fixture.action(PlayerActionKind::Stab);

    assert!(health(&fixture, zombie) < 20.0);
}

#[test]
fn a_given_piercing_weapon_uses_its_knockback_flag_and_sounds() {
    let mut pushes = Vec::new();
    for knockback in [false, true] {
        let mut fixture = archer();
        fixture.command(&format!(
            "give alice iron_spear[piercing_weapon={{deals_knockback:{knockback},sound:\"minecraft:block.bell.use\",hit_sound:\"minecraft:block.note_block.pling\"}}]"
        ));
        for _ in 0..25 {
            fixture.idle();
        }
        let zombie = stationary_zombie(&mut fixture);
        let jab = fixture.action(PlayerActionKind::Stab);
        assert!(health(&fixture, zombie) < 20.0);
        pushes.push(
            fixture
                .world
                .stored_entity_for_test(OVERWORLD, zombie)
                .unwrap()
                .delta_movement()
                .z,
        );
        let heard = sounds(&jab, BOB);
        assert!(heard.contains(&"minecraft:block.bell.use"), "{heard:?}");
        assert!(
            heard.contains(&"minecraft:block.note_block.pling"),
            "{heard:?}"
        );
        assert!(!heard.contains(&"minecraft:item.spear.attack"));
        assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 1);
    }
    assert!(pushes[1] > pushes[0], "{pushes:?}");
}

#[test]
fn a_given_kinetic_weapon_uses_its_delay_speed_conditions_and_sound() {
    for min_speed in [0, 1_000] {
        let mut fixture = archer();
        fixture.command(&format!(
            "give alice iron_spear[kinetic_weapon={{delay_ticks:10,damage_conditions:{{max_duration_ticks:40,min_speed:{min_speed}}},damage_multiplier:0,sound:\"minecraft:block.bell.use\"}}]"
        ));
        let zombie = stationary_zombie(&mut fixture);
        let started = fixture.use_item(InteractionHand::MainHand);
        assert!(fixture.use_state().0);
        assert!(sounds(&started, BOB).contains(&"minecraft:block.bell.use"));
        for _ in 0..3 {
            fixture.idle();
        }
        assert_eq!(health(&fixture, zombie), 20.0);
        for _ in 0..20 {
            fixture.idle();
        }
        assert_eq!(health(&fixture, zombie) < 20.0, min_speed == 0);
    }
}

#[test]
fn a_given_spear_keeps_inline_sound_ids_and_fixed_ranges() {
    let mut fixture = archer();
    fixture.command(r#"give alice iron_spear[piercing_weapon={sound:{sound_id:"test:jab",range:32.0f},hit_sound:{sound_id:"test:hit",range:32.0f}}]"#);
    for _ in 0..25 {
        fixture.idle();
    }
    let zombie = stationary_zombie(&mut fixture);
    let jab = fixture.action(PlayerActionKind::Stab);
    assert!(health(&fixture, zombie) < 20.0);
    for location in ["test:jab", "test:hit"] {
        assert!(packets_to(&jab, BOB).any(|packet| matches!(
            packet,
            ClientboundPlayPacket::Sound(bbb_protocol::packets::SoundEvent {
                sound: bbb_protocol::packets::SoundEventHolder::Direct { location: id, fixed_range: Some(range) }, ..
            }) if id == location && *range == 32.0
        )), "{location}");
    }
}

/// Lunge's `post_piercing_attack`: the jab throws the jabber forward along
/// its look, costs four exhaustion, plays the lunge sound and wears the
/// spear one more point.
#[test]
fn lunge_throws_the_jabber_forward() {
    let mut fixture = spearman(|fixture| {
        enchanted(
            fixture,
            stack("minecraft:iron_spear", 1),
            &[("minecraft:lunge", 1)],
        )
    });
    let zombie = zombie_at(&mut fixture, 3.5);

    let jab = fixture.action(PlayerActionKind::Stab);

    let pushed = motions(&jab, ALICE, fixture.alice);
    assert_eq!(pushed.len(), 1, "{pushed:?}");
    assert!((0.4..0.5).contains(&pushed[0].z), "{pushed:?}");
    assert!(sounds(&jab, BOB).contains(&"minecraft:item.spear.lunge_1"));
    assert!(health(&fixture, zombie) < 20.0);
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 2);
    // Four for the lunge and a tenth for the strike pass the exhaustion
    // threshold, which spends a point of saturation.
    fixture.idle();
    let (_, saturation) = fixture
        .world
        .player_food_levels_for_test(ConnectionId(ALICE));
    assert!(saturation < 5.0, "{saturation}");
}

/// `KineticWeapon.damageEntities`: past its delay, a held spear carried
/// forward fast enough strikes what it reaches, and the strike is
/// announced with entity event 2.
#[test]
fn a_charging_spear_strikes_what_it_reaches() {
    let mut fixture = spearman(|_| stack("minecraft:iron_spear", 1));
    let zombie = zombie_at(&mut fixture, 9.5);
    fixture.use_item(InteractionHand::MainHand);
    assert!(fixture.use_state().0);

    let mut events = Vec::new();
    for step in 1..=16 {
        let tick = fixture.play(
            ALICE,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5 + 0.4 * f64::from(step),
                }),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        );
        events.extend(entity_events(&tick, BOB, fixture.alice));
        if health(&fixture, zombie) < 20.0 {
            break;
        }
    }

    assert!(
        health(&fixture, zombie) < 20.0,
        "{}",
        health(&fixture, zombie)
    );
    assert_eq!(events, [2]);
}
