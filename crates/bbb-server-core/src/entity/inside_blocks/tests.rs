use super::*;
use crate::entity::tests::native_level::{state, NativeLevel};
use crate::entity::{EntityIdCounter, EntityStore};
use crate::fluid::with_property;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;

fn position(x: f64, y: f64) -> Vec3d {
    Vec3d { x, y, z: 0.5 }
}

fn body(position: Vec3d, living: bool) -> (Box<Entity>, EntityStore) {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = if living {
        store
            .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position)
            .unwrap()
    } else {
        crate::entity::experience_orb::spawn_experience_orb(&mut store, position, 1).unwrap()
    };
    (store.detach(id).unwrap(), store)
}

#[test]
fn body_contact_cauldron_empty_space_does_not_hide_later_entry() {
    let cauldron = state("minecraft:water_cauldron");
    let mut native = NativeLevel::new();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    native.set(pos, cauldron);
    let outside = EntityMovement::direct(position(0.5, 64.75), position(0.5, 64.7));
    let (entity, _store) = body(outside.from, false);
    let geometry = body_contact::BlockContactGeometry {
        width: entity.width,
        height: entity.height,
    };
    let enter = EntityMovement::direct(outside.to, position(0.5, 64.5));
    let contacts = |movements: Vec<EntityMovement>, native: &mut NativeLevel| {
        native.with_level(|level| {
            let mut cursor = body_contact::BodyContactCursor::new(geometry, movements);
            let mut contacts = Vec::new();
            while let Some(contact) = cursor
                .next_contact(level.chunks(), entity.fall_distance)
                .expect("loaded cauldron")
            {
                contacts.push(contact);
            }
            contacts
        })
    };
    assert!(!contacts(vec![outside], &mut native)
        .iter()
        .any(|contact| contact.pos == pos && contact.inside_block));
    assert!(contacts(vec![outside, enter], &mut native)
        .iter()
        .any(|contact| contact.pos == pos && contact.state == cauldron && contact.inside_block));
}

#[test]
fn lava_cauldron_hurts_ignites_and_thaws_only_contacting_living_bodies() {
    for (y, touches) in [(64.5, true), (64.95, false)] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID,
                position(0.5, y),
            )
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        if let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        entity.set_no_gravity(true);
        entity.set_ticks_frozen(17);
        let mut native = NativeLevel::new();
        native.set(
            BlockPos { x: 0, y: 64, z: 0 },
            state("minecraft:lava_cauldron"),
        );
        native.with_level(|level| crate::entity::tick_entities(&mut store, level, true));
        let entity = store.get(id).unwrap();
        assert_eq!(
            entity.living().unwrap().health(),
            if touches { 6.0 } else { 10.0 }
        );
        assert_eq!(entity.ticks_frozen() == 0, touches);
        assert_eq!(entity.remaining_fire_ticks() > 0, touches);
        assert_eq!(
            store.drain_lifecycle_events().iter().any(|event| matches!(
                event,
                crate::entity::EntityLifecycleEvent::Sound { sound }
                    if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
                        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                            "minecraft:entity.generic.burn",
                        ).unwrap(),
                    }
            )),
            touches
        );
    }
}

#[test]
fn layered_cauldron_extinguishes_entities_and_consumes_contents_with_interaction_permission() {
    use crate::block_behavior::cauldron;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, VANILLA_BLOCK_WATER_CAULDRON_ID,
    };
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let mut native = NativeLevel::new();
    native.world.set_game_rule_value(
        "minecraft:mob_griefing",
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
        &mut Vec::new(),
    );
    for (kind, fill, expected) in [
        (VANILLA_BLOCK_WATER_CAULDRON_ID, 3, 2),
        (VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, 3, 2),
        (VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, 2, 1),
        (VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, 1, 0),
    ] {
        native.set(pos, cauldron::layered_cauldron_state(kind, fill).unwrap());
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID,
                position(0.5, 64.5),
            )
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        if let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        entity.set_no_gravity(true);
        entity.set_remaining_fire_ticks(100);
        native.with_level(|level| {
            assert!(!level.mob_griefing());
            crate::entity::tick_entities(&mut store, level, true);
        });
        let entity = store.get(id).unwrap();
        assert!(entity.remaining_fire_ticks() <= 0);
        assert_eq!(entity.ticks_frozen(), 0);
        let expected = if expected == 0 {
            cauldron::empty_cauldron_state()
        } else {
            cauldron::layered_cauldron_state(VANILLA_BLOCK_WATER_CAULDRON_ID, expected).unwrap()
        };
        native.with_level(|level| {
            assert_eq!(level.block_state(pos), expected);
            crate::entity::tick_entities(&mut store, level, true);
            assert_eq!(level.block_state(pos), expected);
        });
    }
    for owned in [false, true] {
        native.set(pos, cauldron::filled_water_cauldron_state());
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let owner = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position(5.5, 64.5))
            .unwrap();
        let owner = store.get(owner).unwrap().uuid();
        let id = crate::entity::spawn_arrow(
            &mut store,
            position(0.5, 64.5),
            Vec3d::default(),
            crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
        )
        .unwrap();
        let entity = store.get_mut(id).unwrap();
        if let crate::entity::EntityData::Arrow(data) = &mut entity.data {
            data.projectile.set_owner(owned.then_some(owner));
        }
        entity.set_no_gravity(true);
        entity.set_remaining_fire_ticks(100);
        native.with_level(|level| {
            crate::entity::tick_entities(&mut store, level, true);
            assert_eq!(
                level.block_state(pos),
                cauldron::layered_cauldron_state(
                    VANILLA_BLOCK_WATER_CAULDRON_ID,
                    if owned { 3 } else { 2 },
                )
                .unwrap()
            );
        });
        assert!(store.get(id).unwrap().remaining_fire_ticks() <= 0);
    }
}

