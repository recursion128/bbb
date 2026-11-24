use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::level_event::VanillaGameEvent;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_LINGERING_POTION_ID,
    VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id;
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityStore};

use crate::entity::thrown_potion::registry_potion_stack;
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{
    spawn_lingering_potion, spawn_splash_potion, tick_entities, tick_one_entity,
    AreaEffectCloudData, EntityData, ThrownPotionData,
};

fn effect(resource_id: &str, amplifier: i8, duration: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::string(resource_id),
        },
        NbtSummaryEntry {
            name: "amplifier".into(),
            value: NbtSummaryValue::Byte(amplifier),
        },
        NbtSummaryEntry {
            name: "duration".into(),
            value: NbtSummaryValue::Int(duration),
        },
        NbtSummaryEntry {
            name: "show_icon".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ])
}

fn custom_contents(effects: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "custom_effects".into(),
        value: NbtSummaryValue::List(effects),
    }])
}

fn potion_stack(
    item_resource_id: &str,
    contents: NbtSummaryValue,
    duration_scale: Option<f32>,
) -> RuntimeItemStack {
    let mut components = vec![NbtSummaryEntry {
        name: "minecraft:potion_contents".into(),
        value: contents,
    }];
    if let Some(scale) = duration_scale {
        components.push(NbtSummaryEntry {
            name: "minecraft:potion_duration_scale".into(),
            value: NbtSummaryValue::Float(scale.to_bits()),
        });
    }
    RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(item_resource_id),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(components),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .expect("the potion stack uses the persistent ItemStack shape")
}

fn potion_data(store: &EntityStore, id: i32) -> ThrownPotionData {
    match store.get(id).expect("the potion is live").data() {
        EntityData::SplashPotion(data) | EntityData::LingeringPotion(data) => data.clone(),
        other => panic!("expected a thrown potion, found {other:?}"),
    }
}

fn land_potion(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..20 {
        level.with_level(|level| tick_entities(store, level, true));
        if store.get(id).is_none() {
            return;
        }
    }
    panic!("the thrown potion never hit")
}

#[test]
fn thrown_potion_enderman_only_takes_clean_water_damage_before_teleporting() {
    let mut level = NativeLevel::new();
    for x in -2..=2 {
        for z in -2..=2 {
            level
                .world
                .load_entity_chunk_for_test(DIMENSION, (x, z))
                .unwrap();
        }
    }
    for (potion_id, custom_harm) in [
        ("minecraft:water", false),
        ("minecraft:harming", false),
        ("minecraft:water", true),
    ] {
        let mut store = EntityStore::new(17, EntityIdCounter::new());
        let target = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID,
                Vec3d {
                    x: 0.5,
                    y: 96.0,
                    z: 0.5,
                },
            )
            .unwrap();
        let cow = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 2.5,
                    y: 96.0,
                    z: 0.5,
                },
            )
            .unwrap();
        for id in [target, cow] {
            let body = store.get_mut(id).unwrap();
            let EntityData::GeneralLiving(data) = &mut body.data else {
                unreachable!()
            };
            data.set_no_ai(true);
            body.set_no_gravity(true);
        }
        let target_health = store.get(target).unwrap().living().unwrap().health();
        let cow_health = store.get(cow).unwrap().living().unwrap().health();
        let stack = if custom_harm {
            potion_stack(
                "minecraft:splash_potion",
                NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "potion".into(),
                        value: NbtSummaryValue::String(potion_id.into()),
                    },
                    NbtSummaryEntry {
                        name: "custom_effects".into(),
                        value: NbtSummaryValue::List(vec![effect(
                            "minecraft:instant_damage",
                            0,
                            1,
                        )]),
                    },
                ]),
                None,
            )
        } else {
            registry_potion_stack("minecraft:splash_potion", potion_id).unwrap()
        };
        let saved = stack
            .write(&bbb_registry_codec::UnavailableHolderResolver)
            .unwrap()
            .unwrap();
        let stack =
            RuntimeItemStack::try_read(&saved, &bbb_registry_codec::UnavailableHolderResolver)
                .unwrap();
        let potion = spawn_splash_potion(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 100.0,
                z: 0.5,
            },
            ProjectileShot::new(Vec3d {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            }),
            stack,
            None,
        )
        .unwrap();
        land_potion(&mut store, &mut level, potion);
        let target = store.get(target).unwrap();
        let clean = potion_id == "minecraft:water" && !custom_harm;
        assert_eq!(
            target.living().unwrap().health(),
            target_health - f32::from(clean)
        );
        assert_eq!(target.position().y, 64.0);
        assert_eq!(
            store.get(cow).unwrap().living().unwrap().health() < cow_health,
            !clean
        );
    }
}

