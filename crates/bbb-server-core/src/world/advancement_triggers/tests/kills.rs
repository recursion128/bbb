use super::*;

fn kill(world: &mut ServerWorld, victim: &str, killer: &str) {
    command(
        world,
        &format!("damage {victim} 40 minecraft:generic by {killer}"),
    );
}

/// `awardKillScore` fires one criterion on each side of the kill and neither on
/// the other: `Entity.awardKillScore` (`Entity.java:1931-1935`) offers the
/// dying session its own, and `ServerPlayer.awardKillScore` (`:955`) the killer.
#[test]
fn each_side_of_a_kill_is_offered_only_its_own_criterion() {
    let mut world = kill_world(&[
        (
            "test:killer",
            killed("minecraft:player_killed_entity", json!({})),
        ),
        (
            "test:victim",
            killed("minecraft:entity_killed_player", json!({})),
        ),
    ]);
    kill(&mut world, "victim_player", "inventory_player");
    assert_advancements(&world, CONNECTION, &["test:killer"], &["test:victim"]);
    assert_advancements(&world, VICTIM, &["test:victim"], &["test:killer"]);
}

/// `ServerPlayer.awardKillScore` is behind `victim != this` (`:942`) and calls
/// `super` inside it (`:943`), so a session that killed itself is offered
/// neither.
#[test]
fn a_session_that_killed_itself_is_offered_neither() {
    let mut world = kill_world(&[
        (
            "test:killer",
            killed("minecraft:player_killed_entity", json!({})),
        ),
        (
            "test:victim",
            killed("minecraft:entity_killed_player", json!({})),
        ),
    ]);
    kill(&mut world, "inventory_player", "inventory_player");
    assert_advancements(&world, CONNECTION, &[], &["test:killer", "test:victim"]);
}

/// A kill with nothing to credit it to never reaches `:915` at all
/// (`:912-913`), so the death alone offers neither side anything.
#[test]
fn a_death_without_kill_credit_offers_nothing() {
    let mut world = kill_world(&[(
        "test:victim",
        killed("minecraft:entity_killed_player", json!({})),
    )]);
    command(&mut world, "damage victim_player 40");
    assert!(!world.player_advancement_is_done(VICTIM, "test:victim"));
}

