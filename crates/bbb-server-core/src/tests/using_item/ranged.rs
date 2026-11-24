//! The ranged weapons a session holds and lets go: `BowItem`, `CrossbowItem`,
//! `TridentItem` and the spear's piercing and kinetic weapon components.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::ItemEnchantmentSummary;

use super::*;
use crate::entity::abstract_arrow::ArrowPickup;
use crate::entity::{Entity, EntityData};

mod projectile_spawned;
mod weapon_sounds;

pub(super) const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// `stack` carrying `enchantments` by their holder ids in the loaded registry.
pub(super) fn enchanted(
    fixture: &Fixture,
    mut stack: ItemStackSummary,
    enchantments: &[(&str, i32)],
) -> ItemStackSummary {
    let registry: Vec<String> = fixture
        .world
        .datapacks()
        .expect("the fixture loads the vanilla datapack")
        .registries
        .registry("minecraft:enchantment")
        .expect("the enchantment registry is loaded")
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect();
    stack.component_patch.enchantments = enchantments
        .iter()
        .map(|(id, level)| ItemEnchantmentSummary {
            holder_id: registry.iter().position(|entry| entry == id).unwrap() as i32,
            level: *level,
        })
        .collect();
    stack
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    stack
}

pub(super) fn entities_of(fixture: &Fixture, entity_type_id: i32) -> Vec<Entity> {
    fixture
        .world
        .stored_entity_ids_for_test(OVERWORLD)
        .into_iter()
        .filter_map(|id| fixture.world.stored_entity_for_test(OVERWORLD, id))
        .filter(|entity| entity.entity_type_id() == entity_type_id)
        .cloned()
        .collect()
}

pub(super) fn damage(stack: &ItemStackSummary) -> i32 {
    crate::item_durability::damage_and_max(stack).map_or(0, |(damage, _)| damage)
}

fn speed(entity: &Entity) -> f64 {
    let movement = entity.delta_movement();
    (movement.x * movement.x + movement.y * movement.y + movement.z * movement.z).sqrt()
}

/// Holds the use for `ticks` ticks, then lets go.
fn draw_and_release(fixture: &mut Fixture, ticks: usize) -> TickResult {
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..ticks {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem)
}

/// Alice with Bob moved out of the line of fire.
pub(super) fn archer() -> Fixture {
    archer_with_config(datapack_config())
}

fn archer_with_config(config: ServerWorldConfig) -> Fixture {
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.set_player_position_for_test(
        ConnectionId(BOB),
        bbb_protocol::packets::Vec3d {
            x: 8.5,
            y: 64.0,
            z: -8.5,
        },
    );
    fixture.world.set_player_position_for_test(
        ConnectionId(ALICE),
        bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    fixture
}

/// `BowItem.releaseUsing`: a full draw looses one critical arrow at power
/// three, spends an arrow and a point of durability and counts the use.
#[test]
fn a_fully_drawn_bow_looses_a_critical_arrow() {
    let mut fixture = archer();
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:bow", 1));
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 5));

    let released = draw_and_release(&mut fixture, 20);

    let arrows = entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID);
    assert_eq!(arrows.len(), 1);
    let state = crate::entity::abstract_arrow::state(arrows[0].data()).unwrap();
    assert!(state.crit());
    assert_eq!(state.pickup(), ArrowPickup::Allowed);
    assert!(
        (2.85..3.05).contains(&speed(&arrows[0])),
        "{}",
        speed(&arrows[0])
    );
    assert_eq!(fixture.slot(HOTBAR_SLOT + 1).count, 4);
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 1);
    assert_eq!(fixture.item_used("minecraft:bow"), 1);
    assert!(sounds(&released, BOB).contains(&"minecraft:entity.arrow.shoot"));
}

/// A bow without arrows is not drawn, and a bow let go too early shoots
/// nothing.
#[test]
fn a_bow_needs_arrows_and_a_draw() {
    let mut fixture = archer();
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:bow", 1));
    fixture.use_item(InteractionHand::MainHand);
    assert!(!fixture.use_state().0);

    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 1));
    draw_and_release(&mut fixture, 1);
    assert!(entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID).is_empty());
    assert_eq!(fixture.slot(HOTBAR_SLOT + 1).count, 1);
}