#[test]
fn cobweb_contact_uses_weaving_and_ignores_movement_efficiency() {
    for (living, weaving) in [(false, false), (true, false), (true, true)] {
        for efficiency in [0.0, 1.0] {
            let (mut entity, mut store) = body(position(0.5, 64.0), living);
            entity.fall_distance = 6.0;
            entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
            if let Some(living) = entity.living_mut() {
                living
                    .attributes_mut()
                    .get_instance(
                        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID,
                    )
                    .unwrap()
                    .set_base_value(efficiency);
                if weaving {
                    living.add_active_effect(
                        crate::entity::ActiveMobEffect::new("minecraft:weaving", 200, 0).unwrap(),
                    );
                }
            }
            let mut native = NativeLevel::new();
            native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:cobweb"));
            assert!(
                native.with_level(|level| apply_nonliving_inside_block_effects(
                    &mut entity,
                    level,
                    Some(&mut store)
                ))
            );
            let expected = if weaving {
                Vec3d {
                    x: 0.5,
                    y: 0.25,
                    z: 0.5,
                }
            } else {
                Vec3d {
                    x: 0.25,
                    y: f64::from(0.05_f32),
                    z: 0.25,
                }
            };
            assert_eq!(entity.stuck_speed_multiplier(), expected);
            assert_eq!(entity.fall_distance(), 0.0);
        }
    }
}

#[test]
fn campfire_contact_above_collision_shape_hurts_careful_living_bodies() {
    for (resource, amount) in [
        ("minecraft:campfire", 1.0),
        ("minecraft:soul_campfire", 2.0),
    ] {
        for health in [10.0, 1.0] {
            let (mut entity, mut store) = body(position(0.5, 64.75), true);
            entity.living_mut().unwrap().set_health(health);
            entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
            entity.set_ticks_frozen(17);
            let mut native = NativeLevel::new();
            native.set(BlockPos { x: 0, y: 64, z: 0 }, state(resource));
            assert!(
                native.with_level(|level| apply_nonliving_inside_block_effects(
                    &mut entity,
                    level,
                    Some(&mut store)
                ))
            );
            assert_eq!(
                entity.living().unwrap().health(),
                (health - amount).max(0.0)
            );
            assert_eq!(entity.remaining_fire_ticks(), 0);
            assert_eq!(entity.ticks_frozen(), 17);
        }
    }
}

#[test]
fn campfire_nonliving_and_unlit_contacts_do_not_ignite_or_hurt() {
    for (living, lit) in [(false, "true"), (true, "false")] {
        let (mut entity, mut store) = body(position(0.5, 64.0), living);
        let health = entity.living().map(|living| living.health());
        let mut native = NativeLevel::new();
        native.set(
            BlockPos { x: 0, y: 64, z: 0 },
            with_property(state("minecraft:campfire"), "lit", lit).unwrap(),
        );
        assert!(
            native.with_level(|level| apply_nonliving_inside_block_effects(
                &mut entity,
                level,
                Some(&mut store)
            ))
        );
        assert_eq!(entity.living().map(|living| living.health()), health);
        assert_eq!(entity.remaining_fire_ticks(), 0);
    }
}

#[test]
fn wither_rose_does_not_hurt_nonliving_body() {
    let (mut entity, mut store) = body(position(0.5, 64.0), false);
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 63, z: 0 }, state("minecraft:dirt"));
    native.set(
        BlockPos { x: 0, y: 64, z: 0 },
        state("minecraft:wither_rose"),
    );
    assert!(
        native.with_level(|level| apply_nonliving_inside_block_effects(
            &mut entity,
            level,
            Some(&mut store)
        ))
    );
    assert!(entity.removal_reason().is_none());
}

