//! Ranged and spellcasting goals on live ticks against a survival session:
//! what the shot hits, what it carries and what the player sees spawn.

use super::melee::{fixture, health, hits, ALICE, PLAYER_POSITION};
use super::*;
use crate::entity::hurting_projectile::fireball::LargeFireballData;
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::item::plain_stack;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::*;

mod drowned_trident;
mod entity_data;

fn east(distance: f64) -> Vec3d {
    Vec3d {
        x: PLAYER_POSITION.x + distance,
        ..PLAYER_POSITION
    }
}

fn give(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    uuid: Uuid,
    slot: &str,
    item: &str,
) {
    let tick = console_command(
        sender,
        receiver,
        world,
        20,
        &format!("item replace entity {uuid} {slot} with {item}"),
    );
    assert_eq!(
        console_command_answer(&tick).0,
        Some(1),
        "{item} into {slot}"
    );
}

/// Ticks up to `limit` times and answers the `data` of the first entity of
/// `entity_type_id` added to Alice's view.
fn first_added(world: &mut ServerWorld, limit: usize, entity_type_id: i32) -> Option<i32> {
    for _ in 0..limit {
        let tick = world.tick(Vec::new());
        let added = tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.entity_type_id == entity_type_id => Some(add.data),
            _ => None,
        });
        if added.is_some() {
            return added;
        }
    }
    None
}

/// `RangedBowAttackGoal` draws the bow for twenty ticks and looses an arrow
/// the skeleton owns (`RangedBowAttackGoal.java:123-131`); the arrow hurts
/// the player and stays stuck in her (`AbstractArrow.java:468-470`).
#[test]
fn a_skeleton_with_an_offhand_bow_shoots_the_player() {
    let (sender, receiver, mut world) = fixture();
    let (skeleton, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SKELETON_ID, east(10.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.offhand",
        "minecraft:bow",
    );
    tick_until(&mut world, 100, |world| {
        world
            .stored_entity_for_test(DIMENSION, skeleton)
            .unwrap()
            .living()
            .unwrap()
            .living_entity_flags()
            & 3
            == 3
    });
    assert_eq!(
        first_added(&mut world, 200, VANILLA_ENTITY_TYPE_ARROW_ID),
        Some(skeleton),
        "the arrow names the skeleton as its owner"
    );
    let hits = hits(&mut world, 200);
    assert!(!hits.is_empty(), "an arrow lands");
    assert!(world.player_arrow_count_for_test(ALICE) >= 1);
}

/// `AbstractSkeleton.reassessWeaponGoal` (`AbstractSkeleton.java:132-149`):
/// an empty-handed skeleton walks up and strikes for its `ATTACK_DAMAGE` 2,
/// and once it holds a bow it shoots instead.
#[test]
fn a_skeleton_strikes_without_a_bow_and_shoots_once_it_holds_one() {
    let (sender, receiver, mut world) = fixture();
    let (skeleton, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SKELETON_ID, east(4.0));
    let hits = hits(&mut world, 200);
    assert!(
        hits.first().is_some_and(|(_, lost, _)| *lost == 2.0),
        "a melee strike: {hits:?}"
    );
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:bow",
    );
    assert_eq!(
        first_added(&mut world, 200, VANILLA_ENTITY_TYPE_ARROW_ID),
        Some(skeleton)
    );
}

