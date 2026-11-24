//! `/enchant` (`EnchantCommand.java:39-111`).

use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{ItemEnchantmentSummary, ItemStackSummary};
use bbb_server_datapack::enchantment::builtin_enchantment_registry;

use super::*;

fn facts(id: &str) -> &'static bbb_server_datapack::enchantment::EnchantmentFacts {
    builtin_enchantment_registry()
        .iter()
        .find(|entry| entry.resource_id == id)
        .unwrap_or_else(|| panic!("{id} is in the built-in pack"))
}

fn stack_of(resource_id: &str) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            vanilla_item_type_for_resource_id(resource_id)
                .unwrap_or_else(|| panic!("{resource_id} is a vanilla item"))
                .id,
        ),
        count: 1,
        component_patch: Default::default(),
    }
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn zombie() -> crate::entity::Entity {
    stored_entity(
        9,
        99,
        [1.0, 64.0, 1.0],
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            )
            .expect("zombie is living"),
        ),
    )
}

fn orb() -> crate::entity::Entity {
    stored_entity(
        9,
        99,
        [1.0, 64.0, 1.0],
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(3)),
    )
}

fn zombie_holding_sword() -> crate::entity::Entity {
    let nbt = bbb_registry_codec::item_stack_summary_to_persistent_nbt(&stack_of(
        "minecraft:diamond_sword",
    ))
    .expect("a diamond sword projects");
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &nbt,
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .expect("the projection reads back");
    let mut data = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .expect("zombie is living");
    data.living_mut().set_item_slot(
        bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
        Some(stack),
    );
    stored_entity(
        9,
        99,
        [1.0, 64.0, 1.0],
        crate::entity::EntityData::GeneralLiving(data),
    )
}

/// The enchantments on `name`'s main-hand stack (`weapon.mainhand`, slot 98).
fn main_hand_enchantments(world: &CommandTestWorld, name: &str) -> Vec<ItemEnchantmentSummary> {
    world
        .item_slot(name, 98)
        .map(|stack| stack.summary().component_patch.enchantments.clone())
        .unwrap_or_default()
}

fn sharpness(level: i32) -> Vec<ItemEnchantmentSummary> {
    vec![ItemEnchantmentSummary {
        holder_id: facts("minecraft:sharpness").registry_id,
        level,
    }]
}

/// `EnchantCommand.enchant` (`EnchantCommand.java:78-80`, `:97-103`).
#[test]
fn enchant_writes_the_selected_main_hand_and_names_the_one_target() {
    let players = [user("alice", 1)];
    let mut sworded =
        world(&players).with_player_item("alice", 36, stack_of("minecraft:diamond_sword").clone());
    let run = ran("enchant alice sharpness", &mut sworded);
    assert_eq!(main_hand_enchantments(&sworded, "alice"), sharpness(1));
    assert_eq!(keys(&run), ["commands.enchant.success.single"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(1));
    let args = arguments(&run, 0);
    assert_eq!(
        translate_key(&args[0]).as_deref(),
        Some("enchantment.minecraft.sharpness")
    );
    assert_eq!(field(&args[0], "color"), Some(&text("gray")));
    assert_eq!(args[1], display_name("alice", 1));
}

/// The optional `<level>` (`EnchantCommand.java:49-57`).
#[test]
fn enchant_uses_the_requested_level() {
    let players = [user("alice", 1)];
    let mut sworded =
        world(&players).with_player_item("alice", 36, stack_of("minecraft:diamond_sword").clone());
    let run = ran("enchant alice minecraft:sharpness 3", &mut sworded);
    assert_eq!(main_hand_enchantments(&sworded, "alice"), sharpness(3));
    assert_eq!(run.result, Some(1));
    let args = arguments(&run, 0);
    let extra = field(&args[0], "extra");
    let Some(NbtSummaryValue::List(siblings)) = extra else {
        panic!("a level suffix is appended: {:?}", args[0]);
    };
    assert_eq!(siblings[0], text(" "));
    assert_eq!(
        translate_key(&siblings[1]).as_deref(),
        Some("enchantment.level.3")
    );
}

/// `IntegerArgumentType.integer(0)` (`EnchantCommand.java:49`).
#[test]
fn enchant_allows_level_zero() {
    let players = [user("alice", 1)];
    let mut sworded =
        world(&players).with_player_item("alice", 36, stack_of("minecraft:diamond_sword").clone());
    let run = ran("enchant alice sharpness 0", &mut sworded);
    assert_eq!(keys(&run), ["commands.enchant.success.single"]);
    // `ItemEnchantments.Mutable.upgrade` drops a level below one.
    assert!(main_hand_enchantments(&sworded, "alice").is_empty());
}

/// `ERROR_LEVEL_TOO_HIGH` (`EnchantCommand.java:34-36`, `:68-69`).
#[test]
fn enchant_refuses_a_level_above_the_maximum() {
    let players = [user("alice", 1)];
    let mut sworded =
        world(&players).with_player_item("alice", 36, stack_of("minecraft:diamond_sword").clone());
    let run = ran("enchant alice sharpness 6", &mut sworded);
    assert_eq!(keys(&run), ["commands.enchant.failed.level"]);
    assert!(main_hand_enchantments(&sworded, "alice").is_empty());
    assert_eq!(run.result, None);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(6), NbtSummaryValue::Int(5)]
    );
}

