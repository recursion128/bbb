//! Post-attacks an entity tick queues for the driver.

use crate::entity::projectile_action::{PendingProjectileAction, PendingProjectileHurt};
use crate::level_event::{PendingHurtEntity, PendingPlayerHurt, PendingPostAttack};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{ItemEnchantmentSummary, ItemStackSummary, Vec3d};
use bbb_server_types::save::Java26BuiltinDimension;

use crate::world::{ConnectionId, ServerWorld};

const PLAYER: ConnectionId = ConnectionId(1);
const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn world() -> ServerWorld {
    world_with_config(crate::tests::datapack_config())
}

fn world_with_config(mut config: crate::world::ServerWorldConfig) -> ServerWorld {
    config.game_type = bbb_protocol::packets::GameType::Survival;
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world
}

fn spawn(world: &mut ServerWorld, type_id: i32, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            type_id,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(type_id).unwrap(),
            ),
        )
        .0
}

fn enchanted(world: &ServerWorld, item_id: i32, enchantment: &str, level: i32) -> ItemStackSummary {
    let holder_id = world
        .datapack_tables
        .enchantment_registry
        .iter()
        .position(|id| id == enchantment)
        .unwrap() as i32;
    let mut stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    };
    stack
        .component_patch
        .enchantments
        .push(ItemEnchantmentSummary { holder_id, level });
    stack
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    stack
}

/// A zombie holding a Fire Aspect sword.
fn armed_zombie(world: &mut ServerWorld, position: Vec3d) -> i32 {
    let zombie = spawn(
        world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        position,
    );
    let sword = enchanted(
        world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    );
    let sword = Some(crate::tests::native_stack(sword));
    world
        .stored_entity_mut_for_test(OVERWORLD, zombie)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, sword);
    zombie
}

/// `Mob.doHurtTarget` (`Mob.java:1369`) on a session: the queued hurt lands,
/// then the zombie's Fire Aspect ignites the player.
#[test]
fn a_mob_hit_queued_on_a_session_runs_the_weapons_post_attack() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let zombie = armed_zombie(&mut world, position);
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    let uuid = world.levels[PLAYER].profile.uuid;
    world
        .level_mut(OVERWORLD)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player: uuid,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });

    world.tick(Vec::new());

    let player = world.levels.player_at(PLAYER);
    assert!(player.damage.health < 20.0);
    // `igniteForSeconds(4)` less at most the tick's own decrement.
    assert!(
        player.remaining_fire_ticks >= 79,
        "{}",
        player.remaining_fire_ticks
    );
}

/// The same obligation for a stored victim, whose hurt the entity tick has
/// already applied.
#[test]
fn a_mob_hit_queued_on_a_stored_victim_runs_the_weapons_post_attack() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let zombie = armed_zombie(&mut world, position);
    let cow = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            z: position.z + 1.5,
            ..position
        },
    );
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    world
        .level_mut(OVERWORLD)
        .event_sink
        .post_attack(PendingPostAttack {
            victim_entity_id: cow,
            damage_type: "minecraft:mob_attack",
            causing: Some(attacker.clone()),
            direct: Some(attacker),
        });

    world.tick(Vec::new());

    let fire = world
        .stored_entity_for_test(OVERWORLD, cow)
        .unwrap()
        .remaining_fire_ticks();
    assert!(fire >= 79, "{fire}");
}

/// A small fireball's accepted hit runs the victim's Thorns against its
/// shooter (`SmallFireball.java:41-44`); Thorns VII's chance is 1.05.
#[test]
fn a_small_fireball_hit_runs_the_victims_thorns_against_its_shooter() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let shooter = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: position.z + 4.0,
            ..position
        },
    );
    let chestplate = enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_CHESTPLATE_ID,
        "minecraft:thorns",
        7,
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    let chest = player
        .inventory
        .equipment_slots()
        .into_iter()
        .find(|(slot, _)| *slot == EquipmentSlot::Chest)
        .unwrap()
        .1;
    assert!(player
        .inventory
        .set_inventory_slot(chest, crate::tests::native_stack(chestplate)));
    let uuid = world.levels[PLAYER].profile.uuid;
    let shooter_entity = PendingHurtEntity::Stored { entity_id: shooter };
    world.queue_projectile_action_for_test(
        OVERWORLD,
        PendingProjectileAction::Hurt(PendingProjectileHurt {
            target: PendingHurtEntity::Player { uuid },
            damage_type: "minecraft:fireball",
            amount: 5.0,
            causing: Some(shooter_entity.clone()),
            direct: Some(shooter_entity),
            direct_resource_id: "minecraft:small_fireball",
            registry_tags: crate::entity::EntityRegistryTagView::Frozen(
                world.level_registry_tags_for_test(OVERWORLD),
            ),
        }),
    );

    world.tick(Vec::new());

    let health = world
        .stored_entity_for_test(OVERWORLD, shooter)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(health < 20.0, "{health}");
}