/// `Stray.getArrow` tips its arrow with slowness (`Stray.java:60-67`).
#[test]
fn a_stray_arrow_slows_the_player() {
    let (sender, receiver, mut world) = fixture();
    let (_, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_STRAY_ID, east(8.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:bow",
    );
    tick_until(&mut world, 400, |world| {
        world
            .player_active_effect_for_test(ALICE, "minecraft:slowness")
            .is_some()
    });
}

/// `RangedCrossbowAttackGoal` charges for 25 ticks, waits, then fires
/// (`RangedCrossbowAttackGoal.java:99-126`).
#[test]
fn a_pillager_charges_its_crossbow_and_shoots() {
    let (sender, receiver, mut world) = fixture();
    let (pillager, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_PILLAGER_ID, east(6.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:crossbow",
    );
    assert_eq!(
        first_added(&mut world, 300, VANILLA_ENTITY_TYPE_ARROW_ID),
        Some(pillager)
    );
    let hits = hits(&mut world, 100);
    assert!(!hits.is_empty(), "the bolt lands");
}

/// `ProjectileWeaponItem.shoot` wears the mob's crossbow one point per bolt
/// (`ProjectileWeaponItem.java:71`): a last point breaks it.
#[test]
fn a_pillager_shot_wears_out_its_crossbow() {
    let (sender, receiver, mut world) = fixture();
    let (pillager, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_PILLAGER_ID, east(6.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:crossbow[damage=464]",
    );
    assert_eq!(
        first_added(&mut world, 300, VANILLA_ENTITY_TYPE_ARROW_ID),
        Some(pillager)
    );
    let held = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, pillager)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        .is_some_and(|stack| !stack.is_empty());
    assert!(!held, "the last point broke the crossbow");
}

/// `CrossbowItem.onUseTick` (`CrossbowItem.java:202-238`) plays the
/// enchantment's charging sounds for a mob too, and Quick Charge III shortens
/// the 25-tick charge to 10 (`RangedCrossbowAttackGoal.java:111-113`).
#[test]
fn a_pillager_quick_charges_with_the_enchantments_loading_sounds() {
    let (sender, receiver, mut world) = fixture();
    let (_, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_PILLAGER_ID, east(6.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:crossbow[enchantments={\"minecraft:quick_charge\":3}]",
    );
    let mut heard = Vec::new();
    for tick_index in 0..300 {
        let tick = world.tick(Vec::new());
        for sound in crate::tests::using_item::sounds(&tick, ALICE.0) {
            if sound.starts_with("minecraft:item.crossbow.") {
                heard.push((tick_index, sound));
            }
        }
        if heard
            .iter()
            .any(|(_, sound)| *sound == "minecraft:item.crossbow.loading_end")
        {
            break;
        }
    }
    let names = heard.iter().map(|(_, sound)| *sound).collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "minecraft:item.crossbow.quick_charge_3",
            "minecraft:item.crossbow.loading_end"
        ]
    );
    assert_eq!(
        heard[1].0 - heard[0].0,
        8,
        "start at 20% of 10 ticks, load at 10"
    );
}

/// `GhastShootFireballGoal` charges for twenty ticks and launches a large
/// fireball it owns (`Ghast.java:356-389`).
#[test]
fn a_ghast_shoots_a_fireball_at_the_player() {
    let (_sender, _receiver, mut world) = fixture();
    let (ghast, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_GHAST_ID,
        Vec3d {
            z: 1.5,
            ..PLAYER_POSITION
        },
    );
    assert_eq!(
        first_added(&mut world, 300, VANILLA_ENTITY_TYPE_FIREBALL_ID),
        Some(ghast)
    );
}

/// `Ghast.hurtServer` (`Ghast.java:101-108`): a fireball a player owns kills
/// the fire-immune ghast outright, one nobody owns does nothing to it.
#[test]
fn a_fireball_sent_back_by_a_player_kills_a_ghast() {
    let (_sender, _receiver, mut world) = fixture();
    let alice = Uuid::from_u128(1);
    let ghast_position = Vec3d {
        x: 4.5,
        y: 80.0,
        z: 4.5,
    };
    let (ghast, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_GHAST_ID, ghast_position);
    let launch = |world: &mut ServerWorld, owner: Option<Uuid>| {
        let ghast_position = world
            .stored_entity_for_test(DIMENSION, ghast)
            .unwrap()
            .position();
        let (fireball, _) = world.spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
            Vec3d {
                x: ghast_position.x - 6.0,
                y: ghast_position.y + 1.5,
                ..ghast_position
            },
            EntityData::LargeFireball(LargeFireballData::new(
                HurtingProjectileState::new(owner, 0.1),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
                0,
            )),
        );
        world
            .stored_entity_mut_for_test(DIMENSION, fireball)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.0,
            });
        for _ in 0..20 {
            world.tick(Vec::new());
        }
        assert!(
            world.stored_entity_for_test(DIMENSION, fireball).is_none(),
            "the fireball hit the ghast"
        );
    };
    launch(&mut world, None);
    assert!(
        world
            .stored_entity_for_test(DIMENSION, ghast)
            .is_some_and(|entity| entity.is_alive()),
        "an unowned fireball does not hurt a ghast"
    );
    launch(&mut world, Some(alice));
    assert!(
        world
            .stored_entity_for_test(DIMENSION, ghast)
            .is_none_or(|entity| !entity.is_alive()
                || entity.living().is_some_and(|living| living.health() <= 0.0)),
        "the reflected fireball kills the ghast"
    );
}