/// Infinity's `ammo_use` keeps the arrow, and the shot it makes can only be
/// collected in creative.
#[test]
fn infinity_keeps_the_arrow() {
    let mut fixture = archer();
    let bow = enchanted(
        &fixture,
        stack("minecraft:bow", 1),
        &[("minecraft:infinity", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, bow);
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 1));

    draw_and_release(&mut fixture, 20);

    let arrows = entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID);
    assert_eq!(arrows.len(), 1);
    let state = crate::entity::abstract_arrow::state(arrows[0].data()).unwrap();
    assert_eq!(state.pickup(), ArrowPickup::CreativeOnly);
    assert_eq!(fixture.slot(HOTBAR_SLOT + 1).count, 1);
}

/// Power adds to the arrow's damage through the weapon the arrow carries, and
/// flame sets the arrow alight.
#[test]
fn power_and_flame_ride_on_the_arrow() {
    let mut fixture = archer();
    let bow = enchanted(
        &fixture,
        stack("minecraft:bow", 1),
        &[("minecraft:power", 5), ("minecraft:flame", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, bow);
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 1));
    let victim = target(&mut fixture, "minecraft:cow", VANILLA_ENTITY_TYPE_COW_ID);
    let uuid = fixture
        .world
        .stored_entity_for_test(OVERWORLD, victim)
        .unwrap()
        .uuid();
    fixture.command(&format!(
        "data merge entity {uuid} {{attributes:[{{id:\"minecraft:max_health\",base:20.0d}}],Health:20.0f}}"
    ));
    assert_eq!(
        fixture
            .world
            .stored_entity_for_test(OVERWORLD, victim)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        20.0
    );
    fixture.play(
        ALICE,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 10.0,
        }),
    );
    for _ in 0..20 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let arrows = entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID);
    assert_eq!(arrows.len(), 1);
    assert!(arrows.iter().all(|arrow| arrow.remaining_fire_ticks() > 0));
    for _ in 0..4 {
        fixture.idle();
    }

    let victim = fixture
        .world
        .stored_entity_for_test(OVERWORLD, victim)
        .unwrap();
    assert!(victim.remaining_fire_ticks() > 0);
    let health = victim.living().unwrap().health();
    assert!(health <= 5.0, "{health}");
}

fn charged(stack: &ItemStackSummary) -> Vec<ItemStackSummary> {
    crate::entity::projectile_weapon::charged_projectiles(
        &bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack.clone()),
    )
    .into_iter()
    .map(|stack| stack.summary().clone())
    .collect()
}

/// `CrossbowItem.onUseTick`: a held crossbow plays its loading sounds and,
/// a full charge (25 ticks) in, loads an arrow into `CHARGED_PROJECTILES`;
/// the next use shoots it at power 3.15 as a critical crossbow bolt.
#[test]
fn a_crossbow_charges_and_then_shoots() {
    let mut fixture = archer();
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:crossbow", 1));
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 5));

    fixture.use_item(InteractionHand::MainHand);
    let mut heard = Vec::new();
    for _ in 0..25 {
        let tick = fixture.idle();
        heard.extend(sounds(&tick, BOB));
    }
    assert_eq!(
        heard,
        [
            "minecraft:item.crossbow.loading_start",
            "minecraft:item.crossbow.loading_middle",
            "minecraft:item.crossbow.loading_end",
        ]
    );
    assert_eq!(charged(&fixture.slot(HOTBAR_SLOT)).len(), 1);
    assert_eq!(fixture.slot(HOTBAR_SLOT + 1).count, 4);
    fixture.action(PlayerActionKind::ReleaseUseItem);
    assert!(entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID).is_empty());

    let shot = fixture.use_item(InteractionHand::MainHand);
    assert_eq!(sounds(&shot, BOB), ["minecraft:item.crossbow.shoot"]);
    let arrows = entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID);
    assert_eq!(arrows.len(), 1);
    let state = crate::entity::abstract_arrow::state(arrows[0].data()).unwrap();
    assert!(state.crit());
    assert_eq!(
        state.sound_event_resource_id(),
        "minecraft:item.crossbow.hit"
    );
    assert!(
        (3.0..3.2).contains(&speed(&arrows[0])),
        "{}",
        speed(&arrows[0])
    );
    assert!(charged(&fixture.slot(HOTBAR_SLOT)).is_empty());
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 1);
    assert_eq!(fixture.item_used("minecraft:crossbow"), 1);
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "minecraft:adventure/ol_betsy"));
}

