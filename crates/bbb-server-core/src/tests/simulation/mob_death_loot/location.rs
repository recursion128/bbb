use super::*;

#[test]
fn death_loot_reads_current_block_nbt_at_the_offset_mob_origin() {
    let table = serde_json::json!({
        "type": "minecraft:entity",
        "pools": [{"rolls": 1, "conditions": [{
            "condition": "minecraft:location_check", "offsetX": 1,
            "predicate": {"block": {"nbt": "{id:'minecraft:chest',x:1,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
        }, {
            "condition": "minecraft:time_check", "clock": "minecraft:the_end",
            "period": 100, "value": 7
        }], "entries": [{
            "type": "minecraft:item", "name": "minecraft:emerald",
            "functions": [{"function": "minecraft:set_count", "count": 2}]
        }]}]
    });
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        "minecraft:entities/iron_golem",
        &table,
    ));
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    let placed = console_command(&sender, &receiver, &mut world, 1, "setblock 1 64 0 chest");
    assert_eq!(console_command_answer(&placed).0, Some(1));
    let end_clock = world
        .synchronized_registries()
        .registry_element_index("minecraft:world_clock", "minecraft:the_end")
        .unwrap() as usize;
    for (index, (count, clock_ticks, expected)) in
        [(3, 107, 1), (4, 107, 1), (3, 108, 1), (3, 107, 2)]
            .into_iter()
            .enumerate()
    {
        let replaced = console_command(
            &sender,
            &receiver,
            &mut world,
            index as u64 + 2,
            &format!("item replace block 1 64 0 container.0 with diamond {count}"),
        );
        assert_eq!(console_command_answer(&replaced).0, Some(1));
        let golem = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
        stage_one_blow_from_death(&mut world, golem);
        world.set_world_clock_state_for_test(
            end_clock,
            bbb_server_persistence::Java26WorldClockState {
                total_ticks: clock_ticks,
                paused: true,
                ..Default::default()
            },
        );
        kill_stored(&mut world, golem);

        let dropped = items(&world);
        assert_eq!(
            dropped.len(),
            expected,
            "stored count {count}, clock {clock_ticks}: {dropped:?}"
        );
        assert_eq!(
            (dropped[0].2.as_str(), dropped[0].3),
            ("minecraft:emerald", 2)
        );
    }
}

