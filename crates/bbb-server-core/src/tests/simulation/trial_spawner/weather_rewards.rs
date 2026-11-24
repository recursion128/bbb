use super::*;

#[test]
fn world_predicates_can_prevent_ominous_dispensing() {
    let root =
        std::env::temp_dir().join(format!("bbb-trial-empty-weather-{}", uuid::Uuid::new_v4()));
    let pack = root.join("weather");
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Trial EMPTY weather","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    let table = json!({
        "type":"minecraft:empty",
        "pools":[{"rolls":1,"conditions":[
            {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"trial"},"range":1},
            {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
        ],"entries":[{"type":"minecraft:item","name":"minecraft:apple","functions":[{
            "function":"minecraft:set_custom_data","tag":"{trial_weather:1b}",
            "conditions":[{"condition":"minecraft:weather_check","raining":false}]
        }]}]}]
    });
    std::fs::write(
        pack.join("data/test/loot_table/ominous.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let mut config = dispensing_datapack_config();
    config.datapacks = Some(Arc::new(
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        })
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    for (score, clock) in [(0, 107), (1, 108)] {
        let (sender, receiver, mut world) = dispensing_world_with_config(config.clone(), true, 100);
        set_trial_world_predicate_state(&sender, &receiver, &mut world, score, clock);
        tick_many(&receiver, &mut world, 100);
        assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
        assert!(ominous_item_spawners(&world).is_empty());
        assert!(stored_item_stacks(&world).is_empty());
        assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 100);

        let raining = score == 1;
        let command = if raining {
            "weather rain 100000t"
        } else {
            "weather clear 100000t"
        };
        let changed =
            crate::tests::entities::console_command(&sender, &receiver, &mut world, 200, command);
        assert!(crate::tests::entities::console_command_answer(&changed)
            .0
            .is_some());
        for _ in 0..128 {
            if world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining()
                == raining
            {
                break;
            }
            world.tick(receiver.drain());
        }
        assert_eq!(
            world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining(),
            raining
        );
        for (operation, command) in [
            (201, "scoreboard players set #enabled trial 1"),
            (202, "time of minecraft:the_end set 107"),
        ] {
            let changed = crate::tests::entities::console_command(
                &sender, &receiver, &mut world, operation, command,
            );
            assert!(crate::tests::entities::console_command_answer(&changed)
                .0
                .is_some());
        }
        let mut output = None;
        for _ in 0..200 {
            output = world
                .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
                .into_iter()
                .find_map(|id| {
                    let entity =
                        world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
                    match entity.data() {
                        crate::entity::EntityData::OminousItemSpawner(data) => {
                            Some(data.item().clone())
                        }
                        crate::entity::EntityData::Item(data) => Some(data.item().clone()),
                        _ => None,
                    }
                });
            if output.is_some() {
                break;
            }
            world.tick(receiver.drain());
        }
        let output = output.expect("the active trial naturally dispenses its apple");
        assert_eq!(output.item_resource_id(), Some("minecraft:apple"));
        assert_eq!(output.count(), 1);
        assert_eq!(
            crate::tests::saved_item_component(&world, &output, "minecraft:custom_data"),
            (!raining).then(|| NbtSummaryValue::Compound(vec![entry(
                "trial_weather",
                NbtSummaryValue::Byte(1)
            )]))
        );
    }
}
