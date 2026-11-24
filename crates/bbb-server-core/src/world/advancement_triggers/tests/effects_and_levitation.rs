use bbb_server_datapack::loot::parse_mob_effects_predicate;

use super::*;

fn effects_changed(conditions: Value) -> Value {
    json!({"criteria":{"changed":{"trigger":"minecraft:effects_changed","conditions":conditions}}})
}

fn levitation(conditions: Value) -> Value {
    json!({"criteria":{"levitated":{"trigger":"minecraft:levitation","conditions":conditions}}})
}

pub(super) fn effect(world: &mut ServerWorld, arguments: &str) {
    command(world, &format!("effect {arguments}"));
}

fn active_effects(effects: &[ActiveMobEffect]) -> ActiveMobEffects {
    ActiveMobEffects::from_decoded_saved_effects(effects.to_vec())
}

fn stage_survival_arena(world: &mut ServerWorld, fill: &str, tp: &str) {
    command(world, tp);
    command(world, "gamemode survival @s");
    player_loaded(world);
    command(world, fill);
}

fn has_effect_update(events: &[WorldEvent], effect: &str) -> bool {
    let effect_id = bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(effect)
        .unwrap()
        .id;
    events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: CONNECTION,
                packet: ClientboundPlayPacket::UpdateMobEffect(update),
            } if update.effect_id == effect_id
        )
    })
}

/// `EffectsChangedTrigger.trigger` (`EffectsChangedTrigger.java:21-24`) reads
/// the whole effect map, so a predicate naming two effects is owed by whichever
/// change completes the pair.
#[test]
fn an_effect_change_is_offered_the_map_it_left_behind() {
    let mut world = world(&[(
        "test:both",
        effects_changed(json!({"effects":{"minecraft:speed":{},"minecraft:jump_boost":{}}})),
    )]);
    effect(&mut world, "give @s minecraft:speed 100");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:both"));
    effect(&mut world, "give @s minecraft:jump_boost 100");
    assert!(world.player_advancement_is_done(CONNECTION, "test:both"));
}

#[test]
fn effects_changed_player_object_and_list_read_the_actual_player_location() {
    let predicate = json!({"type":"minecraft:player", "location":{"position":{"x":{"min":1000.0}}},
            "nbt":"{SelectedItem:{id:\"minecraft:diamond\",count:1}}"});
    for player in [
        predicate.clone(),
        json!([this_entity_properties(predicate)]),
    ] {
        let mut advancement = effects_changed(json!({
            "player":player, "effects":{"minecraft:speed":{}}
        }));
        advancement["rewards"] = json!({"experience":13});
        let mut world = world(&[("test:player", advancement)]);
        world.levels[CONNECTION].inventory.set_item_in_hand(
            InteractionHand::MainHand,
            crate::entity::item::plain_stack("minecraft:diamond", 1).unwrap(),
        );
        let position = world.levels[CONNECTION].position();
        for (cmd, eff, expected_xp) in [
            (
                Some(format!("tp @s 1500 {} {}", position.y, position.z)),
                "give @s minecraft:jump_boost 100",
                0,
            ),
            (
                Some(format!("tp @s 10 {} {}", position.y, position.z)),
                "give @s minecraft:speed 100",
                0,
            ),
            (
                Some(format!("tp @s 1500 {} {}", position.y, position.z)),
                "",
                0,
            ),
            (
                Some("item replace entity @s weapon.mainhand with stone".into()),
                "give @s minecraft:jump_boost 200",
                0,
            ),
            (
                Some("item replace entity @s weapon.mainhand with diamond".into()),
                "give @s minecraft:jump_boost 300",
                13,
            ),
            (None, "give @s minecraft:speed 200", 13),
        ] {
            if let Some(cmd) = cmd {
                command(&mut world, &cmd);
            }
            if !eff.is_empty() {
                effect(&mut world, eff);
            }
            assert_eq!(xp(&world), expected_xp);
        }
    }
}