/// `ShulkerAttackGoal` sends a bullet (`Shulker.java:616-620`) whose hit
/// levitates the player (`ShulkerBullet.java:284-291`).
#[test]
fn a_shulker_bullet_levitates_the_player() {
    use crate::level_event::VanillaGameEvent;
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID;
    let (sender, receiver, mut world) = fixture();
    let (shulker, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, east(5.0));
    assert_eq!(shulker_armor(&world, shulker), 20.0);
    let before = world
        .stored_entity_for_test(DIMENSION, shulker)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(
        world
            .hurt_stored_entity_by_command_for_test(
                DIMENSION,
                shulker,
                "minecraft:mob_attack",
                10.0,
                None,
            )
            .unwrap()
            .succeeded
    );
    let closed_damage = before
        - world
            .stored_entity_for_test(DIMENSION, shulker)
            .unwrap()
            .living()
            .unwrap()
            .health();
    let mut opened = false;
    let mut open_sound = false;
    for _ in 0..10 {
        let tick = world.tick(Vec::new());
        opened |= shulker_peek_packet(&tick, shulker, 100);
        open_sound |= shulker_shell_sound(&tick, "minecraft:entity.shulker.open");
        if opened && open_sound {
            break;
        }
    }
    assert!(
        opened && open_sound,
        "attack opens the tracked shell with its sound"
    );
    assert_eq!(shulker_armor(&world, shulker), 0.0);
    assert!(world
        .game_events(DIMENSION)
        .0
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::CONTAINER_OPEN
            && event.source_entity_id == Some(shulker)));
    tick_until(&mut world, 400, |world| {
        world
            .player_active_effect_for_test(ALICE, "minecraft:levitation")
            .is_some()
    });
    let before = world
        .stored_entity_for_test(DIMENSION, shulker)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(
        world
            .hurt_stored_entity_by_command_for_test(
                DIMENSION,
                shulker,
                "minecraft:mob_attack",
                10.0,
                None,
            )
            .unwrap()
            .succeeded
    );
    let open_damage = before
        - world
            .stored_entity_for_test(DIMENSION, shulker)
            .unwrap()
            .living()
            .unwrap()
            .health();
    assert!(
        open_damage > closed_damage,
        "the real damage path consumes shell armor"
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        90,
        "gamemode spectator alice",
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    let mut closed = shulker_peek_packet(&tick, shulker, 0);
    let mut close_sound = shulker_shell_sound(&tick, "minecraft:entity.shulker.close");
    for _ in 0..10 {
        let tick = world.tick(Vec::new());
        closed |= shulker_peek_packet(&tick, shulker, 0);
        close_sound |= shulker_shell_sound(&tick, "minecraft:entity.shulker.close");
        if closed && close_sound {
            break;
        }
    }
    assert!(
        closed && close_sound,
        "losing the target closes the tracked shell with its sound"
    );
    assert_eq!(shulker_armor(&world, shulker), 20.0);
    assert!(world
        .game_events(DIMENSION)
        .0
        .iter()
        .any(
            |event| event.game_event == VanillaGameEvent::CONTAINER_CLOSE
                && event.source_entity_id == Some(shulker)
        ));
    assert!(world
        .stored_entity_for_test(DIMENSION, shulker)
        .unwrap()
        .living()
        .unwrap()
        .attributes()
        .has_modifier(VANILLA_ATTRIBUTE_ARMOR_ID, "minecraft:covered"));
}

