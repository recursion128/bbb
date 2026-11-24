//! `/item` errors, source slots and modifiers over native inventories.
use super::*;
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_types::item_stack::RuntimeItemStack;

const POS: BlockPos = BlockPos { x: 1, y: 64, z: 1 };
fn chest_fixture(players: &[NameAndId]) -> CommandTestWorld {
    world(players).with_chest(POS)
}
fn diamond_id() -> i32 {
    bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID
}
fn diamonds(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(diamond_id()),
        count,
        component_patch: Default::default(),
    }
}

#[test]
fn item_replace_block_refuses_an_overstacked_count() {
    let run = ran(
        "item replace block 1 64 1 container.0 with diamond 65",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&run), ["arguments.item.overstacked"]);
}

#[test]
fn item_replace_block_refuses_a_non_container() {
    let run = ran(
        "item replace block 1 64 1 container.0 with diamond",
        CommandTestWorld::default().with_block(POS, 0),
    );
    assert_eq!(keys(&run), ["commands.item.target.not_a_container"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::Int(64),
            NbtSummaryValue::Int(1)
        ]
    );
}

#[test]
fn item_replace_block_refuses_a_slot_past_the_container() {
    let run = ran(
        "item replace block 1 64 1 weapon.mainhand with diamond",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&run), ["commands.item.target.no_such_slot"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(98)]);
}

#[test]
fn item_replace_block_refuses_an_unknown_slot_name() {
    let run = ran(
        "item replace block 1 64 1 not_a_slot with diamond",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&run), ["slot.unknown", CURSOR_CONTEXT]);
}

#[test]
fn item_replace_block_refuses_a_multi_slot_name() {
    let run = ran(
        "item replace block 1 64 1 container.* with diamond",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&run), ["slot.only_single_allowed", CURSOR_CONTEXT]);
}

#[test]
fn item_replace_from_block_refuses_a_non_container_source() {
    let run = ran(
        "item replace block 1 64 1 container.0 from block 9 9 9 container.0",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&run), ["commands.item.source.not_a_container"]);
}

#[test]
fn item_replace_entity_refuses_a_diamond_in_the_chest_slot() {
    let players = [user("alice", 1)];
    let run = ran(
        "item replace entity alice armor.chest with diamond",
        world(&players),
    );
    assert_eq!(keys(&run), ["commands.item.target.no_changed.known_item"]);
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Int(102));
}

