//! `/give` (`GiveCommand.java:23-103`).

use super::*;

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
};
use bbb_protocol::packets::DataComponentPatchSummary;

fn giving(
    command: &str,
    mut fixture: CommandTestWorld,
) -> (CommandRun, bbb_protocol::packets::ItemStackSummary) {
    let run = ran(command, &mut fixture);
    let stack = fixture
        .player_item("alice", 0)
        .unwrap_or_else(bbb_protocol::packets::ItemStackSummary::empty);
    (run, stack)
}

fn diamond_id() -> i32 {
    bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID
}

/// [`world`] with the item stack publication a registry-backed component
/// resolves against.
fn patched_world(players: &[NameAndId]) -> CommandTestWorld {
    world(players)
}

/// `GiveCommand.giveItem` (`GiveCommand.java:48-103`) for one target and the
/// default count of 1 (`:31`).
#[test]
fn give_publishes_the_stack_and_single_success_line() {
    let players = [user("alice", 1)];
    let (run, stack) = giving("give alice diamond", world(&players));
    assert_eq!(stack.item_id, Some(diamond_id()));
    assert_eq!(stack.count, 1);
    assert_eq!(keys(&run), ["commands.give.success.single"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(1));
    let args = arguments(&run, 0);
    assert_eq!(args[0], NbtSummaryValue::Int(1));
    assert_eq!(
        translate_key(&args[1]).as_deref(),
        Some("chat.square_brackets")
    );
    assert_eq!(args[2], display_name("alice", 1));
}

/// The optional `<count>` (`GiveCommand.java:33-41`) and the multi-target
/// success arm (`:98-100`), which still uses `success.single`.
#[test]
fn give_counts_every_target_and_uses_the_requested_count() {
    let players = [user("alice", 1), user("bob", 2)];
    let (run, stack) = giving("give @a minecraft:diamond 7", world(&players));
    assert_eq!(stack.item_id, Some(diamond_id()));
    assert_eq!(stack.count, 7);
    assert_eq!(keys(&run), ["commands.give.success.single"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(7),
            arguments(&ran("give alice diamond", world(&players)), 0)[1].clone(),
            NbtSummaryValue::Int(2),
        ]
    );
    assert_eq!(run.result, Some(2));
}

/// `GiveCommand.MAX_ALLOWED_ITEMSTACKS` (`GiveCommand.java:21`, `:51-54`).
#[test]
fn give_refuses_more_than_a_hundred_stacks() {
    let players = [user("alice", 1)];
    let (run, _) = giving("give alice diamond 6401", world(&players));
    assert_eq!(keys(&run), ["commands.give.failed.toomanyitems"]);
    assert_eq!(run.result, Some(0));
    assert_eq!(arguments(&run, 0)[0], NbtSummaryValue::Int(6400));
}

/// `ItemParser.readItem` (`ItemParser.java:134-141`) through
/// `ERROR_UNKNOWN_ITEM`.
#[test]
fn give_refuses_an_unknown_item_id() {
    let players = [user("alice", 1)];
    let (run, _) = giving("give alice not_an_item", world(&players));
    assert_eq!(keys(&run), ["argument.item.id.invalid", CURSOR_CONTEXT]);
}

/// The `[...]` component patch (`ItemParser.java:128-131`, `:143-191`): the
/// entry reaches the stack the command hands the world.
#[test]
fn give_carries_a_component_patch_to_the_inventory() {
    let players = [user("alice", 1)];
    let (run, stack) = giving("give alice diamond[minecraft:damage=5]", world(&players));
    assert_eq!(keys(&run), ["commands.give.success.single"]);

    assert_eq!(stack.item_id, Some(diamond_id()));
    assert_eq!(stack.count, 1);
    assert_eq!(
        stack.component_patch,
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
            damage: Some(5),
            ..Default::default()
        }
    );
}

/// The optional `<count>` follows the patch (`GiveCommand.java:32-42`).
#[test]
fn give_reads_the_count_after_the_component_patch() {
    let players = [user("alice", 1)];
    let (_, stack) = giving("give alice diamond[minecraft:damage=5] 3", world(&players));

    assert_eq!(stack.count, 3);
    assert_eq!(stack.component_patch.damage, Some(5));
}

/// `[!<component>]` removes an entry the item's prototype carries
/// (`ItemParser.java:150-160`).
#[test]
fn give_removes_a_prototype_component() {
    let players = [user("alice", 1)];
    let (run, stack) = giving(
        "give alice wooden_sword[!minecraft:damage]",
        world(&players),
    );
    assert_eq!(keys(&run), ["commands.give.success.single"]);

    assert_eq!(
        stack.component_patch,
        DataComponentPatchSummary {
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
            ..Default::default()
        }
    );
}

/// Whitespace around the separator and the assignment is skipped
/// (`ItemParser.java:149`, `:168-171`, `:181-182`); `[]` is an empty patch
/// (`:148`).
#[test]
fn give_skips_whitespace_inside_the_component_patch() {
    let players = [user("alice", 1)];
    let (_, stack) = giving(
        "give alice diamond[ minecraft:damage = 5 , ]",
        world(&players),
    );

    assert_eq!(stack.component_patch.damage, Some(5));

    let (_, empty_stack) = giving("give alice diamond[]", world(&players));
    let stack = &empty_stack;
    assert_eq!(stack.component_patch, DataComponentPatchSummary::default());
}