/// The evoker's spells: three vexes (`Evoker.java:244-264`) and fangs that
/// bite for 6. The vexes are removed so their own charges stay out of the
/// fangs' count.
#[test]
fn an_evoker_summons_vexes_and_bites_with_fangs() {
    let (sender, receiver, mut world) = fixture();
    spawn(&mut world, VANILLA_ENTITY_TYPE_EVOKER_ID, east(6.0));
    assert!(
        first_added(&mut world, 200, VANILLA_ENTITY_TYPE_VEX_ID).is_some(),
        "the evoker summons vexes"
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        30,
        "kill @e[type=minecraft:vex]",
    );
    assert!(console_command_answer(&tick).0.is_some());
    let hits = hits(&mut world, 400);
    assert!(
        hits.iter().any(|(_, lost, _)| *lost == 6.0),
        "the fangs bite for 6: {hits:?}"
    );
}

/// `IllusionerMirrorSpellGoal` turns the illusioner invisible
/// (`Illusioner.java:252-282`).
#[test]
fn an_illusioner_casts_its_mirror_spell() {
    let (_sender, _receiver, mut world) = fixture();
    let (illusioner, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_ILLUSIONER_ID, east(6.0));
    tick_until(&mut world, 200, |world| {
        world
            .stored_entity_for_test(DIMENSION, illusioner)
            .and_then(|entity| entity.living())
            .is_some_and(|living| living.active_effect("minecraft:invisibility").is_some())
    });
}

/// `IllusionerBlindnessSpellGoal` reads the local difficulty: on Normal a fully
/// inhabited chunk is `2 * (0.75 + 0.75) = 3.0`, past NORMAL's ordinal
/// (`Illusioner.java:205-215`).
#[test]
fn an_illusioner_blinds_on_normal_in_a_long_inhabited_chunk() {
    let (_sender, _receiver, mut world) = fixture();
    world.add_inhabited_time_for_test(DIMENSION, 0, 0, 3_600_000);
    spawn(&mut world, VANILLA_ENTITY_TYPE_ILLUSIONER_ID, east(6.0));
    tick_until(&mut world, 600, |world| {
        world
            .player_active_effect_for_test(ALICE, "minecraft:blindness")
            .is_some()
    });
}

/// A snow golem pelts the enemy it targets (`SnowGolem.java:118-133`).
#[test]
fn a_snow_golem_throws_snowballs_at_a_zombie() {
    let (_sender, _receiver, mut world) = fixture();
    let (golem, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, east(3.0));
    spawn(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, east(-3.0));
    assert_eq!(
        first_added(&mut world, 200, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
        Some(golem)
    );
}

/// A landed arrow a player may pick up goes into her inventory
/// (`AbstractArrow.java:643-658`).
#[test]
fn a_player_picks_up_a_landed_arrow() {
    let (_sender, _receiver, mut world) = fixture();
    let mut arrow = crate::entity::ArrowData::new(plain_stack("minecraft:arrow", 1).unwrap());
    arrow.set_pickup(crate::entity::ArrowPickup::Allowed);
    let (id, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 1.0,
            y: PLAYER_POSITION.y + 1.5,
            z: PLAYER_POSITION.z,
        },
        EntityData::Arrow(arrow),
    );
    tick_until(&mut world, 100, |world| {
        world.stored_entity_for_test(DIMENSION, id).is_none()
    });
    let arrows: i32 = world
        .player_inventory_for_test(ALICE)
        .inventory_slots()
        .iter()
        .filter(|stack| stack.item_resource_id() == Some("minecraft:arrow"))
        .map(|stack| stack.count())
        .sum();
    assert_eq!(arrows, 1);
    assert_eq!(health(&world), 20.0);
}

fn shulker_armor(world: &ServerWorld, id: i32) -> f64 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID)
}

fn shulker_peek_packet(tick: &TickResult, id: i32, peek: i8) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
        if update.id == id && update.values.iter().any(|value|
            value.data_id == bbb_protocol::entity_synched_data::VANILLA_SHULKER_PEEK_DATA_ID
            && value.value == bbb_protocol::packets::EntityDataValueKind::Byte(peek)))
    })
}