#[test]
fn item_replace_entity_reports_the_entity_key_when_nobody_matches() {
    let run = ran(
        "item replace entity bob container.0 with diamond",
        world(&[]),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
}

#[test]
fn item_replace_from_entity_refuses_a_slot_without_a_store() {
    let players = [user("alice", 1)];
    let view = chest_fixture(&players);
    let run = ran(
        "item replace block 1 64 1 container.0 from entity alice mob.inventory.0",
        view,
    );
    assert_eq!(keys(&run), ["commands.item.source.no_such_slot"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(300)]);
}

#[test]
fn item_suggests_single_slot_names() {
    let offered = completions_for(
        "item replace block 1 64 1 cont",
        CommandPermission::Owner,
        chest_fixture(&[]),
    );
    assert!(
        texts(&offered).iter().any(|text| text == "container.0"),
        "{offered:?}"
    );
    assert!(
        texts(&offered).iter().all(|text| !text.ends_with('*')),
        "{offered:?}"
    );
}

#[test]
fn item_is_published_among_the_implemented_roots() {
    assert!(catalog::IMPLEMENTED_COMMANDS.contains(&"item"));
    assert_eq!(catalog::IMPLEMENTED_COMMANDS.len(), 87);
}

#[test]
fn item_is_hidden_from_a_moderator() {
    let refusal = refused(
        "item replace block 1 64 1 container.0 with diamond",
        CommandPermission::Moderator,
    );
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

#[test]
fn item_modify_block_reports_the_target_refusals() {
    let missing = ran(
        "item modify block 9 9 9 container.0 {function:'minecraft:discard'}",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&missing), ["commands.item.target.not_a_container"]);
    let past_end = ran(
        "item modify block 1 64 1 container.30 {function:'minecraft:discard'}",
        chest_fixture(&[]),
    );
    assert_eq!(keys(&past_end), ["commands.item.target.no_such_slot"]);
    assert_eq!(arguments(&past_end, 0), vec![NbtSummaryValue::Int(30)]);
}

#[test]
fn item_modify_entity_reports_no_changes_for_a_slot_without_a_store() {
    let players = [user("alice", 1)];
    let run = ran(
        "item modify entity alice mob.inventory.0 {function:'minecraft:discard'}",
        world(&players),
    );
    assert_eq!(keys(&run), ["commands.item.target.no_changes"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(300)]);
}

#[test]
fn a_modifier_name_is_refused_against_an_empty_item_modifier_registry() {
    let refusal = ran(
        "item modify block 1 64 1 container.0 example:bump",
        chest_fixture(&[]),
    );
    assert_eq!(
        keys(&refusal),
        ["argument.resource_or_id.no_such_element", CURSOR_CONTEXT]
    );
    assert_eq!(
        arguments(&refusal, 0),
        vec![
            NbtSummaryValue::string("example:bump"),
            NbtSummaryValue::string("minecraft:item_modifier"),
        ]
    );
}

#[test]
fn item_sources_copy_native_slots_and_preserve_the_resulting_count() {
    let players = [user("alice", 1), user("bob", 2)];
    for (command, block, slot) in [
        (
            "item replace block 1 64 1 container.1 from block 1 64 1 container.0",
            true,
            1,
        ),
        (
            "item replace entity bob container.1 from block 1 64 1 container.0",
            false,
            1,
        ),
        (
            "item replace block 1 64 1 container.1 from entity alice container.0",
            true,
            1,
        ),
        (
            "item replace entity bob container.1 from entity alice container.0",
            false,
            1,
        ),
    ] {
        let mut fixture = chest_fixture(&players)
            .with_container_at(POS, &[RuntimeItemStack::from_summary(diamonds(3))])
            .with_player_item("alice", 36, diamonds(3));
        let run = ran(command, &mut fixture);
        assert_eq!(run.result, Some(1), "{command}");
        let written = if block {
            fixture.container_item(POS, slot as usize).unwrap()
        } else {
            fixture.item_slot("bob", slot).unwrap()
        };
        assert_eq!(written.item_id(), Some(diamond_id()));
        assert_eq!(written.count(), 3);
    }
}

#[test]
fn item_from_reads_the_selected_hand_cursor_and_crafting_grid() {
    let players = [user("alice", 1)];
    for (source_slot, count) in [
        ("weapon.mainhand", 5),
        ("player.cursor", 7),
        ("player.crafting.2", 3),
    ] {
        let mut fixture = chest_fixture(&players).with_selected_slot("alice", 2);
        fixture.setup(&format!(
            "item replace entity alice {source_slot} with diamond {count}"
        ));
        let run = ran(
            &format!("item replace block 1 64 1 container.0 from entity alice {source_slot}"),
            &mut fixture,
        );
        assert_eq!(run.result, Some(1));
        assert_eq!(fixture.container_item(POS, 0).unwrap().count(), count);
    }
}

#[test]
fn item_modifiers_write_their_observable_stack_result() {
    for (modifier, expected_item, expected_count) in [
        ("{function:'minecraft:set_count',count:9}", Some(diamond_id()), 9),
        ("{function:'minecraft:set_count',count:999}", Some(diamond_id()), 64),
        ("{function:'minecraft:discard'}", None, 0),
        ("{function:'minecraft:set_item',item:'minecraft:emerald'}", Some(bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID), 3),
        ("[{function:'minecraft:set_count',count:7},{function:'minecraft:set_count',count:2,add:true}]", Some(diamond_id()), 9),
        ("{function:'minecraft:explosion_decay'}", Some(diamond_id()), 3),
    ] {
        let mut fixture = chest_fixture(&[]).with_container_at(POS, &[RuntimeItemStack::from_summary(diamonds(3))]);
        let run = ran_as(&format!("item modify block 1 64 1 container.0 {modifier}"), &CommandSource::server("console", CommandPermission::Owner), &mut fixture);
        assert_eq!(keys(&run), ["commands.item.block.set.success"], "{modifier}: {run:?}");
        let stack = fixture.container_item(POS, 0).unwrap();
        assert_eq!((stack.item_id(), stack.count()), (expected_item, expected_count), "{modifier}");
    }

    let players = [user("alice", 1)];
    let stand = stored_entity(
        10,
        100,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let wolf = stored_entity(
        11,
        101,
        [1.0, 64.0, 0.0],
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
            )
            .unwrap(),
        ),
    );
    let mut fixture = world(&players).with_entities(&[stand, wolf]);
    fixture.setup("team add red");
    fixture.setup("team add blue");
    for member in ["alice", "@e[type=armor_stand]"] {
        for (team, expected) in [(None, 1), (Some("red"), 7), (Some("blue"), 1)] {
            if let Some(team) = team {
                fixture.setup(&format!("team join {team} {member}"));
            }
            fixture.setup("item replace entity alice hotbar.0 with stone");
            let run = ran(
                &format!("execute as {member} run item modify entity alice hotbar.0 {{function:'minecraft:set_count',count:7,conditions:[{{condition:'minecraft:entity_properties',entity:'this',predicate:{{team:'red'}}}}]}}"),
                &mut fixture,
            );
            assert_eq!(run.result, Some(1), "{member} {team:?}: {run:?}");
            assert_eq!(
                fixture.item_slot("alice", 0).unwrap().count(),
                expected,
                "{member} {team:?}"
            );
        }
    }
    fixture.setup("data merge entity @e[type=wolf,limit=1] {NoAI:1b,Owner:[I;0,0,0,1]}");
    fixture.setup("team join red alice");
    for (team, expected) in [(None, 7), (Some("blue"), 1)] {
        if let Some(team) = team {
            fixture.setup(&format!("team join {team} @e[type=wolf,limit=1]"));
        }
        fixture.setup("item replace entity alice hotbar.0 with stone");
        let run = ran(
            "execute as @e[type=wolf,limit=1] run item modify entity alice hotbar.0 {function:'minecraft:set_count',count:7,conditions:[{condition:'minecraft:entity_properties',entity:'this',predicate:{team:'red'}}]}",
            &mut fixture,
        );
        assert_eq!(run.result, Some(1), "wolf {team:?}: {run:?}");
        assert_eq!(
            fixture.item_slot("alice", 0).unwrap().count(),
            expected,
            "wolf {team:?}"
        );
    }

    fixture.setup("scoreboard objectives add points dummy");
    fixture.setup("scoreboard players set offline points 5");
    fixture.setup("scoreboard players set alice points 7");
    fixture.setup("scoreboard players set @e[type=armor_stand,limit=1] points 11");
    for (member, target, objective, scale, expected) in [
        (
            "alice",
            "{type:'minecraft:fixed',name:'offline'}",
            "points",
            "0.5",
            4,
        ),
        ("alice", "'this'", "points", "1", 8),
        (
            "00000000-0000-0000-0000-00000000000a",
            "'this'",
            "points",
            "1",
            12,
        ),
        ("@e[type=wolf,limit=1]", "'this'", "points", "1", 1),
        ("alice", "'this'", "missing", "1", 1),
    ] {
        fixture.setup("item replace entity alice hotbar.0 with stone");
        let run = ran(
            &format!("execute as {member} run item modify entity alice hotbar.0 {{function:'minecraft:set_count',add:true,count:{{type:'minecraft:score',target:{target},score:'{objective}',scale:{scale}}}}}"),
            &mut fixture,
        );
        assert_eq!(
            run.result,
            Some(1),
            "{member} {target} {objective}: {run:?}"
        );
        assert_eq!(
            fixture.item_slot("alice", 0).unwrap().count(),
            expected,
            "{member} {target} {objective}"
        );
    }
    for (member, score, expected) in [
        ("alice", None, 1),
        ("alice", Some(9), 7),
        ("00000000-0000-0000-0000-00000000000a", None, 7),
        ("@e[type=wolf,limit=1]", None, 1),
        ("@e[type=wolf,limit=1]", Some(10), 7),
    ] {
        if let Some(score) = score {
            fixture.setup(&format!("scoreboard players set {member} points {score}"));
        }
        fixture.setup("item replace entity alice hotbar.0 with stone");
        let run = ran(
            &format!("execute as {member} run item modify entity alice hotbar.0 {{function:'minecraft:set_count',count:7,conditions:[{{condition:'minecraft:entity_scores',entity:'this',scores:{{points:{{min:8,max:11}}}}}}]}}"),
            &mut fixture,
        );
        assert_eq!(run.result, Some(1), "{member} {score:?}: {run:?}");
        assert_eq!(
            fixture.item_slot("alice", 0).unwrap().count(),
            expected,
            "{member} {score:?}"
        );
    }
}