/// `SmallFireball.onHitEntity` (`SmallFireball.java:38-42`) sets the player
/// it hits alight for five seconds before the hurt.
#[test]
fn a_small_fireball_sets_the_player_it_hits_alight() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let blaze = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLAZE_ID,
        Vec3d {
            z: position.z + 4.0,
            ..position
        },
    );
    let uuid = world.levels[PLAYER].profile.uuid;
    let shooter = PendingHurtEntity::Stored { entity_id: blaze };
    world.queue_projectile_action_for_test(
        OVERWORLD,
        PendingProjectileAction::IgniteThenHurt(
            crate::entity::projectile_action::PendingProjectileIgniteHurt {
                hurt: PendingProjectileHurt {
                    target: PendingHurtEntity::Player { uuid },
                    damage_type: "minecraft:fireball",
                    amount: 5.0,
                    causing: Some(shooter.clone()),
                    direct: Some(shooter),
                    direct_resource_id: "minecraft:small_fireball",
                    registry_tags: crate::entity::EntityRegistryTagView::Frozen(
                        world.level_registry_tags_for_test(OVERWORLD),
                    ),
                },
                fire_ticks: 100,
                restore_fire_on_refusal: true,
            },
        ),
    );

    world.tick(Vec::new());

    let player = world.levels.player_at(PLAYER);
    assert!(player.damage.health < 20.0);
    assert!(
        player.remaining_fire_ticks >= 98,
        "{}",
        player.remaining_fire_ticks
    );
}

/// Fire Aspect's `ignite` effect lands through `LivingEntity.igniteForTicks`
/// (`LivingEntity.java:3925-3927`), which scales by `BURNING_TIME`.
#[test]
fn fire_aspect_burns_a_player_for_its_burning_time() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let zombie = armed_zombie(&mut world, position);
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .attributes
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BURNING_TIME_ID)
        .unwrap()
        .set_base_value(0.5);
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    let uuid = world.levels[PLAYER].profile.uuid;
    world
        .level_mut(OVERWORLD)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player: uuid,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });

    world.tick(Vec::new());

    let fire = world.levels.player_at(PLAYER).remaining_fire_ticks;
    assert!((38..=40).contains(&fire), "{fire}");
}

fn world_with_post_attack_effects(
    enchanted: &str,
    affected: &str,
    effects: serde_json::Value,
) -> (ServerWorld, std::path::PathBuf) {
    world_with_effect_components(serde_json::json!({"minecraft:post_attack": [{
        "enchanted": enchanted, "affected": affected,
        "effect": {"type": "minecraft:all_of", "effects": effects}
    }]}))
}

fn world_with_effect_components(effects: serde_json::Value) -> (ServerWorld, std::path::PathBuf) {
    world_with_effect_resources(effects, &[])
}