/// The four terms a kill's views now answer, on the real `/damage` kill path:
/// the killed entity's own type and position, the `LocationPredicate` its
/// position is tested against, and the type of the blow's direct slot.
#[test]
fn a_real_kill_answers_the_type_distance_location_and_direct_entity_terms() {
    let mut world = kill_world(&[
        (
            "test:typed",
            killed(
                "minecraft:player_killed_entity",
                json!({"entity":[this_entity_properties(json!({"type":"minecraft:player",
                    "location":{"block":{"nbt":"{id:'minecraft:chest',x:25,y:70,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
                }))]}),
            ),
        ),
        (
            "test:wrong_type",
            killed(
                "minecraft:player_killed_entity",
                json!({"entity":[this_entity_properties(json!({"type":"minecraft:zombie"}))]}),
            ),
        ),
        (
            "test:near",
            killed(
                "minecraft:player_killed_entity",
                json!({"entity":[this_entity_properties(json!({"distance":{"horizontal":{"max":10.0}}}))]}),
            ),
        ),
        (
            "test:far",
            killed(
                "minecraft:player_killed_entity",
                json!({"entity":[this_entity_properties(json!({"distance":{"horizontal":{"min":20.0}}}))]}),
            ),
        ),
        (
            "test:direct",
            killed(
                "minecraft:player_killed_entity",
                json!({"killing_blow":{"direct_entity":{"type":"minecraft:player",
                    "location":{"block":{"nbt":"{id:'minecraft:chest',x:0,y:70,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
                }}}),
            ),
        ),
    ]);
    command(&mut world, "tp inventory_player 0.5 70 0.5");
    let spot = world.levels[CONNECTION].position();
    assert_eq!(
        spot,
        Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5
        }
    );
    command(
        &mut world,
        &format!("tp victim_player {} {} {}", spot.x + 25.0, spot.y, spot.z),
    );
    assert_eq!(world.levels[VICTIM].position().x, spot.x + 25.0);

    diamond_chest(&mut world, BlockPos { x: 0, y: 70, z: 0 });
    diamond_chest(&mut world, BlockPos { x: 25, y: 70, z: 0 });

    kill(&mut world, "victim_player", "inventory_player");
    assert_advancements(
        &world,
        CONNECTION,
        &["test:typed", "test:far", "test:direct"],
        &["test:wrong_type", "test:near"],
    );
}

/// The same path with `location`, whose `position` half is the one
/// `LocationPredicate` term a kill's view reaches: the killed session's own
/// coordinates (`EntityPredicate.java:118-120`).
#[test]
fn a_real_kill_tests_the_killed_sessions_own_coordinates() {
    let mut world = kill_world(&[
        (
            "test:east",
            killed(
                "minecraft:player_killed_entity",
                json!({"entity":[this_entity_properties(json!({"location":{"position":{"x":{"min":1000.0}}}}))]}),
            ),
        ),
        (
            "test:west",
            killed(
                "minecraft:player_killed_entity",
                json!({"entity":[this_entity_properties(json!({"location":{"position":{"x":{"max":1000.0}}}}))]}),
            ),
        ),
    ]);
    let spot = world.levels[CONNECTION].position();
    command(
        &mut world,
        &format!("tp victim_player 1500.0 {} {}", spot.y, spot.z),
    );
    kill(&mut world, "victim_player", "inventory_player");
    assert_advancements(&world, CONNECTION, &["test:east"], &["test:west"]);
}

/// `DamageSourcePredicate.matches`' tag arm (`DamageSourcePredicate.java:33-37`)
/// over `TagPredicate.matches` (`TagPredicate.java:26-28`).
#[test]
fn a_killing_blow_tag_matches_the_damage_type_that_holds_it() {
    let criteria = |expected: bool| {
        killed(
            "minecraft:player_killed_entity",
            json!({"killing_blow":{"tags":[{"id":"minecraft:is_projectile","expected":expected}]}}),
        )
    };
    for (damage_type, done, not_done) in [
        ("minecraft:arrow", "test:projectile", "test:melee"),
        ("minecraft:generic", "test:melee", "test:projectile"),
    ] {
        let mut world = kill_world(&[
            ("test:projectile", criteria(true)),
            ("test:melee", criteria(false)),
        ]);
        kill_with(
            &mut world,
            "victim_player",
            damage_type,
            "inventory_player",
            None,
        );
        assert_advancements(&world, CONNECTION, &[done], &[not_done]);
    }
}

/// `:39` reads `getDirectEntity()` alone, which the `by` argument fills while
/// the `from` one fills the causing slot.
#[test]
fn a_killing_blow_direct_entity_reads_the_source_slot() {
    let criteria = killed(
        "minecraft:player_killed_entity",
        json!({"killing_blow":{"direct_entity":{"type_specific":{"type":"minecraft:sheep"}}}}),
    );
    for (mob_type, expected) in [
        (VANILLA_ENTITY_TYPE_SHEEP_ID, true),
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, false),
    ] {
        let mut world = kill_world(&[("test:direct", criteria.clone())]);
        let (_, source) = spawn_mob(&mut world, mob_type);
        kill_with(
            &mut world,
            "victim_player",
            "minecraft:generic",
            &source.to_string(),
            Some("inventory_player"),
        );
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:direct"),
            expected
        );
    }
}

/// `:42` reads `getEntity()`. `new DamageSource(type, entity)`
/// (`DamageSource.java:51-53`) puts the one entity in both slots.
#[test]
fn a_killing_blow_source_entity_reads_the_causing_slot() {
    let criteria = killed(
        "minecraft:entity_killed_player",
        json!({"killing_blow":{"source_entity":{"type_specific":{"type":"minecraft:sheep"}}}}),
    );
    for (mob_type, expected) in [
        (VANILLA_ENTITY_TYPE_SHEEP_ID, true),
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, false),
    ] {
        let mut world = kill_world(&[("test:source", criteria.clone())]);
        let (_, source) = spawn_mob(&mut world, mob_type);
        kill_with(
            &mut world,
            "victim_player",
            "minecraft:generic",
            &source.to_string(),
            None,
        );
        assert_eq!(
            world.player_advancement_is_done(VICTIM, "test:source"),
            expected
        );
    }
}

/// `:44` compares the two slots (`DamageSource.isDirect`,
/// `DamageSource.java:30-32`), which the two `by`/`from` shapes decide apart.
#[test]
fn a_killing_blow_is_direct_reads_both_slots() {
    let criteria = |expected: bool| {
        killed(
            "minecraft:player_killed_entity",
            json!({"killing_blow":{"is_direct":expected}}),
        )
    };
    for (separate_direct, done, not_done) in [
        (false, "test:direct", "test:indirect"),
        (true, "test:indirect", "test:direct"),
    ] {
        let mut world = kill_world(&[
            ("test:direct", criteria(true)),
            ("test:indirect", criteria(false)),
        ]);
        if separate_direct {
            let (_, source) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
            kill_with(
                &mut world,
                "victim_player",
                "minecraft:generic",
                &source.to_string(),
                Some("inventory_player"),
            );
        } else {
            kill_with(
                &mut world,
                "victim_player",
                "minecraft:generic",
                "inventory_player",
                None,
            );
        }
        assert_advancements(&world, CONNECTION, &[done], &[not_done]);
    }
}