#[test]
fn item_modifier_set_components_adds_and_removes_native_custom_data() {
    let mut fixture =
        chest_fixture(&[]).with_container_at(POS, &[RuntimeItemStack::from_summary(diamonds(3))]);
    let add = ran_as("item modify block 1 64 1 container.0 {function:'minecraft:set_components',components:{'minecraft:repair_cost':9,'minecraft:custom_data':{bbb:1}}}", &CommandSource::server("console", CommandPermission::Owner), &mut fixture);
    assert_eq!(add.result, Some(1));
    let stack = fixture.container_item(POS, 0).unwrap();
    assert_eq!(stack.summary().component_patch.repair_cost, Some(9));
    assert_eq!(
        stack.summary().component_patch.custom_data,
        Some(compound(vec![("bbb", NbtSummaryValue::Int(1))]))
    );
    let remove = ran_as("item modify block 1 64 1 container.0 {function:'minecraft:set_components',components:{'!minecraft:custom_data':{}}}", &CommandSource::server("console", CommandPermission::Owner), &mut fixture);
    assert_eq!(remove.result, Some(1));
    assert!(fixture
        .container_item(POS, 0)
        .unwrap()
        .summary()
        .component_patch
        .custom_data
        .is_none());
}

#[test]
fn item_named_modifier_uses_the_datapack_generation() {
    use bbb_server_datapack::loot::{
        parse_loot_function, LoadedLootRegistry, LootElement, LootElementValue, LootRegistries,
    };
    let loot = LootRegistries {
        registries: vec![LoadedLootRegistry {
            registry_id: "minecraft:item_modifier".into(),
            elements: vec![LootElement {
                id: "example:bump".into(),
                source_pack: "test".into(),
                value: LootElementValue::Modifier(
                    parse_loot_function(
                        &serde_json::json!({"function":"minecraft:set_count","count":5}),
                        None,
                    )
                    .unwrap(),
                ),
            }],
            tags: None,
        }],
        warnings: vec![],
        tag_warnings: vec![],
    };
    let mut fixture = chest_fixture(&[])
        .with_container_at(POS, &[RuntimeItemStack::from_summary(diamonds(2))])
        .with_loot(&loot);
    let run = ran_as(
        "item modify block 1 64 1 container.0 example:bump",
        &CommandSource::server("console", CommandPermission::Owner),
        &mut fixture,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(fixture.container_item(POS, 0).unwrap().count(), 5);
}

#[test]
fn item_modifiers_read_current_block_nbt_at_the_offset_command_origin() {
    let players = [user("alice", 1)];
    let mut fixture = chest_fixture(&players);
    let source = CommandSource::server("console", CommandPermission::Owner);
    fixture.setup("weather clear");
    fixture.setup("fillbiome -4 60 -4 7 71 7 minecraft:plains");
    let command = "execute positioned 1 65 1 run item modify entity alice hotbar.0 {function:'minecraft:set_count',count:7,conditions:[{condition:'minecraft:weather_check',raining:false,thundering:false},{condition:'minecraft:location_check',predicate:{biomes:'minecraft:plains',can_see_sky:true}},{condition:'minecraft:location_check',offsetY:-1,predicate:{block:{nbt:{id:'minecraft:chest',x:1,y:64,z:1,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}}}}]}";
    for (stored_count, expected_count) in [(3, 7), (4, 1)] {
        fixture.setup(&format!(
            "item replace block 1 64 1 container.0 with diamond {stored_count}"
        ));
        fixture.setup("item replace entity alice hotbar.0 with stone");
        let run = ran_as(command, &source, &mut fixture);
        assert_eq!(run.result, Some(1), "{stored_count}: {run:?}");
        let stack = fixture.item_slot("alice", 0).unwrap();
        assert_eq!(stack.item_resource_id(), Some("minecraft:stone"));
        assert_eq!(stack.count(), expected_count);
    }
}

#[test]
fn item_modifiers_read_storage_numbers_for_counts_conditions_and_components() {
    let mut fixture =
        chest_fixture(&[]).with_container_at(POS, &[RuntimeItemStack::from_summary(diamonds(3))]);
    let source = CommandSource::server("console", CommandPermission::Owner);
    fixture.setup("data merge storage demo:modifier {count:7.9d,fraction:1.0f}");
    let count = ran_as("item modify block 1 64 1 container.0 {function:'minecraft:set_count',count:{type:'minecraft:storage',storage:'demo:modifier',path:'count'}}", &source, &mut fixture);
    assert_eq!(count.result, Some(1));
    assert_eq!(fixture.container_item(POS, 0).unwrap().count(), 7);
    let model = ran_as("item modify block 1 64 1 container.0 {function:'minecraft:set_custom_model_data',conditions:[{condition:'minecraft:value_check',value:{type:'minecraft:storage',storage:'demo:modifier',path:'count'},range:7}],floats:{mode:'replace_all',values:[{type:'minecraft:storage',storage:'demo:modifier',path:'fraction'}]}}", &source, &mut fixture);
    assert_eq!(model.result, Some(1));
    let stack = fixture.container_item(POS, 0).unwrap();
    assert_eq!(stack.count(), 7);
    assert_eq!(
        stack.summary().component_patch.custom_model_data_floats.0,
        [1.0]
    );
}

#[test]
fn item_modifiers_default_non_single_numeric_storage_selections_to_zero() {
    let mut fixture =
        chest_fixture(&[]).with_container_at(POS, &[RuntimeItemStack::from_summary(diamonds(3))]);
    let source = CommandSource::server("console", CommandPermission::Owner);
    fixture.setup("data merge storage demo:modifier {many:[2,4],text:'hello'}");
    for path in ["missing", "many[]", "text"] {
        let run = ran_as(&format!("item modify block 1 64 1 container.0 {{function:'minecraft:set_count',add:true,count:{{type:'minecraft:storage',storage:'demo:modifier',path:'{path}'}}}}"), &source, &mut fixture);
        assert_eq!(run.result, Some(1), "{path}: {run:?}");
        assert_eq!(fixture.container_item(POS, 0).unwrap().count(), 3, "{path}");
    }
}

#[test]
fn item_modifier_tag_entries_use_the_published_datapack_members() {
    use bbb_server_datapack::loot::{parse_loot_table, LootElement, LootElementValue};

    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    let tags = startup
        .tags
        .registries
        .iter_mut()
        .find(|tags| tags.registry_id == "minecraft:item")
        .unwrap();
    tags.tags.insert(
        "example:treasure".into(),
        vec!["minecraft:diamond".into(), "minecraft:emerald".into()],
    );
    tags.tags.insert(
        "minecraft:planks".into(),
        vec!["minecraft:gold_ingot".into()],
    );
    let tables = &mut startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements;
    tables.push(LootElement {
        id: "example:contents".into(),
        source_pack: "test".into(),
        value: LootElementValue::Table(
            parse_loot_table(
                &serde_json::json!({
                    "type":"minecraft:chest", "pools":[{"rolls":1,"entries":[{
                        "type":"minecraft:tag", "name":"example:treasure", "expand":false
                    }]}]
                }),
                None,
            )
            .unwrap(),
        ),
    });
    tables.sort_by(|left, right| left.id.cmp(&right.id));
    let mut fixture = CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            datapacks: Some(std::sync::Arc::new(startup)),
            ..Default::default()
        },
        &[],
    )
    .with_chest(POS);
    fixture.setup("item replace block 1 64 1 container.0 with shulker_box");
    let source = CommandSource::server("console", CommandPermission::Owner);
    for (entry, expected) in [
        (
            "{type:'minecraft:tag',name:'example:treasure',expand:false}",
            vec![
                (bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID, 1),
                (bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID, 1),
            ],
        ),
        (
            "{type:'minecraft:tag',name:'minecraft:planks',expand:true}",
            vec![(bbb_protocol::item_types::VANILLA_ITEM_GOLD_INGOT_ID, 1)],
        ),
        (
            "{type:'minecraft:tag',name:'example:missing',expand:false}",
            vec![],
        ),
        (
            "{type:'minecraft:loot_table',value:'example:contents'}",
            vec![
                (bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID, 1),
                (bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID, 1),
            ],
        ),
    ] {
        let run = ran_as(&format!("item modify block 1 64 1 container.0 {{function:'minecraft:set_contents',component:'minecraft:container',entries:[{entry}]}}"), &source, &mut fixture);
        assert_eq!(run.result, Some(1), "{entry}: {run:?}");
        let stack = fixture.container_item(POS, 0).unwrap();
        let mut contents: Vec<_> = stack
            .summary()
            .component_patch
            .container
            .items()
            .map(|item| (item.item_id, item.count))
            .collect();
        let mut expected = expected;
        contents.sort();
        expected.sort();
        assert_eq!(contents, expected, "{entry}");
    }

    let players = [user("alice", 1)];
    let stand = stored_entity(
        10,
        100,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let mut fixture = chest_fixture(&players).with_entities(&[stand]);
    fixture.setup("item replace block 1 64 1 container.0 with shulker_box");
    fixture.setup("item replace entity alice weapon.mainhand with diamond[minecraft:custom_data={bbb:7},minecraft:repair_cost=9] 3");
    fixture.setup("item replace entity alice weapon.offhand with emerald 2");
    fixture.setup("item replace entity alice hotbar.1 with diamond 5");
    fixture.setup("item replace entity @e[type=armor_stand] weapon.mainhand from entity alice weapon.mainhand");
    let original = fixture.item_slot("alice", 0).unwrap();
    for (executor, slot_source, expected) in [
        (
            "alice",
            "{type:'minecraft:slot_range',source:'this',slots:'weapon.mainhand'}",
            vec![(diamond_id(), 3)],
        ),
        (
            "@e[type=armor_stand]",
            "{type:'minecraft:slot_range',source:'this',slots:'weapon.mainhand'}",
            vec![(diamond_id(), 3)],
        ),
        (
            "alice",
            "{type:'minecraft:slot_range',source:'this',slots:'weapon.*'}",
            vec![(diamond_id(), 3), (bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID, 2)],
        ),
        (
            "alice",
            "{type:'minecraft:group',terms:[{type:'minecraft:empty'},{type:'minecraft:slot_range',source:'this',slots:'weapon.offhand'},{type:'minecraft:slot_range',source:'this',slots:'weapon.mainhand'},{type:'minecraft:slot_range',source:'this',slots:'weapon.offhand'}]}",
            vec![(bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID, 2), (diamond_id(), 3), (bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID, 2)],
        ),
        (
            "alice",
            "{type:'minecraft:empty'}",
            vec![],
        ),
        (
            "alice",
            "{type:'minecraft:filtered',slot_source:{type:'minecraft:slot_range',source:'this',slots:'weapon.*'},item_filter:{items:'minecraft:diamond',components:{'minecraft:custom_data':{bbb:7},'minecraft:repair_cost':9}}}",
            vec![(diamond_id(), 3)],
        ),
        (
            "alice",
            "{type:'minecraft:limit_slots',slot_source:{type:'minecraft:group',terms:[{type:'minecraft:slot_range',source:'this',slots:'hotbar.8'},{type:'minecraft:slot_range',source:'this',slots:'hotbar.1'}]},limit:1}",
            vec![],
        ),
        (
            "alice",
            "{type:'minecraft:limit_slots',slot_source:{type:'minecraft:group',terms:[{type:'minecraft:slot_range',source:'this',slots:'hotbar.8'},{type:'minecraft:slot_range',source:'this',slots:'hotbar.1'}]},limit:2}",
            vec![(diamond_id(), 5)],
        ),
    ] {
        let run = ran_as(&format!("execute as {executor} run item modify block 1 64 1 container.0 {{function:'minecraft:set_contents',component:'minecraft:container',entries:[{{type:'minecraft:slots',slot_source:{slot_source}}}]}}"), &source, &mut fixture);
        assert_eq!(run.result, Some(1), "{slot_source}: {run:?}");
        let stack = fixture.container_item(POS, 0).unwrap();
        let summary = stack.summary();
        let contents = &summary.component_patch.container;
        assert_eq!(contents.items().map(|item| (item.item_id, item.count)).collect::<Vec<_>>(), expected, "{slot_source}");
        for item in contents.items().filter(|item| item.item_id == diamond_id() && item.count == 3) {
            assert_eq!(item.component_patch.custom_data, original.summary().component_patch.custom_data, "{slot_source}");
            assert_eq!(item.component_patch.repair_cost, Some(9), "{slot_source}");
        }
        assert_eq!(fixture.item_slot("alice", 0).unwrap(), original);
    }
    fixture.setup(
        "item replace block 1 64 1 container.1 from entity @e[type=armor_stand,limit=1] weapon.mainhand",
    );
    assert_eq!(fixture.container_item(POS, 1).unwrap(), original);

    for (item, component, value) in [
        (
            "shulker_box",
            "minecraft:container",
            "[{slot:4,item:{id:'minecraft:diamond',count:6}}]",
        ),
        (
            "bundle",
            "minecraft:bundle_contents",
            "[{id:'minecraft:diamond',count:6}]",
        ),
        (
            "crossbow",
            "minecraft:charged_projectiles",
            "[{id:'minecraft:diamond',count:6}]",
        ),
    ] {
        fixture.setup(&format!(
            "item replace entity alice hotbar.2 with {item}[{component}={value}]"
        ));
        let original = fixture.item_slot("alice", 2).unwrap();
        let run = ran_as(&format!("execute as alice run item modify block 1 64 1 container.0 {{function:'minecraft:set_contents',component:'minecraft:container',entries:[{{type:'minecraft:slots',slot_source:{{type:'minecraft:limit_slots',limit:1,slot_source:{{type:'minecraft:contents',slot_source:{{type:'minecraft:slot_range',source:'this',slots:'hotbar.2'}},component:'{component}'}}}}}}]}}"), &source, &mut fixture);
        assert_eq!(run.result, Some(1), "{component}: {run:?}");
        let stack = fixture.container_item(POS, 0).unwrap();
        assert_eq!(
            stack
                .summary()
                .component_patch
                .container
                .items()
                .map(|item| (item.item_id, item.count))
                .collect::<Vec<_>>(),
            [(diamond_id(), 6)],
            "{component}"
        );
        assert_eq!(fixture.item_slot("alice", 2).unwrap(), original);
    }
}