fn world_with_effect_resources(
    effects: serde_json::Value,
    resources: &[(&str, &str)],
) -> (ServerWorld, std::path::PathBuf) {
    let root =
        std::env::temp_dir().join(format!("bbb-session-hurt-order-{}", uuid::Uuid::new_v4()));
    let pack = root.join("order");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"hurt order fixture","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, body) in resources {
        let path = pack.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    let mut config = crate::tests::datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:fire_aspect")
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = effects;
    std::fs::write(
        pack.join("data/minecraft/enchantment/fire_aspect.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    config.datapacks = Some(std::sync::Arc::new(startup));
    (world_with_config(config), root)
}

#[test]
fn inline_sound_holders_reach_post_attack_and_worn_effect_packets() {
    let sound_effect = |id: &str, range: f32| {
        serde_json::json!({
            "type": "minecraft:play_sound", "sound": {"sound_id": id, "range": range},
            "volume": 1, "pitch": 1
        })
    };
    let (mut world, root) = world_with_effect_components(serde_json::json!({
        "minecraft:post_attack": [{"enchanted": "attacker", "affected": "victim",
            "effect": {"type": "minecraft:all_of", "effects": [
                sound_effect("minecraft:block.note_block.bell", 8.0),
                sound_effect("test:hit", 7.0)
            ]}}],
        "minecraft:location_changed": [{"effect": sound_effect("test:location", 6.0)}],
        "minecraft:tick": [{"effect": sound_effect("test:tick", 5.0)}]
    }));
    let position = world.levels[PLAYER].position();
    let zombie_position = Vec3d {
        x: position.x + 2.0,
        ..position
    };
    let zombie = armed_zombie(&mut world, zombie_position);
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    let uuid = world.levels[PLAYER].profile.uuid;
    world
        .level_mut(OVERWORLD)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player: uuid,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
    let result = world.tick(Vec::new());
    let sounds: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| match event {
            crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::Sound(sound),
                ..
            } => Some(sound),
            _ => None,
        })
        .collect();
    for (id, range) in [
        ("minecraft:block.note_block.bell", 8.0),
        ("test:hit", 7.0),
        ("test:location", 6.0),
        ("test:tick", 5.0),
    ] {
        assert!(
            sounds.iter().any(|sound| sound.sound
                == bbb_protocol::packets::SoundEventHolder::Direct {
                    location: id.into(),
                    fixed_range: Some(range),
                }),
            "missing {id}: {sounds:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_thorns_killed_attackers_weapon_drops_and_the_effects_still_run() {
    let (mut world, root) = world_with_post_attack_effects(
        "attacker",
        "attacker",
        serde_json::json!([
            {"type": "minecraft:change_item_damage", "amount": 4},
            {"type": "minecraft:play_sound", "sound": "minecraft:block.note_block.bell", "volume": 1, "pitch": 1}
        ]),
    );
    let position = world.levels[PLAYER].position();
    let zombie = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
    );
    world.load_stored_entity_record_for_test(
        OVERWORLD,
        zombie,
        &[bbb_protocol::NbtSummaryEntry {
            name: "drop_chances".into(),
            value: bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                name: "mainhand".into(),
                value: bbb_protocol::NbtSummaryValue::Float(2.0_f32.to_bits()),
            }]),
        }],
    );
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    let body = world.stored_entity_mut_for_test(OVERWORLD, zombie).unwrap();
    body.set_pos(Vec3d {
        x: position.x + 2.0,
        ..position
    });
    body.living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(sword));
    // Thorns deals `1 + nextInt(5)`, which the zombie's armour trims below 1.
    crate::entity::living::set_health(body, 0.5);
    let chestplate = enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_CHESTPLATE_ID,
        "minecraft:thorns",
        7,
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    let chest = player
        .inventory
        .equipment_slots()
        .into_iter()
        .find(|(slot, _)| *slot == EquipmentSlot::Chest)
        .unwrap()
        .1;
    assert!(player
        .inventory
        .set_inventory_slot(chest, crate::tests::native_stack(chestplate)));
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    let uuid = world.levels[PLAYER].profile.uuid;
    world
        .level_mut(OVERWORLD)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player: uuid,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
    let result = world.tick(Vec::new());
    let body = world.stored_entity_for_test(OVERWORLD, zombie).unwrap();
    assert_eq!(body.living().unwrap().health(), 0.0);
    assert!(body
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::MainHand)
        .is_none());
    let store = &world.level(OVERWORLD).entities;
    assert!(
        store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(id))
            .any(|body| matches!(body.data(),
                crate::entity::EntityData::Item(data)
                    if data
                        .item().item_resource_id().is_some_and(|item| item == "minecraft:iron_sword"))),
        "the sword was dropped"
    );
    assert!(result.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            if matches!(sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id) == Some("minecraft:block.note_block.bell")))));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_broken_armor_stands_equipment_drops_and_the_effects_still_run() {
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{"enchanted":"victim","affected":"victim","effect":{"type":"minecraft:all_of","effects":[
            {"type": "minecraft:damage_entity", "damage_type": "minecraft:explosion", "min_damage": 1, "max_damage": 1},
            {"type": "minecraft:change_item_damage", "amount": 4},
            {"type":"minecraft:run_function","function":"test:removed"},
            {"type": "minecraft:play_sound", "sound": "minecraft:block.note_block.bell", "volume": 1, "pitch": 1}
        ]}}]}),
        &[(
            "data/test/function/removed.mcfunction",
            "setblock ~ ~2 ~ minecraft:gold_block",
        )],
    );
    let position = world.levels[PLAYER].position();
    let (stand, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    world
        .stored_entity_mut_for_test(OVERWORLD, stand)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(sword));
    let player = PendingHurtEntity::Player {
        uuid: world.levels[PLAYER].profile.uuid,
    };
    world
        .level_mut(OVERWORLD)
        .event_sink
        .post_attack(PendingPostAttack {
            victim_entity_id: stand,
            damage_type: "minecraft:mob_attack",
            causing: Some(player.clone()),
            direct: Some(player),
        });
    let result = world.tick(Vec::new());
    assert!(world.stored_entity_for_test(OVERWORLD, stand).is_none());
    let gold = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:gold_block")
            .unwrap()
            .id,
    )
    .unwrap();
    assert_eq!(
        world.block_state(
            OVERWORLD,
            bbb_protocol::packets::BlockPos {
                x: (position.x + 2.0).floor() as i32,
                y: position.y.floor() as i32 + 2,
                z: position.z.floor() as i32,
            }
        ),
        Some(gold)
    );
    let store = &world.level(OVERWORLD).entities;
    assert!(
        store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(id))
            .any(|body| matches!(body.data(),
                crate::entity::EntityData::Item(data)
                    if data
                        .item().item_resource_id().is_some_and(|item| item == "minecraft:iron_sword"))),
        "the armor stand's sword was dropped"
    );
    assert!(result.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            if matches!(sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id) == Some("minecraft:block.note_block.bell")))));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_effects_apply_at_the_offset_for_post_attack_and_stored_location_changes() {
    use crate::level_event::VanillaGameEvent;
    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_property,
    };
    use bbb_protocol::packets::BlockPos;
    let effects = serde_json::json!([
        {"type":"minecraft:replace_block", "offset":[2,2,0],
         "predicate":{"type":"minecraft:matching_blocks", "blocks":"minecraft:stone"},
         "block_state":{"type":"minecraft:weighted_state_provider", "entries":[
             {"weight":1,"data":{"Name":"minecraft:furnace","Properties":{"facing":"north"}}},
             {"weight":1,"data":{"Name":"minecraft:furnace","Properties":{"facing":"east"}}}]},
         "trigger_game_event":"minecraft:block_place"},
        {"type":"minecraft:replace_block", "offset":[3,2,0],
         "predicate":{"type":"minecraft:matching_blocks", "blocks":"minecraft:dirt"},
         "block_state":{"type":"minecraft:simple_state_provider", "state":{"Name":"minecraft:gold_block"}},
         "trigger_game_event":"minecraft:block_place"},
        {"type":"minecraft:set_block_properties", "offset":[2,2,0],
         "properties":{"facing":"west", "lit":"invalid", "missing":"true"},
         "trigger_game_event":"minecraft:block_change"},
        {"type":"minecraft:set_block_properties", "offset":[2,2,0],
         "properties":{"facing":"west"}, "trigger_game_event":"minecraft:step"},
        {"type":"minecraft:set_block_properties", "offset":[4,2,0],
         "properties":{"facing":"south"}},
        {"type":"minecraft:replace_block", "offset":[5,2,0],
         "block_state":{"type":"minecraft:randomized_int_state_provider",
             "source":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:water_cauldron"}},
             "property":"level", "values":2}},
        {"type":"minecraft:replace_block", "offset":[0,0,0],
         "block_state":{"type":"minecraft:rule_based_state_provider", "rules":[
             {"if_true":{"type":"minecraft:unobstructed"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:gold_block"}}}]}},
        {"type":"minecraft:replace_block", "offset":[7,2,0],
         "block_state":{"type":"minecraft:rule_based_state_provider", "rules":[
             {"if_true":{"type":"minecraft:unobstructed"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:gold_block"}}}]}},
        {"type":"minecraft:replace_block", "offset":[0,6,0],
         "predicate":{"type":"minecraft:matching_fluids","fluids":"#test:wet"},
         "block_state":{"type":"minecraft:rule_based_state_provider", "rules":[
             {"if_true":{"type":"minecraft:matching_fluids","fluids":"#minecraft:water"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:iron_block"}}},
             {"if_true":{"type":"minecraft:matching_fluids","fluids":"#test:wet"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:gold_block"}}}]}},
        {"type":"minecraft:replace_block", "offset":[1,6,0],
         "predicate":{"type":"minecraft:matching_fluids","fluids":"#minecraft:water"},
         "block_state":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:gold_block"}}},
        {"type":"minecraft:replace_disk", "offset":[0,4,0], "radius":2, "height":1,
         "predicate":{"type":"minecraft:matching_blocks", "blocks":[
             "minecraft:stone","minecraft:dirt","minecraft:furnace"]},
         "block_state":{"type":"minecraft:rule_based_state_provider", "rules":[
             {"if_true":{"type":"minecraft:matching_blocks","blocks":"minecraft:stone"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:gold_block"}}},
             {"if_true":{"type":"minecraft:matching_blocks","blocks":"minecraft:dirt"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:iron_block"}}},
             {"if_true":{"type":"minecraft:matching_blocks","blocks":"minecraft:iron_block"},
              "then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:gold_block"}}}]}}
    ]);
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({
            "minecraft:post_attack":[{"enchanted":"attacker", "affected":"victim",
                "effect":{"type":"minecraft:all_of","effects":effects}}],
            "minecraft:location_changed":[{"effect":{"type":"minecraft:all_of","effects":effects}}]
        }),
        &[
            (
                "data/test/tags/fluid/wet.json",
                r#"{"values":["minecraft:water"]}"#,
            ),
            (
                "data/minecraft/tags/fluid/water.json",
                r#"{"replace":true,"values":[]}"#,
            ),
        ],
    );
    let player_position = world.levels[PLAYER].position();
    let zombie_position = Vec3d {
        x: player_position.x + 6.0,
        ..player_position
    };
    let zombie = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        zombie_position,
    );
    world.load_stored_entity_record_for_test(
        OVERWORLD,
        zombie,
        &[
            bbb_protocol::NbtSummaryEntry {
                name: "NoAI".into(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "OnGround".into(),
                value: bbb_protocol::NbtSummaryValue::Byte(1),
            },
        ],
    );
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    let body = world.stored_entity_mut_for_test(OVERWORLD, zombie).unwrap();
    body.set_pos(zombie_position);
    body.living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(sword));
    let stone = vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id,
    )
    .unwrap();
    let furnace = vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:furnace")
            .unwrap()
            .id,
    )
    .unwrap();
    let default_state = |name| {
        vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
                .unwrap()
                .id,
        )
        .unwrap()
    };
    let dirt = default_state("minecraft:dirt");
    let gold = default_state("minecraft:gold_block");
    let iron = default_state("minecraft:iron_block");
    let water = default_state("minecraft:water");
    let at = |position: Vec3d, x: i32, y: i32| BlockPos {
        x: position.x.floor() as i32 + x,
        y: position.y.floor() as i32 + y,
        z: position.z.floor() as i32,
    };
    for position in [player_position, zombie_position] {
        world.set_block(OVERWORLD, at(position, 0, -1), stone, 2);
        world.set_block(
            OVERWORLD,
            at(position, 0, 0),
            default_state("minecraft:air"),
            2,
        );
        for x in 0..=1 {
            world.set_block(OVERWORLD, at(position, x, 6), water, 2);
        }
        for (x, state) in [(0, stone), (1, dirt), (-1, furnace)] {
            world.set_block(OVERWORLD, at(position, x, 4), state, 2);
        }
        let mut rejected = at(position, 0, 4);
        rejected.z += 1;
        world.set_block(OVERWORLD, rejected, iron, 2);
        world.set_block(OVERWORLD, at(position, 4, 2), furnace, 2);
        for x in [2, 3] {
            world.set_block(OVERWORLD, at(position, x, 2), stone, 2);
        }
    }
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
    world.tick(Vec::new());
    for (position, source) in [
        (player_position, world.levels[PLAYER].entity_id()),
        (zombie_position, zombie),
    ] {
        assert_eq!(
            world.block_state(OVERWORLD, at(position, 0, 0)),
            Some(default_state("minecraft:air"))
        );
        assert_eq!(world.block_state(OVERWORLD, at(position, 7, 2)), Some(gold));
        assert_eq!(world.block_state(OVERWORLD, at(position, 0, 6)), Some(gold));
        assert_eq!(
            world.block_state(OVERWORLD, at(position, 1, 6)),
            Some(water)
        );
        assert_eq!(world.block_state(OVERWORLD, at(position, 0, 4)), Some(gold));
        assert_eq!(world.block_state(OVERWORLD, at(position, 1, 4)), Some(iron));
        assert_eq!(
            world.block_state(OVERWORLD, at(position, -1, 4)),
            Some(furnace)
        );
        let mut rejected = at(position, 0, 4);
        rejected.z += 1;
        assert_eq!(world.block_state(OVERWORLD, rejected), Some(iron));
        assert_eq!(
            vanilla_block_state_property(
                world.block_state(OVERWORLD, at(position, 5, 2)).unwrap(),
                "level"
            ),
            Some("2")
        );
        let pos = at(position, 2, 2);
        let state = world.block_state(OVERWORLD, pos).unwrap();
        assert_eq!(vanilla_block_state_property(state, "facing"), Some("west"));
        assert_eq!(vanilla_block_state_property(state, "lit"), Some("false"));
        assert_eq!(
            world.block_state(OVERWORLD, at(position, 3, 2)),
            Some(stone)
        );
        let raised = world
            .game_events(OVERWORLD)
            .0
            .into_iter()
            .filter(|event| event.source_entity_id == Some(source) && event.pos == pos)
            .map(|event| event.game_event)
            .collect::<Vec<_>>();
        assert!(raised.contains(&VanillaGameEvent::BLOCK_PLACE));
        assert!(raised.contains(&VanillaGameEvent::BLOCK_CHANGE));
        assert!(!raised.contains(&VanillaGameEvent::STEP));
        let silent = at(position, 4, 2);
        assert_eq!(
            vanilla_block_state_property(world.block_state(OVERWORLD, silent).unwrap(), "facing"),
            Some("south")
        );
        assert!(!world
            .game_events(OVERWORLD)
            .0
            .iter()
            .any(|event| event.source_entity_id == Some(source) && event.pos == silent));
        assert!(
            !world
                .game_events(OVERWORLD)
                .0
                .iter()
                .any(|event| event.source_entity_id == Some(source)
                    && event.pos == at(position, 3, 2))
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_attack_functions_use_the_affected_entity_and_effect_origin() {
    use crate::commands::{CommandPermission, CommandSource, ResultCallback};
    use bbb_protocol::packets::BlockPos;
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{
            "enchanted":"attacker", "affected":"victim", "effect":{
                "type":"minecraft:all_of", "effects":[
                    {"type":"minecraft:run_function", "function":"test:missing"},
                    {"type":"minecraft:run_function", "function":"test:macro"},
                    {"type":"minecraft:run_function", "function":"test:hit"},
                    {"type":"minecraft:ignite", "duration":4}
                ]
            }
        }]}),
        &[
            (
                "data/test/function/hit.mcfunction",
                "effect give @s minecraft:glowing 30 0 true\nsetblock ^ ^2 ^2 minecraft:gold_block",
            ),
            (
                "data/test/function/macro.mcfunction",
                "$setblock ~ ~3 ~ $(block)",
            ),
        ],
    );
    let mut teleport_events = Vec::new();
    world.perform_command(
        bbb_protocol::JavaString::with_utf16(
            "execute in minecraft:the_nether run tp alice 32.5 80 32.5 90 0",
            |units| std::sync::Arc::from(units),
        ),
        &CommandSource::server("Server", CommandPermission::Owner),
        &ServerWorld::console_origin(),
        ResultCallback::Empty,
        None,
        &mut teleport_events,
    );
    let teleport_id = teleport_events
        .iter()
        .find_map(|event| match event {
            crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    for packet in [
        bbb_protocol::packets::PlayServerbound::AcceptTeleportation { id: teleport_id },
        bbb_protocol::packets::PlayServerbound::PlayerLoaded,
    ] {
        world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            crate::world::WorldCommand::Play {
                connection_id: PLAYER,
                packet,
            },
        )]);
    }
    let dimension = Java26BuiltinDimension::Nether;
    assert_eq!(
        world.levels[PLAYER].dimension,
        world.require_level_id(&dimension.into())
    );
    let position = world.levels[PLAYER].position();
    let (zombie, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: position.x + 3.0,
            ..position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            )
            .unwrap(),
        ),
    );
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    world
        .stored_entity_mut_for_test(dimension, zombie)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(sword));
    let uuid = world.levels[PLAYER].profile.uuid;
    let attacker = PendingHurtEntity::Stored { entity_id: zombie };
    world
        .level_mut(dimension)
        .event_sink
        .hurt_player(PendingPlayerHurt {
            player: uuid,
            damage_type: "minecraft:mob_attack",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
    world.tick(Vec::new());
    assert!(world.levels[PLAYER]
        .active_effect("minecraft:glowing")
        .is_some());
    assert!(world.levels[PLAYER].remaining_fire_ticks >= 79);
    let gold = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:gold_block")
            .unwrap()
            .id,
    )
    .unwrap();
    assert_eq!(
        world.block_state(
            dimension,
            BlockPos {
                x: 30,
                y: 82,
                z: 32
            }
        ),
        Some(gold)
    );
    assert_ne!(
        world.block_state(
            OVERWORLD,
            BlockPos {
                x: 30,
                y: 82,
                z: 32
            }
        ),
        Some(gold)
    );
    std::fs::remove_dir_all(root).unwrap();
}

mod explosion_options;

mod summon_entities;

/// `RunFunction.apply` runs as the affected entity, so `say` names it by
/// `Entity.getDisplayName` (Entity.java:3207-3209), team prefix included.
#[test]
fn a_run_function_say_names_the_affected_entity_by_its_display_name() {
    use crate::commands::{CommandPermission, CommandSource, ResultCallback};
    let (mut world, root) = world_with_effect_resources(
        serde_json::json!({"minecraft:post_attack":[{"enchanted":"victim","affected":"victim",
            "effect":{"type":"minecraft:run_function","function":"test:say"}}]}),
        &[("data/test/function/say.mcfunction", "say hello")],
    );
    let position = world.levels[PLAYER].position();
    let (stand, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let uuid = world
        .stored_entity_for_test(OVERWORLD, stand)
        .unwrap()
        .uuid();
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:fire_aspect",
        1,
    ));
    world
        .stored_entity_mut_for_test(OVERWORLD, stand)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(sword));
    let mut events = Vec::new();
    for command in [
        "team add red".to_owned(),
        "team modify red prefix \"[R] \"".to_owned(),
        format!("team join red {uuid}"),
    ] {
        world.perform_command(
            bbb_protocol::JavaString::with_utf16(&command, |units| std::sync::Arc::from(units)),
            &CommandSource::server("Server", CommandPermission::Owner),
            &ServerWorld::console_origin(),
            ResultCallback::Empty,
            None,
            &mut events,
        );
    }
    let player = PendingHurtEntity::Player {
        uuid: world.levels[PLAYER].profile.uuid,
    };
    world
        .level_mut(OVERWORLD)
        .event_sink
        .post_attack(PendingPostAttack {
            victim_entity_id: stand,
            damage_type: "minecraft:mob_attack",
            causing: Some(player.clone()),
            direct: Some(player),
        });
    let result = world.tick(Vec::new());
    let said: Vec<_> = crate::tests::chat_lines(&world, &result)
        .into_iter()
        .filter(|(_, line)| line.contains("chat.type.announcement"))
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].1.contains("[R] ") && said[0].1.contains("entity.minecraft.armor_stand"),
        "{said:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// `Player.attack` (`Player.java:1002`) invokes `postPiercingAttack` on normal
/// melee hits and even when the hurt returns false (invulnerable target).
#[test]
fn player_melee_attack_runs_post_piercing_attack() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let cow = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: position.x + 1.5,
            ..position
        },
    );
    let sword = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID,
        "minecraft:lunge",
        1,
    ));
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = sword;
    // Make the cow invulnerable so `hurtOrSimulate` answers NotHurt, yet
    // `Player.attack` still runs `postPiercingAttack` (`Player.java:961-1002`).
    world
        .stored_entity_mut_for_test(OVERWORLD, cow)
        .unwrap()
        .set_invulnerable(true);
    let exhaustion_before = world.levels[PLAYER].food.exhaustion_level();
    let mut events = Vec::new();
    world
        .player_melee_attack(
            PLAYER,
            crate::world::combat::player_melee::MeleeTarget::Stored(cow),
            &crate::entity::EntityRegistryTagView::Builtin26_1,
            &mut events,
        )
        .unwrap();
    assert!(
        world.levels[PLAYER].food.exhaustion_level() > exhaustion_before,
        "lunge post_piercing_attack must consume food exhaustion on melee attack"
    );
    let has_lunge_sound = events.iter().any(|event| match event {
        crate::world::WorldEvent::Send {
            packet: crate::connection::ClientboundPlayPacket::Sound(sound),
            ..
        } => match &sound.sound {
            bbb_protocol::packets::SoundEventHolder::Reference { registry_id } => {
                bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id)
                    == Some("minecraft:item.spear.lunge_1")
            }
            bbb_protocol::packets::SoundEventHolder::Direct { location, .. } => {
                location == "minecraft:item.spear.lunge_1"
            }
        },
        _ => false,
    });
    assert!(has_lunge_sound, "expected lunge sound from melee attack");
}

/// `Player.getWeaponItem` (`Player.java:521-522`, `:954`): while riptide
/// spinning with an off-hand trident, melee uses the spin trident's
/// enchantments and wears the originating off-hand slot rather than main-hand.
#[test]
fn riptide_spin_attack_uses_auto_spin_weapon_enchantments_and_durability() {
    let mut world = world();
    let position = world.levels[PLAYER].position();
    let cow = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: position.x + 1.0,
            ..position
        },
    );
    let trident = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_TRIDENT_ID,
        "minecraft:fire_aspect",
        1,
    ));
    let plain_sword = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID),
        count: 1,
        component_patch: Default::default(),
    });
    let player = world.levels.player_mut(PLAYER).unwrap();
    *player.inventory.selected_item_mut() = plain_sword;
    player.inventory.set_item_in_hand(
        bbb_protocol::packets::InteractionHand::OffHand,
        trident.clone(),
    );
    world.start_using_item(PLAYER, bbb_protocol::packets::InteractionHand::OffHand);
    world.stop_using_item(PLAYER);
    world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .using_item
        .start_auto_spin_attack(20, 8.0, trident);

    let mut events = Vec::new();
    world
        .player_melee_attack(
            PLAYER,
            crate::world::combat::player_melee::MeleeTarget::Stored(cow),
            &crate::entity::EntityRegistryTagView::Builtin26_1,
            &mut events,
        )
        .unwrap();

    let cow_entity = world.stored_entity_for_test(OVERWORLD, cow).unwrap();
    assert!(
        cow_entity.remaining_fire_ticks() > 0,
        "spin trident's Fire Aspect must ignite the target"
    );
    let main_hand = world.levels[PLAYER].inventory.selected_stack().summary();
    assert_eq!(
        crate::item_durability::damage_and_max(main_hand).map(|(d, _)| d),
        Some(0),
        "main-hand sword must not take spin-attack wear"
    );
    let off_hand = world.levels[PLAYER].inventory.off_hand_stack().summary();
    assert_eq!(
        crate::item_durability::damage_and_max(off_hand).map(|(d, _)| d),
        Some(1),
        "off-hand spin trident must take 1 durability wear"
    );
}