/// Multishot loads one arrow and two intangible copies and looses all three;
/// piercing carries onto each arrow, and quick charge shortens the charge.
#[test]
fn multishot_looses_three_piercing_arrows() {
    let mut fixture = archer();
    let crossbow = enchanted(
        &fixture,
        stack("minecraft:crossbow", 1),
        &[
            ("minecraft:multishot", 1),
            ("minecraft:piercing", 2),
            ("minecraft:quick_charge", 2),
        ],
    );
    fixture.hold(HOTBAR_MENU_SLOT, crossbow);
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 1));

    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..15 {
        fixture.idle();
    }
    assert_eq!(charged(&fixture.slot(HOTBAR_SLOT)).len(), 3);
    assert!(fixture.slot(HOTBAR_SLOT + 1).count == 0);
    fixture.action(PlayerActionKind::ReleaseUseItem);
    fixture.use_item(InteractionHand::MainHand);

    let arrows = entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID);
    assert_eq!(arrows.len(), 3);
    let states: Vec<_> = arrows
        .iter()
        .map(|arrow| crate::entity::abstract_arrow::state(arrow.data()).unwrap())
        .collect();
    assert!(states.iter().all(|state| state.pierce_level() == 2));
    assert_eq!(
        states
            .iter()
            .filter(|state| state.pickup() == ArrowPickup::CreativeOnly)
            .count(),
        2
    );
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 3);
}

/// Alice uses the item in her main hand looking `x_rot` degrees down.
fn use_looking(fixture: &mut Fixture, x_rot: f32) -> TickResult {
    fixture.play(
        ALICE,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot,
        }),
    )
}

/// `TridentItem.releaseUsing`: held ten ticks, the trident leaves the hand
/// as a `ThrownTrident` at power 2.5 that the thrower may pick up, worn by a
/// point of durability; held less, nothing happens.
#[test]
fn a_held_trident_is_thrown() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID;

    let mut fixture = archer();
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:trident", 1));

    draw_and_release(&mut fixture, 5);
    assert!(entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID).is_empty());
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 1);

    let thrown = draw_and_release(&mut fixture, 10);
    let tridents = entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID);
    assert_eq!(tridents.len(), 1);
    let EntityData::Trident(data) = tridents[0].data() else {
        unreachable!()
    };
    assert_eq!(data.pickup(), ArrowPickup::Allowed);
    assert!(!data.foil());
    assert_eq!(damage(data.item().summary()), 1);
    assert!(
        (2.3..2.7).contains(&speed(&tridents[0])),
        "{}",
        speed(&tridents[0])
    );
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 0);
    assert_eq!(fixture.item_used("minecraft:trident"), 1);
    assert!(
        entity_sounds(&thrown, BOB).contains(&("minecraft:item.trident.throw", tridents[0].id()))
    );

    for (has_enchantment, glint_override, removed_override, expected_foil) in [
        (true, None, false, true),
        (true, Some(false), false, false),
        (false, Some(true), false, true),
        (true, None, true, true),
    ] {
        let mut fixture = archer();
        let mut trident = stack("minecraft:trident", 1);
        if has_enchantment {
            trident = enchanted(&fixture, trident, &[("minecraft:impaling", 1)]);
        }
        if let Some(value) = glint_override {
            trident.component_patch.enchantment_glint_override = Some(value);
            trident
                .component_patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID);
        }
        if removed_override {
            trident
                .component_patch
                .removed_type_ids
                .push(VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID);
        }
        fixture.hold(HOTBAR_MENU_SLOT, trident);

        draw_and_release(&mut fixture, 10);

        let tridents = entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID);
        assert_eq!(tridents.len(), 1);
        let EntityData::Trident(data) = tridents[0].data() else {
            unreachable!()
        };
        assert_eq!(data.foil(), expected_foil);
    }
}

