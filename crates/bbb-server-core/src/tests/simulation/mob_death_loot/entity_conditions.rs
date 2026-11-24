use super::*;

#[test]
fn entity_nbt_conditions_read_the_mob_save_and_online_killers_selected_item() {
    let table = serde_json::json!({"type":"minecraft:entity","pools":[
        {"rolls":1,"conditions":[{
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"nbt":"{Silent:1b}"}
        }],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[{
            "condition":"minecraft:entity_properties","entity":"attacking_player",
            "predicate":{"nbt":{"SelectedItem":{"id":"minecraft:stone"}}}
        }],"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]},
        {"rolls":1,"conditions":[{
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"team":"herd"}
        }],"entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]},
        {"rolls":1,"conditions":[{
            "condition":"minecraft:entity_properties","entity":"attacking_player",
            "predicate":{"team":"hunters"}
        }],"entries":[{"type":"minecraft:item","name":"minecraft:iron_ingot"}]}
    ]});
    for (silent, held, mob_team, player_team, expected) in [
        (
            true,
            Some("stone"),
            true,
            true,
            vec![
                "minecraft:diamond",
                "minecraft:emerald",
                "minecraft:gold_ingot",
                "minecraft:iron_ingot",
            ],
        ),
        (
            false,
            Some("stone"),
            true,
            false,
            vec!["minecraft:emerald", "minecraft:gold_ingot"],
        ),
        (
            true,
            Some("dirt"),
            false,
            true,
            vec!["minecraft:diamond", "minecraft:iron_ingot"],
        ),
        (false, None, false, false, vec![]),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = operator_config(&[("alice", 1)]);
        config.datapacks =
            datapack_config_with_loot_table("minecraft:entities/cow", &table).datapacks;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        if let Some(item) = held {
            console_command(
                &sender,
                &receiver,
                &mut world,
                99,
                &format!("give alice minecraft:{item}"),
            );
        }
        let killer = world.nearby_players_for_test(Java26BuiltinDimension::Overworld)[0].uuid;
        let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
        load_record(
            &mut world,
            cow,
            vec![
                nbt_entry("Silent", NbtSummaryValue::Byte(i8::from(silent))),
                nbt_entry("last_hurt_by_player", crate::tests::nbt_uuid(killer)),
                nbt_entry("last_hurt_by_player_memory_time", NbtSummaryValue::Int(100)),
            ],
        );
        for command in ["team add herd", "team add hunters"] {
            console_command(&sender, &receiver, &mut world, 99, command);
        }
        if mob_team {
            let uuid = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
                .unwrap()
                .uuid();
            console_command(
                &sender,
                &receiver,
                &mut world,
                99,
                &format!("team join herd {uuid}"),
            );
        }
        if player_team {
            console_command(
                &sender,
                &receiver,
                &mut world,
                99,
                "team join hunters alice",
            );
        }
        kill_stored(&mut world, cow);
        assert_eq!(
            dropped_ids(&world),
            expected,
            "silent={silent}, held={held:?}, mob_team={mob_team}, player_team={player_team}"
        );
    }
}