/// `EnchantmentHelper.doPostAttackEffectsWithItemSourceOnBreak`
/// (`EnchantmentHelper.java:247-261`, `ThrownTrident.java:135-137`,
/// `AbstractArrow.java:475`): projectile post-attack `damage_item` wears the
/// projectile's own weapon (not the player's held item) and kills a dispenser
/// trident when its weapon breaks without a living attacker.
#[test]
fn projectile_weapon_post_attack_damages_projectile_and_kills_attackerless_trident() {
    let (mut world, root) = world_with_post_attack_effects(
        "attacker",
        "victim",
        serde_json::json!([{"type": "minecraft:change_item_damage", "amount": 500}]),
    );
    let position = world.levels[PLAYER].position();

    // 1. Player-owned arrow: breaking `firedFromWeapon` clears the arrow's
    // weapon and leaves the player's currently held sword untouched.
    let sword = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID),
        count: 1,
        component_patch: Default::default(),
    });
    *world
        .levels
        .player_mut(PLAYER)
        .unwrap()
        .inventory
        .selected_item_mut() = sword;
    let bow = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_BOW_ID,
        "minecraft:fire_aspect",
        1,
    ));
    let mut arrow_data =
        crate::entity::ArrowData::new(crate::tests::native_stack(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_ARROW_ID),
            count: 1,
            component_patch: Default::default(),
        }));
    arrow_data
        .projectile
        .set_owner(Some(world.levels[PLAYER].profile.uuid));
    arrow_data.set_weapon(Some(bow.clone()));
    let arrow_id = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
            position,
            crate::entity::EntityData::Arrow(arrow_data),
        )
        .0;
    let cow = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
    );
    let level_id = world.levels[PLAYER].dimension;
    let damage_types = std::sync::Arc::clone(&world.cold.damage_types);
    let arrow_damage = damage_types
        .iter()
        .find(|entry| entry.resource_id == "minecraft:arrow")
        .unwrap();
    let mut events = Vec::new();
    world
        .projectile_weapon_post_attack(
            &level_id,
            crate::world::combat::player_melee::MeleeTarget::Stored(cow),
            crate::world::combat::player_damage_reduction::PlayerHurtSource {
                damage_type: arrow_damage,
                causing: Some(
                    crate::world::combat::enchanted_damage::SourceEntity::Session(PLAYER),
                ),
                direct: Some(
                    crate::world::combat::enchanted_damage::SourceEntity::Stored(arrow_id),
                ),
            },
            arrow_id,
            Some(&bow),
            false,
            &mut events,
        )
        .unwrap();
    assert_eq!(
        crate::item_durability::damage_and_max(
            world.levels[PLAYER].inventory.selected_stack().summary()
        )
        .map(|(d, _)| d),
        Some(0),
        "player's currently held sword must not be damaged by arrow weapon break"
    );
    let arrow_weapon = world
        .stored_entity_for_test(OVERWORLD, arrow_id)
        .and_then(|e| crate::entity::abstract_arrow::state(e.data()))
        .and_then(|s| s.weapon());
    assert!(
        arrow_weapon.is_none(),
        "broken arrow weapon must be cleared"
    );

    // 2. Dispenser-shot (attackerless) trident: breaking its trident item
    // invokes `this.kill(serverLevel)`.
    let trident_item = crate::tests::native_stack(enchanted(
        &world,
        bbb_protocol::item_types::VANILLA_ITEM_TRIDENT_ID,
        "minecraft:fire_aspect",
        1,
    ));
    let trident_id = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRIDENT_ID,
            position,
            crate::entity::EntityData::Trident(crate::entity::TridentData::new(
                trident_item.clone(),
            )),
        )
        .0;
    let trident_damage = damage_types
        .iter()
        .find(|entry| entry.resource_id == "minecraft:trident")
        .unwrap();
    world
        .projectile_weapon_post_attack(
            &level_id,
            crate::world::combat::player_melee::MeleeTarget::Stored(cow),
            crate::world::combat::player_damage_reduction::PlayerHurtSource {
                damage_type: trident_damage,
                causing: Some(
                    crate::world::combat::enchanted_damage::SourceEntity::Stored(trident_id),
                ),
                direct: Some(
                    crate::world::combat::enchanted_damage::SourceEntity::Stored(trident_id),
                ),
            },
            trident_id,
            Some(&trident_item),
            true,
            &mut events,
        )
        .unwrap();
    assert!(
        world
            .stored_entity_for_test(OVERWORLD, trident_id)
            .is_none(),
        "attackerless dispenser trident must be killed when its weapon breaks"
    );

    // 3. Player-thrown trident: breaking its item shrinks `pickupItemStack` to
    // empty (`ThrownTrident.java:135-137`, `AbstractArrow.java:475`) so walking
    // over or Loyalty-returning it cannot hand back an unbroken trident.
    let mut owned_trident = crate::entity::TridentData::new(trident_item.clone());
    owned_trident
        .projectile
        .set_owner(Some(world.levels[PLAYER].profile.uuid));
    let owned_trident_id = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRIDENT_ID,
            position,
            crate::entity::EntityData::Trident(owned_trident),
        )
        .0;
    world
        .projectile_weapon_post_attack(
            &level_id,
            crate::world::combat::player_melee::MeleeTarget::Stored(cow),
            crate::world::combat::player_damage_reduction::PlayerHurtSource {
                damage_type: trident_damage,
                causing: Some(
                    crate::world::combat::enchanted_damage::SourceEntity::Session(PLAYER),
                ),
                direct: Some(
                    crate::world::combat::enchanted_damage::SourceEntity::Stored(owned_trident_id),
                ),
            },
            owned_trident_id,
            Some(&trident_item),
            true,
            &mut events,
        )
        .unwrap();
    let stored_trident_item = world
        .stored_entity_for_test(OVERWORLD, owned_trident_id)
        .and_then(|e| crate::entity::abstract_arrow::state(e.data()))
        .map(|s| s.item());
    assert!(
        stored_trident_item.is_some_and(bbb_server_types::item_stack::RuntimeItemStack::is_empty),
        "broken player-thrown trident must clear its stored pickup item to empty"
    );
    std::fs::remove_dir_all(root).unwrap();
}