/// `ResourceArgument.parse` (`ResourceArgument.java:113-119`).
#[test]
fn enchant_refuses_an_unknown_enchantment() {
    let players = [user("alice", 1)];
    let run = ran("enchant alice not_an_enchantment", world(&players));
    assert_eq!(
        failure_messages(&run)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.resource.not_found"]
    );
}

#[test]
fn enchant_needs_the_enchantment_argument() {
    let run = ran("enchant alice", world(&[user("alice", 1)]));
    assert_eq!(keys(&run), ["command.unknown.command", CURSOR_CONTEXT]);
}

/// `ERROR_NO_ITEM` (`EnchantCommand.java:28-30`, `:85-86`).
#[test]
fn enchant_refuses_an_empty_main_hand() {
    let players = [user("alice", 1)];
    let run = ran("enchant alice sharpness", world(&players));
    assert_eq!(keys(&run), ["commands.enchant.failed.itemless"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::string("alice")]);
}

/// `ERROR_INCOMPATIBLE` (`EnchantCommand.java:31-33`, `:82-83`) when
/// `canEnchant` is false (`Enchantment.java:186-188`).
#[test]
fn enchant_refuses_an_unsupported_item() {
    let players = [user("alice", 1)];
    let run = ran(
        "enchant alice sharpness",
        world(&players).with_player_item("alice", 36, stack_of("minecraft:dirt").clone()),
    );
    assert_eq!(keys(&run), ["commands.enchant.failed.incompatible"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::string("Dirt")]);
}

/// The same enchantment is not compatible with itself
/// (`Enchantment.areCompatible`, `Enchantment.java:167-168`).
#[test]
fn enchant_refuses_a_duplicate_enchantment() {
    let mut stack = stack_of("minecraft:diamond_sword");
    stack.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
    stack.component_patch.enchantments = vec![ItemEnchantmentSummary {
        holder_id: facts("minecraft:sharpness").registry_id,
        level: 1,
    }];
    let players = [user("alice", 1)];
    let run = ran(
        "enchant alice sharpness",
        world(&players).with_player_item("alice", 36, stack.clone()),
    );
    assert_eq!(keys(&run), ["commands.enchant.failed.incompatible"]);
}

/// Damage enchantments share `exclusive_set/damage`.
#[test]
fn enchant_refuses_an_exclusive_enchantment() {
    let mut stack = stack_of("minecraft:diamond_sword");
    stack.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
    stack.component_patch.enchantments = vec![ItemEnchantmentSummary {
        holder_id: facts("minecraft:smite").registry_id,
        level: 1,
    }];
    let players = [user("alice", 1)];
    let run = ran(
        "enchant alice sharpness",
        world(&players).with_player_item("alice", 36, stack.clone()),
    );
    assert_eq!(keys(&run), ["commands.enchant.failed.incompatible"]);
}

/// `ERROR_NOT_LIVING_ENTITY` (`EnchantCommand.java:25-27`, `:88-89`).
#[test]
fn enchant_refuses_a_non_living_stored_entity() {
    let entities = [orb()];
    let run = ran(
        "enchant 0-0-0-0-9 sharpness",
        CommandTestWorld::default().with_entities(&entities),
    );
    assert_eq!(keys(&run), ["commands.enchant.failed.entity"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::string("Experience Orb")]
    );
}

/// A living stored entity with no main-hand item (`EnchantCommand.java:85-86`).
#[test]
fn enchant_refuses_a_living_stored_entity_without_a_main_hand() {
    let entities = [zombie()];
    let run = ran(
        "enchant 0-0-0-0-9 sharpness",
        CommandTestWorld::default().with_entities(&entities),
    );
    assert_eq!(keys(&run), ["commands.enchant.failed.itemless"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::string("Zombie")]);
}