/// `ERROR_REPEATED_COMPONENT` (`ItemParser.java:47-49`, `:154-155`) is
/// created without context, so `Commands.finishParsing` (`Commands.java:368`)
/// sends no cursor line.
#[test]
fn give_refuses_a_repeated_component() {
    let players = [user("alice", 1)];
    let (run, _) = giving(
        "give alice diamond[minecraft:damage=1,minecraft:damage=2]",
        world(&players),
    );
    assert_eq!(keys(&run), ["arguments.item.component.repeated"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::string("minecraft:damage")]
    );
}

/// `ERROR_UNKNOWN_COMPONENT` (`ItemParser.java:38-40`, `:200-206`).
#[test]
fn give_refuses_an_unknown_component() {
    let players = [user("alice", 1)];
    let (run, _) = giving(
        "give alice diamond[minecraft:not_a_component=1]",
        world(&players),
    );
    assert_eq!(
        keys(&run),
        ["arguments.item.component.unknown", CURSOR_CONTEXT]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::string("minecraft:not_a_component")]
    );
}

/// `ERROR_MALFORMED_COMPONENT` (`ItemParser.java:41-43`, `:209-217`).
#[test]
fn give_refuses_a_malformed_component_value() {
    let players = [user("alice", 1)];
    let (run, _) = giving(
        "give alice diamond[minecraft:damage=\"five\"]",
        world(&players),
    );
    assert_eq!(
        keys(&run),
        ["arguments.item.component.malformed", CURSOR_CONTEXT]
    );
    let args = arguments(&run, 0);
    assert_eq!(args[0], NbtSummaryValue::string("minecraft:damage"));
    assert!(matches!(&args[1], NbtSummaryValue::String(_)), "{args:?}");
}

fn component_entry(name: &str, value: NbtSummaryValue) -> bbb_protocol::NbtSummaryEntry {
    bbb_protocol::NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

/// `ItemInput.createItemStack` (`ItemInput.java:21-25`): the patch the item
/// argument parses reaches the effect as the summary, and that summary still
/// writes the persistent form a save holds.
///
/// This is where the three components of the projection batch used to stop:
/// `minecraft:kinetic_weapon` (`DataComponents.java:211-213`),
/// `minecraft:blocks_attacks` (`:205-207`) and `minecraft:glider` (`:198`) are
/// persistent, and none of them projected, so `createItemStack` answered
/// `ERROR_MALFORMED_ITEM`. All three cross the boundary now, and the two
/// records' `optionalFieldOf` defaults are still dropped on the way out.
#[test]
fn give_carries_the_components_the_stack_summary_now_projects() {
    let players = [user("alice", 1)];
    let (run, stack) = giving(
        "give alice diamond[minecraft:kinetic_weapon={delay_ticks:3},\
         minecraft:glider={},minecraft:blocks_attacks={}]",
        world(&players),
    );
    assert_eq!(keys(&run), ["commands.give.success.single"]);

    let weapon = stack
        .component_patch
        .kinetic_weapon
        .as_ref()
        .expect("the summary carries the weapon");
    assert_eq!(weapon.contact_cooldown_ticks, 10);
    assert_eq!(weapon.delay_ticks, 3);
    assert!(weapon.damage_conditions.is_none());
    assert_eq!(weapon.damage_multiplier_bits, 1.0f32.to_bits());

    let attacks = stack
        .component_patch
        .blocks_attacks
        .as_ref()
        .expect("the summary carries the block attacks");
    assert_eq!(
        attacks.damage_reductions,
        [bbb_protocol::packets::DamageReductionSummary {
            horizontal_blocking_angle_bits: 90.0f32.to_bits(),
            damage_types: None,
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }]
    );
    assert!(
        stack
            .component_patch
            .added_type_ids
            .contains(&bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID),
        "{:?}",
        stack.component_patch.added_type_ids
    );

    // The save form: the two records write only the fields away from their
    // defaults, and the unit component writes the empty compound.
    let persistent = bbb_registry_codec::item_stack_summary_to_persistent_nbt(&stack)
        .expect("the summary the inventory carries has a persistent form");
    let NbtSummaryValue::Compound(entries) = &persistent else {
        panic!("a persistent stack is a compound: {persistent:?}");
    };
    let components = entries
        .iter()
        .find(|entry| entry.name == "components")
        .expect("the patch is written");
    let NbtSummaryValue::Compound(components) = &components.value else {
        panic!("the components field is a compound: {components:?}");
    };
    let mut written = components.clone();
    written.sort_by(|left, right| left.name.cmp(&right.name));
    let mut expected = vec![
        component_entry(
            "minecraft:blocks_attacks",
            NbtSummaryValue::Compound(Vec::new()),
        ),
        component_entry("minecraft:glider", NbtSummaryValue::Compound(Vec::new())),
        component_entry(
            "minecraft:kinetic_weapon",
            NbtSummaryValue::Compound(vec![component_entry(
                "delay_ticks",
                NbtSummaryValue::Int(3),
            )]),
        ),
    ];
    expected.sort_by(|left, right| left.name.cmp(&right.name));
    assert_eq!(written, expected);
}

/// `ItemStack.validateStrict` (`ItemStack.java:159-168`) through
/// `validateComponents` (`:271-274`): a stack cannot be damageable and
/// stackable at once.
#[test]
fn give_refuses_a_patch_that_fails_strict_validation() {
    let players = [user("alice", 1)];
    let (run, _) = giving(
        "give alice wooden_sword[minecraft:max_stack_size=2]",
        world(&players),
    );
    assert_eq!(keys(&run), ["arguments.item.malformed"]);
}

/// A registry-backed component resolves through the publication the view
/// carries, on both the parse and the apply side.
#[test]
fn give_carries_an_enchantment_through_the_item_stack_publication() {
    let players = [user("alice", 1)];

    let (run, stack) = giving(
        "give alice diamond[minecraft:enchantments={\"minecraft:sharpness\":1}]",
        patched_world(&players),
    );
    assert_eq!(keys(&run), ["commands.give.success.single"]);

    assert_eq!(
        stack.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID]
    );
    assert_eq!(stack.component_patch.enchantments.len(), 1);
}