/// Loyalty: a trident that landed turns back to its thrower, without
/// physics, and ends up in the thrower's inventory again.
#[test]
fn a_loyal_trident_returns_to_its_thrower() {
    let mut fixture = archer();
    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:loyalty", 3)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, trident);

    use_looking(&mut fixture, 90.0);
    for _ in 0..10 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let tridents = entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID);
    assert_eq!(tridents.len(), 1);
    let EntityData::Trident(data) = tridents[0].data() else {
        unreachable!()
    };
    assert_eq!(data.loyalty(), 3);

    let mut heard = Vec::new();
    for _ in 0..40 {
        let tick = fixture.idle();
        heard.extend(sounds(&tick, BOB));
        if entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID).is_empty() {
            break;
        }
    }
    assert!(
        heard.contains(&"minecraft:item.trident.return"),
        "{heard:?}"
    );
    assert!(entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID).is_empty());
    assert_eq!(
        fixture.slot(HOTBAR_SLOT).item_id,
        stack("minecraft:trident", 1).item_id
    );
}

/// Riptide: out of water the trident cannot even be drawn; in water the
/// release keeps the trident, launches the thrower into a spin and the spin
/// strikes the first living thing it touches for the trident's eight.
#[test]
fn riptide_spins_the_thrower_into_what_it_touches() {
    let mut fixture = archer();
    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:riptide", 3)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, trident);
    fixture.use_item(InteractionHand::MainHand);
    assert!(!fixture.use_state().0);

    fixture.command("setblock 0 64 0 minecraft:water");
    fixture.command("summon minecraft:zombie 0.5 64 0.5");
    let zombie = entities_of(&fixture, VANILLA_ENTITY_TYPE_ZOMBIE_ID)[0].id();
    // A full attack charge, which the strike scales by.
    for _ in 0..10 {
        fixture.idle();
    }
    let released = draw_and_release(&mut fixture, 10);

    // One point for the launch and one for the strike.
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 1);
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 2);
    assert!(entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID).is_empty());
    assert!(entity_sounds(&released, BOB)
        .contains(&("minecraft:item.trident.riptide_3", fixture.alice)));
    let health = fixture
        .world
        .stored_entity_for_test(OVERWORLD, zombie)
        .and_then(|zombie| zombie.living().map(|living| living.health()))
        .unwrap_or(0.0);
    assert!(health < 13.0, "{health}");
}

/// `checkAutoSpinAttack(beforeTravelBox, getBoundingBox())`
/// (`LivingEntity.java:3092-3094`): the spin strikes what it passed through
/// during the move, not only what the final box touches.
#[test]
fn riptide_spin_strikes_what_the_move_swept_through() {
    let mut fixture = archer();
    fixture.command("summon minecraft:zombie 0.5 64 2.5");
    let zombie = entities_of(&fixture, VANILLA_ENTITY_TYPE_ZOMBIE_ID)[0].id();
    let Some(bbb_protocol::NbtSummaryValue::Compound(mut entries)) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, zombie)
    else {
        panic!("a mob answers its record")
    };
    entries.retain(|entry| entry.name != "NoAI");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "NoAI".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    fixture
        .world
        .load_stored_entity_record_for_test(OVERWORLD, zombie, &entries);
    fixture
        .world
        .start_player_auto_spin_attack_for_test(ConnectionId(ALICE), 20);
    fixture.play(
        ALICE,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 0.5,
                y: 64.0,
                z: 4.5,
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    fixture.idle();
    let health = fixture
        .world
        .stored_entity_for_test(OVERWORLD, zombie)
        .and_then(|zombie| zombie.living().map(|living| living.health()))
        .unwrap_or(0.0);
    assert!(health < 20.0, "{health}");
}

