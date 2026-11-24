//! Selector parser errors and the remaining live selection rules.

use super::*;
use bbb_server_datapack::loot::{
    parse_loot_condition, LoadedLootRegistry, LootElement, LootElementValue, LootRegistries,
};

fn selected(mut fixture: CommandTestWorld, selector: &str) -> Vec<String> {
    ran(&format!("tag {selector} add marked"), &mut fixture);
    fixture.tagged("marked")
}

fn armor_stand(uuid: u128, entity_id: i32, position: [f64; 3]) -> crate::entity::Entity {
    stored_entity(
        uuid,
        entity_id,
        position,
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    )
}
fn zombie(uuid: u128, entity_id: i32, position: [f64; 3]) -> crate::entity::Entity {
    stored_entity(
        uuid,
        entity_id,
        position,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            )
            .expect("zombie is living"),
        ),
    )
}
fn predicates(entries: &[(&str, serde_json::Value)]) -> LootRegistries {
    LootRegistries {
        registries: vec![LoadedLootRegistry {
            registry_id: "minecraft:predicate".to_string(),
            elements: entries
                .iter()
                .map(|(id, body)| LootElement {
                    id: (*id).to_string(),
                    source_pack: "test".to_string(),
                    value: LootElementValue::Predicate(Box::new(
                        parse_loot_condition(body, None).expect("the fixture predicate parses"),
                    )),
                })
                .collect(),
            tags: None,
        }],
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    }
}

#[test]
fn rotations_wrap_around_and_level_reads_the_experience_level() {
    let players = [user("alice", 1), user("bob", 2), user("carol", 3)];
    let entities = [armor_stand(10, 100, [0.0, 0.0, 0.0])];

    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@a[y_rotation=170..-170]"
        ),
        ["bob"]
    );
    // `-450` wraps to `-90`.
    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@a[y_rotation=260..280]"
        ),
        ["carol"]
    );
    // An open minimum is `0` rather than unbounded.
    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@a[x_rotation=..0]"
        ),
        ["alice"]
    );
    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@e[x_rotation=-50..-40]"
        ),
        ["carol"]
    );
    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@e[x_rotation=0]"
        ),
        ["alice", "10"]
    );
    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@e[level=5..]"
        ),
        ["bob", "carol"]
    );
    assert_eq!(
        selected(
            world(&players)
                .with_player_rotation("bob", [175.0, 30.0])
                .with_player_experience("bob", PlayerExperience::from_saved(0.0, 5, 0))
                .with_player_rotation("carol", [-450.0, -45.0])
                .with_player_experience("carol", PlayerExperience::from_saved(0.0, 30, 0))
                .with_entities(&entities),
            "@e[level=..4]"
        ),
        ["alice"]
    );
}

#[test]
fn only_the_entity_selectors_keep_to_the_living() {
    let dead = user("bob", 2);
    let players = [user("alice", 1), dead];
    let mut corpse = zombie(11, 101, [0.0, 0.0, 0.0]);
    crate::entity::living::set_health(&mut corpse, 0.0);
    let entities = [armor_stand(10, 100, [0.0, 0.0, 0.0]), corpse];

    for (command, count) in [
        ("execute if entity @a", 2),
        ("execute if entity @e", 2),
        ("execute if entity @e[type=minecraft:player]", 1),
        ("execute if entity @e[type=minecraft:zombie]", 0),
        ("execute if entity @a[name=bob]", 1),
    ] {
        let run = ran(
            command,
            world(&players)
                .with_entities(&entities)
                .with_player_health("bob", 0.0),
        );
        assert_eq!(run.result, (count > 0).then_some(count), "{command}");
    }
}

#[test]
fn the_modelled_options_refuse_at_parse_with_their_own_message() {
    for (command, key) in [
        ("kill @e[limit=0]", "argument.entity.options.limit.toosmall"),
        (
            "kill @e[sort=sideways]",
            "argument.entity.options.sort.irreversible",
        ),
        (
            "kill @e[type=minecraft:nosuch]",
            "argument.entity.options.type.invalid",
        ),
        // `canUse` (`:490-494`): `@a` already limited itself to the player
        // type (EntitySelectorParser.java:210), `@s` is the current entity
        // (`:246`), and a second `limit=`/`sort=`/`type=` repeats one.
        (
            "kill @a[type=minecraft:armor_stand]",
            "argument.entity.options.inapplicable",
        ),
        ("kill @s[limit=1]", "argument.entity.options.inapplicable"),
        (
            "kill @e[limit=1,limit=2]",
            "argument.entity.options.inapplicable",
        ),
        (
            "kill @e[sort=nearest,sort=furthest]",
            "argument.entity.options.inapplicable",
        ),
        (
            "kill @e[type=!minecraft:zombie,type=minecraft:cow]",
            "argument.entity.options.inapplicable",
        ),
        ("kill @e[nosuchoption=1]", "argument.entity.options.unknown"),
        ("kill @e[limit]", "argument.entity.options.valueless"),
        ("kill @e[limit=1 2]", "argument.entity.options.unterminated"),
        ("kill @e[limit=1", "argument.entity.options.unterminated"),
    ] {
        let run = ran(command, CommandTestWorld::default());
        assert_eq!(keys(&run), [key, CURSOR_CONTEXT], "{command}");
    }
}

#[test]
fn a_refused_option_quotes_the_text_up_to_where_vanilla_stopped() {
    fn context(command: &str) -> Vec<String> {
        let run = ran(command, CommandTestWorld::default());
        run.feedback[1]
            .message
            .styled()
            .iter()
            .map(|styled| styled.text.clone())
            .collect()
    }
    // `setCursor(start)` before the throw (EntitySelectorOptions.java:169) puts
    // it on the value rather than on the `[`.
    assert_eq!(
        context("kill @e[limit=0]"),
        ["... @e[limit=", "0]", "command.context.here"]
    );
    // An unknown key rewinds to where the key began (`:496`), while an
    // inapplicable one reports from where it ended (`:493`).
    assert_eq!(
        context("kill @e[nosuchoption=1]"),
        ["kill @e[", "nosuchoption=1]", "command.context.here"]
    );
    assert_eq!(
        context("kill @s[limit=1]"),
        ["...l @s[limit", "=1]", "command.context.here"]
    );
}

#[test]
fn predicate_fire_uses_item_publication_and_blaze_virtual_override() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    let item = |removed: bool, id| {
        let mut fields = vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:netherite_sword".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
        ];
        if removed {
            fields.push(NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "!minecraft:damage_resistant".into(),
                    value: NbtSummaryValue::Compound(vec![]),
                }]),
            });
        }
        let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(fields),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .unwrap();
        stored_entity(
            id as u128,
            id,
            [0.0, 64.0, 0.0],
            crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(stack, 0.0)),
        )
    };
    let blaze = stored_entity(
        102,
        102,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLAZE_ID,
            )
            .unwrap(),
        ),
    );
    let mut entities = [item(false, 100), item(true, 101), blaze];
    for entity in &mut entities {
        entity.set_remaining_fire_ticks(40);
    }
    let loot = predicates(&[
        (
            "test:burning",
            serde_json::json!({"condition":"minecraft:entity_properties","entity":"this","predicate":{"flags":{"is_on_fire":true}}}),
        ),
        (
            "test:item",
            serde_json::json!({"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:item"}}),
        ),
    ]);

    assert_eq!(
        selected(
            world(&[]).with_entities(&entities).with_loot(&loot),
            "@e[predicate=test:burning]"
        ),
        ["101"]
    );
}