/// `KilledTrigger.trigger`'s entity context (`:22`) binds a view of the other
/// side, so `Sheep.isSheared` (`Sheep.java:249-251`) decides the criterion.
#[test]
fn a_criterion_reading_the_killed_entity_sees_its_own_fields() {
    let criteria = |sheared: bool| {
        killed(
            "minecraft:entity_killed_player",
            json!({"entity":[this_entity_properties(json!({"type_specific":{"type":"minecraft:sheep","sheared":sheared}}))]}),
        )
    };
    for (mob_type, done, not_done) in [
        (
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &["test:unsheared"][..],
            &["test:sheared"][..],
        ),
        (
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            &[][..],
            &["test:unsheared", "test:sheared"][..],
        ),
    ] {
        let mut world = kill_world(&[
            ("test:unsheared", criteria(false)),
            ("test:sheared", criteria(true)),
        ]);
        let (_, source) = spawn_mob(&mut world, mob_type);
        kill_with(
            &mut world,
            "victim_player",
            "minecraft:generic",
            &source.to_string(),
            None,
        );
        assert_advancements(&world, VICTIM, done, not_done);
    }
}

/// The stored victim of a real kill as the entity the criterion reads
/// (`KilledTrigger.java:21-23`).
#[test]
fn a_stored_mobs_death_names_the_victim_and_the_slot_that_landed_the_blow() {
    let typed = |entity_type: &str| {
        killed(
            "minecraft:player_killed_entity",
            json!({"entity":[this_entity_properties(json!({"type":entity_type}))]}),
        )
    };
    let slotted = |slot: &str, item: Value| {
        killed(
            "minecraft:player_killed_entity",
            json!({"entity":[this_entity_properties(json!({"slots":{slot:item}}))]}),
        )
    };
    let mut world = world(&[
        ("test:zombie", typed("minecraft:zombie")),
        ("test:sheep", typed("minecraft:sheep")),
        (
            "test:armed",
            slotted(
                "weapon.mainhand",
                json!({
                    "items":"minecraft:diamond_sword",
                    "predicates":{
                        "minecraft:custom_data":{"loot_slot":"victim"},
                        "minecraft:enchantments":[{"enchantments":"minecraft:sharpness","levels":3}]
                    }
                }),
            ),
        ),
        (
            "test:other_weapon",
            slotted("weapon.mainhand", json!({"items":"minecraft:iron_sword"})),
        ),
        (
            "test:empty_offhand",
            slotted("weapon.offhand", json!({"count":0})),
        ),
        (
            "test:missing_inventory",
            slotted("container.0", json!({"count":0})),
        ),
        (
            "test:direct",
            killed(
                "minecraft:player_killed_entity",
                json!({"killing_blow":{"direct_entity":{"type":"minecraft:player"}}}),
            ),
        ),
        (
            "test:indirect",
            killed(
                "minecraft:player_killed_entity",
                json!({"killing_blow":{"is_direct":false}}),
            ),
        ),
    ]);
    let (_, victim) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    command(
        &mut world,
        &format!("item replace entity {victim} weapon.mainhand with diamond_sword[custom_data={{loot_slot:\"victim\"}},enchantments={{sharpness:3}}]"),
    );
    kill_with(
        &mut world,
        &victim.to_string(),
        "minecraft:generic",
        "inventory_player",
        None,
    );
    assert_advancements(
        &world,
        CONNECTION,
        &[
            "test:zombie",
            "test:armed",
            "test:empty_offhand",
            "test:direct",
        ],
        &[
            "test:sheep",
            "test:other_weapon",
            "test:missing_inventory",
            "test:indirect",
        ],
    );
}

/// `EntityPredicate.java:152`: the killed entity's `vehicle` term reads what
/// it rode.
#[test]
fn a_killed_entitys_vehicle_term_reads_what_it_rode() {
    let ridden = |vehicle: &str| {
        killed(
            "minecraft:player_killed_entity",
            json!({"entity":[this_entity_properties(json!({"vehicle":{"type":vehicle}}))]}),
        )
    };
    let mut world = world(&[
        ("test:minecart", ridden("minecraft:minecart")),
        ("test:boat", ridden("minecraft:oak_boat")),
    ]);
    let (_, victim) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    command(
        &mut world,
        "summon minecraft:minecart ~ ~ ~ {UUID:[I;0,0,0,4201],NoGravity:1b}",
    );
    command(
        &mut world,
        &format!("ride {victim} mount 00000000-0000-0000-0000-000000001069"),
    );
    kill_with(
        &mut world,
        &victim.to_string(),
        "minecraft:generic",
        "inventory_player",
        None,
    );
    assert_advancements(&world, CONNECTION, &["test:minecart"], &["test:boat"]);
}