/// `MobEffectInstancePredicate.matches` (`MobEffectsPredicate.java:76-88`).
#[test]
fn an_effects_predicate_reads_the_instance_terms() {
    let predicate = |body: Value| EffectsChangedCondition {
        player: Vec::new(),
        effects: Some(parse_mob_effects_predicate(&body).unwrap()),
        source: None,
    };
    let speed = |duration, amplifier, particles| {
        active_effects(&[
            ActiveMobEffect::give("minecraft:speed", duration, amplifier, particles).unwrap(),
        ])
    };
    assert!(!predicate(json!({"minecraft:speed":{}})).matches(&active_effects(&[])));
    for (body, (dur, amp, vis), expected) in [
        (json!({"minecraft:speed":{}}), (100, 0, true), true),
        (
            json!({"minecraft:speed":{"amplifier":{"min":2}}}),
            (100, 3, true),
            true,
        ),
        (
            json!({"minecraft:speed":{"amplifier":2}}),
            (100, 3, true),
            false,
        ),
        (
            json!({"minecraft:speed":{"duration":{"max":100}}}),
            (100, 0, true),
            true,
        ),
        (
            json!({"minecraft:speed":{"duration":{"max":99}}}),
            (100, 0, true),
            false,
        ),
        (
            json!({"minecraft:speed":{"ambient":false}}),
            (100, 0, true),
            true,
        ),
        (
            json!({"minecraft:speed":{"ambient":true}}),
            (100, 0, true),
            false,
        ),
        (
            json!({"minecraft:speed":{"visible":false}}),
            (100, 0, false),
            true,
        ),
        (
            json!({"minecraft:speed":{"visible":false}}),
            (100, 0, true),
            false,
        ),
        (json!({"speed":{}}), (100, 0, true), true),
    ] {
        assert_eq!(predicate(body).matches(&speed(dur, amp, vis)), expected);
    }
    assert!(parse_mob_effects_predicate(&json!({"minecraft:nonesuch":{}})).is_err());
    assert!(parse_mob_effects_predicate(
        &json!({"minecraft:speed":{"duration":{"min":2,"max":1}}})
    )
    .is_err());
}

/// The same terms through the session `/effect` moves: a change that only flips
/// the particle flag is still a change (`MobEffectInstance.update`,
/// `MobEffectInstance.java:164-166`).
#[test]
fn a_hidden_particle_effect_answers_the_visible_term() {
    let mut world = world(&[(
        "test:hidden",
        effects_changed(json!({"effects":{"minecraft:speed":{"visible":false}}})),
    )]);
    effect(&mut world, "give @s minecraft:speed 100 0 false");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:hidden"));
    effect(&mut world, "give @s minecraft:speed 200 0 true");
    assert!(world.player_advancement_is_done(CONNECTION, "test:hidden"));
}

/// `TriggerInstance.matches`' source arm (`EffectsChangedTrigger.java:50`) is
/// false when the effect was added without a source entity.
#[test]
fn a_source_naming_criterion_takes_the_null_arm() {
    let mut world = world(&[(
        "test:sourced",
        effects_changed(
            json!({"source":[this_entity_properties(json!({"type":"minecraft:axolotl"}))]}),
        ),
    )]);
    world.add_player_mob_effect(
        CONNECTION,
        ActiveMobEffect::new("minecraft:regeneration", 2000, 0).unwrap(),
        &mut Vec::new(),
    );
    assert!(!world.player_advancement_is_done(CONNECTION, "test:sourced"));
}