/// The spin publishes the living-entity spin flag and the spin pose until its
/// twenty ticks run out.
#[test]
fn riptide_spin_flag_and_pose_last_twenty_ticks() {
    let mut fixture = archer();
    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:riptide", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, trident);
    fixture.command("setblock 0 64 0 minecraft:water");
    fixture.idle();

    let released = draw_and_release(&mut fixture, 10);
    assert!(living_flags(&released, BOB, fixture.alice)
        .iter()
        .any(|flags| flags & 4 != 0));
    fixture.idle();
    assert_eq!(
        fixture.world.player_pose_id_for_test(ConnectionId(ALICE)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
    );
    let mut flags = Vec::new();
    for _ in 0..20 {
        let tick = fixture.idle();
        flags.extend(living_flags(&tick, BOB, fixture.alice));
    }
    assert_eq!(flags.last().map(|flags| flags & 4), Some(0));
    assert_ne!(
        fixture.world.player_pose_id_for_test(ConnectionId(ALICE)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
    );
}

/// A still mob summoned four blocks in front of Alice, in her line of fire.
fn target(fixture: &mut Fixture, resource_id: &str, entity_type_id: i32) -> i32 {
    fixture.command(&format!("summon {resource_id} 0.5 64 4.5"));
    let id = entities_of(fixture, entity_type_id)[0].id();
    let Some(bbb_protocol::NbtSummaryValue::Compound(mut entries)) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, id)
    else {
        panic!("a mob answers its record")
    };
    entries.retain(|entry| entry.name != "NoAI");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "NoAI".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    fixture
        .world
        .load_stored_entity_record_for_test(OVERWORLD, id, &entries);
    id
}

fn charge_and_shoot(fixture: &mut Fixture) {
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:crossbow", 1));
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 5));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..25 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..5 {
        fixture.idle();
    }
}

#[test]
fn closed_shulker_refuses_arrows_and_keeps_thrown_tridents() {
    use crate::entity::item::plain_stack;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    };

    for peek in [0, 100, -128] {
        for (resource_id, entity_type_id) in [
            ("minecraft:arrow", VANILLA_ENTITY_TYPE_ARROW_ID),
            (
                "minecraft:spectral_arrow",
                VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            ),
            ("minecraft:trident", VANILLA_ENTITY_TYPE_TRIDENT_ID),
        ] {
            let mut fixture = archer();
            let shulker = target(
                &mut fixture,
                "minecraft:shulker",
                VANILLA_ENTITY_TYPE_SHULKER_ID,
            );
            let Some(bbb_protocol::NbtSummaryValue::Compound(mut entries)) = fixture
                .world
                .stored_entity_command_data_for_test(OVERWORLD, shulker)
            else {
                panic!("a shulker has a saved record")
            };
            entries.retain(|entry| entry.name != "Peek");
            entries.push(bbb_protocol::NbtSummaryEntry {
                name: "Peek".into(),
                value: bbb_protocol::NbtSummaryValue::Byte(peek),
            });
            fixture
                .world
                .load_stored_entity_record_for_test(OVERWORLD, shulker, &entries);
            let health = |fixture: &Fixture| {
                fixture
                    .world
                    .stored_entity_for_test(OVERWORLD, shulker)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health()
            };
            let before = health(&fixture);
            let item = plain_stack(resource_id, 1).unwrap();
            let data = match entity_type_id {
                VANILLA_ENTITY_TYPE_ARROW_ID => {
                    EntityData::Arrow(crate::entity::ArrowData::new(item))
                }
                VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
                    EntityData::SpectralArrow(crate::entity::SpectralArrowData::new(item))
                }
                VANILLA_ENTITY_TYPE_TRIDENT_ID => {
                    EntityData::Trident(crate::entity::TridentData::new(item))
                }
                _ => unreachable!(),
            };
            let projectile = fixture
                .world
                .spawn_stored_entity_for_test(
                    OVERWORLD,
                    entity_type_id,
                    Vec3d {
                        x: 0.5,
                        y: 64.5,
                        z: 2.5,
                    },
                    data,
                )
                .0;
            fixture
                .world
                .stored_entity_mut_for_test(OVERWORLD, projectile)
                .unwrap()
                .set_delta_movement(Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 2.0,
                });

            fixture.idle();

            if peek == 0 {
                assert_eq!(
                    health(&fixture),
                    before,
                    "{resource_id} against a closed shell"
                );
                let arrow = fixture
                    .world
                    .stored_entity_for_test(OVERWORLD, projectile)
                    .expect("the shell keeps the projectile alive");
                if entity_type_id != VANILLA_ENTITY_TYPE_TRIDENT_ID {
                    assert!(
                        arrow.delta_movement().z < 0.0,
                        "the arrow bounces off the shell"
                    );
                } else {
                    for _ in 0..25 {
                        fixture.idle();
                    }
                    assert_eq!(health(&fixture), before);
                    assert!(fixture
                        .world
                        .stored_entity_for_test(OVERWORLD, projectile)
                        .is_some());
                }
            } else {
                assert!(
                    health(&fixture) < before,
                    "{resource_id} must hurt with raw Peek={peek}"
                );
            }
        }
    }
}