#[test]
fn honey_nonliving_slide_resets_fall_and_publishes_sound_and_entity_event() {
    use crate::connection::ClientboundPlayPacket;
    use crate::entity::EntityLifecycleEvent;
    use crate::random::GameRandom;
    use bbb_protocol::packets::SoundSource;
    for silent in [false, true] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let at = position(0.5, 64.2);
        let random = GameRandom::with_seed(1);
        let ids = [
            crate::entity::minecart::spawn_minecart(&mut store, at, 0.0).unwrap(),
            crate::entity::boat::spawn_boat(
                &mut store,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                at,
                0.0,
            )
            .unwrap(),
            crate::entity::tnt::spawn_tnt(&mut store, &random, at, None).unwrap(),
        ];
        for id in ids {
            let mut native = NativeLevel::new();
            native.set(
                BlockPos { x: 0, y: 64, z: 0 },
                state("minecraft:honey_block"),
            );
            let mut entity = store.detach(id).unwrap();
            entity.set_pos(position(0.03125 - f64::from(entity.bb_width() / 2.0), 64.2));
            entity.set_old_pos_and_rot();
            entity.set_silent(silent);
            store.drain_lifecycle_events();
            for _ in 0..64 {
                entity.set_delta_movement(Vec3d {
                    x: 0.2,
                    y: -0.3,
                    z: 0.1,
                });
                entity.fall_distance = 6.0;
                assert!(
                    native.with_level(|level| apply_nonliving_inside_block_effects(
                        &mut entity,
                        level,
                        Some(&mut store)
                    ))
                );
                assert_eq!(entity.fall_distance(), 0.0);
                assert_eq!(
                    entity.delta_movement().y,
                    (-0.05 - 0.08) * f64::from(0.98_f32)
                );
            }
            let outputs = store.drain_lifecycle_events();
            let sounds: Vec<_> = outputs
                .iter()
                .filter_map(|output| {
                    if let EntityLifecycleEvent::Sound { sound } = output {
                        Some(sound)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(sounds.is_empty(), silent);
            for sound in sounds {
                assert_eq!(sound.position, entity.position());
                assert_eq!(sound.source, SoundSource::Neutral);
                assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
            }
            assert!(outputs.iter().any(|output| matches!(output, EntityLifecycleEvent::Broadcast { entity_id, packet: ClientboundPlayPacket::EntityEvent(event) } if *entity_id == id && event.entity_id == id && event.event_id == 53)));
        }
    }
}

/// `PowderSnowBlock.java:65`: a non-living body brushing the snow is slowed
/// even though the block it stands in is not the snow.
#[test]
fn powder_snow_slows_a_nonliving_body_from_a_neighbouring_cell() {
    let (mut entity, mut store) = body(position(0.9, 64.0), false);
    let mut native = NativeLevel::new();
    native.set(
        BlockPos { x: 1, y: 64, z: 0 },
        state("minecraft:powder_snow"),
    );
    assert!(
        native.with_level(|level| apply_nonliving_inside_block_effects(
            &mut entity,
            level,
            Some(&mut store)
        ))
    );
    assert_eq!(
        entity.stuck_speed_multiplier(),
        Vec3d {
            x: f64::from(0.9_f32),
            y: 1.5,
            z: f64::from(0.9_f32)
        }
    );
}

/// `PowderSnowBlock.java:88-92`: a burning arrow only melts the snow its
/// owner may build at (`Projectile.java:361-363`).
#[test]
fn a_burning_arrow_melts_powder_snow_only_where_its_owner_may_build() {
    let snow = BlockPos { x: 10, y: 64, z: 0 };
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    native.set_border(bbb_server_types::interaction::WorldBorder::from_settings(
        0.0, 0.0, 0.2, 5.0, 5, 15, 8.0, 0, 8.0, 0,
    ));
    let alice = uuid::Uuid::from_u128(1);
    for (owner, melts) in [(None, true), (Some(alice), false)] {
        native.set(snow, state("minecraft:powder_snow"));
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = crate::entity::spawn_arrow(
            &mut store,
            Vec3d {
                x: 10.5,
                y: 64.5,
                z: 0.5,
            },
            Vec3d::default(),
            crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
        )
        .unwrap();
        let mut entity = store.detach(id).unwrap();
        if let crate::entity::EntityData::Arrow(data) = &mut entity.data {
            data.projectile.set_owner(owner);
        }
        entity.set_remaining_fire_ticks(100);
        native.with_level(|level| {
            apply_nonliving_inside_block_effects(&mut entity, level, Some(&mut store));
            assert_eq!(
                level.block_state(snow) == state("minecraft:air"),
                melts,
                "owner {owner:?}"
            );
        });
    }
}
