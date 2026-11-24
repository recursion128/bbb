use super::*;
use crate::entity::EntityData;

mod ambient_context;
mod ambient_states;

#[test]
fn summoned_runtimes_finalize_at_the_effect_origin_and_join_the_affected_player_team() {
    let (mut world, root) = world_with_post_attack_effects(
        "attacker",
        "victim",
        serde_json::json!([
            {"type":"minecraft:summon_entity", "entity":"minecraft:player"},
            {"type":"minecraft:summon_entity", "entity":"minecraft:piglin_brute", "join_team":true},
            {"type":"minecraft:summon_entity", "entity":"minecraft:minecart", "join_team":true}
        ]),
    );
    let position = Vec3d {
        x: 8.125,
        y: 70.25,
        z: -4.875,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(position);
    let zombie = armed_zombie(
        &mut world,
        Vec3d {
            x: 12.5,
            ..position
        },
    );
    let owner = world
        .stored_entity_for_test(OVERWORLD, zombie)
        .unwrap()
        .uuid();
    world.scoreboard_mut().add_team("affected");
    world.scoreboard_mut().add_team("owner");
    world
        .scoreboard_mut()
        .add_player_to_team("alice", "affected");
    world
        .scoreboard_mut()
        .add_player_to_team(&owner.to_string(), "owner");
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    let player = world.levels[PLAYER].profile.uuid;
    world
        .level_mut(OVERWORLD)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
    let result = world.tick(Vec::new());
    assert!(result.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send {
            connection_id,
            packet: crate::connection::ClientboundPlayPacket::Sound(sound),
        } if *connection_id == PLAYER
            && matches!(sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                    == Some("minecraft:entity.piglin_brute.ambient"))
            && sound.source == bbb_protocol::packets::SoundSource::Hostile
            && sound.volume == 1.0
            && (0.8..=1.2).contains(&sound.pitch)
            && sound.position == Vec3d { x: 8.5, y: 70.0, z: -4.5 }
    )));
    let store = &world.level(OVERWORLD).entities;
    for kind in ["minecraft:piglin_brute", "minecraft:minecart"] {
        let type_id = bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(kind)
            .unwrap()
            .id;
        let entity = store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(id))
            .find(|body| body.entity_type_id() == type_id)
            .unwrap();
        assert_eq!(entity.position(), position, "{kind}");
        assert_eq!(
            world
                .scoreboard
                .board()
                .players_team(&entity.uuid().to_string())
                .unwrap()
                .name,
            "affected"
        );
        match kind {
            "minecraft:piglin_brute" => assert_eq!(
                entity
                    .living()
                    .unwrap()
                    .item_by_slot(EquipmentSlot::MainHand)
                    .and_then(|item| item.item_resource_id()),
                Some("minecraft:golden_axe")
            ),
            _ => assert!(matches!(entity.data(), EntityData::Minecart(_))),
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_summoned_lightning_bolt_credits_the_item_owner_and_joins_the_stored_victims_team() {
    let (mut world, root) = world_with_post_attack_effects(
        "attacker",
        "victim",
        serde_json::json!([
            {"type":"minecraft:summon_entity", "entity":"minecraft:lightning_bolt", "join_team":true}
        ]),
    );
    let position = Vec3d {
        x: 11.125,
        y: 70.25,
        z: -4.875,
    };
    let victim = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        position,
    );
    world.load_stored_entity_record_for_test(
        OVERWORLD,
        victim,
        &[
            bbb_protocol::NbtSummaryEntry {
                name: "NoAI".into(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "NoGravity".into(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
        ],
    );
    world
        .stored_entity_mut_for_test(OVERWORLD, victim)
        .unwrap()
        .set_pos(position);
    let victim_uuid = world
        .stored_entity_for_test(OVERWORLD, victim)
        .unwrap()
        .uuid();
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = sword;
    world.scoreboard_mut().add_team("affected");
    world.scoreboard_mut().add_team("owner");
    world
        .scoreboard_mut()
        .add_player_to_team(&victim_uuid.to_string(), "affected");
    world.scoreboard_mut().add_player_to_team("alice", "owner");
    let player_uuid = world.levels[PLAYER].profile.uuid;
    let attacker = PendingHurtEntity::Player { uuid: player_uuid };
    world
        .level_mut(OVERWORLD)
        .event_sink
        .post_attack(PendingPostAttack {
            victim_entity_id: victim,
            damage_type: "minecraft:player_attack",
            causing: Some(attacker.clone()),
            direct: Some(attacker),
        });
    world.tick(Vec::new());
    let store = &world.level(OVERWORLD).entities;
    let bolt = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|entity| matches!(entity.data(), EntityData::LightningBolt(_)))
        .unwrap();
    let EntityData::LightningBolt(data) = bolt.data() else {
        unreachable!()
    };
    assert_eq!(data.cause(), Some(player_uuid));
    assert_eq!(bolt.position(), position);
    assert_eq!(
        world
            .scoreboard
            .board()
            .players_team(&bolt.uuid().to_string())
            .unwrap()
            .name,
        "affected"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn animal_sound_world() -> (ServerWorld, std::path::PathBuf) {
    let config = crate::tests::datapack_config();
    let packs = config.datapacks.as_ref().unwrap();
    let mut resources = Vec::new();
    for family in ["cow", "chicken", "pig", "cat", "wolf"] {
        let registry = packs
            .registries
            .registry(&format!("minecraft:{family}_sound_variant"))
            .unwrap();
        for element in &registry.elements {
            let mut value: serde_json::Value =
                serde_json::from_slice(&element.source_json).unwrap();
            if family == "cow" {
                value["ambient_sound"] =
                    serde_json::json!({"sound_id":"test:cow_ambient", "range":12.0});
            } else if family == "wolf" {
                for age in ["adult", "baby"] {
                    for sound in ["ambient", "growl", "pant", "whine"] {
                        value[format!("{age}_sounds")][format!("{sound}_sound")] = serde_json::json!({
                            "sound_id":format!("test:wolf_{age}_{sound}"), "range":23.0
                        });
                    }
                }
                value["adult_sounds"]["growl_sound"] =
                    serde_json::json!("minecraft:block.note_block.harp");
                value["baby_sounds"]["whine_sound"] =
                    serde_json::json!("minecraft:block.note_block.snare");
            } else {
                value["adult_sounds"]["ambient_sound"] = if family == "chicken" {
                    serde_json::json!("minecraft:block.note_block.bell")
                } else {
                    serde_json::json!({"sound_id":"test:pig_adult", "range":23.0})
                };
                value["baby_sounds"]["ambient_sound"] = serde_json::json!({
                    "sound_id": format!("test:{family}_baby"), "range":23.0
                });
                if family == "cat" {
                    for age in ["adult", "baby"] {
                        for sound in ["ambient", "stray_ambient", "purr", "purreow"] {
                            value[format!("{age}_sounds")][format!("{sound}_sound")] = serde_json::json!({
                                "sound_id":format!("test:cat_{age}_{sound}"), "range":23.0
                            });
                        }
                    }
                }
            }
            resources.push((
                format!(
                    "data/minecraft/{family}_sound_variant/{}.json",
                    element.id.strip_prefix("minecraft:").unwrap()
                ),
                serde_json::to_string(&value).unwrap(),
            ));
        }
    }
    let custom_wolf = resources
        .iter()
        .find(|(path, _)| path == "data/minecraft/wolf_sound_variant/classic.json")
        .unwrap()
        .1
        .clone();
    resources.push((
        "data/aaa/wolf_sound_variant/custom.json".into(),
        custom_wolf,
    ));
    let resources: Vec<_> = resources
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect();
    world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{
            "enchanted":"attacker", "affected":"victim",
            "effect":{"type":"minecraft:all_of", "effects":[
                {"type":"minecraft:summon_entity", "entity":"minecraft:cow"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:chicken"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:pig"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:cat"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:panda"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:enderman"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:polar_bear"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:guardian"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:elder_guardian"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:drowned"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:dolphin"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:allay"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:armadillo"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:goat"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:horse"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:axolotl"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:bat"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:skeleton_horse"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:strider"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:shulker"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:villager"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:zombie_villager"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:zombified_piglin"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:zoglin"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:hoglin"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:breeze"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:sniffer"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:turtle"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:nautilus"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:zombie_nautilus"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:happy_ghast"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:creaking"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:parrot"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:wolf"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:fox"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:piglin"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:warden"},
                {"type":"minecraft:summon_entity", "entity":"minecraft:mooshroom"}
            ]}
        }]}),
        &resources,
    )
}