fn shulker_shell_sound(tick: &TickResult, resource_id: &str) -> bool {
    let id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id).unwrap();
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id }
        && sound.volume == 1.0 && sound.pitch == 1.0)
    })
}

#[test]
fn shulker_shell_state_and_other_armor_modifiers_survive_reload() {
    let (sender, receiver, mut world) = fixture();
    assert!(world.set_block(
        DIMENSION,
        BlockPos { x: 14, y: 65, z: 8 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL
    ));
    let (id, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, east(5.0));
    let mut closed_data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_SHULKER_ID).unwrap();
    closed_data.mob.as_mut().unwrap().set_no_ai(true);
    let (_, closed_uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        east(8.0),
        EntityData::GeneralLiving(closed_data),
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        91,
        &format!("data merge entity {uuid} {{NoAI:1b,AttachFace:5b,Peek:-5b,Color:-31b}}"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    let tick = console_command(&sender, &receiver, &mut world, 92, &format!(
        "data merge entity {uuid} {{attributes:[{{id:\"minecraft:armor\",base:3d,modifiers:[{{id:\"bbb:extra\",amount:2d,operation:\"add_value\"}},{{id:\"minecraft:covered\",amount:99d,operation:\"add_value\"}}]}}]}}"
    ));
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert_eq!(shulker_armor(&world, id), 30.0);
    let request = world
        .save_request(1_786_000_008_000, 212, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    install_forced_ticket(&mut candidate);
    crate::tests::entities::restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(datapack_config(), &candidate);
    let id = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    assert_eq!(shulker_armor(&restored, id), 30.0);
    let closed = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, closed_uuid)
        .unwrap();
    assert_eq!(shulker_armor(&restored, closed), 20.0);
    assert_eq!(
        restored
            .stored_entity_for_test(DIMENSION, closed)
            .unwrap()
            .loot_component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID
            )
            .unwrap(),
        None
    );
    let NbtSummaryValue::Compound(fields) = restored
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity record")
    };
    for (name, expected) in [("AttachFace", 5), ("Peek", -5), ("Color", -31)] {
        assert_eq!(fields.iter().filter(|field| field.name == name).count(), 1);
        assert_eq!(
            fields
                .iter()
                .find(|field| field.name == name)
                .unwrap()
                .value,
            NbtSummaryValue::Byte(expected)
        );
    }
    let entity = restored.stored_entity_for_test(DIMENSION, id).unwrap();
    assert_eq!(
        entity
            .loot_component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID
            )
            .unwrap(),
        Some(NbtSummaryValue::string("white"))
    );
    let armor = entity
        .living()
        .unwrap()
        .attributes()
        .pack()
        .into_iter()
        .find(|attribute| {
            attribute.attribute_id == bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID
        })
        .unwrap();
    assert_eq!(armor.base_value, 3.0);
    assert!(armor
        .modifiers
        .iter()
        .any(|modifier| modifier.id == "bbb:extra" && modifier.amount == 2.0));
    assert!(armor
        .modifiers
        .iter()
        .any(|modifier| modifier.id == "minecraft:covered" && modifier.amount == 99.0));
    let values = crate::entity::tracking::synched_values(entity);
    for (data_id, expected) in [
        (
            bbb_protocol::entity_synched_data::VANILLA_SHULKER_ATTACH_FACE_DATA_ID,
            bbb_protocol::packets::EntityDataValueKind::Direction(5),
        ),
        (
            bbb_protocol::entity_synched_data::VANILLA_SHULKER_PEEK_DATA_ID,
            bbb_protocol::packets::EntityDataValueKind::Byte(-5),
        ),
        (
            bbb_protocol::entity_synched_data::VANILLA_SHULKER_COLOR_DATA_ID,
            bbb_protocol::packets::EntityDataValueKind::Byte(-31),
        ),
    ] {
        assert!(values
            .iter()
            .any(|row| row.value.data_id == data_id && row.value.value == expected));
    }
}