#[test]
fn thrown_potion_block_land_carries_the_affected_state_after_contact() {
    let mut store = EntityStore::new(210, EntityIdCounter::new());
    let potion = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        }),
        registry_potion_stack("minecraft:splash_potion", "minecraft:water").unwrap(),
        None,
    )
    .unwrap();
    let pos = BlockPos { x: 1, y: 67, z: 0 };
    let stone = state_id("minecraft:stone");
    let mut level = NativeLevel::new();
    level.set(pos, stone);

    level.with_level(|level| tick_one_entity(&mut store, level, potion));

    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|positioned| {
            let event = &positioned.recorded;
            event.game_event == crate::level_event::VanillaGameEvent::PROJECTILE_LAND
                && event.pos == pos
                && event.source_entity_id == Some(potion)
                && event.affected_state == Some(stone)
        }));
}

#[test]
fn registry_tag_reload_updates_thrown_potion_water_cache_and_next_tick_inertia() {
    let mut store = EntityStore::new(26, EntityIdCounter::new());
    let potion = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }),
        registry_potion_stack("minecraft:splash_potion", "minecraft:water").unwrap(),
        None,
    )
    .unwrap();
    store.drain_lifecycle_events();
    let mut entity = store.detach(potion).unwrap();
    let mut level = NativeLevel::new();
    for x in 0..=3 {
        for y in 63..=64 {
            level.set(BlockPos { x, y, z: 0 }, state_id("minecraft:water"));
        }
    }
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:fluid",
        "minecraft:water",
        &["minecraft:water"],
    )]));

    level.with_level(|level| crate::entity::thrown_potion::tick(&mut entity, &mut store, level));
    assert!((entity.delta_movement().x - f64::from(0.99_f32)).abs() < 1.0e-12);
    assert!(entity.was_touching_water());

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:fluid",
        "minecraft:water",
        &[],
    )]));
    level.with_level(|level| crate::entity::thrown_potion::tick(&mut entity, &mut store, level));
    assert!((entity.delta_movement().x - f64::from(0.99_f32) * f64::from(0.8_f32)).abs() < 1.0e-12);
    assert!(!entity.was_touching_water());
}

#[test]
fn fixed_wither_add_effect_rejection_does_not_require_live_admission_tags() {
    let mut store = EntityStore::new(0xF1CE, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_WITHER_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let potion = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.1,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        registry_potion_stack("minecraft:splash_potion", "minecraft:slowness").unwrap(),
        None,
    )
    .unwrap();
    store.drain_lifecycle_events();
    let mut entity = store.detach(potion).unwrap();
    let mut level = NativeLevel::new();
    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:fluid", "minecraft:water", &[]),
        (
            "minecraft:entity_type",
            "minecraft:deflects_projectiles",
            &[],
        ),
    ]));

    level.with_level(|level| crate::entity::thrown_potion::tick(&mut entity, &mut store, level));

    assert_eq!(entity.removal_reason(), Some(RemovalReason::Discarded));
    assert!(store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:slowness")
        .is_none());
}

#[test]
fn thrown_potion_splash_applies_custom_effects_and_uses_signed_argb_color() {
    let mut store = EntityStore::new(21, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let before = store.get(target).unwrap().living().unwrap().health();
    let stack = potion_stack(
        "minecraft:splash_potion",
        custom_contents(vec![
            effect("minecraft:instant_damage", 0, 1),
            effect("minecraft:wither", 0, 100),
        ]),
        Some(0.5),
    );
    let potion = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        stack,
        None,
    )
    .unwrap();
    let mut level = NativeLevel::new();

    land_potion(&mut store, &mut level, potion);

    assert_eq!(
        store.get(target).unwrap().living().unwrap().health(),
        before - 6.0
    );
    let wither = store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .active_effects()
        .find(|effect| effect.resource_id() == "minecraft:wither")
        .unwrap();
    assert_eq!(wither.duration(), 50);

    let harm = vanilla_mob_effect_type_for_resource_id("minecraft:instant_damage")
        .unwrap()
        .color;
    let wither = vanilla_mob_effect_type_for_resource_id("minecraft:wither")
        .unwrap()
        .color;
    let rgb = ((((harm >> 16) & 0xff) + ((wither >> 16) & 0xff)) / 2) << 16
        | ((((harm >> 8) & 0xff) + ((wither >> 8) & 0xff)) / 2) << 8
        | (((harm & 0xff) + (wither & 0xff)) / 2);
    let events = level.world.drain_level_events_for_test(DIMENSION);
    let color = events
        .iter()
        .find(|event| event.event_type == 2002)
        .expect("the potion break event")
        .data;
    assert_eq!(color, (0xff00_0000 | rgb) as i32);
    assert!(
        color < 0,
        "PotionContents.getColorOptional returns opaque ARGB"
    );
}