fn expected_animal_ambient(kind: &str, baby: bool) -> bbb_protocol::packets::SoundEventHolder {
    use bbb_protocol::packets::SoundEventHolder;
    match kind {
        "wolf" => expected_wolf_sound(baby, "ambient"),
        "parrot" => SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.parrot.ambient",
            )
            .unwrap(),
        },
        "cow" => SoundEventHolder::Direct {
            location: "test:cow_ambient".into(),
            fixed_range: Some(12.0),
        },
        "chicken" if !baby => SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:block.note_block.bell",
            )
            .unwrap(),
        },
        "mooshroom" => SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.cow.ambient",
            )
            .unwrap(),
        },
        "happy_ghast" => SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                if baby {
                    "minecraft:entity.ghastling.ambient"
                } else {
                    "minecraft:entity.happy_ghast.ambient"
                },
            )
            .unwrap(),
        },
        "cat" => SoundEventHolder::Direct {
            location: format!(
                "test:cat_{}_stray_ambient",
                if baby { "baby" } else { "adult" }
            ),
            fixed_range: Some(23.0),
        },
        _ => SoundEventHolder::Direct {
            location: format!("test:{kind}_{}", if baby { "baby" } else { "adult" }),
            fixed_range: Some(23.0),
        },
    }
}

fn expected_wolf_sound(baby: bool, sound: &str) -> bbb_protocol::packets::SoundEventHolder {
    use bbb_protocol::packets::SoundEventHolder;
    match (baby, sound) {
        (false, "growl") | (true, "whine") => SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                if baby {
                    "minecraft:block.note_block.snare"
                } else {
                    "minecraft:block.note_block.harp"
                },
            )
            .unwrap(),
        },
        _ => SoundEventHolder::Direct {
            location: format!("test:wolf_{}_{sound}", if baby { "baby" } else { "adult" }),
            fixed_range: Some(23.0),
        },
    }
}