/// `EntityPredicate`'s two position terms over `THIS_ENTITY` on a real mob
/// death: `distance` measures from the context origin to `entity.getX/getY/getZ`
/// and `location` is a `LocationPredicate`
/// tested at that same point.
///
/// The dying mob is moved between the two cases, so a view answering a constant
/// could not take both `location` arms: the `y` band each case passes is the
/// band around where the mob actually stood. `is_baby` rides along as the one
/// flag an entity table can spell both ways -- `LivingEntity.shouldDropLoot`
/// refuses a baby's table whole, so the true arm
/// is unreachable by construction and the false arm is the live one.
#[test]
fn a_mob_tables_distance_and_location_terms_read_the_dying_mobs_own_position() {
    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
        vanilla_block_state_identity,
    };
    let this_entity = |predicate: serde_json::Value| {
        serde_json::json!({
            "condition": "minecraft:entity_properties",
            "entity": "this",
            "predicate": predicate,
        })
    };
    let pool = |predicate: serde_json::Value, item: &str| {
        serde_json::json!({
            "rolls": 1.0,
            "conditions": [this_entity(predicate)],
            "entries": [{"type": "minecraft:item", "name": item}],
        })
    };
    let table = serde_json::json!({
        "type": "minecraft:entity",
        "pools": [
            pool(
                serde_json::json!({"distance": {"absolute": {"max": 0.5}}}),
                "minecraft:diamond",
            ),
            pool(
                serde_json::json!({"distance": {"absolute": {"min": 1.0}}}),
                "minecraft:emerald",
            ),
            pool(
                serde_json::json!({"location": {"position": {"y": {"min": 63.0, "max": 65.0}}}}),
                "minecraft:gold_ingot",
            ),
            pool(
                serde_json::json!({"location": {"position": {"y": {"min": 69.0, "max": 71.0}}}}),
                "minecraft:iron_ingot",
            ),
            pool(
                serde_json::json!({"flags": {"is_baby": false}}),
                "minecraft:redstone",
            ),
            pool(
                serde_json::json!({"flags": {"is_baby": true}}),
                "minecraft:coal",
            ),
            pool(
                serde_json::json!({"location":{"fluid":{"state":{"level":{"min":"4","max":"6"}}}}}),
                "minecraft:lapis_lazuli",
            ),
            pool(
                serde_json::json!({"location":{"fluid":{"state":{"level":{"min":"8"}}}}}),
                "minecraft:quartz",
            ),
            pool(
                serde_json::json!({"location":{"fluid":{"state":{"falling":{"max":"false"}}}}}),
                "minecraft:amethyst_shard",
            ),
            pool(
                serde_json::json!({"location":{"fluid":{"fluids":"#test:source_only"}}}),
                "minecraft:copper_ingot",
            ),
            {
                "rolls":1,
                "conditions":[{"condition":"minecraft:location_check","predicate":{"fluid":{"fluids":"#minecraft:water"}}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:prismarine_shard"}]
            },
        ],
    });
    for (y, water_level, publish_tags, expected) in [
        (
            64.0,
            "3",
            true,
            vec![
                "minecraft:amethyst_shard".to_owned(),
                "minecraft:diamond".to_owned(),
                "minecraft:gold_ingot".to_owned(),
                "minecraft:lapis_lazuli".to_owned(),
                "minecraft:prismarine_shard".to_owned(),
                "minecraft:redstone".to_owned(),
            ],
        ),
        (
            70.0,
            "0",
            true,
            vec![
                "minecraft:amethyst_shard".to_owned(),
                "minecraft:copper_ingot".to_owned(),
                "minecraft:diamond".to_owned(),
                "minecraft:iron_ingot".to_owned(),
                "minecraft:redstone".to_owned(),
            ],
        ),
        (
            70.0,
            "0",
            false,
            vec![
                "minecraft:amethyst_shard".to_owned(),
                "minecraft:diamond".to_owned(),
                "minecraft:iron_ingot".to_owned(),
                "minecraft:redstone".to_owned(),
            ],
        ),
    ] {
        let mut config = datapack_config_with_loot_table("minecraft:entities/iron_golem", &table);
        let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
        if publish_tags {
            let tags = &mut startup
                .tags
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:fluid")
                .unwrap()
                .tags;
            tags.insert("test:source_only".into(), vec!["minecraft:water".into()]);
            tags.insert(
                "minecraft:water".into(),
                vec!["minecraft:flowing_water".into()],
            );
        } else {
            startup
                .tags
                .registries
                .retain(|registry| registry.registry_id != "minecraft:fluid");
        }
        let mut world = spawn_pinned_world(config);
        let mut water = vanilla_block_state_identity(
            vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID)
                .unwrap(),
        )
        .unwrap()
        .clone();
        water.properties.insert("level".into(), water_level.into());
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos {
                y: y as i32,
                ..MOB_BLOCK
            },
            vanilla_block_state_id_by_name_and_properties(&water.name, &water.properties).unwrap(),
            SET_BLOCK_UPDATE_ALL,
        ));
        let golem = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, golem)
            .expect("the golem is stored")
            .set_pos(Vec3d {
                x: MOB_POSITION.x,
                y,
                z: MOB_POSITION.z,
            });
        stage_one_blow_from_death(&mut world, golem);

        kill_stored(&mut world, golem);

        let mut dropped = dropped_ids(&world);
        dropped.sort();
        let mut expected = expected;
        expected.sort();
        assert_eq!(dropped, expected, "at y={y}");
    }
}

#[test]
fn death_loot_environment_checks_sample_the_dying_mobs_origin_biome() {
    let table = serde_json::json!({"type": "minecraft:entity", "pools": [{
        "rolls": 1, "conditions": [{"condition": "minecraft:environment_attribute_check",
            "attribute": "minecraft:gameplay/water_evaporates", "value": true}],
        "entries": [{"type": "minecraft:item", "name": "minecraft:diamond"}]
    }]});
    let mut config = datapack_config_with_loot_table("minecraft:entities/iron_golem", &table);
    let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, evaporates) in [("minecraft:plains", false), ("minecraft:desert", true)] {
        let biome = startup
            .registries
            .registries
            .iter_mut()
            .find(|r| r.registry_id == "minecraft:worldgen/biome")
            .unwrap()
            .elements
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&biome.source_json).unwrap();
        value["attributes"]["minecraft:gameplay/water_evaporates"] = serde_json::json!(evaporates);
        biome.source_json = serde_json::to_vec(&value).unwrap().into();
        biome.source = Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
                .unwrap(),
        );
        biome.codec_value = None;
    }
    let mut world = spawn_pinned_world(config);
    for (x, biome) in ["minecraft:plains", "minecraft:desert"]
        .into_iter()
        .enumerate()
    {
        assert!(world.set_noise_biome_for_test(
            Java26BuiltinDimension::Overworld,
            (x as i32, 16, 0),
            biome,
        ));
    }
    for (x, expected) in [(0.5, 0), (4.5, 1)] {
        let golem = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, golem)
            .unwrap()
            .set_pos(Vec3d { x, ..MOB_POSITION });
        stage_one_blow_from_death(&mut world, golem);
        kill_stored(&mut world, golem);
        assert_eq!(
            dropped_ids(&world),
            vec!["minecraft:diamond"; expected],
            "x={x}"
        );
    }
}