#[test]
fn a_lethal_splash_raises_entity_damage_then_entity_die_from_the_victim() {
    let mut store = EntityStore::new(21, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let stack = potion_stack(
        "minecraft:splash_potion",
        custom_contents(vec![effect("minecraft:instant_damage", 2, 1)]),
        None,
    );
    let potion = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        stack,
        None,
    )
    .unwrap();
    let mut level = NativeLevel::new();

    land_potion(&mut store, &mut level, potion);

    assert!(store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .is_dead_or_dying());
    let raised = level.world.positioned_game_events_for_test(DIMENSION);
    for kind in [
        VanillaGameEvent::ENTITY_DAMAGE,
        VanillaGameEvent::ENTITY_DIE,
    ] {
        assert!(raised
            .iter()
            .any(|event| event.recorded.source_entity_id == Some(target)
                && event.recorded.game_event == kind));
    }
}

#[test]
fn thrown_potion_break_event_uses_base_potion_instant_effects_only() {
    for contents in [
        "{potion:'minecraft:harming'}",
        "{potion:'minecraft:harming',custom_color:1193046,custom_name:'named',custom_effects:[{id:'minecraft:speed',duration:100}]}",
    ] {
        let mut store = EntityStore::new(26, EntityIdCounter::new());
        let potion = spawn_splash_potion(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            ProjectileShot::new(Vec3d {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            }),
            potion_stack(
                "minecraft:splash_potion",
                bbb_protocol::nbt_summary::parse_snbt_compound(contents).unwrap(),
                None,
            ),
            None,
        )
        .unwrap();
        let mut level = NativeLevel::new();

        land_potion(&mut store, &mut level, potion);

        assert!(level
            .world
            .drain_level_events_for_test(DIMENSION)
            .iter()
            .any(|event| event.event_type == 2007));
    }
}

#[test]
fn witch_runtime_builtin_registry_splash_hit_applies_effect() {
    let mut store = EntityStore::new(25, EntityIdCounter::new());
    let target = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let potion = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        potion_stack(
            "minecraft:splash_potion",
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "potion".into(),
                value: NbtSummaryValue::string("minecraft:slowness"),
            }]),
            Some(0.5),
        ),
        None,
    )
    .unwrap();
    let mut level = NativeLevel::new();

    land_potion(&mut store, &mut level, potion);

    let slowness = store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:slowness")
        .expect("the built-in registry effect is applied on hit");
    assert_eq!(slowness.duration(), 900);
}

#[test]
fn thrown_potion_lingering_spawns_the_custom_effect_cloud() {
    let mut store = EntityStore::new(22, EntityIdCounter::new());
    let owner = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 10.5,
                y: 64.0,
                z: 10.5,
            },
        )
        .unwrap();
    let owner_uuid = store.get(owner).unwrap().uuid();
    let contents = custom_contents(vec![effect("minecraft:wither", 1, 100)]);
    let stack = potion_stack("minecraft:lingering_potion", contents.clone(), Some(0.5));
    let potion = spawn_lingering_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        stack,
        Some(owner_uuid),
    )
    .unwrap();
    let mut level = NativeLevel::new();

    land_potion(&mut store, &mut level, potion);

    let cloud = store
        .ids_in_chunk((0, 0))
        .into_iter()
        .find_map(|id| match store.get(id).unwrap().data() {
            EntityData::AreaEffectCloud(data) => Some(data.clone()),
            _ => None,
        })
        .expect("the lingering potion creates a cloud");
    assert_eq!(
        cloud,
        AreaEffectCloudData::new()
            .with_radius(3.0)
            .with_radius_on_use(-0.5)
            .with_duration(600)
            .with_wait_time(10)
            .with_radius_per_tick(-3.0 / 600.0)
            .with_owner(Some(owner_uuid))
            .with_potion_duration_scale(0.5)
            .with_potion_contents(Some(
                bbb_registry_codec::persistent_item_stack_projection::potion_contents::read(
                    bbb_registry_codec::RegistryElementValueRef::Nbt(&contents),
                    bbb_protocol::nbt_summary::NbtSummaryDecodeLimits::default().max_depth,
                )
                .unwrap(),
            ))
    );
}

#[test]
fn lingering_cloud_preserves_a_connected_player_owner() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "owner", 1);
    crate::tests::place_player(
        &sender,
        &receiver,
        &mut level.world,
        1,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        None,
    );
    let counter = EntityIdCounter::new();
    let occupied = level
        .world
        .player_entity_id_for_test(crate::connection::ConnectionId(1));
    while counter.next_id() <= occupied {}
    let mut store = EntityStore::new(0x1A2B, counter);
    let owner = uuid::Uuid::from_u128(1);
    let contents = custom_contents(vec![effect("minecraft:wither", 0, 40)]);
    let potion = spawn_lingering_potion(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        potion_stack("minecraft:lingering_potion", contents, None),
        Some(owner),
    )
    .unwrap();

    land_potion(&mut store, &mut level, potion);

    let cloud = store
        .ids_in_chunk((0, 0))
        .into_iter()
        .find_map(|id| match store.get(id).unwrap().data() {
            EntityData::AreaEffectCloud(data) => Some(data),
            _ => None,
        })
        .expect("the lingering potion creates a cloud");
    assert_eq!(cloud.owner(), Some(owner));
}