#[test]
fn effect_command_matches_the_actual_source_entity_and_requires_one_for_empty_lists() {
    let sheep =
        json!({"type":"minecraft:sheep", "location":{"position":{"x":{"min":29.0,"max":31.0}}}});
    let mut world = world(&[
        ("test:object", effects_changed(json!({"source":sheep}))),
        (
            "test:list",
            effects_changed(json!({"source":[this_entity_properties(sheep)]})),
        ),
        (
            "test:wrong",
            effects_changed(json!({"source":{"type":"minecraft:zombie"}})),
        ),
        ("test:any_source", effects_changed(json!({"source":[]}))),
        (
            "test:player",
            effects_changed(
                json!({"source":{"type":"minecraft:player"},"effects":{"minecraft:jump_boost":{}}}),
            ),
        ),
    ]);
    let dimension = world.levels[CONNECTION].dimension;
    command(&mut world, "summon minecraft:sheep 30 80 0");
    let store = &world.levels.get(&dimension).unwrap().entities;
    let source = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SHEEP_ID)
        .unwrap()
        .uuid();
    let origin = ServerWorld::console_origin();
    world.perform_command(
        bbb_protocol::JavaString::with_utf16(
            "effect give inventory_player minecraft:speed 100",
            |units| std::sync::Arc::from(units),
        ),
        &crate::commands::CommandSource::server("console", CommandPermission::Owner),
        &origin,
        crate::commands::ResultCallback::Empty,
        None,
        &mut Vec::new(),
    );
    assert!(!world.player_advancement_is_done(CONNECTION, "test:any_source"));
    effect(&mut world, "clear @s");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:any_source"));
    command(
        &mut world,
        &format!("execute as {source} in minecraft:the_nether positioned 0 80 0 run effect give inventory_player minecraft:speed 100"),
    );
    assert_advancements(
        &world,
        CONNECTION,
        &["test:object", "test:list", "test:any_source"],
        &["test:wrong"],
    );
    effect(&mut world, "give @s minecraft:jump_boost 100");
    assert!(world.player_advancement_is_done(CONNECTION, "test:player"));
}

#[test]
fn a_live_elder_guardian_pulse_matches_its_source_and_sends_the_display() {
    let mut world = world(&[
        (
            "test:elder",
            effects_changed(json!({
                "source":{"type":"minecraft:elder_guardian"},
                "effects":{"minecraft:mining_fatigue":{"amplifier":2}}
            })),
        ),
        (
            "test:wrong",
            effects_changed(json!({"source":{"type":"minecraft:sheep"}})),
        ),
    ]);
    world.cold.config.keep_alive_interval_ticks = 10000;
    command(&mut world, "gamemode survival @s");
    command(&mut world, "tp @s 0 80 0");
    command(&mut world, "summon minecraft:elder_guardian 30 80 0");
    let dimension = world.levels[CONNECTION].dimension;
    let store = &world.levels.get(&dimension).unwrap().entities;
    let source = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|entity| {
            entity.entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
        })
        .unwrap()
        .uuid();
    command(
        &mut world,
        &format!("data merge entity {source} {{NoGravity:1b,PersistenceRequired:1b}}"),
    );
    let mut displayed = false;
    for _ in 0..=1200 {
        let events = world.tick(Vec::new()).events;
        displayed |= events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::GameEvent(event) }
                if event.event_id == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID
                    && event.param == 1.0
        ));
        if displayed {
            break;
        }
    }
    assert!(displayed);
    let fatigue = world.levels[CONNECTION]
        .active_effect("minecraft:mining_fatigue")
        .unwrap();
    assert_eq!(fatigue.amplifier(), 2);
    assert!(fatigue.duration() > 5900);
    assert_advancements(&world, CONNECTION, &["test:elder"], &["test:wrong"]);
}