#[test]
fn the_built_in_tame_criteria_all_compile_for_their_one_consumer() {
    let startup = built_in_startup();
    let tree = AdvancementTree::from_loaded(&startup.advancements);
    let compiled = AdvancementTriggers::compile(&tree, Some(startup));
    assert_eq!(
        compiled_criterion_ids(&tree, &compiled.tamed),
        [
            "minecraft:husbandry/complete_catalogue/minecraft:all_black",
            "minecraft:husbandry/complete_catalogue/minecraft:black",
            "minecraft:husbandry/complete_catalogue/minecraft:british_shorthair",
            "minecraft:husbandry/complete_catalogue/minecraft:calico",
            "minecraft:husbandry/complete_catalogue/minecraft:jellie",
            "minecraft:husbandry/complete_catalogue/minecraft:persian",
            "minecraft:husbandry/complete_catalogue/minecraft:ragdoll",
            "minecraft:husbandry/complete_catalogue/minecraft:red",
            "minecraft:husbandry/complete_catalogue/minecraft:siamese",
            "minecraft:husbandry/complete_catalogue/minecraft:tabby",
            "minecraft:husbandry/complete_catalogue/minecraft:white",
            "minecraft:husbandry/tame_an_animal/tamed_animal",
            "minecraft:husbandry/whole_pack/minecraft:ashen",
            "minecraft:husbandry/whole_pack/minecraft:black",
            "minecraft:husbandry/whole_pack/minecraft:chestnut",
            "minecraft:husbandry/whole_pack/minecraft:pale",
            "minecraft:husbandry/whole_pack/minecraft:rusty",
            "minecraft:husbandry/whole_pack/minecraft:snowy",
            "minecraft:husbandry/whole_pack/minecraft:spotted",
            "minecraft:husbandry/whole_pack/minecraft:striped",
            "minecraft:husbandry/whole_pack/minecraft:woods",
        ]
    );
}

#[test]
fn a_ghast_kill_in_the_overworld_completes_uneasy_alliance() {
    const ADVANCEMENT: &str = "minecraft:nether/uneasy_alliance";
    let mut world = built_in_world();
    player_loaded(&mut world);
    for (dimension, expected) in [
        (Java26BuiltinDimension::Nether, false),
        (Java26BuiltinDimension::Overworld, true),
    ] {
        teleport(&mut world, dimension, 0.0, 0.0);
        player_loaded(&mut world);
        let entity_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GHAST_ID;
        let mut data = GeneralLivingData::new(entity_type).unwrap();
        data.set_no_ai(true);
        let (id, uuid) = world.spawn_stored_entity_for_test(
            dimension,
            entity_type,
            world.levels[CONNECTION].position(),
            EntityData::GeneralLiving(data),
        );
        kill_with(
            &mut world,
            &uuid.to_string(),
            "minecraft:generic",
            "inventory_player",
            None,
        );
        assert_eq!(
            world
                .stored_entity_for_test(dimension, id)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            0.0
        );
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, ADVANCEMENT),
            expected
        );
    }
}

#[test]
fn stored_kill_fire_predicate_uses_live_item_tags_without_poisoning_other_terms() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    for members in [["minecraft:in_fire"], ["minecraft:lava"]] {
        let mut world = world(&[
            (
                "test:burning",
                killed(
                    "minecraft:player_killed_entity",
                    json!({"entity":[this_entity_properties(json!({"flags":{"is_on_fire":true}}))]}),
                ),
            ),
            (
                "test:typed",
                killed(
                    "minecraft:player_killed_entity",
                    json!({"entity":[this_entity_properties(json!({"type":"minecraft:item"}))]}),
                ),
            ),
        ]);
        let startup = Arc::make_mut(world.cold.config.datapacks.as_mut().unwrap());
        startup
            .tags
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:damage_type")
            .unwrap()
            .tags
            .insert(
                "test:item_fire".into(),
                members.iter().map(|member| (*member).into()).collect(),
            );
        let raw = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:stone".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:damage_resistant".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "types".into(),
                        value: NbtSummaryValue::String("#test:item_fire".into()),
                    }]),
                }]),
            },
        ]);
        let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &raw,
            &world.item_stack_resolver_for_test(),
        )
        .unwrap();
        let position = world.levels[CONNECTION].position();
        let (id, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
            position,
            EntityData::Item(crate::entity::ItemEntityData::new(stack, 0.0)),
        );
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
            .set_remaining_fire_ticks(40);
        let entity = world.killed_entity_of_stored(&Java26BuiltinDimension::Overworld.into(), id);
        world.trigger_killed(
            CONNECTION,
            "minecraft:player_killed_entity",
            entity,
            KillingBlow {
                damage_type: "minecraft:generic".into(),
                direct_entity: None,
                causing_entity: KillEntity::Null,
                is_direct: Some(true),
            },
            &mut Vec::new(),
        );
        assert!(world.player_advancement_is_done(CONNECTION, "test:typed"));
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:burning"),
            !members.contains(&"minecraft:in_fire")
        );
    }
}