/// `KilledByArrowTrigger`: a crossbow bolt that kills a pillager is "Who's
/// the Pillager Now?".
#[test]
fn a_crossbow_kill_fires_killed_by_arrow() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    let mut config = datapack_config();
    let mut startup = (*config.datapacks.take().unwrap()).clone();
    for (id, player) in [
        (
            "test:arrow_player",
            serde_json::json!({"type":"minecraft:player"}),
        ),
        (
            "test:arrow_wrong_player",
            serde_json::json!([{
                "condition":"minecraft:entity_properties", "entity":"this",
                "predicate":{"type":"minecraft:zombie"}
            }]),
        ),
    ] {
        startup.advancements.advancements.push(AdvancementElement {
            id: id.to_owned(),
            source_pack: "test".to_owned(),
            advancement: parse_advancement(&serde_json::json!({
                "criteria":{"kill":{"trigger":"minecraft:killed_by_arrow",
                    "conditions":{"player":player,"victims":[[{"condition":"minecraft:entity_properties",
                        "entity":"this","predicate":{"type":"minecraft:pillager"}}]]}}}
            }))
            .unwrap(),
        });
    }
    config.datapacks = Some(std::sync::Arc::new(startup));
    let mut fixture = archer_with_config(config);
    let pillager = target(
        &mut fixture,
        "minecraft:pillager",
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID,
    );
    crate::entity::living::set_health(
        fixture
            .world
            .stored_entity_mut_for_test(OVERWORLD, pillager)
            .unwrap(),
        1.0,
    );

    charge_and_shoot(&mut fixture);

    assert!(fixture
        .world
        .stored_entity_for_test(OVERWORLD, pillager)
        .is_none_or(|pillager| !pillager.is_alive_with_health()));
    assert!(fixture.world.player_advancement_is_done(
        ConnectionId(ALICE),
        "minecraft:adventure/whos_the_pillager_now"
    ));
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:arrow_player"));
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:arrow_wrong_player"));
}

/// The victim's `post_attack` enchantments run after an arrow hit: thorns on
/// the target's chestplate hurts the archer.
#[test]
fn an_arrow_into_thorns_hurts_the_archer() {
    let mut fixture = archer();
    let zombie = target(
        &mut fixture,
        "minecraft:zombie",
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    );
    let chestplate = enchanted(
        &fixture,
        stack("minecraft:diamond_chestplate", 1),
        &[("minecraft:thorns", 10)],
    );
    fixture
        .world
        .stored_entity_mut_for_test(OVERWORLD, zombie)
        .and_then(|zombie| zombie.living_mut())
        .unwrap()
        .set_item_slot(
            bbb_protocol::equipment_slots::EquipmentSlot::Chest,
            Some(bbb_server_types::item_stack::RuntimeItemStack::from_summary(chestplate)),
        );
    let before = fixture.world.player_health_for_test(ConnectionId(ALICE));

    charge_and_shoot(&mut fixture);

    let zombie_health = fixture
        .world
        .stored_entity_for_test(OVERWORLD, zombie)
        .and_then(|zombie| zombie.living().map(|living| living.health()))
        .unwrap();
    assert!(zombie_health < 20.0, "{zombie_health}");
    assert!(fixture.world.player_health_for_test(ConnectionId(ALICE)) < before);
}