/// `IntegerArgumentType.integer(1)` (`GiveCommand.java:33`).
#[test]
fn give_refuses_a_zero_count() {
    let players = [user("alice", 1)];
    let (run, _) = giving("give alice diamond 0", world(&players));
    assert_eq!(keys(&run)[0], "argument.integer.low");
}

/// `ItemParser.fillSuggestions` (`ItemParser.java:101-113`) uses the
/// component stages retained by its parse (`:124-191`, `:219-266`).
#[test]
fn give_suggests_item_ids_and_component_patch_stages() {
    let players = [user("alice", 1)];
    let world = world(&players);
    let offered = completions_for("give alice dia", CommandPermission::Owner, &world);
    assert!(
        texts(&offered)
            .iter()
            .any(|text| text == "minecraft:diamond"),
        "{offered:?}"
    );
    assert_eq!((offered.start, offered.length), (11, 3));
    for (input, expected, suffix) in [
        ("diamond", "[", ""),
        ("diamond[", "minecraft:damage=", ""),
        ("diamond[", "!", ""),
        ("diamond[ ", "minecraft:damage=", ""),
        (
            "diamond[minecraft:dam",
            "minecraft:damage=",
            "minecraft:dam",
        ),
        ("diamond[!dam", "minecraft:damage", "dam"),
        ("diamond[damage", "=", ""),
        ("diamond[damage ", "=", ""),
        ("diamond[damage=1", ",", ""),
        ("diamond[damage=1", "]", ""),
        ("diamond[!damage", "]", ""),
        ("diamond[damage=1,", "!", ""),
        ("diamond[damage=1, unb", "minecraft:unbreakable=", "unb"),
        ("diamond[damage=1,!unb", "minecraft:unbreakable", "unb"),
        (
            "diamond[custom_data={name:'😀'},dam",
            "minecraft:damage=",
            "dam",
        ),
    ] {
        let command = format!("give alice {input}");
        let offered = completions_for(&command, CommandPermission::Owner, &world);
        assert!(
            texts(&offered).iter().any(|text| text == expected),
            "{command}: {offered:?}"
        );
        assert_eq!(
            (offered.start, offered.length),
            (
                command[..command.len() - suffix.len()]
                    .encode_utf16()
                    .count() as i32,
                suffix.encode_utf16().count() as i32
            ),
            "{command}"
        );
    }
    for input in [
        "diamond[damage=",
        "diamond[custom_data={name:'dam",
        "diamond[damage='invalid',",
        "diamond[damage=1]",
        "diamond[damage=1]dam",
    ] {
        let offered = completions_for(
            &format!("give alice {input}"),
            CommandPermission::Owner,
            &world,
        );
        assert!(texts(&offered).is_empty(), "{input}: {offered:?}");
    }
}

/// `ItemArgument` accepts `air`, whose stack is empty
/// (`ItemStack.java:320-322`): `getDisplayName` then names it without the
/// rarity colour or the item hover (`:1026-1028`), because
/// `HoverEvent.ShowItem` only carries a non-empty stack.
#[test]
fn give_names_an_air_stack_without_an_item_hover() {
    let players = [user("alice", 1)];
    let (run, _) = giving("give alice air", world(&players));
    assert_eq!(keys(&run), ["commands.give.success.single"]);
    let NbtSummaryValue::Compound(display) = &arguments(&run, 0)[1] else {
        panic!("the item name is a compound");
    };
    assert!(
        display.iter().all(|entry| entry.name != "hover_event"),
        "{display:?}"
    );
}

/// `GiveCommand.register` (`GiveCommand.java:26`) gates the root on
/// `LEVEL_GAMEMASTERS`; below it the root is gone.
#[test]
fn give_is_hidden_from_a_moderator() {
    let refusal = refused("give alice diamond", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}