#[test]
fn stored_kill_fire_predicate_keeps_the_uncharged_blaze_override() {
    let mut world = world(&[(
        "test:unlit",
        killed(
            "minecraft:player_killed_entity",
            json!({"entity":[this_entity_properties(json!({"flags":{"is_on_fire":false}}))]}),
        ),
    )]);
    let (id, _) = spawn_mob(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLAZE_ID,
    );
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
        .set_remaining_fire_ticks(40);
    let entity = world.killed_entity_of_stored(&Java26BuiltinDimension::Overworld.into(), id);
    world.trigger_killed(
        CONNECTION,
        "minecraft:player_killed_entity",
        entity,
        KillingBlow {
            damage_type: "minecraft:generic".into(),
            direct_entity: None,
            causing_entity: KillEntity::Null,
            is_direct: Some(true),
        },
        &mut Vec::new(),
    );
    assert!(world.player_advancement_is_done(CONNECTION, "test:unlit"));
}

#[test]
fn actual_item_pickup_fires_both_owner_criteria_with_the_restored_stack() {
    let criterion = |trigger, reward, nbt| {
        json!({
            "criteria": {"done": {"trigger": trigger, "conditions": {
                "player": {"nbt":nbt},
                "item": {"items": "minecraft:diamond", "count": {"min": 3}},
                "entity": [this_entity_properties(json!({"type":"minecraft:player"}))]
            }}}, "rewards": {"experience": reward}
        })
    };
    let mut world = world(&[
        (
            "test:owner",
            criterion(
                "minecraft:thrown_item_picked_up_by_entity",
                2,
                "{SelectedItem:{id:\"minecraft:stone\",count:1}}",
            ),
        ),
        (
            "test:picker",
            criterion(
                "minecraft:thrown_item_picked_up_by_player",
                3,
                "{SelectedItem:{id:\"minecraft:diamond\",count:3}}",
            ),
        ),
    ]);
    let owner = ConnectionId(2);
    let owner_uuid = Uuid::from_u128(2);
    world.join(
        owner,
        GameProfile {
            uuid: owner_uuid,
            name: "remote_owner".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    world
        .levels
        .move_player(owner, Java26BuiltinDimension::Nether.into());
    world.levels[owner].inventory.set_item_in_hand(
        InteractionHand::MainHand,
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    );
    give(&mut world, "minecraft:diamond", 3);
    let stack = world.levels[CONNECTION].inventory.slot(0).unwrap().clone();
    world.levels[CONNECTION]
        .inventory
        .clear_all_items(None)
        .unwrap();
    world.broadcast_player_menu(CONNECTION, MenuBroadcastMode::Changes, &mut Vec::new());
    let player = world.levels.player_at(CONNECTION);
    let dimension = player.dimension;
    let position = player.position();
    let item = world.level_mut_by_id(&dimension).entities.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        position,
        EntityData::Item(
            crate::entity::ItemEntityData::new(stack, 0.0).with_saved_fields(
                5,
                0,
                0,
                Some(owner_uuid),
                None,
            ),
        ),
    );
    world
        .load_entity_chunk_for_test(
            Java26BuiltinDimension::Overworld,
            (
                position.x.floor() as i32 >> 4,
                position.z.floor() as i32 >> 4,
            ),
        )
        .unwrap();
    world.tick_player_item_pickup(&dimension, &mut Vec::new());
    assert!(world.level_by_id(&dimension).entities.get(item).is_none());
    assert_eq!(
        world.levels[CONNECTION].inventory.slot(0).unwrap().count(),
        3
    );
    assert!(world.player_advancement_is_done(owner, "test:owner"));
    assert!(world.player_advancement_is_done(CONNECTION, "test:picker"));
    assert_eq!(xp(&world), 3);
    assert_eq!(world.levels[owner].experience.total(), 2);
}
