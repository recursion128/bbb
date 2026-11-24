//! `/loot` source contexts and delivery.

use bbb_protocol::NbtSummaryValue;

use super::*;

const DIAMOND_TABLE: &str =
    "{type:'minecraft:chest',pools:[{rolls:1,entries:[{type:'minecraft:item',name:'minecraft:diamond'}]}]}";

/// `ExplosionCondition` names `EXPLOSION_RADIUS` (`ExplosionCondition.java:23-36`),
/// which `LootContextParamSets.CHEST` does not carry -- the shape every
/// `blocks/*` table has.
const EXPLOSION_TABLE: &str = "{type:'minecraft:block',pools:[{rolls:1,conditions:[{condition:'minecraft:survives_explosion'}],entries:[{type:'minecraft:item',name:'minecraft:diamond'}]}]}";

fn loot_world(players: &[NameAndId]) -> CommandTestWorld {
    world(players)
}

/// `LEVEL_GAMEMASTERS` (`LootCommand.java:59`); below it the root is gone.
#[test]
fn loot_needs_gamemaster() {
    let refusal = refused(
        &format!("loot give alice loot {DIAMOND_TABLE}"),
        CommandPermission::Moderator,
    );
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn loot_give_draws_a_chest_table() {
    let players = [user("alice", 1)];
    let mut fixture = loot_world(&players);
    let run = ran(
        &format!("loot give alice loot {DIAMOND_TABLE}"),
        &mut fixture,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.drop.success.single"]);
    assert!(!run.feedback[0].inform_admins);
    let stack = fixture.player_item("alice", 0).unwrap();
    assert_eq!(
        stack.item_id,
        Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond")
                .expect("diamond")
                .id
        )
    );
    assert_eq!(stack.count, 1);
}

#[test]
fn loot_give_empty_table_reports_multiple_zero() {
    let players = [user("alice", 1)];
    let run = ran(
        "loot give alice loot {type:'minecraft:chest',pools:[]}",
        loot_world(&players),
    );
    assert_eq!(run.result, Some(0));
    assert_eq!(keys(&run), ["commands.drop.success.multiple"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(0)]);
}

#[test]
fn loot_unknown_table_is_no_such_element() {
    let players = [user("alice", 1)];
    let run = ran(
        "loot give alice loot minecraft:missing",
        loot_world(&players),
    );
    assert_eq!(
        keys(&run),
        ["argument.resource_or_id.no_such_element", CURSOR_CONTEXT]
    );
}

#[test]
fn loot_kill_player_draws_its_empty_default_table() {
    let players = [user("alice", 1)];
    let run = ran("loot give alice kill alice", loot_world(&players));
    assert_eq!(run.result, Some(0));
    assert_eq!(keys(&run), ["commands.drop.success.multiple_with_table"]);
}

#[test]
fn a_parameter_the_chest_set_lacks_is_not_a_parse_refusal() {
    let run = ran_as(
        &format!("loot give @a loot {EXPLOSION_TABLE}"),
        &CommandSource::server("console", CommandPermission::Owner),
        loot_world(&[]),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.player"]);

    // The roll is not a refusal either: at roll time the read is
    // `context.getOptionalParameter`, and `survives_explosion` takes its absent
    // branch as true (`ExplosionCondition.java:27-36`), so the pool drops.
    let players = [user("alice", 1)];
    let rolled = ran(
        &format!("loot give @a loot {EXPLOSION_TABLE}"),
        loot_world(&players),
    );
    assert_eq!(keys(&rolled), ["commands.drop.success.single"]);
}

#[test]
fn fishing_uses_loaded_block_center_source_entity_and_actual_hands() {
    let players = [user("alice", 1)];
    let mut fixture = loot_world(&players);
    fixture.setup("item replace entity alice weapon.mainhand with diamond");
    fixture.setup("item replace entity alice weapon.offhand with diamond");
    let table = "{type:'minecraft:fishing',pools:[{rolls:1,conditions:[{condition:'minecraft:match_tool',predicate:{items:'minecraft:diamond'}},{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:player'}},{condition:'minecraft:location_check',predicate:{position:{x:1.5,y:64.5,z:1.5}}}],entries:[{type:'minecraft:item',name:'minecraft:emerald'}]}]}";
    for suffix in ["diamond", "mainhand", "offhand"] {
        let target = if suffix == "offhand" {
            "spawn 2 64 2"
        } else {
            "give alice"
        };
        let run = ran(
            &format!("loot {target} fish {table} 1 64 1 {suffix}"),
            &mut fixture,
        );
        assert_eq!(run.result, Some(1), "{suffix}: {run:?}");
        assert_eq!(keys(&run), ["commands.drop.success.single"]);
    }
    for (pos, suffix) in [("1 64 1", ""), ("1 64 1", "stone"), ("2 64 1", "diamond")] {
        let run = ran(
            format!("loot spawn 2 64 2 fish {table} {pos} {suffix}").trim_end(),
            &mut fixture,
        );
        assert_eq!(run.result, Some(0), "{pos} {suffix}: {run:?}");
    }
    let console = CommandSource::server("console", CommandPermission::Owner);
    let run = ran_as(
        &format!("loot spawn 2 64 2 fish {table} 1 64 1 diamond"),
        &console,
        &mut fixture,
    );
    assert_eq!(run.result, Some(0));
    let run = ran_as(
        &format!("loot give alice fish {table} 1 64 1 mainhand"),
        &console,
        &mut fixture,
    );
    assert_eq!(keys(&run), ["permissions.requires.entity"]);
    let run = ran(
        &format!("loot give alice fish {table} 10000 64 10000 diamond"),
        &mut fixture,
    );
    assert_eq!(keys(&run), ["argument.pos.unloaded"]);
}

#[test]
fn fishing_reads_stored_living_equipment_and_refuses_nonliving_hands() {
    let players = [user("alice", 1)];
    let stand = stored_entity(
        10,
        100,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let marker = stored_entity(
        11,
        101,
        [0.0, 64.0, 0.0],
        crate::entity::EntityData::Marker(crate::entity::MarkerData::default()),
    );
    let mut fixture = loot_world(&players).with_entities(&[stand, marker]);
    fixture.setup("item replace entity @e[type=armor_stand] weapon.offhand with diamond");
    let table = "{type:'minecraft:fishing',pools:[{rolls:1,conditions:[{condition:'minecraft:match_tool',predicate:{items:'minecraft:diamond'}},{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:armor_stand'}}],entries:[{type:'minecraft:item',name:'minecraft:emerald'}]}]}";
    let run = ran(
        &format!("execute as @e[type=armor_stand] run loot give alice fish {table} 0 64 0 offhand"),
        &mut fixture,
    );
    assert_eq!(run.result, Some(1), "{run:?}");
    assert_eq!(
        fixture.player_item("alice", 0).unwrap().item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID)
    );
    let level = level_key(Java26BuiltinDimension::Overworld);
    let source = CommandSource::stored_entity(
        "marker",
        Uuid::from_u128(11),
        fixture.stored_id(&level, Uuid::from_u128(11)).unwrap(),
        level,
        RichTextComponent::plain("marker"),
        CommandPermission::Owner,
    );
    let run = ran_as(
        &format!("loot give alice fish {table} 0 64 0 mainhand"),
        &source,
        &mut fixture,
    );
    assert_eq!(keys(&run), ["commands.drop.no_held_items"]);
}

#[test]
fn loot_mine_without_block_loot_table_uses_translatable_block_name() {
    let players = [user("alice", 1)];
    let mut fixture = loot_world(&players);
    fixture.setup("setblock 1 64 1 bedrock");
    let run = ran("loot give alice mine 1 64 1", &mut fixture);
    assert_eq!(keys(&run), ["commands.drop.no_loot_table.block"]);
    assert_eq!(
        arguments(&run, 0),
        vec![crate::commands::translatable(
            "block.minecraft.bedrock",
            Vec::new()
        )]
    );
}
