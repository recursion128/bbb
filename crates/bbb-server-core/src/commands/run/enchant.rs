//! `/enchant` (`EnchantCommand.java:39-111`).

use crate::inventory;
use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID;
use bbb_protocol::packets::{ItemEnchantmentSummary, ItemStackSummary};
use bbb_protocol::{
    component_literal_nbt, component_with_siblings_nbt, JavaString, NbtSummaryEntry,
    NbtSummaryValue,
};
use bbb_server_datapack::enchantment::EnchantmentFacts;
use bbb_server_datapack::language::component_plain_text;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::CommandPlayer;
use crate::commands::{translatable, CommandCx, CommandPlayerTarget, CommandSource};

use super::{
    integer_argument, resource_argument, select_damage_entity_states, CommandRun, Outcome,
};
use crate::commands::types::CommandDamageEntityRef;
use crate::commands::CommandOrigin;

/// `EquipmentSlot.MAINHAND.getIndex(98)` (`SlotRanges.java:27-32`).
const MAINHAND_SLOT: i32 = 98;

/// `EnchantCommand.enchant` (`EnchantCommand.java:64-111`).
pub(super) fn enchant_items(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let enchantments = cx.world.enchantments();
    let (enchanted, holder_id, level, report) = cx
        .world
        .with_entity_reads(|reads| plan_enchant(parse, source, reads, &enchantments, &cx.origin))?;
    let success = i32::try_from(enchanted.len()).unwrap_or(i32::MAX);
    for target in enchanted {
        match target {
            EnchantTarget::Player(player) => {
                cx.world
                    .enchant_command_item(player, MAINHAND_SLOT, holder_id, level, cx.events)
            }
            EnchantTarget::Stored { level: key, uuid } => cx.world.enchant_stored_command_item(
                &key,
                uuid,
                MAINHAND_SLOT,
                holder_id,
                level,
                cx.events,
            ),
        }
    }
    run.succeed(report, true);
    Ok(success)
}

enum EnchantTarget {
    Player(CommandPlayerTarget),
    Stored {
        level: bbb_server_types::save::Java26LevelKey,
        uuid: uuid::Uuid,
    },
}

/// Everything `enchant` decides before it touches an item: who is enchanted,
/// with what, and the line that reports it.
fn plan_enchant(
    parse: &ParseResult,
    source: &CommandSource,
    reads: &crate::world::EntityReads<'_>,
    enchantments: &[EnchantmentFacts],
    at: &CommandOrigin,
) -> Result<(Vec<EnchantTarget>, i32, i32, NbtSummaryValue), NbtSummaryValue> {
    let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
        unreachable!("targets is an entity argument");
    };
    let targets = select_damage_entity_states(selector, source, at, reads)?;
    if targets.is_empty() {
        return Err(translatable("argument.entity.notfound.entity", Vec::new()));
    }
    let enchantment_id = resource_argument(parse, "enchantment");
    let enchantment = enchantments
        .iter()
        .find(|entry| entry.resource_id == enchantment_id)
        .expect("the resource argument only resolves registry ids");
    let level = parse
        .argument("level")
        .map(|_| integer_argument(parse, "level"))
        .unwrap_or(1);
    if level > enchantment.max_level {
        return Err(translatable(
            "commands.enchant.failed.level",
            vec![
                NbtSummaryValue::Int(level),
                NbtSummaryValue::Int(enchantment.max_level),
            ],
        ));
    }

    let mut enchanted = Vec::new();
    for target in &targets {
        apply_to_target(
            &mut enchanted,
            *target,
            reads,
            enchantment,
            enchantments,
            targets.len(),
        )?;
    }
    if enchanted.is_empty() {
        return Err(translatable("commands.enchant.failed", Vec::new()));
    }

    let fullname = enchantment_fullname(enchantment, level);
    let report = if targets.len() == 1 {
        translatable(
            "commands.enchant.success.single",
            vec![fullname, targets[0].display_name(reads)],
        )
    } else {
        translatable(
            "commands.enchant.success.multiple",
            vec![
                fullname,
                NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX)),
            ],
        )
    };
    Ok((enchanted, enchantment.registry_id, level, report))
}

/// One target's turn: queued for enchanting, skipped, or (alone) refused.
fn apply_to_target(
    enchanted: &mut Vec<EnchantTarget>,
    target: CommandDamageEntityRef<'_>,
    reads: &crate::world::EntityReads<'_>,
    enchantment: &EnchantmentFacts,
    enchantments: &[EnchantmentFacts],
    target_count: usize,
) -> Result<(), NbtSummaryValue> {
    let single = target_count == 1;
    match target {
        CommandDamageEntityRef::Player(player) => {
            apply_to_player(enchanted, player, reads, enchantment, enchantments, single)
        }
        CommandDamageEntityRef::Stored(stored) => {
            let Some(living) = stored.entity.living() else {
                return refuse_or_skip(
                    single,
                    translatable(
                        "commands.enchant.failed.entity",
                        vec![NbtSummaryValue::string(entity_name_string(target, reads))],
                    ),
                );
            };
            let Some(stack) = living.item_by_slot(EquipmentSlot::MainHand) else {
                return refuse_or_skip(
                    single,
                    translatable(
                        "commands.enchant.failed.itemless",
                        vec![NbtSummaryValue::string(entity_name_string(target, reads))],
                    ),
                );
            };
            let Some(item_resource_id) =
                stack.item_resource_id_with_registry(reads.content_registry())
            else {
                return refuse_or_skip(
                    single,
                    translatable(
                        "commands.enchant.failed.itemless",
                        vec![NbtSummaryValue::string(entity_name_string(target, reads))],
                    ),
                );
            };
            if !enchantment.can_enchant(item_resource_id)
                || !enchantments_compatible(stack.summary(), enchantment, enchantments)
            {
                return refuse_or_skip(
                    single,
                    translatable(
                        "commands.enchant.failed.incompatible",
                        vec![NbtSummaryValue::string(item_hover_name_string(
                            stack, reads,
                        ))],
                    ),
                );
            }
            enchanted.push(EnchantTarget::Stored {
                level: stored.dimension.clone(),
                uuid: stored.entity.uuid(),
            });
            Ok(())
        }
    }
}