#[test]
fn summoned_animals_play_the_published_sound_set_but_mooshrooms_keep_the_builtin_set() {
    let (mut world, root) = animal_sound_world();
    let position = Vec3d {
        x: 8.125,
        y: 70.25,
        z: -4.875,
    };
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .set_position(position);
    let zombie = armed_zombie(
        &mut world,
        Vec3d {
            x: 12.5,
            ..position
        },
    );
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    let player = world.levels[PLAYER].profile.uuid;
    world
        .level_mut(OVERWORLD)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
    let result = world.tick(Vec::new());
    let piglins_zombify =
        world.with_level_and_entities(OVERWORLD, |level, _| level.piglins_zombify(position));
    let store = &world.level(OVERWORLD).entities;
    for kind in [
        "cow",
        "chicken",
        "pig",
        "mooshroom",
        "cat",
        "panda",
        "enderman",
        "polar_bear",
        "guardian",
        "elder_guardian",
        "drowned",
        "dolphin",
        "allay",
        "armadillo",
        "goat",
        "horse",
        "axolotl",
        "skeleton_horse",
        "strider",
        "villager",
        "zombie_villager",
        "zombified_piglin",
        "zoglin",
        "hoglin",
        "sniffer",
        "nautilus",
        "zombie_nautilus",
        "happy_ghast",
        "creaking",
        "parrot",
        "wolf",
        "fox",
        "piglin",
    ] {
        let resource = format!("minecraft:{kind}");
        let type_id = bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(&resource)
            .unwrap()
            .id;
        let entity = store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(id))
            .find(|body| body.entity_type_id() == type_id && body.position() == position)
            .unwrap_or_else(|| panic!("missing summoned {kind}"));
        let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
        if kind == "wolf" {
            let bbb_protocol::NbtSummaryValue::Compound(saved) = world
                .stored_entity_command_data_for_test(OVERWORLD, entity.id())
                .unwrap()
            else {
                unreachable!()
            };
            assert!(saved.iter().any(|field| matches!(&field.value,
                bbb_protocol::NbtSummaryValue::String(name)
                    if field.name=="sound_variant" && name.with_utf8(|name| world.registries.wolf_sound_variant(name).is_some()).unwrap_or(false)
            )));
            assert!(world.registries.wolf_sound_variant("aaa:custom").is_some());
        }
        let expected = if matches!(
            kind,
            "enderman"
                | "polar_bear"
                | "guardian"
                | "elder_guardian"
                | "drowned"
                | "dolphin"
                | "allay"
                | "armadillo"
                | "goat"
                | "horse"
                | "axolotl"
                | "skeleton_horse"
                | "strider"
                | "villager"
                | "zombie_villager"
                | "zombified_piglin"
                | "zoglin"
                | "hoglin"
                | "sniffer"
                | "nautilus"
                | "zombie_nautilus"
                | "happy_ghast"
                | "creaking"
                | "fox"
                | "piglin"
        ) {
            assert!(!entity.is_in_water(), "fresh {kind} must start dry");
            if kind == "enderman" {
                let EntityData::GeneralLiving(data) = entity.data() else {
                    unreachable!()
                };
                assert!(!data.enderman_retained().unwrap().creepy());
            }
            if kind == "creaking" {
                assert!(!entity.creaking_is_active());
            }
            if kind == "allay" {
                assert!(!entity
                    .living()
                    .unwrap()
                    .item_by_slot(EquipmentSlot::MainHand)
                    .is_some_and(|stack| !stack.is_empty()));
            }
            if kind == "armadillo" {
                assert!(!crate::entity::armadillo::is_scared(entity));
            }
            if kind == "axolotl" {
                assert!(!crate::entity::axolotl::is_playing_dead(entity));
            }
            if kind == "zombified_piglin" {
                assert!(!crate::entity::ai::mob::target::anger::read(entity)
                    .unwrap()
                    .is_angry(store.game_time()));
            }
            if kind == "zoglin" {
                assert!(!store.mob_ai(entity.id()).unwrap().has_target());
            }
            if kind == "hoglin" {
                let EntityData::GeneralLiving(data) = entity.data() else {
                    unreachable!()
                };
                assert!(!data.is_no_ai() && !data.zombification().unwrap().immune);
                assert_eq!(piglins_zombify, Some(true));
            }
            let suffix = match kind {
                "sniffer" => {
                    assert!(crate::entity::sniffer::allows_idle_stroll(entity));
                    "idle"
                }
                "hoglin" | "piglin" => "retreat",
                "polar_bear" if baby => "ambient_baby",
                "guardian" | "elder_guardian" | "nautilus" | "zombie_nautilus" => "ambient_land",
                "allay" => "ambient_without_item",
                "axolotl" => "idle_air",
                "goat" => {
                    let EntityData::GeneralLiving(data) = entity.data() else {
                        unreachable!()
                    };
                    if data.goat_retained().unwrap().is_screaming() {
                        "screaming.ambient"
                    } else {
                        "ambient"
                    }
                }
                _ => "ambient",
            };
            let sound_kind = if kind == "horse" && baby {
                "baby_horse"
            } else if kind == "nautilus" && baby {
                "baby_nautilus"
            } else if kind == "happy_ghast" && baby {
                "ghastling"
            } else {
                kind
            };
            bbb_protocol::packets::SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    &format!("minecraft:entity.{sound_kind}.{suffix}"),
                )
                .unwrap(),
            }
        } else if kind == "panda" {
            let EntityData::GeneralLiving(data) = entity.data() else {
                unreachable!()
            };
            let state = data.panda_retained().unwrap();
            let sound = if state.is_aggressive() {
                "minecraft:entity.panda.aggressive_ambient"
            } else if state.is_worried() {
                "minecraft:entity.panda.worried_ambient"
            } else {
                "minecraft:entity.panda.ambient"
            };
            bbb_protocol::packets::SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    sound,
                )
                .unwrap(),
            }
        } else {
            expected_animal_ambient(kind, baby)
        };
        let volume = if matches!(kind, "cow" | "mooshroom" | "allay" | "wolf") {
            0.4
        } else if matches!(kind, "horse" | "skeleton_horse") {
            0.8
        } else if kind == "happy_ghast" && !baby {
            4.0
        } else {
            1.0
        };
        let pitch = if kind == "happy_ghast" {
            1.0..=1.0
        } else if baby && kind == "zombie_villager" {
            1.8..=2.2
        } else if baby {
            1.3..=1.7
        } else {
            0.8..=1.2
        };
        let source = if matches!(
            kind,
            "enderman"
                | "guardian"
                | "elder_guardian"
                | "drowned"
                | "zombie_villager"
                | "zombified_piglin"
                | "zoglin"
                | "hoglin"
                | "creaking"
                | "piglin"
        ) {
            bbb_protocol::packets::SoundSource::Hostile
        } else {
            bbb_protocol::packets::SoundSource::Neutral
        };
        let alternate = (kind == "wolf").then(|| expected_wolf_sound(baby, "pant"));
        let permitted = |actual: &bbb_protocol::packets::SoundEventHolder| {
            actual == &expected
                || alternate.as_ref() == Some(actual)
                || (kind == "parrot"
                    && matches!(actual, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                        if crate::entity_runtime_facts_26_1::VANILLA_PARROT_IMITATION_SOUNDS.iter().any(|(_, name)|
                            bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id)==Some(*name))))
        };
        assert!(
            result.events.iter().any(|event| matches!(event,
                crate::world::WorldEvent::Send {
                    connection_id,
                    packet: crate::connection::ClientboundPlayPacket::Sound(sound),
                } if *connection_id == PLAYER
                    && permitted(&sound.sound)
                    && sound.source == source
                    && sound.volume == volume
                    && pitch.contains(&sound.pitch)
                    && sound.position == Vec3d { x: 8.5, y: 70.0, z: -4.5 }
            )),
            "missing {kind} ambient: {expected:?}"
        );
    }
    let warden = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|body| {
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID
                && body.position() == position
        })
        .expect("fresh summoned warden");
    assert!(warden.is_alive_with_health());
    assert_eq!(
        warden.pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_EMERGING_ID
    );
    for ambient in [
        "minecraft:entity.warden.ambient",
        "minecraft:entity.warden.agitated",
        "minecraft:entity.warden.angry",
    ] {
        let registry_id =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(ambient).unwrap();
        assert!(!result.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send { connection_id, packet: crate::connection::ClientboundPlayPacket::Sound(sound) }
                if *connection_id == PLAYER && sound.sound == bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    && sound.volume == 4.0
        )), "emerging warden emitted initial ambient {ambient}");
    }
    let turtle = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|body| {
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID
                && body.position() == position
        })
        .expect("fresh summoned turtle");
    assert!(turtle.is_alive_with_health() && !turtle.on_ground());
    let turtle_sound = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:entity.turtle.ambient_land",
        )
        .unwrap(),
    };
    assert!(
        !result.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send { connection_id,
                packet:crate::connection::ClientboundPlayPacket::Sound(sound) }
                if *connection_id==PLAYER && sound.sound==turtle_sound
        )),
        "fresh airborne turtle emitted land ambient"
    );
    let breeze = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|body| {
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID
                && body.position() == position
        })
        .expect("fresh summoned breeze");
    assert!(breeze.is_alive_with_health());
    for sound in [
        "minecraft:entity.breeze.idle_ground",
        "minecraft:entity.breeze.idle_air",
    ] {
        let sound_id =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(sound).unwrap();
        assert!(!result.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send {
                connection_id,
                packet: crate::connection::ClientboundPlayPacket::Sound(sound),
            } if *connection_id == PLAYER && matches!(sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if registry_id == sound_id)
        )), "server emitted client-local breeze ambient");
    }
    let shulker = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|body| {
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID
                && body.position() == position
        })
        .expect("fresh summoned shulker");
    assert!(matches!(shulker.data(), EntityData::GeneralLiving(data)
        if data.shulker_state().unwrap().is_closed()));
    let shulker_ambient = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.shulker.ambient",
    )
    .unwrap();
    assert!(
        !result.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send {
                connection_id,
                packet: crate::connection::ClientboundPlayPacket::Sound(sound),
            } if *connection_id == PLAYER && matches!(sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    if registry_id == shulker_ambient)
        )),
        "closed fresh shulker emitted ambient"
    );
    let bat = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .find(|body| {
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BAT_ID
                && body.position() == position
        })
        .expect("fresh summoned bat");
    assert!(bat.is_alive_with_health());
    let bat_sound = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:entity.bat.ambient",
        )
        .unwrap(),
    };
    for event in &result.events {
        if let crate::world::WorldEvent::Send {
            connection_id,
            packet: crate::connection::ClientboundPlayPacket::Sound(sound),
        } = event
        {
            if *connection_id == PLAYER && sound.sound == bat_sound {
                assert_eq!(sound.source, bbb_protocol::packets::SoundSource::Neutral);
                assert_eq!(sound.volume, 0.1);
                assert!((0.76..=1.14).contains(&sound.pitch));
                assert_eq!(
                    sound.position,
                    Vec3d {
                        x: 8.5,
                        y: 70.0,
                        z: -4.5
                    }
                );
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