/// Channeling: in a thunderstorm a trident that hits a villager under open
/// sky calls a bolt down on it, crediting the thrower with "Very Very
/// Frightening".
#[test]
fn a_channeling_trident_calls_lightning_onto_its_victim() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    let entity = |predicate| {
        serde_json::json!({
            "condition":"minecraft:entity_properties", "entity":"this", "predicate":predicate
        })
    };
    let source = entity(serde_json::json!({
        "type":"minecraft:pig", "nbt":"{Tags:[\"original\"],data:{marked:23b}}",
        "components":{"minecraft:custom_data":{"marked":23}}
    }));
    let mut config = datapack_config();
    let mut startup = (*config.datapacks.take().unwrap()).clone();
    for (id, victim) in [
        (
            "charged",
            serde_json::json!([entity(serde_json::json!({
                "type":"minecraft:creeper", "nbt":"{powered:1b}", "team":"storm",
                "components":{"minecraft:custom_data":{"marked":17}}
            }))]),
        ),
        (
            "variant",
            serde_json::json!([entity(serde_json::json!({
                "type":"minecraft:mooshroom", "nbt":"{Type:\"brown\"}",
                "components":{"minecraft:mooshroom/variant":"brown"}
            }))]),
        ),
        (
            "source",
            serde_json::json!([source.clone(), {
                "condition":"minecraft:inverted", "term":entity(serde_json::json!({"team":"storm"}))
            }]),
        ),
        (
            "wrong_source_team",
            serde_json::json!([source, entity(serde_json::json!({"team":"storm"}))]),
        ),
    ] {
        startup.advancements.advancements.push(AdvancementElement {
            id: format!("test:storm_{id}"),
            source_pack: "test".into(),
            advancement: parse_advancement(&serde_json::json!({"criteria":{"strike":{
                "trigger":"minecraft:channeled_lightning", "conditions":{"victims":[victim]}
            }}}))
            .unwrap(),
        });
    }
    config.datapacks = Some(std::sync::Arc::new(startup));
    let mut fixture = archer_with_config(config);
    fixture.command("weather thunder 40000t");
    for _ in 0..200 {
        if fixture.world.level_weather(OVERWORLD).is_thundering() {
            break;
        }
        fixture.idle();
    }
    assert!(fixture.world.level_weather(OVERWORLD).is_thundering());
    target(
        &mut fixture,
        "minecraft:villager",
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID,
    );
    fixture.command("summon creeper 2.5 64 4.5 {NoAI:1b,data:{marked:17b}}");
    fixture.command("summon mooshroom -1.5 64 5.5 {NoAI:1b,Type:\"red\"}");
    fixture.command("summon pig 2.5 64 6.5 {NoAI:1b,Tags:[\"original\"],data:{marked:23b}}");
    let creeper = entities_of(
        &fixture,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID,
    )[0]
    .uuid();
    fixture.command("team add storm");
    fixture.command(&format!("team join storm {creeper}"));
    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:channeling", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, trident);

    let mut bolts = false;
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..10 {
        fixture.idle();
    }
    let mut ticks = vec![fixture.action(PlayerActionKind::ReleaseUseItem)];
    for _ in 0..5 {
        ticks.push(fixture.idle());
    }
    for tick in &ticks {
        bolts |= tick.events.iter().any(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::AddEntity(add))
                    if add.entity_type_id
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID
            )
        });
    }
    assert!(bolts);
    assert!(fixture.world.player_advancement_is_done(
        ConnectionId(ALICE),
        "minecraft:adventure/very_very_frightening"
    ));
    for id in ["charged", "variant", "source"] {
        assert!(
            fixture
                .world
                .player_advancement_is_done(ConnectionId(ALICE), &format!("test:storm_{id}")),
            "{id}"
        );
    }
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:storm_wrong_source_team"));
    assert!(entities_of(
        &fixture,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID
    )
    .is_empty());
    assert_eq!(
        entities_of(
            &fixture,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
        )
        .len(),
        1
    );
}