/// Any living entity's compatible main hand is enchanted (`EnchantCommand.java:75-80`).
#[test]
fn enchant_writes_a_stored_living_main_hand() {
    let entities = [zombie_holding_sword()];
    let mut fixture = CommandTestWorld::default().with_entities(&entities);
    let run = ran("enchant 0-0-0-0-9 sharpness 2", &mut fixture);
    assert_eq!(keys(&run), ["commands.enchant.success.single"]);
    assert_eq!(run.result, Some(1));
    let level = bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
    let held = fixture
        .stored_entity(&level, uuid::Uuid::from_u128(9))
        .and_then(|zombie| zombie.living())
        .and_then(|living| {
            living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        })
        .expect("the zombie still holds its sword");
    assert_eq!(held.summary().component_patch.enchantments, sharpness(2));
}

/// Multi-target skips single-target refusals and reports
/// `success.multiple` with `targets.size()` (`EnchantCommand.java:104-107`).
#[test]
fn enchant_skips_empty_hands_among_several_targets() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut sworded =
        world(&players).with_player_item("alice", 36, stack_of("minecraft:diamond_sword").clone());
    let run = ran("enchant @a sharpness", &mut sworded);
    assert_eq!(keys(&run), ["commands.enchant.success.multiple"]);
    assert_eq!(run.result, Some(1));
    assert_eq!(main_hand_enchantments(&sworded, "alice"), sharpness(1));
    assert_eq!(arguments(&run, 0)[1], NbtSummaryValue::Int(2));
}

/// Every skipped target and no write is `ERROR_NOTHING_HAPPENED`
/// (`EnchantCommand.java:93-94`).
#[test]
fn enchant_fails_when_every_target_is_skipped() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("enchant @a sharpness", world(&players));
    assert_eq!(keys(&run), ["commands.enchant.failed"]);
}

#[test]
fn enchant_needs_the_gamemaster_level() {
    let refusal = refused("enchant alice sharpness", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
    let players = [user("alice", 1)];
    let run = ran_as(
        "enchant alice sharpness",
        &source("alice", CommandPermission::GameMaster),
        world(&players).with_player_item("alice", 36, stack_of("minecraft:diamond_sword").clone()),
    );
    assert_eq!(keys(&run), ["commands.enchant.success.single"]);
}

#[test]
fn enchant_suggests_registry_ids() {
    let offered = completions_for(
        "enchant alice shar",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert!(
        texts(&offered)
            .iter()
            .any(|text| text == "minecraft:sharpness"),
        "{offered:?}"
    );
}

/// `Enchantment.getFullname` colours a curse red (`Enchantment.java:173-174`).
#[test]
fn enchant_names_a_curse_in_red() {
    let players = [user("alice", 1)];
    let run = ran(
        "enchant alice binding_curse",
        world(&players).with_player_item(
            "alice",
            36,
            stack_of("minecraft:diamond_chestplate").clone(),
        ),
    );
    assert_eq!(keys(&run), ["commands.enchant.success.single"]);
    let args = arguments(&run, 0);
    assert_eq!(
        translate_key(&args[0]).as_deref(),
        Some("enchantment.minecraft.binding_curse")
    );
    assert_eq!(field(&args[0], "color"), Some(&text("red")));
    assert_eq!(field(&args[0], "extra"), None);
}

#[test]
fn enchant_a_missing_target_uses_the_entity_key() {
    let run = ran("enchant bob sharpness", world(&[]));
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
}

/// `EnchantCommand.enchant` (`EnchantCommand.java:83`, `:86`) resolves
/// custom item hover names and custom entity names into plain strings.
#[test]
fn enchant_errors_use_custom_item_hover_name_and_custom_entity_name() {
    let players = [user("alice", 1)];
    let mut fixture = world(&players);
    fixture.setup(
        "item replace entity alice weapon.mainhand with dirt[minecraft:custom_name='Magic Mud']",
    );
    let run = ran("enchant alice sharpness", &mut fixture);
    assert_eq!(keys(&run), ["commands.enchant.failed.incompatible"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::string("Magic Mud")]
    );

    let mut entity_fixture = CommandTestWorld::default().with_entities(&[zombie()]);
    entity_fixture.setup("data merge entity 0-0-0-0-9 {CustomName:'Bob the Undead'}");
    let run = ran("enchant 0-0-0-0-9 sharpness", &mut entity_fixture);
    assert_eq!(keys(&run), ["commands.enchant.failed.itemless"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::string("Bob the Undead")]
    );
}