fn apply_to_player(
    enchanted: &mut Vec<EnchantTarget>,
    player: CommandPlayer<'_>,
    reads: &crate::world::EntityReads<'_>,
    enchantment: &EnchantmentFacts,
    enchantments: &[EnchantmentFacts],
    single: bool,
) -> Result<(), NbtSummaryValue> {
    let Some(stack) = player.inventory.slot(MAINHAND_SLOT) else {
        return refuse_or_skip(
            single,
            translatable(
                "commands.enchant.failed.itemless",
                vec![NbtSummaryValue::string(player.name)],
            ),
        );
    };
    if inventory::is_empty(stack.summary()) {
        return refuse_or_skip(
            single,
            translatable(
                "commands.enchant.failed.itemless",
                vec![NbtSummaryValue::string(player.name)],
            ),
        );
    }
    let item_resource_id = stack
        .item_resource_id_with_registry(reads.content_registry())
        .unwrap_or("minecraft:air");
    if !enchantment.can_enchant(item_resource_id)
        || !enchantments_compatible(stack.summary(), enchantment, enchantments)
    {
        return refuse_or_skip(
            single,
            translatable(
                "commands.enchant.failed.incompatible",
                vec![NbtSummaryValue::string(item_hover_name_string(
                    &stack, reads,
                ))],
            ),
        );
    }
    enchanted.push(EnchantTarget::Player(player.target()));
    Ok(())
}

fn refuse_or_skip(single: bool, refusal: NbtSummaryValue) -> Result<(), NbtSummaryValue> {
    if single {
        Err(refusal)
    } else {
        Ok(())
    }
}

/// `EnchantmentHelper.isEnchantmentCompatible` over
/// `getEnchantmentsForCrafting` (`EnchantCommand.java:78-79`).
fn enchantments_compatible(
    stack: &ItemStackSummary,
    target: &EnchantmentFacts,
    enchantments: &[EnchantmentFacts],
) -> bool {
    for existing in crafting_enchantments(stack) {
        let Some(facts) = enchantments
            .iter()
            .find(|entry| entry.registry_id == existing.holder_id)
        else {
            return false;
        };
        if !facts.are_compatible(target) {
            return false;
        }
    }
    true
}

/// `EnchantmentHelper.getEnchantmentsForCrafting` (`EnchantmentHelper.java:77-83`).
fn crafting_enchantments(stack: &ItemStackSummary) -> &[ItemEnchantmentSummary] {
    if stack.item_id == Some(VANILLA_ITEM_ENCHANTED_BOOK_ID) {
        &stack.component_patch.stored_enchantments
    } else {
        &stack.component_patch.enchantments
    }
}

/// `Enchantment.getFullname` (`Enchantment.java:171-184`).
fn enchantment_fullname(enchantment: &EnchantmentFacts, level: i32) -> NbtSummaryValue {
    let color = if enchantment.is_curse {
        ChatFormatting::Red
    } else {
        ChatFormatting::Gray
    };
    let mut entries = match enchantment.description.clone() {
        NbtSummaryValue::String(text) => vec![NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String(text),
        }],
        NbtSummaryValue::Compound(entries) => entries,
        _ => unreachable!("the native codec projects a string or compound component"),
    };
    if !entries.iter().any(|entry| entry.name == "color") {
        entries.push(NbtSummaryEntry {
            name: "color".into(),
            value: NbtSummaryValue::String(color.as_str().into()),
        });
    }
    let styled = NbtSummaryValue::Compound(entries);
    if level != 1 || enchantment.max_level != 1 {
        component_with_siblings_nbt(
            styled,
            vec![
                component_literal_nbt(" "),
                translatable(format!("enchantment.level.{level}"), Vec::new()),
            ],
        )
        .expect("a level suffix is representable")
    } else {
        styled
    }
}

/// `Entity.getName().getString()` (`EnchantCommand.java:86`, `:89`).
fn entity_name_string(
    entity: CommandDamageEntityRef<'_>,
    reads: &crate::world::EntityReads<'_>,
) -> JavaString {
    let name = super::selector::entity_name(entity, reads).unwrap_or_else(|_| match entity {
        CommandDamageEntityRef::Player(player) => component_literal_nbt(player.name),
        CommandDamageEntityRef::Stored(stored) => {
            let resource_id = stored.entity.data().entity_resource_id();
            let (namespace, path) = resource_id
                .split_once(':')
                .expect("native entity ids are canonical resource locations");
            translatable(format!("entity.{namespace}.{path}"), Vec::new())
        }
    });
    component_plain_text(&name).into()
}

/// `ItemStack.getHoverName().getString()` (`EnchantCommand.java:83`).
fn item_hover_name_string(
    stack: &RuntimeItemStack,
    reads: &crate::world::EntityReads<'_>,
) -> JavaString {
    let hover =
        super::give::runtime_item_stack_hover_name(stack, reads.resolver()).unwrap_or_else(|_| {
            let resource_id = stack
                .item_resource_id_with_registry(reads.content_registry())
                .unwrap_or("minecraft:air");
            let (namespace, path) = resource_id.split_once(':').unwrap_or(("minecraft", "air"));
            translatable(format!("item.{namespace}.{path}"), Vec::new())
        });
    component_plain_text(&hover).into()
}