#[test]
fn live_mob_melee_effects_match_the_attacker_type_and_location() {
    for (mob, effect_id) in [
        ("minecraft:cave_spider", "minecraft:poison"),
        ("minecraft:husk", "minecraft:hunger"),
    ] {
        let mut world = world(&[
            (
                "test:attacker",
                effects_changed(json!({
                    "source":{"type":mob,"location":{"position":{
                        "x":{"min":28.0,"max":34.0},"y":{"min":79.0,"max":82.0}
                    }}},
                    "effects":{effect_id:{}}
                })),
            ),
            (
                "test:wrong",
                effects_changed(json!({"source":{"type":"minecraft:sheep"}})),
            ),
        ]);
        command(&mut world, "difficulty normal");
        command(&mut world, "time set midnight");
        command(&mut world, "fill 24 79 -8 40 79 8 minecraft:stone");
        command(&mut world, "tp @s 30.5 80 0.5");
        command(&mut world, "gamemode survival @s");
        command(&mut world, &format!("summon {mob} 31.5 80 0.5"));
        for _ in 0..100 {
            if world.player_advancement_is_done(CONNECTION, "test:attacker") {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(
            world.levels[CONNECTION].active_effect(effect_id).is_some(),
            "{mob}"
        );
        assert_advancements(&world, CONNECTION, &["test:attacker"], &["test:wrong"]);
    }
}

#[test]
fn a_live_dolphin_gives_a_swimming_player_grace_with_its_actual_source() {
    let mut world = world(&[
        (
            "test:dolphin",
            effects_changed(json!({
                "source":{"type":"minecraft:dolphin","location":{"position":{
                    "x":{"min":28.0,"max":35.0},"y":{"min":79.0,"max":84.0}
                }}},
                "effects":{"minecraft:dolphins_grace":{}}
            })),
        ),
        (
            "test:wrong",
            effects_changed(json!({"source":{"type":"minecraft:sheep"}})),
        ),
    ]);
    command(&mut world, "fill 27 79 -4 35 79 4 minecraft:stone");
    command(&mut world, "fill 27 80 -4 35 82 4 minecraft:water");
    command(&mut world, "tp @s 30.5 80 0.5");
    command(&mut world, "gamemode survival @s");
    for packet in [
        PlayServerbound::PlayerLoaded,
        PlayServerbound::PlayerCommand(bbb_protocol::packets::PlayerCommand {
            entity_id: world.levels[CONNECTION].entity_id(),
            action: bbb_protocol::packets::PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    ] {
        send_play(&mut world, CONNECTION, packet);
    }
    world.tick(Vec::new());
    assert!(world.levels[CONNECTION]
        .shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING));
    command(&mut world, "summon minecraft:dolphin 29.5 80 0.5");
    for _ in 0..100 {
        if world.levels[CONNECTION]
            .active_effect("minecraft:dolphins_grace")
            .is_some()
        {
            break;
        }
        world.tick(Vec::new());
    }
    assert!(world.levels[CONNECTION]
        .active_effect("minecraft:dolphins_grace")
        .is_some());
    assert_advancements(&world, CONNECTION, &["test:dolphin"], &["test:wrong"]);
}

#[test]
fn fired_tipped_and_spectral_arrows_use_the_actual_player_owner_as_effect_source() {
    for (ammo, effect_id) in [
        (
            "minecraft:tipped_arrow[potion_contents={potion:\"minecraft:poison\"}]",
            "minecraft:poison",
        ),
        ("minecraft:spectral_arrow", "minecraft:glowing"),
    ] {
        let mut world = kill_world(&[
            (
                "test:owner",
                effects_changed(json!({
                    "source":{"type":"minecraft:player","location":{"position":{"z":{"min":2.0,"max":3.0}}}},
                    "effects":{effect_id:{}}
                })),
            ),
            (
                "test:wrong",
                effects_changed(json!({"source":{"type":"minecraft:skeleton"}})),
            ),
        ]);
        command(&mut world, "fill 4 79 0 12 79 12 minecraft:stone");
        command(&mut world, "tp @s 8.5 80 2.5 0 0");
        command(&mut world, "tp victim_player 8.5 80 6.5");
        player_loaded(&mut world);
        give(&mut world, "minecraft:bow", 1);
        give(&mut world, ammo, 1);
        send_play(
            &mut world,
            CONNECTION,
            PlayServerbound::UseItem(bbb_protocol::packets::UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 0.0,
                x_rot: 0.0,
            }),
        );
        for _ in 0..20 {
            world.tick(Vec::new());
        }
        send_play(
            &mut world,
            CONNECTION,
            PlayServerbound::PlayerAction(bbb_protocol::packets::PlayerAction {
                action: bbb_protocol::packets::PlayerActionKind::ReleaseUseItem,
                pos: BlockPos { x: 0, y: 0, z: 0 },
                direction: Direction::Down,
                sequence: 2,
            }),
        );
        for _ in 0..30 {
            if world.levels[VICTIM].active_effect(effect_id).is_some() {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(
            world.levels[VICTIM].active_effect(effect_id).is_some(),
            "{ammo}"
        );
        assert_advancements(&world, VICTIM, &["test:owner"], &["test:wrong"]);
    }
}

#[test]
fn a_real_shulker_bullet_hit_uses_the_shulker_as_its_effect_source() {
    let mut world = world(&[
        (
            "test:shulker",
            effects_changed(json!({
                "source":{"type":"minecraft:shulker","location":{"position":{"x":{"min":11.0,"max":14.0}}}},
                "effects":{"minecraft:levitation":{}}
            })),
        ),
        (
            "test:wrong",
            effects_changed(json!({"source":{"type":"minecraft:shulker_bullet"}})),
        ),
    ]);
    world.cold.config.keep_alive_interval_ticks = 10000;
    command(&mut world, "difficulty normal");
    stage_survival_arena(
        &mut world,
        "fill 4 79 4 15 79 12 minecraft:stone",
        "tp @s 8.5 80 8.5",
    );
    command(&mut world, "summon minecraft:shulker 12.5 80 8.5");
    for _ in 0..400 {
        if world.levels[CONNECTION]
            .active_effect("minecraft:levitation")
            .is_some()
        {
            break;
        }
        world.tick(Vec::new());
    }
    assert!(world.levels[CONNECTION]
        .active_effect("minecraft:levitation")
        .is_some());
    assert_advancements(&world, CONNECTION, &["test:shulker"], &["test:wrong"]);
}

#[test]
fn wither_skulls_use_a_nonliving_owner_or_the_live_projectile_before_removal() {
    use crate::entity::hurting_projectile::wither_skull::WitherSkullData;
    use crate::entity::hurting_projectile::HurtingProjectileState;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
    };
    for (owned, transferred) in [(false, false), (true, false), (true, true)] {
        let (source_type, wrong_type) = if owned {
            ("minecraft:item", "minecraft:wither_skull")
        } else {
            ("minecraft:wither_skull", "minecraft:item")
        };
        let mut source = json!({"type":source_type});
        if transferred {
            source["location"] = json!({"position":{"x":{"min":1000.0,"max":1001.0}}});
        }
        let mut world = world(&[
            (
                "test:source",
                effects_changed(json!({"source":source,"effects":{"minecraft:wither":{}}})),
            ),
            (
                "test:wrong",
                effects_changed(json!({"source":{"type":wrong_type}})),
            ),
        ]);
        command(&mut world, "difficulty normal");
        stage_survival_arena(
            &mut world,
            "fill 4 79 0 12 79 12 minecraft:stone",
            "tp @s 8.5 80 6.5",
        );
        let owner = owned.then(|| {
            world
                .spawn_stored_entity_for_test(
                    Java26BuiltinDimension::Overworld,
                    VANILLA_ENTITY_TYPE_ITEM_ID,
                    Vec3d {
                        x: 8.5,
                        y: 80.0,
                        z: 1.5,
                    },
                    EntityData::Item(crate::entity::item::ItemEntityData::new(
                        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
                        0.0,
                    )),
                )
                .1
        });
        let (skull, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            Vec3d {
                x: 8.5,
                y: 80.8,
                z: 3.5,
            },
            EntityData::WitherSkull(WitherSkullData::new(
                HurtingProjectileState::new(
                    owner,
                    HurtingProjectileState::INITIAL_ACCELERATION_POWER,
                ),
                false,
            )),
        );
        let dimension = world.levels[CONNECTION].dimension;
        crate::entity::projectile_shoot::ProjectileShot::new(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        })
        .install(
            world
                .level_mut_by_id(&dimension)
                .entities
                .get_mut(skull)
                .unwrap(),
        );
        if transferred {
            let owner = owner.unwrap();
            command(
                &mut world,
                &format!("execute in minecraft:the_nether run tp {owner} 1000.5 80 1.5"),
            );
            let nether = world.require_level_id(&Java26BuiltinDimension::Nether.into());
            assert_eq!(world.entity_reference(&dimension, owner).unwrap().0, nether);
        }
        for _ in 0..20 {
            if world.levels[CONNECTION]
                .active_effect("minecraft:wither")
                .is_some()
            {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(world.levels[CONNECTION]
            .active_effect("minecraft:wither")
            .is_some());
        assert_advancements(&world, CONNECTION, &["test:source"], &["test:wrong"]);
        assert!(world.level_by_id(&dimension).entities.get(skull).is_none());
    }
}

#[test]
fn a_network_thrown_splash_uses_the_player_owner_as_its_effect_source() {
    let mut world = world(&[
        (
            "test:owner",
            effects_changed(
                json!({"source":{"type":"minecraft:player"},"effects":{"minecraft:poison":{}}}),
            ),
        ),
        (
            "test:projectile",
            effects_changed(json!({"source":{"type":"minecraft:splash_potion"}})),
        ),
    ]);
    stage_survival_arena(
        &mut world,
        "fill 4 79 4 12 79 12 minecraft:stone",
        "tp @s 8.5 80 8.5",
    );
    give(
        &mut world,
        "minecraft:splash_potion[potion_contents={potion:\"minecraft:poison\"}]",
        1,
    );
    let mut events = send_play(
        &mut world,
        CONNECTION,
        PlayServerbound::UseItem(bbb_protocol::packets::UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 90.0,
        }),
    );
    for _ in 0..40 {
        if world.levels[CONNECTION]
            .active_effect("minecraft:poison")
            .is_some()
        {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    assert!(has_effect_update(&events, "minecraft:poison"));
    assert_advancements(&world, CONNECTION, &["test:owner"], &["test:projectile"]);
}

#[test]
fn an_ownerless_loaded_splash_is_its_effect_source_until_the_hit_finishes() {
    let mut world = world(&[(
        "test:potion",
        effects_changed(
            json!({"source":{"type":"minecraft:splash_potion"},"effects":{"minecraft:poison":{}}}),
        ),
    )]);
    stage_survival_arena(
        &mut world,
        "fill 4 79 4 12 79 12 minecraft:stone",
        "tp @s 8.5 80 8.5",
    );
    let (potion, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
        Vec3d {
            x: 8.5,
            y: 81.0,
            z: 8.5,
        },
        EntityData::SplashPotion(crate::entity::thrown_potion::ThrownPotionData::new(
            crate::entity::item::plain_stack("minecraft:splash_potion", 1).unwrap(),
        )),
    );
    load_effect_source_nbt(
        &mut world,
        potion,
        r#"{Item:{id:"minecraft:splash_potion",count:1,components:{"minecraft:potion_contents":{potion:"minecraft:poison"}}}}"#,
    );
    let mut events = Vec::new();
    let dimension = world.levels[CONNECTION].dimension;
    for _ in 0..40 {
        if world.levels[CONNECTION]
            .active_effect("minecraft:poison")
            .is_some()
        {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    assert!(has_effect_update(&events, "minecraft:poison"));
    assert!(world.player_advancement_is_done(CONNECTION, "test:potion"));
    assert!(world.level_by_id(&dimension).entities.get(potion).is_none());
}

#[test]
fn an_owned_cloud_remains_its_effect_source_when_the_first_use_exhausts_its_radius() {
    let mut world = world(&[
        (
            "test:cloud",
            effects_changed(
                json!({"source":{"type":"minecraft:area_effect_cloud"},"effects":{"minecraft:poison":{}}}),
            ),
        ),
        (
            "test:owner",
            effects_changed(json!({"source":{"type":"minecraft:player"}})),
        ),
    ]);
    stage_survival_arena(
        &mut world,
        "fill 4 79 4 12 79 12 minecraft:stone",
        "tp @s 8.5 80 8.5",
    );
    let (cloud, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        Vec3d {
            x: 8.5,
            y: 80.0,
            z: 8.5,
        },
        EntityData::AreaEffectCloud(crate::entity::area_effect_cloud::AreaEffectCloudData::new()),
    );
    load_effect_source_nbt(
        &mut world,
        cloud,
        r#"{Owner:[I;0,0,0,1],WaitTime:0,Duration:100,Radius:1f,RadiusOnUse:-1f,potion_contents:{potion:"minecraft:poison"}}"#,
    );
    let mut events = Vec::new();
    let dimension = world.levels[CONNECTION].dimension;
    for _ in 0..10 {
        if world.levels[CONNECTION]
            .active_effect("minecraft:poison")
            .is_some()
        {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    assert!(has_effect_update(&events, "minecraft:poison"));
    assert_advancements(&world, CONNECTION, &["test:cloud"], &["test:owner"]);
    assert!(world.level_by_id(&dimension).entities.get(cloud).is_none());
}

#[test]
fn a_remote_cloud_owner_controls_damage_immunity_and_the_damage_packet_source() {
    for (source_x, immune) in [(1000.0, true), (8.0, false)] {
        let mut world = world(&[]);
        let mut packs = (*world.cold.config.datapacks.as_ref().unwrap().as_ref()).clone();
        let frost = packs
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == crate::enchantment_facts::ENCHANTMENT_REGISTRY)
            .unwrap()
            .elements
            .iter_mut()
            .find(|element| element.id == "minecraft:frost_walker")
            .unwrap();
        let mut doc: serde_json::Value = serde_json::from_slice(&frost.source_json).unwrap();
        doc["effects"] = json!({
            "minecraft:damage_immunity":[{
                "effect":{},
                "requirements":{
                    "condition":"minecraft:damage_source_properties",
                    "predicate":{"source_entity":{
                        "type":"minecraft:sheep",
                        "location":{"position":{"x":{"min":source_x,"max":source_x + 1.0}}}
                    }}
                }
            }]
        });
        frost.source_json = serde_json::to_vec(&doc).unwrap().into();
        frost.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&frost.source_json).unwrap());
        Arc::make_mut(&mut world.datapack_tables).enchantment_combat_effects =
            crate::enchantment_combat_effects::load_enchantment_combat_effects(Some(&packs));
        world.cold.config.datapacks = Some(Arc::new(packs));
        let (owner, owner_id) = remote_potion_owner(&mut world);
        command(
            &mut world,
            "item replace entity @s armor.feet with leather_boots[enchantments={frost_walker:1}]",
        );
        let cloud = owned_harming_cloud(&mut world, owner);
        let mut events = Vec::new();
        for _ in 0..10 {
            events.extend(world.tick(Vec::new()).events);
        }
        let damage: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: CONNECTION,
                    packet: ClientboundPlayPacket::DamageEvent(damage),
                } => Some(damage),
                _ => None,
            })
            .collect();
        if immune {
            assert_eq!(world.player_health_for_test(CONNECTION), 20.0);
            assert!(damage.is_empty());
        } else {
            assert!(world.player_health_for_test(CONNECTION) < 20.0);
            assert!(damage
                .iter()
                .any(|d| d.source_cause_id == owner_id && d.source_direct_id == cloud));
        }
    }
}

#[test]
fn a_fatal_cloud_reads_its_remote_owner_in_the_killing_blow_source_predicate() {
    let cloud_components = |scale| {
        killed(
            "minecraft:entity_killed_player",
            json!({
                "killing_blow":{"direct_entity":{
                    "type":"minecraft:area_effect_cloud",
                    "components":{
                        "minecraft:potion_contents":{"potion":"minecraft:harming"},
                        "minecraft:potion_duration_scale":scale
                    }
                }}
            }),
        )
    };
    let mut world = world(&[
        (
            "test:remote",
            killed(
                "minecraft:entity_killed_player",
                json!({"killing_blow":{"source_entity":{"type":"minecraft:sheep","location":{"position":{"x":{"min":1000.0,"max":1001.0}}}}}}),
            ),
        ),
        (
            "test:near",
            killed(
                "minecraft:entity_killed_player",
                json!({"killing_blow":{"source_entity":{"type":"minecraft:sheep","location":{"position":{"x":{"min":8.0,"max":9.0}}}}}}),
            ),
        ),
        ("test:cloud_components", cloud_components(0.5)),
        ("test:wrong_cloud_scale", cloud_components(1.0)),
    ]);
    let (owner, _) = remote_potion_owner(&mut world);
    world.set_player_health_for_test(CONNECTION, 2.0);
    let cloud = owned_harming_cloud(&mut world, owner);
    load_effect_source_nbt(&mut world, cloud, "{potion_duration_scale:0.5f}");
    let dimension = world.levels[CONNECTION].dimension;
    for _ in 0..10 {
        world.tick(Vec::new());
    }
    assert_eq!(world.player_health_for_test(CONNECTION), 0.0);
    assert_advancements(
        &world,
        CONNECTION,
        &["test:remote", "test:cloud_components"],
        &["test:near", "test:wrong_cloud_scale"],
    );
    assert!(world.level_by_id(&dimension).entities.get(cloud).is_none());
}

/// `ServerPlayer.tick`'s levitation arm (`ServerPlayer.java:600-602`) over the
/// window `onEffectAdded` opened (`:1639-1641`).
#[test]
fn levitation_measures_the_position_the_effect_was_added_at() {
    let mut world = world(&[
        (
            "test:up",
            levitation(
                json!({"distance":{"y":{"min":50.0}},"player":{"effects":{"minecraft:levitation":{}}}}),
            ),
        ),
        (
            "test:patient",
            levitation(json!({"distance":{"y":{"min":50.0}},"duration":{"min":1000}})),
        ),
        (
            "test:wrong_player",
            levitation(
                json!({"distance":{"y":{"min":50.0}},"player":{"effects":{"minecraft:speed":{}}}}),
            ),
        ),
    ]);
    let start = world.levels[CONNECTION].position();
    effect(&mut world, "give @s minecraft:levitation 100");
    assert_eq!(
        world.levels[CONNECTION].cold.levitation_start_position,
        Some(start)
    );
    world.tick(Vec::new());
    assert!(!world.player_advancement_is_done(CONNECTION, "test:up"));
    command(
        &mut world,
        &format!("teleport @s {} {} {}", start.x, start.y + 60.0, start.z),
    );
    assert_advancements(
        &world,
        CONNECTION,
        &["test:up"],
        &["test:patient", "test:wrong_player"],
    );
}

/// `:1659-1661` closes the window on the way out, so a session that has landed
/// is offered nothing however far it later travels.
#[test]
fn losing_levitation_closes_the_window() {
    let mut world = world(&[(
        "test:up",
        levitation(json!({"distance":{"y":{"min":50.0}}})),
    )]);
    let start = world.levels[CONNECTION].position();
    effect(&mut world, "give @s minecraft:levitation 100");
    effect(&mut world, "clear @s minecraft:levitation");
    assert_eq!(
        world.levels[CONNECTION].cold.levitation_start_position,
        None
    );
    command(
        &mut world,
        &format!("teleport @s {} {} {}", start.x, start.y + 60.0, start.z),
    );
    assert!(!world.player_advancement_is_done(CONNECTION, "test:up"));
}

/// The whole surface the built-in generation asks of these two triggers: the
/// two potion collections, the axolotl source and the one levitation criterion.
#[test]
fn the_built_in_effect_criteria_all_compile() {
    let startup = built_in_startup();
    let tree = AdvancementTree::from_loaded(&startup.advancements);
    let compiled = AdvancementTriggers::compile(&tree, Some(startup));
    assert_eq!(
        compiled_criterion_ids(&tree, &compiled.effects_changed),
        [
            "minecraft:husbandry/kill_axolotl_target/kill_axolotl_target",
            "minecraft:nether/all_effects/all_effects",
            "minecraft:nether/all_potions/all_effects",
        ]
    );
    assert_eq!(
        compiled_criterion_ids(&tree, &compiled.levitation),
        ["minecraft:end/levitate/levitated"]
    );
}