#[test]
fn thrown_potion_save_tracking_and_owner_round_trip() {
    let mut store = EntityStore::new(23, EntityIdCounter::new());
    let owner = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 20.5,
                y: 64.0,
                z: 20.5,
            },
        )
        .unwrap();
    let owner_uuid = store.get(owner).unwrap().uuid();
    let contents = custom_contents(vec![effect("minecraft:wither", 1, 100)]);
    let splash = spawn_splash_potion(
        &mut store,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        }),
        potion_stack("minecraft:splash_potion", contents.clone(), Some(0.5)),
        Some(owner_uuid),
    )
    .unwrap();
    let lingering = spawn_lingering_potion(
        &mut store,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 3.5,
        },
        ProjectileShot::new(Vec3d {
            x: -0.1,
            y: 0.2,
            z: 0.3,
        }),
        potion_stack("minecraft:lingering_potion", contents, Some(0.5)),
        Some(owner_uuid),
    )
    .unwrap();
    assert_eq!(
        store.get(splash).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_SPLASH_POTION_ID
    );
    assert_eq!(
        store.get(lingering).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_LINGERING_POTION_ID
    );
    for id in [splash, lingering] {
        let item = non_default_synched_values(store.get(id).unwrap())
            .into_iter()
            .find(|value| value.data_id == 8)
            .expect("the exact custom potion item is tracked");
        let EntityDataValueKind::ItemStack(item) = item.value else {
            panic!("the tracked potion item has the wrong serializer")
        };
        assert_eq!(item.component_patch.potion_contents.custom_effects.len(), 1);
        assert_eq!(
            item.component_patch.potion_duration_scale_bits,
            Some(0.5_f32.to_bits())
        );
    }

    let document = write_entity_chunk(&store, (0, 0)).expect("potion entities save");
    let mut reloaded = EntityStore::new(24, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("potion entities reload");
    let restored: Vec<ThrownPotionData> = ids
        .into_iter()
        .filter_map(|id| match reloaded.get(id).unwrap().data() {
            EntityData::SplashPotion(_) | EntityData::LingeringPotion(_) => {
                Some(potion_data(&reloaded, id))
            }
            _ => None,
        })
        .collect();
    assert_eq!(restored.len(), 2);
    assert!(restored
        .iter()
        .all(|data| data.projectile().owner() == Some(owner_uuid)));
    assert!(restored.iter().all(|data| !data.projectile().left_owner()));
    assert!(restored
        .iter()
        .all(|data| !data.projectile().has_been_shot()));
}

fn splash_at_origin(store: &mut EntityStore, stack: RuntimeItemStack) -> i32 {
    spawn_splash_potion(
        store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }),
        stack,
        None,
    )
    .unwrap()
}

#[test]
fn a_potion_break_event_carries_its_custom_colour() {
    let mut store = EntityStore::new(21, EntityIdCounter::new());
    let mut contents = custom_contents(vec![effect("minecraft:slowness", 0, 100)]);
    let NbtSummaryValue::Compound(fields) = &mut contents else {
        unreachable!()
    };
    fields.push(NbtSummaryEntry {
        name: "custom_color".into(),
        value: NbtSummaryValue::Int(0x0012_3456),
    });
    let potion = splash_at_origin(
        &mut store,
        potion_stack("minecraft:splash_potion", contents, None),
    );
    let mut level = NativeLevel::new();
    land_potion(&mut store, &mut level, potion);
    let events = level.world.drain_level_events_for_test(DIMENSION);
    let event = events
        .iter()
        .find(|event| event.event_type == 2002)
        .expect("the potion break event");
    assert_eq!(event.data, 0x0012_3456);
}

#[test]
fn a_water_splash_puts_out_a_burning_armor_stand() {
    let mut store = EntityStore::new(21, EntityIdCounter::new());
    let stand = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        },
    )
    .unwrap();
    store.get_mut(stand).unwrap().set_remaining_fire_ticks(200);
    let potion = splash_at_origin(
        &mut store,
        registry_potion_stack("minecraft:splash_potion", "minecraft:water").unwrap(),
    );
    let mut level = NativeLevel::new();
    land_potion(&mut store, &mut level, potion);
    assert!(!store.get(stand).unwrap().is_on_fire());
}
